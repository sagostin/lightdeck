<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import { app, uid, toast } from "../lib/store.svelte";
  import type { Fixture, Profile, ZigbeeDevice, ZigbeePort, ZigbeeStatus } from "../lib/types";

  const show = app.show!;
  const z = show.zigbee;
  let status = $state<ZigbeeStatus | null>(null);
  let ports = $state<ZigbeePort[]>([]);
  let portsMsg = $state("");
  let showPass = $state(false);
  let showLog = $state(false);
  let filter = $state<"lights" | "all">("lights");
  let now = $state(Date.now());
  let builtins: Profile[] = [];

  const ADAPTERS = [
    { id: "ember", label: "Silicon Labs EFR32 (EmberZNet) — Sonoff ZBDongle‑E, SMLIGHT, most new sticks" },
    { id: "zstack", label: "Texas Instruments CC2652 / CC1352 (Z‑Stack) — Sonoff ZBDongle‑P" },
    { id: "deconz", label: "ConBee / RaspBee (deCONZ)" },
    { id: "zigate", label: "ZiGate" },
    { id: "zboss", label: "Nordic nRF (ZBOSS)" },
    { id: "auto", label: "Auto-detect" },
  ];

  async function poll() {
    try { status = await invoke<ZigbeeStatus>("zigbee_status"); } catch {}
    now = Date.now();
  }
  async function loadPorts() {
    portsMsg = "";
    try {
      ports = await invoke<ZigbeePort[]>("zigbee_ports");
      if (!ports.length) portsMsg = "No USB serial devices found. Plug the dongle in and refresh.";
      // Pick the dongle automatically when there's an obvious one.
      if (!z.serial_port && ports.length === 1) choosePort(ports[0]);
    } catch (e) {
      portsMsg = String(e);
    }
  }
  function choosePort(p: ZigbeePort) {
    z.serial_port = p.path;
    if (p.adapter && p.adapter !== z.adapter) z.adapter = p.adapter;
  }
  onMount(() => {
    poll();
    const t = setInterval(poll, 1000);
    invoke<Profile[]>("zigbee_profiles").then((p) => (builtins = p));
    if (z.enabled && z.mode === "usb") setTimeout(loadPorts, 600);
    return () => clearInterval(t);
  });

  const usb = $derived(z.mode === "usb");
  const devices = $derived((status?.devices ?? []).filter((d) => filter === "all" || d.kind === "light" || d.kind === "switch"));
  const pairingLeft = $derived(status?.permit_join_until ? Math.max(0, Math.round((status.permit_join_until - now) / 1000)) : 0);
  const patched = (key: string) => show.fixtures.find((f) => f.zigbee_device === key);

  async function pair(seconds: number) {
    try {
      await invoke("zigbee_permit_join", { seconds });
      await poll();
    } catch (e) { toast(String(e)); }
  }
  function profileFor(d: ZigbeeDevice) {
    return d.color ? "zigbee-rgb" : d.color_temp ? "zigbee-white" : d.brightness ? "zigbee-dimmer" : "zigbee-switch";
  }
  function ensureProfile(id: string) {
    if (show.profiles.some((p) => p.id === id)) return true;
    const p = builtins.find((p) => p.id === id);
    if (!p) return false;
    show.profiles.push(structuredClone(p));
    return true;
  }
  function addDevice(d: ZigbeeDevice) {
    const pid = profileFor(d);
    if (!ensureProfile(pid)) { toast("Zigbee profiles not loaded yet; try again"); return; }
    const room = show.rooms.find((r) => r.id === show.active_room) ?? show.rooms[0];
    const n = show.fixtures.filter((f) => f.zigbee_device).length;
    const f: Fixture = {
      id: uid("fx"), name: d.friendly_name, profile_id: pid, mode: "Default", output_id: show.outputs[0]?.id ?? "",
      address: 1, room_id: room?.id ?? null, x: 0.6 + (n % 6) * 0.9, y: 3.6 - Math.floor(n / 6) * 0.8, z: 2.2,
      rotation: 270, enabled: true, zigbee_device: d.key, limits: {},
    };
    show.fixtures.push(f);
  }
  function addAll() {
    let n = 0;
    for (const d of devices) if (!patched(d.key) && d.kind !== "other" && d.interview !== "FAILED") { addDevice(d); n++; }
    toast(n ? `Added ${n} Zigbee light${n > 1 ? "s" : ""} to the patch` : "Every light is already patched");
  }
  function rename(d: ZigbeeDevice, name: string) {
    name = name.trim();
    if (name) z.names[d.ieee] = name; else delete z.names[d.ieee];
    const f = patched(d.key);
    if (f && name && (f.name === d.friendly_name)) f.name = name;
  }
  async function run(cmd: string, args: Record<string, unknown>, ok?: string) {
    try { await invoke(cmd, args); if (ok) toast(ok); await poll(); } catch (e) { toast(String(e)); }
  }
  async function remove(d: ZigbeeDevice) {
    try {
      await invoke("zigbee_remove", { ieee: d.ieee, force: false });
      toast(`Removed ${d.friendly_name} from the network`);
    } catch (e) {
      // Unreachable devices can't confirm leaving; drop them from the coordinator anyway.
      await run("zigbee_remove", { ieee: d.ieee, force: true }, `${d.friendly_name} didn't answer; removed it from LightDeck`);
    }
    const f = patched(d.key);
    if (f) f.enabled = false;
    await poll();
  }
  const caps = (d: ZigbeeDevice) =>
    [d.color && "colour", d.color_temp && "white temp", d.brightness && "dimming", d.switch_only && "on/off"].filter(Boolean).join(", ");
  const interviewTag = (s: string) =>
    s === "SUCCESSFUL" ? "" : s === "FAILED" ? "Setup failed" : s === "IN_PROGRESS" ? "Setting up…" : s === "PENDING" ? "Waiting" : "";
</script>

<div class="zb">
  <div class="panel col settings">
    <div class="row">
      <h2 class="grow">Zigbee</h2>
      <label class="row"><input type="checkbox" bind:checked={z.enabled} onchange={() => z.enabled && usb && setTimeout(loadPorts, 600)} /> Enabled</label>
    </div>
    <div class="row">
      <button class="grow" class:on={usb} onclick={() => { z.mode = "usb"; if (z.enabled) setTimeout(loadPorts, 600); }}>USB dongle</button>
      <button class="grow" class:on={!usb} onclick={() => (z.mode = "mqtt")}>Zigbee2MQTT</button>
    </div>

    <div class="statusbox">
      {#if !z.enabled}<span class="tag">Off</span>
      {:else if status?.connected}<span class="tag accent">{usb ? "Coordinator running" : "Broker connected"}</span>
      {:else}<span class="tag warn">{status?.state || "Starting…"}</span>{/if}
      {#if z.enabled && status?.connected && status.state}<span class="small muted">{status.state}</span>{/if}
      {#if usb && status?.coordinator}<div class="small faint">{status.coordinator}</div>{/if}
      {#if !usb && status?.bridge_online === false}<span class="tag warn">Zigbee2MQTT offline</span>{/if}
      {#if z.enabled && status?.error}<div class="err small">{status.error}</div>{/if}
    </div>

    {#if usb}
      <label>Dongle</label>
      <div class="row">
        <select class="grow" value={z.serial_port} onchange={(e) => { const p = ports.find((x) => x.path === (e.target as HTMLSelectElement).value); if (p) choosePort(p); else z.serial_port = (e.target as HTMLSelectElement).value; }}>
          <option value="">Choose…</option>
          {#each ports as p}<option value={p.path}>{p.name}{p.adapter ? ` (${p.adapter})` : ""}</option>{/each}
          {#if z.serial_port && !ports.some((p) => p.path === z.serial_port)}<option value={z.serial_port}>{z.serial_port}</option>{/if}
        </select>
        <button onclick={loadPorts} disabled={!z.enabled}>Refresh</button>
      </div>
      {#if portsMsg}<div class="small muted">{portsMsg}</div>{/if}
      {#if !z.enabled}<div class="small faint">Turn Zigbee on to look for the dongle.</div>{/if}
      <input class="mono small" bind:value={z.serial_port} placeholder="/dev/cu.usbserial-…" />

      <label>Chip type</label>
      <select bind:value={z.adapter}>{#each ADAPTERS as a}<option value={a.id}>{a.label}</option>{/each}</select>

      <label>Channel <span class="faint small">used when the network is first created</span></label>
      <select bind:value={z.channel}>
        {#each [11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26] as c}
          <option value={c}>{c}{[15, 20, 25].includes(c) ? " (avoids common Wi‑Fi)" : ""}</option>
        {/each}
      </select>
    {:else}
      <label>MQTT broker</label>
      <div class="row">
        <input class="grow mono" bind:value={z.host} placeholder="192.168.1.20" />
        <input class="mono port" type="number" min="1" max="65535" bind:value={z.port} />
      </div>
      <div class="row">
        <input class="grow" bind:value={z.username} placeholder="Username (optional)" autocomplete="off" />
        <input class="grow" type={showPass ? "text" : "password"} bind:value={z.password} placeholder="Password" autocomplete="off" />
        <button class="ghost" onclick={() => (showPass = !showPass)}>{showPass ? "Hide" : "Show"}</button>
      </div>
      <label>Base topic</label>
      <input class="mono" bind:value={z.base_topic} />
      <p class="small faint">The broker password is saved in the project file.</p>
    {/if}

    <label>Max messages per second <span class="mono">{z.max_rate_hz}</span></label>
    <input type="range" min="2" max="30" step="1" bind:value={z.max_rate_hz} />
    <label>Fade on each change <span class="mono">{z.transition_s.toFixed(1)} s</span></label>
    <input type="range" min="0" max="2" step="0.1" bind:value={z.transition_s} />
    {#if status?.connected}
      <div class="row small muted">
        <span>{status.sent_per_sec} msg/s</span>
        {#if status.backlog > 0}<span class="tag warn">{status.backlog} light{status.backlog > 1 ? "s" : ""} waiting</span>{/if}
      </div>
      {#if status.last_send_error}<div class="small err">Last send: {status.last_send_error}</div>{/if}
    {/if}
    <div class="row">
      <button onclick={() => invoke("zigbee_reconnect")}>Restart</button>
      {#if usb}<button class="ghost" onclick={() => (showLog = !showLog)}>{showLog ? "Hide" : "Show"} log</button>{/if}
    </div>
    {#if showLog}<pre class="log">{(status?.log ?? []).slice(-40).join("\n") || "No log yet."}</pre>{/if}
    <p class="small muted">
      A Zigbee network handles about 10–20 commands a second in total and each light lags 50–150 ms, so use Zigbee lights
      for washes, slow colour changes and levels (Bass, Vocals, Energy) rather than strobes or per-beat flashes.
    </p>
  </div>

  <div class="col grow">
    {#if usb && z.enabled && status?.connected}
      <div class="panel pairing" class:active={pairingLeft > 0}>
        {#if pairingLeft > 0}
          <div class="row">
            <span class="pulse"></span>
            <div class="grow"><b>Pairing open</b> · {pairingLeft}s left. New lights appear below as they join.</div>
            <button onclick={() => pair(0)}>Stop pairing</button>
          </div>
          <p class="small muted">
            To pair a bulb, power it on near the dongle. If it was used before, reset it first: most bulbs reset by switching
            them off and on 5–6 times (about a second each) until they blink. Philips Hue bulbs need a Hue dimmer or a factory
            reset through their app. Setup takes up to a minute per light.
          </p>
        {:else}
          <div class="row">
            <div class="grow"><b>Add lights</b> · open pairing, then reset or power on each bulb.</div>
            <button class="primary" onclick={() => pair(180)}>Pair new devices</button>
          </div>
        {/if}
      </div>
    {/if}

    <div class="panel col">
      <div class="row">
        <h2 class="grow">Devices</h2>
        <select bind:value={filter}><option value="lights">Lights and plugs</option><option value="all">All devices</option></select>
        <button disabled={!devices.length} onclick={addAll}>Add all to patch</button>
      </div>
      {#if !z.enabled}
        <div class="empty">Turn Zigbee on to start.</div>
      {:else if !status?.connected}
        <div class="empty">{usb ? "Plug in the dongle and choose it on the left." : "Waiting for the broker…"}</div>
      {:else if !devices.length}
        <div class="empty">{usb ? "No devices paired yet. Press Pair new devices." : `No devices yet. Zigbee2MQTT publishes its device list on ${z.base_topic}/bridge/devices.`}</div>
      {:else}
        <table>
          <thead><tr><th>Name</th><th>Device</th><th>Supports</th>{#if usb}<th>Signal</th>{/if}<th>In patch</th><th></th></tr></thead>
          <tbody>
            {#each devices as d (d.key)}
              {@const p = patched(d.key)}
              {@const it = interviewTag(d.interview)}
              <tr>
                <td>
                  {#if usb}
                    <input value={d.friendly_name} onchange={(e) => rename(d, (e.target as HTMLInputElement).value)} />
                  {:else}{d.friendly_name}{/if}
                  {#if it}<span class="tag" class:warn={it !== "Waiting"}>{it}</span>{/if}
                </td>
                <td class="muted small">{d.vendor} {d.model}<br /><span class="faint mono">{d.ieee}</span></td>
                <td class="small">{caps(d) || "—"}</td>
                {#if usb}<td class="small faint">{d.lqi ?? "—"}{d.role ? ` · ${d.role === "Router" ? "mains" : d.role === "EndDevice" ? "battery" : d.role}` : ""}</td>{/if}
                <td>{#if p}<span class="tag accent">{p.name}</span>{:else}<span class="faint">—</span>{/if}</td>
                <td class="actions">
                  <button class="ghost" onclick={() => run("zigbee_identify", { name: d.key })}>Blink</button>
                  {#if !p && d.kind !== "other" && d.interview !== "FAILED"}<button onclick={() => addDevice(d)}>Add</button>{/if}
                  {#if usb && d.interview === "FAILED"}<button onclick={() => run("zigbee_reinterview", { ieee: d.ieee }, "Retrying setup…")}>Retry setup</button>{/if}
                  {#if usb}<ConfirmButton onconfirm={() => remove(d)} confirmLabel="Remove from network?">Remove</ConfirmButton>{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="small muted">
          Added lights land in the current room; drag them into place on the Rooms screen. The fixture type is picked from what
          the device supports. Removing a device makes it leave the network; reset it to pair it again.
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .zb { display: flex; gap: 16px; align-items: flex-start; }
  .settings { width: 380px; flex-shrink: 0; }
  label { margin-top: 4px; display: flex; justify-content: space-between; gap: 8px; }
  .port { width: 80px; }
  .statusbox { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; padding: 8px; background: var(--panel-2); border-radius: 6px; }
  .statusbox > div { width: 100%; }
  .pairing.active { border-color: var(--accent); background: var(--accent-bg); }
  .pulse { width: 12px; height: 12px; border-radius: 50%; background: var(--accent); animation: pulse 1.2s infinite; }
  @keyframes pulse { 50% { opacity: 0.3; } }
  .log { max-height: 220px; overflow: auto; font-size: 10px; background: #0a0b10; padding: 8px; border-radius: 6px; white-space: pre-wrap; margin: 0; }
  td input { width: 100%; min-width: 140px; }
  td.actions { display: flex; gap: 4px; flex-wrap: wrap; }
</style>
