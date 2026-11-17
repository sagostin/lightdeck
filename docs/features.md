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
  - While it's open it takes over the preview fixture: routes and the playing scene pause on it, so it shows only what you set, and get it back when you leave.
  - Catalogued values and looks are one list, with each look nested under the value it's built on. **Update look** saves into a look; on another value, **Save as new** leaves the original as it was.
  - Ticked secondary channels save the value they show (0 included); unticked ones are left alone. Looks never switch a head's on/off channel on by themselves.
- **Looks.** A look is a pattern plus secondary options (zoom, rotation, colour…), saved per fixture type. Looks can be grouped into sets (*drop*, *calm*…), and channels in a look can move: sweeps, steps, random values, lists, catalogue picks or follow-the-level, timed by the beat, by seconds or by hits.
- **Look routes.** Apply saved looks picked by set, tags or name, randomly by rating or in order, *hold at least*, *change after at most*, rests (no look for a while) and the route's every-N / cooldown / chance timing. **Follow the section** picks looks tagged `breakdown`, `groove`, `build` or `drop` for the part of the track playing. **Plays on** uses the head each look was saved on, one head, or every head. Look pickers group looks by pattern, best rated first.
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
  - *Move* (sweep back and forth, step, seeded random, list, catalogue or follow a level, with fixtures chasing each other);
  - *Look* (apply a saved look, picked from a set or by tags).
- **Also do.** Without a separate route, any route can also:
  - **move other channels** on its fixtures (zoom, rotation, position…), restarting on each hit or on a sound of its own, and playing once or looping: a kick flash that sweeps the zoom open on every kick;
  - **switch other routes** on, off or over for a number of beats: a build snare that brings the strobe in for 8 beats.
- **Help on every field.** Hover the **i** for what a setting does; each route sums up its timing in plain words.
- **Effects.** These drive each fixture's brightness and colour automatically, across RGB, colour-preset and dimmerless fixtures:
  - **Strobe:** beat-synced (1, 2, 4 or 8 flashes per beat) or free-running, as a burst or while a level is above a threshold.
  - **Colour palette:** all the same colour, spread across the room, rings out from a focus point, slices around it, or random per fixture. With crossfades.
  - **Twinkle:** sparkles over a dim base that cluster near a focus point.
  - **Wave:** moves left to right, front to back, out from a focus point, around it, or low to high, in time with the beat.
  - **Fade with distance:** any effect lights an area around a focus point instead of the whole room.
- **Timing and chance.** Act on every Nth hit, with a cooldown, with a chance (as a percentage), and only in some sections of a track (**plays in**). A route that's switched on waits for the next hit rather than firing on a stale one.
- **Palettes.** Six built-in palettes (Club, Sunset, Ice, Rainbow, Forest, White), plus your own.
- **Organisation:**
  - route groups, with *All on* / *All off*;
  - group by room, source, device and more;
  - search across routes;
  - reorder runs (later routes win).

## Live control ([guide](control.md))

- **Presets.** Save hand-set channel values (Patch sliders, pattern browser, channel faders), optionally with route states, and recall them from a pad or key. Scenes are on the [Scenes screen](scenes.md).
- **MIDI and keyboard mappings** with Learn:
  - **Actions:** route or group on/off, recall a scene or preset, autopilot on/off, hold a section, blackout, master, set a channel (or move it with a fader), mute bands, band sensitivity, release manual control.
  - **Button modes:** toggle, while held, on, off or fader.
- **Controller setup.**
  - One-click mapping for known controllers (Akai APC mini, Novation Launchpad, Korg nanoKONTROL2) or your own learned layouts.
  - It maps only what the show actually has, and tells you when the mapping is out of date.
- **Pad lights (MIDI feedback).** Pads show which scenes, routes, blackout, autopilot and held sections are on, in the scene's or your chosen colour. Full RGB on the APC mini mk2 and Launchpad X / Mini mk3 (switched to Programmer mode automatically), green/red/yellow on the original APC mini, and on/off for the nanoKONTROL2 and other controllers.
- **Master brightness and blackout.** Always last in the chain. `B` toggles blackout.

## Scenes and the autopilot ([guide](scenes.md))

- **One Scenes screen.** The autopilot bar, your scenes, and the selected scene in one place; recall any scene from here, Live, or a pad or key.
- **Scenes are states of the rig.** For each fixture group: a saved look it sits in, and which routes are on, off or left alone; routes play on top of the looks and nothing needs releasing. **Capture** saves the rig as it is now (routes on, hand-set values) into a scene.
- **Scenes made of blocks.** Colour hits, colour changes, wave, sparkle, flash, strobe, node hop, brightness follows, laser looks and fog bursts. Pick what each drives, which sound triggers it, colours and speed; the scene's routes are rebuilt as you edit, and pads mapped to them keep working.
- **Sections.** LightDeck hears breakdown, groove, build and drop, bar by bar, autopilot on or off:
  - **block sections:** a block plays only in some sections, or with different speed, sound, colours or laser looks in some (peak looks and faster in the drop), so one scene can follow a whole track;
  - **limits per section:** a brightness cap and strobe/fog on or off for each section, for every scene;
  - **hold a section** from the screen or a pad when the detection misses a drop.
- **Autopilot.** Switches to the scenes tagged for each section:
  - stays on a scene a minimum number of bars, lets a drop cut in sooner, and rotates scenes for variety;
  - follows and pauses for you when you pick a scene yourself;
  - scenes are tagged by hand, guessed offline from their names and content, or set up by the AI along with timing and limits;
  - shows what it hears, the last 32 bars, and why it switched.
- **Templates (offline).** Club night, Party, Chill / lounge and Test my rig build tagged block scenes from your rig with fixed rules.
- **AI.** Describe the night for new scenes (routes, palettes, genre-tuned detection, per-section routes, autopilot tags) using your fixtures, looks, rooms and focus points, or ask it to set up the autopilot for the scenes you have. Every plan is checked and repaired, reviewed before adding, and can be refined and undone.
- **Controller mapping afterwards.** Map a controller to whatever was built.

## Projects and settings ([guide](projects-and-settings.md))

- **Project files.** `.lightdeck` files with Save, Save as and Open, plus autosave to a working copy, so nothing is lost if the app closes.
- **App settings.** Export and import AI settings, band presets, controller profiles and the fixture library.
- **Demo show.** `?demo` runs a club show with simulated audio for trying things without hardware.

## Platform

- **Native desktop app.** Built with Tauri 2: a Rust engine and a Svelte 5 UI.
- **Installers.** macOS (Apple Silicon and Intel), Windows and Linux, built by GitHub Actions on every `v*` tag.
- **Tests.** TypeScript tests for the designer, genres, scenes, controllers, templates, blocks, library and aiming. Rust tests for the engine, effects, importer, Zigbee, MIDI, pad feedback, autopilot and AI designer.
