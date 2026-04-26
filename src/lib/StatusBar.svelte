<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-15 PR2 bottom status bar — v0.2 chrome alignment per the
  // canonical bundle JSX in `chromaticity-planning/ui-predev-bundle-v0.2-beta/`
  // (visually confirmed by the v0.2 screenshot pack at
  // `v0.2-ui-wireframe-screenshots/pulse/*.png`).
  //
  // The status bar is a thin strip pinned to the viewport bottom
  // across every routed view. Layout matches the v0.2 chrome:
  //
  //   left  — "Hover any control to see what it does. Press ? for hotkeys."
  //   right — CPU · GPU · MEM · warnings · errors · Console · version
  //
  // Right-side items are placeholders until the corresponding
  // telemetry / IPC sources land:
  //
  //   - CPU% / GPU%             — runtime telemetry IPC (Phase-3+)
  //   - MEM (GB)                — same
  //   - warnings / errors count — read from log subscription (Phase-3+)
  //   - Console                 — opens diagnostics console drawer
  //                                (W15+ when console wires; click is
  //                                 currently a no-op visual stub)
  //   - version                 — build-time `__APP_VERSION__`
  //                                injection target; static `v0.0.0`
  //                                placeholder until vite define plumbs
  //                                the package version through.
  //
  // The W12 theme switcher lives in the Settings view (relocated W14
  // PR3); not duplicated here.
  //
  // W20 view-internal icon sweep: CPU / GPU metric chips pick up the
  // W19 PR3 vendored `Icon.svelte` primitive (`cpu` / `gpu`) per
  // `extras.jsx`'s `<span className="sb-chip"><Icon name="..." size={10} />`
  // mapping.
  import Icon from './Icon.svelte';

  const VERSION = '0.0.0';

  // Placeholder metrics rendered as `—` until real telemetry wires.
  // The values match the v0.2 visual format (CPU 12% · GPU 42% ·
  // 2.1 GB · 0 warnings · 0 errors) so the chrome reads as
  // structurally complete; they are NOT live.
  const CPU_PCT: string | null = null;
  const GPU_PCT: string | null = null;
  const MEM_GB: string | null = null;
  const WARNINGS_COUNT = 0;
  const ERRORS_COUNT = 0;
</script>

<footer class="status-bar" aria-label="Status bar">
  <span class="status-bar__hint">
    Hover any control to see what it does. Press
    <kbd>?</kbd>
    for hotkeys.
  </span>
  <span class="status-bar__right">
    <span class="status-bar__metric" title="CPU utilization (W15+: real telemetry)">
      <Icon name="cpu" size={10} />
      <span class="status-bar__metric-label">CPU</span>
      <span class="status-bar__metric-value">{CPU_PCT ?? '—'}</span>
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span class="status-bar__metric" title="GPU utilization (W15+: real telemetry)">
      <Icon name="gpu" size={10} />
      <span class="status-bar__metric-label">GPU</span>
      <span class="status-bar__metric-value">{GPU_PCT ?? '—'}</span>
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span class="status-bar__metric" title="Memory usage (W15+: real telemetry)">
      <span class="status-bar__metric-value">{MEM_GB ?? '—'}</span>
      <span class="status-bar__metric-label">GB</span>
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span
      class="status-bar__counter"
      data-state={WARNINGS_COUNT > 0 ? 'active' : 'idle'}
      title="Active warnings (W15+: log subscription)"
    >
      {WARNINGS_COUNT} warnings
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span
      class="status-bar__counter"
      data-state={ERRORS_COUNT > 0 ? 'error' : 'idle'}
      title="Active errors (W15+: log subscription)"
    >
      {ERRORS_COUNT} errors
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <button
      type="button"
      class="status-bar__console"
      title="Open diagnostics console (W15+: console drawer)"
    >
      Console
    </button>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span class="status-bar__version" aria-label="Application version">v{VERSION}</span>
  </span>
</footer>

<style>
  /* Token-only styles — no new tokens needed. */
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    background: var(--color-bg-1);
    color: var(--color-text-2);
    border-top: 1px solid var(--color-line-2);
    padding: 0 var(--space-md);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }
  .status-bar__hint {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }
  .status-bar__right {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }
  .status-bar__sep {
    color: var(--color-text-3);
  }
  .status-bar__metric {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
  }
  .status-bar__metric-label {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-3);
    font-size: 10px;
  }
  .status-bar__metric-value {
    color: var(--color-text-1);
    font-variant-numeric: tabular-nums;
  }
  .status-bar__counter {
    color: var(--color-text-3);
  }
  .status-bar__console {
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
  }
  .status-bar__console:hover {
    color: var(--color-text-0);
    text-decoration: underline;
  }
  .status-bar__version {
    color: var(--color-text-3);
    font-variant-numeric: tabular-nums;
  }
  kbd {
    display: inline-block;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    padding: 0 var(--space-xs);
    color: var(--color-text-1);
  }
</style>
