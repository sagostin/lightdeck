<script lang="ts">
  import RoomCanvas from "../components/RoomCanvas.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import { app, uid, profileOf } from "../lib/store.svelte";

  const show = app.show!;
  let roomId = $state(show.active_room ?? show.rooms[0]?.id ?? null);
  let selected = $state<string | null>(null);
  const room = $derived(show.rooms.find((r) => r.id === roomId));
  const sel = $derived(show.fixtures.find((f) => f.id === selected));
  const unplaced = $derived(show.fixtures.filter((f) => f.room_id !== roomId));
  let newW = $state(6), newD = $state(4.5);
  let selectedFocus = $state<string | null>(null);
  const focusSel = $derived(room?.focus.find((f) => f.id === selectedFocus));
  const focusUsedBy = (id: string) => show.routes.filter((r) => r.focus === id).length;

  function addFocus() {
    if (!room) return;
    const xs = room.walls.map((p) => p[0]), ys = room.walls.map((p) => p[1]);
    const fp = {
      id: uid("focus"), name: room.focus.length ? `Focus ${room.focus.length + 1}` : "Dance floor",
      x: Math.round(((Math.min(...xs) + Math.max(...xs)) / 2) * 10) / 10,
      y: Math.round(((Math.min(...ys) + Math.max(...ys)) / 2) * 10) / 10,
      z: 1.2, radius: 2.5, color: ["#ffd27a", "#4fe0d0", "#ff7ad9", "#a594ff"][room.focus.length % 4],
    };
    room.focus.push(fp);
    selectedFocus = fp.id;
    selected = null;
  }
  function removeFocus(id: string) {
    if (!room) return;
    room.focus.splice(room.focus.findIndex((f) => f.id === id), 1);
    for (const r of show.routes) if (r.focus === id) { r.focus = ""; r.falloff = false; }
    selectedFocus = null;
  }
  /** Point a fixture's facing at a focus point. */
  function aim(f: { x: number; y: number; rotation: number }, fp: { x: number; y: number }) {
    if (Math.hypot(fp.x - f.x, fp.y - f.y) < 0.05) return;
    f.rotation = Math.round(((Math.atan2(fp.y - f.y, fp.x - f.x) * 180) / Math.PI + 360) % 360);
  }
  function aimAll(fp: { x: number; y: number }) {
    for (const f of show.fixtures) if (f.room_id === roomId) aim(f, fp);
  }

  function addRoom() {
    const r = { id: uid("room"), name: `Room ${show.rooms.length + 1}`, walls: [[0, 0], [newW, 0], [newW, newD], [0, newD]] as [number, number][], ceiling_m: 2.4, focus: [] };
    show.rooms.push(r);
    roomId = r.id;
  }
  function removeRoom() {
    if (!room) return;
    show.fixtures.forEach((f) => { if (f.room_id === room.id) f.room_id = null; });
    show.rooms.splice(show.rooms.indexOf(room), 1);
    roomId = show.rooms[0]?.id ?? null;
  }
  function place(id: string) {
    const f = show.fixtures.find((f) => f.id === id);
    if (!f || !room) return;
    f.room_id = room.id;
    const xs = room.walls.map((p) => p[0]), ys = room.walls.map((p) => p[1]);
    f.x = Math.round(((Math.min(...xs) + Math.max(...xs)) / 2) * 10) / 10;
    f.y = Math.round(((Math.min(...ys) + Math.max(...ys)) / 2) * 10) / 10;
    selected = id;
  }
</script>

<div class="rooms">
  <div class="col list">
    <h2>Rooms</h2>
    {#each show.rooms as r}
      <button class="item" class:on={r.id === roomId} onclick={() => (roomId = r.id)}>{r.name}</button>
    {/each}
    <div class="panel col">
      <h3>New room</h3>
      <div class="row"><input type="number" step="0.1" bind:value={newW} /> × <input type="number" step="0.1" bind:value={newD} /> m</div>
      <button class="primary" onclick={addRoom}>Add room</button>
    </div>
  </div>

  {#if room}
    <div class="col grow">
      <div class="row">
        <input class="grow" bind:value={room.name} />
        <label>Ceiling</label><input type="number" step="0.1" bind:value={room.ceiling_m} /> m
        <button class:on={show.active_room === room.id} onclick={() => (show.active_room = room.id)}>Show on Live</button>
        <ConfirmButton class="danger" onconfirm={removeRoom} confirmLabel="Delete room? Fixtures become unplaced">Delete</ConfirmButton>
      </div>
      <RoomCanvas {room} editable {selected} onselect={(id) => (selected = id)}
        {selectedFocus} onselectfocus={(id) => (selectedFocus = id)} height={540} />
      <div class="muted small">
        Drag corners, fixtures and focus points (snaps to 10 cm). Drag a selected focus point's outer handle to set its radius.
        Double-click a wall length to add a corner; Alt-click a corner to remove it.
      </div>
    </div>

    <div class="col side">
      {#if sel}
        <div class="panel col">
          <h2>{sel.name}</h2>
          <span class="muted small">{profileOf(sel)?.manufacturer} {profileOf(sel)?.model}</span>
          <div class="grid2">
            <label>X (m)</label><input type="number" step="0.1" bind:value={sel.x} />
            <label>Y (m)</label><input type="number" step="0.1" bind:value={sel.y} />
            <label>Height (m)</label><input type="number" step="0.1" bind:value={sel.z} />
            <label>Facing (°)</label><input type="number" step="5" bind:value={sel.rotation} />
          </div>
          <input type="range" min="0" max="355" step="5" bind:value={sel.rotation} />
          {#if room.focus.length}
            <div class="row wrap small">
              <span class="faint">Aim at</span>
              {#each room.focus as fp (fp.id)}<button class="ghost" onclick={() => aim(sel, fp)}>{fp.name}</button>{/each}
            </div>
          {/if}
          <button onclick={() => { sel.room_id = null; selected = null; }}>Remove from room</button>
        </div>
      {/if}
      <div class="panel col">
        <div class="row"><h3 class="grow">Focus points</h3><button class="ghost" onclick={addFocus}>+ Add</button></div>
        <span class="small muted">Centres that effects can be calculated from: waves ripple out from them, palettes form rings around them, twinkles cluster near them.</span>
        {#each room.focus as fp (fp.id)}
          <button class="item fp" class:on={fp.id === selectedFocus} style="--fc:{fp.color}"
            onclick={() => { selectedFocus = fp.id; selected = null; }}>
            <span class="fdot"></span><span class="grow">{fp.name}</span>
            {#if focusUsedBy(fp.id)}<span class="faint small">{focusUsedBy(fp.id)} route(s)</span>{/if}
          </button>
        {/each}
        {#if focusSel}
          <div class="col fedit">
            <div class="row"><input type="color" bind:value={focusSel.color} /><input class="grow" bind:value={focusSel.name} /></div>
            <div class="grid2">
              <label>X (m)</label><input type="number" step="0.1" bind:value={focusSel.x} />
              <label>Y (m)</label><input type="number" step="0.1" bind:value={focusSel.y} />
              <label>Height (m)</label><input type="number" step="0.1" bind:value={focusSel.z} />
              <label>Radius (m)</label><input type="number" step="0.1" min="0.3" bind:value={focusSel.radius} />
            </div>
            <div class="row">
              <button onclick={() => aimAll(focusSel)}>Aim every fixture here at it</button>
              <span class="grow"></span>
              <ConfirmButton onconfirm={() => removeFocus(focusSel.id)}>Delete</ConfirmButton>
            </div>
          </div>
        {/if}
      </div>
      <div class="panel col">
        <h3>Fixtures not in this room</h3>
        {#each unplaced as f}
          <div class="row"><span class="grow">{f.name}</span><button onclick={() => place(f.id)}>Place</button></div>
        {:else}
          <span class="faint small">All fixtures are placed here.</span>
        {/each}
        <button class="ghost" onclick={() => (app.view = "patch")}>Add fixtures in Patch →</button>
      </div>
    </div>
  {:else}
    <div class="empty grow">Add a room to start laying out fixtures.</div>
  {/if}
</div>

<style>
  .rooms { display: flex; gap: 16px; align-items: flex-start; }
  .list { width: 180px; flex-shrink: 0; }
  .item { text-align: left; }
  .side { width: 260px; flex-shrink: 0; }
  .wrap { flex-wrap: wrap; gap: 4px; }
  .fp { display: flex; align-items: center; gap: 8px; }
  .fdot { width: 10px; height: 10px; border-radius: 50%; border: 2px solid var(--fc); }
  .fp.on .fdot { background: var(--fc); }
  .fedit { border-top: 1px solid var(--line); padding-top: 8px; }
  input[type="color"] { width: 30px; padding: 2px; }
  .grid2 { display: grid; grid-template-columns: auto 1fr; gap: 6px 10px; align-items: center; }
  .grid2 input { width: 100%; }
</style>
