# Scenes and the autopilot

The **Scenes** screen is where a show's scenes are made, edited and played:

- **The autopilot bar** (top): switch the autopilot on, see which part of the track it hears, and hold a section by hand.
- **Your scenes** (left): click one to edit it; **Recall** plays it.
- **The selected scene** (right): the whole rig in that scene, its pads, autopilot tags and building blocks. The same space shows **+ From template or AI…** and the autopilot **Settings**.

A scene is a **state of the whole rig** that one press switches to: every group in a look, with the routes you want running on top. For example, *Node mode*: pars on node hop to the kick with the wash routes off, lasers on a look tagged "node" with their rotation route on, fog off. Recall it from here, Live, or a pad or key, and every fixture changes together.

## The rig in this scene

The scene page lists your rig: **Whole rig** (routes aimed at everything), each fixture group, then any fixtures that aren't in a group. For each one:

- **Look**: a saved look the fixtures sit in while the scene plays, picked from their fixture types' looks (from the pattern browser), listed under the pattern each was made on. Fixtures without that look are left alone.
- **Routes**: the routes aimed at it. Click one to cycle **on** → **off** → **leave** (— : left as it is when the scene is recalled). Routes built from the scene's blocks show as **block** and are always on.
- **Set values**: channel values captured from the rig, with **clear**.

Looks and set values sit **underneath** the routes. Routes still play on top (a rotation or strobe route over a laser look), and nothing needs releasing: the next scene takes over. That's the difference from a [preset](control.md#presets), whose values are manual overrides that win over routes until released.

**Capture the rig as it is now** is the quick way to make a scene: set things up live (switch routes on the Routes or Live screen, set channels on Patch, try looks in the pattern browser), then click it. The scene saves which routes are on and every value set by hand, gives those channels back so the scene holds them, and starts playing. Capture again to update it.

## Making scenes

- **+ New scene** starts a scene with one block (Colour hits). Set looks and routes for the rig, **Capture**, or add blocks.
- **+ From template or AI… → From a template** builds several scenes at once (below) for you to review and add. Template scenes are block scenes, so you edit them afterwards like any other.
- **+ From template or AI… → Ask the AI → New scenes** writes scenes from a description (below).

Scenes made by an AI design, or older shows, switch routes rather than having blocks. Their page says so, and **Add blocks to this scene** lets you build more for them.

### Blocks

Blocks build routes for a scene, so you don't need to make them on the Routes screen first. Each block drives something in time with the music. For each one, set:

- **speed**: slow, medium or fast. This sets wave length, fade and decay times, strobe rate, how often lasers change, and fog cooldown;
- **what it drives**: all colour fixtures, half A or B (two of your groups, or alternate fixtures), one of your groups, lasers or fog machines;
- **what triggers it**: a band's hits, every beat or every bar, or a level (energy, overall level, a band's level) for *Brightness follows*;
- **colours**: the calm or hot palette for the genre, or any of your palettes;
- **which looks** for lasers: any, calm or peak (your best-matching tag), or a specific tag;
- **Sections…**: when it plays and what changes per section (see [Sections](#sections)).

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

Blocks your rig can't drive (lasers without saved looks, no fog machine) are greyed out and skipped, with a note saying why. Each block becomes one route (more with per-section changes), named after the block and the scene, and grouped under the scene's name on the Routes screen. Edit them through the blocks; changes made to a block scene's routes on the Routes screen are replaced the next time its blocks change.

### Recalling and pads

**Recall** on a scene (here, on Live, or from a pad or key) turns its routes on and the other scenes' routes off. **+ MIDI pad** and **+ Key** on a scene add a control that recalls it and start Learn: press the pad or key. **Other routes this scene switches** sets any other route groups On, Off or leave (—) for this scene, for routes you made by hand.

## Sections

LightDeck hears four parts of a track, bar by bar (see [how it decides](#how-the-autopilot-hears-the-music)):

| Section | What it is |
|---|---|
| **Breakdown** | Quiet, or the kick has dropped out (intros, breakdowns, outros) |
| **Groove** | The steady main part with a kick |
| **Build** | Loudness rising towards a drop |
| **Drop** | The loudest part with a full kick |

Sections are used in three separate ways. They work together, and only the first one needs the autopilot:

| | Set on | What it does | Needs the autopilot on? |
|---|---|---|---|
| **Scene tags** ("Autopilot plays it in") | Each scene | Which scenes the autopilot may **switch to** for each section | Yes |
| **Block sections** (a block's **Sections…**) | Each block | What a scene **does** in each section: a block plays only in some sections, or with different settings in some | No: works for any scene that's playing, however it was picked |
| **Limits per section** (autopilot **Settings**) | The whole show | A brightness cap, and whether strobe and fog may run, in each section | No |

### Block sections: one scene that follows the track

Open a block's **Sections…**:

For laser looks, tick **follow the section** on a Laser looks block (or on a [Look route](routes.md#looks)) and tag your looks `breakdown` / `groove` / `build` / `drop` in the pattern browser: the route then picks looks for the section playing, and changes look when the section changes.

- **Plays in**: untick sections where the block should stay quiet. A strobe block that plays only in **Drop**, a slow wave that plays only in **Breakdown**.
- **Different in a section**: change the block's speed, sound, colours or laser looks for one section, and leave the rest as "same". For example, a Laser looks block on calm looks, but **peak looks, fast** in the drop; Colour hits on calm colours, but **hot colours** in the build and drop.

So one *Main* scene can carry a whole track: colour hits all the time, sparkle in the groove, energy-following brightness in the build, strobe and peak lasers in the drop. You can still have separate scenes and let the autopilot switch between them, or mix the two.

Under the hood a block with per-section changes becomes one route per changed section plus one for the rest, and each route runs only in its sections. Routes you make on the Routes screen can be limited to sections the same way, and AI designs use this too.

### Limits per section

Autopilot **Settings → Limits per section** sets, for each section, a **brightness cap** (applied like the master fader), and whether **strobe** routes and route-driven **fog** may run. They apply to every scene, autopilot on or off. Manual control is never limited: a fog pad you hold still fogs. **Suggested limits** gives strobe only in builds and drops, fog only in drops, and dimmer breakdowns; **No limits** clears them.

### Holding a section

The detection can miss a drop, or you may want the drop look early. **Hold a section** on the autopilot bar (or a pad with the *Hold a section* action, best as *While held*) plays as if that section were heard: block sections and limits switch at once, and the autopilot (when on) switches scenes on the next bar, even in silence. Click it again, or let go of the pad, to go back to what's heard.

## Autopilot

The autopilot plays your scenes for you. Each bar, it switches to a scene tagged for the section it hears.

1. **Tag scenes**: on each scene, pick the sections under **Autopilot plays it in**. A scene can have several; untagged scenes are never picked (good for a blackout or test scene). Templates and AI designs come tagged. **Guess untagged scenes** (in Settings) tags them from their names ("Drop", "Chill", "Build up"…) or, failing that, from what they switch on.
2. **Switch it on** with **Off/On** on the autopilot bar, the switch next to the scenes on Live, or a pad or key with the *Autopilot on/off* action.

The bar shows what it hears, a loudness meter, the last 32 bars coloured by section, the scene playing and for how many bars, and why it last switched. Recall any scene yourself and the autopilot follows you, then pauses for a while (**resume** ends the pause early). It doesn't switch during silence.

### When it switches

In **Settings → When to switch scenes**:

| Setting | Default | |
|---|---|---|
| Stay at least | 8 bars | A scene plays at least this long |
| A drop cuts in after | 2 bars | A drop can replace the scene sooner than that |
| Change scene anyway after | 32 bars | Move to another scene for the same section, for variety (0 = never) |
| After you pick a scene, wait | 60 s | How long it holds off after you recall a scene yourself |

If no scene is tagged for a section, it uses the nearest one: a drop with no drop scene uses a build scene, a groove with no groove scene a breakdown scene, and so on. The bar says which sections have no scene.

### How the autopilot hears the music

Each bar, LightDeck compares the bar's loudness with what it heard over the last couple of minutes (so it adapts to quiet and loud rooms and to the input gain), checks whether the kick (the first band on the [Audio](audio.md) screen) is playing, and whether loudness has been rising for a few bars. A section has to last two bars before it counts, except a drop, which counts straight away. Section detection always runs, so block sections and limits follow the music even with the autopilot off.

## Templates

**+ From template or AI… → From a template**, pick a template and optionally a genre, then **Build scenes**.

| Template | Scenes | Autopilot tags |
|---|---|---|
| Club night | Warm-up (slow wave, hi-hat sparkle), Build (colour each bar, energy lifts brightness), Drop (kick colour hits, snare strobe, peak laser looks, fog burst), Breakdown (lights breathe with vocals, drifting colour) | Warm-up: breakdown, groove · Build · Drop · Breakdown |
| Party | Groove (kick colour steps, sparkle), Peak (fast rings, node hop, strobe, fog) | Groove: everything but drops · Peak: drop |
| Chill / lounge | Glow and Drift: long fades and slow waves; no strobe or fog | Glow: breakdown · Drift: the rest |
| Test my rig | One scene: every group flashes on the kick, colour changes each bar, lasers change look. For checking patching and detection | none |

Templates read the rig:

- **Colour fixtures** are fixtures with brightness or colour channels, apart from lasers and fog. They're targeted with your fixture groups where those fit, and call-and-response effects use two of your groups as halves.
- **Lasers** are fixtures with saved looks. Templates use looks tagged `chill` or `peak` when you have them, otherwise any look.
- **Fog** gets short bursts on the drop, at most every 45 seconds.
- **Focus points** make waves and colour hits ripple out from the first focus point.
- **Genre** picks the palettes and, if you keep it in the review, the detection tuning. Drum & bass gets a slower strobe, and Ambient removes strobe entirely.

## Asking the AI

**+ From template or AI… → Ask the AI** has two jobs. Both need an AI provider (set one up on the **AI** screen).

- **New scenes**: describe the night and pick a genre. The AI proposes detection tuning, palettes, routes grouped into scenes, and the scenes, tagged for the autopilot and using per-section routes where it suits (strobe only on drops, say).
- **Set up the autopilot**: the AI looks at what each of your scenes switches on and suggests scene tags, when to switch, and limits per section. You see the changes and **Apply** them; **Undo suggestion** puts things back.

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

## Reviewing and adding

Templates and AI designs show what they'll add, with a checkbox per item; clicking a section heading ticks or unticks the whole section. LightDeck checks every reference first: unknown fixtures, groups and palettes are fixed or removed, routes that wouldn't reach any fixture are skipped, and anything fixed or skipped is listed.

**Add to show** adds what's ticked and opens the first new scene. Only the starting scene's routes are on. **Replace earlier generated designs** first removes routes and scenes from previous designs (templates or AI). Your own work is never removed. **Undo apply** puts the show back as it was before the last **Add to show**.

To refine an AI design, type a change under it, such as "fewer strobes", "laser only on drops" or "slower warm-up", and click **Refine**. The AI revises it and keeps the parts you didn't mention.

After adding scenes, map your controller with [Controller setup](control.md#controller-setup) on the Control screen: it maps what the show actually has, so a rig without a fog machine gets no fog pad.

## Genre presets

Tuning detection for a genre is on the [Audio](audio.md#tuning-for-a-genre) screen. The genre chosen when adding scenes styles the palettes, and the AI's design, for that music.

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

Each genre is a complete set: every band it uses with all its settings, created if missing. Your other bands, frequency ranges, gains, names and colours are kept unless you choose otherwise.

## AI cost and privacy

Each generation, refinement or autopilot suggestion is one request to your provider. With Claude models, LightDeck lets the model think before answering and allows up to 32,000 output tokens, so a design costs more than a fixture interpretation. The request contains the show description above, including fixture names and positions. It never contains audio or your API key, which goes only in the request header.
