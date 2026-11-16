// LightDeck Zigbee helper: runs the Zigbee coordinator (zigbee-herdsman) against a
// USB dongle and talks to LightDeck over stdin/stdout, one JSON object per line.
//
// In:  {"id":1,"cmd":"start", ...}     Out: {"id":1,"ok":true,"result":...}
// Out (unsolicited): {"event":"devices","devices":[...]} etc.
// Logs go to stderr.

import herdsman from "zigbee-herdsman";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { findAllDevices } from "zigbee-herdsman/dist/adapter/adapterDiscovery.js";

const { Controller, setLogger } = herdsman;

const out = (obj) => process.stdout.write(JSON.stringify(obj) + "\n");
const log = (level, msg) => process.stderr.write(`[${level}] ${msg}\n`);
setLogger({
  debug: () => {},
  info: (m, ns) => log("info", `${ns ?? ""} ${typeof m === "function" ? m() : m}`),
  warning: (m, ns) => log("warn", `${ns ?? ""} ${typeof m === "function" ? m() : m}`),
  error: (m, ns) => log("error", `${ns ?? ""} ${typeof m === "function" ? m() : m}`),
});

/** @type {InstanceType<typeof Controller> | null} */
let controller = null;
let startInfo = null;

// ---------- device description ----------

function lightEndpoint(device) {
  return (
    device.endpoints.find((e) => e.supportsInputCluster("lightingColorCtrl")) ??
    device.endpoints.find((e) => e.supportsInputCluster("genLevelCtrl")) ??
    device.endpoints.find((e) => e.supportsInputCluster("genOnOff"))
  );
}

function describe(device) {
  const ep = lightEndpoint(device);
  const has = (c) => !!ep?.supportsInputCluster(c);
  const colorCluster = has("lightingColorCtrl");
  const caps = ep?.getClusterAttributeValue?.("lightingColorCtrl", "colorCapabilities");
  const capsKnown = typeof caps === "number";
  return {
    ieee: device.ieeeAddr,
    nwk: device.networkAddress,
    role: device.type,
    manufacturer: device.manufacturerName ?? "",
    model: device.modelID ?? "",
    power: device.powerSource ?? "",
    interview: device.interviewState,
    lqi: device.linkquality ?? null,
    last_seen: device.lastSeen ?? null,
    on_off: has("genOnOff"),
    level: has("genLevelCtrl"),
    // colorCapabilities bit 3 = XY, bit 4 = colour temperature. Unknown -> assume both.
    color: colorCluster && (!capsKnown || (caps & 0b01000) !== 0 || (caps & 0b00001) !== 0),
    color_temp: colorCluster && (!capsKnown || (caps & 0b10000) !== 0),
  };
}

function devices() {
  if (!controller) return [];
  return controller.getDevices().filter((d) => d.type !== "Coordinator" && !d.isDeleted).map(describe);
}
const pushDevices = () => out({ event: "devices", devices: devices() });

// ---------- colour ----------

/** sRGB 0-255 -> CIE xy scaled to 0..65279 as ZCL expects. */
function rgbToXy([r, g, b]) {
  const lin = (c) => {
    c /= 255;
    return c > 0.04045 ? ((c + 0.055) / 1.055) ** 2.4 : c / 12.92;
  };
  const [R, G, B] = [lin(r), lin(g), lin(b)];
  const X = R * 0.4124 + G * 0.3576 + B * 0.1805;
  const Y = R * 0.2126 + G * 0.7152 + B * 0.0722;
  const Z = R * 0.0193 + G * 0.1192 + B * 0.9505;
  const sum = X + Y + Z;
  if (sum === 0) return [20577, 21609]; // D65 white point
  return [Math.round((X / sum) * 65279), Math.round((Y / sum) * 65279)];
}

// ---------- commands ----------

const SEND = { disableDefaultResponse: true, timeout: 3000 };

function needDevice(ieee) {
  if (!controller) throw new Error("Coordinator not running");
  const d = controller.getDeviceByIeeeAddr(ieee);
  if (!d) throw new Error(`Unknown device ${ieee}`);
  const ep = lightEndpoint(d);
  if (!ep) throw new Error(`${ieee} has no light endpoint`);
  return { d, ep };
}

const handlers = {
  async ports() {
    const all = await findAllDevices();
    // Hide the Mac's built-in ports; prefer the cu.* device (tty.* blocks on open).
    return all
      .filter((p) => !/Bluetooth|debug-console|wlan-debug/i.test(p.path))
      .map((p) => ({
        name: p.name && !p.name.includes("undefined") ? p.name : p.path.split("/").pop(),
        path: p.path.replace("/dev/tty.", "/dev/cu."),
        adapter: p.adapter ?? null,
      }));
  },

  async start({ port, adapter, data_dir, network }) {
    if (controller) await handlers.stop();
    mkdirSync(data_dir, { recursive: true });
    controller = new Controller({
      network: {
        panID: network.pan_id,
        extendedPanID: network.extended_pan_id,
        channelList: [network.channel],
        networkKey: network.network_key,
      },
      transport: { path: port, adapter: adapter === "auto" ? undefined : adapter },
      databasePath: join(data_dir, "database.db"),
      databaseBackupPath: join(data_dir, "database.db.backup"),
      backupPath: join(data_dir, "coordinator_backup.json"),
      adapter: { disableLED: false, concurrent: 16 },
      acceptJoiningDeviceHandler: async () => true,
    });
    controller.on("deviceJoined", ({ device }) => {
      out({ event: "device_joined", device: describe(device) });
      pushDevices();
    });
    controller.on("deviceInterview", async ({ status, device }) => {
      if (status === "successful") {
        // Learn which colour modes the bulb supports.
        const ep = lightEndpoint(device);
        if (ep?.supportsInputCluster("lightingColorCtrl")) {
          try { await ep.read("lightingColorCtrl", ["colorCapabilities"]); } catch {}
        }
      }
      out({ event: "device_interview", status, device: describe(device) });
      pushDevices();
    });
    controller.on("deviceLeave", ({ ieeeAddr }) => {
      out({ event: "device_left", ieee: ieeeAddr });
      pushDevices();
    });
    controller.on("deviceAnnounce", pushDevices);
    controller.on("permitJoinChanged", ({ permitted, time }) => out({ event: "permit_join", permitted, time: time ?? 0 }));
    controller.on("adapterDisconnected", () => {
      out({ event: "adapter_disconnected" });
      controller = null;
    });
    const result = await controller.start();
    const version = await controller.getCoordinatorVersion().catch(() => null);
    const params = await controller.getNetworkParameters().catch(() => null);
    startInfo = { result, version, params };
    pushDevices();
    return startInfo;
  },

  async stop() {
    if (!controller) return null;
    const c = controller;
    controller = null;
    try { await c.stop(); } catch (e) { log("warn", `stop: ${e}`); }
    return null;
  },

  async permit_join({ seconds }) {
    if (!controller) throw new Error("Coordinator not running");
    await controller.permitJoin(Math.max(0, Math.min(254, seconds | 0)));
    return { until: seconds ? Date.now() + seconds * 1000 : 0 };
  },

  async devices() {
    return devices();
  },

  /** {ieee, on, brightness?, rgb?, color_temp?, transition} */
  async set({ ieee, on, brightness, rgb, color_temp, transition }) {
    const { ep } = needDevice(ieee);
    const transtime = Math.round((transition ?? 0) * 10);
    if (!on) {
      if (ep.supportsInputCluster("genLevelCtrl") && transtime > 0) {
        await ep.command("genLevelCtrl", "moveToLevelWithOnOff", { level: 0, transtime, optionsMask: 0, optionsOverride: 0 }, SEND);
      } else {
        await ep.command("genOnOff", "off", {}, SEND);
      }
      return null;
    }
    const jobs = [];
    if (rgb && ep.supportsInputCluster("lightingColorCtrl")) {
      const [colorx, colory] = rgbToXy(rgb);
      jobs.push(ep.command("lightingColorCtrl", "moveToColor", { colorx, colory, transtime, optionsMask: 0, optionsOverride: 0 }, SEND));
    } else if (color_temp && ep.supportsInputCluster("lightingColorCtrl")) {
      jobs.push(ep.command("lightingColorCtrl", "moveToColorTemp", { colortemp: color_temp, transtime, optionsMask: 0, optionsOverride: 0 }, SEND));
    }
    if (brightness != null && ep.supportsInputCluster("genLevelCtrl")) {
      const level = Math.max(1, Math.min(254, brightness));
      jobs.push(ep.command("genLevelCtrl", "moveToLevelWithOnOff", { level, transtime, optionsMask: 0, optionsOverride: 0 }, SEND));
    } else {
      jobs.push(ep.command("genOnOff", "on", {}, SEND));
    }
    await Promise.all(jobs);
    return null;
  },

  async identify({ ieee }) {
    const { d, ep } = needDevice(ieee);
    const idEp = d.endpoints.find((e) => e.supportsInputCluster("genIdentify")) ?? ep;
    if (idEp.supportsInputCluster("genIdentify")) {
      await idEp.command("genIdentify", "identify", { identifytime: 5 }, SEND);
    } else {
      // Fallback: flash it off and on.
      await ep.command("genOnOff", "off", {}, SEND);
      setTimeout(() => ep.command("genOnOff", "on", {}, SEND).catch(() => {}), 700);
    }
    return null;
  },

  async interview({ ieee }) {
    const { d } = needDevice(ieee);
    d.interview(true).then(pushDevices, (e) => log("warn", `interview ${ieee}: ${e}`));
    return null;
  },

  async remove({ ieee, force }) {
    if (!controller) throw new Error("Coordinator not running");
    const d = controller.getDeviceByIeeeAddr(ieee);
    if (!d) return null;
    if (force) d.removeFromDatabase();
    else await d.removeFromNetwork();
    pushDevices();
    return null;
  },
};

// ---------- main loop ----------

const rl = createInterface({ input: process.stdin });
rl.on("line", (line) => {
  let msg;
  try { msg = JSON.parse(line); } catch { return; }
  const h = handlers[msg.cmd];
  if (!h) { out({ id: msg.id, ok: false, error: `unknown command ${msg.cmd}` }); return; }
  Promise.resolve()
    .then(() => h(msg))
    .then((result) => msg.id != null && out({ id: msg.id, ok: true, result: result ?? null }))
    .catch((e) => msg.id != null && out({ id: msg.id, ok: false, error: String(e?.message ?? e) }));
});
rl.on("close", async () => {
  await handlers.stop();
  process.exit(0);
});
for (const sig of ["SIGTERM", "SIGINT"]) process.on(sig, async () => { await handlers.stop(); process.exit(0); });
process.on("unhandledRejection", (e) => log("error", `unhandled: ${e?.stack ?? e}`));
out({ event: "hello", version: "0.1.0" });
