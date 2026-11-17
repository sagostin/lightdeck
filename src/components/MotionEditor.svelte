<script lang="ts">
  // Edit how one look channel moves: sweep / step / random / list / catalogue / follow.
  import { untrack } from "svelte";
  import Tags from "./Tags.svelte";
  import Tip from "./Tip.svelte";
  import { app } from "../lib/store.svelte";
  import { sourceOptions, type ChannelDef, type Motion } from "../lib/types";

  let { motion, channel, inRoute = false, routeSource = "" }: { motion: Motion; channel: ChannelDef; inRoute?: boolean; routeSource?: string } = $props();

  const usable = $derived(channel.capabilities.filter((c) => !c.tags.includes("blocked")));
  const levels = $derived(sourceOptions(app.show!.audio.bands).levels);
  const hits = $derived(sourceOptions(app.show!.audio.bands).triggers);
  const routeSourceLabel = $derived([...hits, ...levels].find((s) => s.id === routeSource)?.label.toLowerCase() ?? "the route's sound");
  const usesHits = $derived(motion.clock === "hit" || !!motion.retrigger);
  const TIP = {
    kind: "Sweep glides through the ranges; Step jumps by a fixed amount; Random picks repeatable random values (same seed, same sequence); List plays your values in order; Catalogued plays the channel's catalogued values with these tags; Follow tracks an audio level.",
    every: "How long one sweep, or one step, takes: in beats (follows the tempo), seconds, or hits.",
    shape: "There and back = up then down; Up / Down = one way, then jump back; Smooth = an eased sine.",
    step: "How far each step moves the value.",
    seed: "The random sequence. The same seed always plays the same values, so a look looks the same each time.",
    follow: "The sound the channel follows: one band (kick, bass, vocals… as set up on the Audio screen) or the whole track's level, energy or beat phase. Silent = bottom of the range, full = top.",
    restart: "Start the motion over each time its sound hits, timed from that hit. E.g. zoom opening over one beat on every kick.",
    once: "One sweep or one pass through the steps, then hold the last value (until it restarts).",
    trigger: "The sound that counts hits for this channel and restarts it. Leave it on the route's own sound, or pick another: e.g. a kick flash whose zoom only restarts on each bar, or a pattern that steps on snares.",
    ranges: "The values the motion moves through. Several ranges are played in order, which skips the values in between (e.g. only the usable parts of a channel).",
    add: "The channel's named ranges from its fixture definition. Click to add one.",
  };
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
    </select><Tip text={TIP.kind} />
    {#if motion.kind !== "follow"}
      <span class="lbl">every</span><Tip text={TIP.every} />
      <input type="number" min="0.25" step="0.25" bind:value={motion.period} />
      <select bind:value={motion.clock}>
        <option value="beat">beats</option><option value="time">seconds</option><option value="hit">{inRoute ? "hits" : "route hits"}</option>
      </select>
    {/if}
    {#if motion.kind === "sweep"}
      <select bind:value={motion.shape}>
        <option value="pingpong">there and back</option><option value="up">up, then jump</option>
        <option value="down">down, then jump</option><option value="sine">smooth (sine)</option>
      </select><Tip text={TIP.shape} />
    {:else if motion.kind === "step"}
      <span class="lbl">by</span><Tip text={TIP.step} /><input type="number" min="1" max="255" bind:value={motion.step} />
    {:else if motion.kind === "random"}
      <span class="lbl">seed</span><Tip text={TIP.seed} /><input type="number" min="1" bind:value={motion.seed} />
      <button class="ghost" onclick={() => (motion.seed = Math.floor(Math.random() * 1e6) + 1)} title="Try a different repeatable sequence">New seed</button>
    {:else if motion.kind === "follow"}
      <span class="lbl">follows</span>
      <select bind:value={motion.source} title="The sound whose level drives this channel">
        <optgroup label="One band">{#each levels.filter((l) => l.id.endsWith(".level")) as l}<option value={l.id}>{l.label.replace(/ level$/, "")}</option>{/each}</optgroup>
        <optgroup label="The whole track">{#each levels.filter((l) => !l.id.endsWith(".level")) as l}<option value={l.id}>{l.label}</option>{/each}</optgroup>
        {#if motion.source && !levels.some((l) => l.id === motion.source)}<option value={motion.source}>{motion.source} (band removed)</option>{/if}
      </select><Tip text={TIP.follow} />
    {/if}
  </div>
  {#if motion.kind !== "follow"}
    <div class="row flow">
      {#if inRoute && motion.clock !== "hit"}
        <label class="chk">
          <input type="checkbox" checked={!!motion.retrigger} onchange={(e) => (motion.retrigger = e.currentTarget.checked)} /> restart on each hit
        </label><Tip text={TIP.restart} />
      {/if}
      <label class="chk">
        <input type="checkbox" checked={!!motion.once} onchange={(e) => (motion.once = e.currentTarget.checked)} /> play once, then hold
      </label><Tip text={TIP.once} />
      {#if inRoute && usesHits}
        <span class="lbl">hits from</span>
        <select value={motion.trigger ?? ""} onchange={(e) => (motion.trigger = e.currentTarget.value || undefined)}>
          <option value="">{routeSourceLabel} (the route's)</option>
          {#each hits.filter((h) => h.id !== routeSource) as h}<option value={h.id}>{h.label}</option>{/each}
        </select><Tip text={TIP.trigger} />
      {/if}
    </div>
  {/if}

  {#if needsRanges}
    <div class="row flow">
      <span class="lbl">ranges</span><Tip text={TIP.ranges} />
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
        <span class="lbl">add</span><Tip text={TIP.add} />
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
  .chk { display: inline-flex; gap: 4px; align-items: center; margin: 0; font-size: 12px; color: var(--muted); }
  .caps .cap { font-size: 11px; padding: 2px 6px; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
