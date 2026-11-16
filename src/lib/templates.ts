// Setup templates: ready-made scene layouts built from the rig with fixed rules, no AI.
// They target your fixture groups, focus points and saved looks, and come back as a
// plan that goes through the same checks and review as an AI design.
import type { Plan } from "./design";
import type { Genre } from "./genres";
import type { Fixture, Palette, Preset, Route, RouteAction, Show } from "./types";
import { activeFixtures, canGlow, hasLooks, hasType, lookTags, profileFor } from "./rig";

export interface Template { id: string; name: string; description: string }

export const TEMPLATES: Template[] = [
  { id: "club", name: "Club night", description: "Warm-up, Build, Drop and Breakdown scenes: slow waves, energy builds, kick-driven colour with strobe and fog on the drop." },
  { id: "party", name: "Party", description: "Groove and Peak scenes: colourful palette steps, sparkles and node hops, strobe and fog at the peak." },
  { id: "chill", name: "Chill / lounge", description: "Glow and Drift scenes: long colour fades and slow waves. No strobe, no fog." },
  { id: "test", name: "Test my rig", description: "One scene where every group flashes on the kick and changes colour each bar, and lasers change look. For checking patching and detection." },
];

// Palette moods per genre: [calm, hot].
const MOODS: Record<string, [string, string]> = {
  house: ["Sunset", "Club"], techno: ["Ice", "White"], trance: ["Ice", "Rainbow"], dnb: ["Ice", "Club"], dubstep: ["Club", "White"],
  hiphop: ["Sunset", "Club"], latin: ["Sunset", "Rainbow"], pop: ["Rainbow", "Club"], rock: ["Sunset", "White"], ambient: ["Forest", "Ice"],
};
const FALLBACK_PALETTES: Record<string, string[]> = {
  Club: ["#ff0040", "#7c00ff", "#0040ff", "#00e0ff"], Sunset: ["#ff2a00", "#ff7a00", "#ffc000", "#ff0060"],
  Ice: ["#ffffff", "#80d8ff", "#0080ff", "#4040ff"], Rainbow: ["#ff0000", "#ff8000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff"],
  Forest: ["#00ff40", "#80ff00", "#00c080", "#ffe000"], White: ["#ffffff"],
};

type Target = Pick<Route, "rooms" | "groups" | "fixtures">;

/** What the rig offers, in template terms. */
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
    glow, lasers, fog, all, a, b, focus,
    laserTarget: target(lasers.map((f) => f.id)), fogTarget: target(fog.map((f) => f.id)),
    kick: band("kick") || show.audio.bands[0]?.id || "beat",
    snare: band("snare", "clap") || "beat",
    hat: band("hat", "hihat", "cymbals") || "beat",
    vocals: band("vocals") ? "vocals.level" : "level",
    calmTag: ["chill", "static", "calm", "slow"].find((t) => tags.includes(t)) ?? "",
    peakTag: ["peak", "dynamic", "drop"].find((t) => tags.includes(t)) ?? "",
  };
}

export function buildTemplate(show: Show, id: string, genre: Genre | null): Plan {
  const rig = analyseRig(show);
  const palettes: Palette[] = [];
  const pal = (name: string) => {
    const found = show.palettes.find((p) => p.name.toLowerCase() === name.toLowerCase());
    if (found) return found.id;
    const pid = `t-pal-${name.toLowerCase()}`;
    if (!palettes.some((p) => p.id === pid)) palettes.push({ id: pid, name, colors: FALLBACK_PALETTES[name] ?? ["#ffffff"] });
    return pid;
  };
  const [calmName, hotName] = MOODS[genre?.id ?? ""] ?? (id === "chill" ? ["Sunset", "Forest"] : ["Sunset", "Club"]);
  const calm = pal(calmName), hot = pal(hotName);
  const fast = (genre?.bpm[0] ?? 120) >= 150;
  const noStrobe = genre?.id === "ambient";

  const routes: Route[] = [];
  let n = 0;
  const add = (scene: string, name: string, source: string, target: Target, action: RouteAction, extra: Partial<Route> = {}) => {
    routes.push({ id: `t-r${++n}`, name, group: scene, enabled: false, source, ...target, channel: "", focus: "", falloff: false,
      action, cooldown_ms: 0, every: 1, probability: 1, ...extra });
  };
  const hasGlow = rig.glow.length > 0;
  const laser = (scene: string, name: string, source: string, tag: string, hold: number, every = 1) => {
    if (rig.lasers.length) add(scene, name, source, rig.laserTarget, { type: "look", set: "", include_defaults: true, tags: tag ? [tag] : [], mode: "random", hold_ms: hold, heads: [] }, { every });
  };
  const fog = (scene: string) => {
    if (rig.fog.length) add(scene, "Fog burst", "bar", rig.fogTarget, { type: "flash", on: 255, off: 0, decay_ms: 3000 }, { channel: "@fog", cooldown_ms: 45000, every: 4 });
  };
  const focus = rig.focus;
  const wave = (scene: string, name: string, palette: string, beats: number, width: number, base: number) =>
    hasGlow && add(scene, name, "beat", rig.all, { type: "wave", palette, axis: focus ? "radial" : "x", beats, width, base }, { focus });

  switch (id) {
    case "club":
      wave("Warm-up", "Slow wave", calm, 16, 0.5, 25);
      if (hasGlow) add("Warm-up", "Hi-hat sparkle", rig.hat, rig.b, { type: "twinkle", palette: hot, amount: 0.3, decay_ms: 400, base: 0 });
      laser("Warm-up", "Calm laser looks", "bar", rig.calmTag, 6000);
      if (hasGlow) add("Build", "Colour each bar", "bar", rig.all, { type: "palette", palette: hot, mode: "spread", fade_ms: 400, brightness: 0 });
      if (hasGlow) add("Build", "Energy lifts brightness", "energy", rig.all, { type: "modulate", min: 40, max: 255, smoothing: 0.6 }, { channel: "@intensity" });
      laser("Build", "Laser changes every 4 kicks", rig.kick, "", 1500, 4);
      if (hasGlow) add("Drop", "Kick colour hits", rig.kick, rig.all, { type: "palette", palette: hot, mode: focus ? "rings" : "random", fade_ms: 120, brightness: 255 }, { focus, falloff: !!focus });
      if (hasGlow && !noStrobe) add("Drop", "Snare strobe", rig.snare, rig.a, { type: "strobe", rate: fast ? "2" : "4", hz: 12, duty: 0.3, burst_ms: 200, threshold: 0.6 });
      laser("Drop", "Peak laser looks", rig.kick, rig.peakTag, 800, 2);
      fog("Drop");
      if (hasGlow) add("Breakdown", "Lights breathe with vocals", rig.vocals, rig.all, { type: "modulate", min: 10, max: 180, smoothing: 0.85 }, { channel: "@intensity" });
      wave("Breakdown", "Drifting colour", calm, 8, 0.35, 0);
      laser("Breakdown", "Calm laser looks", "bar", rig.calmTag, 8000);
      break;
    case "party":
      if (hasGlow) add("Groove", "Kick colour steps", rig.kick, rig.all, { type: "palette", palette: hot, mode: "spread", fade_ms: 150, brightness: 255 });
      if (hasGlow) add("Groove", "Hi-hat sparkle", rig.hat, rig.b, { type: "twinkle", palette: pal("Rainbow"), amount: 0.4, decay_ms: 300, base: 0 });
      laser("Groove", "Laser looks each bar", "bar", "", 3000);
      wave("Peak", "Fast rings", pal("Rainbow"), fast ? 2 : 1, 0.3, 20);
      if (hasGlow) add("Peak", "Node hop on the kick", rig.kick, rig.a, { type: "node_hop", on: 255, off: 40, decay_ms: 250, palette: hot }, { channel: "@intensity" });
      if (hasGlow && !noStrobe) add("Peak", "Snare strobe", rig.snare, rig.all, { type: "strobe", rate: "2", hz: 12, duty: 0.3, burst_ms: 200, threshold: 0.6 });
      laser("Peak", "Peak laser looks", rig.kick, rig.peakTag, 800, 2);
      fog("Peak");
      break;
    case "chill":
      wave("Glow", "Very slow wave", calm, 32, 0.6, 30);
      if (hasGlow) add("Glow", "Gentle sparkle", rig.hat, rig.b, { type: "twinkle", palette: hot, amount: 0.15, decay_ms: 800, base: 0 });
      laser("Glow", "Calm laser looks", "bar", rig.calmTag, 12000);
      if (hasGlow) add("Drift", "Colour fades each bar", "bar", rig.all, { type: "palette", palette: calm, mode: focus ? "around" : "spread", fade_ms: 2500, brightness: 0 }, { focus });
      if (hasGlow) add("Drift", "Brightness follows the music", "level", rig.all, { type: "modulate", min: 40, max: 200, smoothing: 0.9 }, { channel: "@intensity" });
      laser("Drift", "Calm laser looks", "bar", rig.calmTag, 12000, 2);
      break;
    case "test":
      if (hasGlow) {
        const targets: Target[] = rig.a !== rig.all ? [rig.a, rig.b] : [rig.all];
        targets.forEach((t, i) => add("Test", targets.length > 1 ? `Kick flash (half ${i + 1})` : "Kick flash", rig.kick, t, { type: "flash", on: 255, off: 30, decay_ms: 200 }, { channel: "@intensity" }));
        add("Test", "Colour each bar", "bar", rig.all, { type: "palette", palette: hot, mode: "all", fade_ms: 0, brightness: 0 });
      }
      laser("Test", "Laser look every 2 kicks", rig.kick, "", 0, 2);
      break;
  }

  const scenes = [...new Set(routes.map((r) => r.group))];
  const colors = ["#ff8a00", "#ffd27a", "#ff3355", "#4fd1ff"];
  const presets: Preset[] = scenes.map((s, i) => ({
    id: `t-p${i + 1}`, name: s, color: colors[i % colors.length], values: [], exclusive: false,
    routes: Object.fromEntries(routes.map((r) => [r.id, r.group === s])),
  }));
  for (const r of routes) r.enabled = r.group === scenes[0];

  const t = TEMPLATES.find((x) => x.id === id)!;
  const missing = [
    !rig.glow.length && "no colour or brightness fixtures were found, so only lasers and fog are used",
    !rig.lasers.length && "no fixtures have saved looks, so lasers aren't used (save looks in the pattern browser)",
    rig.lasers.length && !rig.peakTag && id !== "chill" && "tag laser looks \"peak\" and \"chill\" to pick looks per scene",
    !rig.focus && "add a focus point in Rooms for ripple and ring effects",
  ].filter(Boolean);
  return {
    title: `${t.name}${genre ? ` · ${genre.name}` : ""}`,
    summary: `${scenes.length} scene${scenes.length === 1 ? "" : "s"}: ${scenes.join(", ")}. ${scenes[0] ?? ""} starts on; recall the scene presets to move between them.` +
      (missing.length ? `\nNote: ${missing.join("; ")}.` : ""),
    sensitivity: null, bands: [], palettes, routes, presets, controls: [],
  };
}
