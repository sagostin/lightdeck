<script lang="ts">
  import { app, uid, footprint } from "../lib/store.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";

  const show = app.show!;
  function add() {
    show.outputs.push({ id: uid("out"), name: `Node ${show.outputs.length + 1}`, ip: "2.0.0.10", net: 0, subnet: 0, universe: show.outputs.length, enabled: false });
  }
  let msg = $state("");
  function remove(i: number) {
    const o = show.outputs[i];
    if (show.fixtures.some((f) => f.output_id === o.id && !f.zigbee_device)) { msg = `Move fixtures off "${o.name}" in Patch before deleting it.`; return; }
    msg = "";
    show.outputs.splice(i, 1);
  }
  const colors = ["#7c5cff", "#45d39a", "#f2b54a", "#ff5c6c", "#4fb3ff", "#ff7ad9"];
</script>

<div class="col">
  <div class="row"><h2 class="grow">Art-Net outputs</h2><button class="primary" onclick={add}>Add output</button></div>
  <p class="muted small">
    One output is one DMX universe on a node (for example a PKnight CR011R). Use the node's IP address rather than broadcast where you can.
    Outputs start disabled so nothing is sent until you're ready.
  </p>
  {#each show.outputs as o, i (o.id)}
    {@const fx = show.fixtures.filter((f) => f.output_id === o.id && !f.zigbee_device).sort((a, b) => a.address - b.address)}
    <div class="panel col">
      <div class="row">
        <label class="row"><input type="checkbox" bind:checked={o.enabled} /> Send</label>
        <input bind:value={o.name} />
        <label>IP</label><input class="mono" bind:value={o.ip} />
        <label>Net</label><input type="number" min="0" max="127" bind:value={o.net} />
        <label>Subnet</label><input type="number" min="0" max="15" bind:value={o.subnet} />
        <label>Universe</label><input type="number" min="0" max="15" bind:value={o.universe} />
        <span class="faint small mono">port {(o.net << 8) | (o.subnet << 4) | o.universe}</span>
        <span class="grow"></span>
        <ConfirmButton onconfirm={() => remove(i)}>Delete</ConfirmButton>
      </div>
      <div class="uni">
        {#each fx as f, j}
          <div class="seg" title="{f.name}: {f.address}–{f.address + footprint(f) - 1}"
            style="left:{((f.address - 1) / 512) * 100}%;width:{(footprint(f) / 512) * 100}%;background:{colors[j % colors.length]}"></div>
        {/each}
      </div>
      <div class="row small muted wrap">
        {#each fx as f, j}<span><span class="dot" style="background:{colors[j % colors.length]}"></span>{f.name} {f.address}–{f.address + footprint(f) - 1}</span>{/each}
        {#if !fx.length}<span>No fixtures on this output.</span>{/if}
      </div>
    </div>
  {/each}
  {#if msg}<div class="panel err">{msg}</div>{/if}
  {#if app.frame?.send_errors.length}
    <div class="panel err">{#each app.frame.send_errors as e}<div>{e}</div>{/each}</div>
  {/if}
</div>

<style>
  .uni { position: relative; height: 18px; background: var(--panel-2); border-radius: 4px; overflow: hidden; }
  .seg { position: absolute; top: 0; bottom: 0; opacity: 0.8; min-width: 2px; }
  .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 4px; }
  .wrap { flex-wrap: wrap; gap: 14px; }
  input:not([type]) { width: 140px; }
</style>
