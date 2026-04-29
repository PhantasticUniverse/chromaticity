// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W62 MIDI view tests — exercise the W18 PR3 + W23 PR3-4 + W24 PR2 routed-
// view surface (tab vocabulary, learn-mode toggle round-trip, binding table
// render + filter, snap grid, devices allowlist toggle, virtual pad cycle,
// BPM sync chain, MTC readout, MPE banner gate). MIDI.svelte is a frozen-
// fixture port — no IPC, no shared store writes — so the only global stub
// required is `matchMedia` (per `feedback_vitest_matchmedia_stubglobal.md`)
// because the view's learn-mode `$effect` queries
// `prefers-reduced-motion: reduce` on every render.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// Most tests stay outside learn mode so the rAF + setInterval pair the
// learn `$effect` spawns is never started; the two tests that DO toggle
// learn mode rely on `cleanup()` running the `$effect`'s return-callback,
// which cancels the rAF and clears the intervals.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import MIDI from './MIDI.svelte';
import {
  BPM_SYNC_SOURCES,
  MIDI_DEVICES,
  MIDI_LEARN_FIXTURE,
  MIDI_MTC,
  MIDI_ROWS_BY_TAB,
  MIDI_SNAP_SLOTS,
  MIDI_TABS,
  MPE_BANNER_COPY,
  RELATIVE_ENCODER_MODES,
} from '../data/midi-fixtures';

function stubMatchMedia(reduce: boolean): void {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((query: string) => ({
      matches: reduce && query.includes('prefers-reduced-motion: reduce'),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  );
}

describe('MIDI view', () => {
  beforeEach(() => {
    // Default: reduced-motion = true. This short-circuits the learn-mode
    // `$effect` so the rAF + setInterval pair never spawns even when a
    // test toggles learn mode — keeps the suite jsdom-clean and
    // deterministic.
    stubMatchMedia(true);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    cleanup();
  });

  it('renders the section landmark, all 7 tabs, and defaults to the global tab', () => {
    const { getByRole, getAllByRole } = render(MIDI);
    expect(getByRole('region', { name: /MIDI mappings and devices/i })).toBeInTheDocument();
    const tabs = getAllByRole('tab');
    expect(tabs.length).toBe(MIDI_TABS.length);
    expect(tabs.length).toBe(7);
    // Global is the default active tab.
    const active = tabs.filter((t) => t.getAttribute('aria-selected') === 'true');
    expect(active.length).toBe(1);
    expect(active[0]?.textContent?.trim()).toBe('Global');
  });

  it('renders the global tab binding table with one row per fixture entry', () => {
    const { container } = render(MIDI);
    const tbody = container.querySelector('.midi__table tbody');
    expect(tbody).not.toBeNull();
    const rows = tbody?.querySelectorAll('tr') ?? [];
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.global.length);
    // Targets render in the row's row-header cell.
    const targets = Array.from(rows).map((r) => r.querySelector('th')?.textContent?.trim());
    expect(targets).toEqual(MIDI_ROWS_BY_TAB.global.map((r) => r.target));
  });

  it('switching to the snaps tab swaps in the snap grid and hides the binding table', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    // Binding table no longer rendered.
    expect(container.querySelector('.midi__table')).toBeNull();
    // 16 slot buttons render; the gold-preset slot carries the gold class.
    const slots = container.querySelectorAll('.midi__snap');
    expect(slots.length).toBe(MIDI_SNAP_SLOTS.length);
    expect(slots.length).toBe(16);
    expect(container.querySelectorAll('.midi__snap--gold').length).toBe(1);
    // Snap-mode default is "Apply".
    const applyBtn = getByRole('button', { name: 'Apply' });
    expect(applyBtn).toHaveAttribute('aria-pressed', 'true');
  });

  it('snap-mode buttons toggle aria-pressed mutually-exclusively', async () => {
    const { getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    const applyBtn = getByRole('button', { name: 'Apply' });
    const saveBtn = getByRole('button', { name: 'Save' });
    const clearBtn = getByRole('button', { name: 'Clear' });
    expect(applyBtn).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(saveBtn);
    expect(applyBtn).toHaveAttribute('aria-pressed', 'false');
    expect(saveBtn).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(clearBtn);
    expect(saveBtn).toHaveAttribute('aria-pressed', 'false');
    expect(clearBtn).toHaveAttribute('aria-pressed', 'true');
  });

  it('switching to the devices tab renders one allowlist row per fixture device', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    const rows = container.querySelectorAll('.midi__device-row');
    expect(rows.length).toBe(MIDI_DEVICES.length);
    expect(rows.length).toBe(8);
    // The MPE-warned Seaboard renders the MPE pill on its name cell.
    const seab = Array.from(rows).find((r) => r.textContent?.includes('Seaboard'));
    expect(seab).toBeDefined();
    expect(seab?.querySelector('.midi__pill--info')?.textContent?.trim()).toBe('MPE');
  });

  it('clicking a device allowlist toggle flips its aria-pressed state', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    // First device (Launchpad X) is allowed by default → aria-pressed=true.
    const lpxRow = container.querySelectorAll('.midi__device-row')[0] as HTMLElement;
    expect(lpxRow).not.toBeNull();
    const toggle = lpxRow.querySelector('.midi__device-toggle-btn') as HTMLButtonElement;
    expect(toggle.getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    // Row gets the "off" modifier class.
    expect(lpxRow.classList.contains('midi__device-row--off')).toBe(true);
    // Re-toggle to flip back.
    await fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-pressed')).toBe('true');
  });

  it('MPE banner only renders on the devices tab and reads the canonical copy', async () => {
    const { container, getByRole, queryByRole } = render(MIDI);
    // Default tab (global): banner not rendered.
    expect(queryByRole('status')).toBeNull();
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    const banner = container.querySelector('.midi__banner');
    expect(banner).not.toBeNull();
    expect(banner?.getAttribute('role')).toBe('status');
    expect(banner?.textContent).toContain(MPE_BANNER_COPY);
  });

  it('learn toggle flips aria-pressed and reveals the learn-mode panel', async () => {
    const { container, getByRole, queryByRole } = render(MIDI);
    const learnBtn = getByRole('button', { name: /Click to map/ });
    expect(learnBtn.getAttribute('aria-pressed')).toBe('false');
    // Panel not yet mounted.
    expect(queryByRole('button', { name: /Close learn panel/i })).toBeNull();
    await fireEvent.click(learnBtn);
    // Same button now reads "Cancel Learn" with aria-pressed=true.
    const cancel = getByRole('button', { name: /Cancel Learn/ });
    expect(cancel.getAttribute('aria-pressed')).toBe('true');
    // Learn panel header is now rendered.
    expect(container.querySelector('.midi__panel--learn')).not.toBeNull();
    expect(getByRole('button', { name: /Close learn panel/i })).toBeInTheDocument();
    // Bind/Skip action buttons land.
    expect(getByRole('button', { name: 'Bind' })).toBeInTheDocument();
    expect(getByRole('button', { name: 'Skip' })).toBeInTheDocument();
    // Round-trip: clicking Cancel Learn closes the panel.
    await fireEvent.click(cancel);
    expect(container.querySelector('.midi__panel--learn')).toBeNull();
  });

  it('learn-mode panel highlights the auto-detected encoder mode pill', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    const pills = container.querySelectorAll('.midi__pill--encoder');
    expect(pills.length).toBe(RELATIVE_ENCODER_MODES.length);
    const active = Array.from(pills).filter((p) => p.getAttribute('aria-pressed') === 'true');
    expect(active.length).toBe(1);
    // The fixture's detected mode (currently 2's-complement) is the active pill.
    expect(active[0]?.textContent).toContain(MIDI_LEARN_FIXTURE.detectedMode);
    expect(active[0]?.textContent).toContain('✓');
  });

  it('filter input narrows the binding table to substring matches and shows empty-state otherwise', async () => {
    const { container, getByLabelText } = render(MIDI);
    const filter = getByLabelText('filter MIDI bindings') as HTMLInputElement;
    // Filter 'hue' matches exactly one global row (meta.hue) out of three —
    // strictly fewer than the full set, but non-empty.
    await fireEvent.input(filter, { target: { value: 'hue' } });
    const filtered = container.querySelectorAll('.midi__table tbody tr');
    expect(filtered.length).toBe(1);
    expect(filtered[0]?.textContent).toContain('meta.hue');
    // Now type something that matches nothing — empty-state copy lands.
    await fireEvent.input(filter, { target: { value: 'zzz-no-match-zzz' } });
    expect(container.querySelectorAll('.midi__table tbody tr').length).toBe(0);
    expect(container.querySelector('.midi__no-rows')?.textContent).toContain(
      'no bindings match filter',
    );
  });

  it('Live Scene Controls render on the global tab and disappear on the meta tab', async () => {
    const { container, getByRole } = render(MIDI);
    // Default = global → Live Scene Controls present.
    expect(container.querySelector('.midi__live')).not.toBeNull();
    await fireEvent.click(getByRole('tab', { name: 'Meta controls' }));
    expect(container.querySelector('.midi__live')).toBeNull();
    // But the binding table is still present on meta.
    expect(container.querySelector('.midi__table')).not.toBeNull();
  });

  it('renders the BPM sync priority chain with one active entry and edge reorder buttons disabled', () => {
    const { container } = render(MIDI);
    const rows = container.querySelectorAll('.midi__bpm-row');
    expect(rows.length).toBe(BPM_SYNC_SOURCES.length);
    expect(rows.length).toBe(4);
    // Exactly one row carries the --active modifier (the fixture's
    // `state: 'active'` source — Ableton Link).
    const activeRows = container.querySelectorAll('.midi__bpm-row--active');
    expect(activeRows.length).toBe(1);
    expect(activeRows[0]?.textContent).toContain('Ableton Link');
    // First row's "up" button is disabled; last row's "down" button is disabled.
    const firstUp = rows[0]?.querySelector('.midi__btn--icon-up') as HTMLButtonElement | null;
    const lastDown = rows[rows.length - 1]?.querySelector(
      '.midi__btn--icon-down',
    ) as HTMLButtonElement | null;
    expect(firstUp?.disabled).toBe(true);
    expect(lastDown?.disabled).toBe(true);
  });

  it('renders the MTC readout panel with the canonical fixture timecode and uniform name', () => {
    const { container, getByText } = render(MIDI);
    expect(getByText(MIDI_MTC.timecode)).toBeInTheDocument();
    expect(getByText(MIDI_MTC.framerate)).toBeInTheDocument();
    expect(container.querySelector('.midi__mtc-uniform')?.textContent).toBe(MIDI_MTC.uniformName);
  });

  it('renders the Virtual Pad with the selected device dimensions (Launchpad X = 8×8 = 64 cells)', () => {
    const { container } = render(MIDI);
    const pad = container.querySelector('.cue-pad');
    expect(pad).not.toBeNull();
    expect(pad?.getAttribute('aria-label')).toBe('Cue pad (8 rows × 8 cols)');
    expect(pad?.querySelectorAll('.cue-pad__cell').length).toBe(64);
  });

  it('selecting the APC40 mkII pad device reseeds the grid to its 5×8 = 40 cells', async () => {
    const { container, getByLabelText } = render(MIDI);
    const select = getByLabelText('Mirrored pad controller') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'dev.apc' } });
    const pad = container.querySelector('.cue-pad');
    expect(pad?.getAttribute('aria-label')).toBe('Cue pad (5 rows × 8 cols)');
    expect(pad?.querySelectorAll('.cue-pad__cell').length).toBe(40);
  });

  it('clicking an idle pad cell cycles its data-state through armed → playing → idle', async () => {
    const { container } = render(MIDI);
    const cells = container.querySelectorAll('.cue-pad__cell');
    // Cell index 1 is seeded idle (i % 5 !== 0 and i % 7 !== 3 in the
    // view's seedPadCells helper) so we can observe a clean cycle.
    const idleCell = cells[1] as HTMLButtonElement;
    expect(idleCell.getAttribute('data-state')).toBe('idle');
    await fireEvent.click(idleCell);
    expect(
      (container.querySelectorAll('.cue-pad__cell')[1] as HTMLButtonElement).getAttribute(
        'data-state',
      ),
    ).toBe('armed');
    await fireEvent.click(container.querySelectorAll('.cue-pad__cell')[1] as HTMLButtonElement);
    expect(
      (container.querySelectorAll('.cue-pad__cell')[1] as HTMLButtonElement).getAttribute(
        'data-state',
      ),
    ).toBe('playing');
    await fireEvent.click(container.querySelectorAll('.cue-pad__cell')[1] as HTMLButtonElement);
    expect(
      (container.querySelectorAll('.cue-pad__cell')[1] as HTMLButtonElement).getAttribute(
        'data-state',
      ),
    ).toBe('idle');
  });

  it('annotates capability surfaces via data-cap on learn toggle, devices rows, and pad panel', async () => {
    const { container, getByRole } = render(MIDI);
    const learnBtn = getByRole('button', { name: /Click to map/ });
    expect(learnBtn.getAttribute('data-cap')).toBe('io.midi.learn');
    // Pad panel body carries the output capability annotation.
    const padBody = container.querySelector('.midi__panel-body[data-cap="io.midi.output"]');
    expect(padBody).not.toBeNull();
    // Devices rows annotate input capability.
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    const devRows = container.querySelectorAll('.midi__device-row[data-cap="io.midi.input"]');
    expect(devRows.length).toBe(MIDI_DEVICES.length);
  });
});
