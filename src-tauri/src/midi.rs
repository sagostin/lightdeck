//! MIDI input: watches for controllers, connects to the selected ones (or all),
//! reconnects on hot-plug, and turns raw messages into `MidiEvent`s.
//! MIDI output: lights pads on the same controllers (see `feedback`).

use crate::feedback::{self, Led};
use crate::model::MidiSettings;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::collections::HashMap;
use serde::Serialize;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MidiEvent {
    pub port: String,
    /// "note" | "cc" | "program"
    pub kind: String,
    /// 1-16
    pub channel: u8,
    pub number: u8,
    /// Note velocity / CC value / 0 for program change. Note-off reports 0.
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MidiStatus {
    pub enabled: bool,
    pub available: Vec<String>,
    pub connected: Vec<String>,
    /// Ports pads are lit on, with the style used.
    pub feedback: Vec<(String, String)>,
    pub error: Option<String>,
}

/// An output to a controller, with what its pads show now.
struct Output {
    name: String,
    style: &'static str,
    conn: MidiOutputConnection,
    sent: HashMap<(bool, u8, u8), Vec<u8>>,
}

pub struct Midi {
    settings: Mutex<MidiSettings>,
    status: Mutex<MidiStatus>,
    outputs: Mutex<Vec<Output>>,
}

/// Parse one MIDI message. Running status isn't used by midir callbacks (each call is one message).
pub fn parse(port: &str, msg: &[u8]) -> Option<MidiEvent> {
    let status = *msg.first()?;
    let channel = (status & 0x0f) + 1;
    let ev = |kind: &str, number: u8, value: u8| MidiEvent { port: port.into(), kind: kind.into(), channel, number, value };
    match status & 0xf0 {
        0x90 => Some(ev("note", *msg.get(1)?, *msg.get(2)?)), // velocity 0 = note off
        0x80 => Some(ev("note", *msg.get(1)?, 0)),
        0xb0 => Some(ev("cc", *msg.get(1)?, *msg.get(2)?)),
        0xc0 => Some(ev("program", *msg.get(1)?, 0)),
        _ => None,
    }
}

impl Midi {
    pub fn start(settings: MidiSettings, tx: Sender<MidiEvent>) -> Arc<Self> {
        let m = Arc::new(Self { settings: Mutex::new(settings), status: Mutex::new(MidiStatus::default()), outputs: Mutex::new(vec![]) });
        let me = m.clone();
        std::thread::Builder::new().name("midi".into()).spawn(move || me.run(tx)).unwrap();
        m
    }

    pub fn configure(&self, s: &MidiSettings) {
        *self.settings.lock().unwrap() = s.clone();
    }

    pub fn status(&self) -> MidiStatus {
        self.status.lock().unwrap().clone()
    }

    /// Light pads: send what changed since last time to every feedback output. Pads that
    /// no longer have a control are switched off.
    pub fn send_leds(&self, leds: &[Led]) {
        let mut outs = self.outputs.lock().unwrap();
        for o in outs.iter_mut() {
            let mut want: HashMap<(bool, u8, u8), Vec<u8>> = leds.iter().map(|l| (l.key(), feedback::encode(o.style, l))).collect();
            for (k, msg) in &o.sent {
                if !want.contains_key(k) {
                    let off = Led { kind: if k.0 { "cc" } else { "note" }.into(), channel: k.1, number: k.2, lit: false, color: [0, 0, 0] };
                    let _ = o.conn.send(&feedback::encode(o.style, &off));
                    let _ = msg;
                }
            }
            o.sent.retain(|k, _| want.contains_key(k));
            for (k, msg) in want.drain() {
                if o.sent.get(&k) != Some(&msg) && o.conn.send(&msg).is_ok() {
                    o.sent.insert(k, msg);
                }
            }
        }
    }

    /// Connect outputs to the controllers we listen to, when feedback is on.
    fn connect_outputs(&self, s: &MidiSettings, inputs: &[String]) -> Vec<(String, String)> {
        let mut outs = self.outputs.lock().unwrap();
        let style = |name: &str| if s.feedback && s.enabled { feedback::style_for(name, &s.feedback_style) } else { None };
        let names: Vec<String> = MidiOutput::new("LightDeck")
            .map(|mo| mo.ports().iter().filter_map(|p| mo.port_name(p).ok()).collect())
            .unwrap_or_default();
        // Drop outputs that are gone, no longer listened to, or changed style.
        outs.retain(|o| names.contains(&o.name) && inputs.contains(&o.name) && style(&o.name) == Some(o.style));
        for name in inputs.iter().filter(|n| names.contains(n)) {
            let Some(st) = style(name) else { continue };
            if outs.iter().any(|o| &o.name == name) {
                continue;
            }
            let Ok(mo) = MidiOutput::new("LightDeck") else { continue };
            let Some(port) = mo.ports().into_iter().find(|p| mo.port_name(p).ok().as_ref() == Some(name)) else { continue };
            if let Ok(mut conn) = mo.connect(&port, "lightdeck-out") {
                for msg in feedback::init_messages(name, st) {
                    let _ = conn.send(&msg);
                }
                outs.push(Output { name: name.clone(), style: st, conn, sent: HashMap::new() });
            }
        }
        outs.iter().map(|o| (o.name.clone(), o.style.to_string())).collect()
    }

    fn run(&self, tx: Sender<MidiEvent>) {
        let mut conns: Vec<(String, MidiInputConnection<()>)> = vec![];
        loop {
            let s = self.settings.lock().unwrap().clone();
            let available: Vec<String> = match MidiInput::new("LightDeck") {
                Ok(mi) => mi.ports().iter().filter_map(|p| mi.port_name(p).ok()).collect(),
                Err(e) => {
                    self.status.lock().unwrap().error = Some(e.to_string());
                    vec![]
                }
            };
            let want = |name: &String| s.enabled && (s.inputs.is_empty() || s.inputs.contains(name));
            // drop connections that are gone or no longer wanted
            conns.retain(|(name, _)| available.contains(name) && want(name));
            let mut error = None;
            for name in available.iter().filter(|n| want(n)) {
                if conns.iter().any(|(c, _)| c == name) {
                    continue;
                }
                let Ok(mi) = MidiInput::new("LightDeck") else { continue };
                let Some(port) = mi.ports().into_iter().find(|p| mi.port_name(p).ok().as_ref() == Some(name)) else { continue };
                let tx = tx.clone();
                let pname = name.clone();
                match mi.connect(&port, "lightdeck-in", move |_, msg, _| {
                    if let Some(ev) = parse(&pname, msg) {
                        let _ = tx.send(ev);
                    }
                }, ()) {
                    Ok(c) => conns.push((name.clone(), c)),
                    Err(e) => error = Some(format!("{name}: {e}")),
                }
            }
            let connected: Vec<String> = conns.iter().map(|(n, _)| n.clone()).collect();
            let fb = self.connect_outputs(&s, &connected);
            {
                let mut st = self.status.lock().unwrap();
                st.enabled = s.enabled;
                st.available = available;
                st.connected = connected;
                st.feedback = fb;
                st.error = error;
            }
            std::thread::sleep(Duration::from_millis(1500));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn parses_messages() {
        let n = parse("p", &[0x91, 36, 100]).unwrap();
        assert_eq!((n.kind.as_str(), n.channel, n.number, n.value), ("note", 2, 36, 100));
        assert_eq!(parse("p", &[0x80, 36, 64]).unwrap().value, 0); // note off
        let c = parse("p", &[0xb0, 7, 127]).unwrap();
        assert_eq!((c.kind.as_str(), c.channel, c.number, c.value), ("cc", 1, 7, 127));
        assert_eq!(parse("p", &[0xcf, 5]).unwrap().channel, 16);
        assert!(parse("p", &[0xf8]).is_none()); // clock ignored
    }
}
