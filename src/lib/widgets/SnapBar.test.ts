// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave-43 SnapBar widget tests — closes the W27 PR1 8-slot Perform-bar
// coverage gap. Verifies the default fixture renders 8 slots, the
// `active` prop highlights the right slot, the `+` save-slot is a
// disabled visual stub, and onPick fires with the clicked slot id.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import SnapBar from './SnapBar.svelte';

describe('SnapBar widget', () => {
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

  it('applies snap-slot--on to the slot matching the active index', () => {
    const { container } = render(SnapBar, { props: { active: 3 } });
    const onSlots = container.querySelectorAll('.snap-slot--on');
    expect(onSlots.length).toBe(1);
    // Slot 3 in the default fixture is "Break".
    expect(onSlots[0]?.getAttribute('title')).toContain('Break');
  });

  it('defaults active to slot 1', () => {
    const { container } = render(SnapBar);
    const onSlots = container.querySelectorAll('.snap-slot--on');
    expect(onSlots.length).toBe(1);
    expect(onSlots[0]?.getAttribute('title')).toContain('Intro');
  });

  it('fires onPick with the clicked slot id', async () => {
    const onPick = vi.fn();
    const { container } = render(SnapBar, { props: { onPick } });
    const slots = container.querySelectorAll('.snap-slot:not(.snap-slot--add)');
    expect(slots.length).toBe(8);
    await fireEvent.click(slots[4] as Element); // 5th slot, i=5 ("Build")
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick).toHaveBeenCalledWith(5);
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
