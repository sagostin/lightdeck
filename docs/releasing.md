# Building and releasing

LightDeck builds installers for macOS (Apple Silicon and Intel), Windows and Linux with GitHub Actions. Release builds bundle the Zigbee helper **and a Node.js runtime**, so USB Zigbee works without installing Node on the target machine.

## Workflows

| Workflow | Runs on | What it does |
|---|---|---|
| `.github/workflows/ci.yml` | every push to `main`, every pull request, or by hand | UI type-check and build; `cargo test` on macOS, Windows and Linux |
| `.github/workflows/release.yml` | pushing a `v*` tag, or by hand | Creates a **draft** GitHub release, builds installers on all four targets, and uploads them to it |

The release workflow outputs:

| Platform | Files |
|---|---|
| macOS | `.dmg` only (`aarch64` = Apple Silicon, `x64` = Intel). Open it and drag LightDeck to Applications. |
| Windows | `.msi` and `-setup.exe` (NSIS) |
| Linux | `.AppImage`, `.deb`, `.rpm` |

## First-time setup

The project folder isn't a git repository yet. To put it on GitHub:

```bash
cd ~/Projects/lightdeck && git init -b main && git add . && git commit -m "Initial commit"
```

Create an empty repository on GitHub, then:

```bash
git remote add origin https://github.com/<you>/lightdeck.git && git push -u origin main
```

`.gitignore` already leaves out `node_modules`, build output, and the downloaded Node runtime. Your show and settings live in `~/Library/Application Support/dev.lightdeck.app/`, outside the repository, so API keys and Zigbee network keys are never committed.

Replace `OWNER` in the badge and download links at the top of `README.md` with your GitHub user or organisation name.

In the repository's **Settings → Actions → General**, make sure *Workflow permissions* allows **Read and write** (the release job creates releases with the built-in token).

## Making a release

1. Set the new version. This updates `package.json`, `package-lock.json`, `tauri.conf.json`, `Cargo.toml` and `Cargo.lock` together:

   ```bash
   npm run version:set 0.2.0
   ```

2. Commit and push, then tag:

   ```bash
   git commit -am "Release 0.2.0" && git push
   ```

   ```bash
   git tag v0.2.0 && git push origin v0.2.0
   ```

3. Watch the **Actions** tab. Each platform takes roughly 10–20 minutes the first time and less after the caches warm up.
4. Open **Releases**, check the draft and its files, edit the notes, and click **Publish**.

Only these installers are attached to the release, never bare binaries or zipped `.app` bundles. Each build also keeps a copy of its installers as a workflow artifact for 14 days (on the run's summary page), which is handy for testing a build before publishing.

The workflow stops early if the tag doesn't match the app version (e.g. tagging `v0.2.0` while `tauri.conf.json` still says `0.1.0`).

**Run workflow** on the Release workflow (Actions tab) builds the current version without a tag. It creates or updates the draft release for that version.

## Building locally

```bash
npm run release:build
```

This is `tauri build` with `src-tauri/tauri.bundle.json` merged in. That config:

- runs `scripts/prepare-bundle.mjs` before bundling. It installs `zigbee-helper/node_modules` if needed, then stages a copy of the helper in `zigbee-helper/staged/` with only the build target's native serial-port files, plus the current Node.js 24 LTS for that target (checked against the official SHA-256 sums and cached in `zigbee-helper/.runtime-cache/`)
- bundles `zigbee-helper/staged/` as the app's Zigbee helper
- ad-hoc signs the macOS app (see below)

Output lands in `src-tauri/target/release/bundle/`. For an Intel build on an Apple Silicon Mac:

```bash
rustup target add x86_64-apple-darwin && npm run release:build -- --target x86_64-apple-darwin
```

Use `npm run release:build` rather than plain `npx tauri build`: the plain build doesn't include the Zigbee helper, so USB Zigbee only works on the machine that built it. Set `NODE_RUNTIME_VERSION=v24.x.y` to pin the bundled Node version.

## Unsigned builds

Builds aren't signed with a developer certificate by default:

- **macOS** builds are *ad-hoc* signed (`signingIdentity: "-"` in `tauri.bundle.json`). Without any signature, a downloaded Apple Silicon app is reported as **"damaged and can't be opened"**. With the ad-hoc signature it's blocked as *not verified* instead, which you can allow: open **System Settings → Privacy & Security**, scroll down and click **Open Anyway** (on macOS 14 and earlier, right-clicking the app and choosing **Open** also works). Or remove the download flag in Terminal:

  ```bash
  xattr -dr com.apple.quarantine /Applications/LightDeck.app
  ```

  Ad-hoc builds turn off the hardened runtime, which needs a real certificate and would otherwise block the microphone without extra entitlements.

- **Windows** SmartScreen shows a warning. Choose **More info → Run anyway**.

### Signing macOS builds (optional)

With an Apple Developer account, add these repository secrets (*Settings → Secrets and variables → Actions*), then uncomment the matching lines in `release.yml`:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | Your *Developer ID Application* certificate exported as `.p12`, base64-encoded (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | The `.p12` export password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | For notarization: your Apple ID, an app-specific password, and team ID |

Then in `src-tauri/tauri.bundle.json`, remove `"signingIdentity": "-"` and set `"hardenedRuntime": true`. Notarization needs the hardened runtime, and with it LightDeck needs an entitlements file granting `com.apple.security.device.audio-input` (microphone), set as `bundle.macOS.entitlements`. See Tauri's [macOS signing guide](https://v2.tauri.app/distribute/sign/macos/). The bundled Node binary is already signed by the Node.js project. If notarization rejects it, re-sign it with your identity in `prepare-bundle.mjs`.

## Platform notes

- **Linux** AppImages are built with linuxdeploy. The workflow sets `APPIMAGE_EXTRACT_AND_RUN=1` and `NO_STRIP=true`, and builds with `--verbose` so a failure shows linuxdeploy's own output.
- **Linux** builds on Ubuntu 22.04 and needs WebKitGTK 4.1 and ALSA at runtime (present on most desktops). For the Zigbee dongle, add yourself to the `dialout` group: `sudo usermod -aG dialout $USER`, then log out and in.
- **Windows** may need the dongle's USB serial driver (CP210x or CH340), depending on the stick.
- LightDeck is developed and tested on macOS. Windows and Linux builds compile and pass the tests in CI, but haven't been tried with real hardware.
