// Development only: a canned AI design for the demo show, so the Design view can be
// previewed (and screenshotted) in a browser without an AI provider.
import type { DesignResult } from "../lib/design";
import type { Route, RouteAction } from "../lib/types";

const r = (id: string, name: string, group: string, source: string, action: RouteAction, extra: Partial<Route> = {}): Route => ({
  id, name, group, enabled: group === "Warm-up", source, rooms: [], fixtures: [], groups: ["grp-pars"], channel: "", focus: "",
  falloff: false, action, cooldown_ms: 0, every: 1, probability: 1, ...extra,
});

export function demoDesign(): DesignResult {
  const routes = [
    r("ai-r-demo1", "Slow sunset wave", "Warm-up", "beat", { type: "wave", palette: "ai-pal-demo1", axis: "x", beats: 16, width: 0.5, base: 40 }),
    r("ai-r-demo2", "Hat sparkle", "Warm-up", "hat", { type: "twinkle", palette: "pal-ice", amount: 0.3, decay_ms: 400, base: 20 }, { groups: ["grp-back"] }),
    r("ai-r-demo3", "Energy lifts brightness", "Build", "energy", { type: "modulate", min: 30, max: 255, smoothing: 0.7 }, { channel: "@intensity" }),
    r("ai-r-demo4", "Rings out from the floor", "Build", "beat", { type: "wave", palette: "ai-pal-demo2", axis: "radial", beats: 2, width: 0.3, base: 20 }, { focus: "focus-floor" }),
    r("ai-r-demo5", "Kick colour hits", "Drop", "kick", { type: "palette", palette: "ai-pal-demo2", mode: "rings", fade_ms: 120, brightness: 255 }, { focus: "focus-floor", falloff: true }),
    r("ai-r-demo6", "Snare strobe", "Drop", "snare", { type: "strobe", rate: "4", hz: 12, duty: 0.25, burst_ms: 220, threshold: 0.6 }),
    r("ai-r-demo7", "Laser peak looks", "Drop", "kick", { type: "look", set: "", include_defaults: true, tags: ["peak"], mode: "random", hold_ms: 1500, heads: [] }, { groups: ["grp-laser"], every: 2 }),
    r("ai-r-demo8", "Fog on the drop", "Drop", "bar", { type: "flash", on: 255, off: 0, decay_ms: 2000 }, { groups: [], fixtures: ["fx-fog"], channel: "@fog", cooldown_ms: 45000 }),
    r("ai-r-demo9", "Vocals breathe", "Breakdown", "vocals.level", { type: "modulate", min: 10, max: 180, smoothing: 0.85 }, { channel: "@intensity" }),
  ];
  const scene = (on: string[]) => Object.fromEntries(routes.map((x) => [x.id, on.includes(x.group)]));
  return {
    warnings: ["Laser peak looks: no looks tagged \"peak\" on Laser head P2, it uses P1 only"],
    plan: {
      title: "Techno night",
      summary: "Four scenes: Warm-up (slow sunset wave, sparkles on the hats), Build (energy lifts brightness, rings pulse out from the dance floor), Drop (kick colour rings, snare strobe, laser peak looks, fog on the first bar) and Breakdown (lights breathe with the vocals).\nOn the APC mini mk2 the bottom row of pads (notes 0–3) selects the scenes, pad 7 is a held strobe, fader 9 (CC 56) is master and faders 1–2 (CC 48–49) set kick and snare sensitivity.",
      sensitivity: 0.5,
      bands: [{ id: "kick", name: "Kick", enabled: true, ranges: [{ lo_hz: 35, hi_hz: 150, weight: 1 }], detect: "transient", level_threshold: 0.6, gain_db: 0, sensitivity: 0.6, min_gap_ms: 190, release: 0.85, color: "#ff5c6c" }],
      palettes: [
        { id: "ai-pal-demo1", name: "Ember", colors: ["#ff3a00", "#ff8a00", "#ffc04d"] },
        { id: "ai-pal-demo2", name: "Cold steel", colors: ["#ffffff", "#2a6bff", "#ff0033"] },
      ],
      routes,
      presets: [
        { id: "ai-p-demo1", name: "Warm-up", color: "#ff8a00", values: [], routes: scene(["Warm-up"]), exclusive: false },
        { id: "ai-p-demo2", name: "Build", color: "#ffd27a", values: [], routes: scene(["Build"]), exclusive: false },
        { id: "ai-p-demo3", name: "Drop", color: "#ff3355", values: [], routes: scene(["Drop", "Build"]), exclusive: false },
        { id: "ai-p-demo4", name: "Breakdown", color: "#4fd1ff", values: [], routes: scene(["Breakdown"]), exclusive: false },
      ],
      controls: [
        ...["ai-p-demo1", "ai-p-demo2", "ai-p-demo3", "ai-p-demo4"].map((p, i) => ({
          id: `ai-c-demo${i + 1}`, name: ["Warm-up", "Build", "Drop", "Breakdown"][i], enabled: true,
          trigger: { source: "midi" as const, kind: "note" as const, channel: 0, number: i, key: "", mode: "on" as const },
          action: { type: "preset" as const, preset: p },
        })),
        { id: "ai-c-demo5", name: "Strobe (hold)", enabled: true, trigger: { source: "midi", kind: "note", channel: 0, number: 7, key: "", mode: "momentary" }, action: { type: "routes", routes: ["ai-r-demo6"] } },
        { id: "ai-c-demo6", name: "Master", enabled: true, trigger: { source: "midi", kind: "cc", channel: 0, number: 56, key: "", mode: "fader" }, action: { type: "master" } },
        { id: "ai-c-demo7", name: "Kick sensitivity", enabled: true, trigger: { source: "midi", kind: "cc", channel: 0, number: 48, key: "", mode: "fader" }, action: { type: "band_sensitivity", band: "kick" } },
        { id: "ai-c-demo8", name: "Drop (key)", enabled: true, trigger: { source: "key", kind: "note", channel: 0, number: 0, key: "3", mode: "on" }, action: { type: "preset", preset: "ai-p-demo3" } },
      ],
    },
  };
}
