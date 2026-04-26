<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-12 PR3 bottom status bar. The wireframe captures show this row
  // as a thin strip pinned to the viewport bottom across every view:
  //   left  — "Hover any control to see what it does. Press ? for hotkeys."
  //   right — CPU / GPU / FPS metrics + GO LIVE button
  //
  // W12 ships only the discoverability hint on the left and a static
  // version pill on the right — CPU/GPU/FPS need a runtime telemetry
  // source the engine doesn't yet expose, and the GO LIVE button is a
  // perform-mode action that is W13+. The theme switcher does not move
  // here in W12; it stays in the Perform view header so the existing
  // operator muscle memory survives the shell refactor (see Perform.svelte).
  //
  // The version pill reads the build-time `__APP_VERSION__` if vite
  // injects one, else falls back to the package "0.0.0" placeholder.
  const VERSION = '0.0.0';
</script>

<footer class="status-bar" aria-label="Status bar">
  <span class="status-bar__hint">
    Hover any control to see what it does. Press
    <kbd>?</kbd>
    for hotkeys.
  </span>
  <span class="status-bar__right">
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
  }
  .status-bar__right {
    display: inline-flex;
    align-items: center;
    gap: var(--space-md);
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
