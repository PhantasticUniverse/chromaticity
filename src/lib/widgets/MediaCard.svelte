<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Shared-primitive backfill — `MediaCard` from
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 251-264. Bundle-faithful media tile: 16/10 aspect-ratio
  // surface that switches its surface treatment by `kind`:
  //   - `none`    — disabled / empty slot ("⊘" glyph, muted)
  //   - `webcam`  — live-source proxy (camera glyph on dark gradient)
  //   - `video`   — procedural thumbnail via the W36 `SceneStill`
  //   - `test`    — SMPTE colour-bar calibration pattern
  //
  // The Perform view's media strip currently inlines this branching
  // in a `{#snippet}`; this primitive moves the markup into the
  // widget barrel so the bundle's *other* MediaCard adoption sites
  // (Library media tab, future media-picker dialogs) can reuse it.
  //
  // The bundle's video tile draws a `<Icon name="play" size={8} />`
  // overlay glyph and a duration string; we mirror both. The W21 PR2
  // 'test' kind was added by chromaticity (no bundle precedent — the
  // bundle lacks SMPTE bars) and is preserved as a localized variant
  // so the existing Perform integration keeps working.
  import Icon from '../Icon.svelte';
  import type { SceneKind } from './scene-renderers';
  import SceneStill from './SceneStill.svelte';

  type MediaKind = 'none' | 'webcam' | 'video' | 'test';

  type Props = {
    /** Display label rendered in the bottom-left chip. */
    label: string;
    /** Surface treatment. Default `video`. */
    kind?: MediaKind;
    /** Per-tile variation seed for the SceneStill renderer (video only). */
    seed?: number;
    /** SceneKind for the SceneStill renderer (video only). */
    sceneKind?: SceneKind;
    /** Whether the card is the currently-selected media source. */
    active?: boolean;
    /** Display duration string (e.g. "0:23"). Rendered on `video` only. */
    dur?: string;
    /** Click handler. */
    onClick?: () => void;
  };

  let {
    label,
    kind = 'video',
    seed = 1,
    sceneKind = 'rings',
    active = false,
    dur,
    onClick,
  }: Props = $props();
</script>

<button
  type="button"
  class="media-card"
  class:active
  data-kind={kind}
  title={label}
  aria-label={label}
  aria-pressed={active ? 'true' : 'false'}
  onclick={onClick}
>
  {#if kind === 'none'}
    <span class="media-card__none" aria-hidden="true">
      <Icon name="x" size={24} />
    </span>
  {:else if kind === 'webcam'}
    <span class="media-card__webcam" aria-hidden="true">
      <Icon name="camera" size={22} />
    </span>
  {:else if kind === 'test'}
    <span class="media-card__test" aria-hidden="true">
      <span class="media-card__test-bar" style="background:#ffffff"></span>
      <span class="media-card__test-bar" style="background:#ffff00"></span>
      <span class="media-card__test-bar" style="background:#00ffff"></span>
      <span class="media-card__test-bar" style="background:#00ff00"></span>
      <span class="media-card__test-bar" style="background:#ff00ff"></span>
      <span class="media-card__test-bar" style="background:#ff0000"></span>
      <span class="media-card__test-bar" style="background:#0000ff"></span>
      <span class="media-card__test-bar" style="background:#000000"></span>
    </span>
  {:else}
    <span class="media-card__thumb" aria-hidden="true">
      <SceneStill kind={sceneKind} {seed} width={160} height={100} />
    </span>
  {/if}

  {#if kind === 'video'}
    <span class="media-card__kind" aria-hidden="true">
      <Icon name="play" size={8} />
    </span>
    {#if dur}
      <span class="media-card__dur mono">{dur}</span>
    {/if}
  {/if}

  <span class="media-card__label mono">{label}</span>
</button>

<style>
  .media-card {
    display: block;
    width: 100%;
    aspect-ratio: 16 / 10;
    min-height: 68px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    position: relative;
    cursor: pointer;
    padding: 0;
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease;
  }
  .media-card:hover {
    border-color: var(--color-primary);
  }
  .media-card.active {
    border-color: var(--color-primary);
    box-shadow:
      0 0 0 1px var(--color-primary),
      0 0 calc(12px * var(--glow, 1)) var(--color-primary);
  }
  .media-card:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .media-card__none {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--color-text-4);
  }
  .media-card__webcam {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, var(--color-bg-1), var(--color-bg-0));
    color: var(--color-text-2);
  }
  .media-card__test {
    position: absolute;
    inset: 0;
    display: flex;
  }
  .media-card__test-bar {
    flex: 1 1 0;
    height: 100%;
  }
  .media-card__thumb {
    position: absolute;
    inset: 0;
    display: block;
  }
  .media-card__kind {
    position: absolute;
    left: 4px;
    top: 4px;
    background: rgba(0, 0, 0, 0.7);
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    color: #fff;
    font-family: var(--font-family-mono);
    font-size: 9px;
    line-height: 1;
    display: inline-flex;
    align-items: center;
  }
  .media-card__dur {
    position: absolute;
    right: 4px;
    top: 4px;
    background: rgba(0, 0, 0, 0.7);
    color: #fff;
    font-size: 9px;
    padding: 1px 4px;
    border-radius: var(--radius-xs);
  }
  /* Wave 2.3 Lane Q.2 (Niggle B follow-up) — bottom-left label chip.
   * Opaque background + bold weight + drop shadow so the label
   * reads cleanly against bright SceneStill content (the bundle's
   * fluid / rings / glitch renderers all reach high saturation at
   * tile centre). Z-index lifts the chip above any sibling
   * pseudo-elements. */
  .media-card__label {
    position: absolute;
    left: 4px;
    bottom: 4px;
    right: 4px;
    background: rgba(0, 0, 0, 0.78);
    color: #fff;
    font-size: 9px;
    font-weight: 500;
    padding: 2px 4px;
    border-radius: var(--radius-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    z-index: 2;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4);
  }
  .mono {
    font-family: var(--font-family-mono);
  }
</style>
