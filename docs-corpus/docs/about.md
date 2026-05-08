<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# About this Help corpus

The Chromaticity Help corpus is a static documentation site built with
[MkDocs](https://www.mkdocs.org/) using the Material theme. The corpus
is authored under `docs-corpus/docs/` in the chromaticity source tree,
built locally with `mkdocs build`, and emitted to `dist/help/` for
inclusion in the desktop application bundle.

## Authoring posture

All Help corpus content is original chromaticity prose. Per the
clean-room policy (`docs/29-cleanroom-legal-policy-baseline.md`), no
text, structure, or per-page taxonomy is derived from any reference
application's embedded documentation. Authors who land corpus content
attest to clean-room provenance in the PR body.

## Build chain

The `mkdocs build` step runs locally before `cargo tauri build` so that
the static HTML output exists at link time and is bundled into the
desktop binary. CI does not currently run `mkdocs build`; that
integration is Wave 1d-β release-eng scope.

This is the second placeholder page that exists to validate routing
between corpus pages. Future waves replace this content with a real
About / Version / Changelog page.
