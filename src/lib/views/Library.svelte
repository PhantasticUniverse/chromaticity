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
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';

  import { EMPTY_LIBRARY, EMPTY_LIBRARY_FILTERED } from '../data/empty-states';
  import EmptyState from '../EmptyState.svelte';
  import ErrorMessage from '../ErrorMessage.svelte';
  import Icon from '../Icon.svelte';
  import type { SceneCard } from '../ipc/bindings';
  import NewSceneModal from '../NewSceneModal.svelte';
  import {
    clearCache,
    fetchBundledSceneCards,
    fetchCustomSceneCards,
    fetchSceneCards,
    getDefaultRoot,
  } from '../stores/library';
  import { libraryFavoritesStore } from '../stores/libraryFavorites.svelte';
  import { libraryRecentStore } from '../stores/libraryRecent.svelte';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';
  import { routeStore } from '../stores/route.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import { toIpcError } from '../types/ipc-error';
  import {
    sceneKindFromMetadata,
    sceneSeedFromMetadata,
  } from '../widgets/scene-kind-from-metadata';
  import SceneCanvas from '../widgets/SceneCanvas.svelte';
  import SceneStill from '../widgets/SceneStill.svelte';

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

  // Wave 2.5 LIB.B — the FAVORITES chip lives alongside the category
  // chip row but is single-select (toggling it on excludes the
  // category narrowing axis since it's an orthogonal filter). It is
  // intentionally NOT in the CATEGORIES const because category chips
  // narrow by `card.categories` membership; the favorites chip
  // narrows by `libraryFavoritesStore.ids` membership.
  const FAVORITES_CHIP = 'FAVORITES';
  let favoritesActive = $state(false);

  // Maximum number of tag chips visible inline on a card; remaining
  // tags collapse into a "+N more" pill with the full list available
  // via the title-attribute tooltip.
  const TAG_CHIPS_VISIBLE = 3;

  let query = $state('');
  const activeCategories = new SvelteSet<string>();
  let size = $state<'small' | 'medium' | 'large'>('medium');

  // Wave 2.5 Lane LIB.C — source filter (panel head) + RECENT chip
  // (category row) + sort axis (toolbar).
  //
  // The bundle's panel-head `<select>` exposes `built-in / my scenes /
  // all` per `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/
  // library.jsx` line 25-29; we default to `all` so a fresh open
  // shows the merged corpus.
  //
  // RECENT is a virtual chip — not in the hardcoded CATEGORIES list —
  // because it filters against `libraryRecentStore.ids` instead of
  // the card's `categories` array.
  let sourceFilter = $state<'all' | 'built-in' | 'my-scenes'>('all');
  let sortMode = $state<'id' | 'title' | 'recent' | 'favorites'>('id');
  let recentChipActive = $state(false);

  // Hydrate persisted favorites + recent on mount. Both stores are
  // module-level singletons; the reads are idempotent and the favorites
  // set + recent list are localStorage-only surfaces (no engine seam).
  $effect(() => {
    libraryFavoritesStore.hydrate();
    libraryRecentStore.hydrate();
  });

  // Push every successful scene load into the recent store.
  // `loadedSceneStore.snapshot?.id` covers BOTH the Library view's own
  // double-click handler AND any external loader (the Perform
  // scene-list click-to-load wire shipped in Wave 2.4 Lane PERF.B), so
  // the recent list reflects every load path.
  $effect(() => {
    const id = loadedSceneStore.snapshot?.id;
    if (id) {
      // `push()` reads + writes `libraryRecentStore.ids`; without
      // `untrack` the write re-triggers this effect on the next
      // microtask, blowing the effect-update-depth budget. Tracking
      // the snapshot id (the meaningful trigger) is sufficient.
      untrack(() => libraryRecentStore.push(id));
    }
  });

  const filteredCards = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const recents = libraryRecentStore.ids;
    const filtered = cards.filter((card) => {
      if (q) {
        const haystack = [card.title, card.credit, card.description, ...card.tags]
          .join(' ')
          .toLowerCase();
        if (!haystack.includes(q)) return false;
      }
      if (sourceFilter === 'built-in' && card.source !== 'Bundled') return false;
      if (sourceFilter === 'my-scenes' && card.source !== 'User') return false;
      if (recentChipActive && !recents.includes(card.id)) return false;
      if (activeCategories.size > 0) {
        const matches = card.categories.some((c) => activeCategories.has(c.toUpperCase()));
        if (!matches) return false;
      }
      if (favoritesActive && !libraryFavoritesStore.isFavorite(card.id)) {
        return false;
      }
      return true;
    });

    const sorted = [...filtered];
    if (sortMode === 'title') {
      sorted.sort((a, b) =>
        (a.title || a.id).localeCompare(b.title || b.id, undefined, { sensitivity: 'base' }),
      );
    } else if (sortMode === 'recent') {
      // Recently loaded first (in the order `libraryRecentStore.ids`
      // lists them — most-recent-first), then everything else in
      // id-asc tie-break order. A plain Record-of-number lets the
      // ephemeral lookup avoid the `svelte/prefer-svelte-reactivity`
      // friction `new Map()` trips (mirrors Perform.svelte's
      // controlValues precedent at line 353).
      const recencyIndex: Record<string, number> = {};
      for (let i = 0; i < recents.length; i++) {
        const id = recents[i];
        if (id !== undefined) recencyIndex[id] = i;
      }
      sorted.sort((a, b) => {
        const aIdx = recencyIndex[a.id];
        const bIdx = recencyIndex[b.id];
        if (aIdx !== undefined && bIdx !== undefined) return aIdx - bIdx;
        if (aIdx !== undefined) return -1;
        if (bIdx !== undefined) return 1;
        return a.id.localeCompare(b.id);
      });
    } else if (sortMode === 'favorites') {
      // TODO(wave-2.5-lib-b): swap the no-op partition for
      // `libraryFavoritesStore.has(card.id)` once Lane LIB.B merges.
      // Until then `favorites` is a stub that preserves id-asc order.
      sorted.sort((a, b) => a.id.localeCompare(b.id));
    }
    // `id` sort is the default; the engine returns id-sorted already
    // so `sorted` is already in id order at this branch.
    return sorted;
  });

  function toggleCategory(cat: string) {
    if (activeCategories.has(cat)) {
      activeCategories.delete(cat);
    } else {
      activeCategories.add(cat);
    }
  }

  function toggleRecentChip() {
    recentChipActive = !recentChipActive;
  }

  function toggleFavoritesChip() {
    favoritesActive = !favoritesActive;
  }

  // Click handler for the heart affordance on a scene card. Click
  // toggles favorite membership; `stopPropagation` prevents the
  // surrounding card's dblclick / keydown card-load wire from firing
  // when the operator targets the heart specifically.
  function handleFavoriteToggle(event: MouseEvent, card: SceneCard): void {
    event.stopPropagation();
    libraryFavoritesStore.toggle(card.id);
  }

  // W11 PR2 / Wave 2.8 LIB.LOCAL: merge bundled (ship-time) + user-
  // staged scene cards + every custom scene folder the operator has
  // configured via Settings → Storage → Add Custom Scene Folder
  // (persisted as `settings.storage.scene_folders`).
  //
  // `Promise.allSettled` is deliberate — bundled is more reliable
  // than user-default (user root may not exist on fresh install) and
  // every custom folder is operator-supplied (may point at a removed
  // external drive). One failed source must not collapse the others;
  // an error surfaces only when EVERY source failed AND the merged
  // list is empty. Each custom folder is fetched in parallel rather
  // than serially so a slow/missing folder doesn't gate the others.
  //
  // `customRoots` is captured from `settingsStore.snapshot` at call
  // time — synchronous, no additional `await` barrier — so the
  // existing W11 PR2 test sequencing (which assumes
  // `list_scene_packages` is invoked as soon as `loadCards` runs)
  // stays valid. The mount-time `$effect` below kicks
  // `settingsStore.load()` in parallel and re-fires `loadCards()`
  // when the persisted `scene_folders` array hydrates, so the AO
  // scenario "drop content in scenes-local, restart, see them on
  // first Library open" still works without coupling loadCards to a
  // sync wait on settings.
  async function loadCards() {
    try {
      const customRoots = settingsStore.snapshot.storage.scene_folders;
      const sources: Array<Promise<SceneCard[]>> = [
        fetchBundledSceneCards(),
        (async () => fetchSceneCards(await getDefaultRoot()))(),
        ...customRoots.map((root) => fetchCustomSceneCards(root)),
      ];

      const settled = await Promise.allSettled(sources);

      const merged: SceneCard[] = [];
      const errors: string[] = [];
      // Dedup by id across all sources — a scene that lives in BOTH
      // the user-default root AND a custom folder must not double-
      // render in the grid. First-seen wins, matching the bundled-
      // vs-user precedent the engine `SceneIndex` already pins. A
      // plain `Record<string, true>` (not `Set`) avoids the
      // `svelte/prefer-svelte-reactivity` lint friction that a
      // builtin `Set` would trip; the dedup is purely transient and
      // does not need to be reactive (mirrors the same pattern at
      // `Perform.svelte` line 343 / `Library.svelte::filteredCards`
      // `recencyIndex` already pinned in this file).
      const seenIds: Record<string, true> = {};

      for (const result of settled) {
        if (result.status === 'fulfilled') {
          for (const card of result.value) {
            if (seenIds[card.id]) continue;
            seenIds[card.id] = true;
            merged.push(card);
          }
        } else {
          const reason = result.reason;
          errors.push(reason instanceof Error ? reason.message : String(reason));
        }
      }

      cards = merged;
      // Only surface an error when EVERY source failed (merged
      // empty + at least one error) — partial results render as
      // the merged grid.
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

  // Wave 2.8 LIB.LOCAL: kick `settingsStore.load()` so the persisted
  // `storage.scene_folders` array hydrates from the on-disk TOML
  // store. Idempotent — if Settings.svelte / AudioUniforms.svelte /
  // ThemeSwitcher already triggered `load()`, this is a no-op against
  // the cached promise. The hydration re-fires `loadCards` via the
  // `$effect` below.
  $effect(() => {
    void settingsStore.load();
  });

  // Re-fire `loadCards` whenever the persisted custom-scene-folders
  // array changes. Covers both (a) the initial settings hydration
  // after mount (the engine returns a populated `scene_folders` array
  // some microtasks after `loadCards` has already run with the empty
  // default), and (b) the operator-adds-folder-via-Settings flow
  // (the user navigates to Settings → Storage → Add Custom Scene
  // Folder, picks a path, then returns to Library). Tracks the
  // serialised array contents (not the array reference) so an
  // unrelated `settingsStore.update()` that returns the same folders
  // doesn't trigger an unnecessary re-scan.
  //
  // Sentinel `null` = "have not observed any key yet"; the first
  // observation (mount-time fire of this effect) seeds the sentinel
  // without re-firing loadCards (the initial-load `$effect` above
  // already covered it). Subsequent observations re-fire on value
  // change.
  let lastCustomRootsKey: string | null = $state(null);
  $effect(() => {
    const key = settingsStore.snapshot.storage.scene_folders.join('\0');
    if (lastCustomRootsKey === null) {
      lastCustomRootsKey = key;
      return;
    }
    if (key === lastCustomRootsKey) return;
    lastCustomRootsKey = key;
    clearCache();
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

  // Wave 2.1 Lane A.1 / 2.5 LIB.E: double-click a scene card to load
  // its `.chroma` package into the runtime and route to the Perform
  // view. The engine-side `load_scene` IPC parses `scene.json` +
  // stores the result on the shell-side AppState; on success the
  // Perform view's Scene Controls header + control-panel surfaces
  // re-render against the loaded scene's control schema instead of
  // the `perform-fixtures.ts` placeholder. Errors bubble through
  // `loadedSceneStore.error` so the UI can surface a non-fatal
  // banner; the route navigation is gated on a successful load (LIB.E
  // canonical "load → play" flow for pre-alpha smoke testing).
  //
  // `loadingPath` tracks the in-flight card so the tile can render a
  // dimmed `aria-busy="true"` affordance — `loadedSceneStore.loading`
  // is global and would dim every tile, which the reviewer flagged as
  // confusing on a multi-tile grid.
  let loadingPath = $state<string | null>(null);

  async function handleSceneOpen(card: SceneCard): Promise<void> {
    if (loadingPath !== null) return;
    loadingPath = card.path;
    try {
      const payload = await loadedSceneStore.load(card.path);
      if (payload) {
        routeStore.set('perform');
      }
    } finally {
      loadingPath = null;
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

  // Wave 2.5 Lane LIB.A — right-rail preview + metadata + author
  // handoff. Replaces the W9 hardcoded "Your Setlist" sidebar (a
  // carry-over from the W9 backbone that doesn't belong in the
  // canonical Library wireframe per the v0.2-beta bundle
  // `views/library.jsx`).
  //
  // Selection model:
  //   - `selectedId` holds a single-click selection.
  //   - The active card is derived: if a scene is loaded, that wins
  //     (so the rail reflects what the runtime is currently bound
  //     to); otherwise the user's selection wins; otherwise the
  //     first card in the filtered list.
  //   - Single-click on a tile updates `selectedId`; double-click
  //     still triggers `handleSceneOpen` per Wave 2.1 Lane A.1.
  let selectedId = $state<string | null>(null);

  const selectedCard = $derived.by<SceneCard | null>(() => {
    if (filteredCards.length === 0) return null;
    const loadedId = loadedSceneStore.snapshot?.id;
    if (loadedId) {
      const match = cards.find((c) => c.id === loadedId);
      if (match) return match;
    }
    if (selectedId) {
      const match = filteredCards.find((c) => c.id === selectedId);
      if (match) return match;
    }
    return filteredCards[0] ?? null;
  });

  function handleSceneSelect(card: SceneCard): void {
    selectedId = card.id;
  }

  async function handleLoadSelected(): Promise<void> {
    if (!selectedCard) return;
    await handleSceneOpen(selectedCard);
  }

  function handleOpenInAuthor(): void {
    // The Author view reads `loadedSceneStore.snapshot` for its
    // editor binding; clicking "Open in Author" without a loaded
    // scene first kicks a `load_scene` IPC and only routes on
    // success, mirroring the double-click handoff semantics.
    if (!selectedCard) return;
    const loadedId = loadedSceneStore.snapshot?.id;
    if (loadedId === selectedCard.id) {
      routeStore.set('author');
      return;
    }
    void (async () => {
      const payload = await loadedSceneStore.load(selectedCard.path);
      if (payload) {
        routeStore.set('author');
      }
    })();
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
      <label class="library__source">
        <span>source</span>
        <select bind:value={sourceFilter} aria-label="filter by source">
          <option value="all">all</option>
          <option value="built-in">built-in</option>
          <option value="my-scenes">my scenes</option>
        </select>
      </label>
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
          <button
            type="button"
            class="library__chip library__chip--favorites"
            class:active={favoritesActive}
            aria-pressed={favoritesActive}
            onclick={toggleFavoritesChip}
            title="Show only favorited scenes"
          >
            {FAVORITES_CHIP}
          </button>
          <button
            type="button"
            class="library__chip"
            class:active={recentChipActive}
            aria-pressed={recentChipActive}
            onclick={toggleRecentChip}
            title="Show only recently loaded scenes"
          >
            RECENT
          </button>
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
        <label class="library__sort">
          <span>sort</span>
          <select bind:value={sortMode} aria-label="sort scenes">
            <option value="id">id</option>
            <option value="title">title</option>
            <option value="recent">recent</option>
            <option value="favorites">favorites</option>
          </select>
        </label>
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
      <EmptyState
        title={EMPTY_LIBRARY.title}
        body={EMPTY_LIBRARY.body}
        iconName="library"
        meta={EMPTY_LIBRARY.meta}
      >
        {#snippet actions()}
          <button type="button" disabled>{EMPTY_LIBRARY.primaryAction?.label ?? ''}</button>
          {#if EMPTY_LIBRARY.secondaryAction}
            <button type="button" disabled>{EMPTY_LIBRARY.secondaryAction.label}</button>
          {/if}
        {/snippet}
      </EmptyState>
    {:else if filteredCards.length === 0}
      <!-- Wave 2.5 LIB.E — library has scenes but the active search
           query / category chip combination excludes them all. Show a
           lighter empty state (no disabled action buttons since the
           toolbar above is the actionable surface — clear the query
           or chips). Supported-formats meta line ports per the
           bundle's empty-state surface. -->
      <EmptyState
        title={EMPTY_LIBRARY_FILTERED.title}
        body={EMPTY_LIBRARY_FILTERED.body}
        iconName="library"
        meta={EMPTY_LIBRARY_FILTERED.meta}
      />
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
            class:scene-tile--loading={loadingPath === card.path}
            class:scene-tile--selected={selectedCard?.id === card.id &&
              loadedSceneStore.snapshot?.path !== card.path}
            tabindex="0"
            role="button"
            aria-label={`Select scene ${card.title || card.id}`}
            aria-pressed={selectedCard?.id === card.id}
            aria-busy={loadingPath === card.path ? 'true' : undefined}
            onclick={() => handleSceneSelect(card)}
            ondblclick={() => void handleSceneOpen(card)}
            onkeydown={(e) => handleSceneKeydown(e, card)}
          >
            <div class="scene-tile__thumbnail">
              <!-- Wave 2.2 Lane R.1 — `<SceneStill>` procedural
                   thumbnail keyed off the card's metadata via the
                   `sceneKindFromMetadata` heuristic. EV-LIBRARY-
                   THUMBNAIL's deterministic-byte-hash stub renderer
                   stays the engine-side contract; this is the UI
                   visual surface, not the engine's rasterised
                   thumbnail. -->
              <SceneStill
                kind={sceneKindFromMetadata(card)}
                seed={sceneSeedFromMetadata(card)}
                width={220}
                height={138}
              />
              <!-- Wave 2.5 LIB.B — tag chips overlaid on the
                   thumbnail (top-left), heart-toggle overlaid on the
                   top-right. Bundle precedent:
                   `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
                   `FxTile`. The heart click stops propagation so it
                   doesn't fire the card's dblclick/Enter load
                   handlers. -->
              {#if card.tags.length > 0}
                <div
                  class="scene-tile__tags"
                  title={card.tags.join(', ')}
                  aria-label={`Tags: ${card.tags.join(', ')}`}
                >
                  {#each card.tags.slice(0, TAG_CHIPS_VISIBLE) as tag (tag)}
                    <span class="scene-tile__tag">{tag}</span>
                  {/each}
                  {#if card.tags.length > TAG_CHIPS_VISIBLE}
                    <span class="scene-tile__tag scene-tile__tag--overflow">
                      +{card.tags.length - TAG_CHIPS_VISIBLE} more
                    </span>
                  {/if}
                </div>
              {/if}
              <button
                type="button"
                class="scene-tile__fav"
                class:active={libraryFavoritesStore.isFavorite(card.id)}
                aria-pressed={libraryFavoritesStore.isFavorite(card.id)}
                aria-label={libraryFavoritesStore.isFavorite(card.id)
                  ? `Remove ${card.title || card.id} from favorites`
                  : `Add ${card.title || card.id} to favorites`}
                onclick={(e) => handleFavoriteToggle(e, card)}
              >
                <Icon name="heart" size={12} />
              </button>
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

  <aside class="scene-detail" aria-label="Scene detail">
    {#if !loading && selectedCard}
      <!-- Wave 2.5 Lane LIB.A — bundle-faithful right rail
           (`views/library.jsx` lines 50-62 in the v0.2-beta bundle).
           Preview pane tops the column, full metadata sits below.
           SceneCanvas reads metadata via the W36 / Wave 2.2 R.1
           heuristic — the same wiring Perform's preview hero uses. -->
      <div class="scene-detail__preview" aria-label="Selected scene preview">
        <SceneCanvas
          kind={sceneKindFromMetadata(selectedCard)}
          seed={sceneSeedFromMetadata(selectedCard)}
          audio
          ariaLabel={`Preview of ${selectedCard.title || selectedCard.id}`}
        />
      </div>
      <div class="scene-detail__body">
        <header class="scene-detail__head">
          <h3 class="scene-detail__title">{selectedCard.title || selectedCard.id}</h3>
          <span
            class="scene-detail__source"
            class:scene-detail__source--bundled={selectedCard.source === 'Bundled'}
          >
            {selectedCard.source === 'Bundled' ? 'BUILT-IN' : 'MY SCENES'}
          </span>
        </header>
        {#if selectedCard.credit}
          <p class="scene-detail__credit">by {selectedCard.credit}</p>
        {/if}
        {#if selectedCard.description}
          <p class="scene-detail__description">{selectedCard.description}</p>
        {/if}
        <dl class="scene-detail__facts">
          {#if selectedCard.categories.length > 0}
            <div class="scene-detail__fact-row">
              <dt class="scene-detail__fact-label">Categories</dt>
              <dd class="scene-detail__fact-value">
                <ul class="scene-detail__chips" aria-label="Scene categories">
                  {#each selectedCard.categories as cat (cat)}
                    <li class="scene-detail__chip scene-detail__chip--category">{cat}</li>
                  {/each}
                </ul>
              </dd>
            </div>
          {/if}
          {#if selectedCard.tags.length > 0}
            <div class="scene-detail__fact-row">
              <dt class="scene-detail__fact-label">Tags</dt>
              <dd class="scene-detail__fact-value">
                <ul class="scene-detail__chips" aria-label="Scene tags">
                  {#each selectedCard.tags as tag (tag)}
                    <li class="scene-detail__chip">{tag}</li>
                  {/each}
                </ul>
              </dd>
            </div>
          {/if}
          {#if loadedSceneStore.snapshot?.id === selectedCard.id}
            <div class="scene-detail__fact-row">
              <dt class="scene-detail__fact-label">Controls</dt>
              <dd class="scene-detail__fact-value scene-detail__controls-count">
                {loadedSceneStore.snapshot.controls.length}
              </dd>
            </div>
          {/if}
        </dl>
        <div class="scene-detail__actions" role="group" aria-label="Scene actions">
          <button
            type="button"
            class="scene-detail__action scene-detail__action--primary"
            onclick={handleLoadSelected}
            disabled={loadedSceneStore.loading}
            title="Load this scene into the runtime"
          >
            <Icon name="play" size={12} />
            {loadedSceneStore.snapshot?.id === selectedCard.id ? 'Reload' : 'Load'}
          </button>
          <button
            type="button"
            class="scene-detail__action"
            onclick={handleOpenInAuthor}
            disabled={loadedSceneStore.loading}
            title="Open this scene in the Author view"
          >
            <Icon name="edit" size={12} />
            Open in Author
          </button>
        </div>
      </div>
    {:else}
      <div class="scene-detail__empty">
        <p>Select a scene to preview its details.</p>
      </div>
    {/if}
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
  .scene-detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-md);
    position: sticky;
    top: 0;
    max-height: calc(100vh - 200px);
    overflow: hidden;
  }
  .scene-detail__preview {
    aspect-ratio: 16 / 9;
    background: var(--color-bg-3);
    border-radius: var(--radius-sm);
    overflow: hidden;
    flex-shrink: 0;
  }
  /* Wave 2.5 Lane LIB.A — SceneCanvas fills its container per the
     Perform preview tile pattern; the widget's $effect-driven
     ResizeObserver handles DPR + bitmap sizing. */
  .scene-detail__preview :global(canvas.scene-canvas) {
    width: 100%;
    height: 100%;
    display: block;
  }
  .scene-detail__body {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }
  .scene-detail__head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-sm);
  }
  .scene-detail__title {
    margin: 0;
    font-size: var(--font-size-body-md);
    color: var(--color-text-0);
    font-weight: 600;
  }
  .scene-detail__source {
    background: var(--color-bg-3);
    color: var(--color-text-2);
    border-radius: var(--radius-xs);
    padding: 1px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
    flex-shrink: 0;
    align-self: flex-start;
  }
  .scene-detail__source--bundled {
    background: var(--color-cat-general);
    color: var(--color-on-primary);
  }
  .scene-detail__credit {
    margin: 0;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    font-style: italic;
  }
  .scene-detail__description {
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
    line-height: 1.5;
  }
  .scene-detail__facts {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    margin: 0;
  }
  .scene-detail__fact-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .scene-detail__fact-label {
    color: var(--color-text-3);
    font-size: 10px;
    font-family: var(--font-family-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .scene-detail__fact-value {
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }
  .scene-detail__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .scene-detail__chip {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 1px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: 10px;
    text-transform: lowercase;
  }
  .scene-detail__chip--category {
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .scene-detail__controls-count {
    font-family: var(--font-family-mono);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-0);
  }
  .scene-detail__actions {
    display: flex;
    gap: var(--space-xs);
    padding-top: var(--space-xs);
    border-top: 1px solid var(--color-line-1);
    margin-top: auto;
  }
  .scene-detail__action {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
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
  .scene-detail__action:hover:not(:disabled) {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .scene-detail__action:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .scene-detail__action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .scene-detail__action--primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .scene-detail__action--primary:hover:not(:disabled) {
    filter: brightness(1.05);
  }
  .scene-detail__empty {
    color: var(--color-text-3);
    font-size: var(--font-size-body-sm);
    padding: var(--space-md);
    text-align: center;
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
  .library__size,
  .library__sort,
  .library__source {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }
  .library__size select,
  .library__sort select,
  .library__source select {
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
  /* Wave 2.5 LIB.E — dim the tile while its `load_scene` IPC is in
     flight so the operator sees the double-click landed. The cursor
     flips to `progress`; pointer-events stay enabled so a focus-loss
     interaction still works. */
  .scene-tile--loading {
    opacity: 0.6;
    cursor: progress;
  }
  /* Wave 2.5 Lane LIB.A — selected-but-not-loaded treatment uses a
     softer outline so it reads distinct from the loaded-scene ring
     above. */
  .scene-tile--selected {
    border-color: var(--color-text-2);
    box-shadow:
      var(--shadow-panel),
      inset 0 0 0 1px var(--color-text-2);
  }
  .scene-tile__thumbnail {
    aspect-ratio: 16 / 10;
    background: var(--color-bg-3);
    border-radius: var(--radius-sm);
    overflow: hidden;
    position: relative;
  }
  /* Wave 2.5 LIB.B — tag/favorite overlay layers on the thumbnail.
     Bundle precedent: `FxTile` .tag (top-left) + .fav (top-right). */
  .scene-tile__tags {
    position: absolute;
    top: var(--space-xs);
    left: var(--space-xs);
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    max-width: calc(100% - 32px); /* leave room for .scene-tile__fav */
    pointer-events: none;
  }
  .scene-tile__tag {
    background: rgba(0, 0, 0, 0.7);
    color: #fff;
    font-family: var(--font-family-mono);
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 2px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    line-height: 1.3;
  }
  .scene-tile__tag--overflow {
    background: rgba(0, 0, 0, 0.55);
    text-transform: none;
    letter-spacing: 0;
  }
  .scene-tile__fav {
    position: absolute;
    top: var(--space-xs);
    right: var(--space-xs);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: rgba(0, 0, 0, 0.4);
    color: rgba(255, 255, 255, 0.7);
    border: 0;
    border-radius: 2px;
    cursor: pointer;
    transition:
      color 0.12s ease,
      background 0.12s ease;
  }
  .scene-tile__fav:hover {
    background: rgba(0, 0, 0, 0.6);
    color: rgba(255, 255, 255, 0.95);
  }
  .scene-tile__fav.active {
    color: var(--color-primary);
  }
  .scene-tile__fav:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .library__chip--favorites {
    /* Visually distinguish the favorites chip from category chips —
       it's an orthogonal filter axis. */
    font-weight: 600;
  }
  /* Wave 2.2 Lane R.1 — `<SceneStill>` canvas fills the thumbnail
     container. Bitmap dimensions are explicit-width/height (220 × 138);
     CSS scales the rendered canvas to fit the grid cell. Wave 2.5
     Lane LIB.D adds a first-paint fade-in transition — the canvas
     animates from opacity:0 → 1 on mount, so the grid resolves with a
     soft reveal instead of a hard pop when scrolling fills new cards.
     `prefers-reduced-motion: reduce` suppresses the animation per the
     repo-wide accessibility posture. */
  .scene-tile__thumbnail :global(canvas.scene-still) {
    width: 100%;
    height: 100%;
    display: block;
    animation: scene-tile-thumb-fade-in 240ms ease-out both;
  }
  @keyframes scene-tile-thumb-fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .scene-tile__thumbnail :global(canvas.scene-still) {
      animation: none;
    }
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
