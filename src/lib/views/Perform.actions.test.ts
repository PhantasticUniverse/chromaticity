// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.4 Lane PERF.A Perform-view action-button tests. The W59
// `Perform.test.ts` covers the routed-view smoke axis; the Wave 2.1
// Lane A.2 `Perform.dispatch.test.ts` covers the per-drag IPC seam.
// This file pins the six previously-stub action buttons on Scene
// Controls + Meta Controls headers (Default / Random / Lock-all per
// panel) plus the dispatch-side guard introduced by the
// `controlLockStore`.
//
// Test surface:
//   1. Clicking "Default" on Scene Controls dispatches every
//      schema-position controlId with its declared default value.
//   2. Clicking "Random" on Scene Controls dispatches every
//      schema-position controlId with a value in [min, max].
//   3. Clicking the Lock-all button on Scene Controls flips
//      `aria-pressed=true` and locks every visible scene control.
//   4. A locked control's drag is a no-op — `set_control` is NOT
//      invoked while the lock store contains the id.
//   5. Clicking Lock-all a second time unlocks every control.
//   6. The Meta Controls Default / Random / Lock-all buttons mirror
//      the Scene buttons over the 1000+ ID namespace.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention.
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// `matchMedia` stub per `feedback_vitest_matchmedia_stubglobal.md` for
// the AudioReactivity / VuStereo rAF widgets composed inside the view.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import Perform from './Perform.svelte';
import { META_CONTROL_GROUPS, META_CONTROL_ID_BASE } from '../data/perform-fixtures';
import type { LoadedScenePayload } from '../ipc/bindings';
import { controlLockStore } from '../stores/controlLock.svelte';
import { loadedSceneStore } from '../stores/loadedScene.svelte';
import { performVariantStore } from '../stores/performVariant.svelte';

const SAMPLE_SCENE: LoadedScenePayload = {
  id: 'demo-multi',
  path: '/tmp/demo-multi.chroma',
  name: 'Demo Multi',
  credit: 'chromaticity demo',
  description: 'Three-control test schema',
  controls: [
    {
      name: 'segments',
      kind: 'Slider',
      default: 0.25,
      min: 0.0,
      max: 1.0,
      ui_group: 'geometry',
      description: null,
      label: null,
    },
    {
      name: 'speed',
      kind: 'Slider',
      default: 0.5,
      min: 0.0,
      max: 2.0,
      ui_group: 'geometry',
      description: null,
      label: null,
    },
    {
      name: 'mute',
      kind: 'Toggle',
      default: false,
      min: null,
      max: null,
      ui_group: 'geometry',
      description: null,
      label: null,
    },
  ],
  schema_version: 1,
};

function stubMatchMedia(): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

function stubRect(track: HTMLElement, height = 100, top = 0): void {
  track.getBoundingClientRect = (): DOMRect =>
    ({
      x: 0,
      y: top,
      left: 0,
      top,
      right: 10,
      bottom: top + height,
      width: 10,
      height,
      toJSON: () => ({}),
    }) as DOMRect;
}

function stubPointerCapture(target: HTMLElement): void {
  const captured = new Set<number>();
  target.setPointerCapture = (pointerId: number) => {
    captured.add(pointerId);
  };
  target.releasePointerCapture = (pointerId: number) => {
    captured.delete(pointerId);
  };
  target.hasPointerCapture = (pointerId: number) => captured.has(pointerId);
}

function defaultInvokeMock(): void {
  let lastSetValue = 0;
  invokeMock.mockImplementation(async (cmd: string, args?: { value?: number }) => {
    if (cmd === 'set_control') {
      lastSetValue = args?.value ?? 0;
      return undefined;
    }
    if (cmd === 'get_control') return lastSetValue;
    if (cmd === 'loaded_scene') return null;
    if (cmd === 'load_scene') return null;
    return null;
  });
}

/** Collect every `set_control` `{ id, value }` recorded by the
 *  invoke mock, ordered by call sequence. */
function setControlCalls(): Array<{ id: number; value: number }> {
  return invokeMock.mock.calls
    .filter((c) => c[0] === 'set_control')
    .map((c) => {
      const args = c[1] as { id: number; value: number } | undefined;
      return { id: args?.id ?? -1, value: args?.value ?? 0 };
    });
}

beforeEach(() => {
  stubMatchMedia();
  invokeMock.mockReset();
  defaultInvokeMock();
  performVariantStore.set('classic');
  controlLockStore.reset();
  loadedSceneStore.snapshot = SAMPLE_SCENE;
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  cleanup();
  loadedSceneStore.reset();
  performVariantStore.set('classic');
  controlLockStore.reset();
});

describe('Perform view — action buttons (Wave 2.4 Lane PERF.A)', () => {
  it('clicking Default on Scene Controls dispatches each control to its declared default', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const head = sceneCtrls.querySelector('.panel__head') as HTMLElement | null;
    expect(head).not.toBeNull();
    if (!head) return;
    const defaultBtn = within(head).getByRole('button', { name: /Default/ });
    invokeMock.mockClear();
    await fireEvent.click(defaultBtn);
    const calls = setControlCalls();
    // Three controls in the sample scene → three dispatches.
    expect(calls.length).toBe(3);
    // segments default=0.25, normalised over [0,1] → 0.25.
    expect(calls).toContainEqual({ id: 0, value: 0.25 });
    // speed default=0.5 normalised over [0,2] → 0.25.
    expect(calls).toContainEqual({ id: 1, value: 0.25 });
    // mute toggle default=false → 0.
    expect(calls).toContainEqual({ id: 2, value: 0 });
  });

  it('clicking Random on Scene Controls dispatches a value in [0,1] for every control', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    // Header "Random" button is the only one that reads "Random" as
    // visible text — group-level Randomize buttons are icon-only with
    // aria-label="Randomize". Scope to the header by the panel head.
    const head = sceneCtrls.querySelector('.panel__head') as HTMLElement | null;
    expect(head).not.toBeNull();
    if (!head) return;
    const randomBtn = within(head).getByRole('button', { name: /Random/ });
    invokeMock.mockClear();
    await fireEvent.click(randomBtn);
    const calls = setControlCalls();
    // Three controls → three dispatches; all in [0,1].
    expect(calls.length).toBe(3);
    for (const call of calls) {
      expect(call.value).toBeGreaterThanOrEqual(0);
      expect(call.value).toBeLessThanOrEqual(1);
    }
    // Every schema-position id should appear exactly once.
    const ids = calls.map((c) => c.id).sort();
    expect(ids).toEqual([0, 1, 2]);
  });

  it('clicking Lock-all on Scene Controls marks aria-pressed and blocks subsequent dispatch', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const lockBtn = within(sceneCtrls).getByRole('button', { name: /Lock all controls/ });
    expect(lockBtn).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(lockBtn);
    expect(lockBtn).toHaveAttribute('aria-pressed', 'true');
    // All three scene control ids should now be locked.
    expect(controlLockStore.isLocked(0)).toBe(true);
    expect(controlLockStore.isLocked(1)).toBe(true);
    expect(controlLockStore.isLocked(2)).toBe(true);

    // Dragging a locked fader should NOT invoke set_control.
    invokeMock.mockClear();
    const slider = within(sceneCtrls).getByRole('slider', { name: 'segments' });
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 20, pointerId: 1 });
    const dispatchCalls = invokeMock.mock.calls.filter((c) => c[0] === 'set_control');
    expect(dispatchCalls.length).toBe(0);
  });

  it('clicking Lock-all a second time unlocks every control', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const lockBtn = within(sceneCtrls).getByRole('button', { name: /Lock all controls/ });
    await fireEvent.click(lockBtn);
    expect(controlLockStore.isLocked(0)).toBe(true);
    // Second click — button label flips to "Unlock all controls".
    const unlockBtn = within(sceneCtrls).getByRole('button', { name: /Unlock all controls/ });
    await fireEvent.click(unlockBtn);
    expect(controlLockStore.isLocked(0)).toBe(false);
    expect(controlLockStore.isLocked(1)).toBe(false);
    expect(controlLockStore.isLocked(2)).toBe(false);
  });

  it('clicking Default on Meta Controls dispatches every meta control to its fixture default', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    const head = metaCtrls.querySelector('.panel__head') as HTMLElement | null;
    expect(head).not.toBeNull();
    if (!head) return;
    const defaultBtn = within(head).getByRole('button', { name: 'Default' });
    invokeMock.mockClear();
    await fireEvent.click(defaultBtn);
    const calls = setControlCalls();
    // Every fixture meta-control row with a controlId should have
    // fired one set_control. Count expected dispatchable controls.
    const expectedDispatchCount = META_CONTROL_GROUPS.flatMap((g) => g.controls).filter(
      (c) => c.kind !== 'swatch' && 'controlId' in c && c.controlId !== undefined,
    ).length;
    expect(calls.length).toBe(expectedDispatchCount);
    // The brightness control (ID base + 1) has fixture default 0.0.
    expect(calls).toContainEqual({ id: META_CONTROL_ID_BASE + 1, value: 0.0 });
    // The invert toggle (ID base + 0) defaults to false → 0.
    expect(calls).toContainEqual({ id: META_CONTROL_ID_BASE, value: 0 });
  });

  it('clicking Random on Meta Controls produces a value in [0,1] for every dispatchable control', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    const head = metaCtrls.querySelector('.panel__head') as HTMLElement | null;
    expect(head).not.toBeNull();
    if (!head) return;
    const randomBtn = within(head).getByRole('button', { name: /Random/ });
    invokeMock.mockClear();
    await fireEvent.click(randomBtn);
    const calls = setControlCalls();
    expect(calls.length).toBeGreaterThan(0);
    for (const call of calls) {
      expect(call.value).toBeGreaterThanOrEqual(0);
      expect(call.value).toBeLessThanOrEqual(1);
      // Every id is in the meta namespace (>= META_CONTROL_ID_BASE).
      expect(call.id).toBeGreaterThanOrEqual(META_CONTROL_ID_BASE);
    }
  });

  it('clicking Lock-all on Meta Controls blocks every meta dispatch', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    const lockBtn = within(metaCtrls).getByRole('button', { name: /Lock all controls/ });
    await fireEvent.click(lockBtn);
    // The brightness control should be locked.
    expect(controlLockStore.isLocked(META_CONTROL_ID_BASE + 1)).toBe(true);
    invokeMock.mockClear();
    // Dragging the brightness fader should NOT invoke set_control.
    const slider = within(metaCtrls).getByRole('slider', { name: 'brightness' });
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 10, pointerId: 1 });
    const dispatchCalls = invokeMock.mock.calls.filter((c) => c[0] === 'set_control');
    expect(dispatchCalls.length).toBe(0);
  });

  it('per-row Lock icon in a ControlGroup header toggles only that group', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    // The Meta Controls panel renders six groups; each ControlGroup's
    // header carries Lock / Reset / Random buttons. The first group
    // is "color" with controlIds 1000-1005.
    const lockButtons = within(metaCtrls).getAllByRole('button', { name: 'Lock controls' });
    expect(lockButtons.length).toBeGreaterThan(0);
    const firstLock = lockButtons[0];
    expect(firstLock).toBeDefined();
    if (!firstLock) return;
    await fireEvent.click(firstLock);
    // Color group controls (1000-1005) should be locked; mirror group
    // controls (1010+) should NOT be locked.
    expect(controlLockStore.isLocked(META_CONTROL_ID_BASE)).toBe(true);
    expect(controlLockStore.isLocked(META_CONTROL_ID_BASE + 1)).toBe(true);
    expect(controlLockStore.isLocked(META_CONTROL_ID_BASE + 10)).toBe(false);
  });

  it('locked rows render with data-locked="true" for the per-row dim affordance', async () => {
    const { container, getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const lockBtn = within(sceneCtrls).getByRole('button', { name: /Lock all controls/ });
    await fireEvent.click(lockBtn);
    // Every cg-row inside Scene Controls should now flag data-locked.
    const lockedRows = sceneCtrls.querySelectorAll('.cg-row[data-locked="true"]');
    expect(lockedRows.length).toBeGreaterThan(0);
    // Unlock — rows should flip back.
    const unlockBtn = within(sceneCtrls).getByRole('button', { name: /Unlock all controls/ });
    await fireEvent.click(unlockBtn);
    const stillLockedRows = container.querySelectorAll('.cg-row[data-locked="true"]');
    // After unlock, no scene-control row should still be locked. Meta
    // panel rows may still match if they had been locked in a prior
    // case; the suite's `beforeEach` reset guarantees the meta panel
    // is unlocked at test start.
    expect(stillLockedRows.length).toBe(0);
  });
});
