import { test } from "node:test";
import assert from "node:assert/strict";
import { aimAt, aimKind, beamOf, defaultBeam, footprint, lightSpot, rayHitsWall } from "../src/lib/aim.ts";
import type { Fixture, Profile, Room } from "../src/lib/types.ts";

const room: Room = { id: "r", name: "R", walls: [[0, 0], [8, 0], [8, 6], [0, 6]], ceiling_m: 3, focus: [] };
const fx = (x: number, y: number, z: number, rotation: number, tilt: number | null, beam: number | null = null) =>
  ({ id: "f", name: "f", x, y, z, rotation, tilt, beam_deg: beam }) as Fixture;
const near = (a: { x: number; y: number; z: number } | null, b: [number, number, number], tol = 0.01) =>
  !!a && Math.abs(a.x - b[0]) < tol && Math.abs(a.y - b[1]) < tol && Math.abs(a.z - b[2]) < tol;

test("spots match the engine's (same cases as model.rs)", () => {
  assert.equal(lightSpot(fx(2, 2, 2.5, 0, null), room), null);
  assert.ok(near(lightSpot(fx(2, 2, 2.5, 0, -90), room), [2, 2, 0]));
  assert.ok(near(lightSpot(fx(2, 2, 0.2, 0, 90), room), [2, 2, 3]));
  assert.ok(near(lightSpot(fx(2, 2, 2, 0, -45), room), [4, 2, 0]));
  assert.ok(near(lightSpot(fx(7.5, 3, 0.2, 0, 30), room), [8, 3, 0.2 + 0.5 * Math.tan(Math.PI / 6)]));
  assert.ok(near(lightSpot(fx(1, 3, 1.5, 180, 0), room), [0, 3, 1.5]));
  assert.equal(lightSpot(fx(2, 2, 2.5, 0, -90), room)!.on, "floor");
  assert.equal(lightSpot(fx(2, 2, 0.2, 0, 90), room)!.on, "ceiling");
  assert.equal(lightSpot(fx(7.5, 3, 0.2, 0, 30), room)!.on, "wall");
});

test("a downlight's footprint is a circle of the right size", () => {
  const pts = footprint(fx(4, 3, 3, 0, -90, 60), 60, room);
  assert.equal(pts.length, 32);
  const r = 3 * Math.tan(Math.PI / 6);
  for (const p of pts) {
    assert.ok(Math.abs(Math.hypot(p.x - 4, p.y - 3) - r) < 0.01, `${p.x},${p.y}`);
    assert.equal(p.on, "floor");
  }
});

test("an angled beam stretches away from the fixture and stops at walls", () => {
  const pts = footprint(fx(1, 3, 2.5, 0, -30, 40), 40, room);
  const xs = pts.map((p) => p.x);
  const spot = lightSpot(fx(1, 3, 2.5, 0, -30), room)!;
  assert.ok(Math.min(...xs) > 1 && Math.max(...xs) <= 8 + 1e-6, "lands ahead of the fixture, inside the room");
  assert.ok(Math.max(...xs) - spot.x > spot.x - Math.min(...xs), "the far edge is further from the centre than the near edge");
  // an uplight near the wall: part of the cone washes the wall
  const wash = footprint(fx(7.6, 3, 0.2, 0, 60, 50), 50, room);
  assert.ok(wash.some((p) => p.on === "wall") && wash.every((p) => p.x <= 8 + 1e-6));
});

test("aiming at a point puts the beam's centre on it", () => {
  for (const [f, target] of [[fx(1, 1, 2.8, 0, -90), { x: 4, y: 3, z: 0 }], [fx(7, 5, 2.5, 0, -90), { x: 2, y: 1, z: 0 }], [fx(4, 3, 0.1, 0, 0), { x: 4.5, y: 3, z: 2.9 }]] as const) {
    const a = aimAt(f, target)!;
    const s = lightSpot({ ...f, ...a }, room);
    assert.ok(near(s, [target.x, target.y, target.z], 0.15), JSON.stringify({ a, s }));
  }
  assert.equal(aimAt(fx(1, 1, 1, 0, null), { x: 1, y: 1, z: 1 }), null);
});

test("presets and beam angles", () => {
  assert.deepEqual([null, -90, 90, -30, 45].map((t) => aimKind(t)), ["none", "down", "up", "angled", "angled"]);
  const par = { kind: "par", beam_deg: 0 } as Profile;
  assert.equal(beamOf(fx(0, 0, 0, 0, -90), par), defaultBeam("par"));
  assert.equal(beamOf(fx(0, 0, 0, 0, -90), { ...par, beam_deg: 40 }), 40);
  assert.equal(beamOf(fx(0, 0, 0, 0, -90, 12), { ...par, beam_deg: 40 }), 12, "the fixture's own angle wins");
  assert.equal(rayHitsWall([4, 3], [1, 0], room.walls), 4);
  assert.equal(rayHitsWall([4, 3], [0, 0], room.walls), null);
});
