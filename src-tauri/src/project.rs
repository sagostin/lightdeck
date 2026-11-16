//! Project files (.lightdeck), settings files and app preferences.

use crate::ai::AiSettings;
use crate::model::{BandDef, ControllerProfile, Profile, Show};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PROJECT_EXT: &str = "lightdeck";
pub const SETTINGS_EXT: &str = "lightdeck-settings";
pub const FIXTURE_EXT: &str = "lightdeck-fixture";
const MAX_RECENT: usize = 8;

/// App-level preferences (not part of any project).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Prefs {
    /// Project file the working copy belongs to, if it has been saved/opened.
    pub current: Option<String>,
    pub recent: Vec<String>,
    /// User-saved band presets, available in every project.
    pub band_presets: Vec<BandDef>,
    /// Your own MIDI controller layouts, available in every project.
    pub controller_profiles: Vec<ControllerProfile>,
    /// Fixture library: fixture types (with their looks) that projects copy from.
    pub fixture_library: Vec<Profile>,
}

impl Prefs {
    pub fn touch_recent(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(MAX_RECENT);
    }
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    format: String,
    app_version: String,
    show: Show,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SettingsFile {
    pub format: String,
    pub ai: Option<AiSettings>,
    pub band_presets: Vec<BandDef>,
    pub controller_profiles: Vec<ControllerProfile>,
    pub fixture_library: Vec<Profile>,
}

#[derive(Serialize, Clone)]
pub struct ProjectInfo {
    pub path: Option<String>,
    pub name: String,
    pub recent: Vec<String>,
}

pub fn info(prefs: &Prefs) -> ProjectInfo {
    ProjectInfo {
        path: prefs.current.clone(),
        name: prefs
            .current
            .as_deref()
            .and_then(|p| Path::new(p).file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into()),
        recent: prefs.recent.clone(),
    }
}

pub fn write_project(path: &Path, show: &Show) -> Result<(), String> {
    let file = ProjectFile {
        format: "lightdeck-project".into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        show: show.clone(),
    };
    write_atomic(path, &serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?)
}

/// Reads a project file. Also accepts a bare show JSON (e.g. the working copy).
pub fn read_project(path: &Path) -> Result<Show, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let mut show = match serde_json::from_slice::<ProjectFile>(&bytes) {
        Ok(f) => f.show,
        Err(_) => serde_json::from_slice::<Show>(&bytes)
            .map_err(|e| format!("{} isn't a LightDeck project: {e}", path.display()))?,
    };
    show.migrate();
    Ok(show)
}

pub fn write_settings(
    path: &Path,
    ai: &AiSettings,
    include_key: bool,
    presets: &[BandDef],
    controllers: &[ControllerProfile],
    library: &[Profile],
) -> Result<(), String> {
    let mut ai = ai.clone();
    if !include_key {
        ai.api_key.clear();
    }
    let file = SettingsFile { format: "lightdeck-settings".into(), ai: Some(ai), band_presets: presets.to_vec(), controller_profiles: controllers.to_vec(), fixture_library: library.to_vec() };
    write_atomic(path, &serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?)
}

pub fn read_settings(path: &Path) -> Result<SettingsFile, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let f: SettingsFile = serde_json::from_slice(&bytes).map_err(|e| format!("Not a LightDeck settings file: {e}"))?;
    if f.format != "lightdeck-settings" {
        return Err("Not a LightDeck settings file".into());
    }
    Ok(f)
}

/// A fixture-type file: one type, or several (a library export).
#[derive(Serialize, Deserialize)]
struct FixtureFile {
    format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<Profile>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    profiles: Vec<Profile>,
}

pub fn fixture_file_json(profiles: &[Profile]) -> Result<Vec<u8>, String> {
    let file = match profiles {
        [one] => FixtureFile { format: "lightdeck-fixture".into(), profile: Some(one.clone()), profiles: vec![] },
        many => FixtureFile { format: "lightdeck-fixtures".into(), profile: None, profiles: many.to_vec() },
    };
    serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())
}

/// Read fixture types from a fixture file, a library export, a settings file's library,
/// a project file, or a bare profile (the old "Copy JSON" format). Missing fields get defaults.
pub fn parse_fixture_file(text: &str) -> Result<Vec<Profile>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("Not JSON: {e}"))?;
    let list = |key: &str| v.get(key).and_then(|x| x.as_array()).cloned();
    let raw: Vec<serde_json::Value> = match v.get("format").and_then(|f| f.as_str()) {
        Some("lightdeck-fixture") => v.get("profile").cloned().into_iter().collect(),
        Some("lightdeck-fixtures") => list("profiles").unwrap_or_default(),
        Some("lightdeck-settings") => list("fixture_library").unwrap_or_default(),
        Some("lightdeck-project") => v.get("show").and_then(|s| s.get("profiles")).and_then(|x| x.as_array()).cloned().unwrap_or_default(),
        Some(other) => return Err(format!("Not a fixture file ({other})")),
        None if v.get("channels").is_some() => vec![v.clone()],
        None => return Err("No fixture types found in this file".into()),
    };
    let profiles: Vec<Profile> = raw
        .into_iter()
        .map(|p| serde_json::from_value::<Profile>(p).map_err(|e| format!("Couldn't read a fixture type: {e}")))
        .collect::<Result<_, _>>()?;
    if profiles.is_empty() {
        return Err("No fixture types found in this file".into());
    }
    if let Some(p) = profiles.iter().find(|p| p.id.trim().is_empty() || p.channels.is_empty()) {
        return Err(format!("\"{}\" has no id or no channels", p.model));
    }
    Ok(profiles)
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp: PathBuf = path.with_extension("tmp-write");
    std::fs::write(&tmp, bytes).map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn with_ext(p: PathBuf, ext: &str) -> PathBuf {
    if p.extension().map(|e| e == ext).unwrap_or(false) { p } else { p.with_extension(ext) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_round_trip() {
        let dir = std::env::temp_dir().join(format!("lightdeck-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.lightdeck");
        let mut show = Show::default();
        show.migrate();
        show.rooms.push(crate::model::Room { id: "r".into(), name: "Room".into(), ..Default::default() });
        write_project(&path, &show).unwrap();
        let back = read_project(&path).unwrap();
        assert_eq!(back.rooms[0].name, "Room");
        assert_eq!(back.audio.bands.len(), show.audio.bands.len());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn settings_strip_key() {
        let dir = std::env::temp_dir().join(format!("lightdeck-set-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.lightdeck-settings");
        let ai = AiSettings { api_key: "secret".into(), ..Default::default() };
        let ctrl = ControllerProfile {
            id: "c".into(),
            name: "Mine".into(),
            pads: vec![crate::model::ControllerInput { kind: "note".into(), number: 36, label: "Pad 1".into() }],
            ..Default::default()
        };
        write_settings(&path, &ai, false, &[], &[ctrl.clone()], &[]).unwrap();
        let back = read_settings(&path).unwrap();
        assert_eq!(back.ai.unwrap().api_key, "");
        assert_eq!(back.controller_profiles, vec![ctrl], "controller profiles travel with settings");
        // files from before controller profiles still read
        std::fs::write(&path, r#"{"format":"lightdeck-settings","band_presets":[]}"#).unwrap();
        assert!(read_settings(&path).unwrap().controller_profiles.is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    fn laser() -> Profile {
        Profile {
            id: "laser".into(),
            model: "L2600".into(),
            channels: vec![crate::model::ChannelDef { key: "pattern".into(), group: "pattern".into(), ..Default::default() }],
            looks: vec![crate::model::Look { id: "l1".into(), name: "Tunnel".into(), tags: vec!["peak".into()], ..Default::default() }],
            ..Default::default()
        }
    }

    #[test]
    fn fixture_files_round_trip() {
        let one = fixture_file_json(&[laser()]).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&one).unwrap();
        assert_eq!(v["format"], "lightdeck-fixture");
        assert!(v.get("profiles").is_none());
        let back = parse_fixture_file(std::str::from_utf8(&one).unwrap()).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].looks[0].tags, vec!["peak"]);

        let mut fog = laser();
        fog.id = "fog".into();
        let many = fixture_file_json(&[laser(), fog]).unwrap();
        let back = parse_fixture_file(std::str::from_utf8(&many).unwrap()).unwrap();
        assert_eq!(back.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["laser", "fog"]);
    }

    #[test]
    fn fixture_types_are_read_from_other_files() {
        // bare profile, as "Copy JSON" used to produce; missing fields get defaults
        let bare = parse_fixture_file(r#"{"id":"par","model":"Par","channels":[{"key":"dimmer"}]}"#).unwrap();
        assert_eq!(bare[0].channels[0].key, "dimmer");
        assert!(bare[0].looks.is_empty() && bare[0].modes.is_empty());
        // a project file
        let show = Show { profiles: vec![laser()], ..Default::default() };
        let proj = serde_json::json!({"format": "lightdeck-project", "app_version": "0", "show": show}).to_string();
        assert_eq!(parse_fixture_file(&proj).unwrap()[0].id, "laser");
        // a settings export's library
        let set = serde_json::json!({"format": "lightdeck-settings", "fixture_library": [laser()]}).to_string();
        assert_eq!(parse_fixture_file(&set).unwrap().len(), 1);
        // things that aren't fixture types
        assert!(parse_fixture_file("not json").is_err());
        assert!(parse_fixture_file(r#"{"format":"lightdeck-settings"}"#).is_err());
        assert!(parse_fixture_file(r#"{"format":"something-else"}"#).unwrap_err().contains("something-else"));
        assert!(parse_fixture_file(r#"{"hello": 1}"#).is_err());
        assert!(parse_fixture_file(r#"{"id":"","channels":[{"key":"a"}]}"#).is_err());
        assert!(parse_fixture_file(r#"{"format":"lightdeck-fixture","profile":{"id":"x","channels":[]}}"#).is_err());
    }

    #[test]
    fn settings_carry_the_fixture_library() {
        let dir = std::env::temp_dir().join(format!("lightdeck-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.lightdeck-settings");
        write_settings(&path, &AiSettings::default(), false, &[], &[], &[laser()]).unwrap();
        assert_eq!(read_settings(&path).unwrap().fixture_library[0].looks.len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }
}
