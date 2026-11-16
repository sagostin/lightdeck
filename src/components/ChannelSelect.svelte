<script lang="ts">
  // Channel choices for a target: channel types across every fixture first, then each fixture type's channels.
  import { app, modeChannels, profileOf, routeFixtures } from "../lib/store.svelte";
  import { GROUP_LABEL, type Target } from "../lib/types";

  let { target, channel = $bindable("") }: { target: Target; channel: string } = $props();

  const opts = $derived.by(() => {
    const fx = routeFixtures(target);
    const types = new Map<string, number>();
    const byProfile = new Map<string, { label: string; opts: Map<string, { name: string; count: number }> }>();
    for (const f of fx) {
      const p = profileOf(f);
      if (!p) continue;
      const chans = modeChannels(f);
      for (const g of new Set(chans.map((c) => c.group))) types.set(g, (types.get(g) ?? 0) + 1);
      const entry = byProfile.get(p.id) ?? { label: `${p.manufacturer} ${p.model}`.trim(), opts: new Map() };
      byProfile.set(p.id, entry);
      for (const c of chans) {
        const o = entry.opts.get(c.key) ?? { name: c.name, count: 0 };
        o.count++;
        entry.opts.set(c.key, o);
      }
    }
    const typeOpts = [...types.entries()]
      .filter(([g]) => g !== "mode" && g !== "other")
      .map(([g, n]) => ({ value: `@${g}`, label: `All ${(GROUP_LABEL[g] ?? g).toLowerCase()} channels`, count: n }));
    return { total: fx.length, typeOpts, profiles: [...byProfile.values()] };
  });
  const known = $derived(opts.typeOpts.some((o) => o.value === channel) || opts.profiles.some((p) => p.opts.has(channel)));
  void app;
</script>

<select bind:value={channel} class:missing={!channel}>
  <option value="">Choose a channel…</option>
  {#if opts.typeOpts.length}
    <optgroup label="Every fixture's…">
      {#each opts.typeOpts as o}<option value={o.value}>{o.label}{o.count < opts.total ? ` (${o.count} of ${opts.total})` : ""}</option>{/each}
    </optgroup>
  {/if}
  {#each opts.profiles as p}
    <optgroup label={p.label}>
      {#each [...p.opts.entries()] as [k, o]}<option value={k}>{o.name}{o.count < opts.total ? ` (${o.count} of ${opts.total})` : ""}</option>{/each}
    </optgroup>
  {/each}
  {#if channel && !known}<option value={channel}>{channel} (not on these fixtures)</option>{/if}
</select>

<style>
  select.missing { border-color: #f2b54a88; }
</style>
