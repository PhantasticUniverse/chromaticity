<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Three-theme switcher. Wave-5 shipped the `[data-theme]` overrides in
  // src/lib/tokens/tokens.css; this widget exercises them. Pulse clears
  // `data-theme` so the :root cascade default applies, which avoids a
  // phantom `data-theme="pulse"` attribute that no rule matches.
  //
  // Wave 2.7 Lane SET.A — theme persistence unification. Previously
  // session-scoped (`$state<Theme>('pulse')` reset on reload); now reads /
  // writes `settingsStore.snapshot.general.theme` via the same
  // `get_settings` / `set_settings` IPC seam the rest of the Settings
  // form uses. localStorage carries a fast-path fallback so the very
  // first paint can pick up the operator's previous choice before the
  // async store load resolves; once the store is hydrated, the engine-
  // persisted value is the source of truth.
  //
  // Dual-mode posture mirrors `audioSnapshot.svelte.ts`: when running
  // outside Tauri (Vitest jsdom, `npm run dev` vite-only mode) the
  // settingsStore.load() promise rejects silently and the snapshot stays
  // at the bundle-faithful default; the widget still drives the
  // `data-theme` attribute via the local-state path so existing tests
  // continue to pass without an IPC mock.
  import type { ThemePayload } from './ipc/bindings/ThemePayload';
  import { settingsStore } from './stores/settings.svelte';

  type Theme = 'pulse' | 'atelier' | 'console';

  const LOCAL_STORAGE_KEY = 'chromaticity.theme';

  function payloadToWire(payload: ThemePayload): Theme {
    // Engine ThemePayload variants are PascalCase ('Pulse' / 'Atelier' /
    // 'Console'); the CSS [data-theme] cascade keys on lower-case
    // ('pulse' / 'atelier' / 'console'). Translate at this edge.
    switch (payload) {
      case 'Pulse':
        return 'pulse';
      case 'Atelier':
        return 'atelier';
      case 'Console':
        return 'console';
    }
  }

  function wireToPayload(theme: Theme): ThemePayload {
    switch (theme) {
      case 'pulse':
        return 'Pulse';
      case 'atelier':
        return 'Atelier';
      case 'console':
        return 'Console';
    }
  }

  function readLocalStorage(): Theme | null {
    try {
      const raw = globalThis.localStorage?.getItem(LOCAL_STORAGE_KEY);
      if (raw === 'pulse' || raw === 'atelier' || raw === 'console') return raw;
    } catch {
      // localStorage may throw in privacy / SSR contexts; fall through.
    }
    return null;
  }

  function writeLocalStorage(theme: Theme): void {
    try {
      globalThis.localStorage?.setItem(LOCAL_STORAGE_KEY, theme);
    } catch {
      // Same swallow as the read side.
    }
  }

  // Initial value: localStorage fast-path → settingsStore snapshot →
  // hard-coded 'pulse' default. localStorage covers the no-Tauri /
  // pre-load window; settingsStore takes over once load() resolves.
  const initialTheme: Theme =
    readLocalStorage() ?? payloadToWire(settingsStore.snapshot.general.theme);

  // Drive the document attribute off settingsStore.snapshot reactively
  // so the store-side value (loaded from disk or updated via setTheme
  // below) flows back into the cascade. Hydrate the store with the
  // localStorage fast-path value so the first paint shows the operator's
  // previous choice while the async load() is in flight.
  if (initialTheme !== payloadToWire(settingsStore.snapshot.general.theme)) {
    settingsStore.update({ general: { theme: wireToPayload(initialTheme) } });
  }

  const theme = $derived<Theme>(payloadToWire(settingsStore.snapshot.general.theme));

  function setTheme(next: Theme): void {
    writeLocalStorage(next);
    settingsStore.update({ general: { theme: wireToPayload(next) } });
  }

  $effect(() => {
    document.documentElement.dataset.theme = theme === 'pulse' ? '' : theme;
  });
</script>

<label class="theme-switcher">
  <span>theme</span>
  <select value={theme} onchange={(e) => setTheme(e.currentTarget.value as Theme)}>
    <option value="pulse">pulse</option>
    <option value="atelier">atelier</option>
    <option value="console">console</option>
  </select>
</label>

<style>
  .theme-switcher {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
  }
  .theme-switcher select {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-xs);
  }
</style>
