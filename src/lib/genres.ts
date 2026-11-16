// Genre presets: detection tuning for a style of music, plus a short note on how it's
// usually lit (sent to the AI designer as guidance). Tuning patches the standard bands
// by id (kick, bass, snare, vocals, hat) and adds a few extra bands where a genre needs
// them. Every value stays editable afterwards in the Audio view.
import { BAND_PRESETS, type BandDef, type Show } from "./types";

export interface Genre {
  id: string;
  name: string;
  bpm: [number, number];
  /** How the genre tends to be lit. */
  style: string;
  sensitivity: number;
  bands: Record<string, Partial<BandDef>>;
  /** Extra bands to add (by BAND_PRESETS name) if the show doesn't have them. */
  add?: { preset: string; patch?: Partial<BandDef> }[];
}

export const GENRES: Genre[] = [
  {
    id: "house", name: "House", bpm: [118, 128], sensitivity: 0.5,
    style: "Four-on-the-floor groove. Warm, saturated palettes that step on the bar; kick pulses brightness without going dark; claps and hats drive twinkle; drops are joyful rather than aggressive, strobe used sparingly.",
    bands: { kick: { sensitivity: 0.55, min_gap_ms: 220 }, bass: { sensitivity: 0.35 }, snare: { sensitivity: 0.45 }, hat: { sensitivity: 0.5, min_gap_ms: 90 } },
    add: [{ preset: "Clap", patch: { sensitivity: 0.5, min_gap_ms: 200 } }],
  },
  {
    id: "techno", name: "Techno", bpm: [125, 140], sensitivity: 0.5,
    style: "Dark and minimal: mostly white, red and cold blue, lots of black space. Tight, hard kick flashes, hypnotic slow waves, long energy-driven builds, white strobes and lasers on peaks.",
    bands: { kick: { sensitivity: 0.6, min_gap_ms: 190 }, bass: { sensitivity: 0.3 }, snare: { sensitivity: 0.4 }, hat: { sensitivity: 0.55, min_gap_ms: 60 }, vocals: { sensitivity: 0.25 } },
  },
  {
    id: "trance", name: "Trance", bpm: [132, 142], sensitivity: 0.5,
    style: "Euphoric: long breakdowns with lead-synth and vocal swells, massive builds tracked by energy, bright saturated colour and lasers on the drop, rolling waves.",
    bands: { kick: { sensitivity: 0.55, min_gap_ms: 190 }, vocals: { sensitivity: 0.35 }, hat: { sensitivity: 0.5, min_gap_ms: 70 } },
    add: [{ preset: "Lead synth" }],
  },
  {
    id: "dnb", name: "Drum & bass", bpm: [165, 178], sensitivity: 0.55,
    style: "Fast breakbeats: the snare on 2 and 4 matters more than the kick. Rapid colour steps and short flashes, bass level modulates brightness, hectic strobe and laser drops, fast waves.",
    bands: { kick: { sensitivity: 0.5, min_gap_ms: 110 }, snare: { sensitivity: 0.6, min_gap_ms: 120 }, bass: { sensitivity: 0.4, release: 0.88 }, hat: { sensitivity: 0.45, min_gap_ms: 50 } },
  },
  {
    id: "dubstep", name: "Dubstep / bass", bpm: [138, 150], sensitivity: 0.55,
    style: "Half-time and heavy: wobbling sub and bass level drives brightness, huge snare hits on 3, aggressive strobes and laser on drops, near-dark breakdowns.",
    bands: { kick: { sensitivity: 0.5, min_gap_ms: 200 }, snare: { sensitivity: 0.6, min_gap_ms: 250 }, bass: { sensitivity: 0.45, release: 0.88 } },
    add: [{ preset: "Sub-bass" }],
  },
  {
    id: "hiphop", name: "Hip-hop / R&B", bpm: [80, 100], sensitivity: 0.5,
    style: "Laid-back groove: kick and snare accents, vocals lead, warm golds and purples, slow colour changes, very little strobe.",
    bands: { kick: { sensitivity: 0.5, min_gap_ms: 260 }, snare: { sensitivity: 0.55, min_gap_ms: 250 }, vocals: { sensitivity: 0.4 } },
  },
  {
    id: "latin", name: "Reggaeton / Latin", bpm: [88, 102], sensitivity: 0.5,
    style: "Dembow rhythm: snare/clap on the off-beat pattern drives colour steps, hot saturated palettes (red, orange, magenta), playful node hops, short strobe hits.",
    bands: { kick: { sensitivity: 0.55, min_gap_ms: 220 }, snare: { sensitivity: 0.6, min_gap_ms: 150 }, vocals: { sensitivity: 0.35 } },
    add: [{ preset: "Clap", patch: { sensitivity: 0.55, min_gap_ms: 150 } }],
  },
  {
    id: "pop", name: "Pop / Top 40", bpm: [95, 130], sensitivity: 0.5,
    style: "Colourful and sing-along: vocal level drives brightness, rainbow and pastel palettes, colour steps on bars, choruses get big moments with strobe and lasers.",
    bands: { vocals: { sensitivity: 0.4 }, kick: { sensitivity: 0.5 }, snare: { sensitivity: 0.5 } },
  },
  {
    id: "rock", name: "Rock / live band", bpm: [100, 170], sensitivity: 0.45,
    style: "Real drums and cymbals: snare and kick hits, cymbal crashes flash white, follow the singer with warm amber/white, colour changes per section not per beat.",
    bands: { kick: { sensitivity: 0.45, min_gap_ms: 140 }, snare: { sensitivity: 0.55, min_gap_ms: 120 }, hat: { sensitivity: 0.35, min_gap_ms: 120, name: "Cymbals" }, vocals: { sensitivity: 0.35 } },
  },
  {
    id: "ambient", name: "Ambient / chill", bpm: [60, 110], sensitivity: 0.35,
    style: "Slow and soft: long palette fades, slow waves over many beats, gentle twinkle, no strobes and no flashes, lasers only on calm static looks.",
    bands: { kick: { sensitivity: 0.35, min_gap_ms: 300 }, hat: { sensitivity: 0.3, min_gap_ms: 200 }, bass: { release: 0.95 }, vocals: { release: 0.95 } },
  },
];

const slug = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "");

/** Tune the show's detection for a genre. Returns what changed, for a toast. */
export function applyGenre(show: Show, g: Genre): string[] {
  const changed: string[] = [];
  show.audio.sensitivity = g.sensitivity;
  for (const b of show.audio.bands) {
    const patch = g.bands[b.id];
    if (patch) { Object.assign(b, structuredClone(patch)); changed.push(b.name); }
  }
  for (const a of g.add ?? []) {
    const p = BAND_PRESETS.find((x) => x.name === a.preset);
    if (!p) continue;
    const id = slug(p.name);
    if (show.audio.bands.some((b) => b.id === id)) continue;
    show.audio.bands.push({ ...structuredClone(p), ...structuredClone(a.patch ?? {}), id } as BandDef);
    changed.push(`+${p.name}`);
  }
  return changed;
}
