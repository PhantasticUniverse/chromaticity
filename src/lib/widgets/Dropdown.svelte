<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-32 visual primitive: discrete-option pill row. Display-only
  // (no click wiring) per the W8 sibling precedent. Ports the bundle's
  // `chroma_inspector.jsx::ChromaCtrlBody` `dropdown` branch (lines
  // 185-197): horizontal pill buttons, one per option, with the
  // currently-selected option highlighted via the `--tint` cascade.
  // Display-only (disabled) unless `onselect` is supplied.
  // When `labels` is omitted, falls back to `opt 0` / `opt 1` / ... up
  // to `optionCount` (matches the bundle's `(ctrl.VALUES || []).map((_, i) =>
  // 'opt ' + (i + 1))` fallback at line 186). Real selection wiring
  // lands with the W11 IPC scene editor wave.
  let {
    value = 0,
    labels,
    optionCount,
    onselect,
  }: {
    value?: number;
    // `| undefined` is explicit so `exactOptionalPropertyTypes` is happy
    // when callers thread an optional fixture field straight through.
    labels?: readonly string[] | undefined;
    optionCount?: number | undefined;
    onselect?: ((index: number) => void) | undefined;
  } = $props();

  const effectiveLabels = $derived(
    labels ?? Array.from({ length: optionCount ?? 1 }, (_, i) => `opt ${i}`),
  );
</script>

<div class="dropdown" role="radiogroup" aria-label="dropdown options">
  {#each effectiveLabels as label, idx (idx)}
    <button
      type="button"
      class="dropdown__opt mono"
      class:dropdown__opt--on={idx === value}
      role="radio"
      aria-checked={idx === value}
      class:dropdown__opt--live={onselect !== undefined}
      disabled={onselect === undefined}
      aria-disabled={onselect === undefined ? 'true' : 'false'}
      onclick={() => onselect?.(idx)}
    >
      {label}
    </button>
  {/each}
</div>

<style>
  .dropdown {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .dropdown__opt {
    background: var(--color-bg-1);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 3px 9px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: not-allowed;
    text-transform: lowercase;
    letter-spacing: 0.02em;
  }
  .dropdown__opt--on {
    background: color-mix(in oklab, var(--tint, var(--color-primary)) 18%, var(--color-bg-1));
    border-color: var(--tint, var(--color-primary));
    color: var(--tint, var(--color-primary));
  }
  .dropdown__opt--live {
    cursor: pointer;
  }
</style>
