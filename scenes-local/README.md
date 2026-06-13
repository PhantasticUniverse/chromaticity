<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

# scenes-local/

Drop your `.chroma` scene packages here for local-only Library testing.
Anything in this folder (except this README and `.gitignore`) is
gitignored so you can use materials you don't own the rights to
commit.

## How to enable

In the running app:

- Settings → Storage → Add Custom Scene Folder
- Pick this directory (`<repo>/scenes-local/`)
- The Settings TOML store persists the path under
  `storage.scene_folders` (Wave 2.7 SET.B persistence)
- The Library auto-loads scenes from this path on next mount + on
  Refresh Index (Wave 2.8 LIB.LOCAL custom-folder scan)

Cards from this folder render in the Library grid tagged
`MY SCENES` (`SceneSource::User`) alongside the bundled corpus.

## Example structure

Each scene is a directory ending in `.chroma`:

```
scenes-local/
  my-demo.chroma/
    scene.json
    fragment.glsl
    …
```

See `engine/scene/tests/fixtures/scenes/library/demo-rings.chroma/`
for a complete example of a `.chroma` package; the loader contract
is documented in `docs/38-scene-library-and-import-spec.md` §2.

## What's gitignored

Everything in this directory except this README and the local
`.gitignore` sentinel. The repo-root `.gitignore` enforces the same
rule at the project level (belt-and-suspenders) so an accidental
`git add -A` from the repo root cannot stage operator content.

## Reference shaders for render-testing (clean-room)

This folder is also the home for **verbatim** copies of third-party /
reference shaders (Synesthesia material from `Contents/`, real MilkDrop,
ShaderToy, …) used to render-test the engine/render pipeline and for
technique _inspiration_.

Per the AO: _"We can copy (not modify) whole shaders in for dev/testing
but these should not ever be committed."_

- **Copy verbatim, never modify** the reference shader's expression — keep
  it unambiguously identifiable as reference material, not our work. A thin
  `.chroma` wrapper to run an unmodified body in the renderer is fine;
  editing the shader body is not.
- **Never commit** them — the gitignore above enforces this; do not
  force-add.
- **Inspiration, not expression.** Our committed shaders are authored
  clean-room: learn _ideas / techniques_ from references here, but never
  copy their _expression_ into first-party
  (`LicenseRef-Chromaticity-Proprietary`) shaders.
- **Firewall (`docs/29`):** whoever studies a reference here should not be
  the one authoring our committed original for the same effect
  (reference-reader ≠ author).

## Dev-mode convenience

Pre-populating `settings.storage.scene_folders` with this path
automatically when launching in dev mode is a small ergonomic
improvement that was considered for this wave and deferred — the
operator path through Settings → Storage → Add Custom Scene Folder
is the canonical flow and works identically in dev and release
builds. If you frequently re-add this folder during development,
add it once via the Settings UI and it will persist across restarts
via the TOML store.
