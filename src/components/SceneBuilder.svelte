<script lang="ts">
  // Compose scenes from building blocks: pick blocks, what they drive, the sound and colours.
  import { BLOCKS, analyseRig, available, blockDef, newBlock, soundOptions, targetOptions, type SceneSpec, type Speed } from "../lib/blocks";
  import { app } from "../lib/store.svelte";

  let { scenes = $bindable() }: { scenes: SceneSpec[] } = $props();
  const show = app.show!;
  const rig = $derived(analyseRig(show));
  const SPEEDS: Speed[] = ["slow", "medium", "fast"];
  const NEEDS_LABEL = { glow: "Colour fixtures", laser: "Lasers", fog: "Fog" } as const;
  const missing = { glow: "no colour fixtures", laser: "no lasers with saved looks", fog: "no fog machine" } as const;

  function addScene() {
    const names = new Set(scenes.map((s) => s.name));
    let n = scenes.length + 1;
    while (names.has(`Scene ${n}`)) n++;
    scenes.push({ name: `Scene ${n}`, blocks: [] });
  }
  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= scenes.length) return;
    [scenes[i], scenes[j]] = [scenes[j], scenes[i]];
  }
  function add(scene: SceneSpec, sel: HTMLSelectElement) {
    if (sel.value) scene.blocks.push(newBlock(sel.value));
    sel.value = "";
  }
  const paletteOptions = $derived([
    { id: "calm", label: "Genre: calm colours" }, { id: "hot", label: "Genre: hot colours" },
    ...show.palettes.map((p) => ({ id: p.id, label: p.name })),
  ]);
  const tagOptions = $derived([
    { id: "", label: "Any look" }, { id: "calm", label: `Calm looks${rig.calmTag ? ` (${rig.calmTag})` : ""}` },
    { id: "peak", label: `Peak looks${rig.peakTag ? ` (${rig.peakTag})` : ""}` },
    ...rig.tags.filter((t) => t !== rig.calmTag && t !== rig.peakTag && t !== "default").map((t) => ({ id: t, label: `Tagged ${t}` })),
  ]);
</script>

<div class="builder col">
  {#each scenes as scene, si (si)}
    <div class="scene col">
      <div class="row">
        <input class="grow name" bind:value={scene.name} placeholder="Scene name" />
        <button class="ghost" onclick={() => move(si, -1)} aria-label="Move scene up" disabled={si === 0}>↑</button>
        <button class="ghost" onclick={() => move(si, 1)} aria-label="Move scene down" disabled={si === scenes.length - 1}>↓</button>
        <button class="ghost" onclick={() => scenes.splice(si, 1)} aria-label="Remove scene">×</button>
      </div>
      {#each scene.blocks as u, bi (bi)}
        {@const d = blockDef(u.block)}
        {#if d}
          <div class="block col" class:off={!available(rig, d.needs)}>
            <div class="row">
              <b class="grow small" title={d.description}>{d.name}</b>
              <div class="seg">
                {#each SPEEDS as sp}<button class:sel={u.speed === sp} onclick={() => (u.speed = sp)}>{sp}</button>{/each}
              </div>
              <button class="ghost x" onclick={() => scene.blocks.splice(bi, 1)} aria-label="Remove block">×</button>
            </div>
            <div class="row flow">
              <select bind:value={u.target} title="What it drives">
                {#each targetOptions(show, rig, d.needs) as o}<option value={o.id}>{o.label}</option>{/each}
              </select>
              {#if d.sound}
                <select bind:value={u.sound} title="What triggers it">
                  {#if d.sound.kind === "hit"}
                    <option value="kick">Kick hit</option><option value="snare">Snare hit</option><option value="hat">Hi-hat hit</option>
                  {:else}
                    <option value="vocals">Vocals level</option>
                  {/if}
                  {#each soundOptions(show, d.sound.kind).filter((o) => !["kick", "snare", "hat", "vocals.level"].includes(o.id)) as o}<option value={o.id}>{o.label}</option>{/each}
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
              {/if}
            </div>
            {#if !available(rig, d.needs)}<span class="small warn">Skipped: {missing[d.needs]} in this rig.</span>{/if}
          </div>
        {/if}
      {:else}
        <span class="small faint">No blocks yet.</span>
      {/each}
      <select class="addb" onchange={(e) => add(scene, e.currentTarget)}>
        <option value="">+ Add a block…</option>
        {#each (["glow", "laser", "fog"] as const) as needs}
          <optgroup label={NEEDS_LABEL[needs] + (available(rig, needs) ? "" : ` (${missing[needs]})`)}>
            {#each BLOCKS.filter((x) => x.needs === needs) as x}<option value={x.id} disabled={!available(rig, needs)}>{x.name}: {x.description}</option>{/each}
          </optgroup>
        {/each}
      </select>
    </div>
  {/each}
  <button onclick={addScene}>+ Add scene</button>
</div>

<style>
  .builder { gap: 8px; }
  .scene { gap: 6px; padding: 8px; border: 1px solid var(--line-2); border-radius: 8px; }
  .name { font-weight: 600; }
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
</style>
