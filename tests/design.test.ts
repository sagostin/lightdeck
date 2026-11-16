import { test } from "node:test";
import assert from "node:assert/strict";
import { aiCounts, applyPlan, describeRoute, describeTrigger, isAi, type Plan } from "../src/lib/design.ts";
import type { ControlMapping } from "../src/lib/types.ts";
import { route, show } from "./fixtures.ts";

const pad = (id: string, number: number, action: ControlMapping["action"], source: "midi" | "key" = "midi"): ControlMapping => ({
  id, name: id, enabled: true, trigger: { source, kind: "note", channel: 0, number, key: source === "key" ? "1" : "", mode: "on" }, action,
});

function plan(tag = "a"): Plan {
  const r1 = `ai-r-${tag}1`, r2 = `ai-r-${tag}2`;
  return {
    title: "Test", summary: "", sensitivity: 0.7,
    bands: [
      { ...show().audio.bands[0], sensitivity: 0.9 },
      { id: "clap", name: "Clap", enabled: true, ranges: [{ lo_hz: 1000, hi_hz: 4000, weight: 1 }], detect: "transient", level_threshold: 0.6, gain_db: 0, sensitivity: 0.5, min_gap_ms: 150, release: 0.8, color: "#ffd27a" },
    ],
    palettes: [{ id: `ai-pal-${tag}1`, name: "Acid", colors: ["#00ff00"] }, { id: `ai-pal-${tag}2`, name: "Unused", colors: ["#000001"] }],
    routes: [
      route(r1, "Warm wave", "Warm-up", { enabled: true, source: "beat", channel: "", action: { type: "wave", palette: `ai-pal-${tag}1`, axis: "x", beats: 8, width: 0.4, base: 10 } }),
      route(r2, "Kick flash", "Drop", { groups: ["back"] }),
    ],
    presets: [
      { id: `ai-p-${tag}1`, name: "Warm-up", color: "#f80", values: [], routes: { [r1]: true, [r2]: false }, exclusive: false },
      { id: `ai-p-${tag}2`, name: "Drop", color: "#f00", values: [], routes: { [r1]: false, [r2]: true }, exclusive: false },
    ],
    controls: [
      pad(`ai-c-${tag}1`, 0, { type: "preset", preset: `ai-p-${tag}1` }),
      pad(`ai-c-${tag}2`, 1, { type: "preset", preset: `ai-p-${tag}2` }),
      pad(`ai-c-${tag}3`, 2, { type: "routes", routes: [r2] }),
      pad(`ai-c-${tag}4`, 3, { type: "route_group", group: "Drop" }),
      pad(`ai-c-${tag}5`, 0, { type: "master" }, "key"),
    ],
  };
}

const everything = (p: Plan) => new Set([
  ...p.routes.map((x) => x.id), ...p.presets.map((x) => x.id), ...p.controls.map((x) => x.id),
  ...p.palettes.map((x) => x.id), ...p.bands.map((b) => `band:${b.id}`), "sensitivity",
]);

test("applying a whole plan adds it after the user's own work", () => {
  const s = show(), p = plan();
  const msg = applyPlan(s, p, everything(p), false);
  assert.equal(msg, "Added 2 routes, 2 scenes, 5 controls, 2 bands");
  assert.deepEqual(s.routes.map((r) => r.id), ["mine", "ai-r-a1", "ai-r-a2"]);
  assert.equal(s.presets.length, 3);
  assert.equal(s.controls.length, 6);
  assert.equal(s.palettes.length, 3);
  assert.equal(s.audio.sensitivity, 0.7);
  assert.equal(s.audio.bands.find((b) => b.id === "kick")!.sensitivity, 0.9);
  assert.equal(s.audio.bands.at(-1)!.id, "clap");
  assert.equal(s.audio.bands[0].id, "kick");
  assert.ok(s.midi.enabled, "MIDI switched on for MIDI mappings");
});

test("the applied show doesn't share objects with the plan", () => {
  const s = show(), p = plan();
  applyPlan(s, p, everything(p), false);
  s.routes[1].name = "Edited";
  s.audio.bands[0].sensitivity = 0.1;
  assert.equal(p.routes[0].name, "Warm wave");
  assert.equal(p.bands[0].sensitivity, 0.9);
});

test("unticked items are left out, and nothing dangles", () => {
  const s = show(), p = plan();
  // keep only the Drop scene and its pad, the wave-less route, and the scene-group pad
  const keep = new Set(["ai-r-a2", "ai-p-a2", "ai-c-a2", "ai-c-a3", "ai-c-a4", "ai-c-a1"]);
  applyPlan(s, p, keep, false);
  assert.deepEqual(s.routes.map((r) => r.id), ["mine", "ai-r-a2"]);
  assert.deepEqual(s.presets.at(-1)!.routes, { "ai-r-a2": true }, "links to unticked routes are removed");
  // the Warm-up pad pointed at an unticked preset, so it's dropped even though it was ticked
  assert.deepEqual(s.controls.map((c) => c.id), ["my-control", "ai-c-a2", "ai-c-a3", "ai-c-a4"]);
  assert.equal(s.palettes.length, 1, "no palettes: none ticked and no kept route needs one");
  assert.equal(s.audio.sensitivity, 0.5);
  assert.equal(s.audio.bands.find((b) => b.id === "kick")!.sensitivity, 0.5);
});

test("a palette a kept route needs comes along even when unticked", () => {
  const s = show(), p = plan();
  applyPlan(s, p, new Set(["ai-r-a1"]), false);
  assert.deepEqual(s.palettes.map((x) => x.id), ["pal-club", "ai-pal-a1"]);
});

test("a scene preset whose routes were all unticked is skipped", () => {
  const s = show(), p = plan();
  applyPlan(s, p, new Set(["ai-p-a1"]), false);
  assert.equal(s.presets.length, 1);
  assert.equal(applyPlan(show(), p, new Set(), false), "Nothing selected");
});

test("replacing removes earlier AI work only", () => {
  const s = show();
  const first = plan("a");
  applyPlan(s, first, everything(first), false);
  // the user edits and links their own preset to an AI route
  s.presets[0].routes["ai-r-a2"] = true;
  s.routes.push(route("ai-r-zz", "Kept? no", "Old", { action: { type: "palette", palette: "ai-pal-a1", mode: "all", fade_ms: 0, brightness: 0 } }));
  const second = plan("b");
  const msg = applyPlan(s, second, everything(second), true);
  assert.equal(msg, "Added 2 routes, 2 scenes, 5 controls, 2 bands");
  assert.deepEqual(s.routes.map((r) => r.id), ["mine", "ai-r-b1", "ai-r-b2"]);
  assert.deepEqual(s.presets.map((p) => p.id), ["my-preset", "ai-p-b1", "ai-p-b2"]);
  assert.deepEqual(s.controls.map((c) => c.id).filter((id) => !isAi(id)), ["my-control"]);
  assert.ok(!s.controls.some((c) => c.id.startsWith("ai-c-a")));
  assert.deepEqual(s.presets[0].routes, { mine: true }, "the user's preset loses only the removed AI route");
  assert.deepEqual(s.palettes.map((x) => x.id), ["pal-club", "ai-pal-b1", "ai-pal-b2"], "old AI palettes nothing uses are removed; ticked new ones added");
  assert.deepEqual(aiCounts(s), { routes: 2, presets: 2, controls: 5 });
});

test("replacing keeps an old AI palette the user's own route still uses", () => {
  const s = show();
  const first = plan("a");
  applyPlan(s, first, everything(first), false);
  s.routes[0].action = { type: "twinkle", palette: "ai-pal-a1", amount: 1, decay_ms: 100, base: 0 };
  applyPlan(s, plan("b"), new Set(), true);
  assert.ok(s.palettes.some((x) => x.id === "ai-pal-a1"));
});

test("key-only mappings don't switch MIDI on", () => {
  const s = show(), p = plan();
  applyPlan(s, p, new Set(["ai-c-a5"]), false);
  assert.equal(s.midi.enabled, false);
  assert.equal(s.controls.length, 2);
});

test("previews describe routes and triggers", () => {
  const s = show(), p = plan();
  assert.equal(describeRoute(p.routes[0], s, p), "beat → Acid wave (x, 8 beats) · Front");
  assert.equal(describeRoute(p.routes[1], s, p), "kick → flash 255→0 over 100 ms · Back · intensity");
  assert.equal(describeRoute({ ...p.routes[1], groups: [] }, s, p), "kick → flash 255→0 over 100 ms · all fixtures · intensity");
  assert.equal(describeTrigger(p.controls[0]), "Note 0");
  assert.equal(describeTrigger({ ...p.controls[0], trigger: { ...p.controls[0].trigger, kind: "cc", number: 48, channel: 2 } }), "CC 48 ch2");
  assert.equal(describeTrigger(p.controls[4]), "Key 1");
  assert.equal(describeTrigger({ ...p.controls[4], trigger: { ...p.controls[4].trigger, key: " " } }), "Key Space");
});
