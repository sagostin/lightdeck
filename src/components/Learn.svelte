<script lang="ts">
  // Learn for controls, on every screen: the next MIDI pad/knob or key pressed becomes the
  // trigger of the control waiting in app.learning (see startLearn).
  import { app, toast } from "../lib/store.svelte";
  import type { ControlMapping } from "../lib/types";

  const FADERS = ["master", "channel_value", "band_sensitivity"];
  const label = (m: ControlMapping) => {
    const t = m.trigger;
    if (t.source === "key") return `Key ${t.key === " " ? "Space" : t.key.toUpperCase()}`;
    return `${t.kind === "cc" ? "CC" : t.kind === "program" ? "Program" : "Note"} ${t.number}`;
  };

  $effect(() => {
    const ev = app.midiLog[0];
    const m = app.show?.controls.find((c) => c.id === app.learning);
    if (!ev || !m || m.trigger.source !== "midi" || ev.at < app.learnStarted) return;
    if (ev.kind === "note" && ev.value === 0) return; // wait for a note-on
    m.trigger.kind = ev.kind;
    m.trigger.number = ev.number;
    m.trigger.channel = ev.channel;
    const fader = FADERS.includes(m.action.type);
    m.trigger.mode = ev.kind === "cc" && fader ? "fader" : m.trigger.mode === "fader" ? "toggle" : m.trigger.mode;
    app.learning = null;
    toast(`Learned ${label(m)} for ${m.name || "the control"}`);
  });

  function onKey(e: KeyboardEvent) {
    const m = app.show?.controls.find((c) => c.id === app.learning);
    if (!m || m.trigger.source !== "key") return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") { app.learning = null; return; }
    m.trigger.key = e.key.toLowerCase();
    app.learning = null;
    toast(`Learned ${label(m)} for ${m.name || "the control"}`);
  }
</script>

<svelte:window onkeydowncapture={onKey} />
