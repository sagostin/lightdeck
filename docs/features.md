# Features

Everything LightDeck does today, by area. Each heading links to the guide with the details. For what's planned next, see the [Roadmap](roadmap.md).

## Fixtures ([guide](fixtures.md))

- **QLC+ import.** Open any `.qxf` fixture definition. LightDeck reads its channels, capabilities, heads and lens beam angle.
- **AI interpretation.** Paste a DMX chart from a manual, or a QLC+ file that needs cleaning up, and an AI model turns it into a fixture type with channels, capabilities and heads. Works with the Anthropic API or any OpenAI-compatible endpoint, including gateways and proxies.
- **Kinds.** Lasers, pars, moving heads, strobes, fog machines, Zigbee lights and anything else. The kind sets defaults such as beam angle and which effects apply.
- **Channel groups.** Each channel is tagged as intensity, colour, pan, tilt, pattern, beam, effect, speed, strobe, mode, fog or other. Effects use this to find a fixture's brightness and colour channels.
- **Tags on values and ranges.** Mark values as *blocked*, *safe*, *peak*, *chill* and so on. Routes and limits use the tags.
- **Multi-head fixtures.** Fixtures with several projectors (P1/P2 lasers) show, route and store looks for each head separately. Looks map across heads by channel name.
- **Pattern browser.** Step through a laser's values on the real device with the keyboard. Tag them, rate them and save the ones you like as looks.
- **Looks.** A look is a pattern plus secondary options (zoom, rotation, colour…), saved per fixture type. Looks can be grouped into sets (*drop*, *calm*…), and channels in a look can move: sweeps, steps, random values, lists, catalogue picks or follow-the-level, timed by the beat, by seconds or by hits.
- **Fixture library.** Keep fixture types and their looks in an app-wide library. Projects keep their own copy and merge updates both ways, so venue-only looks survive.
- **`.lightdeck-fixture` files.** Export a fixture type with its looks to share it, then import it into another library or project.
- **Beam angle per type.** Read from QLC+ lens data, set by hand, or a sensible default for the kind.

## Rooms and layout ([guide](rooms-and-patch.md))

- **Rooms drawn to scale.** Set the size, ceiling height and grid, and keep several rooms per project.
- **Fixture placement.** Place each fixture with its position, height and facing.
- **Aiming.** Point a light *Down*, *Up* or at an angle. LightDeck draws the beam's real footprint on the floor, the ceiling or the first wall in its way, and says where it lands.
- **Effects follow the light.** Waves, spreads, rings, falloff, sparkles and node hop work from where each beam lands, not from where the fixture hangs.
- **Focus points.** Mark the DJ booth, the dance floor and other spots. Effects ripple out from them, rotate around them or fade with distance from them. **Aim at** points a fixture at one, and can do the whole room at once.

## Patch and outputs ([guide](rooms-and-patch.md#patch))

- **Patch.** Address fixtures on Art-Net universes. Overlapping addresses, and fixtures that run past channel 512, are flagged.
- **Manual channel control.** Sliders for every channel, with presets that hold hand-set values.
- **Limits that always apply.** Per fixture instance:
  - lock a channel at a value;
  - clamp it to a range ("max = now" sets the top to the current value);
  - hold each value for a minimum time;
  - only allow random picks with certain tags.
  - Blocked values are never sent. This keeps a laser in DMX manual mode, under a low ceiling and out of its built-in auto/sound modes.
- **Fixture groups.** Named sets such as *Front pars*, filled by hand, by kind or by room, and used as targets everywhere.
- **Art-Net.** Any number of universes, unicast or broadcast, sent 40 times a second. Outputs start off, so nothing is sent until you're ready.
- **Zigbee** ([guide](zigbee.md)):
  - LightDeck can act as the coordinator through a USB dongle, with no hub or MQTT, or connect to an existing Zigbee2MQTT setup.
  - Pairing, renaming and removing devices.
  - Zigbee lights take the same effects as DMX, rate-limited to what Zigbee can handle.

## Audio detection ([guide](audio.md))

- **Live input.** Any audio input, with gain, including music playing on the Mac itself via a loopback device.
- **Detection bands on a live spectrum.** Kick, Bass, Snare, Vocals and Hi-hat by default.
  - Add your own bands, rename and recolour them, or mute them.
  - Drag a band's ranges on the spectrum. A band can listen to several ranges, each with its own weight.
- **Two ways to detect hits:**
  - *transient*, for sudden rises (kicks, snares);
  - *level crossing*, with hysteresis (bass, vocals).
  - Both have per-band gain, sensitivity, minimum gap, release and threshold.
- **Every band produces two sources:** a hit and a smooth 0–1 level.
- **Musical sources:**
  - BPM and a beat clock, driven by the first band;
  - *Beat* and *Bar* hits;
  - *Overall level*;
  - *Energy*, which rises on builds and drops;
  - *Beat phase*.
- **Onset trace** for tuning hits by eye.
- **Genre presets** (10 genres, from techno and house to drum and bass, rock, pop and ambient). Each one is a complete band setup.
  - You preview every change before it's applied, confirm before anything is overwritten, and can undo.
  - Your ranges, gain, names and colours are kept unless you ask otherwise.
  - Routes whose band is removed are switched off and flagged.

## Routes and effects ([guide](routes.md))

- **Routes.** A route connects a sound (a hit or a level) to an action on a target.
  - Targets are chosen room first, then group or device, then channel.
- **Channel actions:**
  - *Flash*;
  - *Random value* (from your catalogue, weighted by rating);
  - *Node hop* (nearest neighbour in 3D);
  - *Modulate* (follow a level);
  - *Cycle values*;
  - *Look* (apply a saved look, picked from a set or by tags).
- **Effects.** These drive each fixture's brightness and colour automatically, across RGB, colour-preset and dimmerless fixtures:
  - **Strobe:** beat-synced (1, 2, 4 or 8 flashes per beat) or free-running, as a burst or while a level is above a threshold.
  - **Colour palette:** all the same colour, spread across the room, rings out from a focus point, slices around it, or random per fixture. With crossfades.
  - **Twinkle:** sparkles over a dim base that cluster near a focus point.
  - **Wave:** moves left to right, front to back, out from a focus point, around it, or low to high, in time with the beat.
  - **Fade with distance:** any effect lights an area around a focus point instead of the whole room.
- **Timing and chance.** Act on every Nth hit, with a cooldown, or with a probability.
- **Palettes.** Six built-in palettes (Club, Sunset, Ice, Rainbow, Forest, White), plus your own.
- **Organisation:**
  - route groups, with *All on* / *All off*;
  - group by room, source, device and more;
  - search across routes;
  - reorder runs (later routes win).

## Live control ([guide](control.md))

- **Scenes.** Switch route groups with *On*, *Off* or *Leave* per group. Presets also hold hand-set channel values.
- **MIDI and keyboard mappings** with Learn:
  - **Actions:** route or group on/off, recall preset, blackout, master, set a channel (or move it with a fader), mute bands, band sensitivity, release manual control.
  - **Button modes:** toggle, while held, on, off or fader.
- **Controller setup.**
  - One-click mapping for known controllers (Akai APC mini, Novation Launchpad, Korg nanoKONTROL2) or your own learned layouts.
  - It maps only what the show actually has, and tells you when the mapping is out of date.
- **Pad lights (MIDI feedback).** Pads show which scenes, routes, blackout and autopilot are on, in the scene's or your chosen colour. Full RGB on the APC mini mk2 and Launchpad X / Mini mk3 (switched to Programmer mode automatically), green/red/yellow on the original APC mini, and on/off for the nanoKONTROL2 and other controllers.
- **Autopilot.** Hears breakdowns, grooves, builds and drops bar by bar and switches to scenes tagged for them:
  - stays on a scene a minimum number of bars, lets a drop cut in sooner, and rotates scenes for variety;
  - pauses when you pick a scene yourself;
  - scenes are tagged by hand, guessed offline from their names and content, or suggested by the AI;
  - a live view of what it hears and why it switched, and a pad or key to switch it on and off.
- **Master brightness and blackout.** Always last in the chain. `B` toggles blackout.

## Show designer ([guide](designer.md))

- **Templates (offline).** Club night, Party, Chill / lounge and Test my rig. Each builds scenes, routes and palettes from your rig with fixed rules.
- **Build your own (offline).** Compose scenes from building blocks: colour hits, colour changes, wave, sparkle, flash, strobe, node hop, brightness follows, laser looks and fog bursts. For each block, pick what it drives, which sound triggers it, the colours and the speed.
- **AI designer.**
  - Describe the night and an AI model designs scenes, routes, palettes and genre-tuned detection, using your fixtures, looks, rooms and focus points.
  - Every plan is checked and repaired before you see it.
  - You can refine it with follow-up instructions.
- **Review before adding.** Untick anything you don't want, replace earlier generated designs, and undo.
- **Ready for the autopilot.** Every designer tags its scenes with the sections they suit.
- **Controller mapping afterwards.** Map a controller to whatever was built.

## Projects and settings ([guide](projects-and-settings.md))

- **Project files.** `.lightdeck` files with Save, Save as and Open, plus autosave to a working copy, so nothing is lost if the app closes.
- **App settings.** Export and import AI settings, band presets, controller profiles and the fixture library.
- **Demo show.** `?demo` runs a club show with simulated audio for trying things without hardware.

## Platform

- **Native desktop app.** Built with Tauri 2: a Rust engine and a Svelte 5 UI.
- **Installers.** macOS (Apple Silicon and Intel), Windows and Linux, built by GitHub Actions on every `v*` tag.
- **Tests.** TypeScript tests for the designer, genres, scenes, controllers, templates, blocks, library and aiming. Rust tests for the engine, effects, importer, Zigbee, MIDI and AI designer.
