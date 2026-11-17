import { test } from "node:test";
import assert from "node:assert/strict";
import { groupLooks, lookMainKey } from "../src/lib/looks.ts";
import type { Look, Profile } from "../src/lib/types.ts";

const profile = {
  id: "laser", channels: [
    { key: "p1_gallery", name: "P1 Gallery", group: "pattern", default: 0, capabilities: [], values: [] },
    { key: "p1_pattern", name: "P1 Pattern", group: "pattern", default: 0, capabilities: [{ min: 1, max: 255, label: "Pattern select", tags: [] }], values: [{ value: 48, label: "Double line", tags: [], rating: 0 }] },
    { key: "p1_zoom", name: "P1 Zoom", group: "beam", default: 0, capabilities: [], values: [] },
  ],
} as unknown as Profile;
const look = (name: string, values: Record<string, number>, rating = 0): Look =>
  ({ id: name, name, head: "P1", values, motions: {}, tags: [], rating, sets: [], builtin: false });

test("looks group under the pattern value they're built on, in pattern order", () => {
  assert.equal(lookMainKey(profile, look("z", { p1_zoom: 9, p1_pattern: 3 })), "p1_pattern", "the pattern channel wins over secondary options");
  const groups = groupLooks([
    look("Zoomed", { p1_pattern: 48, p1_zoom: 100 }),
    look("Plain line", { p1_pattern: 48 }, 4),
    look("Square", { p1_pattern: 12 }),
    look("Gallery beam", { p1_gallery: 200 }),
  ].map((l) => ({ profile, look: l })));
  assert.deepEqual(groups.map((g) => [g.channel, g.value, g.label]), [["P1 Gallery", 200, ""], ["P1 Pattern", 12, "Pattern select"], ["P1 Pattern", 48, "Double line"]]);
  assert.deepEqual(groups[2].looks.map((l) => l.name), ["Plain line", "Zoomed"], "best rated first");
});
