<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-19 PR1 Patch view full port — promotes the previously-disabled
  // sidebar entry to a routed view per the canonical bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/patch.jsx`,
  // the `PatchView` component mapped to `id: "patch"` in the bundle's
  // VIEWS array). Replaces the W16 PR2 sidebar disabled stub. The
  // bundle JSX is the canonical structural reference per the
  // wireframe-alignment memory (`feedback_wireframes_normative_for_ui_ux.md`).
  //
  // Surface ported (mapping the bundle's three columns):
  //   left   — Palette (5 categories: Sources / Modulators / Scenes /
  //            Processors / Outputs).
  //   centre — Node-graph canvas with dotted-grid background, six nodes
  //            (audio / scene / color / lfo / midi / out), six wires
  //            rendered as SVG cubic-bezier curves between port anchors.
  //   right  — Inspector for the currently-selected node (Color
  //            Processor in the bundle's frozen frame), with four
  //            param rows (hue / saturation / brightness / contrast),
  //            inline range bar, and source-routing readout.
  //
  // Locked W19 PR1 deviations from the bundle:
  //   - No drag/drop. Bundle's React-managed selection / drag / commit
  //     interactivity is out of scope; this port renders the same node
  //     graph as a static visual stub. The Auto Layout / Commit / Undo
  //     toolbar buttons remain visible but are no-op stubs (post-Phase-3
  //     scope when the patch-graph runtime adapter lands).
  //   - Bundle uses an `Icon` component for the toolbar undo / inspector
  //     midi+link+x+plus glyphs. PR3 in this same wave is porting the
  //     canonical Icon component; W19 PR1 self-contains by inlining the
  //     same SVG path data the bundle's `components.jsx::Icon` exposes,
  //     so no cross-PR import dependency. Post-merge of PR3, a follow-up
  //     can swap the inline SVG to <Icon name="..." /> if desired.
  //   - Palette items render as static visual rows; the bundle's hover
  //     swap to `--surface-sunken` is preserved via `:hover` pseudo, but
  //     the cursor `grab` affordance is stub-level (no real drag).
  //   - Node-graph SVG is a single layer at z-index 1 (under the node
  //     DOM) so wire endpoints visually meet the port circles. Header
  //     toolbar floats at z-index 2 with `backdrop-filter: blur(6px)`
  //     mirroring the bundle.
  //
  // A11y: the SVG canvas is `role="img"` with an `aria-label` summarising
  // node and wire counts. Each node header repeats its title as
  // accessible text inside the rounded rectangle. Palette items and
  // inspector rows are reachable via keyboard tab order; the toolbar
  // buttons announce their stub-state via `title`.
  import {
    PATCH_HEADER_H,
    PATCH_INSPECTOR_ROWS,
    PATCH_INSPECTOR_TITLE,
    PATCH_KIND_TINT,
    PATCH_NODES,
    PATCH_PALETTE,
    PATCH_ROW_H,
    PATCH_WIRES,
    patchPinPos,
  } from '../data/patch-fixtures';

  // SVG viewbox — scaled to the canonical bundle layout (~1100x600).
  // Wires render in this same coordinate system so the SVG and absolute-
  // positioned node `<div>`s overlay perfectly.
  const CANVAS_W = 1100;
  const CANVAS_H = 600;

  // Pre-compute wire bezier paths so the template stays declarative.
  const wirePaths = PATCH_WIRES.map((w) => {
    const [x1, y1] = patchPinPos(w.from[0], w.from[1], 'out');
    const [x2, y2] = patchPinPos(w.to[0], w.to[1], 'in');
    const mx = (x1 + x2) / 2;
    return {
      d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`,
      x1,
      y1,
      x2,
      y2,
      color: w.color,
    };
  });

  const ariaLabel =
    `Patch graph — ${PATCH_NODES.length} nodes, ${PATCH_WIRES.length} wires.` +
    ` Frozen-frame visual stub; drag and commit interactivity ship post-Phase-3.`;
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
              title="Drag a {item} into the canvas — wired post-MVP"
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
      <button type="button" class="patch__btn" title="Undo — wired post-MVP" aria-label="Undo">
        <svg width="12" height="12" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M3 8h7a3 3 0 0 1 0 6H7"
            stroke="currentColor"
            stroke-width="1.5"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M5 5L3 8l2 3"
            stroke="currentColor"
            stroke-width="1.5"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <button type="button" class="patch__btn" title="Auto-layout — wired post-MVP">
        Auto Layout
      </button>
      <button
        type="button"
        class="patch__btn patch__btn--primary"
        title="Commit patch — wired post-MVP"
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
      {#each PATCH_NODES as node (node.id)}
        <div
          class="patch__node"
          style="left: {node.x}px; top: {node.y}px; width: {node.w}px;
                 --node-tint: {PATCH_KIND_TINT[node.kind]};"
          role="group"
          aria-label="{node.title} — {node.pins.length} pins"
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
      <h2 class="patch__panel-title">{PATCH_INSPECTOR_TITLE}</h2>
    </header>
    <div class="patch__inspector-body">
      {#each PATCH_INSPECTOR_ROWS as row (row.key)}
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
          {#if row.source}
            <div class="patch__inspector-source">
              {#if row.source.icon === 'midi'}
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 16 16"
                  aria-hidden="true"
                  class="patch__inspector-icon"
                >
                  <circle
                    cx="8"
                    cy="8"
                    r="6"
                    stroke="currentColor"
                    stroke-width="1.2"
                    fill="none"
                  />
                  <circle cx="5" cy="8" r="1" fill="currentColor" />
                  <circle cx="8" cy="5" r="1" fill="currentColor" />
                  <circle cx="11" cy="8" r="1" fill="currentColor" />
                </svg>
              {:else}
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 16 16"
                  aria-hidden="true"
                  class="patch__inspector-icon"
                >
                  <path
                    d="M6 10a3 3 0 0 1 0-4l2-2a3 3 0 0 1 4 4"
                    stroke="currentColor"
                    stroke-width="1.4"
                    fill="none"
                    stroke-linecap="round"
                  />
                  <path
                    d="M10 6a3 3 0 0 1 0 4l-2 2a3 3 0 0 1-4-4"
                    stroke="currentColor"
                    stroke-width="1.4"
                    fill="none"
                    stroke-linecap="round"
                  />
                </svg>
              {/if}
              <span class="patch__inspector-source-name mono" style="color: {row.source.tint}">
                {row.source.name}
              </span>
              <span class="patch__inspector-spacer" aria-hidden="true"></span>
              <button
                type="button"
                class="patch__btn patch__btn--ghost patch__btn--sm"
                title="Unroute — wired post-MVP"
                aria-label="Unroute {row.key}"
              >
                <svg width="10" height="10" viewBox="0 0 16 16" aria-hidden="true">
                  <path
                    d="M4 4l8 8M12 4l-8 8"
                    stroke="currentColor"
                    stroke-width="1.4"
                    fill="none"
                    stroke-linecap="round"
                  />
                </svg>
              </button>
            </div>
          {:else}
            <div class="patch__inspector-source patch__inspector-source--empty">
              <span class="patch__inspector-source-empty mono">no source</span>
              <span class="patch__inspector-spacer" aria-hidden="true"></span>
              <button
                type="button"
                class="patch__btn patch__btn--ghost patch__btn--sm"
                title="Route a source — wired post-MVP"
              >
                <svg width="10" height="10" viewBox="0 0 16 16" aria-hidden="true">
                  <path
                    d="M8 3v10M3 8h10"
                    stroke="currentColor"
                    stroke-width="1.4"
                    fill="none"
                    stroke-linecap="round"
                  />
                </svg>
                route
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
