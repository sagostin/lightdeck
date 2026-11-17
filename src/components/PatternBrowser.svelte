<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { app, capabilityAt, setManual, uid, toast } from "../lib/store.svelte";
  import { setId, newMotion, describeMotion, type ChannelDef, type Look, type Motion, type Profile, type ValueNote } from "../lib/types";
  import Tags from "./Tags.svelte";
  import MotionEditor from "./MotionEditor.svelte";
  import { sectionInfo } from "../lib/autopilot";
  import { lookMainKey } from "../lib/looks";

  let { profile }: { profile: Profile } = $props();

  const QUICK = ["safe", "chill", "mid", "peak", "favourite", "large_projection", "blocked"];
  /** Look tags that look routes following the section pick by. */
  const SECTION_TAGS = ["breakdown", "groove", "build", "drop"];
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
  /** Values of the ticked channels, as the look will save them. Kept here rather than read
   *  back from the fixture, so routes, scenes or a head's automatic on/off can't leak in. */
  let held = $state<Record<string, number>>({});
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
    held[key] = v;
    touched.add(`${fxId}|${key}`);
    setManual(fxId, key, v);
  }
  /** Tick: the channel joins the look at its current value, held there. Untick: it leaves
   *  the look and goes back to whatever else drives it. */
  function setIncluded(key: string, on: boolean) {
    include[key] = on;
    if (on) {
      held[key] = untrack(() => liveValue(key));
      if (fxId && !motions[key]) setSecondary(key, held[key]);
    } else {
      delete held[key];
      if (fxId && touched.has(`${fxId}|${key}`)) { setManual(fxId, key, null); touched.delete(`${fxId}|${key}`); }
    }
  }
  /** What a row shows: the look's value when ticked, the fixture's live value when not. */
  const shown = (key: string) => (include[key] && held[key] !== undefined ? held[key] : liveValue(key));
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

  // ----- take over the preview fixture -----
  // While browsing, routes and the playing scene leave the preview fixture alone, so it
  // (and every slider here) shows only what you set. Re-sent every second; the engine
  // lets go a few seconds after it stops hearing it (closing the tab, a crash).
  let takeOver = $state(true);
  $effect(() => {
    const f = takeOver ? fxId : null;
    if (!f) return;
    invoke("browse_fixture", { fixture: f });
    const t = setInterval(() => invoke("browse_fixture", { fixture: f }), 1000);
    return () => { clearInterval(t); invoke("browse_fixture", { fixture: null }); };
  });

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
    if (c.key !== chKey) { include[c.key] = true; held[c.key] ??= untrack(() => liveValue(c.key)); }
    openMotion = c.key;
  }
  const MOTION_KINDS: [string, string][] = [
    ["fixed", "Fixed"], ["sweep", "Sweep"], ["step", "Step"], ["random", "Random"],
    ["list", "List"], ["catalog", "Catalogue"], ["follow", "Follow audio"],
  ];
  /** The look being edited, as it would be saved. */
  function draftLook(): Look {
    const values: Record<string, number> = { [chKey]: value };
    for (const c of secondary) if (include[c.key]) values[c.key] = held[c.key] ?? untrack(() => liveValue(c.key));
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
  // The form below edits either a new look or the look selected in the list on the right.
  let lookName = $state("");
  let lookTags = $state<string[]>([]);
  let lookSets = $state<string[]>([]);
  let lookRating = $state(0);
  let newSet = $state("");
  let editingId = $state<string | null>(null);
  const editing = $derived(profile.looks.find((l) => l.id === editingId));
  const headLooks = $derived(profile.looks.filter((l) => !l.builtin && (l.head ?? "") === (head?.name ?? "")));
  /** Your looks built on value `v` of the channel being browsed. */
  const looksAt = (v: number) => headLooks.filter((l) => primaryKey(l) === chKey && l.values[chKey] === v);
  /** Catalogue rows, plus values your looks use that aren't catalogued, in value order. */
  const catRows = $derived.by(() => {
    const rows = new Map<number, ValueNote | undefined>(sorted.map((n) => [n.value, n]));
    for (const l of headLooks) if (primaryKey(l) === chKey && l.values[chKey] !== undefined && !rows.has(l.values[chKey])) rows.set(l.values[chKey], undefined);
    return [...rows.entries()].sort(([a], [b]) => a - b).map(([v, n]) => ({ value: v, note: n, looks: looksAt(v) }));
  });
  /** Catalogue values opened to show their looks (the value being browsed is open unless closed). */
  let openVal = $state<Record<number, boolean>>({});
  const isOpen = (v: number) => openVal[v] ?? true;
  /** Looks built on another channel (e.g. the gallery when browsing patterns), grouped under the value they use. */
  const lookGroups = $derived.by(() => {
    const m = new Map<string, { key: string; value: number; looks: Look[] }>();
    for (const l of headLooks) {
      const key = primaryKey(l) ?? "";
      if (key === chKey) continue; // these sit under their value above
      const v = l.values[key] ?? 0;
      const id = `${key}:${v}`;
      if (!m.has(id)) m.set(id, { key, value: v, looks: [] });
      m.get(id)!.looks.push(l);
    }
    return [...m.values()].sort((a, b) => a.key.localeCompare(b.key) || a.value - b.value);
  });
  let closedGroup = $state<Record<string, boolean>>({});
  /** "P1 Pattern 48 · Double line": the channel, the value and its catalogue (or range) name. */
  function valueName(key: string, v: number) {
    const c = profile.channels.find((x) => x.key === key);
    const label = c?.values.find((n) => n.value === v)?.label || (c ? capabilityAt(c, v)?.label : "") || "";
    return { channel: c?.name ?? key, label };
  }
  /** Browse a group's pattern value (switching channel if needed). */
  function goTo(key: string, v: number) {
    if (key && key !== chKey && profile.channels.some((c) => c.key === key)) chKey = key;
    go(v);
  }

  // Name, tags, sets and rating of the selected look update it straight away.
  $effect(() => {
    const snapshot = [lookName, [...lookTags], [...lookSets], lookRating] as const;
    untrack(() => {
      const l = editing;
      if (!l || !onOwnPrimary) return; // off its own pattern these are for the new look
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
    if (editing && look.name === editing.name) look.name = onOwnPrimary ? `${look.name} copy` : `${look.name} · ${note?.label ? `${note.label} ` : ""}${value}`;
    profile.looks.push(look);
    const n = Object.keys(look.values).length, m = Object.keys(look.motions).length;
    toast(`Saved look "${look.name}" (${n} channel${n === 1 ? "" : "s"}${m ? `, ${m} moving` : ""})`);
    selectLook(profile.looks.at(-1)!, false);
  }
  /** The selected look has channel changes that "Update look" would save. */
  const unsaved = $derived.by(() => {
    if (!editing) return false;
    JSON.stringify(include); JSON.stringify(held); JSON.stringify(motions); void value; void chKey;
    const d = untrack(draftLook);
    return stable(d.values) !== stable(editing.values) || stable(d.motions) !== stable(editing.motions ?? {});
  });
  /** JSON with keys sorted and unset fields dropped, so key order doesn't count as a change. */
  function stable(v: unknown): string {
    return JSON.stringify(v, (_, x) => (x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).filter(([, y]) => y !== undefined).sort(([a], [b]) => a.localeCompare(b))) : x));
  }
  /** Save the current channel values and motions into the selected look. */
  /** The selected look's main channel and value: a look belongs to the pattern it was made on. */
  const editPrimary = $derived(editing ? primaryKey(editing) : undefined);
  /** Browsing the selected look's own main value? Off it, Update is off and Save as new
   *  makes a new look, leaving the original as it was. */
  const onOwnPrimary = $derived(!editing || (editPrimary === chKey && editing.values[chKey] === value));
  function backToOwn() {
    if (editing && editPrimary) goTo(editPrimary, editing.values[editPrimary]);
  }
  function updateLook() {
    if (!editing || !onOwnPrimary) return;
    const d = draftLook();
    editing.values = d.values;
    editing.motions = d.motions;
    toast(`Updated "${editing.name}"`);
  }
  /** Deselect the look: clear the form and its secondary options (unticked and handed
   *  back), keeping the pattern value you're on. */
  function deselectLook() {
    newLook();
    for (const t of [...touched]) {
      const [f, c] = t.split("|");
      if (c === chKey) continue;
      setManual(f, c, null);
      touched.delete(t);
    }
    include = {};
    held = {};
    motions = {};
    openMotion = null;
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
    // Browse the look's own main channel, so its main option shows and "Update look" saves the right one.
    const primary = primaryKey(l);
    if (primary && primary !== chKey) chKey = primary;
    // Channels the previous look set that this one doesn't go back to normal.
    for (const t of [...touched]) {
      const [f, c] = t.split("|");
      if (f === fxId && !(c in l.values)) { setManual(f, c, null); touched.delete(t); }
    }
    motions = structuredClone($state.snapshot(l.motions ?? {}));
    include = Object.fromEntries(Object.keys(l.values).filter((k) => k !== primary).map((k) => [k, true]));
    held = Object.fromEntries(Object.entries(l.values).filter(([k]) => k !== primary));
    for (const [k, v] of Object.entries(l.values)) {
      if (!motions[k]) {
        touched.add(`${fxId}|${k}`);
        setManual(fxId, k, v);
      }
      if (k === primary) value = v;
    }
  }
  /** A look's main channel: its pattern channel, else the first one you can browse, else its first. */
  function primaryKey(l: Look): string | undefined {
    return lookMainKey(profile, l);
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
      <div class="row wrap">
        <label>Channel</label>
        <select bind:value={chKey}>{#each browsable as c}<option value={c.key}>{c.name}</option>{/each}</select>
        <label>Preview on</label>
        <select bind:value={fxId}>
          {#each previewable as f}<option value={f.id}>{f.name}</option>{:else}<option value={null}>No patched fixture</option>{/each}
        </select>
        <label class="inline" title="Routes and the playing scene don't touch this fixture while you browse, so the sliders show only what you set here. Untick to see it with the show running on top.">
          <input type="checkbox" bind:checked={takeOver} /> pause routes on it
        </label>
        {#if takeOver && fx}<span class="tag accent" title="Routes and the scene take over again when you leave the pattern browser">routes paused on {fx.name}</span>{/if}
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
          {@const v = shown(c.key)}
          {@const mo = motions[c.key]}
          <div class="secrow" class:inc={include[c.key]}>
            <input type="checkbox" checked={!!include[c.key]} onchange={(e) => setIncluded(c.key, e.currentTarget.checked)}
              title={include[c.key] ? "In the look: saved at this value" : "Not in the look: shows the live value; tick (or move it) to add it"} />
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
          {#if onOwnPrimary}
            <span class="small faint grow">Name, tags, sets and rating save as you change them. Use Update look to save channel changes.</span>
          {:else}
            <span class="small warn grow">
              This look is on {chName(editPrimary ?? "")} {editPrimary ? editing.values[editPrimary] : ""}; you're on {ch?.name} {value}.
              <b>Save as new</b> makes a look here with these options, and leaves “{editing.name}” as it was.
              <button class="ghost small" onclick={backToOwn}>Back to {editPrimary ? editing.values[editPrimary] : ""}</button>
            </span>
          {/if}
          <button class="ghost" onclick={newLook} title="Start a new look from the options set now">New look</button>
          <button class="ghost" onclick={deselectLook} title="Deselect the look and reset the secondary options">Deselect</button>
        {:else}
          <span class="tag">New look</span>
          <span class="small faint grow">Pick a look in the list on the right to edit it.</span>
        {/if}
      </div>
      <div class="row wrap">
        <input bind:value={lookName} placeholder="Look name, e.g. Beam 12 + zoom chase" class="grow" />
        <span class="muted small">Tags</span><Tags bind:tags={lookTags} suggestions={[...QUICK, ...SECTION_TAGS, ...yourTags]} />
      </div>
      <div class="row small sectags">
        <span class="muted" title="Look routes that follow the section use looks tagged for the part of the track playing">Sections</span>
        {#each SECTION_TAGS as t}
          <button class="chip" class:on={lookTags.includes(t)} style="--c:{sectionInfo(t)?.color}" onclick={() => { const i = lookTags.indexOf(t); if (i >= 0) lookTags.splice(i, 1); else lookTags.push(t); }}>{t}</button>
        {/each}
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
          {#if onOwnPrimary}
            <button class="primary" onclick={updateLook} disabled={!unsaved} title={unsaved ? "Save the channel changes into this look" : "No channel changes to save"}>{unsaved ? "Update look •" : "Saved"}</button>
            <button onclick={saveAsNew}>Save as new</button>
          {:else}
            <button disabled title="Update only saves into a look on its own main value. Go back to it, or save a new look here.">Update look</button>
            <button class="primary" onclick={saveAsNew}>Save as new</button>
          {/if}
        {:else}
          <button class="primary" onclick={saveAsNew}>Save look</button>
        {/if}
      </div>
    </div>
  </div>

  {#snippet lookEntry(l: Look, main: string)}
    <button class="entry sub" class:cur={l.id === editingId} aria-pressed={l.id === editingId}
      onclick={() => (l.id === editingId ? deselectLook() : selectLook(l))}
      title={l.id === editingId ? "Click again to deselect it and reset the secondary options" : "Edit and preview this look"}>
      <span class="grow lbl">{l.name}</span>
      <span class="stars">{"★".repeat(l.rating)}</span>
      {#each l.tags as t}<span class="tag">{t}</span>{/each}
      <span class="faint small">+ {Object.keys(l.values).filter((k) => k !== main).map(chName).join(", ") || "nothing else"}</span>
    </button>
  {/snippet}
  <div class="col side">
    <div class="panel col">
      <div class="row">
        <h2 class="grow">{ch?.name ?? "Values"} &amp; looks</h2>
        <span class="faint small">{sorted.length} catalogued · {headLooks.length} look{headLooks.length === 1 ? "" : "s"}</span>
      </div>
      <span class="small faint">Values you've tagged, each with the looks built on it. Click a value to browse it, a look to edit and preview it.</span>
      <div class="scroll cat">
        {#each catRows as row (row.value)}
          {@const n = row.note}
          <div class="catrow" class:cur={row.value === value}>
            {#if row.looks.length}
              <button class="ghost caret" onclick={() => (openVal[row.value] = !isOpen(row.value))} aria-expanded={isOpen(row.value)}
                title="{isOpen(row.value) ? 'Hide' : 'Show'} the looks built on this value">{isOpen(row.value) ? "▾" : "▸"}</button>
            {:else}<span class="caret-gap"></span>{/if}
            <button class="entry grow" onclick={() => go(row.value)}>
              <span class="mono">{row.value}</span>
              <span class="grow lbl">{n?.label || capabilityAt(ch!, row.value)?.label || ""}</span>
              {#if n}<span class="stars">{"★".repeat(n.rating)}</span>{#each n.tags as t}<span class="tag" class:blocked={t === "blocked"} class:accent={t === "safe"}>{t}</span>{/each}{/if}
              {#if row.looks.length}<span class="nlooks">{row.looks.length} look{row.looks.length === 1 ? "" : "s"}</span>{/if}
            </button>
          </div>
          {#if row.looks.length && isOpen(row.value)}
            <div class="sublooks">{#each row.looks as l (l.id)}{@render lookEntry(l, chKey)}{/each}</div>
          {/if}
        {:else}
          <span class="faint small">Nothing tagged yet. Step through values and tag the ones you like, or save a look.</span>
        {/each}
        {#if lookGroups.length}
          <span class="other small faint">Looks built on other channels</span>
          {#each lookGroups as g (`${g.key}:${g.value}`)}
            {@const id = `${g.key}:${g.value}`}
            {@const vn = valueName(g.key, g.value)}
            <div class="catrow">
              <button class="ghost caret" onclick={() => (closedGroup[id] = !closedGroup[id])} aria-expanded={!closedGroup[id]}>{closedGroup[id] ? "▸" : "▾"}</button>
              <button class="entry grow" onclick={() => goTo(g.key, g.value)} title="Browse {vn.channel} and this value">
                <span class="small faint">{vn.channel}</span><span class="mono">{g.value}</span>
                <span class="grow lbl">{vn.label}</span>
                <span class="nlooks">{g.looks.length} look{g.looks.length === 1 ? "" : "s"}</span>
              </button>
            </div>
            {#if !closedGroup[id]}
              <div class="sublooks">{#each g.looks as l (l.id)}{@render lookEntry(l, g.key)}{/each}</div>
            {/if}
          {/each}
        {/if}
      </div>
      <div class="row">
        <select class="grow" bind:value={copyTo}>
          <option value="">Copy catalog to…</option>
          {#each profile.channels.filter((c) => c.key !== chKey && c.group === ch?.group) as c}<option value={c.key}>{c.name}</option>{/each}
        </select>
        <button disabled={!copyTo} onclick={copyCatalog}>Copy</button>
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
  .wrap select { max-width: 100%; min-width: 0; }
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
  .cat { max-height: 640px; display: flex; flex-direction: column; gap: 2px; }
  .entry { display: flex; align-items: center; gap: 6px; text-align: left; background: transparent; border-color: transparent; flex-wrap: wrap; }
  .entry.cur { background: var(--accent-bg); border-color: #7c5cff55; }
  .lbl { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--muted); }
  .stars { color: var(--warn); font-size: 11px; }
  .warn { color: var(--warn); }
  .inline { display: inline-flex; align-items: center; gap: 4px; margin: 0; font-size: 12px; color: var(--muted); }
  .catrow { display: flex; align-items: center; gap: 2px; border-radius: 6px; }
  .catrow.cur { background: var(--accent-bg); }
  .catrow .entry { min-width: 0; }
  .nlooks { font-size: 11px; white-space: nowrap; color: var(--accent-2); }
  .caret-gap { width: 25px; flex-shrink: 0; }
  .other { margin: 8px 0 2px; padding-top: 6px; border-top: 1px solid var(--line); }
  .caret { padding: 0 6px; }
  .sublooks { display: flex; flex-direction: column; gap: 1px; margin: 0 0 4px 22px; padding-left: 8px; border-left: 2px solid var(--line-2); }
  .entry.sub { font-size: 12px; padding: 3px 8px; }
  .entry.sub { row-gap: 0; }
  .entry.sub .lbl { color: var(--text); flex: 1 1 0; min-width: 0; }
  .entry.sub .faint.small { flex-basis: 100%; padding: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .caret { font-size: 13px; }
</style>
