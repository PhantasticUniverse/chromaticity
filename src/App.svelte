<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import type { ControlId, ControlValue } from './lib/ipc/bindings';
  import { dispatchAndRead } from './lib/stores/control';
  import {
    clearSnap,
    readSnapState,
    recallSnap,
    saveSnap,
    type SnapState,
  } from './lib/stores/snap';

  const TARGET_ID: ControlId = 0;
  const SLOT_INDICES = Array.from({ length: 16 }, (_, i) => i);

  let sliderValue = $state<ControlValue>(0);
  let readback = $state<ControlValue | null>(null);
  let lastError = $state<string | null>(null);
  let pending = $state(false);

  let snapState = $state<SnapState>({
    active: null,
    saved: SLOT_INDICES.map(() => false),
  });
  let snapPending = $state(false);
  let snapError = $state<string | null>(null);

  async function handleInput(event: Event) {
    const next = parseFloat((event.currentTarget as HTMLInputElement).value);
    sliderValue = next;
    pending = true;
    lastError = null;
    try {
      readback = await dispatchAndRead(TARGET_ID, next);
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err);
    } finally {
      pending = false;
    }
  }

  async function refreshSnap() {
    snapState = await readSnapState();
  }

  async function withSnapAction(action: () => Promise<void>) {
    snapPending = true;
    snapError = null;
    try {
      await action();
      await refreshSnap();
    } catch (err) {
      snapError = err instanceof Error ? err.message : String(err);
    } finally {
      snapPending = false;
    }
  }

  async function handleSlotClick(slot: number) {
    const isSaved = snapState.saved[slot] === true;
    const isActive = snapState.active === slot;
    if (isSaved && !isActive) {
      await withSnapAction(() => recallSnap(slot));
    } else {
      await withSnapAction(() => saveSnap(slot));
    }
  }

  async function handleSlotContext(event: MouseEvent, slot: number) {
    event.preventDefault();
    if (snapState.saved[slot] === true) {
      await withSnapAction(() => clearSnap(slot));
    }
  }
</script>

<main>
  <h1>Chromaticity</h1>
  <p>Tauri 2 + Svelte 5 + TypeScript shell — Phase-1 D3 snap round-trip.</p>

  <label class="control">
    <span>control 0</span>
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={sliderValue}
      oninput={handleInput}
      disabled={pending}
    />
    <span class="value">{sliderValue.toFixed(2)}</span>
  </label>

  <p class="readback">
    runtime read-back: {readback === null ? '—' : readback.toFixed(2)}
  </p>

  {#if lastError}
    <p class="error">error: {lastError}</p>
  {/if}

  <section class="snap">
    <h2>snap slots</h2>
    <div class="snap-grid">
      {#each SLOT_INDICES as slot (slot)}
        {@const isSaved = snapState.saved[slot] === true}
        {@const isActive = snapState.active === slot}
        <button
          type="button"
          class="snap-slot"
          class:saved={isSaved}
          class:active={isActive}
          disabled={snapPending}
          onclick={() => handleSlotClick(slot)}
          oncontextmenu={(event) => handleSlotContext(event, slot)}
        >
          {slot}
        </button>
      {/each}
    </div>
    <p class="snap-readback">
      active: {snapState.active === null ? '—' : snapState.active}
    </p>
    {#if snapError}
      <p class="error">error: {snapError}</p>
    {/if}
  </section>
</main>

<style>
  /* Token-driven demo surface. Hard-coded hex was swapped to tokens from
   * src/lib/tokens/tokens.css; setting `data-theme` on <html> re-themes
   * without further CSS changes. */
  main {
    font-family: var(--font-family-sans);
    color: var(--color-text-0);
    background: var(--color-bg-0);
    padding: var(--space-4xl);
    max-width: 32rem;
  }
  .control {
    display: grid;
    grid-template-columns: 6rem 1fr 4rem;
    align-items: center;
    gap: var(--space-lg);
    margin: var(--space-3xl) 0;
  }
  .control input[type='range'] {
    width: 100%;
  }
  .value {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .readback {
    font-variant-numeric: tabular-nums;
    color: var(--color-text-1);
  }
  .error {
    color: var(--color-danger);
  }
  .snap {
    margin-top: var(--space-4xl);
  }
  .snap h2 {
    font-size: var(--font-size-display-sm);
    margin-bottom: var(--space-lg);
  }
  .snap-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: var(--space-xs);
  }
  .snap-slot {
    aspect-ratio: 1 / 1;
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
    color: var(--color-text-3);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .snap-slot.saved {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-on-primary);
  }
  .snap-slot.active {
    text-decoration: underline;
    font-weight: bold;
  }
  .snap-slot:disabled {
    cursor: progress;
  }
  .snap-readback {
    font-variant-numeric: tabular-nums;
    margin-top: var(--space-md);
    color: var(--color-text-2);
  }
</style>
