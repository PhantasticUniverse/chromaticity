#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# stage-demo-scenes — copy the bundled demo `.chroma` corpus from
# engine/scene/tests/fixtures/scenes/library/ to the platform-default
# scene root resolved by
# app/shell/src/commands/library.rs::default_scene_root_impl. Wave-4b.1
# relocated the demo corpus under the `library/` subdirectory of the
# fixtures root per `docs/38-scene-library-and-import-spec.md` §2.1.
#
# Idempotent: existing directories at the destination are left in place
# (the script reports "skip" rather than overwriting). Removing or
# updating staged fixtures is a manual operation.
#
# As of W11 PR2 the three demo `.chroma` packages are also bundled into
# the Tauri AppBundle's Resources/scenes/ directory at link time via
# bundle.resources in app/shell/tauri.conf.json, and surfaced through
# the bundled_scene_root / list_bundled_scene_packages IPC commands.
# This script is now a "force-stage" / dev-override fallback — useful
# for testing the user-staged path explicitly, populating the corpus
# in environments where the Tauri-bundled assets are unavailable, or
# seeding the user root with a writable copy of the demos.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$REPO_ROOT/engine/scene/tests/fixtures/scenes/library"

case "$(uname -s)" in
    Darwin)
        DEST="$HOME/Library/Application Support/Chromaticity/Scenes"
        ;;
    Linux)
        DEST="${XDG_DATA_HOME:-$HOME/.local/share}/chromaticity/scenes"
        ;;
    *)
        printf 'unsupported platform: %s\n' "$(uname -s)" >&2
        exit 1
        ;;
esac

mkdir -p "$DEST"

for fixture in "$FIXTURES"/*.chroma; do
    [ -d "$fixture" ] || continue
    name="$(basename "$fixture")"
    if [ -e "$DEST/$name" ]; then
        printf 'skip: %s already exists\n' "$DEST/$name"
    else
        cp -R "$fixture" "$DEST/$name"
        printf 'staged: %s\n' "$DEST/$name"
    fi
done
