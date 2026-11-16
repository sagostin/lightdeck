<script lang="ts">
  // Edit how one look channel moves: sweep / step / random / list / catalogue / follow.
  import { untrack } from "svelte";
  import Tags from "./Tags.svelte";
  import { app } from "../lib/store.svelte";
  import { sourceOptions, type ChannelDef, type Motion } from "../lib/types";

  let { motion, channel }: { motion: Motion; channel: ChannelDef } = $props();

  const usable = $derived(channel.capabilities.filter((c) => !c.tags.includes("blocked")));
  const levels = $derived(sourceOptions(app.show!.audio.bands).levels);
  const catTags = $derived([...new Set(channel.values.flatMap((v) => v.tags))].sort());
  const catCount = $derived(
    channel.values.filter((v) => !v.tags.includes("blocked") && motion.tags.every((t) => v.tags.includes(t))).length,
  );
  const needsRanges = $derived(["sweep", "step", "random", "follow"].includes(motion.kind));
  // starting text only; edits are parsed back into motion.values on change
  let listText = $state(untrack(() => motion.values.join(", ")));

  function addRange(r: [number, number]) {
    if (!motion.ranges.some(([a, b]) => a === r[0] && b === r[1])) motion.ranges.push(r);
  }
</script>

<div class="me">
  <div class="row flow">
    <select bind:value={motion.kind}>
      <option value="sweep">Sweep</option>
      <option value="step">Step</option>
      <option value="random">Random (seeded)</option>
      <option value="list">List of values</option>
      <option value="catalog">Catalogued values</option>
      <option value="follow">Follow audio level</option>
    </select>
    {#if motion.kind !== "follow"}
      <span class="lbl">every</span>
      <input type="number" min="0.25" step="0.25" bind:value={motion.period} />
      <select bind:value={motion.clock}>
        <option value="beat">beats</option><option value="time">seconds</option><option value="hit">route hits</option>
      </select>
    {/if}
    {#if motion.kind === "sweep"}
      <select bind:value={motion.shape}>
        <option value="pingpong">there and back</option><option value="up">up, then jump</option>
        <option value="down">down, then jump</option><option value="sine">smooth (sine)</option>
      </select>
    {:else if motion.kind === "step"}
      <span class="lbl">by</span><input type="number" min="1" max="255" bind:value={motion.step} />
    {:else if motion.kind === "random"}
      <span class="lbl">seed</span><input type="number" min="1" bind:value={motion.seed} />
      <button class="ghost" onclick={() => (motion.seed = Math.floor(Math.random() * 1e6) + 1)} title="Try a different repeatable sequence">New seed</button>
    {:else if motion.kind === "follow"}
      <select bind:value={motion.source}>{#each levels as l}<option value={l.id}>{l.label}</option>{/each}</select>
    {/if}
  </div>

  {#if needsRanges}
    <div class="row flow">
      <span class="lbl">ranges</span>
      {#each motion.ranges as r, i}
        <span class="rng">
          <input type="number" min="0" max="255" bind:value={r[0]} />–<input type="number" min="0" max="255" bind:value={r[1]} />
          {#if motion.ranges.length > 1}<button class="x" onclick={() => motion.ranges.splice(i, 1)} aria-label="Remove range">×</button>{/if}
        </span>
      {/each}
      <button class="ghost" onclick={() => motion.ranges.push([0, 255])}>+ range</button>
    </div>
    {#if usable.length > 1}
      <div class="row flow caps">
        <span class="lbl">add</span>
        {#each usable as c}
          <button class="cap" title="{c.min}–{c.max}" onclick={() => addRange([c.min, c.max])}>{c.label}</button>
        {/each}
        <button class="ghost" onclick={() => (motion.ranges = usable.map((c) => [c.min, c.max] as [number, number]))}>all usable</button>
      </div>
    {/if}
  {:else if motion.kind === "list"}
    <div class="row">
      <span class="lbl">values</span>
      <input class="grow" bind:value={listText} placeholder="12, 40, 41, 77"
        onchange={() => (motion.values = listText.split(/[,\s]+/).map(Number).filter((n) => Number.isInteger(n) && n >= 0 && n <= 255))} />
      <button class="ghost" onclick={() => { motion.values = channel.values.map((v) => v.value).sort((a, b) => a - b); listText = motion.values.join(", "); }}>
        from catalogue
      </button>
    </div>
  {:else if motion.kind === "catalog"}
    <div class="row flow">
      <span class="lbl">tagged</span><Tags bind:tags={motion.tags} suggestions={catTags} />
      <span class="small faint">{catCount} catalogued value{catCount === 1 ? "" : "s"} on {channel.name}, played in order</span>
    </div>
  {/if}
</div>

<style>
  .me { display: flex; flex-direction: column; gap: 6px; padding: 8px 10px; margin: 2px 0 6px 28px; border-left: 2px solid var(--accent); background: #ffffff05; border-radius: 0; }
  .flow { flex-wrap: wrap; gap: 6px; }
  .lbl { color: var(--faint); font-size: 12px; }
  input[type="number"] { width: 62px; height: 26px; }
  select { height: 26px; }
  .rng { display: inline-flex; align-items: center; gap: 2px; }
  .x { padding: 0 5px; font-size: 11px; }
  .caps .cap { font-size: 11px; padding: 2px 6px; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
