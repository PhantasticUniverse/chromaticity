<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-17 PR3 Audio Uniforms view full port — replaces the W16 PR2
  // `EmptyState` stub with the spectrum + frequency-band tiles + time
  // uniforms graph + BPM uniform cards layout per the canonical bundle
  // JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/audio.jsx`,
  // the `AudioUniformsView` component mapped to `id: "audio"` in the
  // bundle's VIEWS array). The W16 PR2 sidebar redesign added Audio
  // Uniforms as an enabled stub; this PR fills in the operator-facing
  // content. Bundle JSX is the canonical structural reference per the
  // auto-memory `feedback_wireframes_normative_for_ui_ux.md`; the
  // `v0.2-ui-wireframe-screenshots/pulse/audio-uniforms.png` rendering
  // is the visual confirmation (it renders the same JSX).
  //
  // Locked W17 PR3 deviations from the bundle:
  //   - The bundle's `requestAnimationFrame` driver and signal generators
  //     are gone; this view renders a frozen-frame snapshot sampled at
  //     `t ≈ 17 s` so the static page matches the wireframe screenshot.
  //     Real-time refresh waits for the audio adapter wiring landing in
  //     Phase-3+/4a `EV-INTEGRATION-MATRIX` scope (per docs/26).
  //   - The bundle's `<Spectrum>`, `<Preview>`, and `<Sparkline>` shared
  //     primitives are not yet ported as Svelte widgets; this view inlines
  //     SVG renderings of each. No new icon / canvas primitives are
  //     introduced — the brief constraint and the project's "three similar
  //     lines beats a premature abstraction" anti-pattern both apply.
  //   - Streaming / Pause buttons are visual stubs; toggling `streaming`
  //     locally is fine, but no IPC is wired (Phase-3+/4a scope).
  //   - The `Live Visualizer` preview placeholder is a static gradient blob
  //     field — same approach the W16 PR3 Welcome view took for its
  //     `<Preview>` substitution, for the same reason (runtime / scene
  //     crates are not wired through to the UI yet).
  //   - W20 view-internal icon sweep: Snapshot button, "How to use"
  //     collapse caret, and the docs deep-link arrow render via the W19
  //     PR3 vendored `Icon.svelte` primitive (`camera` / `caret` /
  //     `chevron` / `arrow`) per `views/audio.jsx`. Inline `●` / `○`
  //     hit-state markers in the band-bar value column have no clean
  //     bundle Icon mapping and stay as Unicode (font-driven; the
  //     filled-circle `bang` icon is a different metaphor).
  import {
    AUDIO_UNIFORMS_FIXTURE,
    HOW_TO_USE_BLOCKS,
    type FrequencyBandBar,
  } from '../data/audio-uniform-fixtures';
  import Icon from '../Icon.svelte';

  let streaming = $state<boolean>(AUDIO_UNIFORMS_FIXTURE.streamingActive);
  let docsOpen = $state<boolean>(true);

  function toggleStreaming(): void {
    streaming = !streaming;
  }

  function toggleDocs(): void {
    docsOpen = !docsOpen;
  }

  // Spectrum SVG geometry — 64 bars across a 100-unit viewBox so the SVG
  // scales fluidly to whatever pixel width the panel grants it.
  const SPECTRUM_VIEW_W = 100;
  const SPECTRUM_VIEW_H = 36;
  const SPECTRUM_BAR_GAP = 0.15; // viewBox units
  const SPECTRUM_BAR_W =
    (SPECTRUM_VIEW_W - SPECTRUM_BAR_GAP * (AUDIO_UNIFORMS_FIXTURE.spectrumBars.length - 1)) /
    AUDIO_UNIFORMS_FIXTURE.spectrumBars.length;

  function spectrumBarX(index: number): number {
    return index * (SPECTRUM_BAR_W + SPECTRUM_BAR_GAP);
  }
  function spectrumBarH(value: number): number {
    return Math.max(0.6, value * (SPECTRUM_VIEW_H - 1));
  }
  function spectrumBarY(value: number): number {
    return SPECTRUM_VIEW_H - spectrumBarH(value);
  }

  // Sparkline SVG geometry — matches the bundle's 100×32 viewBox / 60-point
  // window so the existing fixture sample arrays drop straight in.
  const SPARK_W = 100;
  const SPARK_H = 32;

  function sparklinePath(samples: readonly number[]): string {
    if (samples.length === 0) return '';
    return samples
      .map((sample, i) => {
        const x = (i / (samples.length - 1)) * SPARK_W;
        const y = SPARK_H - sample * (SPARK_H - 4) - 2;
        return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  }

  function sparklineHeadX(samples: readonly number[]): number {
    if (samples.length === 0) return 0;
    return ((samples.length - 1) / (samples.length - 1)) * SPARK_W;
  }
  function sparklineHeadY(samples: readonly number[]): number {
    if (samples.length === 0) return SPARK_H / 2;
    const last = samples[samples.length - 1] ?? 0;
    return SPARK_H - last * (SPARK_H - 4) - 2;
  }

  function shortLabel(kind: FrequencyBandBar['kind']): string {
    if (kind === 'level') return 'LVL';
    if (kind === 'hit') return 'HIT';
    return 'PRES';
  }
</script>

<section class="au">
  <div class="au__main">
    <header class="au__header">
      <h2 class="au__title">
        Audio Uniforms <span class="au__title-sep" aria-hidden="true">·</span>
        <span class="au__title-mode">Live</span>
      </h2>
      <div class="au__header-actions">
        <span class="au__pill" class:au__pill--on={streaming}>
          <span class="au__pill-dot" aria-hidden="true"></span>
          {streaming ? 'streaming' : 'paused'}
        </span>
        <button
          type="button"
          class="au__btn"
          onclick={toggleStreaming}
          aria-pressed={streaming ? 'false' : 'true'}
        >
          {streaming ? 'Freeze' : 'Resume'}
        </button>
        <button
          type="button"
          class="au__btn"
          disabled
          aria-label="Snapshot"
          title="Snapshot — wired in Phase-3+"
        >
          <Icon name="camera" size={11} /> Snapshot
        </button>
      </div>
    </header>

    <div class="au__body">
      <!-- Hero: spectrum -->
      <section class="au__hero" aria-label="Spectrum">
        <div class="au__hero-head">
          <span class="au__section-label">waveform · spectrum</span>
          <div class="au__hero-stats">
            <span>peak <b>{AUDIO_UNIFORMS_FIXTURE.peakDb.toFixed(1)} dB</b></span>
            <span>rms <b>{AUDIO_UNIFORMS_FIXTURE.rmsDb.toFixed(1)} dB</b></span>
            <span>src <b>{AUDIO_UNIFORMS_FIXTURE.source}</b></span>
          </div>
        </div>
        <div class="au__hero-canvas" role="img" aria-label="Audio spectrum (frozen frame)">
          <svg
            class="au__spectrum-svg"
            viewBox="0 0 {SPECTRUM_VIEW_W} {SPECTRUM_VIEW_H}"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            {#each AUDIO_UNIFORMS_FIXTURE.spectrumBars as bar, i (i)}
              <rect
                class="au__spectrum-bar"
                x={spectrumBarX(i)}
                y={spectrumBarY(bar)}
                width={SPECTRUM_BAR_W}
                height={spectrumBarH(bar)}
                rx="0.2"
              />
            {/each}
          </svg>
        </div>
      </section>

      <!-- Frequency bands -->
      <section class="au__section" aria-label="Frequency bands">
        <header class="au__section-head">
          <span class="au__section-title">frequency bands</span>
          <span class="au__section-sub">smoothed · hits · presence per band</span>
        </header>
        <div class="au__bands">
          {#each AUDIO_UNIFORMS_FIXTURE.bands as band (band.id)}
            <div class="au__band" style="--band-tint: {band.tint};">
              <div class="au__band-head">
                <span class="au__band-name">{band.label}</span>
                <span class="au__band-sub">{band.subLabel}</span>
              </div>
              <div class="au__band-bars">
                {#each band.bars as bar (bar.key)}
                  <div class="au__bar">
                    <div class="au__bar-track">
                      {#if bar.kind === 'hit'}
                        <div
                          class="au__bar-flash"
                          style="opacity: {Math.max(0, Math.min(1, bar.value))};"
                        ></div>
                      {:else}
                        <div
                          class="au__bar-fill"
                          class:au__bar-fill--presence={bar.kind === 'presence'}
                          style="height: {Math.max(0.02, bar.value) * 100}%;"
                        ></div>
                      {/if}
                    </div>
                    <div
                      class="au__bar-value"
                      class:au__bar-value--hit={bar.kind === 'hit'}
                      class:au__bar-value--lit={bar.kind === 'hit' && bar.value > 0.05}
                    >
                      {#if bar.kind === 'hit'}
                        {bar.value > 0.05 ? '●' : '○'}
                      {:else}
                        {bar.value.toFixed(2)}
                      {/if}
                    </div>
                    <div class="au__bar-label">{shortLabel(bar.kind)}</div>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </section>

      <!-- Time uniforms -->
      <section class="au__section" aria-label="Time uniforms">
        <header class="au__section-head">
          <span class="au__section-title">time uniforms</span>
          <span class="au__section-sub">per-band accumulators · seconds since session start</span>
        </header>
        <div class="au__time-rows">
          {#each AUDIO_UNIFORMS_FIXTURE.timeUniforms as uniform (uniform.key)}
            <div class="au__time-row" style="--time-tint: {uniform.tint};">
              <span class="au__time-key mono">{uniform.key}</span>
              <div class="au__time-track">
                <div class="au__time-fill" style="width: {uniform.phase * 100}%;"></div>
              </div>
              <span class="au__time-val mono">{uniform.value.toFixed(2)}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- BPM uniforms -->
      <section class="au__section" aria-label="BPM uniforms">
        <header class="au__section-head">
          <span class="au__section-title">bpm uniforms</span>
          <span class="au__section-sub"
            >tempo-locked oscillators · live preview of last 3 seconds</span
          >
        </header>
        <div class="au__bpm-grid">
          {#each AUDIO_UNIFORMS_FIXTURE.bpmUniforms as bpm (bpm.key)}
            {#if bpm.kind === 'scalar'}
              <article class="au__bpm-card au__bpm-card--accent">
                <header class="au__bpm-head">
                  <span class="au__bpm-name mono">{bpm.key}</span>
                  <span class="au__bpm-val mono">{bpm.displayValue}</span>
                </header>
                <span class="au__bpm-label">{bpm.label}</span>
              </article>
            {:else if bpm.kind === 'bool'}
              <article class="au__bpm-card au__bpm-card--bool">
                <header class="au__bpm-head">
                  <span class="au__bpm-name mono">{bpm.key}</span>
                  <span
                    class="au__bpm-led"
                    class:au__bpm-led--on={bpm.ledOn ?? false}
                    aria-hidden="true"
                  ></span>
                </header>
                <span class="au__bpm-label">{bpm.label}</span>
              </article>
            {:else}
              <article class="au__bpm-card">
                <header class="au__bpm-head">
                  <span class="au__bpm-name mono">{bpm.key}</span>
                  <span class="au__bpm-val mono">{bpm.displayValue}</span>
                </header>
                {#if bpm.sparkline && bpm.sparkline.length > 0}
                  <div class="au__spark" aria-hidden="true">
                    <div class="au__spark-mid"></div>
                    <svg
                      class="au__spark-svg"
                      viewBox="0 0 {SPARK_W} {SPARK_H}"
                      preserveAspectRatio="none"
                    >
                      <path
                        class="au__spark-path"
                        d={sparklinePath(bpm.sparkline)}
                        vector-effect="non-scaling-stroke"
                      />
                      <circle
                        class="au__spark-head"
                        cx={sparklineHeadX(bpm.sparkline)}
                        cy={sparklineHeadY(bpm.sparkline)}
                        r="1.6"
                      />
                    </svg>
                  </div>
                {/if}
                <span class="au__bpm-label">{bpm.label}</span>
              </article>
            {/if}
          {/each}
        </div>
      </section>
    </div>
  </div>

  <aside class="au__side" aria-label="Audio uniforms reference">
    <div class="au__preview" role="img" aria-label="Live visualizer preview">
      <span class="au__preview-blob au__preview-blob--bass"></span>
      <span class="au__preview-blob au__preview-blob--mid"></span>
      <span class="au__preview-blob au__preview-blob--high"></span>
      <span class="au__preview-label">live visualizer</span>
    </div>

    <section class="au__docs">
      <button
        type="button"
        class="au__docs-toggle"
        aria-expanded={docsOpen ? 'true' : 'false'}
        onclick={toggleDocs}
      >
        <span>how to use</span>
        <span class="au__docs-chevron" class:au__docs-chevron--collapsed={!docsOpen}>
          <Icon name={docsOpen ? 'caret' : 'chevron'} size={11} />
        </span>
      </button>
      {#if docsOpen}
        <div class="au__docs-body">
          {#each HOW_TO_USE_BLOCKS as block, i (i)}
            <div class="au__docs-block">
              {#if block.title}
                <h3 class="au__docs-subhead">{block.title}</h3>
              {/if}
              <p class="au__docs-prose">
                {#if block.intro}{block.intro}{/if}
                {#if block.tokens}
                  {#each block.tokens as token, ti (ti)}
                    <span class="au__docs-token mono" style="color: {token.tint};"
                      >{token.text}</span
                    >{#if block.tokens && ti < block.tokens.length - 1},
                    {/if}
                  {/each}
                {/if}
                {#if block.trailing}{block.trailing}{/if}
              </p>
            </div>
          {/each}
          <a class="au__docs-link mono" href="#audio-uniforms-docs">
            <Icon name="arrow" size={11} /> Read the Chromaticity Audio Uniforms docs
          </a>
        </div>
      {/if}
    </section>
  </aside>
</section>

<style>
  /* ---------------- Outer grid ----------------------------------------- */
  .au {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: var(--space-md);
    align-items: stretch;
    min-height: 0;
  }

  @media (max-width: 980px) {
    .au {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* ---------------- Main panel ----------------------------------------- */
  .au__main {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .au__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
  }

  .au__title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
  }
  .au__title-sep {
    color: var(--color-text-3);
    margin: 0 4px;
  }
  .au__title-mode {
    color: var(--color-primary);
  }

  .au__header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .au__pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 2px 8px;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    background: var(--color-bg-2);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .au__pill--on {
    color: var(--color-success);
    border-color: color-mix(in oklab, var(--color-success) 50%, var(--color-line-2));
  }
  .au__pill-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
  }
  .au__pill--on .au__pill-dot {
    background: var(--color-success);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-success);
  }

  .au__btn {
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    cursor: pointer;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .au__btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .au__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .au__btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  /* ---------------- Body / sections ------------------------------------ */
  .au__body {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    padding: var(--space-lg);
    overflow: auto;
    min-height: 0;
  }

  .au__section {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .au__section-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-md);
  }

  .au__section-label,
  .au__section-title {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .au__section-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }

  /* ---------------- Spectrum hero -------------------------------------- */
  .au__hero {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-md);
  }
  .au__hero-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-md);
  }
  .au__hero-stats {
    display: flex;
    gap: var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  .au__hero-stats b {
    color: var(--color-text-1);
    font-weight: var(--font-weight-label);
  }
  .au__hero-canvas {
    height: 140px;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    overflow: hidden;
    padding: 4px;
  }
  .au__spectrum-svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .au__spectrum-bar {
    fill: var(--color-band-midhi);
    opacity: 0.85;
  }

  /* ---------------- Band tiles ----------------------------------------- */
  .au__bands {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: var(--space-sm);
  }

  @media (max-width: 1100px) {
    .au__bands {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  .au__band {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-top: 2px solid var(--band-tint);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    min-width: 0;
  }
  .au__band-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-xs);
  }
  .au__band-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--band-tint);
  }
  .au__band-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }
  .au__band-bars {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-sm);
  }
  .au__bar {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .au__bar-track {
    position: relative;
    width: 100%;
    height: 56px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-xs);
    overflow: hidden;
  }
  .au__bar-fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--band-tint);
    opacity: 0.95;
  }
  .au__bar-fill--presence {
    opacity: 0.55;
  }
  .au__bar-flash {
    position: absolute;
    inset: 0;
    background: var(--band-tint);
  }
  .au__bar-value {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }
  .au__bar-value--hit {
    color: var(--color-text-3);
  }
  .au__bar-value--lit {
    color: var(--band-tint);
  }
  .au__bar-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
  }

  /* ---------------- Time uniforms -------------------------------------- */
  .au__time-rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
  }
  .au__time-row {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr) 60px;
    gap: var(--space-md);
    align-items: center;
    padding: 4px 0;
  }
  .au__time-key {
    color: var(--color-text-1);
    font-size: var(--font-size-mono-sm);
  }
  .au__time-track {
    height: 6px;
    background: var(--color-bg-3);
    border-radius: var(--radius-full);
    overflow: hidden;
  }
  .au__time-fill {
    height: 100%;
    background: var(--time-tint);
    border-radius: var(--radius-full);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--time-tint);
  }
  .au__time-val {
    text-align: right;
    color: var(--color-text-1);
    font-size: var(--font-size-mono-sm);
  }

  /* ---------------- BPM cards ------------------------------------------ */
  .au__bpm-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-sm);
  }

  .au__bpm-card {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    min-width: 0;
  }
  .au__bpm-card--accent {
    border-color: color-mix(in oklab, var(--color-primary) 35%, var(--color-line-1));
  }
  .au__bpm-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }
  .au__bpm-name {
    color: var(--color-text-1);
    font-size: var(--font-size-mono-sm);
  }
  .au__bpm-val {
    color: var(--color-primary);
    font-size: var(--font-size-mono-md);
    font-weight: var(--font-weight-label);
  }
  .au__bpm-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .au__bpm-led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-text-4);
    border: 1px solid var(--color-line-2);
  }
  .au__bpm-led--on {
    background: var(--color-success);
    border-color: var(--color-success);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-success);
  }

  /* ---------------- Sparkline ------------------------------------------ */
  .au__spark {
    position: relative;
    width: 100%;
    height: 32px;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-xs);
    overflow: hidden;
  }
  .au__spark-mid {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    background: var(--color-line-1);
    opacity: 0.7;
  }
  .au__spark-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .au__spark-path {
    fill: none;
    stroke: var(--color-primary);
    stroke-width: 1.2;
    opacity: 0.95;
  }
  .au__spark-head {
    fill: var(--color-primary);
  }

  /* ---------------- Right sidebar -------------------------------------- */
  .au__side {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
  }

  .au__preview {
    position: relative;
    aspect-ratio: 1 / 1;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-panel);
  }
  .au__preview-blob {
    position: absolute;
    width: 70%;
    height: 70%;
    border-radius: 50%;
    filter: blur(36px);
    opacity: calc(0.65 * var(--glow, 1));
  }
  .au__preview-blob--bass {
    top: -10%;
    left: -10%;
    background: var(--color-band-bass);
  }
  .au__preview-blob--mid {
    top: 20%;
    right: -15%;
    background: var(--color-band-mid);
  }
  .au__preview-blob--high {
    bottom: -10%;
    left: 20%;
    background: var(--color-band-high);
  }
  .au__preview-label {
    position: absolute;
    bottom: var(--space-sm);
    left: var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-1);
    background: color-mix(in oklab, var(--color-bg-0) 65%, transparent);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
  }

  .au__docs {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .au__docs-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-md) var(--space-lg);
    background: var(--color-bg-3);
    border: 0;
    border-bottom: 1px solid var(--color-line-2);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    cursor: pointer;
  }
  .au__docs-toggle:hover {
    color: var(--color-text-0);
  }
  .au__docs-toggle:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .au__docs-chevron {
    color: var(--color-text-3);
  }
  .au__docs-body {
    padding: var(--space-md) var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    overflow: auto;
  }
  .au__docs-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .au__docs-subhead {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }
  .au__docs-prose {
    margin: 0;
    font-size: var(--font-size-body-sm);
    line-height: var(--line-height-body);
    color: var(--color-text-1);
  }
  .au__docs-token {
    color: var(--color-primary);
    font-size: var(--font-size-mono-sm);
  }
  .au__docs-link {
    color: var(--color-primary);
    font-size: var(--font-size-mono-sm);
    text-decoration: none;
  }
  .au__docs-link:hover {
    text-decoration: underline;
  }

  /* ---------------- Shared utility ------------------------------------- */
  .mono {
    font-family: var(--font-family-mono);
  }
</style>
