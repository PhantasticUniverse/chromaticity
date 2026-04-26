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
  // placeholder disc per `EV-LIBRARY-THUMBNAIL` (Phase-4b) and the search
  // / filter surface per `EV-LIBRARY-DISCOVERY` row 40 lands in W9 PR3.
  import { EMPTY_LIBRARY } from '../data/empty-states';
  import EmptyState from '../EmptyState.svelte';
  import type { SceneCard } from '../ipc/bindings';
  import { fetchSceneCards, getDefaultRoot } from '../stores/library';

  let cards = $state<SceneCard[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const root = await getDefaultRoot();
        cards = await fetchSceneCards(root);
      } catch (err) {
        error = err instanceof Error ? err.message : String(err);
      } finally {
        loading = false;
      }
    })();
  });
</script>

<section class="library">
  <header class="library__header">
    <h2>library</h2>
    {#if !loading && cards.length > 0}
      <span class="library__count">{cards.length} scene{cards.length === 1 ? '' : 's'}</span>
    {/if}
  </header>

  {#if loading}
    <p class="library__status">loading…</p>
  {:else if error}
    <p class="library__error">error: {error}</p>
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
    <div class="library__grid">
      {#each cards as card (card.id)}
        <article class="scene-tile">
          <div class="scene-tile__thumbnail" aria-hidden="true">
            <!-- Placeholder thumbnail; EV-LIBRARY-THUMBNAIL is Phase-4b. -->
            <div class="scene-tile__thumbnail-disc"></div>
          </div>
          <div class="scene-tile__meta">
            <h3 class="scene-tile__title">{card.title || card.id}</h3>
            {#if card.credit}
              <p class="scene-tile__credit">{card.credit}</p>
            {/if}
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  /* Token-styled scoped CSS. Grid uses CSS directly; ControlGroup is a
   * widget-row primitive and is intentionally NOT reused here. */
  .library {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .library__header {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
  }
  .library__count {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .library__status {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .library__error {
    color: var(--color-signal-red);
    font-size: var(--font-size-body-sm);
  }
  .library__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-md);
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
  .scene-tile__title {
    margin: 0;
    font-size: var(--font-size-body-md);
    color: var(--color-text-0);
  }
  .scene-tile__credit {
    margin: 0;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
</style>
