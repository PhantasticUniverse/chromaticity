<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Bundled scenes — `shadertoy-fixtures/` subdirectory

This subdirectory holds the curated Shadertoy iChannel fixture pack per
`docs/38-scene-library-and-import-spec.md` §5.2a. It ships in the Tauri
AppBundle at `<AppBundle>/Resources/scenes/shadertoy-fixtures/` and is
read by the Shadertoy import flow when the source code references a
known channel asset.

## Status

Empty placeholder. The fixture pack is authored as part of `EV-IMPORT-SHADERTOY`
(`docs/26-evidence-artifact-registry.md` row 64); the taxonomy directory is
established now per `EV-LIBRARY-BUNDLE-LAYOUT` (row 101) so the
four-subdirectory layout exists at Wave 4b.1 closure even though the
fixtures are not yet authored.

## Planned content

Per `docs/38` §5.2a:

- **Cubemaps** — 6 cubemaps × 6 faces = 36 images covering the typical
  cubemap shape Shadertoy authors import (night-sky, interior, art-gallery-
  atrium, studio, street, test-grid). Files:
  `cubemap-<name>-{px,nx,py,ny,pz,nz}.{jpg|png}`. CC0 or
  Chromaticity-original.
- **2D textures** — at least 12 fixtures covering Shadertoy's common 2D
  asset library (noise, tile, rock, wood, abstract, gradient-grid). CC0
  or Chromaticity-original.
- **Name-to-asset mapping** — `manifest.json` mapping the 20 most-common
  Shadertoy channel names and hashes to the local fixture files.
- **License sidecars** — every fixture carries a LICENSE sidecar recording
  origin, author, license text, and hash. `EV-LICENSE-AUDIT` asserts no
  fixture has unresolved provenance.

## Walk semantics

The walk in
`app/shell/src/commands/library.rs::list_bundled_scene_packages` does NOT
recurse into this directory: the iChannel fixtures are import-flow asset
inputs, not user-facing playable scenes. The directory's presence is still
asserted by the bundle-walk taxonomy test under `EV-LIBRARY-BUNDLE-LAYOUT`.
