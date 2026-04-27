<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-23 PR3 widget primitive: clickable cue-pad grid. Renders a
  // `rows × cols` matrix of pad buttons whose visual state is driven by a
  // flat `state` array indexed `row * cols + col`. Pure component — no IPC
  // dispatch; click events bubble out via `onPadClick` for the parent to
  // own state transitions. Modulated cells animate a pulsing edge gated
  // on `prefers-reduced-motion: reduce`.
  //
  // Source of truth: the bundle's `CuePad` definition in
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // (the `cue-pad` / `cue-cell` JSX shape). The state vocabulary
  // (`idle` / `armed` / `active` / `modulated`) is the W23 PR3 port
  // expansion of the bundle's `playing | armed | idle` triple, adding the
  // motion-cued `modulated` channel that the brief calls out.
  // Visual state vocabulary; the canonical re-export lives in `./index.ts`
  // so consumers import the type from the widget barrel.
  type CuePadState = 'idle' | 'armed' | 'active' | 'modulated';

  let {
    rows = 4,
    cols = 8,
    cells = [],
    onPadClick,
  }: {
    rows?: number;
    cols?: number;
    cells?: readonly CuePadState[];
    onPadClick?: (row: number, col: number) => void;
  } = $props();

  const total = $derived(rows * cols);

  // Modulated-pulse animation phase (0..1), advanced via rAF and consumed
  // as a CSS custom property on the grid root. Skipped when the user has
  // `prefers-reduced-motion: reduce` set, in which case modulated cells
  // render with a static accent edge (no pulse).
  let pulse = $state(0);
  let reduceMotion = $state(false);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reduceMotion = mq.matches;
    const handler = (e: MediaQueryListEvent): void => {
      reduceMotion = e.matches;
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  });

  $effect(() => {
    if (reduceMotion) {
      pulse = 0;
      return;
    }
    let raf = 0;
    const start = performance.now();
    const tick = (now: number): void => {
      const elapsed = (now - start) / 1200;
      pulse = (elapsed - Math.floor(elapsed) + 1) % 1;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  function cellState(i: number): CuePadState {
    return cells[i] ?? 'idle';
  }
</script>

<div
  class="cue-pad"
  style="grid-template-columns: repeat({cols}, 1fr); --cue-pad-pulse: {pulse};"
  role="grid"
  aria-label="Cue pad ({rows} rows × {cols} cols)"
>
  {#each Array.from({ length: total }, (_, idx) => idx) as i (i)}
    {@const r = Math.floor(i / cols)}
    {@const c = i % cols}
    {@const s = cellState(i)}
    <button
      type="button"
      class="cue-pad__cell"
      data-state={s}
      aria-label="Pad row {r + 1} col {c + 1}"
      onclick={() => onPadClick?.(r, c)}
    >
      <span class="cue-pad__coord" aria-hidden="true">[{r},{c}]</span>
    </button>
  {/each}
</div>

<style>
  .cue-pad {
    display: grid;
    gap: 4px;
    width: 100%;
  }
  .cue-pad__cell {
    position: relative;
    aspect-ratio: 1 / 1;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-xs);
    padding: 0;
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .cue-pad__cell:hover {
    border-color: var(--color-line-3);
  }
  .cue-pad__cell:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .cue-pad__coord {
    position: absolute;
    top: 2px;
    left: 4px;
    font-size: 9px;
    line-height: 1;
    opacity: 0.45;
    pointer-events: none;
  }
  .cue-pad__cell[data-state='armed'] {
    border-color: var(--color-primary);
    color: var(--color-primary);
    background: color-mix(in oklab, var(--color-primary) 10%, var(--color-bg-0));
  }
  .cue-pad__cell[data-state='active'] {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-primary);
  }
  .cue-pad__cell[data-state='active'] .cue-pad__coord {
    opacity: 0.6;
  }
  .cue-pad__cell[data-state='modulated'] {
    border-color: var(--color-cat-motion);
    color: var(--color-cat-motion);
    background: color-mix(in oklab, var(--color-cat-motion) 8%, var(--color-bg-0));
    box-shadow: 0 0 calc((4px + 6px * var(--cue-pad-pulse, 0)) * var(--glow, 1))
      var(--color-cat-motion);
  }
  @media (prefers-reduced-motion: reduce) {
    .cue-pad__cell[data-state='modulated'] {
      box-shadow: 0 0 calc(4px * var(--glow, 1)) var(--color-cat-motion);
    }
  }
</style>
