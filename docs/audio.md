# Audio detection

Open **Audio** to choose the input and tune what LightDeck listens for.

## Input

- **Device**: *System default* or a specific input. Click **Refresh** after plugging something in, or **Restart** if input stops.
- **Input gain**: adjust until the level meter peaks around three quarters on loud parts.
- **Global sensitivity**: nudges the hit sensitivity of every "sudden rises" band at once. 50% is neutral.

The status line shows the device, sample rate and current BPM.

### Reacting to music on this Mac

macOS doesn't let apps record other apps' audio directly. Install a free loopback driver such as **BlackHole**, create a Multi-Output Device in *Audio MIDI Setup* that sends to both your speakers and BlackHole, set it as the system output, then pick **BlackHole** as LightDeck's input.

## Detection bands

Each band listens to part of the frequency spectrum and produces two **sources** for routes:

- `<band>` (shown as *Kick hit*, etc.): fires on each detected hit.
- `<band>.level` (shown as *Kick level*): a smooth 0–1 level for modulation.

The defaults are:

| Band | Ranges | Hits on | Best used as |
|---|---|---|---|
| Kick | 35–150 Hz | Sudden rises | Hits. Also drives the BPM and beat clock. |
| Bass | 60–250 Hz | Level crossing | Level |
| Snare | 150–250 Hz + 1.8–5 kHz | Sudden rises | Hits |
| Vocals | 300 Hz–3 kHz | Level crossing | Level |
| Hi-hat | 6–16 kHz | Sudden rises | Hits or level |

**The first band drives the BPM and beat clock**, so keep Kick first. Use the ‹ › buttons on a band card to reorder.

### Editing bands on the spectrum

The spectrum shows live audio from 20 Hz to 20 kHz with each band's ranges drawn over it.

- Drag a range to move it; drag its edges to resize.
- The line inside a range shows the band's current level.
- Select a band to see its **trace** below: the detector value (solid) against the trigger threshold (dashed), with a mark at each hit. Tune until the marks land on the drum you want.

### Band settings

| Setting | Notes |
|---|---|
| Ranges | One or more frequency ranges, each with a **weight**. **+ Add range** adds another (Snare uses its body plus its crack). |
| On / Muted | Muted bands report no level and no hits. You can also mute bands from MIDI or a key. |
| Hits on | **Sudden rises** detects transients (drums). **Level crossing** fires when the level passes a line you set (bass swells, vocals coming in). |
| Band gain | Boosts or cuts this band only |
| Hit sensitivity / Hit when level passes | Sudden-rise bands: how far above its recent average the band must jump. Level bands: the threshold line. |
| Min gap between hits | Stops double triggers |
| Level release | How quickly the level falls after a peak (higher is slower and smoother) |

### Adding bands

**Add band…** offers presets (Sub-bass, Kick, Bass, Snare, Clap, Vocals, Lead synth, Hi-hat, Cymbals and air), your own saved presets, or a blank custom band.

**Save as preset** on a band card saves it for every project. Your presets appear in the Input panel, where you can delete them, and they're included when you [export settings](projects-and-settings.md#app-settings).

### Tuning for a genre

**Tune for a genre…** under Global sensitivity retunes the bands for a style of music: house, techno, trance, drum & bass, dubstep, hip-hop, reggaeton, pop, rock or ambient. It changes sensitivity and timing, and adds a band where the genre needs one (a Clap band for house, Sub-bass for dubstep). See [Genre presets](designer.md#genre-presets) for what each genre changes.

## Other sources

Routes can also use:

| Source | Kind |
|---|---|
| Beat | Hit on every beat |
| Bar (4 beats) | Hit every fourth beat |
| Overall level | Level |
| Energy (build/drop) | Level: short-term loudness against the long-term average, high on drops |
| Beat phase | Level: ramps once per beat |

## Tips

- Bands may overlap. Kick and Bass share the low end: Kick watches for sharp hits, Bass for its level.
- If Snare fires on vocals, narrow it to its crack (about 1.8–5 kHz) or lower its sensitivity.
- If the BPM doubles or halves, check the Kick band is catching only kicks.
