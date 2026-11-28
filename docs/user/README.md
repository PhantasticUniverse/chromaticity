<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# `docs-corpus/` — chromaticity in-app help corpus

Operator runbook for the in-app docs corpus.

## What this is

The static-HTML source tree for chromaticity's in-app embedded
documentation. The corpus is authored under `docs-corpus/docs/`, built
to static HTML with [MkDocs](https://www.mkdocs.org/) + Material
theme, and bundled into the Tauri desktop binary for surfacing through
the Help view (`src/lib/views/Help.svelte`).

The current V1-RC baseline ships an 11-page clean-room corpus covering
Perform mode, scene workflow, library workflow, audio reactivity,
Shadertoy import, hotkeys, OSC routes, uniforms, settings, welcome, and
about content. The Help view surfaces those pages from the bundled
static build output.

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
   `src/lib/stores/helpStore.svelte.ts`; the list is still
   hand-maintained until dynamic enumeration lands.
4. Build locally with `mkdocs build` (see below) and verify the HTML
   output looks correct via `dist/help/index.html`.

## How to build

The build chain is local for authoring changes; CI integration remains
release-eng follow-up work.

```sh
# One-time install (Python toolchain — outside the Rust + Node stack):
pip install mkdocs mkdocs-material

# From the repo root:
cd docs-corpus
mkdocs build

# Output lands at ../dist/help/. The built tree is tracked until CI
# owns the corpus build before `cargo tauri build`.
```

For the desktop build to bundle the Help corpus, run `mkdocs build`
before `cargo tauri build` so the static HTML exists at link time. The
built `dist/help/` tree is committed to the repo because Tauri's
`tauri-build` resource resolver expands `bundle.resources` at link time
and needs every entry to exist on disk. `bundle.resources` lists the
HTML pages plus directory-walk entries for `assets/` (mkdocs-material
CSS/JS) and `search/` (lunr index).

## Where the output goes

- Source: `docs-corpus/docs/*.md`
- Build artifact: `dist/help/*.html` (tracked build output until CI owns
  the corpus build)
- Bundled into Tauri binary at runtime resource path `help/**`
- Surfaced through `src/lib/views/Help.svelte` via Tauri's
  `convertFileSrc` resource resolution.

## Known follow-ups

- Dynamic page enumeration in `Help.svelte` (drop the hardcoded
  page-list so new corpus pages are sidebar-navigable without a
  TypeScript edit).
- Full-text search threaded through to the Help view UI (the lunr
  index ships in the bundle but the sidebar has no search affordance
  yet).
- CI integration: `mkdocs build` invoked from the release-eng workflow
  before `cargo tauri build` so authors stop committing the built tree
  manually.
