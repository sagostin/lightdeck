<script lang="ts">
  // A "⋯" button with a small dropdown of actions. Closes on outside click or Escape.
  import type { Snippet } from "svelte";
  let { children, label = "More actions" }: { children: Snippet; label?: string } = $props();
  let open = $state(false);
  let root: HTMLElement;
  function onDoc(e: PointerEvent) {
    if (open && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onpointerdown={onDoc} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<span class="menu" bind:this={root}>
  <button class="ghost dots" aria-label={label} aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>⋯</button>
  {#if open}
    <div class="pop" role="menu" tabindex="-1" onclick={(e) => { if ((e.target as HTMLElement).closest("[data-close]")) open = false; }} onkeydown={() => {}}>
      {@render children()}
    </div>
  {/if}
</span>

<style>
  .menu { position: relative; display: inline-block; }
  .dots { padding: 2px 9px; font-size: 16px; line-height: 1; }
  .pop {
    position: absolute; right: 0; top: calc(100% + 4px); z-index: 30; min-width: 170px;
    background: var(--panel-2); border: 1px solid var(--line-2); border-radius: 8px; padding: 4px;
    display: flex; flex-direction: column; box-shadow: 0 12px 30px #0008;
  }
  .pop :global(button) { text-align: left; background: transparent; border-color: transparent; width: 100%; }
  .pop :global(button:hover) { background: #ffffff10; }
</style>
