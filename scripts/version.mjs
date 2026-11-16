// Set the app version everywhere it lives:  node scripts/version.mjs 0.2.0
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const v = process.argv[2]?.replace(/^v/, "");
if (!v || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(v)) {
  console.error("Usage: node scripts/version.mjs <major.minor.patch>");
  process.exit(1);
}

function edit(rel, re, optional = false) {
  const p = join(root, rel);
  const before = readFileSync(p, "utf8");
  if (!re.test(before)) {
    if (optional) return;
    throw new Error(`No version found in ${rel}`);
  }
  writeFileSync(p, before.replace(re, `$1${v}$2`));
  console.log(`updated ${rel}`);
}

const jsonVersion = /("version"\s*:\s*")[^"]+(")/;
edit("package.json", jsonVersion);
edit("src-tauri/tauri.conf.json", jsonVersion);
edit("src-tauri/Cargo.toml", /(\[package\][^[]*?\nversion\s*=\s*")[^"]+(")/);
edit("src-tauri/Cargo.lock", /(\[\[package\]\]\nname = "lightdeck"\nversion = ")[^"]+(")/);
edit("package-lock.json", /("name": "lightdeck",\s*"version": ")[^"]+(")/g, true);

console.log(`\nVersion is now ${v}. Commit, then tag and push:\n  git tag v${v} && git push origin v${v}`);
