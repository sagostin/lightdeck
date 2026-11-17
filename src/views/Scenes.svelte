<script module lang="ts">
  // Kept across view switches.
  const ui = $state({ selected: "" as string, pane: "scene" as "scene" | "add" | "autopilot" });
</script>

<script lang="ts">
  // Scenes: everything about the show's scenes in one place. The autopilot bar on top, the
  // scenes on the left, and on the right the selected scene, adding scenes (templates or
  // AI), or the autopilot's settings.
  import { invoke } from "@tauri-apps/api/core";
  import Autopilot from "../components/Autopilot.svelte";
  import SceneDetail from "../components/SceneDetail.svelte";
  import Designer from "./Designer.svelte";
  import { sectionInfo } from "../lib/autopilot";
  import { addBlockScene, isScene, sceneSummaryText } from "../lib/scenes";
  import { newBlock } from "../lib/blocks";
  import { app, uid } from "../lib/store.svelte";

  const show = app.show!;
  const scenes = $derived(show.presets.filter(isScene));
  const selected = $derived(scenes.find((p) => p.id === ui.selected) ?? null);
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("autodesign")) ui.pane = "add";
  if (!show.presets.some((p) => p.id === ui.selected)) {
    ui.selected = show.presets.find(isScene)?.id ?? "";
    if (!ui.selected && ui.pane === "scene") ui.pane = "add";
  }

  function newScene() {
    const p = addBlockScene(show, uid("scene"));
    p.blocks!.push(newBlock("colour_hits"));
    open(p.id);
  }
  function open(id: string) {
    ui.selected = id;
    ui.pane = "scene";
  }
  function move(i: number, d: number) {
    const a = scenes[i], b = scenes[i + d];
    if (!a || !b) return;
    const ia = show.presets.indexOf(a), ib = show.presets.indexOf(b);
    [show.presets[ia], show.presets[ib]] = [show.presets[ib], show.presets[ia]];
  }
  const playing = (id: string) => app.activePresets.includes(id) || (app.autopilot?.running && app.autopilot.scene === id);
</script>

<div class="scenes-view col">
  <Autopilot mode="bar" onsettings={() => (ui.pane = ui.pane === "autopilot" ? "scene" : "autopilot")} />

  <div class="layout">
    <div class="panel col list">
      <div class="row"><h2 class="grow">Scenes</h2></div>
      {#each scenes as p, i (p.id)}
        <div class="item" class:sel={ui.pane === "scene" && ui.selected === p.id} class:playing={playing(p.id)} style="--c:{p.color}"
          role="button" tabindex="0" onclick={() => open(p.id)} onkeydown={(e) => e.key === "Enter" && open(p.id)}>
          <div class="row">
            <b class="grow nm">{p.name}</b>
            <button class="ghost x" onclick={(e) => { e.stopPropagation(); move(i, -1); }} disabled={i === 0} aria-label="Move up">↑</button>
            <button class="ghost x" onclick={(e) => { e.stopPropagation(); move(i, 1); }} disabled={i === scenes.length - 1} aria-label="Move down">↓</button>
            <button class="recall" onclick={(e) => { e.stopPropagation(); invoke("preset_apply", { preset: p.id }); }}>Recall</button>
          </div>
          <div class="row small">
            {#each p.sections ?? [] as s}<span class="dot" style="--d:{sectionInfo(s)?.color}" title={sectionInfo(s)?.name}></span>{/each}
            {#if !p.sections?.length}<span class="faint">not used by the autopilot</span>{/if}
            <span class="grow"></span>
            <span class="faint">{sceneSummaryText(show, p)}</span>
          </div>
        </div>
      {:else}
        <span class="small faint">No scenes yet.</span>
      {/each}
      <div class="row adds">
        <button class="primary" onclick={newScene}>+ New scene</button>
        <button class:on={ui.pane === "add"} onclick={() => (ui.pane = "add")}>+ From template or AI…</button>
      </div>
      <span class="small faint">Hand-set looks (presets of channel values) are on the Control screen.</span>
    </div>

    <div class="panel main">
      {#if ui.pane === "add"}
        <Designer embedded onadded={(id) => id && open(id)} />
      {:else if ui.pane === "autopilot"}
        <div class="row"><h2 class="grow">Autopilot settings</h2><button class="ghost" onclick={() => (ui.pane = "scene")}>Close</button></div>
        <Autopilot mode="settings" />
      {:else if selected}
        {#key selected.id}<SceneDetail preset={selected} onclose={() => (ui.selected = "")} />{/key}
      {:else}
        <div class="empty col">
          <span>Pick a scene on the left, make a <b>+ New scene</b>, or start from a template or the AI.</span>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .scenes-view { gap: 12px; }
  .layout { display: flex; gap: 16px; align-items: flex-start; }
  .list { width: 320px; flex-shrink: 0; gap: 6px; }
  .main { flex: 1; min-width: 0; }
  .item { padding: 6px 8px; border-radius: 8px; border: 1px solid var(--line); border-left: 4px solid var(--c); cursor: pointer; display: flex; flex-direction: column; gap: 3px; }
  .item:hover { background: #ffffff06; }
  .item.sel { border-color: var(--accent); border-left-color: var(--c); background: var(--accent-bg); }
  .item.playing .nm::after { content: " ●"; color: var(--c); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--d); display: inline-block; }
  .x { padding: 0 4px; }
  .recall { font-size: 12px; padding: 2px 8px; }
  .adds { flex-wrap: wrap; margin-top: 4px; }
  button.on { background: var(--accent-bg); border-color: var(--accent); }
  .empty { padding: 40px; align-items: center; color: var(--muted); }
  @media (max-width: 1000px) { .layout { flex-direction: column; align-items: stretch; } .list { width: auto; } }
</style>
