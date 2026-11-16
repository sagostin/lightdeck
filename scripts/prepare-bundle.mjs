// Runs before `tauri build` (release builds only, via src-tauri/tauri.bundle.json).
// Makes the Zigbee helper self-contained so the app works without Node installed:
//  1. installs zigbee-helper/node_modules if missing
//  2. downloads the official Node.js runtime for the target into zigbee-helper/runtime/
// The target comes from TAURI_ENV_TARGET_TRIPLE (set by the Tauri CLI), else this machine.
// Override the Node version with NODE_RUNTIME_VERSION=v24.x.y.
import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const helper = join(root, "zigbee-helper");
const runtime = join(helper, "runtime");
const NODE_MAJOR = 24;

const TRIPLES = {
  "aarch64-apple-darwin": "darwin-arm64",
  "x86_64-apple-darwin": "darwin-x64",
  "x86_64-unknown-linux-gnu": "linux-x64",
  "aarch64-unknown-linux-gnu": "linux-arm64",
  "x86_64-pc-windows-msvc": "win-x64",
  "aarch64-pc-windows-msvc": "win-arm64",
};

function hostPlatform() {
  const os = { darwin: "darwin", linux: "linux", win32: "win" }[process.platform];
  const arch = { arm64: "arm64", x64: "x64" }[process.arch];
  if (!os || !arch) throw new Error(`Unsupported host ${process.platform}-${process.arch}`);
  return `${os}-${arch}`;
}

const triple = process.env.TAURI_ENV_TARGET_TRIPLE;
const platform = triple ? TRIPLES[triple] : hostPlatform();
if (!platform) throw new Error(`No Node.js runtime mapping for target ${triple}`);

// 1. helper dependencies
if (!existsSync(join(helper, "node_modules"))) {
  console.log("[prepare-bundle] installing zigbee-helper dependencies");
  execSync("npm ci --omit=dev", { cwd: helper, stdio: "inherit" });
}

// 2. Node runtime
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

const version = await resolveVersion();
const stamp = `${version} ${platform}`;
const stampFile = join(runtime, ".target");
const exe = platform.startsWith("win") ? "node.exe" : "node";

if (existsSync(stampFile) && readFileSync(stampFile, "utf8").trim() === stamp && existsSync(join(runtime, exe))) {
  console.log(`[prepare-bundle] Node.js ${stamp} already present`);
} else {
  const base = `node-${version}-${platform}`;
  const file = platform.startsWith("win") ? `${base}.zip` : `${base}.tar.gz`;
  const dist = `https://nodejs.org/dist/${version}`;
  console.log(`[prepare-bundle] downloading ${dist}/${file}`);
  const [archive, sums] = await Promise.all([download(`${dist}/${file}`), download(`${dist}/SHASUMS256.txt`)]);

  const expected = sums.toString().split("\n").map((l) => l.trim().split(/\s+/)).find(([, name]) => name === file)?.[0];
  const actual = createHash("sha256").update(archive).digest("hex");
  if (!expected || expected !== actual) throw new Error(`Checksum mismatch for ${file}`);

  const tmp = mkdtempSync(join(tmpdir(), "lightdeck-node-"));
  try {
    const archivePath = join(tmp, file);
    writeFileSync(archivePath, archive);
    const inner = platform.startsWith("win") ? `${base}/node.exe` : `${base}/bin/node`;
    // bsdtar (macOS, Windows 10+) and GNU tar both handle these; Windows tar also reads .zip.
    execSync(`tar -xf "${file}" "${inner}" "${base}/LICENSE"`, { cwd: tmp, stdio: "inherit" });
    rmSync(runtime, { recursive: true, force: true });
    mkdirSync(runtime, { recursive: true });
    copyFileSync(join(tmp, inner), join(runtime, exe));
    copyFileSync(join(tmp, base, "LICENSE"), join(runtime, "LICENSE"));
    if (!platform.startsWith("win")) chmodSync(join(runtime, exe), 0o755);
    writeFileSync(stampFile, stamp + "\n");
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
  console.log(`[prepare-bundle] bundled Node.js ${stamp}`);
}
