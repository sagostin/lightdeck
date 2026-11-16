// Regenerate the README screenshots in docs/images/ from the demo show.
// Needs the dev server running (`npm run dev` or `npx tauri dev`) and Google Chrome.
//   npm run screenshots
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "docs", "images");
const base = process.env.LIGHTDECK_URL ?? "http://localhost:1420";
const chrome = process.env.CHROME ?? [
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
].find(existsSync);
if (!chrome) throw new Error("Google Chrome not found. Set CHROME to its path.");

const shots = {
  live: "view=live",
  routes: "view=routes",
  audio: "view=audio",
  rooms: "view=rooms",
  browser: "view=library&tab=browser",
  design: "view=design&autodesign",
  control: "view=control",
};

const profile = mkdtempSync(join(tmpdir(), "lightdeck-shots-"));
try {
  for (const [name, query] of Object.entries(shots)) {
    execFileSync(chrome, [
      "--headless=new", "--disable-gpu", "--no-first-run", `--user-data-dir=${profile}`,
      "--hide-scrollbars", "--window-size=1440,900", "--virtual-time-budget=5000",
      `--screenshot=${join(out, `${name}.png`)}`, `${base}/?demo&${query}`,
    ], { stdio: "ignore" });
    console.log(`docs/images/${name}.png`);
  }
} finally {
  rmSync(profile, { recursive: true, force: true });
}
