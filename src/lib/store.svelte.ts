import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AutopilotStatus, ChannelDef, ControlState, Fixture, Frame, MidiEvent, Profile, ProjectInfo, Show } from "./types";
import { mergeInto } from "./library";

export const app = $state({
  show: null as Show | null,
  frame: null as Frame | null,
  view: "live" as string,
  manual: {} as Record<string, Record<string, number>>,
  blackout: false,
  saveError: "" as string,
  project: { path: null, name: "Untitled", recent: [] } as ProjectInfo,
  /** JSON of the show as last opened/saved to its project file. */
  savedJson: "" as string,
  toast: "" as string,
  /** Bumped when a different show is loaded so views remount. */
  epoch: 0,
  master: 1,
  activePresets: [] as string[],
  /** The scene playing: its looks are the layer under the routes. */
  scene: null as string | null,
  /** Last control mapping that fired + when (for flashing it in the UI). */
  fired: { id: "", at: 0 },
  midiLog: [] as (MidiEvent & { at: number })[],
  /** A mapping waiting for MIDI/key learn; global key handling pauses meanwhile. */
  learning: null as string | null,
  /** When Learn started; MIDI received before this doesn't count. */
  learnStarted: 0,
  /** What the autopilot hears and plays; updated every bar. */
  autopilot: null as AutopilotStatus | null,
});

export function isDirty() {
  return !!app.show && JSON.stringify(app.show) !== app.savedJson;
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(msg: string) {
  app.toast = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = ""), 3500);
}

export function updateTitle() {
  const t = `${isDirty() ? "• " : ""}${app.project.name} — LightDeck`;
  getCurrentWindow().setTitle(t).catch(() => {});
}

function loaded(show: Show, info: ProjectInfo) {
  app.show = show;
  app.project = info;
  app.savedJson = JSON.stringify(show);
  app.manual = {};
  app.epoch++;
  updateTitle();
}

export async function newProject() {
  const r = await invoke<{ show: Show; info: ProjectInfo }>("project_new");
  // Starter fixture types you've saved to the library come with your looks.
  let fromLib = 0;
  try {
    const library = await invoke<Profile[]>("get_fixture_library");
    for (const p of r.show.profiles) {
      const l = library.find((x) => x.id === p.id);
      if (l) { mergeInto(p, l); fromLib++; }
    }
  } catch { /* library unavailable: keep the starter types */ }
  loaded(r.show, r.info);
  toast(fromLib ? `New project (${fromLib} fixture type${fromLib === 1 ? "" : "s"} from your library)` : "New project");
  if (fromLib) await invoke("save_show", { show: $state.snapshot(r.show) });
}

export async function openProject(path?: string) {
  try {
    const r = await invoke<{ show: Show; info: ProjectInfo } | null>("project_open", { path: path ?? null });
    if (!r) return;
    loaded(r.show, r.info);
    toast(`Opened ${r.info.name}`);
  } catch (e) {
    toast(String(e));
    app.project = await invoke<ProjectInfo>("project_info");
  }
}

export async function saveProject(saveAs = false) {
  if (!app.show) return;
  // make sure the backend has the latest edits before writing the file
  await invoke("save_show", { show: $state.snapshot(app.show) });
  try {
    const info = await invoke<ProjectInfo | null>("project_save", { saveAs });
    if (!info) return;
    app.project = info;
    app.savedJson = JSON.stringify(app.show);
    updateTitle();
    toast(`Saved ${info.name}`);
  } catch (e) {
    toast(String(e));
  }
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
export function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    if (!app.show) return;
    try {
      await invoke("save_show", { show: $state.snapshot(app.show) });
      app.saveError = "";
    } catch (e) {
      app.saveError = String(e);
    }
  }, 250);
}

export async function init() {
  // A previous session (or a reload mid-preview) may have left a look preview running.
  await invoke("clear_previews");
  app.show = await invoke<Show>("get_show");
  app.manual = await invoke("get_manual");
  app.project = await invoke<ProjectInfo>("project_info");
  // The working copy autosaves; treat it as clean only when it was never saved to a project
  // or matches what's on disk. We can't cheaply compare to disk, so start clean.
  app.savedJson = JSON.stringify(app.show);
  updateTitle();
  await listen<Frame>("frame", (e) => {
    app.frame = e.payload;
  });
  await listen<ControlState>("control_state", (e) => applyControlState(e.payload));
  await listen<MidiEvent>("midi", (e) => {
    app.midiLog = [{ ...e.payload, at: Date.now() }, ...app.midiLog].slice(0, 30);
  });
  await listen<AutopilotStatus>("autopilot", (e) => (app.autopilot = e.payload));
  applyControlState(await invoke<ControlState>("control_state"));
  app.autopilot = await invoke<AutopilotStatus>("autopilot_status").catch(() => null);
}

export async function setManual(fixture: string, channel: string, value: number | null) {
  if (value === null) {
    if (app.manual[fixture]) delete app.manual[fixture][channel];
  } else {
    // (assign, then write through the store: writing onto the `??=` result misses the reactive copy)
    app.manual[fixture] ??= {};
    app.manual[fixture][channel] = value;
  }
  await invoke("set_manual", { fixture, channel, value });
}

export async function clearManual(fixture?: string) {
  if (fixture) delete app.manual[fixture];
  else app.manual = {};
  await invoke("clear_manual", { fixture: fixture ?? null });
}

export async function setBlackout(on: boolean) {
  app.blackout = on;
  await invoke("set_blackout", { on });
}

export const uid = (p: string) => `${p}-${Math.random().toString(36).slice(2, 9)}`;

export function profileOf(fx: Fixture): Profile | undefined {
  return app.show?.profiles.find((p) => p.id === fx.profile_id);
}

export function modeChannels(fx: Fixture): ChannelDef[] {
  const p = profileOf(fx);
  if (!p) return [];
  const mode = p.modes.find((m) => m.name === fx.mode) ?? p.modes[0];
  if (!mode) return [];
  return mode.channels.map((k) => p.channels.find((c) => c.key === k)).filter(Boolean) as ChannelDef[];
}

export function capabilityAt(ch: ChannelDef, v: number) {
  return ch.capabilities.find((c) => v >= c.min && v <= c.max);
}

/** Rough visual of a fixture's current output for the room view. */
export function fixtureLook(fx: Fixture, values: number[] | undefined): { color: string; intensity: number } {
  const chans = modeChannels(fx);
  const p = profileOf(fx);
  if (!values || !chans.length) return { color: "#666", intensity: 0 };
  let intensity = -1;
  let color = p?.kind === "laser" ? "#7c5cff" : p?.kind === "fog" ? "#9aa4b8" : "#ffd27a";
  let rgb: number[] = [];
  chans.forEach((c, i) => {
    const v = values[i] ?? 0;
    const cap = capabilityAt(c, v);
    if (cap?.tags.includes("off")) intensity = 0;
    if (c.group === "intensity" || c.group === "fog") intensity = Math.max(intensity, v / 255);
    if (c.group === "color" && cap?.color) color = cap.color;
    const n = c.name.toLowerCase();
    if (c.group === "color" && ["red", "green", "blue"].includes(n)) rgb[["red", "green", "blue"].indexOf(n)] = v;
  });
  if (rgb.length) color = `rgb(${rgb[0] ?? 0},${rgb[1] ?? 0},${rgb[2] ?? 0})`;
  if (intensity < 0) intensity = 1;
  return { color, intensity };
}

export function footprint(fx: Fixture) {
  return modeChannels(fx).length;
}

/** Fixtures a target selects: rooms scope it; fixtures + groups' members pick within (both empty = all in scope). */
export function routeFixtures(r: { rooms?: string[]; fixtures: string[]; groups: string[] }): Fixture[] {
  const show = app.show!;
  const rooms = r.rooms ?? [];
  const inScope = (f: Fixture) => !rooms.length || (f.room_id !== null && rooms.includes(f.room_id));
  if (!r.fixtures.length && !r.groups.length) return show.fixtures.filter(inScope);
  const ids = new Set(r.fixtures);
  for (const g of show.groups) if (r.groups.includes(g.id)) g.fixtures.forEach((f) => ids.add(f));
  return show.fixtures.filter((f) => ids.has(f.id) && inScope(f));
}

/** Mirror changes made by MIDI/keys/buttons in the backend into the UI (and so into the saved show). */
export function applyControlState(cs: ControlState) {
  const show = app.show;
  if (show) {
    for (const r of show.routes) if (cs.routes[r.id] !== undefined && r.enabled !== cs.routes[r.id]) r.enabled = cs.routes[r.id];
    for (const b of show.audio.bands) {
      const st = cs.bands[b.id];
      if (!st) continue;
      if (b.enabled !== st.enabled) b.enabled = st.enabled;
      if (Math.abs(b.sensitivity - st.sensitivity) > 0.001) b.sensitivity = st.sensitivity;
    }
  }
  app.blackout = cs.blackout;
  app.master = cs.master;
  app.manual = cs.manual;
  app.activePresets = cs.active_presets;
  app.scene = cs.scene ?? null;
  if (app.autopilot && cs.autopilot !== undefined) app.autopilot.running = cs.autopilot;
  if (cs.fired) app.fired = { id: cs.fired, at: Date.now() };
}

/** Does the fixture have the route's channel (a key or "@type")? */
export function fixtureHasChannel(f: Fixture, channel: string) {
  const chans = modeChannels(f);
  return channel.startsWith("@") ? chans.some((c) => c.group === channel.slice(1)) : chans.some((c) => c.key === channel);
}

/** Start (or cancel) Learn for a control: the next pad, knob or key sets its trigger. */
export function startLearn(id: string) {
  app.learning = app.learning === id ? null : id;
  app.learnStarted = Date.now();
}
