<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import RoomCanvas from "../components/RoomCanvas.svelte";
  import Meters from "../components/Meters.svelte";
  import Autopilot from "../components/Autopilot.svelte";
  import { app, modeChannels, capabilityAt, profileOf } from "../lib/store.svelte";

  const show = app.show!;
  let selected = $state<string | null>(null);
  const room = $derived(show.rooms.find((r) => r.id === show.active_room) ?? show.rooms[0]);
  const sel = $derived(show.fixtures.find((f) => f.id === selected));
</script>

<div class="live">
  <div class="col grow">
    <div class="row">
      {#each show.rooms as r}
        <button class:on={room?.id === r.id} onclick={() => (show.active_room = r.id)}>{r.name}</button>
      {/each}
      <span class="grow"></span>
      <span class="muted small">Click a fixture to inspect it</span>
    </div>
    {#if room}
      <RoomCanvas {room} {selected} onselect={(id) => (selected = id)} height={520} />
    {:else}
      <div class="empty panel">No rooms yet. Add one in Rooms.</div>
    {/if}

    {#if show.presets.length}
      <div class="panel">
        <div class="row" style="margin-bottom:8px"><h2>Scenes</h2><span class="muted small grow">recall a scene or saved look</span><Autopilot mode="compact" /></div>
        <div class="routes">
          {#each show.presets as p (p.id)}
            <button class="preset" class:on={app.activePresets.includes(p.id)} style="--c:{p.color}"
              onclick={() => invoke("preset_apply", { preset: p.id })}>{p.name}</button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="panel">
      <div class="row" style="margin-bottom:8px"><h2>Routes</h2><span class="muted small">toggle to enable or disable live</span></div>
      <div class="routes">
        {#each show.routes as r}
          <button class="route" class:on={r.enabled} onclick={() => (r.enabled = !r.enabled)}>
            <span class="src">{r.source}</span> → {r.name || r.channel}
            <span class="faint small">{r.action.type.replace("_", " ")}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>

  <div class="col side">
    <div class="panel"><Meters /></div>
    {#if sel}
      {@const values = app.frame?.fixtures[sel.id] ?? []}
      <div class="panel col">
        <div class="row"><h2 class="grow">{sel.name}</h2><span class="tag">{profileOf(sel)?.model}</span></div>
        <div class="scroll chans">
          {#each modeChannels(sel) as ch, i}
            {@const v = values[i] ?? 0}
            {@const cap = capabilityAt(ch, v)}
            <div class="chan">
              <span class="mono faint">{sel.address + i}</span>
              <span class="grow name" title={ch.name}>{ch.name}</span>
              <span class="mono">{v}</span>
            </div>
            <div class="capl small faint" title={cap?.label}>{cap?.label ?? ""}</div>
          {/each}
        </div>
        <button onclick={() => (app.view = "patch")}>Open in Patch</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .live { display: flex; gap: 16px; align-items: flex-start; }
  .side { width: 320px; flex-shrink: 0; }
  .routes { display: flex; flex-wrap: wrap; gap: 6px; }
  .route { color: var(--muted); }
  .preset { border-left: 4px solid var(--c); min-width: 110px; padding: 8px 12px; }
  .preset.on { background: color-mix(in srgb, var(--c) 22%, transparent); border-color: var(--c); color: var(--text); }
  .src { color: var(--accent-2); }
  .chans { max-height: 420px; }
  .chan { display: flex; gap: 8px; padding-top: 4px; }
  .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .capl { padding-left: 30px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; border-bottom: 1px solid #ffffff08; padding-bottom: 3px; }
</style>
