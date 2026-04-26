<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-12 PR3 partial Author view. Mounts the W10 `<Inputs>` Inspector
  // against the W11 hardcoded scene fixtures (NATIVE / COMPAT). The full
  // Author scene editor per `.playwright-mcp/wireframe-reference/02-author-
  // scene-editor.png` (CodeMirror code panes, live preview, scene-load
  // console, LOAD BACKUP / COMPARE & SCENE actions) is W13+; this view
  // is the read-only inspector slice of that surface and exists to give
  // the routed shell a non-trivial Author landing.
  import Inputs from './Inputs.svelte';
  import {
    CHROMA_FIXTURE_NATIVE,
    CHROMA_FIXTURE_COMPAT,
    type ChromaSceneFixture,
  } from '../data/scene-fixtures';

  let selectedFixtureKey = $state<'native' | 'compat'>('native');
  const selectedFixture = $derived<ChromaSceneFixture>(
    selectedFixtureKey === 'native' ? CHROMA_FIXTURE_NATIVE : CHROMA_FIXTURE_COMPAT,
  );
</script>

<section class="author">
  <header class="author__header">
    <h1>author</h1>
    <p class="author__note">
      W12 partial Author view — Inputs Inspector only. The full code-editor pane, live preview, and
      scene-load console land in W13+.
    </p>
  </header>

  <label class="author__picker">
    <span>fixture</span>
    <select bind:value={selectedFixtureKey}>
      <option value="native">native</option>
      <option value="compat">compat</option>
    </select>
  </label>

  <Inputs scene={selectedFixture} />
</section>

<style>
  .author {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    max-width: 48rem;
  }
  .author__header h1 {
    margin: 0 0 var(--space-xs) 0;
  }
  .author__note {
    margin: 0;
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
  }
  .author__picker {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
    align-self: flex-start;
  }
  .author__picker select {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-xs);
  }
</style>
