# Building and releasing

LightDeck builds installers for macOS (Apple Silicon and Intel), Windows and Linux with GitHub Actions. Release builds bundle the Zigbee helper **and a Node.js runtime**, so USB Zigbee works without installing Node on the target machine.

## Workflows

| Workflow | Runs on | What it does |
|---|---|---|
| `.github/workflows/ci.yml` | every push to `main`, every pull request, or by hand | UI type-check and build; `cargo test` on macOS, Windows and Linux |
| `.github/workflows/release.yml` | pushing a `v*` tag, or by hand | Builds installers on all four targets and attaches them to a **draft** GitHub release |

The release workflow outputs:

| Platform | Files |
|---|---|
| macOS | `.dmg` and `.app.tar.gz` (`aarch64` = Apple Silicon, `x64` = Intel) |
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

The workflow stops early if the tag doesn't match the app version (e.g. tagging `v0.2.0` while `tauri.conf.json` still says `0.1.0`).

**Run workflow** on the Release workflow (Actions tab) builds the current version without a tag. It creates or updates the draft release for that version.

## Building locally

```bash
npm run release:build
```

This is `tauri build` with `src-tauri/tauri.bundle.json` merged in. That config:

- runs `scripts/prepare-bundle.mjs` before bundling, which installs `zigbee-helper/node_modules` if needed and downloads the current Node.js 24 LTS for the build target into `zigbee-helper/runtime/` (checked against the official SHA-256 sums, reused on later builds)
- adds `zigbee-helper/runtime/` to the app's resources

Output lands in `src-tauri/target/release/bundle/`. For an Intel build on an Apple Silicon Mac:

```bash
rustup target add x86_64-apple-darwin && npm run release:build -- --target x86_64-apple-darwin
```

Plain `npx tauri build` still works and makes a smaller app that uses the Node.js installed on the machine (see [Zigbee](zigbee.md#requirements)). Set `NODE_RUNTIME_VERSION=v24.x.y` to pin the bundled Node version.

## Unsigned builds

Builds are not code-signed by default:

- **macOS** shows "can't be opened because it is from an unidentified developer" (or "is damaged" for downloaded files). Right-click the app and choose **Open**, or run:

  ```bash
  xattr -dr com.apple.quarantine /Applications/LightDeck.app
  ```

- **Windows** SmartScreen shows a warning. Choose **More info → Run anyway**.

### Signing macOS builds (optional)

With an Apple Developer account, add these repository secrets (*Settings → Secrets and variables → Actions*), then uncomment the matching lines in `release.yml`:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | Your *Developer ID Application* certificate exported as `.p12`, base64-encoded (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | The `.p12` export password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | For notarization: your Apple ID, an app-specific password, and team ID |

See Tauri's [macOS signing guide](https://v2.tauri.app/distribute/sign/macos/) for details. The bundled Node binary is already signed by the Node.js project. If notarization rejects it, re-sign it with your identity in `prepare-bundle.mjs`.

## Platform notes

- **Linux** builds on Ubuntu 22.04 and needs WebKitGTK 4.1 and ALSA at runtime (present on most desktops). For the Zigbee dongle, add yourself to the `dialout` group: `sudo usermod -aG dialout $USER`, then log out and in.
- **Windows** may need the dongle's USB serial driver (CP210x or CH340), depending on the stick.
- LightDeck is developed and tested on macOS. Windows and Linux builds compile and pass the tests in CI, but haven't been tried with real hardware.
