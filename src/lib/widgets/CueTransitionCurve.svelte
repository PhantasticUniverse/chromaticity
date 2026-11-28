<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
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
    hideModeTabs = false,
    onchange,
  }: {
    kind?: CueKind;
    cp1?: [number, number];
    cp2?: [number, number];
    snap?: CueSnap;
    duration?: string;
    hideModeTabs?: boolean;
    onchange?: (payload: ChangePayload) => void;
  } = $props();

  // --- Local state ---------------------------------------------------------
  // The component keeps its own working copy of the four cue inputs. Props
  // act as the initial value AND as a re-sync source: when the parent (e.g.
  // the W13 PR3 Playlist v2 inspector) passes new cue props (a different
  // selected cue), the local state must re-bind so the curve visual tracks
  // the parent's selection. The W13 PR2 implementation seeded `$state` from
  // props via `untrack(...)` to suppress Svelte's `state_referenced_locally`
  // warning — but the same `untrack` also snapshotted props at first mount
  // and detached subsequent prop changes from the local state, which was
  // the W14 PR1 bug surfaced by the W13 PR3 inspector mount.
  //
  // Two-step fix:
  //   1. Seed `$state` from props via `untrack(...)` so the rune initialiser
  //      captures a snapshot value rather than a live prop reference. This
  //      keeps the `state_referenced_locally` warning suppressed; the
  //      warning is the compiler's nudge that "this read happens once" and
  //      `untrack` is the documented way to confirm that intent for a seed.
  //   2. Add a `$effect` that re-reads the props reactively and assigns
  //      them back to the local state. The effect runs once on mount (a
  //      no-op since the seeds already match) and on every subsequent
  //      prop change, re-binding the visual to the new cue. The effect
  //      intentionally clobbers any in-flight handle drag — when the
  //      parent passes new cp1/cp2 it has selected a different cue, and
  //      the canonical source of truth for the visual is the new cue's
  //      curve, not the operator's stale drag against the previous cue.
  //
  // The drag → onchange → parent → prop change → effect round-trip is a
  // no-op once the values match, so this does not loop.
  let activeKind = $state<CueKind>(untrack(() => kind));
  let activeCp1 = $state<[number, number]>(untrack(() => [cp1[0], cp1[1]]));
  let activeCp2 = $state<[number, number]>(untrack(() => [cp2[0], cp2[1]]));
  let activeSnap = $state<CueSnap>(untrack(() => snap));
  let dragging = $state<'cp1' | 'cp2' | null>(null);
  let animT = $state(0);
  let svgEl: SVGSVGElement | undefined = $state();

  // Re-sync internal state from props on parent prop change. The four
  // assignments below are reactive reads; any change to a prop fires the
  // effect and re-binds the local working copy.
  $effect(() => {
    activeKind = kind;
    activeCp1 = [cp1[0], cp1[1]];
    activeCp2 = [cp2[0], cp2[1]];
    activeSnap = snap;
  });

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

  // --- Keyboard nudge (2D) ------------------------------------------------
  // Bezier control points are 2-axis: X (time progress) and Y (visual
  // progress). The 1D slider widgets in this codebase nudge a single axis;
  // bezier handles need both. WAI-ARIA APG slider pattern + a sensible
  // 2D extension:
  //
  //   ArrowLeft  / ArrowRight  → ±X (5% small / 20% with Shift)
  //   ArrowDown  / ArrowUp     → ±Y (5% small / 20% with Shift)
  //   PageDown   / PageUp      → ±Y large step
  //   Home / End               → X snap to 0 / 1 (matches aria-valuenow axis)
  //
  // Closes the DESIGN.md "Bundle alignment (2026-05-18)" §Trail item
  // "CueTransitionCurve 2D keyboard nudge". The 1D Fader / Knob /
  // CrossFader nudge shipped in #491; this handler extends the pattern to
  // bezier control points.
  const NUDGE_STEP_SMALL = 0.05;
  const NUDGE_STEP_LARGE = 0.2;
  function nudgeHandle(target: 'cp1' | 'cp2', e: KeyboardEvent): void {
    if (activeKind !== 'fade') return;
    const current = target === 'cp1' ? activeCp1 : activeCp2;
    const stepX = e.shiftKey ? NUDGE_STEP_LARGE : NUDGE_STEP_SMALL;
    const stepY = e.shiftKey ? NUDGE_STEP_LARGE : NUDGE_STEP_SMALL;
    let nextX = current[0];
    let nextY = current[1];
    let consumed = false;
    switch (e.key) {
      case 'ArrowLeft':
        nextX = current[0] - stepX;
        consumed = true;
        break;
      case 'ArrowRight':
        nextX = current[0] + stepX;
        consumed = true;
        break;
      case 'ArrowDown':
        nextY = current[1] - stepY;
        consumed = true;
        break;
      case 'ArrowUp':
        nextY = current[1] + stepY;
        consumed = true;
        break;
      case 'PageDown':
        nextY = current[1] - NUDGE_STEP_LARGE;
        consumed = true;
        break;
      case 'PageUp':
        nextY = current[1] + NUDGE_STEP_LARGE;
        consumed = true;
        break;
      case 'Home':
        nextX = 0;
        consumed = true;
        break;
      case 'End':
        nextX = 1;
        consumed = true;
        break;
    }
    if (!consumed) return;
    e.preventDefault();
    const clamped: [number, number] = [clamp01(nextX), clamp01(nextY)];
    if (target === 'cp1') {
      activeCp1 = clamped;
    } else {
      activeCp2 = clamped;
    }
    emit();
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
  {#if !hideModeTabs}
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
  {/if}

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
        role="group"
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
            onkeydown={(e) => nudgeHandle('cp1', e)}
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
            onkeydown={(e) => nudgeHandle('cp2', e)}
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
