<script lang="ts">
  import { app, fixtureLook, profileOf } from "../lib/store.svelte";
  import type { Room } from "../lib/types";
  import { beamOf, footprint, lightSpot } from "../lib/aim";

  let {
    room,
    editable = false,
    selected = null,
    onselect = () => {},
    selectedFocus = null,
    onselectfocus = () => {},
    height = 460,
  }: {
    room: Room;
    editable?: boolean;
    selected?: string | null;
    onselect?: (id: string | null) => void;
    selectedFocus?: string | null;
    onselectfocus?: (id: string | null) => void;
    height?: number;
  } = $props();

  const S = 100; // svg units per metre
  const pad = 60;
  const bounds = $derived.by(() => {
    const xs = room.walls.map((p) => p[0]);
    const ys = room.walls.map((p) => p[1]);
    return { x0: Math.min(0, ...xs), y0: Math.min(0, ...ys), x1: Math.max(1, ...xs), y1: Math.max(1, ...ys) };
  });
  const vb = $derived(
    `${bounds.x0 * S - pad} ${bounds.y0 * S - pad} ${(bounds.x1 - bounds.x0) * S + pad * 2} ${(bounds.y1 - bounds.y0) * S + pad * 2}`,
  );
  const fixtures = $derived(app.show!.fixtures.filter((f) => f.room_id === room.id));
  const gridX = $derived(range(Math.floor(bounds.x0), Math.ceil(bounds.x1)));
  const gridY = $derived(range(Math.floor(bounds.y0), Math.ceil(bounds.y1)));
  function range(a: number, b: number) {
    const r = [];
    for (let i = a; i <= b; i += 0.5) r.push(i);
    return r;
  }

  let svg: SVGSVGElement;
  type DragKind = "fx" | "wall" | "focus" | "radius";
  let drag: { kind: DragKind; id: string | number } | null = null;

  function toWorld(e: PointerEvent): [number, number] {
    const pt = svg.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const p = pt.matrixTransform(svg.getScreenCTM()!.inverse());
    const snap = (v: number) => Math.round((v / S) * 10) / 10;
    return [snap(p.x), snap(p.y)];
  }
  function start(e: PointerEvent, kind: DragKind, id: string | number) {
    if (kind === "fx") { onselect(id as string); onselectfocus(null); }
    if (kind === "focus" || kind === "radius") { onselectfocus(id as string); onselect(null); }
    if (!editable) return;
    if (kind === "wall" && e.altKey && room.walls.length > 3) {
      room.walls.splice(id as number, 1);
      return;
    }
    drag = { kind, id };
    svg.setPointerCapture(e.pointerId);
    e.stopPropagation();
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const [x, y] = toWorld(e);
    if (drag.kind === "fx") {
      const f = app.show!.fixtures.find((f) => f.id === drag!.id);
      if (f) { f.x = x; f.y = y; }
    } else if (drag.kind === "focus" || drag.kind === "radius") {
      const fp = room.focus.find((p) => p.id === drag!.id);
      if (!fp) return;
      if (drag.kind === "focus") { fp.x = x; fp.y = y; }
      else fp.radius = Math.max(0.3, Math.round(Math.hypot(x - fp.x, y - fp.y) * 10) / 10);
    } else {
      room.walls[drag.id as number] = [x, y];
    }
  }
  function end() { drag = null; }

  function edgeLen(i: number) {
    const a = room.walls[i], b = room.walls[(i + 1) % room.walls.length];
    return Math.hypot(b[0] - a[0], b[1] - a[1]);
  }
  function addCornerOnEdge(i: number) {
    if (!editable) return;
    const a = room.walls[i], b = room.walls[(i + 1) % room.walls.length];
    room.walls.splice(i + 1, 0, [Math.round(((a[0] + b[0]) / 2) * 10) / 10, Math.round(((a[1] + b[1]) / 2) * 10) / 10]);
  }

  function beam(x: number, y: number, rot: number, len: number, spread: number) {
    const r = (rot * Math.PI) / 180, s = (spread * Math.PI) / 180;
    const p = (a: number) => `${x + Math.cos(a) * len},${y + Math.sin(a) * len}`;
    return `${x},${y} ${p(r - s)} ${p(r + s)}`;
  }
</script>

<svg
  bind:this={svg}
  viewBox={vb}
  style="height:{height}px"
  onpointermove={move}
  onpointerup={end}
  onpointerdown={() => { onselect(null); onselectfocus(null); }}
  role="application"
>
  {#each gridX as gx}<line x1={gx * S} x2={gx * S} y1={bounds.y0 * S} y2={bounds.y1 * S} class="grid" class:major={gx % 1 === 0} />{/each}
  {#each gridY as gy}<line y1={gy * S} y2={gy * S} x1={bounds.x0 * S} x2={bounds.x1 * S} class="grid" class:major={gy % 1 === 0} />{/each}

  <polygon points={room.walls.map((p) => `${p[0] * S},${p[1] * S}`).join(" ")} class="walls" />

  {#each room.walls as _, i}
    {@const a = room.walls[i]}
    {@const b = room.walls[(i + 1) % room.walls.length]}
    <text x={((a[0] + b[0]) / 2) * S} y={((a[1] + b[1]) / 2) * S} class="dim" dy={-8}
      ondblclick={() => addCornerOnEdge(i)} role="button" tabindex="-1">{edgeLen(i).toFixed(1)} m</text>
  {/each}

  {#each room.focus ?? [] as fp (fp.id)}
    {@const cx = fp.x * S}
    {@const cy = fp.y * S}
    <g class="focus" class:sel={selectedFocus === fp.id} style="--fc:{fp.color}">
      <circle {cx} {cy} r={fp.radius * S} class="area" />
      <line x1={cx - 14} x2={cx + 14} y1={cy} y2={cy} class="cross" />
      <line x1={cx} x2={cx} y1={cy - 14} y2={cy + 14} class="cross" />
      <circle {cx} {cy} r="9" class="dot" onpointerdown={(e) => start(e, "focus", fp.id)} role="button" tabindex="-1" />
      <text x={cx} y={cy - 18} class="flabel">{fp.name}</text>
      {#if editable && selectedFocus === fp.id}
        <circle cx={cx + fp.radius * S} {cy} r="7" class="rhandle" onpointerdown={(e) => start(e, "radius", fp.id)} role="button" tabindex="-1" />
        <text x={cx + fp.radius * S} y={cy + 22} class="flabel small">{fp.radius.toFixed(1)} m</text>
      {/if}
    </g>
  {/each}

  {#each fixtures as fx (fx.id)}
    {@const look = fixtureLook(fx, app.frame?.fixtures[fx.id])}
    {@const kind = profileOf(fx)?.kind}
    {@const cx = fx.x * S}
    {@const cy = fx.y * S}
    {#if kind === "laser"}
      <polygon points={beam(cx, cy, fx.rotation, 260, 22)} fill={look.color} opacity={0.08 + 0.3 * look.intensity} />
    {:else if kind === "fog"}
      <circle {cx} {cy} r={30 + 50 * look.intensity} fill={look.color} opacity={0.25 * look.intensity} />
    {:else if fx.tilt !== null && fx.tilt !== undefined}
      <!-- Aimed: draw where the beam actually lands (floor, ceiling or wall). -->
      {@const spot = lightSpot(fx, room)!}
      {@const pts = footprint(fx, beamOf(fx, profileOf(fx)), room)}
      <polygon points={pts.map((p) => `${p.x * S},${p.y * S}`).join(" ")} fill={look.color}
        opacity={(spot.on === "ceiling" ? 0.12 : 0.22) + (spot.on === "ceiling" ? 0.2 : 0.4) * look.intensity}
        class="spot" class:ceiling={spot.on === "ceiling"} style="--c:{look.color}" />
      <line x1={cx} y1={cy} x2={spot.x * S} y2={spot.y * S} class="aimline" />
      <circle cx={spot.x * S} cy={spot.y * S} r="4" class="spotdot" style="--c:{look.color}" />
      <circle {cx} {cy} r={20} fill={look.color} opacity={0.3 * look.intensity} class="glow" />
    {:else}
      <polygon points={beam(cx, cy, fx.rotation, 120, 18)} fill={look.color} opacity={0.4 * look.intensity} />
      <circle {cx} {cy} r={28} fill={look.color} opacity={0.35 * look.intensity} class="glow" />
    {/if}
    <g class="fx" class:sel={selected === fx.id} class:dim-fx={!fx.enabled}
      onpointerdown={(e) => start(e, "fx", fx.id)} role="button" tabindex="-1">
      <circle {cx} {cy} r={16} class="body" style="--c:{look.color}" />
      <circle {cx} {cy} r={9} fill={look.color} opacity={0.15 + 0.85 * look.intensity} />
      <line x1={cx} y1={cy} x2={cx + Math.cos((fx.rotation * Math.PI) / 180) * 22} y2={cy + Math.sin((fx.rotation * Math.PI) / 180) * 22} class="dir" />
      <text x={cx} y={cy + 34} class="label">{fx.name}</text>
    </g>
  {/each}

  {#if editable}
    {#each room.walls as p, i}
      <circle cx={p[0] * S} cy={p[1] * S} r={8} class="handle" onpointerdown={(e) => start(e, "wall", i)} role="button" tabindex="-1" />
    {/each}
  {/if}
</svg>

<style>
  .spot { stroke: var(--c); stroke-width: 2; stroke-opacity: 0.6; }
  .spot.ceiling { stroke-dasharray: 8 6; }
  .aimline { stroke: #ffffff40; stroke-width: 1.5; stroke-dasharray: 4 4; }
  .spotdot { fill: var(--c); stroke: #000a; stroke-width: 1.5; }
  svg { width: 100%; display: block; background: #0a0b10; border-radius: 10px; border: 1px solid var(--line); touch-action: none; user-select: none; }
  .grid { stroke: #ffffff08; stroke-width: 1; }
  .grid.major { stroke: #ffffff12; }
  .walls { fill: #ffffff05; stroke: var(--line-2); stroke-width: 3; stroke-linejoin: round; }
  .dim { fill: var(--faint); font-size: 16px; text-anchor: middle; cursor: copy; }
  .fx { cursor: pointer; }
  .fx .body { fill: var(--panel-2); stroke: var(--line-2); stroke-width: 2; }
  .fx.sel .body { stroke: var(--accent-2); stroke-width: 3; }
  .fx.dim-fx { opacity: 0.4; }
  .dir { stroke: var(--muted); stroke-width: 2; }
  .label { fill: var(--muted); font-size: 15px; text-anchor: middle; }
  .focus .area { fill: color-mix(in srgb, var(--fc) 6%, transparent); stroke: var(--fc); stroke-opacity: 0.45; stroke-width: 2; stroke-dasharray: 8 6; pointer-events: none; }
  .focus.sel .area { stroke-opacity: 0.9; fill: color-mix(in srgb, var(--fc) 12%, transparent); }
  .focus .cross { stroke: var(--fc); stroke-width: 2; pointer-events: none; }
  .focus .dot { fill: transparent; stroke: var(--fc); stroke-width: 2; cursor: move; }
  .focus.sel .dot { fill: var(--fc); }
  .focus .rhandle { fill: var(--fc); stroke: #fff; stroke-width: 2; cursor: ew-resize; }
  .flabel { fill: var(--fc); font-size: 14px; text-anchor: middle; pointer-events: none; }
  .flabel.small { font-size: 12px; }
  .handle { fill: var(--accent); stroke: #fff; stroke-width: 2; cursor: move; }
</style>
