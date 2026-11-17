# LightDeck

<!-- Replace sagostin with your GitHub user or organisation once the repository is on GitHub. -->
[![CI](https://github.com/sagostin/lightdeck/actions/workflows/ci.yml/badge.svg)](https://github.com/sagostin/lightdeck/actions/workflows/ci.yml)
[![Release](https://github.com/sagostin/lightdeck/actions/workflows/release.yml/badge.svg)](https://github.com/sagostin/lightdeck/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/sagostin/lightdeck?sort=semver&label=download)](https://github.com/sagostin/lightdeck/releases/latest)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-555)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-stable-000?logo=rust&logoColor=white)

An audio-reactive lighting controller for macOS. LightDeck listens to music, detects kicks, snares, hi-hats, bass and vocals, and drives DMX fixtures over Art-Net and Zigbee lights in time with it. You lay fixtures out in your rooms, connect what it hears to what the lights do with **routes**, and play it live from MIDI controllers or the keyboard.

Built with [Tauri 2](https://tauri.app) (Rust engine) and Svelte 5 (UI). Developed and tested on macOS; Windows and Linux builds are produced too.

![LightDeck's Live screen: a room plan with eight coloured pars, a laser and fog around a dance floor focus point, with band meters and routes](docs/images/live.png)

## What it does

- **Fixtures:** import QLC+ `.qxf` fixture files, or paste any manual's DMX chart and have an AI model convert it. Tag individual values and ranges ("blocked", "safe", "peak"…), and browse a laser's patterns on the real device to build a catalogue of the looks you like.
- **Fixture library:** keep fixture types and their looks for every project. Projects copy from it and merge updates both ways without losing venue-only looks. Export and import `.lightdeck-fixture` files to share them.
- **Two-head lasers:** fixtures with several projectors (P1/P2) show and route each head separately.
- **Rooms and layout:** draw rooms to scale, place fixtures with height and direction, aim lights at the floor, ceiling or an angle and see where each beam lands (effects then work from the lit spots), use several rooms per project, and add **focus points** (DJ booth, dance floor) that effects ripple out from, rotate around or fade with distance from.
- **Audio detection:** configurable bands on a live spectrum (Kick, Bass, Snare, Vocals, Hi-hat by default; add your own). A band can listen to several frequency ranges, and each has its own gain, sensitivity and detection mode. LightDeck also tracks BPM and the beat.
- **Routes:** connect a sound (a hit or a level) to an action on fixtures.
  - Channel actions: flash, random value from your catalogue, node hop (nearest neighbour in 3D), modulate, cycle.
  - Effects: strobe, colour palette, twinkle, wave. These drive each fixture's brightness and colour automatically.
  - Targets are picked room first, then groups or devices, then channel.
- **Limits that always apply:** lock values, clamp ranges, hold times and blocked values keep a laser below a low ceiling and away from its built-in auto/sound modes, whatever the routes do.
- **Scenes and the autopilot:** a scene switches the whole rig at once (each group in a saved look, with the routes you want running on top), set up per group or captured from the rig as it is. Make scenes from building blocks (colour hits, waves, strobe, laser looks, fog bursts…), from offline templates (club night, party, chill, rig test) or by describing the night to the AI. LightDeck hears breakdowns, grooves, builds and drops: blocks can play only in some sections or change in them (strobe and peak lasers only in the drop), limits per section cap brightness or stop strobe and fog, and the autopilot switches to the scenes tagged for each section, bar by bar.
- **Live control:** MIDI and keyboard mappings with Learn, one-click mapping for known controllers (APC mini, Launchpad, nanoKONTROL2, or your own learned layout) with pads lit in each scene's colour, presets of hand-set channels, a master brightness fader and blackout.
- **Outputs:** Art-Net to any node (e.g. PKnight). Zigbee lights work either through a USB dongle, with LightDeck as the coordinator (no hub or MQTT needed), or through an existing Zigbee2MQTT setup.
- **Projects:** `.lightdeck` project files, autosave, and export/import of app settings.

See the full [feature list](docs/features.md).

### Coming next

Planned work includes pixel bars and LED strips with per-cell effects, a starter pack of common fixture types (generic RGB/RGBW pars, LED bars, dimmers, moving heads), Open Fixture Library import, moving heads that follow the room plan, and sACN and USB DMX output. See the [roadmap](docs/roadmap.md).

## Screenshots

| | |
|---|---|
| [![Routes](docs/images/routes.png)](docs/images/routes.png) **Routes** connect hits and levels to channel actions and effects, grouped and targeted by room, group or device. | [![Audio](docs/images/audio.png)](docs/images/audio.png) **Audio** detection bands drawn on a live spectrum, with an onset trace for tuning hits. |
| [![Rooms](docs/images/rooms.png)](docs/images/rooms.png) **Rooms** drawn to scale, with fixtures, beam directions and focus points. | [![Pattern browser](docs/images/browser.png)](docs/images/browser.png) **Pattern browser** steps through a laser's values on the real device so you can tag, rate and save looks. |
| [![Control](docs/images/control.png)](docs/images/control.png) **Control** maps MIDI pads, faders and keys to routes, presets, master and blackout. | [![Scenes](docs/images/design.png)](docs/images/design.png) **Scenes** adds scenes from templates or an AI brief, for review before adding, next to the autopilot and your scenes. |

Screenshots use the built-in demo show (see [Development](#development)).

## Download

Installers for macOS (Apple Silicon and Intel), Windows and Linux are on the [Releases](https://github.com/sagostin/lightdeck/releases) page. Builds aren't notarized: the first time you open LightDeck on macOS, click **Open Anyway** in *System Settings → Privacy & Security*. See [Building and releasing](docs/releasing.md#unsigned-builds).

## Quick start

You need macOS with Xcode Command Line Tools, Rust, and Node.js 20+.

```bash
brew install rustup node
rustup default stable
```

Install the dependencies (once):

```bash
cd ~/Projects/lightdeck && npm install && (cd zigbee-helper && npm install)
```

Run the app in development mode:

```bash
npx tauri dev
```

On first launch LightDeck opens a starter show with the bundled laser and fog fixtures patched and the Art-Net output **disabled**, so nothing is sent until you set your node's IP. See [Getting started](docs/getting-started.md) for a first-show walkthrough.

To build a release app with the Zigbee helper and a Node.js runtime bundled:

```bash
npm run release:build
```

Pushing a `v*` tag to GitHub builds macOS, Windows and Linux installers automatically. See [Building and releasing](docs/releasing.md).

## Documentation

| Guide | What's in it |
|---|---|
| [Features](docs/features.md) | Everything LightDeck does today, by area |
| [Getting started](docs/getting-started.md) | Install, first launch, connecting a node, a first audio-reactive route |
| [Fixture types](docs/fixtures.md) | Importing QLC+ files, AI interpretation, tags, the fixture library, the pattern browser, laser safety and ceiling limits |
| [Rooms, patch and groups](docs/rooms-and-patch.md) | Drawing rooms, patching fixtures, limits, fixture groups, Art-Net outputs |
| [Audio detection](docs/audio.md) | Input and gain, detection bands, tuning hits, BPM |
| [Routes and effects](docs/routes.md) | Sources, actions, effects, palettes, targets, grouping |
| [Live control](docs/control.md) | MIDI and keyboard mappings, pad lights, controller setup, presets, master, blackout |
| [Scenes and the autopilot](docs/scenes.md) | Scenes made of blocks, sections, the autopilot, templates, the AI, genre presets |
| [Zigbee lights](docs/zigbee.md) | USB dongle coordinator, pairing, Zigbee2MQTT mode, limits |
| [Projects and settings](docs/projects-and-settings.md) | Project files, autosave, settings export, where data lives |
| [Troubleshooting](docs/troubleshooting.md) | Common problems and fixes |
| [Building and releasing](docs/releasing.md) | Local release builds, GitHub Actions CI and releases, versioning, signing |
| [Architecture](docs/architecture.md) | For developers: code layout, engine, data model, helper protocol, tests |
| [Roadmap](docs/roadmap.md) | Planned and possible features: pixel bars, fixture packs, moving heads, more outputs |

## Keyboard shortcuts

| Key | Action |
|---|---|
| `B` | Toggle blackout (unless you've mapped B to something else) |
| `⌘S` / `⇧⌘S` | Save / Save as |
| `⌘O` | Open project |
| Pattern browser: `←` `→` (`⇧` ×10), `↑` `↓`, `1`–`7`, `⇧1`–`⇧5` | Step values, jump between catalogued values, toggle tags, rate |

Any other key can be mapped to an action on the **Control** screen.

## Development

```bash
npm run check                          # type-check the UI
npm test                               # UI logic tests, then Rust: engine, importer, Zigbee, MIDI, effects, AI designer
```

With the dev server running (`npm run dev` or `npx tauri dev`), open the UI in a normal browser to work on screens without hardware:

- <http://localhost:1420/> uses a copy of a real show with a fake backend.
- <http://localhost:1420/?demo> loads the demo club show (eight pars, laser, fog, two rooms) with a simulated 124 BPM audio feed. Add `&view=routes` (or `rooms`, `audio`, `control`, `library&tab=browser`…) to open a screen directly.
- `npm run screenshots` regenerates the images in `docs/images/` from the demo (needs Google Chrome).

See [Architecture](docs/architecture.md) for how the pieces fit together.

## Safety

Lasers can damage eyes. Keep beams above head height or below the audience, use the fixture limits to lock the laser into DMX manual mode and inside a calibrated safe area, and keep a blackout within reach (the `B` key, or map one to a MIDI pad). For fog, keep routes short (a flash with a short fade, plus a cooldown) and avoid long continuous bursts near smoke alarms.
