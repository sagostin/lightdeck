import { test } from "node:test";
import assert from "node:assert/strict";
import { BASE_BANDS, GENRES, applyGenre, applyTune, bandUsage, currentGenre, describeTune, genreBands, missingBand, planGenre, removeBand } from "../src/lib/genres.ts";
import { route, show } from "./fixtures.ts";

const g = (id: string) => GENRES.find((x) => x.id === id)!;
const FIELDS = ["detect", "sensitivity", "min_gap_ms", "release", "level_threshold"] as const;

test("every genre is a complete, valid band set with kick first", () => {
  const ids = new Set<string>();
  for (const genre of GENRES) {
    assert.ok(!ids.has(genre.id)); ids.add(genre.id);
    assert.ok(genre.bpm[0] > 40 && genre.bpm[0] < genre.bpm[1] && genre.bpm[1] < 220, genre.id);
    assert.ok(genre.sensitivity > 0 && genre.sensitivity <= 1);
    assert.ok(genre.style.length > 40);
    assert.equal(genre.use[0], "kick", `${genre.id}: kick drives the beat clock`);
    for (const id of [...genre.use, ...Object.keys(genre.tweaks)]) assert.ok(BASE_BANDS[id], `${genre.id} uses unknown band ${id}`);
    for (const id of Object.keys(genre.tweaks)) assert.ok(genre.use.includes(id), `${genre.id} tweaks ${id} but doesn't use it`);
    for (const b of genreBands(genre)) {
      for (const f of FIELDS) assert.notEqual(b[f], undefined, `${genre.id}/${b.id}.${f}`);
      assert.ok(b.sensitivity >= 0 && b.sensitivity <= 1 && b.release >= 0 && b.release <= 0.98 && b.min_gap_ms >= 20, `${genre.id}/${b.id}`);
      assert.ok(b.ranges.length && b.ranges.every((r) => r.hi_hz > r.lo_hz));
    }
  }
  assert.equal(GENRES.length, 10);
});

test("applying a genre sets every setting, including release", () => {
  const s = show();
  s.audio.bands.forEach((b) => { b.release = 0.5; b.level_threshold = 0.1; b.min_gap_ms = 999; });
  applyGenre(s, g("techno"));
  for (const want of genreBands(g("techno"))) {
    const got = s.audio.bands.find((b) => b.id === want.id)!;
    for (const f of FIELDS) assert.deepEqual(got[f], want[f], `${want.id}.${f}`);
  }
  assert.equal(s.audio.bands.find((b) => b.id === "kick")!.release, 0.8);
});

test("missing bands are always created, the same way every time", () => {
  for (const genre of GENRES) {
    const s = show();
    s.audio.bands = [];
    const p = planGenre(s, genre);
    assert.deepEqual(p.changes.map((c) => c.status), genre.use.map(() => "add"), genre.id);
    applyTune(s, p);
    assert.deepEqual(s.audio.bands.map((b) => b.id), genre.use);
    // applying again changes nothing, and the show now reads as this genre
    assert.ok(planGenre(s, genre).same, genre.id);
  }
});

test("switching genres is consistent whatever was there before", () => {
  const fresh = show();
  applyGenre(fresh, g("dubstep"));
  const viaHouse = show();
  applyGenre(viaHouse, g("house"));
  applyTune(viaHouse, planGenre(viaHouse, g("dubstep"), { keepRanges: true, removeOthers: true }));
  assert.deepEqual(viaHouse.audio, fresh.audio, "house's Clap band goes when removing others");
  const kept = show();
  applyGenre(kept, g("house"));
  applyGenre(kept, g("dubstep"));
  assert.deepEqual(kept.audio.bands.map((b) => b.id), [...g("dubstep").use, "clap"], "by default your other bands are kept, after the genre's");
});

test("your ranges, gain, names, colours and mute are kept unless asked", () => {
  const s = show();
  const kick = s.audio.bands.find((b) => b.id === "kick")!;
  Object.assign(kick, { name: "My kick", color: "#123456", enabled: false, gain_db: 6, ranges: [{ lo_hz: 40, hi_hz: 90, weight: 1 }] });
  applyGenre(s, g("techno"));
  const k = s.audio.bands.find((b) => b.id === "kick")!;
  assert.deepEqual([k.name, k.color, k.enabled, k.gain_db, k.ranges[0].hi_hz], ["My kick", "#123456", false, 6, 90]);
  const p = planGenre(s, g("techno"), { keepRanges: false, removeOthers: false });
  assert.deepEqual(p.changes.find((c) => c.id === "kick")!.fields.map((f) => f.field), ["ranges", "gain"]);
  applyTune(s, p);
  assert.deepEqual(s.audio.bands[0].ranges, BASE_BANDS.kick.ranges);
});

test("the preview says what would be overwritten", () => {
  const s = show();
  s.audio.bands.find((b) => b.id === "snare")!.release = 0.5;
  const p = planGenre(s, g("dnb"));
  assert.ok(p.overwrites && !p.same);
  const snare = p.changes.find((c) => c.id === "snare")!;
  assert.equal(snare.status, "update");
  assert.deepEqual(snare.fields.find((f) => f.field === "release"), { field: "release", from: "0.50", to: "0.80" });
  assert.match(describeTune(p), /retuned/);
  // only adding bands overwrites nothing
  const empty = show();
  empty.audio.bands = [];
  empty.audio.sensitivity = g("pop").sensitivity;
  const add = planGenre(empty, g("pop"));
  assert.ok(!add.overwrites && !add.same);
  assert.equal(describeTune(add), "5 bands added");
});

test("kick moves first, and the preview says so", () => {
  const s = show();
  s.audio.bands.reverse();
  const p = planGenre(s, g("pop"));
  assert.ok(p.reordered);
  applyTune(s, p);
  assert.equal(s.audio.bands[0].id, "kick");
  assert.ok(!planGenre(s, g("pop")).reordered);
});

test("removing other bands switches off their routes, and undo data is complete", () => {
  const s = show();
  s.audio.bands.push({ ...structuredClone(BASE_BANDS.clap), id: "clap" });
  s.routes = [route("r-clap", "Clap flash", "x", { enabled: true, source: "clap" }), route("r-kick", "Kick flash", "x", { enabled: true, source: "kick" })];
  s.controls.push({ id: "c", name: "Clap sens", enabled: true, trigger: { source: "midi", kind: "cc", channel: 0, number: 1, key: "", mode: "fader" }, action: { type: "band_sensitivity", band: "clap" } });
  assert.deepEqual(bandUsage(s, "clap"), { routes: ["Clap flash"], controls: ["Clap sens"] });
  const keep = planGenre(s, g("techno"));
  assert.deepEqual(keep.others.map((b) => [b.id, b.routes.length, b.controls.length]), [["clap", 1, 1]]);
  assert.deepEqual(keep.removed, []);
  const p = planGenre(s, g("techno"), { keepRanges: true, removeOthers: true });
  const r = applyTune(s, p);
  assert.deepEqual(r.routesOff, ["Clap flash"]);
  assert.deepEqual(s.routes.map((x) => x.enabled), [false, true]);
  assert.ok(missingBand(s, "clap") && !missingBand(s, "kick") && !missingBand(s, "kick.level") && !missingBand(s, "energy"));
});

test("deleting a band switches off the routes that used it", () => {
  const s = show();
  s.routes = [route("a", "Bass wobble", "x", { enabled: true, source: "bass.level" }), route("b", "Off already", "x", { enabled: false, source: "bass" }), route("c", "Beat", "x", { enabled: true, source: "beat" })];
  assert.deepEqual(removeBand(s, "bass"), ["Bass wobble"]);
  assert.deepEqual(s.routes.map((x) => x.enabled), [false, false, true]);
});

test("the current genre is recognised", () => {
  const s = show();
  assert.equal(currentGenre(s), null);
  applyGenre(s, g("rock"));
  assert.equal(currentGenre(s)?.id, "rock");
  s.audio.bands[1].sensitivity = 0.99;
  assert.equal(currentGenre(s), null);
});

test("a genre's settings are copied, never shared", () => {
  const a = show(), b = show();
  applyGenre(a, g("rock"));
  applyGenre(b, g("rock"));
  a.audio.bands[0].ranges[0].lo_hz = 1;
  assert.notEqual(b.audio.bands[0].ranges[0].lo_hz, 1);
  assert.notEqual(BASE_BANDS.kick.ranges[0].lo_hz, 1);
});
