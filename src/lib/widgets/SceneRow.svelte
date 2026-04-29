<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Shared-primitive backfill — `SceneRow` from
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 232-249. Bundle-faithful row layout: 4-column grid (idx /
  // thumb / name+meta / drag-glyph) with active + cued state classes,
  // tag chip ("default" / "hot" / "caution"), duration string, and
  // optional BPM-sync indicator. The thumbnail uses the existing W36
  // `SceneStill` widget (canvas-procedural renderer) — closes the W18
  // PR1 gradient surrogate by composing two already-ported primitives.
  //
  // The Perform view's right-rail scene panel and the Library setlist
  // sidebar both adopt this row shape; this widget centralises the
  // markup so future bundle revisions sync through one file.
  //
  // The bundle's `<Icon name="drag" />` glyph rides at the right; this
  // port uses the W19 PR3 vendored `Icon.svelte` with the same name.
  // The bundle's `active` row also renders a small `.beat.on` dot —
  // mirrored here as the `.scene-row__pulse` element to match the
  // existing Perform CSS vocabulary.
  import Icon from '../Icon.svelte';
  import type { SceneKind } from './scene-renderers';
  import SceneStill from './SceneStill.svelte';

  type SceneTag = 'default' | 'hot' | 'caution';

  type Props = {
    /** Row index — formatted to two digits in the leading column. */
    idx: number;
    /** Scene name displayed in the body column. */
    name: string;
    /** Tag chip vocabulary; bundle parity is `default | hot | caution`. */
    tag?: SceneTag;
    /** Display duration string (e.g. "02:00"). */
    dur?: string;
    /** Whether the row is the currently-loaded scene. */
    active?: boolean;
    /** Whether the row is queued for next playback. */
    cued?: boolean;
    /** SceneKind for the thumbnail. */
    kind?: SceneKind;
    /** Per-row variation seed for the thumbnail renderer. */
    seed?: number;
    /** Optional BPM-sync token (e.g. "1/4", "1/8"). When set, the row
     * renders an extra meta marker after the duration. */
    bpmSync?: string;
    /** Click handler. */
    onClick?: () => void;
  };

  let {
    idx,
    name,
    tag = 'default',
    dur = '02:00',
    active = false,
    cued = false,
    kind = 'rings',
    seed = 1,
    bpmSync,
    onClick,
  }: Props = $props();

  const idxLabel = $derived(String(idx).padStart(2, '0'));
</script>

<button
  type="button"
  class="scene-row"
  class:active
  class:cued
  data-tag={tag}
  aria-current={active ? 'true' : undefined}
  aria-pressed={active ? 'true' : undefined}
  onclick={onClick}
>
  <span class="scene-row__idx mono">{idxLabel}</span>
  <span class="scene-row__thumb" aria-hidden="true">
    <SceneStill {kind} {seed} width={120} height={68} />
  </span>
  <span class="scene-row__body">
    <span class="scene-row__name">{name}</span>
    <span class="scene-row__meta mono">
      <span class="scene-row__tag" data-tag={tag}>{tag}</span>
      <span>{dur}</span>
      {#if bpmSync}
        <span class="scene-row__bpm">⏣ {bpmSync}</span>
      {/if}
    </span>
  </span>
  <span class="scene-row__tail" aria-hidden="true">
    {#if active}
      <span class="scene-row__pulse"></span>
    {/if}
    <Icon name="drag" size={12} />
  </span>
</button>

<style>
  /* Token-driven; mirrors the existing Perform `.scene-row` rules so
   * a swap-in is visually identical. The active treatment uses
   * `--color-primary` accenting plus a left-edge pulse bar (the
   * `::before` pseudo) and a small dot in the tail. */
  .scene-row {
    display: grid;
    grid-template-columns: 24px 60px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-md);
    width: 100%;
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--color-line-1);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    text-align: left;
    position: relative;
    transition: background-color 120ms ease;
  }
  .scene-row:hover {
    background: var(--color-bg-3);
  }
  .scene-row:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .scene-row.active {
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--color-primary) 18%, var(--color-bg-2)),
      var(--color-bg-2)
    );
  }
  .scene-row.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--color-primary);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-primary);
  }
  .scene-row.cued {
    background: color-mix(in oklab, var(--color-warning) 12%, transparent);
  }
  .scene-row.active .scene-row__name {
    color: var(--color-primary);
  }
  .scene-row__idx {
    color: var(--color-text-3);
    font-size: var(--font-size-mono-sm);
    text-align: right;
  }
  .scene-row__thumb {
    width: 60px;
    height: 34px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    overflow: hidden;
    display: block;
    background: var(--color-bg-1);
  }
  .scene-row__body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }
  .scene-row__name {
    color: var(--color-text-0);
    font-weight: var(--font-weight-label);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scene-row__meta {
    display: flex;
    gap: var(--space-sm);
    align-items: center;
    color: var(--color-text-3);
    font-size: var(--font-size-caps-sm);
  }
  .scene-row__bpm {
    color: var(--color-primary);
  }
  .scene-row__tag {
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    background: var(--color-bg-3);
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .scene-row__tag[data-tag='hot'] {
    color: var(--color-signal-orange);
    background: color-mix(in oklab, var(--color-signal-orange) 18%, transparent);
  }
  .scene-row__tag[data-tag='caution'] {
    color: var(--color-signal-amber);
    background: color-mix(in oklab, var(--color-signal-amber) 18%, transparent);
  }
  .scene-row__tail {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    color: var(--color-text-4);
  }
  .scene-row__pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
    box-shadow: 0 0 calc(6px * var(--glow, 1)) var(--color-primary);
  }
  .mono {
    font-family: var(--font-family-mono);
  }
</style>
