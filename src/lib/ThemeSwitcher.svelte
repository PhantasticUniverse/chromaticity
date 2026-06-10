<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Three-theme switcher. Wave-5 shipped the `[data-theme]` overrides in
  // src/lib/tokens/tokens.css; this widget exercises them. Pulse clears
  // `data-theme` so the :root cascade default applies, which avoids a
  // phantom `data-theme="pulse"` attribute that no rule matches.
  //
  // Wave 2.7 Lane SET.A — theme persistence unification: reads / writes
  // `settingsStore.snapshot.general.theme` via the same `get_settings` /
  // `set_settings` IPC seam the rest of the Settings form uses.
  //
  // Startup-theme fix: app-level theme application (the localStorage
  // fast-path hint + the snapshot → document sync) now lives in
  // `src/lib/theme.ts`, driven from `AppShell.svelte`, so the persisted
  // theme applies at launch without opening Settings. This widget keeps
  // only the `<select>` (writing through `settingsStore.update()` + the
  // localStorage mirror) plus its own snapshot → document `$effect` so
  // it stays self-sufficient when rendered standalone (tests, no-Tauri
  // dev mode). Both appliers derive from the same snapshot field, so
  // they can never disagree. Crucially the widget no longer seeds the
  // store from localStorage at init — a stale localStorage value must
  // never be persisted over the fresher disk value (the engine-persisted
  // value is the source of truth once the store hydrates).
  import { settingsStore } from './stores/settings.svelte';
  import {
    applyThemeToDocument,
    themeNameToPayload,
    themePayloadToName,
    writeStoredTheme,
    type ThemeName,
  } from './theme';

  const theme = $derived<ThemeName>(themePayloadToName(settingsStore.snapshot.general.theme));

  function setTheme(next: ThemeName): void {
    writeStoredTheme(next);
    settingsStore.update({ general: { theme: themeNameToPayload(next) } });
  }

  $effect(() => {
    applyThemeToDocument(theme);
  });
</script>

<label class="theme-switcher">
  <span>theme</span>
  <select value={theme} onchange={(e) => setTheme(e.currentTarget.value as ThemeName)}>
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
