<script lang="ts">
  import Tags from "../components/Tags.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import TargetPicker from "../components/TargetPicker.svelte";
  import ChannelSelect from "../components/ChannelSelect.svelte";
  import Meters from "../components/Meters.svelte";
  import { app, uid, modeChannels, profileOf, routeFixtures, fixtureHasChannel, clearManual } from "../lib/store.svelte";
  import { sourceOptions, isTriggerSource, GROUP_LABEL, EFFECT_ACTIONS, type Route, type RouteAction } from "../lib/types";
  import PaletteSelect from "../components/PaletteSelect.svelte";
  import PaletteEditor from "../components/PaletteEditor.svelte";

  const show = app.show!;
  const sources = $derived(sourceOptions(show.audio.bands));

  const ACTIONS: { type: RouteAction["type"]; label: string; make: () => RouteAction }[] = [
    { type: "flash", label: "Flash", make: () => ({ type: "flash", on: 255, off: 0, decay_ms: 250 }) },
    { type: "random", label: "Random value", make: () => ({ type: "random", tags: [], hold_ms: 500 }) },
    { type: "node_hop", label: "Node hop", make: () => ({ type: "node_hop", on: 255, off: 0, decay_ms: 300, palette: "" }) },
    { type: "modulate", label: "Modulate", make: () => ({ type: "modulate", min: 0, max: 255, smoothing: 0.6 }) },
    { type: "cycle", label: "Cycle values", make: () => ({ type: "cycle", values: [0, 64, 128, 192] }) },
    { type: "look", label: "Look (pattern + options)", make: () => ({ type: "look", set: "", include_defaults: true, tags: [], mode: "random", hold_ms: 800, heads: [] }) },
    { type: "strobe", label: "Strobe (effect)", make: () => ({ type: "strobe", rate: "4", hz: 10, duty: 0.3, burst_ms: 400, threshold: 0.7 }) },
    { type: "palette", label: "Colour palette (effect)", make: () => ({ type: "palette", palette: firstPalette(), mode: "spread", fade_ms: 150, brightness: 255 }) },
    { type: "twinkle", label: "Twinkle (effect)", make: () => ({ type: "twinkle", palette: firstPalette(), amount: 6, decay_ms: 500, base: 10 }) },
    { type: "wave", label: "Wave (effect)", make: () => ({ type: "wave", palette: firstPalette(), axis: "x", beats: 4, width: 0.3, base: 0 }) },
  ];
  function firstPalette() { return show.palettes[0]?.id ?? ""; }
  /** Focus points to offer: those in the route's rooms (or every room). */
  const focusOptions = (r: Route) =>
    show.rooms
      .filter((room) => !r.rooms.length || r.rooms.includes(room.id))
      .flatMap((room) => (room.focus ?? []).map((f) => ({ id: f.id, label: show.rooms.length > 1 ? `${f.name} (${room.name})` : f.name })));
  const usesFocus = (r: Route) => isEffect(r) || r.action.type === "node_hop";
  /** Target fixtures with channels held by manual control (sliders, browser, presets): routes can't change those. */
  function heldBy(r: Route) {
    return routeFixtures(r)
      .map((f) => ({ f, n: Object.keys(app.manual[f.id] ?? {}).length }))
      .filter((x) => x.n > 0);
  }
  // ----- looks -----
  const isLook = (r: Route) => r.action.type === "look";
  const lookFixtures = (r: Route) => routeFixtures(r).filter((f) => (profileOf(f)?.looks.length ?? 0) > 0);
  function lookInfo(r: Route) {
    const fx = lookFixtures(r);
    const profiles = [...new Map(fx.map((f) => [f.profile_id, profileOf(f)!])).values()];
    const sets = new Map<string, string>();
    const heads = new Set<string>();
    for (const p of profiles) {
      for (const s of p.look_sets) sets.set(s.id, s.name);
      for (const m of p.modes.slice(0, 1)) for (const h of m.heads) heads.add(h.name);
    }
    const a = r.action.type === "look" ? r.action : null;
    const matching = a
      ? profiles.flatMap((p) => p.looks).filter((l) => (!a.set || l.sets.includes(a.set)) && (a.include_defaults || !l.builtin) && a.tags.every((t) => l.tags.includes(t))).length
      : 0;
    const tags = [...new Set(profiles.flatMap((p) => p.looks.flatMap((l) => l.tags)))].sort();
    return { fx: fx.length, sets: [...sets.entries()], heads: [...heads], matching, tags };
  }
  const isEffect = (r: Route) => EFFECT_ACTIONS.includes(r.action.type);
  /** Fixtures an effect can drive: ones with brightness or colour channels. */
  const effectTargets = (r: Route) =>
    routeFixtures(r).filter((f) => modeChannels(f).some((c) => c.group === "intensity" || c.group === "color"));
  const HELP: Record<string, string> = {
    flash: "On each trigger jump to On, then fade back to Off.",
    random: "On each trigger pick a random allowed value. Uses the pattern catalog first, filtered by tags.",
    node_hop: "On each trigger one fixture flashes, hopping to a nearby fixture in the room next time. With a focus point, it starts nearest the focus and keeps getting pulled back towards it.",
    modulate: "Continuously follow a level source (or a decaying pulse for trigger sources), mapped to Min–Max.",
    cycle: "On each trigger step to the next value in the list.",
    look: "On each trigger apply a saved look: a pattern plus its secondary options (gallery, zoom, rotation, colour…) together. Pick from a set and/or by tags. Choosing heads applies P1 looks to P2 too (channels P2 doesn't have are skipped).",
    strobe: "Hit sources: strobe for the burst time after each hit. Level sources: strobe while the level is above the threshold. Beat-locked rates follow the BPM.",
    palette: "Each trigger steps to the palette's next colour. Spread, rings and slices give fixtures different colours by position, so the colours travel across the room, outward from the focus, or around it on each hit.",
    twinkle: "Random fixtures sparkle in palette colours, clustering near the focus point if one is set. Level sources: more sparkles when louder. Hit sources: a burst of sparkles per hit.",
    wave: "A band of light sweeps across the room in time with the beat (or ripples out from / rotates around the focus), changing colour each sweep. A level source scales its brightness.",
  };

  // ----- grouping -----
  type GroupBy = "group" | "kind" | "fixture" | "room" | "source" | "channel" | "none";
  const GROUP_BY: { id: GroupBy; label: string }[] = [
    { id: "group", label: "Your groups" },
    { id: "kind", label: "Device type" },
    { id: "fixture", label: "Fixture / group" },
    { id: "room", label: "Room" },
    { id: "source", label: "Audio source" },
    { id: "channel", label: "Channel type" },
    { id: "none", label: "No grouping" },
  ];
  const KIND_LABEL: Record<string, string> = { laser: "Lasers", par: "Pars", fog: "Fog", moving_head: "Moving heads", strobe: "Strobes", zigbee: "Zigbee lights", other: "Other" };
  const lsGet = (k: string, d: string) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
  const lsSet = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch {} };
  let groupBy = $state<GroupBy>(lsGet("routes.groupBy", "group") as GroupBy);
  let sortBy = $state<"order" | "name" | "source">(lsGet("routes.sortBy", "order") as "order");
  let search = $state("");
  let collapsed = $state<Record<string, boolean>>({});
  $effect(() => { lsSet("routes.groupBy", groupBy); lsSet("routes.sortBy", sortBy); });

  /** Fixtures the route actually drives (targets that have the channel). */
  const targets = (r: Route) => routeFixtures(r).filter((f) => !r.channel || fixtureHasChannel(f, r.channel));
  const targetLabel = (r: Route) => {
    const rooms = show.rooms.filter((x) => r.rooms.includes(x.id)).map((x) => x.name);
    const scope = rooms.length ? ` in ${rooms.join(" + ")}` : "";
    if (!r.fixtures.length && !r.groups.length) return (rooms.length ? "Everything" : "All fixtures") + scope;
    const parts = [
      ...show.groups.filter((g) => r.groups.includes(g.id)).map((g) => g.name),
      ...show.fixtures.filter((f) => r.fixtures.includes(f.id)).map((f) => f.name),
    ];
    return (parts.join(", ") || "Nothing selected") + scope;
  };
  const channelLabel = (r: Route) => {
    if (r.channel.startsWith("@")) return `All ${(GROUP_LABEL[r.channel.slice(1)] ?? r.channel.slice(1)).toLowerCase()} channels`;
    for (const f of routeFixtures(r)) {
      const c = modeChannels(f).find((c) => c.key === r.channel);
      if (c) return c.name;
    }
    return r.channel;
  };
  const sourceLabel = (src: string) => [...sources.triggers, ...sources.levels].find((s) => s.id === src)?.label ?? src;
  function groupKey(r: Route): string {
    switch (groupBy) {
      case "group": return r.group.trim() || "Ungrouped";
      case "kind": {
        const kinds = [...new Set(targets(r).map((f) => profileOf(f)?.kind ?? "other"))];
        return kinds.length ? kinds.map((k) => KIND_LABEL[k] ?? k).sort().join(" + ") : "No matching fixtures";
      }
      case "fixture": return targetLabel(r);
      case "room": {
        const ids = new Set(targets(r).map((f) => f.room_id));
        const names = show.rooms.filter((x) => ids.has(x.id)).map((x) => x.name);
        return names.length ? names.join(" + ") : ids.has(null) ? "Not in a room" : "No matching fixtures";
      }
      case "source": return sourceLabel(r.source);
      case "channel": {
        if (isEffect(r)) return "Effects";
        if (r.channel.startsWith("@")) return GROUP_LABEL[r.channel.slice(1)] ?? r.channel.slice(1);
        const f = targets(r)[0];
        const ch = f ? modeChannels(f).find((c) => c.key === r.channel) : undefined;
        return ch ? GROUP_LABEL[ch.group] ?? ch.group : "No channel";
      }
      default: return "All routes";
    }
  }
  const groups = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const list = show.routes
      .map((r, i) => ({ r, i }))
      .filter(({ r }) => !q || [r.name, r.group, channelLabel(r), sourceLabel(r.source), targetLabel(r), ...targets(r).map((f) => f.name)].join(" ").toLowerCase().includes(q));
    if (sortBy === "name") list.sort((a, b) => (a.r.name || a.r.channel).localeCompare(b.r.name || b.r.channel));
    if (sortBy === "source") list.sort((a, b) => sourceLabel(a.r.source).localeCompare(sourceLabel(b.r.source)));
    const m = new Map<string, { r: Route; i: number }[]>();
    for (const x of list) {
      const k = groupKey(x.r);
      if (!m.has(k)) m.set(k, []);
      m.get(k)!.push(x);
    }
    return [...m.entries()].sort(([a], [b]) => (a === "Ungrouped" ? 1 : b === "Ungrouped" ? -1 : a.localeCompare(b)));
  });
  const groupNames = $derived([...new Set(show.routes.map((r) => r.group.trim()).filter(Boolean))].sort());
  function setAll(items: { r: Route }[], on: boolean) { for (const { r } of items) r.enabled = on; }

  function addRoute(group = "") {
    show.routes.push({
      id: uid("r"), name: "", group: group === "Ungrouped" ? "" : group, enabled: true, source: show.audio.bands[0]?.id ?? "beat", rooms: [], fixtures: [], groups: [], channel: "", focus: "", falloff: false,
      action: ACTIONS[0].make(), cooldown_ms: 0, every: 1, probability: 1,
    });
  }
  /** After changing targets, keep the channel if the new targets still have it. */
  function retarget(r: Route) {
    if (r.channel && !routeFixtures(r).some((f) => fixtureHasChannel(f, r.channel))) r.channel = "";
  }
  function setAction(r: Route, type: string) {
    r.action = ACTIONS.find((a) => a.type === type)!.make();
  }
  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= show.routes.length) return;
    [show.routes[i], show.routes[j]] = [show.routes[j], show.routes[i]];
  }
</script>

<div class="wrap">
  <div class="col grow">
    <div class="row">
      <h2 class="grow">Routes</h2>
      <span class="muted small">Later routes override earlier ones on the same channel. Fixture limits always apply last.</span>
      <button class="primary" onclick={() => addRoute()}>Add route</button>
    </div>
    <div class="row toolbar">
      <label>Group by</label>
      <select bind:value={groupBy}>{#each GROUP_BY as g}<option value={g.id}>{g.label}</option>{/each}</select>
      <label>Sort</label>
      <select bind:value={sortBy}>
        <option value="order">Run order</option><option value="name">Name</option><option value="source">Source</option>
      </select>
      <input class="grow" bind:value={search} placeholder="Search routes, fixtures, channels…" />
      <button class="ghost" onclick={() => { for (const [k] of groups) collapsed[k] = true; }}>Collapse all</button>
      <button class="ghost" onclick={() => (collapsed = {})}>Expand all</button>
    </div>
    <datalist id="route-groups">{#each groupNames as g}<option value={g}></option>{/each}</datalist>
    {#each groups as [key, items] (key)}
      {@const on = items.filter((x) => x.r.enabled).length}
      {#if groupBy !== "none"}
        <div class="ghead">
          <button class="ghost caret" onclick={() => (collapsed[key] = !collapsed[key])}>{collapsed[key] ? "▸" : "▾"}</button>
          <span class="gname">{key}</span>
          <span class="faint small">{on}/{items.length} on</span>
          <span class="grow"></span>
          <button class="ghost small" onclick={() => setAll(items, true)}>All on</button>
          <button class="ghost small" onclick={() => setAll(items, false)}>All off</button>
          {#if groupBy === "group"}<button class="ghost small" onclick={() => addRoute(key)}>+ Route</button>{/if}
        </div>
      {/if}
      {#if !collapsed[key]}
    {#each items as { r, i } (r.id)}
      {@const drives = targets(r).length}
      <div class="panel col route" class:off={!r.enabled}>
        <div class="row">
          <input type="checkbox" bind:checked={r.enabled} title="Enabled" />
          <input class="grow" bind:value={r.name} placeholder="Route name" />
          <input class="grp" list="route-groups" bind:value={r.group} placeholder="Group" title="Group" />
          {#if sortBy === "order"}
            <button class="ghost" onclick={() => move(i, -1)} aria-label="Move up">↑</button>
            <button class="ghost" onclick={() => move(i, 1)} aria-label="Move down">↓</button>
          {/if}
          <ConfirmButton onconfirm={() => show.routes.splice(i, 1)}>Delete</ConfirmButton>
        </div>
        <div class="row flow">
          <span class="lbl">When</span>
          <select bind:value={r.source}>
            <optgroup label="Hits">{#each sources.triggers as s}<option value={s.id}>{s.label}</option>{/each}</optgroup>
            <optgroup label="Levels">{#each sources.levels as s}<option value={s.id}>{s.label}</option>{/each}</optgroup>
            {#if ![...sources.triggers, ...sources.levels].some((s) => s.id === r.source)}<option value={r.source}>{r.source} (missing)</option>{/if}
          </select>
          <span class="lbl">→</span>
          <select value={r.action.type} onchange={(e) => setAction(r, (e.target as HTMLSelectElement).value)}>
            {#each ACTIONS as a}<option value={a.type}>{a.label}</option>{/each}
          </select>
        </div>
        <TargetPicker target={r} onchange={() => retarget(r)} />
        <div class="row flow">
          {#if isEffect(r)}
            {@const n = effectTargets(r).length}
            <span class="lbl">uses</span>
            <span class="small muted">each fixture's brightness and colour channels · {n} fixture{n === 1 ? "" : "s"}</span>
          {:else if isLook(r)}
            {@const li = lookInfo(r)}
            <span class="lbl">uses</span>
            <span class="small muted">saved looks (Fixture types → Patterns) · {li.fx} fixture{li.fx === 1 ? "" : "s"} with looks · {li.matching} look{li.matching === 1 ? "" : "s"} match</span>
          {:else}
          <span class="lbl">channel</span>
          <ChannelSelect target={r} bind:channel={r.channel} />
          {/if}
          {#if isEffect(r) || isLook(r)}{:else if r.channel}
            <span class="faint small">drives {drives} fixture{drives === 1 ? "" : "s"}{r.action.type === "node_hop" ? ", hopping between nearest neighbours in 3D" : ""}</span>
          {:else}
            <span class="small warn-text">Pick a channel for this route to do anything</span>
          {/if}
        </div>
        {#if usesFocus(r)}
          {@const opts = focusOptions(r)}
          <div class="row flow">
            <span class="lbl">from</span>
            <select bind:value={r.focus} onchange={() => { if (!r.focus) r.falloff = false; }}>
              <option value="">Centre of the targets</option>
              {#each opts as o}<option value={o.id}>{o.label}</option>{/each}
              {#if r.focus && !opts.some((o) => o.id === r.focus)}<option value={r.focus}>Focus point in another room</option>{/if}
            </select>
            {#if isEffect(r)}
              <label class="row small" class:faint={!r.focus} title="Brightest at the focus point, fading to dark at its radius">
                <input type="checkbox" bind:checked={r.falloff} disabled={!r.focus} /> fade with distance
              </label>
            {/if}
            {#if !opts.length}<span class="small faint">Add focus points on the Rooms screen to centre effects on a spot.</span>{/if}
          </div>
        {/if}
        <div class="row flow params">
          {#if r.action.type === "flash" || r.action.type === "node_hop"}
            <label>On</label><input type="number" min="0" max="255" bind:value={r.action.on} />
            <label>Off</label><input type="number" min="0" max="255" bind:value={r.action.off} />
            <label>Fade</label><input type="number" min="0" step="50" bind:value={r.action.decay_ms} /> ms
            {#if r.action.type === "node_hop"}<label>Colour</label><PaletteSelect bind:value={r.action.palette} allowNone />{/if}
          {:else if r.action.type === "look"}
            {@const li = lookInfo(r)}
            <label>Set</label>
            <select bind:value={r.action.set}>
              <option value="">Any set</option>
              {#each li.sets as [id, name]}<option value={id}>{name}</option>{/each}
            </select>
            <label>Needs tags</label><Tags bind:tags={r.action.tags} suggestions={li.tags} />
            <label class="row"><input type="checkbox" bind:checked={r.action.include_defaults} /> include defaults</label>
            <label>Pick</label>
            <select bind:value={r.action.mode}><option value="random">Random (by rating)</option><option value="cycle">In order</option></select>
            <label>Hold</label><input type="number" min="0" step="100" bind:value={r.action.hold_ms} /> ms
            {#if li.heads.length}
              {@const act = r.action}
              <label>Apply to</label>
              <button class="chip" class:on={!act.heads.length} onclick={() => (act.heads = [])}>Head it was saved on</button>
              {#each li.heads as h}
                <button class="chip" class:on={act.heads.includes(h)} onclick={() => { const i = act.heads.indexOf(h); if (i >= 0) act.heads.splice(i, 1); else act.heads.push(h); }}>{h}</button>
              {/each}
            {/if}
          {:else if r.action.type === "strobe"}
            <label>Rate</label>
            <select bind:value={r.action.rate}>
              <option value="1">1 per beat</option><option value="2">2 per beat</option>
              <option value="4">4 per beat</option><option value="8">8 per beat</option><option value="free">Free (Hz)</option>
            </select>
            {#if r.action.rate === "free"}<input type="number" min="0.5" max="25" step="0.5" bind:value={r.action.hz} /> Hz{/if}
            <label>On time</label><input type="range" min="0.05" max="0.9" step="0.05" bind:value={r.action.duty} />
            {#if isTriggerSource(r.source, show.audio.bands)}
              <label>Burst</label><input type="number" min="50" step="50" bind:value={r.action.burst_ms} /> ms
            {:else}
              <label>Above</label><input type="range" min="0" max="1" step="0.01" bind:value={r.action.threshold} /><span class="mono small">{Math.round(r.action.threshold * 100)}%</span>
            {/if}
          {:else if r.action.type === "palette"}
            <label>Palette</label><PaletteSelect bind:value={r.action.palette} />
            <label>Mode</label>
            <select bind:value={r.action.mode}>
              <option value="all">All the same</option>
              <option value="spread">Spread left → right</option>
              <option value="rings">Rings out from focus</option>
              <option value="around">Slices around focus</option>
              <option value="random">Random per fixture</option>
            </select>
            <label>Crossfade</label><input type="number" min="0" step="50" bind:value={r.action.fade_ms} /> ms
            <label>Brightness</label><input type="number" min="0" max="255" bind:value={r.action.brightness} title="0 leaves brightness to other routes" />
          {:else if r.action.type === "twinkle"}
            <label>Palette</label><PaletteSelect bind:value={r.action.palette} />
            <label>{isTriggerSource(r.source, show.audio.bands) ? "Sparkles per hit" : "Sparkles/sec at full level"}</label><input type="number" min="1" max="60" bind:value={r.action.amount} />
            <label>Fade</label><input type="number" min="50" step="50" bind:value={r.action.decay_ms} /> ms
            <label>Base</label><input type="number" min="0" max="255" bind:value={r.action.base} />
          {:else if r.action.type === "wave"}
            <label>Palette</label><PaletteSelect bind:value={r.action.palette} />
            <label>Direction</label>
            <select bind:value={r.action.axis}>
              <option value="x">Left → right</option>
              <option value="y">Front → back</option>
              <option value="radial">Out from focus</option>
              <option value="angle">Around focus (rotating)</option>
              <option value="height">Low → high</option>
            </select>
            <label>Beats per sweep</label><input type="number" min="0.25" step="0.25" bind:value={r.action.beats} />
            <label>Width</label><input type="range" min="0.05" max="1" step="0.05" bind:value={r.action.width} />
            <label>Base</label><input type="number" min="0" max="255" bind:value={r.action.base} />
          {:else if r.action.type === "random"}
            <label>Needs tags</label><Tags bind:tags={r.action.tags} />
            <label>Hold</label><input type="number" min="0" step="100" bind:value={r.action.hold_ms} /> ms
          {:else if r.action.type === "modulate"}
            <label>Min</label><input type="number" min="0" max="255" bind:value={r.action.min} />
            <label>Max</label><input type="number" min="0" max="255" bind:value={r.action.max} />
            <label>Smoothing</label><input type="range" min="0" max="0.98" step="0.01" bind:value={r.action.smoothing} />
          {:else if r.action.type === "cycle"}
            {@const act = r.action}
            <label>Values</label>
            <input class="grow" value={act.values.join(", ")}
              onchange={(e) => (act.values = (e.target as HTMLInputElement).value.split(/[,\s]+/).map(Number).filter((n) => n >= 0 && n <= 255))} />
          {/if}
          {#if (r.action.type !== "modulate" && r.action.type !== "wave") || isTriggerSource(r.source, show.audio.bands)}
            <span class="sep"></span>
            <label>Every</label><input type="number" min="1" bind:value={r.every} />
            <label>Cooldown</label><input type="number" min="0" step="50" bind:value={r.cooldown_ms} /> ms
            <label>Chance</label><input type="number" min="0" max="1" step="0.05" bind:value={r.probability} />
          {/if}
        </div>
        {#each [heldBy(r)] as held}
          {#if held.length}
            <div class="row small held">
              <span class="grow">
                {held.map((h) => `${h.f.name}: ${h.n} channel${h.n === 1 ? "" : "s"}`).join(", ")} under manual control. Routes can't change
                manually held channels.
              </span>
              <button onclick={() => held.forEach((h) => clearManual(h.f.id))}>Release</button>
            </div>
          {/if}
        {/each}
        <div class="small faint">{HELP[r.action.type]}</div>
      </div>
    {/each}
      {/if}
    {:else}
      <div class="empty panel">{search ? "No routes match your search." : "No routes yet. A route connects an audio source to a fixture channel."}</div>
    {/each}
  </div>
  <div class="col side">
    <div class="panel"><h3 style="margin-bottom:8px">Live sources</h3><Meters /></div>
    <div class="panel"><PaletteEditor /></div>
  </div>
</div>

<style>
  .wrap { display: flex; gap: 16px; align-items: flex-start; }
  .side { width: 260px; flex-shrink: 0; position: sticky; top: 0; }
  .route.off { opacity: 0.55; }
  .toolbar { flex-wrap: wrap; }
  .toolbar label { color: var(--faint); }
  .ghead { display: flex; align-items: center; gap: 8px; margin-top: 8px; padding: 4px 2px; border-bottom: 1px solid var(--line); }
  .gname { font-weight: 600; }
  .caret { padding: 0 6px; }
  .ghead .small { font-size: 11px; padding: 2px 6px; }
  .grp { width: 130px; }
  .held { background: var(--warn-bg); border: 1px solid #f2b54a55; border-radius: 6px; padding: 6px 8px; color: var(--warn); }
  .chip { font-size: 12px; padding: 3px 8px; }
  .chip.on { background: var(--accent-bg); border-color: var(--accent); }
  .warn-text { color: var(--warn); }
  .flow { flex-wrap: wrap; }
  .lbl { color: var(--faint); }
  .params label { margin-left: 4px; }
  .params input[type="number"] { width: 64px; }
  .sep { width: 1px; height: 20px; background: var(--line); margin: 0 6px; }
</style>
