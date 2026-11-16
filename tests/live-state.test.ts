// The UI passes Svelte's live state (reactive Proxies) into the shared libraries. Proxies
// can't be structuredClone'd, so every library function must copy data another way.
import { test } from "node:test";
import assert from "node:assert/strict";
import { GENRES, applyGenre, applyTune, currentGenre, planGenre, removeBand } from "../src/lib/genres.ts";
import { compare, mergeInto, saveToLibrary, addCopy } from "../src/lib/library.ts";
import { applyPlan } from "../src/lib/design.ts";
import { automap, applyAutomap, BUILTIN_CONTROLLERS, DEFAULT_AUTOMAP, mappingStatus } from "../src/lib/controllers.ts";
import { buildTemplate } from "../src/lib/templates.ts";
import { buildScenes, newBlock } from "../src/lib/blocks.ts";
import type { Show } from "../src/lib/types.ts";
import { clubRig } from "./rig.ts";

/** Wrap an object graph in Proxies, like Svelte's $state does. */
function live<T extends object>(o: T): T {
  return new Proxy(o, {
    get(t, k, r) { const v = Reflect.get(t, k, r); return v && typeof v === "object" ? live(v) : v; },
  });
}

test("a Proxy really can't be structuredClone'd (so these tests mean something)", () => {
  assert.throws(() => structuredClone(live({ a: 1 })));
});

test("genre tuning works on live state", () => {
  const show = live(clubRig());
  assert.equal(currentGenre(show), null);
  const p = planGenre(show, GENRES[0], { keepRanges: true, removeOthers: true });
  applyTune(show, p);
  applyGenre(show, GENRES[1]);
  removeBand(show, "kick");
});

test("library, designs and auto-mapping work on live state", () => {
  const show = live(clubRig({ looksTagged: true }));
  const laser = show.profiles[0];
  assert.ok(compare(laser, laser).same);
  mergeInto(laser, laser);
  saveToLibrary([], laser);
  addCopy(show.profiles, laser);
  const plan = buildTemplate(show, "club", null);
  const keep = new Set([...plan.routes, ...plan.presets].map((x) => x.id));
  applyPlan(show, live(plan), keep, true);
  buildScenes(show, [{ name: "S", blocks: [newBlock("wave")] }], { genre: null, title: "t" });
  const r = automap(show, BUILTIN_CONTROLLERS[0], DEFAULT_AUTOMAP);
  applyAutomap(show as Show, r);
  mappingStatus(show, r);
});
