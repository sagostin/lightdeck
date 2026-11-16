<script lang="ts">
  import ConfirmButton from "./ConfirmButton.svelte";
  import { app, uid } from "../lib/store.svelte";

  const show = app.show!;
  let open = $state<string | null>(null);
  const usedBy = (id: string) => show.routes.filter((r) => "palette" in r.action && r.action.palette === id).length;

  function add() {
    const p = { id: uid("pal"), name: `Palette ${show.palettes.length + 1}`, colors: ["#ff0000", "#0000ff"] };
    show.palettes.push(p);
    open = p.id;
  }
</script>

<div class="col">
  <div class="row"><h3 class="grow">Colour palettes</h3><button class="ghost small" onclick={add}>+ New</button></div>
  {#each show.palettes as p (p.id)}
    <div class="pal">
      <button class="swatches" onclick={() => (open = open === p.id ? null : p.id)} title="Edit {p.name}">
        {#each p.colors as c}<i style="background:{c}"></i>{/each}
      </button>
      <div class="row small"><span class="grow">{p.name}</span>{#if usedBy(p.id)}<span class="faint">{usedBy(p.id)} route(s)</span>{/if}</div>
      {#if open === p.id}
        <div class="edit">
          <input bind:value={p.name} />
          <div class="row wrap">
            {#each p.colors as _, i}
              <span class="c">
                <input type="color" bind:value={p.colors[i]} />
                {#if p.colors.length > 1}<button class="x" onclick={() => p.colors.splice(i, 1)} aria-label="Remove colour">×</button>{/if}
              </span>
            {/each}
            <button class="ghost small" onclick={() => p.colors.push(p.colors.at(-1) ?? "#ffffff")}>+ colour</button>
          </div>
          <div class="row">
            <span class="grow"></span>
            <ConfirmButton onconfirm={() => { show.palettes.splice(show.palettes.indexOf(p), 1); open = null; }}>Delete</ConfirmButton>
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .pal { display: flex; flex-direction: column; gap: 3px; }
  .swatches { display: flex; padding: 0; overflow: hidden; height: 20px; border-radius: 5px; }
  .swatches i { flex: 1; }
  .edit { display: flex; flex-direction: column; gap: 6px; padding: 6px 0 8px; }
  .wrap { flex-wrap: wrap; gap: 4px; }
  .c { position: relative; }
  .c input { width: 32px; height: 28px; padding: 2px; }
  .x { position: absolute; top: -6px; right: -6px; padding: 0 4px; font-size: 10px; line-height: 14px; border-radius: 50%; }
  .small { font-size: 11px; }
</style>
