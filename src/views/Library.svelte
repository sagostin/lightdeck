<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Tags from "../components/Tags.svelte";
  import PatternBrowser from "../components/PatternBrowser.svelte";
  import LookManager from "../components/LookManager.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import { app } from "../lib/store.svelte";
  import { GROUPS, KINDS, type Profile } from "../lib/types";

  const show = app.show!;
  let selectedId = $state<string | null>(show.profiles[0]?.id ?? null);
  let openChan = $state<string | null>(null);
  let tab = $state<"channels" | "browser" | "patterns">("channels");
  let msg = $state("");
  const sel = $derived(show.profiles.find((p) => p.id === selectedId));

  // ----- import -----
  async function importFiles(e: Event) {
    const files = (e.target as HTMLInputElement).files;
    if (!files) return;
    const done: string[] = [];
    for (const file of Array.from(files)) {
      try {
        const p = await invoke<Profile>("import_qxf", { xml: await file.text() });
        addProfile(p);
        done.push(p.model);
      } catch (err) {
        msg = `${file.name}: ${err}`;
        return;
      }
    }
    msg = `Imported ${done.join(", ")}`;
    (e.target as HTMLInputElement).value = "";
  }
  function addProfile(p: Profile, replace = false) {
    const existing = show.profiles.findIndex((x) => x.id === p.id);
    if (existing >= 0 && replace) show.profiles[existing] = p;
    else {
      if (existing >= 0) p.id = `${p.id}-${Math.random().toString(36).slice(2, 6)}`;
      show.profiles.push(p);
    }
    selectedId = p.id;
  }
  function removeProfile(p: Profile) {
    const used = show.fixtures.filter((f) => f.profile_id === p.id).length;
    if (used) { msg = `${p.model} is used by ${used} patched fixture(s); remove those first.`; return; }
    show.profiles.splice(show.profiles.indexOf(p), 1);
    selectedId = show.profiles[0]?.id ?? null;
  }
  async function exportProfile(p: Profile) {
    await navigator.clipboard.writeText(JSON.stringify($state.snapshot(p), null, 2));
    msg = `Copied ${p.model} as LightDeck JSON to the clipboard`;
  }
  async function importJson(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try { addProfile(JSON.parse(await file.text())); msg = "Imported LightDeck profile"; }
    catch (err) { msg = String(err); }
  }

  // ----- AI -----
  let aiOpen = $state(false);
  let aiInput = $state("");
  let aiInstruction = $state("Convert this into a LightDeck profile.");
  let aiBusy = $state(false);
  let aiResult = $state<Profile | null>(null);
  let aiError = $state("");
  async function aiRun(input = aiInput, instruction = aiInstruction) {
    aiBusy = true; aiError = ""; aiResult = null;
    try { aiResult = await invoke<Profile>("ai_interpret", { input, instruction }); }
    catch (e) { aiError = String(e); }
    aiBusy = false;
  }
  function aiRetag() {
    if (!sel) return;
    aiOpen = true;
    aiInput = JSON.stringify($state.snapshot(sel), null, 1);
    aiInstruction = "Review this existing LightDeck profile. Keep every channel key, mode and value range exactly as they are, but improve groups and capability tags (especially blocked, off, static/dynamic and large_projection). Keep the existing values arrays unchanged.";
    aiRun();
  }
  async function aiFile(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) aiInput = await f.text();
  }

  function sortCaps(chKey: string) {
    const ch = sel?.channels.find((c) => c.key === chKey);
    ch?.capabilities.sort((a, b) => a.min - b.min);
  }
</script>

<div class="lib">
  <div class="col list">
    <h2>Fixture types</h2>
    {#each show.profiles as p}
      <button class="item" class:on={p.id === selectedId} onclick={() => (selectedId = p.id)}>
        <span>{p.model}</span><span class="faint small">{p.manufacturer} · {p.kind}</span>
      </button>
    {/each}
    <label class="file"><input type="file" accept=".qxf,.xml" multiple onchange={importFiles} />Import QLC+ .qxf</label>
    <label class="file"><input type="file" accept=".json" onchange={importJson} />Import LightDeck .json</label>
    <button onclick={() => (aiOpen = !aiOpen)} class:on={aiOpen}>✦ Interpret with AI</button>
    {#if msg}<div class="small muted">{msg}</div>{/if}
  </div>

  <div class="col grow">
    {#if aiOpen}
      <div class="panel col">
        <div class="row"><h2 class="grow">Interpret with AI</h2><button class="ghost" onclick={() => (app.view = "ai")}>AI settings</button></div>
        <span class="muted small">Paste a manual's DMX chart, an Open Fixture Library JSON, a QLC+ file or anything else. The reply is converted into LightDeck's format for you to review.</span>
        <textarea rows="6" bind:value={aiInput} placeholder="CH1: 0-9 off, 10-255 dimmer…"></textarea>
        <div class="row">
          <label class="file"><input type="file" onchange={aiFile} />Load text file</label>
          <input class="grow" bind:value={aiInstruction} />
          <button class="primary" disabled={aiBusy || !aiInput.trim()} onclick={() => aiRun()}>{aiBusy ? "Working…" : "Interpret"}</button>
        </div>
        {#if aiError}<div class="err">{aiError}</div>{/if}
        {#if aiResult}
          <div class="row panel result">
            <span class="grow">{aiResult.manufacturer} {aiResult.model} · {aiResult.kind} · {aiResult.channels.length} channels · {aiResult.modes.length} mode(s)</span>
            <button onclick={() => { addProfile(aiResult!); aiResult = null; }}>Add as new type</button>
            {#if sel}<button class="primary" onclick={() => { const r = aiResult!; r.id = sel!.id; addProfile(r, true); aiResult = null; }}>Replace {sel.model}</button>{/if}
          </div>
        {/if}
      </div>
    {/if}

    {#if sel}
      <div class="panel col">
        <div class="row">
          <input bind:value={sel.manufacturer} placeholder="Manufacturer" />
          <input class="grow" bind:value={sel.model} placeholder="Model" />
          <select bind:value={sel.kind}>{#each KINDS as k}<option>{k}</option>{/each}</select>
          <button onclick={aiRetag}>✦ AI tag</button>
          <button onclick={() => exportProfile(sel)}>Copy JSON</button>
          <ConfirmButton class="danger" onconfirm={() => removeProfile(sel)}>Delete</ConfirmButton>
        </div>
        <div class="row small muted">
          {#each sel.modes as m}<span class="tag">{m.name}: {m.channels.length} ch{m.heads.length ? `, ${m.heads.length} projectors` : ""}</span>{/each}
          <span class="grow"></span>
          <button class:on={tab === "channels"} onclick={() => (tab = "channels")}>Channels and tags</button>
          <button class:on={tab === "browser"} onclick={() => (tab = "browser")}>Pattern browser</button>
          <button class:on={tab === "patterns"} onclick={() => (tab = "patterns")}>Patterns{sel.looks.length ? ` (${sel.looks.length})` : ""}</button>
        </div>
      </div>

      {#if tab === "browser"}
        {#key sel.id}<PatternBrowser profile={sel} />{/key}
      {:else if tab === "patterns"}
        {#key sel.id}<LookManager profile={sel} />{/key}
      {:else}
        <div class="panel">
          <table>
            <thead><tr><th>Channel</th><th>Group</th><th>Default</th><th>Ranges</th><th>Catalogued</th><th></th></tr></thead>
            <tbody>
              {#each sel.channels as ch (ch.key)}
                <tr class="clickable" class:sel={openChan === ch.key} onclick={() => (openChan = openChan === ch.key ? null : ch.key)}>
                  <td><input bind:value={ch.name} onclick={(e) => e.stopPropagation()} /> <span class="faint mono small">{ch.key}</span></td>
                  <td><select bind:value={ch.group} onclick={(e) => e.stopPropagation()}>{#each GROUPS as g}<option>{g}</option>{/each}</select></td>
                  <td><input type="number" min="0" max="255" bind:value={ch.default} onclick={(e) => e.stopPropagation()} /></td>
                  <td class="faint">{ch.capabilities.length}
                    {#if ch.capabilities.some((c) => c.tags.includes("blocked"))}<span class="tag blocked">blocked ranges</span>{/if}</td>
                  <td class="faint">{ch.values.length || ""}</td>
                  <td class="faint">{openChan === ch.key ? "▾" : "▸"}</td>
                </tr>
                {#if openChan === ch.key}
                  <tr><td colspan="6">
                    <table class="caps">
                      <thead><tr><th>From</th><th>To</th><th>Label</th><th>Colour</th><th>Tags</th><th></th></tr></thead>
                      <tbody>
                        {#each ch.capabilities as cap, i}
                          <tr class:blk={cap.tags.includes("blocked")}>
                            <td><input type="number" min="0" max="255" bind:value={cap.min} onchange={() => sortCaps(ch.key)} /></td>
                            <td><input type="number" min="0" max="255" bind:value={cap.max} /></td>
                            <td><input class="lbl" bind:value={cap.label} /></td>
                            <td>{#if cap.color}<span class="sw" style="background:{cap.color}"></span>{/if}</td>
                            <td><Tags bind:tags={cap.tags} /></td>
                            <td><button class="ghost" onclick={() => ch.capabilities.splice(i, 1)}>×</button></td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                    <button class="ghost" onclick={() => { const last = ch.capabilities.at(-1); ch.capabilities.push({ min: last ? Math.min(255, last.max + 1) : 0, max: 255, label: "", color: null, tags: [] }); }}>+ Add range</button>
                  </td></tr>
                {/if}
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {:else}
      <div class="empty panel">Import a QLC+ fixture or interpret one with AI to get started.</div>
    {/if}
  </div>
</div>

<style>
  .lib { display: flex; gap: 16px; align-items: flex-start; }
  .list { width: 220px; flex-shrink: 0; }
  .item { text-align: left; display: flex; flex-direction: column; gap: 2px; }
  .file { display: block; text-align: center; padding: 6px 10px; border: 1px dashed var(--line-2); border-radius: 6px; cursor: pointer; color: var(--text); }
  .file:hover { background: var(--panel-2); }
  .file input { display: none; }
  td input:not([type="number"]) { width: 100%; max-width: 260px; }
  .caps td input[type="number"] { width: 58px; }
  .caps .lbl { min-width: 240px; }
  .caps tr.blk td { background: #ff5c6c0c; }
  .sw { display: inline-block; width: 16px; height: 16px; border-radius: 4px; border: 1px solid var(--line-2); }
  .result { background: var(--accent-bg); border-color: #7c5cff55; }
</style>
