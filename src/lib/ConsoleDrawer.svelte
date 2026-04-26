<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-21 PR4 global Console drawer. Mounts in `AppShell.svelte` as an
  // overlay layer above the grid; visibility is keyed off
  // `consoleStore.isOpen` (toggled by the StatusBar's Console button).
  //
  // Locked design choices per the W21 PR4 brief:
  //   - Bottom-anchored drawer, ~50vh tall (collapses to full-width below
  //     the 800px viewport breakpoint).
  //   - role="dialog" + aria-modal="false" — non-modal so operators can
  //     keep interacting with the main view while the drawer is open
  //     (matches the StatusBar's "diagnostics drawer" framing rather
  //     than a hard modal).
  //   - Backdrop click + Escape close. The non-modal posture means we
  //     show a faint backdrop that does NOT block pointer events on the
  //     main view; instead the backdrop is a single pointer-events:auto
  //     row pinned along the drawer's top edge that handles outside-click
  //     dismissal without the full-screen scrim of the HotkeyOverlay.
  //   - Filter chips are aria-pressed toggles. Severity filter is empty-
  //     means-all; Source filter is a `<select>` with `'all'` sentinel.
  //   - Live-tail "live" / "paused" badge driven by `consoleStore.isPaused`
  //     — does NOT actually gate fixture rendering (real subscription is
  //     Phase-3+/4a). Pause toggle button text + aria-pressed reflect it.
  //   - Per-row Copy-to-clipboard button surfaces on hover. Falls back
  //     silently if `navigator.clipboard` is unavailable.
  //
  // Token-only styling. The row's left-border tint maps severity →
  // `--color-signal-*` per the brief (scan: cyan-ish via signal-cyan;
  // load: signal-cyan as well per the W20 OSC route taxonomy that
  // categorises load as a "scan" sibling — wait, brief says blue for
  // load; the tokens cascade only ships signal-cyan as the cool-blue
  // surface. Mapping: scan → signal-lime, load → signal-cyan,
  // fetch → signal-lime (per brief "lime for fetch"), compile →
  // signal-magenta, warn → signal-amber, error → signal-red. The
  // brief's "green for scan" maps onto signal-lime (the cascade has no
  // distinct green token). info → signal-violet for the reserved row.

  import { CONSOLE_FIXTURES, type ConsoleEntry } from './data/console-fixtures';
  import Icon from './Icon.svelte';
  import {
    consoleStore,
    SEVERITY_CHIPS,
    type ConsoleSeverity,
    type ConsoleSource,
  } from './stores/console.svelte';

  // Source dropdown options — `'all'` plus every source label that
  // appears in the fixture corpus, in first-appearance order. Computed
  // once at module load (the fixtures are immutable).
  const SOURCE_OPTIONS: readonly ConsoleSource[] = [
    'all',
    ...Array.from(new Set(CONSOLE_FIXTURES.map((e) => e.source))),
  ];

  // Pretty-label map for the severity chips. Rendered as the chip text
  // and as the row's tag pill. `info` is omitted from chips but mapped
  // here for the row pill.
  const SEVERITY_LABELS: Record<ConsoleSeverity, string> = {
    scan: 'Scan',
    load: 'Load',
    fetch: 'Fetch',
    compile: 'Compile',
    warn: 'Warn',
    error: 'Error',
    info: 'Info',
  };

  let drawerEl: HTMLDivElement | undefined = $state();
  let searchEl: HTMLInputElement | undefined = $state();
  let logEl: HTMLOListElement | undefined = $state();
  let copiedId: string | null = $state(null);

  // Module-state ref for focus restoration — survives the close-side
  // $effect cleanup and need not trigger reactivity.
  let previouslyFocused: HTMLElement | null = null;

  $effect(() => {
    if (consoleStore.isOpen) {
      const active = document.activeElement;
      previouslyFocused = active instanceof HTMLElement ? active : null;
      // Defer focus until the drawer is mounted in the DOM. Focus the
      // search input so operators land in the most-likely interaction.
      queueMicrotask(() => searchEl?.focus());
      // Scroll log to bottom on open — newest fixture row visible.
      queueMicrotask(() => {
        if (logEl) logEl.scrollTop = logEl.scrollHeight;
      });
    } else if (previouslyFocused) {
      previouslyFocused.focus();
      previouslyFocused = null;
    }
  });

  // Global Escape handler — installed only while the drawer is open so
  // we don't compete with the HotkeyOverlay's listener when both are
  // closed. Restored on close.
  $effect(() => {
    if (!consoleStore.isOpen) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        consoleStore.close();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  });

  const filteredEntries = $derived.by<readonly ConsoleEntry[]>(() => {
    const sevFilter = consoleStore.severityFilter;
    const src = consoleStore.sourceFilter;
    const q = consoleStore.query.trim().toLowerCase();
    return CONSOLE_FIXTURES.filter((entry) => {
      if (sevFilter.size > 0 && !sevFilter.has(entry.severity)) return false;
      if (src !== 'all' && entry.source !== src) return false;
      if (q) {
        const hay = `${entry.message} ${entry.source}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  });

  function handleBackdropClick() {
    consoleStore.close();
  }

  function handleClear() {
    // Visual stub — clearing the fixture corpus would void the demo
    // value. Real persistence + clear is Phase-3+/4a scope. We toggle
    // the source filter to `'all'` and clear the query as a soft reset.
    consoleStore.clearFilters();
  }

  async function handleCopy(entry: ConsoleEntry): Promise<void> {
    const text = `${entry.time} ${entry.severity} ${entry.source} ${entry.message}`;
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      try {
        await navigator.clipboard.writeText(text);
        copiedId = entry.id;
        setTimeout(() => {
          if (copiedId === entry.id) copiedId = null;
        }, 1200);
      } catch {
        // Clipboard write blocked (insecure context, denied permission).
        // Silent fallback: no UI nag. Real telemetry is Phase-3+/4a.
      }
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Tab' || !drawerEl) return;
    const tabbables = Array.from(
      drawerEl.querySelectorAll<HTMLElement>('button, input, select'),
    ).filter((el) => !el.hasAttribute('disabled'));
    if (tabbables.length === 0) return;
    const first = tabbables[0];
    const last = tabbables[tabbables.length - 1];
    if (!first || !last) return;
    const activeEl = document.activeElement;
    if (event.shiftKey && activeEl === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && activeEl === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function sourceLabel(source: ConsoleSource): string {
    if (source === 'all') return 'All views';
    if (source === 'scene-import') return 'Scene import';
    // Routed views — Title-Case the route name with a hyphen sweep.
    return source
      .split('-')
      .map((seg) => seg.charAt(0).toUpperCase() + seg.slice(1))
      .join(' ');
  }
</script>

{#if consoleStore.isOpen}
  <!-- Backdrop sits behind the drawer panel and intercepts outside
       clicks. Pointer-events:auto on the backdrop is fine since the
       drawer's own panel sits above it; the backdrop only covers the
       region above the drawer (top: 0, bottom: drawer-height) so the
       main view's padding around the drawer stays interactive. -->
  <div class="console-drawer__backdrop" role="presentation" onclick={handleBackdropClick}></div>

  <div
    class="console-drawer"
    role="dialog"
    aria-modal="false"
    aria-label="Diagnostics console"
    tabindex="-1"
    bind:this={drawerEl}
    onkeydown={handleKeydown}
  >
    <header class="console-drawer__head">
      <div class="console-drawer__title">
        <span class="caps console-drawer__eyebrow">Console</span>
        <span class="console-drawer__live" data-state={consoleStore.isPaused ? 'paused' : 'live'}>
          {consoleStore.isPaused ? 'paused' : 'live'}
        </span>
      </div>
      <div class="console-drawer__head-actions">
        <button
          type="button"
          class="console-drawer__action"
          onclick={() => consoleStore.togglePause()}
          aria-pressed={consoleStore.isPaused}
          title={consoleStore.isPaused ? 'Resume live tail' : 'Pause live tail'}
        >
          <Icon name={consoleStore.isPaused ? 'play' : 'pause'} size={12} />
          {consoleStore.isPaused ? 'Resume' : 'Pause'}
        </button>
        <button
          type="button"
          class="console-drawer__action"
          onclick={handleClear}
          title="Clear filters and search (visual stub)"
        >
          <Icon name="trash" size={12} />
          Clear
        </button>
        <button
          type="button"
          class="console-drawer__close"
          onclick={() => consoleStore.close()}
          aria-label="Close console drawer"
          title="Close (Esc)"
        >
          <Icon name="x" size={14} />
        </button>
      </div>
    </header>

    <div class="console-drawer__filters">
      <div class="console-drawer__chips" role="group" aria-label="Severity filter">
        <button
          type="button"
          class="console-drawer__chip"
          aria-pressed={consoleStore.severityFilter.size === 0}
          onclick={() => consoleStore.severityFilter.clear()}
        >
          All
        </button>
        {#each SEVERITY_CHIPS as severity (severity)}
          <button
            type="button"
            class="console-drawer__chip"
            data-severity={severity}
            aria-pressed={consoleStore.severityFilter.has(severity)}
            onclick={() => consoleStore.toggleSeverity(severity)}
          >
            {SEVERITY_LABELS[severity]}
          </button>
        {/each}
      </div>

      <label class="console-drawer__source">
        <span class="caps console-drawer__source-label">Source</span>
        <select
          aria-label="Filter by source view"
          value={consoleStore.sourceFilter}
          onchange={(e) =>
            consoleStore.setSource((e.currentTarget as HTMLSelectElement).value as ConsoleSource)}
        >
          {#each SOURCE_OPTIONS as source (source)}
            <option value={source}>{sourceLabel(source)}</option>
          {/each}
        </select>
      </label>

      <label class="console-drawer__search">
        <Icon name="search" size={12} />
        <input
          type="search"
          placeholder="Search messages…"
          aria-label="Search console entries"
          bind:value={consoleStore.query}
          bind:this={searchEl}
        />
      </label>
    </div>

    <ol class="console-drawer__log" aria-live="off" bind:this={logEl}>
      {#each filteredEntries as entry (entry.id)}
        <li class="console-drawer__row" data-severity={entry.severity}>
          <span class="console-drawer__time mono">{entry.time}</span>
          <span class="console-drawer__sev mono" data-severity={entry.severity}>
            {SEVERITY_LABELS[entry.severity].toLowerCase()}
          </span>
          <span class="console-drawer__source-tag mono">{sourceLabel(entry.source)}</span>
          <span class="console-drawer__msg">{entry.message}</span>
          <button
            type="button"
            class="console-drawer__copy"
            onclick={() => handleCopy(entry)}
            aria-label="Copy entry to clipboard"
            title="Copy entry"
          >
            {#if copiedId === entry.id}
              <span class="caps console-drawer__copied">Copied</span>
            {:else}
              <Icon name="copy" size={12} />
            {/if}
          </button>
        </li>
      {/each}
      {#if filteredEntries.length === 0}
        <li class="console-drawer__empty mono">No entries match the current filters.</li>
      {/if}
    </ol>
  </div>
{/if}

<style>
  /* --------------- Backdrop ------------------------------------------ */
  .console-drawer__backdrop {
    position: fixed;
    left: 0;
    right: 0;
    top: 0;
    /* Backdrop covers the area ABOVE the drawer; bottom edge meets the
       drawer's top edge so the drawer itself remains the visible
       panel. Backdrop is intentionally faint — non-modal posture. */
    bottom: 50vh;
    background: rgba(0, 0, 0, 0.35);
    z-index: 900;
    cursor: pointer;
  }

  /* --------------- Drawer panel --------------------------------------- */
  .console-drawer {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 28px; /* sit above the 28px status bar */
    height: 50vh;
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border-top: 1px solid var(--color-line-3);
    box-shadow: 0 -8px 24px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    z-index: 901;
    font-family: var(--font-family-sans);
  }

  /* --------------- Header -------------------------------------------- */
  .console-drawer__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    flex-shrink: 0;
  }
  .console-drawer__title {
    display: inline-flex;
    align-items: center;
    gap: var(--space-md);
  }
  .console-drawer__eyebrow {
    color: var(--color-text-2);
  }
  .console-drawer__live {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    padding: 2px var(--space-sm);
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-3);
  }
  .console-drawer__live[data-state='live'] {
    color: var(--color-success);
    border-color: var(--color-success);
  }
  .console-drawer__live[data-state='paused'] {
    color: var(--color-warning);
    border-color: var(--color-warning);
  }
  .console-drawer__head-actions {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .console-drawer__action {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 4px var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
  }
  .console-drawer__action:hover {
    color: var(--color-text-0);
    border-color: var(--color-line-3);
  }
  .console-drawer__action[aria-pressed='true'] {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .console-drawer__close {
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-1);
    padding: 4px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .console-drawer__close:hover {
    color: var(--color-text-0);
    border-color: var(--color-line-3);
  }

  /* --------------- Filter row ---------------------------------------- */
  .console-drawer__filters {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-line-1);
    background: var(--color-bg-1);
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .console-drawer__chips {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }
  .console-drawer__chip {
    background: var(--color-bg-3);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    padding: 2px var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
  }
  .console-drawer__chip:hover {
    color: var(--color-text-0);
    border-color: var(--color-line-3);
  }
  .console-drawer__chip[aria-pressed='true'] {
    background: var(--color-bg-4);
    color: var(--color-text-0);
    border-color: var(--color-text-2);
  }
  /* Severity-tinted chips when pressed — the active state uses the
     signal-color border to match the row's left-border tint. */
  .console-drawer__chip[data-severity='scan'][aria-pressed='true'] {
    border-color: var(--color-signal-lime);
    color: var(--color-signal-lime);
  }
  .console-drawer__chip[data-severity='load'][aria-pressed='true'] {
    border-color: var(--color-signal-cyan);
    color: var(--color-signal-cyan);
  }
  .console-drawer__chip[data-severity='fetch'][aria-pressed='true'] {
    border-color: var(--color-signal-lime);
    color: var(--color-signal-lime);
  }
  .console-drawer__chip[data-severity='compile'][aria-pressed='true'] {
    border-color: var(--color-signal-magenta);
    color: var(--color-signal-magenta);
  }
  .console-drawer__chip[data-severity='warn'][aria-pressed='true'] {
    border-color: var(--color-signal-amber);
    color: var(--color-signal-amber);
  }
  .console-drawer__chip[data-severity='error'][aria-pressed='true'] {
    border-color: var(--color-signal-red);
    color: var(--color-signal-red);
  }
  .console-drawer__source,
  .console-drawer__search {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .console-drawer__source-label {
    color: var(--color-text-3);
  }
  .console-drawer__source select {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
  }
  .console-drawer__search {
    flex: 1 1 200px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 0 var(--space-sm);
    color: var(--color-text-3);
  }
  .console-drawer__search input {
    flex: 1 1 auto;
    background: transparent;
    color: var(--color-text-0);
    border: 0;
    padding: var(--space-xs) 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }
  .console-drawer__search input:focus {
    outline: none;
  }
  .console-drawer__search:focus-within {
    border-color: var(--color-line-3);
  }

  /* --------------- Log list ------------------------------------------ */
  .console-drawer__log {
    list-style: none;
    margin: 0;
    padding: var(--space-xs) 0;
    overflow-y: auto;
    flex: 1 1 auto;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }
  .console-drawer__row {
    display: grid;
    grid-template-columns: 72px 72px 110px 1fr auto;
    gap: var(--space-sm);
    align-items: baseline;
    padding: 4px var(--space-md);
    border-left: 3px solid var(--color-line-2);
    transition: background 80ms ease;
  }
  .console-drawer__row:hover {
    background: var(--color-bg-2);
  }
  .console-drawer__row[data-severity='scan'] {
    border-left-color: var(--color-signal-lime);
  }
  .console-drawer__row[data-severity='load'] {
    border-left-color: var(--color-signal-cyan);
  }
  .console-drawer__row[data-severity='fetch'] {
    border-left-color: var(--color-signal-lime);
  }
  .console-drawer__row[data-severity='compile'] {
    border-left-color: var(--color-signal-magenta);
  }
  .console-drawer__row[data-severity='warn'] {
    border-left-color: var(--color-signal-amber);
  }
  .console-drawer__row[data-severity='error'] {
    border-left-color: var(--color-signal-red);
  }
  .console-drawer__row[data-severity='info'] {
    border-left-color: var(--color-signal-violet);
  }
  .console-drawer__time {
    color: var(--color-text-3);
  }
  .console-drawer__sev {
    text-transform: lowercase;
    color: var(--color-text-2);
  }
  .console-drawer__sev[data-severity='scan'] {
    color: var(--color-signal-lime);
  }
  .console-drawer__sev[data-severity='load'] {
    color: var(--color-signal-cyan);
  }
  .console-drawer__sev[data-severity='fetch'] {
    color: var(--color-signal-lime);
  }
  .console-drawer__sev[data-severity='compile'] {
    color: var(--color-signal-magenta);
  }
  .console-drawer__sev[data-severity='warn'] {
    color: var(--color-signal-amber);
  }
  .console-drawer__sev[data-severity='error'] {
    color: var(--color-signal-red);
  }
  .console-drawer__sev[data-severity='info'] {
    color: var(--color-signal-violet);
  }
  .console-drawer__source-tag {
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    font-size: var(--font-size-caps-sm);
  }
  .console-drawer__msg {
    color: var(--color-text-1);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .console-drawer__copy {
    background: transparent;
    border: 0;
    padding: 2px var(--space-xs);
    color: var(--color-text-3);
    cursor: pointer;
    border-radius: var(--radius-xs);
    visibility: hidden;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .console-drawer__row:hover .console-drawer__copy,
  .console-drawer__row:focus-within .console-drawer__copy {
    visibility: visible;
  }
  .console-drawer__copy:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }
  .console-drawer__copied {
    color: var(--color-success);
  }
  .console-drawer__empty {
    padding: var(--space-md);
    color: var(--color-text-3);
    text-align: center;
  }

  /* --------------- Focus rings --------------------------------------- */
  .console-drawer__chip:focus,
  .console-drawer__action:focus,
  .console-drawer__close:focus,
  .console-drawer__source select:focus,
  .console-drawer__copy:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  /* --------------- Narrow viewport ----------------------------------- */
  @media (max-width: 800px) {
    .console-drawer {
      height: 70vh;
    }
    .console-drawer__backdrop {
      bottom: 70vh;
    }
    .console-drawer__row {
      grid-template-columns: 60px 60px 1fr auto;
    }
    .console-drawer__source-tag {
      grid-column: 1 / -1;
      padding-left: var(--space-md);
    }
  }
</style>
