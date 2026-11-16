<script lang="ts">
  import { app } from "../lib/store.svelte";
  let { value = $bindable(""), allowNone = false }: { value: string; allowNone?: boolean } = $props();
  const palettes = $derived(app.show!.palettes);
  const cur = $derived(palettes.find((p) => p.id === value));
</script>

<span class="ps">
  <select bind:value>
    {#if allowNone}<option value="">No colour</option>{/if}
    {#each palettes as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
  </select>
  {#if cur}<span class="sw">{#each cur.colors as c}<i style="background:{c}"></i>{/each}</span>{/if}
</span>

<style>
  .ps { display: inline-flex; align-items: center; gap: 6px; }
  .sw { display: inline-flex; border-radius: 4px; overflow: hidden; border: 1px solid var(--line-2); }
  .sw i { width: 12px; height: 18px; }
</style>
