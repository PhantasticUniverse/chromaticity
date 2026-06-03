// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W58 Audio Uniforms view tests — exercise the W17 PR3 + W25 PR1 routed-
// view surface (header pill + freeze/resume toggle, spectrum hero, frequency
// band tiles, time uniform rows, BPM uniform cards including scalar / bool /
// wave variants, "how to use" docs collapse, reduced-motion branch).
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.
// `vi.stubGlobal('matchMedia', ...)` per
// `feedback_vitest_matchmedia_stubglobal.md` — the view itself queries
// `prefers-reduced-motion`, AND the embedded `BpmRing` / `BeatDots`
// widgets do too, so the global stub must be in place before the very
// first render.
//
// Canvas warnings ("Not implemented: HTMLCanvasElement's getContext()")
// are expected and harmless in this jsdom + Vitest 4 stack — the view's
// hero / sparkline visuals are inline SVG, but the embedded BpmRing
// widget probes a canvas; the warnings are inert.
//
// The view holds local `$state` only (`streaming`, `docsOpen`, `liveT`,
// `livePeaks`, `audioReducedMotion`); per-test `audioSnapshotStore`
// resets to `null` in `afterEach` so cross-test pollution from the Wave
// 2.6 AUD.D live-band assertions stays contained.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import AudioUniforms from './AudioUniforms.svelte';
import {
  AUDIO_UNIFORMS_FIXTURE,
  HOW_TO_USE_BLOCKS,
  SPECTRUM_BAR_COUNT,
} from '../data/audio-uniform-fixtures';
import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
import { settingsStore } from '../stores/settings.svelte';

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

describe('AudioUniforms view', () => {
  beforeEach(() => {
    stubMatchMedia(false);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    // Reset the shared singletons between tests so the Snapshot-button
    // suite (which writes a live snapshot) cannot leak state into the
    // default-null assertions of sibling tests; settings + audio
    // subscription teardown per the Wave 2.6 AUD.C dual-mode posture.
    audioSnapshotStore.snapshot = null;
    audioSnapshotStore.unsubscribe();
    settingsStore.reset();
    cleanup();
  });

  it('renders the header title with the live mode chip and streaming pill', () => {
    const { container, getByText } = render(AudioUniforms);
    // Title splits into "Audio Uniforms" + a separator + the mode word.
    // The aside also exposes the phrase via an `aria-label` ("Audio uniforms
    // reference"), so match against the explicit `<h2>` element instead.
    const title = container.querySelector('.au__title');
    expect(title).not.toBeNull();
    expect(title?.textContent).toContain('Audio Uniforms');
    expect(getByText('Live')).toBeInTheDocument();
    // Default fixture has `streamingActive: true` → pill shows "streaming"
    // and the toggle button reads "Freeze".
    const pill = container.querySelector('.au__pill');
    expect(pill).not.toBeNull();
    expect(pill?.textContent).toContain('streaming');
    expect(pill).toHaveClass('au__pill--on');
    expect(getByText('Freeze')).toBeInTheDocument();
  });

  it('disables the Snapshot button when no live audio snapshot is present', () => {
    audioSnapshotStore.snapshot = null;
    const { getByRole } = render(AudioUniforms);
    const snap = getByRole('button', { name: 'Snapshot' });
    expect(snap).toBeDisabled();
    expect(snap).toHaveAttribute('aria-disabled', 'true');
    expect(snap.getAttribute('title')).toMatch(/waiting/i);
  });

  it('enables the Snapshot button when audioSnapshotStore holds a live snapshot', async () => {
    audioSnapshotStore.snapshot = {
      bpm: 120,
      bpm_availability: 'Ready',
      rms: 0.3,
      peak: 0.6,
      fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
      fft_availability: 'Ready',
      bass: 1,
      mid: 1,
      treb: 1,
      bass_att: 1,
      mid_att: 1,
      treb_att: 1,
      degraded: false,
    };
    const { getByRole } = render(AudioUniforms);
    // Microtask flush for the $derived recomputation.
    await Promise.resolve();
    const snap = getByRole('button', { name: 'Snapshot' });
    expect(snap).not.toBeDisabled();
    expect(snap).toHaveAttribute('aria-disabled', 'false');
    expect(snap.getAttribute('title')).toMatch(/capture/i);
  });

  it('captures the live snapshot as a JSON blob download when clicked', async () => {
    audioSnapshotStore.snapshot = {
      bpm: 132.5,
      bpm_availability: 'Ready',
      rms: 0.42,
      peak: 0.81,
      fft_bands: [0.11, 0.22, 0.33, 0.44, 0.55, 0.66, 0.77, 0.88],
      fft_availability: 'Ready',
      bass: 1,
      mid: 1,
      treb: 1,
      bass_att: 1,
      mid_att: 1,
      treb_att: 1,
      degraded: false,
    };
    // Capture the Blob handed to URL.createObjectURL so we can read its
    // text contents back out and verify the JSON payload shape.
    let capturedBlob: Blob | null = null;
    const createObjectUrl = vi.fn((blob: Blob): string => {
      capturedBlob = blob;
      return 'blob:mock-snapshot-url';
    });
    const revokeObjectUrl = vi.fn();
    vi.stubGlobal('URL', {
      ...URL,
      createObjectURL: createObjectUrl,
      revokeObjectURL: revokeObjectUrl,
    });
    // Intercept the temp anchor's `.click()` so the test does not actually
    // navigate / trigger a real download in jsdom.
    const anchorClicks: HTMLAnchorElement[] = [];
    const originalCreateElement = document.createElement.bind(document);
    const createElementSpy = vi
      .spyOn(document, 'createElement')
      .mockImplementation((tagName: string) => {
        const el = originalCreateElement(tagName);
        if (tagName.toLowerCase() === 'a') {
          const anchor = el as HTMLAnchorElement;
          anchor.click = (): void => {
            anchorClicks.push(anchor);
          };
        }
        return el;
      });
    const { getByRole } = render(AudioUniforms);
    await Promise.resolve();
    const snap = getByRole('button', { name: 'Snapshot' });
    await fireEvent.click(snap);
    expect(createObjectUrl).toHaveBeenCalledTimes(1);
    expect(revokeObjectUrl).toHaveBeenCalledTimes(1);
    expect(revokeObjectUrl).toHaveBeenCalledWith('blob:mock-snapshot-url');
    expect(anchorClicks.length).toBe(1);
    const anchor = anchorClicks[0];
    expect(anchor?.getAttribute('href')).toBe('blob:mock-snapshot-url');
    expect(anchor?.getAttribute('download')).toMatch(/^audio-snapshot-.*\.json$/);
    expect(capturedBlob).not.toBeNull();
    if (capturedBlob !== null) {
      expect((capturedBlob as Blob).type).toBe('application/json');
      const text = await (capturedBlob as Blob).text();
      const parsed = JSON.parse(text) as {
        bpm: number;
        bpm_availability: string;
        rms: number;
        peak: number;
        fft_bands: number[];
        fft_availability: string;
        degraded: boolean;
        captured_at: string;
      };
      expect(parsed.bpm).toBe(132.5);
      expect(parsed.bpm_availability).toBe('Ready');
      expect(parsed.rms).toBeCloseTo(0.42);
      expect(parsed.peak).toBeCloseTo(0.81);
      expect(parsed.fft_bands).toEqual([0.11, 0.22, 0.33, 0.44, 0.55, 0.66, 0.77, 0.88]);
      expect(parsed.fft_availability).toBe('Ready');
      expect(parsed.degraded).toBe(false);
      // captured_at is an ISO-8601 timestamp emitted at click time.
      expect(parsed.captured_at).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
    }
    createElementSpy.mockRestore();
  });

  it('is a no-op when the Snapshot button is clicked with no live snapshot', async () => {
    audioSnapshotStore.snapshot = null;
    const createObjectUrl = vi.fn((): string => 'blob:should-not-be-called');
    vi.stubGlobal('URL', { ...URL, createObjectURL: createObjectUrl });
    const { getByRole } = render(AudioUniforms);
    const snap = getByRole('button', { name: 'Snapshot' });
    // The button is ARIA-disabled; click handlers can still fire through
    // fireEvent.click in tests but the guard short-circuits before any
    // Blob / URL machinery runs.
    await fireEvent.click(snap);
    expect(createObjectUrl).not.toHaveBeenCalled();
  });

  it('renders the spectrum hero with peak / rms / source stats and 64 SVG bars', () => {
    const { container, getByText } = render(AudioUniforms);
    // The hero is labelled "Spectrum" via `aria-label` on the section.
    const hero = container.querySelector('.au__hero');
    expect(hero).not.toBeNull();
    if (hero === null) return;
    const inHero = within(hero as HTMLElement);
    // Peak / rms come from the fixture, formatted to one decimal place.
    expect(inHero.getByText(`${AUDIO_UNIFORMS_FIXTURE.peakDb.toFixed(1)} dB`)).toBeInTheDocument();
    expect(inHero.getByText(`${AUDIO_UNIFORMS_FIXTURE.rmsDb.toFixed(1)} dB`)).toBeInTheDocument();
    expect(getByText(AUDIO_UNIFORMS_FIXTURE.source)).toBeInTheDocument();
    // 64 spectrum bars render as inline SVG `<rect class="au__spectrum-bar">`.
    const bars = container.querySelectorAll('rect.au__spectrum-bar');
    expect(bars.length).toBe(SPECTRUM_BAR_COUNT);
    expect(bars.length).toBe(64);
  });

  it('switches the hero aria-label to "paused" after toggling Freeze', async () => {
    const { container, getByRole } = render(AudioUniforms);
    const heroCanvas = container.querySelector('.au__hero-canvas');
    expect(heroCanvas?.getAttribute('aria-label')).toMatch(/live synthetic/);
    const toggle = getByRole('button', { name: 'Freeze' });
    await fireEvent.click(toggle);
    // After freeze, the canvas label flips and the button now reads "Resume".
    expect(heroCanvas?.getAttribute('aria-label')).toMatch(/paused/);
    expect(getByRole('button', { name: 'Resume' })).toBeInTheDocument();
    // Pill updates too.
    const pill = container.querySelector('.au__pill');
    expect(pill?.textContent).toContain('paused');
    expect(pill).not.toHaveClass('au__pill--on');
  });

  it('aria-pressed inverts on toggle (true while paused, false while streaming)', async () => {
    const { getByRole } = render(AudioUniforms);
    // The view inverts `aria-pressed` against `streaming` (so the button's
    // pressed state means "Resume is the resting label"). Streaming default
    // → aria-pressed="false".
    const initial = getByRole('button', { name: 'Freeze' });
    expect(initial).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(initial);
    const flipped = getByRole('button', { name: 'Resume' });
    expect(flipped).toHaveAttribute('aria-pressed', 'true');
  });

  it('renders all five frequency bands with their three bars each', () => {
    const { container, getByText } = render(AudioUniforms);
    const bands = container.querySelectorAll('.au__band');
    expect(bands.length).toBe(AUDIO_UNIFORMS_FIXTURE.bands.length);
    expect(bands.length).toBe(5);
    for (const band of AUDIO_UNIFORMS_FIXTURE.bands) {
      expect(getByText(band.label)).toBeInTheDocument();
      expect(getByText(band.subLabel)).toBeInTheDocument();
    }
    // Each band has exactly three bars (level / hit / presence).
    for (const band of bands) {
      expect(band.querySelectorAll('.au__bar').length).toBe(3);
    }
  });

  it('hit-bar marker is filled (●) when value > 0.05 and hollow (○) otherwise', () => {
    const { container } = render(AudioUniforms);
    // Bass: hit value 0.04 → hollow. Mid-High: hit value 0.62 → filled.
    // Scope each assertion to the band's tile so the `●` / `○` glyphs
    // can't drift between bands.
    const bands = container.querySelectorAll('.au__band');
    expect(bands.length).toBe(5);
    const bassBars = bands[0]?.querySelectorAll('.au__bar-value');
    const midhiBars = bands[2]?.querySelectorAll('.au__bar-value');
    expect(bassBars).toBeDefined();
    expect(midhiBars).toBeDefined();
    // Index 1 in each band's bar list is the `hit` variant.
    expect(bassBars?.[1]?.textContent?.trim()).toBe('○');
    expect(midhiBars?.[1]?.textContent?.trim()).toBe('●');
  });

  it('renders one row per time uniform with the formatted accumulator value', () => {
    const { container, getByText } = render(AudioUniforms);
    const rows = container.querySelectorAll('.au__time-row');
    expect(rows.length).toBe(AUDIO_UNIFORMS_FIXTURE.timeUniforms.length);
    expect(rows.length).toBe(7);
    for (const uniform of AUDIO_UNIFORMS_FIXTURE.timeUniforms) {
      // Each row mounts the uniform key in mono.
      expect(getByText(uniform.key)).toBeInTheDocument();
    }
    // syn_Time accumulator value 7.91 — formatted with two decimals.
    expect(getByText('7.91')).toBeInTheDocument();
  });

  it('renders BPM uniform cards across scalar / bool / wave kinds', () => {
    const { container, getByText } = render(AudioUniforms);
    const cards = container.querySelectorAll('.au__bpm-card');
    expect(cards.length).toBe(AUDIO_UNIFORMS_FIXTURE.bpmUniforms.length);
    expect(cards.length).toBe(11);
    // Scope card-name lookups to the BPM grid — `syn_BPMSin` also appears
    // as a token in the docs side panel, which would yield multiple matches.
    const grid = container.querySelector('.au__bpm-grid') as HTMLElement | null;
    expect(grid).not.toBeNull();
    if (grid === null) return;
    const inGrid = within(grid);
    // Scalar card: syn_BPM displayValue 128.0 + label "beats/min".
    expect(inGrid.getByText('syn_BPM')).toBeInTheDocument();
    expect(inGrid.getByText('128.0')).toBeInTheDocument();
    expect(getByText('beats/min')).toBeInTheDocument();
    // Bool card: syn_OnBeat label "1 on downbeat" + LED element.
    expect(inGrid.getByText('syn_OnBeat')).toBeInTheDocument();
    expect(getByText('1 on downbeat')).toBeInTheDocument();
    // Wave card: syn_BPMSin label + sparkline path. The sparkline path is
    // an inline SVG `<path class="au__spark-path">`.
    expect(inGrid.getByText('syn_BPMSin')).toBeInTheDocument();
    const sparkPaths = container.querySelectorAll('path.au__spark-path');
    // One per non-scalar / non-bool card with a sparkline (8 entries).
    expect(sparkPaths.length).toBe(8);
  });

  it('bool LED reflects the `ledOn` fixture flag', () => {
    const { container, getByText } = render(AudioUniforms);
    // syn_OnBeat fixture: ledOn = false → no `--on` modifier.
    const onBeatCard = getByText('syn_OnBeat').closest('.au__bpm-card');
    expect(onBeatCard).not.toBeNull();
    if (onBeatCard === null) return;
    const led = onBeatCard.querySelector('.au__bpm-led');
    expect(led).not.toBeNull();
    expect(led).not.toHaveClass('au__bpm-led--on');
    // Sanity: there are exactly two bool cards (syn_OnBeat, syn_ToggleOnBeat).
    expect(container.querySelectorAll('.au__bpm-card--bool').length).toBe(2);
  });

  it('renders the docs side panel with all How-To-Use blocks expanded by default', () => {
    const { container, getByRole, getByText } = render(AudioUniforms);
    // Toggle starts expanded (docsOpen = true).
    const toggle = getByRole('button', { name: /how to use/i });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    // Each block with a title contributes a subhead.
    const titledBlocks = HOW_TO_USE_BLOCKS.filter((b) => b.title !== undefined);
    for (const block of titledBlocks) {
      if (block.title === undefined) continue;
      expect(getByText(block.title)).toBeInTheDocument();
    }
    // Deep-link footer renders in the open body.
    const docsBody = container.querySelector('.au__docs-body');
    expect(docsBody).not.toBeNull();
    expect(docsBody?.querySelector('a.au__docs-link')).not.toBeNull();
  });

  it('clicking the How-To-Use toggle collapses the body', async () => {
    const { container, getByRole } = render(AudioUniforms);
    const toggle = getByRole('button', { name: /how to use/i });
    expect(container.querySelector('.au__docs-body')).not.toBeNull();
    await fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(container.querySelector('.au__docs-body')).toBeNull();
    // Re-clicking re-expands.
    await fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(container.querySelector('.au__docs-body')).not.toBeNull();
  });

  it('renders the live-visualizer placeholder aside with three blob layers', () => {
    const { container, getByText } = render(AudioUniforms);
    const preview = container.querySelector('.au__preview');
    expect(preview).not.toBeNull();
    expect(preview?.getAttribute('role')).toBe('img');
    expect(preview?.getAttribute('aria-label')).toBe('Live visualizer preview');
    expect(preview?.querySelectorAll('.au__preview-blob').length).toBe(3);
    expect(getByText('live visualizer')).toBeInTheDocument();
  });

  it('renders no peak overlays when reduced-motion is active (frozen-frame branch)', () => {
    // The peak overlay rects only render under `!audioReducedMotion && streaming`.
    // With matchMedia stubbed to `matches: true`, the $effect sets
    // `audioReducedMotion = true` on mount and the `{#if}` guard skips peaks.
    stubMatchMedia(true);
    const { container } = render(AudioUniforms);
    expect(container.querySelectorAll('rect.au__spectrum-bar').length).toBe(SPECTRUM_BAR_COUNT);
    expect(container.querySelectorAll('rect.au__spectrum-peak').length).toBe(0);
  });

  it('renders no peak overlays after pausing the stream (streaming = false)', async () => {
    const { container, getByRole } = render(AudioUniforms);
    // Initial render: peaks may be empty until the first rAF tick (jsdom
    // does not advance animation frames automatically), but the gate is
    // `streaming === true`. Toggle to paused and assert the gate closes.
    await fireEvent.click(getByRole('button', { name: 'Freeze' }));
    expect(container.querySelectorAll('rect.au__spectrum-peak').length).toBe(0);
    // Spectrum bars (frozen fixture path) still render.
    expect(container.querySelectorAll('rect.au__spectrum-bar').length).toBe(SPECTRUM_BAR_COUNT);
  });

  // ---------------------------------------------------------------------
  // Wave 2.6 AUD.D — live band-tile wiring. Five fixture tiles now read
  // live values reduced from `audioSnapshotStore.snapshot.fft_bands` via
  // the 8 → 5 idiom documented in `AudioUniforms.svelte`:
  //   bass   ← avg(fft_bands[0], fft_bands[1])
  //   mid    ← avg(fft_bands[2], fft_bands[3])
  //   midhi  ← avg(fft_bands[4], fft_bands[5])
  //   high   ← avg(fft_bands[6], fft_bands[7])
  //   full   ← avg(fft_bands[0..7])
  // The store is mutated directly because the view subscribes to the
  // singleton (no `snapshot` prop on AudioUniforms — divergence from
  // the AudioReactivity widget surface). `afterEach` resets the
  // singleton's `snapshot` field to `null` so the synthetic-fallback
  // tests above keep their default fixture-driven posture.

  // Carefully chosen so the reduced values bracket the HIT_THRESHOLD
  // (0.5) without ties: bass / mid stay below, midhi / high fire, and
  // the broadband full lands exactly at 0.5 (not strictly greater, so
  // no hit). Per-bar `.toFixed(2)` matches the view's value rendering.
  const SAMPLE_SNAPSHOT: AudioUniformSnapshotPayload = {
    bpm: 120,
    bpm_availability: 'Ready',
    rms: 0.4,
    peak: 0.9,
    fft_bands: [0.1, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.9],
    fft_availability: 'Ready',
    bass: 1,
    mid: 1,
    treb: 1,
    bass_att: 1,
    mid_att: 1,
    treb_att: 1,
    degraded: false,
  };

  it('reduces 8 fft_bands into the 5 band tiles when audioSnapshotStore.snapshot is set', () => {
    audioSnapshotStore.snapshot = SAMPLE_SNAPSHOT;
    const { container } = render(AudioUniforms);
    const bands = container.querySelectorAll('.au__band');
    expect(bands.length).toBe(5);

    // Index 0 (bass) / 2 (midhi) / 3 (high) / 4 (full) — exercise the
    // four frequency-specific pair reductions plus the broadband mean.
    // Each band's bars are level / hit / presence in template order.
    const valueAt = (bandIdx: number, barIdx: number): string =>
      bands[bandIdx]?.querySelectorAll('.au__bar-value')[barIdx]?.textContent?.trim() ?? '';

    // Bass: avg(0.1, 0.2) = 0.15 — below HIT_THRESHOLD, hit = 0 → ○.
    expect(valueAt(0, 0)).toBe('0.15'); // level
    expect(valueAt(0, 1)).toBe('○'); // hit indicator (hollow)
    expect(valueAt(0, 2)).toBe('0.15'); // presence mirrors level

    // Mid-High: avg(0.6, 0.7) = 0.65 — above threshold, hit = 0.30 → ●.
    expect(valueAt(2, 0)).toBe('0.65');
    expect(valueAt(2, 1)).toBe('●');
    expect(valueAt(2, 2)).toBe('0.65');

    // High: avg(0.8, 0.9) = 0.85 — well above threshold, hit = 0.70 → ●.
    expect(valueAt(3, 0)).toBe('0.85');
    expect(valueAt(3, 1)).toBe('●');
    expect(valueAt(3, 2)).toBe('0.85');

    // Full: avg of all eight bins = 0.50 — exactly at threshold, not
    // strictly greater, hit = 0 → ○.
    expect(valueAt(4, 0)).toBe('0.50');
    expect(valueAt(4, 1)).toBe('○');
    expect(valueAt(4, 2)).toBe('0.50');
  });

  it('falls back to fixture band values when audioSnapshotStore.snapshot is null', () => {
    audioSnapshotStore.snapshot = null;
    const { container } = render(AudioUniforms);
    const bands = container.querySelectorAll('.au__band');
    expect(bands.length).toBe(5);

    // Bass fixture: level 0.54, hit 0.04 (→ ○ since ≤ 0.05), presence 0.61.
    const bassValues = bands[0]?.querySelectorAll('.au__bar-value');
    expect(bassValues?.[0]?.textContent?.trim()).toBe('0.54');
    expect(bassValues?.[1]?.textContent?.trim()).toBe('○');
    expect(bassValues?.[2]?.textContent?.trim()).toBe('0.61');

    // Mid-High fixture: hit value 0.62 → ● (the pre-existing hit-marker
    // test pins this; restating here against the live-fallback path).
    const midhiValues = bands[2]?.querySelectorAll('.au__bar-value');
    expect(midhiValues?.[1]?.textContent?.trim()).toBe('●');
  });

  it('preserves the fixture band labels / sub-labels under the live reduction', () => {
    audioSnapshotStore.snapshot = SAMPLE_SNAPSHOT;
    const { getByText } = render(AudioUniforms);
    // Labels / sub-labels come from the fixture metadata — only the
    // per-bar `value` switches to live data, so the visual layout
    // (Bass / Mid / Mid-High / High / Full) stays stable.
    for (const band of AUDIO_UNIFORMS_FIXTURE.bands) {
      expect(getByText(band.label)).toBeInTheDocument();
      expect(getByText(band.subLabel)).toBeInTheDocument();
    }
  });

  it('peak/RMS dB readout reflects audioSnapshotStore.snapshot values', () => {
    // Wave 2.6 AUD.C — when the audioSnapshotStore carries a live
    // snapshot, the hero stats strip drops the frozen
    // `AUDIO_UNIFORMS_FIXTURE.peakDb` / `.rmsDb` text and renders the
    // 20·log10 conversion of `snapshot.peak` / `.rms` instead. peak
    // 0.8 -> 20·log10(0.8) ≈ -1.9 dB; rms 0.25 -> ≈ -12.0 dB.
    const payload: AudioUniformSnapshotPayload = {
      bpm: 128,
      bpm_availability: 'Ready',
      rms: 0.25,
      peak: 0.8,
      fft_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
      fft_availability: 'Ready',
      bass: 1,
      mid: 1,
      treb: 1,
      bass_att: 1,
      mid_att: 1,
      treb_att: 1,
      degraded: false,
    };
    audioSnapshotStore.snapshot = payload;
    const { container } = render(AudioUniforms);
    const stats = container.querySelector('.au__hero-stats');
    expect(stats).not.toBeNull();
    if (stats === null) return;
    const expectedPeak = (20 * Math.log10(0.8)).toFixed(1);
    const expectedRms = (20 * Math.log10(0.25)).toFixed(1);
    expect(stats.textContent).toContain(`${expectedPeak} dB`);
    expect(stats.textContent).toContain(`${expectedRms} dB`);
    // Frozen fixture values should NOT appear once the live snapshot
    // is present (peak -1.7 dB / rms -10.5 dB).
    expect(stats.textContent).not.toContain(`${AUDIO_UNIFORMS_FIXTURE.peakDb.toFixed(1)} dB`);
    expect(stats.textContent).not.toContain(`${AUDIO_UNIFORMS_FIXTURE.rmsDb.toFixed(1)} dB`);
  });

  it('peak/RMS dB readout clamps to -60 dB floor when snapshot reports digital silence', () => {
    // Wave 2.6 AUD.C — guard against -Infinity on `peak === 0` /
    // `rms === 0` (e.g. the engine's `SilentSource` fallback on hosts
    // without a default cpal input device). The conversion clamps the
    // linear amplitude to 1e-3 before the log, yielding -60.0 dB.
    audioSnapshotStore.snapshot = {
      bpm: 0,
      bpm_availability: 'Warming',
      rms: 0,
      peak: 0,
      fft_bands: [0, 0, 0, 0, 0, 0, 0, 0],
      fft_availability: 'Warming',
      bass: 1,
      mid: 1,
      treb: 1,
      bass_att: 1,
      mid_att: 1,
      treb_att: 1,
      degraded: false,
    };
    const { container } = render(AudioUniforms);
    const stats = container.querySelector('.au__hero-stats');
    expect(stats?.textContent).toContain('-60.0 dB');
  });

  it('peak/RMS dB readout falls back to fixture when no live snapshot is present', () => {
    // Wave 2.6 AUD.C — startup-window / vite-dev / Vitest-jsdom branch.
    // No subscribe() resolution → snapshot stays null → fixture values
    // render (peakDb -1.7, rmsDb -10.5 per audio-uniform-fixtures.ts).
    const { container } = render(AudioUniforms);
    const stats = container.querySelector('.au__hero-stats');
    expect(stats?.textContent).toContain(`${AUDIO_UNIFORMS_FIXTURE.peakDb.toFixed(1)} dB`);
    expect(stats?.textContent).toContain(`${AUDIO_UNIFORMS_FIXTURE.rmsDb.toFixed(1)} dB`);
  });

  it('source label reads settingsStore.snapshot.audio.input_device when set', () => {
    // Wave 2.6 AUD.C — when the settings store carries a non-default
    // input_device, the hero `src` label renders it verbatim (in lieu
    // of the bundle's "Mic · Yeti" fixture). Lane AUD.A's live device
    // enumeration will populate this string with an authoritative
    // device name later in Wave 2.6.
    settingsStore.snapshot = {
      ...settingsStore.snapshot,
      audio: { ...settingsStore.snapshot.audio, input_device: 'BlackHole 2ch' },
    };
    const { container } = render(AudioUniforms);
    const stats = container.querySelector('.au__hero-stats');
    expect(stats?.textContent).toContain('BlackHole 2ch');
  });
});
