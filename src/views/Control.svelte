<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import TargetPicker from "../components/TargetPicker.svelte";
  import ChannelSelect from "../components/ChannelSelect.svelte";
  import SceneEditor from "../components/SceneEditor.svelte";
  import ControllerSetup from "../components/ControllerSetup.svelte";
  import Autopilot from "../components/Autopilot.svelte";
  import { newScene, sceneSummary } from "../lib/scenes";
  import { app, uid, toast, modeChannels } from "../lib/store.svelte";
  import { FEEDBACK_STYLES, type ControlAction, type ControlMapping, type MidiStatus, type Preset } from "../lib/types";

  const show = app.show!;
  let midi = $state<MidiStatus | null>(null);
  onMount(() => {
    const poll = async () => { try { midi = await invoke<MidiStatus>("midi_status"); } catch {} };
    poll();
    const t = setInterval(poll, 1500);
    return () => { clearInterval(t); app.learning = null; };
  });

  // ---------- mappings ----------
  const ACTIONS: { type: ControlAction["type"]; label: string; fader: boolean; make: () => ControlAction }[] = [
    { type: "routes", label: "Routes on/off", fader: false, make: () => ({ type: "routes", routes: [] }) },
    { type: "route_group", label: "Route group on/off", fader: false, make: () => ({ type: "route_group", group: routeGroups[0] ?? "" }) },
    { type: "preset", label: "Recall preset", fader: false, make: () => ({ type: "preset", preset: show.presets[0]?.id ?? "" }) },
    { type: "blackout", label: "Blackout", fader: false, make: () => ({ type: "blackout" }) },
    { type: "master", label: "Master brightness", fader: true, make: () => ({ type: "master" }) },
    { type: "channel_value", label: "Set a channel", fader: true, make: () => ({ type: "channel_value", target: { rooms: [], groups: [], fixtures: [] }, channel: "@intensity", min: 0, max: 255 }) },
    { type: "bands", label: "Mute/unmute detection bands", fader: false, make: () => ({ type: "bands", bands: [] }) },
    { type: "band_sensitivity", label: "Band sensitivity", fader: true, make: () => ({ type: "band_sensitivity", band: show.audio.bands[0]?.id ?? "" }) },
    { type: "release_manual", label: "Release all manual control", fader: false, make: () => ({ type: "release_manual" }) },
    { type: "autopilot", label: "Autopilot on/off", fader: false, make: () => ({ type: "autopilot" }) },
  ];
  /** The colour a pad lights in when the control has none of its own (matches control.rs). */
  function autoColor(m: ControlMapping): string {
    const a = m.action;
    switch (a.type) {
      case "preset": return show.presets.find((p) => p.id === a.preset)?.color ?? "#7c5cff";
      case "routes": return a.routes.length && a.routes.every((id) => show.routes.find((r) => r.id === id)?.action.type === "strobe") ? "#ffffff" : "#4fb3ff";
      case "route_group": return "#45d39a";
      case "bands": return show.audio.bands.find((b) => a.bands.includes(b.id))?.color ?? "#f2b54a";
      case "blackout": return "#ff0000";
      case "master": return "#ffffff";
      case "channel_value": return a.channel === "@fog" ? "#4fd8ff" : "#f2b54a";
      case "autopilot": return "#7c5cff";
      default: return "#f2b54a";
    }
  }
  const isFader = (m: ControlMapping) => ACTIONS.find((a) => a.type === m.action.type)?.fader ?? false;
  const routeGroups = $derived([...new Set(show.routes.map((r) => r.group.trim()).filter(Boolean))].sort());
  const routeName = (id: string) => {
    const r = show.routes.find((r) => r.id === id);
    return r ? r.name || `${r.source} → ${r.channel || "?"}` : id;
  };

  function addMapping(source: "midi" | "key") {
    const m: ControlMapping = {
      id: uid("ctl"), name: "", enabled: true,
      trigger: { source, kind: "note", channel: 0, number: 36, key: "", mode: "toggle" },
      action: ACTIONS[0].make(),
    };
    show.controls.push(m);
    learn(m.id);
  }
  function setAction(m: ControlMapping, type: string) {
    m.action = ACTIONS.find((a) => a.type === type)!.make();
    if (isFader(m) && m.trigger.kind === "cc") m.trigger.mode = "fader";
  }
  function toggleIn(list: string[], id: string) {
    const i = list.indexOf(id);
    if (i >= 0) list.splice(i, 1); else list.push(id);
  }
  const triggerLabel = (m: ControlMapping) => {
    const t = m.trigger;
    if (t.source === "key") return t.key ? `Key ${t.key === " " ? "Space" : t.key.toUpperCase()}` : "No key";
    const ch = t.channel ? ` · ch ${t.channel}` : "";
    return t.kind === "cc" ? `CC ${t.number}${ch}` : t.kind === "program" ? `Program ${t.number}${ch}` : `Note ${t.number}${ch}`;
  };

  // ---------- learn ----------
  let learnStarted = 0;
  function learn(id: string) {
    app.learning = app.learning === id ? null : id;
    learnStarted = Date.now();
  }
  $effect(() => {
    const ev = app.midiLog[0];
    const m = show.controls.find((c) => c.id === app.learning);
    if (!ev || !m || m.trigger.source !== "midi" || ev.at < learnStarted) return;
    if (ev.kind === "note" && ev.value === 0) return; // wait for a note-on
    m.trigger.kind = ev.kind;
    m.trigger.number = ev.number;
    m.trigger.channel = ev.channel;
    m.trigger.mode = ev.kind === "cc" && isFader(m) ? "fader" : m.trigger.mode === "fader" ? "toggle" : m.trigger.mode;
    app.learning = null;
    toast(`Learned ${triggerLabel(m)}`);
  });
  function onKeyLearn(e: KeyboardEvent) {
    const m = show.controls.find((c) => c.id === app.learning);
    if (!m || m.trigger.source !== "key") return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") { app.learning = null; return; }
    m.trigger.key = e.key.toLowerCase();
    app.learning = null;
  }

  let tick = $state(0);
  // reads `tick` so the highlight clears on its own
  const fireRecent = (id: string) => tick >= 0 && app.fired.id === id && Date.now() - app.fired.at < 400;
  $effect(() => { const t = setInterval(() => tick++, 200); return () => clearInterval(t); });

  // ---------- presets ----------
  let presetName = $state("");
  let includeRoutes = $state(false);
  function snapshotValues() {
    return Object.entries(app.manual).flatMap(([fixture, chans]) =>
      Object.entries(chans).map(([channel, value]) => ({ fixture, channel, value })));
  }
  function savePreset() {
    const values = snapshotValues();
    if (!values.length && !includeRoutes) { toast("Nothing to save: move some channel sliders in Patch first, or include route states"); return; }
    const p: Preset = {
      id: uid("pre"), name: presetName.trim() || `Preset ${show.presets.length + 1}`,
      color: ["#7c5cff", "#45d39a", "#f2b54a", "#ff5c6c", "#4fb3ff", "#ff7ad9"][show.presets.length % 6],
      values, routes: includeRoutes ? Object.fromEntries(show.routes.map((r) => [r.id, r.enabled])) : {}, exclusive: false, sections: [],
    };
    show.presets.push(p);
    presetName = "";
    toast(`Saved "${p.name}" (${values.length} channel${values.length === 1 ? "" : "s"})`);
  }
  function updatePreset(p: Preset) {
    p.values = snapshotValues();
    if (Object.keys(p.routes).length) p.routes = Object.fromEntries(show.routes.map((r) => [r.id, r.enabled]));
    toast(`Updated "${p.name}"`);
  }
  function describePreset(p: Preset) {
    const fx = new Set(p.values.map((v) => v.fixture));
    const names = show.fixtures.filter((f) => fx.has(f.id)).map((f) => f.name);
    const { on, off } = sceneSummary(p, show.routes);
    const parts = [
      p.values.length && `${p.values.length} value${p.values.length === 1 ? "" : "s"}${names.length ? ` on ${names.join(", ")}` : ""}`,
      (on || off) && `routes: ${on} on, ${off} off`,
    ].filter(Boolean);
    return parts.join(" · ") || "does nothing yet";
  }
  let editingScene = $state<string | null>(null);
  function addScene() {
    const p = newScene(show, uid("pre"), presetName);
    show.presets.push(p);
    presetName = "";
    editingScene = p.id;
  }
  function assignScene(p: Preset, source: "midi" | "key") {
    const m: ControlMapping = {
      id: uid("ctl"), name: p.name, enabled: true,
      trigger: { source, kind: "note", channel: 0, number: 36, key: "", mode: "on" },
      action: { type: "preset", preset: p.id },
    };
    show.controls.push(m);
    learn(m.id);
    toast(source === "key" ? "Press the key for this scene…" : "Press the pad for this scene…");
  }
  function removePreset(p: Preset) {
    show.presets.splice(show.presets.indexOf(p), 1);
    for (const c of show.controls) if (c.action.type === "preset" && c.action.preset === p.id) c.action.preset = "";
  }
  void modeChannels;
</script>

<svelte:window onkeydowncapture={onKeyLearn} />

<div class="ctl">
  <div class="col side">
    <div class="panel col">
      <div class="row"><h2 class="grow">MIDI</h2><label class="row"><input type="checkbox" bind:checked={show.midi.enabled} /> Enabled</label></div>
      {#if show.midi.enabled}
        {#if midi?.available.length}
          <span class="small muted">Listen to {show.midi.inputs.length ? "these inputs" : "all inputs"}:</span>
          {#each midi.available as name}
            <label class="row small">
              <input type="checkbox" checked={!show.midi.inputs.length || show.midi.inputs.includes(name)}
                onchange={() => {
                  const all = midi!.available;
                  const cur = show.midi.inputs.length ? [...show.midi.inputs] : [...all];
                  const i = cur.indexOf(name);
                  if (i >= 0) cur.splice(i, 1); else cur.push(name);
                  show.midi.inputs = cur.length === all.length ? [] : cur;
                }} />
              <span class="grow">{name}</span>
              {#if midi.connected.includes(name)}<span class="tag accent">connected</span>{/if}
            </label>
          {/each}
        {:else}
          <span class="small muted">No MIDI controllers found. Plug one in; it's picked up automatically.</span>
        {/if}
        {#if midi?.error}<div class="small err">{midi.error}</div>{/if}
        <label class="row small" title="Pads show which scenes, routes and switches are on, in their colours">
          <input type="checkbox" bind:checked={show.midi.feedback} /> Light up pads
        </label>
        {#if show.midi.feedback}
          <select bind:value={show.midi.feedback_style} title="How to light the pads">
            {#each FEEDBACK_STYLES as [id, name]}<option value={id}>{name}</option>{/each}
          </select>
          {#each midi?.feedback ?? [] as [port, style]}
            <span class="small faint">{port}: {FEEDBACK_STYLES.find(([id]) => id === style)?.[1] ?? style}</span>
          {/each}
        {/if}
      {/if}
    </div>
    <ControllerSetup />
    <div class="panel col">
      <h3>Incoming</h3>
      {#each app.midiLog.slice(0, 10) as e (e.at + e.number + e.kind + e.value)}
        <div class="small mono row"><span class="grow">{e.kind === "cc" ? "CC" : e.kind === "note" ? "Note" : "Prog"} {e.number} · ch {e.channel}</span><span class="faint">{e.value}</span></div>
      {:else}
        <span class="small faint">Press a pad or move a knob to see it here.</span>
      {/each}
    </div>
  </div>

  <div class="col grow">
    <div class="panel col">
      <div class="row">
        <h2 class="grow">Scenes and presets</h2>
        <input bind:value={presetName} placeholder="Name" />
        <button class="primary" onclick={addScene}>+ New scene</button>
        <button onclick={savePreset} title="Save the channels you've set by hand (and optionally which routes are on)">Save current look</button>
        <label class="row small"><input type="checkbox" bind:checked={includeRoutes} /> with route states</label>
      </div>
      <span class="small muted">
        A scene switches route groups on and off: open <b>Edit scene</b> to choose. A preset can also hold channels you've set by hand
        (Patch sliders, pattern browser, channel faders). Recall either from here, the Live screen, or a pad or key.
      </span>
      {#each show.presets as p (p.id)}
        <div class="preset row" class:active={app.activePresets.includes(p.id)} style="--c:{p.color}">
          <input type="color" bind:value={p.color} />
          <input class="pname" bind:value={p.name} />
          <span class="grow small faint">{describePreset(p)}</span>
          <label class="row small" title="Clear other manual values first"><input type="checkbox" bind:checked={p.exclusive} /> replace others</label>
          <button class="primary" onclick={() => invoke("preset_apply", { preset: p.id })}>Recall</button>
          <button class:on={editingScene === p.id} onclick={() => (editingScene = editingScene === p.id ? null : p.id)}>Edit scene</button>
          <button class="ghost" onclick={() => updatePreset(p)} title="Replace its hand-set channels (and route states, if it has them) with what's set now">Update from current</button>
          <ConfirmButton onconfirm={() => removePreset(p)}>Delete</ConfirmButton>
        </div>
        {#if editingScene === p.id}<SceneEditor preset={p} onassign={(src) => assignScene(p, src)} />{/if}
      {:else}
        <span class="small faint">No scenes or presets yet. Start with <b>+ New scene</b>, or build a whole setup on the Design screen.</span>
      {/each}
    </div>

    <Autopilot />

    <div class="panel col">
      <div class="row">
        <h2 class="grow">Controls</h2>
        <button onclick={() => addMapping("midi")}>+ MIDI control</button>
        <button onclick={() => addMapping("key")}>+ Keyboard key</button>
      </div>
      <span class="small muted">
        Map a pad, key, knob or fader to an action. Use Learn and then touch the control. Keys work whenever LightDeck is focused
        and you're not typing in a field.
      </span>
      {#each show.controls as m, i (m.id)}
        {@const learning = app.learning === m.id}
        <div class="mapping" class:off={!m.enabled} class:fired={fireRecent(m.id)}>
          <div class="row flow">
            <input type="checkbox" bind:checked={m.enabled} title="Enabled" />
            <input class="mname" bind:value={m.name} placeholder="Name (optional)" />
            <select bind:value={m.trigger.source}><option value="midi">MIDI</option><option value="key">Key</option></select>
            {#if m.trigger.source === "midi"}
              <select bind:value={m.trigger.kind}><option value="note">Note</option><option value="cc">CC</option><option value="program">Program</option></select>
              <input type="number" min="0" max="127" bind:value={m.trigger.number} title="Note / CC / program number" />
              <select bind:value={m.trigger.channel} title="MIDI channel">
                <option value={0}>any ch</option>
                {#each Array.from({ length: 16 }, (_, k) => k + 1) as c}<option value={c}>ch {c}</option>{/each}
              </select>
            {:else}
              <span class="keycap">{m.trigger.key ? (m.trigger.key === " " ? "Space" : m.trigger.key.toUpperCase()) : "—"}</span>
            {/if}
            <button class:learn={learning} onclick={() => learn(m.id)}>
              {learning ? (m.trigger.source === "key" ? "Press a key… (Esc cancels)" : "Touch a control…") : "Learn"}
            </button>
            <span class="lbl">→</span>
            <select value={m.action.type} onchange={(e) => setAction(m, (e.target as HTMLSelectElement).value)}>
              {#each ACTIONS as a}<option value={a.type}>{a.label}</option>{/each}
            </select>
            {#if !(isFader(m) && m.trigger.mode === "fader") && m.action.type !== "release_manual"}
              <select bind:value={m.trigger.mode} title="How the button behaves">
                <option value="toggle">Toggle</option>
                <option value="momentary">While held</option>
                <option value="on">Turn on</option>
                <option value="off">Turn off</option>
                {#if m.trigger.kind === "cc" && m.trigger.source === "midi"}<option value="fader">Fader</option>{/if}
              </select>
            {:else if m.trigger.kind === "cc"}
              <select bind:value={m.trigger.mode}>
                <option value="fader">Fader</option><option value="toggle">Toggle</option><option value="momentary">While held</option>
              </select>
            {/if}
            <span class="grow"></span>
            {#if m.trigger.source === "midi" && show.midi.feedback && m.trigger.mode !== "fader"}
              <input type="color" value={m.color || autoColor(m)} oninput={(e) => (m.color = (e.target as HTMLInputElement).value)}
                title="Pad light colour" />
              {#if m.color}<button class="ghost small" onclick={() => (m.color = "")} title="Use the automatic colour">auto</button>{/if}
            {/if}
            <ConfirmButton onconfirm={() => show.controls.splice(i, 1)}>Delete</ConfirmButton>
          </div>

          <div class="params">
            {#if m.action.type === "routes"}
              {@const act = m.action}
              <div class="chips">
                {#each show.routes as r (r.id)}
                  <button class="chip" class:on={act.routes.includes(r.id)} onclick={() => toggleIn(act.routes, r.id)}>{routeName(r.id)}</button>
                {:else}<span class="small faint">No routes yet.</span>{/each}
              </div>
            {:else if m.action.type === "route_group"}
              <select bind:value={m.action.group}>
                <option value="">Choose a route group…</option>
                {#each routeGroups as g}<option value={g}>{g}</option>{/each}
              </select>
              {#if !routeGroups.length}<span class="small faint">Give routes a group on the Routes screen first.</span>{/if}
            {:else if m.action.type === "bands"}
              {@const act = m.action}
              <div class="chips">
                {#each show.audio.bands as b (b.id)}
                  <button class="chip" class:on={act.bands.includes(b.id)} style="--c:{b.color}" onclick={() => toggleIn(act.bands, b.id)}>{b.name}</button>
                {/each}
              </div>
            {:else if m.action.type === "band_sensitivity"}
              <select bind:value={m.action.band}>{#each show.audio.bands as b}<option value={b.id}>{b.name}</option>{/each}</select>
            {:else if m.action.type === "preset"}
              <select bind:value={m.action.preset}>
                <option value="">Choose a preset…</option>
                {#each show.presets as p}<option value={p.id}>{p.name}</option>{/each}
              </select>
            {:else if m.action.type === "channel_value"}
              {@const act = m.action}
              <TargetPicker target={act.target} />
              <div class="row flow">
                <span class="lbl">channel</span>
                <ChannelSelect target={act.target} bind:channel={act.channel} />
                <label>{m.trigger.mode === "fader" ? "From" : "Off"}</label><input type="number" min="0" max="255" bind:value={act.min} />
                <label>{m.trigger.mode === "fader" ? "To" : "On"}</label><input type="number" min="0" max="255" bind:value={act.max} />
                <span class="small faint">{m.trigger.mode === "fader" ? "The fader moves between these." : "Button sets On; switching off gives the channel back to routes."}</span>
              </div>
            {/if}
          </div>

          <div class="row test small">
            <span class="faint">{triggerLabel(m)}</span>
            <span class="grow"></span>
            {#if isFader(m)}
              <span class="faint">Test</span>
              <input type="range" min="0" max="1" step="0.01" oninput={(e) => invoke("control_fire", { mapping: m.id, pressed: true, value: +(e.target as HTMLInputElement).value })} />
            {:else}
              <button class="ghost"
                onpointerdown={() => invoke("control_fire", { mapping: m.id, pressed: true, value: null })}
                onpointerup={() => invoke("control_fire", { mapping: m.id, pressed: false, value: null })}>Test</button>
            {/if}
          </div>
        </div>
      {:else}
        <span class="small faint">No controls yet.</span>
      {/each}
    </div>
  </div>
</div>

<style>
  .ctl { display: flex; gap: 16px; align-items: flex-start; }
  .side { width: 300px; flex-shrink: 0; }
  button.on { background: var(--accent-bg); border-color: var(--accent); }
  .preset { padding: 6px 8px; border-radius: 8px; border: 1px solid var(--line); border-left: 4px solid var(--c); }
  .preset .grow { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 1200px) { .preset { flex-wrap: wrap; } .preset .grow { min-width: 140px; } }
  .preset button { white-space: nowrap; }
  .preset.active { background: color-mix(in srgb, var(--c) 14%, transparent); }
  .pname { width: 180px; }
  .mapping { border: 1px solid var(--line); border-radius: 8px; padding: 8px; display: flex; flex-direction: column; gap: 6px; transition: border-color 120ms; }
  .mapping.off { opacity: 0.55; }
  .mapping.fired { border-color: var(--accent); }
  .flow { flex-wrap: wrap; }
  .mname { width: 150px; }
  .lbl { color: var(--faint); }
  .keycap { min-width: 40px; text-align: center; padding: 3px 8px; border: 1px solid var(--line-2); border-radius: 6px; font-family: var(--mono, ui-monospace); }
  button.learn { background: var(--warn-bg); border-color: var(--warn); color: var(--warn); }
  .params { padding-left: 24px; display: flex; flex-direction: column; gap: 6px; }
  .params input[type="number"] { width: 64px; }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .chip { font-size: 12px; padding: 3px 8px; }
  .chip.on { background: var(--accent-bg); border-color: var(--accent); }
  .test { padding-left: 24px; }
  input[type="color"] { width: 30px; padding: 2px; }
  input[type="number"] { width: 64px; }
</style>
