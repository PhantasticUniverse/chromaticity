<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
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

  // ---- Mutable state -----------------------------------------------------

  // Live, draggable copy of the canonical node fixtures. Cloning the
  // readonly fixture keeps the seed pristine for unit-test
  // determinism while letting drag/undo mutate freely in-component.
  let nodes = $state<MutablePatchNode[]>(PATCH_NODES.map((n) => ({ ...n })));

  // Currently-selected node id (null = bundle's default Color-Processor
  // editorial frame).
  let selectedNodeId = $state<string | null>(null);

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
      return {
        d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`,
        x1,
        y1,
        x2,
        y2,
        color: w.color,
      };
    }),
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
    dragState = dragPointerDown(
      node.id,
      e.clientX,
      e.clientY,
      e.clientX - node.x,
      e.clientY - node.y,
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
      const nextX = e.clientX - activeDrag.offsetX;
      const nextY = e.clientY - activeDrag.offsetY;
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
              title="Drag {item} into the canvas (wiring scheduled for Phase-3+ patch-graph adapter)"
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
      <button
        type="button"
        class="patch__btn"
        title="Auto Layout (Phase-3+ scope: graph-layout algorithm against the patch-graph adapter)"
      >
        Auto Layout
      </button>
      <button
        type="button"
        class="patch__btn patch__btn--primary"
        title="Commit patch (Phase-3+ scope: runtime patch-graph IPC)"
      >
        Commit
      </button>
    </header>

    <!-- Dotted-grid background -->
    <div class="patch__grid" aria-hidden="true"></div>

    <!-- Wire layer -->
    <svg
      class="patch__wires"
      viewBox="0 0 {CANVAS_W} {CANVAS_H}"
      preserveAspectRatio="xMinYMin meet"
      role="img"
      aria-label={ariaLabel}
    >
      {#each wirePaths as wire, i (i)}
        <g>
          <path d={wire.d} stroke={wire.color} stroke-width="2" fill="none" opacity="0.9" />
          <circle cx={wire.x1} cy={wire.y1} r="3" fill={wire.color} />
          <circle cx={wire.x2} cy={wire.y2} r="3" fill={wire.color} />
        </g>
      {/each}
    </svg>

    <!-- Node layer (DOM nodes positioned absolutely in canvas coords) -->
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
          <div class="patch__node-head">
            <span class="patch__node-dot" aria-hidden="true"></span>
            <span class="patch__node-title">{node.title}</span>
          </div>
          {#each node.pins as pin, idx (pin)}
            <div
              class="patch__pin"
              style="height: {PATCH_ROW_H}px;"
              data-row-idx={idx}
              data-header-h={PATCH_HEADER_H}
            >
              <span class="patch__pin-name mono">{pin}</span>
              <span class="patch__pin-port patch__pin-port--in" aria-hidden="true"></span>
              <span class="patch__pin-port patch__pin-port--out" aria-hidden="true"></span>
            </div>
          {/each}
        </div>
      {/each}
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
                title="Route a source (Phase-3+ scope: source-picker against patch-graph adapter)"
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
    padding: var(--space-md);
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
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
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

  .patch__wires {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 1;
    pointer-events: none;
    overflow: visible;
  }

  .patch__nodes {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 100%;
  }

  /* ---------------- Node ------------------------------------------ */
  .patch__node {
    position: absolute;
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
    padding: 6px var(--space-md);
    height: 29px;
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
    background: var(--color-bg-4);
    border: 2px solid var(--node-tint, var(--color-line-3));
    box-sizing: border-box;
  }
  .patch__pin-port--in {
    left: -5px;
  }
  .patch__pin-port--out {
    right: -5px;
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
</style>
