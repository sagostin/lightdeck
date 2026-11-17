//! Autopilot: works out which part of a track is playing (breakdown, groove, build, drop)
//! once per bar, and switches to a scene tagged for it.
//!
//! The detector compares each bar's loudness with the range heard over the last couple of
//! minutes, watches whether the kick is playing and whether loudness is rising. The pilot
//! keeps a scene for a minimum number of bars, lets a drop cut in sooner, moves to another
//! scene for the same section after a while, and stays out of the way for a time after you
//! pick a scene yourself.

use crate::audio::Features;
use crate::control::Input;
use crate::model::{AutopilotSettings, Show};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// What one bar sounded like.
#[derive(Debug, Clone, Copy, Default)]
pub struct BarStats {
    /// Mean overall level, 0..1.
    pub level: f32,
    /// Hits of the first band (normally the kick) in the bar.
    pub kicks: u32,
}

const HISTORY: usize = 64;

#[derive(Debug, Clone)]
pub struct Detector {
    levels: VecDeque<f32>,
    pub section: &'static str,
    candidate: Option<(&'static str, u32)>,
    no_kick: u32,
    /// Where this bar sits between the quiet and loud parts heard recently, 0..1.
    pub rel: f32,
}

impl Default for Detector {
    fn default() -> Self {
        Self { levels: VecDeque::new(), section: "groove", candidate: None, no_kick: 0, rel: 0.5 }
    }
}

fn percentile(v: &[f32], p: f32) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[((s.len() - 1) as f32 * p).round() as usize]
}

impl Detector {
    /// Feed one bar; returns the section, which changes only once a new one is confirmed
    /// (a drop at once, anything else after two bars).
    pub fn bar(&mut self, b: BarStats) -> &'static str {
        self.levels.push_back(b.level);
        while self.levels.len() > HISTORY {
            self.levels.pop_front();
        }
        let v: Vec<f32> = self.levels.iter().copied().collect();
        let (lo, hi) = (percentile(&v, 0.1), percentile(&v, 0.9));
        let mid = percentile(&v, 0.5);
        self.rel = if v.len() < 4 {
            0.6
        } else if hi - lo < 0.03 {
            // Everything so far sounded the same: only a clear change counts.
            if b.level > mid + 0.05 { 1.0 } else if b.level < mid - 0.05 { 0.0 } else { 0.5 }
        } else {
            ((b.level - lo) / (hi - lo)).clamp(0.0, 1.0)
        };
        let range = (hi - lo).max(0.03);
        let n = v.len();
        let trend = if n >= 6 {
            let recent = (v[n - 1] + v[n - 2]) / 2.0;
            let before = v[n - 6..n - 2].iter().sum::<f32>() / 4.0;
            (recent - before) / range
        } else {
            0.0
        };
        let kick = b.kicks >= 2;
        self.no_kick = if kick { 0 } else { self.no_kick + 1 };

        let raw = if kick && (self.rel >= 0.65 || (self.section == "drop" && self.rel >= 0.4)) {
            "drop"
        } else if trend >= 0.15 && self.rel >= 0.3 {
            "build"
        } else if self.no_kick >= 2 || self.rel < 0.2 {
            "breakdown"
        } else {
            "groove"
        };
        if raw == self.section {
            self.candidate = None;
        } else {
            let count = match self.candidate {
                Some((s, c)) if s == raw => c + 1,
                _ => 1,
            };
            if raw == "drop" || count >= 2 {
                self.section = raw;
                self.candidate = None;
            } else {
                self.candidate = Some((raw, count));
            }
        }
        self.section
    }
}

/// A scene the autopilot may use and the sections it's for.
#[derive(Debug, Clone)]
pub struct Scene {
    pub id: String,
    pub sections: Vec<String>,
}

pub fn scenes(show: &Show) -> Vec<Scene> {
    show.presets
        .iter()
        .filter(|p| !p.sections.is_empty())
        .map(|p| Scene { id: p.id.clone(), sections: p.sections.clone() })
        .collect()
}

/// Sections to fall back on, in order, when no scene is tagged for one.
fn fallbacks(section: &str) -> &'static [&'static str] {
    match section {
        "drop" => &["drop", "build", "groove"],
        "build" => &["build", "groove", "drop"],
        "breakdown" => &["breakdown", "groove"],
        _ => &["groove", "breakdown", "build"],
    }
}

#[derive(Debug, Clone, Default)]
pub struct Pilot {
    pub detector: Detector,
    pub scene: Option<String>,
    pub bars_in_scene: u32,
    recent: VecDeque<String>,
    seed: u64,
}

impl Pilot {
    pub fn new(seed: u64) -> Self {
        Self { seed, ..Default::default() }
    }

    fn random(&mut self) -> f32 {
        // splitmix64
        self.seed = self.seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A scene was picked (by you or by the pilot).
    pub fn set_scene(&mut self, id: &str) {
        self.scene = Some(id.to_string());
        self.bars_in_scene = 0;
        self.recent.push_back(id.to_string());
        while self.recent.len() > 2 {
            self.recent.pop_front();
        }
    }

    /// One bar. Returns the scene to switch to and why, if it's time.
    pub fn on_bar(&mut self, b: BarStats, scenes: &[Scene], s: &AutopilotSettings, paused: bool) -> Option<(String, String)> {
        if b.level < 0.02 {
            return None; // silence: nothing to follow
        }
        let section = self.detector.bar(b);
        self.bars_in_scene += 1;
        if paused || scenes.is_empty() {
            return None;
        }
        let (wanted, fits): (&str, Vec<&Scene>) = fallbacks(section)
            .iter()
            .map(|sec| (*sec, scenes.iter().filter(|x| x.sections.iter().any(|y| y == sec)).collect::<Vec<_>>()))
            .find(|(_, v)| !v.is_empty())?;
        let current = self.scene.clone();
        let current_fits = current.as_ref().is_some_and(|c| fits.iter().any(|x| &x.id == c));
        let reason = if current.is_none() || !scenes.iter().any(|x| Some(&x.id) == current.as_ref()) {
            format!("starting on {wanted}")
        } else if !current_fits {
            let wait = if section == "drop" { s.drop_bars.min(s.min_bars) } else { s.min_bars };
            if self.bars_in_scene < wait.max(1) {
                return None;
            }
            if section == wanted { format!("{section} detected") } else { format!("{section} detected; no {section} scene, using {wanted}") }
        } else if s.max_bars > 0 && self.bars_in_scene >= s.max_bars && fits.len() > 1 {
            format!("{} bars of {wanted}, changing it up", self.bars_in_scene)
        } else {
            return None;
        };
        let others: Vec<&Scene> = fits.iter().copied().filter(|x| Some(&x.id) != current.as_ref()).collect();
        let pool = if others.is_empty() { fits } else { others };
        let fresh: Vec<&Scene> = pool.iter().copied().filter(|x| !self.recent.contains(&x.id)).collect();
        let pool = if fresh.is_empty() { pool } else { fresh };
        let pick = pool[((self.random() * pool.len() as f32) as usize).min(pool.len() - 1)].id.clone();
        if Some(&pick) == current.as_ref() {
            return None;
        }
        self.set_scene(&pick);
        Some((pick, reason))
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Status {
    pub running: bool,
    /// The section heard now.
    pub section: String,
    /// Sections of the last bars, oldest first.
    pub history: Vec<String>,
    pub scene: Option<String>,
    pub bars_in_scene: u32,
    /// Seconds left of the pause after you picked a scene yourself.
    pub paused_s: u32,
    /// Why the last switch happened.
    pub reason: String,
    /// How many scenes are tagged for the autopilot.
    pub scenes: usize,
    /// Where this bar sits between the quiet and loud parts, 0..1.
    pub loudness: f32,
}

struct Inner {
    pilot: Pilot,
    running: bool,
    paused_until: Option<Instant>,
    history: VecDeque<String>,
    reason: String,
}

pub struct Autopilot {
    inner: Mutex<Inner>,
    app: Mutex<Option<AppHandle>>,
}

impl Autopilot {
    pub fn new() -> Arc<Self> {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Arc::new(Self {
            inner: Mutex::new(Inner { pilot: Pilot::new(seed), running: false, paused_until: None, history: VecDeque::new(), reason: String::new() }),
            app: Mutex::new(None),
        })
    }

    pub fn status(&self, show: &Show) -> Status {
        let i = self.inner.lock().unwrap();
        let paused_s = i.paused_until.map(|t| t.saturating_duration_since(Instant::now()).as_secs() as u32).unwrap_or(0);
        Status {
            running: i.running,
            section: i.pilot.detector.section.to_string(),
            history: i.history.iter().cloned().collect(),
            scene: i.pilot.scene.clone(),
            bars_in_scene: i.pilot.bars_in_scene,
            paused_s,
            reason: i.reason.clone(),
            scenes: scenes(show).len(),
            loudness: i.pilot.detector.rel,
        }
    }

    pub fn running(&self) -> bool {
        self.inner.lock().unwrap().running
    }

    /// Switch on or off. `current` is the scene playing now, so it isn't replaced straight away.
    pub fn set_running(&self, on: bool, current: Option<String>) {
        let mut i = self.inner.lock().unwrap();
        if on && !i.running {
            i.pilot.scene = None;
            if let Some(c) = current {
                i.pilot.set_scene(&c);
            }
            i.reason = "switched on".into();
        }
        i.running = on;
        i.paused_until = None;
    }

    /// You picked a scene: follow it, and hold off switching for the pause time.
    pub fn manual_scene(&self, id: &str, pause_s: u32) {
        let mut i = self.inner.lock().unwrap();
        i.pilot.set_scene(id);
        if i.running && pause_s > 0 {
            i.paused_until = Some(Instant::now() + Duration::from_secs(pause_s as u64));
            i.reason = "you picked a scene; paused".into();
        }
    }

    pub fn resume(&self) {
        self.inner.lock().unwrap().paused_until = None;
    }

    fn emit(&self, show: &Show) {
        if let Some(app) = self.app.lock().unwrap().as_ref() {
            let _ = app.emit("autopilot", self.status(show));
        }
    }

    /// Follow the audio on a thread of its own, sending scene changes to the controls.
    pub fn start(self: &Arc<Self>, app: AppHandle, show: Arc<RwLock<Show>>, features: Arc<Mutex<Features>>, tx: Sender<Input>) {
        *self.app.lock().unwrap() = Some(app);
        let me = self.clone();
        std::thread::Builder::new()
            .name("autopilot".into())
            .spawn(move || {
                let mut last_bar: Option<u64> = None;
                let mut last_hits: Option<u64> = None;
                let (mut sum, mut count, mut kicks) = (0.0f32, 0u32, 0u32);
                loop {
                    std::thread::sleep(Duration::from_millis(20));
                    let kick_id = show.read().unwrap().audio.bands.first().map(|b| b.id.clone()).unwrap_or_default();
                    let (bar, level, hits) = {
                        let f = features.lock().unwrap();
                        (f.beats / 4, f.level, f.band(&kick_id).map(|b| b.hits).unwrap_or(0))
                    };
                    sum += level;
                    count += 1;
                    if let Some(h) = last_hits {
                        kicks += hits.saturating_sub(h) as u32;
                    }
                    last_hits = Some(hits);
                    if last_bar == Some(bar) {
                        continue;
                    }
                    let first = last_bar.is_none();
                    last_bar = Some(bar);
                    if first || count == 0 {
                        (sum, count, kicks) = (0.0, 0, 0);
                        continue;
                    }
                    let stats = BarStats { level: sum / count as f32, kicks };
                    (sum, count, kicks) = (0.0, 0, 0);
                    let (list, settings) = {
                        let s = show.read().unwrap();
                        (scenes(&s), s.autopilot.clone())
                    };
                    let switch = {
                        let mut i = me.inner.lock().unwrap();
                        if i.paused_until.is_some_and(|t| t <= Instant::now()) {
                            i.paused_until = None;
                        }
                        let paused = !i.running || i.paused_until.is_some();
                        let r = i.pilot.on_bar(stats, &list, &settings, paused);
                        let sec = i.pilot.detector.section.to_string();
                        i.history.push_back(sec);
                        while i.history.len() > 32 {
                            i.history.pop_front();
                        }
                        if let Some((_, why)) = &r {
                            i.reason = why.clone();
                        }
                        r
                    };
                    if let Some((preset, _)) = switch {
                        let _ = tx.send(Input::Autopilot { preset });
                    }
                    me.emit(&show.read().unwrap());
                }
            })
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(level: f32, kicks: u32) -> BarStats {
        BarStats { level, kicks }
    }

    /// A track: intro groove, breakdown, build, drop.
    fn track() -> Vec<BarStats> {
        let mut t = vec![];
        t.extend((0..16).map(|_| bar(0.45, 4)));
        t.extend((0..8).map(|_| bar(0.2, 0)));
        t.extend((0..6).map(|i| bar(0.25 + 0.06 * i as f32, 0)));
        t.extend((0..16).map(|_| bar(0.7, 4)));
        t
    }

    #[test]
    fn detects_the_parts_of_a_track() {
        let mut d = Detector::default();
        let got: Vec<&str> = track().into_iter().map(|b| d.bar(b)).collect();
        assert_eq!(got[10], "groove");
        assert_eq!(got[20], "breakdown", "{got:?}");
        assert!(got[24..30].contains(&"build"), "{got:?}");
        assert_eq!(got[30], "drop", "a drop is recognised in its first bar: {got:?}");
        assert_eq!(got[45], "drop");
    }

    #[test]
    fn one_odd_bar_does_not_change_the_section() {
        let mut d = Detector::default();
        for _ in 0..12 {
            d.bar(bar(0.45, 4));
        }
        d.bar(bar(0.44, 1)); // a fill without kicks
        assert_eq!(d.bar(bar(0.45, 4)), "groove");
    }

    fn scene(id: &str, sections: &[&str]) -> Scene {
        Scene { id: id.into(), sections: sections.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn switches_scenes_with_the_music() {
        let scenes = [scene("calm", &["breakdown"]), scene("main", &["groove"]), scene("rise", &["build"]), scene("peak", &["drop"])];
        let s = AutopilotSettings { min_bars: 4, max_bars: 0, drop_bars: 1, pause_s: 0 };
        let mut p = Pilot::new(7);
        let mut log = vec![];
        for (i, b) in track().into_iter().enumerate() {
            if let Some((id, why)) = p.on_bar(b, &scenes, &s, false) {
                log.push((i, id, why));
            }
        }
        let ids: Vec<&str> = log.iter().map(|(_, id, _)| id.as_str()).collect();
        assert_eq!(ids, ["main", "calm", "rise", "peak"], "{log:?}");
        assert_eq!(log[0].2, "starting on groove");
        assert_eq!(log[3].0, 30, "the drop cuts in at once: {log:?}");
    }

    #[test]
    fn keeps_a_scene_for_the_minimum_bars() {
        let scenes = [scene("a", &["groove"]), scene("b", &["drop"])];
        let s = AutopilotSettings { min_bars: 8, max_bars: 0, drop_bars: 8, pause_s: 0 };
        let mut p = Pilot::new(1);
        p.set_scene("a");
        for _ in 0..3 {
            p.on_bar(bar(0.45, 4), &scenes, &s, false);
        }
        // loud with kicks: a drop, but only 4 bars into the scene
        assert_eq!(p.on_bar(bar(0.9, 4), &scenes, &s, false), None);
        let mut switched = None;
        for _ in 0..6 {
            switched = switched.or(p.on_bar(bar(0.9, 4), &scenes, &s, false));
        }
        assert_eq!(switched.unwrap().0, "b");
    }

    #[test]
    fn falls_back_and_varies() {
        // No drop scene: a drop uses the build scene.
        let scenes = [scene("g1", &["groove"]), scene("g2", &["groove"]), scene("up", &["build"])];
        let s = AutopilotSettings { min_bars: 1, max_bars: 8, drop_bars: 1, pause_s: 0 };
        let mut p = Pilot::new(3);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..40 {
            if let Some((id, _)) = p.on_bar(bar(0.45, 4), &scenes, &s, false) {
                seen.insert(id);
            }
        }
        assert!(seen.contains("g1") && seen.contains("g2") && !seen.contains("up"), "max_bars rotates groove scenes: {seen:?}");
        let mut out = None;
        for _ in 0..3 {
            out = out.or(p.on_bar(bar(1.0, 4), &scenes, &s, false));
        }
        let (id, why) = out.unwrap();
        assert_eq!(id, "up");
        assert!(why.contains("no drop scene"), "{why}");
    }

    #[test]
    fn paused_or_silent_means_no_switching() {
        let scenes = [scene("a", &["groove"])];
        let s = AutopilotSettings::default();
        let mut p = Pilot::new(1);
        assert_eq!(p.on_bar(bar(0.5, 4), &scenes, &s, true), None);
        assert_eq!(p.on_bar(bar(0.0, 0), &scenes, &s, false), None);
        assert_eq!(p.on_bar(bar(0.5, 4), &scenes, &s, false).unwrap().0, "a");
        assert_eq!(p.on_bar(bar(0.5, 4), &[], &s, false), None);
    }
}
