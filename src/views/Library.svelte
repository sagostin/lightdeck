<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Tags from "../components/Tags.svelte";
  import PatternBrowser from "../components/PatternBrowser.svelte";
  import LookManager from "../components/LookManager.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import { onMount } from "svelte";
  import { app, toast } from "../lib/store.svelte";
  import { GROUPS, KINDS, type Profile } from "../lib/types";
  import { addCopy, compare, describeDiff, mergeInto, saveToLibrary } from "../lib/library";
  import { defaultBeam } from "../lib/aim";

  const show = app.show!;
  let selectedId = $state<string | null>(show.profiles[0]?.id ?? null);
  let openChan = $state<string | null>(null);
  // Dev preview only: ?tab=browser|patterns opens that tab (used for README screenshots).
  const devTab = import.meta.env.DEV ? new URLSearchParams(location.search).get("tab") : null;
  let tab = $state<"channels" | "browser" | "patterns">(devTab === "browser" || devTab === "patterns" ? devTab : "channels");
  let msg = $state("");
  const sel = $derived(show.profiles.find((p) => p.id === selectedId));

  // ----- fixture library (app-wide) -----
  let library = $state<Profile[]>([]);
  let listMode = $state<"project" | "library">("project");
  let libSelId = $state<string | null>(null);
  onMount(async () => {
    try { library = await invoke<Profile[]>("get_fixture_library"); } catch {}
  });
  const libSel = $derived(library.find((p) => p.id === libSelId) ?? null);
  const libEntry = (id: string) => library.find((p) => p.id === id) ?? null;
  const inProject = (id: string) => show.profiles.find((p) => p.id === id) ?? null;
  async function storeLibrary(next: Profile[]) {
    await invoke("save_fixture_library", { library: next });
    library = next;
  }
  async function toLibrary(p: Profile) {
    const had = !!libEntry(p.id);
    const { library: next, result } = saveToLibrary($state.snapshot(library) as Profile[], $state.snapshot(p) as Profile);
    await storeLibrary(next);
    msg = !had ? `Saved ${p.model} to the library` : `Updated ${p.model} in the library${result ? `: ${result.added} new, ${result.updated} changed look(s)` : ""}`;
  }
  function fromLibrary(target: Profile, lib: Profile) {
    const r = mergeInto(target, $state.snapshot(lib) as Profile);
    msg = `Updated ${target.model} from the library: ${r.added} new, ${r.updated} changed look(s)${r.kept ? `, kept ${r.kept} of this project's own` : ""}${r.definition ? "; channels and tags now match the library" : ""}`;
  }
  function addFromLibrary(lib: Profile) {
    const c = addCopy(show.profiles, $state.snapshot(lib) as Profile);
    listMode = "project";
    selectedId = c.id;
    msg = `Added ${c.model} to this project`;
  }
  async function removeFromLibrary(p: Profile) {
    await storeLibrary((($state.snapshot(library)) as Profile[]).filter((x) => x.id !== p.id));
    libSelId = null;
  }
  async function exportFile(profiles: Profile[], name: string) {
    try {
      const path = await invoke<string | null>("fixture_export", { profiles: $state.snapshot(profiles), name });
      if (path) msg = `Exported to ${path}`;
    } catch (e) { msg = String(e); }
  }

  // ----- importing LightDeck fixture files -----
  /** Types from a file whose id is already in the project, waiting for "merge" or "copy". */
  let conflicts = $state<Profile[]>([]);
  async function importFixtureFile(e: Event, into: "project" | "library") {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    let found: Profile[];
    try { found = await invoke<Profile[]>("parse_fixture_file", { text: await file.text() }); }
    catch (err) { msg = `${file.name}: ${err}`; return; }
    if (into === "library") {
      let next = $state.snapshot(library) as Profile[];
      for (const p of found) next = saveToLibrary(next, p).library;
      await storeLibrary(next);
      msg = `Imported ${found.length} type(s) into the library`;
      return;
    }
    const added: string[] = [];
    for (const p of found) {
      if (inProject(p.id)) conflicts.push(p);
      else { show.profiles.push(p); added.push(p.model); selectedId = p.id; }
    }
    msg = [added.length && `Imported ${added.join(", ")}`, conflicts.length && `${conflicts.length} already in this project: choose below`].filter(Boolean).join(". ");
  }
  function resolve(p: Profile, how: "merge" | "copy" | "skip") {
    conflicts = conflicts.filter((c) => c !== p);
    const existing = inProject(p.id);
    if (how === "merge" && existing) { mergeInto(existing, $state.snapshot(p) as Profile); selectedId = existing.id; toast(`Merged into ${existing.model}`); }
    if (how === "copy") selectedId = addCopy(show.profiles, $state.snapshot(p) as Profile).id;
  }

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
    <div class="modes">
      <button class:on={listMode === "project"} onclick={() => (listMode = "project")}>This project <span class="faint">{show.profiles.length}</span></button>
      <button class:on={listMode === "library"} onclick={() => (listMode = "library")}>Library <span class="faint">{library.length}</span></button>
    </div>
    {#if listMode === "project"}
      {#each show.profiles as p}
        {@const l = libEntry(p.id)}
        <button class="item" class:on={p.id === selectedId} onclick={() => (selectedId = p.id)}>
          <span>{p.model}{#if l}<span class="lib-dot" class:diff={!compare(p, l).same} title={compare(p, l).same ? "In the library, same" : "In the library, differs"}>●</span>{/if}</span>
          <span class="faint small">{p.manufacturer} · {p.kind}</span>
        </button>
      {/each}
      <label class="file"><input type="file" accept=".qxf,.xml" multiple onchange={importFiles} />Import QLC+ .qxf</label>
      <label class="file"><input type="file" accept=".lightdeck-fixture,.json,.lightdeck,.lightdeck-settings" onchange={(e) => importFixtureFile(e, "project")} />Import LightDeck fixture</label>
      <button onclick={() => (aiOpen = !aiOpen)} class:on={aiOpen}>✦ Interpret with AI</button>
    {:else}
      {#each library as p}
        <button class="item" class:on={p.id === libSelId} onclick={() => (libSelId = p.id)}>
          <span>{p.model}{#if inProject(p.id)}<span class="faint small"> · in project</span>{/if}</span>
          <span class="faint small">{p.manufacturer} · {p.kind} · {p.looks.length} looks</span>
        </button>
      {:else}
        <span class="small faint">Empty. Open a fixture type in this project and click <b>Save to library</b>.</span>
      {/each}
      <label class="file"><input type="file" accept=".lightdeck-fixture,.json,.lightdeck,.lightdeck-settings" onchange={(e) => importFixtureFile(e, "library")} />Import into library</label>
      {#if library.length}<button onclick={() => exportFile(library, "LightDeck library")}>Export whole library…</button>{/if}
    {/if}
    {#if msg}<div class="small muted">{msg}</div>{/if}
  </div>

  <div class="col grow">
    {#each conflicts as c (c)}
      <div class="panel row conflict">
        <span class="grow"><b>{c.model}</b> is already in this project. Merge the file's version into it (its channels and tags, new and changed looks; this project's own looks are kept), or add it as a separate type?</span>
        <button class="primary" onclick={() => resolve(c, "merge")}>Merge</button>
        <button onclick={() => resolve(c, "copy")}>Add as copy</button>
        <button class="ghost" onclick={() => resolve(c, "skip")}>Skip</button>
      </div>
    {/each}

    {#if listMode === "library"}
      {#if libSel}
        {@const here = inProject(libSel.id)}
        <div class="panel col">
          <div class="row">
            <h2 class="grow">{libSel.manufacturer} {libSel.model}</h2>
            <button onclick={() => exportFile([libSel], libSel.model)}>Export…</button>
            <ConfirmButton class="danger" onconfirm={() => removeFromLibrary(libSel)}>Remove from library</ConfirmButton>
          </div>
          <span class="small muted">{libSel.kind} · {libSel.channels.length} channels · {libSel.modes.length} mode(s) · {libSel.looks.filter((l) => !l.builtin).length} saved looks{libSel.look_sets.length ? ` · sets: ${libSel.look_sets.map((x) => x.name).join(", ")}` : ""}</span>
          {#if here}
            <div class="row">
              <span class="grow small">In this project: {describeDiff(compare(here, libSel), "library")}</span>
              {#if !compare(here, libSel).same}<button class="primary" onclick={() => fromLibrary(here, libSel)}>Update this project's copy</button>{/if}
              <button onclick={() => { listMode = "project"; selectedId = here.id; }}>Open</button>
            </div>
          {:else}
            <div class="row"><span class="grow small muted">Not in this project.</span><button class="primary" onclick={() => addFromLibrary(libSel)}>Add to this project</button></div>
          {/if}
          <span class="small faint">Library types are edited in a project: add or open it there, make changes, then click Save to library.</span>
        </div>
      {:else}
        <div class="empty panel col">
          <span>The fixture library keeps fixture types and their looks for every project.</span>
          <span class="small faint">Projects copy from it, so a project file always works on its own and changing the library never changes an existing show. Pick a type on the left.</span>
        </div>
      {/if}
    {:else}
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
          {#if sel.kind !== "laser" && sel.kind !== "fog"}
            <label class="beam small" title="Beam angle, used to draw where aimed fixtures of this type light. Each fixture can override it.">Beam
              <input type="number" min="0" max="170" step="1" placeholder={String(defaultBeam(sel.kind))}
                value={sel.beam_deg || ""} oninput={(e) => (sel!.beam_deg = +e.currentTarget.value || 0)} />°</label>
          {/if}
          <button onclick={aiRetag}>✦ AI tag</button>
          <button onclick={() => exportFile([sel], sel.model)}>Export…</button>
          <ConfirmButton class="danger" onconfirm={() => removeProfile(sel)}>Delete</ConfirmButton>
        </div>
        {#if libEntry(sel.id)}
          {@const lib = libEntry(sel.id)!}
          {@const d = compare(sel, lib)}
          <div class="row small libstat">
            <span class="grow">Library: {describeDiff(d, "library")}</span>
            {#if !d.same}
              <button onclick={() => toLibrary(sel)} title="Copy this project's version to the library (library-only looks are kept)">Save to library</button>
              <button onclick={() => fromLibrary(sel, lib)} title="Take the library's channels and tags, and its new and changed looks (this project's own looks are kept)">Update from library</button>
            {/if}
          </div>
        {:else}
          <div class="row small libstat">
            <span class="grow muted">Not in the library yet. Save it to reuse it, with its looks, in other projects.</span>
            <button onclick={() => toLibrary(sel)}>Save to library</button>
          </div>
        {/if}
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
      <div class="empty panel">Import a QLC+ fixture, add one from the Library, or interpret one with AI to get started.</div>
    {/if}
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
  .beam { display: flex; align-items: center; gap: 4px; margin: 0; white-space: nowrap; }
  .beam input { width: 56px; }
  .modes { display: flex; gap: 4px; }
  .modes button { flex: 1; font-size: 12px; padding: 4px 6px; }
  .modes button.on, .item.on { background: var(--accent-bg); border-color: var(--accent); }
  .lib-dot { color: var(--ok, #45d39a); font-size: 9px; margin-left: 6px; vertical-align: middle; }
  .lib-dot.diff { color: var(--warn, #f2b54a); }
  .libstat { border-top: 1px solid var(--line); padding-top: 6px; }
  .conflict { border-color: var(--warn); align-items: center; }
  .result { background: var(--accent-bg); border-color: #7c5cff55; }
</style>
