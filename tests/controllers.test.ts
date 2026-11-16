import { test } from "node:test";
import assert from "node:assert/strict";
import { BUILTIN_CONTROLLERS, DEFAULT_AUTOMAP, MAP_PREFIX, applyAutomap, automap, learnInput } from "../src/lib/controllers.ts";
import { buildTemplate } from "../src/lib/templates.ts";
import type { ControllerProfile } from "../src/lib/types.ts";
import { clubRig, planProblems } from "./rig.ts";

const apc = BUILTIN_CONTROLLERS.find((c) => c.id === "akai-apc-mini-mk2")!;
const nano = BUILTIN_CONTROLLERS.find((c) => c.id === "korg-nanokontrol2")!;
const launchpad = BUILTIN_CONTROLLERS.find((c) => c.id === "novation-launchpad")!;

test("built-in layouts are valid and have no duplicate inputs", () => {
  for (const c of BUILTIN_CONTROLLERS) {
    const keys = [...c.pads, ...c.faders].map((i) => `${i.kind}${i.number}`);
    assert.equal(new Set(keys).size, keys.length, `${c.name} repeats an input`);
    for (const i of [...c.pads, ...c.faders]) assert.ok(i.number >= 0 && i.number <= 127 && i.label, `${c.name} ${i.label}`);
    assert.ok(c.pads.length >= 16, c.name);
  }
  // spot checks of the documented layouts
  assert.equal(apc.pads.length, 80);
  assert.deepEqual(apc.pads.slice(0, 2).map((p) => p.number), [56, 57], "APC: top-left pad first");
  assert.equal(apc.pads[63].number, 7);
  assert.deepEqual(apc.faders[0], { kind: "cc", number: 56, label: "Master fader" });
  assert.deepEqual(launchpad.pads.slice(0, 2).map((p) => p.number), [81, 82]);
  assert.equal(launchpad.pads[63].number, 18);
  assert.ok(nano.pads.every((p) => p.kind === "cc"), "nanoKONTROL buttons send CC");
});

function sceneRig() {
  const show = clubRig({ looksTagged: true });
  const plan = buildTemplate(show, "club", null);
  show.routes = plan.routes; show.presets = plan.presets; show.palettes.push(...plan.palettes);
  return show;
}

test("auto-map gives scenes the first pads and master the first fader", () => {
  const show = sceneRig();
  const r = automap(show, apc, DEFAULT_AUTOMAP);
  const byName = (n: string) => r.assigned.find((a) => a.control.name === n && a.control.trigger.source === "midi")!;
  assert.equal(byName("Warm-up").control.trigger.number, 56);
  assert.equal(byName("Warm-up").control.trigger.mode, "on");
  assert.equal(byName("Build").control.trigger.number, 57);
  assert.equal(byName("Master").control.trigger.kind, "cc");
  assert.equal(byName("Master").control.trigger.number, 56);
  assert.equal(byName("Strobe (hold)").control.trigger.mode, "momentary");
  assert.equal(byName("Fog (hold)").control.action.type, "channel_value");
  assert.ok(byName("Front brightness"), "a fader per fixture group with brightness");
  assert.ok(!r.assigned.some((a) => a.control.name === "Laser brightness"), "the laser has no brightness channel");
  assert.ok(byName("Kick sensitivity"));
  assert.deepEqual(r.unmapped, []);
  // keys: scenes on 1.., strobe on S, fog on F
  const keys = r.assigned.filter((a) => a.control.trigger.source === "key").map((a) => [a.control.name, a.control.trigger.key]);
  assert.deepEqual(keys.slice(0, 4), [["Warm-up", "1"], ["Build", "2"], ["Drop", "3"], ["Breakdown", "4"]]);
  assert.ok(keys.some(([n, k]) => n === "Strobe (hold)" && k === "s"));
  assert.deepEqual(planProblems(show, { title: "", summary: "", sensitivity: null, bands: [], palettes: [], routes: [], presets: [], controls: r.assigned.map((a) => a.control) }), []);
});

test("auto-map skips pads your own controls use and reports what doesn't fit", () => {
  const show = sceneRig();
  show.controls.push({ id: "mine", name: "Mine", enabled: true, trigger: { source: "midi", kind: "note", channel: 0, number: 56, key: "", mode: "toggle" }, action: { type: "blackout" } });
  show.controls.push({ id: "mine2", name: "Mine", enabled: true, trigger: { source: "key", kind: "note", channel: 0, number: 0, key: "1", mode: "on" }, action: { type: "blackout" } });
  const r = automap(show, apc, DEFAULT_AUTOMAP);
  assert.equal(r.assigned.find((a) => a.control.name === "Warm-up")!.control.trigger.number, 57);
  assert.ok(!r.assigned.some((a) => a.control.trigger.source === "key" && a.control.trigger.key === "1"));
  const tiny: ControllerProfile = { id: "t", name: "Tiny", channel: 2, pads: [{ kind: "note", number: 1, label: "P1" }], faders: [], notes: "" };
  const r2 = automap(show, tiny, { ...DEFAULT_AUTOMAP, keys: false });
  assert.equal(r2.assigned.length, 1);
  assert.equal(r2.assigned[0].control.trigger.channel, 2, "uses the profile's channel");
  assert.ok(r2.unmapped.includes("Build") && r2.unmapped.includes("Master"));
});

test("nanoKONTROL buttons are mapped as CC buttons", () => {
  const r = automap(sceneRig(), nano, { ...DEFAULT_AUTOMAP, keys: false });
  const scene = r.assigned[0].control;
  assert.deepEqual([scene.trigger.kind, scene.trigger.number, scene.trigger.mode], ["cc", 32, "on"]);
  const master = r.assigned.find((a) => a.control.name === "Master")!.control;
  assert.deepEqual([master.trigger.number, master.trigger.mode], [0, "fader"]);
});

test("re-mapping replaces only earlier auto-mapped controls", () => {
  const show = sceneRig();
  show.controls.push({ id: "mine", name: "Mine", enabled: true, trigger: { source: "midi", kind: "note", channel: 0, number: 99, key: "", mode: "toggle" }, action: { type: "blackout" } });
  applyAutomap(show, automap(show, apc, DEFAULT_AUTOMAP));
  const first = show.controls.length;
  assert.ok(show.midi.enabled);
  applyAutomap(show, automap(show, apc, DEFAULT_AUTOMAP));
  assert.equal(show.controls.length, first, "no duplicates");
  applyAutomap(show, automap(show, null, { ...DEFAULT_AUTOMAP, keys: false }));
  assert.deepEqual(show.controls.map((c) => c.id), ["mine"]);
  assert.ok(show.controls.every((c) => c.id === "mine" || c.id.startsWith(MAP_PREFIX)));
});

test("learning builds a profile in press order, once per input", () => {
  const p: ControllerProfile = { id: "x", name: "X", channel: 0, pads: [], faders: [], notes: "" };
  assert.ok(learnInput(p, { kind: "note", number: 40 }, "pads"));
  assert.ok(learnInput(p, { kind: "note", number: 41 }, "pads"));
  assert.ok(!learnInput(p, { kind: "note", number: 40 }, "pads"), "pressing again doesn't add it twice");
  assert.ok(learnInput(p, { kind: "cc", number: 7 }, "faders"));
  assert.ok(!learnInput(p, { kind: "cc", number: 7 }, "pads"), "already a fader");
  assert.ok(!learnInput(p, { kind: "program", number: 1 }, "pads"));
  assert.deepEqual(p.pads.map((i) => [i.number, i.label]), [[40, "Pad 1"], [41, "Pad 2"]]);
  assert.deepEqual(p.faders, [{ kind: "cc", number: 7, label: "Fader 1" }]);
});

test("mapping status notices changed scenes and dead controls", async () => {
  const { mappingStatus, isDead } = await import("../src/lib/controllers.ts");
  const show = sceneRig();
  assert.deepEqual(mappingStatus(show, automap(show, apc, DEFAULT_AUTOMAP)), { mapped: 0, dead: 0, changed: false }, "nothing mapped yet");
  applyAutomap(show, automap(show, apc, DEFAULT_AUTOMAP));
  assert.deepEqual(mappingStatus(show, automap(show, apc, DEFAULT_AUTOMAP)), { mapped: show.controls.length, dead: 0, changed: false });
  // the same actions on other pads is not "changed"…
  const moved = structuredClone(show);
  moved.controls.forEach((c) => { if (c.trigger.source === "midi") c.trigger.number = 127 - c.trigger.number; });
  assert.equal(mappingStatus(moved, automap(show, apc, DEFAULT_AUTOMAP)).changed, false);
  // …but a controller that can't hold everything (the Launchpad has no faders) is
  assert.equal(mappingStatus(show, automap(show, launchpad, DEFAULT_AUTOMAP)).changed, true);
  // add a scene: the mapping is out of date
  show.presets.push({ id: "new", name: "Encore", color: "#fff", values: [], routes: {}, exclusive: false });
  assert.equal(mappingStatus(show, automap(show, apc, DEFAULT_AUTOMAP)).changed, true);
  // remove a scene and the strobe routes: their controls are dead
  const drop = show.presets.find((p) => p.name === "Drop")!;
  show.presets = show.presets.filter((p) => p !== drop);
  show.routes = show.routes.filter((r) => r.action.type !== "strobe");
  const st = mappingStatus(show, automap(show, apc, DEFAULT_AUTOMAP));
  assert.equal(st.dead, 4, "Drop pad and key, Strobe pad and key");
  assert.ok(show.controls.filter((c) => isDead(show, c)).some((c) => c.name === "Strobe (hold)"));
  // re-mapping fixes it, and leaves no strobe or fog control for things the rig no longer has
  applyAutomap(show, automap(show, apc, DEFAULT_AUTOMAP));
  assert.deepEqual(mappingStatus(show, automap(show, apc, DEFAULT_AUTOMAP)), { mapped: show.controls.length, dead: 0, changed: false });
  assert.ok(!show.controls.some((c) => c.name.startsWith("Strobe")));
  show.fixtures = show.fixtures.filter((f) => f.id !== "fog");
  applyAutomap(show, automap(show, apc, DEFAULT_AUTOMAP));
  assert.ok(!show.controls.some((c) => c.name.startsWith("Fog")), "no fog machine, no fog pad");
});
