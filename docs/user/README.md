<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# `docs/user/` — Chromaticity In-App Help Corpus

Operator runbook for the in-app user documentation corpus.

## What this is

The Markdown source tree for Chromaticity's embedded user documentation. Pages
are authored under `docs/user/docs/` using the
[Diátaxis](https://diataxis.fr/) taxonomy (`tutorial`, `howto`, `reference`,
`explanation`), built to static HTML with [MkDocs](https://www.mkdocs.org/) +
Material theme into `resources/help/`, and bundled into the Tauri desktop
binary for surfacing through the Help view (`src/lib/views/Help.svelte`).

The current pre-alpha corpus ships 16 clean-room pages covering Quick Start,
Perform mode, Scene & Library workflows, Audio reactivity, Live streaming,
Shadertoy import, Hotkeys, OSC routes, Chroma uniforms, Settings, System
requirements, Supported codecs, Best practices, Welcome, and About.

## Clean-room attestation

Per `docs/governance/29-cleanroom-legal-policy-baseline.md`, every page
authored under `docs/user/docs/` MUST be original Chromaticity prose.

- Do NOT copy from `.local/Contents/` or any reference application's embedded
  documentation.
- Do NOT copy from `.local/chromaticity-planning/` — the planning archive is
  source-of-ideas only, not paste-through prose.
- Cross-reference Chromaticity's own normative docs (`docs/governance/*.md`,
  `docs/adr/*.md`, `docs/architecture/*.md`) freely; those are the canonical
  authoring inputs.

PRs landing corpus content append a clean-room attestation line to the PR body.
Pages carry inline SPDX headers via HTML comments and a Diátaxis `doc-type:`
YAML front-matter tag (both stripped at render time).

## How to author

1. Edit existing `.md` files under `docs/user/docs/` or add new ones with a
   `doc-type:` front-matter field (`tutorial | howto | reference | explanation`).
2. Add new pages to the `nav:` block in `docs/user/mkdocs.yml`.
3. Add a corresponding entry to the Help-view page list in
   `src/lib/stores/helpStore.svelte.ts`.
4. Build locally with `npm run build:help` (or `just build-help`) and verify
   the HTML output in `resources/help/index.html`.

## How to build

```sh
# One-time install (Python toolchain — outside the Rust + Node stack):
pip install mkdocs mkdocs-material

# From the repo root:
npm run build:help
# (or: just build-help)
```

Output lands in `resources/help/` (decoupled from Vite's `dist/` output so
`npm run build` never clobbers the help bundle). Tauri's `bundle.resources`
maps `../../resources/help/**` into the runtime resource path `help/**`.

## Where the output goes

- Source: `docs/user/docs/*.md`
- Build artifact: `resources/help/*.html` (built via `mkdocs build --strict`)
- Bundled into Tauri binary at runtime resource path `help/**`
- Surfaced through `src/lib/views/Help.svelte` via Tauri's `convertFileSrc`
  resource resolution (with live Markdown fallback in browser dev mode).
