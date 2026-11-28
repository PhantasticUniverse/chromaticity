// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W49 TopBar shell tests — exercises the W15 PR1 / W34a top bar shell.
// Covers the brand wordmark + per-route subtitle, the three adapter
// pills (OUT / MIDI / OSC, post-Wave-2.4-PERF.D bound to live store
// state), the FPS / CPU em-dash placeholders, the REC indicator's
// default off state, and the GO LIVE button. `routeStore` is reset
// to `'perform'` in `afterEach` so cross-test state cannot leak.
//
// Wave 2.4 Lane PERF.D added store-driven adapter pill cases; the
// `invokeMock` is hoisted per `feedback_vitest_invoke_mock_hoisted.md`
// so the `@tauri-apps/api/core` mock can route both the
// `osc_health_snapshot` poll and the `list_midi_devices` enumeration.
//
// `cleanup()` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { MidiDevicePayload, OscHealthSnapshot } from './ipc/bindings';
import { audioSnapshotStore } from './stores/audioSnapshot.svelte';
import { bpmControlStore } from './stores/bpmControl.svelte';
import { djAutopilotStore } from './stores/djAutopilot.svelte';
import { loadedSceneStore } from './stores/loadedScene.svelte';
import { midiStore } from './stores/midiStore.svelte';
import { oscHealthStore } from './stores/oscHealth.svelte';
import { performVariantStore } from './stores/performVariant.svelte';
import { routeStore } from './stores/route.svelte';
import { transportStore } from './stores/transport.svelte';
import TopBar from './TopBar.svelte';

// Hoisted `invoke` mock per `feedback_vitest_invoke_mock_hoisted.md`.
// Default behaviour rejects (no Tauri context) so default-render tests
// observe the stores in their "no-data" baseline state; per-test
// overrides via `invokeMock.mockResolvedValue(...)` exercise the live
// branches.
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

// Shared-primitive backfill — TopBar now mounts the `Timecode` widget
// when the active route is performance-adjacent (perform / author /
// audio-uniforms). `Timecode` reads `prefers-reduced-motion` via
// `window.matchMedia` in a mount-time `$effect`; jsdom does not pre-
// expose `matchMedia`, so the test harness needs the
// `vi.stubGlobal('matchMedia', ...)` shim per the user-memory
// `feedback_vitest_matchmedia_stubglobal.md`.
function stubMatchMedia(reduce = false): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: reduce && query.includes('prefers-reduced-motion: reduce'),
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

describe('TopBar shell component', () => {
  beforeEach(() => {
    stubMatchMedia();
    // Default: every invoke rejects (no Tauri context). Stores stay in
    // their no-data baseline. Per-test calls of
    // `invokeMock.mockImplementation(...)` route specific commands.
    invokeMock.mockReset();
    invokeMock.mockRejectedValue(new Error('no Tauri context in test'));
    // Replace setInterval with a no-op so the OSC poll loop doesn't
    // actually schedule recurring work across tests. The mount-time
    // `refresh()` call still runs and routes through the invoke mock.
    vi.useFakeTimers();
  });

  afterEach(() => {
    // Cleanup BEFORE unstubbing globals — `cleanup()` unmounts the
    // mounted components, which fires the `Timecode` $effect teardown
    // (`removeEventListener` against the cached MediaQueryList from the
    // stubbed matchMedia). Unstubbing globals first would invalidate
    // that listener handle and surface a "matchMedia is not a function"
    // error during the unmount phase.
    cleanup();
    // Wave 2.1 Lane B.2 — the TopBar `$effect` block calls
    // `transportStore.start()` on mount; cleanup() invokes the matching
    // teardown which calls `stop()`. Reset elapsedMs explicitly to keep
    // sibling tests deterministic.
    transportStore.stop();
    transportStore.reset();
    // Clear any audio snapshot a test injected so the next test starts
    // from the default BPM fallback.
    audioSnapshotStore.snapshot = null;
    bpmControlStore.reset();
    djAutopilotStore.reset();
    // Wave 2.4 Lane PERF.D — reset the three pill-driving stores so
    // a snapshot leak from one test doesn't bleed into the next.
    loadedSceneStore.reset();
    midiStore.reset();
    oscHealthStore.reset();
    routeStore.set('perform');
    performVariantStore.set('classic');
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('renders the Chromaticity wordmark', () => {
    const { container } = render(TopBar);
    expect(container.querySelector('.top-bar__wordmark')?.textContent).toBe('Chromaticity');
  });

  it('renders the per-route subtitle for the active routeStore entry', () => {
    routeStore.set('library');
    const { container } = render(TopBar);
    expect(container.querySelector('.top-bar__title')?.textContent).toBe('Library');
  });

  it('updates the subtitle when the routeStore changes', async () => {
    const { container } = render(TopBar);
    expect(container.querySelector('.top-bar__title')?.textContent).toBe('Perform');
    routeStore.set('osc');
    // Svelte 5 runes propagate synchronously after the assignment; assert
    // the next microtask to give the effect graph one tick to flush.
    await Promise.resolve();
    expect(container.querySelector('.top-bar__title')?.textContent).toBe(
      'OSC · PREVIEW — Port 9000',
    );
  });

  it('renders four adapter pills with the OUT / AUDIO / MIDI / OSC kind labels', () => {
    const { container } = render(TopBar);
    const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
    expect(pills.length).toBe(4);
    const kinds = Array.from(pills).map((p) =>
      p.querySelector('.adapter-pill__kind')?.textContent?.trim(),
    );
    expect(kinds).toEqual(['OUT', 'AUDIO', 'MIDI', 'OSC']);
  });

  it('defaults all four adapter pills to off when no live data is present', () => {
    // Wave 2.4 Lane PERF.D — without a loaded scene, MIDI devices, or
    // an OSC health snapshot, every pill renders the off baseline.
    const { container } = render(TopBar);
    const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
    expect(pills[0]?.getAttribute('data-state')).toBe('off');
    expect(pills[1]?.getAttribute('data-state')).toBe('off');
    expect(pills[2]?.getAttribute('data-state')).toBe('off');
    expect(pills[3]?.getAttribute('data-state')).toBe('off');
  });

  it('groups the adapter pills under an Adapter status aria-label', () => {
    const { container } = render(TopBar);
    const group = container.querySelector('.top-bar__pills');
    expect(group?.getAttribute('role')).toBe('group');
    expect(group?.getAttribute('aria-label')).toBe('Adapter status');
  });

  it('renders honest em-dash placeholders for FPS and GPU when telemetry is unavailable', () => {
    const { container } = render(TopBar);
    const values = container.querySelectorAll('.top-bar__metric-value');
    expect(values.length).toBe(2);
    expect(values[0]?.textContent?.trim()).toBe('—');
    expect(values[1]?.textContent?.trim()).toBe('—');
  });

  it('exposes the GPU placeholder value via the dedicated test id', () => {
    const { getByTestId } = render(TopBar);
    expect(getByTestId('top-bar-gpu-value').textContent?.trim()).toBe('—');
  });

  it('renders the REC indicator with the off data-state default', () => {
    const { container } = render(TopBar);
    const rec = container.querySelector('.top-bar__rec');
    expect(rec).not.toBeNull();
    expect(rec?.getAttribute('data-state')).toBe('off');
    expect(rec?.textContent).toContain('REC');
  });

  it('renders a GO LIVE button with a play-glyph icon', () => {
    const { container } = render(TopBar);
    const button = container.querySelector('button.top-bar__go-live');
    expect(button).not.toBeNull();
    expect(button?.textContent).toContain('GO LIVE');
    const glyph = button?.querySelector('.top-bar__go-live-glyph svg');
    expect(glyph).not.toBeNull();
    expect(glyph?.getAttribute('width')).toBe('11');
  });

  // Wave 2.1 Lane B.1 — STANDARD / CONSOLE variant toggle. Renders only
  // on the Perform route; the active button reflects
  // `performVariantStore.current`.
  describe('Perform variant toggle (Lane B.1)', () => {
    it('renders the STANDARD / CONSOLE toggle when the active route is Perform', () => {
      routeStore.set('perform');
      const { container } = render(TopBar);
      const toggle = container.querySelector('.top-bar__variant');
      expect(toggle).not.toBeNull();
      expect(toggle?.getAttribute('role')).toBe('group');
      expect(toggle?.getAttribute('aria-label')).toBe('Perform layout variant');
      const buttons = toggle?.querySelectorAll('.top-bar__variant-btn');
      expect(buttons?.length).toBe(2);
      expect(buttons?.[0]?.textContent?.trim()).toBe('STANDARD');
      expect(buttons?.[1]?.textContent?.trim()).toBe('CONSOLE');
    });

    it('hides the variant toggle on every non-Perform route', () => {
      routeStore.set('library');
      const { container } = render(TopBar);
      expect(container.querySelector('.top-bar__variant')).toBeNull();
    });

    it('marks STANDARD as active by default (classic variant)', () => {
      routeStore.set('perform');
      const { container } = render(TopBar);
      const buttons = container.querySelectorAll('.top-bar__variant-btn');
      expect(buttons[0]?.getAttribute('aria-pressed')).toBe('true');
      expect(buttons[0]).toHaveClass('top-bar__variant-btn--active');
      expect(buttons[1]?.getAttribute('aria-pressed')).toBe('false');
      expect(buttons[1]).not.toHaveClass('top-bar__variant-btn--active');
    });

    it('clicking CONSOLE flips the variant store + active button', async () => {
      routeStore.set('perform');
      const { container } = render(TopBar);
      const buttons = container.querySelectorAll('.top-bar__variant-btn');
      const consoleBtn = buttons[1];
      expect(consoleBtn).toBeDefined();
      if (!consoleBtn) return;
      await fireEvent.click(consoleBtn);
      // Store mutates synchronously.
      expect(performVariantStore.current).toBe('console');
      // Microtask flush for the reactive class update.
      await Promise.resolve();
      expect(consoleBtn.getAttribute('aria-pressed')).toBe('true');
      expect(consoleBtn).toHaveClass('top-bar__variant-btn--active');
      expect(buttons[0]?.getAttribute('aria-pressed')).toBe('false');
    });

    it('clicking STANDARD flips the variant store back to classic', async () => {
      routeStore.set('perform');
      performVariantStore.set('console');
      const { container } = render(TopBar);
      const buttons = container.querySelectorAll('.top-bar__variant-btn');
      const standardBtn = buttons[0];
      expect(standardBtn).toBeDefined();
      if (!standardBtn) return;
      await fireEvent.click(standardBtn);
      expect(performVariantStore.current).toBe('classic');
      await Promise.resolve();
      expect(standardBtn.getAttribute('aria-pressed')).toBe('true');
      expect(buttons[1]?.getAttribute('aria-pressed')).toBe('false');
    });
  });

  // Wave 2.1 Lane B.2 — top-bar transport HUD elements (timecode + beat
  // dots + BPM + VU meter + camera + record + GPU readout). Visible on
  // the Perform route, hidden on non-Perform routes.
  describe('Transport HUD (Lane B.2)', () => {
    it('renders the timecode chip on Perform with the HH:MM:SS:FF placeholder', () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const tc = getByTestId('top-bar-timecode');
      expect(tc).not.toBeNull();
      // Clock has not been driven; reads 00:00:00:00 at mount.
      expect(tc.textContent?.trim()).toBe('00:00:00:00');
    });

    it('hides the timecode chip on every non-Perform route', () => {
      routeStore.set('library');
      const { queryByTestId } = render(TopBar);
      expect(queryByTestId('top-bar-timecode')).toBeNull();
    });

    it('renders the beat-dots widget on Perform', () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const beat = getByTestId('top-bar-beat-dots');
      expect(beat).not.toBeNull();
      // BeatDots widget renders four .beat-dots__dot children.
      expect(beat.querySelectorAll('.beat-dots__dot').length).toBe(4);
    });

    it('hides the beat dots on every non-Perform route', () => {
      routeStore.set('osc');
      const { queryByTestId } = render(TopBar);
      expect(queryByTestId('top-bar-beat-dots')).toBeNull();
    });

    it('displays the default 120.0 BPM when no live snapshot is present', () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const bpm = getByTestId('top-bar-bpm');
      expect(bpm.textContent).toContain('BPM');
      expect(bpm.textContent).toContain('120.0');
    });

    it('reflects the audio snapshot store BPM when a live snapshot is present', async () => {
      routeStore.set('perform');
      audioSnapshotStore.snapshot = {
        bpm: 132.5,
        bpm_availability: 'Ready',
        rms: 0.4,
        peak: 0.7,
        fft_bands: [0, 0, 0, 0, 0, 0, 0, 0],
        fft_availability: 'Ready',
        degraded: false,
      };
      const { getByTestId } = render(TopBar);
      // Microtask flush for the $derived recomputation.
      await Promise.resolve();
      expect(getByTestId('top-bar-bpm').textContent).toContain('132.5');
    });

    it('renders the VU stereo meter on Perform', () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const vu = getByTestId('top-bar-vu');
      expect(vu).not.toBeNull();
      // VuStereo widget renders two .vu-row children.
      expect(vu.querySelectorAll('.vu-row').length).toBe(2);
    });

    it('renders the camera button with the correct tooltip', () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const cam = getByTestId('top-bar-camera');
      expect(cam.tagName).toBe('BUTTON');
      expect(cam.getAttribute('title')).toBe('Camera (Wave 2.2)');
      expect(cam.getAttribute('aria-label')).toBe('Camera');
    });

    it('renders the record button with the live start recording tooltip', () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const rec = getByTestId('top-bar-record');
      expect(rec.tagName).toBe('BUTTON');
      expect(rec.getAttribute('title')).toBe('Start recording');
      expect(rec.getAttribute('aria-label')).toBe('Record');
      expect(rec.getAttribute('aria-pressed')).toBe('false');
    });

    it('hides the camera and record buttons on non-Perform routes', () => {
      routeStore.set('settings');
      const { queryByTestId } = render(TopBar);
      expect(queryByTestId('top-bar-camera')).toBeNull();
      expect(queryByTestId('top-bar-record')).toBeNull();
    });

    it('hides the BPM and VU chips on non-Perform routes', () => {
      routeStore.set('audio-uniforms');
      const { queryByTestId } = render(TopBar);
      expect(queryByTestId('top-bar-bpm')).toBeNull();
      expect(queryByTestId('top-bar-vu')).toBeNull();
    });
  });

  // Wave 2.4 Lane PERF.D — live adapter pill bindings. Each pill reads
  // from a different store; tests exercise the three healthy / off
  // branches plus the OSC pill's recovering / degraded mappings.
  describe('Adapter pill live bindings (Lane PERF.D)', () => {
    it('flips the OUT pill to ok when a scene is loaded', async () => {
      loadedSceneStore.snapshot = {
        id: 'scene-fixture',
        path: '/fake/scene-fixture.chroma',
        name: 'Test Scene',
        credit: '',
        description: '',
        controls: [],
        schema_version: 1,
      };
      const { container } = render(TopBar);
      await Promise.resolve();
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[0]?.getAttribute('data-state')).toBe('ok');
      expect(pills[0]?.getAttribute('title')).toContain('Test Scene');
    });

    it('keeps the OUT pill off when no scene is loaded', () => {
      const { container } = render(TopBar);
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[0]?.getAttribute('data-state')).toBe('off');
      expect(pills[0]?.getAttribute('title')).toContain('No scene loaded');
    });

    it('flips the MIDI pill to ok when at least one device is enumerated', async () => {
      const devices: MidiDevicePayload[] = [{ id: 'input-0', name: 'Launchpad X', kind: 'Input' }];
      midiStore.devices = devices;
      const { container } = render(TopBar);
      await Promise.resolve();
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[2]?.getAttribute('data-state')).toBe('ok');
      expect(pills[2]?.getAttribute('title')).toContain('1 device');
    });

    it('keeps the MIDI pill off when no devices are enumerated', () => {
      midiStore.devices = [];
      const { container } = render(TopBar);
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[2]?.getAttribute('data-state')).toBe('off');
      expect(pills[2]?.getAttribute('title')).toContain('No MIDI devices');
    });

    it('maps degraded audio snapshots to the red AUDIO pill state', async () => {
      audioSnapshotStore.snapshot = {
        bpm: 128,
        bpm_availability: 'Ready',
        rms: 0.3,
        peak: 0.5,
        fft_bands: [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1],
        fft_availability: 'Ready',
        degraded: true,
      };
      const { container } = render(TopBar);
      await Promise.resolve();
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[1]?.querySelector('.adapter-pill__kind')?.textContent?.trim()).toBe('AUDIO');
      expect(pills[1]?.getAttribute('data-state')).toBe('disconnected');
      expect(pills[1]?.getAttribute('title')).toContain('degraded');
    });

    it('maps OSC Healthy snapshot to the ok pill state', async () => {
      const snapshot: OscHealthSnapshot = {
        state: 'Healthy',
        attempted_binds: 1,
        local_addr: '127.0.0.1:9000',
        compat_alias_emits: 0n,
      };
      oscHealthStore.snapshot = snapshot;
      const { container } = render(TopBar);
      await Promise.resolve();
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[3]?.getAttribute('data-state')).toBe('ok');
      expect(pills[3]?.getAttribute('title')).toContain('127.0.0.1:9000');
    });

    it('maps OSC Recovering snapshot to the warming pill state', async () => {
      oscHealthStore.snapshot = {
        state: 'Recovering',
        attempted_binds: 3,
        local_addr: null,
        compat_alias_emits: 0n,
      };
      const { container } = render(TopBar);
      await Promise.resolve();
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[3]?.getAttribute('data-state')).toBe('warming');
    });

    it('maps OSC Degraded snapshot to the disconnected pill state', async () => {
      oscHealthStore.snapshot = {
        state: 'Degraded',
        attempted_binds: 17,
        local_addr: null,
        compat_alias_emits: 0n,
      };
      const { container } = render(TopBar);
      await Promise.resolve();
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[3]?.getAttribute('data-state')).toBe('disconnected');
    });

    it('renders the OSC pill in the off state when no snapshot has arrived', () => {
      const { container } = render(TopBar);
      const pills = container.querySelectorAll('.top-bar__pills .adapter-pill');
      expect(pills[3]?.getAttribute('data-state')).toBe('off');
      expect(pills[3]?.getAttribute('title')).toContain('offline');
    });
  });

  describe('BPM Auto-Listen & Beatmatch popover', () => {
    it('renders the BPM pill with AUTO badge and accessible popover toggle attributes', () => {
      routeStore.set('perform');
      const { getByTestId, queryByTestId } = render(TopBar);
      const pill = getByTestId('top-bar-bpm');
      expect(pill.tagName).toBe('BUTTON');
      expect(pill.getAttribute('aria-expanded')).toBe('false');
      expect(pill.getAttribute('aria-controls')).toBe('top-bar-beatmatch-popover');
      expect(getByTestId('top-bar-bpm-mode').textContent?.trim()).toBe('AUTO');
      expect(queryByTestId('top-bar-beatmatch-popover')).toBeNull();
    });

    it('opens the beatmatch popover on click and closes on Escape or outside click', async () => {
      routeStore.set('perform');
      const { getByTestId, queryByTestId } = render(TopBar);
      const pill = getByTestId('top-bar-bpm');

      await fireEvent.click(pill);
      expect(pill.getAttribute('aria-expanded')).toBe('true');
      expect(getByTestId('top-bar-beatmatch-popover')).not.toBeNull();

      await fireEvent.keyDown(window, { key: 'Escape' });
      expect(pill.getAttribute('aria-expanded')).toBe('false');
      expect(queryByTestId('top-bar-beatmatch-popover')).toBeNull();

      await fireEvent.click(pill);
      expect(getByTestId('top-bar-beatmatch-popover')).not.toBeNull();

      await fireEvent.mouseDown(document.body);
      expect(queryByTestId('top-bar-beatmatch-popover')).toBeNull();
    });

    it('toggles BPM Auto-Listen -> Manual Lock -> Auto-Listen and freezes detected tempo while locked', async () => {
      routeStore.set('perform');
      audioSnapshotStore.snapshot = {
        bpm: 134.0,
        bpm_availability: 'Ready',
        rms: 0.4,
        peak: 0.7,
        fft_bands: [0, 0, 0, 0, 0, 0, 0, 0],
        fft_availability: 'Ready',
        degraded: false,
      };
      const { getByTestId } = render(TopBar);
      await Promise.resolve();
      expect(getByTestId('top-bar-bpm').textContent).toContain('134.0');
      expect(getByTestId('top-bar-bpm-mode').textContent?.trim()).toBe('AUTO');

      await fireEvent.click(getByTestId('top-bar-bpm'));
      await fireEvent.click(getByTestId('bpm-mode-lock'));
      await Promise.resolve();

      expect(bpmControlStore.isListening).toBe(false);
      expect(bpmControlStore.mode).toBe('manual');
      expect(bpmControlStore.manualBpm).toBe(134);
      expect(getByTestId('top-bar-bpm-mode').textContent?.trim()).toBe('LOCK');

      // Live detector drift is ignored while locked.
      audioSnapshotStore.snapshot = {
        ...audioSnapshotStore.snapshot,
        bpm: 146.5,
      };
      await Promise.resolve();
      expect(getByTestId('top-bar-bpm').textContent).toContain('134.0');

      // Switching back to Auto-Listen tracks live audio again.
      await fireEvent.click(getByTestId('bpm-mode-auto'));
      await Promise.resolve();
      expect(bpmControlStore.isListening).toBe(true);
      expect(bpmControlStore.mode).toBe('auto');
      expect(getByTestId('top-bar-bpm-mode').textContent?.trim()).toBe('AUTO');
      expect(getByTestId('top-bar-bpm').textContent).toContain('146.5');
    });

    it('supports manual BPM input, half/double buttons, and tap tempo', async () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      await fireEvent.click(getByTestId('top-bar-bpm'));

      const bpmInput = getByTestId('bpm-manual-input') as HTMLInputElement;
      await fireEvent.input(bpmInput, { target: { value: '140' } });
      await Promise.resolve();
      expect(bpmControlStore.mode).toBe('manual');
      expect(bpmControlStore.manualBpm).toBe(140);
      expect(bpmInput.value).toBe('140');
      expect(getByTestId('top-bar-bpm').textContent).toContain('140.0');

      await fireEvent.click(getByTestId('bpm-half'));
      await Promise.resolve();
      expect(bpmControlStore.manualBpm).toBe(70);

      await fireEvent.click(getByTestId('bpm-double'));
      await Promise.resolve();
      expect(bpmControlStore.manualBpm).toBe(140);

      // Two taps 500ms apart => 120 BPM.
      vi.setSystemTime(new Date('2026-01-01T00:00:00.000Z'));
      await fireEvent.click(getByTestId('bpm-tap'));
      vi.advanceTimersByTime(500);
      await fireEvent.click(getByTestId('bpm-tap'));
      await Promise.resolve();
      expect(bpmControlStore.mode).toBe('manual');
      expect(bpmControlStore.manualBpm).toBe(120);
    });

    it('scrubs beatmatch phase with ASCII range value and Unicode display, nudges platter, and resyncs downbeat', async () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      await fireEvent.click(getByTestId('top-bar-bpm'));

      const scrub = getByTestId('bpm-phase-scrub') as HTMLInputElement;
      expect(scrub.min).toBe('-0.5');
      expect(scrub.max).toBe('0.5');
      expect(scrub.step).toBe('0.01');

      await fireEvent.input(scrub, { target: { value: '-0.15' } });
      await Promise.resolve();
      expect(bpmControlStore.phaseOffsetBeats).toBe(-0.15);
      expect(scrub.value).toBe('-0.15');
      expect(getByTestId('bpm-phase-display').textContent?.trim()).toBe('\u22120.15 beats');

      await fireEvent.click(getByTestId('bpm-nudge-forward'));
      await Promise.resolve();
      expect(bpmControlStore.phaseOffsetBeats).toBe(-0.1);
      expect(getByTestId('bpm-phase-display').textContent?.trim()).toBe('\u22120.10 beats');

      await fireEvent.click(getByTestId('bpm-nudge-back'));
      await Promise.resolve();
      expect(bpmControlStore.phaseOffsetBeats).toBe(-0.15);

      await fireEvent.click(getByTestId('bpm-resync'));
      await Promise.resolve();
      expect(bpmControlStore.phaseOffsetBeats).toBe(0);
      expect(scrub.value).toBe('0');
      expect(getByTestId('bpm-phase-display').textContent?.trim()).toBe('0.00 beats');
    });

    it('switches to OFF mode showing BPM OFF badge and pausing BeatDots', async () => {
      const { getByTestId } = render(TopBar);
      await fireEvent.click(getByTestId('top-bar-bpm'));

      const offBtn = getByTestId('bpm-mode-off');
      await fireEvent.click(offBtn);
      await Promise.resolve();

      expect(bpmControlStore.isOff).toBe(true);
      expect(offBtn.getAttribute('aria-pressed')).toBe('true');
      expect(getByTestId('bpm-mode-auto').getAttribute('aria-pressed')).toBe('false');
      expect(getByTestId('bpm-mode-lock').getAttribute('aria-pressed')).toBe('false');
      expect(getByTestId('top-bar-bpm-mode').textContent?.trim()).toBe('OFF');
      expect(getByTestId('top-bar-bpm').textContent?.replace(/\s+/g, ' ').trim()).toBe('BPM OFF');
      const dots = getByTestId('top-bar-beat-dots').querySelector('[role="img"]');
      expect(dots?.getAttribute('aria-label')).toBe('Beat tracking off');

      // Switching back to Auto Listen restores live numeric BPM display.
      await fireEvent.click(getByTestId('bpm-mode-auto'));
      await Promise.resolve();
      expect(bpmControlStore.isListening).toBe(true);
      expect(getByTestId('top-bar-bpm-mode').textContent?.trim()).toBe('AUTO');
    });
  });

  describe('1-click DJ Autopilot toggle', () => {
    it('renders the DJ Autopilot button off by default and toggles djAutopilotStore', async () => {
      routeStore.set('perform');
      const { getByTestId } = render(TopBar);
      const djBtn = getByTestId('top-bar-dj-autopilot');

      expect(djBtn.tagName).toBe('BUTTON');
      expect(djBtn.getAttribute('aria-pressed')).toBe('false');
      expect(djBtn).not.toHaveClass('top-bar__dj--active');
      expect(djAutopilotStore.isActive).toBe(false);

      await fireEvent.click(djBtn);
      await Promise.resolve();

      expect(djAutopilotStore.isActive).toBe(true);
      expect(djBtn.getAttribute('aria-pressed')).toBe('true');
      expect(djBtn).toHaveClass('top-bar__dj--active');

      await fireEvent.click(djBtn);
      await Promise.resolve();

      expect(djAutopilotStore.isActive).toBe(false);
      expect(djBtn.getAttribute('aria-pressed')).toBe('false');
      expect(djBtn).not.toHaveClass('top-bar__dj--active');
    });
  });

  describe('Recorder IPC toggle', () => {
    it('invokes start_recording and stop_recording via Tauri IPC and updates REC state', async () => {
      const idleStatus = {
        state: 'idle',
        is_recording: false,
        width: 1920,
        height: 1080,
        started_ms: null,
        fragments_written: 0,
        bytes_written: 0,
        replay_gop_count: 0,
        replay_packet_count: 0,
        replay_retained_bytes: 0,
        replay_retained_secs: 0,
        replay_max_duration_secs: 60,
      };
      const recordingStatus = {
        ...idleStatus,
        state: 'recording',
        is_recording: true,
        started_ms: 1_700_000_000_000,
        bytes_written: 1024,
      };

      invokeMock.mockImplementation((cmd: string) => {
        if (cmd === 'get_recorder_status') return Promise.resolve(idleStatus);
        if (cmd === 'start_recording') return Promise.resolve(recordingStatus);
        if (cmd === 'stop_recording') return Promise.resolve(idleStatus);
        return Promise.reject(new Error(`unexpected cmd: ${cmd}`));
      });

      routeStore.set('perform');
      const { container, getByTestId } = render(TopBar);
      await Promise.resolve();

      const recBtn = getByTestId('top-bar-record');
      const recPill = container.querySelector('.top-bar__rec');
      expect(recPill).not.toBeNull();
      expect(recPill?.getAttribute('data-state')).toBe('off');
      expect(recBtn.getAttribute('aria-pressed')).toBe('false');

      await fireEvent.click(recBtn);
      await Promise.resolve();
      await Promise.resolve();

      expect(invokeMock).toHaveBeenCalledWith('start_recording');
      expect(recPill?.getAttribute('data-state')).toBe('active');
      expect(recBtn.getAttribute('aria-pressed')).toBe('true');
      expect(recBtn).toHaveClass('top-bar__icon-btn--recording');

      if (!recPill) return;
      await fireEvent.click(recPill);
      await Promise.resolve();
      await Promise.resolve();

      expect(invokeMock).toHaveBeenCalledWith('stop_recording');
      expect(recPill.getAttribute('data-state')).toBe('off');
      expect(recBtn.getAttribute('aria-pressed')).toBe('false');
      expect(recBtn).not.toHaveClass('top-bar__icon-btn--recording');
    });
  });
});
