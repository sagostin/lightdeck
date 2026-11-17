import type { BlockUse } from "./blocks";
export interface Capability { min: number; max: number; label: string; color: string | null; tags: string[] }
export interface ValueNote { value: number; label: string; tags: string[]; rating: number }
export interface ChannelDef {
  key: string; name: string; group: string; default: number;
  capabilities: Capability[]; values: ValueNote[];
}
export interface Head { name: string; channels: string[] }
export interface Mode { name: string; channels: string[]; heads: Head[] }
export interface Profile {
  id: string; manufacturer: string; model: string; kind: string;
  channels: ChannelDef[]; modes: Mode[]; notes: string;
  /** Beam angle in degrees; 0 = use the default for the kind. */
  beam_deg?: number;
  looks: Look[]; look_sets: LookSet[];
}
/** A pattern plus secondary options (several channel values) applied together. */
export interface Look {
  id: string; name: string; head: string; values: Record<string, number>;
  tags: string[]; rating: number; sets: string[]; builtin: boolean;
  /** Channels that move (sweep/step/random/list/catalog/follow) instead of holding their value. */
  motions: Record<string, Motion>;
}
export interface Motion {
  kind: "sweep" | "step" | "random" | "list" | "catalog" | "follow";
  ranges: [number, number][]; values: number[];
  clock: "beat" | "time" | "hit"; period: number;
  shape: "up" | "down" | "pingpong" | "sine"; step: number; seed: number; source: string; tags: string[];
  /** In routes: start over on each hit (time and beats count from the hit). */
  retrigger?: boolean;
  /** One pass (one sweep, one run through the steps), then hold the last value. */
  once?: boolean;
  /** In routes: a hit source of its own that restarts and counts this motion ("" = the route's hits). */
  trigger?: string;
}
export const newMotion = (kind: Motion["kind"], ranges: [number, number][] = [[0, 255]]): Motion => ({
  kind, ranges, values: [], clock: "beat", period: kind === "sweep" ? 4 : 1, shape: "pingpong", step: 1,
  seed: Math.floor(Math.random() * 1e6) + 1, source: "level", tags: [],
});
export function describeMotion(m: Motion) {
  const span = m.ranges.map(([a, b]) => (a === b ? `${a}` : `${a}–${b}`)).join(", ");
  const per = m.clock === "hit" ? (m.period === 1 ? "per hit" : `every ${m.period} hits`) : m.clock === "time" ? `${m.period}s` : `${m.period} beat${m.period === 1 ? "" : "s"}`;
  const how = `${m.trigger ? `, on ${m.trigger} hits` : ""}${m.retrigger ? ", from each hit" : ""}${m.once ? ", once" : ""}`;
  return describeKind(m, span, per) + how;
}
function describeKind(m: Motion, span: string, per: string) {
  switch (m.kind) {
    case "sweep": return `sweep ${span} (${m.shape}, ${per})`;
    case "step": return `step ${span} by ${m.step}, ${per}`;
    case "random": return `random ${span}, ${per}, seed ${m.seed}`;
    case "list": return `list ${m.values.join(", ")}, ${per}`;
    case "catalog": return `catalogue${m.tags.length ? ` [${m.tags.join(", ")}]` : ""}, ${per}`;
    case "follow": return `follow ${m.source === "level" ? "the overall level" : m.source.replace(/\.level$/, " level")} across ${span}`;
  }
}
export interface LookSet { id: string; name: string; color: string }
/** Set ids come from names, so same-named sets on different fixture types match in routes. */
export const setId = (name: string) => "set-" + name.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
export interface Limit { lock: number | null; range: [number, number] | null; require_tags: string[]; min_hold_ms: number }
export interface Fixture {
  id: string; name: string; profile_id: string; mode: string; output_id: string; address: number;
  room_id: string | null; x: number; y: number; z: number; rotation: number; enabled: boolean;
  /** Degrees up from horizontal: -90 = straight down, 90 = straight up; null = not aimed. */
  tilt?: number | null;
  /** Beam angle override in degrees. */
  beam_deg?: number | null;
  zigbee_device: string | null;
  limits: Record<string, Limit>;
}
export interface FixtureGroup { id: string; name: string; color: string; fixtures: string[] }
export const GROUP_LABEL: Record<string, string> = {
  intensity: "Brightness", color: "Colour", pan: "Pan", tilt: "Tilt", pattern: "Pattern / gobo", beam: "Beam / zoom",
  effect: "Effect", speed: "Speed", strobe: "Strobe", mode: "Mode", fog: "Fog", other: "Other",
};
export interface FocusPoint { id: string; name: string; x: number; y: number; z: number; radius: number; color: string }
export interface Room { id: string; name: string; walls: [number, number][]; ceiling_m: number; focus: FocusPoint[] }
export interface ArtNetOutput { id: string; name: string; ip: string; net: number; subnet: number; universe: number; enabled: boolean }
export type RouteAction =
  | { type: "modulate"; min: number; max: number; smoothing: number }
  | { type: "flash"; on: number; off: number; decay_ms: number }
  | { type: "random"; tags: string[]; hold_ms: number }
  | { type: "node_hop"; on: number; off: number; decay_ms: number; palette: string }
  | { type: "strobe"; rate: string; hz: number; duty: number; burst_ms: number; threshold: number }
  | { type: "palette"; palette: string; mode: "all" | "spread" | "rings" | "around" | "random"; fade_ms: number; brightness: number }
  | { type: "twinkle"; palette: string; amount: number; decay_ms: number; base: number }
  | { type: "wave"; palette: string; axis: "x" | "y" | "radial" | "angle" | "height"; beats: number; width: number; base: number }
  | { type: "cycle"; values: number[] }
  | {
    type: "look"; set: string; include_defaults: boolean; tags: string[]; mode: "random" | "cycle"; hold_ms: number; heads: string[];
    /** Only these looks (names); empty = any matching the set and tags. */
    looks?: string[];
    /** Prefer looks tagged with the section playing, and change when it changes. */
    follow_section?: boolean;
    /** Change anyway after this long without a hit (0 = only on hits). */
    max_ms?: number;
    /** Chance a change shows no look (the scene's look or defaults), and for how long (0 = until the next hit). */
    rest?: number; rest_ms?: number;
  }
  /** Keep moving the route's channel (sweep, step, random…); spread 0..1 makes fixtures chase. */
  | { type: "motion"; motion: Motion; spread: number };
export interface Route {
  id: string; name: string; group: string; enabled: boolean; source: string;
  /** Rooms scope the targets; fixture ids and group ids pick within them (both empty = all in scope). */
  rooms: string[]; fixtures: string[]; groups: string[];
  /** Channel key, or "@<channel type>" for every channel of that type. */
  channel: string;
  /** Focus point id effects are calculated from ("" = centre of the targets). */
  focus: string;
  /** Effects dim with distance from the focus, reaching zero at its radius. */
  falloff: boolean;
  action: RouteAction; cooldown_ms: number; every: number; probability: number;
  /** Runs only during these sections; empty or missing = always. */
  sections?: Section[];
  /** The scene (preset id) that built this route; "" or missing = made by hand. */
  scene?: string;
  /** Other channels this route also moves on its fixtures: channel key (or "@type") -> motion. */
  motions?: Record<string, Motion>;
  /** Other routes this route switches each time it fires. */
  switches?: RouteSwitch[];
}
/** When a route fires, switch another route on, off or over for `beats` beats (0 = until changed). */
export interface RouteSwitch { route: string; to: "on" | "off" | "toggle"; beats: number }
export interface BandRange { lo_hz: number; hi_hz: number; weight: number }
export interface BandDef {
  id: string; name: string; enabled: boolean; ranges: BandRange[]; detect: "transient" | "level"; level_threshold: number;
  gain_db: number; sensitivity: number; min_gap_ms: number; release: number; color: string;
}
export interface AudioSettings { device: string | null; gain_db: number; sensitivity: number; bands: BandDef[] }
export interface BandOut { id: string; level: number; flux: number; threshold: number; hits: number }
export interface Show {
  version: number; profiles: Profile[]; fixtures: Fixture[]; rooms: Room[]; outputs: ArtNetOutput[];
  routes: Route[]; audio: AudioSettings; zigbee: ZigbeeSettings; groups: FixtureGroup[];
  presets: Preset[]; palettes: Palette[]; controls: ControlMapping[]; midi: MidiSettings; active_room: string | null;
  autopilot?: AutopilotSettings;
}
/** Parts of a track the autopilot tells apart. */
export type Section = "breakdown" | "groove" | "build" | "drop";
export interface AutopilotSettings {
  min_bars: number; max_bars: number; drop_bars: number; pause_s: number;
  /** Limits per section; a missing section has none. */
  rules?: Partial<Record<Section, SectionRule>>;
}
/** What's allowed during a section: a brightness cap (0..1), strobe, fog from routes. */
export interface SectionRule { brightness: number; strobe: boolean; fog: boolean }
export interface AutopilotStatus {
  running: boolean; section: Section; history: Section[]; scene: string | null; bars_in_scene: number;
  paused_s: number; reason: string; scenes: number; loudness: number;
  /** A section held on a pad or on screen. */
  forced?: Section | null;
}
/** An autopilot set-up suggested by the AI, checked against the show. */
export interface AutopilotPlan { sections: Record<string, Section[]>; settings: AutopilotSettings; notes: string; warnings: string[] }
export interface Features {
  running: boolean; device: string; sample_rate: number; error: string | null;
  level: number; energy: number; bands: BandOut[];
  beats: number; bpm: number; beat_phase: number;
  spectrum: number[];
}
export interface Frame { features: Features; fixtures: Record<string, number[]>; blackout: boolean; send_errors: string[]; switched?: Record<string, boolean> }
export interface AiSettings {
  schema: "anthropic" | "openai"; base_url: string; api_key: string; model: string;
  auth: "x-api-key" | "bearer" | "none"; extra_headers: Record<string, string>; max_tokens: number;
}

export const GROUPS = ["intensity", "color", "pan", "tilt", "pattern", "beam", "effect", "speed", "strobe", "mode", "fog", "other"];
export const KINDS = ["laser", "par", "fog", "moving_head", "strobe", "zigbee", "other"];

export interface ZigbeeSettings {
  enabled: boolean; mode: "usb" | "mqtt";
  serial_port: string; adapter: string; channel: number; names: Record<string, string>;
  host: string; port: number; username: string; password: string; base_topic: string;
  max_rate_hz: number; transition_s: number;
}
export interface ZigbeeDevice {
  key: string; friendly_name: string; ieee: string; vendor: string; model: string; description: string; kind: string;
  brightness: boolean; color: boolean; color_temp: boolean; switch_only: boolean;
  role: string; power: string; interview: string; lqi: number | null;
}
export interface ZigbeePort { name: string; path: string; adapter: string | null }
export interface ZigbeeStatus {
  enabled: boolean; mode: string; connected: boolean; bridge_online: boolean | null; error: string | null;
  state: string; coordinator: string | null; permit_join_until: number;
  devices: ZigbeeDevice[]; sent_per_sec: number; backlog: number; last_send_error: string | null; log: string[];
}
export type BandPreset = Omit<BandDef, "id">;
const P = (name: string, ranges: [number, number][], detect: BandDef["detect"], sensitivity: number, min_gap_ms: number, release: number, color: string): BandPreset =>
  ({ name, enabled: true, ranges: ranges.map(([lo_hz, hi_hz]) => ({ lo_hz, hi_hz, weight: 1 })), detect, level_threshold: 0.6, sensitivity, min_gap_ms, release, color, gain_db: 0 });
/** Starting points for new bands; every value stays editable afterwards. */
export const BAND_PRESETS: BandPreset[] = [
  P("Sub-bass", [[20, 60]], "level", 0.3, 250, 0.93, "#c45cff"),
  P("Kick", [[35, 150]], "transient", 0.5, 140, 0.85, "#ff5c6c"),
  P("Bass", [[60, 250]], "level", 0.35, 220, 0.92, "#ff9a4d"),
  P("Snare", [[150, 250], [1800, 5000]], "transient", 0.5, 140, 0.85, "#f2b54a"),
  P("Clap", [[1000, 4000]], "transient", 0.5, 140, 0.8, "#ffd27a"),
  P("Vocals", [[300, 3000]], "level", 0.3, 250, 0.93, "#45d39a"),
  P("Lead synth", [[800, 5000]], "level", 0.4, 180, 0.9, "#4fe0d0"),
  P("Hi-hat", [[6000, 16000]], "transient", 0.5, 70, 0.8, "#4fb3ff"),
  P("Cymbals and air", [[10000, 20000]], "transient", 0.45, 120, 0.88, "#a594ff"),
];
export const bandSpan = (b: BandDef) =>
  b.ranges.map((r) => `${fmtHz(r.lo_hz)}–${fmtHz(r.hi_hz)}`).join(" + ");
export const fmtHz = (hz: number) => (hz >= 1000 ? `${+(hz / 1000).toFixed(1)}k` : `${Math.round(hz)}`);

export interface ProjectInfo { path: string | null; name: string; recent: string[] }

export const SPEC_LO = 20;
export const SPEC_HI = 20000;

/** Route sources derived from the show's detection bands. */
export function sourceOptions(bands: BandDef[]) {
  return {
    triggers: [...bands.map((b) => ({ id: b.id, label: `${b.name} hit` })), { id: "beat", label: "Beat" }, { id: "bar", label: "Bar (4 beats)" }],
    levels: [...bands.map((b) => ({ id: `${b.id}.level`, label: `${b.name} level` })), { id: "level", label: "Overall level" },
      { id: "energy", label: "Energy (build/drop)" }, { id: "phase", label: "Beat phase" }],
  };
}
export function isTriggerSource(src: string, bands: BandDef[]) {
  return src === "beat" || src === "bar" || bands.some((b) => b.id === src);
}
export const COMMON_TAGS = ["blocked", "off", "safe", "chill", "mid", "peak", "favourite", "static", "dynamic", "large_projection", "strobe"];

export interface Target { rooms: string[]; groups: string[]; fixtures: string[] }
export interface PresetValue { fixture: string; channel: string; value: number }
export interface Preset {
  id: string; name: string; color: string; values: PresetValue[]; routes: Record<string, boolean>; exclusive: boolean;
  /** Parts of a track the autopilot may use this scene for. */
  sections?: Section[];
  /** The scene's building blocks; the scene's routes are built from them. Missing = a scene of routes. */
  blocks?: BlockUse[];
  /** Looks fixtures sit in while the scene plays, underneath its routes. */
  looks?: SceneLook[];
  /** Channel values the scene sets underneath its routes (captured from the rig). */
  base?: PresetValue[];
}
/** A saved look (by name) some fixtures are put in while a scene plays. */
export interface SceneLook { target: Target; look: string; heads: string[] }
export interface MidiSettings {
  enabled: boolean; inputs: string[];
  /** Light up controller pads to show what's on. */
  feedback: boolean;
  /** "" = from the port name, "off", or a style from FEEDBACK_STYLES. */
  feedback_style: string;
}
export const FEEDBACK_STYLES: [string, string][] = [
  ["", "Automatic (from the controller's name)"],
  ["generic", "On/off (velocity or CC 127)"],
  ["apc_mini_mk2", "Akai APC mini mk2 (RGB)"],
  ["apc_mini", "Akai APC mini (green/red/yellow)"],
  ["launchpad", "Novation Launchpad (RGB palette)"],
  ["nanokontrol2", "Korg nanoKONTROL2 (LED mode External)"],
];
export interface ControlTrigger {
  source: "midi" | "key"; kind: "note" | "cc" | "program"; channel: number; number: number; key: string;
  mode: "toggle" | "momentary" | "on" | "off" | "fader";
}
export type ControlAction =
  | { type: "routes"; routes: string[] }
  | { type: "route_group"; group: string }
  | { type: "bands"; bands: string[] }
  | { type: "blackout" }
  | { type: "master" }
  | { type: "preset"; preset: string }
  | { type: "release_manual" }
  | { type: "channel_value"; target: Target; channel: string; min: number; max: number }
  | { type: "band_sensitivity"; band: string }
  | { type: "autopilot" }
  | { type: "force_section"; section: Section };
export interface ControlMapping {
  id: string; name: string; enabled: boolean; trigger: ControlTrigger; action: ControlAction;
  /** Pad colour for MIDI feedback; "" or missing = a colour for the action. */
  color?: string;
}
export interface MidiEvent { port: string; kind: "note" | "cc" | "program"; channel: number; number: number; value: number }
export interface MidiStatus { enabled: boolean; available: string[]; connected: string[]; feedback?: [string, string][]; error: string | null }
export interface ControlState {
  routes: Record<string, boolean>; bands: Record<string, { enabled: boolean; sensitivity: number }>;
  blackout: boolean; master: number; manual: Record<string, Record<string, number>>; active_presets: string[]; scene?: string | null; autopilot?: boolean; fired: string | null;
}

export interface Palette { id: string; name: string; colors: string[] }
/** Route actions that drive whole fixtures (brightness + colour) instead of one channel. */
export const EFFECT_ACTIONS = ["strobe", "palette", "twinkle", "wave"];

export interface ControllerInput { kind: "note" | "cc"; number: number; label: string }
/** A MIDI controller layout: pads/buttons and faders/knobs in the order auto-mapping assigns them. */
export interface ControllerProfile {
  id: string; name: string; channel: number; pads: ControllerInput[]; faders: ControllerInput[]; notes: string;
  /** Built into LightDeck (not saved; copy it to change it). */
  builtin?: boolean;
}
