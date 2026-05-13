<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-27 PR1 widget primitive: 8-slot Snap quick-access bar. Bundle
  // source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/extras.jsx`
  // lines 14-46 (`SnapBar` component). Mounted in `AppShell.svelte`
  // between the routed view and the StatusBar, gated on
  // `routeStore.current === 'perform'` per the bundle's
  // `Chromaticity.html` adoption site.
  //
  // Wave 2.1 Lane A.3 wires the widget into the engine snap IPC surface:
  // plain click invokes `snapStore.recall(slot)` (engine `recall_snap`);
  // Cmd+click (macOS) / Ctrl+click (Windows / Linux) invokes
  // `snapStore.save(slot)` (engine `save_snap`); Alt+click invokes
  // `snapStore.clear(slot)` (engine `clear_snap`). Visual indicators
  // distinguish three states: empty (never saved → faint outline),
  // saved (`snap_slot_saved` returned true → filled tint border), and
  // active (recalled → bright accent border + box-shadow). Slot 1..8
  // here maps to engine slot 0..7 inside the store. Keyboard parity:
  // Enter / Space recall; Cmd+Enter / Ctrl+Enter save; Alt+Enter clear.
  //
  // The `active` and `onPick` props remain for test fixtures and the
  // `AppShell` adoption site: when `active` is supplied it overrides
  // the store-driven highlight; when `onPick` is supplied the click
  // handler defers to the caller rather than dispatching the IPC.
  // This preserves the W43 SnapBar test surface and the bundle's
  // composability while making the production-mounted widget call
  // engine snap IPC automatically.
  //
  // `SnapSlot` and `DEFAULT_SNAP_SLOTS` live in `snap-bar.fixtures.ts`
  // because Svelte's `*.svelte` TS shim only re-exports the default —
  // named exports from `<script module>` are unreachable to consumers,
  // same constraint that put `CuePadCell` in `widgets/index.ts`.
  //
  // Wave 2.4 Lane PERF.F adds right-click contextmenu inline-rename of
  // per-slot labels backed by `snapLabelsStore` (localStorage). Engine
  // snap state remains binary saved / unsaved — labels are frontend-
  // only until a future Settings TOML migration. The rendered label
  // prefers the operator override over the bundle fixture's
  // `slot.name`; falling back to the fixture name when no override is
  // set keeps the wireframe-faithful default visible on first launch.
  import { onMount, tick } from 'svelte';

  import Icon from '../Icon.svelte';
  import SceneStill from './SceneStill.svelte';
  import { DEFAULT_SNAP_SLOTS, type SnapSlot } from './snap-bar.fixtures';
  import { snapStore } from '../stores/snap.svelte';
  import { snapLabelsStore } from '../stores/snapLabels.svelte';

  let {
    slots = DEFAULT_SNAP_SLOTS,
    active,
    onPick,
  }: {
    slots?: readonly SnapSlot[];
    active?: number;
    onPick?: (slot: number) => void;
  } = $props();

  // Hydrate the store on mount so the saved-state mask reflects the
  // engine's truth (idempotent + swallows no-Tauri rejection).
  onMount(() => {
    void snapStore.refresh();
    snapLabelsStore.hydrate();
  });

  // Tracks which slot (if any) is currently in inline-rename mode.
  // `null` when no rename is active; otherwise the 1-indexed slot id
  // whose `.snap-slot__name` is replaced by the rename input.
  let renamingSlot: number | null = $state(null);
  let renameDraft: string = $state('');
  let renameInputEl: HTMLInputElement | null = $state(null);

  function labelFor(slot: SnapSlot): string {
    return snapLabelsStore.label(slot.i, slot.name);
  }

  async function startRename(slot: SnapSlot): Promise<void> {
    renamingSlot = slot.i;
    renameDraft = labelFor(slot);
    await tick();
    renameInputEl?.focus();
    renameInputEl?.select();
  }

  function commitRename(): void {
    if (renamingSlot === null) return;
    snapLabelsStore.set(renamingSlot, renameDraft);
    renamingSlot = null;
    renameDraft = '';
  }

  function cancelRename(): void {
    renamingSlot = null;
    renameDraft = '';
  }

  function handleContextMenu(event: MouseEvent, slot: SnapSlot): void {
    event.preventDefault();
    void startRename(slot);
  }

  function handleRenameKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      commitRename();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancelRename();
    } else if (event.key === ' ') {
      // Space is the SnapBar's recall hotkey on a focused slot; while
      // the rename input is open it should insert a literal space.
      event.stopPropagation();
    }
  }

  // The widget-facing active slot is the caller's `active` prop when
  // supplied (test fixtures, AppShell legacy prop), else the store's
  // `activeSlot` (production wiring).
  const activeSlot = $derived(active ?? snapStore.activeSlot);

  function isSaved(slot: number): boolean {
    const idx = slot - 1;
    return idx >= 0 && idx < snapStore.saved.length && snapStore.saved[idx] === true;
  }

  function handleClick(event: MouseEvent, slot: number): void {
    // Suppress recall / save / clear while an inline rename is open
    // for any slot — the rename input owns input focus and click
    // dispatch should not double-fire engine snap IPC.
    if (renamingSlot !== null) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (onPick) {
      onPick(slot);
      return;
    }
    if (event.altKey) {
      void snapStore.clear(slot);
    } else if (event.metaKey || event.ctrlKey) {
      void snapStore.save(slot);
    } else {
      void snapStore.recall(slot);
    }
  }

  function handleKeydown(event: KeyboardEvent, slot: number): void {
    if (renamingSlot !== null) {
      return;
    }
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }
    event.preventDefault();
    if (onPick) {
      onPick(slot);
      return;
    }
    if (event.altKey) {
      void snapStore.clear(slot);
    } else if (event.metaKey || event.ctrlKey) {
      void snapStore.save(slot);
    } else {
      void snapStore.recall(slot);
    }
  }
</script>

<div class="snap-bar" aria-label="Snap slots">
  <div class="snap-bar__label">Snap</div>
  {#each slots as slot (slot.i)}
    <button
      type="button"
      class="snap-slot"
      class:snap-slot--on={activeSlot === slot.i}
      class:snap-slot--saved={isSaved(slot.i)}
      class:snap-slot--empty={!isSaved(slot.i) && activeSlot !== slot.i}
      class:snap-slot--renaming={renamingSlot === slot.i}
      style="--tint: {slot.tint};"
      onclick={(event) => handleClick(event, slot.i)}
      onkeydown={(event) => handleKeydown(event, slot.i)}
      oncontextmenu={(event) => handleContextMenu(event, slot)}
      title={`${labelFor(slot)} · ${slot.scene}  (press ${slot.i}; Cmd-click to save, Alt-click to clear, right-click to rename)`}
      aria-pressed={activeSlot === slot.i}
    >
      <span class="snap-slot__thumb" aria-hidden="true">
        <SceneStill kind={slot.kind} seed={slot.seed} width={100} height={44} />
      </span>
      <span class="snap-slot__meta">
        <span class="snap-slot__num">{slot.i}</span>
        {#if renamingSlot === slot.i}
          <input
            class="snap-slot__rename"
            type="text"
            bind:value={renameDraft}
            bind:this={renameInputEl}
            onkeydown={handleRenameKeydown}
            onblur={commitRename}
            onclick={(event) => event.stopPropagation()}
            aria-label={`Rename Snap slot ${slot.i}`}
          />
        {:else}
          <span class="snap-slot__name">{labelFor(slot)}</span>
        {/if}
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
      transform 0.1s,
      opacity 0.1s;
  }
  .snap-slot:hover {
    border-color: var(--tint, var(--color-primary));
  }
  /* Empty slot: faint outline, dim thumbnail. The slot is still
   * clickable so a Cmd-click can save into it. */
  .snap-slot--empty {
    opacity: 0.55;
    border-style: dashed;
  }
  /* Saved (but not active) slot: full opacity, tint-coloured border. */
  .snap-slot--saved {
    opacity: 1;
    border-color: var(--tint, var(--color-primary));
  }
  /* Active slot: bright accent + inset shadow. Wins over `--saved`
   * and `--empty` per the source-order CSS precedence below. */
  .snap-slot--on {
    border-color: var(--tint, var(--color-primary));
    box-shadow: inset 0 0 0 1px var(--tint, var(--color-primary));
    opacity: 1;
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
  /* Inline rename input — sized to fit inside the slot's meta strip
   * without enlarging the slot. Matches `.snap-slot__name` typography. */
  .snap-slot__rename {
    flex: 1;
    min-width: 0;
    padding: 0 var(--space-xs);
    margin: 0;
    background: rgba(0, 0, 0, 0.65);
    border: 1px solid var(--tint, var(--color-primary));
    border-radius: var(--radius-xs, 2px);
    color: var(--color-text-0);
    font-family: inherit;
    font-size: var(--font-size-caps-sm);
    letter-spacing: 0.02em;
    line-height: 1.4;
    outline: none;
  }
  .snap-slot--renaming {
    border-color: var(--tint, var(--color-primary));
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
