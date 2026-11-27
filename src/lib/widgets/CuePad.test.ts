// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W46 CuePad interaction tests — Wave-23 widget primitive; bundle source
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
// (`cue-pad` / `cue-cell`). Click on a cell fires onPadClick(row, col);
// per-cell `state` drives the data-state attribute, `hue` drives the CSS
// custom property, `label` renders inside the button.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import CuePad from './CuePad.svelte';

// Svelte's `*.svelte` TS shim only re-exports the default export, so
// `CuePadCell` (declared inside `<script lang="ts">`) is not reachable from
// outside via a named import. Mirror the cell shape locally per the same
// workaround pattern used for `snap-bar.fixtures.ts` consumers.
interface CuePadCell {
  readonly state?: 'idle' | 'armed' | 'playing';
  readonly hue?: number;
  readonly label?: string;
}

describe('CuePad widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the default 4×8 grid of cells with role=group', () => {
    // Container uses role="group" rather than role="grid" so its
    // <button> children pass `aria-required-children` (an aria grid
    // would require <div role="row">/<div role="gridcell"> wrappers).
    // See `EV-ACCESSIBILITY-AA` Wave 4b.8 PR 2 for the rationale.
    const { container } = render(CuePad);
    const grid = container.querySelector('.cue-pad');
    expect(grid).not.toBeNull();
    expect(grid?.getAttribute('role')).toBe('group');
    expect(grid?.getAttribute('aria-label')).toBe('Cue pad (4 rows × 8 cols)');
    expect(grid?.getAttribute('style')).toContain('grid-template-columns: repeat(8, 1fr)');
    expect(container.querySelectorAll('.cue-pad__cell').length).toBe(32);
  });

  it('renders an arbitrary rows × cols matrix with correct cell count', () => {
    const { container } = render(CuePad, { props: { rows: 2, cols: 3 } });
    expect(container.querySelectorAll('.cue-pad__cell').length).toBe(6);
    expect(container.querySelector('.cue-pad')?.getAttribute('aria-label')).toBe(
      'Cue pad (2 rows × 3 cols)',
    );
  });

  it('cells default to data-state="idle" when no fixture is supplied', () => {
    const { container } = render(CuePad, { props: { rows: 1, cols: 2 } });
    const cells = container.querySelectorAll('.cue-pad__cell');
    cells.forEach((cell) => {
      expect(cell.getAttribute('data-state')).toBe('idle');
    });
  });

  it('cells reflect the per-fixture state (idle / armed / playing)', () => {
    const cells: readonly CuePadCell[] = [
      { state: 'idle' },
      { state: 'armed' },
      { state: 'playing' },
    ];
    const { container } = render(CuePad, { props: { rows: 1, cols: 3, cells } });
    const rendered = container.querySelectorAll('.cue-pad__cell');
    expect(rendered[0]?.getAttribute('data-state')).toBe('idle');
    expect(rendered[1]?.getAttribute('data-state')).toBe('armed');
    expect(rendered[2]?.getAttribute('data-state')).toBe('playing');
  });

  it('per-cell hue propagates as the --cue-pad-hue inline style', () => {
    const cells: readonly CuePadCell[] = [{ state: 'armed', hue: 120 }];
    const { container } = render(CuePad, { props: { rows: 1, cols: 1, cells } });
    const cell = container.querySelector('.cue-pad__cell') as HTMLElement | null;
    expect(cell?.getAttribute('style')).toContain('--cue-pad-hue: 120');
  });

  it('per-cell label renders inside the button and merges into aria-label', () => {
    const cells: readonly CuePadCell[] = [{ state: 'idle', label: 'Kick' }];
    const { container } = render(CuePad, { props: { rows: 1, cols: 1, cells } });
    const cell = container.querySelector('.cue-pad__cell') as HTMLElement | null;
    expect(cell?.querySelector('.cue-pad__label')?.textContent).toBe('Kick');
    expect(cell?.getAttribute('aria-label')).toBe('Pad row 1 col 1: Kick');
  });

  it('clicking a cell fires onPadClick with (row, col)', async () => {
    const onPadClick = vi.fn();
    const { container } = render(CuePad, {
      props: { rows: 2, cols: 4, onPadClick },
    });
    const cells = container.querySelectorAll('.cue-pad__cell');
    // Index 5 in a 2×4 grid → row=1, col=1.
    expect(cells[5]).toBeDefined();
    if (cells[5] === undefined) return;
    await fireEvent.click(cells[5]);
    expect(onPadClick).toHaveBeenCalledTimes(1);
    expect(onPadClick).toHaveBeenCalledWith(1, 1);
  });

  it('clicking the last cell fires onPadClick with the bottom-right coords', async () => {
    const onPadClick = vi.fn();
    const { container } = render(CuePad, {
      props: { rows: 3, cols: 3, onPadClick },
    });
    const cells = container.querySelectorAll('.cue-pad__cell');
    expect(cells.length).toBe(9);
    const last = cells[8];
    if (last === undefined) return;
    await fireEvent.click(last);
    // Index 8 in 3×3 → row=2, col=2.
    expect(onPadClick).toHaveBeenCalledWith(2, 2);
  });
});
