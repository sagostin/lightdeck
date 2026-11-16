# Zigbee lights

Zigbee bulbs and plugs become ordinary fixtures: put them in rooms, group them, and drive them from routes, effects, presets, master and blackout.

There are two ways to connect:

- **USB dongle (recommended):** LightDeck runs the Zigbee network itself. You need no hub, Home Assistant, MQTT broker or Zigbee2MQTT install.
- **Zigbee2MQTT:** if you already run Zigbee2MQTT, LightDeck talks to it through its MQTT broker.

## What to expect from Zigbee

A Zigbee network handles roughly **10–20 commands a second in total**, and each bulb lags about **50–150 ms**. Use Zigbee lights for colour washes, slow palette changes, twinkles and levels (Bass, Vocals, Energy). Use DMX fixtures for strobes and per-beat flashes.

LightDeck protects the network with a **paced sender**:

- It never sends more than **Max messages per second** (default 10) across all Zigbee lights.
- Each slot goes to the light that's most out of date; on/off changes go first.
- Tiny changes are skipped, and colour is only resent when it changes.
- **Fade on each change** asks bulbs to fade over that time, which hides the lag on washes.

The Zigbee screen shows messages per second and how many lights are **waiting**. If the waiting count stays up, use fewer lights or slower routes; raising the rate past what your network handles makes bulbs miss commands.

## USB dongle

### Supported sticks

LightDeck uses [zigbee-herdsman](https://github.com/Koenkk/zigbee-herdsman), the Zigbee stack underneath Zigbee2MQTT. Pick the **Chip type** that matches your stick:

| Chip type | Typical sticks |
|---|---|
| Silicon Labs EFR32 (EmberZNet) | Sonoff ZBDongle-E, SMLIGHT, most EFR32MG21 "Zigbee 3.0" dongles |
| Texas Instruments CC2652 / CC1352 (Z-Stack) | Sonoff ZBDongle-P |
| ConBee / RaspBee (deCONZ) | ConBee II / III |
| ZiGate, Nordic nRF (ZBOSS) | Less common |
| Auto-detect | When you're not sure |

### Requirements

The USB mode runs a small helper (`zigbee-helper/`) with Node.js. Release builds (from GitHub or `npm run release:build`) include Node and the helper, so there's nothing to install.

When running from source, Node must be installed (LightDeck looks for a bundled runtime first, then `/opt/homebrew/bin`, `/usr/local/bin` and your shell path; set `LIGHTDECK_NODE` to point at another one), and the helper's packages installed once:

```bash
cd ~/Projects/lightdeck/zigbee-helper && npm install
```

### Setting up

1. Plug in the dongle. A short USB extension cable away from the Mac's ports improves range noticeably.
2. Open **Zigbee**, tick **Enabled** and choose **USB dongle**.
3. Click **Refresh** and pick the stick. If it's the only USB serial device, it's chosen automatically.
4. Check **Chip type**.
5. Pick a **Channel**. 15, 20 and 25 avoid the common Wi-Fi channels. The channel is only used when the network is first created.

On first start LightDeck forms a new Zigbee network and shows **Coordinator running**. Later starts resume the same network.

### Pairing lights

1. Click **Pair new devices**. Pairing stays open for 3 minutes (**Stop pairing** ends it early).
2. Reset each bulb so it looks for a new network. Most bulbs reset by switching them off and on 5–6 times (about a second each) until they blink. Philips Hue bulbs need a Hue dimmer or their app to factory reset.
3. Joining devices appear in the list. Setup (the *interview*, where LightDeck learns what the device supports) takes up to a minute per bulb.

### Managing devices

| Action | Notes |
|---|---|
| Rename | Type in the name field. Names are saved with the project. |
| Blink | Makes the device identify itself |
| Add | Adds it to the patch in the current room. The fixture type is chosen from what it supports: colour, tunable white, dimmable, or on/off plug. |
| Add all to patch | Adds every unpatched light and plug |
| Retry setup | Shown when setup failed |
| Remove | Asks the device to leave the network. If it doesn't answer, it's removed from LightDeck anyway. Reset it to pair it again. |

The **Signal** column shows link quality and whether the device is mains-powered (a router that extends the mesh) or battery-powered.

**Show log** shows the helper's output, which is useful if the dongle won't start.

### Your network lives on this Mac

The network key and the paired-device database are stored with the app's data, not in project files (see [Where data lives](projects-and-settings.md#where-data-lives)). **Back up the `zigbee` folder.** If it's lost, every device has to be reset and paired again. Opening a different project keeps the same Zigbee network.

## Zigbee2MQTT mode

1. Choose **Zigbee2MQTT** on the Zigbee screen.
2. Enter your MQTT broker's host and port, plus username and password if it needs them, and the base topic (default `zigbee2mqtt`).
3. LightDeck reads Zigbee2MQTT's device list and shows whether the bridge is online.

Pairing, renaming and removal are done in Zigbee2MQTT itself. Blink and Add work as above. The broker password is saved in the project file.

## Using Zigbee fixtures

In **Patch**, a Zigbee fixture's Output is *Zigbee* and its address column picks the device. They work with:

- **Effects** (palette, twinkle, wave, strobe) through their brightness and colour channels;
- **All brightness channels** and **All colour channels** routes alongside DMX fixtures;
- **Master** and **Blackout**.

Built-in Zigbee fixture types:

| Type | Channels |
|---|---|
| Colour light | Brightness, Red, Green, Blue |
| Tunable white light | Brightness, Colour temperature (warm 0 → cool 255) |
| Dimmable light | Brightness |
| On/off plug | On above 50% |
