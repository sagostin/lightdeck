# Getting started

This guide takes you from a fresh install to lights reacting to music.

## 1. Install

You need macOS with Xcode Command Line Tools, Rust and Node.js 20 or later.

```bash
brew install rustup node
rustup default stable
```

If `cargo` isn't found afterwards, add rustup to your shell path:

```bash
echo 'export PATH="/opt/homebrew/opt/rustup/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

Install the app's dependencies and the Zigbee helper's (once, and again after pulling changes to either `package.json`):

```bash
cd ~/Projects/lightdeck && npm install && (cd zigbee-helper && npm install)
```

## 2. Run

```bash
cd ~/Projects/lightdeck && npx tauri dev
```

The first build takes a few minutes; later starts are quick. The app reloads automatically when you edit the code.

macOS asks for microphone access the first time LightDeck listens to audio. Allow it, or audio detection stays off. If you declined, see [Troubleshooting](troubleshooting.md#no-audio).

## 3. The starter show

The first launch opens a starter show with:

- the **3D Animation RGB Laser L2600** (32 channels, two projectors P1 and P2) at DMX address 1, with its Mode channel locked to DMX manual so it can't fall back to its built-in auto/sound programs;
- the **dmxbridge fog-01** at address 33;
- one room, *Main room* (6 × 4.5 m, 2.4 m ceiling);
- one Art-Net output, *PKnight node*, which is **disabled**;
- two example routes: Kick changes the P1 pattern, and the hi-hat level drives P1 rotation.

Nothing is sent to your lights until you enable the output.

## 4. Connect your Art-Net node

1. Open **Art-Net** in the sidebar.
2. Enter your node's IP address. PKnight nodes usually ship on a `2.x.x.x` or `192.168.x.x` address; check its config page or manual.
3. Set **Net**, **Subnet** and **Universe** to match the node (often all 0).
4. Tick **Send**.

The bar under each output shows which addresses each fixture uses. If the top bar shows *Art-Net error*, hover it for details.

To check the connection, open **Patch**, select the laser and move a channel slider. Moving a slider takes manual control of that channel; **Release** gives it back to the routes.

## 5. Make it listen

1. Open **Audio**.
2. Pick your input device. To react to music playing on this Mac, use a loopback driver such as BlackHole (see [Audio detection](audio.md#reacting-to-music-on-this-mac)).
3. Play music and raise **Input gain** until the level meter peaks around three quarters on loud parts.
4. Watch the **Kick** lamp. Raise the Kick band's **Hit sensitivity** until it follows the kick drum without firing on everything else.

The BPM and beat dots in the top bar should settle within a few seconds.

## 6. Your first route

1. Open **Routes** and click **Add route**.
2. **When**: *Kick hit* → **Flash**.
3. **On**: pick a fixture or group (or leave *All fixtures*).
4. **Channel**: choose *All brightness channels*, or a specific channel such as the laser's P1 Zoom.
5. Set **On** to 255, **Off** to 0 and **Fade** to 200 ms.

The light now pulses on every kick. From here:

- Try the effects: **Colour palette** on *Bar*, **Twinkle** on *Hi-hat level*, **Strobe** on *Energy*. See [Routes and effects](routes.md).
- Place fixtures properly in **Rooms** so node hop and waves move through the space.
- Map a MIDI pad or key to turn routes on and off live. See [Live control](control.md).
- Save the project with `⌘S`.

## Safety first

Before running a laser near people, set up its limits as described in [Fixture types → Laser safety and low ceilings](fixtures.md#laser-safety-and-low-ceilings), and keep blackout within reach: the `B` key, the button in the top bar, or a MIDI pad mapped to Blackout.
