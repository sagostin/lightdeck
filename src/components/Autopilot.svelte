<script lang="ts">
  // The autopilot, in three forms:
  //  - "bar": the strip at the top of the Scenes screen (on/off, what it hears, hold a section)
  //  - "settings": when to switch, limits per section, guessing scene tags
  //  - "compact": the switch on the Live screen
  import { invoke } from "@tauri-apps/api/core";
  import { app, toast } from "../lib/store.svelte";
  import { SECTIONS, autopilotSettings, coverageNote, guessAll, sectionInfo } from "../lib/autopilot";
  import type { Section, SectionRule } from "../lib/types";

  let { mode = "bar", onsettings }: { mode?: "bar" | "settings" | "compact"; onsettings?: () => void } = $props();
  const show = app.show!;
  const settings = autopilotSettings(show);
  const st = $derived(app.autopilot);
  const sceneName = (id: string | null | undefined) => show.presets.find((p) => p.id === id)?.name ?? "—";
  const note = $derived(coverageNote(show));
  const tagged = $derived(show.presets.filter((p) => p.sections?.length).length);

  async function setRunning(on: boolean) {
    if (on && !tagged) { toast("Tag some scenes with sections first: pick Breakdown, Groove, Build or Drop on each scene"); return; }
    await invoke("autopilot_set", { on });
    if (app.autopilot) app.autopilot.running = on;
  }
  async function force(s: Section) {
    const next = st?.forced === s ? null : s;
    await invoke("autopilot_force", { section: next });
    if (app.autopilot) app.autopilot.forced = next;
  }
  function guess(overwrite: boolean) {
    const changed = guessAll(show, overwrite);
    toast(changed.length ? `Tagged ${changed.map((c) => `${c.name} (${c.why})`).join("; ")}` : "Nothing to change");
  }

  // ---- limits per section ----
  const DEFAULT_RULE: SectionRule = { brightness: 1, strobe: true, fog: true };
  const rule = (s: Section) => settings.rules?.[s] ?? DEFAULT_RULE;
  function setRule(s: Section, change: Partial<SectionRule>) {
    const next = { ...rule(s), ...change };
    settings.rules ??= {};
    if (next.brightness >= 1 && next.strobe && next.fog) delete settings.rules[s];
    else settings.rules[s] = next;
  }
  function suggestedLimits() {
    settings.rules = {
      breakdown: { brightness: 0.6, strobe: false, fog: false },
      groove: { brightness: 0.85, strobe: false, fog: false },
      build: { brightness: 1, strobe: true, fog: false },
    };
    toast("Strobe only in builds and drops, fog only in drops, breakdowns dimmer");
  }
</script>

{#if mode === "compact"}
  <div class="compact row">
    <button class="toggle" class:on={st?.running} onclick={() => setRunning(!st?.running)} title="Switch scenes automatically with the music">
      Autopilot {st?.running ? "on" : "off"}
    </button>
    {#if st}
      <span class="sec" style="--c:{sectionInfo(st.section)?.color}">{sectionInfo(st.section)?.name}{st.forced ? " (held)" : ""}</span>
      {#if st.running}<span class="small">{sceneName(st.scene)}</span>{/if}
      {#if st.running && st.paused_s}<button class="ghost small" onclick={() => invoke("autopilot_resume")}>paused {st.paused_s}s · resume</button>{/if}
    {/if}
  </div>
{:else if mode === "bar"}
  <div class="panel bar">
    <div class="row wrap">
      <b>Autopilot</b>
      <button class="toggle" class:on={st?.running} onclick={() => setRunning(!st?.running)}>{st?.running ? "On" : "Off"}</button>
      <span class="faint small">Hearing</span>
      <span class="sec" style="--c:{sectionInfo(st?.section ?? "groove")?.color}">{sectionInfo(st?.section ?? "groove")?.name}{st?.forced ? " (held)" : ""}</span>
      <span class="meter" title="Loudness compared with the last couple of minutes"><i style="width:{Math.round((st?.loudness ?? 0) * 100)}%"></i></span>
      <div class="history" title="The last bars, oldest first">
        {#each st?.history ?? [] as h, i (i)}<i style="--c:{sectionInfo(h)?.color}" title={sectionInfo(h)?.name}></i>{/each}
      </div>
      <span class="grow"></span>
      {#if st?.running}
        <span class="small">Playing <b>{sceneName(st.scene)}</b> · {st.bars_in_scene} bar{st.bars_in_scene === 1 ? "" : "s"}</span>
        {#if st.paused_s}<button class="ghost small" onclick={() => invoke("autopilot_resume")}>Paused {st.paused_s}s · resume</button>{/if}
      {/if}
      {#if onsettings}<button class="ghost" onclick={onsettings}>Settings</button>{/if}
    </div>
    <div class="row wrap small">
      <span class="faint">Hold a section:</span>
      {#each SECTIONS as s}
        <button class="chip" class:on={st?.forced === s.id} style="--c:{s.color}" onclick={() => force(s.id)}
          title="Play as if this section were heard, e.g. a drop the detection missed. Click again to let go.">{s.name}</button>
      {/each}
      {#if st?.running && st.reason}<span class="faint grow reason">· {st.reason}</span>{/if}
    </div>
    {#if note}<span class="small warn">{note}</span>{/if}
  </div>
{:else}
  <div class="col settings-pane">
    <h3>When to switch scenes</h3>
    <div class="grid">
      <label for="ap-min">Stay at least</label>
      <span><input id="ap-min" type="number" min="1" max="64" bind:value={settings.min_bars} /> bars</span>
      <label for="ap-drop">A drop cuts in after</label>
      <span><input id="ap-drop" type="number" min="1" max={settings.min_bars} bind:value={settings.drop_bars} /> bars</span>
      <label for="ap-max">Change scene anyway after</label>
      <span><input id="ap-max" type="number" min="0" max="256" bind:value={settings.max_bars} /> bars <span class="faint small">(0 = never)</span></span>
      <label for="ap-pause">After you pick a scene, wait</label>
      <span><input id="ap-pause" type="number" min="0" max="600" bind:value={settings.pause_s} /> s</span>
    </div>

    <h3>Limits per section</h3>
    <span class="small muted">
      These apply to every scene, whether the autopilot or you picked it, as soon as the section changes. Manual control
      (a fog button you hold, Patch sliders) is never limited.
    </span>
    <table class="rules">
      <thead><tr><th></th><th>Brightness</th><th>Strobe</th><th>Fog</th></tr></thead>
      <tbody>
        {#each SECTIONS as s}
          {@const r = rule(s.id)}
          <tr>
            <td><span class="sec" style="--c:{s.color}">{s.name}</span></td>
            <td><input type="range" min="0.1" max="1" step="0.05" value={r.brightness} oninput={(e) => setRule(s.id, { brightness: +e.currentTarget.value })} />
              <span class="small mono">{Math.round(r.brightness * 100)}%</span></td>
            <td><input type="checkbox" checked={r.strobe} onchange={(e) => setRule(s.id, { strobe: e.currentTarget.checked })} /></td>
            <td><input type="checkbox" checked={r.fog} onchange={(e) => setRule(s.id, { fog: e.currentTarget.checked })} /></td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="row">
      <button class="ghost" onclick={suggestedLimits}>Suggested limits</button>
      <button class="ghost" onclick={() => (settings.rules = {})}>No limits</button>
    </div>

    <h3>Scene tags</h3>
    <span class="small muted">Tags are set on each scene. Guessing uses a scene's name ("Drop", "Chill") or what it switches on.</span>
    <div class="row">
      <button onclick={() => guess(false)}>Guess untagged scenes</button>
      <button class="ghost" onclick={() => guess(true)}>Guess all again</button>
    </div>
  </div>
{/if}

<style>
  .toggle { min-width: 56px; }
  .toggle.on { background: var(--accent-bg); border-color: var(--accent); color: var(--text); }
  .sec { padding: 2px 8px; border-radius: 10px; font-size: 12px; background: color-mix(in srgb, var(--c) 25%, transparent); border: 1px solid var(--c); white-space: nowrap; }
  .meter { width: 70px; height: 6px; border-radius: 3px; background: var(--line); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--accent); }
  .bar { display: flex; flex-direction: column; gap: 6px; padding: 8px 12px; }
  .wrap { flex-wrap: wrap; gap: 8px; }
  .history { display: flex; gap: 2px; align-items: center; }
  .history i { width: 6px; height: 12px; border-radius: 2px; background: var(--c); }
  .chip { font-size: 11px; padding: 2px 8px; color: var(--muted); }
  .chip.on { background: color-mix(in srgb, var(--c) 30%, transparent); border-color: var(--c); color: var(--text); }
  .reason { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .grid { display: grid; grid-template-columns: auto auto; gap: 6px 10px; align-items: center; justify-items: start; }
  .grid label { margin: 0; }
  .grid input { width: 64px; }
  .rules { border-collapse: collapse; }
  .rules td, .rules th { padding: 3px 10px 3px 0; text-align: left; font-weight: normal; }
  .rules th { color: var(--faint); font-size: 12px; }
  .rules input[type="range"] { width: 110px; vertical-align: middle; }
  .warn { color: var(--warn); }
  .compact { gap: 8px; }
  .settings-pane { gap: 8px; }
  h3 { margin: 8px 0 0; }
</style>
