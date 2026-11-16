//! Zigbee lights, two ways:
//! - "usb": LightDeck is the coordinator. A small Node helper (zigbee-helper/, using
//!   zigbee-herdsman) drives the USB dongle; we talk to it with JSON lines over stdio.
//! - "mqtt": an existing Zigbee2MQTT install, reached through its broker.
//!
//! Either way, Zigbee fixtures run through the normal engine (routes, limits,
//! blackout). Their channel values become a desired light state, and one paced
//! sender never exceeds the configured message rate (Zigbee meshes choke on
//! floods). Each slot goes to the light whose state is most out of date, so
//! on/off changes land first.

use crate::model::{ChannelDef, Profile, ZigbeeSettings};
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Desired {
    pub on: bool,
    pub brightness: u8,
    pub rgb: Option<[u8; 3]>,
    /// Mireds.
    pub color_temp: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Device {
    /// What fixtures reference: the IEEE address in USB mode, the friendly name in MQTT mode.
    pub key: String,
    pub friendly_name: String,
    pub ieee: String,
    pub vendor: String,
    pub model: String,
    pub description: String,
    /// light | switch | other
    pub kind: String,
    pub brightness: bool,
    pub color: bool,
    pub color_temp: bool,
    /// on/off only (plugs, relays)
    pub switch_only: bool,
    /// USB mode extras
    pub role: String,
    pub power: String,
    pub interview: String,
    pub lqi: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Port {
    pub name: String,
    pub path: String,
    pub adapter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Status {
    pub enabled: bool,
    pub mode: String,
    /// MQTT: broker connected. USB: coordinator running on the dongle.
    pub connected: bool,
    pub bridge_online: Option<bool>,
    pub error: Option<String>,
    /// USB: what's happening right now ("Starting coordinator…", "Network resumed" …)
    pub state: String,
    pub coordinator: Option<String>,
    /// USB: pairing open until this many ms since the epoch (0 = closed).
    pub permit_join_until: u64,
    pub devices: Vec<Device>,
    pub sent_per_sec: f32,
    /// Lights still waiting for an update (backlog > 0 means the rate limit is biting).
    pub backlog: usize,
    pub last_send_error: Option<String>,
    /// Recent helper log lines (USB mode).
    pub log: Vec<String>,
}

/// Network identity, generated once and kept with the app data. Changing it
/// orphans every paired device, so it never lives in a project file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkKeys {
    pan_id: u16,
    extended_pan_id: Vec<u8>,
    network_key: Vec<u8>,
    channel: u8,
}

struct Helper {
    child: Child,
    stdin: ChildStdin,
}

pub struct Zigbee {
    settings: Mutex<ZigbeeSettings>,
    version: AtomicU64,
    desired: Mutex<HashMap<String, Desired>>,
    status: Mutex<Status>,
    mqtt: Mutex<Option<Client>>,
    helper: Mutex<Option<Helper>>,
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, Sender<Result<Value, String>>>>,
    data_dir: PathBuf,
    helper_dir: PathBuf,
}

impl Zigbee {
    pub fn start(settings: ZigbeeSettings, data_dir: PathBuf, helper_dir: PathBuf) -> Arc<Self> {
        let z = Arc::new(Self {
            settings: Mutex::new(settings),
            version: AtomicU64::new(1),
            desired: Mutex::new(HashMap::new()),
            status: Mutex::new(Status::default()),
            mqtt: Mutex::new(None),
            helper: Mutex::new(None),
            next_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            data_dir,
            helper_dir,
        });
        let a = z.clone();
        std::thread::Builder::new().name("zigbee-conn".into()).spawn(move || a.supervise()).unwrap();
        let b = z.clone();
        std::thread::Builder::new().name("zigbee-send".into()).spawn(move || b.send_loop()).unwrap();
        z
    }

    pub fn configure(&self, s: &ZigbeeSettings) {
        let mut cur = self.settings.lock().unwrap();
        if *cur != *s {
            let reconnect = cur.connection_key() != s.connection_key();
            *cur = s.clone();
            if reconnect {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            drop(cur);
            self.refresh_names();
        }
    }

    pub fn set_desired(&self, d: HashMap<String, Desired>) {
        *self.desired.lock().unwrap() = d;
    }

    pub fn status(&self) -> Status {
        let mut st = self.status.lock().unwrap().clone();
        let now = epoch_ms();
        if st.permit_join_until <= now {
            st.permit_join_until = 0;
        }
        st
    }

    pub fn reconnect(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    // ---------- user actions ----------

    pub fn identify(&self, key: &str) -> Result<(), String> {
        let s = self.settings.lock().unwrap().clone();
        if s.mode == "mqtt" {
            self.mqtt_publish(&format!("{}/{key}/set", s.base_topic.trim_end_matches('/')), json!({ "effect": "blink" }))
        } else {
            self.request(json!({ "cmd": "identify", "ieee": key }), Duration::from_secs(8)).map(|_| ())
        }
    }

    pub fn ports(&self) -> Result<Vec<Port>, String> {
        let v = self.request(json!({ "cmd": "ports" }), Duration::from_secs(8))?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .map(|p| Port {
                name: p["name"].as_str().unwrap_or_default().into(),
                path: p["path"].as_str().unwrap_or_default().into(),
                adapter: p["adapter"].as_str().map(String::from),
            })
            .collect())
    }

    pub fn permit_join(&self, seconds: u32) -> Result<(), String> {
        self.request(json!({ "cmd": "permit_join", "seconds": seconds }), Duration::from_secs(10))?;
        self.status.lock().unwrap().permit_join_until =
            if seconds > 0 { epoch_ms() + seconds as u64 * 1000 } else { 0 };
        Ok(())
    }

    pub fn remove(&self, ieee: &str, force: bool) -> Result<(), String> {
        self.request(json!({ "cmd": "remove", "ieee": ieee, "force": force }), Duration::from_secs(15)).map(|_| ())
    }

    pub fn reinterview(&self, ieee: &str) -> Result<(), String> {
        self.request(json!({ "cmd": "interview", "ieee": ieee }), Duration::from_secs(5)).map(|_| ())
    }

    // ---------- supervisor ----------

    fn supervise(self: Arc<Self>) {
        loop {
            let ver = self.version.load(Ordering::Relaxed);
            let s = self.settings.lock().unwrap().clone();
            {
                let mut st = self.status.lock().unwrap();
                st.enabled = s.enabled;
                st.mode = s.mode.clone();
                if !s.enabled {
                    st.connected = false;
                    st.error = None;
                    st.state = "Off".into();
                    st.devices.clear();
                }
            }
            if !s.enabled {
                self.stop_helper();
                *self.mqtt.lock().unwrap() = None;
                self.wait_change(ver, 500);
                continue;
            }
            if s.mode == "mqtt" {
                self.stop_helper();
                self.run_mqtt(&s, ver);
            } else {
                *self.mqtt.lock().unwrap() = None;
                self.run_usb(&s, ver);
            }
        }
    }

    /// Sleep up to `ms`, returning early if settings changed.
    fn wait_change(&self, ver: u64, ms: u64) {
        let until = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < until && self.version.load(Ordering::Relaxed) == ver {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    // ---------- USB coordinator ----------

    fn run_usb(self: &Arc<Self>, s: &ZigbeeSettings, ver: u64) {
        if let Err(e) = self.spawn_helper() {
            let mut st = self.status.lock().unwrap();
            st.connected = false;
            st.state = "Helper not running".into();
            st.error = Some(e);
            drop(st);
            self.wait_change(ver, 5000);
            return;
        }
        if s.serial_port.trim().is_empty() {
            let mut st = self.status.lock().unwrap();
            st.connected = false;
            st.error = None;
            st.state = "Choose the dongle's port".into();
            drop(st);
            while self.version.load(Ordering::Relaxed) == ver && self.helper_alive() {
                std::thread::sleep(Duration::from_millis(200));
            }
            return;
        }
        let keys = match self.network_keys(s.channel) {
            Ok(k) => k,
            Err(e) => {
                self.status.lock().unwrap().error = Some(e);
                self.wait_change(ver, 5000);
                return;
            }
        };
        self.status.lock().unwrap().state = "Starting coordinator…".into();
        let req = json!({
            "cmd": "start",
            "port": s.serial_port,
            "adapter": if s.adapter.is_empty() { "auto" } else { s.adapter.as_str() },
            "data_dir": self.data_dir.join("zigbee").to_string_lossy(),
            "network": keys,
        });
        match self.request(req, Duration::from_secs(90)) {
            Ok(v) => {
                let mut st = self.status.lock().unwrap();
                st.connected = true;
                st.error = None;
                st.state = match v["result"].as_str() {
                    Some("reset") => "New network formed".into(),
                    Some("restored") => "Network restored from backup".into(),
                    _ => "Network resumed".into(),
                };
                let ver_meta = &v["version"];
                let chan = v["params"]["channel"].as_u64();
                st.coordinator = Some(format!(
                    "{}{}",
                    ver_meta["type"].as_str().unwrap_or("Coordinator"),
                    chan.map(|c| format!(" · channel {c}")).unwrap_or_default()
                ));
            }
            Err(e) => {
                let mut st = self.status.lock().unwrap();
                st.connected = false;
                st.state = "Couldn't start the coordinator".into();
                st.error = Some(e);
                drop(st);
                self.stop_helper();
                self.wait_change(ver, 5000);
                return;
            }
        }
        // Stay here while running; the reader thread updates status.
        while self.version.load(Ordering::Relaxed) == ver && self.helper_alive() && self.status.lock().unwrap().connected {
            std::thread::sleep(Duration::from_millis(200));
        }
        if self.version.load(Ordering::Relaxed) != ver {
            let _ = self.request(json!({ "cmd": "stop" }), Duration::from_secs(10));
        } else {
            // Unexpected: dongle unplugged or helper died. Retry shortly.
            self.wait_change(ver, 3000);
        }
        self.stop_helper();
        self.status.lock().unwrap().connected = false;
    }

    fn network_keys(&self, channel: u8) -> Result<NetworkKeys, String> {
        let dir = self.data_dir.join("zigbee");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("network.json");
        if let Ok(b) = std::fs::read(&path) {
            if let Ok(k) = serde_json::from_slice::<NetworkKeys>(&b) {
                return Ok(k);
            }
        }
        let bytes = |n: usize| (0..n).map(|_| fastrand::u8(..)).collect::<Vec<u8>>();
        let k = NetworkKeys {
            pan_id: fastrand::u16(1..0x3fff),
            extended_pan_id: bytes(8),
            network_key: bytes(16),
            channel: channel.clamp(11, 26),
        };
        std::fs::write(&path, serde_json::to_vec_pretty(&k).unwrap()).map_err(|e| e.to_string())?;
        Ok(k)
    }

    fn find_node(helper_dir: &std::path::Path) -> Option<PathBuf> {
        if let Ok(p) = std::env::var("LIGHTDECK_NODE") {
            return Some(PathBuf::from(p));
        }
        // Release builds ship their own Node runtime next to the helper.
        let bundled = helper_dir.join("runtime").join(if cfg!(windows) { "node.exe" } else { "node" });
        if bundled.exists() {
            return Some(bundled);
        }
        // GUI apps on macOS don't get the shell PATH, so check the usual places.
        for p in ["/opt/homebrew/bin/node", "/usr/local/bin/node", "/usr/bin/node"] {
            if std::path::Path::new(p).exists() {
                return Some(PathBuf::from(p));
            }
        }
        let out = Command::new("/bin/sh").args(["-lc", "command -v node"]).output().ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!s.is_empty()).then(|| PathBuf::from(s))
    }

    fn helper_alive(&self) -> bool {
        let mut h = self.helper.lock().unwrap();
        match h.as_mut() {
            Some(x) => matches!(x.child.try_wait(), Ok(None)),
            None => false,
        }
    }

    fn spawn_helper(self: &Arc<Self>) -> Result<(), String> {
        if self.helper_alive() {
            return Ok(());
        }
        let node = Self::find_node(&self.helper_dir).ok_or("Node.js wasn't found. Install it (brew install node) to use a USB dongle.")?;
        let script = self.helper_dir.join("index.mjs");
        if !script.exists() {
            return Err(format!("Zigbee helper missing at {}", script.display()));
        }
        if !self.helper_dir.join("node_modules").exists() {
            return Err(format!("Zigbee helper isn't installed. Run `npm install` in {}", self.helper_dir.display()));
        }
        let mut child = Command::new(node)
            .arg(&script)
            .current_dir(&self.helper_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Couldn't start the Zigbee helper: {e}"))?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let me = self.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    me.on_helper_message(v);
                }
            }
            // helper exited: fail anything still waiting
            for (_, tx) in me.pending.lock().unwrap().drain() {
                let _ = tx.send(Err("Zigbee helper stopped".into()));
            }
        });
        let me = self.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut st = me.status.lock().unwrap();
                st.log.push(line);
                let n = st.log.len();
                if n > 80 {
                    st.log.drain(..n - 80);
                }
            }
        });
        *self.helper.lock().unwrap() = Some(Helper { child, stdin });
        Ok(())
    }

    fn stop_helper(&self) {
        if let Some(mut h) = self.helper.lock().unwrap().take() {
            drop(h.stdin); // helper stops the coordinator and exits on EOF
            for _ in 0..30 {
                if matches!(h.child.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = h.child.kill();
        }
    }

    fn write_helper(&self, msg: &Value) -> Result<(), String> {
        let mut h = self.helper.lock().unwrap();
        let h = h.as_mut().ok_or("Zigbee helper isn't running")?;
        let mut line = msg.to_string();
        line.push('\n');
        h.stdin.write_all(line.as_bytes()).and_then(|_| h.stdin.flush()).map_err(|e| e.to_string())
    }

    fn request(&self, mut msg: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        msg["id"] = json!(id);
        let (tx, rx) = channel();
        self.pending.lock().unwrap().insert(id, tx);
        if let Err(e) = self.write_helper(&msg) {
            self.pending.lock().unwrap().remove(&id);
            return Err(e);
        }
        let r = rx.recv_timeout(timeout).unwrap_or_else(|_| Err("The Zigbee helper didn't answer in time".into()));
        self.pending.lock().unwrap().remove(&id);
        r
    }

    fn on_helper_message(&self, v: Value) {
        if let Some(id) = v["id"].as_u64() {
            let res = if v["ok"] == true {
                Ok(v["result"].clone())
            } else {
                Err(v["error"].as_str().unwrap_or("error").to_string())
            };
            if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
                let _ = tx.send(res);
            } else if let Err(e) = res {
                // fire-and-forget light update failed
                self.status.lock().unwrap().last_send_error = Some(e);
            }
            return;
        }
        let mut st = self.status.lock().unwrap();
        match v["event"].as_str() {
            Some("devices") => {
                let names = self.settings.lock().unwrap().names.clone();
                st.devices = v["devices"].as_array().into_iter().flatten().map(|d| usb_device(d, &names)).collect();
            }
            Some("permit_join") => {
                st.permit_join_until = if v["permitted"] == true {
                    epoch_ms() + v["time"].as_u64().unwrap_or(0) * 1000
                } else {
                    0
                };
            }
            Some("device_interview") => {
                let name = v["device"]["model"].as_str().unwrap_or("device");
                st.state = match v["status"].as_str() {
                    Some("started") => format!("Setting up new {name}…"),
                    Some("successful") => format!("Added {name}"),
                    Some("failed") => format!("Couldn't finish setting up {name}; try pairing again"),
                    _ => st.state.clone(),
                };
            }
            Some("device_joined") => st.state = "New device joining…".into(),
            Some("adapter_disconnected") => {
                st.connected = false;
                st.error = Some("The dongle disconnected".into());
            }
            _ => {}
        }
    }

    fn refresh_names(&self) {
        let names = self.settings.lock().unwrap().names.clone();
        let mut st = self.status.lock().unwrap();
        if st.mode != "usb" {
            return;
        }
        for d in &mut st.devices {
            d.friendly_name = names.get(&d.ieee).cloned().unwrap_or_else(|| default_name(&d.vendor, &d.model, &d.ieee));
        }
    }

    // ---------- MQTT (Zigbee2MQTT) ----------

    fn mqtt_publish(&self, topic: &str, payload: Value) -> Result<(), String> {
        let c = self.mqtt.lock().unwrap();
        let c = c.as_ref().ok_or("Not connected to the MQTT broker")?;
        c.try_publish(topic, QoS::AtMostOnce, false, payload.to_string()).map_err(|e| e.to_string())
    }

    fn run_mqtt(&self, s: &ZigbeeSettings, ver: u64) {
        if s.host.trim().is_empty() {
            self.status.lock().unwrap().state = "Enter your MQTT broker".into();
            self.wait_change(ver, 500);
            return;
        }
        let mut opts = MqttOptions::new(format!("lightdeck-{}", std::process::id()), s.host.trim(), s.port);
        opts.set_keep_alive(Duration::from_secs(15));
        opts.set_max_packet_size(8 * 1024 * 1024, 64 * 1024);
        if !s.username.is_empty() {
            opts.set_credentials(s.username.clone(), s.password.clone());
        }
        let (client, mut conn) = Client::new(opts, 64);
        *self.mqtt.lock().unwrap() = Some(client.clone());
        let base = s.base_topic.trim_end_matches('/').to_string();
        let mut failed = false;
        self.status.lock().unwrap().state = "Connecting to broker…".into();
        while self.version.load(Ordering::Relaxed) == ver {
            match conn.recv_timeout(Duration::from_millis(250)) {
                Ok(Ok(Event::Incoming(Packet::ConnAck(_)))) => {
                    let _ = client.subscribe(format!("{base}/bridge/devices"), QoS::AtMostOnce);
                    let _ = client.subscribe(format!("{base}/bridge/state"), QoS::AtMostOnce);
                    let mut st = self.status.lock().unwrap();
                    st.connected = true;
                    st.error = None;
                    st.state = "Broker connected".into();
                }
                Ok(Ok(Event::Incoming(Packet::Publish(p)))) => {
                    if p.topic.ends_with("/bridge/devices") {
                        if let Ok(v) = serde_json::from_slice::<Value>(&p.payload) {
                            self.status.lock().unwrap().devices = parse_devices(&v);
                        }
                    } else if p.topic.ends_with("/bridge/state") {
                        let txt = String::from_utf8_lossy(&p.payload).to_string();
                        let online = serde_json::from_str::<Value>(&txt)
                            .ok()
                            .and_then(|v| v["state"].as_str().map(|s| s == "online"))
                            .unwrap_or(txt.contains("online"));
                        self.status.lock().unwrap().bridge_online = Some(online);
                    }
                }
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    let mut st = self.status.lock().unwrap();
                    st.connected = false;
                    st.error = Some(format!("MQTT: {e}"));
                    failed = true;
                    break;
                }
                Err(_) => {} // timeout: loop to check for setting changes
            }
        }
        *self.mqtt.lock().unwrap() = None;
        let _ = client.disconnect();
        self.status.lock().unwrap().connected = false;
        if failed {
            self.wait_change(ver, 3000);
        }
    }

    // ---------- paced sender ----------

    fn send_one(&self, s: &ZigbeeSettings, key: &str, d: &Desired, prev: Option<&Desired>) -> Result<(), String> {
        if s.mode == "mqtt" {
            self.mqtt_publish(&format!("{}/{key}/set", s.base_topic.trim_end_matches('/')), payload(d, prev, s.transition_s))
        } else {
            let colour_changed = prev.map(|x| !x.on || x.rgb != d.rgb || x.color_temp != d.color_temp).unwrap_or(true);
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            self.write_helper(&json!({
                "id": id,
                "cmd": "set",
                "ieee": key,
                "on": d.on,
                "brightness": d.brightness,
                "rgb": if colour_changed { json!(d.rgb) } else { Value::Null },
                "color_temp": if colour_changed { json!(d.color_temp) } else { Value::Null },
                "transition": s.transition_s,
            }))
        }
    }

    fn send_loop(self: Arc<Self>) {
        let mut sent: HashMap<String, (Desired, Instant)> = HashMap::new();
        let mut window: Vec<Instant> = vec![];
        loop {
            let s = self.settings.lock().unwrap().clone();
            let rate = s.max_rate_hz.clamp(1.0, 50.0);
            std::thread::sleep(Duration::from_secs_f32(1.0 / rate));
            let connected = self.status.lock().unwrap().connected;
            if !s.enabled || !connected {
                sent.clear();
                continue;
            }
            let desired = self.desired.lock().unwrap().clone();
            let now = Instant::now();
            // Score every light that differs meaningfully from what it last received.
            let mut best: Option<(f32, &String, &Desired)> = None;
            let mut backlog = 0;
            for (name, d) in &desired {
                let (score, stale) = match sent.get(name) {
                    None => (1000.0, true),
                    Some((prev, at)) => {
                        let diff = difference(prev, d);
                        let age = now.duration_since(*at).as_secs_f32();
                        (diff * (1.0 + age * 4.0), diff > 0.0)
                    }
                };
                if stale {
                    backlog += 1;
                    if best.map(|b| score > b.0).unwrap_or(true) {
                        best = Some((score, name, d));
                    }
                }
            }
            if let Some((_, name, d)) = best {
                if self.send_one(&s, name, d, sent.get(name).map(|x| &x.0)).is_ok() {
                    sent.insert(name.clone(), (*d, now));
                    window.push(now);
                    backlog -= 1;
                }
            }
            window.retain(|t| now.duration_since(*t) < Duration::from_secs(1));
            let mut st = self.status.lock().unwrap();
            st.sent_per_sec = window.len() as f32;
            st.backlog = backlog;
        }
    }
}

fn epoch_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn default_name(vendor: &str, model: &str, ieee: &str) -> String {
    let tail = &ieee[ieee.len().saturating_sub(4)..];
    let base = format!("{vendor} {model}").trim().to_string();
    if base.is_empty() { format!("Device {tail}") } else { format!("{base} {tail}") }
}

fn usb_device(d: &Value, names: &std::collections::BTreeMap<String, String>) -> Device {
    let ieee = d["ieee"].as_str().unwrap_or_default().to_string();
    let vendor = d["manufacturer"].as_str().unwrap_or_default().to_string();
    let model = d["model"].as_str().unwrap_or_default().to_string();
    let (on_off, level, color, ct) =
        (d["on_off"] == true, d["level"] == true, d["color"] == true, d["color_temp"] == true);
    let kind = if level || color || ct { "light" } else if on_off { "switch" } else { "other" };
    Device {
        key: ieee.clone(),
        friendly_name: names.get(&ieee).cloned().unwrap_or_else(|| default_name(&vendor, &model, &ieee)),
        ieee,
        vendor,
        model,
        description: String::new(),
        kind: kind.into(),
        brightness: level,
        color,
        color_temp: ct,
        switch_only: kind == "switch",
        role: d["role"].as_str().unwrap_or_default().into(),
        power: d["power"].as_str().unwrap_or_default().into(),
        interview: d["interview"].as_str().unwrap_or_default().into(),
        lqi: d["lqi"].as_u64().map(|x| x.min(255) as u8),
    }
}

/// 0 = same; larger = more visibly different. Small changes below the deadband are ignored.
fn difference(a: &Desired, b: &Desired) -> f32 {
    if a.on != b.on {
        return 500.0;
    }
    if !b.on {
        return 0.0;
    }
    let mut d = 0.0;
    let db = (a.brightness as f32 - b.brightness as f32).abs();
    if db >= 4.0 {
        d += db;
    }
    if let (Some(x), Some(y)) = (a.rgb, b.rgb) {
        let dc: f32 = x.iter().zip(y.iter()).map(|(p, q)| (*p as f32 - *q as f32).abs()).sum();
        if dc >= 24.0 {
            d += dc / 3.0;
        }
    } else if a.rgb != b.rgb {
        d += 100.0;
    }
    if let (Some(x), Some(y)) = (a.color_temp, b.color_temp) {
        let dt = (x as f32 - y as f32).abs();
        if dt >= 10.0 {
            d += dt / 2.0;
        }
    }
    d
}

fn payload(d: &Desired, prev: Option<&Desired>, transition: f32) -> Value {
    if !d.on {
        return json!({ "state": "OFF", "transition": transition });
    }
    let mut p = json!({ "state": "ON", "brightness": d.brightness.max(1) });
    if transition > 0.0 {
        p["transition"] = json!(transition);
    }
    // Only resend colour when it changed (saves airtime on bulbs that are just dimming).
    let colour_changed = prev.map(|x| !x.on || x.rgb != d.rgb || x.color_temp != d.color_temp).unwrap_or(true);
    if colour_changed {
        if let Some([r, g, b]) = d.rgb {
            p["color"] = json!({ "r": r, "g": g, "b": b });
        } else if let Some(ct) = d.color_temp {
            p["color_temp"] = json!(ct);
        }
    }
    p
}

/// Translate a fixture's channel values into a Zigbee light state.
pub fn desired_from(profile: &Profile, chans: &[&ChannelDef], values: &[u8]) -> Desired {
    let mut intensity: Option<u8> = None;
    let mut rgb = [None::<u8>; 3];
    let mut macro_rgb: Option<[u8; 3]> = None;
    let mut ct: Option<u16> = None;
    let mut forced_off = false;
    for (c, v) in chans.iter().zip(values) {
        let cap = c.capabilities.iter().find(|k| *v >= k.min && *v <= k.max);
        if cap.map(|k| k.tags.iter().any(|t| t == "off")).unwrap_or(false) {
            forced_off = true;
        }
        let key = c.key.as_str();
        match c.group.as_str() {
            "intensity" => intensity = Some(intensity.map_or(*v, |i: u8| i.max(*v))),
            "color" => match key {
                "red" => rgb[0] = Some(*v),
                "green" => rgb[1] = Some(*v),
                "blue" => rgb[2] = Some(*v),
                "color_temp" => ct = Some((500.0 - (*v as f32 / 255.0) * (500.0 - 153.0)).round() as u16),
                _ => {
                    if let Some(hex) = cap.and_then(|k| k.color.as_deref()) {
                        macro_rgb = parse_hex(hex);
                    }
                }
            },
            _ => {}
        }
    }
    let rgb = match rgb {
        [Some(r), Some(g), Some(b)] => Some([r, g, b]),
        _ => macro_rgb,
    };
    let brightness = intensity.unwrap_or(255);
    let black = rgb.map(|c| c.iter().all(|x| *x == 0)).unwrap_or(false);
    // Plugs/relays: on above half, and never sent a brightness flood.
    let min_on = if profile.id == "zigbee-switch" { 128 } else { 1 };
    Desired { on: !forced_off && brightness >= min_on && !black, brightness, rgb, color_temp: ct }
}

fn parse_hex(h: &str) -> Option<[u8; 3]> {
    let h = h.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([p(0)?, p(2)?, p(4)?])
}

fn parse_devices(v: &Value) -> Vec<Device> {
    let mut out = vec![];
    for d in v.as_array().into_iter().flatten() {
        if d["type"] == "Coordinator" {
            continue;
        }
        let def = &d["definition"];
        let mut dev = Device {
            key: d["friendly_name"].as_str().unwrap_or_default().into(),
            friendly_name: d["friendly_name"].as_str().unwrap_or_default().into(),
            ieee: d["ieee_address"].as_str().unwrap_or_default().into(),
            vendor: def["vendor"].as_str().unwrap_or_default().into(),
            model: def["model"].as_str().unwrap_or_default().into(),
            description: def["description"].as_str().unwrap_or_default().into(),
            kind: "other".into(),
            ..Default::default()
        };
        for e in def["exposes"].as_array().into_iter().flatten() {
            let t = e["type"].as_str().unwrap_or("");
            if t == "light" || t == "switch" {
                if dev.kind == "other" {
                    dev.kind = t.into();
                }
                for f in e["features"].as_array().into_iter().flatten() {
                    match f["name"].as_str().unwrap_or("") {
                        "brightness" => dev.brightness = true,
                        "color_temp" => dev.color_temp = true,
                        "color_xy" | "color_hs" => dev.color = true,
                        _ => {}
                    }
                }
            }
        }
        dev.switch_only = dev.kind == "switch" && !dev.brightness;
        out.push(dev);
    }
    out.sort_by(|a, b| (a.kind != "light").cmp(&(b.kind != "light")).then(a.friendly_name.cmp(&b.friendly_name)));
    out
}

/// Built-in profiles for Zigbee lights. Channel keys red/green/blue/color_temp are what
/// `desired_from` looks for.
pub fn builtin_profiles() -> Vec<Profile> {
    use crate::model::{Capability, Mode};
    let ch = |key: &str, name: &str, group: &str, default: u8| ChannelDef {
        key: key.into(),
        name: name.into(),
        group: group.into(),
        default,
        capabilities: vec![Capability { min: 0, max: 255, label: format!("{name} 0–100%"), ..Default::default() }],
        values: vec![],
    };
    let mk = |id: &str, model: &str, chans: Vec<ChannelDef>| Profile {
        id: id.into(),
        manufacturer: "Zigbee".into(),
        model: model.into(),
        kind: "zigbee".into(),
        modes: vec![Mode { name: "Default".into(), channels: chans.iter().map(|c| c.key.clone()).collect(), heads: vec![] }],
        channels: chans,
        notes: "Built-in profile for Zigbee lights.".into(),
        looks: vec![],
        look_sets: vec![],
    };
    let mut ct = ch("color_temp", "Colour temperature", "color", 128);
    ct.capabilities[0].label = "Warm (0) to cool (255)".into();
    vec![
        mk(
            "zigbee-rgb",
            "Colour light",
            vec![
                ch("dimmer", "Brightness", "intensity", 0),
                ch("red", "Red", "color", 255),
                ch("green", "Green", "color", 255),
                ch("blue", "Blue", "color", 255),
            ],
        ),
        mk("zigbee-white", "Tunable white light", vec![ch("dimmer", "Brightness", "intensity", 0), ct]),
        mk("zigbee-dimmer", "Dimmable light", vec![ch("dimmer", "Brightness", "intensity", 0)]),
        mk("zigbee-switch", "On/off plug", vec![ch("dimmer", "On (above 50%)", "intensity", 0)]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_rgb_profile() {
        let p = &builtin_profiles()[0];
        let chans: Vec<&ChannelDef> = p.channels.iter().collect();
        let d = desired_from(p, &chans, &[200, 255, 0, 10]);
        assert!(d.on);
        assert_eq!(d.brightness, 200);
        assert_eq!(d.rgb, Some([255, 0, 10]));
        assert!(!desired_from(p, &chans, &[0, 255, 0, 0]).on);
        assert!(!desired_from(p, &chans, &[255, 0, 0, 0]).on);
    }

    #[test]
    fn maps_colour_temp_and_payload() {
        let p = &builtin_profiles()[1];
        let chans: Vec<&ChannelDef> = p.channels.iter().collect();
        let d = desired_from(p, &chans, &[255, 255]);
        assert_eq!(d.color_temp, Some(153));
        let pl = payload(&d, None, 0.0);
        assert_eq!(pl["state"], "ON");
        assert_eq!(pl["color_temp"], 153);
        assert!(pl.get("transition").is_none());
        // brightness-only change doesn't resend colour
        let d2 = Desired { brightness: 100, ..d };
        assert!(payload(&d2, Some(&d), 0.0).get("color_temp").is_none());
    }

    #[test]
    fn deadband_and_priority() {
        let a = Desired { on: true, brightness: 100, rgb: Some([10, 10, 10]), color_temp: None };
        assert_eq!(difference(&a, &Desired { brightness: 102, ..a }), 0.0);
        assert!(difference(&a, &Desired { on: false, ..a }) > difference(&a, &Desired { brightness: 250, ..a }));
    }

    /// Runs the real Node helper through the supervisor (no dongle needed).
    #[test]
    fn usb_helper_round_trip() {
        let helper = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zigbee-helper");
        if Zigbee::find_node(&helper).is_none() || !helper.join("node_modules").exists() {
            eprintln!("node or helper dependencies not installed; skipping");
            return;
        }
        let data = std::env::temp_dir().join(format!("lightdeck-zb-{}", std::process::id()));
        let s = ZigbeeSettings { enabled: true, mode: "usb".into(), ..Default::default() };
        let z = Zigbee::start(s, data.clone(), helper);
        let deadline = Instant::now() + Duration::from_secs(15);
        while z.status().state != "Choose the dongle's port" {
            assert!(Instant::now() < deadline, "helper never came up: {:?}", z.status());
            std::thread::sleep(Duration::from_millis(100));
        }
        let ports = z.ports().expect("ports");
        assert!(ports.iter().all(|p| !p.path.contains("Bluetooth")));
        // Commands that need a running network fail cleanly instead of hanging.
        assert!(z.permit_join(10).unwrap_err().contains("not running"));
        // Turning it off stops the helper.
        z.configure(&ZigbeeSettings { enabled: false, ..Default::default() });
        let deadline = Instant::now() + Duration::from_secs(10);
        while z.helper_alive() {
            assert!(Instant::now() < deadline, "helper didn't stop");
            std::thread::sleep(Duration::from_millis(100));
        }
        std::fs::remove_dir_all(data).ok();
    }

    #[test]
    fn parses_bridge_devices() {
        let v: Value = serde_json::from_str(r#"[
          {"type":"Coordinator","friendly_name":"Coordinator"},
          {"type":"Router","friendly_name":"Desk lamp","ieee_address":"0x01","definition":{"vendor":"IKEA","model":"LED1","description":"bulb",
            "exposes":[{"type":"light","features":[{"name":"state"},{"name":"brightness"},{"name":"color_xy"},{"name":"color_temp"}]}]}},
          {"type":"Router","friendly_name":"Plug","ieee_address":"0x02","definition":{"exposes":[{"type":"switch","features":[{"name":"state"}]}]}}
        ]"#).unwrap();
        let d = parse_devices(&v);
        assert_eq!(d.len(), 2);
        assert!(d[0].color && d[0].brightness && d[0].color_temp);
        assert!(d[1].switch_only);
    }
}
