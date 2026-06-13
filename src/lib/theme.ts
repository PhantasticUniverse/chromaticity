// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// App-level theme application. Hoisted out of `ThemeSwitcher.svelte`
// so the persisted theme applies at startup regardless of route —
// previously the only writer of `document.documentElement.dataset
// .theme` was a `$effect` inside the Settings-mounted ThemeSwitcher,
// so an operator who persisted Atelier / Console got Pulse on every
// launch until they navigated to Settings.
//
// Ownership split:
//   * `AppShell.svelte` calls `applyStartupTheme()` at component init
//     (before first paint) and hosts the `$effect` that calls
//     `syncThemeFromSettings()` so the snapshot theme drives the
//     document attribute app-wide.
//   * `ThemeSwitcher.svelte` keeps the `<select>` that writes the
//     choice through `settingsStore.update()` (+ the localStorage
//     mirror via `writeStoredTheme`).
//   * localStorage is strictly a pre-load paint hint + mirror of the
//     engine-persisted value. It is seeded into the snapshot via
//     `settingsStore.seedHint()` (no dirty-tracking, no persistence)
//     so a successful `load()` always wins — a stale localStorage
//     value can never be persisted over the fresher disk value.

import type { ThemePayload } from './ipc/bindings/ThemePayload';
import { settingsStore } from './stores/settings.svelte';

/** Lower-case wire names the `[data-theme]` CSS cascade keys on. */
export type ThemeName = 'pulse' | 'atelier' | 'console';

export const THEME_STORAGE_KEY = 'chromaticity.theme';

export function themePayloadToName(payload: ThemePayload): ThemeName {
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

export function themeNameToPayload(name: ThemeName): ThemePayload {
  switch (name) {
    case 'pulse':
      return 'Pulse';
    case 'atelier':
      return 'Atelier';
    case 'console':
      return 'Console';
  }
}

export function readStoredTheme(): ThemeName | null {
  try {
    const raw = globalThis.localStorage?.getItem(THEME_STORAGE_KEY);
    if (raw === 'pulse' || raw === 'atelier' || raw === 'console') return raw;
  } catch {
    // localStorage may throw in privacy / SSR contexts; fall through.
  }
  return null;
}

export function writeStoredTheme(name: ThemeName): void {
  try {
    globalThis.localStorage?.setItem(THEME_STORAGE_KEY, name);
  } catch {
    // Same swallow as the read side.
  }
}

/** Write the document-level theme attribute. Pulse clears `data-theme`
 * so the `:root` cascade default applies, which avoids a phantom
 * `data-theme="pulse"` attribute that no rule matches. */
export function applyThemeToDocument(name: ThemeName): void {
  document.documentElement.dataset.theme = name === 'pulse' ? '' : name;
}

/**
 * Startup fast-path: apply the localStorage theme hint to the
 * document before the async `settingsStore.load()` resolves, and seed
 * it into the snapshot (hint-only — no dirty-tracking, no save) so
 * the ThemeSwitcher `<select>` and the app-level applier agree while
 * the load is in flight. The loaded disk value replaces the hint.
 */
export function applyStartupTheme(): void {
  const stored = readStoredTheme();
  if (stored !== null) {
    applyThemeToDocument(stored);
    settingsStore.seedHint({ general: { theme: themeNameToPayload(stored) } });
  }
}

/**
 * Snapshot → document sync. Called from an `$effect` in
 * `AppShell.svelte`; the reactive read of
 * `settingsStore.snapshot.general.theme` inside re-arms the effect.
 * Also writes the localStorage mirror so the stored hint can never
 * stay permanently divergent from the engine-persisted value (the
 * pre-fix flash-and-revert launch loop).
 */
export function syncThemeFromSettings(): void {
  const theme = themePayloadToName(settingsStore.snapshot.general.theme);
  applyThemeToDocument(theme);
  writeStoredTheme(theme);
}
