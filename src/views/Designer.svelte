<script module lang="ts">
  import type { DesignResult } from "../lib/design";
  import type { AutopilotPlan } from "../lib/types";
  import type { applyAutopilotPlan } from "../lib/autopilot";
  // Kept across view switches so a design isn't lost when you look at Routes.
  const saved = $state({
    mode: "" as "" | "template" | "ai",
    /** What to ask the AI for. */
    ask: "scenes" as "scenes" | "autopilot",
    /** An autopilot set-up suggested by the AI, waiting to be applied. */
    suggestion: null as AutopilotPlan | null,
    autopilotUndo: null as ReturnType<typeof applyAutopilotPlan> | null,
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
  import { GENRES, applyGenre, currentGenre } from "../lib/genres";
  import { applyAutopilotPlan as applyAp, sectionInfo, undoAutopilotPlan } from "../lib/autopilot";
  import { aiCounts, applyPlan, describeRoute, describeTrigger, type DesignRequest, type Plan } from "../lib/design";
  import { TEMPLATES, buildTemplate } from "../lib/templates";
  import type { AiSettings, ControlMapping, Show } from "../lib/types";

  let { embedded = false, onadded }: { embedded?: boolean; onadded?: (firstScene: string) => void } = $props();
  const show = app.show!;
  const s = saved;
  // Templates and the AI style scenes for the genre you've tuned detection for on Audio.
  if (!s.genre) s.genre = currentGenre(show)?.id ?? "";
  let busy = $state(false);
  let error = $state("");
  let configured = $state(true);
  let refine = $state("");
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
    toast(`${msg}.`);
    onadded?.(plan.presets.find((p) => keep.has(p.id) && show.presets.some((x) => x.id === p.id))?.id ?? "");
  }
  async function askAutopilot() {
    busy = true; error = ""; s.suggestion = null; started = Date.now();
    try {
      s.suggestion = await invoke<AutopilotPlan>("ai_autopilot", {
        show: $state.snapshot(show) as Show,
        request: { prompt: s.prompt, genre: genre ? { name: genre.name, bpm: genre.bpm, style: genre.style } : null },
      });
    } catch (e) { error = String(e); }
    busy = false;
  }
  function applySuggestion() {
    s.autopilotUndo = applyAp(show, s.suggestion!);
    s.suggestion = null;
    toast("Autopilot set up from the suggestion");
  }
  const tagChanges = $derived(s.suggestion ? show.presets.filter((p) => s.suggestion!.sections[p.id] && JSON.stringify(s.suggestion!.sections[p.id]) !== JSON.stringify(p.sections ?? [])) : []);
  const tagLabel = (x: string[] | undefined) => (x?.length ? x.map((y) => sectionInfo(y)?.name ?? y).join(", ") : "not used");
  const rulesText = (sug: AutopilotPlan) => Object.entries(sug.settings.rules ?? {}).map(([k, r]) =>
    `${sectionInfo(k)?.name}: ${[r!.brightness < 1 && `${Math.round(r!.brightness * 100)}% brightness`, !r!.strobe && "no strobe", !r!.fog && "no fog"].filter(Boolean).join(", ")}`).join("; ");

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
      case "autopilot": return "autopilot on/off";
      case "force_section": return `hold ${a.section}`;
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

<div class="designer" class:embedded>
  <div class="col form" class:panel={!embedded}>
    <h2>Add scenes</h2>
    <div class="modes">
      <button class:on={s.mode === "template"} onclick={() => (s.mode = "template")}>From a template</button>
      <button class:on={s.mode === "ai"} onclick={() => (s.mode = "ai")}>Ask the AI</button>
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
    {:else}
      <div class="modes">
        <button class:on={s.ask === "scenes"} onclick={() => (s.ask = "scenes")}>New scenes</button>
        <button class:on={s.ask === "autopilot"} onclick={() => (s.ask = "autopilot")}>Set up the autopilot</button>
      </div>
      <p class="muted small">
        {#if s.ask === "scenes"}
          Describe the night and the AI writes scenes (routes, palettes, detection tuning) for the rig you've set up, tagged for the
          autopilot, with strobe or looks only in the sections that suit them. You review everything before it's added.
        {:else}
          The AI looks at what each of your scenes switches on, tags it with the sections it suits, and suggests when to switch and
          limits per section. You see the changes before they're applied.
        {/if}
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
    </div>
    {#if genre}<p class="faint small">{genre.style}</p>{/if}
    <p class="faint small">Tuning detection for a genre is on the <button class="link" onclick={() => (app.view = "audio")}>Audio</button> screen.</p>

    {#if s.mode === "template"}
      <div class="row go">
        <button class="primary" onclick={buildFromTemplate}>Build scenes</button>
      </div>
      <p class="faint small">Template scenes are made of blocks, so you can change them on the scene afterwards.</p>
    {:else if s.ask === "autopilot"}
      <div class="row go">
        <button class="primary" onclick={askAutopilot} disabled={busy || !configured || !show.presets.length}>{busy ? "Thinking…" : "Suggest autopilot set-up"}</button>
        {#if s.autopilotUndo}<button class="ghost" onclick={() => { undoAutopilotPlan(show, s.autopilotUndo!); s.autopilotUndo = null; toast("Undone"); }}>Undo suggestion</button>{/if}
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

  <div class="col result" class:panel={!embedded}>
    {#if s.mode === "ai" && s.ask === "autopilot"}
      {#if s.suggestion}
        {@const sug = s.suggestion}
        <h2>Autopilot suggestion</h2>
        {#if sug.notes}<p class="summary">{sug.notes}</p>{/if}
        <section>
          <h3>Scene tags</h3>
          {#each tagChanges as p (p.id)}
            <div class="small">{p.name}: {tagLabel(p.sections)} → <b>{tagLabel(sug.sections[p.id])}</b></div>
          {:else}<div class="small faint">No changes to scene tags.</div>{/each}
        </section>
        <section>
          <h3>Switching</h3>
          <div class="small">Stay {sug.settings.min_bars} bars, drops cut in after {sug.settings.drop_bars}, change after {sug.settings.max_bars || "never"}, pause {sug.settings.pause_s}s after you pick a scene.</div>
          <div class="small">{rulesText(sug) ? `Limits: ${rulesText(sug)}` : "No limits per section."}</div>
        </section>
        {#each sug.warnings as w}<div class="small warn">{w}</div>{/each}
        <div class="row foot"><div class="grow"></div><button class="ghost" onclick={() => (s.suggestion = null)}>Discard</button><button class="primary" onclick={applySuggestion}>Apply</button></div>
      {:else}
        <div class="empty col"><span class="big">✦</span><span>The suggestion will appear here to review.</span></div>
      {/if}
    {:else if !plan}
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
  .designer.embedded { grid-template-columns: 1fr; height: auto; }
  .designer.embedded .result { max-height: none; min-height: 0; border-top: 1px solid var(--line); padding-top: 12px; }
  .link { padding: 0; border: 0; background: none; color: var(--accent); text-decoration: underline; font: inherit; }
  .warn { color: var(--warn); }
</style>
