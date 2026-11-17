<script lang="ts">
  // A small "i" that explains a field on hover or keyboard focus (and to screen readers).
  let { text, below = false }: { text: string; below?: boolean } = $props();
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so keyboard users can read it) -->
<span class="tip" class:below tabindex="0" role="note" aria-label={text}>
  <span class="i" aria-hidden="true">i</span>
  <span class="pop" aria-hidden="true">{text}</span>
</span>

<style>
  .tip { position: relative; display: inline-flex; align-items: center; vertical-align: middle; cursor: help; outline: none; margin: 0 3px 0 1px; }
  .i {
    width: 14px; height: 14px; border-radius: 50%; border: 1px solid var(--line-2); color: var(--faint);
    font: italic 600 10px/12px Georgia, serif; text-align: center; flex-shrink: 0;
  }
  .tip:hover .i, .tip:focus-visible .i { color: var(--text); border-color: var(--accent); }
  .pop {
    display: none; position: absolute; z-index: 50; left: 50%; bottom: calc(100% + 6px); transform: translateX(-50%);
    width: max-content; max-width: 300px; padding: 7px 9px; border-radius: 6px; border: 1px solid var(--line-2);
    background: var(--panel-2); color: var(--text); box-shadow: 0 6px 20px #0008;
    font-size: 12px; font-weight: 400; font-style: normal; line-height: 1.4; white-space: normal; text-align: left; pointer-events: none;
  }
  .below .pop { bottom: auto; top: calc(100% + 6px); }
  .tip:hover .pop, .tip:focus-visible .pop { display: block; }
</style>
