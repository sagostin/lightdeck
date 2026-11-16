# Architecture

LightDeck is a [Tauri 2](https://tauri.app) app. A Rust backend does everything timing-critical (audio analysis, the 40 fps engine, Art-Net, Zigbee pacing, MIDI). A Svelte 5 frontend edits the show and displays live state. A small Node.js helper runs the Zigbee coordinator for USB dongles.

```
┌──────────────── Svelte UI (src/) ────────────────┐
│ views/*  components/*  lib/store.svelte.ts        │
└──── invoke(commands) ▲ events: frame, control_state, midi
┌──────────────── Rust (src-tauri/src) ────────────┐
│ audio ──► features ──┐                            │
│ midi ──► control ────┤                            │
│                      ▼                            │
│ engine (40 fps): routes → manual → limits →       │
│   master/blocked/blackout → universes             │
│        │                         │                │
│     artnet (UDP)            zigbee (paced)        │
│                               │   ├─ MQTT         │
│                               │   └─ helper (stdio JSON)
└───────────────────────────────┼───────────────────┘
                     zigbee-helper/index.mjs (Node, zigbee-herdsman) ─► USB dongle
```

## Repository layout

| Path | What |
|---|---|
| `src/App.svelte` | Shell: sidebar, top bar (BPM, master, blackout), keyboard handling, autosave |
| `src/lib/store.svelte.ts` | Global state (`app`), save debouncing, project actions, target helpers, control-state sync |
| `src/lib/types.ts` | TypeScript mirror of the Rust model, plus presets and labels |
| `src/lib/genres.ts` | Genre presets: detection tuning and lighting-style notes |
| `src/lib/design.ts` | Design plan types, applying a plan to the show, preview text |
| `src/lib/blocks.ts` | Scene building blocks: rig analysis, the block catalogue, building scenes of blocks into a plan |
| `src/lib/templates.ts` | Setup templates: fixed scene lists made of blocks |
| `src/lib/controllers.ts` | Built-in controller layouts, auto-mapping, learning profiles |
| `src/lib/scenes.ts` | Scene (preset route state) editing helpers |
| `src/lib/library.ts` | Fixture library: comparing and merging fixture types and their looks |
| `src/lib/aim.ts` | Fixture aiming: where a beam lands and its footprint (mirrors `light_spot` in model.rs) |
| `src/lib/rig.ts` | Reading fixture capabilities without app state (shared by templates and auto-mapping) |
| `src/views/` | One file per screen: Live, Rooms, Patch, Library (fixture types), Routes, Designer, Control, Audio, Outputs (Art-Net), Zigbee, AiSettings |
| `src/components/` | Shared UI: SceneBuilder, SceneEditor, ControllerSetup, RoomCanvas, SpectrumEditor, PatternBrowser, TargetPicker, ChannelSelect, Palette*, GroupsPanel, FileMenu, ConfirmButton, Tags, Meters |
| `src-tauri/src/model.rs` | The show format (serde), defaults, migrations |
| `src-tauri/src/engine.rs` | The frame loop, routes, effects, limits, target resolution |
| `src-tauri/src/audio.rs` | Input via `cpal`, FFT bands, onset and level detection, BPM and beat clock |
| `src-tauri/src/control.rs` | MIDI/key/button → actions, presets, master |
| `src-tauri/src/midi.rs` | MIDI input via `midir`, hot-plug, message parsing |
| `src-tauri/src/zigbee.rs` | USB-helper and MQTT backends, paced sender, value → light-state mapping, built-in Zigbee profiles |
| `src-tauri/src/artnet.rs` | ArtDmx packets |
| `src-tauri/src/qxf.rs` | QLC+ importer |
| `src-tauri/src/ai.rs` | Anthropic / OpenAI-compatible client for fixture interpretation |
| `src-tauri/src/designer.rs` | AI show designer: rig description for the prompt, and checking a returned plan against the show (id renumbering, reference fixing, clamping) |
| `src-tauri/src/project.rs` | Project and settings files, prefs |
| `src-tauri/src/lib.rs` | App setup, shared state, Tauri commands, the starter show |
| `zigbee-helper/index.mjs` | Zigbee coordinator helper |
| `fixtures/` | QLC+ files bundled into the starter show |
| `scripts/` | `prepare-bundle.mjs` (release bundling), `version.mjs` (set the version everywhere), `screenshots.mjs` (README images) |
| `.github/workflows/` | CI (`ci.yml`) and release builds (`release.yml`) |

## Data model

Everything is one `Show` (see `model.rs`), serialised as JSON. Every struct uses `#[serde(default)]`, so older files load and gain new fields. `Show::migrate()` runs on load and on every save from the UI; it's safe to run repeatedly.

Key types:

- `Profile`: channels (`ChannelDef` with `Capability` ranges, tags and catalogued `ValueNote`s), `Mode`s, `Head`s (projectors), `Look`s (several channel values applied together, optional per-channel `Motion`s, `builtin` for generated defaults) and `LookSet`s.
- `Room`: walls, ceiling and `FocusPoint`s (position, height, radius).
- `Fixture`: an instance with profile, mode, output or `zigbee_device`, position (x, y, z, rotation), room and per-channel `Limit`s.
- `Route`: source, `RouteAction`, target (`rooms`, `groups`, `fixtures`), `channel`, and optional `focus` + `falloff`. The channel is a key, `@<group>` (every channel of that type), or unused by effects.
- `BandDef`: detection band with weighted ranges and a detect mode.
- `Preset`, `ControlMapping`, `Palette`, `FixtureGroup`, `ZigbeeSettings`, `MidiSettings`.

**Ownership:** the UI owns editing. It holds `app.show` and sends the whole show to `save_show` (debounced 250 ms), which applies it to the engine and writes the working copy. When the backend changes the show itself (MIDI or keys toggling routes, muting bands), it emits `control_state` and the UI applies those changes to `app.show`, so they're saved too.

## The engine frame

`engine::compute` runs 40 times a second on its own thread:

1. **Defaults**: every patched channel starts at its default value.
2. **Routes**, in order:
   - Detect triggers: a hit counter changing, or a level crossing 0.6. Then apply every-N, cooldown and chance.
   - Channel actions write `(fixture, channel)` values. The Look action writes every channel of the chosen look, mapped onto other heads by name with `look_values`. Channels with a `Motion` are evaluated by `motion_value` from the beat clock, the seconds since the look started, or the route's hit count. Seeded random uses a splitmix64 hash of (seed, step), so it's repeatable.
   - Looks being previewed from the pattern browser (`preview_look` command, `Engine::preview`) are applied after routes and before manual values. The UI re-sends previews every second; entries not refreshed within `PREVIEW_TTL` (3 s) are dropped, so a reload or crash can't leave a fixture stuck on a preview.
   - Effects resolve fixtures and their `LookChans` (intensity, RGB(+W), colour macros) and write brightness and colour through `set_intensity` / `set_color`.
   - Effects and node hop measure positions from an **origin**: the route's focus point (`Room.focus`, looked up with `Show::focus_point`) or the targets' centroid. "Fade with distance" multiplies brightness by `1 - distance / radius`.
3. **Manual overrides** (sliders, presets, channel controls) replace route values.
4. **Per channel**: limit range → hold time → lock → master (intensity channels) → blocked values → blackout or disabled (`off` values).
5. Values go into 512-byte universes per Art-Net output, and into desired light states for Zigbee fixtures.

Every second frame is emitted to the UI as `frame` (audio features plus every fixture's values) at 20 Hz.

## Audio analysis

`cpal` delivers samples into a ring buffer. Every 512 samples, a Hann-windowed 2048-point FFT feeds each band. For each weighted range the band computes log-magnitude **level** and positive spectral **flux**.

- **Transient** bands fire when flux exceeds mean + k·σ of the last ~50 frames, with k from sensitivity, and the minimum gap has passed.
- **Level** bands fire when the smoothed level crosses a threshold, with hysteresis.

BPM comes from autocorrelating the first two bands' onset envelope over 6 seconds. A beat clock advances by BPM and is nudged by first-band hits.

## Zigbee

`Zigbee` runs a supervisor thread (connect, restart on settings change or helper exit) and a **send loop**. Each tick it sends one update to the light whose desired state differs most from what it last received, weighted by age. Each tick is `1/max_rate` seconds.

**USB mode** spawns `node zigbee-helper/index.mjs` and talks newline-delimited JSON over stdio:

- Requests `{id, cmd, …}` get replies `{id, ok, result | error}`.
- Commands: `ports`, `start {port, adapter, data_dir, network}`, `stop`, `permit_join {seconds}`, `devices`, `set {ieee, on, brightness, rgb, color_temp, transition}`, `identify`, `interview`, `remove {ieee, force}`.
- Events: `hello`, `devices`, `device_joined`, `device_interview`, `device_left`, `permit_join`, `adapter_disconnected`.
- Logs go to stderr; the Zigbee screen shows the last 80 lines.

The network keys are generated once into `zigbee/network.json` in the app data folder. The helper converts RGB to CIE xy for `moveToColor`.

**MQTT mode** publishes `<base>/<friendly_name>/set` and reads `<base>/bridge/devices` and `<base>/bridge/state`.

## MIDI and controls

`midi.rs` polls ports every 1.5 s, connects to wanted inputs and forwards parsed note, CC and program events. `control.rs` receives those, keyboard events (`control_key`) and on-screen tests (`control_fire`) on one channel, matches `ControlMapping`s, applies actions, and emits `control_state` (route and band states, blackout, master, manual values, active presets) and `midi` (for the monitor and Learn).

## Commands

The UI calls these through `invoke` (see `generate_handler!` in `lib.rs`):

- **Show:** `get_show`, `save_show`, `import_qxf`
- **Projects and settings:** `project_*`, `settings_*`, `get_band_presets`, `save_band_presets`, `get_controller_profiles`, `save_controller_profiles`, `get_fixture_library`, `save_fixture_library`, `fixture_export`, `parse_fixture_file`
- **Manual and blackout:** `set_manual`, `clear_manual`, `get_manual`, `set_blackout`
- **Audio:** `list_audio_devices`, `restart_audio`
- **AI:** `get_ai_settings`, `save_ai_settings`, `ai_test`, `ai_interpret`, `ai_design` (takes the show and a design request, returns a checked plan and warnings; the UI applies it), `design_check` (checks a template plan the same way, no AI)
- **Zigbee:** `zigbee_status`, `zigbee_ports`, `zigbee_permit_join`, `zigbee_identify`, `zigbee_remove`, `zigbee_reinterview`, `zigbee_reconnect`, `zigbee_profiles`
- **Control:** `midi_status`, `control_key`, `control_fire`, `control_state`, `preset_apply`, `set_master`

## Previewing the UI in a browser

During development, opening `http://localhost:1420` in a normal browser (while `npx tauri dev` or `npm run dev` runs) uses `src/dev/mock.ts` to fake the backend with `src/dev/sample-show.json`. Add `?view=routes` (or any screen id) to jump to a screen, and `?demo` for the demo show in `src/dev/demo.ts` with a simulated audio feed (`npm run screenshots` captures it for the README). `?view=design&autodesign` runs the designer against a canned plan in `src/dev/demo-design.ts`. Mocks are only loaded in development builds.

## Tests

```bash
npm test                       # both of the below
cd src-tauri && cargo test     # Rust: importer, Art-Net, migrations, targets, effects, Zigbee (incl. the real helper), MIDI, controls, AI designer
npm run test:ui                # UI logic in tests/: genre presets, applying designs, scenes, controller auto-mapping, building blocks, templates, fixture library merging
npm run check                  # UI type-check
```

The UI tests use Node's built-in test runner on the TypeScript sources directly (Node 24+ strips the types; `tests/resolve-ts.mjs` resolves extensionless imports). They aren't type-checked. `tests/rig.ts` builds a club rig from the bundled laser and fog profiles, and `planProblems` checks that a plan only references things that exist; every template is run against several rig variants and genres.

The AI designer tests never call a model. They check that the prompt documents every route and control action and field the data model has, so a new action fails the tests until the prompt describes it. They also check that plans are cleaned up against a rig: unknown ids, unreachable routes, out-of-range numbers, duplicate triggers, scope, and refining.

The Zigbee helper test starts the real Node helper without a dongle, so it needs Node and `zigbee-helper/node_modules`; it's skipped if Node isn't found.

## Packaging notes

`npm run release:build` merges `src-tauri/tauri.bundle.json`, which runs `scripts/prepare-bundle.mjs` to stage the helper in `zigbee-helper/staged/` (only the target's native prebuilds, plus the target's Node.js runtime in `runtime/`) and bundles that folder as `zigbee-helper/`. It also ad-hoc signs the macOS app. `Zigbee::find_node` prefers `LIGHTDECK_NODE`, then `<helper>/runtime/node`, then a system Node. In development (and plain `npx tauri build`) the helper runs from the source folder. CI and releases are in `.github/workflows/`; see [Building and releasing](releasing.md).
