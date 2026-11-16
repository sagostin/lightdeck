<script lang="ts">
  import { onMount } from "svelte";
  import { app, init, scheduleSave, setBlackout, clearManual, updateTitle } from "./lib/store.svelte";
  import FileMenu from "./components/FileMenu.svelte";
  import Live from "./views/Live.svelte";
  import Rooms from "./views/Rooms.svelte";
  import Patch from "./views/Patch.svelte";
  import Library from "./views/Library.svelte";
  import Routes from "./views/Routes.svelte";
  import Audio from "./views/Audio.svelte";
  import Outputs from "./views/Outputs.svelte";
  import AiSettings from "./views/AiSettings.svelte";
  import Zigbee from "./views/Zigbee.svelte";
  import Control from "./views/Control.svelte";
  import Designer from "./views/Designer.svelte";
  import { invoke } from "@tauri-apps/api/core";

  const nav = [
    { id: "live", label: "Live", icon: "◉" },
    { id: "rooms", label: "Rooms", icon: "▭" },
    { id: "patch", label: "Patch", icon: "⌗" },
    { id: "library", label: "Fixture types", icon: "☰" },
    { id: "routes", label: "Routes", icon: "⇢" },
    { id: "design", label: "Design", icon: "✧" },
    { id: "control", label: "Control", icon: "◎" },
    { id: "audio", label: "Audio", icon: "∿" },
    { id: "outputs", label: "Art-Net", icon: "⇄" },
    { id: "zigbee", label: "Zigbee", icon: "⌁" },
    { id: "ai", label: "AI", icon: "✦" },
  ];

  let loadError = $state("");
  onMount(() => {
    init().catch((e) => (loadError = String(e)));
    const typing = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      return t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable;
    };
    const mapped = (key: string) =>
      !!app.show?.controls.some((c) => c.enabled && c.trigger.source === "key" && c.trigger.key === key);
    const onDown = (e: KeyboardEvent) => {
      if (e.repeat || e.metaKey || e.ctrlKey || app.learning || typing(e)) return;
      const key = e.key.toLowerCase();
      if (mapped(key)) {
        e.preventDefault();
        invoke("control_key", { key, down: true });
      } else if (key === "b") setBlackout(!app.blackout);
    };
    const onUp = (e: KeyboardEvent) => {
      if (app.learning || typing(e)) return;
      const key = e.key.toLowerCase();
      if (mapped(key)) invoke("control_key", { key, down: false });
    };
    window.addEventListener("keydown", onDown);
    window.addEventListener("keyup", onUp);
    return () => { window.removeEventListener("keydown", onDown); window.removeEventListener("keyup", onUp); };
  });

  // Persist whenever the show changes.
  let first = true;
  $effect(() => {
    if (!app.show) return;
    JSON.stringify(app.show);
    updateTitle();
    if (first) { first = false; return; }
    scheduleSave();
  });

  const f = $derived(app.frame?.features);
  const beat = $derived(f ? f.beats % 4 : -1);
  const manualCount = $derived(Object.values(app.manual).reduce((n, m) => n + Object.keys(m).length, 0));
</script>

<div class="shell">
  <aside>
    <div class="brand"><span class="dot"></span> LightDeck</div>
    {#if app.show}<FileMenu />{/if}
    {#each nav as n}
      <button class="nav" class:active={app.view === n.id} onclick={() => (app.view = n.id)}>
        <span class="ic">{n.icon}</span>{n.label}
      </button>
    {/each}
    <div class="grow"></div>
    {#if app.saveError}<div class="err small">Save failed: {app.saveError}</div>{/if}
  </aside>

  <main>
    <header>
      <div class="row">
        <span class="bpm mono">{f ? Math.round(f.bpm) : "--"}</span><span class="muted small">BPM</span>
        <div class="beats">
          {#each [0, 1, 2, 3] as i}<span class:lit={i === beat}></span>{/each}
        </div>
        <div class="meter lvl"><i style="width:{(f?.level ?? 0) * 100}%"></i></div>
        {#if f && !f.running}<span class="tag warn">Audio off{f.error ? `: ${f.error}` : ""}</span>{/if}
        {#if app.frame?.send_errors.length}<span class="tag blocked" title={app.frame.send_errors.join("\n")}>Art-Net error</span>{/if}
      </div>
      <div class="row">
        {#if manualCount}
          <button onclick={() => clearManual()} title="Give control back to the routes">Release {manualCount} manual</button>
        {/if}
        <label class="master" title="Master brightness: scales every brightness channel">
          <span class="muted small">Master</span>
          <input type="range" min="0" max="1" step="0.01" value={app.master}
            oninput={(e) => { app.master = +(e.target as HTMLInputElement).value; invoke("set_master", { value: app.master }); }} />
          <span class="mono small">{Math.round(app.master * 100)}%</span>
        </label>
        <button class="blackout" class:engaged={app.blackout} onclick={() => setBlackout(!app.blackout)}>
          {app.blackout ? "Blackout on" : "Blackout"} <kbd>B</kbd>
        </button>
      </div>
    </header>

    <section class="view">
      {#if loadError}
        <div class="empty err">{loadError}</div>
      {:else if !app.show}
        <div class="empty">Loading…</div>
      {:else}{#key app.epoch}{#if app.view === "live"}<Live />
      {:else if app.view === "rooms"}<Rooms />
      {:else if app.view === "patch"}<Patch />
      {:else if app.view === "library"}<Library />
      {:else if app.view === "routes"}<Routes />
      {:else if app.view === "design"}<Designer />
      {:else if app.view === "audio"}<Audio />
      {:else if app.view === "outputs"}<Outputs />
      {:else if app.view === "zigbee"}<Zigbee />
      {:else if app.view === "control"}<Control />
      {:else if app.view === "ai"}<AiSettings />
      {/if}{/key}
      {/if}
      {#if app.toast}<div class="toast">{app.toast}</div>{/if}
    </section>
  </main>
</div>

<style>
  .shell { display: flex; height: 100%; }
  aside { width: 176px; background: var(--panel); border-right: 1px solid var(--line); padding: 12px 8px; display: flex; flex-direction: column; gap: 2px; }
  .brand { font-weight: 700; font-size: 14px; padding: 6px 10px 14px; display: flex; align-items: center; gap: 8px; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 12px var(--accent); }
  .nav { text-align: left; background: transparent; border-color: transparent; color: var(--muted); padding: 7px 10px; display: flex; gap: 10px; }
  .nav:hover { color: var(--text); background: var(--panel-2); border-color: transparent; }
  .nav.active { background: var(--accent-bg); color: var(--accent-2); }
  .ic { width: 14px; text-align: center; }
  main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  header { height: 52px; border-bottom: 1px solid var(--line); display: flex; align-items: center; justify-content: space-between; padding: 0 16px; gap: 12px; }
  .bpm { font-size: 20px; font-weight: 600; min-width: 36px; text-align: right; }
  .beats { display: flex; gap: 5px; margin: 0 6px; }
  .beats span { width: 10px; height: 10px; border-radius: 50%; border: 1px solid var(--line-2); }
  .beats span.lit { background: var(--accent); border-color: var(--accent); }
  .lvl { width: 120px; }
  .master { display: flex; align-items: center; gap: 6px; }
  .master input { width: 110px; }
  .blackout { font-weight: 600; color: var(--danger); border-color: #ff5c6c66; }
  .blackout.engaged { background: var(--danger); color: #fff; border-color: var(--danger); }
  .toast { position: fixed; bottom: 18px; left: 50%; transform: translateX(-50%); background: var(--panel-2); border: 1px solid var(--line-2); padding: 8px 14px; border-radius: 8px; box-shadow: 0 8px 24px #0008; z-index: 30; }
  .view { flex: 1; min-height: 0; overflow: auto; padding: 16px; }
</style>
