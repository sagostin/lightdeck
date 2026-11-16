//! AI show designer: describes the rig to the model, asks for a show plan
//! (palettes, band tuning, routes grouped into scenes, scene presets and MIDI /
//! keyboard mappings), then checks every reference against the real show so the
//! plan can be applied as-is. Nothing here changes the show: the UI previews the
//! plan and applies the parts the user keeps.

use crate::ai::{self, AiSettings};
use crate::engine::{channel_keys, look_chans, resolve_fixtures};
use crate::model::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GenreHint {
    pub name: String,
    pub bpm: [f32; 2],
    /// How the genre tends to be lit.
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Scope {
    pub routes: bool,
    pub presets: bool,
    pub audio: bool,
    pub palettes: bool,
    pub controls: bool,
}

impl Default for Scope {
    fn default() -> Self {
        Self { routes: true, presets: true, audio: true, palettes: true, controls: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DesignRequest {
    pub prompt: String,
    pub genre: Option<GenreHint>,
    pub scope: Scope,
    /// MIDI controller model ("Akai APC mini mk2"), or empty for none.
    pub controller: String,
    /// MIDI channel the controller sends on (1-16; 0 = any).
    pub midi_channel: u8,
    /// Also map computer-keyboard keys.
    pub keyboard: bool,
    /// A plan to refine (the user's prompt then says what to change).
    pub previous: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Plan {
    pub title: String,
    pub summary: String,
    /// Global onset sensitivity, if the plan changes it.
    pub sensitivity: Option<f32>,
    /// Band changes, merged into the show's bands by id.
    pub bands: Vec<BandDef>,
    pub palettes: Vec<Palette>,
    pub routes: Vec<Route>,
    pub presets: Vec<Preset>,
    pub controls: Vec<ControlMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DesignResult {
    pub plan: Plan,
    /// What was fixed or dropped while checking the plan against the rig.
    pub warnings: Vec<String>,
}

const SYSTEM: &str = r##"You are a lighting designer programming LightDeck, an audio-reactive DMX/Zigbee lighting controller. You receive the rig (fixtures, channel types, groups, rooms, focus points, saved looks, detection bands, palettes, existing routes/controls) and a brief. Reply with a show plan as ONE JSON object, no prose, no code fences.

How LightDeck works
- Detection bands analyse the audio. Each band id is a TRIGGER source (fires on hits) and "<id>.level" is a LEVEL source (0..1). Other sources: "beat", "bar" (triggers); "level", "energy" (rises in builds, peaks on drops), "phase" (beat phase 0..1) (levels). The first band drives the beat clock, so keep kick first.
- A route = source -> action on target fixtures. Targets: "rooms" scope, then "fixtures" and "groups" (ids) pick within; all empty = every fixture.
- "channel": a channel key, or "@<type>" for every channel of that type (types: intensity, color, pan, tilt, pattern, beam, effect, speed, strobe, mode, fog, other). Needed for modulate/flash/random/node_hop/cycle. Ignored ("") for palette/twinkle/wave/strobe (they drive brightness+colour of whole fixtures) and look.
- "focus": a focus point id effects centre on ("" = centre of targets); "falloff": dim with distance from it.
- Routes are organised by "group" (a folder name). Use groups as SCENES ("Warm-up", "Groove", "Build", "Drop", "Breakdown", "Always on").
- Route fields: id, name, group, enabled, source, rooms, fixtures, groups, channel, focus, falloff, action, cooldown_ms, every (fire every Nth event, >=1), probability (0..1).
- Actions (field "type"):
  modulate {min,max 0-255, smoothing 0..0.98}            level source -> channel value
  flash {on, off 0-255, decay_ms}                         trigger -> jump to on, decay to off
  random {tags:[...], hold_ms}                            trigger -> random value whose capability/value carries all tags
  node_hop {on, off, decay_ms, palette}                   trigger -> one fixture flashes, hops to a neighbour next hit
  cycle {values:[...]}                                    trigger -> step through values
  strobe {rate:"free"|"1"|"2"|"4"|"8" (flashes per beat), hz, duty 0.05..0.9, burst_ms, threshold 0..1}
  palette {palette, mode:"all"|"spread"|"rings"|"around"|"random", fade_ms, brightness 0-255 (0 = leave brightness to other routes)}  each trigger steps colour
  twinkle {palette, amount, decay_ms, base 0-255}         sparkles; amount = per second (level source) or per hit
  wave {palette, axis:"x"|"y"|"radial"|"angle"|"height", beats, width 0.05..1, base}  moving band of light, beat-synced (use source "beat")
  look {set:"" or look-set id, include_defaults, tags:[...], mode:"random"|"cycle", hold_ms, heads:[...]}  applies saved looks (lasers/patterns) chosen by tags
- Presets (scenes): {id, name, color "#rrggbb", values:[{fixture, channel, value}], routes:{route_id: true|false}, exclusive}. A preset with routes switches those routes; every generated route NOT listed in a scene preset is switched OFF by it, so list the ones that should be on. Keep "values" empty unless a fixed static look is wanted.
- Controls: {id, name, enabled, trigger:{source:"midi"|"key", kind:"note"|"cc"|"program", channel 0-16 (0 = any), number 0-127, key (lower-case KeyboardEvent.key for source "key"), mode:"toggle"|"momentary"|"on"|"off"|"fader"}, action}.
  Control actions (field "type"): routes {routes:[ids]}, route_group {group}, bands {bands:[band ids]} (mute), blackout {}, master {} (fader = master brightness), preset {preset}, release_manual {}, channel_value {target:{rooms,groups,fixtures}, channel, min, max}, band_sensitivity {band} (fader).
  Faders/knobs: kind "cc", mode "fader". Pads/buttons: kind "note"; scene presets use mode "on"; momentary effects (strobe, fog blast) use "momentary".
- Bands: {id (lower-case slug), name, enabled, ranges:[{lo_hz, hi_hz, weight}], detect:"transient"|"level", level_threshold, gain_db, sensitivity 0..1, min_gap_ms, release 0..0.98, color}. Bands are MERGED by id: reuse existing ids to retune them; new ids add bands. Never rename ids existing routes use.

Design rules
- Only use ids, channel keys, look tags and look sets that exist in the rig (or that you create in this plan). Prefer groups and "@type" channels over long fixture lists.
- Respect fixture kinds: never strobe or flash fog continuously (fog: short flash on drops/bars, with cooldown_ms >= 20000); lasers use look routes with tags from their catalogue and avoid tags "blocked"/"large_projection" unless asked.
- Make scenes musically distinct: low energy = slow palette fades, waves over many beats, gentle twinkle, no strobe; builds = energy-modulated brightness, faster waves, rising strobe rate; drops = kick-driven flashes/palette steps, strobe on snare or high energy, laser looks; breakdowns = vocals.level modulation, sparse twinkle.
- New palettes: 2-6 colours, named for the mood. Reuse existing palettes when they fit.
- Only the routes of the starting scene should have "enabled": true.
- Give routes, presets and controls short human names. Use simple ids like "r1", "p1", "c1", "pal1"; LightDeck renumbers them.
- MIDI: if a controller model is named, use its real factory note/CC layout (e.g. pad grid notes, fader CCs) and say which pad/fader does what in "summary". With no controller, use notes from 36 upward and CCs from 1 upward. Don't reuse a trigger that the rig's existing controls already use.

Reply shape:
{"title": short name for this design, "summary": 2-6 sentences: the scenes, how to perform it, and the controller layout,
 "sensitivity": number or null, "bands": [...], "palettes": [...], "routes": [...], "presets": [...], "controls": [...]}
Leave out (empty array) any section you were not asked for."##;

/// The user message: the rig, what to generate, and the brief.
pub fn build_user(show: &Show, req: &DesignRequest) -> String {
    let mut user = String::new();
    user.push_str("RIG\n");
    user.push_str(&serde_json::to_string(&rig_context(show)).unwrap_or_default());
    user.push_str("\n\nWHAT TO GENERATE\n");
    let s = &req.scope;
    for (on, what) in [
        (s.audio, "bands + sensitivity: tune detection for the music"),
        (s.palettes, "palettes"),
        (s.routes, "routes grouped into scenes"),
        (s.presets, "scene presets that switch the routes"),
        (s.controls, "controls (MIDI and/or keyboard mappings)"),
    ] {
        user.push_str(&format!("- {what}: {}\n", if on { "yes" } else { "no (empty array)" }));
    }
    if let Some(g) = &req.genre {
        user.push_str(&format!("\nGENRE: {} ({:.0}-{:.0} BPM). Typical lighting: {}\n", g.name, g.bpm[0], g.bpm[1], g.style));
    }
    if s.controls {
        if req.controller.trim().is_empty() {
            user.push_str("\nMIDI CONTROLLER: none named");
        } else {
            user.push_str(&format!("\nMIDI CONTROLLER: {}", req.controller.trim()));
        }
        user.push_str(&format!(
            ", channel {}\nKEYBOARD KEYS: {}\n",
            if req.midi_channel == 0 { "any".to_string() } else { req.midi_channel.to_string() },
            if req.keyboard { "also map number keys / letters (avoid \"b\", it is blackout)" } else { "no" }
        ));
    }
    if let Some(prev) = &req.previous {
        user.push_str("\nPREVIOUS PLAN (revise it as the brief asks; keep what isn't mentioned)\n");
        user.push_str(&serde_json::to_string(prev).unwrap_or_default());
        user.push('\n');
    }
    user.push_str("\nBRIEF\n");
    user.push_str(if req.prompt.trim().is_empty() { "Design a complete, varied show for this rig." } else { req.prompt.trim() });
    user
}

pub async fn design(settings: &AiSettings, show: &Show, req: &DesignRequest) -> Result<DesignResult, String> {
    let s = &req.scope;
    let user = build_user(show, req);
    let text = ai::complete_long(settings, SYSTEM, &user).await?;
    let mut raw: Value = ai::extract_json(&text)
        .and_then(|j| serde_json::from_str(j).ok())
        .ok_or("The model reply did not contain a valid JSON plan")?;
    restrict(&mut raw, s);
    let (plan, warnings) = sanitize(show, &raw);
    Ok(DesignResult { plan, warnings })
}

/// Drop the parts of a raw reply that weren't asked for, before checking it, so
/// nothing kept can point at something that was dropped.
pub fn restrict(raw: &mut Value, s: &Scope) {
    for (on, keys) in [
        (s.audio, &["bands", "sensitivity"][..]),
        (s.palettes, &["palettes"][..]),
        (s.routes, &["routes"][..]),
        (s.presets, &["presets"][..]),
        (s.controls, &["controls"][..]),
    ] {
        if !on {
            if let Some(o) = raw.as_object_mut() {
                for k in keys {
                    o.remove(*k);
                }
            }
        }
    }
}

/// A compact description of the rig for the prompt.
pub fn rig_context(show: &Show) -> Value {
    let used: HashSet<&str> = show.fixtures.iter().map(|f| f.profile_id.as_str()).collect();
    let profiles: Vec<Value> = show
        .profiles
        .iter()
        .filter(|p| used.contains(p.id.as_str()))
        .map(|p| {
            let mode = p.modes.first();
            let chans: Vec<Value> = mode
                .map(|m| m.channels.iter().filter_map(|k| p.channel(k)).collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    let mut tags: Vec<&str> = c.capabilities.iter().flat_map(|x| x.tags.iter().map(|s| s.as_str())).collect();
                    tags.extend(c.values.iter().flat_map(|v| v.tags.iter().map(|s| s.as_str())));
                    tags.sort();
                    tags.dedup();
                    json!({"key": c.key, "type": c.group, "name": c.name, "tags": tags})
                })
                .collect();
            // Look tags (with counts) say more than a long list of looks.
            let mut look_tags: BTreeMap<&str, usize> = BTreeMap::new();
            for l in &p.looks {
                for t in &l.tags {
                    *look_tags.entry(t.as_str()).or_default() += 1;
                }
            }
            let favourites: Vec<Value> = {
                let mut ls: Vec<&Look> = p.looks.iter().filter(|l| !l.builtin).collect();
                ls.sort_by(|a, b| b.rating.cmp(&a.rating));
                ls.iter().take(25).map(|l| json!({"name": l.name, "tags": l.tags, "rating": l.rating, "head": l.head})).collect()
            };
            json!({
                "id": p.id, "name": format!("{} {}", p.manufacturer, p.model).trim(), "kind": p.kind,
                "channels": chans,
                "heads": mode.map(|m| m.heads.iter().map(|h| h.name.clone()).collect::<Vec<_>>()).unwrap_or_default(),
                "looks": p.looks.len(), "look_tags": look_tags,
                "look_sets": p.look_sets.iter().map(|s| json!({"id": s.id, "name": s.name})).collect::<Vec<_>>(),
                "own_looks": favourites,
            })
        })
        .collect();
    let fixtures: Vec<Value> = show
        .fixtures
        .iter()
        .filter(|f| f.enabled)
        .map(|f| json!({"id": f.id, "name": f.name, "type": f.profile_id, "room": f.room_id, "x": f.x, "y": f.y, "z": f.z}))
        .collect();
    json!({
        "fixture_types": profiles,
        "fixtures": fixtures,
        "groups": show.groups.iter().map(|g| json!({"id": g.id, "name": g.name, "fixtures": g.fixtures})).collect::<Vec<_>>(),
        "rooms": show.rooms.iter().map(|r| json!({
            "id": r.id, "name": r.name, "ceiling_m": r.ceiling_m,
            "focus": r.focus.iter().map(|f| json!({"id": f.id, "name": f.name, "x": f.x, "y": f.y, "radius": f.radius})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "bands": show.audio.bands.iter().map(|b| json!({
            "id": b.id, "name": b.name, "detect": b.detect, "sensitivity": b.sensitivity,
            "ranges": b.ranges.iter().map(|r| [r.lo_hz, r.hi_hz]).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "sensitivity": show.audio.sensitivity,
        "palettes": show.palettes.iter().map(|p| json!({"id": p.id, "name": p.name, "colors": p.colors})).collect::<Vec<_>>(),
        "existing_routes": show.routes.iter().map(|r| json!({"id": r.id, "name": r.name, "group": r.group, "source": r.source})).collect::<Vec<_>>(),
        "existing_presets": show.presets.iter().map(|p| json!({"id": p.id, "name": p.name})).collect::<Vec<_>>(),
        "existing_controls": show.controls.iter().map(|c| json!({"name": c.name, "trigger": c.trigger})).collect::<Vec<_>>(),
    })
}

fn new_ids() -> impl FnMut(&str) -> String {
    let tag: String = (0..4).map(|_| fastrand::alphanumeric().to_ascii_lowercase()).collect();
    let mut n = 0;
    move |kind: &str| {
        n += 1;
        format!("ai-{kind}-{tag}{n}")
    }
}

fn hex_ok(c: &str) -> bool {
    parse_hex(c).is_some()
}

/// Fields stored as 0-255 integers (DMX values, MIDI numbers).
const BYTE_KEYS: &[&str] = &["on", "off", "min", "max", "base", "brightness", "value", "number", "channel"];

/// Round and clamp integer fields so one stray number ("on": 300, "base": 127.5,
/// "decay_ms": -10) doesn't make a whole route or control unreadable.
fn fix_numbers(v: &mut Value) {
    match v {
        Value::Object(o) => {
            for (k, x) in o.iter_mut() {
                let byte = BYTE_KEYS.contains(&k.as_str());
                let whole = k.ends_with("_ms") || k == "every";
                match x {
                    Value::Number(n) if byte || whole => {
                        let f = n.as_f64().unwrap_or(0.0).round();
                        *x = json!(if byte { f.clamp(0.0, 255.0) } else { f.clamp(0.0, 4.0e9) } as u64);
                    }
                    // cycle values
                    Value::Array(a) if k == "values" => {
                        for e in a.iter_mut() {
                            if let Some(f) = e.as_f64() {
                                *e = json!(f.round().clamp(0.0, 255.0) as u64);
                            } else {
                                fix_numbers(e);
                            }
                        }
                    }
                    _ => fix_numbers(x),
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(fix_numbers),
        _ => {}
    }
}

/// Check a raw plan against the show: renumber ids, fix or drop anything that
/// points at something that doesn't exist, clamp values into range.
pub fn sanitize(show: &Show, raw: &Value) -> (Plan, Vec<String>) {
    let mut raw = raw.clone();
    fix_numbers(&mut raw);
    let raw = &raw;
    let mut warn = Vec::new();
    let mut id = new_ids();
    let mut plan = Plan {
        title: raw["title"].as_str().unwrap_or("AI design").chars().take(60).collect(),
        summary: raw["summary"].as_str().unwrap_or_default().to_string(),
        sensitivity: raw["sensitivity"].as_f64().map(|v| (v as f32).clamp(0.0, 1.0)),
        ..Default::default()
    };
    let list = |k: &str| raw[k].as_array().cloned().unwrap_or_default();

    // ---- palettes
    let mut pal_map: HashMap<String, String> = HashMap::new();
    for v in list("palettes") {
        let Ok(mut p) = serde_json::from_value::<Palette>(v) else {
            warn.push("Skipped a palette that wasn't readable".into());
            continue;
        };
        p.colors.retain(|c| hex_ok(c));
        if p.colors.is_empty() {
            warn.push(format!("Skipped palette \"{}\": no valid colours", p.name));
            continue;
        }
        let new = id("pal");
        pal_map.insert(p.id.clone(), new.clone());
        p.id = new;
        if p.name.trim().is_empty() {
            p.name = "AI palette".into();
        }
        plan.palettes.push(p);
    }
    let palette_ids: HashSet<String> = show.palettes.iter().map(|p| p.id.clone()).chain(plan.palettes.iter().map(|p| p.id.clone())).collect();
    let fallback_palette = show.palettes.first().map(|p| p.id.clone()).or_else(|| plan.palettes.first().map(|p| p.id.clone())).unwrap_or_default();
    let fix_palette = |p: &mut String, route: &str, warn: &mut Vec<String>| {
        if let Some(n) = pal_map.get(p.as_str()) {
            *p = n.clone();
        } else if !palette_ids.contains(p.as_str()) {
            // Match by name as a courtesy ("Club").
            if let Some(found) = show.palettes.iter().chain(plan.palettes.iter()).find(|x| x.name.eq_ignore_ascii_case(p)) {
                *p = found.id.clone();
            } else {
                warn.push(format!("{route}: unknown palette \"{p}\", using the first palette"));
                *p = fallback_palette.clone();
            }
        }
    };

    // ---- bands
    let mut seen_bands = HashSet::new();
    for v in list("bands") {
        let Ok(mut b) = serde_json::from_value::<BandDef>(v) else {
            warn.push("Skipped a band that wasn't readable".into());
            continue;
        };
        b.id = b.id.trim().to_lowercase().replace(' ', "_");
        if b.id.is_empty() || !seen_bands.insert(b.id.clone()) {
            continue;
        }
        if let Some(lo) = b.lo_hz.take() {
            b.ranges.push(BandRange { lo_hz: lo, hi_hz: b.hi_hz.take().unwrap_or(lo * 4.0), weight: 1.0 });
        }
        b.ranges.retain_mut(|r| {
            r.lo_hz = r.lo_hz.clamp(20.0, 20000.0);
            r.hi_hz = r.hi_hz.clamp(20.0, 20000.0);
            r.weight = r.weight.clamp(0.0, 2.0);
            r.hi_hz > r.lo_hz
        });
        let existing = show.audio.bands.iter().find(|x| x.id == b.id);
        if b.ranges.is_empty() {
            match existing {
                Some(e) => b.ranges = e.ranges.clone(),
                None => {
                    warn.push(format!("Skipped band \"{}\": no frequency range", b.name));
                    continue;
                }
            }
        }
        if b.detect != "level" {
            b.detect = "transient".into();
        }
        b.sensitivity = b.sensitivity.clamp(0.0, 1.0);
        b.level_threshold = b.level_threshold.clamp(0.0, 1.0);
        b.gain_db = b.gain_db.clamp(-24.0, 24.0);
        b.release = b.release.clamp(0.0, 0.98);
        b.min_gap_ms = b.min_gap_ms.clamp(20, 2000);
        if !hex_ok(&b.color) {
            b.color = existing.map(|e| e.color.clone()).unwrap_or_else(|| "#7c5cff".into());
        }
        if b.name.trim().is_empty() {
            b.name = existing.map(|e| e.name.clone()).unwrap_or_else(|| b.id.clone());
        }
        plan.bands.push(b);
    }
    let band_ids: HashSet<String> = show.audio.bands.iter().map(|b| b.id.clone()).chain(plan.bands.iter().map(|b| b.id.clone())).collect();
    let source_ok = |s: &str| {
        matches!(s, "beat" | "bar" | "level" | "energy" | "phase")
            || band_ids.contains(s)
            || s.strip_suffix(".level").map(|b| band_ids.contains(b)).unwrap_or(false)
    };

    // ---- routes
    let fixture_ids: HashSet<&str> = show.fixtures.iter().map(|f| f.id.as_str()).collect();
    let group_ids: HashSet<&str> = show.groups.iter().map(|g| g.id.as_str()).collect();
    let room_ids: HashSet<&str> = show.rooms.iter().map(|r| r.id.as_str()).collect();
    let look_sets: HashSet<&str> = show.profiles.iter().flat_map(|p| p.look_sets.iter().map(|s| s.id.as_str())).collect();
    let profiles: HashMap<&str, &Profile> = show.profiles.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut route_map: HashMap<String, String> = HashMap::new();
    for v in list("routes") {
        let label = v["name"].as_str().unwrap_or("A route").to_string();
        let mut r = match serde_json::from_value::<Route>(v) {
            Ok(r) => r,
            Err(e) => {
                warn.push(format!("Skipped route \"{label}\": {e}"));
                continue;
            }
        };
        let name = if r.name.trim().is_empty() { "AI route".to_string() } else { r.name.clone() };
        r.name = name.clone();
        if r.group.trim().is_empty() {
            r.group = "AI".into();
        }
        if !source_ok(&r.source) {
            warn.push(format!("Skipped route \"{name}\": unknown source \"{}\"", r.source));
            continue;
        }
        let before = r.fixtures.len() + r.groups.len() + r.rooms.len();
        r.fixtures.retain(|f| fixture_ids.contains(f.as_str()));
        r.groups.retain(|g| group_ids.contains(g.as_str()));
        r.rooms.retain(|x| room_ids.contains(x.as_str()));
        if r.fixtures.len() + r.groups.len() + r.rooms.len() < before {
            warn.push(format!("{name}: dropped target ids that don't exist"));
            if r.fixtures.is_empty() && r.groups.is_empty() && r.rooms.is_empty() {
                warn.push(format!("Skipped route \"{name}\": none of its targets exist"));
                continue;
            }
        }
        if !r.focus.is_empty() && show.focus_point(&r.focus).is_none() {
            r.focus.clear();
        }
        r.every = r.every.max(1);
        if !(r.probability > 0.0) {
            r.probability = 1.0;
        }
        r.probability = r.probability.min(1.0);
        match &mut r.action {
            RouteAction::Modulate { smoothing, .. } => *smoothing = smoothing.clamp(0.0, 0.98),
            RouteAction::NodeHop { palette, .. } if !palette.is_empty() => fix_palette(palette, &name, &mut warn),
            RouteAction::Strobe { rate, hz, duty, threshold, .. } => {
                if !["free", "1", "2", "4", "8"].contains(&rate.as_str()) {
                    *rate = "2".into();
                }
                *hz = hz.clamp(0.5, 25.0);
                *duty = duty.clamp(0.05, 0.9);
                *threshold = threshold.clamp(0.0, 1.0);
            }
            RouteAction::Palette { palette, mode, .. } => {
                fix_palette(palette, &name, &mut warn);
                if !["all", "spread", "rings", "around", "random"].contains(&mode.as_str()) {
                    *mode = "all".into();
                }
            }
            RouteAction::Twinkle { palette, .. } => fix_palette(palette, &name, &mut warn),
            RouteAction::Wave { palette, axis, beats, width, .. } => {
                fix_palette(palette, &name, &mut warn);
                if !["x", "y", "radial", "angle", "height"].contains(&axis.as_str()) {
                    *axis = "x".into();
                }
                *beats = beats.clamp(0.25, 64.0);
                *width = width.clamp(0.05, 1.0);
            }
            RouteAction::Look { set, mode, .. } => {
                if !set.is_empty() && !look_sets.contains(set.as_str()) {
                    warn.push(format!("{name}: unknown look set, using any look"));
                    set.clear();
                }
                if mode != "cycle" {
                    *mode = "random".into();
                }
            }
            _ => {}
        }
        // Does the route reach anything?
        let fixtures = resolve_fixtures(show, &r.rooms, &r.groups, &r.fixtures);
        let reaches = match &r.action {
            RouteAction::Strobe { .. } | RouteAction::Palette { .. } | RouteAction::Twinkle { .. } | RouteAction::Wave { .. } => {
                r.channel.clear();
                fixtures.iter().any(|f| look_chans(&profiles, f).is_some())
            }
            RouteAction::Look { .. } => fixtures.iter().any(|f| profiles.get(f.profile_id.as_str()).map(|p| !p.looks.is_empty()).unwrap_or(false)),
            _ => fixtures.iter().any(|f| !channel_keys(&profiles, f, &r.channel).is_empty()),
        };
        if !reaches {
            warn.push(format!("Skipped route \"{name}\": no targeted fixture has what it drives{}", if r.channel.is_empty() { String::new() } else { format!(" ({})", r.channel) }));
            continue;
        }
        let new = id("r");
        route_map.insert(r.id.clone(), new.clone());
        r.id = new;
        plan.routes.push(r);
    }
    let existing_routes: HashSet<&str> = show.routes.iter().map(|r| r.id.as_str()).collect();
    let map_route = |rid: &str| route_map.get(rid).cloned().or_else(|| existing_routes.contains(rid).then(|| rid.to_string()));
    let route_groups: HashSet<String> = show.routes.iter().map(|r| r.group.clone()).chain(plan.routes.iter().map(|r| r.group.clone())).collect();

    // ---- presets
    let mut preset_map: HashMap<String, String> = HashMap::new();
    for v in list("presets") {
        let Ok(mut p) = serde_json::from_value::<Preset>(v) else {
            warn.push("Skipped a preset that wasn't readable".into());
            continue;
        };
        if p.name.trim().is_empty() {
            p.name = "AI scene".into();
        }
        p.values.retain(|pv| {
            show.fixtures.iter().find(|f| f.id == pv.fixture).map(|f| !channel_keys(&profiles, f, &pv.channel).is_empty()).unwrap_or(false)
        });
        let mut routes = BTreeMap::new();
        for (rid, on) in std::mem::take(&mut p.routes) {
            if let Some(n) = map_route(&rid) {
                routes.insert(n, on);
            }
        }
        // A scene switches off the generated routes it doesn't mention.
        if !routes.is_empty() {
            for r in &plan.routes {
                routes.entry(r.id.clone()).or_insert(false);
            }
        }
        p.routes = routes;
        if p.values.is_empty() && p.routes.is_empty() {
            warn.push(format!("Skipped preset \"{}\": it doesn't change anything that exists", p.name));
            continue;
        }
        if !hex_ok(&p.color) {
            p.color = "#7c5cff".into();
        }
        let new = id("p");
        preset_map.insert(p.id.clone(), new.clone());
        p.id = new;
        plan.presets.push(p);
    }
    let existing_presets: HashSet<&str> = show.presets.iter().map(|p| p.id.as_str()).collect();

    // ---- controls
    let mut triggers: HashSet<String> = HashSet::new();
    let trig_key = |t: &ControlTrigger| {
        if t.source == "key" { format!("key:{}", t.key) } else { format!("midi:{}:{}:{}", t.kind, t.channel, t.number) }
    };
    for v in list("controls") {
        let label = v["name"].as_str().unwrap_or("A control").to_string();
        let mut c = match serde_json::from_value::<ControlMapping>(v) {
            Ok(c) => c,
            Err(e) => {
                warn.push(format!("Skipped control \"{label}\": {e}"));
                continue;
            }
        };
        if c.name.trim().is_empty() {
            c.name = label.clone();
        }
        let t = &mut c.trigger;
        if t.source != "key" {
            t.source = "midi".into();
            if !["note", "cc", "program"].contains(&t.kind.as_str()) {
                t.kind = "note".into();
            }
            t.channel = t.channel.min(16);
            t.number = t.number.min(127);
        } else {
            t.key = t.key.to_lowercase();
            if t.key.is_empty() {
                warn.push(format!("Skipped control \"{}\": no key", c.name));
                continue;
            }
            if t.key == "b" {
                warn.push(format!("{}: \"b\" is the blackout key; skipped", c.name));
                continue;
            }
        }
        if !["toggle", "momentary", "on", "off", "fader"].contains(&t.mode.as_str()) {
            t.mode = "toggle".into();
        }
        let ok = match &mut c.action {
            ControlAction::Routes { routes } => {
                *routes = routes.iter().filter_map(|r| map_route(r)).collect();
                !routes.is_empty()
            }
            ControlAction::RouteGroup { group } => route_groups.contains(group.as_str()),
            ControlAction::Bands { bands } => {
                bands.retain(|b| band_ids.contains(b));
                !bands.is_empty()
            }
            ControlAction::Preset { preset } => match preset_map.get(preset.as_str()) {
                Some(n) => {
                    *preset = n.clone();
                    true
                }
                None => existing_presets.contains(preset.as_str()),
            },
            ControlAction::ChannelValue { target, channel, .. } => {
                target.fixtures.retain(|f| fixture_ids.contains(f.as_str()));
                target.groups.retain(|g| group_ids.contains(g.as_str()));
                target.rooms.retain(|x| room_ids.contains(x.as_str()));
                resolve_fixtures(show, &target.rooms, &target.groups, &target.fixtures)
                    .iter()
                    .any(|f| !channel_keys(&profiles, f, channel).is_empty())
            }
            ControlAction::BandSensitivity { band } => band_ids.contains(band.as_str()),
            ControlAction::Blackout {} | ControlAction::Master {} | ControlAction::ReleaseManual {} => true,
        };
        if !ok {
            warn.push(format!("Skipped control \"{}\": what it controls doesn't exist", c.name));
            continue;
        }
        let k = trig_key(&c.trigger);
        if !triggers.insert(k) {
            warn.push(format!("Skipped control \"{}\": its trigger is already used in this plan", c.name));
            continue;
        }
        if show.controls.iter().any(|x| !x.id.starts_with("ai-") && trig_key(&x.trigger) == trig_key(&c.trigger)) {
            warn.push(format!("{}: its trigger is also used by one of your existing controls", c.name));
        }
        c.enabled = true;
        c.id = id("c");
        plan.controls.push(c);
    }
    (plan, warn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig() -> Show {
        let par = Profile {
            id: "par".into(),
            kind: "par".into(),
            channels: ["dimmer:intensity", "red:color", "green:color", "blue:color"]
                .iter()
                .map(|s| {
                    let (k, g) = s.split_once(':').unwrap();
                    ChannelDef { key: k.into(), name: k.into(), group: g.into(), ..Default::default() }
                })
                .collect(),
            modes: vec![Mode { name: "4ch".into(), channels: vec!["dimmer".into(), "red".into(), "green".into(), "blue".into()], heads: vec![] }],
            ..Default::default()
        };
        let fx = |id: &str| Fixture { id: id.into(), name: id.into(), profile_id: "par".into(), mode: "4ch".into(), enabled: true, ..Default::default() };
        let mut s = Show { profiles: vec![par], fixtures: vec![fx("a"), fx("b")], ..Default::default() };
        s.groups.push(FixtureGroup { id: "g1".into(), name: "Pars".into(), fixtures: vec!["a".into(), "b".into()], ..Default::default() });
        s.migrate();
        s
    }

    #[test]
    fn sanitizes_and_links_a_plan() {
        let show = rig();
        let raw = json!({
            "title": "Techno",
            "palettes": [{"id": "pal1", "name": "Acid", "colors": ["#00ff00", "nope", "#ff00ff"]}],
            "bands": [{"id": "kick", "sensitivity": 3.0}, {"id": "clap", "name": "Clap", "ranges": [{"lo_hz": 1000, "hi_hz": 4000, "weight": 1}]}],
            "routes": [
                {"id": "r1", "name": "Kick pump", "group": "Drop", "enabled": true, "source": "kick", "groups": ["g1"], "channel": "@intensity",
                 "action": {"type": "flash", "on": 255, "off": 40, "decay_ms": 180}},
                {"id": "r2", "name": "Colour", "group": "Drop", "source": "bar", "channel": "ignored",
                 "action": {"type": "palette", "palette": "pal1", "mode": "spread", "fade_ms": 200, "brightness": 0}},
                {"id": "r3", "name": "Bad source", "source": "nothing", "channel": "@intensity", "action": {"type": "flash", "on": 255, "off": 0, "decay_ms": 100}},
                {"id": "r4", "name": "Pan", "source": "kick", "channel": "@pan", "action": {"type": "flash", "on": 255, "off": 0, "decay_ms": 100}},
                {"id": "r5", "name": "Weird", "source": "kick", "action": {"type": "teleport"}},
                {"id": "r6", "name": "Clap flash", "source": "clap", "fixtures": ["a", "ghost"], "channel": "dimmer", "action": {"type": "flash", "on": 255, "off": 0, "decay_ms": 100}}
            ],
            "presets": [{"id": "p1", "name": "Drop", "routes": {"r1": true}}, {"id": "p2", "name": "Empty", "routes": {"zzz": true}}],
            "controls": [
                {"id": "c1", "name": "Drop pad", "trigger": {"source": "midi", "kind": "note", "channel": 1, "number": 36, "mode": "on"}, "action": {"type": "preset", "preset": "p1"}},
                {"id": "c2", "name": "Same pad", "trigger": {"source": "midi", "kind": "note", "channel": 1, "number": 36, "mode": "on"}, "action": {"type": "blackout"}},
                {"id": "c3", "name": "Drop group", "trigger": {"source": "key", "key": "D", "mode": "toggle"}, "action": {"type": "route_group", "group": "Drop"}},
                {"id": "c4", "name": "Blackout key", "trigger": {"source": "key", "key": "b"}, "action": {"type": "blackout"}}
            ]
        });
        let (plan, warn) = sanitize(&show, &raw);
        assert_eq!(plan.palettes[0].colors, vec!["#00ff00", "#ff00ff"]);
        assert_eq!(plan.bands.len(), 2);
        assert_eq!(plan.bands[0].sensitivity, 1.0);
        assert_eq!(plan.bands[0].ranges, show.audio.bands[0].ranges, "retuned band keeps its ranges");
        let names: Vec<&str> = plan.routes.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Kick pump", "Colour", "Clap flash"]);
        assert!(plan.routes.iter().all(|r| r.id.starts_with("ai-r-") && r.probability == 1.0 && r.every == 1));
        assert_eq!(plan.routes[2].fixtures, vec!["a"]);
        let RouteAction::Palette { palette, .. } = &plan.routes[1].action else { panic!() };
        assert_eq!(palette, &plan.palettes[0].id);
        assert_eq!(plan.routes[1].channel, "");
        // the scene turns r1 on and the other generated routes off
        assert_eq!(plan.presets.len(), 1);
        let p = &plan.presets[0];
        assert_eq!(p.routes[&plan.routes[0].id], true);
        assert_eq!(p.routes[&plan.routes[1].id], false);
        // controls: duplicate trigger and blackout key skipped; preset linked; key lower-cased
        assert_eq!(plan.controls.len(), 2);
        let ControlAction::Preset { preset } = &plan.controls[0].action else { panic!() };
        assert_eq!(preset, &p.id);
        assert_eq!(plan.controls[1].trigger.key, "d");
        assert!(warn.iter().any(|w| w.contains("Bad source")));
        assert!(warn.iter().any(|w| w.contains("\"Pan\"")));
        assert!(warn.iter().any(|w| w.contains("Weird")));
    }

    #[test]
    fn context_lists_the_rig() {
        let c = rig_context(&rig());
        assert_eq!(c["fixtures"].as_array().unwrap().len(), 2);
        assert_eq!(c["fixture_types"][0]["channels"][0]["type"], "intensity");
        assert_eq!(c["bands"][0]["id"], "kick");
    }

    /// The rig plus a laser with looks, a fog machine, a moving head with only pan, and a room with a focus point.
    fn big_rig() -> Show {
        let mut s = rig();
        let ch = |k: &str, g: &str| ChannelDef { key: k.into(), name: k.into(), group: g.into(), ..Default::default() };
        let mut laser = Profile {
            id: "laser".into(),
            kind: "laser".into(),
            channels: vec![ch("p1_pattern", "pattern"), ch("p1_rotation", "effect")],
            modes: vec![Mode { name: "m".into(), channels: vec!["p1_pattern".into(), "p1_rotation".into()], heads: vec![Head { name: "P1".into(), channels: vec!["p1_pattern".into(), "p1_rotation".into()] }] }],
            look_sets: vec![LookSet { id: "set-peak".into(), name: "Peak".into(), color: String::new() }],
            ..Default::default()
        };
        laser.looks.push(Look { id: "l1".into(), name: "Tunnel".into(), head: "P1".into(), tags: vec!["peak".into()], rating: 5, ..Default::default() });
        let fog = Profile { id: "fog".into(), kind: "fog".into(), channels: vec![ch("fog", "fog")], modes: vec![Mode { name: "m".into(), channels: vec!["fog".into()], heads: vec![] }], ..Default::default() };
        let mover = Profile { id: "mover".into(), kind: "moving_head".into(), channels: vec![ch("pan", "pan")], modes: vec![Mode { name: "m".into(), channels: vec!["pan".into()], heads: vec![] }], ..Default::default() };
        s.profiles.extend([laser, fog, mover]);
        let fx = |id: &str, p: &str| Fixture { id: id.into(), name: id.into(), profile_id: p.into(), mode: "m".into(), enabled: true, room_id: Some("room".into()), ..Default::default() };
        s.fixtures.extend([fx("las", "laser"), fx("fogger", "fog"), fx("mh", "mover")]);
        s.groups.push(FixtureGroup { id: "g-laser".into(), name: "Laser".into(), fixtures: vec!["las".into()], ..Default::default() });
        s.groups.push(FixtureGroup { id: "g-mover".into(), name: "Mover".into(), fixtures: vec!["mh".into()], ..Default::default() });
        s.rooms.push(Room { id: "room".into(), name: "Main".into(), focus: vec![FocusPoint { id: "floor".into(), ..Default::default() }], ..Default::default() });
        s
    }

    fn route(id: &str, source: &str, target: Value, channel: &str, action: Value) -> Value {
        let mut r = json!({"id": id, "name": id, "group": "Scene", "source": source, "channel": channel, "action": action});
        r.as_object_mut().unwrap().extend(target.as_object().unwrap().clone());
        r
    }

    fn route_type(a: &RouteAction) -> &'static str {
        // Exhaustive on purpose: a new action fails to compile here until the prompt test covers it.
        match a {
            RouteAction::Modulate { .. } => "modulate",
            RouteAction::Flash { .. } => "flash",
            RouteAction::Random { .. } => "random",
            RouteAction::NodeHop { .. } => "node_hop",
            RouteAction::Strobe { .. } => "strobe",
            RouteAction::Palette { .. } => "palette",
            RouteAction::Twinkle { .. } => "twinkle",
            RouteAction::Wave { .. } => "wave",
            RouteAction::Cycle { .. } => "cycle",
            RouteAction::Look { .. } => "look",
        }
    }

    fn control_type(a: &ControlAction) -> &'static str {
        match a {
            ControlAction::Routes { .. } => "routes",
            ControlAction::RouteGroup { .. } => "route_group",
            ControlAction::Bands { .. } => "bands",
            ControlAction::Blackout {} => "blackout",
            ControlAction::Master {} => "master",
            ControlAction::Preset { .. } => "preset",
            ControlAction::ReleaseManual {} => "release_manual",
            ControlAction::ChannelValue { .. } => "channel_value",
            ControlAction::BandSensitivity { .. } => "band_sensitivity",
        }
    }

    fn all_route_actions() -> Vec<RouteAction> {
        vec![
            RouteAction::Modulate { min: 0, max: 255, smoothing: 0.5 },
            RouteAction::Flash { on: 255, off: 0, decay_ms: 100 },
            RouteAction::Random { tags: vec![], hold_ms: 0 },
            RouteAction::NodeHop { on: 255, off: 0, decay_ms: 100, palette: String::new() },
            RouteAction::Strobe { rate: "2".into(), hz: 10.0, duty: 0.3, burst_ms: 200, threshold: 0.5 },
            RouteAction::Palette { palette: String::new(), mode: "all".into(), fade_ms: 0, brightness: 0 },
            RouteAction::Twinkle { palette: String::new(), amount: 1.0, decay_ms: 100, base: 0 },
            RouteAction::Wave { palette: String::new(), axis: "x".into(), beats: 4.0, width: 0.3, base: 0 },
            RouteAction::Cycle { values: vec![] },
            RouteAction::Look { set: String::new(), include_defaults: true, tags: vec![], mode: "random".into(), hold_ms: 0, heads: vec![] },
        ]
    }

    fn all_control_actions() -> Vec<ControlAction> {
        vec![
            ControlAction::Routes { routes: vec![] },
            ControlAction::RouteGroup { group: String::new() },
            ControlAction::Bands { bands: vec![] },
            ControlAction::Blackout {},
            ControlAction::Master {},
            ControlAction::Preset { preset: String::new() },
            ControlAction::ReleaseManual {},
            ControlAction::ChannelValue { target: Target::default(), channel: String::new(), min: 0, max: 0 },
            ControlAction::BandSensitivity { band: String::new() },
        ]
    }

    /// Every field name of a serialized struct appears in the prompt as `name` or `"name"`.
    fn assert_documented(what: &str, v: &Value, skip: &[&str]) {
        for k in v.as_object().unwrap().keys().filter(|k| !skip.contains(&k.as_str())) {
            let mentioned = SYSTEM.contains(&format!("{k},")) || SYSTEM.contains(&format!("{k} ")) || SYSTEM.contains(&format!("{k}:"))
                || SYSTEM.contains(&format!("{k}}}")) || SYSTEM.contains(&format!("\"{k}\""));
            assert!(mentioned, "{what}: field `{k}` is not described in the designer prompt");
        }
    }

    #[test]
    fn prompt_documents_every_action_and_field() {
        let routes = all_route_actions();
        assert_eq!(routes.len(), 10);
        for a in &routes {
            let t = route_type(a);
            assert!(SYSTEM.contains(&format!("\n  {t} {{")), "route action `{t}` is missing from the prompt");
            let v = serde_json::to_value(a).unwrap();
            assert_eq!(v["type"], t);
            assert_documented(t, &v, &["type"]);
        }
        for a in &all_control_actions() {
            let t = control_type(a);
            assert!(SYSTEM.contains(&format!("{t} {{")), "control action `{t}` is missing from the prompt");
            let v = serde_json::to_value(a).unwrap();
            assert_documented(t, &v, &["type"]);
        }
        assert_documented("route", &serde_json::to_value(Route::default()).unwrap(), &["action"]);
        assert_documented("preset", &serde_json::to_value(Preset::default()).unwrap(), &[]);
        assert_documented("control", &serde_json::to_value(ControlMapping::default()).unwrap(), &["action"]);
        assert_documented("trigger", &serde_json::to_value(ControlTrigger::default()).unwrap(), &[]);
        assert_documented("band", &serde_json::to_value(BandDef::default()).unwrap(), &[]);
        assert_documented("band range", &serde_json::to_value(BandRange::default()).unwrap(), &[]);
        // Channel types in the prompt match the ones the UI offers.
        for g in ["intensity", "color", "pan", "tilt", "pattern", "beam", "effect", "speed", "strobe", "mode", "fog", "other"] {
            assert!(SYSTEM.contains(g), "channel type {g} missing");
        }
    }

    #[test]
    fn every_action_type_can_be_planned() {
        let show = big_rig();
        let pars = json!({"groups": ["g1"]});
        let raw = json!({
            "routes": [
                route("modulate", "energy", pars.clone(), "@intensity", json!({"type": "modulate", "min": 10, "max": 255, "smoothing": 0.5})),
                route("flash", "kick", pars.clone(), "dimmer", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 150})),
                route("random", "snare", json!({"groups": ["g-laser"]}), "p1_pattern", json!({"type": "random", "tags": ["peak"], "hold_ms": 500})),
                route("node_hop", "kick", pars.clone(), "@intensity", json!({"type": "node_hop", "on": 255, "off": 0, "decay_ms": 200, "palette": "pal-club"})),
                route("strobe", "snare", pars.clone(), "", json!({"type": "strobe", "rate": "4", "hz": 12, "duty": 0.3, "burst_ms": 200, "threshold": 0.6})),
                route("palette", "bar", pars.clone(), "", json!({"type": "palette", "palette": "pal-club", "mode": "rings", "fade_ms": 100, "brightness": 0})),
                route("twinkle", "hat", pars.clone(), "", json!({"type": "twinkle", "palette": "pal-ice", "amount": 2, "decay_ms": 200, "base": 10})),
                route("wave", "beat", json!({"groups": ["g1"], "focus": "floor"}), "", json!({"type": "wave", "palette": "pal-sunset", "axis": "radial", "beats": 8, "width": 0.3, "base": 0})),
                route("cycle", "beat", json!({"fixtures": ["las"]}), "p1_rotation", json!({"type": "cycle", "values": [0, 64, 128]})),
                route("look", "kick", json!({"groups": ["g-laser"]}), "", json!({"type": "look", "set": "set-peak", "include_defaults": false, "tags": ["peak"], "mode": "cycle", "hold_ms": 800, "heads": ["P1"]})),
                route("fog", "bar", json!({"fixtures": ["fogger"]}), "@fog", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 2000})),
            ]
        });
        let (plan, warn) = sanitize(&show, &raw);
        let names: Vec<&str> = plan.routes.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["modulate", "flash", "random", "node_hop", "strobe", "palette", "twinkle", "wave", "cycle", "look", "fog"], "warnings: {warn:?}");
        assert!(warn.is_empty(), "{warn:?}");
        let wave = plan.routes.iter().find(|r| r.name == "wave").unwrap();
        assert_eq!(wave.focus, "floor");
        let look = plan.routes.iter().find(|r| r.name == "look").unwrap();
        let RouteAction::Look { set, mode, heads, .. } = &look.action else { panic!() };
        assert_eq!((set.as_str(), mode.as_str(), heads.as_slice()), ("set-peak", "cycle", &["P1".to_string()][..]));
    }

    #[test]
    fn skips_routes_that_cannot_reach_anything() {
        let show = big_rig();
        let raw = json!({"routes": [
            // the moving head has no brightness or colour, so no effects
            route("wave on mover", "beat", json!({"groups": ["g-mover"]}), "", json!({"type": "wave", "palette": "pal-club", "axis": "x", "beats": 4, "width": 0.3, "base": 0})),
            // pars have no saved looks
            route("look on pars", "kick", json!({"groups": ["g1"]}), "", json!({"type": "look", "set": "", "tags": [], "mode": "random", "hold_ms": 0, "heads": []})),
            // channel that doesn't exist on the target
            route("tilt", "kick", json!({"groups": ["g-mover"]}), "@tilt", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 100})),
            // empty channel on a channel action
            route("no channel", "kick", json!({"groups": ["g1"]}), "", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 100})),
            // every target id is unknown
            route("ghosts", "kick", json!({"groups": ["nope"], "fixtures": ["nobody"]}), "@intensity", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 100})),
            // unknown look set falls back to any set; focus that doesn't exist is cleared; bad enums fixed
            route("loose", "kick.level", json!({"groups": ["g-laser"], "focus": "nowhere"}), "", json!({"type": "look", "set": "missing", "tags": [], "mode": "shuffle", "hold_ms": 0, "heads": []})),
            route("bad enums", "beat", json!({"groups": ["g1"]}), "", json!({"type": "wave", "palette": "Sunset", "axis": "diagonal", "beats": 1000, "width": 9, "base": 0})),
        ]});
        let (plan, warn) = sanitize(&show, &raw);
        let names: Vec<&str> = plan.routes.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["loose", "bad enums"], "{warn:?}");
        let RouteAction::Look { set, mode, .. } = &plan.routes[0].action else { panic!() };
        assert_eq!((set.as_str(), mode.as_str()), ("", "random"));
        assert_eq!(plan.routes[0].focus, "");
        let RouteAction::Wave { palette, axis, beats, width, .. } = &plan.routes[1].action else { panic!() };
        assert_eq!(palette, "pal-sunset", "palette matched by name");
        assert_eq!((axis.as_str(), *beats, *width), ("x", 64.0, 1.0));
        for n in ["wave on mover", "look on pars", "tilt", "no channel", "ghosts"] {
            assert!(warn.iter().any(|w| w.contains(n)), "no warning for {n}: {warn:?}");
        }
    }

    #[test]
    fn bands_are_cleaned_up() {
        let show = rig();
        let raw = json!({"sensitivity": 7, "bands": [
            {"id": "Sub Bass", "name": "", "lo_hz": 25, "hi_hz": 60, "detect": "level"},
            {"id": "weird", "ranges": [{"lo_hz": 5000, "hi_hz": 100, "weight": 1}]},
            {"id": "air", "ranges": [{"lo_hz": 1, "hi_hz": 99999, "weight": 5}], "detect": "magic", "release": 3, "min_gap_ms": 1, "color": "red"},
            {"id": "air", "name": "Duplicate"},
        ]});
        let (plan, warn) = sanitize(&show, &raw);
        assert_eq!(plan.sensitivity, Some(1.0));
        let ids: Vec<&str> = plan.bands.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["sub_bass", "air"]);
        let sub = &plan.bands[0];
        assert_eq!((sub.name.as_str(), sub.detect.as_str()), ("sub_bass", "level"));
        assert_eq!(sub.ranges, vec![BandRange { lo_hz: 25.0, hi_hz: 60.0, weight: 1.0 }], "old single-range fields are read");
        let air = &plan.bands[1];
        assert_eq!(air.ranges, vec![BandRange { lo_hz: 20.0, hi_hz: 20000.0, weight: 2.0 }]);
        assert_eq!((air.detect.as_str(), air.release, air.min_gap_ms, air.color.as_str()), ("transient", 0.98, 20, "#7c5cff"));
        assert!(warn.iter().any(|w| w.contains("no frequency range")));
    }

    #[test]
    fn presets_and_controls_point_at_real_things() {
        let show = big_rig();
        let raw = json!({
            "routes": [route("r1", "kick", json!({"groups": ["g1"]}), "@intensity", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 100}))],
            "presets": [
                {"id": "p1", "name": "Static", "values": [{"fixture": "a", "channel": "red", "value": 255}, {"fixture": "a", "channel": "pan", "value": 1}, {"fixture": "ghost", "channel": "red", "value": 1}]},
                {"id": "p2", "name": "", "routes": {"r1": true, "not-a-route": true}, "color": "blue"},
            ],
            "controls": [
                {"id": "c1", "name": "Pars up", "trigger": {"source": "midi", "kind": "cc", "number": 300, "channel": 40, "mode": "fader"},
                 "action": {"type": "channel_value", "target": {"groups": ["g1", "zzz"]}, "channel": "@intensity", "min": 0, "max": 255}},
                {"id": "c2", "name": "Pan pars", "trigger": {"source": "midi", "kind": "cc", "number": 2, "mode": "fader"},
                 "action": {"type": "channel_value", "target": {"groups": ["g1"]}, "channel": "@pan", "min": 0, "max": 255}},
                {"id": "c3", "name": "Mute", "trigger": {"source": "midi", "kind": "note", "number": 40, "mode": "toggle"}, "action": {"type": "bands", "bands": ["kick", "nope"]}},
                {"id": "c4", "name": "Scene", "trigger": {"source": "midi", "kind": "note", "number": 41, "mode": "on"}, "action": {"type": "route_group", "group": "Scene"}},
                {"id": "c5", "name": "No scene", "trigger": {"source": "midi", "kind": "note", "number": 42, "mode": "on"}, "action": {"type": "route_group", "group": "Missing"}},
                {"id": "c6", "name": "Routes", "trigger": {"source": "midi", "kind": "note", "number": 43, "mode": "sometimes"}, "action": {"type": "routes", "routes": ["r1", "zz"]}},
                {"id": "c7", "name": "Gone", "trigger": {"source": "midi", "kind": "note", "number": 44}, "action": {"type": "preset", "preset": "nope"}},
                {"id": "c8", "name": "Static", "trigger": {"source": "key", "key": "1", "kind": "note"}, "action": {"type": "preset", "preset": "p1"}},
                {"id": "c9", "name": "Sens", "trigger": {"source": "midi", "kind": "aftertouch", "number": 3, "mode": "fader"}, "action": {"type": "band_sensitivity", "band": "snare"}},
            ]
        });
        let (plan, warn) = sanitize(&show, &raw);
        let p1 = &plan.presets[0];
        assert_eq!(p1.values.len(), 1, "only a's red survives");
        let p2 = &plan.presets[1];
        assert_eq!((p2.name.as_str(), p2.color.as_str()), ("AI scene", "#7c5cff"));
        assert_eq!(p2.routes.keys().collect::<Vec<_>>(), vec![&plan.routes[0].id]);
        let names: Vec<&str> = plan.controls.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Pars up", "Mute", "Scene", "Routes", "Static", "Sens"], "{warn:?}");
        let c1 = &plan.controls[0];
        assert_eq!((c1.trigger.number, c1.trigger.channel), (127, 16));
        let ControlAction::ChannelValue { target, .. } = &c1.action else { panic!() };
        assert_eq!(target.groups, vec!["g1"]);
        let ControlAction::Bands { bands } = &plan.controls[1].action else { panic!() };
        assert_eq!(bands, &vec!["kick".to_string()]);
        let c6 = &plan.controls[3];
        assert_eq!(c6.trigger.mode, "toggle");
        let ControlAction::Routes { routes } = &c6.action else { panic!() };
        assert_eq!(routes, &vec![plan.routes[0].id.clone()]);
        let ControlAction::Preset { preset } = &plan.controls[4].action else { panic!() };
        assert_eq!(preset, &p1.id);
        assert_eq!(plan.controls[5].trigger.kind, "note");
        assert!(plan.controls.iter().all(|c| c.enabled && c.id.starts_with("ai-c-")));
    }

    #[test]
    fn warns_about_triggers_your_controls_already_use() {
        let mut show = rig();
        show.controls.push(ControlMapping {
            id: "mine".into(),
            name: "Mine".into(),
            enabled: true,
            trigger: ControlTrigger { number: 36, ..Default::default() },
            action: ControlAction::Blackout {},
        });
        let raw = json!({"controls": [{"name": "Clash", "trigger": {"source": "midi", "kind": "note", "channel": 0, "number": 36}, "action": {"type": "master"}}]});
        let (plan, warn) = sanitize(&show, &raw);
        assert_eq!(plan.controls.len(), 1, "kept, but flagged");
        assert!(warn.iter().any(|w| w.contains("Clash") && w.contains("existing")));
    }

    #[test]
    fn scope_drops_parts_before_checking() {
        let show = rig();
        let mut raw = json!({
            "sensitivity": 0.4, "bands": [{"id": "kick", "sensitivity": 0.2}],
            "palettes": [{"id": "x", "name": "X", "colors": ["#ffffff"]}],
            "routes": [route("r1", "kick", json!({"groups": ["g1"]}), "@intensity", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 100}))],
            "presets": [{"id": "p1", "name": "Scene", "routes": {"r1": true}}],
            "controls": [{"name": "Pad", "trigger": {"number": 1}, "action": {"type": "preset", "preset": "p1"}}],
        });
        restrict(&mut raw, &Scope { routes: false, audio: false, palettes: false, ..Default::default() });
        let (plan, warn) = sanitize(&show, &raw);
        assert!(plan.routes.is_empty() && plan.bands.is_empty() && plan.palettes.is_empty() && plan.sensitivity.is_none());
        // the scene only switched routes that weren't asked for, so it and its pad go too
        assert!(plan.presets.is_empty() && plan.controls.is_empty());
        assert!(warn.iter().any(|w| w.contains("Scene")));
    }

    #[test]
    fn refining_a_checked_plan_keeps_it_intact() {
        // A checked plan is sent back as "previous" and the model may return it unchanged.
        let show = big_rig();
        let raw = json!({
            "palettes": [{"id": "pal1", "name": "Acid", "colors": ["#00ff00"]}],
            "routes": [
                route("r1", "kick", json!({"groups": ["g1"]}), "@intensity", json!({"type": "flash", "on": 255, "off": 0, "decay_ms": 100})),
                route("r2", "bar", json!({"groups": ["g1"]}), "", json!({"type": "palette", "palette": "pal1", "mode": "all", "fade_ms": 0, "brightness": 0})),
            ],
            "presets": [{"id": "p1", "name": "Drop", "routes": {"r1": true, "r2": true}}],
            "controls": [{"name": "Drop", "trigger": {"number": 1, "mode": "on"}, "action": {"type": "preset", "preset": "p1"}}],
        });
        let (first, _) = sanitize(&show, &raw);
        let (again, warn) = sanitize(&show, &serde_json::to_value(&first).unwrap());
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!((again.routes.len(), again.presets.len(), again.controls.len(), again.palettes.len()), (2, 1, 1, 1));
        let RouteAction::Palette { palette, .. } = &again.routes[1].action else { panic!() };
        assert_eq!(palette, &again.palettes[0].id);
        assert!(again.presets[0].routes.keys().all(|k| again.routes.iter().any(|r| &r.id == k)));
        let ControlAction::Preset { preset } = &again.controls[0].action else { panic!() };
        assert_eq!(preset, &again.presets[0].id);
        assert!(again.routes.iter().all(|r| !first.routes.iter().any(|f| f.id == r.id)), "ids are renumbered every time");
    }

    #[test]
    fn user_message_carries_the_request() {
        let show = big_rig();
        let req = DesignRequest {
            prompt: "  Big drops please ".into(),
            genre: Some(GenreHint { name: "Techno".into(), bpm: [125.0, 140.0], style: "dark".into() }),
            scope: Scope { palettes: false, ..Default::default() },
            controller: "APC mini".into(),
            midi_channel: 3,
            keyboard: true,
            previous: Some(json!({"title": "Before"})),
        };
        let u = build_user(&show, &req);
        assert!(u.starts_with("RIG\n{"));
        assert!(u.contains("- palettes: no (empty array)") && u.contains("- routes grouped into scenes: yes"));
        assert!(u.contains("GENRE: Techno (125-140 BPM). Typical lighting: dark"));
        assert!(u.contains("MIDI CONTROLLER: APC mini, channel 3"));
        assert!(u.contains("avoid \"b\""));
        assert!(u.contains("PREVIOUS PLAN") && u.contains("\"title\":\"Before\""));
        assert!(u.ends_with("BRIEF\nBig drops please"));
        // the rig part is valid JSON
        let rig_json = u.lines().nth(1).unwrap();
        let v: Value = serde_json::from_str(rig_json).unwrap();
        assert_eq!(v["fixture_types"].as_array().unwrap().len(), 4);
        let laser = v["fixture_types"].as_array().unwrap().iter().find(|p| p["id"] == "laser").unwrap();
        assert_eq!(laser["look_tags"]["peak"], 1);
        assert_eq!(laser["look_sets"][0]["id"], "set-peak");
        assert_eq!(laser["own_looks"][0]["name"], "Tunnel");
        assert_eq!(v["rooms"][0]["focus"][0]["id"], "floor");

        let plain = build_user(&show, &DesignRequest { scope: Scope { controls: false, ..Default::default() }, ..Default::default() });
        assert!(!plain.contains("MIDI CONTROLLER") && !plain.contains("GENRE") && !plain.contains("PREVIOUS"));
        assert!(plain.ends_with("Design a complete, varied show for this rig."));
    }

    #[test]
    fn plan_json_matches_the_ui_types() {
        // The UI reads these exact keys (src/lib/design.ts).
        let v = serde_json::to_value(DesignResult::default()).unwrap();
        assert!(v["warnings"].is_array());
        for k in ["title", "summary", "sensitivity", "bands", "palettes", "routes", "presets", "controls"] {
            assert!(v["plan"].get(k).is_some(), "plan.{k}");
        }
        // and the request keys it sends
        let req: DesignRequest = serde_json::from_value(json!({
            "prompt": "x", "genre": {"name": "House", "bpm": [118, 128], "style": "warm"},
            "scope": {"routes": true, "presets": false, "audio": true, "palettes": true, "controls": true},
            "controller": "", "midi_channel": 0, "keyboard": true, "previous": null
        })).unwrap();
        assert!(!req.scope.presets && req.genre.unwrap().bpm == [118.0, 128.0]);
    }

    #[test]
    fn stray_numbers_are_rounded_not_rejected() {
        let show = rig();
        let raw = json!({
            "routes": [
                route("fractions", "kick", json!({"groups": ["g1"]}), "@intensity", json!({"type": "flash", "on": 300, "off": -4, "decay_ms": 120.6})),
                route("wave", "beat", json!({"groups": ["g1"]}), "", json!({"type": "wave", "palette": "pal-club", "axis": "x", "beats": 2.5, "width": 0.25, "base": 12.7})),
                route("cycle", "beat", json!({"groups": ["g1"]}), "red", json!({"type": "cycle", "values": [0, 127.5, 999]})),
                route("smooth", "energy", json!({"groups": ["g1"]}), "dimmer", json!({"type": "modulate", "min": 0, "max": 255, "smoothing": 0.5})),
            ],
            "presets": [{"name": "Red", "values": [{"fixture": "a", "channel": "red", "value": 255.4}]}],
            "controls": [{"name": "Pad", "trigger": {"number": 36.0, "channel": 1}, "action": {"type": "blackout"}}],
        });
        let (plan, warn) = sanitize(&show, &raw);
        assert_eq!(plan.routes.len(), 4, "{warn:?}");
        let RouteAction::Flash { on, off, decay_ms } = &plan.routes[0].action else { panic!() };
        assert_eq!((*on, *off, *decay_ms), (255, 0, 121));
        let RouteAction::Wave { beats, width, base, .. } = &plan.routes[1].action else { panic!() };
        assert_eq!((*beats, *width, *base), (2.5, 0.25, 13), "real-number fields are left alone");
        let RouteAction::Cycle { values } = &plan.routes[2].action else { panic!() };
        assert_eq!(values, &vec![0, 128, 255]);
        let RouteAction::Modulate { smoothing, .. } = &plan.routes[3].action else { panic!() };
        assert_eq!(*smoothing, 0.5);
        assert_eq!(plan.presets[0].values[0].value, 255);
        assert_eq!(plan.controls[0].trigger.number, 36);
    }
}
