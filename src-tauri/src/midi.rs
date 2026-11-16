//! MIDI input: watches for controllers, connects to the selected ones (or all),
//! reconnects on hot-plug, and turns raw messages into `MidiEvent`s.

use crate::model::MidiSettings;
use midir::{MidiInput, MidiInputConnection};
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
    pub error: Option<String>,
}

pub struct Midi {
    settings: Mutex<MidiSettings>,
    status: Mutex<MidiStatus>,
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
        let m = Arc::new(Self { settings: Mutex::new(settings), status: Mutex::new(MidiStatus::default()) });
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
            {
                let mut st = self.status.lock().unwrap();
                st.enabled = s.enabled;
                st.available = available;
                st.connected = conns.iter().map(|(n, _)| n.clone()).collect();
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
