// A look belongs to the pattern (main option) it was made on: its pattern channel's value,
// else the first channel you can browse, else its first channel. These helpers find that and
// group looks under it, so lists show "P1 Pattern 48 · Double line" with its looks underneath.
import type { Look, Profile } from "./types";

/** Channel groups the pattern browser can browse as a main option. */
export const BROWSABLE = ["pattern", "effect", "beam", "color"];

/** The channel a look is built on. */
export function lookMainKey(p: Profile, l: Look): string | undefined {
  const keys = Object.keys(l.values);
  const group = (k: string) => p.channels.find((c) => c.key === k)?.group ?? "";
  return keys.find((k) => group(k) === "pattern") ?? keys.find((k) => BROWSABLE.includes(group(k))) ?? keys[0];
}

/** "P1 Pattern", "48", "Double line": the channel, the value, and its catalogue (or range) name. */
export function valueName(p: Profile, key: string, v: number) {
  const c = p.channels.find((x) => x.key === key);
  const label = c?.values.find((n) => n.value === v)?.label || c?.capabilities.find((x) => v >= x.min && v <= x.max)?.label || "";
  return { channel: c?.name ?? key, label };
}

export interface LookGroup {
  /** Same for the same channel name and value across fixture types, so groups merge. */
  id: string;
  channel: string;
  value: number;
  label: string;
  looks: Look[];
}

/** Looks grouped under their main channel and value, in channel order then value order. */
export function groupLooks(items: { profile: Profile; look: Look }[]): LookGroup[] {
  const m = new Map<string, LookGroup & { order: number }>();
  for (const { profile: p, look: l } of items) {
    const key = lookMainKey(p, l) ?? "";
    const value = l.values[key] ?? 0;
    const { channel, label } = valueName(p, key, value);
    const id = `${channel}:${value}`;
    const order = p.channels.findIndex((c) => c.key === key);
    const g = m.get(id) ?? { id, channel, value, label, looks: [], order: order < 0 ? 999 : order };
    g.looks.push(l);
    m.set(id, g);
  }
  return [...m.values()]
    .sort((a, b) => a.order - b.order || a.value - b.value)
    .map(({ order: _, ...g }) => ({ ...g, looks: [...g.looks].sort((a, b) => b.rating - a.rating || a.name.localeCompare(b.name)) }));
}
