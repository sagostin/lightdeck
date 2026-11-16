//! LightDeck's own show format. Everything is serde JSON so the UI, the AI
//! interpreter and the engine all speak the same shape.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tag that means "the engine must never output this value/range".
pub const TAG_BLOCKED: &str = "blocked";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Show {
    pub version: u32,
    pub profiles: Vec<Profile>,
    pub fixtures: Vec<Fixture>,
    pub rooms: Vec<Room>,
    pub outputs: Vec<ArtNetOutput>,
    pub routes: Vec<Route>,
    pub audio: AudioSettings,
    pub zigbee: ZigbeeSettings,
    /// Named sets of fixtures, used as route targets.
    pub groups: Vec<FixtureGroup>,
    pub presets: Vec<Preset>,
    pub palettes: Vec<Palette>,
    pub controls: Vec<ControlMapping>,
    pub midi: MidiSettings,
    pub active_room: Option<String>,
}

// ---------- palettes ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Palette {
    pub id: String,
    pub name: String,
    /// "#rrggbb"
    pub colors: Vec<String>,
}

pub fn default_palettes() -> Vec<Palette> {
    let p = |id: &str, name: &str, c: &[&str]| Palette { id: id.into(), name: name.into(), colors: c.iter().map(|s| s.to_string()).collect() };
    vec![
        p("pal-club", "Club", &["#ff0040", "#7c00ff", "#0040ff", "#00e0ff"]),
        p("pal-sunset", "Sunset", &["#ff2a00", "#ff7a00", "#ffc000", "#ff0060"]),
        p("pal-ice", "Ice", &["#ffffff", "#80d8ff", "#0080ff", "#4040ff"]),
        p("pal-rainbow", "Rainbow", &["#ff0000", "#ff8000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff"]),
        p("pal-forest", "Forest", &["#00ff40", "#80ff00", "#00c080", "#ffe000"]),
        p("pal-white", "White", &["#ffffff"]),
    ]
}

pub fn parse_hex(h: &str) -> Option<[u8; 3]> {
    let h = h.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([p(0)?, p(2)?, p(4)?])
}

// ---------- presets and controls ----------

/// A saved look: channel values (applied as manual overrides) and optionally
/// which routes are on.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub color: String,
    pub values: Vec<PresetValue>,
    /// route id -> enabled. Empty = leave routes alone.
    pub routes: std::collections::BTreeMap<String, bool>,
    /// Clear other manual overrides before applying.
    pub exclusive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PresetValue {
    pub fixture: String,
    pub channel: String,
    pub value: u8,
}

/// Which fixtures something applies to. Rooms scope the selection; groups and
/// fixtures pick within it. Everything empty = all fixtures.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Target {
    pub rooms: Vec<String>,
    pub groups: Vec<String>,
    pub fixtures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MidiSettings {
    pub enabled: bool,
    /// Input port names to listen to; empty = all.
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ControlMapping {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger: ControlTrigger,
    pub action: ControlAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ControlTrigger {
    /// "midi" | "key"
    pub source: String,
    /// MIDI: "note" | "cc" | "program"
    pub kind: String,
    /// MIDI channel 1-16; 0 = any.
    pub channel: u8,
    pub number: u8,
    /// Keyboard key (KeyboardEvent.key, lower-cased), e.g. "1", "q", "f1", " ".
    pub key: String,
    /// Buttons: "toggle" | "momentary" (on while held) | "on" | "off".
    pub mode: String,
}

impl Default for ControlTrigger {
    fn default() -> Self {
        Self { source: "midi".into(), kind: "note".into(), channel: 0, number: 36, key: String::new(), mode: "toggle".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlAction {
    /// Turn routes on/off.
    Routes { routes: Vec<String> },
    /// Turn every route in a route group on/off.
    RouteGroup { group: String },
    /// Mute/unmute detection bands (muted bands report no level and no hits).
    Bands { bands: Vec<String> },
    Blackout {},
    /// Fader: master brightness. Button: full / zero.
    Master {},
    Preset { preset: String },
    /// Give every manual override back to the routes.
    ReleaseManual {},
    /// Fader sets the channel between min and max; button sets max (released on toggle-off / key-up).
    ChannelValue { target: Target, channel: String, min: u8, max: u8 },
    /// Fader: a band's hit sensitivity.
    BandSensitivity { band: String },
}

impl Default for ControlAction {
    fn default() -> Self {
        ControlAction::Blackout {}
    }
}

/// A MIDI controller layout: its pads/buttons and faders/knobs in the order they're
/// assigned by auto-mapping. Built-in profiles live in the UI; your own are saved app-wide.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct ControllerProfile {
    pub id: String,
    pub name: String,
    /// MIDI channel it sends on (0 = any).
    pub channel: u8,
    pub pads: Vec<ControllerInput>,
    pub faders: Vec<ControllerInput>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct ControllerInput {
    /// "note" | "cc"
    pub kind: String,
    pub number: u8,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FixtureGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub fixtures: Vec<String>,
}

// ---------- zigbee ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ZigbeeSettings {
    pub enabled: bool,
    /// "usb": LightDeck is the coordinator, talking to a USB dongle directly.
    /// "mqtt": an existing Zigbee2MQTT install, reached through its MQTT broker.
    pub mode: String,
    // --- USB coordinator ---
    /// Serial port of the dongle, e.g. /dev/cu.usbserial-110.
    pub serial_port: String,
    /// zigbee-herdsman adapter: ember (EFR32 / EmberZNet), zstack (TI), deconz, zigate, zboss, auto.
    pub adapter: String,
    /// Zigbee channel used when the network is first formed (11-26).
    pub channel: u8,
    /// Your names for paired devices, keyed by IEEE address.
    pub names: std::collections::BTreeMap<String, String>,
    // --- Zigbee2MQTT ---
    /// MQTT broker the Zigbee2MQTT instance publishes to.
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    /// Zigbee2MQTT base topic.
    pub base_topic: String,
    // --- both ---
    /// Upper bound on messages per second across all Zigbee lights.
    pub max_rate_hz: f32,
    /// Fade time sent with each change, seconds (0 = instant).
    pub transition_s: f32,
}

impl Default for ZigbeeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "usb".into(),
            serial_port: String::new(),
            adapter: "ember".into(),
            channel: 25,
            names: Default::default(),
            host: "localhost".into(),
            port: 1883,
            username: String::new(),
            password: String::new(),
            base_topic: "zigbee2mqtt".into(),
            max_rate_hz: 10.0,
            transition_s: 0.0,
        }
    }
}

impl ZigbeeSettings {
    /// Everything whose change needs a reconnect / coordinator restart.
    pub fn connection_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.enabled, self.mode, self.serial_port, self.adapter, self.host, self.port,
            self.username, self.password, self.base_topic, self.channel
        )
    }
}

// ---------- fixture profiles (types) ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub manufacturer: String,
    pub model: String,
    /// laser | par | fog | moving_head | strobe | other
    pub kind: String,
    pub channels: Vec<ChannelDef>,
    pub modes: Vec<Mode>,
    pub notes: String,
    /// Beam angle in degrees (0 = use the default for the kind).
    pub beam_deg: f32,
    /// Saved looks: a pattern plus secondary options, applied together.
    pub looks: Vec<Look>,
    /// Named groups of looks ("Chill", "Drop"…).
    pub look_sets: Vec<LookSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LookSet {
    pub id: String,
    pub name: String,
    pub color: String,
}

/// Several channel values that make one look (e.g. gallery + pattern + zoom + colour).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Look {
    pub id: String,
    pub name: String,
    /// Head the values were saved on ("P1"); empty for fixtures without heads.
    pub head: String,
    /// channel key -> value
    pub values: std::collections::BTreeMap<String, u8>,
    pub tags: Vec<String>,
    /// 0 = unrated, 1..5
    pub rating: u8,
    /// Look sets this look belongs to.
    pub sets: Vec<String>,
    /// Generated from the fixture's own ranges ("default looks"); regenerating
    /// replaces these and never touches your own looks.
    pub builtin: bool,
    /// Channels that move instead of holding their value (sweeps, steps, seeded random…).
    pub motions: std::collections::BTreeMap<String, Motion>,
}

/// How a look's channel moves over time. Everything is deterministic: the same
/// motion at the same beat/hit gives the same value, so patterns are repeatable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Motion {
    /// "sweep" | "step" | "random" | "list" | "catalog" | "follow"
    pub kind: String,
    /// Value ranges the motion moves through, in order ([min, max] inclusive).
    pub ranges: Vec<[u8; 2]>,
    /// "list": the values to play.
    pub values: Vec<u8>,
    /// "beat" | "time" (seconds) | "hit" (each time the route fires)
    pub clock: String,
    /// Beats (or seconds) per sweep / per step.
    pub period: f32,
    /// sweep: "up" | "down" | "pingpong" | "sine"
    pub shape: String,
    /// step: how far each step moves.
    pub step: u32,
    /// random: seed; the same seed always plays the same sequence.
    pub seed: u32,
    /// follow: a level source ("bass.level", "energy"…).
    pub source: String,
    /// catalog: catalogued values must carry these tags.
    pub tags: Vec<String>,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            kind: "sweep".into(),
            ranges: vec![[0, 255]],
            values: vec![],
            clock: "beat".into(),
            period: 4.0,
            shape: "pingpong".into(),
            step: 1,
            seed: 1,
            source: "level".into(),
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ChannelDef {
    /// Stable key used by modes, limits and routes.
    pub key: String,
    pub name: String,
    /// intensity | color | pan | tilt | pattern | beam | effect | speed | strobe | mode | fog | other
    pub group: String,
    pub default: u8,
    pub capabilities: Vec<Capability>,
    /// Notes/tags on individual values (pattern catalog).
    pub values: Vec<ValueNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Capability {
    pub min: u8,
    pub max: u8,
    pub label: String,
    /// Optional display colour, e.g. "#ff0000" for colour macros.
    pub color: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ValueNote {
    pub value: u8,
    pub label: String,
    pub tags: Vec<String>,
    /// 0 = unrated, 1..5
    pub rating: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Mode {
    pub name: String,
    /// Channel keys in DMX order.
    pub channels: Vec<String>,
    /// Sub-projectors / heads. Each head lists channel keys it owns.
    pub heads: Vec<Head>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Head {
    pub name: String,
    pub channels: Vec<String>,
}

impl Profile {
    pub fn mode(&self, name: &str) -> Option<&Mode> {
        self.modes.iter().find(|m| m.name == name).or(self.modes.first())
    }
    pub fn channel(&self, key: &str) -> Option<&ChannelDef> {
        self.channels.iter().find(|c| c.key == key)
    }
}

impl ChannelDef {
    pub fn value_blocked(&self, v: u8) -> bool {
        self.capabilities
            .iter()
            .any(|c| v >= c.min && v <= c.max && c.tags.iter().any(|t| t == TAG_BLOCKED))
            || self
                .values
                .iter()
                .any(|n| n.value == v && n.tags.iter().any(|t| t == TAG_BLOCKED))
    }

    /// Does value `v` carry all of `tags` (on its capability range or its value note)?
    pub fn value_has_tags(&self, v: u8, tags: &[String]) -> bool {
        tags.iter().all(|t| {
            self.capabilities
                .iter()
                .any(|c| v >= c.min && v <= c.max && c.tags.contains(t))
                || self.values.iter().any(|n| n.value == v && n.tags.contains(t))
        })
    }
}

// ---------- patched fixtures (instances) ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Fixture {
    pub id: String,
    pub name: String,
    pub profile_id: String,
    pub mode: String,
    pub output_id: String,
    /// 1-based DMX start address.
    pub address: u16,
    pub room_id: Option<String>,
    /// Position in metres within the room, and facing angle in degrees.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: f32,
    /// Where it points, in degrees up from horizontal: -90 = straight down at the floor,
    /// 90 = straight up at the ceiling. None = not aimed (effects use the fixture's position).
    pub tilt: Option<f32>,
    /// Beam angle in degrees, overriding the fixture type's.
    pub beam_deg: Option<f32>,
    /// Where the beam's centre lands (floor, ceiling or wall), worked out from the aim.
    /// Not saved: recalculated whenever the show goes live (see `Show::place_lights`).
    #[serde(skip)]
    pub spot: Option<[f32; 3]>,
    pub enabled: bool,
    /// Zigbee2MQTT friendly name. When set, the fixture is sent over Zigbee
    /// instead of Art-Net (output_id and address are ignored).
    pub zigbee_device: Option<String>,
    /// Per-channel limits keyed by channel key (venue calibration).
    pub limits: HashMap<String, Limit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Limit {
    /// Always output this value.
    pub lock: Option<u8>,
    /// Clamp into [min, max].
    pub range: Option<(u8, u8)>,
    /// Random/pattern picks must carry all of these tags.
    pub require_tags: Vec<String>,
    /// Minimum ms a value is held before it may change.
    pub min_hold_ms: u32,
}

// ---------- rooms ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Room {
    pub id: String,
    pub name: String,
    /// Wall outline, metres.
    pub walls: Vec<[f32; 2]>,
    pub ceiling_m: f32,
    /// Points effects can be centred on (DJ booth, dance floor…).
    pub focus: Vec<FocusPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FocusPoint {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Area of influence, metres. Used for "fade with distance" and twinkle clustering.
    pub radius: f32,
    pub color: String,
}

impl Default for FocusPoint {
    fn default() -> Self {
        Self { id: String::new(), name: "Focus".into(), x: 0.0, y: 0.0, z: 1.2, radius: 2.5, color: "#ffd27a".into() }
    }
}

impl Show {
    pub fn focus_point(&self, id: &str) -> Option<&FocusPoint> {
        if id.is_empty() {
            return None;
        }
        self.rooms.iter().flat_map(|r| r.focus.iter()).find(|f| f.id == id)
    }
}

// ---------- outputs ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ArtNetOutput {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub net: u8,
    pub subnet: u8,
    pub universe: u8,
    pub enabled: bool,
}

impl ArtNetOutput {
    /// 15-bit Art-Net port address.
    pub fn port_address(&self) -> u16 {
        ((self.net as u16 & 0x7f) << 8) | ((self.subnet as u16 & 0x0f) << 4) | (self.universe as u16 & 0x0f)
    }
}

// ---------- audio ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub device: Option<String>,
    /// Input gain in dB.
    pub gain_db: f32,
    /// Global onset sensitivity 0..1 (scales each band's own sensitivity).
    pub sensitivity: f32,
    /// User-defined detection bands. Each is a trigger source (its id) and a
    /// level source ("<id>.level").
    pub bands: Vec<BandDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BandRange {
    pub lo_hz: f32,
    pub hi_hz: f32,
    /// How much this range contributes to the band (0..2).
    pub weight: f32,
}

impl Default for BandRange {
    fn default() -> Self {
        Self { lo_hz: 100.0, hi_hz: 1000.0, weight: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BandDef {
    pub id: String,
    pub name: String,
    /// Muted bands report no level and no hits.
    pub enabled: bool,
    /// One or more frequency ranges, combined by weight
    /// (e.g. snare body 150-250 Hz + crack 2-5 kHz).
    pub ranges: Vec<BandRange>,
    /// "transient": hits on sudden rises (drums).
    /// "level": hits when the level crosses `level_threshold` (bass swells, vocals).
    pub detect: String,
    /// 0..1, used by "level" detection.
    pub level_threshold: f32,
    /// Band gain in dB (affects level output and onset floor).
    pub gain_db: f32,
    /// 0..1, higher = more triggers.
    pub sensitivity: f32,
    /// Minimum time between two hits.
    pub min_gap_ms: u32,
    /// Level release smoothing 0..0.98 (higher = slower fall).
    pub release: f32,
    pub color: String,
    /// Single-range fields from v1/v2 shows; read only, folded into `ranges`.
    #[serde(skip_serializing)]
    pub lo_hz: Option<f32>,
    #[serde(skip_serializing)]
    pub hi_hz: Option<f32>,
}

impl Default for BandDef {
    fn default() -> Self {
        Self {
            id: "band".into(),
            name: "Band".into(),
            enabled: true,
            ranges: vec![],
            detect: "transient".into(),
            level_threshold: 0.6,
            gain_db: 0.0,
            sensitivity: 0.5,
            min_gap_ms: 120,
            release: 0.85,
            color: "#7c5cff".into(),
            lo_hz: None,
            hi_hz: None,
        }
    }
}

/// Default detection bands. Kick stays first: the first band drives the beat clock.
/// Bass and vocals are sustained sources, so they use level detection and a slower
/// release (they're mostly useful as levels).
pub fn default_bands() -> Vec<BandDef> {
    let b = |id: &str, name: &str, ranges: &[(f32, f32)], detect: &str, sens, gap, release, color: &str| BandDef {
        id: id.into(),
        name: name.into(),
        ranges: ranges.iter().map(|&(lo_hz, hi_hz)| BandRange { lo_hz, hi_hz, weight: 1.0 }).collect(),
        detect: detect.into(),
        sensitivity: sens,
        min_gap_ms: gap,
        release,
        color: color.into(),
        ..Default::default()
    };
    vec![
        b("kick", "Kick", &[(35.0, 150.0)], "transient", 0.5, 140, 0.85, "#ff5c6c"),
        b("bass", "Bass", &[(60.0, 250.0)], "level", 0.35, 220, 0.92, "#ff9a4d"),
        b("snare", "Snare", &[(150.0, 250.0), (1800.0, 5000.0)], "transient", 0.5, 140, 0.85, "#f2b54a"),
        b("vocals", "Vocals", &[(300.0, 3000.0)], "level", 0.3, 250, 0.93, "#45d39a"),
        b("hat", "Hi-hat", &[(6000.0, 16000.0)], "transient", 0.5, 70, 0.8, "#4fb3ff"),
    ]
}

pub const SHOW_VERSION: u32 = 3;

impl Show {
    /// Upgrade shows saved by older versions. Safe to run repeatedly.
    pub fn migrate(&mut self) {
        if self.version < 2 {
            // v2 added bass and vocals bands; insert any defaults the show lacks.
            for (i, d) in default_bands().into_iter().enumerate() {
                if !self.audio.bands.iter().any(|b| b.id == d.id) {
                    let at = i.min(self.audio.bands.len());
                    self.audio.bands.insert(at, d);
                }
            }
        }
        if self.palettes.is_empty() {
            self.palettes = default_palettes();
        }
        // v3: bands hold a list of ranges.
        for b in &mut self.audio.bands {
            if b.ranges.is_empty() {
                b.ranges.push(BandRange {
                    lo_hz: b.lo_hz.unwrap_or(100.0),
                    hi_hz: b.hi_hz.unwrap_or(1000.0),
                    weight: 1.0,
                });
            }
            b.lo_hz = None;
            b.hi_hz = None;
            if b.detect.is_empty() {
                b.detect = "transient".into();
            }
        }
        self.version = SHOW_VERSION;
        self.place_lights();
    }

    /// Work out where every aimed fixture's light lands. Called by `migrate`, which runs
    /// whenever a show goes live.
    pub fn place_lights(&mut self) {
        let rooms = self.rooms.clone();
        for f in &mut self.fixtures {
            let room = f.room_id.as_ref().and_then(|id| rooms.iter().find(|r| &r.id == id));
            f.spot = light_spot(f, room);
        }
    }
}

impl Fixture {
    /// Where effects treat this fixture as being: where its light lands if it's aimed,
    /// otherwise where it hangs.
    pub fn pos(&self) -> (f32, f32, f32) {
        match self.spot {
            Some([x, y, z]) => (x, y, z),
            None => (self.x, self.y, self.z),
        }
    }
}

/// Ceiling height used when a room doesn't say.
pub const DEFAULT_CEILING_M: f32 = 3.0;
/// How far a beam is followed when it hits nothing (no walls drawn, pointing level).
const MAX_THROW_M: f32 = 30.0;

/// Where an aimed fixture's beam centre lands: the floor (z = 0), the ceiling, or the
/// first wall in the way. None if the fixture isn't aimed.
pub fn light_spot(f: &Fixture, room: Option<&Room>) -> Option<[f32; 3]> {
    let deg = f.tilt?.clamp(-90.0, 90.0);
    let (tilt, heading) = (deg.to_radians(), f.rotation.to_radians());
    // Straight up/down exactly (cos 90° isn't quite 0 in floating point).
    let d = if deg.abs() >= 89.99 { [0.0, 0.0, deg.signum()] } else { [tilt.cos() * heading.cos(), tilt.cos() * heading.sin(), tilt.sin()] };
    let ceiling = room.map(|r| r.ceiling_m).filter(|c| *c > 0.0).unwrap_or(DEFAULT_CEILING_M);
    let mut t = if d[2] < -1e-4 {
        f.z.max(0.0) / -d[2]
    } else if d[2] > 1e-4 {
        (ceiling - f.z).max(0.0) / d[2]
    } else {
        MAX_THROW_M
    };
    if let Some(walls) = room.map(|r| &r.walls).filter(|w| w.len() >= 3) {
        if let Some(s) = ray_hits_wall([f.x, f.y], [d[0], d[1]], walls) {
            t = t.min(s);
        }
    }
    Some([f.x + t * d[0], f.y + t * d[1], (f.z + t * d[2]).clamp(0.0, ceiling)])
}

/// Smallest s > 0 where the 2D ray `p + s·d` crosses one of the polygon's edges.
pub fn ray_hits_wall(p: [f32; 2], d: [f32; 2], walls: &[[f32; 2]]) -> Option<f32> {
    let mut best: Option<f32> = None;
    for i in 0..walls.len() {
        let (a, b) = (walls[i], walls[(i + 1) % walls.len()]);
        let e = [b[0] - a[0], b[1] - a[1]];
        let den = d[0] * e[1] - d[1] * e[0];
        if den.abs() < 1e-6 {
            continue; // parallel
        }
        let w = [a[0] - p[0], a[1] - p[1]];
        let s = (w[0] * e[1] - w[1] * e[0]) / den; // along the ray
        let u = (w[0] * d[1] - w[1] * d[0]) / den; // along the edge
        if s > 1e-4 && (-1e-4..=1.0 + 1e-4).contains(&u) {
            best = Some(best.map_or(s, |b: f32| b.min(s)));
        }
    }
    best
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { device: None, gain_db: 0.0, sensitivity: 0.5, bands: default_bands() }
    }
}

// ---------- routes ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Route {
    pub id: String,
    pub name: String,
    /// Free-form folder name used to organise routes in the UI.
    pub group: String,
    pub enabled: bool,
    /// Band id (hits), "<band>.level", "level", "energy", "phase", "beat" or "bar".
    pub source: String,
    /// Fixture ids and fixture-group ids to target. Both empty = every fixture
    /// that has the channel.
    pub fixtures: Vec<String>,
    pub groups: Vec<String>,
    /// Rooms scope the targets: only fixtures in these rooms (empty = any room).
    pub rooms: Vec<String>,
    /// A channel key ("p1_pattern"), or "@<channel type>" ("@intensity") to drive
    /// every channel of that type on each target, whatever it's called.
    pub channel: String,
    /// Focus point effects are calculated from ("" = centre of the targets).
    pub focus: String,
    /// Effects get dimmer with distance from the focus point, reaching zero at its radius.
    pub falloff: bool,
    pub action: RouteAction,
    pub cooldown_ms: u32,
    /// Trigger only every Nth event (1 = every).
    pub every: u32,
    /// 0..1 chance a trigger fires.
    pub probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteAction {
    /// Continuous: map the source level into [min, max].
    Modulate { min: u8, max: u8, smoothing: f32 },
    /// On trigger: jump to `on`, decay back to `off` over decay_ms.
    Flash { on: u8, off: u8, decay_ms: u32 },
    /// On trigger: pick a random value carrying `tags` (from catalog or capability ranges).
    Random { tags: Vec<String>, hold_ms: u32 },
    /// On trigger: one fixture flashes, hopping to a nearby fixture next time.
    /// With a palette, each hop also takes the palette's next colour.
    NodeHop {
        on: u8,
        off: u8,
        decay_ms: u32,
        #[serde(default)]
        palette: String,
    },
    // ----- fixture effects: drive each fixture's brightness/colour channels -----
    /// Strobe brightness. Trigger sources: a burst after each hit. Level sources:
    /// strobe while the level is above `threshold`.
    Strobe {
        /// "free" (uses `hz`) or flashes per beat: "1", "2", "4", "8".
        rate: String,
        hz: f32,
        /// Fraction of each flash that's on (0.05-0.9).
        duty: f32,
        burst_ms: u32,
        threshold: f32,
    },
    /// Colour from a palette. Each trigger steps to the next colour.
    Palette {
        palette: String,
        /// "all" (everyone the same), "spread" (left to right), "rings" (bands outward
        /// from the focus), "around" (slices around the focus), "random"
        mode: String,
        fade_ms: u32,
        /// 0 = leave brightness to other routes; otherwise set it.
        brightness: u8,
    },
    /// Random fixtures sparkle in palette colours over a dim base.
    Twinkle {
        palette: String,
        /// Sparkles per second at full level (level sources) or per hit (trigger sources).
        amount: f32,
        decay_ms: u32,
        base: u8,
    },
    /// A band of light/colour moving across the room in time with the beat.
    Wave {
        palette: String,
        /// "x" | "y" | "radial" (out from the focus) | "angle" (around the focus) | "height"
        axis: String,
        /// Beats per sweep.
        beats: f32,
        /// Width of the lit band, as a fraction of the sweep (0.05-1).
        width: f32,
        base: u8,
    },
    /// On trigger: step through `values` in order.
    Cycle { values: Vec<u8> },
    /// On trigger: apply a saved look (pattern + secondary options) from the fixture
    /// type, chosen by tags.
    Look {
        /// Only looks in this set ("" = any).
        #[serde(default)]
        set: String,
        /// Include generated default looks.
        #[serde(default = "yes")]
        include_defaults: bool,
        /// Looks must carry all of these tags.
        tags: Vec<String>,
        /// "random" (weighted by rating) | "cycle"
        mode: String,
        hold_ms: u32,
        /// Heads to apply to ("P1", "P2"). Empty = the head each look was saved on.
        heads: Vec<String>,
    },
}

impl Default for RouteAction {
    fn default() -> Self {
        RouteAction::Flash { on: 255, off: 0, decay_ms: 200 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn aimed(x: f32, y: f32, z: f32, rotation: f32, tilt: Option<f32>) -> Fixture {
        Fixture { x, y, z, rotation, tilt, ..Default::default() }
    }
    fn room() -> Room {
        Room { walls: vec![[0.0, 0.0], [8.0, 0.0], [8.0, 6.0], [0.0, 6.0]], ceiling_m: 3.0, ..Default::default() }
    }
    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01)
    }

    #[test]
    fn light_lands_where_it_points() {
        let r = room();
        // not aimed: no spot, effects use the fixture's position
        let f = aimed(2.0, 2.0, 2.5, 0.0, None);
        assert_eq!(light_spot(&f, Some(&r)), None);
        assert_eq!(f.pos(), (2.0, 2.0, 2.5));
        // straight down / up
        assert!(near(light_spot(&aimed(2.0, 2.0, 2.5, 0.0, Some(-90.0)), Some(&r)).unwrap(), [2.0, 2.0, 0.0]));
        assert!(near(light_spot(&aimed(2.0, 2.0, 0.2, 0.0, Some(90.0)), Some(&r)).unwrap(), [2.0, 2.0, 3.0]));
        // 45° down from 2 m facing +x lands 2 m away on the floor
        assert!(near(light_spot(&aimed(2.0, 2.0, 2.0, 0.0, Some(-45.0)), Some(&r)).unwrap(), [4.0, 2.0, 0.0]));
        // a wall in the way: uplight at 30° facing the near wall hits the wall first
        let wash = light_spot(&aimed(7.5, 3.0, 0.2, 0.0, Some(30.0)), Some(&r)).unwrap();
        assert!(near(wash, [8.0, 3.0, 0.2 + 0.5 * 30f32.to_radians().tan()]), "{wash:?}");
        // pointing level across the room hits the far wall at its own height
        assert!(near(light_spot(&aimed(1.0, 3.0, 1.5, 180.0, Some(0.0)), Some(&r)).unwrap(), [0.0, 3.0, 1.5]));
        // no room: default ceiling, no walls
        assert!(near(light_spot(&aimed(0.0, 0.0, 1.0, 0.0, Some(90.0)), None).unwrap(), [0.0, 0.0, DEFAULT_CEILING_M]));
        // tilt is clamped
        assert!(near(light_spot(&aimed(2.0, 2.0, 2.5, 0.0, Some(-400.0)), Some(&r)).unwrap(), [2.0, 2.0, 0.0]));
    }

    #[test]
    fn migrate_places_lights_and_spots_are_not_saved() {
        let mut s = Show { rooms: vec![Room { id: "r".into(), ..room() }], ..Default::default() };
        s.fixtures.push(Fixture { id: "f".into(), room_id: Some("r".into()), x: 4.0, y: 3.0, z: 2.8, tilt: Some(-90.0), ..Default::default() });
        s.migrate();
        assert_eq!(s.fixtures[0].pos(), (4.0, 3.0, 0.0));
        let json = serde_json::to_value(&s).unwrap();
        assert!(json["fixtures"][0].get("spot").is_none());
        assert_eq!(json["fixtures"][0]["tilt"], -90.0);
        // old shows without the fields still load
        let old: Fixture = serde_json::from_str(r#"{"id":"x","x":1,"y":2,"z":3,"rotation":90}"#).unwrap();
        assert_eq!((old.tilt, old.beam_deg, old.spot), (None, None, None));
    }

    #[test]
    fn migrates_v1_bands() {
        let mut s: Show = serde_json::from_str(
            r#"{"version":1,"audio":{"bands":[
              {"id":"kick","lo_hz":40,"hi_hz":120},{"id":"snare"},{"id":"hat"},{"id":"mine"}]}}"#,
        )
        .unwrap();
        s.migrate();
        let ids: Vec<&str> = s.audio.bands.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["kick", "bass", "snare", "vocals", "hat", "mine"]);
        assert_eq!(s.audio.bands[0].ranges, vec![BandRange { lo_hz: 40.0, hi_hz: 120.0, weight: 1.0 }]); // user edits kept
        assert_eq!(s.version, SHOW_VERSION);
        // running again changes nothing
        s.migrate();
        assert_eq!(s.audio.bands.len(), 6);
    }
}

fn yes() -> bool {
    true
}

impl Profile {
    /// Generate starter looks from the fixture's own pattern ranges: for each head's
    /// pattern channel(s), a few values spread across every usable range. Replaces
    /// previously generated defaults; your own looks are kept.
    pub fn generate_default_looks(&mut self, per_range: usize) -> usize {
        self.looks.retain(|l| !l.builtin);
        let Some(mode) = self.modes.first().cloned() else { return 0 };
        let mut heads: Vec<(String, Vec<String>)> = mode.heads.iter().map(|h| (h.name.clone(), h.channels.clone())).collect();
        if heads.is_empty() {
            heads.push((String::new(), mode.channels.clone()));
        }
        let mut made = 0;
        for (head, keys) in heads {
            for key in &keys {
                let Some(ch) = self.channel(key).cloned() else { continue };
                if ch.group != "pattern" {
                    continue;
                }
                for cap in &ch.capabilities {
                    if cap.tags.iter().any(|t| t == TAG_BLOCKED || t == "off") || cap.label.to_lowercase().contains("undefined") {
                        continue;
                    }
                    let span = cap.max as usize - cap.min as usize;
                    let n = per_range.max(1).min(span + 1);
                    for i in 0..n {
                        let v = (cap.min as usize + if n == 1 { span / 2 } else { span * i / (n - 1) }) as u8;
                        let mut values = std::collections::BTreeMap::new();
                        values.insert(key.clone(), v);
                        let mut tags = vec!["default".to_string()];
                        tags.extend(cap.tags.iter().cloned());
                        self.looks.push(Look {
                            id: format!("default-{}-{}", key, v),
                            name: format!("{}{} {}", if head.is_empty() { String::new() } else { format!("{head} · ") }, cap.label, v),
                            head: head.clone(),
                            values,
                            tags,
                            rating: 0,
                            sets: vec![],
                            builtin: true,
                            motions: Default::default(),
                        });
                        made += 1;
                    }
                }
            }
        }
        made
    }
}
