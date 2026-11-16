// Building blocks for scenes: rig-aware recipes ("colour hits on the kick", "slow wave",
// "laser looks tagged peak") that each become one route. Setup templates are fixed lists
// of blocks; the scene builder lets you pick your own. Nothing here needs AI.
import type { Plan } from "./design";
import type { Genre } from "./genres";
import type { Fixture, Palette, Preset, Route, RouteAction, Show } from "./types";
import { activeFixtures, canGlow, hasLooks, hasType, lookTags, profileFor } from "./rig";

type Target = Pick<Route, "rooms" | "groups" | "fixtures">;

/** What the rig offers, in building-block terms. */
export function analyseRig(show: Show) {
  const fixtures = activeFixtures(show);
  const kind = (f: Fixture) => profileFor(show, f)?.kind ?? "";
  const glow = fixtures.filter((f) => canGlow(show, f) && kind(f) !== "laser" && kind(f) !== "fog" && !hasType(show, f, "fog"));
  const lasers = fixtures.filter((f) => hasLooks(show, f) && !glow.includes(f));
  const fog = fixtures.filter((f) => hasType(show, f, "fog"));
  const glowIds = new Set(glow.map((f) => f.id));
  // Groups made only of colour fixtures, biggest first.
  const glowGroups = show.groups
    .filter((g) => g.fixtures.length && g.fixtures.every((id) => glowIds.has(id) || !fixtures.some((f) => f.id === id)) && g.fixtures.some((id) => glowIds.has(id)))
    .sort((a, b) => b.fixtures.length - a.fixtures.length);
  /** Target exactly these fixtures, using your groups where they fit (biggest first), then the leftovers. */
  const target = (ids: string[]): Target => {
    const want = new Set(ids);
    const groups: string[] = [];
    const covered = new Set<string>();
    const fitting = show.groups.filter((g) => g.fixtures.length && g.fixtures.every((f) => want.has(f))).sort((a, b) => b.fixtures.length - a.fixtures.length);
    for (const g of fitting) {
      if (g.fixtures.every((f) => covered.has(f))) continue;
      groups.push(g.id);
      g.fixtures.forEach((f) => covered.add(f));
    }
    return { rooms: [], groups, fixtures: ids.filter((f) => !covered.has(f)) };
  };
  const all = target(glow.map((f) => f.id));
  // Two halves for call-and-response: two groups that don't cover everything, else split the list.
  const parts = glowGroups.filter((g) => g.fixtures.length < glow.length);
  const [a, b] = parts.length >= 2
    ? [{ rooms: [], groups: [parts[0].id], fixtures: [] }, { rooms: [], groups: [parts[1].id], fixtures: [] }] as Target[]
    : glow.length >= 2
      ? [target(glow.filter((_, i) => i % 2 === 0).map((f) => f.id)), target(glow.filter((_, i) => i % 2 === 1).map((f) => f.id))]
      : [all, all];
  const room = show.rooms.find((r) => r.id === show.active_room) ?? show.rooms.find((r) => r.focus.length);
  const focus = room?.focus[0]?.id ?? show.rooms.flatMap((r) => r.focus)[0]?.id ?? "";
  const band = (...ids: string[]) => ids.find((id) => show.audio.bands.some((b) => b.id === id && b.enabled)) ?? "";
  const tags = lookTags(show);
  return {
    glow, lasers, fog, all, a, b, focus, glowGroups, tags, target,
    laserTarget: target(lasers.map((f) => f.id)), fogTarget: target(fog.map((f) => f.id)),
    kick: band("kick") || show.audio.bands[0]?.id || "beat",
    snare: band("snare", "clap") || "beat",
    hat: band("hat", "hihat", "cymbals") || "beat",
    vocals: band("vocals") ? "vocals.level" : "level",
    calmTag: ["chill", "static", "calm", "slow"].find((t) => tags.includes(t)) ?? "",
    peakTag: ["peak", "dynamic", "drop"].find((t) => tags.includes(t)) ?? "",
  };
}
export type Rig = ReturnType<typeof analyseRig>;

// ---------- the catalogue ----------

export type Speed = "slow" | "medium" | "fast";
/** What a block drives: colour fixtures (brightness+colour), lasers (looks) or fog. */
export type Needs = "glow" | "laser" | "fog";

export interface BlockDef {
  id: string;
  name: string;
  description: string;
  needs: Needs;
  /** Default sound. "hit" blocks take a hit (band, beat, bar); "level" blocks a level. */
  sound?: { kind: "hit" | "level"; default: string };
  /** Default palette (a mood token or palette name) if the block uses colour. */
  palette?: string;
  /** Laser looks: default tag token. */
  tag?: string;
  /** Uses the focus point when there is one. */
  focus?: boolean;
}

export const BLOCKS: BlockDef[] = [
  { id: "colour_hits", name: "Colour hits", description: "Each hit steps to the next palette colour, at full brightness (rings out from the focus point).", needs: "glow", sound: { kind: "hit", default: "kick" }, palette: "hot", focus: true },
  { id: "colour_bar", name: "Colour changes", description: "Colour moves to the next palette colour on each hit (each bar by default) and leaves brightness to other blocks.", needs: "glow", sound: { kind: "hit", default: "bar" }, palette: "calm", focus: true },
  { id: "wave", name: "Wave", description: "A band of colour moves across the room in time with the beat (out from the focus point if there is one).", needs: "glow", palette: "calm", focus: true },
  { id: "sparkle", name: "Sparkle", description: "Random fixtures twinkle in palette colours.", needs: "glow", sound: { kind: "hit", default: "hat" }, palette: "hot" },
  { id: "flash", name: "Flash on hit", description: "Brightness jumps up on each hit and fades back down.", needs: "glow", sound: { kind: "hit", default: "kick" } },
  { id: "strobe", name: "Strobe", description: "A strobe burst after each hit.", needs: "glow", sound: { kind: "hit", default: "snare" } },
  { id: "node_hop", name: "Node hop", description: "One fixture flashes per hit, hopping to a neighbour each time, in palette colours.", needs: "glow", sound: { kind: "hit", default: "kick" }, palette: "hot" },
  { id: "follow", name: "Brightness follows", description: "Brightness follows a level: energy (rises in builds), overall level, or a band such as vocals.", needs: "glow", sound: { kind: "level", default: "energy" } },
  { id: "laser", name: "Laser looks", description: "Changes to a saved look on the hit (each bar by default), picked by tag.", needs: "laser", sound: { kind: "hit", default: "bar" }, tag: "" },
  { id: "fog", name: "Fog burst", description: "A short burst every few bars, never more than one per cooldown.", needs: "fog" },
];
export const blockDef = (id: string) => BLOCKS.find((b) => b.id === id);

/** One block in a scene. Tokens are resolved against the rig when built. */
export interface BlockUse {
  block: string;
  /** "all" | "a" | "b" (halves) | "lasers" | "fog" | "g:<group id>" */
  target: string;
  speed: Speed;
  /** Sound token ("kick", "snare", "hat", "beat", "bar", "energy", "level", "vocals") or a band id / "<band>.level". */
  sound?: string;
  /** "calm" | "hot" (the genre's moods) or a palette id or name. */
  palette?: string;
  /** Laser look tag: "" any, "calm"/"peak" (best match in your tags), or a tag. */
  tag?: string;
}
export interface SceneSpec { name: string; blocks: BlockUse[] }

export function defaultTarget(needs: Needs) {
  return needs === "laser" ? "lasers" : needs === "fog" ? "fog" : "all";
}
export function newBlock(id: string): BlockUse {
  const d = blockDef(id)!;
  return { block: id, target: defaultTarget(d.needs), speed: "medium", sound: d.sound?.default, palette: d.palette, tag: d.tag };
}

/** Does the rig have anything this block can drive? */
export function available(rig: Rig, needs: Needs) {
  return needs === "laser" ? rig.lasers.length > 0 : needs === "fog" ? rig.fog.length > 0 : rig.glow.length > 0;
}

/** Target choices for a block, with labels. */
export function targetOptions(show: Show, rig: Rig, needs: Needs): { id: string; label: string }[] {
  const name = (t: Target) => [...t.groups.map((g) => show.groups.find((x) => x.id === g)?.name ?? g),
    ...(t.fixtures.length > 2 ? [`${t.fixtures.length} fixtures`] : t.fixtures.map((f) => show.fixtures.find((x) => x.id === f)?.name ?? f))].join(" + ");
  const memberOf = (ids: Set<string>) => show.groups.filter((g) => g.fixtures.some((f) => ids.has(f)));
  if (needs === "laser") return [{ id: "lasers", label: "All lasers" }, ...memberOf(new Set(rig.lasers.map((f) => f.id))).map((g) => ({ id: `g:${g.id}`, label: g.name }))];
  if (needs === "fog") return [{ id: "fog", label: "All fog machines" }, ...memberOf(new Set(rig.fog.map((f) => f.id))).map((g) => ({ id: `g:${g.id}`, label: g.name }))];
  const opts = [{ id: "all", label: "All colour fixtures" }];
  if (rig.glow.length >= 2) opts.push({ id: "a", label: `Half A (${name(rig.a)})` }, { id: "b", label: `Half B (${name(rig.b)})` });
  for (const g of memberOf(new Set(rig.glow.map((f) => f.id)))) opts.push({ id: `g:${g.id}`, label: g.name });
  return opts;
}

/** Sound choices for a block. */
export function soundOptions(show: Show, kind: "hit" | "level") {
  const bands = show.audio.bands.filter((b) => b.enabled);
  return kind === "hit"
    ? [...bands.map((b) => ({ id: b.id, label: `${b.name} hit` })), { id: "beat", label: "Every beat" }, { id: "bar", label: "Every bar" }]
    : [{ id: "energy", label: "Energy (build/drop)" }, { id: "level", label: "Overall level" }, ...bands.map((b) => ({ id: `${b.id}.level`, label: `${b.name} level` }))];
}

// ---------- building ----------

// Palette moods per genre: [calm, hot].
const MOODS: Record<string, [string, string]> = {
  house: ["Sunset", "Club"], techno: ["Ice", "White"], trance: ["Ice", "Rainbow"], dnb: ["Ice", "Club"], dubstep: ["Club", "White"],
  hiphop: ["Sunset", "Club"], latin: ["Sunset", "Rainbow"], pop: ["Rainbow", "Club"], rock: ["Sunset", "White"], ambient: ["Forest", "Ice"],
};
export const moodsFor = (genre: Genre | null, fallback: [string, string] = ["Sunset", "Club"]) => MOODS[genre?.id ?? ""] ?? fallback;
const FALLBACK_PALETTES: Record<string, string[]> = {
  Club: ["#ff0040", "#7c00ff", "#0040ff", "#00e0ff"], Sunset: ["#ff2a00", "#ff7a00", "#ffc000", "#ff0060"],
  Ice: ["#ffffff", "#80d8ff", "#0080ff", "#4040ff"], Rainbow: ["#ff0000", "#ff8000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff"],
  Forest: ["#00ff40", "#80ff00", "#00c080", "#ffe000"], White: ["#ffffff"],
};

const pick = <T,>(s: Speed, slow: T, medium: T, fast: T) => (s === "slow" ? slow : s === "fast" ? fast : medium);

export interface BuildOptions { genre: Genre | null; moods?: [string, string]; title: string }

/** Turn scenes of blocks into a plan: one route per block, one scene preset per scene. */
export function buildScenes(show: Show, scenes: SceneSpec[], opts: BuildOptions): { plan: Plan; skipped: string[] } {
  const rig = analyseRig(show);
  const palettes: Palette[] = [];
  const [calmName, hotName] = opts.moods ?? moodsFor(opts.genre);
  const palette = (token = "calm"): string => {
    const nameOrId = token === "calm" ? calmName : token === "hot" ? hotName : token;
    const found = show.palettes.find((p) => p.id === nameOrId) ?? show.palettes.find((p) => p.name.toLowerCase() === nameOrId.toLowerCase());
    if (found) return found.id;
    const known = Object.keys(FALLBACK_PALETTES).find((n) => n.toLowerCase() === nameOrId.toLowerCase());
    if (!known) return show.palettes[0]?.id ?? palette("calm");
    const pid = `t-pal-${known.toLowerCase()}`;
    if (!palettes.some((p) => p.id === pid)) palettes.push({ id: pid, name: known, colors: FALLBACK_PALETTES[known] });
    return pid;
  };
  const bandIds = new Set(show.audio.bands.map((b) => b.id));
  /** A hit source for hit blocks and a level source for level blocks, whatever the token. */
  const sound = (token: string | undefined, kind: "hit" | "level") => {
    const t = token ?? (kind === "hit" ? "kick" : "energy");
    const band = t.endsWith(".level") ? t.slice(0, -6) : t;
    if (kind === "level") {
      if (["energy", "level", "phase"].includes(t)) return t;
      if (bandIds.has(band)) return `${band}.level`;
      return band === "vocals" ? rig.vocals : "energy";
    }
    if (t === "beat" || t === "bar") return t;
    if (bandIds.has(band)) return band;
    return band === "snare" ? rig.snare : band === "hat" ? rig.hat : rig.kick;
  };
  const tagFor = (t = "") => (t === "calm" ? rig.calmTag : t === "peak" ? rig.peakTag : t);
  const fastGenre = (opts.genre?.bpm[0] ?? 120) >= 150;
  const resolveTarget = (u: BlockUse, needs: Needs): Target | null => {
    if (u.target.startsWith("g:")) {
      const g = show.groups.find((x) => x.id === u.target.slice(2));
      return g ? { rooms: [], groups: [g.id], fixtures: [] } : null;
    }
    const t = needs === "laser" ? rig.laserTarget : needs === "fog" ? rig.fogTarget
      : u.target === "a" ? rig.a : u.target === "b" ? rig.b : rig.all;
    return t.groups.length || t.fixtures.length ? t : null;
  };
  const isHit = (src: string) => !["energy", "level", "phase"].includes(src) && !src.endsWith(".level");

  const routes: Route[] = [];
  const skipped: string[] = [];
  let n = 0;
  for (const scene of scenes) {
    const used = new Map<string, number>();
    for (const u of scene.blocks) {
      const d = blockDef(u.block);
      if (!d) continue;
      const target = resolveTarget(u, d.needs);
      if (!target || !available(rig, d.needs)) { skipped.push(`${scene.name}: ${d.name} (no ${d.needs === "glow" ? "colour fixtures" : d.needs === "laser" ? "lasers with saved looks" : "fog machine"} to drive)`); continue; }
      const s = u.speed;
      const focus = d.focus ? rig.focus : "";
      const src = d.sound ? sound(u.sound ?? d.sound.default, d.sound.kind) : d.needs === "fog" ? "bar" : "beat";
      let action: RouteAction;
      const extra: Partial<Route> = {};
      switch (d.id) {
        case "colour_hits":
          action = { type: "palette", palette: palette(u.palette), mode: focus ? "rings" : "spread", fade_ms: pick(s, 400, 150, 80), brightness: 255 };
          if (focus) { extra.focus = focus; extra.falloff = true; }
          break;
        case "colour_bar":
          action = { type: "palette", palette: palette(u.palette), mode: focus ? "around" : "spread", fade_ms: pick(s, 2500, 400, 100), brightness: 0 };
          if (focus) extra.focus = focus;
          break;
        case "wave":
          action = { type: "wave", palette: palette(u.palette), axis: focus ? "radial" : "x", beats: pick(s, 16, 8, 2), width: pick(s, 0.5, 0.35, 0.3), base: pick(s, 25, 10, 20) };
          if (focus) extra.focus = focus;
          break;
        case "sparkle":
          action = { type: "twinkle", palette: palette(u.palette), amount: pick(s, 0.15, 0.3, 0.5), decay_ms: pick(s, 800, 400, 250), base: 0 };
          break;
        case "flash":
          action = { type: "flash", on: 255, off: 30, decay_ms: pick(s, 400, 200, 100) };
          extra.channel = "@intensity";
          break;
        case "strobe": {
          const rates = ["1", "2", "4", "8"];
          const i = pick(s, 1, 2, 3) - (fastGenre ? 1 : 0);
          action = { type: "strobe", rate: rates[Math.max(0, i)], hz: 12, duty: 0.3, burst_ms: 200, threshold: 0.6 };
          break;
        }
        case "node_hop":
          action = { type: "node_hop", on: 255, off: 40, decay_ms: pick(s, 400, 250, 150), palette: palette(u.palette) };
          extra.channel = "@intensity";
          break;
        case "follow": {
          const energy = src === "energy";
          action = { type: "modulate", min: energy ? 40 : 10, max: energy ? 255 : 200, smoothing: pick(s, 0.9, 0.75, 0.5) };
          extra.channel = "@intensity";
          break;
        }
        case "laser": {
          const tag = tagFor(u.tag);
          action = { type: "look", set: "", include_defaults: true, tags: tag ? [tag] : [], mode: "random", hold_ms: pick(s, 8000, 3000, 800), heads: [] };
          if (isHit(src) && src !== "bar" && src !== "beat") extra.every = pick(s, 8, 4, 2);
          break;
        }
        case "fog":
          action = { type: "flash", on: 255, off: 0, decay_ms: 3000 };
          Object.assign(extra, { channel: "@fog", every: pick(s, 8, 4, 2), cooldown_ms: pick(s, 90000, 45000, 30000) });
          break;
        default:
          continue;
      }
      const where = u.target === "a" ? "half A" : u.target === "b" ? "half B"
        : u.target.startsWith("g:") ? show.groups.find((g) => g.id === u.target.slice(2))?.name ?? "" : "";
      const base = where ? `${d.name} · ${where}` : d.name;
      const count = (used.get(base) ?? 0) + 1;
      used.set(base, count);
      routes.push({
        id: `t-r${++n}`, name: count > 1 ? `${base} ${count}` : base, group: scene.name, enabled: false, source: src,
        ...target, channel: "", focus: "", falloff: false, action, cooldown_ms: 0, every: 1, probability: 1, ...extra,
      });
    }
  }

  const names = [...new Set(scenes.map((sc) => sc.name).filter((nm) => routes.some((r) => r.group === nm)))];
  const colors = ["#ff8a00", "#ffd27a", "#ff3355", "#4fd1ff", "#45d39a", "#7c5cff"];
  const presets: Preset[] = names.map((nm, i) => ({
    id: `t-p${i + 1}`, name: nm, color: colors[i % colors.length], values: [], exclusive: false,
    routes: Object.fromEntries(routes.map((r) => [r.id, r.group === nm])),
  }));
  for (const r of routes) r.enabled = r.group === names[0];

  const notes = [
    !rig.glow.length && "no colour or brightness fixtures were found, so only lasers and fog are used",
    !rig.lasers.length && "no fixtures have saved looks, so lasers aren't used (save looks in the pattern browser)",
    rig.lasers.length && routes.some((r) => r.action.type === "look") && !rig.peakTag && "tag laser looks \"peak\" and \"chill\" to pick looks per scene",
    !rig.focus && "add a focus point in Rooms for ripple and ring effects",
  ].filter(Boolean);
  return {
    skipped,
    plan: {
      title: opts.title,
      summary: `${names.length} scene${names.length === 1 ? "" : "s"}: ${names.join(", ")}. ${names[0] ?? ""} starts on; recall the scene presets to move between them.` +
        (notes.length ? `\nNote: ${notes.join("; ")}.` : ""),
      sensitivity: null, bands: [], palettes, routes, presets, controls: [],
    },
  };
}
