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

## Presets

A **preset** is a saved look: the channels you've set by hand, and optionally which routes are on.

1. Set channels by hand: Patch sliders, the pattern browser, or **Set a channel** controls.
2. On **Control**, enter a name and click **Save current look**. Tick *include which routes are on* to save route states as well.

For each preset:

- **Recall** applies it. Its values become manual overrides, so routes leave those channels alone until released.
- **replace others** clears every other manual value first.
- **Update from current** re-saves it from what's set now.

Recall presets from the Control screen, from the **Presets** row on **Live**, or from a MIDI pad or key. With **While held**, a preset applies only while the pad is down, which is useful for a quick hit. With **Toggle**, a second press takes the look off again.

## Master and blackout

- **Master** (top bar) scales every brightness channel, on DMX and Zigbee alike. It doesn't touch lasers' pattern channels or fog output. Map it to a MIDI fader with the *Master brightness* action.
- **Blackout** sends every fixture its off values (including the laser's *laser off* mode and fog off) until released.
