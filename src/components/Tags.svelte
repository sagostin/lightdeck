<script lang="ts">
  import { COMMON_TAGS } from "../lib/types";
  let { tags = $bindable([]), suggestions = COMMON_TAGS }: { tags: string[]; suggestions?: string[] } = $props();
  let draft = $state("");
  const listId = `tags-${Math.random().toString(36).slice(2)}`;
  function add() {
    const t = draft.trim().toLowerCase().replace(/\s+/g, "_");
    if (t && !tags.includes(t)) tags.push(t);
    draft = "";
  }
  function cls(t: string) {
    return t === "blocked" ? "blocked" : t === "off" ? "off" : t === "safe" || t === "favourite" ? "accent" : t === "large_projection" ? "warn" : "";
  }
</script>

<span class="tags">
  {#each tags as t, i}
    <span class="tag {cls(t)}">{t}<button onclick={() => tags.splice(i, 1)} aria-label="Remove {t}">×</button></span>
  {/each}
  <input list={listId} bind:value={draft} placeholder="+ tag" onkeydown={(e) => e.key === "Enter" && add()} onchange={add} />
  <datalist id={listId}>{#each suggestions.filter((s) => !tags.includes(s)) as s}<option value={s}></option>{/each}</datalist>
</span>

<style>
  .tags { display: inline-flex; flex-wrap: wrap; gap: 4px; align-items: center; }
  input { width: 70px; height: 22px; padding: 0 6px; font-size: 11px; background: transparent; border-style: dashed; }
</style>
