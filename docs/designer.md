# Show designer

The **Design** screen builds a show for the rig you've set up, in one of two ways:

- **Templates** (no AI) build ready-made scene layouts from your rig with fixed rules. They work offline and give the same result every time.
- **AI designer** writes a design from your own description.

Both give you a design to review, untick, add and undo in the same way. Everything they add is ordinary routes, scenes and controls that you can edit afterwards, or build by hand instead (see [Live control](control.md#scenes-and-presets)).

## Templates

Pick a template, optionally a genre and a MIDI controller, and click **Build setup**.

| Template | Scenes |
|---|---|
| Club night | Warm-up (slow wave, hi-hat sparkle), Build (colour each bar, energy lifts brightness), Drop (kick colour hits, snare strobe, peak laser looks, fog burst), Breakdown (lights breathe with vocals, drifting colour) |
| Party | Groove (kick colour steps, sparkle), Peak (fast rings, node hop, strobe, fog) |
| Chill / lounge | Glow and Drift: long fades and slow waves; no strobe or fog |
| Test my rig | One scene: every group flashes on the kick, colour changes each bar, lasers change look. For checking patching and detection |

Templates read the rig:

- **Colour fixtures** are fixtures with brightness or colour channels, apart from lasers and fog. They're targeted with your fixture groups where those fit, and call-and-response effects use two of your groups as halves.
- **Lasers** are fixtures with saved looks. Templates use looks tagged `chill` or `peak` when you have them, otherwise any look.
- **Fog** gets short bursts on the drop, at most every 45 seconds.
- **Focus points** make waves and colour hits ripple out from the first focus point.
- **Genre** tunes detection and picks the palettes. Drum & bass gets a slower strobe, and Ambient removes strobe entirely.
- **MIDI controller** maps the scenes, strobe, fog, master, group brightness and band sensitivity to the controller (see [Controller setup](control.md#controller-setup)).

The design summary says what would improve the result, for example tagging laser looks or adding a focus point.

## AI designer

Describe the night, pick a genre and your MIDI controller, and the AI proposes:

- **Audio detection tuning:** band sensitivity, timing and extra bands for the music.
- **Colour palettes** for the mood.
- **Routes grouped into scenes** (for example Warm-up, Build, Drop, Breakdown). Each scene is a route group, so the Routes screen shows it as one folder you can switch on and off.
- **Scene presets** that switch between those scenes.
- **MIDI and keyboard mappings:** pads for scenes, a held strobe, faders for master and band sensitivity, and so on.

Nothing changes until you click **Add to show**. You review the design first and untick anything you don't want.

It needs an AI provider. Set one up on the **AI** screen (see [Fixture types](fixtures.md) for the same setup). Without one, the Design screen opens on Templates.

## What the AI knows about your rig

The AI receives a description of the show and never the audio:

- Fixtures with their channel types and positions, fixture groups, rooms and focus points.
- Each fixture type's saved looks: tags, look sets and your rated looks.
- Detection bands, palettes, and existing routes, presets and controls.

The better that information is, the better the design. Before generating:

- **Name your groups** for how you'd talk about them ("Front pars", "Back wash", "Laser").
- **Add focus points**, such as the dance floor or DJ booth, so effects can ripple out from them.
- **Tag your laser looks** (`chill`, `peak`, `static`…) in the pattern browser. Look routes pick looks by tag.
- **Mark blocked values** on fixture types. [Limits](rooms-and-patch.md) still apply to everything the AI makes.

## Generating

1. Write what you want in **What do you want?**, or start from one of the suggestions.
2. Pick a **Genre**. It tunes detection (see below) and tells the AI how that music is usually lit. "Any / mixed" leaves both to your brief.
3. Choose what to **Generate**.
4. Under **MIDI controller**, name the model, such as "Akai APC mini mk2" or "Novation Launchpad X". The AI uses the controller's factory note and CC layout and explains the layout in the design summary. Leave it empty to use notes from 36 and CCs from 1. **Also map keyboard keys** adds keys for the main scenes. B is never used, because it's blackout.
5. Click **Generate design**. Large designs can take a few minutes.

## Reviewing and adding

The design lists its sections with a checkbox per item, and clicking a section heading ticks or unticks the whole section.

LightDeck checks every reference before showing the design:

- Unknown fixtures, groups and palettes are fixed or removed.
- Routes that wouldn't reach any fixture, such as a pan route on fixtures without pan, are skipped.
- Duplicate triggers are dropped.

Anything fixed or skipped is listed under **things were fixed or skipped**.

**Add to show** adds what's ticked:

- Routes, presets and controls are added after your own.
- Retuned bands replace those bands' settings. Their ids stay the same, so your existing routes keep working.
- MIDI input is switched on if the design maps MIDI.
- Only the starting scene's routes are on. Use the scene presets, or your controller, to move between scenes.

**Replace earlier generated designs** first removes routes, presets and controls from previous designs (templates or AI). Your own work is never removed. Generated items have ids starting with `ai-`, so they're kept apart even after you edit them.

**Undo apply** puts the show back as it was before the last **Add to show**.

## Refining (AI)

Type a change under the design, such as "fewer strobes", "add a fog button" or "use pads 56–63 for scenes", and click **Refine**. The AI revises the current design and keeps the parts you didn't mention. Review it again, then add it.

## Genre presets

The genres also work without AI. On the Design screen, **Tune detection only** applies the selected genre's detection settings. On the Audio screen, use **Tune for a genre…** under Global sensitivity.

| Genre | BPM | Detection changes |
|---|---|---|
| House | 118–128 | Steady kick, softer snare, adds a Clap band |
| Techno | 125–140 | Tight kick, quick hats, vocals turned down |
| Trance | 132–142 | Adds a Lead synth band, vocals up |
| Drum & bass | 165–178 | Snare leads, short gaps for fast breaks, faster bass release |
| Dubstep / bass | 138–150 | Half-time snare, adds a Sub-bass band, bass level up |
| Hip-hop / R&B | 80–100 | Longer gaps, vocals up |
| Reggaeton / Latin | 88–102 | Dembow snare, adds a Clap band |
| Pop / Top 40 | 95–130 | Vocals up, balanced drums |
| Rock / live band | 100–170 | Real drums: lower cymbal sensitivity (Hi-hat renamed Cymbals) |
| Ambient / chill | 60–110 | Low sensitivity overall, slow release |

A genre preset changes only the bands it names, and adds a band only if the show doesn't already have it. Every value stays editable on the [Audio](audio.md) screen.

## Cost and privacy

Each generation or refinement is one request to your provider. With Claude models, LightDeck lets the model think before answering and allows up to 32,000 output tokens, so a design costs more than a fixture interpretation. The request contains the show description above, including fixture names and positions. It never contains audio or your API key, which goes only in the request header.
