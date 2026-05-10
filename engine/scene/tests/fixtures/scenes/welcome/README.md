<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Bundled scenes — `welcome/` subdirectory

This subdirectory holds the one-scene welcome package shown on first launch
per `docs/38-scene-library-and-import-spec.md` §2.1. It ships in the Tauri
AppBundle at `<AppBundle>/Resources/scenes/welcome/`.

## Status

Wave 1d-β PR 1 (2026-05-09) seeded this subdirectory with its first
clean-room scene under `EV-LIBRARY-CORPUS-MIN`. The subdirectory was
established empty at Wave 4b.1 closure under
`EV-LIBRARY-BUNDLE-LAYOUT` (`docs/26-evidence-artifact-registry.md`).

## Scenes

- `welcome.chroma` — first-launch onboarding scene introducing the
  controls palette via a gradient-drift backdrop with three controls
  (`intro_scale` slider, `drift_speed` slider, `accent` color). Mirrors
  the bundle-port `views/onboard.jsx` copy intent at
  `chromaticity-planning/ui-predev-bundle-v0.2-beta/`.

## Clean-room provenance

Every scene in this subdirectory is authored from scratch using only
the chromaticity-internal scene schema (`engine/scene/src/schema.rs`),
ADRs, and `docs/29-cleanroom-legal-policy-baseline.md` §Authoring as
references. Scenes are NOT derived from `Contents/` or any other
reference-app material per the clean-room SOP.

## Walk semantics

Scenes that land here surface through the walk in
`app/shell/src/commands/library.rs::list_bundled_scene_packages`
alongside the `library/` corpus with the same `SceneSource::Bundled`
tag.
