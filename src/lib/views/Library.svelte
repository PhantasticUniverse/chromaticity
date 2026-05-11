<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-9 view: scene library grid. Establishes the `src/lib/views/`
  // directory pattern and consumes the W9 PR-112 IPC backbone
  // (`list_scene_packages` + `default_scene_root`) via the per-root cache
  // in `src/lib/stores/library.ts`. When the default scene root yields no
  // `.chroma` packages — common on a fresh build host — the EMPTY_LIBRARY
  // copy bundle renders the Wave-8 EmptyState primitive with disabled
  // action buttons; W9 PR3 wires those handlers. Thumbnail tiles render a
  // placeholder disc per `EV-LIBRARY-THUMBNAIL` (Phase-4b). Search input,
  // 8 hardcoded category chips, and a small/medium/large size selector
  // realize `EV-LIBRARY-DISCOVERY` row 40 (default-root + hardcoded-
  // categories scope; dynamic vocabulary is gated on docs/38 §4 catalog
  // promotion).
  import { SvelteSet } from 'svelte/reactivity';

  import { EMPTY_LIBRARY } from '../data/empty-states';
  import { SETLIST_ACTIVE_INDEX, SETLIST_FIXTURE, SETLIST_SCENES } from '../data/setlist-fixtures';
  import EmptyState from '../EmptyState.svelte';
  import ErrorMessage from '../ErrorMessage.svelte';
  import Icon from '../Icon.svelte';
  import type { SceneCard } from '../ipc/bindings';
  import NewSceneModal from '../NewSceneModal.svelte';
  import {
    clearCache,
    fetchBundledSceneCards,
    fetchSceneCards,
    getDefaultRoot,
  } from '../stores/library';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import { routeStore } from '../stores/route.svelte';
  import { toIpcError } from '../types/ipc-error';

  let cards = $state<SceneCard[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // W22 PR3 header CTAs. `refreshing` drives the refresh-icon spin while
  // a re-scan is in flight; `newSceneOpen` toggles the frozen-frame
  // modal. The marketplace CTA is disabled in browser-mode where
  // `window.__TAURI_INTERNALS__` is absent — the same signal
  // `toIpcError` keys off in `types/ipc-error.ts`.
  let refreshing = $state(false);
  let newSceneOpen = $state(false);
  const tauriAvailable =
    typeof window !== 'undefined' &&
    Object.prototype.hasOwnProperty.call(window, '__TAURI_INTERNALS__');

  const CATEGORIES = [
    'BUILT-IN',
    'ABSTRACT',
    'GEOMETRIC',
    'ORGANIC',
    'GLITCH',
    '3D',
    'AUDIO REACTIVE',
    'TEXT',
  ] as const;
  // TODO: docs/38 §4 catalog promotion will replace this hardcoded list with
  // a curated base tag vocabulary loaded from the registry.

  let query = $state('');
  const activeCategories = new SvelteSet<string>();
  let size = $state<'small' | 'medium' | 'large'>('medium');

  const filteredCards = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return cards.filter((card) => {
      if (q) {
        const haystack = [card.title, card.credit, card.description, ...card.tags]
          .join(' ')
          .toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      if (activeCategories.size > 0) {
        const matches = card.categories.some((c) => activeCategories.has(c.toUpperCase()));
        if (!matches) return false;
      }
      return true;
    });
  });

  function toggleCategory(cat: string) {
    if (activeCategories.has(cat)) {
      activeCategories.delete(cat);
    } else {
      activeCategories.add(cat);
    }
  }

  // W11 PR2: merge bundled (ship-time) + user-staged scene cards.
  // `Promise.allSettled` is deliberate — bundled is more reliable
  // than user (the user root may not exist on a fresh install), so
  // a failed user-root fetch must not collapse the bundled corpus,
  // and vice versa. Extracted from the mount-time `$effect` so the
  // W22 PR3 Refresh CTA can re-invoke it.
  async function loadCards() {
    try {
      const [bundledResult, userResult] = await Promise.allSettled([
        fetchBundledSceneCards(),
        (async () => fetchSceneCards(await getDefaultRoot()))(),
      ]);

      const merged: SceneCard[] = [];
      const errors: string[] = [];

      if (bundledResult.status === 'fulfilled') {
        merged.push(...bundledResult.value);
      } else {
        const reason = bundledResult.reason;
        errors.push(reason instanceof Error ? reason.message : String(reason));
      }
      if (userResult.status === 'fulfilled') {
        merged.push(...userResult.value);
      } else {
        const reason = userResult.reason;
        errors.push(reason instanceof Error ? reason.message : String(reason));
      }

      cards = merged;
      // Only surface an error when both fetches failed — partial
      // results render as the merged grid.
      error = merged.length === 0 && errors.length > 0 ? errors.join('; ') : null;
    } catch (err) {
      error = toIpcError(err, 'list_scene_packages').message;
    }
  }

  $effect(() => {
    void (async () => {
      try {
        await loadCards();
      } finally {
        loading = false;
      }
    })();
  });

  async function handleRefresh() {
    if (refreshing) return;
    refreshing = true;
    clearCache();
    try {
      await loadCards();
    } finally {
      refreshing = false;
    }
  }

  function handleOpenNewScene() {
    newSceneOpen = true;
  }

  function handleCloseNewScene() {
    newSceneOpen = false;
  }

  function handleCreateNewScene(name: string) {
    // Frozen-frame: real scene creation is Phase-3+. Emit a console
    // line so the W21 console drawer's future log subscription has a
    // breadcrumb to render, then close the modal.
    console.info(
      `TODO: create new scene "${name || 'untitled scene'}" — Phase-3+ EV-AUTHORING-SCAFFOLDER scope`,
    );
    newSceneOpen = false;
  }

  function handleGetMoreScenes() {
    console.info('TODO: external link to scene marketplace — Phase-3+');
  }

  // Wave 2.1 Lane A.1: double-click a scene card to load its `.chroma`
  // package into the runtime and route to the Perform view. The
  // engine-side `load_scene` IPC parses `scene.json` + stores the
  // result on the shell-side AppState; on success the Perform view's
  // Scene Controls header + control-panel surfaces re-render against
  // the loaded scene's control schema instead of the
  // `perform-fixtures.ts` placeholder. Errors bubble through
  // `loadedSceneStore.error` so the UI can surface a non-fatal
  // banner; the route navigation is gated on a successful load.
  async function handleSceneOpen(card: SceneCard): Promise<void> {
    const payload = await loadedSceneStore.load(card.path);
    if (payload) {
      routeStore.set('perform');
    }
  }

  function handleSceneKeydown(event: KeyboardEvent, card: SceneCard): void {
    // Mirror the bundle's keyboard-accessibility pattern (Enter +
    // Space activate). Wave 2.1 only wires Enter; Space-bar
    // activation is post-MVP UX.
    if (event.key === 'Enter') {
      event.preventDefault();
      void handleSceneOpen(card);
    }
  }
</script>

<div class="library-page">
  <section class="library">
    <header class="library__header">
      <h2>library</h2>
      {#if !loading && cards.length > 0}
        <span class="library__count">
          {filteredCards.length} of {cards.length} scene{cards.length === 1 ? '' : 's'}
        </span>
      {/if}
      <div class="library__ctas" role="group" aria-label="Library actions">
        <button
          type="button"
          class="library__cta primary"
          onclick={handleOpenNewScene}
          title="Create a new scene (stub)"
        >
          <Icon name="plus" size={12} />
          New Scene
        </button>
        <button
          type="button"
          class="library__cta"
          onclick={handleRefresh}
          aria-busy={refreshing}
          disabled={refreshing}
          title="Re-scan the bundled and user scene roots"
        >
          <span class="library__cta-icon" class:spinning={refreshing} aria-hidden="true">
            <Icon name="refresh" size={12} />
          </span>
          Refresh Index
        </button>
        <button
          type="button"
          class="library__cta"
          onclick={handleGetMoreScenes}
          disabled={!tauriAvailable}
          title={tauriAvailable
            ? 'Open the scene marketplace (stub)'
            : 'External-link CTA disabled in browser preview — available in the native shell'}
        >
          <Icon name="upload" size={12} />
          Get More Scenes
        </button>
      </div>
    </header>

    {#if !loading && cards.length > 0}
      <div class="library__toolbar">
        <input
          type="search"
          placeholder="search title, credit, tag…"
          bind:value={query}
          class="library__search"
          aria-label="filter scenes"
        />
        <div class="library__chips" role="group" aria-label="category filters">
          {#each CATEGORIES as cat (cat)}
            <button
              type="button"
              class="library__chip"
              class:active={activeCategories.has(cat)}
              aria-pressed={activeCategories.has(cat)}
              onclick={() => toggleCategory(cat)}
            >
              {cat}
            </button>
          {/each}
        </div>
        <label class="library__size">
          <span>size</span>
          <select bind:value={size}>
            <option value="small">small</option>
            <option value="medium">medium</option>
            <option value="large">large</option>
          </select>
        </label>
      </div>
    {/if}

    {#if loading}
      <p class="library__status">loading…</p>
    {:else if error}
      <ErrorMessage {error} />
    {:else if cards.length === 0}
      <EmptyState title={EMPTY_LIBRARY.title} body={EMPTY_LIBRARY.body} iconName="library">
        {#snippet actions()}
          <button type="button" disabled>{EMPTY_LIBRARY.primaryAction?.label ?? ''}</button>
          {#if EMPTY_LIBRARY.secondaryAction}
            <button type="button" disabled>{EMPTY_LIBRARY.secondaryAction.label}</button>
          {/if}
        {/snippet}
      </EmptyState>
    {:else}
      <!--
        TODO(scale): adopt @tanstack/svelte-virtual (or equivalent) when fixture
        cardinality exceeds ~200 cards. The library scene grid currently renders
        full DOM; the first 1000-scene QA build will need windowing to keep
        scroll perf acceptable. Pre-flag for the operator-cohort residency.
      -->
      <div class="library__grid" data-size={size}>
        {#each filteredCards as card (card.id)}
          <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
          <article
            class="scene-tile"
            class:scene-tile--active={loadedSceneStore.snapshot?.path === card.path}
            tabindex="0"
            role="button"
            aria-label={`Load scene ${card.title || card.id}`}
            ondblclick={() => void handleSceneOpen(card)}
            onkeydown={(e) => handleSceneKeydown(e, card)}
          >
            <div class="scene-tile__thumbnail" aria-hidden="true">
              <!-- Placeholder thumbnail; EV-LIBRARY-THUMBNAIL is Phase-4b. -->
              <div class="scene-tile__thumbnail-disc"></div>
            </div>
            <div class="scene-tile__meta">
              <div class="scene-tile__title-row">
                <h3 class="scene-tile__title">{card.title || card.id}</h3>
                {#if card.source === 'Bundled'}
                  <span class="scene-tile__builtin" aria-label="bundled scene">BUILT-IN</span>
                {/if}
              </div>
              {#if card.credit}
                <p class="scene-tile__credit">{card.credit}</p>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>

  <aside class="setlist" aria-label="Setlist">
    <header class="setlist__head">
      <span class="setlist__title">Your Setlist</span>
      <span class="setlist__sep" aria-hidden="true">·</span>
      <span class="setlist__count">{SETLIST_FIXTURE.length} scenes</span>
    </header>
    <ol class="setlist__list">
      {#each SETLIST_FIXTURE as scene (scene.idx)}
        <li
          class="setlist__row"
          data-tag={scene.tag}
          class:active={scene.idx === SETLIST_ACTIVE_INDEX}
        >
          <span class="setlist__thumb" aria-hidden="true" data-seed={scene.seed}></span>
          <span class="setlist__name">{scene.name}</span>
          <span class="setlist__tag" data-tag={scene.tag}>{scene.tag}</span>
          <span class="setlist__dur">{scene.duration}</span>
        </li>
      {/each}
    </ol>
    <p class="setlist__hint">
      Hardcoded fixture mirroring v0.2 bundle <code>views/_data.jsx</code> SCENES (12 scenes
      repeated to 14 cue slots; row {SETLIST_ACTIVE_INDEX} active per JSX). Real setlist persistence +
      cue-edit IPC is Phase-3 <code>EV-PRESET-PLAYLIST</code> scope.
      {SETLIST_SCENES.length} unique scenes in the fixture.
    </p>
  </aside>
</div>

<NewSceneModal open={newSceneOpen} onCreate={handleCreateNewScene} onClose={handleCloseNewScene} />

<style>
  /* Token-styled scoped CSS. Grid uses CSS directly; ControlGroup is a
   * widget-row primitive and is intentionally NOT reused here. */
  .library-page {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 380px);
    gap: var(--space-lg);
    align-items: start;
  }
  .library {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
  }
  .setlist {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-md);
    position: sticky;
    top: 0;
    max-height: calc(100vh - 200px);
    overflow: hidden;
  }
  .setlist__head {
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
    padding-bottom: var(--space-xs);
    border-bottom: 1px solid var(--color-line-1);
  }
  .setlist__title {
    font-size: var(--font-size-body-md);
    font-weight: 600;
    color: var(--color-text-0);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .setlist__sep {
    color: var(--color-text-3);
  }
  .setlist__count {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-family: var(--font-family-mono);
  }
  .setlist__list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .setlist__row {
    display: grid;
    grid-template-columns: 24px 1fr max-content max-content;
    gap: var(--space-xs);
    align-items: center;
    padding: var(--space-xs);
    border-radius: var(--radius-xs);
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    transition: background-color 100ms;
  }
  .setlist__row:hover {
    background: var(--color-bg-2);
  }
  .setlist__row.active {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    box-shadow: inset 2px 0 0 var(--color-primary);
  }
  .setlist__thumb {
    width: 18px;
    height: 18px;
    border-radius: var(--radius-xs);
    background: linear-gradient(135deg, var(--color-primary), var(--color-line-2));
    opacity: 0.7;
  }
  .setlist__row.active .setlist__thumb {
    opacity: 1;
  }
  .setlist__name {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .setlist__tag {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 10px;
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
  }
  .setlist__tag[data-tag='hot'] {
    color: var(--color-signal-amber);
  }
  .setlist__tag[data-tag='caution'] {
    color: var(--color-signal-red);
  }
  .setlist__dur {
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }
  .setlist__hint {
    color: var(--color-text-3);
    font-size: 10px;
    line-height: 1.4;
    margin: 0;
    padding-top: var(--space-xs);
    border-top: 1px solid var(--color-line-1);
  }
  .library__header {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .library__count {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .library__ctas {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    margin-left: auto;
  }
  .library__cta {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }
  .library__cta:hover:not(:disabled) {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .library__cta:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .library__cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .library__cta.primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .library__cta.primary:hover:not(:disabled) {
    filter: brightness(1.05);
  }
  .library__cta-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .library__cta-icon.spinning {
    animation: library-cta-spin 0.9s linear infinite;
  }
  @keyframes library-cta-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
  .library__status {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .library__toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-md);
    align-items: center;
  }
  .library__search {
    flex: 1 1 240px;
    min-width: 200px;
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-body-sm);
  }
  .library__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }
  .library__chip {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }
  .library__chip:hover {
    background: var(--color-bg-4);
  }
  .library__chip.active {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .library__size {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }
  .library__size select {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-xs);
  }
  .library__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-md);
  }
  .library__grid[data-size='small'] {
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  }
  .library__grid[data-size='medium'] {
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  }
  .library__grid[data-size='large'] {
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  }
  .scene-tile {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    box-shadow: var(--shadow-panel);
    cursor: pointer;
    transition:
      border-color 0.12s ease,
      box-shadow 0.12s ease;
  }
  .scene-tile:hover {
    border-color: var(--color-line-3, var(--color-text-3));
  }
  .scene-tile:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .scene-tile--active {
    border-color: var(--color-primary);
    box-shadow:
      var(--shadow-panel),
      inset 0 0 0 1px var(--color-primary);
  }
  .scene-tile__thumbnail {
    aspect-ratio: 16 / 10;
    background: var(--color-bg-3);
    border-radius: var(--radius-sm);
    display: grid;
    place-items: center;
  }
  .scene-tile__thumbnail-disc {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--color-bg-4);
    border: 1px solid var(--color-line-2);
  }
  .scene-tile__meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .scene-tile__title-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }
  .scene-tile__title {
    margin: 0;
    font-size: var(--font-size-body-md);
    color: var(--color-text-0);
  }
  .scene-tile__builtin {
    /* Reuses --color-cat-general; no new theme tokens needed. */
    background: var(--color-cat-general);
    color: var(--color-on-primary);
    border-radius: var(--radius-xs);
    padding: 1px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
  }
  .scene-tile__credit {
    margin: 0;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
</style>
