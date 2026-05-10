<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# `docs-corpus/` — chromaticity in-app help corpus

Operator runbook for the Wave 1d-α docs corpus rails.

## What this is

The static-HTML source tree for chromaticity's in-app embedded
documentation. The corpus is authored under `docs-corpus/docs/`, built
to static HTML with [MkDocs](https://www.mkdocs.org/) + Material
theme, and bundled into the Tauri desktop binary for surfacing through
the Help view (`src/lib/views/Help.svelte`).

This wave (Wave 1d-α PR 1) ships the rails only — `mkdocs.yml`, two
placeholder pages, the Help view, and the build-output wiring. Real
corpus content authoring is PR 2+ scope.

## Clean-room attestation

Per `docs/29-cleanroom-legal-policy-baseline.md`, every page authored
under `docs-corpus/docs/` MUST be original chromaticity prose.

- Do NOT copy from `Contents/Resources/views/embedded_docs/` or any
  reference application's embedded documentation.
- Do NOT copy from `chromaticity-planning/` — the planning archive is
  source-of-ideas only, not paste-through prose.
- Cross-reference chromaticity's own normative docs (`docs/*.md`,
  `docs/adr/*.md`) freely; those are the canonical authoring inputs.

PRs landing corpus content append a clean-room attestation line to the
PR body. The corpus directory is also covered by the chromaticity
proprietary REUSE annotation; pages carry inline SPDX headers via HTML
comments (the markdown rendering pipeline strips them at build time).

## How to author

1. Edit existing `.md` files under `docs-corpus/docs/` or add new ones.
2. Add new pages to the `nav:` block in `docs-corpus/mkdocs.yml`.
3. Add a corresponding entry to the Help-view page list in
   `src/lib/views/Help.svelte` (PR 1 hardcodes two entries; PR 3+
   expands to dynamic enumeration).
4. Build locally with `mkdocs build` (see below) and verify the HTML
   output looks correct via `dist/help/index.html`.

## How to build

The build chain is local-only at PR 1; CI integration is Wave 1d-β
release-eng scope.

```sh
# One-time install (Python toolchain — outside the Rust + Node stack):
pip install mkdocs mkdocs-material

# From the repo root:
cd docs-corpus
mkdocs build

# Output lands at ../dist/help/. The `dist/.gitkeep` sentinel survives;
# the help/ subdir is gitignored alongside the rest of dist/*.
```

For the desktop build to bundle the Help corpus, run `mkdocs build`
before `cargo tauri build` so the static HTML exists at link time.
At Wave 1d-α PR 2, the built `dist/help/` tree is committed to the
repo verbatim — Tauri's `tauri-build` resource resolver expands
`bundle.resources` at link time and needs every entry to exist on
disk; committing the built tree is the simplest forward-compatible
posture until Wave 1d-β CI integration lands and `mkdocs build` runs
in CI ahead of `cargo tauri build`. The PR 1 `PLACEHOLDER.txt`
sentinel has been replaced with explicit per-page entries in
`bundle.resources` (six HTML pages + 404 + sitemap pair) plus
directory-walk entries for `assets/` (mkdocs-material CSS/JS) and
`search/` (lunr index).

## Where the output goes

- Source: `docs-corpus/docs/*.md`
- Build artifact: `dist/help/*.html` (gitignored — see `.gitignore`)
- Bundled into Tauri binary at runtime resource path `help/**`
- Surfaced through `src/lib/views/Help.svelte` via Tauri's
  `convertFileSrc` resource resolution.

## Known follow-ups (Wave 1d-α PR 3+)

- Continued corpus content authoring beyond the PR 2 first wave
  (scene format, MIDI, troubleshooting, settings reference, library
  workflow, perform mode, onboarding). Target ≈ 92 pages to match
  the V1-RC content gap.
- Dynamic page enumeration in `Help.svelte` (drop the hardcoded
  two-entry page-list so the new pages from PR 2 are sidebar-
  navigable). PR 2 ships the pages in the bundle but the Help-view
  sidebar continues to surface only Welcome / About until the store
  widens.
- Full-text search threaded through to the Help view UI (the lunr
  index ships in PR 2's bundle but the sidebar has no search
  affordance yet).
- Wave 1d-β CI integration: `mkdocs build` invoked from the
  release-eng workflow before `cargo tauri build` so authors stop
  committing the built tree manually.
- `EV-DOCS-CORPUS` row at `docs/26` advances from `InProgress` to
  `Pass` once full corpus expansion + CI integration land.
