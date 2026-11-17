import { test } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_AUTOPILOT, SECTIONS, applyAutopilotPlan, autopilotSettings, coverage, coverageNote, guessAll, guessSections, toggleSection, undoAutopilotPlan,
} from "../src/lib/autopilot.ts";
import { TEMPLATES, buildTemplate } from "../src/lib/templates.ts";
import { buildScenes } from "../src/lib/blocks.ts";
import { BUILTIN_CONTROLLERS, DEFAULT_AUTOMAP, automap } from "../src/lib/controllers.ts";
import type { Preset, RouteAction, Show } from "../src/lib/types.ts";
import { route, show } from "./fixtures.ts";
import { clubRig } from "./rig.ts";

const scene = (id: string, name: string, routes: Record<string, boolean> = {}, sections?: Preset["sections"]): Preset =>
  ({ id, name, color: "#fff", values: [], routes, exclusive: false, sections });

const strobe: RouteAction = { type: "strobe", rate: "4", hz: 10, duty: 0.3, burst_ms: 200, threshold: 0.6 } as RouteAction;
const wave: RouteAction = { type: "wave", palette: "pal-club", axis: "x", beats: 8, width: 0.4, base: 0.1 } as RouteAction;
const palette: RouteAction = { type: "palette", palette: "pal-club", mode: "spread", fade_ms: 100, brightness: 255 } as RouteAction;

test("sections are guessed from a scene's name first", () => {
  const s = show();
  const g = (name: string) => guessSections(s, scene("x", name)).sections;
  assert.deepEqual(g("Drop"), ["drop"]);
  assert.deepEqual(g("Peak time"), ["drop"]);
  assert.deepEqual(g("Build"), ["build"]);
  assert.deepEqual(g("Breakdown"), ["breakdown"]);
  assert.deepEqual(g("Chill"), ["breakdown"]);
  assert.deepEqual(g("Warm-up"), ["breakdown", "groove"]);
  assert.deepEqual(g("Groove"), ["groove"]);
  assert.deepEqual(g("Test"), [], "utility scenes aren't played by the autopilot");
  assert.deepEqual(g("Blackout"), []);
});

test("then from what the scene switches on", () => {
  const s = show();
  s.routes = [
    route("st", "Strobe", "A", { source: "kick", action: strobe }),
    route("hit", "Kick colour", "A", { source: "kick", action: palette }),
    route("hit2", "Snare colour", "A", { source: "snare", action: palette }),
    route("fog", "Fog", "A", { source: "bar", channel: "@fog" }),
    route("en", "Energy", "B", { source: "energy", action: { type: "modulate", min: 0, max: 255, smoothing: 0.5 } }),
    route("wv", "Wave", "C", { source: "beat", action: wave }),
    route("vox", "Vocals", "C", { source: "vocals.level", action: { type: "modulate", min: 0, max: 255, smoothing: 0.5 } }),
  ];
  const g = (routes: string[]) => guessSections(s, scene("x", "Scene 1", Object.fromEntries(routes.map((r) => [r, true])))).sections;
  assert.deepEqual(g(["st", "fog"]), ["drop"]);
  assert.deepEqual(g(["st", "hit", "hit2"]), ["drop"]);
  assert.deepEqual(g(["en", "hit"]), ["build"]);
  assert.deepEqual(g(["wv", "vox"]), ["breakdown"]);
  assert.deepEqual(g(["hit"]), ["groove"]);
  assert.deepEqual(g([]), []);
  // routes switched off by a scene don't count
  assert.deepEqual(guessSections(s, scene("x", "Scene 2", { st: false, fog: false, hit: true })).sections, ["groove"]);
});

test("guessing leaves tagged scenes alone unless asked", () => {
  const s = show();
  s.presets = [scene("a", "Drop", {}, ["groove"]), scene("b", "Chill")];
  assert.deepEqual(guessAll(s).map((c) => c.name), ["Chill"]);
  assert.deepEqual(s.presets.map((p) => p.sections), [["groove"], ["breakdown"]]);
  guessAll(s, true);
  assert.deepEqual(s.presets[0].sections, ["drop"]);
  assert.deepEqual(guessAll(s, true), [], "nothing left to change");
});

test("templates tag their scenes so the autopilot can play them", () => {
  const rig = clubRig({ looksTagged: true });
  for (const t of TEMPLATES) {
    const plan = buildTemplate(structuredClone(rig), t.id, null);
    const covered = new Set(plan.presets.flatMap((p) => p.sections ?? []));
    if (t.id === "test") assert.equal(covered.size, 0, "the rig test isn't for the autopilot");
    else assert.deepEqual(SECTIONS.map((x) => x.id).filter((x) => !covered.has(x)), [], `${t.name} covers every section`);
  }
});

test("scenes from the builder are tagged from their names", () => {
  const rig = clubRig({ looksTagged: true });
  const { plan } = buildScenes(rig, [
    { name: "Big drop", blocks: [{ block: "colour_hits", speed: "fast", target: "all", sound: "kick", palette: "hot" }] },
    { name: "Mine", sections: ["build"], blocks: [{ block: "wave", speed: "slow", target: "all", palette: "calm" }] },
  ], { genre: null, title: "t" });
  assert.deepEqual(plan.presets.map((p) => p.sections), [["drop"], ["build"]]);
});

test("controller mapping adds an autopilot pad once scenes are tagged", () => {
  const apc = BUILTIN_CONTROLLERS.find((c) => c.id === "akai-apc-mini-mk2")!;
  const s = show();
  const pad = (sh: Show) => automap(sh, apc, DEFAULT_AUTOMAP).assigned.filter((a) => a.control.action.type === "autopilot");
  assert.equal(pad(s).length, 0);
  s.presets[0].sections = ["groove"];
  const got = pad(s);
  assert.deepEqual(got.map((a) => [a.control.trigger.source, a.control.trigger.mode]), [["midi", "toggle"], ["key", "toggle"]]);
  assert.equal(got[1].control.trigger.key, "a");
  assert.equal(automap(s, apc, { ...DEFAULT_AUTOMAP, autopilot: false }).assigned.filter((a) => a.control.action.type === "autopilot").length, 0);
});

test("an AI suggestion applies and undoes cleanly", () => {
  const s = show();
  s.presets = [scene("a", "One", {}, ["groove"]), scene("b", "Two")];
  assert.deepEqual(autopilotSettings(s), DEFAULT_AUTOPILOT, "older shows get the defaults");
  const before = JSON.stringify(s);
  const undo = applyAutopilotPlan(s, { sections: { b: ["drop", "build"] }, settings: { min_bars: 4, max_bars: 16, drop_bars: 1, pause_s: 30 }, notes: "", warnings: [] });
  assert.deepEqual(s.presets.map((p) => p.sections), [["groove"], ["drop", "build"]]);
  assert.equal(s.autopilot!.min_bars, 4);
  undoAutopilotPlan(s, undo);
  assert.equal(JSON.stringify(s), before.replace('"name":"Two","color":"#fff","values":[],"routes":{},"exclusive":false', '"name":"Two","color":"#fff","values":[],"routes":{},"exclusive":false,"sections":[]'));
});

test("coverage says which sections have no scene", () => {
  const s = show();
  s.presets = [scene("a", "One")];
  assert.match(coverageNote(s), /No scenes are tagged/);
  toggleSection(s.presets[0], "drop");
  toggleSection(s.presets[0], "groove");
  assert.deepEqual(s.presets[0].sections, ["groove", "drop"], "kept in a fixed order");
  assert.equal(coverageNote(s), "No scene for breakdown, build: the autopilot uses the nearest section instead.");
  assert.deepEqual(coverage(s).drop.map((p) => p.id), ["a"]);
  toggleSection(s.presets[0], "drop");
  assert.deepEqual(s.presets[0].sections, ["groove"]);
});
