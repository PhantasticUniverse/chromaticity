<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Changelog

All notable changes to Chromaticity are documented here in reverse
chronological order. The format is based on
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/);
versioning follows the compatibility-profile matrix in
`docs/architecture/22-compatibility-versioning-policy.md`, not SemVer-as-such.

Release automation (`release-please` per
`docs/governance/43-development-standards.md` §11.1) populates per-release
sections from Conventional Commits on merges to `main`. Manual
edits above `## [Unreleased]` are bot-driven.

## [Unreleased]

Pre-alpha desktop application and Rust engine workspace under active
development. Highlights since initial scaffolding:

### Added
- Rust workspace crates under `engine/` (audio, authoring, core, entitlement,
  library, media, observability, runtime, scene), `integrations/` (MIDI, OSC,
  NDI, Syphon), and `app/` (Tauri 2 `shell`, `ui-bridge`).
- Svelte 5 desktop UI covering Perform, Mix, Library, Author, Playlist,
  Sources, MIDI, OSC, Import, Patch, Settings, Audio Uniforms, License, and
  embedded Help.
- 16-page Diátaxis in-app user documentation corpus (`docs/user/`) built via
  MkDocs Material into `resources/help/`.
- Bundled production scenes (`resources/scenes/`), root `justfile`, and
  unified `docs/{adr,user,architecture,governance,research,archive}/` layout.

### Changed
- Standardized audio-reactive uniform naming to native `chroma_*` identifiers
  across UI, docs, and engine surfaces.
- Decoupled bundled help and scene resources under `resources/` from Vite's
  `dist/` build output and engine unit-test fixtures.
- Live multi-pass scenes now run on a persistent GPU pipeline
  (`CachedMultipass`): passes compile once per scene and resolution, and
  `PERSISTENT`/`FEEDBACK` pass history accumulates across live frames instead
  of being re-simulated from black (capped at 4 iterations) on every frame.
