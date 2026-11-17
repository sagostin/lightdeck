//! Fixed-rate engine: audio features -> routes -> manual overrides -> limits -> DMX.

use crate::artnet::ArtNetSender;
use crate::audio::Features;
use crate::model::*;
use crate::zigbee::{self, Zigbee};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const FPS: u64 = 40;
pub const PREVIEW_TTL: Duration = Duration::from_secs(3);

type Key = (String, String); // (fixture id, channel key)

pub struct Engine {
    pub show: Arc<RwLock<Show>>,
    pub manual: Arc<Mutex<HashMap<Key, u8>>>,
    pub blackout: Arc<AtomicBool>,
    /// Master brightness 0..1 (f32 bits), scales every intensity channel.
    pub master: Arc<std::sync::atomic::AtomicU32>,
    /// Looks being previewed (pattern browser): fixture -> (look, started, last refreshed).
    /// Entries the UI stops refreshing expire after PREVIEW_TTL, so a reload or crash
    /// mid-preview can never leave a fixture stuck on it.
    pub preview: Arc<Mutex<HashMap<String, (Look, Instant, Instant)>>>,
    /// The section playing now (index into `SECTIONS`), set by the autopilot's detector
    /// or a force-section pad. Routes with sections run only in theirs.
    pub section: Arc<AtomicU8>,
    /// The scene playing (recalled last) and when: its looks and base values are the
    /// layer underneath the routes.
    pub scene: Arc<Mutex<Option<(String, Instant)>>>,
    /// Fixtures open in the pattern browser, and when the UI last said so. While open,
    /// routes and the scene leave them alone, so only what you set there shows. Entries the
    /// UI stops refreshing expire after PREVIEW_TTL, like previews.
    pub browsing: Arc<Mutex<HashMap<String, Instant>>>,
}

#[derive(Serialize, Clone)]
pub struct Frame {
    pub features: Features,
    /// fixture id -> values in mode channel order
    pub fixtures: HashMap<String, Vec<u8>>,
    pub blackout: bool,
    pub send_errors: Vec<String>,
    /// Routes another route has switched for now: route id -> on.
    pub switched: HashMap<String, bool>,
}

#[derive(Default)]
struct RouteState {
    last_counter: u64,
    last_level: f32,
    last_fire: Option<Instant>,
    events: u64,
    hop_current: Option<String>,
    hop_recent: Vec<String>,
    cycle_idx: usize,
    smoothed: HashMap<String, f32>,
    // effects
    palette_idx: usize,
    /// fixture -> colour fade (from, to, started)
    colors: HashMap<String, ([u8; 3], [u8; 3], Instant)>,
    /// (fixture, started, colour)
    sparkles: Vec<(String, Instant, [u8; 3])>,
    strobe_until: Option<Instant>,
    /// Look action: fixture -> (look id, held until)
    /// Look action: fixture -> (look id, held until, started, route hits when started)
    looks: HashMap<String, (String, Instant, Instant, u64)>,
    accum: f32,
    last_tick: Option<Instant>,
    /// Beat position (beats + phase) when the route last fired.
    fire_beat: f32,
    /// "Also move" channels with a trigger of their own: channel -> its hits.
    side: HashMap<String, Hits>,
}

/// Hits of a sound, tracked for a route or one of its channels.
#[derive(Default)]
struct Hits {
    last_counter: u64,
    last_level: f32,
    events: u64,
    last: Option<Instant>,
    beat: f32,
}

/// A route switched on or off by another route, until `until` (None = until changed).
/// `base` is the route's own on/off when it was switched: if that changes (a scene, or
/// you), the switch is dropped.
struct Switched {
    on: bool,
    until: Option<Instant>,
    base: bool,
}

/// Did the sound fire since the last frame? Hits: the counter moved. Levels: rose past 60%.
fn detect(src: &str, feat: &Features, last_counter: &mut u64, last_level: &mut f32) -> bool {
    if let Some(c) = source_counter(src, feat) {
        if c != *last_counter {
            let t = *last_counter != 0 || c == 1;
            *last_counter = c;
            return t;
        }
        false
    } else if let Some(l) = source_level(src, feat) {
        let t = l > 0.6 && *last_level <= 0.6;
        *last_level = l;
        t
    } else {
        false
    }
}

/// Keep up with a sound without firing (a route that isn't running), so it doesn't fire
/// on a stale hit the moment it starts.
fn follow(src: &str, feat: &Features, last_counter: &mut u64, last_level: &mut f32) {
    if let Some(c) = source_counter(src, feat) {
        *last_counter = c.max(1);
    } else if let Some(l) = source_level(src, feat) {
        *last_level = l;
    }
}

#[derive(Clone, Copy)]
struct Pulse {
    on: u8,
    off: u8,
    start: Instant,
    decay_ms: u32,
}

#[derive(Default)]
struct Runtime {
    routes: HashMap<String, RouteState>,
    pulses: HashMap<Key, Pulse>,
    held: HashMap<Key, (u8, Instant)>, // random picks: value until
    last_out: HashMap<Key, (u8, Instant)>,
    clock: Option<Instant>,
    switched: HashMap<String, Switched>,
    /// Fixtures open in the pattern browser this frame.
    browsing: std::collections::HashSet<String>,
}

impl Engine {
    pub fn start(app: AppHandle, show: Arc<RwLock<Show>>, features: Arc<Mutex<Features>>, zb: Arc<Zigbee>) -> Self {
        let manual = Arc::new(Mutex::new(HashMap::new()));
        let blackout = Arc::new(AtomicBool::new(false));
        let master = Arc::new(std::sync::atomic::AtomicU32::new(1.0f32.to_bits()));
        let preview = Arc::new(Mutex::new(HashMap::new()));
        let section = Arc::new(AtomicU8::new(1)); // groove
        let scene = Arc::new(Mutex::new(None));
        let browsing = Arc::new(Mutex::new(HashMap::new()));
        let (s, m, b, ms, pv, sec, sc, br) = (show.clone(), manual.clone(), blackout.clone(), master.clone(), preview.clone(), section.clone(), scene.clone(), browsing.clone());
        std::thread::Builder::new()
            .name("engine".into())
            .spawn(move || run(app, s, features, m, b, ms, pv, sec, sc, br, zb))
            .expect("spawn engine");
        Self { show, manual, blackout, master, preview, section, scene, browsing }
    }
}

fn run(
    app: AppHandle,
    show: Arc<RwLock<Show>>,
    features: Arc<Mutex<Features>>,
    manual: Arc<Mutex<HashMap<Key, u8>>>,
    blackout: Arc<AtomicBool>,
    master: Arc<std::sync::atomic::AtomicU32>,
    preview: Arc<Mutex<HashMap<String, (Look, Instant, Instant)>>>,
    section: Arc<AtomicU8>,
    scene: Arc<Mutex<Option<(String, Instant)>>>,
    browsing: Arc<Mutex<HashMap<String, Instant>>>,
    zb: Arc<Zigbee>,
) {
    let mut sender = ArtNetSender::new().ok();
    let mut rt = Runtime::default();
    let period = Duration::from_micros(1_000_000 / FPS);
    let mut next = Instant::now();
    let mut tick: u64 = 0;
    loop {
        next += period;
        let feat = features.lock().unwrap().clone();
        let bo = blackout.load(Ordering::Relaxed);
        let manual_now = manual.lock().unwrap().clone();
        let preview_now: HashMap<String, (Look, Instant)> = {
            let mut p = preview.lock().unwrap();
            p.retain(|_, (_, _, seen)| seen.elapsed() < PREVIEW_TTL);
            p.iter().map(|(k, (l, started, _))| (k.clone(), (l.clone(), *started))).collect()
        };
        rt.browsing = {
            let mut b = browsing.lock().unwrap();
            b.retain(|_, seen| seen.elapsed() < PREVIEW_TTL);
            b.keys().cloned().collect()
        };
        let (universes, fixtures_out) = {
            let show = show.read().unwrap();
            let m = f32::from_bits(master.load(Ordering::Relaxed)).clamp(0.0, 1.0);
            let sec = SECTIONS.get(section.load(Ordering::Relaxed) as usize).copied().unwrap_or("groove");
            let sc = scene.lock().unwrap().clone();
            compute(&show, &feat, &manual_now, &preview_now, bo, m, sec, sc.as_ref().map(|(id, t)| (id.as_str(), *t)), &mut rt)
        };
        let switched: HashMap<String, bool> = rt.switched.iter().map(|(k, v)| (k.clone(), v.on)).collect();
        zb.set_desired(zigbee_states(&show.read().unwrap(), &fixtures_out));
        let mut send_errors = vec![];
        if let Some(s) = sender.as_mut() {
            let show = show.read().unwrap();
            for o in show.outputs.iter().filter(|o| o.enabled) {
                let data = universes.get(&o.id).copied().unwrap_or([0u8; 512]);
                if let Err(e) = s.send(&o.ip, o.port_address(), &data) {
                    send_errors.push(format!("{}: {e}", o.name));
                }
            }
        } else {
            send_errors.push("Could not open UDP socket".into());
        }
        tick += 1;
        if tick % 2 == 0 {
            let _ = app.emit(
                "frame",
                Frame { features: feat, fixtures: fixtures_out, blackout: bo, send_errors, switched },
            );
        }
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else {
            next = now;
        }
    }
}

/// Level sources: "level", "energy", "phase", or "<band id>.level".
fn source_level(src: &str, f: &Features) -> Option<f32> {
    if let Some(id) = src.strip_suffix(".level") {
        return f.band(id).map(|b| b.level);
    }
    Some(match src {
        "level" => f.level,
        "energy" => ((f.energy - 0.6) / 1.2).clamp(0.0, 1.0),
        "phase" => 1.0 - f.beat_phase,
        // older shows: low/mid/high map onto the first three bands
        "low" => f.bands.first()?.level,
        "mid" => f.bands.get(1)?.level,
        "high" => f.bands.get(2)?.level,
        _ => return None,
    })
}

/// Trigger sources: "beat", "bar", or a band id (its hits).
fn source_counter(src: &str, f: &Features) -> Option<u64> {
    match src {
        "beat" => Some(f.beats),
        "bar" => Some(f.beats / 4),
        id => f.band(id).map(|b| b.hits),
    }
}

fn compute(
    show: &Show,
    feat: &Features,
    manual: &HashMap<Key, u8>,
    preview: &HashMap<String, (Look, Instant)>,
    blackout: bool,
    master: f32,
    section: &str,
    scene: Option<(&str, Instant)>,
    rt: &mut Runtime,
) -> (HashMap<String, [u8; 512]>, HashMap<String, Vec<u8>>) {
    let now = Instant::now();
    let rule = show.autopilot.rules.get(section).cloned().unwrap_or_default();
    let master = master * rule.brightness.clamp(0.0, 1.0);
    let profiles: HashMap<&str, &Profile> = show.profiles.iter().map(|p| (p.id.as_str(), p)).collect();

    // 1. defaults
    let mut values: HashMap<Key, u8> = HashMap::new();
    for fx in &show.fixtures {
        let Some(p) = profiles.get(fx.profile_id.as_str()) else { continue };
        let Some(mode) = p.mode(&fx.mode) else { continue };
        for key in &mode.channels {
            let d = p.channel(key).map(|c| c.default).unwrap_or(0);
            values.insert((fx.id.clone(), key.clone()), d);
        }
    }

    // 1b. the playing scene's looks and base values, underneath the routes
    if let Some(p) = scene.and_then(|(id, _)| show.presets.iter().find(|p| p.id == id)) {
        let ctx = MotionCtx { feat, secs: scene.map(|(_, t)| now.duration_since(t).as_secs_f32()).unwrap_or(0.0), hits: feat.beats, offset: 0.0, since_hit: None };
        for sl in &p.looks {
            for fx in resolve_fixtures(show, &sl.target.rooms, &sl.target.groups, &sl.target.fixtures) {
                let Some(pr) = profiles.get(fx.profile_id.as_str()) else { continue };
                let Some(m) = pr.mode(&fx.mode) else { continue };
                let Some(look) = pr.looks.iter().find(|l| l.id == sl.look).or_else(|| pr.looks.iter().find(|l| l.name == sl.look)) else { continue };
                let heads: Vec<&str> = if sl.heads.is_empty() { vec![look.head.as_str()] } else { sl.heads.iter().map(|h| h.as_str()).collect() };
                for head in heads {
                    for (k, base, motion) in look_values(m, look, head) {
                        let key = (fx.id.clone(), k.clone());
                        if values.contains_key(&key) {
                            let v = motion.map(|mo| motion_value(&mo, base, &ctx, &catalog(pr, &k, &mo))).unwrap_or(base);
                            values.insert(key, v);
                        }
                    }
                }
            }
        }
        for v in &p.base {
            let key = (v.fixture.clone(), v.channel.clone());
            if values.contains_key(&key) {
                values.insert(key, v.value);
            }
        }
    }

    // 2. routes
    let clock = *rt.clock.get_or_insert(now);
    // Switches that ran out, or whose route was changed by a scene or by hand, end.
    rt.switched.retain(|id, sw| {
        sw.until.map_or(true, |u| u > now) && show.routes.iter().any(|r| &r.id == id && r.enabled == sw.base)
    });
    for route in &show.routes {
        let enabled = rt.switched.get(&route.id).map(|sw| sw.on).unwrap_or(route.enabled);
        let runs = enabled
            && (route.sections.is_empty() || route.sections.iter().any(|s| s == section))
            && (rule.strobe || !matches!(route.action, RouteAction::Strobe { .. }));
        let st = rt.routes.entry(route.id.clone()).or_default();
        if !runs {
            follow(&route.source, feat, &mut st.last_counter, &mut st.last_level);
            for (k, mo) in &route.motions {
                if !mo.trigger.is_empty() {
                    let h = st.side.entry(k.clone()).or_default();
                    follow(&mo.trigger, feat, &mut h.last_counter, &mut h.last_level);
                }
            }
            continue;
        }
        let is_effect = matches!(
            route.action,
            RouteAction::Strobe { .. } | RouteAction::Palette { .. } | RouteAction::Twinkle { .. } | RouteAction::Wave { .. }
        );
        let targets = if is_effect { vec![] } else { route_targets(show, &profiles, route) };
        // Effects drive whole fixtures through their brightness/colour channels.
        let fx_targets: Vec<(&Fixture, LookChans)> = if is_effect {
            resolve_fixtures(show, &route.rooms, &route.groups, &route.fixtures)
                .into_iter()
                .filter_map(|f| look_chans(&profiles, f).map(|l| (f, l)))
                .collect()
        } else {
            vec![]
        };
        // Look routes drive whichever channels each saved look covers.
        let look_targets: Vec<&Fixture> = if matches!(route.action, RouteAction::Look { .. }) {
            resolve_fixtures(show, &route.rooms, &route.groups, &route.fixtures)
                .into_iter()
                .filter(|f| profiles.get(f.profile_id.as_str()).map(|p| !p.looks.is_empty()).unwrap_or(false))
                .collect()
        } else {
            vec![]
        };
        // (a route that only switches other routes needs no fixtures of its own)
        if targets.is_empty() && fx_targets.is_empty() && look_targets.is_empty() && route.switches.is_empty() {
            continue;
        }
        let dt = st.last_tick.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.0).min(0.2);
        st.last_tick = Some(now);
        // Effects are calculated from the route's focus point, or the centre of its targets.
        let focus = show.focus_point(&route.focus);
        let origin = focus.map(|f| (f.x, f.y, f.z)).unwrap_or_else(|| {
            let fx: Vec<&Fixture> = if is_effect { fx_targets.iter().map(|(f, _)| *f).collect() } else { targets.iter().map(|(f, _)| *f).collect() };
            centroid(&fx)
        });
        // Brightness multiplier for "fade with distance" (1 everywhere when off).
        let fall = |fx: &Fixture| -> f32 {
            match focus {
                Some(f) if route.falloff => {
                    let (x, y, _) = fx.pos();
                    (1.0 - ((x - f.x).powi(2) + (y - f.y).powi(2)).sqrt() / f.radius.max(0.1)).clamp(0.0, 1.0)
                }
                _ => 1.0,
            }
        };

        // trigger detection
        let mut triggered = detect(&route.source, feat, &mut st.last_counter, &mut st.last_level);
        if triggered {
            st.events += 1;
            let every = route.every.max(1) as u64;
            let cooled = st
                .last_fire
                .map(|t| now.duration_since(t).as_millis() as u32 >= route.cooldown_ms)
                .unwrap_or(true);
            let prob = if route.probability <= 0.0 { 1.0 } else { route.probability };
            triggered = st.events % every == 0 && cooled && fastrand::f32() <= prob;
            if triggered {
                st.last_fire = Some(now);
                st.fire_beat = feat.beats as f32 + feat.beat_phase;
            }
        }

        // switch other routes on, off or over
        if triggered {
            let beat_secs = 60.0 / if feat.bpm > 1.0 { feat.bpm } else { 120.0 };
            for sw in &route.switches {
                let Some(other) = show.routes.iter().find(|r| r.id == sw.route && r.id != route.id) else { continue };
                let cur = rt.switched.get(&other.id).map(|x| x.on).unwrap_or(other.enabled);
                let on = match sw.to.as_str() {
                    "off" => false,
                    "toggle" => !cur,
                    _ => true,
                };
                let until = (sw.beats > 0.0).then(|| now + Duration::from_secs_f32(sw.beats * beat_secs));
                if on == other.enabled && until.is_none() {
                    rt.switched.remove(&other.id); // back to how it's saved
                } else {
                    rt.switched.insert(other.id.clone(), Switched { on, until, base: other.enabled });
                }
            }
        }
        let st = rt.routes.get_mut(&route.id).unwrap();

        let pairs = || targets.iter().flat_map(|(fx, keys)| keys.iter().map(move |k| (*fx, k)));
        match &route.action {
            RouteAction::Modulate { min, max, smoothing } => {
                let lvl = source_level(&route.source, feat).unwrap_or_else(|| {
                    // counters: decaying pulse based on time since last fire
                    st.last_fire.map(|t| (1.0 - now.duration_since(t).as_secs_f32() * 4.0).max(0.0)).unwrap_or(0.0)
                });
                for (fx, key) in pairs() {
                    let s = st.smoothed.entry(format!("{}/{key}", fx.id)).or_insert(0.0);
                    let a = smoothing.clamp(0.0, 0.98);
                    *s = *s * a + lvl * (1.0 - a);
                    let v = *min as f32 + (*max as f32 - *min as f32) * *s;
                    values.insert((fx.id.clone(), key.clone()), v.round().clamp(0.0, 255.0) as u8);
                }
            }
            RouteAction::Motion { motion, spread } => {
                let ps: Vec<_> = pairs().collect();
                // chase in fixture order: one offset per fixture, not per channel
                let order: Vec<&str> = {
                    let mut ids: Vec<&str> = vec![];
                    for (fx, _) in &ps {
                        if !ids.contains(&fx.id.as_str()) {
                            ids.push(fx.id.as_str());
                        }
                    }
                    ids
                };
                let n = order.len().max(1) as f32;
                let secs = now.duration_since(clock).as_secs_f32();
                for (fx, key) in ps {
                    let i = order.iter().position(|x| *x == fx.id).unwrap_or(0) as f32;
                    let k = (fx.id.clone(), key.clone());
                    let base = values.get(&k).copied().unwrap_or(0);
                    let since_hit = st.last_fire.map(|t| (now.duration_since(t).as_secs_f32(), (feat.beats as f32 + feat.beat_phase - st.fire_beat).max(0.0)));
                    let ctx = MotionCtx { feat, secs, hits: st.events, offset: spread.clamp(0.0, 1.0) * i / n, since_hit };
                    let p = profiles[fx.profile_id.as_str()];
                    values.insert(k, motion_value(motion, base, &ctx, &catalog(p, key, motion)));
                }
            }
            RouteAction::Flash { on, off, decay_ms } => {
                for (fx, key) in pairs() {
                    let k = (fx.id.clone(), key.clone());
                    if triggered {
                        rt.pulses.insert(k.clone(), Pulse { on: *on, off: *off, start: now, decay_ms: *decay_ms });
                    }
                    values.insert(k.clone(), pulse_value(rt.pulses.get(&k), *off, now));
                }
            }
            RouteAction::NodeHop { on, off, decay_ms, palette } => {
                if triggered {
                    let fixtures: Vec<&Fixture> = targets.iter().map(|(f, _)| *f).collect();
                    let next = hop_next(&fixtures, st, focus.map(|f| (f.x, f.y, f.z)));
                    st.hop_recent.push(next.clone());
                    if st.hop_recent.len() > (fixtures.len() / 2).max(1) {
                        st.hop_recent.remove(0);
                    }
                    st.hop_current = Some(next.clone());
                    if !palette.is_empty() {
                        let cols = palette_colors(show, palette);
                        st.palette_idx = st.palette_idx.wrapping_add(1);
                        let c = cols[st.palette_idx % cols.len()];
                        st.colors.insert(next.clone(), (c, c, now));
                    }
                    if let Some((_, keys)) = targets.iter().find(|(f, _)| f.id == next) {
                        for key in keys {
                            rt.pulses.insert(
                                (next.clone(), key.clone()),
                                Pulse { on: *on, off: *off, start: now, decay_ms: *decay_ms },
                            );
                        }
                    }
                }
                for (fx, key) in pairs() {
                    let k = (fx.id.clone(), key.clone());
                    values.insert(k.clone(), pulse_value(rt.pulses.get(&k), *off, now));
                }
                // each fixture keeps the colour of its last hop
                for (fx, _) in &targets {
                    if let (Some((_, c, _)), Some(lc)) = (st.colors.get(&fx.id), look_chans(&profiles, fx)) {
                        set_color(&mut values, &fx.id, &lc, *c);
                    }
                }
            }
            RouteAction::Strobe { rate, hz, duty, burst_ms, threshold } => {
                let per_sec = match rate.as_str() {
                    "free" => *hz,
                    n => n.parse::<f32>().unwrap_or(4.0) * feat.bpm.max(40.0) / 60.0,
                }
                .clamp(0.5, 25.0);
                let active = if source_counter(&route.source, feat).is_some() {
                    if triggered {
                        st.strobe_until = Some(now + Duration::from_millis(*burst_ms as u64));
                    }
                    st.strobe_until.map(|u| now < u).unwrap_or(false)
                } else {
                    source_level(&route.source, feat).unwrap_or(0.0) > *threshold
                };
                if active {
                    let phase = (now.duration_since(clock).as_secs_f32() * per_sec).fract();
                    let on = phase < duty.clamp(0.05, 0.9);
                    for (fx, lc) in &fx_targets {
                        set_intensity(&mut values, &fx.id, lc, if on { fall(fx) } else { 0.0 });
                    }
                }
            }
            RouteAction::Palette { palette, mode, fade_ms, brightness } => {
                let cols = palette_colors(show, palette);
                if triggered {
                    st.palette_idx = st.palette_idx.wrapping_add(1);
                }
                let order = position_order(&fx_targets);
                let rings = normalized_positions(&fx_targets, "radial", origin);
                let around = normalized_positions(&fx_targets, "angle", origin);
                let n = cols.len();
                for (i, (fx, lc)) in fx_targets.iter().enumerate() {
                    let band = |p: f32| ((p * n as f32) as usize).min(n - 1);
                    let target = match mode.as_str() {
                        "spread" => cols[(st.palette_idx + order[i]) % n],
                        // concentric bands from the focus, moving outward on each hit
                        "rings" => cols[(band(rings[i] / 0.85) + n - st.palette_idx % n) % n],
                        // slices around the focus, rotating on each hit
                        "around" => cols[(band(around[i]) + st.palette_idx) % n],
                        "random" => match st.colors.get(&fx.id) {
                            Some((_, to, _)) if !triggered => *to,
                            _ => cols[fastrand::usize(..cols.len())],
                        },
                        _ => cols[st.palette_idx % cols.len()],
                    };
                    let shown = fade_to(&mut st.colors, &fx.id, target, *fade_ms, now);
                    set_color(&mut values, &fx.id, lc, shown);
                    if *brightness > 0 {
                        set_intensity(&mut values, &fx.id, lc, *brightness as f32 / 255.0 * fall(fx));
                    }
                }
            }
            RouteAction::Twinkle { palette, amount, decay_ms, base } => {
                let cols = palette_colors(show, palette);
                let spawn = if source_counter(&route.source, feat).is_some() {
                    if triggered { amount.max(1.0).round() as usize } else { 0 }
                } else {
                    st.accum += amount * source_level(&route.source, feat).unwrap_or(0.0) * dt;
                    let n = st.accum.floor();
                    st.accum -= n;
                    n as usize
                };
                // With a focus point, sparkles cluster near it (weight falls off over its radius).
                let weights: Vec<f32> = fx_targets
                    .iter()
                    .map(|(fx, _)| match focus {
                        Some(f) => {
                            let (x, y, _) = fx.pos();
                            let d = ((x - f.x).powi(2) + (y - f.y).powi(2)).sqrt() / f.radius.max(0.1);
                            1.0 / (1.0 + d * d * 4.0)
                        }
                        None => 1.0,
                    })
                    .collect();
                for _ in 0..spawn {
                    let (fx, _) = &fx_targets[weighted_pick(&weights)];
                    let c = cols[fastrand::usize(..cols.len())];
                    st.sparkles.push((fx.id.clone(), now, c));
                    st.colors.insert(fx.id.clone(), (c, c, now));
                }
                let decay = (*decay_ms).max(1) as f32 / 1000.0;
                st.sparkles.retain(|(_, t, _)| now.duration_since(*t).as_secs_f32() < decay);
                for (fx, lc) in &fx_targets {
                    let spark = st
                        .sparkles
                        .iter()
                        .filter(|(id, _, _)| id == &fx.id)
                        .map(|(_, t, _)| (1.0 - now.duration_since(*t).as_secs_f32() / decay).powi(2))
                        .fold(0.0f32, f32::max);
                    let level = (*base as f32 / 255.0).max(spark) * fall(fx);
                    let c = st.colors.get(&fx.id).map(|(_, c, _)| *c).unwrap_or(cols[0]);
                    set_color(&mut values, &fx.id, lc, c);
                    set_intensity(&mut values, &fx.id, lc, level);
                }
            }
            RouteAction::Wave { palette, axis, beats, width, base } => {
                let cols = palette_colors(show, palette);
                let t_beats = feat.beats as f32 + feat.beat_phase;
                let per = beats.max(0.25);
                let t = (t_beats / per).fract();
                let cycle = (t_beats / per).floor() as usize;
                let half = (width.clamp(0.05, 1.0)) / 2.0;
                let scale = source_level(&route.source, feat).unwrap_or(1.0);
                let pos = normalized_positions(&fx_targets, axis, origin);
                for ((fx, lc), p) in fx_targets.iter().zip(pos) {
                    let d = (p - t).abs();
                    let d = d.min(1.0 - d); // wrap around
                    let w = if d < half { 0.5 + 0.5 * (std::f32::consts::PI * d / half).cos() } else { 0.0 };
                    let b = *base as f32 / 255.0;
                    set_intensity(&mut values, &fx.id, lc, (b + (1.0 - b) * w) * scale.max(b) * fall(fx));
                    set_color(&mut values, &fx.id, lc, cols[cycle % cols.len()]);
                }
            }
            RouteAction::Look { set, include_defaults, tags, mode, hold_ms, heads, looks, follow_section, max_ms, rest, rest_ms } => {
                if triggered {
                    st.cycle_idx = st.cycle_idx.wrapping_add(1);
                }
                for fx in &look_targets {
                    let p = profiles[fx.profile_id.as_str()];
                    let cands: Vec<&Look> = p
                        .looks
                        .iter()
                        .filter(|l| set.is_empty() || l.sets.contains(set))
                        .filter(|l| *include_defaults || !l.builtin)
                        .filter(|l| tags.iter().all(|t| l.tags.contains(t)))
                        .filter(|l| looks.is_empty() || looks.iter().any(|x| x == &l.id || x == &l.name))
                        .collect();
                    // Following the section: looks tagged for it, when there are any.
                    let in_section: Vec<&Look> = if *follow_section { cands.iter().copied().filter(|l| l.tags.iter().any(|t| t == section)).collect() } else { vec![] };
                    let cands = if in_section.is_empty() { cands } else { in_section };
                    let cur = st.looks.get(&fx.id).cloned();
                    let resting = cur.as_ref().is_some_and(|(id, ..)| id.is_empty());
                    let held = cur.as_ref().is_some_and(|(_, until, ..)| now < *until);
                    let too_long = *max_ms > 0 && !resting && cur.as_ref().is_some_and(|(_, _, started, _)| now.duration_since(*started).as_millis() >= *max_ms as u128);
                    let rest_over = resting && *rest_ms > 0 && !held;
                    let wrong_section = !held && !resting && *follow_section && cur.as_ref().is_some_and(|(id, ..)| !cands.iter().any(|l| &l.id == id));
                    if ((triggered && !held) || too_long || rest_over || wrong_section) && !cands.is_empty() {
                        if *rest > 0.0 && !resting && !wrong_section && cur.is_some() && fastrand::f32() < *rest {
                            st.looks.insert(fx.id.clone(), (String::new(), now + Duration::from_millis(*rest_ms as u64), now, st.events));
                        } else {
                            let pick = if mode == "cycle" {
                                // the look after the current one (or where the route's count is, to start)
                                let at = cur.as_ref().and_then(|(id, ..)| cands.iter().position(|l| &l.id == id));
                                cands[at.map(|i| i + 1).unwrap_or(st.cycle_idx) % cands.len()]
                            } else {
                                let w: Vec<f32> = cands.iter().map(|l| l.rating.max(1) as f32).collect();
                                cands[weighted_pick(&w)]
                            };
                            st.looks.insert(fx.id.clone(), (pick.id.clone(), now + Duration::from_millis(*hold_ms as u64), now, st.events));
                        }
                    }
                    let Some((look, started, hits0)) = st
                        .looks
                        .get(&fx.id)
                        .and_then(|(id, _, started, h)| p.looks.iter().find(|l| &l.id == id).map(|l| (l, *started, *h)))
                    else {
                        continue;
                    };
                    let ctx = MotionCtx { feat, secs: now.duration_since(started).as_secs_f32(), hits: st.events - hits0, offset: 0.0, since_hit: None };
                    let Some(m) = p.mode(&fx.mode) else { continue };
                    let dest: Vec<&str> = if heads.is_empty() { vec![look.head.as_str()] } else { heads.iter().map(|h| h.as_str()).collect() };
                    for head in dest {
                        for (k, base, motion) in look_values(m, look, head) {
                            if values.contains_key(&(fx.id.clone(), k.clone())) {
                                let v = motion.map(|mo| motion_value(&mo, base, &ctx, &catalog(p, &k, &mo))).unwrap_or(base);
                                values.insert((fx.id.clone(), k), v);
                            }
                        }
                    }

                }
            }
            RouteAction::Random { tags, hold_ms } => {
                for (fx, key) in pairs() {
                    let k = (fx.id.clone(), key.clone());
                    let held = rt.held.get(&k).filter(|(_, until)| now < *until).map(|(v, _)| *v);
                    if triggered && held.is_none() {
                        if let Some(v) = pick_random(show, &profiles, fx, key, tags) {
                            rt.held.insert(k.clone(), (v, now + Duration::from_millis(*hold_ms as u64)));
                        }
                    }
                    if let Some((v, _)) = rt.held.get(&k) {
                        values.insert(k, *v);
                    }
                }
            }
            RouteAction::Cycle { values: seq } => {
                if seq.is_empty() {
                    continue;
                }
                if triggered {
                    st.cycle_idx = (st.cycle_idx + 1) % seq.len();
                }
                for (fx, key) in pairs() {
                    values.insert((fx.id.clone(), key.clone()), seq[st.cycle_idx % seq.len()]);
                }
            }
        }

        // other channels the route also moves (e.g. zoom sweeping open on each kick)
        if !route.motions.is_empty() {
            let beat_now = feat.beats as f32 + feat.beat_phase;
            let own = MotionCtx {
                feat,
                secs: now.duration_since(clock).as_secs_f32(),
                hits: st.events,
                offset: 0.0,
                since_hit: st.last_fire.map(|t| (now.duration_since(t).as_secs_f32(), (beat_now - st.fire_beat).max(0.0))),
            };
            // channels with a trigger of their own count and restart on its hits
            let mut ctxs: HashMap<&str, MotionCtx> = HashMap::new();
            for (key, mo) in &route.motions {
                if mo.trigger.is_empty() || mo.trigger == route.source {
                    continue;
                }
                let h = st.side.entry(key.clone()).or_default();
                if detect(&mo.trigger, feat, &mut h.last_counter, &mut h.last_level) {
                    h.events += 1;
                    h.last = Some(now);
                    h.beat = beat_now;
                }
                let since_hit = h.last.map(|t| (now.duration_since(t).as_secs_f32(), (beat_now - h.beat).max(0.0)));
                ctxs.insert(key.as_str(), MotionCtx { hits: h.events, since_hit, ..own });
            }
            for fx in resolve_fixtures(show, &route.rooms, &route.groups, &route.fixtures) {
                let Some(p) = profiles.get(fx.profile_id.as_str()) else { continue };
                for (key, mo) in &route.motions {
                    let ctx = ctxs.get(key.as_str()).unwrap_or(&own);
                    for k in channel_keys(&profiles, fx, key) {
                        let kk = (fx.id.clone(), k.clone());
                        if let Some(base) = values.get(&kk).copied() {
                            values.insert(kk, motion_value(mo, base, &ctx, &catalog(p, &k, mo)));
                        }
                    }
                }
            }
        }
    }

    // 2a. sections that allow no fog: routes leave fog channels at their defaults
    if !rule.fog {
        for fx in &show.fixtures {
            let Some(p) = profiles.get(fx.profile_id.as_str()) else { continue };
            let Some(mode) = p.mode(&fx.mode) else { continue };
            for key in &mode.channels {
                if let Some(ch) = p.channel(key).filter(|c| c.group == "fog") {
                    values.insert((fx.id.clone(), key.clone()), ch.default);
                }
            }
        }
    }

    // 2a'. fixtures open in the pattern browser: back to their defaults, so routes and the
    // scene don't move them; only the browser's preview and values (below) show
    for fx in show.fixtures.iter().filter(|f| rt.browsing.contains(&f.id)) {
        let Some(p) = profiles.get(fx.profile_id.as_str()) else { continue };
        let Some(mode) = p.mode(&fx.mode) else { continue };
        for key in &mode.channels {
            values.insert((fx.id.clone(), key.clone()), p.channel(key).map(|c| c.default).unwrap_or(0));
        }
    }

    // 2b. looks being previewed in the pattern browser
    for (fx_id, (look, started)) in preview {
        let Some(fx) = show.fixtures.iter().find(|f| &f.id == fx_id) else { continue };
        let Some(p) = profiles.get(fx.profile_id.as_str()) else { continue };
        let Some(m) = p.mode(&fx.mode) else { continue };
        let secs = now.duration_since(*started).as_secs_f32();
        // In preview there's no route firing, so "hit" clocks step on each beat.
        let ctx = MotionCtx { feat, secs, hits: feat.beats, offset: 0.0, since_hit: None };
        for (k, base, motion) in look_values(m, look, &look.head) {
            if values.contains_key(&(fx.id.clone(), k.clone())) {
                let v = motion.map(|mo| motion_value(&mo, base, &ctx, &catalog(p, &k, &mo))).unwrap_or(base);
                values.insert((fx.id.clone(), k), v);
            }
        }
    }

    // 3. manual overrides win
    for (k, v) in manual {
        if values.contains_key(k) {
            values.insert(k.clone(), *v);
        }
    }

    // 4. limits, blocked values, blackout -> universes
    let mut universes: HashMap<String, [u8; 512]> = HashMap::new();
    let mut out: HashMap<String, Vec<u8>> = HashMap::new();
    for fx in &show.fixtures {
        let Some(p) = profiles.get(fx.profile_id.as_str()) else { continue };
        let Some(mode) = p.mode(&fx.mode) else { continue };
        let mut row = Vec::with_capacity(mode.channels.len());
        for (i, key) in mode.channels.iter().enumerate() {
            let k = (fx.id.clone(), key.clone());
            let ch = p.channel(key);
            let mut v = *values.get(&k).unwrap_or(&0);
            if let Some(lim) = fx.limits.get(key) {
                if let Some((lo, hi)) = lim.range {
                    v = v.clamp(lo.min(hi), hi.max(lo));
                }
                if lim.min_hold_ms > 0 {
                    if let Some((prev, at)) = rt.last_out.get(&k) {
                        if *prev != v && now.duration_since(*at).as_millis() < lim.min_hold_ms as u128 {
                            v = *prev;
                        }
                    }
                }
                if let Some(l) = lim.lock {
                    v = l;
                }
            }
            if let Some(ch) = ch {
                if ch.group == "intensity" && master < 1.0 {
                    v = (v as f32 * master).round() as u8;
                }
                if ch.value_blocked(v) {
                    v = nearest_allowed(ch, v, fx.limits.get(key));
                }
                if blackout || !fx.enabled {
                    v = off_value(ch);
                }
            }
            match rt.last_out.get(&k) {
                Some((prev, _)) if *prev == v => {}
                _ => {
                    rt.last_out.insert(k, (v, now));
                }
            }
            row.push(v);
            let addr = fx.address as usize + i;
            if fx.zigbee_device.is_none() && (1..=512).contains(&addr) {
                universes.entry(fx.output_id.clone()).or_insert([0u8; 512])[addr - 1] = v;
            }
        }
        out.insert(fx.id.clone(), row);
    }
    rt.pulses.retain(|_, p| now.duration_since(p.start).as_millis() < p.decay_ms as u128 + 50);
    (universes, out)
}

fn pulse_value(p: Option<&Pulse>, off: u8, now: Instant) -> u8 {
    match p {
        None => off,
        Some(p) => {
            let t = now.duration_since(p.start).as_millis() as f32 / p.decay_ms.max(1) as f32;
            if t >= 1.0 {
                p.off
            } else {
                let k = (1.0 - t).powi(2);
                (p.off as f32 + (p.on as f32 - p.off as f32) * k).round() as u8
            }
        }
    }
}

/// Next fixture for node hop: one of the two nearest (in 3D) not visited recently.
/// With a focus point, hops start nearest it and are pulled back towards it.
fn hop_next(targets: &[&Fixture], st: &RouteState, focus: Option<(f32, f32, f32)>) -> String {
    let d2 = |f: &Fixture, (x, y, z): (f32, f32, f32)| {
        let (fx, fy, fz) = f.pos();
        (fx - x).powi(2) + (fy - y).powi(2) + (fz - z).powi(2)
    };
    let cur = st.hop_current.as_ref().and_then(|id| targets.iter().find(|f| &f.id == id));
    let Some(cur) = cur else {
        return match focus {
            Some(p) => targets.iter().min_by(|a, b| d2(a, p).partial_cmp(&d2(b, p)).unwrap()).unwrap().id.clone(),
            None => targets[fastrand::usize(..targets.len())].id.clone(),
        };
    };
    let mut cands: Vec<(&Fixture, f32)> = targets
        .iter()
        .filter(|f| f.id != cur.id && !st.hop_recent.contains(&f.id))
        // 3D: height counts too, so a ceiling bulb isn't "next to" the floor par below it.
        .map(|f| (*f, d2(f, cur.pos()) + focus.map(|p| 0.5 * d2(f, p)).unwrap_or(0.0)))
        .collect();
    if cands.is_empty() {
        cands = targets.iter().filter(|f| f.id != cur.id).map(|f| (*f, 0.0)).collect();
    }
    if cands.is_empty() {
        return cur.id.clone();
    }
    cands.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let n = cands.len().min(2);
    cands[fastrand::usize(..n)].0.id.clone()
}

fn pick_random(
    _show: &Show,
    profiles: &HashMap<&str, &Profile>,
    fx: &Fixture,
    channel: &str,
    route_tags: &[String],
) -> Option<u8> {
    let p = profiles.get(fx.profile_id.as_str())?;
    let ch = p.channel(channel)?;
    let lim = fx.limits.get(channel);
    let mut tags: Vec<String> = route_tags.to_vec();
    if let Some(l) = lim {
        tags.extend(l.require_tags.iter().cloned());
    }
    let (lo, hi) = lim.and_then(|l| l.range).unwrap_or((0, 255));
    let ok = |v: u8| v >= lo && v <= hi && !ch.value_blocked(v) && ch.value_has_tags(v, &tags);
    // Prefer catalogued values (pattern browser), weighted by rating.
    let mut cands: Vec<u8> = vec![];
    for n in ch.values.iter().filter(|n| ok(n.value)) {
        for _ in 0..n.rating.max(1) {
            cands.push(n.value);
        }
    }
    if cands.is_empty() {
        cands = (lo..=hi).filter(|v| ok(*v)).collect();
    }
    (!cands.is_empty()).then(|| cands[fastrand::usize(..cands.len())])
}

fn nearest_allowed(ch: &ChannelDef, v: u8, lim: Option<&Limit>) -> u8 {
    let (lo, hi) = lim.and_then(|l| l.range).unwrap_or((0, 255));
    (0..=255u16)
        .map(|d| d as i16)
        .flat_map(|d| [v as i16 - d, v as i16 + d])
        .filter(|c| (lo as i16..=hi as i16).contains(c))
        .map(|c| c as u8)
        .find(|c| !ch.value_blocked(*c))
        .unwrap_or(ch.default)
}

fn off_value(ch: &ChannelDef) -> u8 {
    if let Some(c) = ch.capabilities.iter().find(|c| c.tags.iter().any(|t| t == "off")) {
        return c.min;
    }
    match ch.group.as_str() {
        "intensity" | "fog" | "strobe" => 0,
        _ => ch.default,
    }
}

/// Desired Zigbee light states keyed by Zigbee2MQTT friendly name.
fn zigbee_states(show: &Show, out: &HashMap<String, Vec<u8>>) -> HashMap<String, zigbee::Desired> {
    let mut m = HashMap::new();
    for fx in &show.fixtures {
        let Some(dev) = fx.zigbee_device.as_ref().filter(|d| !d.is_empty()) else { continue };
        let Some(p) = show.profiles.iter().find(|p| p.id == fx.profile_id) else { continue };
        let Some(mode) = p.mode(&fx.mode) else { continue };
        let chans: Vec<&ChannelDef> = mode.channels.iter().filter_map(|k| p.channel(k)).collect();
        if let Some(vals) = out.get(&fx.id) {
            m.insert(dev.clone(), zigbee::desired_from(p, &chans, vals));
        }
    }
    m
}

/// Fixtures selected by rooms (scope) + groups + fixtures. Groups and fixtures both
/// empty = every fixture in scope; rooms empty = any room.
pub fn resolve_fixtures<'a>(show: &'a Show, rooms: &[String], groups: &[String], fixtures: &[String]) -> Vec<&'a Fixture> {
    let everyone = fixtures.is_empty() && groups.is_empty();
    let mut wanted: std::collections::HashSet<&str> = fixtures.iter().map(|s| s.as_str()).collect();
    for g in show.groups.iter().filter(|g| groups.contains(&g.id)) {
        wanted.extend(g.fixtures.iter().map(|s| s.as_str()));
    }
    show.fixtures
        .iter()
        .filter(|f| everyone || wanted.contains(f.id.as_str()))
        .filter(|f| rooms.is_empty() || f.room_id.as_ref().map(|r| rooms.contains(r)).unwrap_or(false))
        .collect()
}

/// Channel keys on `fx` matching `channel` (a key, or "@type" for every channel of that type).
pub fn channel_keys(profiles: &HashMap<&str, &Profile>, fx: &Fixture, channel: &str) -> Vec<String> {
    let Some(p) = profiles.get(fx.profile_id.as_str()) else { return vec![] };
    let Some(mode) = p.mode(&fx.mode) else { return vec![] };
    match channel.strip_prefix('@') {
        Some(group) => mode
            .channels
            .iter()
            .filter(|k| p.channel(k).map(|c| c.group == group).unwrap_or(false))
            .cloned()
            .collect(),
        None => mode.channels.iter().filter(|k| *k == channel).cloned().collect(),
    }
}

/// The fixtures a route drives, each with the channel keys it touches.
pub fn route_targets<'a>(
    show: &'a Show,
    profiles: &HashMap<&str, &Profile>,
    route: &Route,
) -> Vec<(&'a Fixture, Vec<String>)> {
    resolve_fixtures(show, &route.rooms, &route.groups, &route.fixtures)
        .into_iter()
        .map(|fx| (fx, channel_keys(profiles, fx, &route.channel)))
        .filter(|(_, k)| !k.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_use_where_the_light_lands() {
        // Two pars hung side by side at the back wall: one aimed down, one thrown across the room.
        let room = Room { id: "r".into(), walls: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 6.0], [0.0, 6.0]], ceiling_m: 3.0, ..Default::default() };
        let par = |id: &str, x: f32, tilt: Option<f32>| Fixture {
            id: id.into(), room_id: Some("r".into()), x, y: 0.5, z: 2.5, rotation: 90.0, tilt, ..Default::default()
        };
        let mut show = Show { rooms: vec![room], fixtures: vec![par("down", 1.0, Some(-90.0)), par("throw", 1.2, Some(-30.0)), par("plain", 1.1, None)], ..Default::default() };
        show.migrate();
        let t: Vec<(&Fixture, LookChans)> = show.fixtures.iter().map(|f| (f, LookChans::default())).collect();
        // Along y (front to back) the thrown par is far from the others, though they hang together.
        let pos = normalized_positions(&t, "y", (0.0, 0.0, 0.0));
        assert!(pos[0] < 0.01 && pos[2] < 0.01, "{pos:?}");
        assert!((pos[1] - 0.85).abs() < 1e-3, "{pos:?}"); // 0.85 = the far end (a gap is left for wrapping)
        let (_, y, z) = show.fixtures[1].pos();
        assert!((y - (0.5 + 2.5 / 30f32.to_radians().tan())).abs() < 0.01 && z == 0.0);
        // Node hop distances use the spots too: from "down", "plain" (0.1 m away) is nearer than "throw".
        let st = RouteState { hop_current: Some("down".into()), ..Default::default() };
        let refs: Vec<&Fixture> = show.fixtures.iter().collect();
        let picks: std::collections::HashSet<String> = (0..20).map(|_| hop_next(&refs, &st, None)).collect();
        assert!(picks.contains("plain"));
        // centroid of the three is pulled out into the room by the thrown one
        assert!(centroid(&refs).1 > 1.0);
    }

    fn fixture(id: &str, profile: &str, x: f32, z: f32) -> Fixture {
        Fixture { id: id.into(), profile_id: profile.into(), mode: "Default".into(), x, z, enabled: true, ..Default::default() }
    }

    fn show() -> Show {
        let mut s = Show::default();
        s.profiles = crate::zigbee::builtin_profiles(); // rgb (dimmer+r+g+b), white (dimmer+ct), ...
        s.fixtures = vec![
            fixture("a", "zigbee-rgb", 0.0, 0.0),
            fixture("b", "zigbee-white", 1.0, 0.0),
            fixture("c", "zigbee-rgb", 5.0, 0.0),
        ];
        s.groups = vec![FixtureGroup { id: "g".into(), name: "Left".into(), fixtures: vec!["a".into(), "b".into()], ..Default::default() }];
        s
    }

    #[test]
    fn targets_by_group_and_channel_type() {
        let s = show();
        let profiles: HashMap<&str, &Profile> = s.profiles.iter().map(|p| (p.id.as_str(), p)).collect();
        let r = Route { groups: vec!["g".into()], channel: "@intensity".into(), ..Default::default() };
        let t = route_targets(&s, &profiles, &r);
        assert_eq!(t.iter().map(|(f, k)| (f.id.as_str(), k.clone())).collect::<Vec<_>>(),
            vec![("a", vec!["dimmer".to_string()]), ("b", vec!["dimmer".to_string()])]);
        // a specific key only matches fixtures that have it
        let r = Route { channel: "red".into(), ..Default::default() };
        let ids: Vec<&str> = route_targets(&s, &profiles, &r).iter().map(|(f, _)| f.id.as_str()).collect();
        assert_eq!(ids, ["a", "c"]);
        // group plus an extra fixture
        let r = Route { groups: vec!["g".into()], fixtures: vec!["c".into()], channel: "@color".into(), ..Default::default() };
        let t = route_targets(&s, &profiles, &r);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].1, vec!["red", "green", "blue"]);
        assert_eq!(t[1].1, vec!["color_temp"]);
    }

    #[test]
    fn rooms_scope_targets() {
        let mut s = show();
        s.fixtures[0].room_id = Some("r1".into());
        s.fixtures[1].room_id = Some("r2".into());
        s.fixtures[2].room_id = Some("r1".into());
        let profiles: HashMap<&str, &Profile> = s.profiles.iter().map(|p| (p.id.as_str(), p)).collect();
        // room alone = everything in it
        let r = Route { rooms: vec!["r1".into()], channel: "@intensity".into(), ..Default::default() };
        let ids: Vec<&str> = route_targets(&s, &profiles, &r).iter().map(|(f, _)| f.id.as_str()).collect();
        assert_eq!(ids, ["a", "c"]);
        // room + group = the group's members that are in the room
        let r = Route { rooms: vec!["r1".into()], groups: vec!["g".into()], channel: "@intensity".into(), ..Default::default() };
        let ids: Vec<&str> = route_targets(&s, &profiles, &r).iter().map(|(f, _)| f.id.as_str()).collect();
        assert_eq!(ids, ["a"]);
    }

    fn effect_show(action: RouteAction, source: &str) -> Show {
        let mut s = show(); // a (rgb, x=0), b (white, x=1), c (rgb, x=5)
        s.palettes = default_palettes();
        s.routes = vec![Route { id: "r".into(), enabled: true, source: source.into(), action, ..Default::default() }];
        s
    }
    fn run_once(s: &Show, feat: &Features, rt: &mut Runtime) -> HashMap<String, Vec<u8>> {
        compute(s, feat, &HashMap::new(), &HashMap::new(), false, 1.0, "groove", None, rt).1
    }

    #[test]
    fn palette_spread_gives_neighbours_different_colours() {
        let s = effect_show(
            RouteAction::Palette { palette: "pal-club".into(), mode: "spread".into(), fade_ms: 0, brightness: 255 },
            "beat",
        );
        let out = run_once(&s, &Features::default(), &mut Runtime::default());
        // zigbee-rgb channels: dimmer, red, green, blue
        assert_eq!(out["a"][0], 255);
        assert_ne!(out["a"][1..4], out["c"][1..4]);
        assert_eq!(out["a"][1..4], [0xff, 0x00, 0x40]); // first club colour, leftmost fixture
    }

    #[test]
    fn the_playing_scene_sets_looks_and_values_under_the_routes() {
        let mut s = laser_show();
        let look = s.profiles[0].looks.first().cloned().expect("laser has looks");
        let (key, val) = look.values.iter().next().map(|(k, v)| (k.clone(), *v)).unwrap();
        let fx = s.fixtures[0].id.clone();
        s.presets.push(Preset {
            id: "sc".into(), name: "Calm".into(),
            looks: vec![SceneLook { target: Target::default(), look: look.name.clone(), heads: vec![] }],
            ..Default::default()
        });
        let f = Features::default();
        let out = |s: &Show, scene: Option<(&str, Instant)>| compute(s, &f, &HashMap::new(), &HashMap::new(), false, 1.0, "groove", scene, &mut Runtime::default()).1;
        let i = s.profiles[0].mode(&s.fixtures[0].mode).unwrap().channels.iter().position(|c| c == &key).unwrap();
        assert_eq!(out(&s, Some(("sc", Instant::now())))[&fx][i], val, "the scene's look is applied");
        // a base value, and a route on the same channel wins over it
        s.presets[0].looks.clear();
        s.presets[0].base = vec![PresetValue { fixture: fx.clone(), channel: key.clone(), value: 77 }];
        assert_eq!(out(&s, Some(("sc", Instant::now())))[&fx][i], 77);
        assert_ne!(out(&s, None)[&fx][i], 77, "no scene playing: defaults");
        s.routes = vec![Route { id: "r".into(), enabled: true, source: "level".into(), fixtures: vec![fx.clone()], channel: key.clone(),
            action: RouteAction::Modulate { min: 200, max: 200, smoothing: 0.0 }, ..Default::default() }];
        assert_eq!(out(&s, Some(("sc", Instant::now())))[&fx][i], 200, "routes play on top of the scene");
    }

    #[test]
    fn routes_run_only_in_their_sections_and_rules_apply() {
        let mut s = show();
        let level = RouteAction::Modulate { min: 0, max: 255, smoothing: 0.0 };
        s.routes = vec![Route { id: "r".into(), enabled: true, source: "level".into(), channel: "@intensity".into(), action: level, sections: vec!["drop".into()], ..Default::default() }];
        let f = Features { level: 1.0, ..Default::default() };
        let at = |s: &Show, sec: &str| compute(s, &f, &HashMap::new(), &HashMap::new(), false, 1.0, sec, None, &mut Runtime::default()).1["a"][0];
        assert_eq!(at(&s, "groove"), 0, "a drop-only route stays out of the groove");
        assert_eq!(at(&s, "drop"), 255);
        s.routes[0].sections.clear();
        assert_eq!(at(&s, "breakdown"), 255, "no sections = always");
        // a brightness cap for the breakdown, like the master fader
        s.autopilot.rules.insert("breakdown".into(), SectionRule { brightness: 0.5, ..Default::default() });
        assert_eq!(at(&s, "breakdown"), 128);
        assert_eq!(at(&s, "drop"), 255);
        // no strobe in the breakdown
        let mut st = effect_show(RouteAction::Strobe { rate: "free".into(), hz: 10.0, duty: 0.99, burst_ms: 0, threshold: 0.5 }, "level");
        st.autopilot.rules.insert("breakdown".into(), SectionRule { strobe: false, ..Default::default() });
        let lit = |sec: &str| (0..10).any(|_| compute(&st, &f, &HashMap::new(), &HashMap::new(), false, 1.0, sec, None, &mut Runtime::default()).1["a"][0] > 0);
        assert!(lit("drop"));
        assert!(!lit("breakdown"));
    }

    #[test]
    fn strobe_follows_level_threshold() {
        let s = effect_show(
            RouteAction::Strobe { rate: "free".into(), hz: 10.0, duty: 0.5, burst_ms: 0, threshold: 0.5 },
            "level",
        );
        let mut rt = Runtime::default();
        let quiet = Features { level: 0.1, ..Default::default() };
        assert_eq!(run_once(&s, &quiet, &mut rt)["a"][0], 0); // untouched default
        let loud = Features { level: 0.9, ..Default::default() };
        let mut seen = std::collections::HashSet::new();
        for _ in 0..40 {
            seen.insert(run_once(&s, &loud, &mut rt)["a"][0]);
            std::thread::sleep(Duration::from_millis(7));
        }
        assert!(seen.contains(&0) && seen.contains(&255), "strobe should flash: {seen:?}");
    }

    #[test]
    fn wave_lights_one_end_at_a_time() {
        let s = effect_show(
            RouteAction::Wave { palette: "pal-white".into(), axis: "x".into(), beats: 4.0, width: 0.2, base: 0 },
            "beat",
        );
        let at = |beats: u64| run_once(&s, &Features { beats, ..Default::default() }, &mut Runtime::default());
        let start = at(0); // t=0: leftmost (a) lit, rightmost (c) dark
        assert!(start["a"][0] > 200 && start["c"][0] < 50);
    }

    #[test]
    fn laser_colour_macro_picks_nearest() {
        let laser = crate::qxf::import(include_str!("../../fixtures/Generic-3D-Animation-RGB-Laser-L2600-005.qxf")).unwrap();
        let fx = Fixture { id: "L".into(), profile_id: laser.id.clone(), mode: laser.modes[0].name.clone(), ..Default::default() };
        let profiles: HashMap<&str, &Profile> = [(laser.id.as_str(), &laser)].into_iter().collect();
        let lc = look_chans(&profiles, &fx).expect("laser has colour macros");
        let mut v = HashMap::new();
        set_color(&mut v, "L", &lc, [0, 250, 10]); // nearly pure green
        let p1 = v[&("L".to_string(), "p1_colour".to_string())];
        assert!((32..=39).contains(&p1), "green macro is 32-39, got {p1}");
    }

    fn with_focus(mut s: Show, x: f32, radius: f32, falloff: bool) -> Show {
        s.rooms = vec![Room {
            id: "room".into(),
            focus: vec![FocusPoint { id: "f".into(), x, y: 0.0, z: 0.0, radius, ..Default::default() }],
            ..Default::default()
        }];
        for r in &mut s.routes {
            r.focus = "f".into();
            r.falloff = falloff;
        }
        s
    }

    #[test]
    fn falloff_dims_with_distance_from_focus() {
        // a at x=0 (on the focus), b at 1 m, c at 5 m; radius 2 m.
        let s = with_focus(
            effect_show(RouteAction::Palette { palette: "pal-white".into(), mode: "all".into(), fade_ms: 0, brightness: 255 }, "beat"),
            0.0, 2.0, true,
        );
        let out = run_once(&s, &Features::default(), &mut Runtime::default());
        assert_eq!(out["a"][0], 255);
        assert!((120..=135).contains(&out["b"][0]), "half way out should be ~half: {}", out["b"][0]);
        assert_eq!(out["c"][0], 0, "outside the radius is dark");
    }

    #[test]
    fn palette_rings_colour_by_distance_from_focus() {
        // focus at c (x=5): c is ring 0, a (5 m away) is the outer ring
        let s = with_focus(
            effect_show(RouteAction::Palette { palette: "pal-club".into(), mode: "rings".into(), fade_ms: 0, brightness: 255 }, "beat"),
            5.0, 3.0, false,
        );
        let out = run_once(&s, &Features::default(), &mut Runtime::default());
        assert_eq!(out["c"][1..4], [0xff, 0x00, 0x40], "the fixture at the focus gets the first colour");
        assert_ne!(out["a"][1..4], out["c"][1..4]);
    }

    #[test]
    fn node_hop_starts_at_focus() {
        let a = fixture("a", "p", 0.0, 0.0);
        let b = fixture("b", "p", 4.0, 0.0);
        let c = fixture("c", "p", 9.0, 0.0);
        let st = RouteState::default();
        for _ in 0..10 {
            assert_eq!(hop_next(&[&a, &b, &c], &st, Some((8.5, 0.0, 0.0))), "c");
        }
    }

    #[test]
    fn angle_positions_go_round_the_focus() {
        let l = LookChans::default();
        let north = fixture("n", "p", 0.0, 0.0);
        let mut south = fixture("s", "p", 0.0, 0.0);
        let mut north2 = north.clone();
        north2.y = -2.0; // towards the front wall
        south.y = 2.0;
        let t = vec![(&north2, l.clone()), (&south, l.clone())];
        let p = normalized_positions(&t, "angle", (0.0, 0.0, 0.0));
        assert!(p[0] < 0.01 || p[0] > 0.99, "front = 0: {p:?}");
        assert!((p[1] - 0.5).abs() < 0.01, "back = 0.5: {p:?}");
    }

    fn laser_show() -> Show {
        let mut laser = crate::qxf::import(include_str!("../../fixtures/Generic-3D-Animation-RGB-Laser-L2600-005.qxf")).unwrap();
        let mut v = std::collections::BTreeMap::new();
        v.insert("p1_gallery".to_string(), 10); // beam gallery
        v.insert("p1_pattern".to_string(), 12);
        v.insert("p1_zoom".to_string(), 140); // dynamic zoom in
        laser.looks.push(Look { id: "beam12".into(), name: "Beam 12 zoom".into(), head: "P1".into(), values: v, tags: vec!["peak".into()], sets: vec!["drop".into()], ..Default::default() });
        let mut v2 = std::collections::BTreeMap::new();
        v2.insert("p1_pattern".to_string(), 99);
        laser.looks.push(Look { id: "chill".into(), name: "Chill".into(), head: "P1".into(), values: v2, tags: vec!["chill".into()], ..Default::default() });
        let fx = Fixture { id: "L".into(), profile_id: laser.id.clone(), mode: laser.modes[0].name.clone(), enabled: true, ..Default::default() };
        Show { profiles: vec![laser], fixtures: vec![fx], ..Default::default() }
    }

    #[test]
    fn routes_leave_a_fixture_open_in_the_pattern_browser_alone() {
        let mut s = laser_show();
        s.routes = vec![Route { id: "rot".into(), enabled: true, source: "level".into(), channel: "p1_rotation".into(),
            action: RouteAction::Modulate { min: 200, max: 200, smoothing: 0.0 }, ..Default::default() }];
        let rot = |out: &HashMap<String, Vec<u8>>| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == "p1_rotation").unwrap()];
        let f = Features { level: 1.0, ..Default::default() };
        let mut rt = Runtime::default();
        assert_eq!(rot(&run_once(&s, &f, &mut rt)), 200);
        rt.browsing.insert("L".into());
        let out = run_once(&s, &f, &mut rt);
        assert_eq!(rot(&out), s.profiles[0].channel("p1_rotation").unwrap().default, "the route doesn't move it while browsing");
        let mut manual = HashMap::new();
        manual.insert(("L".to_string(), "p1_rotation".to_string()), 33u8);
        let out = compute(&s, &f, &manual, &HashMap::new(), false, 1.0, "groove", None, &mut rt).1;
        assert_eq!(rot(&out), 33, "what you set in the browser still shows");
        rt.browsing.clear();
        assert_eq!(rot(&run_once(&s, &f, &mut rt)), 200, "and routes take over again after");
    }

    #[test]
    fn a_look_only_sets_its_own_channels_so_on_off_stays_yours() {
        let mut s = laser_show();
        let look = |heads: Vec<String>| RouteAction::Look { set: String::new(), include_defaults: true, tags: vec![], mode: "cycle".into(), hold_ms: 0, heads,
            looks: vec!["Beam 12 zoom".into()], follow_section: false, max_ms: 0, rest: 0.0, rest_ms: 0 };
        // a second route (or a pad, or a scene value) holds P2 on; the look leaves it be
        s.routes = vec![
            Route { id: "on".into(), enabled: true, source: "level".into(), channel: "p2_on_off".into(), action: RouteAction::Modulate { min: 200, max: 200, smoothing: 0.0 }, ..Default::default() },
            Route { id: "r".into(), enabled: true, source: "beat".into(), action: look(vec!["P2".into()]), ..Default::default() },
        ];
        let at = |s: &Show, out: &HashMap<String, Vec<u8>>, k: &str| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == k).unwrap()];
        let mut rt = Runtime::default();
        let out = run_once(&s, &Features { beats: 1, level: 1.0, ..Default::default() }, &mut rt);
        assert_eq!((at(&s, &out, "p2_pattern"), at(&s, &out, "p2_zoom"), at(&s, &out, "p2_on_off")), (12, 140, 200), "a P1 look on P2: mapped; on/off untouched");
        // with nothing else driving it, on/off stays at its default (off)
        s.routes.remove(0);
        let mut rt = Runtime::default();
        let out = run_once(&s, &Features { beats: 1, ..Default::default() }, &mut rt);
        assert_eq!(at(&s, &out, "p2_on_off"), 0, "a look never switches a head on by itself");
    }

    #[test]
    fn look_applies_pattern_and_secondary_options() {
        let mut s = laser_show();
        s.routes = vec![Route {
            id: "r".into(), enabled: true, source: "beat".into(),
            action: RouteAction::Look { set: "drop".into(), include_defaults: true, tags: vec![], mode: "random".into(), hold_ms: 0, heads: vec![], looks: vec![], follow_section: false, max_ms: 0, rest: 0.0, rest_ms: 0 },
            ..Default::default()
        }];
        let mut rt = Runtime::default();
        run_once(&s, &Features { beats: 1, ..Default::default() }, &mut rt); // first counter change triggers
        let out = run_once(&s, &Features { beats: 1, ..Default::default() }, &mut rt);
        let mode = &s.profiles[0].modes[0];
        let at = |k: &str| out["L"][mode.channels.iter().position(|c| c == k).unwrap()];
        assert_eq!((at("p1_gallery"), at("p1_pattern"), at("p1_zoom")), (10, 12, 140));
        // P2 untouched when no heads are chosen
        assert_eq!(at("p2_pattern"), 0);
    }

    #[test]
    fn look_routes_follow_the_section_change_after_a_while_and_rest() {
        let mut s = laser_show();
        s.profiles[0].looks[0].tags.push("drop".into()); // Beam 12: pattern 12
        s.profiles[0].looks[1].tags.push("breakdown".into()); // Chill: pattern 99
        let look = |follow: bool, max_ms: u32, rest: f32, rest_ms: u32| RouteAction::Look {
            set: String::new(), include_defaults: true, tags: vec![], mode: "cycle".into(), hold_ms: 0, heads: vec![],
            looks: vec![], follow_section: follow, max_ms, rest, rest_ms,
        };
        s.routes = vec![Route { id: "r".into(), enabled: true, source: "bar".into(), action: look(true, 0, 0.0, 0), ..Default::default() }];
        let pattern = |out: &HashMap<String, Vec<u8>>| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == "p1_pattern").unwrap()];
        let mut rt = Runtime::default();
        let f = Features { beats: 4, ..Default::default() };
        let run = |s: &Show, sec: &str, rt: &mut Runtime| compute(s, &f, &HashMap::new(), &HashMap::new(), false, 1.0, sec, None, rt).1;
        assert_eq!(pattern(&run(&s, "drop", &mut rt)), 12, "the drop look in the drop");
        assert_eq!(pattern(&run(&s, "breakdown", &mut rt)), 99, "changes as soon as the section does, without waiting for a hit");
        assert_eq!(pattern(&run(&s, "groove", &mut rt)), 99, "no look tagged groove: keeps going with what it has");
        // only named looks
        let mut only = look(false, 0, 0.0, 0);
        if let RouteAction::Look { looks, .. } = &mut only {
            *looks = vec!["Beam 12 zoom".into()];
        }
        s.routes[0].action = only;
        let mut rt = Runtime::default();
        assert_eq!(pattern(&run(&s, "breakdown", &mut rt)), 12);
        // change anyway after max_ms, with no new hits
        s.routes[0].action = look(false, 30, 0.0, 0);
        let mut rt = Runtime::default();
        let first = pattern(&run(&s, "groove", &mut rt));
        std::thread::sleep(Duration::from_millis(40));
        assert_ne!(pattern(&run(&s, "groove", &mut rt)), first, "cycled on after max_ms");
        // resting: always rest after the first look, for a while, then a look again
        s.routes[0].action = look(false, 30, 1.0, 30);
        let mut rt = Runtime::default();
        assert_ne!(pattern(&run(&s, "groove", &mut rt)), 0);
        std::thread::sleep(Duration::from_millis(40));
        assert_eq!(pattern(&run(&s, "groove", &mut rt)), 0, "resting: back to the fixture's default");
        std::thread::sleep(Duration::from_millis(40));
        assert_ne!(pattern(&run(&s, "groove", &mut rt)), 0, "a rest is followed by a look");
    }

    #[test]
    fn motion_routes_sweep_a_channel_and_chase_across_fixtures() {
        let mut s = show(); // zigbee rgb a, white b, rgb c: all have a dimmer
        let sweep = Motion { kind: "sweep".into(), ranges: vec![[0, 200]], clock: "beat".into(), period: 4.0, shape: "up".into(), ..Default::default() };
        s.routes = vec![Route { id: "m".into(), enabled: true, source: "beat".into(), channel: "@intensity".into(),
            action: RouteAction::Motion { motion: sweep.clone(), spread: 0.0 }, ..Default::default() }];
        let at = |s: &Show, beats: u64, phase: f32| run_once(s, &Features { beats, beat_phase: phase, ..Default::default() }, &mut Runtime::default());
        assert_eq!(at(&s, 0, 0.0)["a"][0], 0);
        assert_eq!(at(&s, 2, 0.0)["a"][0], 100, "half way up after 2 of 4 beats");
        assert_eq!(at(&s, 2, 0.0)["c"][0], 100, "no spread: all together");
        s.routes[0].action = RouteAction::Motion { motion: sweep, spread: 1.0 };
        let o = at(&s, 0, 0.0);
        assert_eq!((o["a"][0], o["b"][0], o["c"][0]), (0, 67, 133), "spread over the period: they chase");
    }

    #[test]
    fn routes_also_move_other_channels_and_restart_on_each_hit() {
        let mut s = laser_show();
        let mut motions = std::collections::BTreeMap::new();
        // zoom sweeps open over one beat after each hit, then holds
        motions.insert("p1_zoom".to_string(), Motion { kind: "sweep".into(), ranges: vec![[0, 100]], clock: "beat".into(), period: 1.0, shape: "up".into(), retrigger: true, once: true, ..Default::default() });
        motions.insert("p1_rotation".to_string(), Motion { kind: "sweep".into(), ranges: vec![[0, 100]], clock: "beat".into(), period: 4.0, shape: "up".into(), ..Default::default() });
        s.routes = vec![Route { id: "r".into(), enabled: true, source: "beat".into(), motions,
            action: RouteAction::Look { set: "drop".into(), include_defaults: true, tags: vec![], mode: "random".into(), hold_ms: 0, heads: vec![],
                looks: vec![], follow_section: false, max_ms: 0, rest: 0.0, rest_ms: 0 },
            ..Default::default() }];
        let mut rt = Runtime::default();
        let f = |beats: u64, phase: f32| Features { beats, beat_phase: phase, ..Default::default() };
        let ch = |out: &HashMap<String, Vec<u8>>, k: &str| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == k).unwrap()];
        let out = run_once(&s, &f(1, 0.0), &mut rt); // fires at beat 1
        assert_eq!(ch(&out, "p1_rotation"), 25, "rotation sweeps on top of the look");
        assert_eq!(ch(&out, "p1_zoom"), 0, "zoom starts its sweep at the hit");
        assert_eq!(ch(&run_once(&s, &f(1, 0.5), &mut rt), "p1_zoom"), 50, "half a beat after the hit");
        assert_eq!(ch(&run_once(&s, &f(1, 0.9), &mut rt), "p1_zoom"), 90);
        assert_eq!(ch(&run_once(&s, &f(2, 0.0), &mut rt), "p1_zoom"), 0, "the next hit starts it again");
        assert_eq!(ch(&run_once(&s, &f(2, 0.5), &mut rt), "p1_zoom"), 50);
        // without a hit yet, a restarting motion leaves the channel alone
        let mut rt2 = Runtime::default();
        let mut s2 = s.clone();
        s2.routes[0].source = "kick".into();
        assert_eq!(ch(&run_once(&s2, &f(2, 0.0), &mut rt2), "p1_zoom"), 0);
    }

    #[test]
    fn routes_switch_other_routes_on_off_and_over() {
        let mut s = laser_show();
        let switcher = |to: &str, beats: f32| Route {
            id: "s".into(), enabled: true, source: "bar".into(), action: RouteAction::Cycle { values: vec![0] },
            switches: vec![RouteSwitch { route: "z".into(), to: to.into(), beats }], ..Default::default()
        };
        let zoom = Route {
            id: "z".into(), enabled: false, source: "level".into(), channel: "p1_zoom".into(),
            action: RouteAction::Modulate { min: 200, max: 200, smoothing: 0.0 }, ..Default::default()
        };
        s.routes = vec![switcher("on", 0.0), zoom];
        let at = |out: &HashMap<String, Vec<u8>>| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == "p1_zoom").unwrap()];
        let f = |beats: u64| Features { beats, level: 1.0, bpm: 120.0, ..Default::default() };
        let mut rt = Runtime::default();
        let off = at(&run_once(&s, &f(1), &mut rt));
        assert_ne!(off, 200);
        assert_eq!(at(&run_once(&s, &f(4), &mut rt)), 200, "the bar switches the zoom route on, the same frame");
        assert_eq!(at(&run_once(&s, &f(5), &mut rt)), 200, "and it stays on");
        assert!(!s.routes[1].enabled, "the saved show is never changed");
        // switching the route by hand (or a scene) ends the switch
        s.routes[1].enabled = true;
        run_once(&s, &f(6), &mut rt);
        s.routes[1].enabled = false;
        assert_eq!(at(&run_once(&s, &f(7), &mut rt)), off);
        // toggle flips it on each bar
        s.routes[0] = switcher("toggle", 0.0);
        assert_eq!(at(&run_once(&s, &f(8), &mut rt)), 200);
        assert_eq!(at(&run_once(&s, &f(12), &mut rt)), off);
        // for a number of beats, then back
        s.routes[0] = switcher("on", 0.001);
        assert_eq!(at(&run_once(&s, &f(16), &mut rt)), 200);
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(at(&run_once(&s, &f(17), &mut rt)), off, "it runs out");
    }

    #[test]
    fn a_moved_channel_can_have_its_own_trigger() {
        let mut s = laser_show();
        let mut motions = std::collections::BTreeMap::new();
        // the route fires on every beat, but zoom restarts only on each bar
        motions.insert("p1_zoom".to_string(), Motion { kind: "sweep".into(), ranges: vec![[0, 100]], clock: "beat".into(), period: 1.0, shape: "up".into(), retrigger: true, once: true, trigger: "bar".into(), ..Default::default() });
        s.routes = vec![Route { id: "r".into(), enabled: true, source: "beat".into(), channel: "p1_pattern".into(), motions,
            action: RouteAction::Cycle { values: vec![1, 2] }, ..Default::default() }];
        let mut rt = Runtime::default();
        let f = |beats: u64, phase: f32| Features { beats, beat_phase: phase, ..Default::default() };
        let ch = |out: &HashMap<String, Vec<u8>>| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == "p1_zoom").unwrap()];
        run_once(&s, &f(1, 0.0), &mut rt);
        assert_eq!(ch(&run_once(&s, &f(4, 0.0), &mut rt)), 0, "the bar starts the sweep");
        assert_eq!(ch(&run_once(&s, &f(4, 0.5), &mut rt)), 50);
        assert_eq!(ch(&run_once(&s, &f(5, 0.0), &mut rt)), 100, "the route's own beat doesn't restart it");
        assert_eq!(ch(&run_once(&s, &f(8, 0.0), &mut rt)), 0, "the next bar does");
    }

    #[test]
    fn a_route_switched_on_waits_for_the_next_hit() {
        let mut s = laser_show();
        s.routes = vec![Route { id: "r".into(), enabled: false, source: "beat".into(), channel: "p1_pattern".into(),
            action: RouteAction::Cycle { values: vec![10, 20, 30] }, ..Default::default() }];
        let ch = |out: &HashMap<String, Vec<u8>>| out["L"][s.profiles[0].modes[0].channels.iter().position(|c| c == "p1_pattern").unwrap()];
        let f = |beats: u64| Features { beats, ..Default::default() };
        let mut rt = Runtime::default();
        run_once(&s, &f(5), &mut rt);
        s.routes[0].enabled = true;
        assert_eq!(ch(&run_once(&s, &f(5), &mut rt)), 10, "no stale beat when it comes on");
        assert_eq!(ch(&run_once(&s, &f(6), &mut rt)), 20);
    }

    #[test]
    fn once_plays_a_motion_through_and_holds() {
        let fe = |b: u64| Features { beats: b, ..Default::default() };
        let m = Motion { kind: "list".into(), values: vec![1, 2, 3], clock: "beat".into(), period: 1.0, once: true, ..Default::default() };
        let seq: Vec<u8> = (0..6).map(|b| { let f = fe(b); motion_value(&m, 0, &MotionCtx { feat: &f, secs: 0.0, hits: 0, offset: 0.0, since_hit: None }, &[]) }).collect();
        assert_eq!(seq, [1, 2, 3, 3, 3, 3]);
        let sw = Motion { kind: "sweep".into(), ranges: vec![[0, 100]], period: 2.0, shape: "up".into(), once: true, ..Default::default() };
        let v = |b: u64| { let f = fe(b); motion_value(&sw, 0, &MotionCtx { feat: &f, secs: 0.0, hits: 0, offset: 0.0, since_hit: None }, &[]) };
        assert_eq!((v(1), v(2), v(5)), (50, 100, 100));
    }

    #[test]
    fn look_maps_onto_other_head() {
        let s = laser_show();
        let p = &s.profiles[0];
        let vals = look_values(&p.modes[0], &p.looks[0], "P2");
        let m: HashMap<String, u8> = vals.into_iter().map(|(k, v, _)| (k, v)).collect();
        assert_eq!(m.get("p2_pattern"), Some(&12));
        assert_eq!(m.get("p2_zoom"), Some(&140));
        // P2 has no gallery channel, so that part of the look is dropped, not misplaced
        assert_eq!(m.len(), 2, "{m:?}");
    }

    #[test]
    fn default_looks_are_regenerated_without_touching_yours() {
        let mut p = laser_show().profiles.remove(0);
        let made = p.generate_default_looks(4);
        assert!(made >= 8, "two heads × pattern ranges, got {made}");
        assert!(p.looks.iter().filter(|l| l.builtin).all(|l| l.tags.contains(&"default".to_string())));
        assert!(p.looks.iter().any(|l| l.builtin && l.head == "P2"));
        let again = p.generate_default_looks(2);
        assert_eq!(p.looks.iter().filter(|l| l.builtin).count(), again);
        assert_eq!(p.looks.iter().filter(|l| !l.builtin).count(), 2, "your looks are kept");
    }

    fn ctx_at(beats: u64, phase: f32) -> Features {
        Features { beats, beat_phase: phase, ..Default::default() }
    }

    #[test]
    fn sweep_covers_multiple_ranges_and_returns() {
        // two ranges = one strip of 10 values: 10..14 then 200..204
        let m = Motion { kind: "sweep".into(), ranges: vec![[10, 14], [200, 204]], period: 4.0, shape: "pingpong".into(), ..Default::default() };
        let v = |b: u64, ph: f32| { let f = ctx_at(b, ph); motion_value(&m, 0, &MotionCtx { feat: &f, secs: 0.0, hits: 0, offset: 0.0, since_hit: None }, &[]) };
        assert_eq!(v(0, 0.0), 10, "starts at the first value");
        assert_eq!(v(2, 0.0), 204, "half way through a ping-pong is the far end");
        assert_eq!(v(4, 0.0), 10, "and back again");
        assert!([12, 13, 200, 201].contains(&v(1, 0.0)) || v(1, 0.0) == 14, "in between walks through both ranges");
    }

    #[test]
    fn step_and_list_advance_per_beat() {
        let step = Motion { kind: "step".into(), ranges: vec![[0, 3]], period: 1.0, step: 2, ..Default::default() };
        let list = Motion { kind: "list".into(), values: vec![7, 9, 11], period: 1.0, ..Default::default() };
        let seq = |m: &Motion| (0..5).map(|b| { let f = ctx_at(b, 0.2); motion_value(m, 0, &MotionCtx { feat: &f, secs: 0.0, hits: 0, offset: 0.0, since_hit: None }, &[]) }).collect::<Vec<_>>();
        assert_eq!(seq(&step), [0, 2, 0, 2, 0]);
        assert_eq!(seq(&list), [7, 9, 11, 7, 9]);
    }

    #[test]
    fn seeded_random_is_repeatable_and_in_range() {
        let m = Motion { kind: "random".into(), ranges: vec![[20, 29], [100, 109]], period: 1.0, seed: 42, ..Default::default() };
        let run = |m: &Motion| (0..32).map(|b| { let f = ctx_at(b, 0.0); motion_value(m, 0, &MotionCtx { feat: &f, secs: 0.0, hits: 0, offset: 0.0, since_hit: None }, &[]) }).collect::<Vec<_>>();
        let a = run(&m);
        assert_eq!(a, run(&m), "same seed, same sequence");
        assert!(a.iter().all(|v| (20..=29).contains(v) || (100..=109).contains(v)));
        assert!(a.iter().collect::<std::collections::HashSet<_>>().len() > 5, "actually varies");
        assert_ne!(a, run(&Motion { seed: 43, ..m.clone() }), "a new seed gives a different sequence");
    }

    #[test]
    fn catalog_motion_plays_tagged_patterns_on_hits() {
        let mut s = laser_show();
        {
            let ch = s.profiles[0].channels.iter_mut().find(|c| c.key == "p1_pattern").unwrap();
            for (v, t) in [(30, "peak"), (5, "peak"), (60, "chill"), (90, "peak")] {
                ch.values.push(ValueNote { value: v, tags: vec![t.into()], ..Default::default() });
            }
        }
        let p = &s.profiles[0];
        let m = Motion { kind: "catalog".into(), tags: vec!["peak".into()], clock: "hit".into(), period: 1.0, ..Default::default() };
        let cat = catalog(p, "p1_pattern", &m);
        assert_eq!(cat, [5, 30, 90]);
        let f = Features::default();
        let at = |hits| motion_value(&m, 0, &MotionCtx { feat: &f, secs: 0.0, hits, offset: 0.0, since_hit: None }, &cat);
        assert_eq!([at(0), at(1), at(2), at(3)], [5, 30, 90, 5]);
    }

    #[test]
    fn motions_follow_looks_onto_other_heads() {
        let mut s = laser_show();
        s.profiles[0].looks[0].motions.insert("p1_zoom".into(), Motion { kind: "sweep".into(), ranges: vec![[128, 159]], ..Default::default() });
        let p = &s.profiles[0];
        let vals = look_values(&p.modes[0], &p.looks[0], "P2");
        let zoom = vals.iter().find(|(k, _, _)| k == "p2_zoom").unwrap();
        assert_eq!(zoom.2.as_ref().map(|m| m.ranges.clone()), Some(vec![[128, 159]]));
    }

    #[test]
    fn node_hop_uses_height() {
        // From a, the floor fixtures b (1 m) and d (2 m) are nearer than c, which sits
        // straight above a but 3 m up. Ignoring height, c would be the nearest (0 m).
        let a = fixture("a", "p", 0.0, 0.0);
        let b = fixture("b", "p", 1.0, 0.0);
        let d = fixture("d", "p", 2.0, 0.0);
        let c = fixture("c", "p", 0.0, 3.0);
        let st = RouteState { hop_current: Some("a".into()), ..Default::default() };
        for _ in 0..50 {
            let next = hop_next(&[&a, &b, &c, &d], &st, None);
            assert!(next == "b" || next == "d", "hopped to {next}");
        }
    }
}

// ---------- fixture looks (colour + brightness) ----------

/// The channels an effect uses on a fixture.
#[derive(Debug, Clone, Default)]
pub struct LookChans {
    intensity: Vec<String>,
    rgb: Option<[String; 3]>,
    white: Option<String>,
    /// Colour-macro channels: (key, [(value, colour)]) — lasers, fog, cheap pars.
    macros: Vec<(String, Vec<(u8, [u8; 3])>)>,
}

pub(crate) fn look_chans(profiles: &HashMap<&str, &Profile>, fx: &Fixture) -> Option<LookChans> {
    let p = profiles.get(fx.profile_id.as_str())?;
    let mode = p.mode(&fx.mode)?;
    let mut lc = LookChans::default();
    let (mut r, mut g, mut b) = (None, None, None);
    for key in &mode.channels {
        let Some(c) = p.channel(key) else { continue };
        let name = c.name.to_lowercase();
        let is = |w: &str| c.key == w || name == w || c.key.ends_with(&format!("_{w}"));
        match c.group.as_str() {
            "intensity" => lc.intensity.push(key.clone()),
            "color" if is("red") => r = Some(key.clone()),
            "color" if is("green") => g = Some(key.clone()),
            "color" if is("blue") => b = Some(key.clone()),
            "color" if is("white") => lc.white = Some(key.clone()),
            "color" => {
                let m: Vec<(u8, [u8; 3])> = c
                    .capabilities
                    .iter()
                    .filter_map(|cap| Some((((cap.min as u16 + cap.max as u16) / 2) as u8, parse_hex(cap.color.as_deref()?)?)))
                    .filter(|(_, c)| c.iter().any(|x| *x > 0)) // skip "off"/black macros
                    .collect();
                if m.len() >= 2 {
                    lc.macros.push((key.clone(), m));
                }
            }
            _ => {}
        }
    }
    if let (Some(r), Some(g), Some(b)) = (r, g, b) {
        lc.rgb = Some([r, g, b]);
    }
    (!lc.intensity.is_empty() || lc.rgb.is_some() || !lc.macros.is_empty()).then_some(lc)
}

fn set_color(values: &mut HashMap<Key, u8>, fx: &str, lc: &LookChans, c: [u8; 3]) {
    if let Some(keys) = &lc.rgb {
        for (k, v) in keys.iter().zip(c) {
            values.insert((fx.to_string(), k.clone()), v);
        }
        if let Some(w) = &lc.white {
            values.insert((fx.to_string(), w.clone()), 0);
        }
    }
    for (key, options) in &lc.macros {
        let best = options
            .iter()
            .min_by_key(|(_, o)| o.iter().zip(c).map(|(a, b)| (*a as i32 - b as i32).pow(2)).sum::<i32>())
            .map(|(v, _)| *v);
        if let Some(v) = best {
            values.insert((fx.to_string(), key.clone()), v);
        }
    }
}

/// Brightness 0..1. Fixtures without a dimmer get their RGB scaled instead.
fn set_intensity(values: &mut HashMap<Key, u8>, fx: &str, lc: &LookChans, level: f32) {
    let level = level.clamp(0.0, 1.0);
    if !lc.intensity.is_empty() {
        for k in &lc.intensity {
            values.insert((fx.to_string(), k.clone()), (level * 255.0).round() as u8);
        }
    } else if let Some(keys) = &lc.rgb {
        for k in keys {
            let e = values.entry((fx.to_string(), k.clone())).or_insert(255);
            *e = (*e as f32 * level).round() as u8;
        }
    }
}

fn palette_colors(show: &Show, id: &str) -> Vec<[u8; 3]> {
    let cols: Vec<[u8; 3]> = show
        .palettes
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.colors.iter().filter_map(|c| parse_hex(c)).collect())
        .unwrap_or_default();
    if cols.is_empty() { vec![[255, 255, 255]] } else { cols }
}

/// Crossfade a fixture toward `target`; returns the colour to show now.
fn fade_to(colors: &mut HashMap<String, ([u8; 3], [u8; 3], Instant)>, fx: &str, target: [u8; 3], fade_ms: u32, now: Instant) -> [u8; 3] {
    let shown = |(from, to, start): &([u8; 3], [u8; 3], Instant)| {
        let t = if fade_ms == 0 { 1.0 } else { (now.duration_since(*start).as_millis() as f32 / fade_ms as f32).min(1.0) };
        [0, 1, 2].map(|i| (from[i] as f32 + (to[i] as f32 - from[i] as f32) * t).round() as u8)
    };
    match colors.get(fx) {
        Some(e) if e.1 == target => shown(e),
        Some(e) => {
            let cur = shown(e);
            colors.insert(fx.to_string(), (cur, target, now));
            cur
        }
        None => {
            colors.insert(fx.to_string(), (target, target, now));
            target
        }
    }
}

/// Rank of each target along the room's x axis (then y), for "spread" palettes.
fn position_order(t: &[(&Fixture, LookChans)]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..t.len()).collect();
    idx.sort_by(|a, b| (t[*a].0.x, t[*a].0.y).partial_cmp(&(t[*b].0.x, t[*b].0.y)).unwrap_or(std::cmp::Ordering::Equal));
    let mut rank = vec![0; t.len()];
    for (r, i) in idx.into_iter().enumerate() {
        rank[i] = r;
    }
    rank
}

/// Fixture positions along an axis, normalised to 0..1 across the targets.
/// "radial" and "angle" are measured from `origin` (the focus point or the targets' centre).
fn normalized_positions(t: &[(&Fixture, LookChans)], axis: &str, origin: (f32, f32, f32)) -> Vec<f32> {
    let (ox, oy, _) = origin;
    if axis == "angle" {
        // Around the origin, 0..1 starting at "north"; circular, so no end gap.
        return t
            .iter()
            .map(|(f, _)| {
                let (x, y, _) = f.pos();
                let a = (x - ox).atan2(-(y - oy)); // -pi..pi, 0 = towards the front wall
                (a / std::f32::consts::TAU + 1.0).fract()
            })
            .collect();
    }
    let raw: Vec<f32> = t
        .iter()
        .map(|(f, _)| {
            let (x, y, z) = f.pos();
            match axis {
                "y" => y,
                "height" => z,
                "radial" => ((x - ox).powi(2) + (y - oy).powi(2)).sqrt(),
                _ => x,
            }
        })
        .collect();
    let (lo, hi) = raw.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    // Radial distances start at the origin, not at the nearest fixture.
    let lo = if axis == "radial" { 0.0 } else { lo };
    if hi - lo < 1e-3 {
        return vec![0.0; raw.len()];
    }
    // Leave a gap at the end so the first and last fixtures aren't lit together on wrap.
    raw.iter().map(|v| (v - lo) / (hi - lo) * 0.85).collect()
}

/// A look's values for `head`. Looks saved on one head (P1) map onto another (P2)
/// by name: "p1_pattern" -> "p2_pattern". Heads don't always share a layout (the
/// L2600's P2 has no gallery), so channels the other head lacks are dropped rather
/// than guessed. Channels outside the look's head (shared ones) stay as they are.
pub fn look_values(mode: &Mode, look: &Look, head: &str) -> Vec<(String, u8, Option<Motion>)> {
    let with_motion = |k: &String, out: String, v: u8| (out, v, look.motions.get(k).cloned());
    if head.is_empty() || head == look.head {
        return look.values.iter().map(|(k, v)| with_motion(k, k.clone(), *v)).collect();
    }
    let src = mode.heads.iter().find(|h| h.name == look.head);
    let dst = mode.heads.iter().find(|h| h.name == head);
    let (Some(src), Some(dst)) = (src, dst) else { return vec![] };
    let (sp, dp) = (format!("{}_", src.name.to_lowercase()), format!("{}_", dst.name.to_lowercase()));
    look.values
        .iter()
        .filter_map(|(k, v)| {
            if !src.channels.contains(k) {
                return Some(with_motion(k, k.clone(), *v)); // shared channel
            }
            let mapped = format!("{dp}{}", k.strip_prefix(&sp)?);
            dst.channels.contains(&mapped).then(|| with_motion(k, mapped, *v))
        })
        .collect()
}

// ---------- motions ----------

pub struct MotionCtx<'a> {
    pub feat: &'a Features,
    /// Seconds since the look started.
    pub secs: f32,
    /// Route hits since the look started.
    pub hits: u64,
    /// Extra position along the motion, in periods (fixtures chasing each other).
    pub offset: f32,
    /// Seconds and beats since the route last fired (None = not yet), for motions that
    /// restart on each hit.
    pub since_hit: Option<(f32, f32)>,
}

/// Catalogued values for a "catalog" motion on channel `key`, in value order.
fn catalog(p: &Profile, key: &str, m: &Motion) -> Vec<u8> {
    if m.kind != "catalog" {
        return vec![];
    }
    let Some(ch) = p.channel(key) else { return vec![] };
    let mut v: Vec<u8> = ch
        .values
        .iter()
        .filter(|n| !n.tags.iter().any(|t| t == TAG_BLOCKED))
        .filter(|n| m.tags.iter().all(|t| n.tags.contains(t)))
        .map(|n| n.value)
        .collect();
    v.sort_unstable();
    v
}

/// Deterministic hash for seeded random motions (splitmix64).
fn mix(seed: u64, n: u64) -> u64 {
    let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ n.wrapping_add(0x632B_E59B_D9B4_E019);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Value of a moving channel right now. Ranges are walked in order as one long
/// strip of values, so several ranges behave like one.
pub fn motion_value(m: &Motion, base: u8, ctx: &MotionCtx, catalog: &[u8]) -> u8 {
    let ranges: Vec<(u8, u8)> = m.ranges.iter().map(|r| (r[0].min(r[1]), r[0].max(r[1]))).collect();
    let total: u32 = ranges.iter().map(|(a, b)| (*b - *a) as u32 + 1).sum();
    let at = |i: u32| -> u8 {
        let mut i = i % total.max(1);
        for (a, b) in &ranges {
            let len = (*b - *a) as u32 + 1;
            if i < len {
                return a + i as u8;
            }
            i -= len;
        }
        base
    };
    let period = m.period.max(0.05);
    // Restarting on each hit: time and beats count from the last hit (nothing before the first).
    let (secs, beats) = if m.retrigger && m.clock != "hit" {
        match ctx.since_hit {
            Some(s) => s,
            None => return base,
        }
    } else {
        (ctx.secs, ctx.feat.beats as f32 + ctx.feat.beat_phase)
    };
    // continuous position (sweeps) and step count (everything else)
    let (pos, steps) = match m.clock.as_str() {
        "time" => {
            let p = secs / period + ctx.offset;
            (p, p.floor() as u64)
        }
        "hit" => (ctx.hits as f32 / period + ctx.offset, (ctx.hits as f32 / period.round().max(1.0) + ctx.offset).floor() as u64),
        _ => {
            let p = beats / period + ctx.offset;
            (p, p.floor() as u64)
        }
    };
    // Once: one pass, then hold the end.
    let last = |n: u64| if m.once { steps.min(n.saturating_sub(1)) } else { steps };
    match m.kind.as_str() {
        "sweep" if total > 0 => {
            let f = if m.once { pos.clamp(0.0, 1.0) } else { pos.fract() };
            let u = match m.shape.as_str() {
                "down" => 1.0 - f,
                "sine" => 0.5 - 0.5 * (f * std::f32::consts::TAU).cos(),
                "up" => f,
                _ => if f < 0.5 { f * 2.0 } else { 2.0 - f * 2.0 }, // pingpong
            };
            at(((u * (total - 1) as f32).round()) as u32)
        }
        "step" if total > 0 => at((last((total as u64).div_ceil(m.step.max(1) as u64)) * m.step.max(1) as u64 % total as u64) as u32),
        "random" if total > 0 => at((mix(m.seed as u64, steps) % total as u64) as u32),
        "list" if !m.values.is_empty() => m.values[(last(m.values.len() as u64) % m.values.len() as u64) as usize],
        "catalog" if !catalog.is_empty() => catalog[(last(catalog.len() as u64) % catalog.len() as u64) as usize],
        "follow" if total > 0 => {
            let lvl = source_level(&m.source, ctx.feat).unwrap_or(0.0).clamp(0.0, 1.0);
            at((lvl * (total - 1) as f32).round() as u32)
        }
        _ => base,
    }
}

fn centroid(fx: &[&Fixture]) -> (f32, f32, f32) {
    let n = fx.len().max(1) as f32;
    (
        fx.iter().map(|f| f.pos().0).sum::<f32>() / n,
        fx.iter().map(|f| f.pos().1).sum::<f32>() / n,
        fx.iter().map(|f| f.pos().2).sum::<f32>() / n,
    )
}

fn weighted_pick(w: &[f32]) -> usize {
    let total: f32 = w.iter().sum();
    if total <= 0.0 {
        return fastrand::usize(..w.len());
    }
    let mut r = fastrand::f32() * total;
    for (i, x) in w.iter().enumerate() {
        r -= x;
        if r <= 0.0 {
            return i;
        }
    }
    w.len() - 1
}
