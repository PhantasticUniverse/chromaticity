// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 1d-α PR 1 — in-app docs corpus help store.
//
// Tracks which Help corpus page the user has selected and persists the
// selection across sessions via `localStorage`. Pages are addressed by
// their corpus slug (e.g. `index`, `about`); the resolved bundled-HTML
// path is computed by the Help view at render time via Tauri's
// `convertFileSrc` against the `help/` resource directory bundled by
// `app/shell/tauri.conf.json` `bundle.resources`.
//
// Hardcoded two-entry catalog at PR 1 (matches the PR 1 corpus pages:
// Welcome / About). PR 2+ widens to dynamic enumeration once the real
// content lands. The catalog is exported so the Help view + tests can
// reference the same source-of-truth without duplicating the slug
// list.
//
// `.svelte.ts` extension lets the Svelte compiler process `$state`
// outside a `.svelte` component, same as the route store sibling.

const STORAGE_KEY = 'chromaticity:help:selected-page';

export interface HelpPage {
  readonly slug: string;
  readonly label: string;
  readonly htmlPath: string;
}

export const HELP_PAGES: readonly HelpPage[] = [
  { slug: 'index', label: 'Welcome', htmlPath: 'help/index.html' },
  { slug: 'about', label: 'About', htmlPath: 'help/about.html' },
] as const;

const DEFAULT_SLUG = 'index';

function isKnownSlug(slug: string): boolean {
  return HELP_PAGES.some((page) => page.slug === slug);
}

function readPersistedSlug(): string {
  // Guard for non-DOM contexts (SSR, build-time tooling). The Vitest
  // jsdom environment exposes `window`, so the typical test path lands
  // in the populated branch.
  if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
    return DEFAULT_SLUG;
  }
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored && isKnownSlug(stored)) {
      return stored;
    }
  } catch {
    // localStorage access can throw under privacy-mode browsers; fall
    // through to the default slug.
  }
  return DEFAULT_SLUG;
}

function writePersistedSlug(slug: string): void {
  if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
    return;
  }
  try {
    window.localStorage.setItem(STORAGE_KEY, slug);
  } catch {
    // Same defensive posture as the read path.
  }
}

class HelpStore {
  selectedSlug: string = $state(readPersistedSlug());

  set(slug: string): void {
    if (!isKnownSlug(slug)) {
      return;
    }
    this.selectedSlug = slug;
    writePersistedSlug(slug);
  }

  page(): HelpPage {
    const found = HELP_PAGES.find((p) => p.slug === this.selectedSlug);
    // `selectedSlug` is constructor-validated against `HELP_PAGES` and
    // every mutator goes through `isKnownSlug`, so the find above
    // always resolves. The fallback is defensive only.
    return found ?? HELP_PAGES[0]!;
  }
}

export const helpStore = new HelpStore();
