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
// Wave 1d-α PR 3 widened the static catalog to the V1 corpus shipped
// under `docs-corpus/docs/` and built into `dist/help/`. Parity-closure
// wave 1 Lane B (PR 496, 2026-05-22) added 5 more corpus pages
// (quickstart / livestream / best-practices / system-requirements /
// supported-codecs) and mapped them into `bundle.resources`; this store
// surfaces them in the Help-view nav at the same 16-page baseline. The
// list mirrors the `nav:` block in `docs-corpus/mkdocs.yml`; the
// matching `bundle.resources` entries in `app/shell/tauri.conf.json`
// resolve each `htmlPath` at `generate_context!()` time. Dynamic
// enumeration (so the sidebar surfaces new pages without a TypeScript
// edit) is still future scope; the catalog stays a hand-maintained
// source of truth and `helpStore.test.ts` drift-pins it against the
// mkdocs nav order.
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
  { slug: 'quickstart', label: 'Quickstart', htmlPath: 'help/quickstart.html' },
  { slug: 'perform-mode', label: 'Perform Mode', htmlPath: 'help/perform-mode.html' },
  { slug: 'scene-workflow', label: 'Scene Workflow', htmlPath: 'help/scene-workflow.html' },
  { slug: 'library-workflow', label: 'Library Workflow', htmlPath: 'help/library-workflow.html' },
  { slug: 'audio-reactivity', label: 'Audio Reactivity', htmlPath: 'help/audio-reactivity.html' },
  { slug: 'livestream', label: 'Livestreaming', htmlPath: 'help/livestream.html' },
  {
    slug: 'import-shadertoy',
    label: 'Importing Shadertoy Scenes',
    htmlPath: 'help/import-shadertoy.html',
  },
  { slug: 'best-practices', label: 'Best Practices', htmlPath: 'help/best-practices.html' },
  { slug: 'hotkeys-reference', label: 'Hotkeys', htmlPath: 'help/hotkeys-reference.html' },
  { slug: 'osc-routes', label: 'OSC Routes', htmlPath: 'help/osc-routes.html' },
  { slug: 'chroma-uniforms', label: 'Uniforms', htmlPath: 'help/chroma-uniforms.html' },
  { slug: 'settings-reference', label: 'Settings', htmlPath: 'help/settings-reference.html' },
  {
    slug: 'system-requirements',
    label: 'System Requirements',
    htmlPath: 'help/system-requirements.html',
  },
  { slug: 'supported-codecs', label: 'Supported Codecs', htmlPath: 'help/supported-codecs.html' },
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
