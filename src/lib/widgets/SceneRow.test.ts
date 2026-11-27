// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Shared-primitive backfill — `SceneRow` widget tests. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
// lines 232-249. Covers structural mount, idx zero-padding, name +
// duration round-trip, tag-class conditional ("default" / "hot" /
// "caution"), active branch (aria-current + .active class + pulse
// dot), cued branch, optional bpmSync marker, and click dispatch.
// The thumbnail is the W36 SceneStill canvas — jsdom does not paint
// pixels, so we assert the canvas mounts but do not inspect the bitmap.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import SceneRow from './SceneRow.svelte';

describe('SceneRow widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the row with idx zero-padded to two digits', () => {
    const { container } = render(SceneRow, {
      props: { idx: 3, name: 'Glitch' },
    });
    expect(container.querySelector('.scene-row__idx')?.textContent).toBe('03');
  });

  it('renders idx >= 10 without truncation', () => {
    const { container } = render(SceneRow, {
      props: { idx: 14, name: 'Test Card' },
    });
    expect(container.querySelector('.scene-row__idx')?.textContent).toBe('14');
  });

  it('renders the scene name + duration', () => {
    const { container } = render(SceneRow, {
      props: { idx: 1, name: 'Sun_Set', dur: '01:30' },
    });
    expect(container.querySelector('.scene-row__name')?.textContent).toBe('Sun_Set');
    expect(container.textContent).toContain('01:30');
  });

  it('mounts the SceneStill canvas thumbnail', () => {
    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Quicksand', kind: 'dunes', seed: 2 },
    });
    expect(container.querySelector('.scene-row__thumb canvas')).not.toBeNull();
  });

  it('renders the tag chip and reflects the tag in data-tag', () => {
    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Glitch', tag: 'hot' },
    });
    const tag = container.querySelector('.scene-row__tag');
    expect(tag?.textContent).toBe('hot');
    expect(tag?.getAttribute('data-tag')).toBe('hot');
  });

  it('omits active state by default — no aria-current, no pulse dot', () => {
    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Glob' },
    });
    const row = container.querySelector('.scene-row');
    expect(row?.classList.contains('active')).toBe(false);
    expect(row?.hasAttribute('aria-current')).toBe(false);
    expect(container.querySelector('.scene-row__pulse')).toBeNull();
  });

  it('renders the active treatment when active=true', () => {
    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Glitch', active: true },
    });
    const row = container.querySelector('.scene-row');
    expect(row?.classList.contains('active')).toBe(true);
    expect(row?.getAttribute('aria-current')).toBe('true');
    expect(container.querySelector('.scene-row__pulse')).not.toBeNull();
  });

  it('applies the cued class when cued=true', () => {
    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Glob', cued: true },
    });
    expect(container.querySelector('.scene-row')?.classList.contains('cued')).toBe(true);
  });

  it('renders the bpmSync marker only when supplied', () => {
    const without = render(SceneRow, {
      props: { idx: 0, name: 'Plain' },
    });
    expect(without.container.querySelector('.scene-row__bpm')).toBeNull();
    without.unmount();

    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Sync Scene', bpmSync: '1/4' },
    });
    const bpm = container.querySelector('.scene-row__bpm');
    expect(bpm).not.toBeNull();
    expect(bpm?.textContent).toContain('1/4');
  });

  it('invokes onClick when the row is clicked', async () => {
    const onClick = vi.fn();
    const { container } = render(SceneRow, {
      props: { idx: 0, name: 'Glob', onClick },
    });
    const row = container.querySelector('.scene-row');
    expect(row).not.toBeNull();
    await fireEvent.click(row as Element);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
