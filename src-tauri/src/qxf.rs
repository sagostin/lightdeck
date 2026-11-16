//! QLC+ fixture definition (.qxf) importer -> LightDeck Profile.

use crate::model::{Capability, ChannelDef, Head, Mode, Profile, TAG_BLOCKED};

pub fn import(xml: &str) -> Result<Profile, String> {
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(xml, opts).map_err(|e| format!("Not a valid .qxf file: {e}"))?;
    let root = doc.root_element();
    let text = |name: &str| {
        root.children()
            .find(|n| n.has_tag_name(name))
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let manufacturer = text("Manufacturer");
    let model = text("Model");
    let qtype = text("Type");

    let mut channels = Vec::new();
    for ch in root.children().filter(|n| n.has_tag_name("Channel")) {
        let name = ch.attribute("Name").unwrap_or("Channel").to_string();
        let qgroup = ch
            .children()
            .find(|n| n.has_tag_name("Group"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();
        let preset = ch.attribute("Preset").unwrap_or("");
        let mut caps = Vec::new();
        for cap in ch.children().filter(|n| n.has_tag_name("Capability")) {
            let min = cap.attribute("Min").and_then(|v| v.parse().ok()).unwrap_or(0);
            let max = cap.attribute("Max").and_then(|v| v.parse().ok()).unwrap_or(255);
            let label = cap.text().unwrap_or("").trim().to_string();
            let color = cap
                .attribute("Res1")
                .or(cap.attribute("Res"))
                .filter(|r| r.starts_with('#'))
                .map(String::from);
            let mut tags = Vec::new();
            if cap.attribute("Preset") == Some("LampOff") {
                tags.push("off".into());
            }
            caps.push(Capability { min, max, label, color, tags });
        }
        let group = map_group(&qgroup, preset, &name);
        auto_tag(&group, &mut caps);
        channels.push(ChannelDef {
            key: slug(&name),
            name,
            group,
            default: 0,
            capabilities: caps,
            values: vec![],
        });
    }
    dedupe_keys(&mut channels);

    let mut modes = Vec::new();
    for m in root.children().filter(|n| n.has_tag_name("Mode")) {
        let mut ordered: Vec<(usize, String)> = m
            .children()
            .filter(|n| n.has_tag_name("Channel"))
            .filter_map(|n| {
                let num = n.attribute("Number")?.parse().ok()?;
                let name = n.text()?.trim();
                let key = channels.iter().find(|c| c.name == name)?.key.clone();
                Some((num, key))
            })
            .collect();
        ordered.sort_by_key(|(n, _)| *n);
        let chans: Vec<String> = ordered.into_iter().map(|(_, k)| k).collect();
        let heads = m
            .children()
            .filter(|n| n.has_tag_name("Head"))
            .enumerate()
            .map(|(i, h)| Head {
                name: format!("P{}", i + 1),
                channels: h
                    .children()
                    .filter(|n| n.has_tag_name("Channel"))
                    .filter_map(|n| n.text()?.trim().parse::<usize>().ok())
                    .filter_map(|idx| chans.get(idx).cloned())
                    .collect(),
            })
            .collect();
        modes.push(Mode { name: m.attribute("Name").unwrap_or("Default").to_string(), channels: chans, heads });
    }
    if modes.is_empty() {
        modes.push(Mode { name: "Default".into(), channels: channels.iter().map(|c| c.key.clone()).collect(), heads: vec![] });
    }

    Ok(Profile {
        id: format!("{}-{}", slug(&manufacturer), slug(&model)),
        manufacturer,
        model,
        kind: map_kind(&qtype),
        channels,
        modes,
        notes: format!("Imported from QLC+ ({qtype})"),
        looks: vec![],
        look_sets: vec![],
    })
}

fn map_kind(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "laser" => "laser",
        "smoke" | "hazer" => "fog",
        "color changer" | "led bar (pixels)" | "led bar (beams)" => "par",
        "moving head" | "scanner" => "moving_head",
        "strobe" => "strobe",
        _ => "other",
    }
    .into()
}

fn map_group(g: &str, preset: &str, name: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("mode") && g == "Maintenance" {
        return "mode".into();
    }
    if n.contains("fog") || n.contains("smoke") {
        return "fog".into();
    }
    if preset.starts_with("Intensity") {
        return "intensity".into();
    }
    match g {
        "Intensity" => "intensity",
        "Colour" => "color",
        "Pan" => "pan",
        "Tilt" => "tilt",
        "Gobo" => "pattern",
        "Beam" => "beam",
        "Speed" => "speed",
        "Shutter" => "strobe",
        "Maintenance" => "mode",
        "Nothing" => "other",
        _ => "effect",
    }
    .into()
}

/// Heuristic tags so a fresh import is immediately useful; refine in the UI or with AI.
fn auto_tag(group: &str, caps: &mut [Capability]) {
    for c in caps.iter_mut() {
        let l = c.label.to_lowercase();
        if group == "mode" && (l.contains("auto") || l.contains("sound")) {
            push(&mut c.tags, TAG_BLOCKED);
        }
        if l.contains("undefined") || l.contains("reserved") {
            push(&mut c.tags, TAG_BLOCKED);
        }
        if l.starts_with("dynamic") || l.contains("chase") || l.contains("auto drawing") {
            push(&mut c.tags, "dynamic");
        }
        if l.starts_with("static") {
            push(&mut c.tags, "static");
        }
        if l.contains("strobe") {
            push(&mut c.tags, "strobe");
        }
    }
}

fn push(tags: &mut Vec<String>, t: &str) {
    if !tags.iter().any(|x| x == t) {
        tags.push(t.into());
    }
}

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

fn dedupe_keys(chs: &mut [ChannelDef]) {
    let mut seen = std::collections::HashMap::<String, u32>::new();
    for c in chs.iter_mut() {
        let n = seen.entry(c.key.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            c.key = format!("{}_{}", c.key, n);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn imports_real_fixtures() {
        let laser = super::import(include_str!("../../fixtures/Generic-3D-Animation-RGB-Laser-L2600-005.qxf")).unwrap();
        let m = &laser.modes[0];
        assert_eq!(m.channels.len(), 32);
        assert_eq!(m.heads.len(), 2);
        assert_eq!(m.heads[0].channels.len(), 16);
        assert_eq!(m.heads[1].channels[4], "p2_pattern");
        let mode = laser.channel("mode").unwrap();
        assert!(mode.value_blocked(210) && mode.value_blocked(150) && !mode.value_blocked(50));
        let fog = super::import(include_str!("../../fixtures/dmxbridge.qxf")).unwrap();
        assert_eq!(fog.kind, "fog");
        assert_eq!(fog.channel("fog").unwrap().group, "fog");
    }

    #[test]
    fn imports_heads() {
        let xml = r#"<?xml version="1.0"?><FixtureDefinition xmlns="http://www.qlcplus.org/FixtureDefinition">
        <Manufacturer>Generic</Manufacturer><Model>L</Model><Type>Laser</Type>
        <Channel Name="Mode"><Group Byte="0">Maintenance</Group>
          <Capability Min="0" Max="99">DMX manual</Capability><Capability Min="100" Max="255">Sound-active program</Capability></Channel>
        <Channel Name="P1 Pattern"><Group Byte="0">Gobo</Group><Capability Min="0" Max="255">Pattern</Capability></Channel>
        <Channel Name="P2 Pattern"><Group Byte="0">Gobo</Group><Capability Min="0" Max="255">Pattern</Capability></Channel>
        <Mode Name="3ch"><Channel Number="0">Mode</Channel><Channel Number="1">P1 Pattern</Channel><Channel Number="2">P2 Pattern</Channel>
          <Head><Channel>1</Channel></Head><Head><Channel>2</Channel></Head></Mode></FixtureDefinition>"#;
        let p = super::import(xml).unwrap();
        assert_eq!(p.kind, "laser");
        assert_eq!(p.modes[0].channels, vec!["mode", "p1_pattern", "p2_pattern"]);
        assert_eq!(p.modes[0].heads[1].channels, vec!["p2_pattern"]);
        assert!(p.channels[0].value_blocked(150));
        assert!(!p.channels[0].value_blocked(50));
    }
}
