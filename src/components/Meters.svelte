<script lang="ts">
  import { app } from "../lib/store.svelte";

  const f = $derived(app.frame?.features);
  const bands = $derived(app.show?.audio.bands ?? []);
  // flash lamps when hit counters change
  let lampAt = $state<Record<string, number>>({});
  let last: Record<string, number> = {};
  $effect(() => {
    if (!f) return;
    const now = performance.now();
    for (const b of f.bands) {
      if (last[b.id] !== undefined && b.hits !== last[b.id]) lampAt[b.id] = now;
      last[b.id] = b.hits;
    }
  });
  const lit = (id: string) => (app.frame ? performance.now() - (lampAt[id] ?? 0) < 120 : false);
  const out = (id: string) => f?.bands.find((b) => b.id === id);
</script>

<div class="col">
  <div class="row lamps">
    {#each bands as b (b.id)}
      <span class="lamp" class:lit={lit(b.id)} style="--c:{b.color}">{b.name}</span>
    {/each}
  </div>
  {#each bands as b (b.id)}
    <div class="row"><span class="lbl muted">{b.name}</span>
      <div class="meter grow"><i style="width:{(out(b.id)?.level ?? 0) * 100}%;background:{b.color}"></i></div></div>
  {/each}
  <div class="row"><span class="lbl muted">Level</span><div class="meter grow"><i style="width:{(f?.level ?? 0) * 100}%"></i></div></div>
  <div class="row"><span class="lbl muted">Energy</span><div class="meter grow"><i style="width:{Math.min(1, (f?.energy ?? 0) / 2) * 100}%"></i></div></div>
</div>

<style>
  .lbl { width: 54px; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .lamps { flex-wrap: wrap; margin-bottom: 4px; gap: 4px; }
  .lamp { flex: 1; min-width: 56px; text-align: center; padding: 6px 0; border-radius: 6px; background: var(--panel-2); color: var(--faint); font-size: 12px; border: 1px solid var(--line); transition: background 80ms; }
  .lamp.lit { background: var(--c); color: #0d0e13; border-color: var(--c); }
</style>
