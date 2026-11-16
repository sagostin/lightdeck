<script lang="ts">
  // Pick a genre, see exactly what it would change in detection, then apply (with undo).
  import { untrack } from "svelte";
  import { app, toast } from "../lib/store.svelte";
  import { DEFAULT_TUNE, GENRES, applyTune, currentGenre, describeTune, planGenre, type TuneOptions } from "../lib/genres";

  let { genreId = $bindable(""), picker = true }: { genreId?: string; picker?: boolean } = $props();
  const show = app.show!;
  // Without the picker (Design screen), open straight on the given genre's preview.
  let open = $state(untrack(() => !picker && !!genreId));
  let opts = $state<TuneOptions>({ ...DEFAULT_TUNE });
  let undo = $state<string | null>(null);
  const genre = $derived(GENRES.find((g) => g.id === genreId) ?? null);
  const plan = $derived(genre && open ? planGenre(show, genre, opts) : null);
  const current = $derived(currentGenre(show));

  export function preview(id = genreId) {
    genreId = id;
    open = !!id;
  }
  function apply() {
    if (!plan) return;
    undo = JSON.stringify({ audio: $state.snapshot(show.audio), routes: show.routes.map((r) => [r.id, r.enabled]) });
    const r = applyTune(show, $state.snapshot(plan) as typeof plan);
    open = false;
    const parts = [r.changed.length && `retuned ${r.changed.join(", ")}`, r.added.length && `added ${r.added.join(", ")}`,
      r.removed.length && `removed ${r.removed.join(", ")}`, r.routesOff.length && `switched off ${r.routesOff.length} route(s) that used them`].filter(Boolean);
    toast(`${plan.genre.name}: ${parts.join("; ") || "sensitivity set"}`);
  }
  function undoTune() {
    if (!undo) return;
    const u = JSON.parse(undo) as { audio: typeof show.audio; routes: [string, boolean][] };
    show.audio = u.audio;
    for (const [id, on] of u.routes) { const r = show.routes.find((x) => x.id === id); if (r) r.enabled = on; }
    undo = null;
    toast("Detection put back");
  }
</script>

<div class="tune col">
  {#if picker}
    <div class="row">
      <select class="grow" value={open ? genreId : ""} onchange={(e) => preview(e.currentTarget.value)}
        title="Retune detection for a style of music. You'll see what changes before anything is applied.">
        <option value="">Tune for a genre…</option>
        {#each GENRES as g}<option value={g.id}>{g.name} ({g.bpm[0]}–{g.bpm[1]} BPM){current?.id === g.id ? " · current" : ""}</option>{/each}
      </select>
      {#if undo}<button class="ghost" onclick={undoTune}>Undo</button>{/if}
    </div>
    {#if current && !open}<span class="small faint">Detection matches {current.name}.</span>{/if}
  {/if}

  {#if plan}
    <div class="preview col">
      <div class="row"><b class="grow">{plan.genre.name}</b><button class="ghost x" onclick={() => (open = false)} aria-label="Close">×</button></div>
      <span class="small">{describeTune(plan)}</span>
      {#if !plan.same}
        <div class="changes small">
          {#if plan.sensitivity.from !== plan.sensitivity.to}
            <div class="row"><span class="nm">Global sensitivity</span><span class="mono faint">{Math.round(plan.sensitivity.from * 100)}% → {Math.round(plan.sensitivity.to * 100)}%</span></div>
          {/if}
          {#each plan.changes.filter((c) => c.status !== "same") as c}
            <div class="row top">
              <span class="nm">{c.status === "add" ? "+ " : ""}{c.name}</span>
              <span class="col grow">
                {#if c.status === "add"}<span class="faint">new band</span>{/if}
                {#each c.fields as f}<span class="mono faint">{f.field}: {f.from} → <span class="to">{f.to}</span></span>{/each}
              </span>
            </div>
          {/each}
          {#if plan.reordered}<div class="faint">{plan.bands[0].name} moves first: the first band drives the beat clock.</div>{/if}
        </div>
      {/if}
      {#if plan.others.length}
        <div class="small">
          <span class="muted">Not part of {plan.genre.name}:</span>
          {#each plan.others as b}
            <span class="tag" class:warn={opts.removeOthers && (b.routes.length || b.controls.length)}>{b.name}{b.routes.length ? ` · ${b.routes.length} route${b.routes.length === 1 ? "" : "s"}` : ""}{b.controls.length ? ` · ${b.controls.length} control${b.controls.length === 1 ? "" : "s"}` : ""}</span>
          {/each}
        </div>
      {/if}
      <label class="opt small"><input type="checkbox" bind:checked={opts.keepRanges} /> Keep my frequency ranges and gains</label>
      {#if plan.others.length}
        <label class="opt small"><input type="checkbox" bind:checked={opts.removeOthers} /> Remove the bands {plan.genre.name} doesn't use</label>
        {#if opts.removeOthers && plan.removed.some((b) => b.routes.length || b.controls.length)}
          <span class="small warn">Routes that use them will be switched off and flagged on the Routes screen so you can pick a new sound.</span>
        {/if}
      {/if}
      <div class="row">
        {#if plan.same}
          <button onclick={() => (open = false)}>OK</button>
        {:else}
          <button class="primary" onclick={apply}>{plan.overwrites ? "Apply and replace these settings" : "Apply"}</button>
          <button class="ghost" onclick={() => (open = false)}>Cancel</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .tune { gap: 6px; }
  .preview { gap: 6px; padding: 8px; border: 1px solid var(--accent); border-radius: 8px; background: var(--accent-bg); }
  .changes { display: flex; flex-direction: column; gap: 4px; max-height: 260px; overflow: auto; }
  .row.top { align-items: flex-start; }
  .nm { min-width: 110px; font-weight: 600; }
  .to { color: var(--text); }
  .opt { display: flex; gap: 6px; align-items: center; margin: 0; }
  .warn { color: var(--warn, #f2b54a); }
  .tag.warn { border-color: var(--warn); }
  .x { padding: 0 5px; }
</style>
