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
    LIVE_SOURCES,
    MEDIA_ASSETS,
    SOURCE_COLLECTIONS,
    type AssetFormat,
    type LiveSource,
    type MediaAsset,
  } from '../data/sources-fixtures';
  import Icon from '../Icon.svelte';
  import { cameraStore } from '../stores/cameraStore.svelte';
  import { currentPlatform } from '../stores/platform';
  import { DEFAULT_OUTPUT_NAME, syphonStore } from '../stores/syphonStore.svelte';
  import { toIpcError } from '../types/ipc-error';

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

  // W22 PR2 Live Sources (Spout / NDI) section state. Connect button only
  // flips a local fixture flag — real Spout / NDI ingest is Phase-3+/4a.
  // SvelteSet would be the idiomatic Svelte-5 reactive collection but a
  // plain `Record<string, boolean>` keeps the consumer surface simpler
  // here (boolean lookup per row, no membership semantics needed).
  const connectedLiveSources = $state<Record<string, boolean>>({});

  function liveSourceDisabled(src: LiveSource): boolean {
    if (!src.available) return true;
    return src.kind === 'spout' && currentPlatform !== 'win';
  }

  function liveSourceTooltip(src: LiveSource): string {
    if (!src.available)
      return 'Source is offline — start the producer application to advertise it.';
    if (src.kind === 'spout' && currentPlatform !== 'win') {
      return 'Spout is Windows-only — use NDI on macOS.';
    }
    return connectedLiveSources[src.id]
      ? 'Connected — click to disconnect.'
      : 'Connect to this live source.';
  }

  function toggleLiveSource(src: LiveSource): void {
    if (liveSourceDisabled(src)) return;
    connectedLiveSources[src.id] = !connectedLiveSources[src.id];
  }

  // Wave 1c-β Phase 4 Lane A camera section. The store rehydrates the
  // last-selected device from `localStorage` on mount, then dispatches
  // the `list_camera_devices` Tauri command to populate the visible
  // device list. The current command returns an empty vec under the
  // documented stub posture (see `cameraStore.svelte.ts` module prose);
  // the empty-state copy guides the operator to System Settings for
  // permission granting until the per-platform nokhwa enumeration arms
  // wire end-to-end.
  $effect(() => {
    cameraStore.loadPersistedSelection();
    void cameraStore.refresh();
  });

  function selectCamera(deviceId: string): void {
    if (cameraStore.selected === deviceId) {
      cameraStore.select(null);
    } else {
      cameraStore.select(deviceId);
    }
  }

  /** Operator-facing label for the camera-kind discriminator. Mirrors
   * the `chromaticity_camera_adapter::CameraDeviceKind` variants per
   * ADR-020 §Decision §2 / `docs/54` Phase-2 lesson 9. */
  function cameraKindLabel(kind: string): string {
    switch (kind) {
      case 'BuiltIn':
        return 'Built-in';
      case 'External':
        return 'External';
      case 'Virtual':
        return 'Virtual';
      case 'Continuity':
        return 'Continuity';
      default:
        return 'Other';
    }
  }

  // Wave 1c-β Phase 4 Lane B Syphon section. macOS-only at the engine
  // seam per ADR-021 §1; the consumer view detects non-macOS via
  // `currentPlatform` and renders the "Unavailable on this platform"
  // empty-state branch without dispatching the Tauri commands. On
  // macOS the store rehydrates the last-selected input UUID from
  // `localStorage`, then dispatches `list_syphon_servers` to populate
  // the visible server list.
  //
  // **CRITICAL** per ADR-021 §10 + `docs/29-cleanroom-legal-policy-baseline.md`
  // Wave 1c-β threat-model addendum: when `syphonStore.output.active
  // === true`, this view MUST render the non-dismissible
  // trust-boundary warning banner. The banner copy is canonical and
  // MUST NOT be modified or made dismissible while output is active.
  let publishName = $state<string>(DEFAULT_OUTPUT_NAME);
  $effect(() => {
    if (currentPlatform !== 'mac') return;
    syphonStore.loadPersistedSelection();
    void syphonStore.refreshServers();
  });

  function selectSyphonInput(uuid: string): void {
    if (syphonStore.selectedInput === uuid) {
      syphonStore.selectInput(null);
    } else {
      syphonStore.selectInput(uuid);
    }
  }

  async function toggleSyphonOutput(): Promise<void> {
    if (syphonStore.output.active) {
      await syphonStore.unpublishOutput();
    } else {
      const trimmed = publishName.trim();
      const finalName = trimmed.length > 0 ? trimmed : DEFAULT_OUTPUT_NAME;
      await syphonStore.publishOutput(finalName);
    }
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
        title="Add source"
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
        <span class="sources__grid-eyebrow">Local</span>
        <span class="sources__grid-sep" aria-hidden="true">·</span>
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

    <section class="sources__live sources__live--camera" aria-label="Camera devices">
      <header class="sources__live-header">
        <h2 class="sources__live-title">
          <span class="sources__grid-eyebrow">Live</span>
          <span class="sources__grid-sep" aria-hidden="true">·</span>
          Cameras
          <span class="sources__grid-sep" aria-hidden="true">·</span>
          <span class="sources__grid-count">{cameraStore.devices.length}</span>
        </h2>
        <p class="sources__live-sub">
          Cross-platform camera input (CoreMediaIO on macOS, Media Foundation on Windows). Selection
          persists across launches.
        </p>
      </header>

      {#if cameraStore.isLoading}
        <p class="sources__live-status" role="status">Searching for cameras…</p>
      {:else if cameraStore.error && toIpcError(cameraStore.error).kind !== 'invoke_unavailable'}
        <p class="sources__live-status sources__live-status--error" role="alert">
          Camera enumeration failed: {cameraStore.error}
        </p>
      {:else if cameraStore.devices.length === 0}
        <p class="sources__live-status">
          No cameras found — grant camera permission via System Settings (macOS: Privacy &amp;
          Security › Camera; Windows: Settings › Privacy &amp; security › Camera) and reopen this
          view.
        </p>
      {:else}
        <ul class="sources__live-list" data-testid="camera-device-list">
          {#each cameraStore.devices as device (device.device_id)}
            {@const isSelected = cameraStore.selected === device.device_id}
            <li
              class="sources__live-row"
              class:connected={isSelected}
              data-device-id={device.device_id}
            >
              <span class="sources__live-glyph" aria-hidden="true">
                <Icon name="camera" size={14} />
              </span>
              <div class="sources__live-meta">
                <p class="sources__live-name">{device.display_name}</p>
                <p class="sources__live-origin">{device.device_id}</p>
              </div>
              <span class="sources__live-spec">
                <span>{cameraKindLabel(device.kind)}</span>
              </span>
              <span
                class="sources__live-chip sources__live-chip--cross"
                aria-label="Cross-platform"
              >
                CAMERA
              </span>
              <button
                type="button"
                class="sources__live-action"
                class:sources__live-action--connected={isSelected}
                onclick={() => selectCamera(device.device_id)}
                aria-pressed={isSelected}
              >
                {#if isSelected}
                  Selected
                {:else}
                  Connect
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="sources__live sources__live--syphon" aria-label="Syphon servers">
      <header class="sources__live-header">
        <h2 class="sources__live-title">
          <span class="sources__grid-eyebrow">Live</span>
          <span class="sources__grid-sep" aria-hidden="true">·</span>
          Syphon
          <span class="sources__grid-sep" aria-hidden="true">·</span>
          <span class="sources__grid-count">{syphonStore.servers.length}</span>
        </h2>
        <p class="sources__live-sub">
          GPU-surface I/O via Syphon (macOS-only). Subscribe to a server published by another
          application, or publish Chromaticity's output back onto the local-machine bus.
        </p>
      </header>

      {#if currentPlatform !== 'mac'}
        <p class="sources__live-status">
          Unavailable on this platform — Syphon is macOS-only. Use NDI on Windows for cross-app live
          texture I/O.
        </p>
      {:else}
        <!-- Input section: discovered servers on the bus. -->
        <div class="sources__syphon-section">
          <h3 class="sources__syphon-subhead">Input · subscribe</h3>
          <p class="sources__syphon-sub">
            Syphon input may originate from any application on this machine; verify the source
            before binding.
          </p>
          {#if syphonStore.isLoading}
            <p class="sources__live-status" role="status">Searching for Syphon servers…</p>
          {:else if syphonStore.error && toIpcError(syphonStore.error).kind !== 'invoke_unavailable'}
            <p class="sources__live-status sources__live-status--error" role="alert">
              Syphon error: {syphonStore.error}
            </p>
          {:else if syphonStore.servers.length === 0}
            <p class="sources__live-status">
              No Syphon sources found — start a Syphon-compatible app (Resolume, MadMapper, VDMX,
              etc.) and refresh.
            </p>
          {:else}
            <ul class="sources__live-list" data-testid="syphon-server-list">
              {#each syphonStore.servers as server (server.uuid)}
                {@const isSelected = syphonStore.selectedInput === server.uuid}
                <li
                  class="sources__live-row"
                  class:connected={isSelected}
                  data-server-uuid={server.uuid}
                >
                  <span class="sources__live-glyph" aria-hidden="true">
                    <Icon name="link" size={14} />
                  </span>
                  <div class="sources__live-meta">
                    <p class="sources__live-name">{server.name}</p>
                    <p class="sources__live-origin">{server.app_name}</p>
                  </div>
                  <span class="sources__live-spec">
                    <span>SYPHON</span>
                  </span>
                  <span
                    class="sources__live-chip sources__live-chip--cross"
                    aria-label="macOS-only"
                  >
                    MAC
                  </span>
                  <button
                    type="button"
                    class="sources__live-action"
                    class:sources__live-action--connected={isSelected}
                    onclick={() => selectSyphonInput(server.uuid)}
                    aria-pressed={isSelected}
                  >
                    {#if isSelected}
                      Selected
                    {:else}
                      Subscribe
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <button
            type="button"
            class="sources__syphon-refresh"
            onclick={() => void syphonStore.refreshServers()}
            disabled={syphonStore.isLoading}
          >
            <Icon name="refresh" size={12} /> Refresh
          </button>
        </div>

        <!-- Output section: publish chromaticity's output onto the bus. -->
        <div class="sources__syphon-section">
          <h3 class="sources__syphon-subhead">Output · publish</h3>
          <p class="sources__syphon-sub">
            Default-off. Publishing exposes chromaticity's GPU output to any application on this
            machine.
          </p>
          <div class="sources__syphon-publish">
            <label class="sources__syphon-publish-label">
              <span class="sources__syphon-publish-label-text">Server name</span>
              <input
                type="text"
                class="sources__syphon-publish-input"
                bind:value={publishName}
                placeholder={DEFAULT_OUTPUT_NAME}
                disabled={syphonStore.output.active}
                aria-label="Syphon output server name"
              />
            </label>
            <button
              type="button"
              class="sources__live-action"
              class:sources__live-action--connected={syphonStore.output.active}
              onclick={() => void toggleSyphonOutput()}
              aria-pressed={syphonStore.output.active}
            >
              {#if syphonStore.output.active}
                Stop publishing
              {:else}
                Publish output
              {/if}
            </button>
          </div>

          {#if syphonStore.output.active}
            <!-- CRITICAL non-dismissible trust-boundary warning per
              ADR-021 §10 + docs/29 Wave 1c-β threat-model addendum.
              Copy is canonical; do not modify or make dismissible
              while output is active. -->
            <p
              class="sources__syphon-warning"
              role="alert"
              data-testid="syphon-trust-boundary-warning"
            >
              Syphon output is visible to any application on this machine. Do not publish surfaces
              containing PII or secrets.
            </p>
          {/if}
        </div>
      {/if}
    </section>

    <section class="sources__live" aria-label="Live sources">
      <header class="sources__live-header">
        <h2 class="sources__live-title">
          <span class="sources__grid-eyebrow">Live</span>
          <span class="sources__grid-sep" aria-hidden="true">·</span>
          Spout &amp; NDI
          <span class="sources__grid-sep" aria-hidden="true">·</span>
          <span class="sources__grid-count">{LIVE_SOURCES.length}</span>
        </h2>
        <p class="sources__live-sub">
          System-level live texture inputs from other applications on your machine or local network.
        </p>
      </header>

      <ul class="sources__live-list">
        {#each LIVE_SOURCES as src (src.id)}
          {@const disabled = liveSourceDisabled(src)}
          {@const connected = connectedLiveSources[src.id] === true}
          <li
            class="sources__live-row"
            class:disabled
            class:connected
            title={liveSourceTooltip(src)}
          >
            <span class="sources__live-glyph" aria-hidden="true">
              <Icon name={src.kind === 'spout' ? 'flash' : 'link'} size={14} />
            </span>
            <div class="sources__live-meta">
              <p class="sources__live-name">{src.name}</p>
              <p class="sources__live-origin">{src.origin}</p>
            </div>
            <span class="sources__live-spec">
              <span>{src.kind.toUpperCase()}</span>
              <span class="sources__asset-spec-sep" aria-hidden="true">·</span>
              <span>{src.resolution}</span>
              <span class="sources__asset-spec-sep" aria-hidden="true">·</span>
              <span>{src.refreshHz} Hz</span>
            </span>
            <span
              class="sources__live-chip"
              class:sources__live-chip--win={src.availability === 'win'}
              class:sources__live-chip--cross={src.availability === 'cross'}
              aria-label={src.availability === 'win' ? 'Windows-only' : 'Cross-platform'}
            >
              {src.availability === 'win' ? 'WIN' : 'CROSS-PLATFORM'}
            </span>
            <button
              type="button"
              class="sources__live-action"
              class:sources__live-action--connected={connected}
              {disabled}
              onclick={() => toggleLiveSource(src)}
              aria-pressed={connected}
            >
              {#if !src.available}
                Unavailable
              {:else if connected}
                Disconnect
              {:else}
                Connect
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    </section>
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
            title="Add tag"
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
          title="Add selected media to the active scene"
          disabled
        >
          Add to scene
        </button>
        <button
          type="button"
          class="sources__action sources__action--icon"
          aria-label="Toggle favourite"
          title="Toggle favourite"
          disabled
        >
          <Icon name="heart" size={12} />
        </button>
        <button
          type="button"
          class="sources__action sources__action--icon sources__action--danger"
          aria-label="More actions"
          title="More actions"
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
    height: 100%;
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
        hsl(calc(var(--asset-hue) + 60deg) 65% 45% / calc(0.5 * var(--glow, 1))),
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
        hsl(calc(var(--asset-hue) + 60deg) 65% 45% / calc(0.6 * var(--glow, 1))),
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

  /* ---------------- Centre footer: Live Sources (Spout / NDI) ----------- */
  /* The grid pane is a flex column. The asset grid above has `flex: 1` and
   * scrolls internally; the Live Sources section is a fixed-height tail.
   * On compact viewports the grid yields height first; the live list still
   * shows its first 2-3 rows above the fold. */
  .sources__grid-eyebrow {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
    font-weight: 500;
  }
  .sources__live {
    border-top: 1px solid var(--color-line-1);
    background: var(--color-bg-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding: var(--space-md);
    flex: 0 0 auto;
    max-height: 40%;
    overflow: auto;
  }
  .sources__live-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sources__live-title {
    margin: 0;
    font-size: var(--font-size-body-md);
    font-weight: 600;
    color: var(--color-text-0);
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-xs);
  }
  .sources__live-sub {
    margin: 0;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-2);
  }
  .sources__live-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .sources__live-row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1.6fr) minmax(0, 1.4fr) max-content max-content;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    transition:
      border-color 100ms ease,
      opacity 100ms ease;
  }
  .sources__live-row.disabled {
    opacity: 0.55;
  }
  .sources__live-row.connected {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 1px var(--color-primary);
  }
  .sources__live-glyph {
    color: var(--color-text-2);
    line-height: 0;
  }
  .sources__live-row.connected .sources__live-glyph {
    color: var(--color-primary);
  }
  .sources__live-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .sources__live-name {
    margin: 0;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sources__live-origin {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sources__live-spec {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    display: inline-flex;
    gap: var(--space-xs);
    align-items: baseline;
  }
  .sources__live-chip {
    border-radius: var(--radius-xs);
    padding: 1px var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
    color: var(--color-on-primary);
    line-height: 1.4;
  }
  .sources__live-chip--win {
    background: var(--color-cat-motion);
  }
  .sources__live-chip--cross {
    background: var(--color-cat-fx);
  }
  .sources__live-action {
    background: var(--color-bg-4);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    padding: var(--space-xs) var(--space-md);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-label);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
    transition:
      background 100ms ease,
      border-color 100ms ease,
      color 100ms ease;
  }
  .sources__live-action:hover:not(:disabled) {
    border-color: var(--color-line-3);
  }
  .sources__live-action--connected {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary);
  }
  .sources__live-action:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  /* Wave 1c-β Phase 4 Lane A camera section status copy. Renders the
   * loading / error / empty-state messages above the device list. */
  .sources__live-status {
    margin: 0;
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-3);
    border: 1px dashed var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .sources__live-status--error {
    color: var(--color-danger);
    border-color: var(--color-danger);
  }

  /* ---------------- Wave 1c-β Phase 4 Lane B Syphon section ------------- */
  .sources__syphon-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    margin-top: var(--space-sm);
    padding-top: var(--space-sm);
    border-top: 1px dashed var(--color-line-1);
  }
  .sources__syphon-section:first-of-type {
    border-top: 0;
    padding-top: 0;
    margin-top: 0;
  }
  .sources__syphon-subhead {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
    font-weight: 500;
  }
  .sources__syphon-sub {
    margin: 0 0 var(--space-xs) 0;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-2);
  }
  .sources__syphon-refresh {
    align-self: flex-start;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-label);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .sources__syphon-refresh:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .sources__syphon-publish {
    display: flex;
    align-items: flex-end;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .sources__syphon-publish-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 200px;
  }
  .sources__syphon-publish-label-text {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }
  .sources__syphon-publish-input {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-0);
    padding: 4px var(--space-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
  }
  .sources__syphon-publish-input:focus {
    outline: none;
    border-color: var(--color-line-3);
  }
  .sources__syphon-publish-input:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
  /* CRITICAL non-dismissible trust-boundary warning per ADR-021 §10 +
   * docs/29 Wave 1c-β threat-model addendum. Visual treatment follows
   * the danger-band pattern so the warning is unmissable while Syphon
   * output is active. */
  .sources__syphon-warning {
    margin: var(--space-sm) 0 0 0;
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-danger);
    border-left: 4px solid var(--color-danger);
    border-radius: var(--radius-sm);
    color: var(--color-danger);
    font-size: var(--font-size-body-sm);
    font-weight: 500;
  }
</style>
