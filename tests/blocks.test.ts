import { test } from "node:test";
import assert from "node:assert/strict";
import { BLOCKS, analyseRig, available, buildScenes, newBlock, soundOptions, targetOptions, type SceneSpec, type Speed } from "../src/lib/blocks.ts";
import { buildTemplate, templateScenes, TEMPLATES } from "../src/lib/templates.ts";
import { GENRES } from "../src/lib/genres.ts";
import { clubRig, planProblems } from "./rig.ts";

const SPEEDS: Speed[] = ["slow", "medium", "fast"];
const one = (block: string, more = {}): SceneSpec[] => [{ name: "S", blocks: [{ ...newBlock(block), ...more }] }];
const build = (show = clubRig({ looksTagged: true }), scenes: SceneSpec[], genre = null as (typeof GENRES)[number] | null) =>
  buildScenes(show, scenes, { genre, title: "t" });

test("every block builds a valid route at every speed", () => {
  const show = clubRig({ looksTagged: true });
  const types: Record<string, string> = { colour_hits: "palette", colour_bar: "palette", wave: "wave", sparkle: "twinkle", flash: "flash",
    strobe: "strobe", node_hop: "node_hop", follow: "modulate", laser: "look", fog: "flash" };
  assert.deepEqual(BLOCKS.map((b) => b.id).sort(), Object.keys(types).sort(), "every block is covered here");
  for (const b of BLOCKS) for (const speed of SPEEDS) {
    const { plan, skipped } = build(show, one(b.id, { speed }));
    assert.deepEqual(skipped, [], `${b.id} ${speed}`);
    assert.equal(plan.routes.length, 1);
    assert.equal(plan.routes[0].action.type, types[b.id], b.id);
    assert.deepEqual(planProblems(show, plan), [], `${b.id} ${speed}`);
  }
});

test("speed changes the feel", () => {
  const show = clubRig();
  const wave = (s: Speed) => { const a = build(show, one("wave", { speed: s })).plan.routes[0].action; return a.type === "wave" ? a.beats : 0; };
  assert.ok(wave("slow") > wave("medium") && wave("medium") > wave("fast"));
  const rate = (s: Speed, g = null as (typeof GENRES)[number] | null) => { const a = build(show, one("strobe", { speed: s }), g).plan.routes[0].action; return a.type === "strobe" ? a.rate : ""; };
  assert.deepEqual(SPEEDS.map((s) => rate(s)), ["2", "4", "8"]);
  const dnb = GENRES.find((g) => g.id === "dnb")!;
  assert.deepEqual(SPEEDS.map((s) => rate(s, dnb)), ["1", "2", "4"], "fast music strobes fewer times per beat");
  for (const s of SPEEDS) assert.ok(build(show, one("fog", { speed: s })).plan.routes[0].cooldown_ms >= 30000, "fog always has a long cooldown");
  const laser = (s: Speed, sound: string) => build(show, one("laser", { speed: s, sound })).plan.routes[0].every;
  assert.deepEqual(SPEEDS.map((s) => laser(s, "kick")), [8, 4, 2], "kick-driven lasers change every few kicks");
  assert.equal(laser("fast", "bar"), 1);
});

test("sounds, palettes and tags resolve against the rig", () => {
  const show = clubRig({ looksTagged: true });
  const src = (block: string, sound: string) => build(show, one(block, { sound })).plan.routes[0].source;
  assert.equal(src("flash", "snare"), "snare");
  assert.equal(src("flash", "bass"), "bass", "band ids pass through");
  assert.equal(src("follow", "vocals"), "vocals.level");
  assert.equal(src("follow", "kick.level"), "kick.level");
  show.audio.bands = show.audio.bands.filter((b) => b.id !== "snare");
  assert.equal(src("strobe", "snare"), "beat", "no snare band: falls back to the beat");

  const pal = (palette: string, genre = null as (typeof GENRES)[number] | null) => {
    const { plan } = build(show, one("colour_hits", { palette }), genre);
    const a = plan.routes[0].action;
    const id = a.type === "palette" ? a.palette : "";
    return [...show.palettes, ...plan.palettes].find((p) => p.id === id)?.name;
  };
  assert.equal(pal("hot"), "Club");
  assert.equal(pal("calm", GENRES.find((g) => g.id === "techno")!), "Ice");
  assert.equal(pal("pal-sunset"), "Sunset", "a palette id");
  assert.equal(pal("sunset"), "Sunset", "a name, any case");
  assert.equal(pal("no such palette"), show.palettes[0].name);

  const tags = (tag: string) => { const a = build(show, one("laser", { tag })).plan.routes[0].action; return a.type === "look" ? a.tags : []; };
  assert.deepEqual(tags("peak"), ["peak"]);
  assert.deepEqual(tags("calm"), ["chill"]);
  assert.deepEqual(tags(""), []);
});

test("blocks the rig can't drive are skipped, with a reason", () => {
  const show = clubRig();
  show.fixtures = show.fixtures.filter((f) => f.id !== "fog" && f.id !== "laser");
  const rig = analyseRig(show);
  assert.ok(!available(rig, "fog") && !available(rig, "laser") && available(rig, "glow"));
  const { plan, skipped } = build(show, [{ name: "Drop", blocks: [newBlock("fog"), newBlock("laser"), newBlock("colour_hits")] }]);
  assert.deepEqual(plan.routes.map((r) => r.name), ["Colour hits"]);
  assert.deepEqual(skipped, ["Drop: Fog burst (no fog machine to drive)", "Drop: Laser looks (no lasers with saved looks to drive)"]);
});

test("scenes become presets; names say where repeated blocks go", () => {
  const show = clubRig();
  const scenes: SceneSpec[] = [
    { name: "Empty", blocks: [] },
    { name: "One", blocks: [newBlock("flash"), { ...newBlock("flash"), target: "a" }, { ...newBlock("flash"), target: "a" }, { ...newBlock("flash"), target: "g:g-back" }] },
    { name: "Two", blocks: [newBlock("wave")] },
  ];
  const { plan } = build(show, scenes);
  assert.deepEqual(plan.presets.map((p) => p.name), ["One", "Two"], "a scene with no blocks makes no preset");
  assert.deepEqual(plan.routes.filter((r) => r.group === "One").map((r) => r.name), ["Flash on hit", "Flash on hit · half A", "Flash on hit · half A 2", "Flash on hit · Back"]);
  assert.ok(plan.routes.every((r) => r.enabled === (r.group === "One")), "the first real scene starts on");
  assert.deepEqual(plan.routes.find((r) => r.name.endsWith("Back"))!.groups, ["g-back"]);
  // a group that's been deleted is skipped
  const gone = build(show, one("flash", { target: "g:nope" }));
  assert.equal(gone.plan.routes.length, 0);
  assert.equal(gone.skipped.length, 1);
});

test("target and sound choices come from the rig", () => {
  const show = clubRig();
  const rig = analyseRig(show);
  const glow = targetOptions(show, rig, "glow");
  assert.deepEqual(glow.map((o) => o.id), ["all", "a", "b", "g:g-all", "g:g-front", "g:g-back"]);
  assert.equal(glow[1].label, "Half A (Front)");
  assert.deepEqual(targetOptions(show, rig, "laser").map((o) => o.id), ["lasers", "g:g-laser"]);
  assert.deepEqual(targetOptions(show, rig, "fog").map((o) => o.id), ["fog"]);
  show.audio.bands[1].enabled = false;
  assert.ok(!soundOptions(show, "hit").some((o) => o.id === show.audio.bands[1].id), "muted bands aren't offered");
  assert.deepEqual(soundOptions(show, "level").slice(0, 2).map((o) => o.id), ["energy", "level"]);
});

test("a template opened in the builder builds the same setup", () => {
  for (const t of TEMPLATES) {
    const show = clubRig({ looksTagged: true });
    const viaTemplate = buildTemplate(show, t.id, null);
    const viaBuilder = buildScenes(show, templateScenes(t.id, show), { genre: null, title: viaTemplate.title, moods: t.id === "chill" ? ["Sunset", "Forest"] : undefined }).plan;
    assert.deepEqual(viaBuilder, viaTemplate, t.name);
  }
});

test("hit blocks always get a hit source and level blocks a level source", () => {
  const show = clubRig();
  const isLevel = (s: string) => ["energy", "level", "phase"].includes(s) || s.endsWith(".level");
  for (const b of BLOCKS.filter((x) => x.sound)) {
    for (const sound of ["kick", "snare", "hat", "vocals", "bass", "kick.level", "vocals.level", "energy", "level", "beat", "bar", "nonsense"]) {
      const r = build(show, one(b.id, { sound })).plan.routes[0];
      assert.equal(isLevel(r.source), b.sound!.kind === "level", `${b.id} with "${sound}" got ${r.source}`);
    }
  }
  // every template too
  for (const t of TEMPLATES) for (const r of buildTemplate(show, t.id, null).routes) {
    assert.equal(isLevel(r.source), r.action.type === "modulate", `${t.name}: ${r.name} ← ${r.source}`);
  }
});
