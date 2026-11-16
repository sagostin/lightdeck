<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { app, isDirty, newProject, openProject, saveProject, toast } from "../lib/store.svelte";

  let open = $state(false);
  let includeKey = $state(false);
  // When there are unsaved changes, New/Open need a second click.
  let armed = $state<string | null>(null);
  const dirty = $derived(app.show ? isDirty() : false);

  function guarded(key: string, fn: () => void) {
    if (dirty && armed !== key) { armed = key; return; }
    armed = null;
    open = false;
    fn();
  }
  async function exportSettings() {
    open = false;
    try {
      const p = await invoke<string | null>("settings_export", { includeKey });
      if (p) toast(`Settings exported${includeKey ? " (with API key)" : ""}`);
    } catch (e) { toast(String(e)); }
  }
  async function importSettings() {
    open = false;
    try {
      const msg = await invoke<string | null>("settings_import");
      if (msg) toast(msg);
    } catch (e) { toast(String(e)); }
  }
  const base = (p: string) => p.split("/").pop()?.replace(/\.lightdeck$/, "") ?? p;
  const dir = (p: string) => p.split("/").slice(-3, -1).join("/");

  function onKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === "s") { e.preventDefault(); saveProject(e.shiftKey); }
    else if (e.key === "o") { e.preventDefault(); guarded("open", () => openProject()); }
  }
</script>

<svelte:window onkeydown={onKey} onpointerdown={() => { open = false; armed = null; }} />

<div class="fm" onpointerdown={(e) => e.stopPropagation()} role="presentation">
  <button class="proj" onclick={() => (open = !open)} title={app.project.path ?? "Not saved to a file yet"}>
    <span class="name">{app.project.name}</span>
    {#if dirty}<span class="dot" title="Unsaved changes"></span>{/if}
    <span class="faint">▾</span>
  </button>
  {#if open}
    <div class="menu">
      <button onclick={() => guarded("new", newProject)} class:warn={armed === "new"}>
        {armed === "new" ? "Discard changes and start new?" : "New project"}
      </button>
      <button onclick={() => guarded("open", () => openProject())} class:warn={armed === "open"}>
        <span>{armed === "open" ? "Discard changes and open?" : "Open…"}</span><kbd>⌘O</kbd>
      </button>
      <button onclick={() => { open = false; saveProject(false); }}><span>Save</span><kbd>⌘S</kbd></button>
      <button onclick={() => { open = false; saveProject(true); }}><span>Save as…</span><kbd>⇧⌘S</kbd></button>
      {#if app.project.recent.length}
        <div class="sep"></div>
        <div class="hdr">Recent</div>
        {#each app.project.recent as p}
          <button onclick={() => guarded(p, () => openProject(p))} class:warn={armed === p} title={p}>
            {#if armed === p}Discard changes and open?{:else}<span>{base(p)}</span><span class="faint small">{dir(p)}</span>{/if}
          </button>
        {/each}
      {/if}
      <div class="sep"></div>
      <div class="hdr">App settings (AI provider, band presets)</div>
      <button onclick={exportSettings}>Export settings…</button>
      <label class="chk"><input type="checkbox" bind:checked={includeKey} /> Include API key in export</label>
      <button onclick={importSettings}>Import settings…</button>
    </div>
  {/if}
</div>

<style>
  .fm { position: relative; margin-bottom: 10px; }
  .proj { width: 100%; display: flex; align-items: center; gap: 6px; text-align: left; }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--warn); }
  .menu {
    position: absolute; z-index: 20; top: calc(100% + 4px); left: 0; width: 280px;
    background: var(--panel-2); border: 1px solid var(--line-2); border-radius: 8px; padding: 4px;
    display: flex; flex-direction: column; box-shadow: 0 12px 30px #0008;
  }
  .menu button { display: flex; justify-content: space-between; align-items: center; gap: 8px; background: transparent; border-color: transparent; text-align: left; }
  .menu button:hover { background: #ffffff10; }
  .menu button.warn { background: var(--danger); color: #fff; }
  .sep { height: 1px; background: var(--line); margin: 4px 2px; }
  .hdr { font-size: 11px; color: var(--faint); padding: 4px 10px 2px; }
  .chk { display: flex; gap: 6px; align-items: center; padding: 2px 10px 6px; font-size: 12px; }
</style>
