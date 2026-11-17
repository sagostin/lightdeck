<script lang="ts">
  // One scene, laid out by the rig: for each fixture group (or fixture outside groups), the
  // look it sits in while the scene plays, the routes the scene switches on or off for it, and
  // values captured from the rig. Plus capture, autopilot tags, pads, and building blocks.
  import { invoke } from "@tauri-apps/api/core";
  import { untrack } from "svelte";
  import ConfirmButton from "./ConfirmButton.svelte";
  import SceneBlocks from "./SceneBlocks.svelte";
  import SceneEditor from "./SceneEditor.svelte";
  import { SECTIONS, toggleSection } from "../lib/autopilot";
  import { currentGenre } from "../lib/genres";
  import { captureScene, deleteScene, rebuildScene, renameScene, rigRows, routeState, rowLook, setRoutes, setRowLook, type RigRow, type RouteState } from "../lib/scenes";
  import { describeTrigger } from "../lib/design";
  import { app, clearManual, startLearn, toast, uid } from "../lib/store.svelte";
  import type { ControlMapping, Preset, Route } from "../lib/types";

  let { preset: p, onclose }: { preset: Preset; onclose: () => void } = $props();
  const show = app.show!;
  let skipped = $state<string[]>([]);
  let byGroup = $state(false);
  let name = $state(untrack(() => p.name));

  // Rebuild the scene's routes a moment after its blocks change.
  let built = untrack(() => JSON.stringify(p.blocks ?? null));
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const key = JSON.stringify(p.blocks ?? null);
    if (key === built || !p.blocks) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      built = key;
      skipped = rebuildScene(show, p.id, currentGenre(show));
    }, 350);
    return () => clearTimeout(timer);
  });

  const playing = $derived(app.scene === p.id);
  const rows = $derived(rigRows(show).filter((r) => r.fixtures.length && (r.id !== "all" || r.routes.length)));
  const valuesOn = (row: RigRow) => (p.base ?? []).filter((v) => row.id !== "all" && row.fixtures.some((f) => f.id === v.fixture));
  const NEXT: Record<RouteState, RouteState> = { leave: "on", on: "off", off: "leave" };
  const own = (r: Route) => r.scene === p.id;
  function cycle(r: Route) {
    if (own(r)) return;
    setRoutes(p, [r.id], NEXT[routeState(p, r.id)]);
  }
  function setLook(row: RigRow, look: string) {
    setRowLook(p, row, look); // a playing scene picks it up straight away
  }
  function clearValues(row: RigRow) {
    const ids = new Set(row.fixtures.map((f) => f.id));
    p.base = (p.base ?? []).filter((v) => !ids.has(v.fixture));
  }

  async function capture() {
    const manualCount = Object.values(app.manual).reduce((n, c) => n + Object.keys(c).length, 0);
    const r = captureScene(show, p, $state.snapshot(app.manual));
    // The scene now holds those values itself: give the channels back and play the scene.
    if (manualCount) await clearManual();
    await invoke("preset_apply", { preset: p.id });
    toast(`Captured ${r.routes} routes on${r.values ? ` and ${r.values} set values` : ""} into "${p.name}"`);
  }

  const triggers = $derived(show.controls.filter((c) => c.action.type === "preset" && c.action.preset === p.id));
  function assign(source: "midi" | "key") {
    const m: ControlMapping = {
      id: uid("ctl"), name: p.name, enabled: true,
      trigger: { source, kind: "note", channel: 0, number: 36, key: "", mode: "on" },
      action: { type: "preset", preset: p.id },
    };
    show.controls.push(m);
    startLearn(m.id);
    toast(source === "key" ? "Press the key for this scene… (Esc cancels)" : "Press the pad for this scene…");
  }
  $effect(() => {
    // A cancelled key Learn leaves an empty control behind: remove it.
    if (app.learning) return;
    const empty = untrack(() => show.controls.filter((c) => c.action.type === "preset" && c.action.preset === p.id && c.trigger.source === "key" && !c.trigger.key));
    if (empty.length) untrack(() => (show.controls = show.controls.filter((c) => !empty.includes(c))));
  });
</script>

<div class="detail col">
  <div class="row head">
    <input type="color" bind:value={p.color} title="Scene colour (also its pad colour)" />
    <input class="name grow" bind:value={name} onchange={() => renameScene(show, p.id, name)} />
    {#if playing}<span class="tag accent">playing</span>{/if}
    <button class="primary" onclick={() => invoke("preset_apply", { preset: p.id })}>Recall</button>
    <ConfirmButton onconfirm={() => { deleteScene(show, p.id); onclose(); }}>Delete scene</ConfirmButton>
  </div>

  <div class="row flow">
    <span class="lbl">Recalled by</span>
    {#each triggers as c (c.id)}<span class="tag">{app.learning === c.id ? "Learning…" : describeTrigger(c)}</span>{/each}
    {#if !triggers.length}<span class="small faint">nothing yet</span>{/if}
    <button class="ghost small" onclick={() => assign("midi")}>+ MIDI pad</button>
    <button class="ghost small" onclick={() => assign("key")}>+ Key</button>
    <span class="grow"></span>
    <span class="lbl">Autopilot plays it in</span>
    {#each SECTIONS as s}
      <button class="chip" class:on={p.sections?.includes(s.id)} style="--c:{s.color}" title={s.hint} onclick={() => toggleSection(p, s.id)}>{s.name}</button>
    {/each}
  </div>

  <div class="row">
    <h3 class="grow">The rig in this scene</h3>
    <button onclick={capture} title="Save which routes are on and the channels you've set by hand into this scene, then play it">Capture the rig as it is now</button>
  </div>
  <span class="small faint">
    Recalling the scene puts each fixture in its <b>look</b> and switches its <b>routes</b>; routes play on top of the look. Click a route
    to cycle <b>on</b> → <b>off</b> → <b>leave</b> (left as it is). To set this up live: switch routes on the Routes or Live screen, set
    channels or try looks by hand, then <b>Capture</b>.
  </span>

  <div class="rig col">
    {#each rows as row (row.id)}
      {@const stored = rowLook(p, row)}
      {@const look = row.looks.find((l) => l.value === stored)?.value ?? row.looks.find((l) => l.name === stored)?.value ?? stored}
      {@const vals = valuesOn(row)}
      <div class="rrow">
        <div class="who">
          <b>{row.label}</b>
          <span class="small faint">{row.id === "all" ? "routes aimed at everything" : `${row.fixtures.length} fixture${row.fixtures.length === 1 ? "" : "s"}`}</span>
        </div>
        <div class="what col">
          {#if row.looks.length}
            <div class="row flow">
              <span class="lbl">Look</span>
              <select value={look} onchange={(e) => setLook(row, e.currentTarget.value)}>
                <option value="">— none —</option>
                {#each [...new Set(row.looks.map((l) => l.group))] as g}
                  <optgroup label={g}>
                    {#each row.looks.filter((l) => l.group === g) as l}<option value={l.value}>{l.name}{l.tags.length ? ` (${l.tags.join(", ")})` : ""}{l.count < row.fixtures.length ? ` · ${l.count} of ${row.fixtures.length}` : ""}</option>{/each}
                  </optgroup>
                {/each}
                {#if look && !row.looks.some((l) => l.value === look)}<option value={look}>{look} (missing)</option>{/if}
              </select>
            </div>
          {/if}
          {#if row.routes.length}
            <div class="row flow">
              <span class="lbl">Routes</span>
              {#each row.routes as r (r.id)}
                {@const st = own(r) ? "on" : routeState(p, r.id)}
                <button class="rt s-{st}" class:own={own(r)} onclick={() => cycle(r)}
                  title={own(r) ? "Built from this scene's blocks: always on in this scene" : `${r.source} → ${r.action.type.replace("_", " ")} · click: on → off → leave`}>
                  {r.name || r.action.type}<span class="st">{own(r) ? "block" : st === "leave" ? "—" : st}</span>
                </button>
              {/each}
            </div>
          {/if}
          {#if vals.length}
            <div class="row flow small">
              <span class="lbl">Set values</span>
              <span class="faint">{vals.length} channel{vals.length === 1 ? "" : "s"} ({[...new Set(vals.map((v) => v.channel))].slice(0, 4).join(", ")}{new Set(vals.map((v) => v.channel)).size > 4 ? "…" : ""})</span>
              <button class="ghost small" onclick={() => clearValues(row)}>clear</button>
            </div>
          {/if}
          {#if !row.looks.length && !row.routes.length && !vals.length}<span class="small faint">No looks or routes for these fixtures yet.</span>{/if}
        </div>
      </div>
    {/each}
  </div>

  <h3>Build routes from blocks</h3>
  {#if p.blocks}
    <span class="small faint">Blocks make routes for this scene (colour hits, waves, strobe, laser looks, fog…). Open a block's <b>Sections…</b> to have it play only in some parts of a track, or differently in the drop.</span>
    <SceneBlocks bind:blocks={p.blocks} />
    {#each skipped as s}<span class="small warn">Skipped {s}</span>{/each}
  {:else}
    <div><button onclick={() => (p.blocks = [])}>Add blocks to this scene</button></div>
  {/if}

  <button class="ghost adv" onclick={() => (byGroup = !byGroup)}>{byGroup ? "▾" : "▸"} Switch routes by route group</button>
  {#if byGroup}<SceneEditor preset={p} />{/if}
</div>

<style>
  .detail { gap: 10px; }
  .head input[type="color"] { width: 34px; padding: 2px; }
  .name { font-size: 16px; font-weight: 600; }
  .flow { flex-wrap: wrap; gap: 6px; }
  .lbl { color: var(--faint); font-size: 12px; min-width: 38px; }
  .chip { font-size: 11px; padding: 2px 8px; color: var(--muted); }
  .chip.on { background: color-mix(in srgb, var(--c) 30%, transparent); border-color: var(--c); color: var(--text); }
  .rig { gap: 0; border: 1px solid var(--line); border-radius: 8px; }
  .rrow { display: flex; gap: 12px; padding: 8px 10px; border-top: 1px solid var(--line); }
  .rrow:first-child { border-top: 0; }
  .who { width: 150px; flex-shrink: 0; display: flex; flex-direction: column; }
  .what { flex: 1; min-width: 0; gap: 6px; }
  .what select { max-width: 100%; }
  .rt { font-size: 12px; padding: 2px 4px 2px 8px; display: inline-flex; gap: 6px; align-items: center; opacity: 0.7; }
  .rt .st { font-size: 10px; padding: 0 5px; border-radius: 6px; background: #ffffff10; text-transform: uppercase; }
  .rt.s-on { opacity: 1; border-color: var(--accent); background: var(--accent-bg); }
  .rt.s-off { opacity: 1; border-color: #ff5c6c88; }
  .rt.s-off .st { background: #ff5c6c44; }
  .rt.own { cursor: default; }
  .warn { color: var(--warn); }
  .adv { align-self: flex-start; }
  h3 { margin: 6px 0 0; }
  @media (max-width: 1100px) { .rrow { flex-direction: column; gap: 4px; } .who { width: auto; flex-direction: row; gap: 8px; align-items: baseline; } }
</style>
