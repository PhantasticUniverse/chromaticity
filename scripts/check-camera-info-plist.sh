#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# scripts/check-camera-info-plist.sh
#
# Asserts that a bundled chromaticity macOS .app contains the
# `NSCameraUsageDescription` key in its Contents/Info.plist after
# `tauri build` per ADR-020 §5 + `docs/54-wave-1c-beta-research-lessons.md`
# Phase-2 lesson 5.
#
# Tauri 2's plist merging has historically been finicky around
# bundle-resource overrides; multiple Tauri community issues report that
# Info.plist key overrides specified in `tauri.conf.json` (or merged from
# a sibling `Info.plist`) silently drop on certain config-file shapes.
# Missing `NSCameraUsageDescription` causes the macOS TCC subsystem to
# skip the consent dialog entirely — the OS denies access without
# surfacing a UI, leaving the app in a permission-denied state with no
# operator-visible explanation.
#
# Usage:
#   bash scripts/check-camera-info-plist.sh /path/to/Chromaticity.app
#
# Exit codes:
#   0 — `NSCameraUsageDescription` present in the bundled Info.plist.
#   1 — key missing OR plist-extract failed.
#   2 — incorrect invocation (missing argument, bundle path not found,
#       not a macOS host).
#
# This script is reviewer-checklist-grade for Phase 2 PR 2; AO can wire
# it into a release-validation step (e.g. `tauri build` post-hook or a
# release CI workflow) at their discretion. The script itself is a
# one-shot binary check (key present / absent) and does NOT validate the
# copy text — the operator-facing copy is reviewed manually for App
# Store compliance.

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "ERROR: $(basename "$0") only runs on macOS hosts" >&2
  echo "       (host: $(uname -s); the bundled Info.plist surface is" \
       "macOS-only)" >&2
  exit 2
fi

if [[ $# -lt 1 ]]; then
  echo "usage: $(basename "$0") <path-to-bundled-.app>" >&2
  echo "" >&2
  echo "Example: bash scripts/check-camera-info-plist.sh \\" >&2
  echo "  target/release/bundle/macos/Chromaticity.app" >&2
  exit 2
fi

APP_PATH="$1"
INFO_PLIST="${APP_PATH}/Contents/Info.plist"

if [[ ! -d "${APP_PATH}" ]]; then
  echo "ERROR: bundle path not found: ${APP_PATH}" >&2
  exit 2
fi

if [[ ! -f "${INFO_PLIST}" ]]; then
  echo "ERROR: Info.plist not found in bundle: ${INFO_PLIST}" >&2
  exit 1
fi

# `plutil -extract NSCameraUsageDescription raw <plist>` exits non-zero
# if the key is absent, and prints the value (a string for this key) to
# stdout if present. The `raw` output format strips XML wrapper bytes
# and surfaces the raw string value.
if ! VALUE="$(plutil -extract NSCameraUsageDescription raw "${INFO_PLIST}" 2>/dev/null)"; then
  echo "ERROR: NSCameraUsageDescription key missing from ${INFO_PLIST}" >&2
  echo "" >&2
  echo "  Tauri 2 may have silently dropped the Info.plist merge per" >&2
  echo "  docs/54-wave-1c-beta-research-lessons.md Phase-2 lesson 5." >&2
  echo "  Verify app/shell/Info.plist exists and is referenced by" >&2
  echo "  bundle.macOS.infoPlist in app/shell/tauri.conf.json." >&2
  exit 1
fi

if [[ -z "${VALUE}" ]]; then
  echo "ERROR: NSCameraUsageDescription is present but empty in ${INFO_PLIST}" >&2
  exit 1
fi

echo "OK: NSCameraUsageDescription present in ${INFO_PLIST}"
echo "    value: ${VALUE}"
exit 0
