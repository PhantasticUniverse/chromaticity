<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-13 PR3 view: Playlist v2 cue table editor.
  //
  // Source of truth (port reference):
  //   chromaticity-planning/ui-predev-bundle-v0.2-beta/views/playlist.jsx
  // Wireframe target:
  //   .playwright-mcp/wireframe-reference/03-playlist-cue-table.png
  //
  // Embeds the W13 PR2 `CueTransitionCurve` widget primitive in the
  // right-hand inspector panel. This view replaces the prev/next button
  // surface in `Perform` with the wireframe-aligned 3-column layout:
  // playlist sidebar (left) + 10-row cue table (centre) + cue inspector
  // (right). The W6 prev/next + position IPC stays in the Perform view —
  // Playlist v2 is a UI-only surface for W13 (cue persistence is Phase-3
  // EV-PRESET-PLAYLIST scope per docs/26).
  //
  // Per the project memory note `feedback_wireframes_normative_for_ui_ux`,
  // the wireframe `.png` is the visual contract; deviations are deliberate
  // and documented inline. Cue selection and inspector tabs / chips /
  // toggles flip local component state; W14+ will wire IPC persistence
  // when the cue model lands runtime-side.
  //
  // W20 view-internal icon sweep: playlist sidebar row glyphs (`▶` /
  // `☰`) now render via the W19 PR3 vendored `Icon.svelte` primitive
  // (`play` / `list`) per `views/playlist.jsx`'s
  // `<Icon name={active ? "play" : "list"} size={12} />` mapping.
  import { EMPTY_PLAYLIST_CUES } from '../data/empty-states';
  import {
    PLAYLISTS,
    FESTIVAL_SET_CUES,
    type CueFixture,
    type CueTransitionKind,
  } from '../data/playlist-fixtures';
  import EmptyState from '../EmptyState.svelte';
  import Icon from '../Icon.svelte';
  import { CueTransitionCurve } from '../widgets';
  import type { CueKind } from '../widgets/cue-transition-curve.math';

  let selectedPlaylistId = $state<string>('festival-set-dusk');
  // Default selection per the wireframe: row 3 ("First Drop") is the
  // highlighted cue and the inspector panel reflects its values.
  let selectedCueIndex = $state<number>(3);
  let autoAdvance = $state(true);
  let quantizeToBeat = $state(false);
  let loop = $state(true);

  const TRANSITION_TABS: ReadonlyArray<{ id: CueTransitionKind; label: string }> = [
    { id: 'cut', label: 'Cut' },
    { id: 'fade', label: 'Fade' },
    { id: 'snap', label: 'Snap' },
    { id: 'beat-snap', label: 'Beat Snap' },
  ];

  const currentCues = $derived<readonly CueFixture[]>(
    selectedPlaylistId === 'festival-set-dusk' ? FESTIVAL_SET_CUES : [],
  );

  const selectedCue = $derived<CueFixture | undefined>(
    currentCues.find((c) => c.index === selectedCueIndex),
  );

  const activePlaylistName = $derived<string>(
    PLAYLISTS.find((p) => p.id === selectedPlaylistId)?.name ?? '',
  );

  function selectPlaylist(id: string): void {
    selectedPlaylistId = id;
    // Reset to the wireframe-canonical cue default when switching back to
    // the populated playlist; otherwise leave the index as-is (the empty
    // playlists render the EmptyState placeholder, not the inspector).
    if (id === 'festival-set-dusk') {
      selectedCueIndex = 3;
    }
  }

  function selectCue(index: number): void {
    selectedCueIndex = index;
  }

  function setTransition(cue: CueFixture, kind: CueTransitionKind): void {
    // W13 is read-mostly — the per-cue transition mutation is W14+ scope
    // (depends on cue persistence). Local-only echo: clicking a tab
    // selects it visually but does not edit the underlying fixture row.
    // The CueTransitionCurve widget owns its own local state for cp1/cp2/
    // snap/kind and emits onchange; we ignore that payload at this slice.
    void cue;
    void kind;
  }

  // Map the inspector-side 4-way `CueTransitionKind` onto the
  // CueTransitionCurve widget's 3-way `CueKind`. The widget collapses
  // 'beat-snap' onto 'snap' (both render the snap-divisor tile grid);
  // the inspector retains the 4-tab strip per the wireframe.
  function curveKindFor(t: CueTransitionKind): CueKind {
    if (t === 'beat-snap') return 'snap';
    if (t === 'cut') return 'cut';
    if (t === 'snap') return 'snap';
    return 'fade';
  }
</script>

<section class="playlist-view">
  <aside class="playlist-view__sidebar" aria-label="Playlists">
    <header class="playlist-view__sidebar-header">
      <h2>playlists</h2>
      <span class="playlist-view__sidebar-count">{PLAYLISTS.length}</span>
    </header>
    <ul class="playlist-view__list">
      {#each PLAYLISTS as playlist (playlist.id)}
        <li>
          <button
            type="button"
            class="playlist-view__list-btn"
            class:active={playlist.id === selectedPlaylistId}
            aria-current={playlist.id === selectedPlaylistId ? 'true' : undefined}
            onclick={() => selectPlaylist(playlist.id)}
          >
            <span class="playlist-view__list-glyph">
              <Icon name={playlist.id === selectedPlaylistId ? 'play' : 'list'} size={12} />
            </span>
            <span class="playlist-view__list-name">{playlist.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <main class="playlist-view__cues">
    <header class="playlist-view__cues-header">
      <h2>{activePlaylistName}</h2>
      <span class="playlist-view__cues-count">
        {currentCues.length} cue{currentCues.length === 1 ? '' : 's'}
      </span>
    </header>

    {#if currentCues.length === 0}
      <EmptyState title={EMPTY_PLAYLIST_CUES.title} body={EMPTY_PLAYLIST_CUES.body}>
        {#snippet actions()}
          <button type="button" disabled>{EMPTY_PLAYLIST_CUES.primaryAction?.label ?? ''}</button>
          {#if EMPTY_PLAYLIST_CUES.secondaryAction}
            <button type="button" disabled>{EMPTY_PLAYLIST_CUES.secondaryAction.label}</button>
          {/if}
        {/snippet}
      </EmptyState>
    {:else}
      <table class="playlist-view__cue-table">
        <thead>
          <tr>
            <th scope="col">#</th>
            <th scope="col">name</th>
            <th scope="col">scene</th>
            <th scope="col">dur</th>
            <th scope="col">crossfade</th>
            <th scope="col">preset</th>
          </tr>
        </thead>
        <tbody>
          {#each currentCues as cue (cue.index)}
            <tr class:active={cue.index === selectedCueIndex} onclick={() => selectCue(cue.index)}>
              <td class="playlist-view__cue-index">{String(cue.index).padStart(2, '0')}</td>
              <td class="playlist-view__cue-name">{cue.name}</td>
              <td class="playlist-view__cue-scene">{cue.scene}</td>
              <td class="playlist-view__cue-mono">{cue.duration}</td>
              <td class="playlist-view__cue-mono">{cue.crossfade}</td>
              <td class="playlist-view__cue-preset">{cue.controlPreset}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </main>

  <aside class="playlist-view__inspector" aria-label="Cue inspector">
    {#if selectedCue}
      <header class="playlist-view__inspector-header">
        <span class="playlist-view__inspector-eyebrow">inspector</span>
        <h2>{selectedCue.name}</h2>
      </header>

      <dl class="playlist-view__cue-meta">
        <div>
          <dt>scene</dt>
          <dd>{selectedCue.scene}</dd>
        </div>
        <div>
          <dt>duration</dt>
          <dd class="playlist-view__cue-mono">{selectedCue.duration}</dd>
        </div>
        <div>
          <dt>crossfade</dt>
          <dd class="playlist-view__cue-mono">{selectedCue.crossfade}</dd>
        </div>
      </dl>

      <nav class="playlist-view__transition-tabs" aria-label="Transition mode">
        {#each TRANSITION_TABS as tab (tab.id)}
          <button
            type="button"
            class="playlist-view__transition-tab"
            class:on={tab.id === selectedCue.transition}
            aria-pressed={tab.id === selectedCue.transition}
            onclick={() => setTransition(selectedCue, tab.id)}
          >
            {tab.label}
          </button>
        {/each}
      </nav>

      {#if selectedCue.transition !== 'cut'}
        <CueTransitionCurve
          kind={curveKindFor(selectedCue.transition)}
          cp1={[selectedCue.cp1[0], selectedCue.cp1[1]]}
          cp2={[selectedCue.cp2[0], selectedCue.cp2[1]]}
          snap={selectedCue.snap}
          duration={selectedCue.crossfade}
        />
      {/if}

      <section class="playlist-view__preset">
        <h3>control preset</h3>
        <div class="playlist-view__preset-chip">{selectedCue.controlPreset}</div>
      </section>

      <section class="playlist-view__toggles" aria-label="Playlist settings">
        <label class="playlist-view__toggle">
          <input type="checkbox" bind:checked={autoAdvance} />
          <span>auto advance</span>
        </label>
        <label class="playlist-view__toggle">
          <input type="checkbox" bind:checked={quantizeToBeat} />
          <span>quantize to beat</span>
        </label>
        <label class="playlist-view__toggle">
          <input type="checkbox" bind:checked={loop} />
          <span>loop</span>
        </label>
      </section>
    {:else}
      <p class="playlist-view__inspector-empty">select a cue to inspect</p>
    {/if}
  </aside>
</section>

<style>
  /* Token-driven scoped surface. 3-column CSS Grid mirrors the wireframe
   * proportions (sidebar ~200px, cue table flex-1, inspector ~320px). */
  .playlist-view {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr) 320px;
    gap: var(--space-md);
    align-items: stretch;
    min-height: 0;
  }

  /* --- Left: playlists sidebar ------------------------------------------ */
  .playlist-view__sidebar {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-sm);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    min-width: 0;
  }
  .playlist-view__sidebar-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-sm);
    padding: 0 var(--space-xs);
  }
  .playlist-view__sidebar-header h2 {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .playlist-view__sidebar-count {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  .playlist-view__list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: auto;
    min-height: 0;
  }
  .playlist-view__list-btn {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    background: transparent;
    color: var(--color-text-1);
    border: 1px solid transparent;
    border-left: 2px solid transparent;
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    text-align: left;
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease,
      border-color 0.12s ease;
  }
  .playlist-view__list-btn:hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .playlist-view__list-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .playlist-view__list-btn.active {
    background: var(--color-bg-3);
    color: var(--color-primary);
    border-left-color: var(--color-primary);
  }
  .playlist-view__list-glyph {
    font-size: var(--font-size-body-md);
    line-height: 1;
    width: 14px;
    text-align: center;
    color: inherit;
  }
  .playlist-view__list-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* --- Centre: cue table ------------------------------------------------- */
  .playlist-view__cues {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    min-width: 0;
    overflow: auto;
  }
  .playlist-view__cues-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-sm);
  }
  .playlist-view__cues-header h2 {
    margin: 0;
    font-size: var(--font-size-display-sm);
    color: var(--color-text-0);
  }
  .playlist-view__cues-count {
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }
  .playlist-view__cue-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body-sm);
  }
  .playlist-view__cue-table thead th {
    text-align: left;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
    border-bottom: 1px solid var(--color-line-2);
    padding: var(--space-xs) var(--space-sm);
    font-weight: var(--font-weight-label);
  }
  .playlist-view__cue-table tbody tr {
    border-bottom: 1px solid var(--color-line-1);
    cursor: pointer;
    transition:
      background 0.1s ease,
      color 0.1s ease;
  }
  .playlist-view__cue-table tbody tr:nth-child(odd) {
    background: var(--color-bg-1);
  }
  .playlist-view__cue-table tbody tr:hover {
    background: var(--color-bg-3);
  }
  .playlist-view__cue-table tbody tr.active {
    background: var(--color-bg-4);
    color: var(--color-text-0);
    box-shadow: inset 2px 0 0 var(--color-primary);
  }
  .playlist-view__cue-table tbody tr.active .playlist-view__cue-scene,
  .playlist-view__cue-table tbody tr.active .playlist-view__cue-name {
    color: var(--color-primary);
  }
  .playlist-view__cue-table td {
    padding: var(--space-sm);
    color: var(--color-text-1);
    vertical-align: middle;
  }
  .playlist-view__cue-index {
    font-family: var(--font-family-mono);
    color: var(--color-text-3);
    width: 3ch;
  }
  .playlist-view__cue-name {
    color: var(--color-text-0);
  }
  .playlist-view__cue-scene {
    font-family: var(--font-family-mono);
    color: var(--color-text-1);
  }
  .playlist-view__cue-mono {
    font-family: var(--font-family-mono);
    color: var(--color-text-2);
    font-variant-numeric: tabular-nums;
  }
  .playlist-view__cue-preset {
    font-family: var(--font-family-mono);
    color: var(--color-text-2);
  }

  /* --- Right: cue inspector --------------------------------------------- */
  .playlist-view__inspector {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
    overflow: auto;
  }
  .playlist-view__inspector-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .playlist-view__inspector-eyebrow {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .playlist-view__inspector-header h2 {
    margin: 0;
    font-size: var(--font-size-display-sm);
    color: var(--color-text-0);
  }
  .playlist-view__inspector-empty {
    margin: 0;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .playlist-view__cue-meta {
    margin: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: var(--space-md);
    row-gap: var(--space-sm);
  }
  .playlist-view__cue-meta > div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .playlist-view__cue-meta > div:first-child {
    grid-column: span 2;
  }
  .playlist-view__cue-meta dt {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .playlist-view__cue-meta dd {
    margin: 0;
    color: var(--color-text-0);
    font-size: var(--font-size-body-md);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .playlist-view__transition-tabs {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-xs);
  }
  .playlist-view__transition-tab {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease,
      border-color 0.12s ease;
  }
  .playlist-view__transition-tab:hover {
    border-color: var(--color-line-3);
    color: var(--color-text-0);
  }
  .playlist-view__transition-tab.on {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .playlist-view__preset {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .playlist-view__preset h3 {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    font-weight: var(--font-weight-label);
  }
  .playlist-view__preset-chip {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
  }
  .playlist-view__toggles {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    border-top: 1px solid var(--color-line-1);
    padding-top: var(--space-sm);
  }
  .playlist-view__toggle {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    cursor: pointer;
  }
  .playlist-view__toggle input {
    accent-color: var(--color-primary);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }
</style>
