<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-13 PR2 widget primitive: cue transition curve editor. Pure
  // component — takes cue shape props (kind, cp1, cp2, snap, duration) and
  // emits an `onchange` callback when the operator drags a control handle
  // or selects a preset / mode / snap divisor. No IPC; no view-level state.
  //
  // Source of truth: `chromaticity-planning/ui-predev-bundle-v0.2-beta/cue_curve.jsx`
  // (per the Wave-13 PR2 brief and the `feedback_wireframes_normative_for_ui_ux`
  // memory note). Pure math (`bezier1d`, `presetFor`) lives in the sibling
  // `cue-transition-curve.math.ts` module so it is unit-testable without a
  // renderer; this file owns the SVG render, pointer-drag lifecycle, and
  // ghost-dot animation.
  //
  // Wave-13 PR3 (Playlist v2 view) embeds this component in the cue-row
  // inspector once the playlist scaffold lands; PR2 ships the primitive
  // and Vitest spec only — there is no view-level mount in this PR.
  import { untrack } from 'svelte';

  import {
    CUE_PRESETS,
    bezier1d,
    clamp01,
    presetFor,
    type CueKind,
    type CuePoint,
    type CueSnap,
  } from './cue-transition-curve.math';

  type ChangePayload = {
    kind: CueKind;
    cp1: [number, number];
    cp2: [number, number];
    snap: CueSnap;
  };

  let {
    kind = 'fade',
    cp1 = [0.42, 0.0],
    cp2 = [0.58, 1.0],
    snap = '1',
    duration = '00:00.5',
    onchange,
  }: {
    kind?: CueKind;
    cp1?: [number, number];
    cp2?: [number, number];
    snap?: CueSnap;
    duration?: string;
    onchange?: (payload: ChangePayload) => void;
  } = $props();

  // --- Local state ---------------------------------------------------------
  // The component keeps its own working copy of the four cue inputs. Props
  // act as the initial value; subsequent edits live in this state and bubble
  // out via `onchange`. This mirrors the React `useState(initialX)` pattern
  // in the source jsx. The seed values are read inside `untrack(...)` so the
  // rune initialiser captures the snapshot value rather than the live prop
  // reference (suppresses Svelte's `state_referenced_locally` warning).
  let activeKind = $state<CueKind>(untrack(() => kind));
  let activeCp1 = $state<[number, number]>(untrack(() => [cp1[0], cp1[1]]));
  let activeCp2 = $state<[number, number]>(untrack(() => [cp2[0], cp2[1]]));
  let activeSnap = $state<CueSnap>(untrack(() => snap));
  let dragging = $state<'cp1' | 'cp2' | null>(null);
  let animT = $state(0);
  let svgEl: SVGSVGElement | undefined = $state();

  // --- Layout constants ---------------------------------------------------
  const W = 280;
  const H = 148;
  const PAD_X = 16;
  const PAD_Y = 14;
  const IW = W - PAD_X * 2;
  const IH = H - PAD_Y * 2;

  // Project a unit-square point `[x, y]` (with y = 0 at the bottom) onto the
  // SVG viewBox coords (with y = 0 at the top). All anchor / handle / curve
  // / ghost-dot positions flow through this helper.
  function toSvg(p: CuePoint): readonly [number, number] {
    return [PAD_X + p[0] * IW, PAD_Y + (1 - p[1]) * IH];
  }

  const p0 = $derived(toSvg([0, 0]));
  const p3 = $derived(toSvg([1, 1]));
  const h1 = $derived(toSvg(activeCp1));
  const h2 = $derived(toSvg(activeCp2));
  const dotX = $derived(bezier1d(animT, 0, activeCp1[0], activeCp2[0], 1));
  const dotY = $derived(bezier1d(animT, 0, activeCp1[1], activeCp2[1], 1));
  const dot = $derived(toSvg([dotX, dotY]));
  const activePreset = $derived(presetFor(activeCp1, activeCp2));

  // --- Change emission ----------------------------------------------------
  function emit(): void {
    onchange?.({
      kind: activeKind,
      cp1: [activeCp1[0], activeCp1[1]],
      cp2: [activeCp2[0], activeCp2[1]],
      snap: activeSnap,
    });
  }

  function setKind(next: CueKind): void {
    if (activeKind === next) return;
    activeKind = next;
    emit();
  }

  function setSnap(next: CueSnap): void {
    if (activeSnap === next) return;
    activeSnap = next;
    emit();
  }

  function applyPreset(p: (typeof CUE_PRESETS)[number]): void {
    activeCp1 = [p.cp1[0], p.cp1[1]];
    activeCp2 = [p.cp2[0], p.cp2[1]];
    emit();
  }

  // --- Drag lifecycle -----------------------------------------------------
  // Pointer-down on a handle stashes which handle is dragging; window-level
  // `pointermove` / `pointerup` handlers do the projection + state update.
  // Window listeners (rather than SVG-local) match the source jsx and let
  // the operator drag past the SVG bounds without dropping the gesture.
  function startDrag(target: 'cp1' | 'cp2', e: PointerEvent): void {
    e.preventDefault();
    dragging = target;
  }

  $effect(() => {
    if (dragging === null) return;
    const onMove = (e: PointerEvent): void => {
      if (svgEl === undefined) return;
      const rect = svgEl.getBoundingClientRect();
      const innerW = rect.width - PAD_X * 2;
      const innerH = rect.height - PAD_Y * 2;
      if (innerW <= 0 || innerH <= 0) return;
      const x = clamp01((e.clientX - rect.left - PAD_X) / innerW);
      const y = clamp01(1 - (e.clientY - rect.top - PAD_Y) / innerH);
      if (dragging === 'cp1') {
        activeCp1 = [x, y];
      } else if (dragging === 'cp2') {
        activeCp2 = [x, y];
      }
      emit();
    };
    const onUp = (): void => {
      dragging = null;
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  });

  // --- Ghost-dot animation -------------------------------------------------
  // Only animates while `kind === 'fade'`; cut hides the curve entirely and
  // snap dims it. Animation period 1.6s, matching the bundle.
  $effect(() => {
    if (activeKind !== 'fade') {
      animT = 0;
      return;
    }
    let raf = 0;
    const start = performance.now();
    const tick = (now: number): void => {
      const elapsed = (now - start) / 1600;
      animT = elapsed - Math.floor(elapsed);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  // --- Static row data ----------------------------------------------------
  const KIND_TABS: ReadonlyArray<{ id: CueKind; label: string }> = [
    { id: 'cut', label: 'Cut' },
    { id: 'fade', label: 'Fade' },
    { id: 'snap', label: 'Beat Snap' },
  ];

  const SNAP_TILES: ReadonlyArray<{ id: CueSnap; label: string; sub: string }> = [
    { id: '0.25', label: '1/4', sub: 'beat' },
    { id: '0.5', label: '1/2', sub: 'bar' },
    { id: '1', label: '1', sub: 'bar' },
    { id: '2', label: '2', sub: 'bars' },
    { id: '4', label: '4', sub: 'bars' },
  ];

  const GRID_LINES: ReadonlyArray<number> = [0.25, 0.5, 0.75];
</script>

<div class="cue-curve" data-active-kind={activeKind}>
  <div class="cue-curve__kind" role="tablist" aria-label="Transition kind">
    {#each KIND_TABS as tab (tab.id)}
      <button
        type="button"
        role="tab"
        aria-selected={activeKind === tab.id}
        class="cue-curve__kind-btn"
        class:on={activeKind === tab.id}
        onclick={() => setKind(tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  {#if activeKind === 'fade'}
    <div
      class="cue-curve__presets"
      role="tablist"
      aria-label="Preset curves"
      data-active-preset={activePreset?.id ?? 'custom'}
    >
      {#each CUE_PRESETS as preset (preset.id)}
        <button
          type="button"
          role="tab"
          aria-selected={activePreset?.id === preset.id}
          class="cue-curve__preset"
          class:on={activePreset?.id === preset.id}
          onclick={() => applyPreset(preset)}
          title={preset.label}
        >
          {preset.label}
        </button>
      {/each}
      <span class="cue-curve__custom" class:on={activePreset === undefined}>Custom</span>
    </div>
  {/if}

  {#if activeKind === 'fade' || activeKind === 'snap'}
    <div class="cue-curve__svg-wrap" class:dim={activeKind === 'snap'}>
      <svg
        bind:this={svgEl}
        class="cue-curve__svg"
        viewBox="0 0 {W} {H}"
        preserveAspectRatio="none"
        role="img"
        aria-label="Cue transition curve"
      >
        <g class="cue-curve__grid">
          {#each GRID_LINES as v (`vx${v}`)}
            <line x1={PAD_X + v * IW} y1={PAD_Y} x2={PAD_X + v * IW} y2={PAD_Y + IH} />
          {/each}
          {#each GRID_LINES as v (`hy${v}`)}
            <line x1={PAD_X} y1={PAD_Y + v * IH} x2={PAD_X + IW} y2={PAD_Y + v * IH} />
          {/each}
          <rect x={PAD_X} y={PAD_Y} width={IW} height={IH} class="cue-curve__frame" />
          <line x1={p0[0]} y1={p0[1]} x2={p3[0]} y2={p3[1]} class="cue-curve__baseline" />
        </g>

        <line x1={p0[0]} y1={p0[1]} x2={h1[0]} y2={h1[1]} class="cue-curve__guide" />
        <line x1={p3[0]} y1={p3[1]} x2={h2[0]} y2={h2[1]} class="cue-curve__guide" />

        <path
          class="cue-curve__path"
          d="M {p0[0]} {p0[1]} C {h1[0]} {h1[1]}, {h2[0]} {h2[1]}, {p3[0]} {p3[1]}"
        />

        {#if activeKind === 'fade'}
          <circle class="cue-curve__ghost" cx={dot[0]} cy={dot[1]} r={4} />
        {/if}

        <circle class="cue-curve__anchor" cx={p0[0]} cy={p0[1]} r={3} />
        <circle class="cue-curve__anchor" cx={p3[0]} cy={p3[1]} r={3} />

        {#if activeKind === 'fade'}
          <g
            class="cue-curve__handle"
            class:dragging={dragging === 'cp1'}
            data-handle="cp1"
            role="slider"
            tabindex="0"
            aria-label="Bezier control point 1"
            aria-valuemin="0"
            aria-valuemax="1"
            aria-valuenow={activeCp1[0]}
            onpointerdown={(e) => startDrag('cp1', e)}
          >
            <circle cx={h1[0]} cy={h1[1]} r={10} class="cue-curve__handle-hit" />
            <circle cx={h1[0]} cy={h1[1]} r={5} class="cue-curve__handle-dot" />
            <line
              x1={h1[0] - 4}
              y1={h1[1]}
              x2={h1[0] + 4}
              y2={h1[1]}
              class="cue-curve__handle-cross"
            />
            <line
              x1={h1[0]}
              y1={h1[1] - 4}
              x2={h1[0]}
              y2={h1[1] + 4}
              class="cue-curve__handle-cross"
            />
          </g>
          <g
            class="cue-curve__handle"
            class:dragging={dragging === 'cp2'}
            data-handle="cp2"
            role="slider"
            tabindex="0"
            aria-label="Bezier control point 2"
            aria-valuemin="0"
            aria-valuemax="1"
            aria-valuenow={activeCp2[0]}
            onpointerdown={(e) => startDrag('cp2', e)}
          >
            <circle cx={h2[0]} cy={h2[1]} r={10} class="cue-curve__handle-hit" />
            <circle cx={h2[0]} cy={h2[1]} r={5} class="cue-curve__handle-dot" />
            <line
              x1={h2[0] - 4}
              y1={h2[1]}
              x2={h2[0] + 4}
              y2={h2[1]}
              class="cue-curve__handle-cross"
            />
            <line
              x1={h2[0]}
              y1={h2[1] - 4}
              x2={h2[0]}
              y2={h2[1] + 4}
              class="cue-curve__handle-cross"
            />
          </g>
        {/if}

        <text x={PAD_X} y={H - 2} class="cue-curve__axis">0s</text>
        <text x={W - PAD_X} y={H - 2} text-anchor="end" class="cue-curve__axis">{duration}</text>
      </svg>
    </div>

    {#if activeKind === 'fade'}
      <div class="cue-curve__readout">
        <span>cp1 <b>{activeCp1[0].toFixed(2)}</b>, <b>{activeCp1[1].toFixed(2)}</b></span>
        <span>cp2 <b>{activeCp2[0].toFixed(2)}</b>, <b>{activeCp2[1].toFixed(2)}</b></span>
      </div>
    {/if}

    {#if activeKind === 'snap'}
      <div class="cue-curve__snap">
        <div class="cue-curve__snap-caps">Snap to</div>
        <div class="cue-curve__snap-grid" role="tablist" aria-label="Beat snap divisor">
          {#each SNAP_TILES as tile (tile.id)}
            <button
              type="button"
              role="tab"
              aria-selected={activeSnap === tile.id}
              class="cue-curve__snap-btn"
              class:on={activeSnap === tile.id}
              onclick={() => setSnap(tile.id)}
            >
              <span class="cue-curve__snap-n">{tile.label}</span>
              <span class="cue-curve__snap-sub">{tile.sub}</span>
            </button>
          {/each}
        </div>
        <div class="cue-curve__note">Transition armed on trigger · fires at next boundary</div>
      </div>
    {/if}
  {:else}
    <div class="cue-curve__empty" aria-label="Cut transition preview">
      <svg
        viewBox="0 0 {W} {H}"
        preserveAspectRatio="none"
        class="cue-curve__svg cue-curve__svg--cut"
      >
        <rect x={PAD_X} y={PAD_Y} width={IW} height={IH} class="cue-curve__frame" />
        <line x1={PAD_X} y1={H - PAD_Y} x2={140} y2={H - PAD_Y} class="cue-curve__path" />
        <line x1={140} y1={H - PAD_Y} x2={140} y2={PAD_Y} class="cue-curve__path" />
        <line x1={140} y1={PAD_Y} x2={W - PAD_X} y2={PAD_Y} class="cue-curve__path" />
        <circle cx={140} cy={H / 2} r={3} class="cue-curve__ghost" />
      </svg>
      <div class="cue-curve__note">Instant switch at trigger point · no fade</div>
    </div>
  {/if}
</div>

<style>
  .cue-curve {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-sm) var(--space-md);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    box-shadow: var(--shadow-panel);
  }
  .cue-curve__kind {
    display: flex;
    gap: var(--space-xs);
  }
  .cue-curve__kind-btn,
  .cue-curve__preset,
  .cue-curve__snap-btn {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: inherit;
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    cursor: pointer;
  }
  .cue-curve__kind-btn:hover,
  .cue-curve__preset:hover,
  .cue-curve__snap-btn:hover {
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .cue-curve__kind-btn.on,
  .cue-curve__preset.on,
  .cue-curve__snap-btn.on {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .cue-curve__presets {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
    align-items: center;
  }
  .cue-curve__custom {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
    padding: 2px var(--space-xs);
    border-radius: var(--radius-xs);
    border: 1px dashed var(--color-line-2);
  }
  .cue-curve__custom.on {
    color: var(--color-primary);
    border-color: var(--color-primary);
  }
  .cue-curve__svg-wrap {
    width: 100%;
    aspect-ratio: 280 / 148;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .cue-curve__svg-wrap.dim {
    opacity: 0.55;
  }
  .cue-curve__svg {
    width: 100%;
    height: 100%;
    display: block;
    touch-action: none;
  }
  .cue-curve__svg--cut {
    aspect-ratio: 280 / 148;
  }
  .cue-curve__grid line {
    stroke: var(--color-line-1);
    stroke-width: 1;
    stroke-dasharray: 2 3;
    opacity: 0.7;
  }
  .cue-curve__frame {
    fill: none;
    stroke: var(--color-line-2);
    stroke-width: 1;
  }
  .cue-curve__baseline {
    stroke: var(--color-line-2);
    stroke-width: 1;
    stroke-dasharray: 1 3;
  }
  .cue-curve__guide {
    stroke: var(--color-line-3);
    stroke-width: 1;
    stroke-dasharray: 1 2;
  }
  .cue-curve__path {
    fill: none;
    stroke: var(--color-primary);
    stroke-width: 2;
    stroke-linecap: round;
    filter: drop-shadow(0 0 calc(3px * var(--glow)) var(--color-primary));
  }
  .cue-curve__ghost {
    fill: var(--color-on-primary);
    stroke: var(--color-primary);
    stroke-width: 1.5;
  }
  .cue-curve__anchor {
    fill: var(--color-text-1);
    stroke: var(--color-bg-1);
    stroke-width: 1;
  }
  .cue-curve__handle {
    cursor: grab;
    touch-action: none;
  }
  .cue-curve__handle:focus-visible {
    outline: none;
  }
  .cue-curve__handle:focus-visible .cue-curve__handle-dot {
    stroke: var(--color-primary);
    stroke-width: 2;
  }
  .cue-curve__handle.dragging {
    cursor: grabbing;
  }
  .cue-curve__handle-hit {
    fill: transparent;
    stroke: transparent;
  }
  .cue-curve__handle-dot {
    fill: var(--color-bg-1);
    stroke: var(--color-primary);
    stroke-width: 1.5;
  }
  .cue-curve__handle.dragging .cue-curve__handle-dot {
    fill: var(--color-primary);
  }
  .cue-curve__handle-cross {
    stroke: var(--color-primary);
    stroke-width: 1;
  }
  .cue-curve__handle.dragging .cue-curve__handle-cross {
    stroke: var(--color-on-primary);
  }
  .cue-curve__axis {
    fill: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: 9px;
  }
  .cue-curve__readout {
    display: flex;
    gap: var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .cue-curve__readout b {
    color: var(--color-text-0);
    font-weight: var(--font-weight-label);
  }
  .cue-curve__snap {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .cue-curve__snap-caps {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }
  .cue-curve__snap-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: var(--space-xs);
  }
  .cue-curve__snap-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: var(--space-xs);
  }
  .cue-curve__snap-n {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
  }
  .cue-curve__snap-sub {
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
    opacity: 0.8;
  }
  .cue-curve__note {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .cue-curve__empty {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
</style>
