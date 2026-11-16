import { test } from "node:test";
import assert from "node:assert/strict";
import { GENRES, applyGenre } from "../src/lib/genres.ts";
import { BAND_PRESETS } from "../src/lib/types.ts";
import { show } from "./fixtures.ts";

const STANDARD = ["kick", "bass", "snare", "vocals", "hat"];

test("every genre is well formed", () => {
  const ids = new Set<string>();
  for (const g of GENRES) {
    assert.ok(!ids.has(g.id), `duplicate genre ${g.id}`);
    ids.add(g.id);
    assert.ok(g.bpm[0] > 40 && g.bpm[0] < g.bpm[1] && g.bpm[1] < 220, `${g.id} BPM range`);
    assert.ok(g.sensitivity > 0 && g.sensitivity <= 1, `${g.id} sensitivity`);
    assert.ok(g.style.length > 40, `${g.id} needs a lighting style note for the AI`);
    for (const [id, patch] of Object.entries(g.bands)) {
      assert.ok(STANDARD.includes(id), `${g.id} patches unknown band ${id}`);
      assert.ok(!("id" in patch) && !("ranges" in patch), `${g.id}/${id} must not change ids or ranges`);
      if (patch.sensitivity !== undefined) assert.ok(patch.sensitivity >= 0 && patch.sensitivity <= 1);
      if (patch.release !== undefined) assert.ok(patch.release >= 0 && patch.release <= 0.98);
      if (patch.min_gap_ms !== undefined) assert.ok(patch.min_gap_ms >= 20 && patch.min_gap_ms <= 2000);
    }
    for (const a of g.add ?? []) assert.ok(BAND_PRESETS.some((p) => p.name === a.preset), `${g.id} adds unknown preset ${a.preset}`);
  }
  assert.equal(GENRES.length, 10);
});

test("applying a genre patches bands by id and keeps their ranges", () => {
  const s = show();
  const before = structuredClone(s.audio.bands);
  const dnb = GENRES.find((g) => g.id === "dnb")!;
  const changed = applyGenre(s, dnb);
  assert.deepEqual(changed, ["Kick", "Bass", "Snare", "Hi-hat"]);
  const snare = s.audio.bands.find((b) => b.id === "snare")!;
  assert.equal(snare.sensitivity, 0.6);
  assert.equal(snare.min_gap_ms, 120);
  assert.deepEqual(s.audio.bands.map((b) => [b.id, b.ranges]), before.map((b) => [b.id, b.ranges]));
  assert.equal(s.audio.bands[0].id, "kick", "kick stays first (it drives the beat clock)");
  // vocals untouched
  assert.deepEqual(s.audio.bands.find((b) => b.id === "vocals"), before.find((b) => b.id === "vocals"));
});

test("genres add their extra bands once, at the end", () => {
  const s = show();
  const house = GENRES.find((g) => g.id === "house")!;
  assert.ok(applyGenre(s, house).includes("+Clap"));
  assert.equal(s.audio.bands.at(-1)!.id, "clap");
  assert.equal(s.audio.bands.at(-1)!.min_gap_ms, 200, "the genre's patch applies to the added band");
  assert.ok(!applyGenre(s, house).includes("+Clap"));
  assert.equal(s.audio.bands.filter((b) => b.id === "clap").length, 1);
  const dub = GENRES.find((g) => g.id === "dubstep")!;
  applyGenre(s, dub);
  assert.equal(s.audio.bands.at(-1)!.id, "sub_bass");
});

test("a genre's patch is copied, not shared between shows", () => {
  const a = show(), b = show();
  const rock = GENRES.find((g) => g.id === "rock")!;
  applyGenre(a, rock);
  applyGenre(b, rock);
  a.audio.bands.find((x) => x.id === "hat")!.name = "Changed";
  assert.equal(b.audio.bands.find((x) => x.id === "hat")!.name, "Cymbals");
  assert.equal(rock.bands.hat.name, "Cymbals");
});

test("genres only touch bands the show has", () => {
  const s = show();
  s.audio.bands = s.audio.bands.filter((b) => b.id === "kick");
  const changed = applyGenre(s, GENRES.find((g) => g.id === "techno")!);
  assert.deepEqual(changed, ["Kick"]);
  assert.equal(s.audio.sensitivity, 0.5);
});
