// Plain helpers for reading the rig (no app state), shared by auto-mapping and setup templates.
import type { Fixture, Profile, Show } from "./types";

export function profileFor(show: Show, fx: Fixture): Profile | undefined {
  return show.profiles.find((p) => p.id === fx.profile_id);
}

/** Channel types ("intensity", "color", "fog"…) the fixture's mode has. */
export function channelTypes(show: Show, fx: Fixture): Set<string> {
  const p = profileFor(show, fx);
  const mode = p?.modes.find((m) => m.name === fx.mode) ?? p?.modes[0];
  return new Set((mode?.channels ?? []).map((k) => p!.channels.find((c) => c.key === k)?.group ?? "").filter(Boolean));
}

/** Can colour/brightness effects (palette, wave, twinkle, strobe) drive it? */
export const canGlow = (show: Show, fx: Fixture) => {
  const t = channelTypes(show, fx);
  return t.has("intensity") || t.has("color");
};
export const hasLooks = (show: Show, fx: Fixture) => (profileFor(show, fx)?.looks.length ?? 0) > 0;
export const hasType = (show: Show, fx: Fixture, type: string) => channelTypes(show, fx).has(type);

export const activeFixtures = (show: Show) => show.fixtures.filter((f) => f.enabled !== false);

export function groupMembers(show: Show, groupId: string): Fixture[] {
  const g = show.groups.find((x) => x.id === groupId);
  return g ? show.fixtures.filter((f) => g.fixtures.includes(f.id) && f.enabled !== false) : [];
}

/** Look tags available across fixtures with looks, most used first. */
export function lookTags(show: Show): string[] {
  const count = new Map<string, number>();
  for (const f of activeFixtures(show)) for (const l of profileFor(show, f)?.looks ?? []) for (const t of l.tags) count.set(t, (count.get(t) ?? 0) + 1);
  return [...count.entries()].sort((a, b) => b[1] - a[1]).map(([t]) => t);
}
