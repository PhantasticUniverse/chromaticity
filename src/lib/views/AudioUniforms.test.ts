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
// `livePeaks`, `audioReducedMotion`); no IPC, no shared store writes.
// `cleanup()` alone is sufficient between tests.

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

  it('renders the disabled Snapshot button as a Phase-3+ stub', () => {
    const { getByRole } = render(AudioUniforms);
    const snap = getByRole('button', { name: 'Snapshot' });
    expect(snap).toBeDisabled();
    expect(snap).toHaveAttribute('title', expect.stringMatching(/Phase-3/));
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
});
