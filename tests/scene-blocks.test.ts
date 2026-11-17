import { test } from "node:test";
import assert from "node:assert/strict";
import { blockParts, buildScenes, newBlock, type BlockUse } from "../src/lib/blocks.ts";
import { buildTemplate } from "../src/lib/templates.ts";
import { addBlockScene, deleteScene, isScene, ownedRoutes, rebuildScene, renameScene } from "../src/lib/scenes.ts";
import { applyPlan } from "../src/lib/design.ts";
import type { Show } from "../src/lib/types.ts";
import { clubRig, planProblems } from "./rig.ts";
import { show as small } from "./fixtures.ts";

const strobe = (more: Partial<BlockUse> = {}): BlockUse => ({ ...newBlock("strobe"), ...more });

test("a block becomes one route, or one per section it differs in", () => {
  assert.deepEqual(blockParts(strobe()).map((p) => p.sections), [[]], "plays always: one route, no sections");
  assert.deepEqual(blockParts(strobe({ sections: ["drop"] })).map((p) => p.sections), [["drop"]]);
  const parts = blockParts(strobe({ sections: ["build", "drop"], variants: { drop: { speed: "fast" }, groove: { speed: "slow" } } }));
  assert.deepEqual(parts.map((p) => [p.sections, p.use.speed, p.label]), [[["build"], "medium", ""], [["drop"], "fast", "drop"]],
    "a variant for a section it doesn't play in is ignored");
  const all = blockParts(strobe({ variants: { breakdown: { speed: "slow" }, groove: { speed: "slow" }, build: { speed: "fast" }, drop: { speed: "fast" } } }));
  assert.equal(all.length, 4, "no base route when every section differs");
  assert.deepEqual(blockParts(strobe({ variants: { drop: {} } })).length, 1, "an empty variant changes nothing");
});

test("built routes carry their sections, scene and settings per section", () => {
  const rig = clubRig({ looksTagged: true });
  const { plan } = buildScenes(rig, [{
    name: "Main", blocks: [
      { ...newBlock("colour_hits") },
      strobe({ sections: ["drop"] }),
      { ...newBlock("laser"), tag: "calm", variants: { drop: { tag: "peak", speed: "fast" } } },
    ],
  }], { genre: null, title: "t" });
  assert.deepEqual(planProblems(rig, plan), []);
  const by = (name: string) => plan.routes.filter((r) => r.name.startsWith(name));
  assert.deepEqual(by("Colour hits").map((r) => r.sections), [[]]);
  assert.deepEqual(by("Strobe").map((r) => r.sections), [["drop"]]);
  const lasers = by("Laser looks");
  assert.deepEqual(lasers.map((r) => r.sections), [["breakdown", "groove", "build"], ["drop"]]);
  assert.match(lasers[1].name, /drop$/);
  const tags = lasers.map((r) => (r.action.type === "look" ? r.action.tags[0] : ""));
  assert.notEqual(tags[0], tags[1], "the drop uses the peak looks");
  assert.ok(plan.routes.every((r) => r.scene === plan.presets[0].id));
  assert.equal(plan.presets[0].blocks?.length, 3, "the scene keeps its blocks");
});

test("templates make block scenes that own their routes", () => {
  const rig = clubRig({ looksTagged: true });
  const plan = buildTemplate(structuredClone(rig), "club", null);
  for (const p of plan.presets) {
    assert.ok(p.blocks?.length, p.name);
    const own = plan.routes.filter((r) => r.scene === p.id);
    assert.ok(own.length, p.name);
    assert.ok(own.every((r) => p.routes[r.id] === true));
    assert.ok(plan.routes.filter((r) => r.scene !== p.id).every((r) => p.routes[r.id] === false), "other scenes' routes go off");
  }
});

function withScenes(): Show {
  const s = structuredClone(clubRig({ looksTagged: true }));
  const plan = buildTemplate(structuredClone(s), "party", null);
  applyPlan(s, plan, new Set([...plan.routes.map((r) => r.id), ...plan.presets.map((p) => p.id), ...plan.palettes.map((p) => p.id)]), false);
  return s;
}

test("editing a scene's blocks rebuilds only its routes, with stable ids", () => {
  const s = withScenes();
  const [groove, peak] = s.presets;
  const peakRoutes = ownedRoutes(s, peak.id).map((r) => r.id);
  groove.blocks!.push(strobe({ sections: ["build"] }));
  rebuildScene(s, groove.id);
  const own = ownedRoutes(s, groove.id);
  assert.equal(own.length, groove.blocks!.length);
  assert.ok(own.every((r) => groove.routes[r.id] === true && peak.routes[r.id] === false), "the other scene switches them off");
  assert.deepEqual(ownedRoutes(s, peak.id).map((r) => r.id), peakRoutes, "the other scene is untouched");
  assert.deepEqual(own.at(-1)!.sections, ["build"]);
  // Rebuilding again keeps the ids, so pads mapped to them keep working.
  const ids = own.map((r) => r.id);
  groove.blocks![0].speed = "fast";
  rebuildScene(s, groove.id);
  assert.deepEqual(ownedRoutes(s, groove.id).map((r) => r.id), ids);
  assert.deepEqual(planProblems(s, { title: "", summary: "", sensitivity: null, bands: [], palettes: [], routes: s.routes, presets: [], controls: [] }), []);
  // Nothing dangles: every scene only mentions routes that exist.
  const all = new Set(s.routes.map((r) => r.id));
  for (const p of s.presets) for (const k of Object.keys(p.routes)) assert.ok(all.has(k));
});

test("a new scene starts empty; rename and delete keep things tidy", () => {
  const s = withScenes();
  const p = addBlockScene(s, "mine", "Mine");
  assert.ok(isScene(p));
  p.blocks!.push(newBlock("wave"));
  rebuildScene(s, p.id);
  assert.equal(ownedRoutes(s, "mine").length, 1);
  assert.ok(ownedRoutes(s, "mine").every((r) => !r.enabled), "a scene that isn't playing builds its routes off");
  renameScene(s, "mine", "Ocean");
  assert.deepEqual(ownedRoutes(s, "mine").map((r) => r.group), ["Ocean"]);
  s.controls.push({ id: "pad", name: "", enabled: true, trigger: { source: "key", kind: "note", channel: 0, number: 0, key: "o", mode: "on" }, action: { type: "preset", preset: "mine" } });
  const before = s.routes.length;
  deleteScene(s, "mine");
  assert.equal(s.routes.length, before - 1);
  assert.ok(!s.presets.some((x) => x.id === "mine"));
  assert.equal(s.controls.at(-1)!.action.type === "preset" && s.controls.at(-1)!.action.preset, "");
});

test("presets of hand-set values aren't scenes", () => {
  const s = small();
  s.presets.push({ id: "look", name: "Warm", color: "#fff", values: [{ fixture: "p1", channel: "dim", value: 9 }], routes: {}, exclusive: false });
  assert.deepEqual(s.presets.map(isScene), [true, false]);
});

// ---------- scenes as a state of the rig ----------
import { captureScene, rigRows, rowLook, setRowLook, routeState } from "../src/lib/scenes.ts";
import { route } from "./fixtures.ts";

test("the scene page lists the rig: groups, loose fixtures, and routes aimed at each", () => {
  const s = structuredClone(clubRig({ looksTagged: true }));
  s.fixtures.push({ ...s.fixtures[0], id: "spare", name: "Spare par" });
  s.routes = [
    route("all", "Everything", "", { groups: [] }),
    route("front", "Front flash", "", { groups: ["g-front"] }),
    route("laser", "Laser look", "", { groups: ["g-laser"] }),
    route("spare", "Spare only", "", { groups: [], fixtures: ["spare"] }),
  ];
  const rows = rigRows(s);
  assert.deepEqual(rows.map((r) => r.label), ["Whole rig", "All pars", "Front", "Back", "Laser", "fog", "Spare par"]);
  const by = (label: string) => rows.find((r) => r.label === label)!;
  assert.deepEqual(by("Whole rig").routes.map((r) => r.id), ["all"]);
  assert.deepEqual(by("Front").routes.map((r) => r.id), ["front"]);
  assert.deepEqual(by("All pars").routes.map((r) => r.id), [], "a route shows under the group it targets, not every group sharing fixtures");
  assert.deepEqual(by("Spare par").routes.map((r) => r.id), ["spare"]);
  assert.ok(by("Laser").looks.length > 0 && by("Front").looks.length === 0, "only fixtures with saved looks offer them");
});

test("a scene puts rows in looks, one look per row", () => {
  const s = structuredClone(clubRig({ looksTagged: true }));
  const laser = rigRows(s).find((r) => r.label === "Laser")!;
  const p = addBlockScene(s, "sc", "Node mode");
  const [a, b] = laser.looks.map((l) => l.name);
  setRowLook(p, laser, a);
  setRowLook(p, laser, b);
  assert.equal(p.looks!.length, 1);
  assert.equal(rowLook(p, laser), b);
  assert.deepEqual(p.looks![0].target, { rooms: [], groups: ["g-laser"], fixtures: [] });
  setRowLook(p, laser, "");
  assert.equal(p.looks!.length, 0);
});

test("capture saves the routes that are on and the hand-set values", () => {
  const s = withScenes();
  const [groove] = s.presets;
  const mine = route("mine", "My route", "Mine", { enabled: true });
  s.routes.push(mine);
  for (const r of s.routes) if (r.scene && r.scene !== groove.id) r.enabled = true; // the other scene's routes are on right now
  const got = captureScene(s, groove, { laser: { p1_pattern: 12 } });
  assert.equal(routeState(groove, "mine"), "on");
  assert.ok(s.routes.filter((r) => r.scene === groove.id).every((r) => groove.routes[r.id] === true), "its own block routes stay on");
  assert.deepEqual(groove.base, [{ fixture: "laser", channel: "p1_pattern", value: 12 }]);
  assert.equal(got.values, 1);
  captureScene(s, groove, { laser: { p1_pattern: 30 } });
  assert.deepEqual(groove.base, [{ fixture: "laser", channel: "p1_pattern", value: 30 }], "capturing again updates, not duplicates");
  assert.ok(isScene({ ...groove, routes: {}, blocks: undefined }), "values under the routes make it a scene, not a preset");
});

test("a laser looks block can follow the section", () => {
  const rig = clubRig({ looksTagged: true });
  const { plan } = buildScenes(rig, [{ name: "Lasers", blocks: [{ ...newBlock("laser"), follow: true }, newBlock("laser")] }], { genre: null, title: "t" });
  assert.deepEqual(plan.routes.map((r) => r.action.type === "look" && r.action.follow_section), [true, false]);
  assert.deepEqual(planProblems(rig, plan), []);
});

test("looks with the same name on different patterns are told apart", () => {
  const s = structuredClone(clubRig({ looksTagged: true }));
  const laser = rigRows(s).find((r) => r.label === "Laser")!;
  const prof = s.profiles.find((p) => p.id === laser.fixtures[0].profile_id)!;
  const key = Object.keys(prof.looks[0].values).find((k) => prof.channels.find((c) => c.key === k)?.group === "pattern")!;
  const base = prof.looks[0];
  prof.looks.push({ ...structuredClone(base), id: "twin-a", name: "Twin", values: { [key]: 201 } }, { ...structuredClone(base), id: "twin-b", name: "Twin", values: { [key]: 202 } });
  const row = rigRows(s).find((r) => r.label === "Laser")!;
  const twins = row.looks.filter((l) => l.name === "Twin");
  assert.equal(twins.length, 2, "one entry per pattern");
  assert.notEqual(twins[0].group, twins[1].group);
  assert.deepEqual(twins.map((t) => t.value), ["twin-a", "twin-b"], "picked by id, not name");
  const p = addBlockScene(s, "sc", "Twins");
  setRowLook(p, row, twins[1].value);
  assert.equal(rowLook(p, row), "twin-b");
  assert.ok(row.looks.filter((l) => l.name !== "Twin").every((l) => l.value === l.name), "unique names still pick by name, so they work across fixture types");
});
