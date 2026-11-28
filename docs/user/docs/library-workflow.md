<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Library Workflow

The Chromaticity library is the operator's home for browsing, searching,
and launching scenes. It indexes scenes from three disjoint sources and
presents them through one logical browser.

## Scene sources

The library merges three sources into a single result set:

- **Bundled scenes** — read-only packages shipped inside the application
  bundle under `<AppBundle>/Resources/scenes/`.
- **User scenes** — scenes under the active user-data scenes folder
  (`<UserDataRoot>/Scenes/`). Operators may register additional folders
  through Settings; results merge into the same logical library.
- **Author workspace** — scenes currently open in the in-app authoring
  environment.

## Bundled scenes — the four-subdirectory taxonomy

The bundled scene root is exactly the four subdirectories named below.
Discovery never walks any other path under the bundle's `Resources/`:

| Subdirectory | Purpose |
| --- | --- |
| `welcome/` | Single-package first-launch onboarding scene |
| `library/` | Full Chromaticity-original clean-room corpus |
| `examples/` | Minimal demonstration scenes for each authoring feature (multi-pass, script, transitions, media, audio-reactive) |
| `shadertoy-fixtures/` | iChannel asset fixtures consumed by Shadertoy import — not user-launchable scenes |

The `shadertoy-fixtures/` subdirectory is intentionally excluded from
the launcher; its contents back the import path described in
[Scene Workflow](scene-workflow.md).

## Per-scene metadata

Each scene exposes the following metadata to the library browser, drawn
from `scene.json` and the package's filesystem state:

- `TITLE` — display name; falls back to the package directory name when
  absent.
- `CREDIT` — author attribution.
- `DESCRIPTION` — short prose summary, surfaced in the detail pane.
- `IMAGE_PATH` — relative path to the canonical thumbnail; see the
  thumbnail-fallback chain below.
- `TAGS` and legacy `CATEGORIES` — searchable / filterable token lists.
- Package path, last-modified time, scene-format tag.

The format tag is one of `native-chroma-scene`, `compat-syn-scene`,
`shadertoy-v1-import`, or `isf-import`. Format tags are catalog-only —
they are distinct from the runtime compatibility profile each scene
also carries (`compat-syn-v1` or `native-chroma-v1`).

## Thumbnails

When a scene declares an explicit `IMAGE_PATH`, the library loads it
directly. Otherwise the loader walks a deterministic fallback chain:

1. `<package>/<package_stem>.png` (matching the package directory name).
2. An auto-generated preview captured at frame 60 with default
   audio-silent state.
3. A format-family fallback icon shipped under
   `<AppBundle>/Resources/thumbnails/` — one per scene-format tag.

Thumbnails regenerate on operator request through the scene context
menu and are captured with a deterministic seeded RNG so re-renders
match across runs.

## Browser capabilities

The library view supports both grid and list presentation. From either:

- Full-text search across `TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`, and
  `CATEGORIES`.
- Filter by compat profile, by tag, by presence of `script.js`, by
  presence of bundled media.
- Sort by title, credit, last-modified, tag relevance, or
  author-defined order.
- Drag a scene into the playlist editor from any view.
- Keyboard navigation — arrow keys move the selection,
  `Enter` launches the selected scene, `Shift+Enter` launches with a
  preset prompt. See [Hotkey Reference](hotkeys-reference.md) for the
  full vocabulary.

## Launching a scene

Single-click selects a scene; double-click or `Enter` launches it.
The launch target depends on the active transport mode:

- In **playlist** mode, launching a scene replaces the active playlist
  slot and follows the slot's transition descriptor.
- In **instant-scene** mode, launching cuts directly to the scene with
  the operator's chosen transition.

Optional preset launch (`Shift+Enter`) prompts for a Snap-slot name;
the default Snap is slot zero ("gold preset"). Snap slots are the
operator-facing name for per-scene quick-recall presets — see the
[Hotkey Reference](hotkeys-reference.md) for the keyboard mappings and
[OSC Routes](osc-routes.md) for the wire-level surface (where the
literal `favslots` is retained for `compat-syn-v1` compatibility).

## Indexing posture

Library discovery is filesystem-backed. The indexer rescans on operator
request and on filesystem-change events; for large user-scene folders,
the rescan is incremental against the cached prior index. The search
and filter surface in the operator HUD reads from this index over
in-process IPC, not from a re-walk of disk on every keystroke.

---

<!-- Clean-room authored from docs/38-scene-library-and-import-spec.md sections 2-4, app/shell/src/commands/library.rs, app/shell/src/commands/library_index.rs, app/shell/src/commands/library_search.rs, and the EV-LIBRARY-BUNDLE-LAYOUT + EV-LIBRARY-DISCOVERY rows in docs/26-evidence-artifact-registry.md; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
