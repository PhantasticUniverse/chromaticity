// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W61 Settings view tests — landmark coverage for the largest non-
// Author/MIDI routed view (1925 LoC). Settings carries six bundle-
// faithful sub-sections per the W21/W38-W42 cohort: Audio Device
// (with W39 Channel Routing + Input Gain + Monitoring meter), Graphics
// / Output (with W41 Color Space / Multi-Display / Render Scale /
// Screen Fitting / Visualizer Window / FPS Slider rows), Live Sources
// (W42 Spout/Syphon + NDI + Constant Output Resolution + Sender Name),
// Storage (Media + Scene folders + Backups + Cache), Tempo & Sync (W22
// + W40 Ableton Link + MIDI Clock + Quantize), and General + Hotkeys +
// About + a right-rail Diagnostics panel. Tests scope to the article
// landmarks via `within(...)` rather than enumerate every form row, per
// the W61 brief's "landmark coverage rather than per-element exhaustive
// assertions" guidance.
//
// Mock conventions:
//   * `pickFolder` calls `invoke('pick_folder', ...)` from
//     `@tauri-apps/api/core`. Per
//     `feedback_vitest_invoke_mock_hoisted.md`, the mock fn lives in
//     `vi.hoisted` so the `vi.mock` factory can reference it without
//     tripping the TDZ that bites a hoist-ahead module initialisation.
//   * `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`
//     — Vitest 4 + jsdom + @testing-library/svelte does NOT auto-clean
//     between renders, and multi-render tests in this file will collide
//     on global queries without it.
//   * ThemeSwitcher writes to `document.documentElement.dataset.theme`
//     via a `$effect`; a jsdom `documentElement` survives across tests,
//     so `afterEach` clears it BEFORE `cleanup()` to keep the theme test
//     deterministic.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import Settings from './Settings.svelte';
import {
  AUDIO_INPUT_DEVICES,
  AUDIO_METER_BARS,
  AUDIO_METER_LEVEL,
  AUDIO_OUTPUT_DEVICES,
  AUDIO_SAMPLE_RATES,
  COLOR_SPACE_OPTIONS,
  DEFAULT_MEDIA_FOLDERS,
  DEFAULT_SCENE_FOLDERS,
  FPS_TARGETS,
  QUANTIZE_TRANSITION_OPTIONS,
  RENDER_SCALE_DEFAULT,
  RESOLUTION_PRESETS,
  SENDER_NAME_DEFAULT,
} from '../data/settings-fixtures';
import type { AudioDevicePayload } from '../ipc/bindings/AudioDevicePayload';
import { audioDevicesStore } from '../stores/audioDevices.svelte';
import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
import { settingsStore } from '../stores/settings.svelte';

function getSection(container: HTMLElement, heading: string): HTMLElement {
  const headings = Array.from(
    container.querySelectorAll<HTMLHeadingElement>('article.settings__section > h2'),
  );
  const match = headings.find((h) => h.textContent?.trim() === heading);
  expect(match, `expected an <h2> matching "${heading}"`).toBeDefined();
  const article = match?.closest('article.settings__section') as HTMLElement | null;
  expect(article, `expected an enclosing article for "${heading}"`).not.toBeNull();
  return article as HTMLElement;
}

describe('Settings view', () => {
  afterEach(() => {
    invokeMock.mockReset();
    // ThemeSwitcher writes `document.documentElement.dataset.theme` via
    // a `$effect`; cleanup() unmounts but leaves the attribute behind.
    // Reset so subsequent tests start from the bundle default ("pulse"
    // clears the attribute).
    delete document.documentElement.dataset.theme;
    // Wave 2.1 Lane C.3 — the settingsStore is a module-level singleton
    // shared across the test suite. Reset its load-idempotency latch +
    // in-memory snapshot so each test mounts against the bundle-faithful
    // defaults, just like the previous local-$state-only view.
    settingsStore.reset();
    // Wave 2.6 AUD.B — the audioSnapshotStore is also a module-level
    // singleton; clear any test-injected snapshot so the next render
    // starts in the no-snapshot (synthetic fallback) state.
    audioSnapshotStore.unsubscribe();
    audioSnapshotStore.snapshot = null;
    // Wave 2.6 Lane AUD.A — `audioDevicesStore` is a module-level
    // singleton shared across the suite. Reset between tests so each
    // case mounts against an empty live list (falls back to the
    // fixture) unless the test explicitly seeds devices.
    audioDevicesStore.reset();
    cleanup();
  });

  it('renders the heading and intro copy', () => {
    const { getByRole, getByText } = render(Settings);
    expect(getByRole('heading', { level: 1, name: /settings/i })).toBeInTheDocument();
    expect(getByText(/Operator preferences, devices, output, storage/)).toBeInTheDocument();
  });

  it('renders all eight settings sections in document order', () => {
    const { container } = render(Settings);
    const headings = Array.from(
      container.querySelectorAll<HTMLHeadingElement>('article.settings__section > h2'),
    ).map((h) => h.textContent?.trim());
    // Six "operator" sub-sections plus Hotkeys, About, and the right-
    // rail Diagnostics article. Order is bundle-faithful with operator
    // infra ABOVE the W14-banked General row pair.
    expect(headings).toEqual([
      'Audio Device',
      'Graphics / Output',
      'Live Sources',
      'Storage',
      'Tempo & Sync',
      'General',
      'Hotkeys',
      'About',
      'Diagnostics',
    ]);
  });

  it('Audio Device section binds the input-device select to the first fixture and round-trips a change', async () => {
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const input = within(audio).getByLabelText('Input Device') as HTMLSelectElement;
    expect(input).toBeInTheDocument();
    expect(input.value).toBe(AUDIO_INPUT_DEVICES[0]?.id ?? 'default');
    // Round-trip: choose a non-default fixture entry and confirm the
    // bound state followed.
    const next = AUDIO_INPUT_DEVICES[2];
    expect(next).toBeDefined();
    if (!next) return;
    await fireEvent.change(input, { target: { value: next.id } });
    expect(input.value).toBe(next.id);
    // Sample rate select renders all four fixture options.
    const sampleRate = within(audio).getByLabelText('Sample Rate') as HTMLSelectElement;
    expect(sampleRate.querySelectorAll('option').length).toBe(AUDIO_SAMPLE_RATES.length);
  });

  it('Input Device dropdown shows enumerated devices when audioDevicesStore.devices populated', async () => {
    // Wave 2.6 Lane AUD.A — when the engine `list_audio_devices`
    // command returns a non-empty list, the dropdown options come
    // from `audioDevicesStore.devices` (mapped to `{id, label: name}`)
    // rather than the W21 PR1 `AUDIO_INPUT_DEVICES` fixture. Seed the
    // store BEFORE rendering so the derived options reflect the live
    // list on first render.
    const live: AudioDevicePayload[] = [
      {
        id: 'input-built-in-microphone',
        name: 'Built-in Microphone',
        kind: 'Input',
      },
      {
        id: 'input-blackhole-16ch',
        name: 'BlackHole 16ch',
        kind: 'Input',
      },
    ];
    audioDevicesStore.devices = live;
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const input = within(audio).getByLabelText('Input Device') as HTMLSelectElement;
    const options = Array.from(input.querySelectorAll('option'));
    // Live list replaces the fixture entirely; the option count
    // matches the seeded device count, not the fixture length.
    expect(options.length).toBe(live.length);
    expect(options.map((o) => o.value)).toEqual(live.map((d) => d.id));
    expect(options.map((o) => o.textContent?.trim())).toEqual(live.map((d) => d.name));
  });

  it('Input Device dropdown falls back to the fixture when audioDevicesStore is empty', () => {
    // The "no Tauri context" / "no audio subsystem" / "no devices
    // connected" branches all collapse to `audioDevicesStore.devices`
    // being empty; the dropdown then renders the W21 PR1 fixture so
    // the operator-facing surface always has something to pick.
    audioDevicesStore.reset();
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const input = within(audio).getByLabelText('Input Device') as HTMLSelectElement;
    const options = Array.from(input.querySelectorAll('option'));
    expect(options.length).toBe(AUDIO_INPUT_DEVICES.length);
    expect(options.map((o) => o.value)).toEqual(AUDIO_INPUT_DEVICES.map((d) => d.id));
  });

  it('Output Device dropdown shows enumerated output devices when audioDevicesStore.outputDevices populated', () => {
    // Wave 2.7 Lane SET.C — Output dropdown counterpart to the
    // AUD.A Input dropdown enumeration pin. When
    // `audioDevicesStore.outputDevices` carries a non-empty live list
    // (engine `list_audio_output_devices` returned host output
    // devices), the dropdown options come from the live list rather
    // than the W21 PR1 `AUDIO_OUTPUT_DEVICES` fixture. Seed BEFORE
    // rendering so the derived options reflect the live list on first
    // render.
    const live: AudioDevicePayload[] = [
      {
        id: 'output-built-in-output',
        name: 'Built-in Output',
        kind: 'Output',
      },
      {
        id: 'output-external-interface',
        name: 'External Audio Interface',
        kind: 'Output',
      },
    ];
    audioDevicesStore.outputDevices = live;
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const output = within(audio).getByLabelText('Output Device') as HTMLSelectElement;
    const options = Array.from(output.querySelectorAll('option'));
    expect(options.length).toBe(live.length);
    expect(options.map((o) => o.value)).toEqual(live.map((d) => d.id));
    expect(options.map((o) => o.textContent?.trim())).toEqual(live.map((d) => d.name));
  });

  it('Output Device dropdown falls back to the fixture when audioDevicesStore.outputDevices is empty', () => {
    // Mirror of the Input fallback pin: the no-Tauri / no-audio /
    // no-devices-connected branches all collapse to an empty
    // `outputDevices` vec; the dropdown then renders the W21 PR1
    // `AUDIO_OUTPUT_DEVICES` fixture.
    audioDevicesStore.reset();
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const output = within(audio).getByLabelText('Output Device') as HTMLSelectElement;
    const options = Array.from(output.querySelectorAll('option'));
    expect(options.length).toBe(AUDIO_OUTPUT_DEVICES.length);
    expect(options.map((o) => o.value)).toEqual(AUDIO_OUTPUT_DEVICES.map((d) => d.id));
  });

  it('Audio Device input-gain slider updates the dB readout aria-valuetext', async () => {
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const slider = within(audio).getByLabelText('Input Gain') as HTMLInputElement;
    expect(slider).toBeInTheDocument();
    expect(slider.type).toBe('range');
    // Bundle default is `AUDIO_INPUT_GAIN_DEFAULT = 0.59` → +3.2 dB
    // under the symmetric ±18 dB mapping (the bundle wireframe's
    // canonical readout). The readout span lives in
    // `.settings__bpm-readout--gain`; whitespace around the
    // `{inputGainDbLabel}` literal breaks plain `getByText`, so the
    // assertion follows `aria-valuetext` (which the slider mirrors)
    // and confirms the readout span text content via `.textContent`.
    expect(slider.getAttribute('aria-valuetext')).toBe('+3.2 dB');
    const readout = audio.querySelector('.settings__bpm-readout--gain') as HTMLElement | null;
    expect(readout).not.toBeNull();
    expect(readout?.textContent?.trim()).toBe('+3.2 dB');
    await fireEvent.input(slider, { target: { value: '1' } });
    // (1 - 0.5) * 36 dB = +18.0 dB at the +max position.
    expect(slider.getAttribute('aria-valuetext')).toBe('+18.0 dB');
    expect(readout?.textContent?.trim()).toBe('+18.0 dB');
    // Negative excursion.
    await fireEvent.input(slider, { target: { value: '0' } });
    expect(slider.getAttribute('aria-valuetext')).toBe('-18.0 dB');
    expect(readout?.textContent?.trim()).toBe('-18.0 dB');
  });

  it('Audio Device renders the Monitoring meter widget with its aria label', () => {
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    expect(within(audio).getByLabelText('Audio input monitor')).toBeInTheDocument();
  });

  it('Audio Device monitoring meter reflects audioSnapshotStore.snapshot.rms', async () => {
    // Wave 2.6 AUD.B dual-mode posture: with no live snapshot, the
    // Meter falls back to the bundle-faithful fixture (`rms = 0.68`,
    // `peak = 0.82`). Injecting a snapshot directly on the singleton
    // store flips the derived `meterLevel` / `meterPeak`, and the
    // Meter widget's `level * bars` segment classification reflects
    // the change. Asserts at the segment-count level rather than
    // string-content because Meter exposes its level through the
    // LED-segment `.on` class density, not text content.
    const { container } = render(Settings);
    const audio = getSection(container, 'Audio Device');
    const meter = within(audio).getByLabelText('Audio input monitor');
    // Baseline: fixture-driven (`AUDIO_METER_LEVEL = 0.68` → floor(0.68 * 28) = 19 on-segments).
    const fixtureOnCount = Math.floor(AUDIO_METER_LEVEL * AUDIO_METER_BARS);
    const baselineOn = meter.querySelectorAll('.meter-bar.on, .meter-bar.clip').length;
    expect(baselineOn).toBe(fixtureOnCount);
    // Inject a live snapshot with a distinct rms value and re-assert.
    audioSnapshotStore.snapshot = {
      bpm: 120,
      rms: 0.25,
      peak: 0.45,
      fft_bands: [0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1],
    };
    // Allow Svelte to flush the $derived re-evaluation.
    await Promise.resolve();
    const liveOn = meter.querySelectorAll('.meter-bar.on, .meter-bar.clip').length;
    expect(liveOn).toBe(Math.floor(0.25 * AUDIO_METER_BARS));
    expect(liveOn).not.toBe(fixtureOnCount);
  });

  it('Graphics / Output renders all expected rows including the W41 expansion', () => {
    const { container } = render(Settings);
    const graphics = getSection(container, 'Graphics / Output');
    // W21 PR1 baseline rows.
    expect(within(graphics).getByLabelText('Render Resolution')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('Custom resolution')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('Target FPS')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('V-Sync')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('HDR Target')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('Display')).toBeInTheDocument();
    // W41 expansion rows.
    expect(within(graphics).getByLabelText('Color space')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('Multi-display mode')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('Render Scale')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('Screen fitting')).toBeInTheDocument();
    expect(within(graphics).getByLabelText('FPS slider')).toBeInTheDocument();
    // Resolution preset enumerates all bundle fixture entries.
    const preset = within(graphics).getByLabelText('Render Resolution') as HTMLSelectElement;
    expect(preset.querySelectorAll('option').length).toBe(RESOLUTION_PRESETS.length);
    // Visualizer Window action buttons are intentionally stub-disabled
    // pending Phase-3+/4a window IPC.
    const showResizable = within(graphics).getByRole('button', { name: 'Show resizable window' });
    const toggleFullscreen = within(graphics).getByRole('button', { name: 'Toggle fullscreen' });
    expect(showResizable).toBeDisabled();
    expect(toggleFullscreen).toBeDisabled();
  });

  it('Graphics / Output Custom Resolution toggle enables the W/H number inputs', async () => {
    const { container } = render(Settings);
    const graphics = getSection(container, 'Graphics / Output');
    const customToggle = within(graphics).getByLabelText('Custom resolution') as HTMLButtonElement;
    expect(customToggle).toHaveAttribute('aria-pressed', 'false');
    const widthInput = container.querySelector<HTMLInputElement>('#custom-width');
    const heightInput = container.querySelector<HTMLInputElement>('#custom-height');
    expect(widthInput).not.toBeNull();
    expect(heightInput).not.toBeNull();
    if (!widthInput || !heightInput) return;
    expect(widthInput.disabled).toBe(true);
    expect(heightInput.disabled).toBe(true);
    // The Render Resolution preset is also disabled while the toggle is
    // off — flipping the toggle activates the inputs and disables the
    // preset, mirroring the bundle wireframe's mutually exclusive UX.
    await fireEvent.click(customToggle);
    expect(customToggle).toHaveAttribute('aria-pressed', 'true');
    expect(widthInput.disabled).toBe(false);
    expect(heightInput.disabled).toBe(false);
    const preset = within(graphics).getByLabelText('Render Resolution') as HTMLSelectElement;
    expect(preset.disabled).toBe(true);
  });

  it('Graphics / Output Color Space tab-segment round-trips selection via aria-selected', async () => {
    const { container } = render(Settings);
    const graphics = getSection(container, 'Graphics / Output');
    // The W41 TabSegment widget renders `<button role="tab">` inside a
    // `role="tablist"` container labeled "Color space". `aria-selected`
    // (not `aria-pressed`) tracks the active tab. Bundle default is
    // `srgb` → "sRGB" tab.
    const tablist = within(graphics).getByRole('tablist', { name: 'Color space' });
    const tabs = within(tablist).getAllByRole('tab');
    expect(tabs.length).toBe(COLOR_SPACE_OPTIONS.length);
    const initiallyActive = tabs.find((t) => t.getAttribute('aria-selected') === 'true');
    expect(initiallyActive?.textContent?.trim()).toBe('sRGB');
    // Click a different fixture entry — Display P3.
    const targetLabel = COLOR_SPACE_OPTIONS[2]?.label;
    expect(targetLabel).toBeDefined();
    if (!targetLabel) return;
    const target = tabs.find((t) => t.textContent?.trim() === targetLabel);
    expect(target).toBeDefined();
    if (!target) return;
    await fireEvent.click(target);
    expect(target).toHaveAttribute('aria-selected', 'true');
    expect(initiallyActive).toHaveAttribute('aria-selected', 'false');
  });

  it('Graphics / Output Render Scale slider is initialised to the fixture default', () => {
    const { container } = render(Settings);
    const graphics = getSection(container, 'Graphics / Output');
    const scale = within(graphics).getByLabelText('Render Scale') as HTMLInputElement;
    expect(scale.type).toBe('range');
    expect(parseFloat(scale.value)).toBeCloseTo(RENDER_SCALE_DEFAULT, 5);
    // Readout span renders alongside the slider.
    expect(within(graphics).getByText(`${RENDER_SCALE_DEFAULT.toFixed(2)}×`)).toBeInTheDocument();
  });

  it('Live Sources renders five toggles plus the Sender Name input with bundle defaults', () => {
    const { container } = render(Settings);
    const live = getSection(container, 'Live Sources');
    // All five toggles surface via aria-label.
    for (const label of [
      'Spout / Syphon Input',
      'Spout / Syphon Output',
      'NDI Input',
      'NDI Output',
      'Constant Output Resolution',
    ]) {
      const btn = within(live).getByLabelText(label) as HTMLButtonElement;
      expect(btn).toBeInTheDocument();
      expect(btn.tagName).toBe('BUTTON');
      expect(btn).toHaveAttribute('aria-pressed');
    }
    const sender = within(live).getByLabelText('Sender Name') as HTMLInputElement;
    expect(sender.value).toBe(SENDER_NAME_DEFAULT);
    expect(sender.maxLength).toBe(32);
  });

  it('Live Sources rows carry data-cap attributes for the W15+ entitlement infra', () => {
    const { container } = render(Settings);
    const live = getSection(container, 'Live Sources');
    const expectedCaps = new Set([
      'io.syphon.input',
      'io.syphon.output',
      'io.ndi.input',
      'io.ndi.output',
      'render.custom-resolution',
    ]);
    const observed = new Set(
      Array.from(live.querySelectorAll<HTMLElement>('[data-cap]')).map((el) =>
        el.getAttribute('data-cap'),
      ),
    );
    for (const cap of expectedCaps) {
      expect(observed.has(cap)).toBe(true);
    }
  });

  it('Live Sources NDI Input toggle round-trips its aria-pressed state', async () => {
    const { container } = render(Settings);
    const live = getSection(container, 'Live Sources');
    const ndiIn = within(live).getByLabelText('NDI Input') as HTMLButtonElement;
    expect(ndiIn).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(ndiIn);
    expect(ndiIn).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(ndiIn);
    expect(ndiIn).toHaveAttribute('aria-pressed', 'false');
  });

  it('Storage renders the bundle default media + scene folders and the backup path', () => {
    const { container } = render(Settings);
    const storage = getSection(container, 'Storage');
    for (const folder of DEFAULT_MEDIA_FOLDERS) {
      expect(within(storage).getByText(folder.path)).toBeInTheDocument();
    }
    for (const folder of DEFAULT_SCENE_FOLDERS) {
      expect(within(storage).getByText(folder.path)).toBeInTheDocument();
    }
    // Headings for each storage group.
    expect(within(storage).getByRole('heading', { name: 'Media folders' })).toBeInTheDocument();
    expect(
      within(storage).getByRole('heading', { name: 'Custom scene folders' }),
    ).toBeInTheDocument();
    expect(within(storage).getByRole('heading', { name: 'Backups' })).toBeInTheDocument();
    expect(within(storage).getByRole('heading', { name: 'Cache' })).toBeInTheDocument();
  });

  it('Storage Add-folder browse invokes pick_folder via the IPC mock and appends the row', async () => {
    // Wave 2.6 Lane AUD.A added a `list_audio_devices` invoke on
    // Settings mount; route it to an empty array so the once-mock
    // below only fires for `pick_folder`. Otherwise mount-time
    // audio enumeration consumes the `mockResolvedValueOnce` value.
    invokeMock.mockImplementation((command: string) => {
      if (command === 'list_audio_devices') return Promise.resolve([]);
      if (command === 'pick_folder') {
        return Promise.resolve('/Users/operator/Movies/Chromaticity/Loops');
      }
      return Promise.resolve(undefined);
    });
    const { container } = render(Settings);
    const storage = getSection(container, 'Storage');
    const addBtn = within(storage).getByRole('button', { name: 'Add media folder' });
    await fireEvent.click(addBtn);
    // Wait for the awaited pickFolder promise + state assignment to
    // flush. In practice the resolution lands by next microtask.
    await Promise.resolve();
    await Promise.resolve();
    expect(invokeMock).toHaveBeenCalledWith('pick_folder', { title: 'Pick a Media folder' });
    expect(
      within(storage).getByText('/Users/operator/Movies/Chromaticity/Loops'),
    ).toBeInTheDocument();
  });

  it('Storage remove-folder action filters the row from the media list', async () => {
    const { container } = render(Settings);
    const storage = getSection(container, 'Storage');
    const firstFolder = DEFAULT_MEDIA_FOLDERS[0];
    expect(firstFolder).toBeDefined();
    if (!firstFolder) return;
    const folderRow = within(storage)
      .getByText(firstFolder.path)
      .closest('.settings__folder-row') as HTMLElement | null;
    expect(folderRow).not.toBeNull();
    if (!folderRow) return;
    const remove = within(folderRow).getByRole('button', { name: 'Remove media folder' });
    await fireEvent.click(remove);
    expect(within(storage).queryByText(firstFolder.path)).toBeNull();
  });

  it('Tempo & Sync renders the BPM source priority list and the Quantize tab-segment', () => {
    const { container } = render(Settings);
    const tempo = getSection(container, 'Tempo & Sync');
    // Four-row BPM chain.
    const bpmRows = tempo.querySelectorAll('.settings__bpm-row');
    expect(bpmRows.length).toBe(4);
    // Quantize TabSegment renders `role="tab"` not `role="button"`.
    const quantizeTablist = within(tempo).getByRole('tablist', { name: 'Quantize transitions' });
    const quantizeTabs = within(quantizeTablist).getAllByRole('tab');
    expect(quantizeTabs.length).toBe(QUANTIZE_TRANSITION_OPTIONS.length);
    // Ableton Link toggle is on by default (peer pill matches fixture).
    const link = within(tempo).getByLabelText('Ableton Link') as HTMLButtonElement;
    expect(link).toHaveAttribute('aria-pressed', 'true');
  });

  it('Tempo & Sync tap-tempo button advances the readout once two taps are recorded', async () => {
    const { container } = render(Settings);
    const tempo = getSection(container, 'Tempo & Sync');
    const tapBtn = within(tempo).getByRole('button', { name: 'Tap tempo' });
    // Initially the readout shows the muted "— BPM" placeholder.
    expect(within(tempo).getByText(/—\s*BPM/)).toBeInTheDocument();
    // First tap doesn't compute (need >= 2 timestamps).
    await fireEvent.click(tapBtn);
    // Second tap turns the placeholder into a numeric readout. The
    // exact value depends on `performance.now()` deltas in jsdom; just
    // confirm the muted placeholder is gone and a "BPM" unit span
    // remains visible inside the readout.
    await fireEvent.click(tapBtn);
    const readout = tempo.querySelector('.settings__bpm-readout') as HTMLElement | null;
    expect(readout).not.toBeNull();
    if (!readout) return;
    expect(readout.textContent).toContain('BPM');
  });

  it('Tempo & Sync MIDI Clock Input device select is disabled until the toggle is on', async () => {
    const { container } = render(Settings);
    const tempo = getSection(container, 'Tempo & Sync');
    const inDevice = container.querySelector<HTMLSelectElement>('#midi-clock-in-device');
    expect(inDevice).not.toBeNull();
    if (!inDevice) return;
    expect(inDevice.disabled).toBe(true);
    // The string "MIDI Clock Input" is both a `<label for=...>` (for
    // the device select) and an `aria-label` on the pill toggle. Scope
    // to the toggle by role + name so the two labels don't collide.
    const inToggle = within(tempo).getByRole('button', {
      name: 'MIDI Clock Input',
    }) as HTMLButtonElement;
    await fireEvent.click(inToggle);
    expect(inToggle).toHaveAttribute('aria-pressed', 'true');
    expect(inDevice.disabled).toBe(false);
  });

  it('General section renders the ThemeSwitcher and writes a non-default theme to dataset', async () => {
    const { container } = render(Settings);
    const general = getSection(container, 'General');
    const themeSelect = within(general).getByRole('combobox') as HTMLSelectElement;
    expect(themeSelect).toBeInTheDocument();
    // Bundle default is "pulse" which clears the dataset attribute.
    await fireEvent.change(themeSelect, { target: { value: 'atelier' } });
    expect(themeSelect.value).toBe('atelier');
    // The $effect flushes synchronously after the bind:value update.
    expect(document.documentElement.dataset.theme).toBe('atelier');
  });

  it('General section locked entitlement rows are disabled checkboxes with data-cap markers', () => {
    const { container } = render(Settings);
    const general = getSection(container, 'General');
    const lockedRows = general.querySelectorAll('.settings__row--locked');
    expect(lockedRows.length).toBe(2);
    const caps = Array.from(lockedRows).map((el) => el.getAttribute('data-cap'));
    expect(caps).toContain('io.osc.input');
    expect(caps).toContain('io.camera');
    // Each row's checkbox is disabled.
    for (const row of Array.from(lockedRows)) {
      const cb = (row as HTMLElement).querySelector<HTMLInputElement>('input[type="checkbox"]');
      expect(cb).not.toBeNull();
      expect(cb?.disabled).toBe(true);
    }
  });

  it('Diagnostics rail renders the six fixture rows and the disabled support / log buttons', () => {
    const { container } = render(Settings);
    const diag = getSection(container, 'Diagnostics');
    // Six DT keys (GPU / FPS / Audio Latency / MIDI Devices / Syphon /
    // Spout / Ableton Link). Iterate landmark text to confirm coverage.
    expect(within(diag).getByText('GPU')).toBeInTheDocument();
    expect(within(diag).getByText('FPS / Frame')).toBeInTheDocument();
    expect(within(diag).getByText('Audio Latency')).toBeInTheDocument();
    expect(within(diag).getByText('MIDI Devices')).toBeInTheDocument();
    expect(within(diag).getByText('Syphon / Spout')).toBeInTheDocument();
    expect(within(diag).getByText('Ableton Link')).toBeInTheDocument();
    // The two action buttons are stub-disabled pending W15+ work.
    const supportBundle = within(diag).getByRole('button', { name: /Generate Support Bundle/ });
    const viewLogs = within(diag).getByRole('button', { name: 'View Logs' });
    expect(supportBundle).toBeDisabled();
    expect(viewLogs).toBeDisabled();
  });

  it('Hotkeys section renders accessible reference tables', () => {
    const { container } = render(Settings);
    const hotkeys = getSection(container, 'Hotkeys');
    // Each hotkey group renders a <table> with the canonical four-column
    // header (keys / action / osc / cap). Confirm at least one table
    // mounted with the expected columns.
    const tables = hotkeys.querySelectorAll('table.settings__hotkey-table');
    expect(tables.length).toBeGreaterThan(0);
    const firstTable = tables[0] as HTMLElement;
    const headers = Array.from(firstTable.querySelectorAll('thead th')).map((th) =>
      th.textContent?.trim(),
    );
    expect(headers).toEqual(['keys', 'action', 'osc', 'cap']);
  });

  it('Target FPS select enumerates all fixture entries', () => {
    const { container } = render(Settings);
    const graphics = getSection(container, 'Graphics / Output');
    const fps = within(graphics).getByLabelText('Target FPS') as HTMLSelectElement;
    expect(fps.querySelectorAll('option').length).toBe(FPS_TARGETS.length);
  });

  it('mount fires get_settings via the settingsStore IPC seam', async () => {
    // Wave 2.1 Lane C.3 settings-persistence wiring: the Settings
    // view's mount $effect calls `settingsStore.load()`, which
    // invokes `get_settings`. The store catches the rejection (the
    // jsdom harness has no Tauri runtime + the invokeMock above
    // returns undefined by default) and renders fall back to the
    // bundle-faithful defaults so existing assertions keep passing.
    invokeMock.mockResolvedValue(undefined);
    render(Settings);
    // The component's mount `$effect` calls
    // `settingsStore.load()` which dynamic-imports the IPC module
    // before invoking. Awaiting the store's load() promise directly
    // is the cleanest way to settle the entire chain — `load()` is
    // idempotent so calling it again from the test just returns
    // the in-flight promise from the mount $effect.
    await settingsStore.load();
    expect(invokeMock).toHaveBeenCalledWith('get_settings');
  });

  it('changing Target FPS dispatches a debounced set_settings IPC call', async () => {
    // Wave 2.1 Lane C.3 write-through: changing a persisted field
    // (Target FPS lives under graphics.fps_target) schedules a
    // debounced set_settings invocation. The store debounce is
    // ~250ms; fake timers fast-forward past it.
    vi.useFakeTimers();
    try {
      // Settings.svelte mounts and calls load() — let invokeMock
      // resolve with `undefined` for both get_settings and set_settings.
      invokeMock.mockResolvedValue(undefined);
      const { container } = render(Settings);
      await Promise.resolve();
      const graphics = getSection(container, 'Graphics / Output');
      const fps = within(graphics).getByLabelText('Target FPS') as HTMLSelectElement;
      await fireEvent.change(fps, { target: { value: '120' } });
      // Advance past the debounce window.
      await vi.advanceTimersByTimeAsync(300);
      const setCalls = invokeMock.mock.calls.filter((c) => c[0] === 'set_settings');
      expect(setCalls.length).toBeGreaterThanOrEqual(1);
      // The last set_settings carries the updated graphics.fps_target.
      const lastPayload = setCalls.at(-1)?.[1] as
        | { settings: { graphics: { fps_target: string } } }
        | undefined;
      expect(lastPayload?.settings.graphics.fps_target).toBe('120');
    } finally {
      vi.useRealTimers();
    }
  });
});
