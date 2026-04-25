<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import type { ControlId, ControlValue } from './lib/ipc/bindings';
  import { dispatchAndRead } from './lib/stores/control';

  const TARGET_ID: ControlId = 0;

  let sliderValue = $state<ControlValue>(0);
  let readback = $state<ControlValue | null>(null);
  let lastError = $state<string | null>(null);
  let pending = $state(false);

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
</script>

<main>
  <h1>Chromaticity</h1>
  <p>Tauri 2 + Svelte 5 + TypeScript shell — Phase-1 D2 control round-trip.</p>

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
</main>

<style>
  main {
    font-family: system-ui, sans-serif;
    padding: 2rem;
    max-width: 32rem;
  }
  .control {
    display: grid;
    grid-template-columns: 6rem 1fr 4rem;
    align-items: center;
    gap: 0.75rem;
    margin: 1.5rem 0;
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
  }
  .error {
    color: #c0392b;
  }
</style>
