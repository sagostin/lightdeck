import { test } from "node:test";
import assert from "node:assert/strict";
import { addCopy, compare, describeDiff, mergeInto, saveToLibrary } from "../src/lib/library.ts";
import type { Look, Profile } from "../src/lib/types.ts";

const look = (id: string, name = id, tags: string[] = []): Look => ({ id, name, head: "", values: { pattern: 10 }, tags, rating: 0, sets: [], builtin: false, motions: {} });
const laser = (looks: Look[] = [look("a"), look("b")]): Profile => ({
  id: "laser", manufacturer: "Gen", model: "L2600", kind: "laser", notes: "",
  channels: [{ key: "pattern", name: "Pattern", group: "pattern", default: 0, capabilities: [{ min: 0, max: 9, label: "Auto", color: null, tags: ["blocked"] }], values: [] }],
  modes: [{ name: "m", channels: ["pattern"], heads: [] }], looks, look_sets: [{ id: "set1", name: "Peak", color: "#f00" }],
});

test("identical types compare as the same", () => {
  const d = compare(laser(), laser());
  assert.ok(d.same);
  assert.equal(describeDiff(d, "library"), "Same as the library");
});

test("compare counts looks each side has, and definition changes", () => {
  const mine = laser([look("a"), look("b", "Changed"), look("mine")]);
  const lib = laser([look("a"), look("b"), look("new1"), look("new2")]);
  lib.channels[0].capabilities[0].tags = []; // someone un-blocked a range
  const d = compare(mine, lib);
  assert.deepEqual([d.definition, d.incoming, d.local, d.changed, d.same], [true, 2, 1, 1, false]);
  assert.equal(describeDiff(d, "library"), "channels, tags or details differ · 2 looks only in the library · 1 only here · 1 changed");
  assert.ok(!compare(laser(), { ...laser(), id: "other-id" }).definition, "the id isn't part of the definition");
});

test("merging brings new and changed looks and keeps local ones", () => {
  const project = laser([look("a"), look("b", "Old name"), look("venue-only")]);
  const lib = laser([look("a"), look("b", "New name", ["peak"]), look("lib-only")]);
  lib.look_sets.push({ id: "set2", name: "Chill", color: "#0ff" });
  lib.channels[0].name = "Gobo";
  const r = mergeInto(project, lib);
  assert.deepEqual(r, { added: 1, updated: 1, kept: 1, definition: true });
  assert.deepEqual(project.looks.map((l) => [l.id, l.name]), [["a", "a"], ["b", "New name"], ["venue-only", "venue-only"], ["lib-only", "lib-only"]]);
  assert.equal(project.channels[0].name, "Gobo");
  assert.deepEqual(project.look_sets.map((s) => s.id), ["set1", "set2"]);
  assert.equal(project.id, "laser");
  // merging again changes nothing
  assert.deepEqual(mergeInto(project, lib), { added: 0, updated: 0, kept: 1, definition: false });
  // and the result doesn't share objects with the source
  project.looks[1].tags.push("edited");
  assert.deepEqual(lib.looks[1].tags, ["peak"]);
});

test("saving to the library adds new types and merges existing ones", () => {
  const first = saveToLibrary([], laser());
  assert.equal(first.library.length, 1);
  assert.equal(first.result, null);
  const project = laser([look("a"), look("c")]);
  const second = saveToLibrary(first.library, project);
  assert.deepEqual(second.library[0].looks.map((l) => l.id), ["a", "b", "c"], "library-only look b is kept");
  assert.equal(first.library[0].looks.length, 2, "the old library array is untouched");
  project.looks[0].name = "edited after saving";
  assert.equal(second.library[0].looks[0].name, "a");
});

test("adding a copy renames when the id is taken", () => {
  const list = [laser()];
  const c = addCopy(list, laser(), "x1");
  assert.deepEqual([c.id, c.model, list.length], ["laser-x1", "L2600 (copy)", 2]);
  const fresh = addCopy([], laser());
  assert.deepEqual([fresh.id, fresh.model], ["laser", "L2600"]);
});
