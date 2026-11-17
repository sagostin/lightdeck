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

If a route's band is deleted (or removed by a genre preset), the route is switched off and highlighted, with a note at the top of the Routes screen. Pick a new sound in **When** and switch it back on.

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
| Move | Keep the channel moving, like a look's [moving channel](fixtures.md#moving-channels-motions): sweep back and forth (or up, down, smooth), step, seeded random, a list, catalogued values, or follow a level. Timed in beats, seconds or this route's hits. **Spread across fixtures** offsets each fixture so they chase instead of moving together | Motion, Spread |
| Look | Apply a saved [look](#looks) (pattern + secondary options) on each hit | Set, Needs tags, include defaults, Pick, Hold, Apply to |

### Looks

The **Look** action applies a saved look from a fixture type's [Patterns](fixtures.md#patterns-tab-looks-sets-and-defaults): every channel in the look at once, such as gallery + pattern + zoom + colour. It doesn't need a channel.

| Setting | Effect |
|---|---|
| Set | Only looks in this set (e.g. *Drop*), or any set |
| Needs tags | Looks must carry all of these tags |
| include defaults | Whether generated default looks can be picked |
| Pick | **Random** (weighted by rating) or **In order** |
| Only | Pick the looks to use, or **Any of these** (everything matching the set and tags). Looks are grouped under the pattern each was made on (e.g. *P1 Pattern 48 · Double line*), best rated first; click a pattern to pick all of its looks at once. Looks are picked individually, so two looks with the same name on different patterns are told apart |
| follow the section | Prefer looks tagged `breakdown`, `groove`, `build` or `drop` for the part of the track playing, and change look as soon as the section changes (after *Hold at least*). Sections without tagged looks use the other matching looks. See [Sections](scenes.md#sections). |
| Hold at least | Keep a look at least this long before changing |
| Change after at most | Change look even without a hit after this long (0 = only on hits) |
| Rest | Chance that a change shows **no look** for a while, so the fixture falls back to the scene's look or its defaults. **for** sets how long (0 = until the next hit) |
| Plays on | Normally **the head each look was saved on** in the pattern browser. Pick *P2 only* to play every look on P2, or *every head* to mirror it on both. Channels are matched by name (P1 Pattern → P2 Pattern), and ones the other head doesn't have (the L2600's P2 has no gallery) are skipped. A look only sets the channels ticked in it, so a head's on/off channel (the L2600's *P2 On/Off*) stays under your control: switch it with a pad, a route, a scene's values or by hand. If a P2 look shows nothing, check that P2 is on |

Looks with [moving channels](fixtures.md#moving-channels-motions) keep moving while they're applied. Motions timed in *route hits* advance each time this route fires; *beats* follow the BPM; *seconds* count from when the look was picked.

**How often it changes** comes from the route's trigger and timing: **When** *Bar* with **Every** 4 changes every 4 bars, *Kick hit* on each kick; **Cooldown** is the minimum wait between changes and **Chance** skips some. Together with *Hold at least*, *Change after at most* and *Rest*, that covers "change every few bars, never faster than 8 s, never stay longer than 30 s, and sometimes go blank".

The route shows how many fixtures have looks and how many looks match. One Look route with **follow the section** can play calm looks in breakdowns and peak looks in drops; or put a *Chill* and a *Drop* Look route in different scenes and switch scenes from a pad or with the autopilot.

### Also do: secondary channels and switching routes

Every route has an **Also do** part under its settings, for things that should happen on the same hits without building a separate route.

#### Move channels

A route can also move **other channels** on its fixtures, the way a look's secondary options move. Under **Move channels**, pick a channel (or *All … channels* of a type) and set its motion as for **Move**: sweep there and back, up or down, step, seeded random, a list, catalogued values, or follow a level.

| Option | Effect |
|---|---|
| restart on each hit | The motion starts over each time the route fires, timed from that hit. On by default for hit sounds |
| play once, then hold | One sweep (or one pass through the steps) and then hold the end, instead of looping |
| hits from | The sound that counts and restarts this channel. Normally the route's own sound; pick another to give the channel a **trigger of its own**, e.g. a kick flash whose zoom only restarts on each *Bar* |

Examples:

- **Flash on the kick** that also sweeps the laser's **zoom open over one beat** on each kick: Zoom, sweep *up*, 1 beat, *restart on each hit*, *play once*.
- **A Look route** playing your saved patterns with a **slow rotation sweep** over whichever look is showing: Rotation, sweep *there and back*, 8 beats.
- **Snare hits stepping** a pattern channel through a list, while the route's own action does the brightness.

A moved channel replaces what the route's action or look set for it; the next route down the list can still override it.

#### Switch routes

Each time the route acts, it can switch **other routes** on, off or over:

| Option | Effect |
|---|---|
| Switch on / Switch off | Turn that route on or off |
| Toggle | Flip it: on if it was off, off if it was on |
| for … beats | How long the switch lasts at the current tempo, then the route goes back to how it was. 0 = until something else changes it |

Switches never change the saved show. A route that's switched shows **switched on now** (or off) in its header, and the switch ends early when a scene, a pad or you change that route. Switches follow the route's **Every**, **Cooldown**, **Chance** and **plays in** sections, and a route that only switches others needs no channel.

Examples:

- **Snare in the build** switches the **strobe** route on for 8 beats: a snare route with *plays in* Build and a switch, no channel needed.
- **Every 4 bars** toggles a **wave** route: When *Bar*, Every 4, Toggle the wave.
- **A big kick** (Chance 25%) switches the **lasers' look route** off for 2 beats, for a moment of darkness.

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

Effects and node hop are calculated **from** a point: by default the centre of the route's targets, or a [focus point](rooms-and-patch.md#focus-points) you place in a room (the DJ booth, the dance floor). Pick it in the route's **from** setting. For fixtures that are [aimed](rooms-and-patch.md#aiming-where-the-light-lands), distances and positions are measured from where each beam lands rather than where the fixture hangs.

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

Hit-based actions also have these, applied in this order. The route sums them up underneath ("Acts on every 2nd kick hit, at most once every 0.5 s, about half the time"), and every field has an **i** to hover for what it does.

| Setting | Effect |
|---|---|
| Every | Only act on every Nth hit (2 = every other kick, 4 on *Beat* = once a bar) |
| Cooldown | Ignore hits for this long after acting, so busy sounds don't fire it too often |
| Chance | Of the hits that get past Every and Cooldown, act on only this percentage, at random. 100% = always; 50% = about half, never in a predictable pattern. Good for variety: a look that changes on some kicks, a strobe that sometimes joins in |
| plays in | Run only in these sections of a track (breakdown, groove, build, drop). None = always |

Route switches, and moved channels that restart on the route's own hits, follow the same hits: a route at 50% chance switches its routes and restarts those channels on about half the hits. A channel with **hits from** set to another sound follows that sound instead.

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
