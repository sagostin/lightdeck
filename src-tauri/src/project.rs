//! Project files (.lightdeck), settings files and app preferences.

use crate::ai::AiSettings;
use crate::model::{BandDef, Show};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PROJECT_EXT: &str = "lightdeck";
pub const SETTINGS_EXT: &str = "lightdeck-settings";
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

pub fn write_settings(path: &Path, ai: &AiSettings, include_key: bool, presets: &[BandDef]) -> Result<(), String> {
    let mut ai = ai.clone();
    if !include_key {
        ai.api_key.clear();
    }
    let file = SettingsFile { format: "lightdeck-settings".into(), ai: Some(ai), band_presets: presets.to_vec() };
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
        write_settings(&path, &ai, false, &[]).unwrap();
        assert_eq!(read_settings(&path).unwrap().ai.unwrap().api_key, "");
        std::fs::remove_dir_all(dir).ok();
    }
}
