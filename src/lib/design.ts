// AI show designs: the plan the backend returns (already checked against the rig),
// and applying the parts the user keeps to the show.
import { describeMotion, type BandDef, type ControlMapping, type Palette, type Preset, type Route, type Show } from "./types";
import { clone as deepClone } from "./clone";

export interface Plan {
  title: string;
  summary: string;
  sensitivity: number | null;
  bands: BandDef[];
  palettes: Palette[];
  routes: Route[];
  presets: Preset[];
  controls: ControlMapping[];
}
export interface DesignResult { plan: Plan; warnings: string[] }
export interface DesignRequest {
  prompt: string;
  genre: { name: string; bpm: [number, number]; style: string } | null;
  scope: { routes: boolean; presets: boolean; audio: boolean; palettes: boolean; controls: boolean };
  controller: string;
  midi_channel: number;
  keyboard: boolean;
  previous: Plan | null;
}

/** Everything AI designs create has an id starting with this. */
export const AI_PREFIX = "ai-";
export const isAi = (id: string) => id.startsWith(AI_PREFIX);

/** Count of earlier AI-made items in the show. */
export function aiCounts(show: Show) {
  return {
    routes: show.routes.filter((r) => isAi(r.id)).length,
    presets: show.presets.filter((p) => isAi(p.id)).length,
    controls: show.controls.filter((c) => isAi(c.id)).length,
  };
}

/**
 * Apply a plan. `keep` holds the ids of routes/presets/controls/palettes to add and band
 * ids to merge ("band:<id>"); `audio` also sets global sensitivity. With `replace`, earlier
 * AI-made routes, presets and controls are removed first.
 */
export function applyPlan(show: Show, plan: Plan, keep: Set<string>, replace: boolean): string {
  if (replace) {
    show.routes = show.routes.filter((r) => !isAi(r.id));
    show.presets = show.presets.filter((p) => !isAi(p.id));
    show.controls = show.controls.filter((c) => !isAi(c.id));
    // Drop AI palettes nothing uses any more.
    const used = new Set(show.routes.map((r) => ("palette" in r.action ? r.action.palette : "")));
    show.palettes = show.palettes.filter((p) => !isAi(p.id) || used.has(p.id));
    // Presets that referenced removed routes keep working; drop the dangling ids.
    const ids = new Set(show.routes.map((r) => r.id));
    for (const p of show.presets) for (const k of Object.keys(p.routes)) if (!ids.has(k)) delete p.routes[k];
  }
  const clone = deepClone;
  const routes = plan.routes.filter((r) => keep.has(r.id));
  // Palettes: chosen ones, plus any a kept route needs.
  const needed = new Set(routes.map((r) => ("palette" in r.action ? r.action.palette : "")));
  for (const p of plan.palettes) if ((keep.has(p.id) || needed.has(p.id)) && !show.palettes.some((x) => x.id === p.id)) show.palettes.push(clone(p));
  let bands = 0;
  for (const b of plan.bands) {
    if (!keep.has(`band:${b.id}`)) continue;
    const cur = show.audio.bands.find((x) => x.id === b.id);
    if (cur) Object.assign(cur, clone(b));
    else show.audio.bands.push(clone(b));
    bands++;
  }
  if (plan.sensitivity !== null && keep.has("sensitivity")) show.audio.sensitivity = plan.sensitivity;
  show.routes.push(...routes.map(clone));
  const routeIds = new Set(show.routes.map((r) => r.id));
  const presets = plan.presets.filter((p) => keep.has(p.id)).map((p) => {
    const c = clone(p);
    for (const k of Object.keys(c.routes)) if (!routeIds.has(k)) delete c.routes[k];
    return c;
  }).filter((p) => p.values.length || Object.keys(p.routes).length || p.looks?.length || p.base?.length);
  show.presets.push(...presets);
  const presetIds = new Set(show.presets.map((p) => p.id));
  const groups = new Set(show.routes.map((r) => r.group));
  const controls = plan.controls.filter((c) => keep.has(c.id)).map(clone).filter((c) => {
    const a = c.action;
    if (a.type === "routes") { a.routes = a.routes.filter((r) => routeIds.has(r)); return a.routes.length > 0; }
    if (a.type === "preset") return presetIds.has(a.preset);
    if (a.type === "route_group") return groups.has(a.group);
    return true;
  });
  show.controls.push(...controls);
  if (controls.some((c) => c.trigger.source === "midi")) show.midi.enabled = true;
  const n = (k: number, one: string, many: string) => k && `${k} ${k === 1 ? one : many}`;
  const parts = [
    n(routes.length, "route", "routes"), n(presets.length, "scene", "scenes"),
    n(controls.length, "control", "controls"), n(bands, "band", "bands"),
  ].filter(Boolean);
  return parts.length ? `Added ${parts.join(", ")}` : "Nothing selected";
}

/** One-line description of a route, for previews. */
export function describeRoute(r: Route, show: Show, plan: Plan): string {
  const a = r.action;
  const pal = (id: string) => [...show.palettes, ...plan.palettes].find((p) => p.id === id)?.name ?? "palette";
  const what = (() => {
    switch (a.type) {
      case "modulate": return `brightness ${a.min}–${a.max}`;
      case "flash": return `flash ${a.on}→${a.off} over ${a.decay_ms} ms`;
      case "random": return `random ${a.tags.join(", ") || "value"}`;
      case "node_hop": return `node hop${a.palette ? ` in ${pal(a.palette)}` : ""}`;
      case "strobe": return `strobe ${a.rate === "free" ? `${a.hz} Hz` : `${a.rate}/beat`}`;
      case "palette": return `${pal(a.palette)} colours (${a.mode})`;
      case "twinkle": return `twinkle in ${pal(a.palette)}`;
      case "wave": return `${pal(a.palette)} wave (${a.axis}, ${a.beats} beats)`;
      case "cycle": return `cycle ${a.values.length} values`;
      case "motion": return `${describeMotion(a.motion)}${a.spread ? `, chasing ${Math.round(a.spread * 100)}%` : ""}`;
      case "look": return `looks ${a.looks?.length ? [...new Set(a.looks.map((x) => show.profiles.flatMap((p) => p.looks).find((l) => l.id === x)?.name ?? x))].join(", ") : a.tags.length ? `tagged ${a.tags.join(", ")}` : "(any)"}${a.follow_section ? ", following the section" : ""}${a.max_ms ? `, change within ${Math.round(a.max_ms / 1000)} s` : ""}${a.rest ? `, rests ${Math.round(a.rest * 100)}%` : ""}`;
    }
  })();
  const names = (ids: string[], list: { id: string; name: string }[]) => ids.map((i) => list.find((x) => x.id === i)?.name ?? i);
  const targets = [...names(r.groups, show.groups), ...names(r.fixtures, show.fixtures), ...names(r.rooms, show.rooms)];
  const ch = r.channel ? ` · ${r.channel.startsWith("@") ? r.channel.slice(1) : r.channel}` : "";
  const also = Object.keys(r.motions ?? {}).length ? ` · also moves ${Object.keys(r.motions!).map((k) => k.replace(/^@/, "")).join(", ")}` : "";
  const sw = (r.switches ?? []).map((x) => `${x.to === "toggle" ? "toggles" : `switches ${x.to}`} ${[...show.routes, ...plan.routes].find((o) => o.id === x.route)?.name || x.route}${x.beats ? ` for ${x.beats} beats` : ""}`);
  const swText = sw.length ? ` · ${sw.join(", ")}` : "";
  return `${r.source} → ${what} · ${targets.length ? targets.join(", ") : "all fixtures"}${ch}${also}${swText}`;
}

export function describeTrigger(c: ControlMapping): string {
  const t = c.trigger;
  if (t.source === "key") return `Key ${t.key === " " ? "Space" : t.key.toUpperCase()}`;
  const ch = t.channel ? ` ch${t.channel}` : "";
  return t.kind === "cc" ? `CC ${t.number}${ch}` : t.kind === "program" ? `Program ${t.number}${ch}` : `Note ${t.number}${ch}`;
}
