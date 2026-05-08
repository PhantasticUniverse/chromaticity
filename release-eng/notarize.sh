#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# release-eng/notarize.sh
#
# Wave 1e-alpha scaffold stub for Apple notarization + stapling per
# docs/adr/ADR-014-signing-notarization-auto-update.md Sec.1 (macOS) +
# Sec.5 step 2.
#
# Stub-mode behavior: when APPLE_ID, APPLE_APP_PASSWORD, APPLE_TEAM_ID, or
# BUILD_ARTIFACT_PATH is unset the script prints a stub-mode notice and
# exits 0.
#
# Real-mode behavior: when all four variables are set, the script submits
# the build artifact via `xcrun notarytool submit --wait` then staples the
# resulting ticket via `xcrun stapler staple`.
#
# Production credentials (Apple ID, app-specific password / API key, team
# id) are AO-gated; see release-eng/README.md.

set -euo pipefail

if [[ -z "${APPLE_ID:-}" || -z "${APPLE_APP_PASSWORD:-}" || -z "${APPLE_TEAM_ID:-}" || -z "${BUILD_ARTIFACT_PATH:-}" ]]; then
  echo "release-eng: stub mode (set APPLE_ID, APPLE_APP_PASSWORD, APPLE_TEAM_ID, BUILD_ARTIFACT_PATH for real notarization)"
  exit 0
fi

if [[ ! -e "${BUILD_ARTIFACT_PATH}" ]]; then
  echo "release-eng/notarize.sh: error: BUILD_ARTIFACT_PATH does not exist: ${BUILD_ARTIFACT_PATH}" >&2
  exit 1
fi

echo "release-eng/notarize.sh: submitting ${BUILD_ARTIFACT_PATH} to notarytool (team ${APPLE_TEAM_ID})"
xcrun notarytool submit \
  --apple-id "${APPLE_ID}" \
  --password "${APPLE_APP_PASSWORD}" \
  --team-id "${APPLE_TEAM_ID}" \
  --wait \
  "${BUILD_ARTIFACT_PATH}"

echo "release-eng/notarize.sh: stapling ticket onto ${BUILD_ARTIFACT_PATH}"
xcrun stapler staple "${BUILD_ARTIFACT_PATH}"
