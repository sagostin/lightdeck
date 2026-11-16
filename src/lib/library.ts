// Fixture library: fixture types (with their looks) kept app-wide. Projects hold their own
// copy of each type, so a project file always works on its own and editing the library
// never changes an existing show by surprise. Copying either way merges looks: new and
// changed looks come across, looks that exist only on the receiving side are kept.
import type { Profile } from "./types";
import { clone } from "./clone";

/** Everything about a type except its looks (channels, tags, blocked values, modes…). */
export function definition(p: Profile) {
  const { id: _id, looks: _l, look_sets: _s, ...def } = p;
  return def;
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

export interface Diff {
  /** Channels, tags, modes, names or notes differ. */
  definition: boolean;
  /** Looks only in `other`, only in `mine`, and in both but different. */
  incoming: number;
  local: number;
  changed: number;
  /** Nothing differs. */
  same: boolean;
}

/** How `other` differs from `mine`, from `mine`'s point of view. */
export function compare(mine: Profile, other: Profile): Diff {
  const byId = (p: Profile) => new Map<string, unknown>([...(p.looks ?? []).map((l): [string, unknown] => [`l:${l.id}`, l]), ...(p.look_sets ?? []).map((s): [string, unknown] => [`s:${s.id}`, s])]);
  const a = byId(mine), b = byId(other);
  let incoming = 0, local = 0, changed = 0;
  for (const [k, v] of b) { if (!a.has(k)) incoming++; else if (!same(a.get(k), v)) changed++; }
  for (const k of a.keys()) if (!b.has(k)) local++;
  const def = !same(definition(mine), definition(other));
  return { definition: def, incoming, local, changed, same: !def && !incoming && !changed && !local };
}

export interface MergeResult { added: number; updated: number; kept: number; definition: boolean }

/**
 * Copy `source` into `target`: the definition becomes the source's, looks and look sets
 * are merged by id (source wins for ones in both), and target-only ones are kept.
 */
export function mergeInto(target: Profile, source: Profile): MergeResult {
  const src = clone(source);
  const defChanged = !same(definition(target), definition(src));
  Object.assign(target, definition(src));
  const merge = <T extends { id: string }>(mine: T[], theirs: T[]) => {
    let added = 0, updated = 0;
    for (const t of theirs) {
      const i = mine.findIndex((m) => m.id === t.id);
      if (i < 0) { mine.push(t); added++; }
      else if (!same(mine[i], t)) { mine[i] = t; updated++; }
    }
    return { added, updated };
  };
  const theirLooks = new Set(src.looks.map((l) => l.id));
  const kept = target.looks.filter((l) => !theirLooks.has(l.id)).length;
  const looks = merge(target.looks, src.looks);
  merge(target.look_sets, src.look_sets);
  return { added: looks.added, updated: looks.updated, kept, definition: defChanged };
}

/** Save a project's type to the library (adding it, or merging into the library's copy). */
export function saveToLibrary(library: Profile[], p: Profile): { library: Profile[]; result: MergeResult | null } {
  const lib = clone(library);
  const entry = lib.find((x) => x.id === p.id);
  if (!entry) { lib.push(clone(p)); return { library: lib, result: null }; }
  return { library: lib, result: mergeInto(entry, p) };
}

/** Add a type to a project's list: as itself, or as a copy with a new id if the id is taken. */
export function addCopy(profiles: Profile[], p: Profile, suffix = Math.random().toString(36).slice(2, 6)): Profile {
  const c = clone(p);
  if (profiles.some((x) => x.id === c.id)) {
    c.id = `${c.id}-${suffix}`;
    c.model = `${c.model} (copy)`;
  }
  profiles.push(c);
  return c;
}

/** One-line description of a diff for the UI. */
export function describeDiff(d: Diff, other: string): string {
  if (d.same) return `Same as the ${other}`;
  const parts = [
    d.definition && "channels, tags or details differ",
    d.incoming && `${d.incoming} look${d.incoming === 1 ? "" : "s"} only in the ${other}`,
    d.local && `${d.local} only here`,
    d.changed && `${d.changed} changed`,
  ].filter(Boolean);
  return parts.join(" · ");
}
