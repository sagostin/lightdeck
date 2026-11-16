// Genre presets: a complete detection setup for a style of music (every band it uses, with
// all of its settings, plus global sensitivity), and a note on how the genre is usually lit
// (sent to the AI designer). Applying one is previewed first: it shows what changes, never
// removes your other bands unless you ask, and switches off routes that would lose their band.
import type { BandDef, Show } from "./types";
import { clone } from "./clone";

type Base = Omit<BandDef, "id">;
const band = (name: string, ranges: [number, number][], detect: BandDef["detect"], sensitivity: number, min_gap_ms: number, release: number, color: string): Base =>
  ({ name, enabled: true, ranges: ranges.map(([lo_hz, hi_hz]) => ({ lo_hz, hi_hz, weight: 1 })), detect, level_threshold: 0.6, gain_db: 0, sensitivity, min_gap_ms, release, color });

/** Starting values for every band a genre can use (the app's defaults). */
export const BASE_BANDS: Record<string, Base> = {
  kick: band("Kick", [[35, 150]], "transient", 0.5, 140, 0.85, "#ff5c6c"),
  bass: band("Bass", [[60, 250]], "level", 0.35, 220, 0.92, "#ff9a4d"),
  snare: band("Snare", [[150, 250], [1800, 5000]], "transient", 0.5, 140, 0.85, "#f2b54a"),
  vocals: band("Vocals", [[300, 3000]], "level", 0.3, 250, 0.93, "#45d39a"),
  hat: band("Hi-hat", [[6000, 16000]], "transient", 0.5, 70, 0.8, "#4fb3ff"),
  clap: band("Clap", [[1000, 4000]], "transient", 0.5, 140, 0.8, "#ffd27a"),
  sub_bass: band("Sub-bass", [[20, 60]], "level", 0.3, 250, 0.93, "#c45cff"),
  lead_synth: band("Lead synth", [[800, 5000]], "level", 0.4, 180, 0.9, "#4fe0d0"),
};
const STANDARD = ["kick", "bass", "snare", "vocals", "hat"];

export interface Genre {
  id: string;
  name: string;
  bpm: [number, number];
  /** How the genre tends to be lit. */
  style: string;
  sensitivity: number;
  /** Band ids the genre uses, in order (kick first: it drives the beat clock). */
  use: string[];
  /** Changes from BASE_BANDS for this genre. */
  tweaks: Record<string, Partial<Base>>;
}

export const GENRES: Genre[] = [
  {
    id: "house", name: "House", bpm: [118, 128], sensitivity: 0.5, use: [...STANDARD, "clap"],
    style: "Four-on-the-floor groove. Warm, saturated palettes that step on the bar; kick pulses brightness without going dark; claps and hats drive twinkle; drops are joyful rather than aggressive, strobe used sparingly.",
    tweaks: { kick: { sensitivity: 0.55, min_gap_ms: 220 }, snare: { sensitivity: 0.45 }, hat: { min_gap_ms: 90 }, clap: { min_gap_ms: 200 } },
  },
  {
    id: "techno", name: "Techno", bpm: [125, 140], sensitivity: 0.5, use: STANDARD,
    style: "Dark and minimal: mostly white, red and cold blue, lots of black space. Tight, hard kick flashes, hypnotic slow waves, long energy-driven builds, white strobes and lasers on peaks.",
    tweaks: { kick: { sensitivity: 0.6, min_gap_ms: 190, release: 0.8 }, bass: { sensitivity: 0.3 }, snare: { sensitivity: 0.4 }, hat: { sensitivity: 0.55, min_gap_ms: 60, release: 0.75 }, vocals: { sensitivity: 0.25 } },
  },
  {
    id: "trance", name: "Trance", bpm: [132, 142], sensitivity: 0.5, use: [...STANDARD, "lead_synth"],
    style: "Euphoric: long breakdowns with lead-synth and vocal swells, massive builds tracked by energy, bright saturated colour and lasers on the drop, rolling waves.",
    tweaks: { kick: { sensitivity: 0.55, min_gap_ms: 190 }, vocals: { sensitivity: 0.35, release: 0.95 }, hat: { min_gap_ms: 70 }, lead_synth: { release: 0.94 } },
  },
  {
    id: "dnb", name: "Drum & bass", bpm: [165, 178], sensitivity: 0.55, use: STANDARD,
    style: "Fast breakbeats: the snare on 2 and 4 matters more than the kick. Rapid colour steps and short flashes, bass level modulates brightness, hectic strobe and laser drops, fast waves.",
    tweaks: { kick: { min_gap_ms: 110, release: 0.8 }, snare: { sensitivity: 0.6, min_gap_ms: 120, release: 0.8 }, bass: { sensitivity: 0.4, release: 0.88 }, hat: { sensitivity: 0.45, min_gap_ms: 50, release: 0.75 } },
  },
  {
    id: "dubstep", name: "Dubstep / bass", bpm: [138, 150], sensitivity: 0.55, use: [...STANDARD, "sub_bass"],
    style: "Half-time and heavy: wobbling sub and bass level drives brightness, huge snare hits on 3, aggressive strobes and laser on drops, near-dark breakdowns.",
    tweaks: { kick: { min_gap_ms: 200 }, snare: { sensitivity: 0.6, min_gap_ms: 250 }, bass: { sensitivity: 0.45, release: 0.88 }, sub_bass: { sensitivity: 0.4, release: 0.9 } },
  },
  {
    id: "hiphop", name: "Hip-hop / R&B", bpm: [80, 100], sensitivity: 0.5, use: STANDARD,
    style: "Laid-back groove: kick and snare accents, vocals lead, warm golds and purples, slow colour changes, very little strobe.",
    tweaks: { kick: { min_gap_ms: 260 }, snare: { sensitivity: 0.55, min_gap_ms: 250 }, vocals: { sensitivity: 0.4 } },
  },
  {
    id: "latin", name: "Reggaeton / Latin", bpm: [88, 102], sensitivity: 0.5, use: [...STANDARD, "clap"],
    style: "Dembow rhythm: snare/clap on the off-beat pattern drives colour steps, hot saturated palettes (red, orange, magenta), playful node hops, short strobe hits.",
    tweaks: { kick: { sensitivity: 0.55, min_gap_ms: 220 }, snare: { sensitivity: 0.6, min_gap_ms: 150 }, vocals: { sensitivity: 0.35 }, clap: { sensitivity: 0.55, min_gap_ms: 150 } },
  },
  {
    id: "pop", name: "Pop / Top 40", bpm: [95, 130], sensitivity: 0.5, use: STANDARD,
    style: "Colourful and sing-along: vocal level drives brightness, rainbow and pastel palettes, colour steps on bars, choruses get big moments with strobe and lasers.",
    tweaks: { vocals: { sensitivity: 0.4 } },
  },
  {
    id: "rock", name: "Rock / live band", bpm: [100, 170], sensitivity: 0.45, use: STANDARD,
    style: "Real drums and cymbals: snare and kick hits, cymbal crashes flash white, follow the singer with warm amber/white, colour changes per section not per beat.",
    tweaks: { kick: { sensitivity: 0.45 }, snare: { sensitivity: 0.55, min_gap_ms: 120 }, hat: { name: "Cymbals", sensitivity: 0.35, min_gap_ms: 120, release: 0.88 }, vocals: { sensitivity: 0.35 } },
  },
  {
    id: "ambient", name: "Ambient / chill", bpm: [60, 110], sensitivity: 0.35, use: STANDARD,
    style: "Slow and soft: long palette fades, slow waves over many beats, gentle twinkle, no strobes and no flashes, lasers only on calm static looks.",
    tweaks: { kick: { sensitivity: 0.35, min_gap_ms: 300 }, hat: { sensitivity: 0.3, min_gap_ms: 200 }, bass: { release: 0.95 }, vocals: { release: 0.95 } },
  },
];

/** The genre's bands, complete, in order. */
export function genreBands(g: Genre): BandDef[] {
  return g.use.map((id) => ({ ...clone(BASE_BANDS[id]), ...clone(g.tweaks[id] ?? {}), id }));
}

// ---------- previewing and applying ----------

export interface TuneOptions {
  /** Keep your frequency ranges and gains on bands you already have (they depend on your room and mic more than the music). */
  keepRanges: boolean;
  /** Remove your bands the genre doesn't use (routes that used them are switched off). */
  removeOthers: boolean;
}
export const DEFAULT_TUNE: TuneOptions = { keepRanges: true, removeOthers: false };

export interface FieldChange { field: string; from: string; to: string }
export interface BandChange { id: string; name: string; status: "add" | "update" | "same"; fields: FieldChange[] }
export interface Usage { routes: string[]; controls: string[] }

export interface TunePlan {
  genre: Genre;
  /** The bands after applying, in order. */
  bands: BandDef[];
  sensitivity: { from: number; to: number };
  changes: BandChange[];
  /** Your bands the genre doesn't use: kept, or removed with `removeOthers`. */
  others: (BandDef & Usage)[];
  removed: (BandDef & Usage)[];
  /** Kick (or the genre's first band) becomes the first band, which drives the beat clock. */
  reordered: boolean;
  /** Existing settings would be overwritten (worth confirming). */
  overwrites: boolean;
  /** Nothing would change. */
  same: boolean;
}

const COMPARED: { field: keyof BandDef; label: string; fmt: (b: BandDef) => string; range?: boolean }[] = [
  { field: "detect", label: "detection", fmt: (b) => b.detect },
  { field: "sensitivity", label: "sensitivity", fmt: (b) => `${Math.round(b.sensitivity * 100)}%` },
  { field: "min_gap_ms", label: "min gap", fmt: (b) => `${b.min_gap_ms} ms` },
  { field: "release", label: "release", fmt: (b) => b.release.toFixed(2) },
  { field: "level_threshold", label: "level line", fmt: (b) => `${Math.round(b.level_threshold * 100)}%` },
  { field: "ranges", label: "ranges", fmt: (b) => b.ranges.map((r) => `${Math.round(r.lo_hz)}–${Math.round(r.hi_hz)}`).join(" + "), range: true },
  { field: "gain_db", label: "gain", fmt: (b) => `${b.gain_db} dB`, range: true },
];

/** Routes and controls that use a band. */
export function bandUsage(show: Show, id: string): Usage {
  return {
    routes: show.routes.filter((r) => r.source === id || r.source === `${id}.level`).map((r) => r.name || r.id),
    controls: show.controls.filter((c) => (c.action.type === "band_sensitivity" && c.action.band === id) || (c.action.type === "bands" && c.action.bands.includes(id))).map((c) => c.name || c.id),
  };
}

/** What applying `g` would change. Nothing is modified. */
export function planGenre(show: Show, g: Genre, opts: TuneOptions = DEFAULT_TUNE): TunePlan {
  const target = genreBands(g);
  const changes: BandChange[] = [];
  const bands: BandDef[] = target.map((t) => {
    const cur = show.audio.bands.find((b) => b.id === t.id);
    if (!cur) { changes.push({ id: t.id, name: t.name, status: "add", fields: [] }); return t; }
    // Your name, colour and on/off stay; ranges and gain too unless asked.
    const next: BandDef = { ...t, name: cur.name, color: cur.color, enabled: cur.enabled };
    if (opts.keepRanges) { next.ranges = clone(cur.ranges); next.gain_db = cur.gain_db; }
    const fields = COMPARED
      .filter((f) => JSON.stringify(cur[f.field]) !== JSON.stringify(next[f.field]))
      .map((f) => ({ field: f.label, from: f.fmt(cur), to: f.fmt(next) }));
    changes.push({ id: t.id, name: cur.name, status: fields.length ? "update" : "same", fields });
    return next;
  });
  const others = show.audio.bands.filter((b) => !g.use.includes(b.id)).map((b) => ({ ...clone(b), ...bandUsage(show, b.id) }));
  const removed = opts.removeOthers ? others : [];
  const result = [...bands, ...(opts.removeOthers ? [] : others.map(({ routes: _r, controls: _c, ...b }) => b))];
  const reordered = JSON.stringify(show.audio.bands.map((b) => b.id).filter((id) => result.some((r) => r.id === id)))
    !== JSON.stringify(result.map((b) => b.id).filter((id) => show.audio.bands.some((b) => b.id === id)));
  const overwrites = changes.some((c) => c.status === "update") || removed.length > 0;
  const sensitivity = { from: show.audio.sensitivity, to: g.sensitivity };
  return {
    genre: g, bands: result, sensitivity, changes, others, removed, reordered, overwrites,
    same: !overwrites && !changes.some((c) => c.status === "add") && !reordered && sensitivity.from === sensitivity.to,
  };
}

export interface TuneResult { changed: string[]; added: string[]; removed: string[]; routesOff: string[] }

/** Apply a previewed plan. Routes that used a removed band are switched off (the Routes screen flags them). */
export function applyTune(show: Show, plan: TunePlan): TuneResult {
  show.audio.bands = clone(plan.bands);
  show.audio.sensitivity = plan.sensitivity.to;
  const gone = new Set(plan.removed.map((b) => b.id));
  const routesOff: string[] = [];
  for (const r of show.routes) {
    const band = r.source.endsWith(".level") ? r.source.slice(0, -6) : r.source;
    if (gone.has(band) && r.enabled) { r.enabled = false; routesOff.push(r.name || r.id); }
  }
  return {
    changed: plan.changes.filter((c) => c.status === "update").map((c) => c.name),
    added: plan.changes.filter((c) => c.status === "add").map((c) => c.name),
    removed: plan.removed.map((b) => b.name),
    routesOff,
  };
}

/** Plan and apply in one go with the safe defaults (used when building designs). */
export function applyGenre(show: Show, g: Genre): string[] {
  const r = applyTune(show, planGenre(show, g));
  return [...r.changed, ...r.added.map((a) => `+${a}`)];
}

/** The genre the show's detection already matches (with your ranges), if any. */
export function currentGenre(show: Show): Genre | null {
  return GENRES.find((g) => {
    const p = planGenre(show, g);
    return !p.changes.some((c) => c.status !== "same") && p.sensitivity.from === p.sensitivity.to;
  }) ?? null;
}

/** One-line summary of a tune plan. */
export function describeTune(p: TunePlan): string {
  if (p.same) return `Detection already matches ${p.genre.name}`;
  const n = (k: number, w: string) => `${k} ${w}${k === 1 ? "" : "s"}`;
  const upd = p.changes.filter((c) => c.status === "update").length, add = p.changes.filter((c) => c.status === "add").length;
  return [upd && `${n(upd, "band")} retuned`, add && `${n(add, "band")} added`, p.removed.length && `${n(p.removed.length, "band")} removed`,
    p.reordered && "reordered", p.sensitivity.from !== p.sensitivity.to && "sensitivity changed"].filter(Boolean).join(", ");
}

/** A route whose sound is a band that doesn't exist any more (it never fires). */
export function missingBand(show: Show, source: string): boolean {
  if (["beat", "bar", "level", "energy", "phase"].includes(source)) return false;
  const band = source.endsWith(".level") ? source.slice(0, -6) : source;
  return !show.audio.bands.some((b) => b.id === band);
}

/** Remove a band; routes that used it are switched off (and flagged as missing their band). */
export function removeBand(show: Show, id: string): string[] {
  show.audio.bands = show.audio.bands.filter((b) => b.id !== id);
  const off: string[] = [];
  for (const r of show.routes) if (r.enabled && missingBand(show, r.source)) { r.enabled = false; off.push(r.name || r.id); }
  return off;
}
