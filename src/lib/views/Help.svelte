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

  const RAW_DOCS = import.meta.glob<string>('../../../docs/user/docs/*.md', {
    query: '?raw',
    import: 'default',
    eager: true,
  });

  interface DocBlock {
    readonly kind: 'h1' | 'h2' | 'h3' | 'p' | 'li' | 'code';
    readonly text: string;
  }

  function loadBundledMarkdown(slug: string): readonly DocBlock[] {
    const entry = Object.entries(RAW_DOCS).find(([path]) => path.endsWith(`/${slug}.md`));
    const raw = entry?.[1];
    if (!raw) return [];
    const lines = raw.replace(/^<!--[\s\S]*?-->\s*/u, '').split(/\r?\n/u);
    const blocks: DocBlock[] = [];
    let inCode = false;
    let codeBuf: string[] = [];
    let paraBuf: string[] = [];

    const flushPara = () => {
      if (paraBuf.length > 0) {
        blocks.push({ kind: 'p', text: paraBuf.join(' ').trim() });
        paraBuf = [];
      }
    };

    for (const line of lines) {
      if (line.trimStart().startsWith('```')) {
        flushPara();
        if (inCode) {
          blocks.push({ kind: 'code', text: codeBuf.join('\n') });
          codeBuf = [];
          inCode = false;
        } else {
          inCode = true;
        }
        continue;
      }
      if (inCode) {
        codeBuf.push(line);
        continue;
      }
      const trimmed = line.trim();
      if (!trimmed) {
        flushPara();
        continue;
      }
      if (trimmed.startsWith('### ')) {
        flushPara();
        blocks.push({ kind: 'h3', text: trimmed.slice(4) });
      } else if (trimmed.startsWith('## ')) {
        flushPara();
        blocks.push({ kind: 'h2', text: trimmed.slice(3) });
      } else if (trimmed.startsWith('# ')) {
        flushPara();
        blocks.push({ kind: 'h1', text: trimmed.slice(2) });
      } else if (/^[-*]\s+/u.test(trimmed)) {
        flushPara();
        blocks.push({ kind: 'li', text: trimmed.replace(/^[-*]\s+/u, '') });
      } else {
        paraBuf.push(trimmed);
      }
    }
    flushPara();
    return blocks;
  }

  interface InlineSegment {
    readonly kind: 'text' | 'code' | 'bold' | 'link';
    readonly text: string;
    readonly slug?: string;
  }

  function parseInline(input: string): readonly InlineSegment[] {
    const out: InlineSegment[] = [];
    const tokenRe = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/gu;
    let last = 0;
    for (const match of input.matchAll(tokenRe)) {
      const idx = match.index ?? 0;
      if (idx > last) {
        out.push({ kind: 'text', text: input.slice(last, idx) });
      }
      const raw = match[0];
      if (raw.startsWith('`') && raw.endsWith('`')) {
        out.push({ kind: 'code', text: raw.slice(1, -1) });
      } else if (raw.startsWith('**') && raw.endsWith('**')) {
        out.push({ kind: 'bold', text: raw.slice(2, -2) });
      } else {
        const m = /^\[([^\]]+)\]\(([^)]+)\)$/u.exec(raw);
        if (m) {
          const label = m[1] ?? raw;
          const href = m[2] ?? '';
          const slug = href.endsWith('.md') ? href.replace(/\.md$/u, '') : undefined;
          out.push(slug ? { kind: 'link', text: label, slug } : { kind: 'text', text: label });
        } else {
          out.push({ kind: 'text', text: raw });
        }
      }
      last = idx + raw.length;
    }
    if (last < input.length) {
      out.push({ kind: 'text', text: input.slice(last) });
    }
    return out;
  }

  const fallbackBlocks = $derived(loadBundledMarkdown(helpStore.selectedSlug));

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
      {#if fallbackBlocks.length > 0}
        <article class="help__doc" aria-label={helpStore.page().label}>
          {#each fallbackBlocks as block, i (i)}
            {#if block.kind === 'h1'}
              <h2 class="help__doc-h1">{block.text}</h2>
            {:else if block.kind === 'h2'}
              <h3 class="help__doc-h2">{block.text}</h3>
            {:else if block.kind === 'h3'}
              <h4 class="help__doc-h3">{block.text}</h4>
            {:else if block.kind === 'li'}
              <p class="help__doc-li">
                <span class="help__doc-bullet" aria-hidden="true">•</span>
                <span>
                  {#each parseInline(block.text) as seg, j (j)}
                    {#if seg.kind === 'code'}
                      <code class="help__doc-inline-code">{seg.text}</code>
                    {:else if seg.kind === 'bold'}
                      <strong>{seg.text}</strong>
                    {:else if seg.kind === 'link' && seg.slug}
                      <button
                        type="button"
                        class="help__doc-link"
                        onclick={() => helpStore.set(seg.slug!)}
                      >
                        {seg.text}
                      </button>
                    {:else}
                      {seg.text}
                    {/if}
                  {/each}
                </span>
              </p>
            {:else if block.kind === 'code'}
              <pre class="help__doc-code"><code>{block.text}</code></pre>
            {:else}
              <p class="help__doc-p">
                {#each parseInline(block.text) as seg, j (j)}
                  {#if seg.kind === 'code'}
                    <code class="help__doc-inline-code">{seg.text}</code>
                  {:else if seg.kind === 'bold'}
                    <strong>{seg.text}</strong>
                  {:else if seg.kind === 'link' && seg.slug}
                    <button
                      type="button"
                      class="help__doc-link"
                      onclick={() => helpStore.set(seg.slug!)}
                    >
                      {seg.text}
                    </button>
                  {:else}
                    {seg.text}
                  {/if}
                {/each}
              </p>
            {/if}
          {/each}
        </article>
      {:else}
        <div class="help__placeholder" role="status">
          <div class="help__fallback-card">
            <h2 class="help__fallback-title">Quickstart &amp; Essential Booth Hotkeys</h2>
            <ul class="help__hotkey-list">
              <li><kbd>?</kbd> Toggle full Hotkey Reference overlay</li>
              <li><kbd>⌘K</kbd> / <kbd>Ctrl+K</kbd> Open Command Palette</li>
              <li><kbd>Space</kbd> Play / Pause active Playlist</li>
              <li><kbd>B</kbd> Toggle Blackout · Hold <kbd>S</kbd> for momentary Strobe</li>
              <li><kbd>1</kbd>–<kbd>8</kbd> Recall Snap slots on Perform</li>
            </ul>
          </div>
        </div>
      {/if}
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
  .help__fallback-card {
    max-width: 520px;
    width: 100%;
    text-align: left;
    padding: var(--space-xl);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .help__fallback-title {
    margin: 0;
    font-size: var(--font-size-body-lg, 1rem);
    color: var(--color-text-0);
  }
  .help__hotkey-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }
  .help__hotkey-list kbd {
    display: inline-block;
    min-width: 24px;
    padding: 1px 6px;
    margin-right: 6px;
    text-align: center;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-primary);
  }
  .help__doc {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2xl) var(--space-3xl);
    max-width: 72ch;
    color: var(--color-text-1);
    line-height: 1.6;
    font-size: var(--font-size-body-sm);
  }
  .help__doc-h1 {
    margin: 0 0 var(--space-md);
    font-family: var(--font-family-display);
    font-size: var(--font-size-display-md);
    color: var(--color-text-0);
  }
  .help__doc-h2 {
    margin: var(--space-xl) 0 var(--space-sm);
    font-size: var(--font-size-display-sm);
    color: var(--color-text-0);
    border-bottom: 1px solid var(--color-line-1);
    padding-bottom: 4px;
  }
  .help__doc-h3 {
    margin: var(--space-md) 0 var(--space-xs);
    font-size: var(--font-size-body-md);
    color: var(--color-text-0);
  }
  .help__doc-p {
    margin: 0 0 var(--space-sm);
  }
  .help__doc-li {
    margin: 0 0 6px;
    padding-left: var(--space-sm);
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .help__doc-bullet {
    color: var(--color-info);
    flex-shrink: 0;
  }
  .help__doc-inline-code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-1);
    color: var(--color-info);
    white-space: nowrap;
  }
  .help__doc-link {
    background: transparent;
    border: none;
    padding: 0;
    font: inherit;
    color: var(--color-info);
    text-decoration: underline;
    cursor: pointer;
  }
  .help__doc-link:hover {
    color: var(--color-primary);
  }
  .help__doc-code {
    margin: var(--space-sm) 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    overflow-x: auto;
    color: var(--color-info);
  }
</style>
