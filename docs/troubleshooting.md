# Troubleshooting

## Install and build

**`cargo: command not found`**: rustup was installed but isn't on your path:

```bash
echo 'export PATH="/opt/homebrew/opt/rustup/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

**First build is slow**: normal, since Rust compiles every dependency once. Later builds take seconds.

**Port 1420 already in use**: another `tauri dev` is still running. Quit it, or stop it with `pkill -f "vite"`.

## No audio

- Check the **Audio** screen's status line for an error.
- macOS may have blocked the microphone. Open *System Settings → Privacy & Security → Microphone* and allow LightDeck (in development the entry may be your terminal app). Then click **Restart** on the Audio screen.
- To react to music playing on the Mac, you need a loopback driver such as BlackHole (see [Audio detection](audio.md#reacting-to-music-on-this-mac)).

## Hits fire too often or not at all

- Set input gain first: loud parts should peak around three quarters.
- Select the band and watch its trace. Raise **Hit sensitivity** if the marks miss the drum; lower it, or raise **Min gap**, if they fire on everything.
- Narrow overlapping bands (e.g. Snare against Vocals), or switch sustained sources like bass and vocals to **Level crossing**.

## Lights don't respond

- **Art-Net:** is **Send** ticked? Do the IP and Net/Subnet/Universe match the node? Is the Mac on the same network as the node (PKnight nodes often use `2.x.x.x`; give the Mac an address in that range too)?
- **Patch:** is the fixture **On**, at the right address and mode, with no red **!**?
- **Blackout** on, or **Master** at 0? Check the top bar.
- **Route:** is it enabled, does it have a channel (effects don't need one), and does the "drives N fixtures" line show more than 0?
- **Manual control:** a channel you moved by hand ignores routes until released. Click *Release N manual* in the top bar.
- **Limits:** a Lock or Range limit on the channel always wins.

## A fixture is stuck on one pattern

Something is holding its channels. Routes can't change channels under **manual control** (Patch sliders, presets, channel controls). Each affected route shows a yellow note with a **Release** button, and the top bar shows *Release N manual*. The pattern browser and the Patterns tab release their previews when you leave them, and LightDeck clears any leftover previews when it starts.

## The laser does its own thing

It's in an auto or sound mode. Lock its Mode channel to a DMX-manual value under Patch → Limits, and make sure the auto and sound ranges are tagged `blocked` (see [Laser safety](fixtures.md#laser-safety-and-low-ceilings)).

## Zigbee

**"Node.js wasn't found"**: only happens when running from source or a plain `npx tauri build`. Install Node (`brew install node`), set `LIGHTDECK_NODE` to its full path, or use a release build, which includes Node.

**"Zigbee helper isn't installed"**: run `npm install` in `zigbee-helper/`.

**The dongle isn't listed**: replug it and click **Refresh**. Some sticks need a driver (CH340/CP210x) on older macOS versions. Make sure no other app (Zigbee2MQTT, Home Assistant) has the port open.

**"Couldn't start the coordinator"**: check **Chip type** matches the stick (EFR32 sticks are *Ember*, Sonoff ZBDongle-P is *Z-Stack*) and open **Show log** for details. Unplug and replug, then **Restart**.

**Bulbs won't pair**: pairing must be open (the pulsing banner). Reset the bulb properly (usually 5–6 off/on cycles until it blinks) and keep it near the dongle or near a mains-powered Zigbee device. If setup fails, use **Retry setup** or remove it and pair again.

**Bulbs lag or miss changes**: the network is saturated. Lower the number of Zigbee lights driven by fast routes, use Fade on each change, and keep **Max messages per second** around 10. Mains-powered bulbs act as repeaters; a spread-out mesh helps.

**All devices gone after reinstalling**: the network lives in `~/Library/Application Support/dev.lightdeck.app/zigbee/`. Restore that folder from a backup, or re-pair.

## MIDI

- Tick **Enabled** on the Control screen and check the controller is listed and *connected*.
- Use the **Incoming** list to see what a pad actually sends; pads that send CC instead of notes need a *CC* trigger.
- If a mapping fires but nothing changes, check its action's settings (routes or bands selected, a channel picked).

## AI interpretation

- **Save and test** on the AI screen shows the exact error.
- "Reply was cut off": raise **Max tokens**.
- Behind a proxy, check the base URL. LightDeck adds `/v1/messages` or `/chat/completions` unless the URL already ends with it. Also check how the key is sent.

## Something else

Run the app from a terminal (`npx tauri dev`) and check the output for errors. The project working copy is at `~/Library/Application Support/dev.lightdeck.app/show.json`.
