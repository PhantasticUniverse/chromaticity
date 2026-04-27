<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-10 view: scene Inputs Inspector. Read-only port of the
  // chromaticity-planning bundle's chroma_inspector.jsx — controls grouped
  // by `group` field with category-tint accents on the group rail.
  // Wave-32 closes the placeholder gap: the W8 mappable primitives
  // (Fader / Knob / Toggle / Swatch) render for slider / knob / toggle /
  // color, and the new W32 primitives (Xy / Bang / Dropdown) cover the
  // remaining bundle control types per `chroma_inspector.jsx::
  // ChromaCtrlBody` lines 138-197. Wave-33c adds the bundle's inspector
  // toolbar — caseless substring search across `name` / `description` /
  // `type` plus a group-tab radiogroup that filters down to one group
  // at a time — per `chroma_inspector.jsx` lines 262-420 (the
  // `ChromaInspector` component's `query` / `group` state + the
  // toolbar at lines 374-386). Bundle profile / format-tag badges,
  // MIDI learn / lock affordances, and coercion-applied chips are
  // separate Inputs-expansion B-E waves that depend on the live
  // capabilities-store profile bit + the MIDI adapter + the scene-
  // validator diagnostic surface respectively. Fixtures are hardcoded
  // in `src/lib/data/scene-fixtures.ts`; W11+ IPC-fed loader will
  // replace them without changing this view's rendering contract.
  import type { ChromaSceneFixture, ChromaControl } from '../data/scene-fixtures';
  import { Bang, Dropdown, Fader, Knob, Swatch, Toggle, Xy } from '../widgets';

  let { scene }: { scene: ChromaSceneFixture } = $props();

  // Wave-33c filter state. `searchQuery` is a caseless substring
  // matched against control name / description / type per the bundle's
  // `q` filter at `chroma_inspector.jsx` lines 300-309. `activeGroup`
  // is `'all'` (no filter) or one of the scene's group names.
  let searchQuery = $state('');
  let activeGroup = $state<string>('all');

  // Group controls by `group` field, preserving first-seen order so the
  // group rail mirrors scene.json author intent. Plain `[name, controls]`
  // tuple list — using a Map here trips `svelte/prefer-svelte-reactivity`
  // even though the value is fully derived.
  const grouped = $derived.by(() => {
    const order: string[] = [];
    const buckets: Record<string, ChromaControl[]> = {};
    for (const c of scene.controls) {
      const g = c.group ?? 'general';
      const bucket = buckets[g];
      if (bucket === undefined) {
        order.push(g);
        buckets[g] = [c];
      } else {
        bucket.push(c);
      }
    }
    return order.map((g): [string, ChromaControl[]] => [g, buckets[g] ?? []]);
  });

  /**
   * Wave-33c group tab vocabulary: `'all'` plus each scene group in
   * first-seen order. Mirrors the bundle's `groupOrder` at
   * `chroma_inspector.jsx` line 298.
   */
  const groupTabs = $derived(['all', ...grouped.map(([g]) => g)] as const);

  /**
   * Apply the search + active-group filter against `grouped` per the
   * bundle's `visible` derivation at `chroma_inspector.jsx` lines 301-
   * 309. Returns the same `[groupName, controls[]]` tuple shape so the
   * existing render loop is untouched. Empty bucket groups are dropped
   * so the empty-state below renders only when ZERO groups survive.
   */
  const filtered = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    return grouped
      .map(([group, controls]): [string, ChromaControl[]] => {
        if (activeGroup !== 'all' && group !== activeGroup) return [group, []];
        if (!q) return [group, controls];
        const matched = controls.filter(
          (c) =>
            c.name.toLowerCase().includes(q) ||
            (c.description?.toLowerCase().includes(q) ?? false) ||
            c.type.toLowerCase().includes(q),
        );
        return [group, matched];
      })
      .filter(([, controls]) => controls.length > 0);
  });

  /** Total visible-control count for the bundle's "N/M" tally. */
  const totalControls = $derived(scene.controls.length);
  const visibleCount = $derived(filtered.reduce((sum, [, c]) => sum + c.length, 0));

  // Map group name to a `--color-cat-*` token. `tokens.css` exposes
  // color / motion / fx / media / limit / geom / general — `geometry`
  // resolves to `geom` (token spelling). Unknown groups fall through
  // to general.
  const KNOWN_GROUPS: Readonly<Record<string, string>> = {
    color: 'color',
    motion: 'motion',
    fx: 'fx',
    media: 'media',
    limit: 'limit',
    geometry: 'geom',
    geom: 'geom',
    general: 'general',
  };

  function categoryTintToken(group: string): string {
    const slug = group.toLowerCase();
    const key = KNOWN_GROUPS[slug] ?? 'general';
    return `var(--color-cat-${key})`;
  }
</script>

<section class="inputs">
  <header class="inputs__header">
    <h2>{scene.title}</h2>
    <span class="inputs__credit">{scene.credit}</span>
    <span class="inputs__format-tag" data-tag={scene.format_tag}>{scene.format_tag}</span>
  </header>

  <div class="inputs__toolbar">
    <label class="inputs__search">
      <span class="inputs__search-icon" aria-hidden="true">⌕</span>
      <input
        type="search"
        placeholder="filter · uniform name or type…"
        bind:value={searchQuery}
        aria-label="Filter inputs by name, description, or type"
      />
      {#if searchQuery}
        <button
          type="button"
          class="inputs__search-clear"
          aria-label="Clear search"
          onclick={() => (searchQuery = '')}
        >
          ✕
        </button>
      {/if}
    </label>
    <span class="inputs__count mono" aria-live="polite">{visibleCount}/{totalControls}</span>
    <div class="inputs__group-tabs" role="radiogroup" aria-label="Filter by group">
      {#each groupTabs as tab (tab)}
        <button
          type="button"
          class="inputs__group-tab"
          class:inputs__group-tab--active={activeGroup === tab}
          role="radio"
          aria-checked={activeGroup === tab}
          onclick={() => (activeGroup = tab)}
        >
          {tab}
        </button>
      {/each}
    </div>
  </div>

  {#if filtered.length === 0}
    <div class="inputs__empty mono" role="status">
      no controls match
      {#if searchQuery}<span>"{searchQuery}"</span>{/if}
    </div>
  {/if}

  {#each filtered as [group, controls] (group)}
    <div class="inputs__group" style:--group-tint={categoryTintToken(group)}>
      <h3 class="inputs__group-name">{group}</h3>
      <div class="inputs__group-body">
        {#each controls as control (control.name)}
          <div class="inputs__row">
            <span class="inputs__row-name">{control.label ?? control.name}</span>
            <span class="inputs__row-type" data-type={control.type}>{control.type}</span>
            <div class="inputs__row-widget">
              {#if control.type === 'slider'}
                <Fader value={typeof control.default === 'number' ? control.default : 0} />
              {:else if control.type === 'knob'}
                <Knob value={typeof control.default === 'number' ? control.default : 0} showValue />
              {:else if control.type === 'toggle'}
                <Toggle value={control.default === true} />
              {:else if control.type === 'color'}
                <Swatch color={typeof control.default === 'string' ? control.default : '#888'} />
              {:else if control.type === 'xy'}
                <Xy
                  value={Array.isArray(control.default) && control.default.length >= 2
                    ? [control.default[0] as number, control.default[1] as number]
                    : [0, 0]}
                />
              {:else if control.type === 'bang'}
                <Bang />
              {:else if control.type === 'dropdown'}
                <Dropdown
                  value={typeof control.default === 'number' ? control.default : 0}
                  labels={control.labels}
                />
              {/if}
            </div>
            {#if control.min !== undefined && control.max !== undefined}
              <span class="inputs__row-range">{control.min} → {control.max}</span>
            {:else}
              <span class="inputs__row-range" aria-hidden="true"></span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}
</section>

<style>
  .inputs {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .inputs__header {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .inputs__header h2 {
    margin: 0;
  }
  .inputs__credit {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .inputs__format-tag {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
  }

  /* ---------------- W33c toolbar (search + group tabs) ---------------- */

  .inputs__toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
  }
  .inputs__search {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    flex: 1 1 240px;
    min-width: 200px;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-xs);
  }
  .inputs__search-icon {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    flex-shrink: 0;
  }
  .inputs__search input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    padding: var(--space-xs) 0;
    min-width: 0;
  }
  .inputs__search input::placeholder {
    color: var(--color-text-3);
  }
  .inputs__search input:focus-visible {
    outline: none;
  }
  .inputs__search:focus-within {
    border-color: var(--color-primary);
  }
  .inputs__search-clear {
    background: transparent;
    border: none;
    color: var(--color-text-2);
    cursor: pointer;
    font-size: var(--font-size-body-sm);
    padding: 2px var(--space-xs);
    border-radius: var(--radius-xs);
  }
  .inputs__search-clear:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }
  .inputs__count {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    flex-shrink: 0;
    min-width: 4ch;
    text-align: right;
  }
  .inputs__group-tabs {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .inputs__group-tab {
    background: var(--color-bg-1);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-xs);
    padding: 3px var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    text-transform: lowercase;
  }
  .inputs__group-tab:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }
  .inputs__group-tab--active {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border-color: var(--color-line-3);
  }
  .inputs__group-tab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* ---------------- W33c empty-state when filter clears all groups ---- */

  .inputs__empty {
    padding: var(--space-md);
    color: var(--color-text-2);
    background: var(--color-bg-2);
    border: 1px dashed var(--color-line-2);
    border-radius: var(--radius-md);
    text-align: center;
    font-size: var(--font-size-body-sm);
  }
  .inputs__empty span {
    color: var(--color-text-1);
  }

  .inputs__group {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-left: 3px solid var(--group-tint);
    border-radius: var(--radius-md);
    padding: var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .inputs__group-name {
    margin: 0;
    color: var(--group-tint);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .inputs__group-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .inputs__row {
    display: grid;
    grid-template-columns: minmax(120px, 1fr) auto auto auto;
    align-items: center;
    gap: var(--space-md);
  }
  .inputs__row-name {
    color: var(--color-text-0);
  }
  .inputs__row-type {
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
  }
  .inputs__row-widget {
    display: flex;
    align-items: center;
  }
  .inputs__row-range {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    font-family: var(--font-family-mono);
    min-width: 6ch;
    text-align: right;
  }
</style>
