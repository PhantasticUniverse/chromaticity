<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-23 widget primitive: clickable cue-pad grid. Renders a
  // `rows × cols` matrix of pad buttons; each cell carries an optional
  // `state` (`idle` | `armed` | `playing`), an optional per-pad `hue`
  // (HSL degrees, 0–360), and an optional `label`. Pure component — no
  // IPC dispatch; click events bubble out via `onPadClick` for the
  // parent to own state transitions.
  //
  // Source of truth: the bundle's `CuePad` definition in
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // (the `cue-pad` / `cue-cell` JSX shape with per-cell `s.c` color and
  // `s.label`). The Svelte port keeps the bundle's per-pad-color
  // contract via the `hue` field, rendering as
  // `hsl(<hue> 90% 55%)`. The W23 PR4 refactor moved cell state from a
  // flat `CuePadState[]` (PR3 invention) to the bundle-faithful object
  // shape and dropped the debug `[r,c]` coord overlay that did not
  // appear in any wireframe.
  export interface CuePadCell {
    readonly state?: 'idle' | 'armed' | 'playing';
    readonly hue?: number;
    readonly label?: string;
  }

  let {
    rows = 4,
    cols = 8,
    cells = [],
    onPadClick,
  }: {
    rows?: number;
    cols?: number;
    cells?: readonly CuePadCell[];
    onPadClick?: (row: number, col: number) => void;
  } = $props();

  const total = $derived(rows * cols);

  function cellAt(i: number): CuePadCell {
    return cells[i] ?? {};
  }
</script>

<div
  class="cue-pad"
  style="grid-template-columns: repeat({cols}, 1fr);"
  role="group"
  aria-label="Cue pad ({rows} rows × {cols} cols)"
>
  {#each Array.from({ length: total }, (_, idx) => idx) as i (i)}
    {@const r = Math.floor(i / cols)}
    {@const c = i % cols}
    {@const cell = cellAt(i)}
    <button
      type="button"
      class="cue-pad__cell"
      data-state={cell.state ?? 'idle'}
      style:--cue-pad-hue={cell.hue ?? null}
      aria-label="Pad row {r + 1} col {c + 1}{cell.label ? `: ${cell.label}` : ''}"
      onclick={() => onPadClick?.(r, c)}
    >
      {#if cell.label}<span class="cue-pad__label">{cell.label}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .cue-pad {
    display: grid;
    gap: 6px;
    width: 100%;
  }
  .cue-pad__cell {
    position: relative;
    aspect-ratio: 1 / 1;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: 0;
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      box-shadow 120ms ease;
  }
  .cue-pad__cell:hover {
    border-color: var(--color-line-3);
  }
  .cue-pad__cell:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .cue-pad__label {
    font-size: 9px;
    line-height: 1;
    pointer-events: none;
    color: inherit;
  }
  /* Armed = outlined accent; the cell carries a hue so the outline picks
     up the per-pad color. Falls back to `--color-primary` when no hue is
     assigned. */
  .cue-pad__cell[data-state='armed'] {
    background: hsl(var(--cue-pad-hue, 80) 70% 55% / 0.18);
    border-color: hsl(var(--cue-pad-hue, 80) 80% 55%);
    color: hsl(var(--cue-pad-hue, 80) 80% 75%);
    box-shadow: inset 0 0 0 1px hsl(var(--cue-pad-hue, 80) 80% 55% / 0.6);
  }
  /* Playing = solid hue fill with a soft halo, matching the bundle
     reference rendering of an active Launchpad X RGB pad. */
  .cue-pad__cell[data-state='playing'] {
    background: hsl(var(--cue-pad-hue, 80) 90% 55%);
    border-color: hsl(var(--cue-pad-hue, 80) 90% 60%);
    color: var(--color-bg-0);
    box-shadow: 0 0 calc(10px * var(--glow, 1)) hsl(var(--cue-pad-hue, 80) 90% 55% / 0.55);
  }
</style>
