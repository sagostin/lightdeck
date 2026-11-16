<script lang="ts">
  // Two-click confirm: the webview has no window.confirm(), so the first click arms
  // the button and a second click within 3 s performs the action.
  import type { Snippet } from "svelte";
  let { onconfirm, children, confirmLabel = "Click again to confirm", class: cls = "ghost danger" }:
    { onconfirm: () => void; children: Snippet; confirmLabel?: string; class?: string } = $props();
  let armed = $state(false);
  let t: ReturnType<typeof setTimeout> | undefined;
  function click(e: MouseEvent) {
    e.stopPropagation();
    if (armed) {
      clearTimeout(t);
      armed = false;
      onconfirm();
    } else {
      armed = true;
      t = setTimeout(() => (armed = false), 3000);
    }
  }
</script>

<button class={cls} class:armed onclick={click}>{#if armed}{confirmLabel}{:else}{@render children()}{/if}</button>

<style>
  .armed { background: var(--danger) !important; color: #fff !important; border-color: var(--danger) !important; }
</style>
