// A small show for the UI tests: four pars in two groups, the default bands and palettes.
import type { BandDef, Route, Show } from "../src/lib/types.ts";

const band = (id: string, name: string, lo: number, hi: number, detect: BandDef["detect"]): BandDef => ({
  id, name, enabled: true, ranges: [{ lo_hz: lo, hi_hz: hi, weight: 1 }], detect, level_threshold: 0.6,
  gain_db: 0, sensitivity: 0.5, min_gap_ms: 140, release: 0.85, color: "#ffffff",
});

export function show(): Show {
  return {
    version: 3, profiles: [], rooms: [], outputs: [], zigbee: {} as Show["zigbee"],
    fixtures: ["p1", "p2", "p3", "p4"].map((id) => ({ id, name: id }) as Show["fixtures"][number]),
    groups: [
      { id: "front", name: "Front", color: "#fff", fixtures: ["p1", "p2"] },
      { id: "back", name: "Back", color: "#fff", fixtures: ["p3", "p4"] },
    ],
    routes: [route("mine", "My route", "Mine")],
    presets: [{ id: "my-preset", name: "Mine", color: "#fff", values: [], routes: { mine: true }, exclusive: false }],
    controls: [{ id: "my-control", name: "Mine", enabled: true, trigger: { source: "key", kind: "note", channel: 0, number: 0, key: "m", mode: "toggle" }, action: { type: "blackout" } }],
    palettes: [{ id: "pal-club", name: "Club", colors: ["#ff0040", "#0040ff"] }],
    audio: {
      device: null, gain_db: 0, sensitivity: 0.5,
      bands: [band("kick", "Kick", 35, 150, "transient"), band("bass", "Bass", 60, 250, "level"), band("snare", "Snare", 150, 5000, "transient"),
        band("vocals", "Vocals", 300, 3000, "level"), band("hat", "Hi-hat", 6000, 16000, "transient")],
    },
    midi: { enabled: false, inputs: [], feedback: true, feedback_style: "" },
    active_room: null,
  };
}

export function route(id: string, name: string, group: string, extra: Partial<Route> = {}): Route {
  return {
    id, name, group, enabled: false, source: "kick", rooms: [], fixtures: [], groups: ["front"], channel: "@intensity", focus: "",
    falloff: false, action: { type: "flash", on: 255, off: 0, decay_ms: 100 }, cooldown_ms: 0, every: 1, probability: 1, ...extra,
  };
}
