<script lang="ts">
  // Pick fixtures in three steps: rooms (scope) -> groups / single devices in those rooms.
  import { app, routeFixtures } from "../lib/store.svelte";
  import type { Target } from "../lib/types";

  let { target, onchange = () => {} }: { target: Target; onchange?: () => void } = $props();
  const show = app.show!;

  const inScope = (roomId: string | null) => !target.rooms.length || (roomId !== null && target.rooms.includes(roomId));
  const fixtures = $derived(show.fixtures.filter((f) => inScope(f.room_id)));
  const groups = $derived(show.groups.filter((g) => show.fixtures.some((f) => g.fixtures.includes(f.id) && inScope(f.room_id))));
  const selected = $derived(routeFixtures(target).length);

  function toggle(list: string[], id: string) {
    const i = list.indexOf(id);
    if (i >= 0) list.splice(i, 1); else list.push(id);
    onchange();
  }
  function setRooms(ids: string[]) {
    target.rooms = ids;
    // drop picks that are no longer in scope
    target.fixtures = target.fixtures.filter((id) => fixtures.some((f) => f.id === id));
    target.groups = target.groups.filter((id) => groups.some((g) => g.id === id));
    onchange();
  }
</script>

<div class="tp">
  {#if show.rooms.length > 1}
    <div class="row flow">
      <span class="lbl">in</span>
      <button class="chip" class:on={!target.rooms.length} onclick={() => setRooms([])}>Any room</button>
      {#each show.rooms as r (r.id)}
        <button class="chip room" class:on={target.rooms.includes(r.id)} onclick={() => toggle(target.rooms, r.id)}>{r.name}</button>
      {/each}
    </div>
  {/if}
  <div class="row flow">
    <span class="lbl">on</span>
    <button class="chip" class:on={!target.fixtures.length && !target.groups.length}
      onclick={() => { target.fixtures = []; target.groups = []; onchange(); }}>
      {target.rooms.length ? "Everything in " + (target.rooms.length === 1 ? show.rooms.find((r) => r.id === target.rooms[0])?.name : "these rooms") : "All fixtures"}
    </button>
    {#each groups as g (g.id)}
      <button class="chip grp" style="--c:{g.color}" class:on={target.groups.includes(g.id)} onclick={() => toggle(target.groups, g.id)}
        title={show.fixtures.filter((f) => g.fixtures.includes(f.id)).map((f) => f.name).join(", ")}>{g.name}</button>
    {/each}
    {#if groups.length && fixtures.length}<span class="sep"></span>{/if}
    {#each fixtures as f (f.id)}
      <button class="chip" class:on={target.fixtures.includes(f.id)} onclick={() => toggle(target.fixtures, f.id)}>{f.name}</button>
    {/each}
    <span class="faint small">{selected} selected</span>
  </div>
</div>

<style>
  .tp { display: flex; flex-direction: column; gap: 6px; }
  .flow { flex-wrap: wrap; }
  .lbl { color: var(--faint); min-width: 18px; }
  .chip { font-size: 12px; padding: 3px 8px; }
  .chip.room.on { background: var(--accent-bg); border-color: var(--accent); }
  .grp { border-color: color-mix(in srgb, var(--c) 50%, var(--line)); }
  .grp.on { background: color-mix(in srgb, var(--c) 25%, transparent); border-color: var(--c); color: var(--text); }
  .sep { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
</style>
