<script lang="ts">
  import Tags from "../components/Tags.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import TargetPicker from "../components/TargetPicker.svelte";
  import ChannelSelect from "../components/ChannelSelect.svelte";
  import Meters from "../components/Meters.svelte";
  import { app, uid, modeChannels, profileOf, routeFixtures, fixtureHasChannel, clearManual } from "../lib/store.svelte";
  import { missingBand } from "../lib/genres";
  import { SECTIONS } from "../lib/autopilot";
  import { sourceOptions, isTriggerSource, GROUP_LABEL, EFFECT_ACTIONS, newMotion, describeMotion, type ChannelDef, type Route, type RouteAction } from "../lib/types";
  import MotionEditor from "../components/MotionEditor.svelte";
  import Tip from "../components/Tip.svelte";
  import { groupLooks } from "../lib/looks";
  import type { RouteSwitch, Section } from "../lib/types";
  import PaletteSelect from "../components/PaletteSelect.svelte";
  import PaletteEditor from "../components/PaletteEditor.svelte";

  const show = app.show!;
  const sources = $derived(sourceOptions(show.audio.bands));
  const orphans = $derived(show.routes.filter((r) => missingBand(show, r.source)));

  const ACTIONS: { type: RouteAction["type"]; label: string; make: () => RouteAction }[] = [
    { type: "flash", label: "Flash", make: () => ({ type: "flash", on: 255, off: 0, decay_ms: 250 }) },
    { type: "random", label: "Random value", make: () => ({ type: "random", tags: [], hold_ms: 500 }) },
    { type: "node_hop", label: "Node hop", make: () => ({ type: "node_hop", on: 255, off: 0, decay_ms: 300, palette: "" }) },
    { type: "modulate", label: "Modulate", make: () => ({ type: "modulate", min: 0, max: 255, smoothing: 0.6 }) },
    { type: "cycle", label: "Cycle values", make: () => ({ type: "cycle", values: [0, 64, 128, 192] }) },
    { type: "motion", label: "Move (sweep, step, random…)", make: () => ({ type: "motion", motion: newMotion("sweep"), spread: 0 }) },
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
    const fitsLook = (l: (typeof profiles)[number]["looks"][number]) => !!a && (!a.set || l.sets.includes(a.set)) && (a.include_defaults || !l.builtin) && a.tags.every((t) => l.tags.includes(t));
    const fits = profiles.flatMap((p) => p.looks).filter(fitsLook);
    // grouped under the pattern value each look is built on
    const groups = groupLooks(profiles.flatMap((p) => p.looks.filter(fitsLook).map((look) => ({ profile: p, look }))));
    const names = [...new Set(fits.map((l) => l.name))];
    // looks are picked by id (names can repeat across patterns); older routes picked by name
    const chosen = a?.looks?.length ? fits.filter((l) => a.looks!.includes(l.id) || a.looks!.includes(l.name)) : fits;
    const sectionTagged = SECTIONS.filter((s) => chosen.some((l) => l.tags.includes(s.id))).map((s) => s.name);
    const tags = [...new Set(profiles.flatMap((p) => p.looks.flatMap((l) => l.tags)))].sort();
    return { fx: fx.length, sets: [...sets.entries()], heads: [...heads], matching: new Set(chosen.map((l) => `${l.name}|${JSON.stringify(l.values)}`)).size, names, groups, sectionTagged, tags };
  }
  /** The channel a route drives, as defined on its first target that has it (for the motion editor). */
  function channelDefFor(r: Route, key = r.channel): ChannelDef {
    for (const f of routeFixtures(r)) {
      const p = profileOf(f);
      const keys = modeChannels(f);
      const c = key.startsWith("@") ? keys.find((c) => c.group === key.slice(1)) : keys.find((c) => c.key === key);
      if (p && c) return c;
    }
    return { key, name: key.startsWith("@") ? `${key.slice(1)} channels` : key, group: "other", default: 0, capabilities: [], values: [] } as unknown as ChannelDef;
  }
  /** Channels a route can also move: those of its fixtures, plus "every <type> channel". */
  function moveChannels(r: Route): ChannelDef[] {
    const seen = new Map<string, ChannelDef>();
    const groups = new Set<string>();
    for (const f of routeFixtures(r)) for (const c of modeChannels(f)) {
      if (c.group === "mode") continue;
      groups.add(c.group);
      if (!seen.has(c.key)) seen.set(c.key, c);
    }
    const types = [...groups].map((g) => ({ ...channelDefFor(r, `@${g}`), key: `@${g}`, name: `All ${GROUP_LABEL[g as keyof typeof GROUP_LABEL] ?? g} channels` }) as ChannelDef);
    return [...types, ...seen.values()];
  }
  let openAuto = $state<Record<string, string>>({});
  /** A group's looks as chips: one per name within the pattern, with the ids it stands for
   *  (the same look on several fixture types has an id on each). */
  function lookChips(g: { looks: { id: string; name: string; rating: number; tags: string[] }[] }) {
    const m = new Map<string, { name: string; ids: string[]; rating: number; tags: string[] }>();
    for (const l of g.looks) {
      const c = m.get(l.name) ?? { name: l.name, ids: [], rating: l.rating, tags: l.tags };
      c.ids.push(l.id);
      m.set(l.name, c);
    }
    return [...m.values()];
  }
  /** Is this look picked? By id, or by name on routes saved before looks were picked by id. */
  const lookOn = (picked: string[] | undefined, ids: string[], name: string) => !!picked?.some((x) => ids.includes(x) || x === name);
  /** Pick or drop looks by id; drops any old by-name entry for them too. */
  function setLooks(r: Route, ids: string[], names: string[], on: boolean) {
    if (r.action.type !== "look") return;
    const rest = (r.action.looks ?? []).filter((x) => !ids.includes(x) && !names.includes(x));
    r.action.looks = on ? [...rest, ...ids] : rest;
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
    motion: "Keeps the channel moving like a look's moving channel: sweep back and forth, step, seeded random, a list, catalogued values, or follow a level. Timed in beats, seconds, or this route's hits (with Every / Cooldown / Chance). Spread offsets each fixture so they chase instead of moving together.",
    look: "On each trigger apply a saved look from the pattern browser: a pattern plus its secondary options and motions. Pick from a set, by tags, or name the looks. Follow the section uses looks tagged breakdown/groove/build/drop for the part of the track playing. Timing: the trigger (e.g. Bar with Every 4 = every 4 bars), hold at least, change after at most, rests, cooldown and chance. Looks play on the head they were saved on; Plays on can move them to another head.",
    strobe: "Hit sources: strobe for the burst time after each hit. Level sources: strobe while the level is above the threshold. Beat-locked rates follow the BPM.",
    palette: "Each trigger steps to the palette's next colour. Spread, rings and slices give fixtures different colours by position, so the colours travel across the room, outward from the focus, or around it on each hit.",
    twinkle: "Random fixtures sparkle in palette colours, clustering near the focus point if one is set. Level sources: more sparkles when louder. Hit sources: a burst of sparkles per hit.",
    wave: "A band of light sweeps across the room in time with the beat (or ripples out from / rotates around the focus), changing colour each sweep. A level source scales its brightness.",
  };

  /** Hover help for each field. */
  const TIP = {
    when: "The sound that drives this route. Hits (kick, snare, beat, bar…) fire once per hit; levels (bass level, energy…) are a smooth 0–100% that follows the music, and count as a hit when they rise past 60%.",
    action: "What the route does to its fixtures when the sound fires (or continuously, for levels). The grey text under the route explains the chosen action.",
    channel: "The channel on the targets this route changes. \"All … channels\" drives every channel of that type on each fixture, whatever it's called.",
    focus: "The spot effects are calculated from: waves ripple out from it, rings and slices centre on it, node hop starts near it. Add focus points on the Rooms screen.",
    every: "Act on every Nth hit only. 1 = every hit, 2 = every other hit, 4 on a Beat source = once a bar.",
    cooldown: "After acting, ignore hits for this long. Stops fast sounds (hi-hats, busy kicks) from firing it too often. 0 = no wait.",
    chance: "Each time a hit gets past Every and Cooldown, the route acts only this often, at random. 100% = always. 50% = about half the hits, never in a predictable pattern. Use it for variety: a look that changes on some kicks, a strobe that only sometimes joins in. Switches, and moved channels restarting on this route's hits, follow the same hits.",
    sections: "Run this route only in these parts of a track, as heard by the autopilot's section detector (or a held section). None selected = always.",
    alsoMove: "Other channels on this route's fixtures that move as well, each with a sweep, steps, random values, a list, catalogued values or following a level. They can restart on each hit of this route, or on a sound of their own, without making a separate route. A moved channel replaces what the action set for it.",
    switches: "Each time this route acts (after Every, Cooldown and Chance), switch other routes on, off or over (toggle). For a number of beats, then back to how they were, or 0 = until something else changes them. Nothing is saved: a scene, a pad or you switching the route by hand ends it.",
    switchFor: "How long the switch lasts, in beats at the current tempo. 0 = until another switch, a scene, a pad or you change that route.",
    on: "The value the channel jumps to when the route fires.",
    off: "The value it fades back to.",
    fade: "How long the fade from On back to Off takes.",
    lookSet: "Only use looks in this set (sets are made in the pattern browser).",
    lookTags: "Only use looks that carry all these tags.",
    lookDefaults: "Also use the fixture's built-in looks, not just ones you saved.",
    lookPick: "Random picks weighted by your star ratings; In order steps through them, one per change.",
    lookOnly: "Limit the route to these looks. They're grouped under the pattern each was made on: click a pattern to pick all of its looks, or pick looks one by one. \"Any of these\" uses every look that matches the set and tags.",
    hold: "Keep a look at least this long before another hit can change it.",
    maxTime: "Change look after this long even without a hit, so a quiet stretch doesn't freeze on one look. 0 = only on hits.",
    rest: "Chance that a change shows no look at all: the fixture falls back to the scene's look or its defaults, giving the room a breather.",
    restFor: "How long a rest lasts. 0 = until the next hit changes it.",
    heads: "Which projector a look plays on. Normally the one it was saved on in the pattern browser. Pick a head to play every look there (a P1 look is copied onto P2's matching channels), or every head to mirror it. A look only sets the channels it has ticked, so a head's on/off channel (P2 On/Off) stays yours: switch it with a pad, a route, a scene value or by hand.",
    rate: "Flashes per beat, locked to the BPM, or a free rate in flashes per second.",
    duty: "How much of each flash is on. Low = short sharp flashes.",
    burst: "How long the strobe runs after each hit.",
    above: "Strobe while the level is above this.",
    paletteMode: "How colours are spread over the fixtures: all the same, travelling across the room, rings out from the focus, slices around it, or random per fixture.",
    crossfade: "How long each colour change fades over. 0 = instant.",
    brightness: "Brightness the colour is shown at. 0 leaves brightness to other routes.",
    sparkles: "How many fixtures sparkle per hit, or per second at full level.",
    base: "Brightness of fixtures that aren't lit by the effect.",
    beats: "How many beats one sweep across the room takes.",
    width: "How wide the band of light is, as a share of the room.",
    randomHold: "Keep each random value at least this long.",
    min: "Value at silence (level 0%).",
    max: "Value at full level (100%).",
    smoothing: "Higher = slower, smoother movement; lower = follows the level tightly.",
    cycle: "The values to step through, one per hit, then back to the first.",
    spread: "Offset each fixture along the motion so they chase each other instead of moving together. 100% spreads them over one whole cycle.",
  };
  const pct = (n: number) => `${Math.round(n * 100)}%`;
  const ordinal = (n: number) => `${n}${n % 100 >= 11 && n % 100 <= 13 ? "th" : ["th", "st", "nd", "rd"][n % 10] ?? "th"}`;
  /** Plain-English timing: "Acts on every 2nd kick hit, at most once every 0.5 s, about half the time." */
  function timingText(r: Route) {
    const src = sourceLabel(r.source).toLowerCase();
    const hit = isTriggerSource(r.source, show.audio.bands) ? src : `${src} rising past 60%`;
    const every = r.every > 1 ? `every ${ordinal(r.every)} ${hit}` : `every ${hit}`;
    const cool = r.cooldown_ms > 0 ? `, at most once every ${r.cooldown_ms >= 1000 ? `${+(r.cooldown_ms / 1000).toFixed(1)} s` : `${r.cooldown_ms} ms`}` : "";
    const p = r.probability > 0 && r.probability < 1 ? `, ${r.probability >= 0.45 && r.probability <= 0.55 ? "about half the time" : `${pct(r.probability)} of the time`} (at random)` : "";
    return `Acts on ${every}${cool}${p}.`;
  }
  // ----- switching other routes -----
  const routeName = (id: string) => { const o = show.routes.find((x) => x.id === id); return o ? o.name || `${sourceLabel(o.source)} → ${o.action.type}` : "(deleted route)"; };
  function addSwitch(r: Route, id: string) {
    if (!id) return;
    // (assign, then read back: pushing onto the `??=` result would miss the reactive copy)
    r.switches = [...(r.switches ?? []), { route: id, to: "on", beats: 8 } satisfies RouteSwitch];
  }
  const switchText = (sw: RouteSwitch) =>
    `${sw.to === "toggle" ? "Toggles" : sw.to === "on" ? "Switches on" : "Switches off"} “${routeName(sw.route)}” ${sw.beats ? `for ${sw.beats} beat${sw.beats === 1 ? "" : "s"}` : "until something else changes it"}`;
  function toggleSection(r: Route, id: Section) {
    const cur = new Set<string>(r.sections ?? []);
    if (cur.has(id)) cur.delete(id); else cur.add(id);
    r.sections = SECTIONS.map((x) => x.id).filter((x) => cur.has(x));
  }

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
    {#if orphans.length}
      <div class="orphans row">
        <span class="grow small"><b>{orphans.length} route{orphans.length === 1 ? "" : "s"} need{orphans.length === 1 ? "s" : ""} a new sound:</b>
          {orphans.map((r) => r.name || r.id).join(", ")}. {orphans.length === 1 ? "Its band was" : "Their bands were"} removed or renamed on the Audio screen, so {orphans.length === 1 ? "it" : "they"} never fire{orphans.length === 1 ? "s" : ""}. Pick a sound in <b>When</b>.</span>
      </div>
    {/if}
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
      <div class="panel col route" class:off={!r.enabled} class:orphan={missingBand(show, r.source)}>
        <div class="row">
          <input type="checkbox" bind:checked={r.enabled} title="Enabled" />
          <input class="grow" bind:value={r.name} placeholder="Route name" />
          {#if app.frame?.switched && r.id in app.frame.switched}
            <span class="tag live" title="Another route has switched this one for now. It goes back when the switch runs out.">switched {app.frame.switched[r.id] ? "on" : "off"} now</span>
          {/if}
          <input class="grp" list="route-groups" bind:value={r.group} placeholder="Group" title="Group" />
          {#if sortBy === "order"}
            <button class="ghost" onclick={() => move(i, -1)} aria-label="Move up">↑</button>
            <button class="ghost" onclick={() => move(i, 1)} aria-label="Move down">↓</button>
          {/if}
          <ConfirmButton onconfirm={() => show.routes.splice(i, 1)}>Delete</ConfirmButton>
        </div>
        <div class="row flow">
          <span class="lbl">When</span><Tip text={TIP.when} below />
          <select bind:value={r.source}>
            <optgroup label="Hits">{#each sources.triggers as s}<option value={s.id}>{s.label}</option>{/each}</optgroup>
            <optgroup label="Levels">{#each sources.levels as s}<option value={s.id}>{s.label}</option>{/each}</optgroup>
            {#if ![...sources.triggers, ...sources.levels].some((s) => s.id === r.source)}<option value={r.source}>{r.source} (band removed: pick another)</option>{/if}
          </select>
          <span class="lbl">→</span>
          <select value={r.action.type} onchange={(e) => setAction(r, (e.target as HTMLSelectElement).value)}>
            {#each ACTIONS as a}<option value={a.type}>{a.label}</option>{/each}
          </select>
          <Tip text={HELP[r.action.type]} below />
          <span class="grow"></span>
          <span class="lbl">plays in</span><Tip text={TIP.sections} below />
          {#each SECTIONS as sec}
            <button class="chip sec" class:on={r.sections?.includes(sec.id)} style="--c:{sec.color}" title={sec.hint} onclick={() => toggleSection(r, sec.id)}>{sec.name}</button>
          {/each}
          {#if !r.sections?.length}<span class="small faint">always</span>{/if}
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
          <span class="lbl">channel</span><Tip text={TIP.channel} />
          <ChannelSelect target={r} bind:channel={r.channel} />
          {/if}
          {#if isEffect(r) || isLook(r)}{:else if r.channel}
            <span class="faint small">drives {drives} fixture{drives === 1 ? "" : "s"}{r.action.type === "node_hop" ? ", hopping between nearest neighbours in 3D" : ""}</span>
          {:else if r.switches?.length}
            <span class="small faint">No channel: this route only switches other routes.</span>
          {:else}
            <span class="small warn-text">Pick a channel for this route to do anything (or have it switch other routes, below)</span>
          {/if}
        </div>
        {#if usesFocus(r)}
          {@const opts = focusOptions(r)}
          <div class="row flow">
            <span class="lbl">from</span><Tip text={TIP.focus} />
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
            <label>On</label><Tip text={TIP.on} /><input type="number" min="0" max="255" bind:value={r.action.on} />
            <label>Off</label><Tip text={TIP.off} /><input type="number" min="0" max="255" bind:value={r.action.off} />
            <label>Fade</label><Tip text={TIP.fade} /><input type="number" min="0" step="50" bind:value={r.action.decay_ms} /> ms
            {#if r.action.type === "node_hop"}<label>Colour</label><PaletteSelect bind:value={r.action.palette} allowNone />{/if}
          {:else if r.action.type === "look"}
            {@const li = lookInfo(r)}
            <label>Set</label><Tip text={TIP.lookSet} />
            <select bind:value={r.action.set}>
              <option value="">Any set</option>
              {#each li.sets as [id, name]}<option value={id}>{name}</option>{/each}
            </select>
            <label>Needs tags</label><Tip text={TIP.lookTags} /><Tags bind:tags={r.action.tags} suggestions={li.tags} />
            <label class="row"><input type="checkbox" bind:checked={r.action.include_defaults} /> include defaults</label><Tip text={TIP.lookDefaults} />
            <label>Pick</label><Tip text={TIP.lookPick} />
            <select bind:value={r.action.mode}><option value="random">Random (by rating)</option><option value="cycle">In order</option></select>
            <span class="small faint">{li.matching} look{li.matching === 1 ? "" : "s"}</span>
            {#if li.names.length}
              {@const act = r.action}
              <div class="looksel row">
                <label>Only</label><Tip text={TIP.lookOnly} />
                <button class="chip" class:on={!act.looks?.length} onclick={() => (act.looks = [])}>Any of these</button>
              </div>
              <div class="lookgroups">
                {#each li.groups as g (g.id)}
                  {@const chips = lookChips(g)}
                  {@const allIds = chips.flatMap((c) => c.ids)}
                  {@const all = chips.every((c) => lookOn(act.looks, c.ids, c.name))}
                  <div class="lg row">
                    <button class="ghost lgh" class:on={all} title={all ? "Leave out every look on this pattern" : "Use every look on this pattern"}
                      onclick={() => setLooks(r, allIds, chips.map((c) => c.name), !all)}>
                      <span class="faint">{g.channel}</span> <b class="mono">{g.value}</b> {g.label}
                    </button>
                    {#each chips as c (c.name)}
                      {@const on = lookOn(act.looks, c.ids, c.name)}
                      <button class="chip" class:on title={c.tags.join(", ")} onclick={() => setLooks(r, c.ids, [c.name], !on)}>{c.name}{c.rating ? ` ${"★".repeat(c.rating)}` : ""}</button>
                    {/each}
                  </div>
                {/each}
              </div>
            {/if}
            <label class="row" title="Prefer looks tagged breakdown, groove, build or drop for the section playing, and change look when the section changes">
              <input type="checkbox" checked={!!r.action.follow_section} onchange={(e) => { if (r.action.type === "look") r.action.follow_section = e.currentTarget.checked; }} /> follow the section
            </label>
            {#if r.action.follow_section}
              <span class="small faint">{li.sectionTagged.length ? `tagged for ${li.sectionTagged.join(", ")}` : "tag looks breakdown / groove / build / drop in the pattern browser"}</span>
            {/if}
            <span class="sep"></span>
            <label>Hold at least</label><Tip text={TIP.hold} /><input type="number" min="0" step="100" bind:value={r.action.hold_ms} /> ms
            <label>Change after at most</label><Tip text={TIP.maxTime} />
            <input type="number" min="0" step="1" value={Math.round((r.action.max_ms ?? 0) / 1000)} onchange={(e) => { if (r.action.type === "look") r.action.max_ms = Math.max(0, +e.currentTarget.value) * 1000; }} /> s
            <label>Rest</label><Tip text={TIP.rest} />
            <input type="number" min="0" max="90" step="5" value={Math.round((r.action.rest ?? 0) * 100)} onchange={(e) => { if (r.action.type === "look") r.action.rest = Math.min(0.9, Math.max(0, +e.currentTarget.value / 100)); }} /> %
            {#if r.action.rest}
              <label>for</label><Tip text={TIP.restFor} />
              <input type="number" min="0" step="1" value={Math.round((r.action.rest_ms ?? 0) / 1000)} onchange={(e) => { if (r.action.type === "look") r.action.rest_ms = Math.max(0, +e.currentTarget.value) * 1000; }} /> s
              <span class="small faint">(0 = until the next hit)</span>
            {/if}
            {#if li.heads.length}
              {@const act = r.action}
              <label>Plays on</label><Tip text={TIP.heads} />
              <select value={!act.heads.length ? "" : act.heads.length >= li.heads.length ? "*" : act.heads[0]}
                onchange={(e) => { const v = e.currentTarget.value; act.heads = v === "" ? [] : v === "*" ? [...li.heads] : [v]; }}>
                <option value="">the head each look was saved on</option>
                {#each li.heads as h}<option value={h}>{h} only</option>{/each}
                <option value="*">every head ({li.heads.join(" + ")})</option>
              </select>
            {/if}
          {:else if r.action.type === "strobe"}
            <label>Rate</label><Tip text={TIP.rate} />
            <select bind:value={r.action.rate}>
              <option value="1">1 per beat</option><option value="2">2 per beat</option>
              <option value="4">4 per beat</option><option value="8">8 per beat</option><option value="free">Free (Hz)</option>
            </select>
            {#if r.action.rate === "free"}<input type="number" min="0.5" max="25" step="0.5" bind:value={r.action.hz} /> Hz{/if}
            <label>On time</label><Tip text={TIP.duty} /><input type="range" min="0.05" max="0.9" step="0.05" bind:value={r.action.duty} />
            {#if isTriggerSource(r.source, show.audio.bands)}
              <label>Burst</label><Tip text={TIP.burst} /><input type="number" min="50" step="50" bind:value={r.action.burst_ms} /> ms
            {:else}
              <label>Above</label><Tip text={TIP.above} /><input type="range" min="0" max="1" step="0.01" bind:value={r.action.threshold} /><span class="mono small">{Math.round(r.action.threshold * 100)}%</span>
            {/if}
          {:else if r.action.type === "palette"}
            <label>Palette</label><PaletteSelect bind:value={r.action.palette} />
            <label>Mode</label><Tip text={TIP.paletteMode} />
            <select bind:value={r.action.mode}>
              <option value="all">All the same</option>
              <option value="spread">Spread left → right</option>
              <option value="rings">Rings out from focus</option>
              <option value="around">Slices around focus</option>
              <option value="random">Random per fixture</option>
            </select>
            <label>Crossfade</label><Tip text={TIP.crossfade} /><input type="number" min="0" step="50" bind:value={r.action.fade_ms} /> ms
            <label>Brightness</label><Tip text={TIP.brightness} /><input type="number" min="0" max="255" bind:value={r.action.brightness} />
          {:else if r.action.type === "twinkle"}
            <label>Palette</label><PaletteSelect bind:value={r.action.palette} />
            <label>{isTriggerSource(r.source, show.audio.bands) ? "Sparkles per hit" : "Sparkles/sec at full level"}</label><Tip text={TIP.sparkles} /><input type="number" min="1" max="60" bind:value={r.action.amount} />
            <label>Fade</label><Tip text={TIP.fade} /><input type="number" min="50" step="50" bind:value={r.action.decay_ms} /> ms
            <label>Base</label><Tip text={TIP.base} /><input type="number" min="0" max="255" bind:value={r.action.base} />
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
            <label>Beats per sweep</label><Tip text={TIP.beats} /><input type="number" min="0.25" step="0.25" bind:value={r.action.beats} />
            <label>Width</label><Tip text={TIP.width} /><input type="range" min="0.05" max="1" step="0.05" bind:value={r.action.width} />
            <label>Base</label><Tip text={TIP.base} /><input type="number" min="0" max="255" bind:value={r.action.base} />
          {:else if r.action.type === "random"}
            <label>Needs tags</label><Tags bind:tags={r.action.tags} />
            <label>Hold</label><Tip text={TIP.randomHold} /><input type="number" min="0" step="100" bind:value={r.action.hold_ms} /> ms
          {:else if r.action.type === "modulate"}
            <label>Min</label><Tip text={TIP.min} /><input type="number" min="0" max="255" bind:value={r.action.min} />
            <label>Max</label><Tip text={TIP.max} /><input type="number" min="0" max="255" bind:value={r.action.max} />
            <label>Smoothing</label><Tip text={TIP.smoothing} /><input type="range" min="0" max="0.98" step="0.01" bind:value={r.action.smoothing} />
          {:else if r.action.type === "motion"}
            {@const act = r.action}
            <div class="autos col">
              <MotionEditor motion={act.motion} channel={channelDefFor(r)} />
              <div class="row">
                <label>Spread across fixtures</label><Tip text={TIP.spread} />
                <input type="range" min="0" max="1" step="0.05" bind:value={act.spread} />
                <span class="mono small">{Math.round(act.spread * 100)}%</span>
                <span class="small faint">{describeMotion(act.motion)}</span>
              </div>
            </div>
          {:else if r.action.type === "cycle"}
            {@const act = r.action}
            <label>Values</label><Tip text={TIP.cycle} />
            <input class="grow" value={act.values.join(", ")}
              onchange={(e) => (act.values = (e.target as HTMLInputElement).value.split(/[,\s]+/).map(Number).filter((n) => n >= 0 && n <= 255))} />
          {/if}
          {#if (r.action.type !== "modulate" && r.action.type !== "wave") || isTriggerSource(r.source, show.audio.bands)}
            <div class="timing row flow">
              <label>Every</label><Tip text={TIP.every} /><input type="number" min="1" bind:value={r.every} />
              <label>Cooldown</label><Tip text={TIP.cooldown} /><input type="number" min="0" step="50" bind:value={r.cooldown_ms} /> ms
              <label>Chance</label><Tip text={TIP.chance} />
              <input type="number" min="1" max="100" step="5" value={Math.round((r.probability > 0 ? r.probability : 1) * 100)}
                onchange={(e) => (r.probability = Math.min(1, Math.max(0.01, +e.currentTarget.value / 100)))} /> %
              <span class="small faint">{timingText(r)}</span>
            </div>
          {/if}
          <div class="also col">
            <div class="row"><b class="small">Also do</b><span class="small faint">on top of the action, without a separate route</span></div>
            <div class="row flow">
              <label>Move channels</label><Tip text={TIP.alsoMove} />
              {#each Object.entries(r.motions ?? {}) as [k, m] (k)}
                <button class="chip" class:on={openAuto[r.id] === k} onclick={() => (openAuto[r.id] = openAuto[r.id] === k ? "" : k)}
                  title={describeMotion(m)}>{channelDefFor(r, k).name}: {m.kind}{m.retrigger ? " on hit" : ""}</button>
              {/each}
              {#if moveChannels(r).length}
                <select onchange={(e) => { const k = e.currentTarget.value; e.currentTarget.value = ""; if (!k) return; r.motions ??= {}; r.motions[k] ??= { ...newMotion("sweep"), retrigger: isTriggerSource(r.source, show.audio.bands) }; openAuto[r.id] = k; }}>
                  <option value="">+ a channel…</option>
                  {#each moveChannels(r).filter((c) => !r.motions?.[c.key]) as c}<option value={c.key}>{c.name}</option>{/each}
                </select>
              {/if}
            </div>
            {#if openAuto[r.id] && r.motions?.[openAuto[r.id]]}
              {@const k = openAuto[r.id]}
              <div class="row"><span class="small faint grow">{channelDefFor(r, k).name}: {describeMotion(r.motions[k])}</span>
                <button class="ghost small" onclick={() => { delete r.motions![k]; openAuto[r.id] = ""; }}>Remove</button></div>
              <MotionEditor motion={r.motions[k]} channel={channelDefFor(r, k)} inRoute routeSource={r.source} />
            {/if}
            <div class="row flow">
              <label>Switch routes</label><Tip text={TIP.switches} />
              <select onchange={(e) => { addSwitch(r, e.currentTarget.value); e.currentTarget.value = ""; }}>
                <option value="">+ a route…</option>
                {#each show.routes.filter((o) => o.id !== r.id && !r.switches?.some((x) => x.route === o.id)) as o}<option value={o.id}>{routeName(o.id)}{o.group ? ` (${o.group})` : ""}</option>{/each}
              </select>
            </div>
            {#each r.switches ?? [] as sw, si}
              <div class="row flow sw">
                <select bind:value={sw.to}><option value="on">Switch on</option><option value="off">Switch off</option><option value="toggle">Toggle</option></select>
                <select bind:value={sw.route}>
                  {#each show.routes.filter((o) => o.id !== r.id) as o}<option value={o.id}>{routeName(o.id)}</option>{/each}
                  {#if !show.routes.some((o) => o.id === sw.route)}<option value={sw.route}>(deleted route)</option>{/if}
                </select>
                <label>for</label><input type="number" min="0" step="1" bind:value={sw.beats} /> beats<Tip text={TIP.switchFor} />
                <button class="ghost small" onclick={() => (r.switches = r.switches!.filter((_, j) => j !== si))} aria-label="Remove switch">×</button>
                <span class="small faint swtext">{switchText(sw)}, each time this route acts.</span>
              </div>
            {/each}
          </div>
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
  .orphans { padding: 8px 10px; border: 1px solid var(--warn); border-radius: 8px; background: var(--warn-bg); }
  .route.orphan { border-color: var(--warn); }
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
  .looksel { flex-wrap: wrap; gap: 4px; flex-basis: 100%; }
  .lookgroups { flex-basis: 100%; display: flex; flex-direction: column; gap: 3px; max-height: 260px; overflow: auto; padding-left: 8px; border-left: 2px solid var(--line-2); }
  .lg { flex-wrap: wrap; gap: 4px; align-items: center; }
  .lgh { font-size: 12px; padding: 2px 6px; min-width: 180px; text-align: left; }
  .lgh.on { color: var(--accent-2); }
  .autos { flex-basis: 100%; gap: 4px; }
  .also { flex-basis: 100%; gap: 4px; margin-top: 4px; padding-top: 6px; border-top: 1px dashed var(--line); }
  .also label { margin-left: 0; }
  .sw { padding-left: 12px; gap: 4px; }
  .swtext { flex-basis: 100%; }
  .timing input[type="number"] { width: 72px; }
  .timing { flex-basis: 100%; gap: 4px; }
  .timing label { margin-left: 0; }
  .chip.sec { font-size: 11px; padding: 2px 7px; color: var(--muted); }
  .chip.sec.on { background: color-mix(in srgb, var(--c) 30%, transparent); border-color: var(--c); color: var(--text); }
  .tag.live { background: var(--accent-bg); border-color: var(--accent); }
  .sep { width: 1px; height: 20px; background: var(--line); margin: 0 6px; }
</style>
