<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import {
    YOUTUBE_BLEND_MODES,
    youtubeBlendModeToCss,
    youtubeLayerStore,
    type YouTubeBlendMode,
  } from '../stores/youtubeLayer.svelte';

  const BLEND_MODE_LABELS: Readonly<Record<YouTubeBlendMode, string>> = {
    normal: 'Normal',
    screen: 'Screen',
    add: 'Add',
    multiply: 'Multiply',
    'luma-key': 'Luma Key',
  };

  let urlInput = $state('');

  const inputId = 'yt-layer-url-input';
  const errorId = 'yt-layer-url-error';
  const blendSelectId = 'yt-layer-blend-mode';
  const opacitySliderId = 'yt-layer-opacity';

  const machine = $derived(youtubeLayerStore.state);
  const embedUrl = $derived(youtubeLayerStore.embedUrl);
  const canClear = $derived(machine.kind !== 'empty' || urlInput.trim().length > 0);

  function handleSubmit(event: SubmitEvent): void {
    event.preventDefault();
    youtubeLayerStore.load(urlInput);
  }

  function handleClear(): void {
    urlInput = '';
    youtubeLayerStore.clear();
  }

  function handleBlendChange(event: Event): void {
    const target = event.currentTarget as HTMLSelectElement;
    const candidate = target.value as YouTubeBlendMode;
    if ((YOUTUBE_BLEND_MODES as readonly string[]).includes(candidate)) {
      youtubeLayerStore.setBlendMode(candidate);
    }
  }

  function handleOpacityInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    const parsed = Number.parseFloat(target.value);
    if (Number.isFinite(parsed)) {
      youtubeLayerStore.setOpacity(parsed);
    }
  }
</script>

<section class="yt-layer" aria-label="YouTube visual stream layer">
  <form class="yt-paste-bar" onsubmit={handleSubmit} novalidate>
    <label class="sr-only" for={inputId}>YouTube URL or video ID</label>
    <input
      id={inputId}
      class="yt-url-input"
      type="text"
      bind:value={urlInput}
      placeholder="Paste YouTube URL or ID…"
      autocomplete="off"
      spellcheck={false}
      aria-invalid={machine.kind === 'error' ? 'true' : undefined}
      aria-describedby={machine.kind === 'error' ? errorId : undefined}
    />
    <button type="submit" class="yt-btn yt-btn-primary">Load</button>
    <button type="button" class="yt-btn yt-btn-ghost" disabled={!canClear} onclick={handleClear}>
      Clear
    </button>
  </form>

  {#if machine.kind === 'error'}
    <p id={errorId} class="yt-error" role="alert">
      {machine.message}
    </p>
  {/if}

  {#if machine.kind === 'ready' && embedUrl !== null}
    <div class="yt-controls" role="group" aria-label="YouTube layer controls">
      <button
        type="button"
        class="yt-chip"
        class:yt-chip-safe={machine.muted}
        class:yt-chip-live={!machine.muted}
        aria-pressed={!machine.muted}
        onclick={() => youtubeLayerStore.toggleMuted()}
      >
        {machine.muted ? 'Muted (DJ Safe)' : 'Audio On'}
      </button>

      <!-- Audit 2026-11-28: no audio path from the YouTube player into the
           visuals exists (the embedded player is cross-origin), so this
           stays disabled rather than looking like it works. -->
      <button
        type="button"
        class="yt-chip"
        disabled
        aria-disabled="true"
        aria-pressed="false"
        title="Not available yet: YouTube plays in a protected embedded player whose audio cannot reach the visuals"
      >
        Route Audio to Visuals · planned
      </button>

      <div class="yt-field">
        <label class="yt-field-label" for={blendSelectId}>Blend Mode</label>
        <select
          id={blendSelectId}
          class="yt-select"
          value={machine.blendMode}
          onchange={handleBlendChange}
        >
          {#each YOUTUBE_BLEND_MODES as mode (mode)}
            <option value={mode}>{BLEND_MODE_LABELS[mode]}</option>
          {/each}
        </select>
      </div>

      <div class="yt-field yt-field-opacity">
        <label class="yt-field-label" for={opacitySliderId}>Opacity</label>
        <input
          id={opacitySliderId}
          class="yt-range"
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={String(machine.opacity)}
          aria-valuetext={`${Math.round(machine.opacity * 100)}%`}
          oninput={handleOpacityInput}
        />
        <span class="yt-readout" aria-hidden="true">{Math.round(machine.opacity * 100)}%</span>
      </div>
    </div>

    <div class="yt-stage" data-blend-mode={machine.blendMode}>
      <iframe
        class="yt-iframe"
        class:yt-iframe-luma-key={machine.blendMode === 'luma-key'}
        src={embedUrl}
        title={`YouTube visual layer (${machine.videoId})`}
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
        referrerpolicy="origin"
        style={`opacity: ${machine.opacity}; mix-blend-mode: ${youtubeBlendModeToCss(machine.blendMode)};`}
      ></iframe>
    </div>
  {/if}
</section>

<style>
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .yt-layer {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-md);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-md);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    box-shadow: var(--shadow-panel);
  }

  .yt-paste-bar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .yt-url-input {
    flex: 1 1 auto;
    min-width: 0;
    height: 30px;
    padding: 0 var(--space-md);
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    transition: border-color var(--duration-fast) var(--easing-out);
  }

  .yt-url-input::placeholder {
    color: var(--color-text-2);
  }

  .yt-url-input:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
    border-color: var(--color-primary);
  }

  .yt-url-input[aria-invalid='true'] {
    border-color: var(--color-danger);
  }

  .yt-btn {
    height: 30px;
    padding: 0 var(--space-lg);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-label);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--easing-out),
      border-color var(--duration-fast) var(--easing-out),
      opacity var(--duration-fast) var(--easing-out);
  }

  .yt-btn:focus-visible,
  .yt-chip:focus-visible,
  .yt-select:focus-visible,
  .yt-range:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .yt-btn-primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }

  .yt-btn-primary:hover {
    filter: brightness(1.06);
  }

  .yt-btn-ghost {
    background: var(--color-bg-3);
    color: var(--color-text-1);
  }

  .yt-btn-ghost:hover:not(:disabled) {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
  }

  .yt-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .yt-error {
    margin: 0;
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-1);
    border-left: 3px solid var(--color-danger);
    border-radius: var(--radius-xs);
    color: var(--color-danger);
    font-size: var(--font-size-body-sm);
    line-height: var(--line-height-body);
  }

  .yt-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }

  .yt-chip {
    height: 28px;
    padding: 0 var(--space-md);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-label);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--easing-out),
      border-color var(--duration-fast) var(--easing-out),
      color var(--duration-fast) var(--easing-out);
  }

  .yt-chip-safe {
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }

  .yt-chip-live {
    background: var(--color-warning);
    border-color: var(--color-warning);
    color: var(--color-on-primary);
  }

  .yt-chip:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .yt-field {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .yt-field-label {
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    letter-spacing: var(--letter-spacing-caps-md);
    text-transform: uppercase;
    color: var(--color-text-2);
  }

  .yt-select {
    height: 28px;
    padding: 0 var(--space-md);
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
  }

  .yt-field-opacity {
    margin-left: auto;
  }

  .yt-range {
    width: 108px;
    accent-color: var(--color-primary);
    cursor: pointer;
  }

  .yt-readout {
    min-width: 36px;
    text-align: right;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .yt-stage {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .yt-iframe {
    display: block;
    width: 100%;
    height: 100%;
    border: 0;
  }

  .yt-iframe-luma-key {
    filter: contrast(1.35) brightness(0.95);
  }
</style>
