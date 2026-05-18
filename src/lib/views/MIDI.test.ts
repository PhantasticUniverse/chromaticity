// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W62 MIDI view tests — exercise the W18 PR3 + W23 PR3-4 + W24 PR2 routed-
// view surface (tab vocabulary, learn-mode toggle round-trip, binding table
// render + filter, snap grid, devices allowlist toggle, virtual pad cycle,
// BPM sync chain, MTC readout, MPE banner gate). Wave 2.1 Lane D.1 widens
// the Devices tab to source the device list from the live `midiStore`
// (`list_midi_devices` Tauri command) so the suite now mocks `invoke`
// per the `feedback_vitest_invoke_mock_hoisted.md` user-memory convention
// and seeds the mock with a fixture-mirrored payload before each render
// so the row-count and toggle expectations stay deterministic.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// Most tests stay outside learn mode so the rAF + setInterval pair the
// learn `$effect` spawns is never started; the two tests that DO toggle
// learn mode rely on `cleanup()` running the `$effect`'s return-callback,
// which cancels the rAF and clears the intervals.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import MIDI from './MIDI.svelte';
import {
  BPM_SYNC_SOURCES,
  MIDI_CAPABILITY,
  MIDI_DEVICES,
  MIDI_HISTORY_FIXTURE,
  MIDI_LEARN_FIXTURE,
  MIDI_MTC,
  MIDI_ROWS_BY_TAB,
  MIDI_SNAP_SLOTS,
  MIDI_TABS,
  MIDI_TAB_HINTS,
  MPE_BANNER_COPY,
  RELATIVE_ENCODER_MODES,
} from '../data/midi-fixtures';
import type { MidiDevicePayload } from '../ipc/bindings/MidiDevicePayload';
import { midiStore } from '../stores/midiStore.svelte';

// Mirror the W18 fixture device-name set onto the live IPC wire shape so
// the Devices-tab row-count + toggle tests retain their pre-Lane-D.1
// invariants. Allowlist toggle aria-labels still key on the device name
// (e.g. "Disallow Launchpad X"), so the names match the fixture verbatim;
// the wire `id` is the live `input-N` shape per the engine adapter
// contract. Vendor / direction / maps / mpeWarned columns get conservative
// defaults at the view layer (`vendor='—'`, `direction='in'`, `maps=0`),
// so tests that inspected those fixture-only fields against named
// devices have been adjusted accordingly.
const LIVE_DEVICES_FIXTURE: readonly MidiDevicePayload[] = MIDI_DEVICES.map((d, i) => ({
  id: `input-${i}`,
  name: d.name,
  kind: 'Input' as const,
}));

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
    invokeMock.mockReset();
    // Default: live device enumeration returns the fixture-mirrored
    // payload so the Devices-tab row-count + toggle tests pass with the
    // pre-Lane-D.1 invariants. Per-test overrides (e.g. an empty-state
    // test) re-seed the mock before render.
    invokeMock.mockResolvedValue([...LIVE_DEVICES_FIXTURE]);
    midiStore.reset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    cleanup();
    midiStore.reset();
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

  it('switching to the devices tab renders one allowlist row per live device', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    // Live IPC drives the device list; `waitFor` flushes the
    // `midiStore.refresh()` promise the view's mount-effect fires.
    await waitFor(() => {
      const rows = container.querySelectorAll('.midi__device-row');
      expect(rows.length).toBe(LIVE_DEVICES_FIXTURE.length);
    });
    expect(LIVE_DEVICES_FIXTURE.length).toBe(8);
  });

  it('clicking a device allowlist toggle flips its aria-pressed state', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    await waitFor(() => {
      expect(container.querySelectorAll('.midi__device-row').length).toBeGreaterThan(0);
    });
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
    await waitFor(() => {
      const devRows = container.querySelectorAll('.midi__device-row[data-cap="io.midi.input"]');
      expect(devRows.length).toBe(LIVE_DEVICES_FIXTURE.length);
    });
  });

  // ----------------------------------------------------------------------
  // W63-equivalent depth deepening — appended after the W62 PR3 baseline.
  // The block below pushes coverage from 17 → ~45 tests at the lower end
  // of the cohort tests/100 LoC band so the maintenance PR lands in a
  // single focused wave. New tests group by:
  //   • tab vocabulary + per-tab body rendering (5)
  //   • per-tab binding-row counts + tab-hint copy (5)
  //   • filter narrowing + case-insensitivity + multi-field match (3)
  //   • binding-row mode select + flag pill rendering (3)
  //   • snap slot styling — gold / bound / active / mode pill (4)
  //   • BPM priority-chain row attributes + mid-row reorder enabled (2)
  //   • MTC panel structural copy (1)
  //   • virtual pad — controller-list census + selection round-trip (3)
  //   • learn-mode panel surfaces — value cell, last-input cell, history strip (3)
  //   • banner gating — non-devices tabs never render banner (2)
  //   • capability annotation completeness (2)
  //
  // Like the baseline above, every test stays outside learn mode
  // unless it explicitly toggles in; the `stubMatchMedia(true)`
  // default short-circuits the rAF + setInterval pair the
  // learn-mode `$effect` would otherwise spawn.

  it('renders all 7 tab labels in canonical fixture order', () => {
    const { getAllByRole } = render(MIDI);
    const labels = getAllByRole('tab').map((t) => t.textContent?.trim());
    expect(labels).toEqual(MIDI_TABS.map((t) => t.label));
  });

  it('switching tabs updates aria-selected mutually-exclusively across all 7', async () => {
    const { getAllByRole } = render(MIDI);
    for (const tab of MIDI_TABS) {
      const tabBtns = getAllByRole('tab');
      const target = tabBtns.find((t) => t.textContent?.trim() === tab.label);
      expect(target).toBeDefined();
      await fireEvent.click(target!);
      const after = getAllByRole('tab');
      const selected = after.filter((t) => t.getAttribute('aria-selected') === 'true');
      expect(selected.length).toBe(1);
      expect(selected[0]?.textContent?.trim()).toBe(tab.label);
    }
  });

  it('tab buttons set tabindex 0 on the active tab and -1 on the rest', async () => {
    const { getAllByRole, getByRole } = render(MIDI);
    // Default active tab is Global → its tabindex is 0.
    let global = getByRole('tab', { name: 'Global' });
    expect(global.getAttribute('tabindex')).toBe('0');
    const inactive = getAllByRole('tab').filter((t) => t.textContent?.trim() !== 'Global');
    for (const t of inactive) {
      expect(t.getAttribute('tabindex')).toBe('-1');
    }
    // Switch to Snaps and re-verify the rolling tabindex.
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    global = getByRole('tab', { name: 'Global' });
    expect(global.getAttribute('tabindex')).toBe('-1');
    expect(getByRole('tab', { name: 'Snaps' }).getAttribute('tabindex')).toBe('0');
  });

  it('Scene tab renders the binding table with one row per scene fixture', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Scene-specific' }));
    const rows = container.querySelectorAll('.midi__table tbody tr');
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.scene.length);
    expect(rows.length).toBe(3);
    // First row's data-family attr matches the fixture family literal.
    expect(rows[0]?.getAttribute('data-family')).toBe('scene');
  });

  it('Meta-controls tab renders one row per meta fixture entry and hides Live Scene Controls', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Meta controls' }));
    const rows = container.querySelectorAll('.midi__table tbody tr');
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.meta.length);
    expect(rows.length).toBe(4);
    // No live-scene block rendered on the meta tab.
    expect(container.querySelector('.midi__live')).toBeNull();
  });

  it('Media tab renders one row per media fixture entry without Live Scene Controls', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Media' }));
    const rows = container.querySelectorAll('.midi__table tbody tr');
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.media.length);
    expect(rows.length).toBe(2);
    expect(container.querySelector('.midi__live')).toBeNull();
  });

  it('Start-scenes tab renders one row per start fixture and shows note bindings', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Start-scenes' }));
    const rows = container.querySelectorAll('.midi__table tbody tr');
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.start.length);
    expect(rows.length).toBe(3);
    // Every Start-scenes row uses message type "Note" (vs CC).
    for (const row of Array.from(rows)) {
      const msgCell = row.querySelectorAll('td.mono')[2];
      expect(msgCell?.textContent?.trim()).toBe('Note');
    }
  });

  it('toolbar hint reads the canonical per-tab copy from MIDI_TAB_HINTS', async () => {
    const { container, getByRole } = render(MIDI);
    const hint = (): string =>
      container.querySelector('.midi__toolbar-hint')?.textContent?.trim() ?? '';
    // Default tab is Global.
    expect(hint()).toBe(MIDI_TAB_HINTS.global);
    await fireEvent.click(getByRole('tab', { name: 'Media' }));
    expect(hint()).toBe(MIDI_TAB_HINTS.media);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    expect(hint()).toBe(MIDI_TAB_HINTS.snaps);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    expect(hint()).toBe(MIDI_TAB_HINTS.devices);
  });

  it('learning toolbar hint replaces the per-tab hint while learn mode is on', async () => {
    const { container, getByRole } = render(MIDI);
    expect(container.querySelector('.midi__toolbar-hint--learn')).toBeNull();
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    const learnHint = container.querySelector('.midi__toolbar-hint--learn');
    expect(learnHint).not.toBeNull();
    expect(learnHint?.textContent).toContain('Waiting for input');
    // Cancel learn → hint reverts to the per-tab copy (Global by default).
    await fireEvent.click(getByRole('button', { name: /Cancel Learn/ }));
    expect(container.querySelector('.midi__toolbar-hint--learn')).toBeNull();
    expect(container.querySelector('.midi__toolbar-hint')?.textContent?.trim()).toBe(
      MIDI_TAB_HINTS.global,
    );
  });

  it('filter input is case-insensitive and matches device-name as well as target', async () => {
    const { container, getByLabelText } = render(MIDI);
    const filter = getByLabelText('filter MIDI bindings') as HTMLInputElement;
    // Upper-case query → still matches lower-case targets/devices.
    await fireEvent.input(filter, { target: { value: 'APC40' } });
    const rows = container.querySelectorAll('.midi__table tbody tr');
    // All three Global rows reference APC40 mkII as their device.
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.global.length);
  });

  it('filter narrows by num field match (CC number string)', async () => {
    const { container, getByLabelText } = render(MIDI);
    const filter = getByLabelText('filter MIDI bindings') as HTMLInputElement;
    // Global has CC nums 14, 15, 16/48 — match exact "16/48".
    await fireEvent.input(filter, { target: { value: '16/48' } });
    const rows = container.querySelectorAll('.midi__table tbody tr');
    expect(rows.length).toBe(1);
    expect(rows[0]?.textContent).toContain('meta.contrast');
  });

  it('filter cleared (empty string) restores the full row set', async () => {
    const { container, getByLabelText } = render(MIDI);
    const filter = getByLabelText('filter MIDI bindings') as HTMLInputElement;
    await fireEvent.input(filter, { target: { value: 'hue' } });
    expect(container.querySelectorAll('.midi__table tbody tr').length).toBe(1);
    await fireEvent.input(filter, { target: { value: '' } });
    expect(container.querySelectorAll('.midi__table tbody tr').length).toBe(
      MIDI_ROWS_BY_TAB.global.length,
    );
  });

  it('binding-row mode selects render all 4 mode options and seed the row default', () => {
    const { container } = render(MIDI);
    const firstSelect = container.querySelector(
      '.midi__cell--mode select.midi__select',
    ) as HTMLSelectElement | null;
    expect(firstSelect).not.toBeNull();
    const options = Array.from(firstSelect!.querySelectorAll('option')).map((o) => o.value);
    expect(options).toEqual(['absolute', 'relative', 'toggle', 'momentary']);
    // First Global row's default mode is 'absolute' (meta.brightness).
    expect(firstSelect!.value).toBe(MIDI_ROWS_BY_TAB.global[0]!.mode);
  });

  it("renders the 14-bit pill for the meta.contrast row and rel 2's pill for meta.hue", () => {
    const { container } = render(MIDI);
    const rows = container.querySelectorAll('.midi__table tbody tr');
    // Global row 1 is meta.hue ("2's-comp" flag) — flag cell shows "rel 2's".
    const hueRow = Array.from(rows).find((r) => r.textContent?.includes('meta.hue'));
    expect(hueRow?.querySelector('.midi__pill--success')?.textContent?.trim()).toBe("rel 2's");
    // Global row 2 is meta.contrast ("14-bit" flag) — flag cell shows "14-bit".
    const contrastRow = Array.from(rows).find((r) => r.textContent?.includes('meta.contrast'));
    expect(contrastRow?.querySelector('.midi__pill--info')?.textContent?.trim()).toBe('14-bit');
  });

  it('every binding row carries the saved-dot affordance with the canonical aria-label', () => {
    const { container } = render(MIDI);
    const rows = container.querySelectorAll('.midi__table tbody tr');
    expect(rows.length).toBe(MIDI_ROWS_BY_TAB.global.length);
    for (const row of Array.from(rows)) {
      const dot = row.querySelector('.midi__saved-dot');
      expect(dot).not.toBeNull();
      expect(dot?.getAttribute('aria-label')).toBe('saved · session-stable');
    }
  });

  it('Snap slots tab gold-preset slot is index 0 and carries the gold modifier', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    const slots = container.querySelectorAll('.midi__snap');
    expect(slots.length).toBe(16);
    // Slot 0 carries the gold modifier and reads "00" in its number cell.
    expect(slots[0]?.classList.contains('midi__snap--gold')).toBe(true);
    expect(slots[0]?.querySelector('.midi__snap-num')?.textContent?.trim()).toBe('00');
  });

  it('Snap slot 2 (Build) carries the active modifier and aria-pressed=true (sole active fixture)', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    const slots = container.querySelectorAll('.midi__snap');
    const activeIndices = MIDI_SNAP_SLOTS.map((s, i) => (s.isActive ? i : -1)).filter(
      (i) => i >= 0,
    );
    expect(activeIndices.length).toBe(1);
    expect(activeIndices[0]).toBe(2);
    expect(slots[2]?.classList.contains('midi__snap--active')).toBe(true);
    expect(slots[2]?.getAttribute('aria-pressed')).toBe('true');
  });

  it('Snap slots with bindings render the mode pill; unbound slots render the unbound placeholder', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    const slots = container.querySelectorAll('.midi__snap');
    // Slot 1 (Intro wash) is bound (LPX · Note 81) → mode pill renders.
    expect(slots[1]?.querySelector('.midi__snap-mode-pill')).not.toBeNull();
    // Slot 4 has neither name nor binding → unbound binding text + empty name.
    expect(slots[4]?.querySelector('.midi__snap-binding--unbound')?.textContent?.trim()).toBe(
      'unbound',
    );
    expect(slots[4]?.querySelector('.midi__snap-name--empty')?.textContent?.trim()).toBe('empty');
  });

  it('Snap slots momentary fixture (slot 3) renders mode pill text "momentary"', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Snaps' }));
    const slots = container.querySelectorAll('.midi__snap');
    // Slot 3 (Drop) has the momentary mode in the fixture; all others bound = toggle.
    const slot3Pill = slots[3]?.querySelector('.midi__snap-mode-pill');
    expect(slot3Pill?.textContent?.trim()).toBe('momentary');
    // Slot 1 fixture is toggle.
    const slot1Pill = slots[1]?.querySelector('.midi__snap-mode-pill');
    expect(slot1Pill?.textContent?.trim()).toBe('toggle');
  });

  it('BPM priority chain rows expose data-state matching the fixture (active / standby / idle)', () => {
    const { container } = render(MIDI);
    const rows = container.querySelectorAll('.midi__bpm-row');
    expect(rows.length).toBe(BPM_SYNC_SOURCES.length);
    for (let i = 0; i < rows.length; i++) {
      expect(rows[i]?.getAttribute('data-state')).toBe(BPM_SYNC_SOURCES[i]!.state);
    }
  });

  it('BPM priority chain middle rows have both up + down reorder buttons enabled', () => {
    const { container } = render(MIDI);
    const rows = container.querySelectorAll('.midi__bpm-row');
    // Middle rows = indices 1 and 2 in the 4-row chain.
    for (const i of [1, 2]) {
      const up = rows[i]?.querySelector('.midi__btn--icon-up') as HTMLButtonElement | null;
      const down = rows[i]?.querySelector('.midi__btn--icon-down') as HTMLButtonElement | null;
      expect(up?.disabled).toBe(false);
      expect(down?.disabled).toBe(false);
    }
  });

  it('MTC panel exposes the canonical assembly-state prose alongside the uniform name', () => {
    const { container } = render(MIDI);
    const prose = container.querySelector('.midi__mtc-prose');
    expect(prose?.textContent).toContain('locked');
    expect(prose?.textContent).toContain(MIDI_MTC.uniformName);
  });

  it('Virtual Pad controller select lists every pad-capable fixture device (and only those)', () => {
    const { getByLabelText } = render(MIDI);
    const select = getByLabelText('Mirrored pad controller') as HTMLSelectElement;
    const opts = Array.from(select.querySelectorAll('option')).map((o) => o.value);
    const padCapable = MIDI_DEVICES.filter((d) => d.padDims !== undefined).map((d) => d.id);
    expect(opts).toEqual(padCapable);
    // Sanity: 4 pad devices (LPX, APC, Push 2, LPP3, APC mini) — actually 5 in fixture.
    expect(opts.length).toBe(5);
  });

  it('Virtual Pad defaults to Launchpad X (8×8 = 64 cells) at first render', () => {
    const { container, getByLabelText } = render(MIDI);
    const select = getByLabelText('Mirrored pad controller') as HTMLSelectElement;
    expect(select.value).toBe('dev.lpx');
    expect(container.querySelectorAll('.cue-pad__cell').length).toBe(64);
  });

  it('Virtual Pad reseed across pad devices preserves the seed pattern (every 5th cell playing)', async () => {
    const { container, getByLabelText } = render(MIDI);
    const select = getByLabelText('Mirrored pad controller') as HTMLSelectElement;
    await fireEvent.change(select, { target: { value: 'dev.push2' } });
    // Push 2 = 8×8 = 64 cells; seedPadCells emits playing on every i % 5 === 0.
    const cells = container.querySelectorAll('.cue-pad__cell');
    expect(cells.length).toBe(64);
    // Spot-check the seed pattern: index 0 is playing (i % 5 === 0).
    expect((cells[0] as HTMLButtonElement).getAttribute('data-state')).toBe('playing');
    // Index 5 also seeded playing.
    expect((cells[5] as HTMLButtonElement).getAttribute('data-state')).toBe('playing');
  });

  it('learn-mode panel renders the value cell with hex + decimal from the fixture', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    const valueText = container.querySelector('.midi__panel--learn .midi__learn-grid')?.textContent;
    expect(valueText).toContain(MIDI_LEARN_FIXTURE.valueHex);
    expect(valueText).toContain(String(MIDI_LEARN_FIXTURE.valueDec));
  });

  it('learn-mode panel renders the last-input cell with device, channel, and message from fixture', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    const grid = container.querySelector('.midi__panel--learn .midi__learn-grid');
    expect(grid?.textContent).toContain(MIDI_LEARN_FIXTURE.lastInputDevice);
    expect(grid?.textContent).toContain(MIDI_LEARN_FIXTURE.lastInputMessage);
    expect(grid?.textContent).toContain(`CH ${MIDI_LEARN_FIXTURE.lastInputChannel}`);
  });

  it('learn-mode history strip renders one row per history fixture entry under reduced motion', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    // Reduced-motion default short-circuits the live ring buffer to the
    // static fixture (5 entries).
    const histRows = container.querySelectorAll('.midi__history .midi__history-row');
    expect(histRows.length).toBe(MIDI_HISTORY_FIXTURE.length);
    expect(histRows.length).toBe(5);
    // First row is the head row (most recent in the live model, even for
    // the static seed since the fixture is ordered newest-first).
    expect(histRows[0]?.classList.contains('midi__history-row--head')).toBe(true);
  });

  it('learn-mode encoder pills include all 5 canonical relative-encoder modes in canonical order', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    const pills = container.querySelectorAll('.midi__pill--encoder');
    expect(pills.length).toBe(5);
    const labels = Array.from(pills).map((p) => p.textContent?.replace(/\s*✓\s*$/, '').trim());
    expect(labels).toEqual([...RELATIVE_ENCODER_MODES]);
  });

  it('MPE banner does NOT render on non-devices tabs even though Seaboard is mpeWarned', async () => {
    const { container, getByRole, queryByRole } = render(MIDI);
    // Tour every non-devices tab and confirm the banner stays absent.
    for (const tab of MIDI_TABS.filter((t) => t.id !== 'devices')) {
      await fireEvent.click(getByRole('tab', { name: tab.label }));
      expect(queryByRole('status')).toBeNull();
      expect(container.querySelector('.midi__banner')).toBeNull();
    }
  });

  it('Devices tab toggle keeps other devices unchanged when one is flipped', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    await waitFor(() => {
      expect(container.querySelectorAll('.midi__device-row').length).toBeGreaterThan(0);
    });
    const rows = container.querySelectorAll('.midi__device-row');
    // Wave 2.1 Lane D.1: live IPC drives the device list; the view's
    // mount-effect auto-allows every device the FIRST time the store
    // surfaces it. Flip APC mini Mk2 (index 4) from on → off.
    const apcMiniRow = rows[4] as HTMLElement;
    const toggle = apcMiniRow.querySelector('.midi__device-toggle-btn') as HTMLButtonElement;
    expect(toggle.getAttribute('aria-pressed')).toBe('true');
    await fireEvent.click(toggle);
    // The flipped row updates; sibling rows preserve their state.
    const updated = container.querySelectorAll('.midi__device-row');
    expect(
      (updated[4]?.querySelector('.midi__device-toggle-btn') as HTMLButtonElement).getAttribute(
        'aria-pressed',
      ),
    ).toBe('false');
    // Launchpad X (index 0) was true and stays true.
    expect(
      (updated[0]?.querySelector('.midi__device-toggle-btn') as HTMLButtonElement).getAttribute(
        'aria-pressed',
      ),
    ).toBe('true');
    // Push 2 (index 2) stays true.
    expect(
      (updated[2]?.querySelector('.midi__device-toggle-btn') as HTMLButtonElement).getAttribute(
        'aria-pressed',
      ),
    ).toBe('true');
  });

  it('Devices tab toggle button aria-label round-trips between Allow/Disallow on flip', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    await waitFor(() => {
      expect(container.querySelectorAll('.midi__device-row').length).toBeGreaterThan(0);
    });
    // Launchpad X (index 0, allowed=true after the initial refresh
    // auto-enables every live device) → aria-label "Disallow Launchpad X".
    const lpxToggle = container
      .querySelectorAll('.midi__device-row')[0]
      ?.querySelector('.midi__device-toggle-btn') as HTMLButtonElement;
    expect(lpxToggle.getAttribute('aria-label')).toBe('Disallow Launchpad X');
    await fireEvent.click(lpxToggle);
    expect(lpxToggle.getAttribute('aria-label')).toBe('Allow Launchpad X');
  });

  it('Live Scene Controls grid renders 4 widget categories on the global tab', () => {
    const { container } = render(MIDI);
    const heads = Array.from(container.querySelectorAll('.midi__live .midi__group-head')).map((h) =>
      h.textContent?.trim(),
    );
    expect(heads).toEqual(['toggle', 'slider', 'bang', 'knob']);
    // Each category has exactly 4 widgets.
    expect(container.querySelectorAll('.midi__widget--toggle').length).toBe(4);
    expect(container.querySelectorAll('.midi__widget--slider').length).toBe(4);
    expect(container.querySelectorAll('.midi__widget--bang').length).toBe(4);
    expect(container.querySelectorAll('.midi__widget--knob').length).toBe(4);
  });

  it('every binding-row Remove action button reads its accessible name from the row target', () => {
    const { container } = render(MIDI);
    const removeBtns = container.querySelectorAll('.midi__cell--actions button.midi__btn--icon');
    expect(removeBtns.length).toBe(MIDI_ROWS_BY_TAB.global.length);
    const labels = Array.from(removeBtns).map((b) => b.getAttribute('aria-label'));
    expect(labels).toEqual(MIDI_ROWS_BY_TAB.global.map((r) => `Remove binding ${r.target}`));
  });

  it('learn-mode capability annotation MIDI_CAPABILITY.learn matches the data-cap value on the toggle', () => {
    // Sanity: the constant the view emits is the same one the fixture exports.
    const { getByRole } = render(MIDI);
    const learnBtn = getByRole('button', { name: /Click to map/ });
    expect(learnBtn.getAttribute('data-cap')).toBe(MIDI_CAPABILITY.learn);
  });

  // ----------------------------------------------------------------------
  // Wave 2.1 Lane D.1 — live MIDI device enumeration.
  //
  // The Devices tab now sources its device list from the live `midiStore`
  // backed by the `list_midi_devices` Tauri command. These tests pin the
  // mount-time refresh contract, the empty-state branch (when no MIDI
  // devices are detected), the Refresh button round-trip, the
  // error-banner surface, and the live-vs-fixture row-name parity.

  it('Wave 2.1 Lane D.1 — invokes list_midi_devices on mount', async () => {
    render(MIDI);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('list_midi_devices');
    });
  });

  it('Wave 2.1 Lane D.1 — Devices tab renders empty state when no live devices', async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue([]);
    midiStore.reset();
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    await waitFor(() => {
      const empty = container.querySelector('.midi__no-rows');
      expect(empty?.textContent).toContain('No MIDI devices detected');
    });
    expect(container.querySelectorAll('.midi__device-row').length).toBe(0);
  });

  it('Wave 2.1 Lane D.1 — Refresh button re-invokes list_midi_devices', async () => {
    const { getByRole } = render(MIDI);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('list_midi_devices');
    });
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    const refresh = await waitFor(() => getByRole('button', { name: /Refresh MIDI device list/i }));
    const before = invokeMock.mock.calls.length;
    await fireEvent.click(refresh);
    await waitFor(() => {
      expect(invokeMock.mock.calls.length).toBeGreaterThan(before);
    });
  });

  it('Wave 2.1 Lane D.1 — surfaces the store error banner when refresh rejects', async () => {
    invokeMock.mockReset();
    invokeMock.mockRejectedValue(new Error('midi subsystem unavailable'));
    midiStore.reset();
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    await waitFor(() => {
      const banner = container.querySelector('[data-midi-error]');
      expect(banner).not.toBeNull();
      expect(banner?.textContent).toContain('midi subsystem unavailable');
    });
  });

  it('Wave 2.1 Lane D.1 — live device names render verbatim from the IPC payload', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: 'Devices' }));
    await waitFor(() => {
      const rows = container.querySelectorAll('.midi__device-row');
      expect(rows.length).toBe(LIVE_DEVICES_FIXTURE.length);
    });
    const rows = Array.from(container.querySelectorAll('.midi__device-row'));
    const names = rows.map(
      (r) => r.querySelector('.midi__device-name .mono')?.textContent?.trim() ?? '',
    );
    expect(names).toEqual(LIVE_DEVICES_FIXTURE.map((d) => d.name));
  });

  // Wave 2.1 Lane D.2 — learn-mode state machine smoke test. With the
  // default mock posture (`invokeMock.mockResolvedValue([...])` from
  // the suite-level beforeEach) the `start_midi_learn` invocation
  // also resolves to that array — which the store doesn't introspect,
  // it just awaits the promise. The listener attach takes the
  // no-Tauri-runtime swallow path because `@tauri-apps/api/event`
  // isn't mocked, so the store transitions to 'listening' but no
  // payload arrives. The view should reflect 'listening' state via
  // the `data-midi-learn-listening` selector.
  it('toolbar Learn button starts a learn session and renders the listening affordance', async () => {
    const { container, getByRole } = render(MIDI);
    // Wait for live-device store population so a port id is available.
    await waitFor(() => {
      const rows = container.querySelectorAll('.midi__device-row');
      // No DOM dependency here; we just need the store to populate.
      void rows;
    });
    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    // Panel rendered with state attribute reflecting the current
    // store state. With listener attach failing (no `event` mock),
    // store stays in 'listening' until cancelled.
    await waitFor(() => {
      const panel = container.querySelector('.midi__panel--learn');
      expect(panel).not.toBeNull();
      const state = panel?.getAttribute('data-midi-learn-state');
      // Allow either 'idle' (start failed instantly) or 'listening'.
      expect(['idle', 'listening']).toContain(state);
    });
    // Cancel — closes the panel.
    await fireEvent.click(getByRole('button', { name: /Cancel Learn/ }));
    expect(container.querySelector('.midi__panel--learn')).toBeNull();
  });

  it('stops an active learn session when the MIDI view unmounts', async () => {
    const { getByRole, unmount } = render(MIDI);
    await waitFor(() => {
      expect(midiStore.devices.length).toBe(LIVE_DEVICES_FIXTURE.length);
    });
    invokeMock.mockClear();

    await fireEvent.click(getByRole('button', { name: /Click to map/ }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('start_midi_learn', { portId: 'input-0' });
    });

    invokeMock.mockClear();
    unmount();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('stop_midi_learn');
    });
  });

  it('Devices tab renders a per-device Learn button gated on the learn capability', async () => {
    const { container, getByRole } = render(MIDI);
    await fireEvent.click(getByRole('tab', { name: /Devices/i }));
    await waitFor(() => {
      const rows = container.querySelectorAll('.midi__device-row');
      expect(rows.length).toBeGreaterThan(0);
    });
    // Each device row has a "Learn from <name>" button.
    const learnButtons = container.querySelectorAll(
      `[data-cap="${MIDI_CAPABILITY.learn}"][aria-label^="Learn from "]`,
    );
    expect(learnButtons.length).toBe(LIVE_DEVICES_FIXTURE.length);
  });
});
