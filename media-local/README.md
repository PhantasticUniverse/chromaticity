<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# media-local/

Drop your video, image, and scene-still assets here for local-only
Media-tab testing. Anything in this folder (except this README and
`.gitignore`) is gitignored so you can use materials you don't own
the rights to commit.

## How to enable

In the running app:

- Settings → Storage → Add Custom Media Folder
- Pick this directory (`<repo>/media-local/`)
- The Settings TOML store persists the path under
  `storage.media_folders` (Wave 2.7 SET.B persistence)

Note: as of Wave 2.8, the engine does not yet scan custom media
folders — Perform's media tab still reads the `MEDIA_CARDS` fixture
shipped with the v0.2-beta UI bundle. The `storage.media_folders`
array is persisted end-to-end, so when the media-indexer wave lands
(post-Wave-2.8 scope) the operator workflow is already wired and the
custom paths flow straight through to the Media surface.

This README + the persistence path are scaffolded ahead of the
media indexer so the operator workflow stays consistent with
`scenes-local/`.

## Supported asset shapes

When the media indexer lands the expected layout is flat:

```
media-local/
  my-clip.mp4
  my-image.jpg
  my-scene-still.png
  …
```

Exact format support is post-Wave-2.8 scope. Today the directory
exists as a scaffold so the workflow is documented and the
`storage.media_folders` Settings entry has a target to point at.

## What's gitignored

Everything in this directory except this README and the local
`.gitignore` sentinel. The repo-root `.gitignore` enforces the same
rule at the project level (belt-and-suspenders) so an accidental
`git add -A` from the repo root cannot stage operator content.
