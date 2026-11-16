<script lang="ts">
  import Tags from "../components/Tags.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import GroupsPanel from "../components/GroupsPanel.svelte";
  import { app, uid, profileOf, modeChannels, capabilityAt, setManual, clearManual, footprint } from "../lib/store.svelte";
  import type { ChannelDef, Fixture, Limit, ZigbeeStatus } from "../lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  const show = app.show!;
  let zbDevices = $state<{ key: string; name: string }[]>([]);
  onMount(async () => {
    try { zbDevices = (await invoke<ZigbeeStatus>("zigbee_status")).devices.map((d) => ({ key: d.key, name: d.friendly_name })); } catch {}
  });
  let selected = $state<string | null>(show.fixtures[0]?.id ?? null);
  let limitsOpen = $state<Record<string, boolean>>({});
  const sel = $derived(show.fixtures.find((f) => f.id === selected));

  function addFixture() {
    const p = show.profiles[0];
    if (!p) { app.view = "library"; return; }
    const out = show.outputs[0]?.id ?? "";
    const used = show.fixtures.filter((f) => f.output_id === out).map((f) => f.address + footprint(f));
    const f: Fixture = {
      id: uid("fx"), name: `${p.model} ${show.fixtures.length + 1}`, profile_id: p.id, mode: p.modes[0]?.name ?? "",
      output_id: out, address: Math.max(1, ...used), room_id: show.active_room, x: 1, y: 1, z: 2, rotation: 90, enabled: true, zigbee_device: null, limits: {},
    };
    show.fixtures.push(f);
    selected = f.id;
  }
  function removeFixture(f: Fixture) {
    clearManual(f.id);
    show.fixtures.splice(show.fixtures.indexOf(f), 1);
    show.routes.forEach((r) => (r.fixtures = r.fixtures.filter((id) => id !== f.id)));
    show.groups.forEach((g) => (g.fixtures = g.fixtures.filter((id) => id !== f.id)));
    selected = show.fixtures[0]?.id ?? null;
  }

  function conflicts(f: Fixture) {
    if (f.zigbee_device !== null) {
      const dup = show.fixtures.find((o) => o.id !== f.id && o.zigbee_device === f.zigbee_device && f.zigbee_device);
      return [...(f.zigbee_device ? [] : ["no Zigbee device chosen"]), ...(dup ? [`same device as ${dup.name}`] : [])];
    }
    const a0 = f.address, a1 = f.address + footprint(f) - 1;
    const hits = show.fixtures.filter((o) => o.id !== f.id && o.zigbee_device === null && o.output_id === f.output_id &&
      o.address <= a1 && o.address + footprint(o) - 1 >= a0);
    const over = a1 > 512;
    return [...hits.map((h) => `overlaps ${h.name}`), ...(over ? ["runs past 512"] : [])];
  }

  /** Group channels by head (projector); channels in no head are "Shared". */
  function sections(f: Fixture) {
    const p = profileOf(f);
    const chans = modeChannels(f);
    const mode = p?.modes.find((m) => m.name === f.mode) ?? p?.modes[0];
    const heads = mode?.heads ?? [];
    const inHead = new Set(heads.flatMap((h) => h.channels));
    const idx = (k: string) => chans.findIndex((c) => c.key === k);
    const res: { name: string; chans: { ch: ChannelDef; i: number }[] }[] = [];
    const shared = chans.map((ch, i) => ({ ch, i })).filter(({ ch }) => !inHead.has(ch.key));
    if (shared.length) res.push({ name: heads.length ? "Shared" : "Channels", chans: shared });
    for (const h of heads) res.push({ name: `Projector ${h.name}`, chans: h.channels.map((k) => ({ ch: chans[idx(k)], i: idx(k) })).filter((x) => x.ch) });
    return res;
  }

  function ensureLimit(f: Fixture, key: string) {
    f.limits[key] ??= { lock: null, range: null, require_tags: [], min_hold_ms: 0 } as Limit;
  }
  function hasLimit(f: Fixture, key: string) {
    const l = f.limits[key];
    return !!l && (l.lock !== null || l.range !== null || l.require_tags.length > 0 || l.min_hold_ms > 0);
  }
</script>

<div class="col">
  <div class="panel">
    <div class="row" style="margin-bottom:8px">
      <h2 class="grow">Patch</h2>
      <button class="primary" onclick={addFixture}>Add fixture</button>
    </div>
    <table>
      <thead><tr><th>Name</th><th>Type</th><th>Mode</th><th>Output</th><th>Address / device</th><th>Ch</th><th>Room</th><th>On</th><th></th></tr></thead>
      <tbody>
        {#each show.fixtures as f (f.id)}
          {@const p = profileOf(f)}
          {@const c = conflicts(f)}
          <tr class:sel={f.id === selected} class="clickable" onclick={() => (selected = f.id)}>
            <td><input bind:value={f.name} /></td>
            <td>
              <select bind:value={f.profile_id} onchange={() => { f.mode = profileOf(f)?.modes[0]?.name ?? ""; f.limits = {}; }}>
                {#each show.profiles as pr}<option value={pr.id}>{pr.manufacturer} {pr.model}</option>{/each}
              </select>
            </td>
            <td><select bind:value={f.mode}>{#each p?.modes ?? [] as m}<option>{m.name}</option>{/each}</select></td>
            <td>
              <select value={f.zigbee_device !== null ? "__zigbee" : f.output_id}
                onchange={(e) => { const v = (e.target as HTMLSelectElement).value;
                  if (v === "__zigbee") f.zigbee_device ??= ""; else { f.zigbee_device = null; f.output_id = v; } }}>
                {#each show.outputs as o}<option value={o.id}>Art-Net: {o.name}</option>{/each}
                <option value="__zigbee">Zigbee</option>
              </select>
            </td>
            <td>
              {#if f.zigbee_device !== null}
                {#if zbDevices.length}
                  <select bind:value={f.zigbee_device} class="zb">
                    <option value="">Choose device…</option>
                    {#each zbDevices as d}<option value={d.key}>{d.name}</option>{/each}
                    {#if f.zigbee_device && !zbDevices.some((d) => d.key === f.zigbee_device)}<option value={f.zigbee_device}>{f.zigbee_device} (not found)</option>{/if}
                  </select>
                {:else}
                  <input bind:value={f.zigbee_device} placeholder="Device (turn Zigbee on to list)" class="zb" />
                {/if}
              {:else}
                <input type="number" min="1" max="512" bind:value={f.address} />
              {/if}
              {#if c.length}<span class="tag blocked" title={c.join(", ")}>!</span>{/if}</td>
            <td class="mono faint">{footprint(f)}</td>
            <td><select bind:value={f.room_id}><option value={null}>—</option>{#each show.rooms as r}<option value={r.id}>{r.name}</option>{/each}</select></td>
            <td><input type="checkbox" bind:checked={f.enabled} /></td>
            <td><ConfirmButton onconfirm={() => removeFixture(f)} confirmLabel="Confirm remove">Remove</ConfirmButton></td>
          </tr>
        {:else}
          <tr><td colspan="9" class="empty">No fixtures patched yet.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>


  <GroupsPanel />
  {#if sel}
    {@const values = app.frame?.fixtures[sel.id] ?? []}
    {@const manual = app.manual[sel.id] ?? {}}
    <div class="row">
      <h2 class="grow">{sel.name} · channels</h2>
      <span class="muted small">Moving a slider takes manual control of that channel. Limits always apply, even to routes.</span>
      <button onclick={() => clearManual(sel.id)}>Release all</button>
    </div>
    <div class="sections">
      {#each sections(sel) as sec}
        <div class="panel col section">
          <h3>{sec.name}</h3>
          {#each sec.chans as { ch, i } (ch.key)}
            {@const v = values[i] ?? 0}
            {@const cap = capabilityAt(ch, v)}
            {@const isManual = manual[ch.key] !== undefined}
            <div class="chan" class:manual={isManual}>
              <div class="row">
                <span class="mono faint addr">{sel.zigbee_device !== null ? i + 1 : sel.address + i}</span>
                <span class="grow name" title={ch.name}>{ch.name}</span>
                {#if hasLimit(sel, ch.key)}<span class="tag accent">limited</span>{/if}
                <button class="ghost small" class:on={limitsOpen[sel.id + ch.key]} onclick={() => { ensureLimit(sel, ch.key); limitsOpen[sel.id + ch.key] = !limitsOpen[sel.id + ch.key]; }}>Limits</button>
                {#if isManual}<button class="ghost small" onclick={() => setManual(sel.id, ch.key, null)}>Release</button>{/if}
              </div>
              <div class="row">
                <input class="grow" type="range" min="0" max="255" value={isManual ? manual[ch.key] : v}
                  oninput={(e) => setManual(sel.id, ch.key, +(e.target as HTMLInputElement).value)} />
                <span class="mono val">{v}</span>
              </div>
              <div class="cap small" class:blk={cap?.tags.includes("blocked")}>{cap?.label ?? "—"}</div>
              {#if limitsOpen[sel.id + ch.key] && sel.limits[ch.key]}
                {@const l = sel.limits[ch.key]}
                <div class="limits col">
                  <div class="row">
                    <label><input type="checkbox" checked={l.lock !== null} onchange={(e) => (l.lock = (e.target as HTMLInputElement).checked ? v : null)} /> Lock at</label>
                    <input type="number" min="0" max="255" disabled={l.lock === null} bind:value={l.lock} />
                  </div>
                  <div class="row">
                    <label><input type="checkbox" checked={l.range !== null} onchange={(e) => (l.range = (e.target as HTMLInputElement).checked ? [0, 255] : null)} /> Range</label>
                    {#if l.range}
                      <input type="number" min="0" max="255" bind:value={l.range[0]} /> –
                      <input type="number" min="0" max="255" bind:value={l.range[1]} />
                      <button class="ghost small" onclick={() => (l.range![1] = v)}>max = now</button>
                    {/if}
                  </div>
                  <div class="row"><label>Hold at least</label><input type="number" min="0" step="50" bind:value={l.min_hold_ms} /> ms</div>
                  <div class="row"><label>Random picks need tags</label><Tags bind:tags={l.require_tags} /></div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  td input:not([type="checkbox"]) { width: 100%; min-width: 60px; }
  td input[type="number"] { width: 64px; min-width: 0; }
  td input.zb { min-width: 140px; }
  .sections { display: grid; grid-template-columns: repeat(auto-fill, minmax(330px, 1fr)); gap: 12px; align-items: start; }
  .chan { padding: 6px 6px 8px; border-radius: 6px; border: 1px solid transparent; }
  .chan.manual { border-color: #7c5cff55; background: var(--accent-bg); }
  .addr { width: 26px; }
  .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .val { width: 28px; text-align: right; }
  .cap { color: var(--muted); padding-left: 34px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cap.blk { color: var(--danger); }
  .limits { margin: 6px 0 0 34px; padding: 8px; background: var(--panel-2); border-radius: 6px; gap: 6px; }
  .limits label { display: flex; gap: 4px; align-items: center; }
  .small { font-size: 11px; padding: 2px 6px; }
</style>
