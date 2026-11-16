<script lang="ts">
  import { untrack } from "svelte";
  import { app } from "../lib/store.svelte";
  import { SPEC_HI, SPEC_LO, type BandDef, type BandRange } from "../lib/types";

  let { selected = $bindable<string | null>(null) }: { selected?: string | null } = $props();

  const W = 960, H = 230, TH = 110;
  const LR = Math.log(SPEC_HI / SPEC_LO);
  const xOf = (hz: number) => (W * Math.log(Math.max(SPEC_LO, hz) / SPEC_LO)) / LR;
  const hzOf = (x: number) => SPEC_LO * Math.exp((Math.max(0, Math.min(W, x)) / W) * LR);
  const ticks = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
  const fmt = (hz: number) => (hz >= 1000 ? `${+(hz / 1000).toFixed(1)}k` : `${Math.round(hz)}`);

  const bands = $derived(app.show!.audio.bands);
  const spec = $derived(app.frame?.features.spectrum ?? []);
  const sel = $derived(bands.find((b) => b.id === selected));

  // rolling onset trace for the selected band
  const N = 160;
  let trace = $state<{ flux: number; thr: number; hit: boolean }[]>([]);
  let lastHits = -1;
  let lastBand = "";
  $effect(() => {
    const out = app.frame?.features.bands.find((b) => b.id === selected);
    if (!out) return;
    if (lastBand !== out.id) { trace = []; lastHits = out.hits; lastBand = out.id; }
    const hit = out.hits !== lastHits;
    lastHits = out.hits;
    const prev = untrack(() => trace);
    trace = [...prev.slice(-(N - 1)), { flux: out.flux, thr: out.threshold, hit }];
  });
  const traceMax = $derived(Math.max(0.01, ...trace.map((t) => Math.max(t.flux, t.thr))) * 1.15);
  const ty = (v: number) => TH - (v / traceMax) * (TH - 8);
  const tx = (i: number) => (i / (N - 1)) * W;

  let svg: SVGSVGElement;
  let drag: { band: BandDef; r: BandRange; mode: "lo" | "hi" | "move"; x0: number; lo0: number; hi0: number } | null = null;
  function svgX(e: PointerEvent) {
    const pt = svg.createSVGPoint();
    pt.x = e.clientX; pt.y = e.clientY;
    return pt.matrixTransform(svg.getScreenCTM()!.inverse()).x;
  }
  function down(e: PointerEvent, band: BandDef, r: BandRange, mode: "lo" | "hi" | "move") {
    e.stopPropagation();
    selected = band.id;
    drag = { band, r, mode, x0: svgX(e), lo0: r.lo_hz, hi0: r.hi_hz };
    svg.setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const x = svgX(e);
    const r = drag.r;
    const rd = (v: number) => Math.round(v);
    if (drag.mode === "lo") r.lo_hz = rd(Math.min(hzOf(x), r.hi_hz * 0.95));
    else if (drag.mode === "hi") r.hi_hz = rd(Math.max(hzOf(x), r.lo_hz * 1.05));
    else {
      const dx = x - drag.x0;
      let lo = xOf(drag.lo0) + dx, hi = xOf(drag.hi0) + dx;
      if (lo < 0) { hi -= lo; lo = 0; }
      if (hi > W) { lo -= hi - W; hi = W; }
      r.lo_hz = rd(hzOf(lo)); r.hi_hz = rd(hzOf(hi));
    }
  }
  function up() { drag = null; }
</script>

<svg bind:this={svg} viewBox="0 0 {W} {H + 22}" onpointermove={move} onpointerup={up} role="application" aria-label="Spectrum with detection bands">
  {#each ticks as t}
    <line x1={xOf(t)} x2={xOf(t)} y1="0" y2={H} class="tick" />
    <text x={xOf(t)} y={H + 16} class="axis">{fmt(t)}</text>
  {/each}

  {#each spec as v, i}
    <rect x={(i / spec.length) * W + 0.5} y={H - v * H} width={W / spec.length - 1} height={v * H} class="bar" />
  {/each}

  {#each bands as b (b.id)}
    {@const out = app.frame?.features.bands.find((o) => o.id === b.id)}
    {#each b.ranges as r, ri}
      {@const x0 = xOf(r.lo_hz)}
      {@const x1 = xOf(r.hi_hz)}
      <g class="band" class:sel={b.id === selected}>
        <rect x={x0} y="0" width={Math.max(2, x1 - x0)} height={H} fill={b.color} fill-opacity={(b.id === selected ? 0.18 : 0.09) * Math.min(1, 0.4 + r.weight * 0.6)}
          stroke={b.color} stroke-opacity="0.6" stroke-dasharray={ri > 0 ? "6 4" : undefined} class="body"
          onpointerdown={(e) => down(e, b, r, "move")} role="button" tabindex="-1" />
        <rect x={x0} y={H - (out?.level ?? 0) * H} width={Math.max(2, x1 - x0)} height="3" fill={b.color} />
        <rect x={x0 - 5} y="0" width="10" height={H} class="edge" onpointerdown={(e) => down(e, b, r, "lo")} role="button" tabindex="-1" />
        <rect x={x1 - 5} y="0" width="10" height={H} class="edge" onpointerdown={(e) => down(e, b, r, "hi")} role="button" tabindex="-1" />
        <text x={(x0 + x1) / 2} y="16" class="name" fill={b.color}>{b.name}{b.ranges.length > 1 ? ` ${ri + 1}` : ""}</text>
        <text x={(x0 + x1) / 2} y="31" class="range">{fmt(r.lo_hz)}–{fmt(r.hi_hz)}{r.weight !== 1 ? ` ×${r.weight}` : ""}</text>
      </g>
    {/each}
  {/each}
</svg>

{#if sel}
  <div class="trace-head row small muted">
    <span class="grow">{sel.name}: {sel.detect === "level" ? "level" : "onset strength"} (solid) vs. trigger threshold (dashed). Marks are hits.</span>
  </div>
  <svg viewBox="0 0 {W} {TH}" class="trace" aria-label="Onset trace">
    {#each trace as t, i}{#if t.hit}<line x1={tx(i)} x2={tx(i)} y1="0" y2={TH} stroke={sel.color} stroke-opacity="0.5" />{/if}{/each}
    <polyline points={trace.map((t, i) => `${tx(i)},${ty(t.thr)}`).join(" ")} class="thr" />
    <polyline points={trace.map((t, i) => `${tx(i)},${ty(t.flux)}`).join(" ")} fill="none" stroke={sel.color} stroke-width="2" />
  </svg>
{/if}

<style>
  svg { width: 100%; display: block; background: #0a0b10; border-radius: 8px; border: 1px solid var(--line); touch-action: none; user-select: none; }
  .trace { margin-top: 4px; }
  .trace-head { margin-top: 10px; }
  .tick { stroke: #ffffff0d; }
  .axis { fill: var(--faint); font-size: 11px; text-anchor: middle; }
  .bar { fill: #ffffff2a; }
  .body { cursor: grab; }
  .edge { fill: transparent; cursor: ew-resize; }
  .edge:hover { fill: #ffffff22; }
  .name { font-size: 13px; font-weight: 600; text-anchor: middle; pointer-events: none; }
  .range { fill: var(--muted); font-size: 11px; text-anchor: middle; pointer-events: none; }
  .thr { fill: none; stroke: var(--muted); stroke-width: 1.5; stroke-dasharray: 5 4; }
</style>
