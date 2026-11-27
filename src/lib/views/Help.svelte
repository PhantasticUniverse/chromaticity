<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave 1d-α Help view.
  //
  // Surfaces the 11-page in-app docs corpus (built from
  // `docs-corpus/` to `dist/help/` via mkdocs and bundled into the
  // Tauri binary at resource path `help/**` per
  // `app/shell/tauri.conf.json` `bundle.resources`). The static
  // sidebar is populated from `helpStore` `HELP_PAGES`; dynamic page
  // enumeration and full-text search remain follow-up work.
  //
  // The iframe approach matches Tauri 2's recommended posture for
  // displaying bundled static HTML inside the app: the asset protocol
  // serves files from the bundle's resource directory through a
  // sandboxed origin, isolating the corpus content from the host UI's
  // CSP without coupling to the runtime / IPC layer.
  //
  // The startup wallclock when the bundled corpus is missing (operator
  // forgot to run `mkdocs build` before `cargo tauri build`) renders
  // a graceful fallback message — better than a blank iframe that
  // leaves the operator confused about whether anything is wired up.
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { resolveResource } from '@tauri-apps/api/path';

  import { helpStore, HELP_PAGES } from '../stores/helpStore.svelte';

  let resolvedSrc = $state<string | null>(null);
  let resolveError = $state<string | null>(null);

  $effect(() => {
    // Re-resolve whenever the user picks a different page. The
    // `helpStore.selectedSlug` read inside the effect makes the
    // dependency tracking explicit.
    const page = helpStore.page();
    let cancelled = false;
    resolveError = null;
    resolveResource(page.htmlPath)
      .then((path) => {
        if (cancelled) return;
        resolvedSrc = convertFileSrc(path);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const detail = err instanceof Error ? err.message : String(err);
        resolveError = detail;
        resolvedSrc = null;
      });
    return () => {
      cancelled = true;
    };
  });
</script>

<section class="help">
  <aside class="help__sidebar" aria-label="Help pages">
    <h1 class="help__title">Help</h1>
    <ul class="help__pagelist">
      {#each HELP_PAGES as page (page.slug)}
        <li>
          <button
            type="button"
            class="help__pagelink"
            class:active={helpStore.selectedSlug === page.slug}
            aria-current={helpStore.selectedSlug === page.slug ? 'page' : undefined}
            onclick={() => helpStore.set(page.slug)}
          >
            {page.label}
          </button>
        </li>
      {/each}
    </ul>
  </aside>
  <div class="help__content">
    {#if resolveError}
      <div class="help__placeholder" role="status">
        <p>The Help corpus was not found in the application bundle.</p>
        <p class="help__placeholder-detail">
          Run <code>mkdocs build</code> from <code>docs-corpus/</code> before
          <code>cargo tauri build</code>; see <code>docs-corpus/README.md</code>.
        </p>
      </div>
    {:else if resolvedSrc}
      <iframe
        class="help__iframe"
        src={resolvedSrc}
        title={helpStore.page().label}
        sandbox="allow-same-origin allow-popups"
      ></iframe>
    {:else}
      <div class="help__placeholder" role="status">
        <p>Loading Help…</p>
      </div>
    {/if}
  </div>
</section>

<style>
  .help {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: var(--space-lg);
    height: 100%;
    min-height: 0;
  }
  .help__sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-lg);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow-y: auto;
  }
  .help__title {
    margin: 0;
    font-family: var(--font-family-display);
    font-size: var(--font-size-display-sm);
    color: var(--color-text-0);
  }
  .help__pagelist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .help__pagelink {
    width: 100%;
    text-align: left;
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    color: var(--color-text-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease,
      border-color 0.12s ease;
  }
  .help__pagelink:hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .help__pagelink:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .help__pagelink.active {
    background: var(--color-bg-3);
    color: var(--color-primary);
    border-color: var(--color-primary);
  }
  .help__content {
    display: flex;
    flex-direction: column;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    min-height: 0;
  }
  .help__iframe {
    flex: 1;
    width: 100%;
    border: none;
    background: var(--color-bg-0);
  }
  .help__placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-md);
    padding: var(--space-2xl);
    color: var(--color-text-2);
    text-align: center;
  }
  .help__placeholder-detail {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
</style>
