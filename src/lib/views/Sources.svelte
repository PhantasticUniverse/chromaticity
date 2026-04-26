<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-17 PR1 Sources view full port — replaces the W16 PR2
  // `EmptyState` stub with the canonical 3-column layout per the v0.2
  // bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/media.jsx`,
  // the `MediaManagerView` component routed at `id: "media"` in the
  // bundle, hoisted to a top-level `Sources` route in the W16 PR2
  // sidebar redesign). Bundle JSX is the canonical structural reference
  // per the auto-memory `feedback_wireframes_normative_for_ui_ux.md`;
  // the `v0.2-ui-wireframe-screenshots/pulse/sources.png` capture is
  // the visual confirmation (it renders the same JSX).
  //
  // Layout: sources sidebar (left, 220px) + asset grid (centre, flex)
  // + inspector pane (right, 320px). State is local to the component;
  // no IPC wiring at this slice — real media adapter integration is
  // `EV-INTEGRATION-MATRIX` Phase-4a scope, and the inspector preview
  // tile depends on the ADR-011 rendering backend (Phase-2+).
  //
  // Locked W17 PR1 deviations from the bundle:
  //   - The bundle's `<SceneStill seed={…} kind={…} />` thumbnail
  //     primitive is replaced with a token-driven placeholder gradient
  //     keyed off the asset seed. Real preview rendering needs the
  //     runtime / scene crates wired through to the UI; the gradient
  //     honours the wireframe's coloured-tile aesthetic without
  //     coupling to runtime scope. Same pattern as the W16 PR3
  //     Welcome view's blob field.
  //   - W20 view-internal icon sweep: per-collection sidebar glyphs,
  //     search input, grid/list toggle, sidebar add, inspector tag-add,
  //     and inspector favourite all render via the W19 PR3 vendored
  //     `Icon.svelte` primitive. Bundle-canonical names per
  //     `views/media.jsx` (`folder` / `camera` / `heart` / `refresh` /
  //     `fullscreen` / `search` / `grid` / `list` / `plus`). The "More"
  //     three-dot button has no clean bundle Icon mapping and stays as
  //     `⋯` text per the policy on Unicode-only fallbacks.
  //   - `Add to scene` is a visual stub at this slice (W14+ wires the
  //     real path into Library / Perform when the media adapter lands).
  //   - Heart-favourite and 3-dot (more) buttons are visual-only stubs.
  import {
    DEFAULT_ASSET_ID,
    DEFAULT_COLLECTION_ID,
    INSPECTOR_META,
    MEDIA_ASSETS,
    SOURCE_COLLECTIONS,
    type AssetFormat,
    type MediaAsset,
  } from '../data/sources-fixtures';
  import Icon from '../Icon.svelte';

  let selectedCollectionId = $state<string>(DEFAULT_COLLECTION_ID);
  let selectedAssetId = $state<string>(DEFAULT_ASSET_ID);
  let searchQuery = $state<string>('');
  let formatFilter = $state<AssetFormat>('any');

  const filteredAssets = $derived<readonly MediaAsset[]>(
    (() => {
      const q = searchQuery.trim().toLowerCase();
      return MEDIA_ASSETS.filter((asset) => {
        if (q && !asset.filename.toLowerCase().includes(q)) return false;
        if (formatFilter !== 'any' && asset.format !== formatFilter) return false;
        return true;
      });
    })(),
  );

  const activeCollection = $derived(
    SOURCE_COLLECTIONS.find((c) => c.id === selectedCollectionId) ?? SOURCE_COLLECTIONS[0],
  );

  const selectedAsset = $derived<MediaAsset | undefined>(
    MEDIA_ASSETS.find((a) => a.id === selectedAssetId),
  );

  /** Deterministic placeholder hue from the asset seed.
   * Mirrors the per-tile variation the bundle gets from `<SceneStill seed>`
   * without depending on the unported renderer. */
  function tintAngle(seed: number): number {
    return (seed * 37) % 360;
  }

  function selectCollection(id: string): void {
    selectedCollectionId = id;
  }

  function selectAsset(id: string): void {
    selectedAssetId = id;
  }
</script>

<section class="sources">
  <aside class="sources__sidebar" aria-label="Source collections">
    <header class="sources__sidebar-header">
      <span class="sources__sidebar-title">Sources</span>
      <button
        type="button"
        class="sources__sidebar-add"
        aria-label="Add source"
        title="Add source — wired in W17+ when the media adapter lands"
        disabled
      >
        <Icon name="plus" size={12} />
      </button>
    </header>
    <ul class="sources__collections">
      {#each SOURCE_COLLECTIONS as collection (collection.id)}
        <li>
          <button
            type="button"
            class="sources__collection-row"
            class:active={collection.id === selectedCollectionId}
            data-cap={collection.cap ?? null}
            aria-current={collection.id === selectedCollectionId ? 'true' : undefined}
            onclick={() => selectCollection(collection.id)}
          >
            <span class="sources__collection-glyph"><Icon name={collection.icon} size={14} /></span>
            <span class="sources__collection-name">{collection.name}</span>
            <span class="sources__collection-count">{collection.count}</span>
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <main class="sources__grid-pane">
    <header class="sources__grid-header">
      <h2 class="sources__grid-title">
        {activeCollection?.name ?? 'All Media'}
        <span class="sources__grid-sep" aria-hidden="true">·</span>
        <span class="sources__grid-count">{activeCollection?.count ?? MEDIA_ASSETS.length}</span>
      </h2>

      <div class="sources__grid-toolbar">
        <label class="sources__search">
          <span class="sources__search-glyph"><Icon name="search" size={14} /></span>
          <input
            type="search"
            class="sources__search-input"
            placeholder="Search media…"
            bind:value={searchQuery}
            aria-label="Search media"
          />
        </label>
        <select
          class="sources__format-select"
          bind:value={formatFilter}
          aria-label="Filter by format"
        >
          <option value="any">Any type</option>
          <option value="video">Video</option>
          <option value="loop">Loop</option>
          <option value="image">Image</option>
          <option value="live">Live</option>
        </select>
        <div class="sources__view-toggle" role="group" aria-label="View mode">
          <button type="button" class="sources__view-btn on" aria-pressed="true" title="Grid view">
            <Icon name="grid" size={12} />
          </button>
          <button
            type="button"
            class="sources__view-btn"
            aria-pressed="false"
            title="List view"
            disabled
          >
            <Icon name="list" size={12} />
          </button>
        </div>
      </div>
    </header>

    <ul class="sources__grid">
      {#each filteredAssets as asset (asset.id)}
        <li class="sources__asset-cell">
          <button
            type="button"
            class="sources__asset"
            class:active={asset.id === selectedAssetId}
            onclick={() => selectAsset(asset.id)}
            aria-pressed={asset.id === selectedAssetId}
          >
            <div
              class="sources__asset-thumb"
              style:--asset-hue="{tintAngle(asset.seed)}deg"
              aria-hidden="true"
            >
              <span class="sources__asset-res">{asset.resolution}</span>
              {#if asset.duration}
                <span class="sources__asset-dur">{asset.duration}</span>
              {/if}
            </div>
            <div class="sources__asset-meta">
              <p class="sources__asset-name" title={asset.filename}>{asset.filename}</p>
              <p class="sources__asset-spec">
                <span>{asset.format}</span>
                <span class="sources__asset-spec-sep" aria-hidden="true">·</span>
                <span>{asset.fileSize}</span>
              </p>
            </div>
          </button>
        </li>
      {/each}
      {#if filteredAssets.length === 0}
        <li class="sources__grid-empty-cell">
          <p class="sources__grid-empty">No assets match the current filters.</p>
        </li>
      {/if}
    </ul>
  </main>

  <aside class="sources__inspector" aria-label="Asset inspector">
    {#if selectedAsset}
      <header class="sources__inspector-header">
        <span class="sources__inspector-eyebrow">Inspector</span>
        <h2 class="sources__inspector-title" title={selectedAsset.filename}>
          {selectedAsset.filename}
        </h2>
      </header>

      <div
        class="sources__inspector-preview"
        style:--asset-hue="{tintAngle(selectedAsset.seed)}deg"
        aria-hidden="true"
      ></div>

      <dl class="sources__inspector-meta">
        <div class="sources__inspector-row">
          <dt>Dimensions</dt>
          <dd>{INSPECTOR_META.dimensions}</dd>
        </div>
        <div class="sources__inspector-row">
          <dt>Duration</dt>
          <dd>{INSPECTOR_META.duration}</dd>
        </div>
        <div class="sources__inspector-row">
          <dt>Codec</dt>
          <dd>{INSPECTOR_META.codec}</dd>
        </div>
        <div class="sources__inspector-row">
          <dt>Size</dt>
          <dd>{INSPECTOR_META.size}</dd>
        </div>
        <div class="sources__inspector-row">
          <dt>Loop</dt>
          <dd>{INSPECTOR_META.loop}</dd>
        </div>
        <div class="sources__inspector-row">
          <dt>Added</dt>
          <dd>{INSPECTOR_META.added}</dd>
        </div>
      </dl>

      <section class="sources__inspector-tags" aria-label="Tags">
        <h3 class="sources__inspector-subhead">Tags</h3>
        <div class="sources__tag-row">
          {#each INSPECTOR_META.tags as tag (tag)}
            <span class="sources__tag">{tag}</span>
          {/each}
          <button
            type="button"
            class="sources__tag-add"
            disabled
            title="Add tag — editing wired in W17+"
            aria-label="Add tag"
          >
            <Icon name="plus" size={10} />
          </button>
        </div>
      </section>

      <div class="sources__inspector-actions">
        <button
          type="button"
          class="sources__action sources__action--primary"
          title="Add to scene — wired in W17+ when the media adapter lands"
          disabled
        >
          Add to scene
        </button>
        <button
          type="button"
          class="sources__action sources__action--icon"
          aria-label="Toggle favourite"
          title="Favourite — wired in W17+"
          disabled
        >
          <Icon name="heart" size={12} />
        </button>
        <button
          type="button"
          class="sources__action sources__action--icon sources__action--danger"
          aria-label="More actions"
          title="More — wired in W17+"
          disabled
        >
          ⋯
        </button>
      </div>
    {:else}
      <p class="sources__inspector-empty">Select an asset to inspect</p>
    {/if}
  </aside>
</section>

<style>
  /* 3-column flex layout per the v0.2 wireframe.
   * Widths mirror the bundle JSX `gridTemplateColumns: "260px 1fr 380px"`,
   * tightened slightly to fit the W12 shell's standard padding. */
  .sources {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr) 320px;
    gap: var(--space-md);
    align-items: stretch;
    min-height: 0;
  }

  /* ---------------- Left: source-collections sidebar -------------------- */
  .sources__sidebar {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    min-width: 0;
  }
  .sources__sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-xs) var(--space-xs) 0 var(--space-xs);
  }
  .sources__sidebar-title {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .sources__sidebar-add {
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-2);
    width: 20px;
    height: 20px;
    line-height: 1;
    cursor: pointer;
    font-size: 13px;
  }
  .sources__sidebar-add:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .sources__collections {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: auto;
    min-height: 0;
  }
  .sources__collection-row {
    width: 100%;
    display: grid;
    grid-template-columns: 18px 1fr max-content;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: transparent;
    border: 0;
    border-radius: var(--radius-xs);
    color: var(--color-text-1);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    text-align: left;
    cursor: pointer;
    transition:
      background 100ms ease,
      color 100ms ease;
  }
  .sources__collection-row:hover {
    background: var(--color-bg-3);
  }
  .sources__collection-row.active {
    background: var(--color-bg-3);
    color: var(--color-primary);
  }
  .sources__collection-glyph {
    font-family: var(--font-family-mono);
    font-size: 12px;
    color: var(--color-text-2);
    line-height: 1;
  }
  .sources__collection-row.active .sources__collection-glyph {
    color: var(--color-primary);
  }
  .sources__collection-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sources__collection-count {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }

  /* ---------------- Centre: asset grid pane ----------------------------- */
  .sources__grid-pane {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .sources__grid-header {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
    flex-wrap: wrap;
  }
  .sources__grid-title {
    margin: 0;
    font-size: var(--font-size-body-md);
    font-weight: 600;
    color: var(--color-text-0);
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-xs);
  }
  .sources__grid-sep {
    color: var(--color-text-3);
    font-weight: 400;
  }
  .sources__grid-count {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .sources__grid-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-left: auto;
    flex-wrap: wrap;
  }
  .sources__search {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  .sources__search-glyph {
    position: absolute;
    left: var(--space-sm);
    color: var(--color-text-3);
    font-size: 12px;
    pointer-events: none;
  }
  .sources__search-input {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    padding: 4px var(--space-sm) 4px 22px;
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    width: 200px;
  }
  .sources__search-input:focus {
    outline: none;
    border-color: var(--color-line-3);
  }
  .sources__format-select {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    padding: 3px var(--space-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
  }
  .sources__view-toggle {
    display: inline-flex;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .sources__view-btn {
    background: var(--color-bg-3);
    border: 0;
    color: var(--color-text-2);
    padding: 4px var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .sources__view-btn + .sources__view-btn {
    border-left: 1px solid var(--color-line-2);
  }
  .sources__view-btn.on {
    background: var(--color-bg-4);
    color: var(--color-primary);
  }
  .sources__view-btn:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .sources__grid {
    list-style: none;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--space-md);
    padding: var(--space-md);
    overflow: auto;
    min-height: 0;
    flex: 1;
  }
  .sources__asset-cell {
    display: contents;
  }
  .sources__grid-empty-cell {
    grid-column: 1 / -1;
    list-style: none;
  }
  .sources__grid-empty {
    margin: 0;
    padding: var(--space-2xl);
    text-align: center;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .sources__asset {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
    cursor: pointer;
    padding: 0;
    text-align: left;
    color: inherit;
    font: inherit;
    display: flex;
    flex-direction: column;
    transition:
      border-color 100ms ease,
      transform 100ms ease;
  }
  .sources__asset:hover {
    border-color: var(--color-line-3);
  }
  .sources__asset.active {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 1px var(--color-primary);
  }
  .sources__asset-thumb {
    aspect-ratio: 16 / 10;
    position: relative;
    /* Per-tile placeholder gradient — hue rotates from `--asset-hue`
     * (set via inline style:`-vars). Real preview rendering is gated on
     * the ADR-011 backend (Phase-2+). The two-stop conic + radial mix
     * gives the wireframe's lava-lamp look without a renderer. */
    background:
      radial-gradient(
        circle at 30% 30%,
        hsl(var(--asset-hue) 70% 55% / calc(0.6 * var(--glow, 1))),
        transparent 60%
      ),
      radial-gradient(
        circle at 70% 70%,
        hsl(calc(var(--asset-hue) + 60) 65% 45% / calc(0.5 * var(--glow, 1))),
        transparent 55%
      ),
      var(--color-bg-4);
  }
  .sources__asset-res {
    position: absolute;
    top: 4px;
    right: 4px;
    background: rgba(0, 0, 0, 0.6);
    color: #f4f6fb;
    font-family: var(--font-family-mono);
    font-size: 9px;
    padding: 1px 5px;
    border-radius: var(--radius-xs);
  }
  .sources__asset-dur {
    position: absolute;
    bottom: 4px;
    left: 4px;
    background: rgba(0, 0, 0, 0.6);
    color: #f4f6fb;
    font-family: var(--font-family-mono);
    font-size: 9px;
    padding: 1px 5px;
    border-radius: var(--radius-xs);
  }
  .sources__asset-meta {
    padding: var(--space-xs) var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sources__asset-name {
    margin: 0;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sources__asset-spec {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
    display: inline-flex;
    gap: var(--space-xs);
  }
  .sources__asset-spec-sep {
    color: var(--color-text-4);
  }

  /* ---------------- Right: inspector pane -------------------------------- */
  .sources__inspector {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-md);
    min-width: 0;
    overflow: auto;
  }
  .sources__inspector-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: var(--space-xs);
    border-bottom: 1px solid var(--color-line-1);
  }
  .sources__inspector-eyebrow {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .sources__inspector-title {
    margin: 0;
    font-size: var(--font-size-body-md);
    font-weight: 600;
    color: var(--color-text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sources__inspector-empty {
    margin: 0;
    padding: var(--space-2xl) 0;
    text-align: center;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .sources__inspector-preview {
    aspect-ratio: 16 / 10;
    border-radius: var(--radius-sm);
    background:
      radial-gradient(
        circle at 30% 30%,
        hsl(var(--asset-hue) 70% 55% / calc(0.7 * var(--glow, 1))),
        transparent 60%
      ),
      radial-gradient(
        circle at 70% 70%,
        hsl(calc(var(--asset-hue) + 60) 65% 45% / calc(0.6 * var(--glow, 1))),
        transparent 55%
      ),
      var(--color-bg-4);
    border: 1px solid var(--color-line-2);
  }
  .sources__inspector-meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .sources__inspector-row {
    display: flex;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-xs) 0;
    border-bottom: 1px solid var(--color-line-1);
  }
  .sources__inspector-row dt {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
    margin: 0;
  }
  .sources__inspector-row dd {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-0);
  }
  .sources__inspector-subhead {
    margin: 0 0 var(--space-xs) 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
    font-weight: 500;
  }
  .sources__tag-row {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }
  .sources__tag {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    padding: 2px var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-1);
  }
  .sources__tag-add {
    background: transparent;
    border: 1px dashed var(--color-line-2);
    border-radius: var(--radius-full);
    padding: 2px var(--space-sm);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    cursor: pointer;
  }
  .sources__tag-add:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .sources__inspector-actions {
    display: flex;
    gap: var(--space-xs);
    margin-top: auto;
  }
  .sources__action {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    padding: var(--space-sm) var(--space-md);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-label);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
    transition:
      background 100ms ease,
      border-color 100ms ease;
  }
  .sources__action:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .sources__action--primary {
    flex: 1;
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary);
  }
  .sources__action--icon {
    width: 36px;
    padding: var(--space-sm) 0;
    text-align: center;
  }
  .sources__action--danger {
    color: var(--color-danger);
  }
</style>
