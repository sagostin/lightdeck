<script lang="ts">
  // The autopilot: switch it on, see what it hears, tag scenes with the sections they
  // suit, tune when it switches, and get tags from a guess or from the AI.
  import { invoke } from "@tauri-apps/api/core";
  import { app, toast } from "../lib/store.svelte";
  import { currentGenre } from "../lib/genres";
  import {
    SECTIONS, applyAutopilotPlan, autopilotSettings, coverageNote, guessAll, sectionInfo, toggleSection, undoAutopilotPlan,
  } from "../lib/autopilot";
  import type { AutopilotPlan, Show } from "../lib/types";

  let { compact = false }: { compact?: boolean } = $props();
  const show = app.show!;
  const settings = autopilotSettings(show);
  const st = $derived(app.autopilot);
  const sceneName = (id: string | null | undefined) => show.presets.find((p) => p.id === id)?.name ?? "—";
  const note = $derived(coverageNote(show));
  const tagged = $derived(show.presets.filter((p) => p.sections?.length).length);

  async function setRunning(on: boolean) {
    if (on && !tagged) { toast("Tag some scenes with sections first (Guess or Suggest with AI can do it)"); return; }
    await invoke("autopilot_set", { on });
    if (app.autopilot) app.autopilot.running = on;
  }

  function guess(overwrite: boolean) {
    const changed = guessAll(show, overwrite);
    toast(changed.length ? `Tagged ${changed.map((c) => `${c.name} (${c.why})`).join("; ")}` : "Nothing to change");
  }

  // ---- AI ----
  let brief = $state("");
  let busy = $state(false);
  let error = $state("");
  let suggestion = $state<AutopilotPlan | null>(null);
  let undo = $state<ReturnType<typeof applyAutopilotPlan> | null>(null);
  async function suggest() {
    busy = true; error = ""; suggestion = null;
    const g = currentGenre(show);
    try {
      suggestion = await invoke<AutopilotPlan>("ai_autopilot", {
        show: $state.snapshot(show) as Show,
        request: { prompt: brief, genre: g ? { name: g.name, bpm: g.bpm, style: g.style } : null },
      });
    } catch (e) { error = String(e); }
    busy = false;
  }
  function apply() {
    undo = applyAutopilotPlan(show, suggestion!);
    suggestion = null;
    toast("Autopilot set up from the suggestion");
  }
  const changes = $derived(suggestion ? show.presets.filter((p) => suggestion!.sections[p.id] && JSON.stringify(suggestion!.sections[p.id]) !== JSON.stringify(p.sections ?? [])) : []);
  const label = (s: string[] | undefined) => (s?.length ? s.map((x) => sectionInfo(x)?.name ?? x).join(", ") : "not used");
</script>

{#if compact}
  <div class="compact row">
    <button class="toggle" class:on={st?.running} onclick={() => setRunning(!st?.running)} title="Switch scenes automatically with the music">
      Autopilot {st?.running ? "on" : "off"}
    </button>
    {#if st?.running}
      <span class="sec" style="--c:{sectionInfo(st.section)?.color}">{sectionInfo(st.section)?.name}</span>
      <span class="small">{sceneName(st.scene)}</span>
      {#if st.paused_s}<button class="ghost small" onclick={() => invoke("autopilot_resume")}>paused {st.paused_s}s · resume</button>{/if}
    {/if}
  </div>
{:else}
  <div class="panel col">
    <div class="row">
      <h2 class="grow">Autopilot</h2>
      <button class="toggle" class:on={st?.running} onclick={() => setRunning(!st?.running)}>{st?.running ? "On" : "Off"}</button>
    </div>
    <span class="small muted">
      Listens for breakdowns, grooves, builds and drops, and switches to a scene tagged for each, on the bar. Pick a scene yourself
      at any time: the autopilot follows you and pauses for a while.
    </span>

    {#if st}
      <div class="status col">
        <div class="row">
          <span class="faint small">Hearing</span>
          <span class="sec" style="--c:{sectionInfo(st.section)?.color}">{sectionInfo(st.section)?.name ?? st.section}</span>
          <span class="meter" title="Loudness compared with the last couple of minutes"><i style="width:{Math.round(st.loudness * 100)}%"></i></span>
          <span class="grow"></span>
          {#if st.running}
            <span class="small">{sceneName(st.scene)} · {st.bars_in_scene} bar{st.bars_in_scene === 1 ? "" : "s"}</span>
          {/if}
        </div>
        <div class="history" title="The last bars, oldest first">
          {#each st.history as h, i (i)}<i style="--c:{sectionInfo(h)?.color}" title={sectionInfo(h)?.name}></i>{/each}
          {#if !st.history.length}<span class="small faint">Nothing heard yet: play some music.</span>{/if}
        </div>
        {#if st.running && (st.paused_s || st.reason)}
          <div class="row small">
            <span class="faint grow">{st.reason}</span>
            {#if st.paused_s}<button class="ghost" onclick={() => invoke("autopilot_resume")}>Resume now ({st.paused_s}s)</button>{/if}
          </div>
        {/if}
      </div>
    {/if}

    <h3>Scenes</h3>
    {#if note}<span class="small warn">{note}</span>{/if}
    <div class="scenes">
      {#each show.presets as p (p.id)}
        <div class="scene row" style="--c:{p.color}" class:playing={st?.running && st.scene === p.id}>
          <span class="name grow">{p.name}</span>
          {#each SECTIONS as s}
            <button class="chip" class:on={p.sections?.includes(s.id)} style="--c:{s.color}" title={s.hint} onclick={() => toggleSection(p, s.id)}>{s.name}</button>
          {/each}
        </div>
      {:else}
        <span class="small faint">No scenes yet. Build some on the Design screen or with + New scene.</span>
      {/each}
    </div>
    {#if show.presets.length}
      <div class="row flow">
        <button onclick={() => guess(false)} title="Guess sections for untagged scenes from their names and what they switch on">Guess untagged</button>
        <button class="ghost" onclick={() => guess(true)} title="Guess every scene again">Guess all again</button>
      </div>
    {/if}

    <h3>When to switch</h3>
    <div class="settings">
      <label for="ap-min">Stay at least</label>
      <span><input id="ap-min" type="number" min="1" max="64" bind:value={settings.min_bars} /> bars</span>
      <label for="ap-drop">A drop cuts in after</label>
      <span><input id="ap-drop" type="number" min="1" max={settings.min_bars} bind:value={settings.drop_bars} /> bars</span>
      <label for="ap-max">Change scene anyway after</label>
      <span><input id="ap-max" type="number" min="0" max="256" bind:value={settings.max_bars} /> bars <span class="faint small">(0 = never)</span></span>
      <label for="ap-pause">After you pick a scene, wait</label>
      <span><input id="ap-pause" type="number" min="0" max="600" bind:value={settings.pause_s} /> s</span>
    </div>

    <h3>Suggest with AI</h3>
    <span class="small muted">The AI looks at what each scene switches on and tags it, and sets when to switch. You see the changes before they're applied.</span>
    <textarea rows="2" bind:value={brief} placeholder="Optional: about the night, e.g. long progressive builds, keep the strobe for the biggest drops"></textarea>
    <div class="row">
      <button class="primary" disabled={busy || !show.presets.length} onclick={suggest}>{busy ? "Thinking…" : "Suggest"}</button>
      {#if undo}<button class="ghost" onclick={() => { undoAutopilotPlan(show, undo!); undo = null; toast("Undone"); }}>Undo suggestion</button>{/if}
    </div>
    {#if error}<div class="small err">{error}</div>{/if}
    {#if suggestion}
      <div class="suggestion col">
        {#if suggestion.notes}<p class="small">{suggestion.notes}</p>{/if}
        {#each changes as p (p.id)}
          <div class="small"><b>{p.name}</b>: {label(p.sections)} → <b>{label(suggestion.sections[p.id])}</b></div>
        {:else}<div class="small faint">No scene changes.</div>{/each}
        {#if JSON.stringify(settings) !== JSON.stringify(suggestion.settings)}
          {@const b = suggestion.settings}
          <div class="small">Switching: stay {b.min_bars} bars, drops after {b.drop_bars}, change after {b.max_bars || "never"}, pause {b.pause_s}s</div>
        {/if}
        {#each suggestion.warnings as w}<div class="small warn">{w}</div>{/each}
        <div class="row"><button class="primary" onclick={apply}>Apply</button><button class="ghost" onclick={() => (suggestion = null)}>Discard</button></div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .toggle { min-width: 64px; }
  .toggle.on { background: var(--accent-bg); border-color: var(--accent); color: var(--text); }
  .sec { padding: 2px 8px; border-radius: 10px; font-size: 12px; background: color-mix(in srgb, var(--c) 25%, transparent); border: 1px solid var(--c); }
  .meter { width: 80px; height: 6px; border-radius: 3px; background: var(--line); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); }
  .status { padding: 8px; border: 1px solid var(--line); border-radius: 8px; gap: 6px; }
  .history { display: flex; gap: 2px; min-height: 12px; align-items: center; }
  .history i { width: 10px; height: 12px; border-radius: 2px; background: var(--c); }
  .scenes { display: flex; flex-direction: column; gap: 4px; }
  .scene { padding: 4px 6px; border-radius: 6px; border: 1px solid var(--line); border-left: 4px solid var(--c); gap: 4px; }
  .scene.playing { background: color-mix(in srgb, var(--c) 14%, transparent); }
  .scene .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chip { font-size: 11px; padding: 2px 7px; color: var(--faint); }
  .chip.on { background: color-mix(in srgb, var(--c) 25%, transparent); border-color: var(--c); color: var(--text); }
  .settings { display: grid; grid-template-columns: auto auto; gap: 6px 10px; align-items: center; justify-items: start; }
  .settings label { margin: 0; }
  .settings input { width: 64px; }
  .flow { flex-wrap: wrap; }
  .suggestion { border: 1px solid var(--accent); border-radius: 8px; padding: 8px; gap: 4px; }
  .suggestion p { margin: 0; }
  .warn { color: var(--warn); }
  .compact { gap: 8px; }
  textarea { width: 100%; resize: vertical; }
  h3 { margin: 6px 0 0; }
</style>
