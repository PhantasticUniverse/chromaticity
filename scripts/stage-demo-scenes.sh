#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
#
# stage-demo-scenes — copy the bundled demo `.chroma` corpus from
# engine/scene/tests/fixtures/scenes/ to the platform-default scene
# root resolved by app/shell/src/commands/library.rs::default_scene_root_impl.
#
# Idempotent: existing directories at the destination are left in place
# (the script reports "skip" rather than overwriting). Removing or
# updating staged fixtures is a manual operation; the demo corpus is
# repo-resident and never bundled into release artifacts.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$REPO_ROOT/engine/scene/tests/fixtures/scenes"

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
