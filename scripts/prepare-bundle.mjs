// Runs before `tauri build` for release builds (via src-tauri/tauri.bundle.json).
// Stages a self-contained Zigbee helper in zigbee-helper/staged/ for the build target:
//  - index.mjs, package.json and node_modules (installed if missing)
//  - native addon prebuilds for the target only (others are dropped: they bloat the app,
//    and linuxdeploy fails on ELF files for other architectures when making the AppImage)
//  - the official Node.js runtime for the target in runtime/ (SHA-256 checked, cached in
//    zigbee-helper/.runtime-cache/ between builds)
// The target comes from TAURI_ENV_TARGET_TRIPLE (set by the Tauri CLI), else this machine.
// Override the Node version with NODE_RUNTIME_VERSION=v24.x.y.
import { execFileSync, execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const helper = join(root, "zigbee-helper");
const stage = join(helper, "staged");
const cache = join(helper, ".runtime-cache");
const NODE_MAJOR = 24;

// Rust target triple → [Node.js download platform, prebuild os, prebuild arch]
const TRIPLES = {
  "aarch64-apple-darwin": ["darwin-arm64", "darwin", "arm64"],
  "x86_64-apple-darwin": ["darwin-x64", "darwin", "x64"],
  "x86_64-unknown-linux-gnu": ["linux-x64", "linux", "x64"],
  "aarch64-unknown-linux-gnu": ["linux-arm64", "linux", "arm64"],
  "x86_64-pc-windows-msvc": ["win-x64", "win32", "x64"],
  "aarch64-pc-windows-msvc": ["win-arm64", "win32", "arm64"],
};

function hostTriple() {
  const key = `${process.platform}-${process.arch}`;
  const t = {
    "darwin-arm64": "aarch64-apple-darwin", "darwin-x64": "x86_64-apple-darwin",
    "linux-x64": "x86_64-unknown-linux-gnu", "linux-arm64": "aarch64-unknown-linux-gnu",
    "win32-x64": "x86_64-pc-windows-msvc", "win32-arm64": "aarch64-pc-windows-msvc",
  }[key];
  if (!t) throw new Error(`Unsupported host ${key}`);
  return t;
}

const triple = process.env.TAURI_ENV_TARGET_TRIPLE || hostTriple();
if (!TRIPLES[triple]) throw new Error(`No Node.js runtime mapping for target ${triple}`);
const [platform, pbOs, pbArch] = TRIPLES[triple];
const win = pbOs === "win32";
const log = (m) => console.log(`[prepare-bundle] ${m}`);

// ---- 1. helper dependencies --------------------------------------------------
if (!existsSync(join(helper, "node_modules"))) {
  log("installing zigbee-helper dependencies");
  execSync("npm ci --omit=dev", { cwd: helper, stdio: "inherit" });
}

// ---- 2. stage the helper with only this target's prebuilds -------------------
/** Keep a file under a `prebuilds/` folder only if it's for this target. */
function keep(src) {
  const parts = src.split(sep);
  const i = parts.lastIndexOf("prebuilds");
  if (i < 0 || i + 1 >= parts.length) return true;
  const dir = parts[i + 1]; // e.g. darwin-x64+arm64, linux-x64, win32-ia32
  const [os, archs = ""] = dir.split("-");
  if (os !== pbOs || !archs.split("+").includes(pbArch)) return false;
  // glibc builds only (the AppImage and .deb/.rpm target glibc distros)
  if (parts.length > i + 2 && basename(src).includes(".musl.")) return false;
  return true;
}

rmSync(stage, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });
for (const f of ["index.mjs", "package.json"]) copyFileSync(join(helper, f), join(stage, f));
cpSync(join(helper, "node_modules"), join(stage, "node_modules"), { recursive: true, filter: keep, verbatimSymlinks: true });
log(`staged helper for ${triple} (prebuilds: ${pbOs}-${pbArch})`);

// ---- 3. Node runtime ----------------------------------------------------------
async function resolveVersion() {
  if (process.env.NODE_RUNTIME_VERSION) return process.env.NODE_RUNTIME_VERSION;
  const res = await fetch("https://nodejs.org/dist/index.json");
  if (!res.ok) throw new Error(`Couldn't list Node.js releases: HTTP ${res.status}`);
  const list = await res.json();
  const hit = list.find((r) => r.version.startsWith(`v${NODE_MAJOR}.`) && r.lts);
  if (!hit) throw new Error(`No Node.js ${NODE_MAJOR} LTS release found`);
  return hit.version;
}

async function download(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Download failed (${res.status}): ${url}`);
  return Buffer.from(await res.arrayBuffer());
}

/** Extract `members` of an archive into `cwd`. */
function extract(cwd, archive, members) {
  if (archive.endsWith(".zip")) {
    // Use Windows' own bsdtar: Git for Windows puts GNU tar first on PATH, and it can't read zips.
    const tar = process.platform === "win32" ? join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe") : "tar";
    execFileSync(tar, ["-xf", archive, ...members], { cwd, stdio: "inherit" });
  } else {
    execFileSync("tar", ["-xzf", archive, ...members], { cwd, stdio: "inherit" });
  }
}

const version = await resolveVersion();
const exe = win ? "node.exe" : "node";
const cached = join(cache, `${version}-${platform}`);

if (!existsSync(join(cached, exe))) {
  const base = `node-${version}-${platform}`;
  const file = win ? `${base}.zip` : `${base}.tar.gz`;
  const dist = `https://nodejs.org/dist/${version}`;
  log(`downloading ${dist}/${file}`);
  const [archive, sums] = await Promise.all([download(`${dist}/${file}`), download(`${dist}/SHASUMS256.txt`)]);

  const expected = sums.toString().split("\n").map((l) => l.trim().split(/\s+/)).find(([, name]) => name === file)?.[0];
  const actual = createHash("sha256").update(archive).digest("hex");
  if (!expected || expected !== actual) throw new Error(`Checksum mismatch for ${file}`);

  const tmp = mkdtempSync(join(tmpdir(), "lightdeck-node-"));
  try {
    writeFileSync(join(tmp, file), archive);
    const inner = win ? `${base}/node.exe` : `${base}/bin/node`;
    extract(tmp, file, [inner, `${base}/LICENSE`]);
    rmSync(cached, { recursive: true, force: true });
    mkdirSync(cached, { recursive: true });
    copyFileSync(join(tmp, inner), join(cached, exe));
    copyFileSync(join(tmp, base, "LICENSE"), join(cached, "LICENSE"));
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

const runtime = join(stage, "runtime");
mkdirSync(runtime, { recursive: true });
copyFileSync(join(cached, exe), join(runtime, exe));
copyFileSync(join(cached, "LICENSE"), join(runtime, "LICENSE"));
if (!win) chmodSync(join(runtime, exe), 0o755);
writeFileSync(join(runtime, ".target"), `${version} ${platform}\n`);
log(`bundled Node.js ${version} (${platform})`);
// Sanity check that the staged helper is complete.
for (const p of ["index.mjs", "node_modules", join("runtime", exe)]) {
  if (!existsSync(join(stage, p))) throw new Error(`staged helper is missing ${p}`);
}
