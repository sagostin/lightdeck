# Show designer

The **Design** screen builds a show for the rig you've set up, in one of three ways:

- **Templates** (no AI) build ready-made scene layouts from your rig with fixed rules. They work offline and give the same result every time.
- **Build your own** (no AI) puts scenes together from building blocks you choose.
- **AI designer** writes a design from your own description.

All three give you a design to review, untick, add and undo in the same way. Everything they add is ordinary routes, palettes, scenes and band settings that you can edit afterwards, or build by hand instead: routes on the Routes screen, scenes on the Control screen (see [Live control](control.md#scenes-and-presets)).

The designers build the **lighting**. MIDI and key mappings are a separate step, done afterwards with [Controller setup](control.md#controller-setup) on the Control screen. Controller setup maps what the show actually has, so a rig without a fog machine gets no fog pad, and you can map again any time the scenes or rig change. After you add a design with scenes, the Design screen points you there.

## Templates

Pick a template and optionally a genre, then click **Build setup**.

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

The design summary says what would improve the result, for example tagging laser looks or adding a focus point.

Click **Customise…** to open the selected template in the scene builder and change it.

## Build your own

The scene builder puts scenes together from building blocks. Each block becomes one route, and each scene becomes a scene preset that turns its own routes on and the others off.

1. Start from **A blank scene**, or **Start again from…** a template.
2. Name each scene, and reorder or remove scenes with ↑ ↓ ×.
3. **+ Add a block…** to a scene, then set:
   - **what it drives**: all colour fixtures, half A or B (two of your groups, or alternate fixtures), one of your groups, lasers or fog machines;
   - **what triggers it**: a band's hits, every beat or every bar, or a level (energy, overall level, a band's level) for *Brightness follows*;
   - **colours**: the genre's calm or hot palette, or any of your palettes;
   - **which looks** for lasers: any, calm or peak (your best-matching tag), or a specific tag;
   - **speed**: slow, medium or fast. This sets wave length, fade and decay times, strobe rate, how often lasers change, and fog cooldown.
4. Click **Build scenes**.

| Block | Drives | What it does |
|---|---|---|
| Colour hits | Colour fixtures | Each hit steps to the next palette colour at full brightness; rings out from the focus point |
| Colour changes | Colour fixtures | Colour moves on each hit (each bar by default) and leaves brightness to other blocks |
| Wave | Colour fixtures | A band of colour moves across the room with the beat |
| Sparkle | Colour fixtures | Random fixtures twinkle in palette colours |
| Flash on hit | Colour fixtures | Brightness jumps on each hit and fades back |
| Strobe | Colour fixtures | A strobe burst after each hit |
| Node hop | Colour fixtures | One fixture flashes per hit, hopping to a neighbour |
| Brightness follows | Colour fixtures | Brightness follows energy, overall level or a band's level |
| Laser looks | Lasers | Changes to a saved look, picked by tag |
| Fog burst | Fog | A short burst every few bars, at most one per 30–90 s depending on speed |

Blocks your rig can't drive (lasers without saved looks, no fog machine) are greyed out and skipped. The design lists anything skipped. The same block twice in a scene is named by where it goes ("Flash on hit · half A").

The templates are made of these same blocks, so a template opened with **Customise…** builds exactly the same setup until you change it.

## AI designer

Describe the night and pick a genre, and the AI proposes:

- **Audio detection tuning:** band sensitivity, timing and extra bands for the music.
- **Colour palettes** for the mood.
- **Routes grouped into scenes** (for example Warm-up, Build, Drop, Breakdown). Each scene is a route group, so the Routes screen shows it as one folder you can switch on and off.
- **Scene presets** that switch between those scenes.

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
4. Click **Generate design**. Large designs can take a few minutes.

## Reviewing and adding

The design lists its sections with a checkbox per item, and clicking a section heading ticks or unticks the whole section.

LightDeck checks every reference before showing the design:

- Unknown fixtures, groups and palettes are fixed or removed.
- Routes that wouldn't reach any fixture, such as a pan route on fixtures without pan, are skipped.

Anything fixed or skipped is listed under **things were fixed or skipped**.

**Add to show** adds what's ticked:

- Routes, presets and controls are added after your own.
- Retuned bands replace those bands' settings. Their ids stay the same, so your existing routes keep working.
- Only the starting scene's routes are on. Use the scene presets, or your controller, to move between scenes.

**Replace earlier generated designs** first removes routes and presets from previous designs (templates or AI), and controls earlier versions of the designer made. Your own work is never removed. Generated items have ids starting with `ai-`, so they're kept apart even after you edit them.

**Undo apply** puts the show back as it was before the last **Add to show**.

## Refining (AI)

Type a change under the design, such as "fewer strobes", "laser only on drops" or "slower warm-up", and click **Refine**. The AI revises the current design and keeps the parts you didn't mention. Review it again, then add it.

## Genre presets

The genres also work without AI. On the Design screen, **Tune detection only…** previews and applies the selected genre's detection settings. On the Audio screen, use **Tune for a genre…** under Global sensitivity. Either way you see what changes before anything is applied (see [Tuning for a genre](audio.md#tuning-for-a-genre)).

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

Each genre is a complete set: every band it uses with all its settings, created if missing. Your other bands, frequency ranges, gains, names and colours are kept unless you choose otherwise. Every value stays editable on the [Audio](audio.md) screen.

## Cost and privacy

Each generation or refinement is one request to your provider. With Claude models, LightDeck lets the model think before answering and allows up to 32,000 output tokens, so a design costs more than a fixture interpretation. The request contains the show description above, including fixture names and positions. It never contains audio or your API key, which goes only in the request header.
