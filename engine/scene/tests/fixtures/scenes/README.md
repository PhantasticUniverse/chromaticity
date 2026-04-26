<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

# Demo `.chroma` corpus

This directory holds the bundled clean-room demo scene corpus used by Wave-10
critical-path work. Each `<name>.chroma/` directory carries a `scene.json`
that satisfies the slice-3a `Scene` deserializer in
`engine/scene/src/schema.rs` and the `docs/38-scene-library-and-import-spec.md`
§2.2 metadata fields (`TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`, `CATEGORIES`).

## Provenance

The fixtures and this README were authored clean-room by Xavier Butler on
2026-04-26 per `docs/29-cleanroom-legal-policy-baseline.md`. No prose, control
names, default values, palette tokens, or category labels were copied from the
reference app, ISF, Shadertoy, or any third-party scene corpus. Category
spellings (`BUILT-IN`, `ABSTRACT`, `GEOMETRIC`, `AUDIO REACTIVE`, `TEXT`) match
the wireframe-01 chip set, which is itself clean-room-coined. Each `scene.json`
falls under the repo-default REUSE annotation (`LicenseRef-Chromaticity-Proprietary`,
`2026 Chromaticity`); JSON cannot carry an inline SPDX header.

## Consumers

- `app/shell/src/commands/library.rs` tests — un-ignored in Wave-10 PR1; the
  fixture-backed coverage exercises `chromaticity_scene::loader::load_chroma_dir`
  on `demo-kaleido.chroma` directly without staging under `$HOME`.
- `engine/scene/tests/import.rs` — Wave-10 PR3 conformance harness; closes
  `EV-SCENE-IMPORT` (T2) over the corpus.
- `scripts/stage-demo-scenes.sh` — developer-facing idempotent copy of the
  three packages to the platform-default scene root.

## Adding new fixtures

1. Create `<name>.chroma/scene.json` in this directory.
2. Make sure the JSON deserializes via `chromaticity_scene::loader::parse_scene_json`
   (every wire key in `engine/scene/src/schema.rs` is honored; unknown root keys
   flatten into `Scene::extensions`).
3. Carry the §2.2 metadata block (`TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`,
   `CATEGORIES`) so library tooling has something to surface.
4. Re-run `cargo test -p chromaticity-scene` to refresh the loader-level
   goldens, then `cargo test -p chromaticity-app` for the library-handler
   coverage.
