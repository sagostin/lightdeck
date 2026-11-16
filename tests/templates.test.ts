import { test } from "node:test";
import assert from "node:assert/strict";
import { TEMPLATES, analyseRig, buildTemplate } from "../src/lib/templates.ts";
import { GENRES } from "../src/lib/genres.ts";
import { show as emptyRig } from "./fixtures.ts";
import { clubRig, planProblems } from "./rig.ts";

const variants = {
  "full rig": clubRig({ looksTagged: true }),
  "untagged looks": clubRig(),
  "no focus point": clubRig({ focus: false }),
  "no groups": clubRig({ groups: false }),
};

for (const t of TEMPLATES) {
  for (const [what, rig] of Object.entries(variants)) {
    for (const g of [null, GENRES.find((x) => x.id === "dnb")!, GENRES.find((x) => x.id === "ambient")!]) {
      test(`${t.name} on ${what}${g ? ` (${g.name})` : ""} only references what exists`, () => {
        const plan = buildTemplate(structuredClone(rig), t.id, g);
        assert.ok(plan.routes.length > 0);
        assert.deepEqual(planProblems(rig, plan), []);
        // exactly the first scene starts on
        const first = plan.presets[0].name;
        assert.ok(plan.routes.every((r) => r.enabled === (r.group === first)));
        // each scene turns its own routes on and the rest off
        for (const p of plan.presets) for (const r of plan.routes) assert.equal(p.routes[r.id], r.group === p.name);
      });
    }
  }
}

test("club night has its four scenes and uses the whole rig", () => {
  const plan = buildTemplate(clubRig({ looksTagged: true }), "club", null);
  assert.deepEqual(plan.presets.map((p) => p.name), ["Warm-up", "Build", "Drop", "Breakdown"]);
  const drop = plan.routes.filter((r) => r.group === "Drop");
  assert.deepEqual(drop.map((r) => r.action.type).sort(), ["flash", "look", "palette", "strobe"]);
  const look = drop.find((r) => r.action.type === "look")!;
  assert.deepEqual(look.action.type === "look" && look.action.tags, ["peak"], "uses the peak tag when looks have it");
  assert.deepEqual(look.groups, ["g-laser"], "targets the laser by its group");
  const fog = drop.find((r) => r.action.type === "flash")!;
  assert.ok(fog.cooldown_ms >= 20000, "fog never runs continuously");
  const kick = drop.find((r) => r.action.type === "palette")!;
  assert.deepEqual([kick.focus, kick.groups], ["floor", ["g-all"]]);
  assert.doesNotMatch(plan.summary, /Note:/);
});

test("chill has no strobe or fog; ambient genre removes strobe everywhere", () => {
  const rig = clubRig();
  const chill = buildTemplate(rig, "chill", null);
  assert.ok(!chill.routes.some((r) => r.action.type === "strobe" || r.channel === "@fog"));
  for (const t of TEMPLATES) {
    const plan = buildTemplate(rig, t.id, GENRES.find((g) => g.id === "ambient")!);
    assert.ok(!plan.routes.some((r) => r.action.type === "strobe"), t.name);
  }
});

test("halves come from groups, or split the fixtures when there are no groups", () => {
  assert.deepEqual([analyseRig(clubRig()).a.groups, analyseRig(clubRig()).b.groups], [["g-front"], ["g-back"]]);
  const noGroups = analyseRig(clubRig({ groups: false }));
  assert.deepEqual(noGroups.a.fixtures, ["par1", "par3", "par5", "par7"]);
  assert.deepEqual(noGroups.all.fixtures.length, 8);
});

test("missing palettes are added; existing ones are reused by name", () => {
  const rig = clubRig();
  const plan = buildTemplate(rig, "club", GENRES.find((g) => g.id === "techno")!);
  assert.deepEqual(plan.palettes, [], "Ice and White exist in the show");
  rig.palettes = rig.palettes.filter((p) => p.name !== "Ice");
  const again = buildTemplate(rig, "club", GENRES.find((g) => g.id === "techno")!);
  assert.deepEqual(again.palettes.map((p) => p.name), ["Ice"]);
  assert.deepEqual(planProblems(rig, again), []);
});

test("an empty rig builds nothing and says why", () => {
  const plan = buildTemplate(emptyRig(), "club", null);
  assert.equal(plan.routes.length, 0);
  assert.match(plan.summary, /no colour or brightness fixtures/);
});

test("the summary points out what would make it better", () => {
  const plan = buildTemplate(clubRig({ focus: false }), "club", null);
  assert.match(plan.summary, /tag laser looks/);
  assert.match(plan.summary, /focus point/);
});

test("the plan checker used above does catch broken plans", () => {
  const rig = clubRig();
  const plan = buildTemplate(rig, "club", null);
  const bad = structuredClone(plan);
  const wave = bad.routes.find((r) => r.action.type === "wave")!;
  wave.groups = []; wave.fixtures = ["ghost-fixture"];
  const look = bad.routes.find((r) => r.action.type === "look")!;
  look.groups = ["nope"];
  bad.routes.find((r) => r.action.type === "twinkle")!.source = "theremin";
  const flash = bad.routes.find((r) => r.channel === "@intensity")!;
  flash.channel = "@pan";
  bad.presets[0].routes["ghost"] = true;
  bad.controls.push({ id: "c", name: "Dup", enabled: true, trigger: { source: "key", kind: "note", channel: 0, number: 0, key: "x", mode: "on" }, action: { type: "preset", preset: "nope" } },
    { id: "d", name: "Dup2", enabled: true, trigger: { source: "key", kind: "note", channel: 0, number: 0, key: "x", mode: "on" }, action: { type: "blackout" } });
  const problems = planProblems(rig, bad).join("\n");
  for (const m of ["unknown group nope", "unknown source theremin", "unknown fixture ghost-fixture", "effect reaches no colour fixture", "no target has channel @pan", "unknown route ghost", "unknown preset", "used twice"])
    assert.match(problems, new RegExp(m));
});

test("targets use your groups and list only the fixtures no group covers", () => {
  const rig = clubRig();
  rig.groups = rig.groups.filter((g) => g.id !== "g-all");
  const t = analyseRig(rig).all;
  assert.deepEqual(t, { rooms: [], groups: ["g-front", "g-back"], fixtures: [] });
  rig.groups = rig.groups.filter((g) => g.id !== "g-back");
  assert.deepEqual(analyseRig(rig).all, { rooms: [], groups: ["g-front"], fixtures: ["par5", "par6", "par7", "par8"] });
});
