// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane B.1 — Perform view layout-variant store.
//
// Tracks which Perform layout variant the operator has chosen — `classic`
// (2-column: controls stacked left, preview + scene panel stacked right;
// matches the canonical wireframe + the bundle's
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/perform.jsx`
// `variant === "classic"` branch) or `console` (3-column: controls /
// preview+audio / scenes; the bundle's `variant === "dense"` branch).
//
// Default is `classic` per AO direction surfaced in the 2026-05-11
// pre-alpha smoke audit: the W18 PR1 → W26 PR1 Perform port hardcoded
// the 3-column dense layout, which doesn't match the canonical
// wireframe. STANDARD button activates classic; CONSOLE button activates
// console (mapping to the bundle's `dense` shape).
//
// The choice is persisted to `localStorage` so it survives an app
// restart within the same machine; the persistence write is best-effort
// (privacy-mode browsers can throw on `setItem`).
//
// `.svelte.ts` extension lets the Svelte compiler process `$state`
// outside a `.svelte` component, same as the `routeStore` /
// `helpStore` siblings.

const STORAGE_KEY = 'chromaticity.performVariant';

/** Layout variant for the Perform view.
 *
 * - `classic`: 2-column wireframe layout (controls / preview+scenes).
 *   Default; matches the canonical wireframe.
 * - `console`: 3-column dense layout (controls / preview+audio /
 *   scenes). Maps to the bundle's `dense` variant.
 */
export type PerformVariant = 'classic' | 'console';

const DEFAULT_VARIANT: PerformVariant = 'classic';

function isVariant(value: string): value is PerformVariant {
  return value === 'classic' || value === 'console';
}

function readPersisted(): PerformVariant {
  if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
    return DEFAULT_VARIANT;
  }
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored !== null && isVariant(stored)) {
      return stored;
    }
  } catch {
    // localStorage access can throw under privacy-mode browsers; fall
    // through to the default variant.
  }
  return DEFAULT_VARIANT;
}

function writePersisted(variant: PerformVariant): void {
  if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
    return;
  }
  try {
    window.localStorage.setItem(STORAGE_KEY, variant);
  } catch {
    // Same defensive posture as the read path.
  }
}

class PerformVariantStore {
  current: PerformVariant = $state(readPersisted());

  set(variant: PerformVariant): void {
    if (this.current === variant) {
      return;
    }
    this.current = variant;
    writePersisted(variant);
  }
}

export const performVariantStore = new PerformVariantStore();
