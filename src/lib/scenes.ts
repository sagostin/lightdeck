// Scenes are presets that switch routes. A scene says, per route, "on", "off" or
// nothing (leave it as it is). These helpers edit that by route group as well as by route.
import type { Fixture, Preset, Route, SceneLook, Show, Target } from "./types";
import { profileFor } from "./rig";
import { groupLooks } from "./looks";
import { buildScenes, type BlockUse } from "./blocks";
import type { Genre } from "./genres";

export type RouteState = "on" | "off" | "leave";
export type GroupState = RouteState | "mixed";

export const groupKey = (r: Route) => r.group.trim() || "Ungrouped";

/** Route groups in the order they first appear, with their routes. */
export function routeGroups(routes: Route[]): [string, Route[]][] {
  const m = new Map<string, Route[]>();
  for (const r of routes) m.set(groupKey(r), [...(m.get(groupKey(r)) ?? []), r]);
  return [...m.entries()];
}

export function routeState(p: Preset, id: string): RouteState {
  const v = p.routes[id];
  return v === undefined ? "leave" : v ? "on" : "off";
}

export function groupState(p: Preset, routes: Route[]): GroupState {
  const states = new Set(routes.map((r) => routeState(p, r.id)));
  return states.size === 1 ? [...states][0] : "mixed";
}

export function setRoutes(p: Preset, ids: string[], state: RouteState) {
  for (const id of ids) {
    if (state === "leave") delete p.routes[id];
    else p.routes[id] = state === "on";
  }
}

/** Record every route as it is right now (on or off). */
export function captureRoutes(p: Preset, routes: Route[]) {
  p.routes = Object.fromEntries(routes.map((r) => [r.id, r.enabled]));
}

/** These groups on, every other route off. */
export function soloGroups(p: Preset, routes: Route[], groups: string[]) {
  p.routes = Object.fromEntries(routes.map((r) => [r.id, groups.includes(groupKey(r))]));
}

/** How many routes a scene turns on and off. */
export function sceneSummary(p: Preset, routes: Route[]) {
  const ids = new Set(routes.map((r) => r.id));
  let on = 0, off = 0;
  for (const [id, v] of Object.entries(p.routes)) if (ids.has(id)) v ? on++ : off++;
  return { on, off };
}

const COLORS = ["#7c5cff", "#45d39a", "#f2b54a", "#ff5c6c", "#4fb3ff", "#ff7ad9"];

export function newScene(show: Show, id: string, name?: string): Preset {
  return {
    id, name: name?.trim() || `Scene ${show.presets.length + 1}`, color: COLORS[show.presets.length % COLORS.length],
    values: [], routes: {}, exclusive: false, sections: [],
  };
}

/** Drop links to routes that no longer exist. */
export function pruneScene(p: Preset, routes: Route[]) {
  const ids = new Set(routes.map((r) => r.id));
  for (const k of Object.keys(p.routes)) if (!ids.has(k)) delete p.routes[k];
}

// ---------- scenes built from blocks ----------
// A scene with `blocks` owns the routes built from them (route.scene = its id). Editing
// the blocks rebuilds just those routes; other scenes switch them off.

/** Scenes (switch routes or have blocks), as opposed to presets of hand-set channels only. */
export const isScene = (p: Preset) => !!p.blocks || Object.keys(p.routes).length > 0 || !!p.looks?.length || !!p.base?.length || !p.values.length;

/** Routes a scene built and owns. */
export const ownedRoutes = (show: Show, id: string) => show.routes.filter((r) => r.scene === id);

/** Rebuild a block scene's routes from its blocks. Returns what was skipped and why. */
export function rebuildScene(show: Show, id: string, genre: Genre | null = null): string[] {
  const p = show.presets.find((x) => x.id === id);
  if (!p?.blocks) return [];
  const old = ownedRoutes(show, id);
  const wasOn = old.some((r) => r.enabled);
  const oldIds = new Set(old.map((r) => r.id));
  const { plan, skipped } = buildScenes(show, [{ name: p.name, blocks: p.blocks, sections: p.sections }], { genre, title: p.name });
  for (const pal of plan.palettes) if (!show.palettes.some((x) => x.id === pal.id)) show.palettes.push(pal);
  // Ids stay the same from one rebuild to the next, so pads mapped to these routes keep working.
  const fresh = plan.routes.map((r, i) => ({ ...r, id: `ai-r-${id}-${i}`, scene: id, group: p.name, enabled: wasOn }));
  // Keep the routes where the old ones were, so run order stays put.
  const at = show.routes.findIndex((r) => oldIds.has(r.id));
  show.routes = show.routes.filter((r) => !oldIds.has(r.id));
  show.routes.splice(at < 0 ? show.routes.length : at, 0, ...fresh);
  for (const q of show.presets) {
    for (const k of oldIds) delete q.routes[k];
    if (q.id === id) for (const r of fresh) q.routes[r.id] = true;
    else if (isScene(q) && Object.keys(q.routes).length) for (const r of fresh) q.routes[r.id] = false;
  }
  return skipped;
}

/** A new empty block scene. */
export function addBlockScene(show: Show, id: string, name?: string): Preset {
  const p = { ...newScene(show, id, name), blocks: [] as BlockUse[] };
  show.presets.push(p);
  return p;
}

/** Delete a scene and the routes it built. */
export function deleteScene(show: Show, id: string) {
  const owned = new Set(ownedRoutes(show, id).map((r) => r.id));
  show.routes = show.routes.filter((r) => !owned.has(r.id));
  show.presets = show.presets.filter((p) => p.id !== id);
  for (const p of show.presets) for (const k of owned) delete p.routes[k];
  for (const c of show.controls) if (c.action.type === "preset" && c.action.preset === id) c.action.preset = "";
  for (const c of show.controls) if (c.action.type === "routes") c.action.routes = c.action.routes.filter((r) => !owned.has(r));
}

/** Rename a scene; its own routes' group follows. */
export function renameScene(show: Show, id: string, name: string) {
  const p = show.presets.find((x) => x.id === id);
  if (!p || !name.trim()) return;
  for (const r of ownedRoutes(show, id)) if (r.group === p.name) r.group = name.trim();
  p.name = name.trim();
}

/** One line about a scene for lists. */
export function sceneSummaryText(show: Show, p: Preset) {
  const { on } = sceneSummary(p, show.routes);
  const parts = [
    on && `${on} route${on === 1 ? "" : "s"} on`,
    p.looks?.length && `${p.looks.length} look${p.looks.length === 1 ? "" : "s"}`,
    p.base?.length && `${p.base.length} set value${p.base.length === 1 ? "" : "s"}`,
  ].filter(Boolean);
  return parts.join(" · ") || (p.values.length ? `${p.values.length} hand-set values` : "does nothing yet");
}

// ---------- the scene page, in rig order ----------

/** A line of the rig on the scene page: a fixture group, a fixture outside groups, or the whole rig. */
export interface RigRow {
  id: string; label: string; target: Target; fixtures: Fixture[];
  /** Routes aimed at this row (routes aimed at everything are on the whole-rig row). */
  routes: Route[];
  /** Saved looks the row's fixtures have, by name, with how many of them have it. */
  /** In pattern order: `group` is the pattern value the look is built on ("P1 Pattern 48 · Double line"). */
  /** `value` is what a scene stores: the name, or the look's id when the name repeats on another pattern. */
  looks: { name: string; value: string; count: number; tags: string[]; group: string }[];
}

const T = (t: Partial<Target>): Target => ({ rooms: [], groups: [], fixtures: [], ...t });
export const sameTarget = (a: Target, b: Target) =>
  [a.rooms, a.groups, a.fixtures].map((x) => [...x].sort().join(",")).join("|") === [b.rooms, b.groups, b.fixtures].map((x) => [...x].sort().join(",")).join("|");

export function rigRows(show: Show): RigRow[] {
  const looksOf = (fx: Fixture[]): RigRow["looks"] => {
    // in pattern order, grouped under the pattern value each look is built on
    const items = fx.flatMap((f) => { const p = profileFor(show, f); return p ? p.looks.map((look) => ({ profile: p, look })) : []; });
    const out: (RigRow["looks"][number] & { ids: string[] })[] = [];
    for (const g of groupLooks(items)) {
      const group = `${g.channel} ${g.value}${g.label ? ` · ${g.label}` : ""}`;
      for (const l of g.looks) {
        // one entry per name within a pattern; the same look on several fixture types counts each
        const e = out.find((x) => x.group === group && x.name === l.name);
        if (e) { e.ids.push(l.id); l.tags.forEach((t) => e.tags.includes(t) || e.tags.push(t)); continue; }
        out.push({ name: l.name, value: l.name, count: 0, tags: [...l.tags], group, ids: [l.id] });
      }
    }
    for (const e of out) {
      e.count = fx.filter((f) => profileFor(show, f)?.looks.some((l) => e.ids.includes(l.id))).length;
      // a name used on more than one pattern is picked by id, so choosing one doesn't choose the other
      if (out.filter((x) => x.name === e.name).length > 1) e.value = e.ids[0];
    }
    return out.map(({ ids: _, ...e }) => e);
  };
  const everything = (r: Route) => !r.groups.length && !r.fixtures.length;
  const rows: RigRow[] = [{ id: "all", label: "Whole rig", target: T({}), fixtures: show.fixtures, routes: show.routes.filter(everything), looks: [] }];
  const grouped = new Set<string>();
  for (const g of show.groups) {
    const fx = show.fixtures.filter((f) => g.fixtures.includes(f.id));
    fx.forEach((f) => grouped.add(f.id));
    rows.push({
      id: `g:${g.id}`, label: g.name, target: T({ groups: [g.id] }), fixtures: fx,
      routes: show.routes.filter((r) => r.groups.includes(g.id) || r.fixtures.some((id) => g.fixtures.includes(id))),
      looks: looksOf(fx),
    });
  }
  for (const f of show.fixtures.filter((x) => !grouped.has(x.id))) {
    rows.push({ id: `f:${f.id}`, label: f.name, target: T({ fixtures: [f.id] }), fixtures: [f], routes: show.routes.filter((r) => r.fixtures.includes(f.id)), looks: looksOf([f]) });
  }
  return rows;
}

/** The look a scene puts a row in ("" = none). */
export const rowLook = (p: Preset, row: RigRow) => p.looks?.find((l) => sameTarget(l.target, row.target))?.look ?? "";

/** Put a row in a look for this scene ("" removes it). */
export function setRowLook(p: Preset, row: RigRow, look: string) {
  const rest = (p.looks ?? []).filter((l) => !sameTarget(l.target, row.target));
  p.looks = look ? [...rest, { target: structuredClone(row.target), look, heads: [] } satisfies SceneLook] : rest;
}

/**
 * Capture the rig as it is now into a scene: which routes are on, and the values set by hand
 * (they become the scene's own values, underneath its routes). A block scene's own routes stay on.
 */
export function captureScene(show: Show, p: Preset, manual: Record<string, Record<string, number>>) {
  p.routes = Object.fromEntries(show.routes.map((r) => [r.id, r.scene === p.id ? true : r.enabled]));
  const base = new Map((p.base ?? []).map((v) => [`${v.fixture}|${v.channel}`, v]));
  for (const [fixture, chans] of Object.entries(manual)) for (const [channel, value] of Object.entries(chans)) base.set(`${fixture}|${channel}`, { fixture, channel, value });
  p.base = [...base.values()];
  return { routes: Object.values(p.routes).filter(Boolean).length, values: Object.values(manual).reduce((n, c) => n + Object.keys(c).length, 0) };
}
