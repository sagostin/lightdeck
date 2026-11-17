//! MIDI feedback: turns "this pad is on, in this colour" into the messages a controller
//! needs to light it. Controllers differ, so each port gets a style, worked out from its
//! name unless you pick one.

/// One pad to light: the trigger it's mapped to, whether its action is on, and a colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Led {
    /// "note" | "cc"
    pub kind: String,
    /// MIDI channel 1-16 (0 = any, sent on channel 1).
    pub channel: u8,
    pub number: u8,
    pub lit: bool,
    pub color: [u8; 3],
}

impl Led {
    pub fn key(&self) -> (bool, u8, u8) {
        (self.kind == "cc", self.channel, self.number)
    }
}

/// Styles you can pick, with names for the UI.
pub const STYLES: [(&str, &str); 6] = [
    ("generic", "On/off (velocity or CC 127)"),
    ("apc_mini_mk2", "Akai APC mini mk2 (RGB)"),
    ("apc_mini", "Akai APC mini (green/red/yellow)"),
    ("launchpad", "Novation Launchpad (RGB palette)"),
    ("nanokontrol2", "Korg nanoKONTROL2 (LED mode External)"),
    ("off", "Off"),
];

/// The style for a port: `chosen` unless empty, else from the port name. None = don't send.
pub fn style_for(port: &str, chosen: &str) -> Option<&'static str> {
    let n = port.to_lowercase();
    // Second ports that never take LED messages.
    if n.contains("daw") || (n.contains("apc mini mk2") && n.contains("notes")) {
        return None;
    }
    if !chosen.is_empty() {
        return STYLES.iter().map(|(s, _)| *s).find(|s| *s == chosen && *s != "off");
    }
    Some(if n.contains("apc mini mk2") || n.contains("apc mini mk 2") {
        "apc_mini_mk2"
    } else if n.contains("apc mini") || n.contains("apc_mini") {
        "apc_mini"
    } else if n.contains("launchpad") || n.contains("lpx") || n.contains("lpmini") || n.contains("lppro") {
        "launchpad"
    } else if n.contains("nanokontrol2") {
        "nanokontrol2"
    } else {
        "generic"
    })
}

/// Messages to send once on connecting, e.g. putting a Launchpad into Programmer mode.
pub fn init_messages(port: &str, style: &str) -> Vec<Vec<u8>> {
    if style != "launchpad" {
        return vec![];
    }
    let n = port.to_lowercase();
    let device = if n.contains("lpx") || n.contains("launchpad x") {
        0x0c
    } else if n.contains("lpmini") || n.contains("launchpad mini") {
        0x0d
    } else {
        return vec![];
    };
    vec![vec![0xf0, 0x00, 0x20, 0x29, 0x02, device, 0x0e, 0x01, 0xf7]]
}

/// Messages that show `led` in `style`.
pub fn encode(style: &str, led: &Led) -> Vec<u8> {
    let ch = led.channel.saturating_sub(1).min(15);
    let cc = led.kind == "cc";
    let n = led.number.min(127);
    let on_off = |lit: bool| vec![if cc { 0xb0 } else { 0x90 } | ch, n, if lit { 127 } else { 0 }];
    match style {
        // Grid pads take a palette colour; the MIDI channel sets brightness (1 = 10%, 7 = 100%).
        "apc_mini_mk2" if !cc && n < 64 => {
            let (c, _) = palette(led.color);
            vec![if led.lit { 0x96 } else { 0x90 }, n, c]
        }
        // Track and scene buttons have one colour: 1 = on.
        "apc_mini_mk2" if !cc && ((100..=107).contains(&n) || (112..=119).contains(&n)) => vec![0x90, n, led.lit as u8],
        "apc_mini" if !cc && n < 64 => vec![0x90, n, if led.lit { apc_colour(led.color) } else { 0 }],
        "apc_mini" if !cc && ((64..=71).contains(&n) || (82..=89).contains(&n)) => vec![0x90, n, led.lit as u8],
        // Programmer mode: velocity is a palette colour on channel 1. Unlit pads show a dim version.
        "launchpad" => {
            let (bright, dim) = palette(led.color);
            vec![if cc { 0xb0 } else { 0x90 }, n, if led.lit { bright } else { dim }]
        }
        _ => on_off(led.lit),
    }
}

/// "#rrggbb" → RGB (purple if unreadable).
pub fn parse_hex(s: &str) -> [u8; 3] {
    let h = s.trim().trim_start_matches('#');
    let v = |i: usize| h.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok());
    match (h.len(), v(0), v(2), v(4)) {
        (6, Some(r), Some(g), Some(b)) => [r, g, b],
        _ => [0x7c, 0x5c, 0xff],
    }
}

fn hue_sat(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|x| x as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max > 0.0 { d / max } else { 0.0 };
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s, max)
}

/// The nearest colour in the 128-colour palette Novation Launchpads and the APC mini mk2
/// share: (full, dim). Colours come in fours from 4 (light, full, dim, darkest).
pub fn palette(c: [u8; 3]) -> (u8, u8) {
    let (h, s, v) = hue_sat(c);
    if v < 0.08 {
        return (0, 0);
    }
    if s < 0.25 {
        return (3, 1); // white, dark grey
    }
    let full = match h {
        h if h < 15.0 => 5,   // red
        h if h < 40.0 => 9,   // orange
        h if h < 65.0 => 13,  // yellow
        h if h < 95.0 => 17,  // lime
        h if h < 150.0 => 21, // green
        h if h < 185.0 => 33, // aqua
        h if h < 215.0 => 37, // sky blue
        h if h < 245.0 => 45, // blue
        h if h < 275.0 => 49, // violet
        h if h < 315.0 => 53, // magenta
        h if h < 345.0 => 57, // pink
        _ => 5,
    };
    (full, full + 1)
}

/// Original APC mini: 1 green, 3 red, 5 yellow.
fn apc_colour(c: [u8; 3]) -> u8 {
    let (h, s, _) = hue_sat(c);
    if s < 0.25 {
        return 5;
    }
    match h {
        h if !(20.0..340.0).contains(&h) => 3,
        h if h < 70.0 => 5,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn led(kind: &str, number: u8, lit: bool, color: &str) -> Led {
        Led { kind: kind.into(), channel: 0, number, lit, color: parse_hex(color) }
    }

    #[test]
    fn colours_map_to_the_palette() {
        assert_eq!(palette(parse_hex("#ff0000")), (5, 6));
        assert_eq!(palette(parse_hex("#0000ff")), (45, 46));
        assert_eq!(palette(parse_hex("#00ff00")), (21, 22));
        assert_eq!(palette(parse_hex("#ffffff")), (3, 1));
        assert_eq!(palette(parse_hex("#000000")), (0, 0));
        assert_eq!(palette(parse_hex("#7c5cff")).0, 49);
        assert_eq!(palette(parse_hex("#ff8a00")).0, 9);
        assert_eq!(parse_hex("nope"), [0x7c, 0x5c, 0xff]);
    }

    #[test]
    fn styles_from_port_names() {
        assert_eq!(style_for("APC mini mk2 Control", ""), Some("apc_mini_mk2"));
        assert_eq!(style_for("APC mini mk2 Notes", ""), None);
        assert_eq!(style_for("APC MINI", ""), Some("apc_mini"));
        assert_eq!(style_for("LPX MIDI", ""), Some("launchpad"));
        assert_eq!(style_for("LPX DAW", ""), None);
        assert_eq!(style_for("nanoKONTROL2 SLIDER/KNOB", ""), Some("nanokontrol2"));
        assert_eq!(style_for("MPD218 Port A", ""), Some("generic"));
        assert_eq!(style_for("MPD218 Port A", "launchpad"), Some("launchpad"));
        assert_eq!(style_for("APC MINI", "off"), None);
        assert_eq!(init_messages("LPX MIDI", "launchpad")[0][5], 0x0c);
        assert!(init_messages("MPD218", "generic").is_empty());
    }

    #[test]
    fn messages_per_style() {
        let red = led("note", 56, true, "#ff0000");
        assert_eq!(encode("apc_mini_mk2", &red), vec![0x96, 56, 5]);
        assert_eq!(encode("apc_mini_mk2", &Led { lit: false, ..red.clone() }), vec![0x90, 56, 5], "dim, not off");
        assert_eq!(encode("apc_mini_mk2", &led("note", 112, true, "#ff0000")), vec![0x90, 112, 1]);
        assert_eq!(encode("apc_mini", &red), vec![0x90, 56, 3]);
        assert_eq!(encode("apc_mini", &Led { lit: false, ..red.clone() }), vec![0x90, 56, 0]);
        assert_eq!(encode("launchpad", &Led { lit: false, ..red.clone() }), vec![0x90, 56, 6]);
        assert_eq!(encode("launchpad", &led("cc", 19, true, "#0000ff")), vec![0xb0, 19, 45]);
        assert_eq!(encode("generic", &red), vec![0x90, 56, 127]);
        assert_eq!(encode("nanokontrol2", &led("cc", 32, false, "#fff")), vec![0xb0, 32, 0]);
        let ch3 = Led { channel: 3, ..red };
        assert_eq!(encode("generic", &ch3), vec![0x92, 56, 127]);
    }
}
