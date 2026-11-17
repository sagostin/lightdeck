<script lang="ts">
  // A scene's building blocks: what each block drives, what sound triggers it, colours and
  // speed, which sections it plays in, and what changes in some sections.
  import { ALL_SECTIONS, BLOCKS, analyseRig, available, blockDef, newBlock, soundOptions, targetOptions, type BlockUse, type Speed, type Variant } from "../lib/blocks";
  import { SECTIONS, sectionInfo } from "../lib/autopilot";
  import { app } from "../lib/store.svelte";
  import type { Section } from "../lib/types";

  let { blocks = $bindable() }: { blocks: BlockUse[] } = $props();
  const show = app.show!;
  const rig = $derived(analyseRig(show));
  const SPEEDS: Speed[] = ["slow", "medium", "fast"];
  const NEEDS_LABEL = { glow: "Colour fixtures", laser: "Lasers", fog: "Fog" } as const;
  const missing = { glow: "no colour fixtures", laser: "no lasers with saved looks", fog: "no fog machine" } as const;
  let open = $state<Record<number, boolean>>({});

  function add(sel: HTMLSelectElement) {
    if (sel.value) blocks.push(newBlock(sel.value));
    sel.value = "";
  }
  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= blocks.length) return;
    [blocks[i], blocks[j]] = [blocks[j], blocks[i]];
  }
  const paletteOptions = $derived([
    { id: "calm", label: "Calm colours" }, { id: "hot", label: "Hot colours" },
    ...show.palettes.map((p) => ({ id: p.id, label: p.name })),
  ]);
  const tagOptions = $derived([
    { id: "", label: "Any look" }, { id: "calm", label: `Calm looks${rig.calmTag ? ` (${rig.calmTag})` : ""}` },
    { id: "peak", label: `Peak looks${rig.peakTag ? ` (${rig.peakTag})` : ""}` },
    ...rig.tags.filter((t) => t !== rig.calmTag && t !== rig.peakTag && t !== "default").map((t) => ({ id: t, label: `Tagged ${t}` })),
  ]);
  const sounds = (kind: "hit" | "level") => {
    const extra = kind === "hit" ? [{ id: "kick", label: "Kick hit" }, { id: "snare", label: "Snare hit" }, { id: "hat", label: "Hi-hat hit" }] : [{ id: "vocals", label: "Vocals level" }];
    return [...extra, ...soundOptions(show, kind).filter((o) => !extra.some((x) => x.id === o.id || `${x.id}.level` === o.id))];
  };

  // ---- sections ----
  const plays = (u: BlockUse) => (u.sections?.length ? u.sections : ALL_SECTIONS);
  function togglePlays(u: BlockUse, s: Section) {
    const cur = new Set(plays(u));
    if (cur.has(s)) cur.delete(s); else cur.add(s);
    if (!cur.size) return; // a block that never plays: remove it instead
    u.sections = cur.size === 4 ? [] : ALL_SECTIONS.filter((x) => cur.has(x));
    if (u.variants) for (const k of Object.keys(u.variants) as Section[]) if (!cur.has(k)) delete u.variants[k];
  }
  const variant = (u: BlockUse, s: Section): Variant => u.variants?.[s] ?? {};
  function setVariant(u: BlockUse, s: Section, key: keyof Variant, value: string) {
    u.variants ??= {};
    const v = { ...variant(u, s) } as Record<string, string>;
    if (value === "") delete v[key]; else v[key] = value;
    if (Object.keys(v).length) u.variants[s] = v as Variant; else delete u.variants[s];
  }
  const variedCount = (u: BlockUse) => Object.values(u.variants ?? {}).filter((v) => v && Object.keys(v).length).length;
  function playsLabel(u: BlockUse) {
    const p = plays(u);
    return p.length === 4 ? "always" : p.map((s) => sectionInfo(s)?.name).join(", ");
  }
</script>

<div class="blocks col">
  {#each blocks as u, bi (bi)}
    {@const d = blockDef(u.block)}
    {#if d}
      <div class="block col" class:off={!available(rig, d.needs)}>
        <div class="row">
          <b class="grow" title={d.description}>{d.name}</b>
          <span class="small faint">plays {playsLabel(u)}{variedCount(u) ? ` · different in ${variedCount(u)}` : ""}</span>
          <button class="ghost x" onclick={() => move(bi, -1)} disabled={bi === 0} aria-label="Move up">↑</button>
          <button class="ghost x" onclick={() => move(bi, 1)} disabled={bi === blocks.length - 1} aria-label="Move down">↓</button>
          <button class="ghost x" onclick={() => blocks.splice(bi, 1)} aria-label="Remove block">×</button>
        </div>
        <div class="row flow">
          <div class="seg">
            {#each SPEEDS as sp}<button class:sel={u.speed === sp} onclick={() => (u.speed = sp)}>{sp}</button>{/each}
          </div>
          <select bind:value={u.target} title="What it drives">
            {#each targetOptions(show, rig, d.needs) as o}<option value={o.id}>{o.label}</option>{/each}
          </select>
          {#if d.sound}
            <select bind:value={u.sound} title="What triggers it">
              {#each sounds(d.sound.kind) as o}<option value={o.id}>{o.label}</option>{/each}
            </select>
          {/if}
          {#if d.palette}
            <select bind:value={u.palette} title="Colours">
              {#each paletteOptions as o}<option value={o.id}>{o.label}</option>{/each}
              {#if u.palette && !paletteOptions.some((o) => o.id === u.palette)}<option value={u.palette}>{u.palette}</option>{/if}
            </select>
          {/if}
          {#if d.needs === "laser"}
            <select bind:value={u.tag} title="Which looks">
              {#each tagOptions as o}<option value={o.id}>{o.label}</option>{/each}
            </select>
            <label class="small inline" title="Use looks tagged breakdown, groove, build or drop for the part of the track playing (tag them in the pattern browser)">
              <input type="checkbox" checked={!!u.follow} onchange={(e) => (u.follow = e.currentTarget.checked)} /> follow the section
            </label>
          {/if}
          <button class="ghost small" class:on={open[bi]} onclick={() => (open[bi] = !open[bi])}>Sections…</button>
        </div>
        {#if open[bi]}
          <div class="sections col">
            <div class="row flow small">
              <span class="faint">Plays in</span>
              {#each SECTIONS as s}
                <button class="chip" class:on={plays(u).includes(s.id)} style="--c:{s.color}" title={s.hint} onclick={() => togglePlays(u, s.id)}>{s.name}</button>
              {/each}
            </div>
            <span class="small faint">Different in a section: leave "same" to use the settings above.</span>
            {#each SECTIONS.filter((s) => plays(u).includes(s.id)) as s}
              {@const v = variant(u, s.id)}
              <div class="row flow vrow small">
                <span class="sec" style="--c:{s.color}">{s.name}</span>
                <select value={v.speed ?? ""} onchange={(e) => setVariant(u, s.id, "speed", e.currentTarget.value)} title="Speed in this section">
                  <option value="">same speed</option>{#each SPEEDS as sp}<option value={sp}>{sp}</option>{/each}
                </select>
                {#if d.sound}
                  <select value={v.sound ?? ""} onchange={(e) => setVariant(u, s.id, "sound", e.currentTarget.value)} title="Trigger in this section">
                    <option value="">same sound</option>{#each sounds(d.sound.kind) as o}<option value={o.id}>{o.label}</option>{/each}
                  </select>
                {/if}
                {#if d.palette}
                  <select value={v.palette ?? ""} onchange={(e) => setVariant(u, s.id, "palette", e.currentTarget.value)} title="Colours in this section">
                    <option value="">same colours</option>{#each paletteOptions as o}<option value={o.id}>{o.label}</option>{/each}
                  </select>
                {/if}
                {#if d.needs === "laser"}
                  <select value={v.tag ?? ""} onchange={(e) => setVariant(u, s.id, "tag", e.currentTarget.value)} title="Looks in this section">
                    <option value="">same looks</option>{#each tagOptions as o}<option value={o.id}>{o.label}</option>{/each}
                  </select>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
        {#if !available(rig, d.needs)}<span class="small warn">Skipped: {missing[d.needs]} in this rig.</span>{/if}
      </div>
    {/if}
  {:else}
    <span class="small faint">No blocks yet. Add one below.</span>
  {/each}
  <select class="addb" onchange={(e) => add(e.currentTarget)}>
    <option value="">+ Add a block…</option>
    {#each (["glow", "laser", "fog"] as const) as needs}
      <optgroup label={NEEDS_LABEL[needs] + (available(rig, needs) ? "" : ` (${missing[needs]})`)}>
        {#each BLOCKS.filter((x) => x.needs === needs) as x}<option value={x.id} disabled={!available(rig, needs)}>{x.name}: {x.description}</option>{/each}
      </optgroup>
    {/each}
  </select>
</div>

<style>
  .blocks { gap: 6px; }
  .block { gap: 4px; padding: 6px 8px; border-radius: 6px; background: #ffffff06; border: 1px solid var(--line); }
  .block.off { opacity: 0.6; }
  .flow { flex-wrap: wrap; gap: 4px; }
  .flow select { max-width: 100%; font-size: 12px; }
  .seg { display: inline-flex; border: 1px solid var(--line-2); border-radius: 6px; overflow: hidden; }
  .seg button { border: 0; border-radius: 0; padding: 1px 7px; font-size: 11px; background: transparent; }
  .seg button.sel { background: color-mix(in srgb, var(--accent) 45%, transparent); color: var(--text); }
  .x { padding: 0 5px; }
  .warn { color: var(--warn, #f2b54a); }
  .addb { font-size: 12px; }
  button.on { background: var(--accent-bg); border-color: var(--accent); }
  .sections { gap: 4px; padding: 6px; border-radius: 6px; border: 1px dashed var(--line-2); }
  .chip { font-size: 11px; padding: 2px 8px; color: var(--muted); }
  .chip.on { background: color-mix(in srgb, var(--c) 30%, transparent); border-color: var(--c); color: var(--text); }
  .sec { min-width: 74px; text-align: center; padding: 1px 6px; border-radius: 10px; font-size: 11px; background: color-mix(in srgb, var(--c) 25%, transparent); border: 1px solid var(--c); }
  .vrow select { font-size: 11px; }
  .inline { display: inline-flex; gap: 4px; align-items: center; margin: 0; }
</style>
