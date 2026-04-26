<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-12 PR3 Perform view — the default landing route. Extracts the
  // pre-shell `App.svelte` single-page surfaces verbatim: control 0
  // slider + runtime read-back, 16-slot snap grid, preset prev/recall/
  // next, playlist prev/next, and the W8 widgets showcase. The IPC
  // round-trip wiring (dispatch + readback + error coercion via
  // `toIpcError`) transfers without functional change — the only
  // delta is that this lives under a routed view container instead
  // of the page root.
  //
  // The `<ThemeSwitcher>` lived in the Perform header through W12; W14
  // PR3 relocates it to the Settings view (the wireframe-aligned home
  // for theme preferences). Operators reach it via the SETTINGS rail
  // entry. This matches the bundle's `views/settings.jsx` layout.
  import ErrorMessage from '../ErrorMessage.svelte';
  import type { ControlId, ControlValue, PresetId } from '../ipc/bindings';
  import { dispatchAndRead } from '../stores/control';
  import { dispatchPlaylistAdvanceAndRead, readPlaylistPosition } from '../stores/playlist';
  import { dispatchPresetRecallAndRead, readActivePreset } from '../stores/preset';
  import { clearSnap, readSnapState, recallSnap, saveSnap, type SnapState } from '../stores/snap';
  import { toIpcError } from '../types/ipc-error';
  import { Fader, Knob, Toggle, Swatch, ControlGroup } from '../widgets';

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

  let activePresetId = $state<PresetId | null>(null);
  let presetPending = $state(false);
  let presetError = $state<string | null>(null);

  let playlistPos = $state<number>(0);
  let playlistPending = $state(false);
  let playlistError = $state<string | null>(null);

  // Mount-time hydration: read canonical state once on first render so
  // the UI reflects any pre-existing runtime state. Empty-deps `$effect`
  // runs once after the component mounts.
  $effect(() => {
    void (async () => {
      try {
        snapState = await readSnapState();
      } catch (err) {
        snapError = toIpcError(err, 'read_snap_state').message;
      }
      try {
        activePresetId = await readActivePreset();
      } catch (err) {
        presetError = toIpcError(err, 'read_active_preset').message;
      }
      try {
        playlistPos = await readPlaylistPosition();
      } catch (err) {
        playlistError = toIpcError(err, 'read_playlist_position').message;
      }
    })();
  });

  async function handleInput(event: Event) {
    const next = parseFloat((event.currentTarget as HTMLInputElement).value);
    sliderValue = next;
    pending = true;
    lastError = null;
    try {
      readback = await dispatchAndRead(TARGET_ID, next);
    } catch (err) {
      lastError = toIpcError(err, 'dispatch_control').message;
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
      snapError = toIpcError(err, 'snap_action').message;
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

  async function handlePresetStep(delta: number) {
    if (activePresetId === null) {
      return;
    }
    const target = activePresetId + delta;
    if (target < 0) {
      return;
    }
    presetPending = true;
    presetError = null;
    try {
      activePresetId = await dispatchPresetRecallAndRead(target);
    } catch (err) {
      presetError = toIpcError(err, 'dispatch_preset_recall').message;
    } finally {
      presetPending = false;
    }
  }

  async function handlePresetRecallZero() {
    presetPending = true;
    presetError = null;
    try {
      activePresetId = await dispatchPresetRecallAndRead(0);
    } catch (err) {
      presetError = toIpcError(err, 'dispatch_preset_recall').message;
    } finally {
      presetPending = false;
    }
  }

  async function handlePlaylistStep(step: number) {
    playlistPending = true;
    playlistError = null;
    try {
      playlistPos = await dispatchPlaylistAdvanceAndRead(step);
    } catch (err) {
      playlistError = toIpcError(err, 'dispatch_playlist_advance').message;
    } finally {
      playlistPending = false;
    }
  }
</script>

<section class="perform">
  <header class="perform__header">
    <h1>Chromaticity</h1>
  </header>
  <p class="perform__intro">
    Tauri 2 + Svelte 5 + TypeScript shell — Phase-1 control / snap / preset / playlist IPC
    round-trips with three-theme token spread.
  </p>

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

  <ErrorMessage error={lastError} />

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
    <ErrorMessage error={snapError} />
  </section>

  <section class="preset">
    <h2>presets</h2>
    <div class="row">
      <button
        type="button"
        disabled={presetPending || activePresetId === null || activePresetId === 0}
        onclick={() => handlePresetStep(-1)}
      >
        prev
      </button>
      {#if activePresetId === null}
        <button type="button" disabled={presetPending} onclick={handlePresetRecallZero}>
          recall 0
        </button>
      {:else}
        <button type="button" disabled={presetPending} onclick={() => handlePresetStep(1)}>
          next
        </button>
      {/if}
    </div>
    <p class="readback">
      active preset: {activePresetId === null ? '—' : activePresetId}
    </p>
    <ErrorMessage error={presetError} />
  </section>

  <section class="playlist">
    <h2>playlist</h2>
    <div class="row">
      <button type="button" disabled={playlistPending} onclick={() => handlePlaylistStep(-1)}>
        prev
      </button>
      <button type="button" disabled={playlistPending} onclick={() => handlePlaylistStep(1)}>
        next
      </button>
    </div>
    <p class="readback">position: {playlistPos}</p>
    <ErrorMessage error={playlistError} />
  </section>

  <section class="widgets">
    <h2>widgets</h2>
    <ControlGroup name="motion">
      <Fader value={0.62} label="amount" modulated />
      <Knob value={0.3} label="speed" showValue />
      <Toggle value={true} label="enabled" />
      <Swatch color="#c4ff3d" label="primary" />
    </ControlGroup>
  </section>
</section>

<style>
  /* Token-driven scoped surface. CSS preserves the pre-shell layout so
   * the Perform view reads as the prior single-page surface. */
  .perform {
    color: var(--color-text-0);
    max-width: 32rem;
  }
  .perform__header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
  }
  .perform__header h1 {
    margin: 0;
  }
  .perform__intro {
    color: var(--color-text-1);
    margin: 0 0 var(--space-3xl) 0;
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
  .preset,
  .playlist {
    margin-top: var(--space-4xl);
  }
  .preset h2,
  .playlist h2 {
    font-size: var(--font-size-display-sm);
    margin-bottom: var(--space-lg);
  }
  .row {
    display: flex;
    gap: var(--space-sm);
  }
  .widgets {
    margin-top: var(--space-4xl);
  }
  .widgets h2 {
    font-size: var(--font-size-display-sm);
    margin-bottom: var(--space-lg);
  }
</style>
