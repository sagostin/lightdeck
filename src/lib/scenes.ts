// Scenes are presets that switch routes. A scene says, per route, "on", "off" or
// nothing (leave it as it is). These helpers edit that by route group as well as by route.
import type { Preset, Route, Show } from "./types";

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
    values: [], routes: {}, exclusive: false,
  };
}

/** Drop links to routes that no longer exist. */
export function pruneScene(p: Preset, routes: Route[]) {
  const ids = new Set(routes.map((r) => r.id));
  for (const k of Object.keys(p.routes)) if (!ids.has(k)) delete p.routes[k];
}
