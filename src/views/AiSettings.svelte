<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { AiSettings } from "../lib/types";

  let s = $state<AiSettings | null>(null);
  let headers = $state<{ k: string; v: string }[]>([]);
  let status = $state("");
  let busy = $state(false);
  let showKey = $state(false);

  onMount(async () => {
    s = await invoke<AiSettings>("get_ai_settings");
    headers = Object.entries(s.extra_headers).map(([k, v]) => ({ k, v }));
  });

  function presets(schema: "anthropic" | "openai") {
    if (!s) return;
    s.schema = schema;
    if (schema === "anthropic") { s.base_url ||= "https://api.anthropic.com"; s.auth = "x-api-key"; }
    else { s.auth = "bearer"; }
  }
  async function save() {
    if (!s) return;
    s.extra_headers = Object.fromEntries(headers.filter((h) => h.k.trim()).map((h) => [h.k.trim(), h.v]));
    await invoke("save_ai_settings", { settings: $state.snapshot(s) });
    status = "Saved";
  }
  async function test() {
    busy = true;
    await save();
    try { status = `Connected: "${(await invoke<string>("ai_test")).trim().slice(0, 60)}"`; }
    catch (e) { status = String(e); }
    busy = false;
  }
</script>

{#if s}
  <div class="panel col form">
    <h2>AI provider</h2>
    <p class="muted small">
      Used to interpret fixture docs into LightDeck's format and to suggest tags. Optional: everything else works without it.
      The key is stored only on this Mac, in the app's data folder.
    </p>

    <label>API schema</label>
    <div class="row">
      <button class:on={s.schema === "anthropic"} onclick={() => presets("anthropic")}>Anthropic Messages</button>
      <button class:on={s.schema === "openai"} onclick={() => presets("openai")}>OpenAI-compatible</button>
    </div>

    <label>Base URL or proxy</label>
    <input class="mono" bind:value={s.base_url} placeholder={s.schema === "anthropic" ? "https://api.anthropic.com" : "https://my-gateway.example.com/v1"} />
    <span class="faint small">
      {s.schema === "anthropic" ? "Requests go to {base}/v1/messages." : "Requests go to {base}/chat/completions."} A full endpoint URL also works.
    </span>

    <label>API key</label>
    <div class="row">
      <input class="grow mono" type={showKey ? "text" : "password"} bind:value={s.api_key} autocomplete="off" />
      <button onclick={() => (showKey = !showKey)}>{showKey ? "Hide" : "Show"}</button>
    </div>

    <label>Send key as</label>
    <select bind:value={s.auth}>
      <option value="x-api-key">x-api-key header</option>
      <option value="bearer">Authorization: Bearer</option>
      <option value="none">Don't send (proxy adds it)</option>
    </select>

    <div class="row">
      <div class="col grow"><label>Model</label><input class="mono" bind:value={s.model} /></div>
      <div class="col"><label>Max tokens</label><input type="number" min="256" step="1000" bind:value={s.max_tokens} /></div>
    </div>

    <label>Extra headers</label>
    {#each headers as h, i}
      <div class="row">
        <input class="mono" bind:value={h.k} placeholder="Header" />
        <input class="grow mono" bind:value={h.v} placeholder="Value" />
        <button class="ghost" onclick={() => headers.splice(i, 1)}>×</button>
      </div>
    {/each}
    <button class="ghost add" onclick={() => headers.push({ k: "", v: "" })}>+ Add header</button>

    <div class="row">
      <button class="primary" onclick={save}>Save</button>
      <button onclick={test} disabled={busy}>{busy ? "Testing…" : "Save and test"}</button>
      <span class="small" class:err={status && !status.startsWith("Saved") && !status.startsWith("Connected")}>{status}</span>
    </div>
  </div>
{/if}

<style>
  .form { max-width: 640px; }
  label { margin-top: 6px; }
  .add { align-self: flex-start; }
  input[type="number"] { width: 110px; }
</style>
