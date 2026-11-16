// Where fixtures point and what their light covers. Mirrors `light_spot` in model.rs
// (the engine's copy), so the map shows the same spots effects are calculated from.
import type { Fixture, Profile, Room } from "./types";

export const DEFAULT_CEILING_M = 3.0;
const MAX_THROW_M = 30;

/** Default beam angle for a kind of fixture, degrees. */
export function defaultBeam(kind: string | undefined): number {
  switch (kind) {
    case "par": return 25;
    case "moving_head": return 15;
    case "strobe": return 60;
    case "zigbee": return 120;
    default: return 30;
  }
}
export function beamOf(fx: Fixture, p: Profile | undefined): number {
  return fx.beam_deg ?? (p?.beam_deg ? p.beam_deg : defaultBeam(p?.kind));
}

/** Aim presets. */
export const AIMS = [
  { id: "none", label: "Not set", tilt: null },
  { id: "down", label: "Down", tilt: -90 },
  { id: "up", label: "Up", tilt: 90 },
  { id: "angled", label: "Angled", tilt: -45 },
] as const;
export function aimKind(tilt: number | null | undefined): (typeof AIMS)[number]["id"] {
  if (tilt === null || tilt === undefined) return "none";
  if (tilt <= -89.99) return "down";
  if (tilt >= 89.99) return "up";
  return "angled";
}

/** Smallest s > 0 where the 2D ray p + s·d crosses an edge of the polygon. */
export function rayHitsWall(p: [number, number], d: [number, number], walls: [number, number][]): number | null {
  let best: number | null = null;
  for (let i = 0; i < walls.length; i++) {
    const a = walls[i], b = walls[(i + 1) % walls.length];
    const e = [b[0] - a[0], b[1] - a[1]];
    const den = d[0] * e[1] - d[1] * e[0];
    if (Math.abs(den) < 1e-6) continue;
    const w = [a[0] - p[0], a[1] - p[1]];
    const s = (w[0] * e[1] - w[1] * e[0]) / den;
    const u = (w[0] * d[1] - w[1] * d[0]) / den;
    if (s > 1e-4 && u >= -1e-4 && u <= 1 + 1e-4) best = best === null ? s : Math.min(best, s);
  }
  return best;
}

type Hit = { x: number; y: number; z: number; on: "floor" | "ceiling" | "wall" };

/** Follow a ray from the fixture until it reaches the floor, the ceiling or a wall. */
function cast(fx: Pick<Fixture, "x" | "y" | "z">, d: [number, number, number], room: Room | null): Hit {
  const ceiling = room && room.ceiling_m > 0 ? room.ceiling_m : DEFAULT_CEILING_M;
  let t = d[2] < -1e-4 ? Math.max(0, fx.z) / -d[2] : d[2] > 1e-4 ? Math.max(0, ceiling - fx.z) / d[2] : MAX_THROW_M;
  let on: Hit["on"] = d[2] < -1e-4 ? "floor" : d[2] > 1e-4 ? "ceiling" : "wall";
  if (room && room.walls.length >= 3) {
    const s = rayHitsWall([fx.x, fx.y], [d[0], d[1]], room.walls);
    if (s !== null && s < t) { t = s; on = "wall"; }
  }
  return { x: fx.x + t * d[0], y: fx.y + t * d[1], z: Math.min(ceiling, Math.max(0, fx.z + t * d[2])), on };
}

function direction(rotationDeg: number, tiltDeg: number): [number, number, number] {
  const t = Math.max(-90, Math.min(90, tiltDeg));
  if (Math.abs(t) >= 89.99) return [0, 0, Math.sign(t)];
  const r = (rotationDeg * Math.PI) / 180, a = (t * Math.PI) / 180;
  return [Math.cos(a) * Math.cos(r), Math.cos(a) * Math.sin(r), Math.sin(a)];
}

/** Where the beam's centre lands, or null if the fixture isn't aimed. */
export function lightSpot(fx: Fixture, room: Room | null): Hit | null {
  if (fx.tilt === null || fx.tilt === undefined) return null;
  return cast(fx, direction(fx.rotation, fx.tilt), room);
}

/**
 * The area the beam lights, as a plan-view outline: the cone's edge traced to the floor,
 * ceiling or walls it reaches. Points that end on a wall are where it's cut off.
 */
export function footprint(fx: Fixture, beamDeg: number, room: Room | null, steps = 32): Hit[] {
  if (fx.tilt === null || fx.tilt === undefined) return [];
  const axis = direction(fx.rotation, fx.tilt);
  // Two unit vectors perpendicular to the beam axis.
  const ref = Math.abs(axis[2]) > 0.9 ? [1, 0, 0] : [0, 0, 1];
  const cross = (a: number[], b: number[]) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const norm = (v: number[]) => { const l = Math.hypot(v[0], v[1], v[2]) || 1; return v.map((x) => x / l); };
  const u = norm(cross(axis, ref)), v = norm(cross(axis, u));
  const half = (Math.max(1, Math.min(170, beamDeg)) / 2) * (Math.PI / 180);
  const pts: Hit[] = [];
  for (let i = 0; i < steps; i++) {
    const a = (i / steps) * Math.PI * 2;
    const d = [0, 1, 2].map((k) => Math.cos(half) * axis[k] + Math.sin(half) * (Math.cos(a) * u[k] + Math.sin(a) * v[k])) as [number, number, number];
    pts.push(cast(fx, d, room));
  }
  return pts;
}

/** Facing and tilt that put the beam's centre on a point (e.g. a focus point at its height). */
export function aimAt(fx: Pick<Fixture, "x" | "y" | "z">, p: { x: number; y: number; z: number }): { rotation: number; tilt: number } | null {
  const dx = p.x - fx.x, dy = p.y - fx.y, dz = p.z - fx.z;
  const flat = Math.hypot(dx, dy);
  if (flat < 0.05 && Math.abs(dz) < 0.05) return null;
  const rotation = flat < 0.05 ? 0 : Math.round(((Math.atan2(dy, dx) * 180) / Math.PI + 360) % 360);
  const tilt = Math.round((Math.atan2(dz, flat) * 180) / Math.PI);
  return { rotation, tilt };
}
