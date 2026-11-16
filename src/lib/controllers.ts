// MIDI controller profiles and auto-mapping. A profile lists a controller's pads and
// faders in the order they're handed out; auto-mapping assigns scenes, route groups,
// strobe, fog and blackout to pads, and master, group dimmers and band sensitivity to
// faders, from whatever the show has now. Re-run it after changing scenes or the rig.
import type { ControlMapping, ControllerInput, ControllerProfile, Preset, Route, Show } from "./types";
import { clone } from "./clone";
import { activeFixtures, groupMembers, hasType } from "./rig";

const notes = (nums: number[], label: (i: number) => string): ControllerInput[] => nums.map((number, i) => ({ kind: "note", number, label: label(i) }));
const ccs = (nums: number[], label: (i: number) => string): ControllerInput[] => nums.map((number, i) => ({ kind: "cc", number, label: label(i) }));
const range = (a: number, n: number) => Array.from({ length: n }, (_, i) => a + i);

// Grid controllers hand out pads from the top row down, left to right.
const apcGrid = range(0, 8).flatMap((row) => range((7 - row) * 8, 8));
const lpGrid = range(0, 8).flatMap((row) => range((8 - row) * 10 + 1, 8));

export const BUILTIN_CONTROLLERS: ControllerProfile[] = [
  {
    id: "akai-apc-mini-mk2", name: "Akai APC mini mk2", channel: 0, builtin: true,
    notes: "8×8 pad grid (top row first), then the track and scene buttons. Faders 1–8 and the master fader.",
    pads: [...notes(apcGrid, (i) => `Row ${Math.floor(i / 8) + 1} pad ${(i % 8) + 1}`),
      ...notes(range(100, 8), (i) => `Track button ${i + 1}`), ...notes(range(112, 8), (i) => `Scene button ${i + 1}`)],
    faders: [...ccs([56], () => "Master fader"), ...ccs(range(48, 8), (i) => `Fader ${i + 1}`)],
  },
  {
    id: "novation-launchpad", name: "Novation Launchpad X / Mini mk3 (Programmer mode)", channel: 0, builtin: true,
    notes: "Put the Launchpad in Programmer mode. 8×8 grid, top row first. No faders.",
    pads: notes(lpGrid, (i) => `Row ${Math.floor(i / 8) + 1} pad ${(i % 8) + 1}`),
    faders: [],
  },
  {
    id: "korg-nanokontrol2", name: "Korg nanoKONTROL2 (CC mode)", channel: 0, builtin: true,
    notes: "Factory CC mode. Buttons are S, M and R per channel, then transport. Faders 1–8, then knobs 1–8.",
    pads: [...ccs(range(32, 8), (i) => `S${i + 1}`), ...ccs(range(48, 8), (i) => `M${i + 1}`), ...ccs(range(64, 8), (i) => `R${i + 1}`),
      ...ccs([41, 42, 45, 46, 43, 44], (i) => ["Play", "Stop", "Record", "Cycle", "Rewind", "Fast forward"][i])],
    faders: [...ccs(range(0, 8), (i) => `Fader ${i + 1}`), ...ccs(range(16, 8), (i) => `Knob ${i + 1}`)],
  },
  {
    id: "generic-pads", name: "Generic pad controller (notes 36–51, CC 1–8)", channel: 0, builtin: true,
    notes: "Typical drum-pad layout (MPD, LPD, Arturia…). If your pads send other numbers, copy this profile and Learn them.",
    pads: notes(range(36, 16), (i) => `Pad ${i + 1}`),
    faders: ccs(range(1, 8), (i) => `Knob ${i + 1}`),
  },
];

export interface AutomapOptions {
  scenes: boolean; groups: boolean; strobe: boolean; fog: boolean; blackout: boolean;
  master: boolean; dimmers: boolean; bands: boolean; keys: boolean;
}
export const DEFAULT_AUTOMAP: AutomapOptions = { scenes: true, groups: true, strobe: true, fog: true, blackout: true, master: true, dimmers: true, bands: true, keys: true };

/** Auto-mapped controls have ids starting with this, so re-mapping replaces only them. */
export const MAP_PREFIX = "map-";

export interface Assignment { control: ControlMapping; input: string }
export interface AutomapResult { assigned: Assignment[]; unmapped: string[] }

type Want = { name: string; mode: ControlMapping["trigger"]["mode"]; action: ControlMapping["action"]; key?: string };

/**
 * Work out a mapping for `profile`. Routes and presets default to the show's; pass others
 * to map a plan before it's added. Triggers your own (non-auto) controls use are skipped.
 */
export function automap(show: Show, profile: ControllerProfile | null, opts: AutomapOptions,
  routes: Route[] = show.routes, presets: Preset[] = show.presets): AutomapResult {
  const pads: Want[] = [], faders: Want[] = [];
  if (opts.scenes) for (const p of presets) pads.push({ name: p.name, mode: "on", action: { type: "preset", preset: p.id } });
  if (opts.groups) {
    const groups = [...new Set(routes.map((r) => r.group.trim()).filter(Boolean))];
    for (const g of groups) pads.push({ name: `${g} on/off`, mode: "toggle", action: { type: "route_group", group: g } });
  }
  const strobes = routes.filter((r) => r.action.type === "strobe").map((r) => r.id);
  if (opts.strobe && strobes.length) pads.push({ name: "Strobe (hold)", mode: "momentary", action: { type: "routes", routes: strobes }, key: "s" });
  const foggers = activeFixtures(show).filter((f) => hasType(show, f, "fog")).map((f) => f.id);
  if (opts.fog && foggers.length) pads.push({ name: "Fog (hold)", mode: "momentary", action: { type: "channel_value", target: { rooms: [], groups: [], fixtures: foggers }, channel: "@fog", min: 0, max: 255 }, key: "f" });
  if (opts.blackout) pads.push({ name: "Blackout", mode: "toggle", action: { type: "blackout" } });
  if (opts.blackout) pads.push({ name: "Release manual", mode: "on", action: { type: "release_manual" } });
  if (opts.master) faders.push({ name: "Master", mode: "fader", action: { type: "master" } });
  if (opts.dimmers) for (const g of show.groups) {
    if (groupMembers(show, g.id).some((f) => hasType(show, f, "intensity")))
      faders.push({ name: `${g.name} brightness`, mode: "fader", action: { type: "channel_value", target: { rooms: [], groups: [g.id], fixtures: [] }, channel: "@intensity", min: 0, max: 255 } });
  }
  if (opts.bands) for (const b of show.audio.bands.filter((b) => b.enabled)) faders.push({ name: `${b.name} sensitivity`, mode: "fader", action: { type: "band_sensitivity", band: b.id } });

  // Triggers already used by your own controls.
  const own = show.controls.filter((c) => !c.id.startsWith(MAP_PREFIX));
  const used = new Set(own.map((c) => trigKey(c.trigger.source, c.trigger.kind, c.trigger.number, c.trigger.key)));
  const assigned: Assignment[] = [];
  const unmapped: string[] = [];
  let n = 0;
  const give = (wants: Want[], inputs: ControllerInput[]) => {
    const free = inputs.filter((i) => !used.has(trigKey("midi", i.kind, i.number, "")));
    wants.forEach((w, i) => {
      const input = free[i];
      if (!input) { unmapped.push(w.name); return; }
      assigned.push({
        input: input.label,
        control: {
          id: `${MAP_PREFIX}${++n}`, name: w.name, enabled: true, action: clone(w.action),
          // CC buttons (nanoKONTROL) act as buttons unless the action is a fader.
          trigger: { source: "midi", kind: input.kind, channel: profile!.channel, number: input.number, key: "", mode: w.mode },
        },
      });
    });
  };
  if (profile) { give(pads, profile.pads); give(faders, profile.faders); }
  if (opts.keys) {
    const scenes = pads.filter((w) => w.action.type === "preset").slice(0, 10);
    const keyed: [Want, string][] = [...scenes.map((w, i): [Want, string] => [w, String((i + 1) % 10)]), ...pads.filter((w) => w.key).map((w): [Want, string] => [w, w.key!])];
    for (const [w, key] of keyed) {
      if (used.has(trigKey("key", "", 0, key))) continue;
      assigned.push({ input: `Key ${key.toUpperCase()}`, control: {
        id: `${MAP_PREFIX}k${++n}`, name: w.name, enabled: true, action: clone(w.action),
        trigger: { source: "key", kind: "note", channel: 0, number: 0, key, mode: w.mode === "toggle" ? "toggle" : w.mode },
      } });
    }
  }
  return { assigned, unmapped };
}

function trigKey(source: string, kind: string, number: number, key: string) {
  return source === "key" ? `key:${key}` : `midi:${kind}:${number}`;
}

/** Replace earlier auto-mapped controls with a new mapping. */
export function applyAutomap(show: Show, result: AutomapResult) {
  show.controls = [...show.controls.filter((c) => !c.id.startsWith(MAP_PREFIX)), ...result.assigned.map((a) => a.control)];
  if (result.assigned.some((a) => a.control.trigger.source === "midi")) show.midi.enabled = true;
}

/** Add a learned MIDI event to a profile being built. Returns false if it's already there. */
export function learnInput(p: ControllerProfile, ev: { kind: string; number: number }, as: "pads" | "faders"): boolean {
  if (ev.kind !== "note" && ev.kind !== "cc") return false;
  const all = [...p.pads, ...p.faders];
  if (all.some((i) => i.kind === ev.kind && i.number === ev.number)) return false;
  const list = p[as];
  list.push({ kind: ev.kind, number: ev.number, label: `${as === "pads" ? "Pad" : "Fader"} ${list.length + 1}` });
  return true;
}

/** Does a control point at something that no longer exists? */
export function isDead(show: Show, c: ControlMapping): boolean {
  const a = c.action;
  switch (a.type) {
    case "preset": return !show.presets.some((p) => p.id === a.preset);
    case "routes": return !a.routes.length || a.routes.some((r) => !show.routes.some((x) => x.id === r));
    case "route_group": return !show.routes.some((r) => r.group.trim() === a.group.trim());
    case "band_sensitivity": return !show.audio.bands.some((b) => b.id === a.band);
    case "bands": return a.bands.some((b) => !show.audio.bands.some((x) => x.id === b));
    case "channel_value": return [...a.target.groups.filter((g) => !show.groups.some((x) => x.id === g)), ...a.target.fixtures.filter((f) => !show.fixtures.some((x) => x.id === f))].length > 0;
    default: return false;
  }
}

/**
 * Compare the current auto-mapping with what mapping now would give: `changed` when scenes,
 * strobe/fog, groups or bands were added or removed since (ignoring which pad each is on).
 */
export function mappingStatus(show: Show, preview: AutomapResult) {
  const current = show.controls.filter((c) => c.id.startsWith(MAP_PREFIX));
  const sig = (c: ControlMapping) => `${c.trigger.source}|${c.name}|${JSON.stringify(c.action)}`;
  const a = current.map(sig).sort(), b = preview.assigned.map((x) => sig(x.control)).sort();
  return {
    mapped: current.length,
    dead: current.filter((c) => isDead(show, c)).length,
    changed: current.length > 0 && JSON.stringify(a) !== JSON.stringify(b),
  };
}
