// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Drift pin: `HELP_PAGES` must stay in step with the mkdocs `nav:`
// block in `docs-corpus/mkdocs.yml` and the `help/*.html` mappings in
// `app/shell/tauri.conf.json` `bundle.resources`. When a new corpus
// page lands, three places must move together — this pin catches an
// out-of-sync `HELP_PAGES` at test time so the Help view never falls
// silently behind the bundled corpus the way the 11/16 gap did between
// Wave 1d-α PR 3 and parity-closure wave 1 Lane B.

import { describe, expect, it } from 'vitest';

import { HELP_PAGES } from './helpStore.svelte';

// The expected order mirrors the `nav:` block in
// `docs/user/mkdocs.yml` exactly (Welcome -> Tutorials -> How-to Guides ->
// Reference -> Explanation). When you add a page to the mkdocs nav, you must
// also add it to `HELP_PAGES`, to `app/shell/tauri.conf.json` `bundle.resources`,
// and to this expected list.
const EXPECTED_SLUGS_IN_NAV_ORDER = [
  'index',
  'quickstart',
  'perform-mode',
  'scene-workflow',
  'library-workflow',
  'audio-reactivity',
  'livestream',
  'import-shadertoy',
  'hotkeys-reference',
  'osc-routes',
  'chroma-uniforms',
  'settings-reference',
  'system-requirements',
  'supported-codecs',
  'best-practices',
  'about',
] as const;

describe('HELP_PAGES drift pin', () => {
  it('matches the mkdocs nav slug order at the documented 16-page baseline', () => {
    expect(HELP_PAGES.map((p) => p.slug)).toEqual(EXPECTED_SLUGS_IN_NAV_ORDER);
  });

  it('every slug is unique', () => {
    const slugs = HELP_PAGES.map((p) => p.slug);
    expect(new Set(slugs).size).toBe(slugs.length);
  });

  it('every htmlPath matches the `help/<slug>.html` convention', () => {
    for (const page of HELP_PAGES) {
      expect(page.htmlPath).toBe(`help/${page.slug}.html`);
    }
  });

  it('every page carries a non-empty label', () => {
    for (const page of HELP_PAGES) {
      expect(page.label.length).toBeGreaterThan(0);
    }
  });
});
