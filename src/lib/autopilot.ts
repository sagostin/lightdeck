// Autopilot helpers for the UI: what the sections mean, which scenes cover them, and an
// offline guess of each scene's sections from its name and what it switches on.
// The autopilot itself runs in the engine (src-tauri/src/autopilot.rs).
import type { AutopilotPlan, AutopilotSettings, Preset, Route, Section, Show } from "./types";
import { clone } from "./clone";

export const SECTIONS: { id: Section; name: string; color: string; hint: string }[] = [
  { id: "breakdown", name: "Breakdown", color: "#4fb3ff", hint: "Quiet parts with no kick: intros, breakdowns, outros" },
  { id: "groove", name: "Groove", color: "#45d39a", hint: "The steady main part with a kick" },
  { id: "build", name: "Build", color: "#f2b54a", hint: "Loudness rising towards a drop" },
  { id: "drop", name: "Drop", color: "#ff5c6c", hint: "The loudest part, full kick" },
];
export const sectionInfo = (id: string) => SECTIONS.find((s) => s.id === id);

export const DEFAULT_AUTOPILOT: AutopilotSettings = { min_bars: 8, max_bars: 32, drop_bars: 2, pause_s: 60 };

/** The show's autopilot settings, filled in if the show predates them. */
export function autopilotSettings(show: Show): AutopilotSettings {
  show.autopilot ??= { ...DEFAULT_AUTOPILOT };
  return show.autopilot;
}

/** Scenes per section. */
export function coverage(show: Show): Record<Section, Preset[]> {
  const out = { breakdown: [], groove: [], build: [], drop: [] } as Record<Section, Preset[]>;
  for (const p of show.presets) for (const s of p.sections ?? []) out[s]?.push(p);
  return out;
}

export function toggleSection(p: Preset, s: Section) {
  const cur = new Set(p.sections ?? []);
  if (cur.has(s)) cur.delete(s); else cur.add(s);
  p.sections = SECTIONS.map((x) => x.id).filter((x) => cur.has(x));
}

const NAMES: [Section[], RegExp][] = [
  [[], /\b(test|blackout|black ?out|off|house ?lights?|work ?lights?|static)\b/i],
  [["drop"], /drop|peak|climax|hype|banger|chorus|full ?on|big/i],
  [["build"], /build|rise|riser|tension|lift|ramp/i],
  [["breakdown"], /break|chill|calm|ambient|intro|outro|glow|drift|soft|slow|lounge|sunset|mellow/i],
  [["breakdown", "groove"], /warm/i],
  [["groove"], /groove|main|verse|steady|dance|party|vibe/i],
];

const HIT_SOURCES = new Set(["beat"]);
const isHitSource = (show: Show, src: string) => HIT_SOURCES.has(src) || show.audio.bands.some((b) => b.id === src && b.detect === "transient");

/** A guess at a scene's sections, with why. Offline: name first, then what the scene switches on. */
export function guessSections(show: Show, p: Preset): { sections: Section[]; why: string } {
  for (const [sections, re] of NAMES) {
    const m = p.name.match(re);
    if (m) return { sections, why: sections.length ? `named "${m[0]}"` : "looks like a utility scene" };
  }
  const on: Route[] = show.routes.filter((r) => p.routes[r.id] === true);
  if (!on.length) return { sections: [], why: p.values.length ? "a static look, not switched by music" : "switches nothing on" };
  const strobe = on.filter((r) => r.action.type === "strobe").length;
  const fog = on.some((r) => r.channel === "@fog");
  const peak = on.some((r) => r.action.type === "look" && r.action.tags.includes("peak"));
  const kickHits = on.filter((r) => isHitSource(show, r.source) && r.source !== "beat").length;
  const energy = on.some((r) => r.source === "energy");
  const calm = on.every((r) => !isHitSource(show, r.source) || ["wave", "twinkle"].includes(r.action.type));
  if (strobe && (fog || peak || kickHits >= 2)) return { sections: ["drop"], why: "strobe with kick hits, fog or peak looks" };
  if (energy) return { sections: ["build"], why: "follows the energy" };
  if (calm) return { sections: ["breakdown"], why: "slow effects, no hits" };
  if (strobe) return { sections: ["drop"], why: "has strobe" };
  return { sections: ["groove"], why: "hit-driven effects" };
}

/** Guess sections for every scene that has none (or all, with `overwrite`). Returns what changed. */
export function guessAll(show: Show, overwrite = false): { name: string; sections: Section[]; why: string }[] {
  const changed = [];
  for (const p of show.presets) {
    if (!overwrite && p.sections?.length) continue;
    const g = guessSections(show, p);
    if (JSON.stringify(g.sections) === JSON.stringify(p.sections ?? [])) continue;
    p.sections = g.sections;
    changed.push({ name: p.name, ...g });
  }
  return changed;
}

/** Apply an AI suggestion; returns what's needed to undo it. */
export function applyAutopilotPlan(show: Show, plan: AutopilotPlan) {
  const before = { settings: clone(autopilotSettings(show)), sections: Object.fromEntries(show.presets.map((p) => [p.id, [...(p.sections ?? [])]])) };
  show.autopilot = clone(plan.settings);
  for (const p of show.presets) if (plan.sections[p.id]) p.sections = [...plan.sections[p.id]];
  return before;
}

export function undoAutopilotPlan(show: Show, before: ReturnType<typeof applyAutopilotPlan>) {
  show.autopilot = before.settings;
  for (const p of show.presets) if (before.sections[p.id]) p.sections = before.sections[p.id] as Section[];
}

/** What the autopilot can't play yet, in words; "" when every section has a scene. */
export function coverageNote(show: Show): string {
  const c = coverage(show);
  const tagged = show.presets.filter((p) => p.sections?.length).length;
  if (!tagged) return "No scenes are tagged yet. Tag them below, or use Guess or Suggest with AI.";
  const missing = SECTIONS.filter((s) => !c[s.id].length).map((s) => s.name.toLowerCase());
  return missing.length ? `No scene for ${missing.join(", ")}: the autopilot uses the nearest section instead.` : "";
}
