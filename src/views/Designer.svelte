<script module lang="ts">
  import type { DesignResult } from "../lib/design";
  import type { SceneSpec } from "../lib/blocks";
  // Kept across view switches so a design isn't lost when you look at Routes.
  const saved = $state({
    mode: "" as "" | "template" | "builder" | "ai",
    /** Scenes being composed in the scene builder. */
    scenes: [] as SceneSpec[],
    template: "club",
    fromTemplate: false,
    prompt: "",
    genre: "",
    scope: { routes: true, presets: true, audio: true, palettes: true },
    /** Show "map your controller next" after a design with scenes was added. */
    mapNext: false,
    replace: true,
    result: null as DesignResult | null,
    keep: [] as string[],
    undo: "" as string,
  });
</script>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { app, toast } from "../lib/store.svelte";
  import { GENRES, applyGenre } from "../lib/genres";
  import { aiCounts, applyPlan, describeRoute, describeTrigger, type DesignRequest, type Plan } from "../lib/design";
  import { TEMPLATES, buildTemplate, templateScenes } from "../lib/templates";
  import { buildScenes, newBlock } from "../lib/blocks";
  import SceneBuilder from "../components/SceneBuilder.svelte";
  import GenreTune from "../components/GenreTune.svelte";
  import type { AiSettings, ControlMapping, Show } from "../lib/types";

  const show = app.show!;
  const s = saved;
  let busy = $state(false);
  let error = $state("");
  let configured = $state(true);
  let refine = $state("");
  let tuneOpen = $state(false);
  let started = $state(0);
  let now = $state(Date.now());

  onMount(() => {
    invoke<AiSettings>("get_ai_settings").then((a) => {
      configured = !!a.base_url && (!!a.api_key || a.auth === "none");
      s.mode ||= configured ? "ai" : "template";
    }).catch(() => (s.mode ||= "template"));
    const t = setInterval(() => (now = Date.now()), 1000);
    // Dev only: ?autodesign runs the (mocked) generator so screenshots can show a result.
    if (import.meta.env.DEV && new URLSearchParams(location.search).has("autodesign") && !s.result) {
      s.genre = "techno";
      s.prompt = IDEAS[0];
      generate(false);
    }
    return () => clearInterval(t);
  });

  const genre = $derived(GENRES.find((g) => g.id === s.genre) ?? null);
  const plan = $derived(s.result?.plan ?? null);
  const keep = $derived(new Set(s.keep));
  const prior = $derived(aiCounts(show));
  const priorTotal = $derived(prior.routes + prior.presets + prior.controls);

  const IDEAS = [
    "A full night: warm-up, groove, build, drop and breakdown scenes, with pads to switch scenes and a strobe button.",
    "Make the drops hit harder: kick flashes, snare strobes, lasers on peak looks, fog on the first beat of the drop.",
    "Something calm for a lounge: slow colour waves, gentle twinkle, no strobe, lasers only on static looks.",
    "Two rooms: keep the bar warm and calm all night while the main room follows the scenes.",
  ];

  async function generate(previous = false) {
    if (busy) return;
    error = "";
    busy = true;
    started = Date.now();
    // Start from the genre's detection tuning so routes are written for the bands as they'll be.
    const base: Show = $state.snapshot(show) as Show;
    if (genre && s.scope.audio) applyGenre(base, genre);
    const request: DesignRequest = {
      prompt: previous ? refine : s.prompt,
      genre: genre ? { name: genre.name, bpm: genre.bpm, style: genre.style } : null,
      // MIDI and key mappings are made afterwards in Controller setup, from what's actually in the show.
      scope: { ...s.scope, controls: false },
      controller: "",
      midi_channel: 0,
      keyboard: false,
      previous: previous && plan ? $state.snapshot(plan) : null,
    };
    try {
      const r = await invoke<DesignResult>("ai_design", { show: base, request });
      if (s.scope.audio) addGenreBands(r.plan, base);
      show_result(r, false);
      if (previous) refine = "";
    } catch (e) {
      error = String(e);
    }
    busy = false;
  }

  /** Genre tuning is part of the plan too (an AI design may retune on top of it). */
  function addGenreBands(plan: Plan, base: Show) {
    if (!genre) return;
    for (const b of base.audio.bands) {
      const before = show.audio.bands.find((x) => x.id === b.id);
      const changed = !before || JSON.stringify(before) !== JSON.stringify(b);
      if (changed && !plan.bands.some((x) => x.id === b.id)) plan.bands.push(b);
    }
    plan.sensitivity ??= base.audio.sensitivity;
  }

  function show_result(r: DesignResult, fromTemplate: boolean) {
    s.result = r;
    s.fromTemplate = fromTemplate;
    s.keep = [
      ...r.plan.routes.map((x) => x.id), ...r.plan.presets.map((x) => x.id), ...r.plan.controls.map((x) => x.id),
      ...r.plan.palettes.map((x) => x.id), ...r.plan.bands.map((b) => `band:${b.id}`), "sensitivity",
    ];
  }

  /** Build a setup from a template: no AI, same checks and review. */
  async function buildFromTemplate() {
    error = "";
    const base: Show = $state.snapshot(show) as Show;
    if (genre) applyGenre(base, genre);
    const plan = buildTemplate(base, s.template, genre);
    if (!plan.routes.length) { error = "Nothing to build: add fixtures with brightness/colour channels, or lasers with saved looks, first."; return; }
    addGenreBands(plan, base);
    try {
      const r = await invoke<DesignResult>("design_check", { show: base, plan });
      show_result(r, true);
    } catch (e) {
      error = String(e);
    }
  }

  /** Build the scene builder's scenes: no AI, same checks and review. */
  async function buildFromScenes() {
    error = "";
    const base: Show = $state.snapshot(show) as Show;
    if (genre) applyGenre(base, genre);
    const scenes = $state.snapshot(s.scenes) as SceneSpec[];
    const empty = scenes.filter((x) => !x.blocks.length).map((x) => x.name);
    const { plan, skipped } = buildScenes(base, scenes, { genre, title: `Your scenes${genre ? ` · ${genre.name}` : ""}` });
    if (!plan.routes.length) { error = "Nothing to build: add blocks your rig can drive to at least one scene."; return; }
    addGenreBands(plan, base);
    try {
      const r = await invoke<DesignResult>("design_check", { show: base, plan });
      r.warnings = [...skipped.map((x) => `Skipped ${x}`), ...empty.map((x) => `Skipped scene "${x}": no blocks`), ...r.warnings];
      show_result(r, true);
    } catch (e) {
      error = String(e);
    }
  }
  function customise() {
    s.scenes = templateScenes(s.template, $state.snapshot(show) as Show);
    s.mode = "builder";
  }
  function startFrom(sel: HTMLSelectElement) {
    const v = sel.value;
    sel.value = "";
    if (v === "blank") s.scenes = [{ name: "Scene 1", blocks: [newBlock("colour_hits")] }];
    else if (v) s.scenes = templateScenes(v, $state.snapshot(show) as Show);
  }

  function toggle(id: string) {
    s.keep = keep.has(id) ? s.keep.filter((k) => k !== id) : [...s.keep, id];
  }
  function toggleAll(ids: string[]) {
    const all = ids.every((i) => keep.has(i));
    s.keep = all ? s.keep.filter((k) => !ids.includes(k)) : [...new Set([...s.keep, ...ids])];
  }

  function apply() {
    if (!plan) return;
    s.undo = JSON.stringify($state.snapshot(show));
    const msg = applyPlan(show, $state.snapshot(plan) as typeof plan, keep, s.replace);
    s.mapNext = plan.presets.some((p) => keep.has(p.id));
    toast(`${msg}. Find them under Routes and Control.`);
  }
  function undo() {
    if (!s.undo || !app.show) return;
    app.show = JSON.parse(s.undo);
    s.undo = "";
    s.mapNext = false;
    app.epoch++;
    toast("Undid the design");
  }


  function describeAction(c: ControlMapping): string {
    const a = c.action;
    const routeName = (id: string) => [...show.routes, ...(plan?.routes ?? [])].find((r) => r.id === id)?.name ?? id;
    switch (a.type) {
      case "routes": return `${c.trigger.mode} ${a.routes.map(routeName).join(", ")}`;
      case "route_group": return `${c.trigger.mode} scene "${a.group}"`;
      case "bands": return `mute ${a.bands.join(", ")}`;
      case "blackout": return "blackout";
      case "master": return "master brightness";
      case "preset": return `scene: ${[...show.presets, ...(plan?.presets ?? [])].find((p) => p.id === a.preset)?.name ?? a.preset}`;
      case "release_manual": return "release manual";
      case "channel_value": return `${a.channel.replace(/^@/, "")} ${a.min}–${a.max}`;
      case "band_sensitivity": return `${a.band} sensitivity`;
    }
  }

  // Routes by scene (route group) for the preview.
  const scenes = $derived.by(() => {
    const m = new Map<string, NonNullable<typeof plan>["routes"]>();
    for (const r of plan?.routes ?? []) m.set(r.group, [...(m.get(r.group) ?? []), r]);
    return [...m.entries()];
  });
  const bandName = (id: string) => show.audio.bands.find((b) => b.id === id)?.name;
</script>

<div class="designer">
  <div class="panel col form">
    <h2>Design a show</h2>
    <div class="modes">
      <button class:on={s.mode === "template"} onclick={() => (s.mode = "template")}>Templates</button>
      <button class:on={s.mode === "builder"} onclick={() => { s.mode = "builder"; if (!s.scenes.length) s.scenes = [{ name: "Scene 1", blocks: [newBlock("colour_hits")] }]; }}>Build your own</button>
      <button class:on={s.mode === "ai"} onclick={() => (s.mode = "ai")}>AI designer</button>
    </div>

    {#if s.mode === "template"}
      <p class="muted small">
        Ready-made scene layouts built from your rig: your fixture groups, focus points and laser looks. Works offline.
        You review everything before it's added, and it's all editable afterwards.
      </p>
      <div class="templates">
        {#each TEMPLATES as t}
          <label class="tpl" class:on={s.template === t.id}>
            <input type="radio" name="tpl" value={t.id} bind:group={s.template} />
            <span class="col"><b>{t.name}</b><span class="small muted">{t.description}</span></span>
          </label>
        {/each}
      </div>
    {:else if s.mode === "builder"}
      <p class="muted small">
        Put scenes together from building blocks: colour hits, waves, sparkle, strobe, laser looks, fog bursts… Pick what each one drives,
        what sound triggers it and how fast. Works offline, and you review everything before it's added.
      </p>
      <div class="row">
        <select class="grow" onchange={(e) => startFrom(e.currentTarget)}>
          <option value="">Start again from…</option>
          <option value="blank">A blank scene</option>
          {#each TEMPLATES as t}<option value={t.id}>{t.name} template</option>{/each}
        </select>
      </div>
      <SceneBuilder bind:scenes={s.scenes} />
    {:else}
      <p class="muted small">
        Describe the night and the AI writes routes, scenes, palettes and detection tuning for the rig you've set up:
        your fixtures, groups, rooms, focus points and saved looks. You review everything before it's added.
      </p>
      {#if !configured}
        <div class="tag warn note">Set up an AI provider first, or use Templates. <button class="ghost" onclick={() => (app.view = "ai")}>AI settings</button></div>
      {/if}
      <label for="d-prompt">What do you want?</label>
      <textarea id="d-prompt" rows="5" bind:value={s.prompt} placeholder="e.g. Friday club night: start mellow, build to a big peak around midnight. Keep the laser for drops only."></textarea>
      <div class="ideas">
        {#each IDEAS as idea}<button class="ghost small" onclick={() => (s.prompt = idea)}>{idea.split(":")[0]}</button>{/each}
      </div>
    {/if}

    <label for="d-genre">Genre</label>
    <div class="row">
      <select id="d-genre" class="grow" bind:value={s.genre}>
        <option value="">Any / mixed</option>
        {#each GENRES as g}<option value={g.id}>{g.name} ({g.bpm[0]}–{g.bpm[1]} BPM)</option>{/each}
      </select>
      {#if genre}<button class:on={tuneOpen} onclick={() => (tuneOpen = !tuneOpen)} title="Preview and apply this genre's detection settings now, without building anything">Tune detection only…</button>{/if}
    </div>
    {#if genre && tuneOpen}{#key genre.id}<GenreTune genreId={genre.id} picker={false} />{/key}{/if}
    {#if genre}<p class="faint small">{genre.style}</p>{/if}

    {#if s.mode === "template"}
      <div class="row go">
        <button class="primary" onclick={buildFromTemplate}>Build setup</button>
        <button onclick={customise} title="Open this template in the scene builder to change its scenes and blocks">Customise…</button>
      </div>
    {:else if s.mode === "builder"}
      <div class="row go sticky">
        <button class="primary" onclick={buildFromScenes} disabled={!s.scenes.some((x) => x.blocks.length)}>Build scenes</button>
      </div>
    {:else}
      <span class="label">Generate</span>
      <div class="checks">
        <label><input type="checkbox" bind:checked={s.scope.routes} /> Routes and scenes</label>
        <label><input type="checkbox" bind:checked={s.scope.presets} /> Scene presets</label>
        <label><input type="checkbox" bind:checked={s.scope.audio} /> Audio detection tuning</label>
        <label><input type="checkbox" bind:checked={s.scope.palettes} /> Colour palettes</label>
      </div>
      <div class="row go">
        <button class="primary" onclick={() => generate(false)} disabled={busy || !configured}>{busy ? "Designing…" : "Generate design"}</button>
        {#if busy}<span class="muted small">{Math.max(0, Math.round((now - started) / 1000))} s · big designs can take a few minutes</span>{/if}
      </div>
    {/if}
    {#if error}<p class="err small">{error}</p>{/if}
  </div>

  <div class="panel col result">
    {#if !plan}
      <div class="empty col">
        <span class="big">✧</span>
        <span>Your design will appear here to review.</span>
        <span class="faint small">Tip: name your fixture groups, add focus points, and tag your laser looks (chill, peak…) first. Templates and the AI both use them.</span>
      </div>
    {:else}
      <div class="row">
        <h2 class="grow">{plan.title}</h2>
        {#if s.undo}<button onclick={undo}>Undo apply</button>{/if}
      </div>
      {#if s.mapNext && s.undo}
        <div class="next row">
          <span class="grow small"><b>Next: map your controller.</b> Controller setup assigns these scenes to your pads and keys, plus strobe, fog, master and group faders for whatever your rig has.</span>
          <button class="primary" onclick={() => { s.mapNext = false; app.view = "control"; }}>Open Controller setup</button>
        </div>
      {/if}
      {#if plan.summary}<p class="summary">{plan.summary}</p>{/if}
      {#if s.result?.warnings.length}
        <details class="warnings">
          <summary class="small">{s.result.warnings.length === 1 ? "1 thing was" : `${s.result.warnings.length} things were`} fixed or skipped to fit your rig</summary>
          <ul class="small muted">{#each s.result.warnings as w}<li>{w}</li>{/each}</ul>
        </details>
      {/if}

      <div class="scroll sections">
        {#if plan.bands.length || plan.sensitivity !== null}
          <section>
            <h3><button class="ghost all" onclick={() => toggleAll([...plan.bands.map((b) => `band:${b.id}`), "sensitivity"])}>Audio detection</button></h3>
            {#if plan.sensitivity !== null}
              <label class="item"><input type="checkbox" checked={keep.has("sensitivity")} onchange={() => toggle("sensitivity")} />
                <span class="grow">Global sensitivity</span><span class="mono small">{Math.round(show.audio.sensitivity * 100)}% → {Math.round(plan.sensitivity * 100)}%</span></label>
            {/if}
            {#each plan.bands as b}
              <label class="item"><input type="checkbox" checked={keep.has(`band:${b.id}`)} onchange={() => toggle(`band:${b.id}`)} />
                <span class="swatch" style="background:{b.color}"></span>
                <span class="grow">{b.name} <span class="faint small">{bandName(b.id) ? "retuned" : "new band"}</span></span>
                <span class="mono small faint">{b.ranges.map((r) => `${Math.round(r.lo_hz)}–${Math.round(r.hi_hz)}`).join(" + ")} Hz · {b.detect} · {Math.round(b.sensitivity * 100)}%</span></label>
            {/each}
          </section>
        {/if}

        {#if plan.palettes.length}
          <section>
            <h3><button class="ghost all" onclick={() => toggleAll(plan.palettes.map((p) => p.id))}>Palettes</button></h3>
            {#each plan.palettes as p}
              <label class="item"><input type="checkbox" checked={keep.has(p.id)} onchange={() => toggle(p.id)} />
                <span class="grow">{p.name}</span>
                <span class="pal">{#each p.colors as c}<i style="background:{c}"></i>{/each}</span></label>
            {/each}
          </section>
        {/if}

        {#each scenes as [scene, routes]}
          <section>
            <h3><button class="ghost all" onclick={() => toggleAll(routes.map((r) => r.id))}>Scene: {scene}</button>
              <span class="faint small">{routes.length} routes{routes.some((r) => r.enabled) ? " · starts on" : ""}</span></h3>
            {#each routes as r}
              <label class="item"><input type="checkbox" checked={keep.has(r.id)} onchange={() => toggle(r.id)} />
                <span class="name">{r.name}</span>
                <span class="grow small muted">{describeRoute(r, show, plan)}</span></label>
            {/each}
          </section>
        {/each}

        {#if plan.presets.length}
          <section>
            <h3><button class="ghost all" onclick={() => toggleAll(plan.presets.map((p) => p.id))}>Scene presets</button></h3>
            {#each plan.presets as p}
              {@const on = Object.entries(p.routes).filter(([, v]) => v).length}
              <label class="item"><input type="checkbox" checked={keep.has(p.id)} onchange={() => toggle(p.id)} />
                <span class="swatch" style="background:{p.color}"></span><span class="name">{p.name}</span>
                <span class="grow small muted">turns on {on} route{on === 1 ? "" : "s"}{p.values.length ? `, sets ${p.values.length} channels` : ""}</span></label>
            {/each}
          </section>
        {/if}

        {#if plan.controls.length}
          <section>
            <h3><button class="ghost all" onclick={() => toggleAll(plan.controls.map((c) => c.id))}>Controls</button></h3>
            {#each plan.controls as c}
              <label class="item"><input type="checkbox" checked={keep.has(c.id)} onchange={() => toggle(c.id)} />
                <span class="tag mono">{describeTrigger(c)}</span><span class="name">{c.name}</span>
                <span class="grow small muted">{describeAction(c)}</span></label>
            {/each}
          </section>
        {/if}
      </div>

      <div class="row foot">
        <label class="inline small" title="Remove routes, scenes and controls from earlier generated designs (templates or AI) before adding these">
          <input type="checkbox" checked={s.replace && priorTotal > 0} onchange={(e) => (s.replace = e.currentTarget.checked)} disabled={!priorTotal} />
          Replace earlier generated designs{priorTotal ? ` (${prior.routes} routes, ${prior.presets} scenes, ${prior.controls} controls)` : ""}
        </label>
        <div class="grow"></div>
        <button class="primary" onclick={apply} disabled={!s.keep.length}>Add to show</button>
      </div>
      {#if !s.fromTemplate}<div class="row">
        <input class="grow" bind:value={refine} placeholder="Refine it: “fewer strobes”, “laser only on drops”, “slower warm-up”…"
          onkeydown={(e) => e.key === "Enter" && refine.trim() && generate(true)} />
        <button onclick={() => generate(true)} disabled={busy || !refine.trim() || !configured}>{busy ? "Working…" : "Refine"}</button>
      </div>{/if}
    {/if}
  </div>
</div>

<style>
  .designer { display: grid; grid-template-columns: minmax(320px, 420px) 1fr; gap: 12px; align-items: start; height: 100%; }
  .form label, .form .label { margin-top: 6px; }
  .label { font-size: 12px; color: var(--muted, #9aa4b8); }
  textarea { resize: vertical; font: inherit; }
  .modes { display: flex; gap: 4px; }
  .modes button { flex: 1; padding: 4px 6px; }
  .modes button.on { background: var(--accent-bg); border-color: var(--accent); }
  .templates { display: flex; flex-direction: column; gap: 6px; }
  .tpl { display: flex; gap: 8px; align-items: flex-start; padding: 8px; border: 1px solid var(--line-2); border-radius: 8px; margin: 0; cursor: pointer; }
  .tpl.on { border-color: var(--accent); background: var(--accent-bg); }
  .tpl input { margin-top: 3px; }
  .ideas { display: flex; flex-wrap: wrap; gap: 4px; }
  .ideas button { padding: 2px 8px; font-size: 12px; border: 1px solid var(--line-2); border-radius: 999px; }
  .checks { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 12px; }
  .checks label, .inline { display: flex; gap: 6px; align-items: center; margin: 0; font-size: 13px; }
  .go { margin-top: 10px; }
  .go.sticky { position: sticky; bottom: 0; background: var(--panel); padding: 8px 0; border-top: 1px solid var(--line); z-index: 2; }
  .note { display: flex; align-items: center; gap: 6px; padding: 6px 8px; }
  .result { min-height: 420px; max-height: calc(100vh - 120px); }
  .empty { align-items: center; justify-content: center; gap: 8px; text-align: center; flex: 1; padding: 60px 20px; }
  .big { font-size: 34px; opacity: 0.5; }
  .summary { margin: 0; line-height: 1.5; white-space: pre-wrap; }
  .warnings summary { cursor: pointer; color: var(--warn, #f2b54a); }
  .warnings ul { margin: 6px 0; padding-left: 18px; }
  .sections { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 10px; }
  section h3 { margin: 4px 0; display: flex; align-items: baseline; gap: 8px; }
  .all { padding: 0; font: inherit; font-weight: 600; }
  .item { display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-radius: 6px; margin: 0; cursor: pointer; }
  .item:hover { background: #ffffff08; }
  .item .name { min-width: 140px; }
  .swatch { width: 12px; height: 12px; border-radius: 3px; flex: none; }
  .pal { display: flex; gap: 2px; }
  .pal i { width: 16px; height: 12px; border-radius: 2px; }
  .next { padding: 8px 10px; border: 1px solid var(--accent); border-radius: 8px; background: var(--accent-bg); }
  .foot { border-top: 1px solid var(--line-2); padding-top: 8px; }
  @media (max-width: 980px) { .designer { grid-template-columns: 1fr; } }
</style>
