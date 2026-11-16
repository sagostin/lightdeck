<script lang="ts">
  // Pick a controller profile and auto-map the show to it; make or edit your own profiles by Learn.
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { app, toast, uid } from "../lib/store.svelte";
  import { BUILTIN_CONTROLLERS, DEFAULT_AUTOMAP, MAP_PREFIX, applyAutomap, automap, learnInput, type AutomapOptions } from "../lib/controllers";
  import type { ControllerInput, ControllerProfile } from "../lib/types";

  const show = app.show!;
  let mine = $state<ControllerProfile[]>([]);
  let profileId = $state(BUILTIN_CONTROLLERS[0].id);
  let opts = $state<AutomapOptions>({ ...DEFAULT_AUTOMAP });
  let editing = $state<ControllerProfile | null>(null);
  let learning = $state<"pads" | "faders" | null>(null);
  let learnFrom = 0;
  let undo = $state("");

  onMount(async () => {
    try { mine = await invoke<ControllerProfile[]>("get_controller_profiles"); } catch {}
  });
  const profiles = $derived([...mine, ...BUILTIN_CONTROLLERS]);
  const profile = $derived(profiles.find((p) => p.id === profileId) ?? null);
  const preview = $derived(automap(show, profile, opts));
  const mapped = $derived(show.controls.filter((c) => c.id.startsWith(MAP_PREFIX)).length);

  function apply() {
    undo = JSON.stringify($state.snapshot(show.controls));
    const midiWas = show.midi.enabled;
    applyAutomap(show, $state.snapshot(preview) as typeof preview);
    toast(`Mapped ${preview.assigned.length} controls${!midiWas && show.midi.enabled ? " and switched MIDI on" : ""}`);
  }
  function undoApply() {
    show.controls = JSON.parse(undo);
    undo = "";
  }

  // ----- profiles -----
  async function persist() {
    await invoke("save_controller_profiles", { profiles: $state.snapshot(mine) });
  }
  function startNew(from?: ControllerProfile) {
    const p: ControllerProfile = from
      ? { ...structuredClone($state.snapshot(from)), id: uid("ctrl"), name: `${from.name} (copy)`, builtin: undefined }
      : { id: uid("ctrl"), name: "My controller", channel: 0, pads: [], faders: [], notes: "" };
    delete p.builtin;
    editing = p;
    learning = from ? null : "pads";
    learnFrom = Date.now();
  }
  function edit(p: ControllerProfile) {
    editing = structuredClone($state.snapshot(p));
    learning = null;
  }
  async function saveEditing() {
    if (!editing) return;
    const e = $state.snapshot(editing) as ControllerProfile;
    mine = [...mine.filter((p) => p.id !== e.id), e];
    await persist();
    profileId = e.id;
    editing = null;
    learning = null;
    toast(`Saved controller "${e.name}"`);
  }
  async function remove(p: ControllerProfile) {
    mine = mine.filter((x) => x.id !== p.id);
    await persist();
    if (profileId === p.id) profileId = BUILTIN_CONTROLLERS[0].id;
  }
  function learn(as: "pads" | "faders") {
    learning = learning === as ? null : as;
    learnFrom = Date.now();
  }
  // Every new note-on / CC adds an input while learning.
  $effect(() => {
    const ev = app.midiLog[0];
    if (!ev || !editing || !learning || ev.at < learnFrom) return;
    if (ev.kind === "note" && ev.value === 0) return;
    if (learnInput(editing, ev, learning) && !editing.channel) editing.channel = 0;
  });
</script>

{#snippet inputs(list: ControllerInput[], title: string)}
  {#if list.length}
    <span class="small muted">{title}</span>
    <div class="inputs">
      {#each list as inp, i}
        <span class="tag mono">
          <input class="lbl" bind:value={inp.label} />{inp.kind === "cc" ? "CC" : "N"}{inp.number}
          <button class="ghost x" onclick={() => list.splice(i, 1)} aria-label="Remove">×</button>
        </span>
      {/each}
    </div>
  {/if}
{/snippet}

<div class="panel col">
  <div class="row"><h2 class="grow">Controller setup</h2></div>
  <span class="small muted">Map scenes, strobe, fog, blackout, master, group brightness and band sensitivity to a controller in one go. Run it again after changing scenes or the rig. It replaces only earlier auto-mapped controls{mapped ? ` (${mapped} now)` : ""}.</span>
  <div class="row">
    <select class="grow" bind:value={profileId}>
      {#if mine.length}<optgroup label="Your controllers">{#each mine as p}<option value={p.id}>{p.name}</option>{/each}</optgroup>{/if}
      <optgroup label="Built in">{#each BUILTIN_CONTROLLERS as p}<option value={p.id}>{p.name}</option>{/each}</optgroup>
    </select>
  </div>
  {#if profile}
    <span class="small faint">{profile.notes || `${profile.pads.length} pads, ${profile.faders.length} faders`}</span>
    <div class="row small flow">
      {#if profile.builtin}
        <button class="ghost" onclick={() => startNew(profile)}>Copy to edit</button>
      {:else}
        <button class="ghost" onclick={() => edit(profile)}>Edit</button>
        <button class="ghost" onclick={() => remove(profile)}>Delete</button>
      {/if}
      <button class="ghost" onclick={() => startNew()}>+ New by Learn</button>
    </div>
  {/if}

  {#if editing}
    <div class="editor col">
      <input bind:value={editing.name} placeholder="Controller name" />
      <div class="row small">
        <label for="cs-ch">Channel</label>
        <select id="cs-ch" bind:value={editing.channel}>
          <option value={0}>Any</option>
          {#each Array.from({ length: 16 }, (_, i) => i + 1) as c}<option value={c}>{c}</option>{/each}
        </select>
      </div>
      <div class="row small">
        <button class:learn={learning === "pads"} onclick={() => learn("pads")}>{learning === "pads" ? "Press pads in order…" : "Learn pads"}</button>
        <button class:learn={learning === "faders"} onclick={() => learn("faders")}>{learning === "faders" ? "Move faders in order…" : "Learn faders"}</button>
      </div>
      <span class="small faint">Press each pad (or move each fader) in the order you want them used. Pads go to scenes first.</span>
      {@render inputs(editing.pads, "Pads and buttons")}
      {@render inputs(editing.faders, "Faders and knobs")}
      <div class="row">
        <button class="primary" onclick={saveEditing} disabled={!editing.pads.length && !editing.faders.length}>Save controller</button>
        <button class="ghost" onclick={() => { editing = null; learning = null; }}>Cancel</button>
      </div>
    </div>
  {:else}
    <span class="small muted">Map</span>
    <div class="opts small">
      <label><input type="checkbox" bind:checked={opts.scenes} /> Scenes (presets)</label>
      <label><input type="checkbox" bind:checked={opts.groups} /> Route groups on/off</label>
      <label><input type="checkbox" bind:checked={opts.strobe} /> Strobe (hold)</label>
      <label><input type="checkbox" bind:checked={opts.fog} /> Fog (hold)</label>
      <label><input type="checkbox" bind:checked={opts.blackout} /> Blackout, release</label>
      <label><input type="checkbox" bind:checked={opts.master} /> Master fader</label>
      <label><input type="checkbox" bind:checked={opts.dimmers} /> Group brightness</label>
      <label><input type="checkbox" bind:checked={opts.bands} /> Band sensitivity</label>
      <label><input type="checkbox" bind:checked={opts.keys} /> Keyboard keys too</label>
    </div>
    <details>
      <summary class="small">Preview: {preview.assigned.length} controls{preview.unmapped.length ? `, ${preview.unmapped.length} won't fit` : ""}</summary>
      <div class="small preview">
        {#each preview.assigned as a}<div class="row"><span class="mono faint inp">{a.input}</span><span>{a.control.name}</span></div>{/each}
        {#if preview.unmapped.length}<div class="faint">No room for: {preview.unmapped.join(", ")}</div>{/if}
      </div>
    </details>
    <div class="row">
      <button class="primary" onclick={apply} disabled={!preview.assigned.length}>Map controller</button>
      {#if undo}<button class="ghost" onclick={undoApply}>Undo</button>{/if}
    </div>
  {/if}
</div>

<style>
  .flow { flex-wrap: wrap; }
  .editor { gap: 6px; border-top: 1px solid var(--line); padding-top: 6px; }
  .opts { display: grid; grid-template-columns: 1fr 1fr; gap: 2px 8px; }
  .opts label { display: flex; gap: 4px; align-items: center; margin: 0; }
  .inputs { display: flex; flex-wrap: wrap; gap: 4px; }
  .inputs .tag { display: inline-flex; align-items: center; gap: 3px; }
  .lbl { width: 70px; padding: 0 3px; font-size: 11px; }
  .x { padding: 0 3px; }
  .preview { max-height: 220px; overflow: auto; display: flex; flex-direction: column; gap: 2px; margin-top: 4px; }
  .inp { width: 110px; flex: none; }
  button.learn { background: var(--warn-bg); border-color: var(--warn); color: var(--warn); }
</style>
