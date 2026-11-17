//! Control surface: MIDI notes/CCs, keyboard keys and on-screen buttons mapped to
//! actions (routes, bands, presets, blackout, master, channel faders).
//!
//! Actions change the live show immediately (so the engine reacts at once) and
//! emit a "control_state" event so the UI mirrors the change and saves it.

use crate::audio::AudioParams;
use crate::autopilot::Autopilot;
use crate::feedback::{parse_hex, Led};
use crate::engine::{channel_keys, resolve_fixtures};
use crate::midi::MidiEvent;
use crate::model::*;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter};

type Key = (String, String);

pub enum Input {
    Midi(MidiEvent),
    Key { key: String, down: bool },
    /// From an on-screen button: pressed/released, or a fader value 0..1.
    Fire { mapping: String, pressed: bool, value: Option<f32> },
    Preset { preset: String },
    Master { value: f32 },
    /// The autopilot picked a scene.
    Autopilot { preset: String },
    /// Switch the autopilot on/off from the screen.
    AutopilotSet { on: bool },
}

#[derive(Serialize, Clone, Default)]
pub struct BandState {
    pub enabled: bool,
    pub sensitivity: f32,
}

#[derive(Serialize, Clone, Default)]
pub struct ControlState {
    pub routes: BTreeMap<String, bool>,
    pub bands: BTreeMap<String, BandState>,
    pub blackout: bool,
    pub master: f32,
    pub manual: HashMap<String, HashMap<String, u8>>,
    pub active_presets: Vec<String>,
    /// The scene playing (its looks are the layer under the routes).
    pub scene: Option<String>,
    pub autopilot: bool,
    /// The mapping that just fired, for on-screen feedback.
    pub fired: Option<String>,
}

pub struct Controls {
    pub show: Arc<RwLock<Show>>,
    pub manual: Arc<Mutex<HashMap<Key, u8>>>,
    pub blackout: Arc<AtomicBool>,
    pub master: Arc<AtomicU32>,
    pub audio: Arc<AudioParams>,
    pub app: AppHandle,
    pub autopilot: Arc<Autopilot>,
    /// The playing scene, shared with the engine.
    pub scene: Arc<Mutex<Option<(String, std::time::Instant)>>>,
    toggles: Mutex<HashSet<String>>,
    active_presets: Mutex<HashSet<String>>,
    /// The scene recalled last, by anyone.
    last_scene: Mutex<Option<String>>,
}

impl Controls {
    pub fn start(
        show: Arc<RwLock<Show>>,
        manual: Arc<Mutex<HashMap<Key, u8>>>,
        blackout: Arc<AtomicBool>,
        master: Arc<AtomicU32>,
        audio: Arc<AudioParams>,
        app: AppHandle,
        autopilot: Arc<Autopilot>,
        scene: Arc<Mutex<Option<(String, std::time::Instant)>>>,
    ) -> (Arc<Self>, Sender<Input>) {
        let c = Arc::new(Self {
            show,
            manual,
            blackout,
            master,
            audio,
            app,
            autopilot,
            scene,
            toggles: Mutex::new(HashSet::new()),
            active_presets: Mutex::new(HashSet::new()),
            last_scene: Mutex::new(None),
        });
        let (tx, rx) = channel::<Input>();
        let me = c.clone();
        std::thread::Builder::new()
            .name("controls".into())
            .spawn(move || {
                while let Ok(input) = rx.recv() {
                    me.handle(input);
                }
            })
            .unwrap();
        (c, tx)
    }

    fn handle(&self, input: Input) {
        let mappings: Vec<ControlMapping> =
            self.show.read().unwrap().controls.iter().filter(|m| m.enabled).cloned().collect();
        let mut fired = None;
        match input {
            Input::Midi(ev) => {
                let _ = self.app.emit("midi", &ev);
                for m in mappings.iter().filter(|m| midi_matches(&m.trigger, &ev)) {
                    let (pressed, value) = match ev.kind.as_str() {
                        "cc" => (ev.value >= 64, Some(ev.value as f32 / 127.0)),
                        "program" => (true, None),
                        _ => (ev.value > 0, None),
                    };
                    self.run(m, pressed, value);
                    fired = Some(m.id.clone());
                }
            }
            Input::Key { key, down } => {
                let key = key.to_lowercase();
                for m in mappings.iter().filter(|m| m.trigger.source == "key" && m.trigger.key.to_lowercase() == key) {
                    self.run(m, down, None);
                    fired = Some(m.id.clone());
                }
            }
            Input::Fire { mapping, pressed, value } => {
                // Look up first: an `if let` on the read guard would hold it while run() takes the write lock.
                let found = self.show.read().unwrap().controls.iter().find(|m| m.id == mapping).cloned();
                if let Some(m) = found {
                    self.run(&m, pressed, value);
                    fired = Some(m.id.clone());
                }
            }
            Input::Preset { preset } => {
                self.apply_preset(&preset, true);
                self.picked_scene(&preset);
            }
            Input::Master { value } => self.master.store(value.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed),
            Input::Autopilot { preset } => {
                if self.autopilot.running() {
                    self.apply_preset(&preset, true);
                }
            }
            Input::AutopilotSet { on } => self.set_autopilot(on),
        }
        self.emit_state(fired);
    }

    /// Resolve a button press/release into a new on/off state given the trigger mode.
    fn button(mode: &str, pressed: bool, current: bool) -> Option<bool> {
        match (mode, pressed) {
            ("momentary", p) => Some(p),
            ("on", true) => Some(true),
            ("off", true) => Some(false),
            ("toggle", true) | ("", true) => Some(!current),
            _ => None,
        }
    }

    fn run(&self, m: &ControlMapping, pressed: bool, value: Option<f32>) {
        let mode = m.trigger.mode.as_str();
        // A fader (CC) on an on/off action acts as a button: up half = on.
        match &m.action {
            ControlAction::Routes { routes } => {
                let mut show = self.show.write().unwrap();
                let current = show.routes.iter().find(|r| routes.contains(&r.id)).map(|r| r.enabled).unwrap_or(false);
                if let Some(on) = fader_or_button(mode, pressed, value, current) {
                    show.routes.iter_mut().filter(|r| routes.contains(&r.id)).for_each(|r| r.enabled = on);
                }
            }
            ControlAction::RouteGroup { group } => {
                let mut show = self.show.write().unwrap();
                let current = show.routes.iter().any(|r| &r.group == group && r.enabled);
                if let Some(on) = fader_or_button(mode, pressed, value, current) {
                    show.routes.iter_mut().filter(|r| &r.group == group).for_each(|r| r.enabled = on);
                }
            }
            ControlAction::Bands { bands } => {
                let mut show = self.show.write().unwrap();
                let current = show.audio.bands.iter().find(|b| bands.contains(&b.id)).map(|b| b.enabled).unwrap_or(true);
                if let Some(on) = fader_or_button(mode, pressed, value, current) {
                    show.audio.bands.iter_mut().filter(|b| bands.contains(&b.id)).for_each(|b| b.enabled = on);
                    self.audio.set(show.audio.gain_db, show.audio.sensitivity, &show.audio.bands);
                }
            }
            ControlAction::BandSensitivity { band } => {
                if let Some(v) = value {
                    let mut show = self.show.write().unwrap();
                    if let Some(b) = show.audio.bands.iter_mut().find(|b| &b.id == band) {
                        b.sensitivity = v.clamp(0.0, 1.0);
                    }
                    self.audio.set(show.audio.gain_db, show.audio.sensitivity, &show.audio.bands);
                }
            }
            ControlAction::Blackout {} => {
                let current = self.blackout.load(Ordering::Relaxed);
                if let Some(on) = fader_or_button(mode, pressed, value, current) {
                    self.blackout.store(on, Ordering::Relaxed);
                }
            }
            ControlAction::Master {} => {
                let current = f32::from_bits(self.master.load(Ordering::Relaxed)) > 0.5;
                let v = match value {
                    Some(v) => Some(v),
                    None => Self::button(mode, pressed, current).map(|on| if on { 1.0 } else { 0.0 }),
                };
                if let Some(v) = v {
                    self.master.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
                }
            }
            ControlAction::Preset { preset } => {
                let current = self.active_presets.lock().unwrap().contains(preset);
                // Presets: toggle/momentary can take a look off again; "on" always applies.
                let state = if mode == "on" || mode == "" { pressed.then_some(true) } else { fader_or_button(mode, pressed, value, current) };
                if let Some(on) = state {
                    self.apply_preset(preset, on);
                    if on {
                        self.picked_scene(preset);
                    }
                }
            }
            ControlAction::Autopilot {} => {
                if let Some(on) = fader_or_button(mode, pressed, value, self.autopilot.running()) {
                    self.set_autopilot(on);
                }
            }
            ControlAction::ForceSection { section } => {
                let current = self.autopilot.forced() == Some(section.as_str());
                match fader_or_button(mode, pressed, value, current) {
                    Some(true) => self.autopilot.force(Some(section)),
                    // Letting go only clears it if this pad set it.
                    Some(false) if current => self.autopilot.force(None),
                    _ => {}
                }
            }
            ControlAction::ReleaseManual {} => {
                if pressed {
                    self.manual.lock().unwrap().clear();
                    self.active_presets.lock().unwrap().clear();
                }
            }
            ControlAction::ChannelValue { target, channel, min, max } => {
                let keys = self.target_keys(target, channel);
                let mut manual = self.manual.lock().unwrap();
                match value {
                    // fader: scale into min..max
                    Some(v) => {
                        let val = (*min as f32 + (*max as f32 - *min as f32) * v.clamp(0.0, 1.0)).round() as u8;
                        for k in keys {
                            manual.insert(k, val);
                        }
                    }
                    _ => {
                        let mut toggles = self.toggles.lock().unwrap();
                        let current = toggles.contains(&m.id);
                        if let Some(on) = Self::button(mode, pressed, current) {
                            if on {
                                toggles.insert(m.id.clone());
                                for k in keys {
                                    manual.insert(k, *max);
                                }
                            } else {
                                toggles.remove(&m.id);
                                for k in keys {
                                    manual.remove(&k);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// You picked a scene: the autopilot follows it and pauses.
    fn picked_scene(&self, id: &str) {
        let pause = self.show.read().unwrap().autopilot.pause_s;
        self.autopilot.manual_scene(id, pause);
    }

    fn set_autopilot(&self, on: bool) {
        let current = self.last_scene.lock().unwrap().clone();
        self.autopilot.set_running(on, current);
    }

    /// Is a scene showing? A scene: while it's the one playing. A preset of manual values:
    /// while recalled.
    fn scene_on(p: &Preset, active: &HashSet<String>, playing: Option<&str>) -> bool {
        if p.is_scene() { playing == Some(p.id.as_str()) } else { active.contains(&p.id) }
    }

    /// The pads to light for MIDI feedback, one per MIDI trigger.
    pub fn leds(&self) -> Vec<Led> {
        // Small locks first, then the show: apply_preset holds them while it waits to write the show.
        let active = self.active_presets.lock().unwrap().clone();
        let toggles = self.toggles.lock().unwrap().clone();
        let any_manual = !self.manual.lock().unwrap().is_empty();
        let playing = self.scene.lock().unwrap().as_ref().map(|(id, _)| id.clone());
        let show = self.show.read().unwrap();
        let blackout = self.blackout.load(Ordering::Relaxed);
        let master = f32::from_bits(self.master.load(Ordering::Relaxed));
        let autopilot = self.autopilot.running();
        let forced = self.autopilot.forced();
        let mut out: Vec<Led> = vec![];
        for m in show.controls.iter().filter(|m| m.enabled && m.trigger.source == "midi" && m.trigger.kind != "program") {
            // Faders have no light, except on/off actions on CC buttons.
            if m.trigger.mode == "fader" && matches!(m.action, ControlAction::Master {} | ControlAction::BandSensitivity { .. } | ControlAction::ChannelValue { .. }) {
                continue;
            }
            let (lit, auto) = match &m.action {
                ControlAction::Preset { preset } => {
                    let p = show.presets.iter().find(|p| &p.id == preset);
                    (p.is_some_and(|p| Self::scene_on(p, &active, playing.as_deref())), p.map(|p| p.color.clone()).unwrap_or_default())
                }
                ControlAction::Routes { routes } => {
                    let rs: Vec<&Route> = show.routes.iter().filter(|r| routes.contains(&r.id)).collect();
                    let strobe = !rs.is_empty() && rs.iter().all(|r| matches!(r.action, RouteAction::Strobe { .. }));
                    (rs.iter().any(|r| r.enabled), if strobe { "#ffffff" } else { "#4fb3ff" }.to_string())
                }
                ControlAction::RouteGroup { group } => (show.routes.iter().any(|r| &r.group == group && r.enabled), "#45d39a".into()),
                ControlAction::Bands { bands } => (
                    show.audio.bands.iter().filter(|b| bands.contains(&b.id)).all(|b| b.enabled),
                    show.audio.bands.iter().find(|b| bands.contains(&b.id)).map(|b| b.color.clone()).unwrap_or_else(|| "#f2b54a".into()),
                ),
                ControlAction::Blackout {} => (blackout, "#ff0000".into()),
                ControlAction::Master {} => (master > 0.5, "#ffffff".into()),
                ControlAction::ReleaseManual {} => (any_manual, "#f2b54a".into()),
                ControlAction::ChannelValue { channel, .. } => {
                    (toggles.contains(&m.id), if channel == "@fog" { "#4fd8ff" } else { "#f2b54a" }.into())
                }
                ControlAction::BandSensitivity { .. } => (false, "#f2b54a".into()),
                ControlAction::Autopilot {} => (autopilot, "#7c5cff".into()),
                ControlAction::ForceSection { section } => (forced == Some(section.as_str()), section_color(section).into()),
            };
            let color = parse_hex(if m.color.is_empty() { &auto } else { &m.color });
            let led = Led { kind: m.trigger.kind.clone(), channel: m.trigger.channel, number: m.trigger.number, lit, color };
            // Two controls on one pad: lit if either is on.
            match out.iter_mut().find(|l| l.key() == led.key()) {
                Some(l) => {
                    if lit && !l.lit {
                        *l = led;
                    }
                }
                None => out.push(led),
            }
        }
        out
    }

    fn target_keys(&self, target: &Target, channel: &str) -> Vec<Key> {
        let show = self.show.read().unwrap();
        let profiles: HashMap<&str, &Profile> = show.profiles.iter().map(|p| (p.id.as_str(), p)).collect();
        resolve_fixtures(&show, &target.rooms, &target.groups, &target.fixtures)
            .into_iter()
            .flat_map(|fx| channel_keys(&profiles, fx, channel).into_iter().map(move |k| (fx.id.clone(), k)))
            .collect()
    }

    fn apply_preset(&self, id: &str, on: bool) {
        let Some(p) = self.show.read().unwrap().presets.iter().find(|p| p.id == id).cloned() else { return };
        let mut manual = self.manual.lock().unwrap();
        let mut active = self.active_presets.lock().unwrap();
        if on {
            if p.exclusive {
                manual.clear();
                active.clear();
            }
            for v in &p.values {
                manual.insert((v.fixture.clone(), v.channel.clone()), v.value);
            }
            if !p.routes.is_empty() {
                let mut show = self.show.write().unwrap();
                for r in show.routes.iter_mut() {
                    if let Some(on) = p.routes.get(&r.id) {
                        r.enabled = *on;
                    }
                }
            }
            active.insert(p.id.clone());
            *self.last_scene.lock().unwrap() = Some(p.id.clone());
            if p.is_scene() {
                *self.scene.lock().unwrap() = Some((p.id.clone(), std::time::Instant::now()));
            }
        } else {
            let mut sc = self.scene.lock().unwrap();
            if sc.as_ref().is_some_and(|(id, _)| id == &p.id) {
                *sc = None;
            }
            drop(sc);
            for v in &p.values {
                manual.remove(&(v.fixture.clone(), v.channel.clone()));
            }
            active.remove(&p.id);
        }
    }

    pub fn state(&self, fired: Option<String>) -> ControlState {
        let mut manual: HashMap<String, HashMap<String, u8>> = HashMap::new();
        for ((f, c), v) in self.manual.lock().unwrap().iter() {
            manual.entry(f.clone()).or_default().insert(c.clone(), *v);
        }
        let active_presets = self.active_presets.lock().unwrap().iter().cloned().collect();
        let scene = self.scene.lock().unwrap().as_ref().map(|(id, _)| id.clone());
        let show = self.show.read().unwrap();
        ControlState {
            routes: show.routes.iter().map(|r| (r.id.clone(), r.enabled)).collect(),
            bands: show
                .audio
                .bands
                .iter()
                .map(|b| (b.id.clone(), BandState { enabled: b.enabled, sensitivity: b.sensitivity }))
                .collect(),
            blackout: self.blackout.load(Ordering::Relaxed),
            master: f32::from_bits(self.master.load(Ordering::Relaxed)),
            manual,
            active_presets,
            scene,
            autopilot: self.autopilot.running(),
            fired,
        }
    }

    fn emit_state(&self, fired: Option<String>) {
        let _ = self.app.emit("control_state", self.state(fired));
    }
}

/// Section colours, as in the UI.
pub fn section_color(s: &str) -> &'static str {
    match s {
        "breakdown" => "#4fb3ff",
        "build" => "#f2b54a",
        "drop" => "#ff5c6c",
        _ => "#45d39a",
    }
}

/// On/off actions: a CC fader acts as a switch (top half = on); buttons use the mode.
fn fader_or_button(mode: &str, pressed: bool, value: Option<f32>, current: bool) -> Option<bool> {
    match value {
        Some(v) if mode.is_empty() || mode == "fader" => Some(v >= 0.5),
        _ => Controls::button(mode, pressed, current),
    }
}

fn midi_matches(t: &ControlTrigger, ev: &MidiEvent) -> bool {
    t.source == "midi" && t.kind == ev.kind && t.number == ev.number && (t.channel == 0 || t.channel == ev.channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_modes() {
        assert_eq!(Controls::button("toggle", true, false), Some(true));
        assert_eq!(Controls::button("toggle", false, true), None);
        assert_eq!(Controls::button("momentary", true, false), Some(true));
        assert_eq!(Controls::button("momentary", false, true), Some(false));
        assert_eq!(Controls::button("off", true, true), Some(false));
        assert_eq!(Controls::button("on", false, false), None);
    }

    #[test]
    fn matching() {
        let t = ControlTrigger { source: "midi".into(), kind: "note".into(), channel: 0, number: 36, ..Default::default() };
        let ev = |ch, n| MidiEvent { port: "x".into(), kind: "note".into(), channel: ch, number: n, value: 100 };
        assert!(midi_matches(&t, &ev(10, 36)));
        assert!(!midi_matches(&t, &ev(10, 37)));
        let t2 = ControlTrigger { channel: 2, ..t };
        assert!(!midi_matches(&t2, &ev(1, 36)));
    }
}
