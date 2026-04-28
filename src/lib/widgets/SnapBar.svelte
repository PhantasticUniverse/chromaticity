<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-27 PR1 widget primitive: 8-slot Snap quick-access bar. Bundle
  // source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/extras.jsx`
  // lines 14-46 (`SnapBar` component). Mounted in `AppShell.svelte`
  // between the TopBar and the routed view, gated on
  // `routeStore.current === 'perform'` per the bundle's
  // `Chromaticity.html` adoption site (`{showSnapBar && <SnapBar … />}`,
  // where `showSnapBar = view === "perform"`).
  //
  // Frozen-frame contract: clicking a slot only updates local UI state;
  // the "Save current state as a Snap" `+` slot is a disabled visual
  // stub. Real snap recall / save is the engine `SnapSlotId` IPC surface
  // already implemented at `src/lib/snap.ts` — that module's domain is
  // sixteen engine slots indexed `0..15`, a different domain from this
  // bar's eight named perform-time bookmarks. The deferred wiring
  // (mapping a Perform-bar slot onto an engine `SnapSlotId`, plus the
  // save / clear lifecycle) is `EV-INTEGRATION-MATRIX` scope.
  //
  // Per-slot thumbnails: W36 PR1 wires the `SceneStill` widget primitive
  // (`./SceneStill.svelte`) to render bundle-faithful procedural canvas
  // thumbnails per slot, replacing the W27 PR1 hue-rotating gradient
  // surrogate. The `kind` + `seed` mapping is fixture data on the
  // `SnapSlot` type; see `snap-bar.fixtures.ts` for the per-slot
  // assignments mirroring the bundle's `_data.jsx` scene → renderer
  // mapping. The dark-gradient `::after` overlay is preserved for
  // meta-text readability over the canvas content.
  //
  // `SnapSlot` and `DEFAULT_SNAP_SLOTS` live in `snap-bar.fixtures.ts`
  // because Svelte's `*.svelte` TS shim only re-exports the default —
  // named exports from `<script module>` are unreachable to consumers,
  // same constraint that put `CuePadCell` in `widgets/index.ts`.
  import Icon from '../Icon.svelte';
  import SceneStill from './SceneStill.svelte';
  import { DEFAULT_SNAP_SLOTS, type SnapSlot } from './snap-bar.fixtures';

  let {
    slots = DEFAULT_SNAP_SLOTS,
    active = 1,
    onPick,
  }: {
    slots?: readonly SnapSlot[];
    active?: number;
    onPick?: (slot: number) => void;
  } = $props();
</script>

<div class="snap-bar" aria-label="Snap slots">
  <div class="snap-bar__label">Snap</div>
  {#each slots as slot (slot.i)}
    <button
      type="button"
      class="snap-slot"
      class:snap-slot--on={active === slot.i}
      style="--tint: {slot.tint};"
      onclick={() => onPick?.(slot.i)}
      title={`${slot.name} · ${slot.scene}  (press ${slot.i})`}
    >
      <span class="snap-slot__thumb" aria-hidden="true">
        <SceneStill kind={slot.kind} seed={slot.seed} width={100} height={44} />
      </span>
      <span class="snap-slot__meta">
        <span class="snap-slot__num">{slot.i}</span>
        <span class="snap-slot__name">{slot.name}</span>
      </span>
    </button>
  {/each}
  <button
    type="button"
    class="snap-slot snap-slot--add"
    title="Save current state as a Snap"
    disabled
    aria-label="Save current state as a Snap"
  >
    <Icon name="plus" size={14} />
  </button>
</div>

<style>
  .snap-bar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-lg);
    background: var(--color-bg-1);
    border-top: 1px solid var(--color-line-1);
    border-bottom: 1px solid var(--color-line-1);
    min-height: 56px;
  }
  .snap-bar__label {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    align-self: stretch;
    display: flex;
    align-items: center;
    padding: 0 var(--space-unit);
    margin-right: var(--space-unit);
    border-right: 1px solid var(--color-line-1);
    color: var(--color-text-3);
    font-size: var(--font-size-caps-sm);
    font-weight: var(--font-weight-label);
    letter-spacing: 0.2em;
    text-transform: uppercase;
  }
  .snap-slot {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    width: 100px;
    height: 44px;
    padding: 0;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
    cursor: pointer;
    transition:
      border-color 0.1s,
      transform 0.1s;
  }
  .snap-slot:hover {
    border-color: var(--tint, var(--color-primary));
  }
  .snap-slot--on {
    border-color: var(--tint, var(--color-primary));
    box-shadow: inset 0 0 0 1px var(--tint, var(--color-primary));
  }
  .snap-slot__thumb {
    flex: 1;
    position: relative;
    background: var(--color-bg-3); /* solid fallback before the SceneStill canvas paints */
    overflow: hidden;
  }
  .snap-slot__thumb::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, transparent 40%, rgba(0, 0, 0, 0.6) 100%);
    pointer-events: none;
  }
  .snap-slot__meta {
    position: absolute;
    left: var(--space-xs);
    right: var(--space-xs);
    bottom: var(--space-unit);
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    z-index: 1;
  }
  .snap-slot__num {
    color: var(--tint, var(--color-primary));
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    font-weight: var(--font-weight-display);
    letter-spacing: 0.04em;
  }
  .snap-slot__name {
    color: var(--color-text-0);
    font-size: var(--font-size-caps-sm);
    letter-spacing: 0.02em;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .snap-slot--on .snap-slot__name {
    font-weight: var(--font-weight-display);
  }
  .snap-slot--add {
    width: 34px;
    align-items: center;
    justify-content: center;
    background: transparent;
    border-style: dashed;
    color: var(--color-text-3);
  }
  .snap-slot--add:disabled {
    cursor: not-allowed;
  }
  .snap-slot--add:hover:not(:disabled) {
    color: var(--color-primary);
    border-color: var(--color-primary);
  }
</style>
