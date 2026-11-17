// Development only: a demo show plus a fake audio feed, for README screenshots and
// trying the UI without hardware. Open http://localhost:1420/?demo (add &view=<screen>).
import { app } from "../lib/store.svelte";
import type { BandOut, Fixture, Frame, Profile, Route, RouteAction, Section, Show } from "../lib/types";

const PAR: Profile = {
  id: "generic-rgbw-par",
  manufacturer: "Generic",
  model: "RGBW LED Par 6ch",
  kind: "par",
  notes: "",
  looks: [],
  look_sets: [],
  modes: [{ name: "6 Channel", channels: ["dimmer", "red", "green", "blue", "white", "strobe"], heads: [] }],
  channels: [
    { key: "dimmer", name: "Dimmer", group: "intensity", default: 0, capabilities: [{ min: 0, max: 255, label: "Dimmer", color: null, tags: [] }], values: [] },
    { key: "red", name: "Red", group: "color", default: 0, capabilities: [], values: [] },
    { key: "green", name: "Green", group: "color", default: 0, capabilities: [], values: [] },
    { key: "blue", name: "Blue", group: "color", default: 0, capabilities: [], values: [] },
    { key: "white", name: "White", group: "color", default: 0, capabilities: [], values: [] },
    {
      key: "strobe", name: "Strobe", group: "strobe", default: 0, values: [],
      capabilities: [
        { min: 0, max: 9, label: "Off", color: null, tags: [] },
        { min: 10, max: 255, label: "Strobe slow → fast", color: null, tags: [] },
      ],
    },
  ],
};

const fixture = (id: string, name: string, profile_id: string, mode: string, output_id: string, address: number, room_id: string, x: number, y: number, z: number, rotation: number): Fixture =>
  ({ id, name, profile_id, mode, output_id, address, room_id, x, y, z, rotation, enabled: true, zigbee_device: null, limits: {} });

const route = (id: string, name: string, group: string, source: string, action: RouteAction, target: Partial<Route> = {}): Route => ({
  id, name, group, enabled: true, source, rooms: [], fixtures: [], groups: [], channel: "", focus: "", falloff: false,
  action, cooldown_ms: 0, every: 1, probability: 1, ...target,
});

export function demoShow(sample: Show): Show {
  const laser = sample.profiles.find((p) => p.kind === "laser")!;
  const fog = sample.profiles.find((p) => p.kind === "fog")!;
  const laserMode = laser.modes[0].name;
  const fogMode = fog.modes[0].name;

  const fixtures: Fixture[] = [
    fixture("fx-laser", "Laser", laser.id, laserMode, "out-a", 1, "room-main", 4, 0.5, 2.3, 90),
    fixture("fx-fog", "Fog", fog.id, fogMode, "out-b", 1, "room-main", 0.6, 0.6, 0.3, 45),
  ];
  // Eight pars around the dance floor, aimed at the middle.
  const ring = [[1.2, 0.8], [3, 0.4], [5, 0.4], [6.8, 0.8], [6.8, 5.2], [5, 5.6], [3, 5.6], [1.2, 5.2]];
  ring.forEach(([x, y], i) => {
    const rot = (Math.atan2(3 - y, 4 - x) * 180) / Math.PI;
    // Angled down so each beam lands on the floor part-way to the middle.
    const tilt = -Math.round((Math.atan2(2.4, Math.hypot(4 - x, 3 - y) * 0.6) * 180) / Math.PI);
    fixtures.push({ ...fixture(`fx-par${i + 1}`, `Par ${i + 1}`, PAR.id, "6 Channel", "out-b", 10 + i * 6, "room-main", x, y, 2.4, rot), tilt });
  });
  // Bar pars point straight down at the bar.
  [[1, 1], [3, 1], [5, 1]].forEach(([x, y], i) =>
    fixtures.push({ ...fixture(`fx-bar${i + 1}`, `Bar par ${i + 1}`, PAR.id, "6 Channel", "out-b", 60 + i * 6, "room-bar", x, y, 2.2, 90), tilt: -90 }));

  const pars = fixtures.filter((f) => f.id.startsWith("fx-par")).map((f) => f.id);
  const front = pars.slice(0, 4);
  const back = pars.slice(4);
  const lookSet = laser.look_sets[0]?.id ?? "";

  const audio = {
    ...sample.audio,
    bands: sample.audio.bands.map((b) =>
      b.id === "snare" ? { ...b, ranges: [{ lo_hz: 1600, hi_hz: 5000, weight: 1 }] }
      : b.id === "vocals" ? { ...b, ranges: [{ lo_hz: 300, hi_hz: 1400, weight: 1 }], sensitivity: 0.4 }
      : b),
  };

  return {
    ...sample,
    audio,
    profiles: [laser, fog, PAR],
    fixtures,
    rooms: [
      {
        id: "room-main", name: "Main room", walls: [[0, 0], [8, 0], [8, 6], [0, 6]], ceiling_m: 2.6,
        focus: [
          { id: "focus-floor", name: "Dance floor", x: 4, y: 3, z: 1.2, radius: 2.4, color: "#ffd27a" },
          { id: "focus-bar", name: "Bar edge", x: 7, y: 3, z: 1.2, radius: 0.8, color: "#4fd1ff" },
        ],
      },
      { id: "room-bar", name: "Bar", walls: [[0, 0], [6, 0], [6, 4], [0, 4]], ceiling_m: 2.4, focus: [] },
    ],
    outputs: [
      { id: "out-a", name: "Laser node", ip: "2.0.0.10", net: 0, subnet: 0, universe: 0, enabled: true },
      { id: "out-b", name: "Main node", ip: "2.0.0.11", net: 0, subnet: 0, universe: 1, enabled: true },
    ],
    groups: [
      { id: "grp-pars", name: "Dance floor pars", color: "#ff5c6c", fixtures: pars },
      { id: "grp-front", name: "Front pars", color: "#ffb84d", fixtures: front },
      { id: "grp-back", name: "Back pars", color: "#4fd1ff", fixtures: back },
      { id: "grp-laser", name: "Laser", color: "#7c5cff", fixtures: ["fx-laser"] },
    ],
    routes: [
      route("r-kick-spread", "Kick colour spread", "Pars", "kick", { type: "palette", palette: "pal-club", mode: "rings", fade_ms: 180, brightness: 1 }, { groups: ["grp-pars"], focus: "focus-floor", falloff: true }),
      route("r-hat-twinkle", "Hi-hat twinkle", "Pars", "hat", { type: "twinkle", palette: "pal-ice", amount: 0.35, decay_ms: 220, base: 0.1 }, { groups: ["grp-back"] }),
      route("r-bass-wave", "Bass wave", "Pars", "bass", { type: "wave", palette: "pal-sunset", axis: "radial", beats: 4, width: 0.4, base: 0.15 }, { groups: ["grp-front"], focus: "focus-floor" }),
      route("r-snare-strobe", "Snare strobe burst", "Pars", "snare", { type: "strobe", rate: "beat", hz: 12, duty: 0.3, burst_ms: 250, threshold: 0.6 }, { groups: ["grp-pars"], enabled: false }),
      route("r-laser-look", "Kick changes laser look", "Laser", "kick", { type: "look", set: lookSet, include_defaults: true, tags: [], mode: "random", hold_ms: 1200, heads: [] }, { groups: ["grp-laser"], every: 4 }),
      route("r-laser-rot", "Vocals drive rotation", "Laser", "vocals", { type: "modulate", min: 0, max: 127, smoothing: 0.6 }, { groups: ["grp-laser"], channel: "p1_rotation" }),
      route("r-bar-hop", "Bar node hop", "Bar", "kick", { type: "node_hop", on: 255, off: 0, decay_ms: 300, palette: "pal-sunset" }, { rooms: ["room-bar"] }),
      route("r-fog", "Energy bursts fog", "Atmosphere", "energy", { type: "flash", on: 255, off: 0, decay_ms: 2500 }, { fixtures: ["fx-fog"], channel: "fog", cooldown_ms: 45000 }),
    ],
    presets: [
      { id: "pre-chill", name: "Chill", color: "#4fd1ff", values: [], routes: { "r-snare-strobe": false, "r-kick-spread": false, "r-bass-wave": true }, exclusive: true, sections: ["breakdown", "groove"] },
      { id: "pre-peak", name: "Peak time", color: "#ff5c6c", values: [], routes: { "r-snare-strobe": true, "r-kick-spread": true, "r-hat-twinkle": true }, exclusive: true, sections: ["build", "drop"] },
    ],
    controls: [
      { id: "c-blackout", name: "Blackout", enabled: true, trigger: { source: "midi", kind: "note", channel: 1, number: 36, key: "", mode: "toggle" }, action: { type: "blackout" } },
      { id: "c-master", name: "Master", enabled: true, trigger: { source: "midi", kind: "cc", channel: 1, number: 7, key: "", mode: "fader" }, action: { type: "master" } },
      { id: "c-strobe", name: "Strobe (hold)", enabled: true, trigger: { source: "key", kind: "note", channel: 1, number: 0, key: "s", mode: "momentary" }, action: { type: "routes", routes: ["r-snare-strobe"] } },
      { id: "c-peak", name: "Peak time", enabled: true, trigger: { source: "midi", kind: "note", channel: 1, number: 40, key: "", mode: "on" }, action: { type: "preset", preset: "pre-peak" } },
      { id: "c-auto", name: "Autopilot", enabled: true, trigger: { source: "midi", kind: "note", channel: 1, number: 41, key: "", mode: "toggle" }, action: { type: "autopilot" } },
      { id: "c-chill", name: "Chill", enabled: true, trigger: { source: "key", kind: "note", channel: 1, number: 0, key: "c", mode: "on" }, action: { type: "preset", preset: "pre-chill" } },
    ],
    midi: { enabled: true, inputs: [], feedback: true, feedback_style: "" },
    autopilot: { min_bars: 8, max_bars: 32, drop_bars: 2, pause_s: 60 },
    zigbee: { ...sample.zigbee, enabled: false, password: "", host: "" },
    active_room: "room-main",
  };
}

/** Animate meters, spectrum and fixture colours as if music were playing (124 BPM). */
export function startDemoFeed() {
  const bpm = 124;
  const t0 = performance.now();
  const hits: Record<string, number> = {};
  const hex = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const club = ["#ff1f5a", "#8a2bff", "#2b6bff", "#00d4ff"].map(hex);
  const sunset = ["#ff3d00", "#ff8a00", "#ffc400", "#ff2d6f"].map(hex);

  setInterval(() => {
    const show = app.show;
    if (!show) return;
    const secs = (performance.now() - t0) / 1000;
    const beatF = (secs * bpm) / 60;
    const beats = Math.floor(beatF);
    const phase = beatF - beats;
    const kick = Math.exp(-phase * 6);
    const snare = beats % 2 === 1 ? Math.exp(-phase * 8) : 0.05;
    const half = (beatF * 2) % 1;
    const hat = Math.exp(-half * 10) * 0.8;
    const bass = 0.55 + 0.35 * Math.sin(beatF * Math.PI / 2) * kick;
    const vocals = 0.4 + 0.3 * Math.sin(secs * 1.3) + 0.1 * Math.sin(secs * 5.1);

    const levels: Record<string, number> = { bass, kick, snare, vocals, hihat: hat };
    const bands: BandOut[] = show.audio.bands.map((b) => {
      const level = Math.max(0, Math.min(1, levels[b.id] ?? 0.3 + 0.2 * Math.sin(secs * 2 + b.id.length)));
      const hit = b.id === "kick" ? phase < 0.05 : b.id === "snare" ? beats % 2 === 1 && phase < 0.05 : b.id === "hat" ? half < 0.08 : false;
      if (hit) hits[b.id] = (hits[b.id] ?? 0) + 1;
      return { id: b.id, level, flux: level * 0.9, threshold: 0.45, hits: hits[b.id] ?? 0 };
    });

    const spectrum = Array.from({ length: 96 }, (_, i) => {
      const x = i / 96;
      const low = kick * Math.exp(-(((x - 0.12) / 0.07) ** 2));
      const mid = (0.35 + 0.25 * vocals) * Math.exp(-(((x - 0.45) / 0.18) ** 2));
      const high = hat * 0.6 * Math.exp(-(((x - 0.85) / 0.1) ** 2));
      const noise = 0.08 * Math.abs(Math.sin(i * 12.9898 + secs * 7));
      return Math.min(1, 0.12 + low * 0.85 + mid + high + noise - x * 0.1);
    });

    const fixtures: Record<string, number[]> = {};
    for (const f of show.fixtures) {
      if (f.profile_id === PAR.id) {
        const n = Number(f.id.replace(/\D/g, "")) || 1;
        const bar = f.id.startsWith("fx-bar");
        const pal = bar ? sunset : club;
        const [r, g, b] = pal[(n + beats + (bar ? 0 : Math.floor(n / 4))) % pal.length];
        const dim = bar ? 120 + 135 * (beats % 3 === n - 1 ? kick : 0.15) : 150 + 105 * (n <= 4 ? kick : Math.max(hat, 0.3));
        fixtures[f.id] = [Math.round(dim), r, g, b, 0, 0];
      } else if (f.id === "fx-fog") {
        fixtures[f.id] = [secs % 20 < 3 ? 255 : 0, 0, 0, 0];
      } else {
        const p = show.profiles.find((x) => x.id === f.profile_id);
        const len = p?.modes.find((m) => m.name === f.mode)?.channels.length ?? 0;
        fixtures[f.id] = Array(len).fill(0);
      }
    }

    const frame: Frame = {
      features: {
        running: true, device: "Demo input", sample_rate: 48000, error: null,
        level: 0.55 + 0.3 * kick, energy: 0.5 + 0.2 * Math.sin(secs / 3),
        bands, beats, bpm, beat_phase: phase, spectrum,
      },
      fixtures,
      blackout: false,
      send_errors: [],
    };
    app.frame = frame;
    demoAutopilot(beats, kick);
  }, 40);
}

// A scripted track for the autopilot panel: 8 bars groove, 4 breakdown, 4 build, 8 drop.
const SCRIPT: Section[] = [...Array(8).fill("groove"), ...Array(4).fill("breakdown"), ...Array(4).fill("build"), ...Array(8).fill("drop")];
let lastBar = -1;
function demoAutopilot(beats: number, kick: number) {
  const ap = (app.autopilot ??= { running: false, section: "groove", history: [], scene: null, bars_in_scene: 0, paused_s: 0, reason: "", scenes: 0, loudness: 0.5 });
  const bar = Math.floor(beats / 4);
  if (bar === lastBar || !app.show) return;
  lastBar = bar;
  const section = SCRIPT[bar % SCRIPT.length];
  ap.section = section;
  ap.history = [...ap.history, section].slice(-32);
  ap.loudness = { breakdown: 0.15, groove: 0.5, build: 0.7, drop: 0.95 }[section] * (0.9 + 0.1 * kick);
  ap.scenes = app.show.presets.filter((p) => p.sections?.length).length;
  ap.bars_in_scene++;
  if (!ap.running) return;
  const fits = app.show.presets.filter((p) => p.sections?.includes(section));
  if (fits.length && !fits.some((p) => p.id === ap.scene)) {
    ap.scene = fits[0].id;
    ap.bars_in_scene = 0;
    ap.reason = `${section} detected`;
    app.activePresets = [fits[0].id];
  }
}
