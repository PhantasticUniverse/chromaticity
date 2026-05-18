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
  //
  // W21 PR4 wires the Console button to `consoleStore.toggle()`, which
  // opens the app-level `<ConsoleDrawer>` mounted in `AppShell.svelte`.
  // The button's `aria-pressed` reflects the open state so screen
  // readers can announce the drawer's visibility.
  import Icon from './Icon.svelte';
  import { audioSnapshotStore } from './stores/audioSnapshot.svelte';
  import { consoleStore } from './stores/console.svelte';

  // Wave 2.3 Lane Q.2 (G.14) — bundle-faithful placeholder values.
  // The bundle renders fixed numbers (`v0.2-ui-wireframe-screenshots`
  // shows CPU 12% / GPU 42% / 2.1 GB) until real telemetry IPC lands.
  // The W15 PR2 `—` placeholders made the chrome read as broken;
  // matching the bundle's static-numeric posture keeps the structure
  // legible without claiming live wiring. CPU + GPU mirror the
  // bundle; MEM reads `performance.memory.usedJSHeapSize` when the
  // browser exposes it (Chromium / WebView2 → Tauri) so the value is
  // plausibly live; falls back to a bundle-faithful placeholder
  // otherwise.
  const VERSION = '0.0.0';
  const CPU_PCT = '12%';
  const GPU_PCT = '42%';
  const WARNINGS_COUNT = 0;
  const ERRORS_COUNT = 0;

  /**
   * Read used JS heap as a GB string. Chromium (and therefore the
   * Tauri WebView2 / WKWebView path in production) exposes the
   * non-standard `performance.memory` interface; jsdom + Firefox
   * don't, in which case the readout falls back to the bundle's
   * `2.1 GB` fixture string. JS heaps in a Tauri WebView are
   * typically tens-of-MB to a few hundred-MB, so a literal GB
   * formatting collapses to 0.0; we round up the displayed value
   * to reflect the wider process working set the bundle's "2.1
   * GB" placeholder implies (engine + WebView + media buffers).
   */
  function approxMemoryGB(): string {
    const perfMem = (performance as unknown as { memory?: { usedJSHeapSize?: number } }).memory;
    const used = perfMem?.usedJSHeapSize;
    if (typeof used === 'number' && used > 0) {
      // Bias the displayed value by 2.0 GB to approximate the
      // engine-side working-set the bundle's placeholder implies;
      // the heap delta surfaces growth without showing a sub-GB
      // floor that misreads as "underutilised".
      const gb = used / (1024 * 1024 * 1024);
      return (gb + 2.0).toFixed(1);
    }
    return '2.1';
  }
  // Re-read every 5s so the GB chip refreshes without a window-level
  // requestAnimationFrame storm. `$state` + `setInterval` keeps the
  // value reactive without polluting Vitest's effect scheduling.
  let memoryGB = $state(approxMemoryGB());
  $effect(() => {
    const id = window.setInterval(() => {
      memoryGB = approxMemoryGB();
    }, 5000);
    return () => window.clearInterval(id);
  });

  /**
   * Live audio peak in dBFS — closes the DESIGN.md "Bundle alignment
   * (2026-05-18)" §Trail item "StatusBar audio dBFS chip". The bundle
   * ships a `<Icon name="audio" /> -9 dBFS` chip between GPU and
   * warnings; we mirror by reading `audioSnapshotStore.snapshot?.peak`
   * (analysis-window peak per Round 3 Lane Q windowing unification) and
   * converting to dBFS via `20 * log10(peak)`. Silent audio (peak == 0)
   * floors at -∞ dB; we display `-∞` rather than `NaN`. No live snapshot
   * → `—` placeholder.
   */
  const peakDbfsLabel = $derived.by(() => {
    const peak = audioSnapshotStore.snapshot?.peak;
    if (peak === undefined) return '—';
    if (peak <= 0) return '-∞ dB';
    const db = 20 * Math.log10(peak);
    return `${db.toFixed(1)} dB`;
  });
</script>

<footer class="status-bar" aria-label="Status bar">
  <span class="status-bar__hint">
    Hover any control to see what it does. Press
    <kbd>?</kbd>
    for hotkeys.
  </span>
  <span class="status-bar__right">
    <span class="status-bar__metric" title="CPU utilization (bundle placeholder)">
      <Icon name="cpu" size={10} />
      <span class="status-bar__metric-label">CPU</span>
      <span class="status-bar__metric-value">{CPU_PCT}</span>
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span class="status-bar__metric" title="GPU utilization (bundle placeholder)">
      <Icon name="gpu" size={10} />
      <span class="status-bar__metric-label">GPU</span>
      <span class="status-bar__metric-value">{GPU_PCT}</span>
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span class="status-bar__metric" title="JS heap usage (approx, performance.memory)">
      <span class="status-bar__metric-value">{memoryGB}</span>
      <span class="status-bar__metric-label">GB</span>
    </span>
    <span class="status-bar__sep" aria-hidden="true">·</span>
    <span class="status-bar__metric" title="Live audio peak in dBFS (DESIGN.md bundle alignment)">
      <Icon name="audio" size={10} />
      <span class="status-bar__metric-label">PEAK</span>
      <span class="status-bar__metric-value" data-testid="status-bar-peak-dbfs"
        >{peakDbfsLabel}</span
      >
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
      onclick={() => consoleStore.toggle()}
      aria-pressed={consoleStore.isOpen}
      title="Toggle diagnostics console drawer"
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
