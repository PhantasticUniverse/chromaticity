<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-31 Patch view interactivity port — extends the W19 PR1 frozen-
  // frame port with click-to-select, pointer-drag node repositioning,
  // a position-snapshot undo stack, and inspector livedata binding.
  // Promotes the view from PARTIAL to DONE per R-37 in `docs/13-risk-
  // register.md`. Bundle JSX (`chromaticity-planning/ui-predev-bundle-
  // v0.2-beta/views/patch.jsx`) is the canonical structural reference;
  // the JSX itself is also frozen-frame (no drag in the React reference)
  // so this wave goes BEYOND bundle scope per AO direction to make the
  // patch surface authorable in MVP scope.
  //
  // Locked W31 conventions:
  //   - Node positions are `$state` mutable; the read-only `PATCH_NODES`
  //     fixture is the seed (deep-cloned at component mount).
  //   - Wire bezier paths are `$derived` from the live nodes so wires
  //     re-route automatically during drag.
  //   - Pointer-drag uses the `CueTransitionCurve.svelte` $effect-based
  //     idiom: pointerdown → set `draggingId` → window-level pointermove
  //     and pointerup attach via $effect → cleanup on release.
  //   - Click-vs-drag is discriminated by total pointer travel against a
  //     4px threshold; drags ≥4px don't change selection, drags <4px
  //     promote to selection on pointerup.
  //   - Undo stack snapshots node positions on drag start; the toolbar
  //     Undo button pops one snapshot at a time. Capped at 10 entries.
  //   - Inspector livedata: `selectedNodeId` drives the inspector title
  //     and rows. Color Processor (the bundle's editorial state) is the
  //     default unselected frame; other nodes get fallback rows
  //     generated from their pin lists per
  //     `inspectorRowsForNode` in `patch-fixtures.ts`.
  //   - Inspector "Unroute" buttons clear the source on a per-row
  //     local override map; "Route" stays a no-op stub (real source-
  //     picker is Phase-3+).
  //   - Phase-3+ deferrals: Auto Layout (graph-layout algorithm), Commit
  //     (runtime patch-graph IPC), edge-creation drag (port-to-port
  //     wiring), multi-select, range-thumb editing.
  //
  // A11y: nodes get `aria-pressed={selectedNodeId === node.id}` so AT
  // announces selection. The canvas aria-label summarises node and wire
  // counts plus an interactive-state hint.
  import {
    inspectorRowsForNode,
    inspectorTitleForNode,
    type MutablePatchNode,
    PATCH_HEADER_H,
    PATCH_KIND_TINT,
    PATCH_NODES,
    PATCH_PALETTE,
    PATCH_ROW_H,
    PATCH_WIRES,
    type PatchInspectorSource,
    patchPinPosFrom,
  } from '../data/patch-fixtures';
  import Icon from '../Icon.svelte';
  import { patchModulationStore } from '../modulation/store.svelte';
  import {
    dragPointerDown,
    dragPointerMove,
    dragPointerUp,
    IDLE_DRAG_STATE,
    type DragOrClickState,
  } from '../state';

  // SVG viewbox — scaled to the canonical bundle layout (~1100x600).
  // Wires render in this same coordinate system so the SVG and absolute-
  // positioned node `<div>`s overlay perfectly.
  const CANVAS_W = 1100;
  const CANVAS_H = 600;

  // Click-vs-drag threshold in pixels. Pointer travel below this on
  // pointerup promotes to selection; equal-or-above is treated as drag
  // completion with no selection change.
  const CLICK_TRAVEL_THRESHOLD = 4;

  // Maximum undo-stack depth. Older snapshots drop off the bottom on
  // overflow; bounded to keep memory predictable across long sessions.
  const UNDO_DEPTH = 10;

  // Signal-type wire palette (keyed by source node category) so every
  // cable's signal domain is immediately legible at a glance.
  type WireSignalKind = 'audio' | 'mod' | 'midi' | 'video';

  const WIRE_SIGNALS: readonly {
    readonly kind: WireSignalKind;
    readonly label: string;
    readonly color: string;
  }[] = [
    { kind: 'audio', label: 'Audio', color: '#38bdf8' },
    { kind: 'mod', label: 'Mod', color: '#a78bfa' },
    { kind: 'midi', label: 'MIDI/OSC', color: '#a3e635' },
    { kind: 'video', label: 'Video', color: '#fbbf24' },
  ];

  const WIRE_SIGNAL_COLOR: Readonly<Record<WireSignalKind, string>> = {
    audio: '#38bdf8',
    mod: '#a78bfa',
    midi: '#a3e635',
    video: '#fbbf24',
  };

  function signalKindForSource(
    node: MutablePatchNode | undefined,
    fallbackId: string,
  ): WireSignalKind {
    const id = (node?.id ?? fallbackId).toLowerCase();
    const title = (node?.title ?? '').toLowerCase();
    if (id === 'audio' || title.includes('audio')) return 'audio';
    if (
      node?.kind === 'mod' ||
      id === 'lfo' ||
      title.includes('lfo') ||
      title.includes('envelope') ||
      title.includes('math')
    ) {
      return 'mod';
    }
    if (id === 'midi' || id === 'osc' || title.includes('midi') || title.includes('osc')) {
      return 'midi';
    }
    return 'video';
  }

  // ---- Mutable state -----------------------------------------------------

  // Live viewport size observed via ResizeObserver so the shared 1100x600
  // stage (containing BOTH SVG wires and DOM nodes) scales in lockstep.
  let viewportEl = $state<HTMLDivElement | null>(null);
  let viewportSize = $state<{ width: number; height: number }>({ width: 0, height: 0 });

  $effect(() => {
    const el = viewportEl;
    if (el === null) return;
    const update = (): void => {
      const rect = el.getBoundingClientRect();
      viewportSize = { width: rect.width, height: rect.height };
    };
    update();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry === undefined) return;
      viewportSize = {
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      };
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  // Unified scale factor applied identically to the shared stage wrapper
  // (SVG wires + DOM nodes). Falls back to 1 when unmeasured (e.g. jsdom).
  const stageScale = $derived.by(() => {
    if (viewportSize.width <= 0 || viewportSize.height <= 0) return 1;
    const fitX = viewportSize.width / CANVAS_W;
    const fitY = viewportSize.height / CANVAS_H;
    return Math.max(0.55, Math.min(1, fitX, fitY));
  });

  // Live, draggable copy of the canonical node fixtures. Cloning the
  // readonly fixture keeps the seed pristine for unit-test
  // determinism while letting drag/undo mutate freely in-component.
  let nodes = $state<MutablePatchNode[]>(PATCH_NODES.map((n) => ({ ...n })));

  // Currently-selected node id (null = bundle's default Color-Processor
  // editorial frame).
  let selectedNodeId = $state<string | null>(null);

  // Hovered wire index for cable focus + connection readout pill.
  let hoveredWireIdx = $state<number | null>(null);

  // Explicit click-vs-drag discriminated state machine (`idle` | `pending` | `dragging`).
  let dragState = $state<DragOrClickState>(IDLE_DRAG_STATE);
  const draggingId = $derived(dragState.kind === 'idle' ? null : dragState.id);

  // Position-snapshot undo stack. Each entry is the array of `(id, x, y)`
  // tuples captured immediately before a drag began.
  type PositionSnapshot = readonly {
    readonly id: string;
    readonly x: number;
    readonly y: number;
  }[];
  let undoStack = $state<PositionSnapshot[]>([]);

  // Per-row inspector source-override map. `null` value clears the source
  // (Unroute). Absent key falls back to the fixture-declared source.
  let sourceOverride = $state<Record<string, null>>({});

  // ---- Derived inspector + wire state ------------------------------------

  const selectedNode = $derived(
    selectedNodeId === null ? undefined : nodes.find((n) => n.id === selectedNodeId),
  );

  const inspectorTitle = $derived(inspectorTitleForNode(selectedNodeId, nodes));

  const inspectorRows = $derived(inspectorRowsForNode(selectedNodeId, nodes));

  const wirePaths = $derived(
    PATCH_WIRES.map((w) => {
      const [x1, y1] = patchPinPosFrom(nodes, w.from[0], w.from[1], 'out');
      const [x2, y2] = patchPinPosFrom(nodes, w.to[0], w.to[1], 'in');
      const mx = (x1 + x2) / 2;
      const sourceNode = nodes.find((n) => n.id === w.from[0]);
      const signalKind = signalKindForSource(sourceNode, w.from[0]);
      const color = WIRE_SIGNAL_COLOR[signalKind];
      return {
        d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`,
        x1,
        y1,
        x2,
        y2,
        midX: mx,
        midY: (y1 + y2) / 2,
        signalKind,
        color,
        label: `${w.from[0]}.${w.from[1]} → ${w.to[0]}.${w.to[1]}`,
      };
    }),
  );

  interface ConnectedPortInfo {
    readonly color: string;
    readonly signalKind: WireSignalKind;
    readonly wireIndices: readonly number[];
  }

  const connectedPorts = $derived.by((): Readonly<Record<string, ConnectedPortInfo>> => {
    const ports: Record<
      string,
      { color: string; signalKind: WireSignalKind; wireIndices: number[] }
    > = {};
    const addPort = (
      key: string,
      color: string,
      signalKind: WireSignalKind,
      wireIdx: number,
    ): void => {
      const existing = ports[key];
      if (existing !== undefined) {
        existing.wireIndices.push(wireIdx);
      } else {
        ports[key] = { color, signalKind, wireIndices: [wireIdx] };
      }
    };
    PATCH_WIRES.forEach((w, idx) => {
      const wire = wirePaths[idx];
      if (wire === undefined) return;
      addPort(`${w.from[0]}:${w.from[1]}:out`, wire.color, wire.signalKind, idx);
      addPort(`${w.to[0]}:${w.to[1]}:in`, wire.color, wire.signalKind, idx);
    });
    return ports;
  });

  const hoveredWire = $derived(
    hoveredWireIdx === null ? null : (wirePaths[hoveredWireIdx] ?? null),
  );

  const canUndo = $derived(undoStack.length > 0);

  const ariaLabel = $derived(
    `Patch graph — ${nodes.length} nodes, ${PATCH_WIRES.length} wires.` +
      (selectedNode === undefined
        ? ' Click a node to select; drag to reposition.'
        : ` Selected: ${selectedNode.title}.`),
  );

  // ---- Snapshot + undo helpers -------------------------------------------

  function captureSnapshot(): void {
    const snap: PositionSnapshot = nodes.map((n) => ({ id: n.id, x: n.x, y: n.y }));
    const next = [...undoStack, snap];
    // Cap at UNDO_DEPTH; drop the oldest snapshot on overflow.
    if (next.length > UNDO_DEPTH) next.shift();
    undoStack = next;
  }

  function restoreSnapshot(snap: PositionSnapshot): void {
    for (const entry of snap) {
      const node = nodes.find((n) => n.id === entry.id);
      if (node !== undefined) {
        node.x = entry.x;
        node.y = entry.y;
      }
    }
  }

  function handleUndo(): void {
    const last = undoStack[undoStack.length - 1];
    if (last === undefined) return;
    undoStack = undoStack.slice(0, -1);
    restoreSnapshot(last);
  }

  // ---- Drag lifecycle (CueTransitionCurve $effect idiom) -----------------

  function handleNodePointerDown(e: PointerEvent, node: MutablePatchNode): void {
    // Don't start drag on the inspector / palette buttons inside the
    // node — only the header surface initiates drag. Buttons inside the
    // node body call `e.stopPropagation()` themselves; this is the
    // catch-all in case a non-button child bubbles.
    if (e.button !== 0) return;
    e.preventDefault();
    if (e.currentTarget instanceof Element) {
      e.currentTarget.setPointerCapture?.(e.pointerId);
    }
    const scale = stageScale || 1;
    dragState = dragPointerDown(
      node.id,
      e.clientX,
      e.clientY,
      e.clientX / scale - node.x,
      e.clientY / scale - node.y,
    );
  }

  $effect(() => {
    if (dragState.kind === 'idle') return;
    const onMove = (e: PointerEvent): void => {
      const moved = dragPointerMove(dragState, e.clientX, e.clientY, CLICK_TRAVEL_THRESHOLD);
      if (moved.promotedToDrag) {
        captureSnapshot();
      }
      const activeDrag = moved.state;
      dragState = activeDrag;
      if (activeDrag.kind === 'idle') return;
      const node = nodes.find((n) => n.id === activeDrag.id);
      if (node === undefined) return;
      const scale = stageScale || 1;
      const nextX = e.clientX / scale - activeDrag.offsetX;
      const nextY = e.clientY / scale - activeDrag.offsetY;
      // Soft-clamp to canvas bounds (allow node to peek 40px out so
      // wires stay visible during repositioning at edges).
      node.x = Math.max(-40, Math.min(CANVAS_W - 40, nextX));
      node.y = Math.max(-40, Math.min(CANVAS_H - 40, nextY));
    };
    const onUp = (): void => {
      const outcome = dragPointerUp(dragState);
      dragState = outcome.state;
      if (outcome.action === 'click') {
        selectedNodeId = outcome.id;
      }
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  });

  // ---- Inspector source override -----------------------------------------

  function handleUnroute(rowKey: string): void {
    sourceOverride = { ...sourceOverride, [rowKey]: null };
  }

  function effectiveSource(
    rowKey: string,
    fallback: PatchInspectorSource | null,
  ): PatchInspectorSource | null {
    if (Object.prototype.hasOwnProperty.call(sourceOverride, rowKey)) return null;
    return fallback;
  }

  // ---- Live modulation matrix toolbar ------------------------------------

  const isModRunning = $derived(patchModulationStore.machine.kind === 'running');
  const activeRouteCount = $derived(patchModulationStore.routes.filter((r) => r.enabled).length);

  function handleToggleLiveMod(): void {
    if (patchModulationStore.machine.kind === 'running') {
      patchModulationStore.stop();
    } else {
      patchModulationStore.start();
    }
  }
</script>

<section class="patch" aria-label="Patch view">
  <!-- Left column: Palette -->
  <aside class="patch__panel patch__palette" aria-label="Patch palette">
    <header class="patch__panel-head">
      <h2 class="patch__panel-title">Palette</h2>
    </header>
    <div class="patch__palette-body">
      {#each PATCH_PALETTE as group (group.category)}
        <div class="patch__palette-group">
          <div class="patch__palette-cat">{group.category}</div>
          {#each group.items as item (item)}
            <div
              class="patch__palette-item"
              role="button"
              tabindex="0"
              title="Drag {item} into the canvas"
            >
              <span class="patch__palette-name">{item}</span>
            </div>
          {/each}
        </div>
      {/each}
    </div>
  </aside>

  <!-- Centre column: Node graph canvas -->
  <article class="patch__panel patch__canvas" aria-label="Patch canvas">
    <header class="patch__canvas-head">
      <h2 class="patch__panel-title">Patch · Glitch</h2>
      <div class="patch__legend" role="list" aria-label="Wire signal legend">
        {#each WIRE_SIGNALS as sig, idx (sig.kind)}
          {#if idx > 0}
            <span class="patch__legend-sep" aria-hidden="true">·</span>
          {/if}
          <span class="patch__legend-item" role="listitem">
            <span class="patch__legend-dot" style="background: {sig.color};" aria-hidden="true"
            ></span>
            <span class="patch__legend-label">{sig.label}</span>
          </span>
        {/each}
      </div>
      <span class="patch__canvas-spacer" aria-hidden="true"></span>
      <button
        type="button"
        class="patch__btn"
        title="Undo last node move"
        aria-label="Undo last node move"
        disabled={!canUndo}
        onclick={handleUndo}
      >
        <Icon name="undo" size={12} />
      </button>
      <button type="button" class="patch__btn" title="Auto Layout"> Auto Layout </button>
      <button type="button" class="patch__btn patch__btn--primary" title="Commit patch">
        Commit
      </button>
    </header>

    <!-- Scrollable / auto-fitted viewport containing the unified 1:1 stage -->
    <div class="patch__viewport" bind:this={viewportEl}>
      <!-- Dotted-grid background -->
      <div class="patch__grid" aria-hidden="true"></div>

      <div
        class="patch__stage-sizer"
        style="width: {Math.round(CANVAS_W * stageScale)}px; height: {Math.round(
          CANVAS_H * stageScale,
        )}px;"
      >
        <div
          class="patch__stage"
          style="width: {CANVAS_W}px; height: {CANVAS_H}px; transform: scale({stageScale});"
        >
          <!-- Wire layer (1:1 stage pixel coordinate system) -->
          <svg
            class="patch__wires"
            width={CANVAS_W}
            height={CANVAS_H}
            viewBox="0 0 {CANVAS_W} {CANVAS_H}"
            role="img"
            aria-label={ariaLabel}
          >
            {#each wirePaths as wire, i (i)}
              <g
                class="patch__wire"
                class:patch__wire--hovered={hoveredWireIdx === i}
                class:patch__wire--dimmed={hoveredWireIdx !== null && hoveredWireIdx !== i}
                style="color: {wire.color};"
                role="presentation"
                data-signal={wire.signalKind}
                data-connection={wire.label}
                onpointerenter={() => (hoveredWireIdx = i)}
                onpointerleave={() => {
                  if (hoveredWireIdx === i) hoveredWireIdx = null;
                }}
              >
                <title>{wire.label}</title>
                <path
                  class="patch__wire-hit"
                  d={wire.d}
                  stroke="transparent"
                  stroke-width="14"
                  fill="none"
                />
                <path
                  class="patch__wire-path"
                  d={wire.d}
                  stroke={wire.color}
                  stroke-width={hoveredWireIdx === i ? 3.2 : 2.2}
                  stroke-linecap="round"
                  fill="none"
                />
                <path
                  class="patch__wire-flow"
                  class:patch__wire-flow--active={isModRunning}
                  d={wire.d}
                  stroke="#ffffff"
                  stroke-width={hoveredWireIdx === i ? 2 : 1.4}
                  stroke-dasharray="6 14"
                  stroke-linecap="round"
                  fill="none"
                />
                <circle
                  class="patch__wire-endpoint"
                  data-endpoint="start"
                  cx={wire.x1}
                  cy={wire.y1}
                  r={hoveredWireIdx === i ? 5.2 : 4.5}
                  fill={wire.color}
                />
                <circle
                  class="patch__wire-endpoint"
                  data-endpoint="end"
                  cx={wire.x2}
                  cy={wire.y2}
                  r={hoveredWireIdx === i ? 5.2 : 4.5}
                  fill={wire.color}
                />
              </g>
            {/each}
          </svg>

          {#if hoveredWire}
            <div
              class="patch__wire-tooltip mono"
              style="left: {hoveredWire.midX}px; top: {hoveredWire.midY}px; --wire-color: {hoveredWire.color};"
              role="status"
              aria-live="polite"
            >
              <span
                class="patch__wire-tooltip-dot"
                style="background: {hoveredWire.color};"
                aria-hidden="true"
              ></span>
              <span>{hoveredWire.label}</span>
            </div>
          {/if}

          <!-- Node layer (DOM nodes positioned absolutely in the exact same 1:1 stage coords) -->
          <div class="patch__nodes" aria-hidden="false">
            {#each nodes as node (node.id)}
              <div
                class="patch__node"
                class:patch__node--selected={selectedNodeId === node.id}
                class:patch__node--dragging={draggingId === node.id}
                style="left: {node.x}px; top: {node.y}px; width: {node.w}px;
                       --node-tint: {PATCH_KIND_TINT[node.kind]};"
                role="button"
                tabindex="0"
                aria-pressed={selectedNodeId === node.id}
                aria-label="{node.title} — {node.pins.length} pins"
                onpointerdown={(e) => handleNodePointerDown(e, node)}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    selectedNodeId = node.id;
                  }
                }}
              >
                <div
                  class="patch__node-head"
                  style="height: {PATCH_HEADER_H}px; box-sizing: border-box;"
                >
                  <span class="patch__node-dot" aria-hidden="true"></span>
                  <span class="patch__node-title">{node.title}</span>
                </div>
                {#each node.pins as pin, idx (pin)}
                  {@const inPort = connectedPorts[`${node.id}:${pin}:in`]}
                  {@const outPort = connectedPorts[`${node.id}:${pin}:out`]}
                  {@const inActive =
                    inPort !== undefined &&
                    hoveredWireIdx !== null &&
                    inPort.wireIndices.includes(hoveredWireIdx)}
                  {@const outActive =
                    outPort !== undefined &&
                    hoveredWireIdx !== null &&
                    outPort.wireIndices.includes(hoveredWireIdx)}
                  <div
                    class="patch__pin"
                    style="height: {PATCH_ROW_H}px; box-sizing: border-box;"
                    data-row-idx={idx}
                    data-header-h={PATCH_HEADER_H}
                  >
                    <span class="patch__pin-name mono">{pin}</span>
                    {#if node.kind !== 'source' && node.kind !== 'mod'}
                      <span
                        class="patch__pin-port patch__pin-port--in"
                        class:patch__pin-port--connected={inPort !== undefined}
                        class:patch__pin-port--active={inActive}
                        class:patch__pin-port--dimmed={inPort !== undefined &&
                          hoveredWireIdx !== null &&
                          !inActive}
                        data-port-key="{node.id}:{pin}:in"
                        data-connected={inPort !== undefined ? 'true' : 'false'}
                        data-signal={inPort?.signalKind}
                        style={inPort !== undefined ? `--port-color: ${inPort.color};` : undefined}
                        aria-hidden="true"
                      ></span>
                    {/if}
                    {#if node.kind !== 'output'}
                      <span
                        class="patch__pin-port patch__pin-port--out"
                        class:patch__pin-port--connected={outPort !== undefined}
                        class:patch__pin-port--active={outActive}
                        class:patch__pin-port--dimmed={outPort !== undefined &&
                          hoveredWireIdx !== null &&
                          !outActive}
                        data-port-key="{node.id}:{pin}:out"
                        data-connected={outPort !== undefined ? 'true' : 'false'}
                        data-signal={outPort?.signalKind}
                        style={outPort !== undefined
                          ? `--port-color: ${outPort.color};`
                          : undefined}
                        aria-hidden="true"
                      ></span>
                    {/if}
                  </div>
                {/each}
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <!-- Live Modulation toolbar strip -->
    <div class="patch__mod-bar" role="region" aria-label="Live modulation">
      <button
        type="button"
        class="patch__btn patch__mod-toggle"
        class:patch__btn--primary={isModRunning}
        aria-pressed={isModRunning}
        onclick={handleToggleLiveMod}
      >
        <span class="patch__mod-dot" class:patch__mod-dot--live={isModRunning} aria-hidden="true"
        ></span>
        {isModRunning ? 'Stop Live Mod' : 'Start Live Mod'}
      </button>
      <span class="patch__mod-count mono" aria-live="polite">
        {activeRouteCount}/{patchModulationStore.routes.length} active
      </span>
      <div class="patch__mod-routes">
        {#each patchModulationStore.routes as route (route.id)}
          {@const ctrlVal = patchModulationStore.lastOutputs.get(route.targetControlIndex) ?? 0}
          <button
            type="button"
            class="patch__mod-chip"
            class:patch__mod-chip--disabled={!route.enabled}
            aria-pressed={route.enabled}
            aria-label="Toggle route {route.source} to ctrl_{route.targetControlIndex}"
            title="{route.source} ({route.op}) → ctrl_{route.targetControlIndex}"
            onclick={() => patchModulationStore.toggleRoute(route.id)}
          >
            <span class="patch__mod-chip-src mono">{route.source}</span>
            <span class="patch__mod-chip-arrow" aria-hidden="true">→</span>
            <span class="patch__mod-chip-dst mono">ctrl_{route.targetControlIndex}</span>
            <span class="patch__mod-chip-val mono">{ctrlVal.toFixed(2)}</span>
          </button>
        {/each}
      </div>
    </div>
  </article>

  <!-- Right column: Inspector -->
  <aside class="patch__panel patch__inspector" aria-label="Patch inspector">
    <header class="patch__panel-head">
      <h2 class="patch__panel-title">{inspectorTitle}</h2>
    </header>
    <div class="patch__inspector-body">
      {#each inspectorRows as row (row.key)}
        {@const liveSource = effectiveSource(row.key, row.source)}
        <div class="patch__inspector-row">
          <div class="patch__inspector-head">
            <span class="patch__inspector-key mono">{row.key}</span>
            <span class="patch__inspector-spacer" aria-hidden="true"></span>
            <span class="patch__inspector-value mono">
              {row.value.toFixed(2)}{row.unit}
            </span>
          </div>
          <div class="patch__range" aria-hidden="true">
            <div class="patch__range-fill" style="width: {row.value * 100}%"></div>
            <div class="patch__range-thumb" style="left: {row.value * 100}%"></div>
          </div>
          {#if liveSource}
            <div class="patch__inspector-source">
              <span class="patch__inspector-icon"><Icon name={liveSource.icon} size={10} /></span>
              <span class="patch__inspector-source-name mono" style="color: {liveSource.tint}">
                {liveSource.name}
              </span>
              <span class="patch__inspector-spacer" aria-hidden="true"></span>
              <button
                type="button"
                class="patch__btn patch__btn--ghost patch__btn--sm"
                title="Unroute {row.key}"
                aria-label="Unroute {row.key}"
                onclick={() => handleUnroute(row.key)}
              >
                <Icon name="x" size={10} />
              </button>
            </div>
          {:else}
            <div class="patch__inspector-source patch__inspector-source--empty">
              <span class="patch__inspector-source-empty mono">no source</span>
              <span class="patch__inspector-spacer" aria-hidden="true"></span>
              <button
                type="button"
                class="patch__btn patch__btn--ghost patch__btn--sm"
                title="Route a source"
              >
                <Icon name="plus" size={10} /> route
              </button>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </aside>
</section>

<style>
  /* ---------------- Layout (3-column grid) ------------------------- */
  .patch {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr) 300px;
    gap: var(--space-md);
    align-items: stretch;
    height: 100%;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .patch {
      grid-template-columns: 180px minmax(0, 1fr) 240px;
      gap: var(--space-sm);
    }
  }

  /* ---------------- Shared panel chrome ---------------------------- */
  .patch__panel {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .patch__panel-head {
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-shrink: 0;
  }
  .patch__panel-title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
    font-weight: 600;
  }

  /* ---------------- Buttons --------------------------------------- */
  .patch__btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-xs);
    padding: 4px var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
    transition:
      background 120ms ease,
      color 120ms ease,
      border-color 120ms ease;
  }
  .patch__btn:hover {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .patch__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .patch__btn--primary {
    background: var(--color-primary);
    color: var(--color-bg-0);
    border-color: var(--color-primary);
    font-weight: 600;
  }
  .patch__btn--primary:hover {
    filter: brightness(1.05);
    color: var(--color-bg-0);
  }
  .patch__btn--ghost {
    background: transparent;
    border-color: transparent;
    color: var(--color-text-2);
  }
  .patch__btn--ghost:hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .patch__btn--sm {
    padding: 2px var(--space-xs);
    font-size: var(--font-size-caps-sm);
  }

  /* ---------------- Palette (left column) ------------------------- */
  .patch__palette-body {
    overflow: auto;
    padding: var(--space-md) var(--space-md) 28px;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .patch__palette-group {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .patch__palette-cat {
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .patch__palette-item {
    padding: 5px var(--space-md);
    border-radius: var(--radius-xs);
    cursor: grab;
    transition: background 120ms ease;
  }
  .patch__palette-item:hover,
  .patch__palette-item:focus-visible {
    background: var(--color-bg-1);
    outline: none;
  }
  .patch__palette-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }

  /* ---------------- Canvas (centre column) ------------------------ */
  .patch__canvas {
    position: relative;
    background: radial-gradient(circle at 50% 50%, var(--color-bg-2), var(--color-bg-0));
    overflow: hidden;
  }
  .patch__canvas-head {
    position: relative;
    z-index: 2;
    padding: var(--space-sm) var(--space-md);
    background: color-mix(in oklab, var(--color-bg-3) 85%, transparent);
    backdrop-filter: blur(6px);
    border-bottom: 1px solid var(--color-line-2);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-shrink: 0;
  }
  .patch__canvas-spacer {
    flex: 1;
  }
  .patch__legend {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px;
    background: color-mix(in oklab, var(--color-bg-1) 78%, transparent);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    flex-shrink: 0;
  }
  .patch__legend-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }
  .patch__legend-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .patch__legend-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-1);
    letter-spacing: 0.03em;
  }
  .patch__legend-sep {
    color: var(--color-text-3);
    font-size: var(--font-size-caps-sm);
    line-height: 1;
    user-select: none;
  }

  /* Auto-fitted / scrollable viewport for the unified 1:1 graph stage */
  .patch__viewport {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    min-width: 0;
    overflow: auto;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Dotted grid (matches bundle's `radial-gradient` background) */
  .patch__grid {
    position: absolute;
    inset: 0;
    background-image: radial-gradient(circle, var(--color-line-2) 1px, transparent 1px);
    background-size: 24px 24px;
    opacity: 0.35;
    pointer-events: none;
    z-index: 0;
  }

  .patch__stage-sizer {
    position: relative;
    flex-shrink: 0;
    z-index: 1;
  }

  .patch__stage {
    position: relative;
    transform-origin: 0 0;
  }

  .patch__wires {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 1;
    pointer-events: none;
    overflow: visible;
  }
  .patch__wire {
    pointer-events: stroke;
    cursor: pointer;
    transition: opacity 120ms ease;
  }
  .patch__wire-hit {
    pointer-events: stroke;
  }
  .patch__wire-path {
    opacity: 0.9;
    transition:
      opacity 120ms ease,
      stroke-width 120ms ease,
      filter 120ms ease;
  }
  .patch__wire--hovered .patch__wire-path {
    opacity: 1;
    filter: drop-shadow(0 0 6px currentColor);
  }
  .patch__wire-flow {
    pointer-events: none;
    opacity: 0;
    transition: opacity 180ms ease;
  }
  .patch__wire-flow--active {
    opacity: 0.48;
    animation: patch-wire-flow 1.4s linear infinite;
  }
  .patch__wire--hovered .patch__wire-flow--active {
    opacity: 0.72;
  }
  @keyframes patch-wire-flow {
    from {
      stroke-dashoffset: 20;
    }
    to {
      stroke-dashoffset: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .patch__wire-flow--active {
      animation: none;
    }
  }
  .patch__wire--dimmed {
    opacity: 0.22;
  }
  .patch__wire-tooltip {
    position: absolute;
    z-index: 4;
    transform: translate(-50%, -125%);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    background: color-mix(in oklab, var(--color-bg-0) 92%, transparent);
    border: 1px solid
      color-mix(in oklab, var(--wire-color, var(--color-primary)) 65%, var(--color-line-2));
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
    color: var(--color-text-0);
    font-size: var(--font-size-mono-sm);
    white-space: nowrap;
    pointer-events: none;
  }
  .patch__wire-tooltip-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .patch__nodes {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
  }

  /* ---------------- Node ------------------------------------------ */
  .patch__node {
    position: absolute;
    pointer-events: auto;
    box-sizing: border-box;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-top: 2px solid var(--node-tint, var(--color-line-3));
    border-radius: var(--radius-md);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.4);
    cursor: grab;
    user-select: none;
    touch-action: none;
  }
  .patch__node:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .patch__node--selected {
    border-color: var(--color-primary);
    box-shadow:
      0 8px 20px rgba(0, 0, 0, 0.4),
      0 0 0 1px var(--color-primary);
  }
  .patch__node--dragging {
    cursor: grabbing;
    opacity: 0.85;
    box-shadow: 0 16px 32px rgba(0, 0, 0, 0.55);
  }
  .patch__node-head {
    padding: 5px var(--space-md);
    height: 27px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    border-bottom: 1px solid var(--color-line-1);
  }
  .patch__node-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--node-tint, var(--color-line-3));
    flex-shrink: 0;
  }
  .patch__node-title {
    flex: 1;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Pin row (height set inline so geometry stays tied to fixtures). */
  .patch__pin {
    position: relative;
    padding: 0 16px;
    display: flex;
    align-items: center;
    box-sizing: border-box;
  }
  .patch__pin-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .patch__pin-port {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-bg-0);
    border: 1.5px solid
      color-mix(in oklab, var(--node-tint, var(--color-line-3)) 55%, var(--color-line-2));
    box-sizing: border-box;
    transition:
      transform 120ms ease,
      box-shadow 120ms ease,
      opacity 120ms ease;
  }
  .patch__pin-port--connected {
    background: var(--port-color);
    border: 1.5px solid color-mix(in oklab, var(--port-color) 65%, #ffffff);
    box-shadow:
      0 0 0 2px color-mix(in oklab, var(--port-color) 26%, transparent),
      0 0 8px color-mix(in oklab, var(--port-color) 55%, transparent);
  }
  .patch__pin-port--active {
    transform: translateY(-50%) scale(1.2);
    box-shadow:
      0 0 0 3px color-mix(in oklab, var(--port-color) 38%, transparent),
      0 0 12px var(--port-color);
  }
  .patch__pin-port--dimmed {
    opacity: 0.35;
  }
  .patch__pin-port--in {
    left: -6px;
  }
  .patch__pin-port--out {
    right: -6px;
  }

  /* The local `mono` utility — bundle's `<span className="mono">` analog. */
  :global(.patch .mono),
  .mono {
    font-family: var(--font-family-mono);
  }

  /* ---------------- Inspector (right column) ---------------------- */
  .patch__inspector-body {
    overflow: auto;
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
  }
  .patch__inspector-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .patch__inspector-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
  }
  .patch__inspector-key {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    letter-spacing: 0.02em;
  }
  .patch__inspector-spacer {
    flex: 1;
  }
  .patch__inspector-value {
    font-size: var(--font-size-mono-md);
    color: var(--color-primary);
    font-variant-numeric: tabular-nums;
  }

  .patch__range {
    position: relative;
    height: 6px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
  }
  .patch__range-fill {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    background: var(--color-primary);
    border-radius: var(--radius-full);
    opacity: 0.85;
  }
  .patch__range-thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-primary);
    transform: translate(-50%, -50%);
    box-shadow: 0 0 6px color-mix(in oklab, var(--color-primary) 40%, transparent);
  }

  .patch__inspector-source {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding-left: 2px;
    min-height: 20px;
  }
  .patch__inspector-icon {
    flex-shrink: 0;
    color: var(--color-text-2);
  }
  .patch__inspector-source-name {
    font-size: var(--font-size-caps-sm);
    letter-spacing: 0.02em;
  }
  .patch__inspector-source-empty {
    font-family: var(--font-family-mono);
    font-size: 9px;
    color: var(--color-text-3);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    opacity: 0.6;
  }

  /* ---------------- Live Modulation bar --------------------------- */
  .patch__mod-bar {
    position: relative;
    z-index: 2;
    flex-shrink: 0;
    padding: var(--space-xs) var(--space-md);
    background: color-mix(in oklab, var(--color-bg-3) 88%, transparent);
    backdrop-filter: blur(6px);
    border-top: 1px solid var(--color-line-2);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
  }
  .patch__mod-toggle {
    flex-shrink: 0;
  }
  .patch__mod-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
    flex-shrink: 0;
  }
  .patch__mod-dot--live {
    background: currentColor;
  }
  .patch__mod-count {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-2);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .patch__mod-routes {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    overflow-x: auto;
    min-width: 0;
    flex: 1;
  }
  .patch__mod-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-sm);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-size: var(--font-size-caps-sm);
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .patch__mod-chip:hover {
    border-color: var(--color-primary);
  }
  .patch__mod-chip--disabled {
    opacity: 0.45;
  }
  .patch__mod-chip-arrow {
    color: var(--color-text-3);
  }
  .patch__mod-chip-val {
    color: var(--color-primary);
    font-variant-numeric: tabular-nums;
  }
</style>
