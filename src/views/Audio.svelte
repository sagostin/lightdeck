<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import SpectrumEditor from "../components/SpectrumEditor.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import { app, toast } from "../lib/store.svelte";
  import { BAND_PRESETS, bandSpan, type BandDef, type BandPreset } from "../lib/types";

  const show = app.show!;
  let devices = $state<string[]>([]);
  let userPresets = $state<BandDef[]>([]);
  async function refresh() { devices = await invoke<string[]>("list_audio_devices"); }
  onMount(async () => {
    refresh();
    userPresets = await invoke<BandDef[]>("get_band_presets");
  });
  const f = $derived(app.frame?.features);
  let selected = $state<string | null>(show.audio.bands[0]?.id ?? null);

  let preset = $state("");
  function addBand(key: string) {
    preset = "";
    if (!key) return;
    let p: BandPreset | undefined;
    if (key.startsWith("user:")) p = userPresets.find((u) => u.name === key.slice(5));
    else p = BAND_PRESETS.find((p) => p.name === key);
    p ??= { name: `Band ${show.audio.bands.length + 1}`, enabled: true, ranges: [{ lo_hz: 400, hi_hz: 1200, weight: 1 }], detect: "transient",
      level_threshold: 0.6, gain_db: 0, sensitivity: 0.5, min_gap_ms: 120, release: 0.85, color: "#ff7ad9" };
    const base = p.name.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "") || "band";
    let id = base, n = 2;
    while (show.audio.bands.some((b) => b.id === id)) id = `${base}_${n++}`;
    const copy = structuredClone($state.snapshot(p)) as BandPreset;
    show.audio.bands.push({ ...copy, id } as BandDef);
    selected = id;
  }
  function moveBand(b: BandDef, d: number) {
    const i = show.audio.bands.indexOf(b), j = i + d;
    if (j < 0 || j >= show.audio.bands.length) return;
    [show.audio.bands[i], show.audio.bands[j]] = [show.audio.bands[j], show.audio.bands[i]];
  }
  const usedBy = (id: string) => show.routes.filter((r) => r.source === id || r.source === `${id}.level`).length;
  function removeBand(b: BandDef) {
    show.audio.bands.splice(show.audio.bands.indexOf(b), 1);
    selected = show.audio.bands[0]?.id ?? null;
  }
  function addRange(b: BandDef) {
    const last = b.ranges.at(-1);
    const lo = last ? Math.min(16000, last.hi_hz * 1.5) : 400;
    b.ranges.push({ lo_hz: Math.round(lo), hi_hz: Math.round(Math.min(20000, lo * 2.5)), weight: 1 });
  }
  async function savePreset(b: BandDef) {
    const { id: _id, ...rest } = structuredClone($state.snapshot(b));
    userPresets = [...userPresets.filter((p) => p.name !== b.name), { ...rest, id: "" } as BandDef];
    await invoke("save_band_presets", { presets: userPresets });
    toast(`Saved "${b.name}" as a preset`);
  }
  async function deletePreset(name: string) {
    userPresets = userPresets.filter((p) => p.name !== name);
    await invoke("save_band_presets", { presets: userPresets });
  }
  const out = (id: string) => f?.bands.find((o) => o.id === id);
</script>

<div class="col">
  <div class="top">
    <div class="panel col settings">
      <h2>Input</h2>
      <div class="row">
        <select class="grow" bind:value={show.audio.device}>
          <option value={null}>System default</option>
          {#each devices as d}<option value={d}>{d}</option>{/each}
        </select>
        <button onclick={refresh}>Refresh</button>
        <button onclick={() => invoke("restart_audio")}>Restart</button>
      </div>
      <label>Input gain <span class="mono">{show.audio.gain_db > 0 ? "+" : ""}{show.audio.gain_db.toFixed(1)} dB</span></label>
      <input type="range" min="-24" max="36" step="0.5" bind:value={show.audio.gain_db} />
      <div class="meter"><i style="width:{(f?.level ?? 0) * 100}%"></i></div>
      <label>Global sensitivity <span class="mono">{Math.round(show.audio.sensitivity * 100)}%</span></label>
      <input type="range" min="0" max="1" step="0.01" bind:value={show.audio.sensitivity} />
      <div class="small">
        {#if f?.running}<span class="tag accent">Listening</span> {f.device} · {f.sample_rate} Hz · {f.bpm.toFixed(1)} BPM
        {:else}<span class="tag warn">Not running</span> {f?.error ?? ""}{/if}
      </div>
      <p class="small muted">
        To react to music playing on this Mac rather than a mic, install a loopback driver such as BlackHole, send your audio to it and pick it here.
      </p>
      {#if userPresets.length}
        <h3>Your band presets</h3>
        {#each userPresets as p}
          <div class="row small">
            <span class="swatch" style="background:{p.color}"></span><span class="grow">{p.name}</span>
            <span class="faint">{bandSpan(p)}</span>
            <button class="ghost" onclick={() => deletePreset(p.name)} aria-label="Delete preset {p.name}">×</button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="panel col grow">
      <div class="row">
        <h2 class="grow">Detection bands</h2>
        <span class="muted small">Drag a range to move it, drag its edges to resize.</span>
        <select bind:value={preset} onchange={() => addBand(preset)}>
          <option value="">Add band…</option>
          <optgroup label="Presets">
            {#each BAND_PRESETS as p}<option value={p.name}>{p.name} ({bandSpan({ ...p, id: "" })} Hz)</option>{/each}
          </optgroup>
          {#if userPresets.length}
            <optgroup label="Your presets">
              {#each userPresets as p}<option value={"user:" + p.name}>{p.name} ({bandSpan(p)} Hz)</option>{/each}
            </optgroup>
          {/if}
          <option value="custom">Custom (blank)</option>
        </select>
      </div>
      <SpectrumEditor bind:selected />
    </div>
  </div>

  <div class="bands">
    {#each show.audio.bands as b (b.id)}
      {@const o = out(b.id)}
      <div class="panel col band" class:sel={b.id === selected} style="--c:{b.color}" onpointerdown={() => (selected = b.id)} role="group">
        <div class="row">
          <input type="color" bind:value={b.color} />
          <input class="grow" bind:value={b.name} />
          <button class="ghost mute" class:muted={!b.enabled} onclick={() => (b.enabled = !b.enabled)}
            title={b.enabled ? "Mute this band (no level, no hits)" : "Unmute"}>{b.enabled ? "On" : "Muted"}</button>
          <button class="ghost arrow" onclick={() => moveBand(b, -1)} aria-label="Move earlier">‹</button>
          <button class="ghost arrow" onclick={() => moveBand(b, 1)} aria-label="Move later">›</button>
          <span class="hitlamp" class:on={!!o && o.flux > o.threshold}></span>
        </div>

        <div class="ranges">
          {#each b.ranges as r, ri}
            <div class="row small">
              <input type="number" min="20" max="20000" bind:value={r.lo_hz} /> –
              <input type="number" min="20" max="20000" bind:value={r.hi_hz} /> Hz
              <span class="faint">×</span><input class="w" type="number" min="0" max="2" step="0.1" bind:value={r.weight} title="Weight" />
              {#if b.ranges.length > 1}<button class="ghost" onclick={() => b.ranges.splice(ri, 1)} aria-label="Remove range">×</button>{/if}
            </div>
          {/each}
          <button class="ghost add" onclick={() => addRange(b)}>+ Add range</button>
        </div>

        <div class="row small">
          <span class="muted">Hits on</span>
          <select bind:value={b.detect}>
            <option value="transient">Sudden rises (drums)</option>
            <option value="level">Level crossing (swells)</option>
          </select>
          <span class="grow"></span><span class="faint mono">{o?.hits ?? 0}</span>
        </div>

        <label>Band gain <span class="mono">{b.gain_db > 0 ? "+" : ""}{b.gain_db.toFixed(1)} dB</span></label>
        <input type="range" min="-12" max="24" step="0.5" bind:value={b.gain_db} />
        <div class="meter"><i style="width:{(o?.level ?? 0) * 100}%;background:{b.color}"></i></div>
        {#if b.detect === "level"}
          <label>Hit when level passes <span class="mono">{Math.round(b.level_threshold * 100)}%</span></label>
          <input type="range" min="0.05" max="1" step="0.01" bind:value={b.level_threshold} />
        {:else}
          <label>Hit sensitivity <span class="mono">{Math.round(b.sensitivity * 100)}%</span></label>
          <input type="range" min="0" max="1" step="0.01" bind:value={b.sensitivity} />
        {/if}
        <label>Min gap between hits <span class="mono">{b.min_gap_ms} ms</span></label>
        <input type="range" min="20" max="1000" step="10" bind:value={b.min_gap_ms} />
        <label>Level release <span class="mono">{Math.round(b.release * 100)}%</span></label>
        <input type="range" min="0" max="0.98" step="0.01" bind:value={b.release} />
        <div class="row small faint">
          <span class="grow">Sources: <span class="mono">{b.id}</span>, <span class="mono">{b.id}.level</span>
            {#if usedBy(b.id)} · {usedBy(b.id)} route(s){/if}</span>
        </div>
        <div class="row">
          <button class="ghost" onclick={() => savePreset(b)}>Save as preset</button>
          <span class="grow"></span>
          <ConfirmButton onconfirm={() => removeBand(b)} confirmLabel={usedBy(b.id) ? "Routes use it. Delete?" : "Delete?"}>Delete</ConfirmButton>
        </div>
      </div>
    {/each}
  </div>
  {#if show.audio.bands.length === 0}<div class="empty panel">No bands. Add one to detect hits in a frequency range.</div>{/if}
  <p class="small muted">
    The first band drives the BPM and beat clock, so keep Kick first. A band can listen to several ranges at once (Snare uses its
    body plus its crack) and each range has a weight. Drums suit "sudden rises"; bass and vocals suit "level crossing", which hits when
    the level passes a line you set. Saved presets are app-wide and show up in every project.
  </p>
</div>

<style>
  .top { display: flex; gap: 16px; align-items: flex-start; }
  .settings { width: 320px; flex-shrink: 0; }
  label { margin-top: 4px; display: flex; justify-content: space-between; }
  .bands { display: grid; grid-template-columns: repeat(auto-fill, minmax(270px, 1fr)); gap: 12px; }
  .band { border-top: 3px solid var(--c); gap: 6px; }
  .band:has(.muted) { opacity: 0.6; }
  .mute { padding: 2px 6px; font-size: 11px; }
  .mute.muted { color: var(--warn); border-color: #f2b54a55; }
  .band.sel { border-color: var(--c); box-shadow: 0 0 0 1px var(--c) inset; }
  .ranges { display: flex; flex-direction: column; gap: 4px; }
  .ranges input[type="number"] { width: 68px; }
  .ranges input.w { width: 52px; }
  .add { align-self: flex-start; padding: 2px 6px; font-size: 12px; }
  input[type="color"] { width: 30px; padding: 2px; }
  .arrow { padding: 2px 6px; }
  .swatch { width: 10px; height: 10px; border-radius: 3px; }
  .hitlamp { width: 14px; height: 14px; border-radius: 50%; background: var(--panel-2); border: 1px solid var(--line-2); }
  .hitlamp.on { background: var(--c); border-color: var(--c); }
</style>
