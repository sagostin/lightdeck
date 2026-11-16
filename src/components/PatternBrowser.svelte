<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { app, capabilityAt, setManual, uid, toast } from "../lib/store.svelte";
  import { setId, newMotion, describeMotion, type ChannelDef, type Look, type Motion, type Profile, type ValueNote } from "../lib/types";
  import Tags from "./Tags.svelte";
  import MotionEditor from "./MotionEditor.svelte";

  let { profile }: { profile: Profile } = $props();

  const QUICK = ["safe", "chill", "mid", "peak", "favourite", "large_projection", "blocked"];
  const browsable = $derived(profile.channels.filter((c) => ["pattern", "effect", "beam", "color"].includes(c.group)));
  let chKey = $state(untrack(() => profile.channels.find((c) => c.group === "pattern")?.key ?? profile.channels[0]?.key));
  const ch = $derived(profile.channels.find((c) => c.key === chKey));
  const previewable = $derived(app.show!.fixtures.filter((f) => f.profile_id === profile.id));
  let fxId = $state<string | null>(null);
  $effect(() => { if (!fxId && previewable[0]) fxId = previewable[0].id; });
  let value = $state(0);
  let copyTo = $state("");

  const note = $derived(ch?.values.find((n) => n.value === value));
  const cap = $derived(ch ? capabilityAt(ch, value) : undefined);
  const sorted = $derived([...(ch?.values ?? [])].sort((a, b) => a.value - b.value));

  // ----- head + secondary options -----
  const mode = $derived(profile.modes[0]);
  const head = $derived(mode?.heads.find((h) => h.channels.includes(chKey)));
  /** Other channels on the same projector (or every channel when the fixture has no heads). */
  const secondary = $derived(
    (head ? head.channels : (mode?.channels ?? []))
      .filter((k) => k !== chKey)
      .map((k) => profile.channels.find((c) => c.key === k)!)
      .filter((c) => c && c.group !== "mode"),
  );
  let include = $state<Record<string, boolean>>({});
  const fx = $derived(app.show!.fixtures.find((f) => f.id === fxId));
  /** Live value of a channel on the preview fixture. */
  function liveValue(key: string) {
    if (!fx || !mode) return 0;
    const manual = app.manual[fx.id]?.[key];
    if (manual !== undefined) return manual;
    const i = mode.channels.indexOf(key);
    return app.frame?.fixtures[fx.id]?.[i] ?? 0;
  }

  // ----- preview -----
  let touched = new Set<string>();
  function preview() {
    if (!fxId || !chKey) return;
    touched.add(`${fxId}|${chKey}`);
    setManual(fxId, chKey, value);
  }
  function setSecondary(key: string, v: number) {
    if (!fxId) return;
    include[key] = true;
    touched.add(`${fxId}|${key}`);
    setManual(fxId, key, v);
  }
  function go(v: number) {
    value = Math.max(0, Math.min(255, v));
    preview();
  }
  function releaseAll() {
    for (const t of touched) {
      const [f, c] = t.split("|");
      setManual(f, c, null);
    }
    touched = new Set();
    if (previewingOn) invoke("preview_look", { fixture: previewingOn, look: null });
    previewingOn = null;
    motions = {};
  }
  onDestroy(releaseAll);

  // ----- motions: channels that move instead of holding a value -----
  let motions = $state<Record<string, Motion>>({});
  let openMotion = $state<string | null>(null);
  let previewingOn: string | null = null;
  /** "fixed" removes the motion; any other kind creates or changes it (keeping its ranges). */
  function setMotionKind(c: ChannelDef, kind: string) {
    if (kind === "fixed") {
      delete motions[c.key];
      if (openMotion === c.key) openMotion = null;
      return;
    }
    const existing = motions[c.key];
    if (existing) {
      existing.kind = kind as Motion["kind"];
    } else {
      const cur = c.key === chKey ? value : untrack(() => liveValue(c.key));
      const capNow = capabilityAt(c, cur);
      const m = newMotion(kind as Motion["kind"], [[capNow?.min ?? 0, capNow?.max ?? 255]]);
      if (kind === "catalog") m.tags = note?.tags.filter((t) => t !== "blocked").slice(0, 1) ?? [];
      motions[c.key] = m;
    }
    if (c.key !== chKey) include[c.key] = true;
    openMotion = c.key;
  }
  const MOTION_KINDS: [string, string][] = [
    ["fixed", "Fixed"], ["sweep", "Sweep"], ["step", "Step"], ["random", "Random"],
    ["list", "List"], ["catalog", "Catalogue"], ["follow", "Follow audio"],
  ];
  /** The look being edited, as it would be saved. */
  function draftLook(): Look {
    const values: Record<string, number> = { [chKey]: value };
    for (const c of secondary) if (include[c.key]) values[c.key] = untrack(() => liveValue(c.key));
    const mo: Record<string, Motion> = {};
    for (const [k, m] of Object.entries(motions)) if (k in values) mo[k] = $state.snapshot(m) as Motion;
    return {
      id: "draft", name: "draft", head: head?.name ?? "", values, motions: mo,
      tags: [], rating: 0, sets: [], builtin: false,
    };
  }
  // The engine drops previews that stop being refreshed, so keep this one alive while open.
  $effect(() => {
    const t = setInterval(() => {
      if (previewingOn && Object.keys(motions).length) invoke("preview_look", { fixture: previewingOn, look: untrack(draftLook) });
    }, 1000);
    return () => clearInterval(t);
  });
  // Live preview through the engine while any channel has a motion.
  $effect(() => {
    JSON.stringify(motions);
    JSON.stringify(include);
    void value;
    const fx = fxId;
    const t = setTimeout(() => {
      if (!fx) return;
      const keys = Object.keys(motions);
      if (!keys.length) {
        if (previewingOn) invoke("preview_look", { fixture: previewingOn, look: null });
        previewingOn = null;
        return;
      }
      // moving channels must not be pinned by manual control, or the motion can't show
      for (const k of keys) if (app.manual[fx]?.[k] !== undefined) setManual(fx, k, null);
      previewingOn = fx;
      invoke("preview_look", { fixture: fx, look: untrack(draftLook) });
    }, 120);
    return () => clearTimeout(t);
  });

  // ----- value notes (catalogue) -----
  function ensureNote(): ValueNote {
    let n = ch!.values.find((n) => n.value === value);
    if (!n) { n = { value, label: "", tags: [], rating: 0 }; ch!.values.push(n); n = ch!.values.at(-1)!; }
    return n;
  }
  function pruneNote() {
    const i = ch!.values.findIndex((n) => n.value === value);
    const n = ch!.values[i];
    if (n && !n.tags.length && !n.label && !n.rating) ch!.values.splice(i, 1);
  }
  function toggle(tag: string) {
    const n = ensureNote();
    const i = n.tags.indexOf(tag);
    if (i >= 0) n.tags.splice(i, 1); else n.tags.push(tag);
    pruneNote();
  }
  function rate(r: number) {
    const n = ensureNote();
    n.rating = n.rating === r ? 0 : r;
    pruneNote();
  }
  /** Tags already used on this fixture type, beyond the quick ones. */
  const yourTags = $derived(
    [...new Set([...profile.channels.flatMap((c) => c.values.flatMap((n) => n.tags)), ...profile.looks.flatMap((l) => l.tags)])]
      .filter((t) => !QUICK.includes(t) && t !== "default")
      .sort(),
  );
  let newTag = $state("");
  function addTag() {
    const t = newTag.trim().toLowerCase().replace(/\s+/g, "_");
    if (t && !note?.tags.includes(t)) toggle(t);
    newTag = "";
  }
  function copyCatalog() {
    const target = profile.channels.find((c) => c.key === copyTo);
    if (!target || !ch) return;
    for (const n of ch.values) {
      const existing = target.values.find((t) => t.value === n.value);
      if (existing) Object.assign(existing, structuredClone($state.snapshot(n)));
      else target.values.push(structuredClone($state.snapshot(n)));
    }
  }

  // ----- looks -----
  // The form below edits either a new look or the look selected in "Your looks".
  let lookName = $state("");
  let lookTags = $state<string[]>([]);
  let lookSets = $state<string[]>([]);
  let lookRating = $state(0);
  let newSet = $state("");
  let editingId = $state<string | null>(null);
  const editing = $derived(profile.looks.find((l) => l.id === editingId));
  const headLooks = $derived(profile.looks.filter((l) => !l.builtin && (l.head ?? "") === (head?.name ?? "")));

  // Name, tags, sets and rating of the selected look update it straight away.
  $effect(() => {
    const snapshot = [lookName, [...lookTags], [...lookSets], lookRating] as const;
    untrack(() => {
      const l = editing;
      if (!l) return;
      if (snapshot[0].trim() && l.name !== snapshot[0].trim()) l.name = snapshot[0].trim();
      if (JSON.stringify(l.tags) !== JSON.stringify(snapshot[1])) l.tags = [...snapshot[1]];
      if (JSON.stringify(l.sets) !== JSON.stringify(snapshot[2])) l.sets = [...snapshot[2]];
      if (l.rating !== snapshot[3]) l.rating = snapshot[3];
    });
  });
  // Changing projector (head) leaves the selected look behind.
  $effect(() => {
    const h = head?.name ?? "";
    untrack(() => { if (editing && (editing.head ?? "") !== h) newLook(); });
  });

  function saveAsNew() {
    const d = draftLook();
    const look: Look = {
      ...d,
      id: uid("look"), name: lookName.trim() || `${cap?.label ?? ch?.name ?? "Look"} ${value}`,
      tags: [...lookTags, ...(note?.tags ?? []).filter((t) => !lookTags.includes(t))], rating: lookRating || note?.rating || 0,
      sets: [...lookSets],
    };
    if (editing && look.name === editing.name) look.name = `${look.name} copy`;
    profile.looks.push(look);
    const n = Object.keys(look.values).length, m = Object.keys(look.motions).length;
    toast(`Saved look "${look.name}" (${n} channel${n === 1 ? "" : "s"}${m ? `, ${m} moving` : ""})`);
    selectLook(profile.looks.at(-1)!, false);
  }
  /** Save the current channel values and motions into the selected look. */
  function updateLook() {
    if (!editing) return;
    const d = draftLook();
    editing.values = d.values;
    editing.motions = d.motions;
    toast(`Updated "${editing.name}"`);
  }
  function newLook() {
    editingId = null;
    lookName = "";
    lookTags = [];
    lookSets = [];
    lookRating = 0;
  }
  function toggleSet(id: string) {
    const i = lookSets.indexOf(id);
    if (i >= 0) lookSets.splice(i, 1); else lookSets.push(id);
  }
  function addSet() {
    const name = newSet.trim();
    if (!name) return;
    const id = setId(name);
    if (!profile.look_sets.some((s) => s.id === id)) {
      profile.look_sets.push({ id, name, color: ["#7c5cff", "#45d39a", "#f2b54a", "#ff5c6c", "#4fb3ff"][profile.look_sets.length % 5] });
    }
    if (!lookSets.includes(id)) lookSets.push(id);
    newSet = "";
  }
  /** Open a look for editing (and preview it on the fixture). */
  function selectLook(l: Look, apply = true) {
    editingId = l.id;
    lookName = l.name;
    lookTags = [...l.tags];
    lookSets = [...l.sets];
    lookRating = l.rating;
    if (!apply || !fxId) return;
    motions = structuredClone($state.snapshot(l.motions ?? {}));
    for (const c of secondary) include[c.key] = c.key in l.values;
    for (const [k, v] of Object.entries(l.values)) {
      if (!motions[k]) {
        touched.add(`${fxId}|${k}`);
        setManual(fxId, k, v);
      }
      if (k === chKey) value = v;
    }
  }
  const chName = (k: string) => profile.channels.find((c) => c.key === k)?.name ?? k;

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT") return;
    if (e.key === "ArrowRight") { go(value + (e.shiftKey ? 10 : 1)); e.preventDefault(); }
    else if (e.key === "ArrowLeft") { go(value - (e.shiftKey ? 10 : 1)); e.preventDefault(); }
    else if (e.key === "ArrowDown") { const n = sorted.find((n) => n.value > value); if (n) go(n.value); e.preventDefault(); }
    else if (e.key === "ArrowUp") { const n = [...sorted].reverse().find((n) => n.value < value); if (n) go(n.value); e.preventDefault(); }
    else if (/^[1-7]$/.test(e.key) && !e.shiftKey) toggle(QUICK[+e.key - 1]);
    else if (e.shiftKey && /^Digit[1-5]$/.test(e.code)) rate(+e.code.slice(5));
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="pb">
  <div class="col main">
    <div class="panel col">
      <div class="row">
        <label>Channel</label>
        <select bind:value={chKey}>{#each browsable as c}<option value={c.key}>{c.name}</option>{/each}</select>
        <label>Preview on</label>
        <select bind:value={fxId}>
          {#each previewable as f}<option value={f.id}>{f.name}</option>{:else}<option value={null}>No patched fixture</option>{/each}
        </select>
        <span class="grow"></span>
        <button class="ghost" onclick={releaseAll}>Release preview</button>
      </div>

      <div class="big">
        <button onclick={() => go(value - 1)} aria-label="Previous value">◀</button>
        <div class="col center grow">
          <span class="val mono">{value}</span>
          <span class="muted">{cap?.label ?? "—"}</span>
        </div>
        <button onclick={() => go(value + 1)} aria-label="Next value">▶</button>
      </div>
      <input type="range" min="0" max="255" bind:value oninput={preview} />
      {#if ch}
        <div class="row">
          <span class="muted small">This channel</span>
          <select class="kind" class:moving={!!motions[chKey]} value={motions[chKey]?.kind ?? "fixed"}
            onchange={(e) => setMotionKind(ch, (e.target as HTMLSelectElement).value)}>
            {#each MOTION_KINDS as [k, label]}<option value={k}>{label}</option>{/each}
          </select>
          {#if motions[chKey]}
            <button class="ghost summary" onclick={() => (openMotion = openMotion === chKey ? null : chKey)}>
              ⟳ {describeMotion(motions[chKey])} · {openMotion === chKey ? "hide settings" : "edit"}
            </button>
          {:else}
            <span class="small faint">holds the value above. Pick a motion to sweep, step or randomise it.</span>
          {/if}
        </div>
        {#if motions[chKey] && openMotion === chKey}<MotionEditor motion={motions[chKey]} channel={ch} />{/if}
      {/if}

      <div class="row wrap">
        {#each QUICK as t, i}
          <button class:on={note?.tags.includes(t)} class:danger={t === "blocked"} onclick={() => toggle(t)}><kbd>{i + 1}</kbd> {t.replace("_", " ")}</button>
        {/each}
      </div>
      <div class="row wrap">
        {#each yourTags as t}
          <button class="small" class:on={note?.tags.includes(t)} onclick={() => toggle(t)}>{t.replace("_", " ")}</button>
        {/each}
        <input class="tagin" list="pb-tags" bind:value={newTag} placeholder="+ your own tag" onkeydown={(e) => e.key === "Enter" && addTag()} onchange={addTag} />
        <datalist id="pb-tags">{#each yourTags as t}<option value={t}></option>{/each}</datalist>
      </div>
      <div class="row">
        <span class="muted">Rating</span>
        {#each [1, 2, 3, 4, 5] as r}
          <button class="star" class:on={(note?.rating ?? 0) >= r} onclick={() => rate(r)}>★</button>
        {/each}
        <input class="grow" placeholder="Name this value (optional)" value={note?.label ?? ""}
          onchange={(e) => { ensureNote().label = (e.target as HTMLInputElement).value; pruneNote(); }} />
      </div>
      <div class="small faint">
        <kbd>←</kbd><kbd>→</kbd> step · <kbd>Shift</kbd> ×10 · <kbd>↑</kbd><kbd>↓</kbd> jump between catalogued values · <kbd>1</kbd>–<kbd>7</kbd> quick tags · <kbd>Shift</kbd>+<kbd>1</kbd>–<kbd>5</kbd> rate.
      </div>
    </div>

    <div class="panel col">
      <div class="col">
        <h2>Secondary options{head ? ` · ${head.name}` : ""}</h2>
        <span class="small muted">
          Set other channels to go with this pattern. Each one can hold a value or move: pick Sweep, Step, Random, List,
          Catalogue or Follow, then set its ranges and timing (every N beats, seconds or route hits). Tick the ones to keep and save it as a look.
        </span>
      </div>
      <div class="sec">
        {#each secondary as c (c.key)}
          {@const v = liveValue(c.key)}
          {@const mo = motions[c.key]}
          <div class="secrow" class:inc={include[c.key]}>
            <input type="checkbox" bind:checked={include[c.key]} title="Include in the look" />
            <span class="nm" title={c.name}>{c.name}</span>
            <select class="kind" class:moving={!!mo} value={mo?.kind ?? "fixed"}
              onchange={(e) => setMotionKind(c, (e.target as HTMLSelectElement).value)} title="Hold a value, or make this channel move">
              {#each MOTION_KINDS as [k, label]}<option value={k}>{label}</option>{/each}
            </select>
            {#if mo}
              <button class="ghost summary" onclick={() => (openMotion = openMotion === c.key ? null : c.key)} title="Show or hide the motion settings">
                ⟳ {describeMotion(mo)}
              </button>
            {:else}
              <input type="range" min="0" max="255" value={v} oninput={(e) => setSecondary(c.key, +(e.target as HTMLInputElement).value)} />
            {/if}
            <span class="mono small">{v}</span>
            <span class="cl small faint" title={capabilityAt(c, v)?.label}>{capabilityAt(c, v)?.label ?? ""}</span>
          </div>
          {#if mo && openMotion === c.key}<MotionEditor motion={mo} channel={c} />{/if}
        {/each}
      </div>
      <div class="row savelook">
        {#if editing}
          <span class="tag accent">Editing “{editing.name}”</span>
          <span class="small faint grow">Name, tags, sets and rating save as you change them. Use Update look to save channel changes.</span>
          <button class="ghost" onclick={newLook}>New look</button>
        {:else}
          <span class="tag">New look</span>
          <span class="small faint grow">Pick a look under Your looks to edit it.</span>
        {/if}
      </div>
      <div class="row wrap">
        <input bind:value={lookName} placeholder="Look name, e.g. Beam 12 + zoom chase" class="grow" />
        <span class="muted small">Tags</span><Tags bind:tags={lookTags} suggestions={[...QUICK, ...yourTags]} />
      </div>
      <div class="row wrap">
        <span class="muted small">Sets</span>
        {#each profile.look_sets as s (s.id)}
          <button class="chip" style="--c:{s.color}" class:on={lookSets.includes(s.id)} aria-pressed={lookSets.includes(s.id)}
            onclick={() => toggleSet(s.id)}>{lookSets.includes(s.id) ? "✓ " : ""}{s.name}</button>
        {/each}
        <input class="tagin" bind:value={newSet} placeholder="+ new set" onkeydown={(e) => e.key === "Enter" && addSet()} />
        <span class="grow"></span>
        {#each [1, 2, 3, 4, 5] as r}<button class="star" class:on={lookRating >= r} onclick={() => (lookRating = lookRating === r ? 0 : r)}>★</button>{/each}
        {#if editing}
          <button class="primary" onclick={updateLook}>Update look</button>
          <button onclick={saveAsNew}>Save as new</button>
        {:else}
          <button class="primary" onclick={saveAsNew}>Save look</button>
        {/if}
      </div>
    </div>
  </div>

  <div class="col side">
    <div class="panel col">
      <div class="row"><h2 class="grow">Catalog</h2><span class="faint">{sorted.length}</span></div>
      <div class="scroll cat">
        {#each sorted as n}
          <button class="entry" class:cur={n.value === value} onclick={() => go(n.value)}>
            <span class="mono">{n.value}</span>
            <span class="grow lbl">{n.label || capabilityAt(ch!, n.value)?.label || ""}</span>
            <span class="stars">{"★".repeat(n.rating)}</span>
            {#each n.tags as t}<span class="tag" class:blocked={t === "blocked"} class:accent={t === "safe"}>{t}</span>{/each}
          </button>
        {:else}
          <span class="faint small">Nothing tagged yet. Step through values and tag the ones you like.</span>
        {/each}
      </div>
      <div class="row">
        <select class="grow" bind:value={copyTo}>
          <option value="">Copy catalog to…</option>
          {#each profile.channels.filter((c) => c.key !== chKey && c.group === ch?.group) as c}<option value={c.key}>{c.name}</option>{/each}
        </select>
        <button disabled={!copyTo} onclick={copyCatalog}>Copy</button>
      </div>
    </div>
    <div class="panel col">
      <div class="row"><h2 class="grow">Your looks{head ? ` · ${head.name}` : ""}</h2><span class="faint">{headLooks.length}</span></div>
      <div class="scroll cat">
        {#each headLooks as l (l.id)}
          <button class="entry" class:cur={l.id === editingId} onclick={() => selectLook(l)} title="Edit and preview this look">
            <span class="grow lbl">{l.name}</span>
            <span class="stars">{"★".repeat(l.rating)}</span>
            <span class="faint small">{Object.keys(l.values).map(chName).join(" + ")}</span>
            {#each l.tags as t}<span class="tag">{t}</span>{/each}
          </button>
        {:else}
          <span class="faint small">Save a look to see it here. Manage looks and sets on the Patterns tab.</span>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .pb { display: flex; gap: 12px; align-items: flex-start; }
  .main { flex: 1; min-width: 0; }
  .main :global(.panel) { min-width: 0; }
  .side { width: 360px; flex-shrink: 0; }
  .big { display: flex; align-items: center; gap: 12px; padding: 16px 0 6px; }
  .big button { font-size: 18px; width: 52px; height: 52px; }
  .center { align-items: center; text-align: center; }
  .val { font-size: 44px; font-weight: 600; line-height: 1; }
  .wrap { flex-wrap: wrap; }
  .small { font-size: 12px; padding: 3px 8px; }
  .tagin { width: 140px; height: 26px; font-size: 12px; border-style: dashed; }
  .star { color: var(--faint); padding: 2px 6px; }
  .star.on { color: var(--warn); background: var(--warn-bg); border-color: #f2b54a55; }
  .sec { display: grid; grid-template-columns: 1fr; gap: 2px; max-height: 340px; overflow: auto; }
  .secrow { display: grid; grid-template-columns: 20px minmax(90px, 150px) 104px minmax(100px, 1fr) 30px minmax(0, 150px); gap: 8px; align-items: center; padding: 2px 4px; border-radius: 6px; }
  .kind { height: 26px; font-size: 12px; padding: 0 6px; }
  .kind.moving { border-color: var(--accent); color: var(--accent-2); background: var(--accent-bg); }
  .summary { text-align: left; font-size: 12px; padding: 2px 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--accent-2); min-width: 0; }
  .sec { max-height: 460px; }
  .secrow.inc { background: var(--accent-bg); }
  .nm, .cl { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .savelook { border-top: 1px solid var(--line); padding-top: 8px; }
  .chip { font-size: 12px; padding: 3px 8px; border: 1px dashed color-mix(in srgb, var(--c) 45%, var(--line)); color: var(--muted); background: transparent; }
  .chip.on { background: color-mix(in srgb, var(--c) 35%, transparent); border: 1px solid var(--c); color: var(--text); font-weight: 600; }
  .cat { max-height: 320px; display: flex; flex-direction: column; gap: 2px; }
  .entry { display: flex; align-items: center; gap: 6px; text-align: left; background: transparent; border-color: transparent; flex-wrap: wrap; }
  .entry.cur { background: var(--accent-bg); border-color: #7c5cff55; }
  .lbl { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--muted); }
  .stars { color: var(--warn); font-size: 11px; }
</style>
