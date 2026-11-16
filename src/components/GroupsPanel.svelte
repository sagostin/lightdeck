<script lang="ts">
  import ConfirmButton from "./ConfirmButton.svelte";
  import { app, uid, profileOf } from "../lib/store.svelte";
  import type { FixtureGroup } from "../lib/types";

  const show = app.show!;
  const PALETTE = ["#7c5cff", "#45d39a", "#f2b54a", "#ff5c6c", "#4fb3ff", "#ff7ad9", "#4fe0d0", "#ff9a4d"];
  let open = $state<string | null>(null);

  const kinds = $derived([...new Set(show.fixtures.map((f) => profileOf(f)?.kind ?? "other"))]);
  const rooms = $derived(show.rooms.filter((r) => show.fixtures.some((f) => f.room_id === r.id)));
  const usedBy = (id: string) => show.routes.filter((r) => r.groups.includes(id)).length;

  function add() {
    const g: FixtureGroup = { id: uid("grp"), name: `Group ${show.groups.length + 1}`, color: PALETTE[show.groups.length % PALETTE.length], fixtures: [] };
    show.groups.push(g);
    open = g.id;
  }
  function toggle(g: FixtureGroup, id: string) {
    const i = g.fixtures.indexOf(id);
    if (i >= 0) g.fixtures.splice(i, 1); else g.fixtures.push(id);
  }
  function addWhere(g: FixtureGroup, pred: (f: (typeof show.fixtures)[number]) => boolean) {
    for (const f of show.fixtures) if (pred(f) && !g.fixtures.includes(f.id)) g.fixtures.push(f.id);
  }
  function remove(g: FixtureGroup) {
    show.groups.splice(show.groups.indexOf(g), 1);
    for (const r of show.routes) r.groups = r.groups.filter((x) => x !== g.id);
  }
</script>

<div class="panel col">
  <div class="row">
    <h2 class="grow">Fixture groups</h2>
    <span class="muted small">Use groups as route targets, e.g. "Front pars" or "Ceiling bulbs".</span>
    <button onclick={add}>New group</button>
  </div>
  {#each show.groups as g (g.id)}
    <div class="group" style="--c:{g.color}">
      <div class="row">
        <input type="color" bind:value={g.color} />
        <input class="grow" bind:value={g.name} />
        <span class="faint small">{g.fixtures.length} fixture{g.fixtures.length === 1 ? "" : "s"}{usedBy(g.id) ? ` · ${usedBy(g.id)} route(s)` : ""}</span>
        <button class="ghost" onclick={() => (open = open === g.id ? null : g.id)}>{open === g.id ? "Done" : "Edit members"}</button>
        <ConfirmButton onconfirm={() => remove(g)}>Delete</ConfirmButton>
      </div>
      {#if open === g.id}
        <div class="chips">
          {#each show.fixtures as f}
            <button class="chip" class:on={g.fixtures.includes(f.id)} onclick={() => toggle(g, f.id)}>{f.name}</button>
          {/each}
        </div>
        <div class="row small">
          <span class="faint">Add all:</span>
          {#each kinds as k}<button class="ghost" onclick={() => addWhere(g, (f) => profileOf(f)?.kind === k)}>{k.replace("_", " ")}</button>{/each}
          {#each rooms as r}<button class="ghost" onclick={() => addWhere(g, (f) => f.room_id === r.id)}>in {r.name}</button>{/each}
          <span class="grow"></span>
          <button class="ghost" onclick={() => (g.fixtures = [])}>Clear</button>
        </div>
      {:else if g.fixtures.length}
        <div class="members small muted">
          {show.fixtures.filter((f) => g.fixtures.includes(f.id)).map((f) => f.name).join(", ")}
        </div>
      {/if}
    </div>
  {:else}
    <div class="faint small">No groups yet.</div>
  {/each}
</div>

<style>
  .group { border-left: 3px solid var(--c); border-radius: 0; padding: 6px 0 6px 10px; display: flex; flex-direction: column; gap: 6px; }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .chip { font-size: 12px; padding: 3px 8px; }
  .chip.on { background: color-mix(in srgb, var(--c) 25%, transparent); border-color: var(--c); color: var(--text); }
  .members { padding-left: 38px; }
  input[type="color"] { width: 30px; padding: 2px; }
  .small .ghost { font-size: 12px; padding: 2px 6px; }
</style>
