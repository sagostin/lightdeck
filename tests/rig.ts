// A realistic club rig for the UI tests: the real bundled laser and fog profiles from the
// sample show, eight RGBW pars in two groups, a focus point, and the default bands.
import { readFileSync } from "node:fs";
import type { ControlMapping, Fixture, Profile, Route, Show } from "../src/lib/types.ts";
import type { Plan } from "../src/lib/design.ts";
import { canGlow, channelTypes, hasLooks } from "../src/lib/rig.ts";

const sample = JSON.parse(readFileSync(new URL("../src/dev/sample-show.json", import.meta.url), "utf8")) as Show;

const PAR: Profile = {
  id: "par", manufacturer: "Generic", model: "RGBW par", kind: "par", notes: "", looks: [], look_sets: [],
  channels: ["dimmer:intensity", "red:color", "green:color", "blue:color", "white:color", "strobe:strobe"].map((s) => {
    const [key, group] = s.split(":");
    return { key, name: key, group, default: 0, capabilities: [], values: [] };
  }),
  modes: [{ name: "6ch", channels: ["dimmer", "red", "green", "blue", "white", "strobe"], heads: [] }],
};

export function clubRig(opts: { looksTagged?: boolean; focus?: boolean; groups?: boolean } = {}): Show {
  const s = structuredClone(sample);
  const laser = s.profiles.find((p) => p.kind === "laser")!;
  const fog = s.profiles.find((p) => p.kind === "fog")!;
  if (opts.looksTagged) laser.looks[1].tags.push("peak");
  const fx = (id: string, profile: Profile, x: number): Fixture => ({
    id, name: id, profile_id: profile.id, mode: profile.modes[0].name, output_id: "", address: 1, room_id: "room",
    x, y: 1, z: 2, rotation: 0, enabled: true, zigbee_device: null, limits: {},
  } as unknown as Fixture);
  const pars = Array.from({ length: 8 }, (_, i) => fx(`par${i + 1}`, PAR, i));
  s.profiles = [laser, fog, PAR];
  s.fixtures = [...pars, fx("laser", laser, 4), fx("fog", fog, 0)];
  s.groups = opts.groups === false ? [] : [
    { id: "g-all", name: "All pars", color: "#fff", fixtures: pars.map((p) => p.id) },
    { id: "g-front", name: "Front", color: "#fff", fixtures: pars.slice(0, 4).map((p) => p.id) },
    { id: "g-back", name: "Back", color: "#fff", fixtures: pars.slice(4).map((p) => p.id) },
    { id: "g-laser", name: "Laser", color: "#fff", fixtures: ["laser"] },
  ];
  s.rooms = [{ id: "room", name: "Main", walls: [], ceiling_m: 3, focus: opts.focus === false ? [] : [{ id: "floor", name: "Floor", x: 4, y: 3, z: 1, radius: 3, color: "#fff" }] }];
  s.active_room = "room";
  s.routes = []; s.presets = []; s.controls = [];
  s.midi = { enabled: false, inputs: [] };
  return s;
}

/** Everything that would make the backend drop or fix part of a plan, as readable problems. */
export function planProblems(show: Show, plan: Plan): string[] {
  const out: string[] = [];
  const bands = new Set([...show.audio.bands, ...plan.bands].map((b) => b.id));
  const palettes = new Set([...show.palettes, ...plan.palettes].map((p) => p.id));
  const sourceOk = (s: string) => ["beat", "bar", "level", "energy", "phase"].includes(s) || bands.has(s) || (s.endsWith(".level") && bands.has(s.slice(0, -6)));
  const targets = (t: Pick<Route, "rooms" | "groups" | "fixtures">) => {
    const ids = new Set(t.fixtures);
    for (const g of show.groups) if (t.groups.includes(g.id)) g.fixtures.forEach((f) => ids.add(f));
    const all = !t.fixtures.length && !t.groups.length;
    return show.fixtures.filter((f) => (all || ids.has(f.id)) && (!t.rooms.length || t.rooms.includes(f.room_id ?? "")));
  };
  const channelOk = (f: Fixture, ch: string) => ch.startsWith("@") ? channelTypes(show, f).has(ch.slice(1))
    : !!show.profiles.find((p) => p.id === f.profile_id)?.modes[0].channels.includes(ch);
  for (const r of plan.routes) {
    const p = (m: string) => out.push(`route "${r.name}": ${m}`);
    if (!sourceOk(r.source)) p(`unknown source ${r.source}`);
    for (const g of r.groups) if (!show.groups.some((x) => x.id === g)) p(`unknown group ${g}`);
    for (const f of r.fixtures) if (!show.fixtures.some((x) => x.id === f)) p(`unknown fixture ${f}`);
    if (r.focus && !show.rooms.some((room) => room.focus.some((x) => x.id === r.focus))) p(`unknown focus ${r.focus}`);
    if ("palette" in r.action && r.action.palette && !palettes.has(r.action.palette)) p(`unknown palette ${r.action.palette}`);
    const t = targets(r);
    const type = r.action.type;
    if (["strobe", "palette", "twinkle", "wave"].includes(type)) { if (!t.some((f) => canGlow(show, f))) p("effect reaches no colour fixture"); }
    else if (type === "look") { if (!t.some((f) => hasLooks(show, f))) p("look route reaches no fixture with looks"); }
    else if (!t.some((f) => channelOk(f, r.channel))) p(`no target has channel ${r.channel}`);
    if (r.every < 1 || r.probability <= 0) p("never fires");
    // Modulate follows a level; triggered actions need hits (strobe and twinkle take either).
    const level = ["energy", "level", "phase"].includes(r.source) || r.source.endsWith(".level");
    if (type === "modulate" && !level) p(`modulate needs a level source, got ${r.source}`);
    if (["flash", "random", "node_hop", "cycle", "look", "palette", "wave"].includes(type) && level) p(`${type} needs a hit source, got ${r.source}`);
  }
  const routeIds = new Set([...show.routes, ...plan.routes].map((r) => r.id));
  const presetIds = new Set([...show.presets, ...plan.presets].map((x) => x.id));
  for (const pr of plan.presets) for (const id of Object.keys(pr.routes)) if (!routeIds.has(id)) out.push(`scene "${pr.name}": unknown route ${id}`);
  const seen = new Set<string>();
  for (const c of [...show.controls, ...plan.controls] as ControlMapping[]) {
    const k = c.trigger.source === "key" ? `key:${c.trigger.key}` : `midi:${c.trigger.kind}:${c.trigger.number}`;
    if (seen.has(k)) out.push(`control "${c.name}": trigger ${k} used twice`);
    seen.add(k);
    const a = c.action;
    if (a.type === "preset" && !presetIds.has(a.preset)) out.push(`control "${c.name}": unknown preset`);
    if (a.type === "routes" && (!a.routes.length || a.routes.some((r) => !routeIds.has(r)))) out.push(`control "${c.name}": bad routes`);
    if (a.type === "band_sensitivity" && !bands.has(a.band)) out.push(`control "${c.name}": unknown band`);
    if (a.type === "channel_value" && !targets(a.target).some((f) => channelOk(f, a.channel))) out.push(`control "${c.name}": channel reaches nothing`);
  }
  return out;
}
