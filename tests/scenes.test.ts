import { test } from "node:test";
import assert from "node:assert/strict";
import { captureRoutes, groupState, newScene, pruneScene, routeGroups, routeState, sceneSummary, setRoutes, soloGroups } from "../src/lib/scenes.ts";
import { route, show } from "./fixtures.ts";

function rig() {
  const s = show();
  s.routes = [route("a1", "A1", "Drop", { enabled: true }), route("a2", "A2", "Drop"), route("b1", "B1", "Chill"), route("u1", "U1", "  ")];
  return s;
}

test("routes are grouped in order, blank groups as Ungrouped", () => {
  assert.deepEqual(routeGroups(rig().routes).map(([g, rs]) => [g, rs.map((r) => r.id)]), [["Drop", ["a1", "a2"]], ["Chill", ["b1"]], ["Ungrouped", ["u1"]]]);
});

test("a group is on, off, left alone or mixed", () => {
  const s = rig(), p = newScene(s, "p");
  const drop = s.routes.filter((r) => r.group === "Drop");
  assert.equal(groupState(p, drop), "leave");
  setRoutes(p, ["a1", "a2"], "on");
  assert.equal(groupState(p, drop), "on");
  setRoutes(p, ["a2"], "off");
  assert.equal(groupState(p, drop), "mixed");
  assert.equal(routeState(p, "a2"), "off");
  setRoutes(p, ["a1", "a2"], "leave");
  assert.deepEqual(p.routes, {});
});

test("solo turns one group on and everything else off", () => {
  const s = rig(), p = newScene(s, "p");
  soloGroups(p, s.routes, ["Chill"]);
  assert.deepEqual(p.routes, { a1: false, a2: false, b1: true, u1: false });
  assert.deepEqual(sceneSummary(p, s.routes), { on: 1, off: 3 });
});

test("capture records the live state; prune drops deleted routes", () => {
  const s = rig(), p = newScene(s, "p");
  captureRoutes(p, s.routes);
  assert.deepEqual(p.routes, { a1: true, a2: false, b1: false, u1: false });
  s.routes = s.routes.filter((r) => r.id !== "a2");
  assert.deepEqual(sceneSummary(p, s.routes), { on: 1, off: 2 }, "summary ignores routes that are gone");
  pruneScene(p, s.routes);
  assert.ok(!("a2" in p.routes));
});

test("new scenes are named and coloured", () => {
  const s = rig();
  assert.equal(newScene(s, "x").name, "Scene 2", "the show already has one preset");
  assert.equal(newScene(s, "x", "  Drop ").name, "Drop");
  assert.match(newScene(s, "x").color, /^#[0-9a-f]{6}$/);
});
