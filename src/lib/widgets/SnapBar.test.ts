// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// SnapBar widget tests — W43 baseline covered the 8-slot Perform-bar
// rendering surface; Wave 2.1 Lane A.3 widens to cover the engine snap
// IPC wiring (plain click → `recall_snap`, Cmd / Ctrl click →
// `save_snap`, Alt click → `clear_snap`) + keyboard parity + the
// per-slot empty / saved / active visual indicators driven from the
// `snapStore` rune singleton.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// `vi.hoisted` invoke-mock pattern per
// `feedback_vitest_invoke_mock_hoisted.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import SnapBar from './SnapBar.svelte';
import { snapStore } from '../stores/snap.svelte';
import { snapLabelsStore } from '../stores/snapLabels.svelte';

describe('SnapBar widget — rendering surface (W43 baseline)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    snapStore.reset();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders the snap-bar root with the vertical Snap label', () => {
    const { container, getByText } = render(SnapBar);
    expect(container.querySelector('.snap-bar')).not.toBeNull();
    expect(getByText('Snap')).toBeInTheDocument();
  });

  it('renders 8 snap slots from the default fixture', () => {
    const { container } = render(SnapBar);
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    expect(slots.length).toBe(8);
  });

  it('renders the disabled save slot (+) at the end', () => {
    const { container } = render(SnapBar);
    const addSlot = container.querySelector('.snap-slot--add');
    expect(addSlot).not.toBeNull();
    expect((addSlot as HTMLButtonElement).disabled).toBe(true);
    expect(addSlot?.getAttribute('aria-label')).toBe('Save current state as a Snap');
  });

  it('applies snap-slot--on to the slot matching the active prop override', () => {
    const { container } = render(SnapBar, { props: { active: 3 } });
    const onSlots = container.querySelectorAll('.snap-slot--on');
    expect(onSlots.length).toBe(1);
    // Slot 3 in the default fixture is "Break".
    expect(onSlots[0]?.getAttribute('title')).toContain('Break');
  });

  it('renders no active slot when neither the prop nor the store identifies one', () => {
    const { container } = render(SnapBar);
    const onSlots = container.querySelectorAll('.snap-slot--on');
    expect(onSlots.length).toBe(0);
  });

  it('honors a caller-provided slots fixture', () => {
    const { container } = render(SnapBar, {
      props: {
        slots: [
          {
            i: 1,
            name: 'Solo',
            scene: 'OneScene',
            tint: 'var(--color-primary)',
            kind: 'dunes',
            seed: 1,
          },
        ],
      },
    });
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    expect(slots.length).toBe(1);
    expect(slots[0]?.getAttribute('title')).toContain('Solo');
  });
});

describe('SnapBar widget — back-compat onPick prop (W43 baseline)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    snapStore.reset();
  });

  afterEach(() => {
    cleanup();
  });

  it('fires onPick with the clicked slot id when the caller supplies onPick', async () => {
    const onPick = vi.fn();
    const { container } = render(SnapBar, { props: { onPick } });
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    expect(slots.length).toBe(8);
    await fireEvent.click(slots[4] as Element); // 5th slot, i=5 ("Build")
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick).toHaveBeenCalledWith(5);
  });

  it('does not invoke engine snap IPC when onPick is supplied', async () => {
    const onPick = vi.fn();
    const { container } = render(SnapBar, { props: { onPick } });
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.click(slots[0] as Element);
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe('SnapBar widget — engine snap IPC wiring (Wave 2.1 A.3)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    snapStore.reset();
  });

  afterEach(() => {
    cleanup();
  });

  /** Drain the microtask queue enough times to clear the
   *  `await import('@tauri-apps/api/core')` + `await invoke(...)` chain
   *  the store kicks off in its `recall`/`save`/`clear` methods. The
   *  onMount `refresh()` does 1 import + 9 invokes; the per-click path
   *  does 1 import + 1 invoke; 20 microtask cycles is generous slack. */
  async function flushIpc(): Promise<void> {
    for (let i = 0; i < 20; i += 1) {
      await Promise.resolve();
    }
  }

  it('plain click invokes recall_snap with the engine slot id (slot.i - 1)', async () => {
    const { container } = render(SnapBar);
    // Drain the onMount refresh() so its calls do not pollute the
    // interaction count, then clear and drive the click.
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.click(slots[2] as Element); // 3rd tile, i=3 → engine slot 2
    await flushIpc();
    const recallCalls = invokeMock.mock.calls.filter((c) => c[0] === 'recall_snap');
    expect(recallCalls.length).toBe(1);
    expect(recallCalls[0]?.[1]).toEqual({ slot: 2 });
  });

  it('Cmd-click invokes save_snap with the engine slot id', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.click(slots[0] as Element, { metaKey: true });
    await flushIpc();
    const saveCalls = invokeMock.mock.calls.filter((c) => c[0] === 'save_snap');
    expect(saveCalls.length).toBe(1);
    expect(saveCalls[0]?.[1]).toEqual({ slot: 0 });
  });

  it('Ctrl-click invokes save_snap (Windows / Linux parity)', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.click(slots[3] as Element, { ctrlKey: true });
    await flushIpc();
    const saveCalls = invokeMock.mock.calls.filter((c) => c[0] === 'save_snap');
    expect(saveCalls.length).toBe(1);
    expect(saveCalls[0]?.[1]).toEqual({ slot: 3 });
  });

  it('Alt-click invokes clear_snap with the engine slot id', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.click(slots[5] as Element, { altKey: true });
    await flushIpc();
    const clearCalls = invokeMock.mock.calls.filter((c) => c[0] === 'clear_snap');
    expect(clearCalls.length).toBe(1);
    expect(clearCalls[0]?.[1]).toEqual({ slot: 5 });
  });

  it('Alt+Cmd click prefers Alt → clear_snap (Alt wins over the save modifier)', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.click(slots[1] as Element, { altKey: true, metaKey: true });
    await flushIpc();
    const clearCalls = invokeMock.mock.calls.filter((c) => c[0] === 'clear_snap');
    const saveCalls = invokeMock.mock.calls.filter((c) => c[0] === 'save_snap');
    expect(clearCalls.length).toBe(1);
    expect(saveCalls.length).toBe(0);
  });

  it('Enter key on a slot invokes recall_snap', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.keyDown(slots[0] as Element, { key: 'Enter' });
    await flushIpc();
    const recallCalls = invokeMock.mock.calls.filter((c) => c[0] === 'recall_snap');
    expect(recallCalls.length).toBe(1);
    expect(recallCalls[0]?.[1]).toEqual({ slot: 0 });
  });

  it('Cmd+Enter key on a slot invokes save_snap', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.keyDown(slots[2] as Element, { key: 'Enter', metaKey: true });
    await flushIpc();
    const saveCalls = invokeMock.mock.calls.filter((c) => c[0] === 'save_snap');
    expect(saveCalls.length).toBe(1);
    expect(saveCalls[0]?.[1]).toEqual({ slot: 2 });
  });

  it('Alt+Enter key on a slot invokes clear_snap', async () => {
    const { container } = render(SnapBar);
    await flushIpc();
    invokeMock.mockClear();
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    await fireEvent.keyDown(slots[4] as Element, { key: 'Enter', altKey: true });
    await flushIpc();
    const clearCalls = invokeMock.mock.calls.filter((c) => c[0] === 'clear_snap');
    expect(clearCalls.length).toBe(1);
    expect(clearCalls[0]?.[1]).toEqual({ slot: 4 });
  });
});

describe('SnapBar widget — visual state indicators', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    snapStore.reset();
  });

  afterEach(() => {
    cleanup();
  });

  it('marks slots without a saved bit as `snap-slot--empty`', () => {
    const { container } = render(SnapBar);
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    // Default store state: nothing saved, nothing active → all 8 empty.
    const emptySlots = container.querySelectorAll('.snap-slot--empty');
    expect(emptySlots.length).toBe(slots.length);
  });

  it('marks store-saved slots as `snap-slot--saved` (not `--empty`)', async () => {
    snapStore.saved = [false, false, true, false, false, false, false, false];
    const { container } = render(SnapBar);
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    const slot3 = slots[2] as HTMLElement;
    expect(slot3.classList.contains('snap-slot--saved')).toBe(true);
    expect(slot3.classList.contains('snap-slot--empty')).toBe(false);
  });

  it('marks the store active slot as `snap-slot--on`', () => {
    snapStore.activeSlot = 6;
    const { container } = render(SnapBar);
    const onSlots = container.querySelectorAll('.snap-slot--on');
    expect(onSlots.length).toBe(1);
    expect(onSlots[0]?.getAttribute('title')).toContain('Ambient'); // slot 6 fixture
  });

  it('sets aria-pressed=true on the active slot and false elsewhere', () => {
    snapStore.activeSlot = 2;
    const { container } = render(SnapBar);
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    expect((slots[1] as HTMLElement).getAttribute('aria-pressed')).toBe('true');
    expect((slots[0] as HTMLElement).getAttribute('aria-pressed')).toBe('false');
  });
});

describe('snapLabelsStore — localStorage round-trip (Wave 2.4 PERF.F)', () => {
  beforeEach(() => {
    snapLabelsStore.reset();
    localStorage.removeItem('snap-labels');
  });

  afterEach(() => {
    snapLabelsStore.reset();
    localStorage.removeItem('snap-labels');
  });

  it('persists a label across hydrate (set → reset → hydrate → label)', () => {
    snapLabelsStore.set(1, 'Intro');
    expect(snapLabelsStore.label(1)).toBe('Intro');

    // Drop the in-memory state and re-read from localStorage; the
    // label should come back.
    snapLabelsStore.reset();
    expect(snapLabelsStore.label(1)).toBe('1');

    snapLabelsStore.hydrate();
    expect(snapLabelsStore.label(1)).toBe('Intro');
  });

  it('falls back to the supplied fallback when no label is set', () => {
    expect(snapLabelsStore.label(3, 'Break')).toBe('Break');
  });

  it('falls back to the slot id as a string when no label and no fallback', () => {
    expect(snapLabelsStore.label(7)).toBe('7');
  });

  it('trims whitespace before persisting', () => {
    snapLabelsStore.set(2, '  Drop  ');
    expect(snapLabelsStore.label(2)).toBe('Drop');
    snapLabelsStore.reset();
    snapLabelsStore.hydrate();
    expect(snapLabelsStore.label(2)).toBe('Drop');
  });

  it('removes the entry when set with an empty / whitespace-only label', () => {
    snapLabelsStore.set(4, 'Bass');
    expect(snapLabelsStore.label(4)).toBe('Bass');
    snapLabelsStore.set(4, '   ');
    expect(snapLabelsStore.label(4, 'fallback')).toBe('fallback');
  });

  it('survives corrupt JSON in localStorage (hydrate is no-op on parse failure)', () => {
    localStorage.setItem('snap-labels', '{not-json');
    snapLabelsStore.hydrate();
    expect(snapLabelsStore.label(1, 'def')).toBe('def');
  });
});

describe('SnapBar widget — slot label rename (Wave 2.4 PERF.F)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    snapStore.reset();
    snapLabelsStore.reset();
    localStorage.removeItem('snap-labels');
  });

  afterEach(() => {
    cleanup();
    snapLabelsStore.reset();
    localStorage.removeItem('snap-labels');
  });

  it('renders the fixture name when no override is set', () => {
    const { container } = render(SnapBar);
    const slot1 = container.querySelectorAll('.snap-slot:not(.snap-slot--add)')[0] as HTMLElement;
    expect(slot1.querySelector('.snap-slot__name')?.textContent).toBe('Intro');
  });

  it('renders the operator override label when set in the store', () => {
    snapLabelsStore.set(1, 'Verse');
    const { container } = render(SnapBar);
    const slot1 = container.querySelectorAll('.snap-slot:not(.snap-slot--add)')[0] as HTMLElement;
    expect(slot1.querySelector('.snap-slot__name')?.textContent).toBe('Verse');
  });

  it('right-click opens an inline rename input', async () => {
    const { container } = render(SnapBar);
    const slot1 = container.querySelectorAll('.snap-slot:not(.snap-slot--add)')[0] as HTMLElement;
    expect(container.querySelector('.snap-slot__rename')).toBeNull();
    await fireEvent.contextMenu(slot1);
    expect(container.querySelector('.snap-slot__rename')).not.toBeNull();
  });

  it('right-click → type → Enter persists the label and closes the input', async () => {
    const { container } = render(SnapBar);
    const slot1 = container.querySelectorAll('.snap-slot:not(.snap-slot--add)')[0] as HTMLElement;
    await fireEvent.contextMenu(slot1);
    const input = container.querySelector('.snap-slot__rename') as HTMLInputElement;
    expect(input).not.toBeNull();
    await fireEvent.input(input, { target: { value: 'Intro' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(container.querySelector('.snap-slot__rename')).toBeNull();
    expect(snapLabelsStore.label(1)).toBe('Intro');
    const refreshedSlot1 = container.querySelectorAll(
      '.snap-slot:not(.snap-slot--add)',
    )[0] as HTMLElement;
    expect(refreshedSlot1.querySelector('.snap-slot__name')?.textContent).toBe('Intro');
  });

  it('Escape cancels the rename without persisting', async () => {
    snapLabelsStore.set(1, 'Old');
    const { container } = render(SnapBar);
    const slot1 = container.querySelectorAll('.snap-slot:not(.snap-slot--add)')[0] as HTMLElement;
    await fireEvent.contextMenu(slot1);
    const input = container.querySelector('.snap-slot__rename') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'New' } });
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(container.querySelector('.snap-slot__rename')).toBeNull();
    expect(snapLabelsStore.label(1)).toBe('Old');
  });

  it('suppresses recall_snap dispatch while a rename is open', async () => {
    const { container } = render(SnapBar);
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    // Wait out the onMount refresh() so its calls do not pollute the
    // interaction count.
    for (let i = 0; i < 20; i += 1) await Promise.resolve();
    invokeMock.mockClear();

    await fireEvent.contextMenu(slots[0] as Element);
    // Click another slot while the rename is open — recall_snap MUST
    // NOT fire on either the renaming slot or its neighbour.
    await fireEvent.click(slots[2] as Element);
    for (let i = 0; i < 20; i += 1) await Promise.resolve();
    const recallCalls = invokeMock.mock.calls.filter((c) => c[0] === 'recall_snap');
    expect(recallCalls.length).toBe(0);
  });
});
