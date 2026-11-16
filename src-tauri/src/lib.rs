mod ai;
mod designer;
mod artnet;
mod audio;
mod control;
mod midi;
mod engine;
mod model;
mod project;
mod qxf;
mod zigbee;

use ai::AiSettings;
use audio::Audio;
use engine::Engine;
use model::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};
use project::{Prefs, ProjectInfo};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    engine: Engine,
    audio: Audio,
    ai: Mutex<AiSettings>,
    prefs: Mutex<Prefs>,
    zigbee: Arc<zigbee::Zigbee>,
    midi: Arc<midi::Midi>,
    controls: Arc<control::Controls>,
    control_tx: Mutex<std::sync::mpsc::Sender<control::Input>>,
    dir: PathBuf,
}

fn show_path(dir: &PathBuf) -> PathBuf {
    dir.join("show.json")
}
fn ai_path(dir: &PathBuf) -> PathBuf {
    dir.join("ai.json")
}
fn prefs_path(dir: &PathBuf) -> PathBuf {
    dir.join("prefs.json")
}

fn write_json<T: serde::Serialize>(path: &PathBuf, v: &T) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_show(state: State<AppState>) -> Show {
    state.engine.show.read().unwrap().clone()
}

/// Make `show` the live show: engine, audio settings and the autosaved working copy.
fn apply_show(state: &AppState, mut show: Show) -> Result<(), String> {
    show.migrate();
    let device_changed;
    {
        let mut cur = state.engine.show.write().unwrap();
        device_changed = cur.audio.device != show.audio.device;
        *cur = show.clone();
    }
    state.audio.params.set(show.audio.gain_db, show.audio.sensitivity, &show.audio.bands);
    state.zigbee.configure(&show.zigbee);
    state.midi.configure(&show.midi);
    if device_changed {
        state.audio.select_device(show.audio.device.clone());
    }
    write_json(&show_path(&state.dir), &show)
}

#[tauri::command]
fn save_show(state: State<AppState>, show: Show) -> Result<(), String> {
    apply_show(&state, show)
}

fn save_prefs(state: &AppState) -> Result<(), String> {
    let p = state.prefs.lock().unwrap().clone();
    write_json(&prefs_path(&state.dir), &p)
}

#[derive(serde::Serialize)]
struct Opened {
    show: Show,
    info: ProjectInfo,
}

#[tauri::command]
fn project_info(state: State<AppState>) -> ProjectInfo {
    project::info(&state.prefs.lock().unwrap())
}

#[tauri::command]
fn project_new(state: State<AppState>) -> Result<Opened, String> {
    let mut show = default_show();
    show.migrate();
    state.engine.manual.lock().unwrap().clear();
    apply_show(&state, show.clone())?;
    state.prefs.lock().unwrap().current = None;
    save_prefs(&state)?;
    Ok(Opened { show, info: project::info(&state.prefs.lock().unwrap()) })
}

/// Open a project; with no path, ask with a file dialog. Returns None if cancelled.
#[tauri::command]
async fn project_open(app: AppHandle, state: State<'_, AppState>, path: Option<String>) -> Result<Option<Opened>, String> {
    let path = match path {
        Some(p) => PathBuf::from(p),
        None => match app
            .dialog()
            .file()
            .set_title("Open LightDeck project")
            .add_filter("LightDeck project", &[project::PROJECT_EXT])
            .blocking_pick_file()
        {
            Some(fp) => fp.into_path().map_err(|e| e.to_string())?,
            None => return Ok(None),
        },
    };
    let show = match project::read_project(&path) {
        Ok(s) => s,
        Err(e) => {
            // Drop missing files from the recent list.
            if !path.exists() {
                let p = path.to_string_lossy().into_owned();
                state.prefs.lock().unwrap().recent.retain(|r| r != &p);
                save_prefs(&state)?;
            }
            return Err(e);
        }
    };
    state.engine.manual.lock().unwrap().clear();
    apply_show(&state, show.clone())?;
    {
        let mut p = state.prefs.lock().unwrap();
        let s = path.to_string_lossy().into_owned();
        p.current = Some(s.clone());
        p.touch_recent(&s);
    }
    save_prefs(&state)?;
    Ok(Some(Opened { show, info: project::info(&state.prefs.lock().unwrap()) }))
}

/// Save the live show to its project file, or ask where (always when `save_as`).
#[tauri::command]
async fn project_save(app: AppHandle, state: State<'_, AppState>, save_as: bool) -> Result<Option<ProjectInfo>, String> {
    let current = state.prefs.lock().unwrap().current.clone();
    let path = match current {
        Some(p) if !save_as => PathBuf::from(p),
        _ => {
            let name = project::info(&state.prefs.lock().unwrap()).name;
            match app
                .dialog()
                .file()
                .set_title("Save LightDeck project")
                .set_file_name(format!("{name}.{}", project::PROJECT_EXT))
                .add_filter("LightDeck project", &[project::PROJECT_EXT])
                .blocking_save_file()
            {
                Some(fp) => project::with_ext(fp.into_path().map_err(|e| e.to_string())?, project::PROJECT_EXT),
                None => return Ok(None),
            }
        }
    };
    let show = state.engine.show.read().unwrap().clone();
    project::write_project(&path, &show)?;
    {
        let mut p = state.prefs.lock().unwrap();
        let s = path.to_string_lossy().into_owned();
        p.current = Some(s.clone());
        p.touch_recent(&s);
    }
    save_prefs(&state)?;
    Ok(Some(project::info(&state.prefs.lock().unwrap())))
}

#[tauri::command]
async fn settings_export(app: AppHandle, state: State<'_, AppState>, include_key: bool) -> Result<Option<String>, String> {
    let Some(fp) = app
        .dialog()
        .file()
        .set_title("Export LightDeck settings")
        .set_file_name(format!("LightDeck.{}", project::SETTINGS_EXT))
        .add_filter("LightDeck settings", &[project::SETTINGS_EXT])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = project::with_ext(fp.into_path().map_err(|e| e.to_string())?, project::SETTINGS_EXT);
    let ai = state.ai.lock().unwrap().clone();
    let presets = state.prefs.lock().unwrap().band_presets.clone();
    project::write_settings(&path, &ai, include_key, &presets)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Import settings. AI settings are replaced (keeping the current key if the file has none);
/// band presets are merged by name.
#[tauri::command]
async fn settings_import(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let Some(fp) = app
        .dialog()
        .file()
        .set_title("Import LightDeck settings")
        .add_filter("LightDeck settings", &[project::SETTINGS_EXT, "json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let f = project::read_settings(&fp.into_path().map_err(|e| e.to_string())?)?;
    let mut summary = vec![];
    if let Some(mut ai) = f.ai {
        let mut cur = state.ai.lock().unwrap();
        if ai.api_key.is_empty() {
            ai.api_key = cur.api_key.clone();
        }
        *cur = ai;
        write_json(&ai_path(&state.dir), &*cur)?;
        summary.push("AI settings".to_string());
    }
    if !f.band_presets.is_empty() {
        let mut p = state.prefs.lock().unwrap();
        for b in &f.band_presets {
            p.band_presets.retain(|x| x.name != b.name);
            p.band_presets.push(b.clone());
        }
        summary.push(format!("{} band preset(s)", f.band_presets.len()));
    }
    save_prefs(&state)?;
    Ok(Some(if summary.is_empty() { "Nothing to import".into() } else { format!("Imported {}", summary.join(" and ")) }))
}

#[tauri::command]
fn zigbee_status(state: State<AppState>) -> zigbee::Status {
    state.zigbee.status()
}

#[tauri::command]
fn zigbee_reconnect(state: State<AppState>) {
    state.zigbee.reconnect();
}

#[tauri::command]
async fn zigbee_identify(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let z = state.zigbee.clone();
    tauri::async_runtime::spawn_blocking(move || z.identify(&name)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn zigbee_ports(state: State<'_, AppState>) -> Result<Vec<zigbee::Port>, String> {
    let z = state.zigbee.clone();
    tauri::async_runtime::spawn_blocking(move || z.ports()).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn zigbee_permit_join(state: State<'_, AppState>, seconds: u32) -> Result<(), String> {
    let z = state.zigbee.clone();
    tauri::async_runtime::spawn_blocking(move || z.permit_join(seconds)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn zigbee_remove(state: State<'_, AppState>, ieee: String, force: bool) -> Result<(), String> {
    let z = state.zigbee.clone();
    tauri::async_runtime::spawn_blocking(move || z.remove(&ieee, force)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn zigbee_reinterview(state: State<'_, AppState>, ieee: String) -> Result<(), String> {
    let z = state.zigbee.clone();
    tauri::async_runtime::spawn_blocking(move || z.reinterview(&ieee)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn zigbee_profiles() -> Vec<Profile> {
    zigbee::builtin_profiles()
}

fn control_send(state: &AppState, input: control::Input) {
    let _ = state.control_tx.lock().unwrap().send(input);
}

#[tauri::command]
fn midi_status(state: State<AppState>) -> midi::MidiStatus {
    state.midi.status()
}

/// Keyboard input from the window (ignored while typing in a field).
#[tauri::command]
fn control_key(state: State<AppState>, key: String, down: bool) {
    control_send(&state, control::Input::Key { key, down });
}

/// On-screen test button / fader for a mapping.
#[tauri::command]
fn control_fire(state: State<AppState>, mapping: String, pressed: bool, value: Option<f32>) {
    control_send(&state, control::Input::Fire { mapping, pressed, value });
}

#[tauri::command]
fn preset_apply(state: State<AppState>, preset: String) {
    control_send(&state, control::Input::Preset { preset });
}

#[tauri::command]
fn set_master(state: State<AppState>, value: f32) {
    control_send(&state, control::Input::Master { value });
}

#[tauri::command]
fn control_state(state: State<AppState>) -> control::ControlState {
    state.controls.state(None)
}

#[tauri::command]
fn get_band_presets(state: State<AppState>) -> Vec<BandDef> {
    state.prefs.lock().unwrap().band_presets.clone()
}

#[tauri::command]
fn save_band_presets(state: State<AppState>, presets: Vec<BandDef>) -> Result<(), String> {
    state.prefs.lock().unwrap().band_presets = presets;
    save_prefs(&state)
}

#[tauri::command]
fn list_audio_devices() -> Vec<String> {
    audio::list_devices()
}

#[tauri::command]
fn restart_audio(state: State<AppState>) {
    let dev = state.engine.show.read().unwrap().audio.device.clone();
    state.audio.select_device(dev);
}

#[tauri::command]
fn set_manual(state: State<AppState>, fixture: String, channel: String, value: Option<u8>) {
    let mut m = state.engine.manual.lock().unwrap();
    match value {
        Some(v) => m.insert((fixture, channel), v),
        None => m.remove(&(fixture, channel)),
    };
}

/// Give channels back to the routes: clears manual values and any look previews.
#[tauri::command]
fn clear_manual(state: State<AppState>, fixture: Option<String>) {
    let mut m = state.engine.manual.lock().unwrap();
    let mut p = state.engine.preview.lock().unwrap();
    match fixture {
        Some(f) => {
            m.retain(|(fx, _), _| fx != &f);
            p.remove(&f);
        }
        None => {
            m.clear();
            p.clear();
        }
    }
}

/// Stop every look preview (called when the UI starts).
#[tauri::command]
fn clear_previews(state: State<AppState>) {
    state.engine.preview.lock().unwrap().clear();
}

#[tauri::command]
fn get_manual(state: State<AppState>) -> HashMap<String, HashMap<String, u8>> {
    let mut out: HashMap<String, HashMap<String, u8>> = HashMap::new();
    for ((f, c), v) in state.engine.manual.lock().unwrap().iter() {
        out.entry(f.clone()).or_default().insert(c.clone(), *v);
    }
    out
}

#[tauri::command]
fn set_blackout(state: State<AppState>, on: bool) {
    state.engine.blackout.store(on, Ordering::Relaxed);
}

#[tauri::command]
fn import_qxf(xml: String) -> Result<Profile, String> {
    qxf::import(&xml)
}

/// Preview a look (with its motions) on a fixture; `None` stops previewing it.
#[tauri::command]
fn preview_look(state: State<AppState>, fixture: String, look: Option<Look>) {
    let mut p = state.engine.preview.lock().unwrap();
    let now = std::time::Instant::now();
    match look {
        Some(l) => {
            // keep the start time while editing so sweeps don't restart on every change;
            // the UI re-sends every second, which keeps the preview alive
            let started = p.get(&fixture).map(|(_, t, _)| *t).unwrap_or(now);
            p.insert(fixture, (l, started, now));
        }
        None => {
            p.remove(&fixture);
        }
    }
}

/// Regenerate a fixture type's default looks (your own looks are kept).
#[tauri::command]
fn generate_default_looks(mut profile: Profile, per_range: usize) -> Profile {
    profile.generate_default_looks(per_range);
    profile
}

#[tauri::command]
fn get_ai_settings(state: State<AppState>) -> AiSettings {
    state.ai.lock().unwrap().clone()
}

#[tauri::command]
fn save_ai_settings(state: State<AppState>, settings: AiSettings) -> Result<(), String> {
    *state.ai.lock().unwrap() = settings.clone();
    write_json(&ai_path(&state.dir), &settings)
}

#[tauri::command]
async fn ai_test(state: State<'_, AppState>) -> Result<String, String> {
    let s = state.ai.lock().unwrap().clone();
    ai::test(&s).await
}

#[tauri::command]
async fn ai_design(state: State<'_, AppState>, show: Show, request: designer::DesignRequest) -> Result<designer::DesignResult, String> {
    let s = state.ai.lock().unwrap().clone();
    designer::design(&s, &show, &request).await
}

#[tauri::command]
async fn ai_interpret(state: State<'_, AppState>, input: String, instruction: String) -> Result<Profile, String> {
    let s = state.ai.lock().unwrap().clone();
    ai::interpret(&s, &input, &instruction).await
}

fn default_show() -> Show {
    let laser = qxf::import(include_str!("../../fixtures/Generic-3D-Animation-RGB-Laser-L2600-005.qxf")).unwrap();
    let fog = qxf::import(include_str!("../../fixtures/dmxbridge.qxf")).unwrap();
    let room = Room {
        id: "room-main".into(),
        name: "Main room".into(),
        walls: vec![[0.0, 0.0], [6.0, 0.0], [6.0, 4.5], [0.0, 4.5]],
        ceiling_m: 2.4,
        focus: vec![FocusPoint { id: "focus-floor".into(), name: "Dance floor".into(), x: 3.0, y: 2.6, ..Default::default() }],
    };
    let output = ArtNetOutput {
        id: "out-1".into(),
        name: "PKnight node".into(),
        ip: "2.0.0.10".into(),
        net: 0,
        subnet: 0,
        universe: 0,
        enabled: false,
    };
    let mut laser_limits = HashMap::new();
    laser_limits.insert("mode".to_string(), Limit { lock: Some(50), ..Default::default() });
    laser_limits.insert("p1_pattern".to_string(), Limit { min_hold_ms: 900, ..Default::default() });
    let fixtures = vec![
        Fixture {
            id: "fx-laser".into(),
            name: "Laser".into(),
            profile_id: laser.id.clone(),
            mode: laser.modes[0].name.clone(),
            output_id: output.id.clone(),
            address: 1,
            room_id: Some(room.id.clone()),
            x: 3.0,
            y: 0.6,
            z: 2.0,
            rotation: 90.0,
            enabled: true,
            zigbee_device: None,
            limits: laser_limits,
        },
        Fixture {
            id: "fx-fog".into(),
            name: "Fog".into(),
            profile_id: fog.id.clone(),
            mode: fog.modes[0].name.clone(),
            output_id: output.id.clone(),
            address: 33,
            room_id: Some(room.id.clone()),
            x: 5.2,
            y: 0.6,
            z: 0.3,
            rotation: 90.0,
            enabled: true,
            zigbee_device: None,
            limits: HashMap::new(),
        },
    ];
    let routes = vec![
        Route {
            id: "r-kick-pattern".into(),
            name: "Kick changes P1 pattern".into(),
            group: "Laser".into(),
            enabled: true,
            source: "kick".into(),
            fixtures: vec!["fx-laser".into()],
            groups: vec![],
            rooms: vec![],
            channel: "p1_pattern".into(),
            focus: String::new(),
            falloff: false,
            action: RouteAction::Random { tags: vec![], hold_ms: 900 },
            cooldown_ms: 0,
            every: 1,
            probability: 1.0,
        },
        Route {
            id: "r-hat-rot".into(),
            name: "Hi-hats drive P1 rotation".into(),
            group: "Laser".into(),
            enabled: true,
            source: "hat.level".into(),
            fixtures: vec!["fx-laser".into()],
            groups: vec![],
            rooms: vec![],
            channel: "p1_rotation".into(),
            focus: String::new(),
            falloff: false,
            action: RouteAction::Modulate { min: 0, max: 127, smoothing: 0.8 },
            cooldown_ms: 0,
            every: 1,
            probability: 1.0,
        },
    ];
    Show {
        version: SHOW_VERSION,
        profiles: vec![laser, fog],
        fixtures,
        rooms: vec![room],
        outputs: vec![output],
        routes,
        audio: AudioSettings::default(),
        zigbee: ZigbeeSettings::default(),
        groups: vec![],
        presets: vec![],
        palettes: default_palettes(),
        controls: vec![],
        midi: MidiSettings::default(),
        active_room: Some("room-main".into()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let mut show: Show = match std::fs::read(show_path(&dir)) {
                Err(_) => default_show(),
                Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                    // Never silently replace a show we couldn't read: keep it aside.
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let aside = dir.join(format!("show.unreadable-{ts}.json"));
                    let _ = std::fs::write(&aside, &bytes);
                    eprintln!("show.json could not be read ({e}); saved a copy to {}", aside.display());
                    default_show()
                }),
            };
            show.migrate();
            let ai: AiSettings = std::fs::read(ai_path(&dir))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let prefs: Prefs = std::fs::read(prefs_path(&dir))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let audio = Audio::start(
                show.audio.device.clone(),
                show.audio.gain_db,
                show.audio.sensitivity,
                &show.audio.bands,
            );
            // Bundled app: resources/zigbee-helper. Dev: the folder next to src-tauri.
            let helper_dir = app
                .path()
                .resource_dir()
                .ok()
                .map(|d| d.join("zigbee-helper"))
                .filter(|d| d.join("index.mjs").exists())
                .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zigbee-helper"));
            let zb = zigbee::Zigbee::start(show.zigbee.clone(), dir.clone(), helper_dir);
            let show = Arc::new(RwLock::new(show));
            let midi_settings = show.read().unwrap().midi.clone();
            let engine = Engine::start(app.handle().clone(), show, audio.features.clone(), zb.clone());
            let (controls, control_tx) = control::Controls::start(
                engine.show.clone(),
                engine.manual.clone(),
                engine.blackout.clone(),
                engine.master.clone(),
                audio.params.clone(),
                app.handle().clone(),
            );
            let (midi_tx, midi_rx) = std::sync::mpsc::channel::<midi::MidiEvent>();
            let fwd = control_tx.clone();
            std::thread::spawn(move || {
                while let Ok(ev) = midi_rx.recv() {
                    let _ = fwd.send(control::Input::Midi(ev));
                }
            });
            let midi = midi::Midi::start(midi_settings, midi_tx);
            app.manage(AppState {
                engine,
                audio,
                ai: Mutex::new(ai),
                prefs: Mutex::new(prefs),
                zigbee: zb,
                midi,
                controls,
                control_tx: Mutex::new(control_tx),
                dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_show,
            save_show,
            list_audio_devices,
            restart_audio,
            set_manual,
            clear_manual,
            get_manual,
            set_blackout,
            import_qxf,
            generate_default_looks,
            preview_look,
            clear_previews,
            get_ai_settings,
            save_ai_settings,
            ai_test,
            ai_interpret,
            ai_design,
            project_info,
            project_new,
            project_open,
            project_save,
            settings_export,
            settings_import,
            get_band_presets,
            save_band_presets,
            zigbee_status,
            zigbee_reconnect,
            zigbee_identify,
            zigbee_profiles,
            zigbee_ports,
            zigbee_permit_join,
            zigbee_remove,
            zigbee_reinterview,
            midi_status,
            control_key,
            control_fire,
            preset_apply,
            set_master,
            control_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running LightDeck");
}
