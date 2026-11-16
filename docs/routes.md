# Routes and effects

A **route** connects something LightDeck hears to something the lights do:

> **When** *source* → *action* **in** *rooms* **on** *groups / fixtures* **channel** *channel*

Open **Routes** and click **Add route**.

## Sources

Sources are either **hits** (events) or **levels** (0–1 values that change continuously). See [Audio detection](audio.md) for what each band listens to.

| Hits | Levels |
|---|---|
| Each band's hit (*Kick hit*, *Snare hit*…) | Each band's level (*Bass level*, *Vocals level*…) |
| Beat | Overall level |
| Bar (4 beats) | Energy (build/drop) |
| | Beat phase |

A level source used with a hit-style action fires when the level rises past 60%.

## Targets: rooms, then groups and devices, then channel

1. **In**: *Any room*, or one or more rooms. This scopes the route. (Only shown when the project has more than one room.)
2. **On**: *Everything* in scope, or any mix of [fixture groups](rooms-and-patch.md#fixture-groups) and single fixtures. Only groups and fixtures in the chosen rooms are listed.
3. **Channel**: only channels those fixtures actually have:
   - **Every fixture's…** entries such as *All brightness channels* or *All colour channels* drive the matching channel on every target, whatever it's called. One route can flash DMX pars, Zigbee bulbs and the fog's brightness together.
   - One section per fixture type, with each channel. A note like *(2 of 5)* means only some targets have it.

The line under the channel shows how many fixtures the route drives. Changing the targets clears the channel if the new fixtures don't have it.

## Channel actions

| Action | What it does | Settings |
|---|---|---|
| Flash | Jump to On on each hit, then fade back to Off | On, Off, Fade |
| Random value | Pick a random allowed value on each hit. Uses the [pattern catalogue](fixtures.md#the-pattern-browser) first, weighted by rating. | Needs tags, Hold |
| Node hop | One fixture flashes per hit, hopping to a nearby fixture (in 3D) next time | On, Off, Fade, optional Colour palette |
| Modulate | Follow a level, mapped between Min and Max. With a hit source it follows a decaying pulse. | Min, Max, Smoothing |
| Cycle values | Step through a list of values, one per hit | Values |
| Look | Apply a saved [look](#looks) (pattern + secondary options) on each hit | Set, Needs tags, include defaults, Pick, Hold, Apply to |

### Looks

The **Look** action applies a saved look from a fixture type's [Patterns](fixtures.md#patterns-tab-looks-sets-and-defaults): every channel in the look at once, such as gallery + pattern + zoom + colour. It doesn't need a channel.

| Setting | Effect |
|---|---|
| Set | Only looks in this set (e.g. *Drop*), or any set |
| Needs tags | Looks must carry all of these tags |
| include defaults | Whether generated default looks can be picked |
| Pick | **Random** (weighted by rating) or **In order** |
| Hold | Keep a look at least this long before changing |
| Apply to | The head each look was saved on, or chosen heads. Picking P1 and P2 applies a look saved on P1 to both projectors. Channels are matched by name (P1 Pattern → P2 Pattern), and ones the other head doesn't have (the L2600's P2 has no gallery) are skipped. |

Looks with [moving channels](fixtures.md#moving-channels-motions) keep moving while they're applied. Motions timed in *route hits* advance each time this route fires; *beats* follow the BPM; *seconds* count from when the look was picked.

The route shows how many fixtures have looks and how many looks match. Put a *Chill* Look route and a *Drop* Look route in different route groups, then switch between them from a MIDI pad.

## Effects

Effects drive whole fixtures rather than one channel. They use each fixture's **brightness** and **colour** channels automatically:

- RGB fixtures (DMX pars, Zigbee colour bulbs): red, green and blue are set directly; a white channel is set to off.
- Colour-preset channels (fog colours, a laser's P1/P2 colour, cheap pars): set to the nearest preset colour.
- Fixtures without a dimmer have their RGB scaled for brightness.

The route shows *uses each fixture's brightness and colour channels · N fixtures*.

| Effect | What it does | Settings |
|---|---|---|
| **Strobe** | Hit source: strobes for the burst time after each hit. Level source: strobes while the level is above the threshold. | Rate (1, 2, 4 or 8 flashes per beat, following the BPM, or free in Hz), On time, Burst or Above |
| **Colour palette** | Each hit steps to the palette's next colour | Palette; Mode (see below); Crossfade; Brightness (0 leaves brightness to other routes) |
| **Twinkle** | Random fixtures sparkle in palette colours and fade over a dim base. Level sources: more sparkles when louder. Hit sources: a burst per hit. With a focus point, sparkles cluster near it. | Palette, Sparkles, Fade, Base |
| **Wave** | A band of light moves in time with the beat, changing colour each sweep. A level source scales its brightness. | Palette, Direction (left → right, front → back, **out from focus**, **around focus** like a rotating beacon, low → high), Beats per sweep, Width, Base |

Colour palette modes:

| Mode | Colours |
|---|---|
| All the same | Every fixture shows the same colour |
| Spread left → right | Neighbouring fixtures get different colours by position, moving across the room on each hit |
| Rings out from focus | Concentric bands of colour around the focus point, moving outward on each hit |
| Slices around focus | Pie slices of colour around the focus point, rotating on each hit |
| Random per fixture | Each fixture picks its own colour on each hit |

### Focus points

Effects and node hop are calculated **from** a point: by default the centre of the route's targets, or a [focus point](rooms-and-patch.md#focus-points) you place in a room (the DJ booth, the dance floor). Pick it in the route's **from** setting.

| With a focus point | |
|---|---|
| Wave, *Out from focus* | Ripples outward from the point |
| Wave, *Around focus* | Rotates around the point |
| Palette, *Rings* / *Slices* | Colour bands around the point |
| Twinkle | Sparkles cluster near the point, thinning out over its radius |
| Node hop | Starts at the fixture nearest the point and keeps being pulled back towards it |
| **fade with distance** (any effect) | Brightest at the point, fading to dark at its radius, so the effect lights an area rather than the whole room |

### Palettes

The **Colour palettes** panel on the right of the Routes screen holds the palettes effects use. Six come with every project: Club, Sunset, Ice, Rainbow, Forest and White. Click a palette's swatches to rename it, change or add colours, or delete it; **+ New** creates one.

## Timing and chance

Hit-based actions also have:

| Setting | Effect |
|---|---|
| Every | Only act on every Nth hit (2 = every other kick) |
| Cooldown | Ignore hits for this long after acting |
| Chance | Probability (0–1) that a hit acts |

## How routes combine

Routes run top to bottom each frame (40 times a second), so **later routes override earlier ones** on the same channel. A typical stack:

1. Colour palette on *Bar*, Spread across room
2. Twinkle on *Hi-hat level*
3. Strobe on *Kick hit*, 200 ms burst (last, so it wins on the drop)

After routes come manual sliders and presets, then [limits](rooms-and-patch.md#limits), then the master fader and blackout.

## Organising routes

- Give routes a **Group** name (with suggestions from your existing groups).
- **Group by**: your groups, device type, fixture or group, room, audio source, channel type, or nothing. Each heading has **All on** / **All off** and can be collapsed.
- **Sort** by run order (↑ ↓ to reorder), name or source.
- **Search** across names, groups, channels, sources and fixtures.
- On the **Live** screen, click a route to toggle it. You can also map routes and route groups to MIDI pads or keys (see [Live control](control.md)).
