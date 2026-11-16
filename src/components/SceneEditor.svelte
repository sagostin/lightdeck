<script lang="ts">
  // Edit which routes a scene (preset) turns on and off, by route group or one route at a time.
  import { app } from "../lib/store.svelte";
  import { captureRoutes, groupState, routeGroups, routeState, setRoutes, soloGroups, type RouteState } from "../lib/scenes";
  import type { Preset } from "../lib/types";

  let { preset, onassign }: { preset: Preset; onassign: (source: "midi" | "key") => void } = $props();
  const show = app.show!;
  const groups = $derived(routeGroups(show.routes));
  let open = $state<Record<string, boolean>>({});
  const STATES: { id: RouteState; label: string; title: string }[] = [
    { id: "on", label: "On", title: "This scene turns these routes on" },
    { id: "off", label: "Off", title: "This scene turns these routes off" },
    { id: "leave", label: "—", title: "This scene leaves these routes as they are" },
  ];
  const triggers = $derived(show.controls.filter((c) => c.action.type === "preset" && c.action.preset === preset.id));
  const trig = (c: (typeof show.controls)[number]) =>
    c.trigger.source === "key" ? `Key ${c.trigger.key.toUpperCase() || "?"}` : `${c.trigger.kind === "cc" ? "CC" : "Note"} ${c.trigger.number}`;
</script>

<div class="scene col">
  <div class="row flow small">
    <span class="muted grow">Choose what this scene does to each route group. "—" leaves a group as it is.</span>
    <button class="ghost" onclick={() => captureRoutes(preset, show.routes)} title="Record which routes are on right now">Capture what's on now</button>
    <button class="ghost" onclick={() => (preset.routes = {})}>Clear</button>
  </div>
  {#each groups as [name, routes] (name)}
    {@const st = groupState(preset, routes)}
    <div class="grp">
      <div class="row">
        <button class="ghost caret" onclick={() => (open[name] = !open[name])} aria-label="Show routes">{open[name] ? "▾" : "▸"}</button>
        <span class="grow">{name} <span class="faint small">{routes.length} route{routes.length === 1 ? "" : "s"}{st === "mixed" ? " · mixed" : ""}</span></span>
        <button class="ghost small" onclick={() => soloGroups(preset, show.routes, [name])} title="This group on, every other route off">Only this</button>
        <div class="seg">
          {#each STATES as x}
            <button class:sel={st === x.id} class="s-{x.id}" title={x.title} onclick={() => setRoutes(preset, routes.map((r) => r.id), x.id)}>{x.label}</button>
          {/each}
        </div>
      </div>
      {#if open[name]}
        {#each routes as r (r.id)}
          {@const rs = routeState(preset, r.id)}
          <div class="row route">
            <span class="grow small">{r.name || `${r.source} → ${r.channel || r.action.type}`}</span>
            <div class="seg small">
              {#each STATES as x}
                <button class:sel={rs === x.id} class="s-{x.id}" title={x.title} onclick={() => setRoutes(preset, [r.id], x.id)}>{x.label}</button>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <span class="small faint">No routes yet. Add routes on the Routes screen, and give them a group to switch them together.</span>
  {/each}
  <div class="row small">
    <span class="muted">Triggered by:</span>
    {#each triggers as c}<span class="tag mono">{trig(c)}</span>{:else}<span class="faint">nothing yet</span>{/each}
    <span class="grow"></span>
    <button class="ghost" onclick={() => onassign("midi")}>+ MIDI pad</button>
    <button class="ghost" onclick={() => onassign("key")}>+ Key</button>
  </div>
</div>

<style>
  .scene { gap: 6px; padding: 8px 4px 2px 28px; }
  .flow { flex-wrap: wrap; }
  .grp { border-top: 1px solid var(--line); padding-top: 4px; }
  .caret { padding: 0 4px; width: 22px; }
  .route { padding-left: 26px; }
  .seg { display: inline-flex; border: 1px solid var(--line-2); border-radius: 6px; overflow: hidden; }
  .seg button { border: 0; border-radius: 0; padding: 2px 10px; background: transparent; }
  .seg.small button { padding: 1px 8px; font-size: 12px; }
  .seg button.s-on.sel { background: color-mix(in srgb, var(--accent) 45%, transparent); color: var(--text); }
  .seg button.s-off.sel { background: #ff5c6c55; color: var(--text); }
  .seg button.s-leave.sel { background: #ffffff14; color: var(--text); }
</style>
