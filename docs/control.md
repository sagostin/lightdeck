# Live control

The **Control** screen maps MIDI controllers and keyboard keys to actions, and manages presets. The top bar always has the **Master** fader and **Blackout**.

To have a whole controller layout and its scene presets set up for you, name the controller on the [Design](designer.md) screen.

## MIDI

Tick **Enabled**. LightDeck finds controllers automatically and reconnects if one is unplugged and plugged back in. Untick inputs you don't want to listen to (all are used by default).

**Incoming** shows the last messages received (note, CC or program change, number, channel and value), which helps you find what a pad or knob sends.

## Controls

Click **+ MIDI control** or **+ Keyboard key**. Learn starts straight away: touch the pad, knob or fader, or press the key (`Esc` cancels). You can also set the trigger by hand: Note, CC or Program; number; and MIDI channel (*any* or 1–16).

### Actions

| Action | Button / key | Fader (CC) |
|---|---|---|
| Routes on/off | Selected routes | Top half on, bottom half off |
| Route group on/off | Every route in a [route group](routes.md#organising-routes) | Same |
| Recall preset | Applies the preset | – |
| Blackout | | Same |
| Master brightness | Full / zero | Sets the master level |
| Set a channel | Sets **On**; switching off gives the channel back to routes | Moves the channel between **From** and **To** |
| Mute/unmute detection bands | Selected bands | Same |
| Band sensitivity | – | Sets a band's hit sensitivity |
| Release all manual control | Clears every manual value | – |

**Set a channel** uses the same room → group/device → channel picker as routes, so one fader can dim *All brightness channels* in *Front pars*.

### Button behaviour

| Mode | Behaviour |
|---|---|
| Toggle | Each press flips on/off |
| While held | On while held, off when released (good for strobe or blackout hits) |
| Turn on / Turn off | Always sets that state |
| Fader | CC only: continuous value |

Pads that send CC instead of notes work as buttons: values of 64 and above count as pressed.

Each control has a **Test** button (or slider for fader actions) and is highlighted when it fires. Changes made from MIDI or keys update the on-screen switches and are saved with the project.

### Keyboard

Mapped keys work whenever LightDeck has focus and you're not typing in a field. `B` toggles blackout unless you map B to something else.

## Scenes and presets

Scenes and presets are the same thing on the **Scenes and presets** panel. A scene switches routes on and off, and a preset can also hold channels you've set by hand. One entry can do both.

### Scenes

Give your routes a **Group** on the Routes screen ("Warm-up", "Drop", "Lasers"…). Groups are what scenes switch.

1. Enter a name and click **+ New scene**. The scene editor opens.
2. For each route group choose **On**, **Off** or **—** (leave it as it is). Click the arrow next to a group to set its routes one at a time.
   - **Only this** turns that group on and every other route off.
   - **Capture what's on now** records the current state of every route, so you can set things up live on the Routes screen and then capture them.
   - **Clear** sets everything back to "—".
3. **+ MIDI pad** or **+ Key** adds a control that recalls the scene and starts Learn, so press the pad or key. The editor lists what already triggers the scene.

Click **Edit scene** to open the editor again.

### Presets with hand-set channels

1. Set channels by hand: Patch sliders, the pattern browser, or **Set a channel** controls.
2. Enter a name and click **Save current look**. Tick *with route states* to also record which routes are on.

### Recalling

- **Recall** applies a scene or preset. Hand-set values become manual overrides, so routes leave those channels alone until released.
- **replace others** clears every other manual value first.
- **Update from current** re-saves the hand-set channels, and the route states if it has them, from what's set now.

Recall from the Control screen, from the **Presets** row on **Live**, or from a MIDI pad or key. With **While held**, it applies only while the pad is down, which is useful for a quick hit. With **Toggle**, a second press takes it off again.

## Controller setup

**Controller setup** (on the left of the Control screen) maps a whole controller in one go, from what the show has now:

- **Pads, in order:** scenes, route groups (on/off), **Strobe (hold)** for every strobe route, **Fog (hold)** for fog machines, then **Blackout** and **Release manual**.
- **Faders, in order:** master, brightness per fixture group, then each band's sensitivity.
- **Keys** (optional): 1–9 and 0 for the first ten scenes, S for strobe, F for fog.

It only maps what exists: strobe needs a strobe route, fog needs a fixture with a fog channel, and group faders need groups with a brightness channel. Untick anything you don't want. **Preview** lists which pad or fader gets what and anything that didn't fit. Pads and keys your own controls already use are skipped. **Map controller** replaces only controls from an earlier auto-map, never your own, so run it again whenever you add scenes or fixtures. **Undo** puts the previous controls back.

If you add or remove scenes, strobe routes, fog machines, groups or bands after mapping, the panel says the mapping is out of date, or that some mapped controls point at things that were removed. Click **Map controller** again to bring it up to date.

Built-in layouts:

| Controller | Layout |
|---|---|
| Akai APC mini mk2 | 8×8 grid from the top-left, then track and scene buttons; master fader, then faders 1–8 |
| Novation Launchpad X / Mini mk3 | 8×8 grid in Programmer mode, from the top-left; no faders |
| Korg nanoKONTROL2 | S, M, R buttons and transport (CC mode); faders, then knobs |
| Generic pad controller | Notes 36–51 and CC 1–8, as most MPD/LPD-style pads send by default |

### Your own controllers

If your controller isn't listed, or sends different numbers (check **Incoming**), make a profile:

- **+ New by Learn** starts an empty profile. Click **Learn pads** and press each pad in the order you want them used (scenes come first). Then click **Learn faders** and move each fader or knob. Rename inputs and remove mistakes with ×, then **Save controller**.
- **Copy to edit** copies a built-in layout so you can change it.

Your profiles are saved for every project and included in [settings export](projects-and-settings.md#app-settings).

## Master and blackout

- **Master** (top bar) scales every brightness channel, on DMX and Zigbee alike. It doesn't touch lasers' pattern channels or fog output. Map it to a MIDI fader with the *Master brightness* action.
- **Blackout** sends every fixture its off values (including the laser's *laser off* mode and fog off) until released.
