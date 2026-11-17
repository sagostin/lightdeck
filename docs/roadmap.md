# Roadmap

Ideas for where LightDeck goes next, roughly in order of priority. Done so far from this list: **MIDI feedback** (pad lights) and the **autopilot** (see [Live control](control.md)). Nothing here is a promise. The order follows how much each item helps a small live rig (a DJ night, a club room, a home setup) compared with how much work it is. For what exists today, see [Features](features.md).

Suggestions and pull requests are welcome. Open an issue to discuss bigger items first.

## Next up

### Pixel bars and LED strips
Today an effect gives each fixture **one** brightness and **one** colour. LED bars, pixel strips and multi-cell pars have many independently coloured cells.

- **Multi-cell fixture types.** Cells are defined like the heads lasers already use (*Cell 1…12*), each with its own RGB(W) channels. QLC+ heads and the AI interpreter fill them in.
- **Cells laid out on the map.** A bar is drawn as a line of a given length and angle, so each cell has a real position. Waves, spreads, rings and focus-point falloff then run *along* the bar as well as across the room, with no new effect settings needed.
- **Pixel effects:**
  - chase / comet;
  - a VU meter (a level fills the bar);
  - centre-out bursts on hits;
  - colour gradients that scroll with the beat;
  - per-cell twinkle.
- **Strips with hundreds of pixels.** These go over Art-Net or [WLED](#wled-and-pixel-controllers) and need more than one universe per fixture.

### Starter fixture pack
Ready-made types for the cheap, common fixtures that rarely ship with a QLC+ file. Each would have sensible tags, looks and beam angles:

| Type | Modes |
|---|---|
| Generic RGB par | 3 ch (RGB), 4 ch (dimmer + RGB), 7 ch (dimmer, RGB, strobe, mode, speed) |
| RGBW / RGBA / RGBWA+UV par | 4, 6, 8 and 10 ch |
| LED bar | 8- and 12-cell RGB, with whole-bar and per-cell modes (needs pixel bars) |
| Dimmer pack / single dimmer | 1 ch per circuit, for tungsten and practicals |
| Strobe | 2 ch (rate, intensity) and LED strobe/wash hybrids |
| UV bar, blinder | Intensity only; a blinder as 2 or 4 cells |
| Moving head (spot / wash) | 8- and 16-bit pan/tilt, colour wheel, gobo, dimmer, strobe |
| Fog, haze | Output, fan, plus a timer/interval mode |
| Laser | Common ILDA-less "DMX party laser" layouts, with blocked auto/sound modes pre-tagged |

These would ship in the fixture library and could be updated like any other library type.

### Open Fixture Library import
Import from the [Open Fixture Library](https://open-fixture-library.org) JSON format. It's a large, community-maintained catalogue with modes, matrices (pixels) and physical data such as beam angle and weight. It complements QLC+ import and the AI interpreter.

### Moving heads that follow the map
Aiming already knows each fixture's position, facing and tilt. For moving heads this becomes live:

- **Calibration.** Set two or three reference points per head, and LightDeck works out pan/tilt for any spot in the room.
- **Point at a focus point** or a fixture's footprint, and follow it if the point moves.
- **Position effects in time with the beat:** circle, figure-eight, sweep, ballyhoo and fan out/in. Sized in metres on the floor, not in DMX values.
- **Safe zones.** No-go areas on the map, such as the audience at eye height for lasers or the DJ's face, that heads and lasers are never pointed into.

### More outputs
- **sACN (E1.31)**, alongside Art-Net.
- **USB DMX interfaces**, for example Enttec DMX USB Pro and Open DMX-style FTDI dongles, so a single-universe rig doesn't need a network node.
- **Philips Hue Entertainment** (fast streaming over the Hue bridge), and possibly LIFX / Nanoleaf over LAN.

## Later

### WLED and pixel controllers
Drive WLED controllers (DDP or Art-Net) as pixel strips, with segments mapped onto the room plan. This pairs with pixel bars.

### Autopilot, next steps
The [autopilot](control.md#autopilot) is in. Ideas for taking it further:

- **Per-track memory.** With a setlist or [tempo sync](#tempo-sync), remember which scenes worked for a track.
- **Learn from you.** Notice which scenes you pick by hand for each section and prefer them.
- **More sections.** Tell intros and outros apart from breakdowns, and spot a "fake drop" that cuts back to a build.
- **AI live (optional).** Every few minutes, let the AI review the last sections heard and adjust scene tags or timing for the rest of the set.

### Setlists and cues
A list of songs or sets, each with its own genre tuning and scene list, stepped through with a pad or a key. Optionally, a cue stack per song for planned moments: a fog burst, a blinder hit or blackout at a set time or bar.

### Tempo sync
- **Ableton Link** and **MIDI clock in** to lock the beat clock to the DJ software or a drum machine, instead of (or as a check on) audio detection.
- **MIDI clock out**, to sync other gear to LightDeck.
- **OSC in and out**, for TouchOSC, Lemur, Resolume and other show software.

### Phone and tablet remote
A simple web control surface served on the local network: scenes, master, blackout, strobe and fog buttons. Usable from a phone behind the decks with no app install, and locked by a PIN.

### Rehearsal mode
Play an audio file or playlist instead of a live input. You see the show on the room plan, tune bands against the actual tracks, and the AI designer or genre presets can analyse a track's tempo and energy before the gig.

### 3D preview
A 3D view of the rooms with beams, haze and footprints drawn from the aiming data, for checking a design before you get to the venue.

## Ideas worth exploring

- **Pixel mapping from video or images.** Map a video, an image or a generated pattern across pixel bars, WLED strips and pars by their position on the map.
- **RDM discovery.** Find fixtures on the line, read their addresses and modes, and set them from LightDeck.
- **Fixture sharing.** A place to publish and find `.lightdeck-fixture` files with tested looks for specific cheap lasers and pars.
- **Multiple universes per node and auto-patching.** "Patch these 8 pars from address 1" with no gaps or overlaps.
- **Backup output.** Mirror the show to a second machine that takes over if the first one stops sending.
- **Accessibility and safety:**
  - a photosensitivity limit on strobe rates and large flashes;
  - a "quiet hours" brightness cap for venues.
