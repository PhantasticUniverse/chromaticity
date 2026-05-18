// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane A.2 Perform-view dispatch integration tests. The W59
// `Perform.test.ts` sibling covers the routed-view smoke axis; this
// file pins the cross-seam contract between Scene Controls / Meta
// Controls widgets and the `set_control` / `get_control` IPC pair via
// the `controlStore.dispatchAndRead` round-trip.
//
// Test surface:
//   1. Scene-control fader drag fires `set_control` with the
//      schema-position `controlId` (the index in
//      `loadedSceneStore.snapshot.controls`).
//   2. The engine read-back value flows back into the widget state
//      (`controlValues` derived overlay).
//   3. Meta-control fader drag fires `set_control` with the 1000+
//      offset `controlId` namespace.
//   4. Meta-control toggle click encodes boolean as numeric 0/1 on
//      the wire (the engine `ControlValue` is `f32`).
//   5. Scene-control toggle click fires set_control with 0/1 encoding.
//
// `vi.hoisted` invoke-mock pattern per the
// `feedback_vitest_invoke_mock_hoisted.md` user-memory convention.
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// `matchMedia` stub per `feedback_vitest_matchmedia_stubglobal.md` for
// the AudioReactivity / VuStereo rAF widgets composed inside the view.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import Perform from './Perform.svelte';
import { META_CONTROL_ID_BASE } from '../data/perform-fixtures';
import type { LoadedScenePayload } from '../ipc/bindings';
import { loadedSceneStore } from '../stores/loadedScene.svelte';
import { performVariantStore } from '../stores/performVariant.svelte';

const SAMPLE_SCENE: LoadedScenePayload = {
  id: 'demo-kaleido',
  path: '/tmp/demo-kaleido.chroma',
  name: 'Demo Kaleido',
  credit: 'chromaticity demo',
  description: 'Six-fold radial mirror',
  controls: [
    {
      name: 'segments',
      kind: 'Slider',
      default: 0.5,
      min: 0.0,
      max: 1.0,
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

const SAMPLE_SCENE_B: LoadedScenePayload = {
  ...SAMPLE_SCENE,
  id: 'demo-spectrum',
  path: '/tmp/demo-spectrum.chroma',
  name: 'Demo Spectrum',
  controls: [
    {
      name: 'segments',
      kind: 'Slider',
      default: 0.2,
      min: 0.0,
      max: 1.0,
      ui_group: 'geometry',
      description: null,
      label: null,
    },
    {
      name: 'mute',
      kind: 'Toggle',
      default: true,
      min: null,
      max: null,
      ui_group: 'geometry',
      description: null,
      label: null,
    },
  ],
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

/**
 * Default invoke mock — set_control resolves void, get_control returns
 * the same value the caller set, and loaded_scene (hydrate) returns
 * null so the scene-mount $effect doesn't overwrite our pre-seeded
 * store snapshot. Per-test overrides re-route via mockImplementation.
 */
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

beforeEach(() => {
  stubMatchMedia();
  invokeMock.mockReset();
  defaultInvokeMock();
  performVariantStore.set('classic');
  // Seed the loaded-scene store so Scene Controls binds to a real
  // schema (two interactive controls at positions 0 + 1).
  loadedSceneStore.snapshot = SAMPLE_SCENE;
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  cleanup();
  loadedSceneStore.reset();
  performVariantStore.set('classic');
});

describe('Perform view — control dispatch (Wave 2.1 Lane A.2)', () => {
  it('dragging a scene-control fader dispatches set_control with the schema-position id', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    // The fader exposes aria-label="segments" (schema position 0).
    const slider = within(sceneCtrls).getByRole('slider', { name: 'segments' });
    expect(slider).toBeInTheDocument();
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    // clientY=20 → value = 1 - 20/100 = 0.8.
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 20, pointerId: 1 });
    // First invoke = set_control with id=0 (schema position).
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 0, value: 0.8 });
  });

  it('engine read-back value flows into the widget aria-valuenow', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_control') return undefined;
      // Engine clamps + reports a different value than the UI sent.
      if (cmd === 'get_control') return 0.42;
      if (cmd === 'loaded_scene') return null;
      return null;
    });
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const slider = within(sceneCtrls).getByRole('slider', { name: 'segments' });
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 20, pointerId: 1 });
    // Flush the dispatchAndRead promise chain so the readback updates
    // controlValues and re-renders the widget.
    await Promise.resolve();
    await Promise.resolve();
    expect(slider.getAttribute('aria-valuenow')).toBe('0.42');
  });

  it('does not reuse a touched scene-control value after the loaded scene id changes', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const slider = within(sceneCtrls).getByRole('slider', { name: 'segments' });
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 20, pointerId: 1 });
    await Promise.resolve();
    await Promise.resolve();
    expect(slider.getAttribute('aria-valuenow')).toBe('0.8');

    loadedSceneStore.snapshot = SAMPLE_SCENE_B;
    await tick();

    const nextSlider = within(sceneCtrls).getByRole('slider', { name: 'segments' });
    expect(nextSlider.getAttribute('aria-valuenow')).toBe('0.2');
  });

  it('toggling a scene-control switch encodes boolean as numeric 1.0', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    // mute is at schema position 1, default off → click should set 1.
    const toggle = within(sceneCtrls).getByRole('switch', { name: 'mute' });
    await fireEvent.click(toggle);
    expect(invokeMock).toHaveBeenCalledWith('set_control', { id: 1, value: 1 });
  });

  it('dragging a meta-control fader dispatches with a 1000+ control id', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    // Brightness is META_CONTROL_ID_BASE + 1 = 1001 per the fixture.
    const slider = within(metaCtrls).getByRole('slider', { name: 'brightness' });
    expect(slider).toBeInTheDocument();
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 10, pointerId: 1 });
    // value = 1 - 10/100 = 0.9.
    expect(invokeMock).toHaveBeenCalledWith('set_control', {
      id: META_CONTROL_ID_BASE + 1,
      value: 0.9,
    });
  });

  it('clicking a meta-control toggle encodes as numeric 1 (off → on)', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    // "invert" is META_CONTROL_ID_BASE + 0 = 1000, default off.
    const toggle = within(metaCtrls).getByRole('switch', { name: 'invert' });
    await fireEvent.click(toggle);
    expect(invokeMock).toHaveBeenCalledWith('set_control', {
      id: META_CONTROL_ID_BASE,
      value: 1,
    });
  });

  it('clicking a meta-control toggle that is on encodes as numeric 0', async () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    // "invert media" is on by default per the fixture (controlId 1020).
    const toggle = within(metaCtrls).getByRole('switch', { name: 'invert media' });
    await fireEvent.click(toggle);
    expect(invokeMock).toHaveBeenCalledWith('set_control', {
      id: 1020,
      value: 0,
    });
  });

  it('meta-control swatches stay display-only (no onchange wired)', () => {
    const { getByLabelText } = render(Perform);
    const metaCtrls = getByLabelText('Meta Controls');
    // The limit-colors swatches render but don't expose role="switch"
    // / role="slider" — they're presentational <div>s.
    const swatchWraps = metaCtrls.querySelectorAll('.swatch-wrap');
    expect(swatchWraps.length).toBeGreaterThan(0);
    // None of the swatches should expose an interactive role.
    swatchWraps.forEach((wrap) => {
      expect(wrap.querySelector('[role="switch"]')).toBeNull();
      expect(wrap.querySelector('[role="slider"]')).toBeNull();
    });
  });

  it('drag-then-release dispatches the final value via set_control once per pointer-event', async () => {
    const { getByLabelText } = render(Perform);
    const sceneCtrls = getByLabelText('Scene Controls');
    const slider = within(sceneCtrls).getByRole('slider', { name: 'segments' });
    stubRect(slider, 100, 0);
    stubPointerCapture(slider);
    // Track all set_control calls.
    const calls: number[] = [];
    invokeMock.mockImplementation(async (cmd: string, args?: { value?: number }) => {
      if (cmd === 'set_control') {
        calls.push(args?.value ?? -1);
        return undefined;
      }
      if (cmd === 'get_control') return args?.value ?? 0;
      if (cmd === 'loaded_scene') return null;
      return null;
    });
    // Pointerdown at clientY=30 → value 0.7.
    await fireEvent.pointerDown(slider, { clientX: 5, clientY: 30, pointerId: 1 });
    // Pointermove to clientY=50 → value 0.5.
    await fireEvent.pointerMove(slider, { clientX: 5, clientY: 50, pointerId: 1 });
    // Pointerup at clientY=50 (no further onchange).
    await fireEvent.pointerUp(slider, { clientX: 5, clientY: 50, pointerId: 1 });
    expect(calls).toEqual([0.7, 0.5]);
  });
});
