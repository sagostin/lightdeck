<script lang="ts">
  // Pattern manager for one fixture type: your looks, default looks, and look sets.
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, untrack } from "svelte";
  import ConfirmButton from "./ConfirmButton.svelte";
  import Menu from "./Menu.svelte";
  import Tags from "./Tags.svelte";
  import { app, setManual, toast, uid } from "../lib/store.svelte";
  import { setId, describeMotion, type Look, type Profile } from "../lib/types";

  let { profile }: { profile: Profile } = $props();

  let filter = $state<"all" | "yours" | "defaults">("all");
  let setFilter = $state("");
  let search = $state("");
  let perRange = $state(4);
  let newSet = $state("");
  let previewFx = $state<string | null>(null);
  const previewable = $derived(app.show!.fixtures.filter((f) => f.profile_id === profile.id));
  $effect(() => { if (!previewFx && previewable[0]) previewFx = previewable[0].id; });

  const looks = $derived(
    profile.looks.filter((l) =>
      (filter === "all" || (filter === "yours" ? !l.builtin : l.builtin)) &&
      (!setFilter || l.sets.includes(setFilter)) &&
      (!search || [l.name, ...l.tags].join(" ").toLowerCase().includes(search.toLowerCase())),
    ),
  );
  const counts = $derived({ yours: profile.looks.filter((l) => !l.builtin).length, defaults: profile.looks.filter((l) => l.builtin).length });
  const chName = (k: string) => profile.channels.find((c) => c.key === k)?.name ?? k;
  let openInfo = $state<string | null>(null);
  const channelSummary = (l: Look) =>
    Object.entries(l.values).map(([k, v]) => `${chName(k)}: ${l.motions?.[k] ? describeMotion(l.motions[k]) : v}`).join("\n");
  const usedBy = (setIdv: string) => app.show!.routes.filter((r) => r.action.type === "look" && r.action.set === setIdv).length;

  async function generate() {
    const p = await invoke<Profile>("generate_default_looks", { profile: $state.snapshot(profile), perRange });
    profile.looks = p.looks;
    toast(`Generated ${p.looks.filter((l) => l.builtin).length} default looks`);
  }
  function addSet() {
    const name = newSet.trim();
    if (!name) return;
    const id = setId(name);
    if (!profile.look_sets.some((s) => s.id === id))
      profile.look_sets.push({ id, name, color: ["#7c5cff", "#45d39a", "#f2b54a", "#ff5c6c", "#4fb3ff"][profile.look_sets.length % 5] });
    newSet = "";
  }
  function removeSet(id: string) {
    profile.look_sets = profile.look_sets.filter((s) => s.id !== id);
    for (const l of profile.looks) l.sets = l.sets.filter((s) => s !== id);
    if (setFilter === id) setFilter = "";
  }
  function toggleSet(l: Look, id: string) {
    const i = l.sets.indexOf(id);
    if (i >= 0) l.sets.splice(i, 1); else l.sets.push(id);
  }
  function keep(l: Look) {
    l.builtin = false;
    l.tags = l.tags.filter((t) => t !== "default");
  }
  function duplicate(l: Look) {
    const copy: Look = { ...structuredClone($state.snapshot(l)), id: uid("look"), name: `${l.name} copy`, builtin: false };
    copy.tags = copy.tags.filter((t) => t !== "default");
    profile.looks.splice(profile.looks.indexOf(l) + 1, 0, copy);
  }
  // Previews run through the engine and stop when you leave this tab (or the app reloads),
  // so nothing stays stuck on a look you only meant to try.
  let previewing = $state<{ fx: string; look: Look } | null>(null);
  function preview(l: Look) {
    if (!previewFx) { toast("Patch a fixture of this type to preview looks"); return; }
    for (const k of Object.keys(l.values)) setManual(previewFx, k, null);
    previewing = { fx: previewFx, look: $state.snapshot(l) as Look };
    invoke("preview_look", { fixture: previewing.fx, look: previewing.look });
  }
  function stopPreview() {
    if (previewing) invoke("preview_look", { fixture: previewing.fx, look: null });
    previewing = null;
  }
  $effect(() => {
    const t = setInterval(() => {
      const p = untrack(() => previewing);
      if (p) invoke("preview_look", { fixture: p.fx, look: p.look });
    }, 1000);
    return () => clearInterval(t);
  });
  onDestroy(stopPreview);
</script>

<div class="lm">
  <div class="col side">
    <div class="panel col">
      <h2>Sets</h2>
      <span class="small muted">Group looks, e.g. Chill, Build, Drop. Routes can pick from a set. Same-named sets on different fixture types work together.</span>
      <button class="item" class:on={!setFilter} onclick={() => (setFilter = "")}>All looks <span class="faint">{profile.looks.length}</span></button>
      {#each profile.look_sets as s (s.id)}
        <div class="row setrow" style="--c:{s.color}">
          <button class="item grow" class:on={setFilter === s.id} onclick={() => (setFilter = s.id)}>
            <span class="dot"></span>{s.name} <span class="faint">{profile.looks.filter((l) => l.sets.includes(s.id)).length}</span>
          </button>
          <input type="color" bind:value={s.color} />
          <ConfirmButton onconfirm={() => removeSet(s.id)} confirmLabel={usedBy(s.id) ? "Routes use it. Delete?" : "Delete?"}>×</ConfirmButton>
        </div>
      {/each}
      <input bind:value={newSet} placeholder="+ new set (Enter)" onkeydown={(e) => e.key === "Enter" && addSet()} />
    </div>
    <div class="panel col">
      <h2>Default looks</h2>
      <span class="small muted">
        Generated from this fixture's own pattern ranges, spread evenly through each range and tagged <span class="tag">default</span>.
        Regenerating replaces defaults only; your looks are never touched. "Keep" turns a default into one of yours.
      </span>
      <div class="row"><label>Per range</label><input type="number" min="1" max="32" bind:value={perRange} /></div>
      <div class="row">
        <button onclick={generate}>{counts.defaults ? "Regenerate" : "Generate"} defaults</button>
        {#if counts.defaults}
          <ConfirmButton onconfirm={() => (profile.looks = profile.looks.filter((l) => !l.builtin))}>Remove defaults</ConfirmButton>
        {/if}
      </div>
    </div>
  </div>

  <div class="panel col grow">
    <div class="row">
      <h2 class="grow">Looks</h2>
      <button class:on={filter === "all"} onclick={() => (filter = "all")}>All</button>
      <button class:on={filter === "yours"} onclick={() => (filter = "yours")}>Yours {counts.yours}</button>
      <button class:on={filter === "defaults"} onclick={() => (filter = "defaults")}>Defaults {counts.defaults}</button>
      <input bind:value={search} placeholder="Search names and tags" />
      <label>Preview on</label>
      <select bind:value={previewFx}>{#each previewable as f}<option value={f.id}>{f.name}</option>{:else}<option value={null}>—</option>{/each}</select>
    </div>
    {#if !looks.length}
      <div class="empty">
        {profile.looks.length ? "No looks match." : "No looks yet. Save one from the Pattern browser, or generate defaults."}
      </div>
    {:else}
      <div class="looklist">
        {#each looks as l (l.id)}
          {@const n = Object.keys(l.values).length}
          {@const moving = Object.keys(l.motions ?? {}).length}
          <div class="look" class:builtin={l.builtin} class:active={previewing?.look.id === l.id}>
            <div class="line">
              <input class="name" bind:value={l.name} aria-label="Look name" />
              {#if l.head}<span class="faint small">{l.head}</span>{/if}
              {#if l.builtin}<span class="tag">default</span>{/if}
              <button class="ghost info" class:open={openInfo === l.id} onclick={() => (openInfo = openInfo === l.id ? null : l.id)}
                title={channelSummary(l)} aria-expanded={openInfo === l.id}>ⓘ {n} ch{moving ? ` · ${moving} moving` : ""}</button>
              <span class="stars">
                {#each [1, 2, 3, 4, 5] as r}<button class="star" class:on={l.rating >= r} onclick={() => (l.rating = l.rating === r ? 0 : r)} aria-label="Rate {r}">★</button>{/each}
              </span>
              {#if previewing?.look.id === l.id}
                <button class="on" onclick={stopPreview}>Stop</button>
              {:else}
                <button onclick={() => preview(l)}>Preview</button>
              {/if}
              <Menu label="Actions for {l.name}">
                {#if l.builtin}<button data-close onclick={() => keep(l)}>Keep as my look</button>{/if}
                <button data-close onclick={() => duplicate(l)}>Duplicate</button>
                <button data-close onclick={() => (openInfo = l.id)}>Show channels</button>
                <ConfirmButton class="danger" onconfirm={() => profile.looks.splice(profile.looks.indexOf(l), 1)} confirmLabel="Click again to delete">Delete</ConfirmButton>
              </Menu>
            </div>
            <div class="line meta">
              <span class="lbl">Tags</span><Tags bind:tags={l.tags} suggestions={["breakdown", "groove", "build", "drop", "chill", "peak"]} />
              <span class="lbl">Sets</span>
              {#each profile.look_sets as s (s.id)}
                <button class="chip" style="--c:{s.color}" class:on={l.sets.includes(s.id)} aria-pressed={l.sets.includes(s.id)}
                  onclick={() => toggleSet(l, s.id)}>{l.sets.includes(s.id) ? "✓ " : ""}{s.name}</button>
              {:else}<span class="faint small">add sets on the left</span>{/each}
            </div>
            {#if openInfo === l.id}
              <div class="details">
                <div class="chlist">
                  {#each Object.entries(l.values) as [k, v]}
                    <div class:moving={!!l.motions?.[k]}>
                      <span class="chn">{chName(k)}</span>
                      <span class="mono">{l.motions?.[k] ? `⟳ ${describeMotion(l.motions[k])}` : v}</span>
                    </div>
                  {/each}
                </div>
                <div class="small faint">To change channels or motions, select this look under Your looks in the Pattern browser, edit it, then click Update look.</div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
      <div class="row small">
        {#if previewing}
          <span class="tag accent">Previewing “{previewing.look.name}”</span>
          <button class="ghost" onclick={stopPreview}>Stop preview</button>
        {:else}
          <span class="faint">Preview shows a look on the fixture until you stop it or leave this tab; routes take over again afterwards.</span>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .lm { display: flex; gap: 12px; align-items: flex-start; }
  .side { width: 280px; flex-shrink: 0; }
  .item { text-align: left; display: flex; gap: 6px; align-items: center; }
  .setrow { gap: 4px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--c); }
  input[type="color"] { width: 28px; padding: 2px; }
  input[type="number"] { width: 64px; }

  .moving { color: var(--accent-2); }
  .chip { font-size: 11px; padding: 2px 6px; border: 1px dashed color-mix(in srgb, var(--c) 45%, var(--line)); color: var(--muted); background: transparent; }
  .chip.on { background: color-mix(in srgb, var(--c) 35%, transparent); border: 1px solid var(--c); color: var(--text); font-weight: 600; }
  .looklist { display: flex; flex-direction: column; gap: 6px; }
  .look { border: 1px solid var(--line); border-radius: 8px; padding: 6px 8px; display: flex; flex-direction: column; gap: 4px; }
  .look.active { border-color: var(--accent); background: var(--accent-bg); }
  .look.builtin .name { color: var(--muted); }
  .line { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; min-width: 0; }
  .line .name { flex: 1; min-width: 120px; }
  .meta { padding-left: 2px; }
  .lbl { color: var(--faint); font-size: 11px; }
  .details { border-top: 1px solid var(--line); padding-top: 6px; }
  .star { color: var(--faint); padding: 0 3px; border: none; background: none; }
  .star.on { color: var(--warn); }
  .stars { white-space: nowrap; }
  .info { font-size: 12px; padding: 2px 6px; white-space: nowrap; color: var(--muted); }
  .info.open { color: var(--accent-2); }
  .chlist { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 3px 16px; margin-bottom: 6px; font-size: 12px; }
  .chlist > div { display: flex; justify-content: space-between; gap: 8px; border-bottom: 1px solid #ffffff08; padding: 2px 0; }
  .chn { color: var(--muted); }
</style>
