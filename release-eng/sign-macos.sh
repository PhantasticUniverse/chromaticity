#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# release-eng/sign-macos.sh
#
# Wave 1e-alpha scaffold stub for the macOS code-signing step of the
# release pipeline per docs/adr/ADR-014-signing-notarization-auto-update.md
# Sec.1 (macOS).
#
# Stub-mode behavior: when the required environment variables are unset
# this script prints a stub-mode notice and exits 0 so that callers (CI
# matrices, local dry-runs, downstream pipeline scripts) can compose the
# scaffold without real Developer ID material.
#
# Real-mode behavior: when MACOS_CERT_NAME, APPLE_TEAM_ID, and
# BUILD_ARTIFACT_PATH are all set, the script invokes `codesign` with the
# hardened-runtime + timestamp + entitlements posture mandated by ADR-014
# Sec.1 macOS.
#
# Production cert procurement (Apple Developer ID Application certificate,
# hardware-stored on YubiKey or equivalent FIPS-140-2 key) is AO-gated work
# happening outside this scaffold; see release-eng/README.md.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENTITLEMENTS_PATH="${SCRIPT_DIR}/entitlements.plist"

if [[ -z "${MACOS_CERT_NAME:-}" || -z "${APPLE_TEAM_ID:-}" || -z "${BUILD_ARTIFACT_PATH:-}" ]]; then
  echo "release-eng: stub mode (set MACOS_CERT_NAME, APPLE_TEAM_ID, BUILD_ARTIFACT_PATH for real signing)"
  exit 0
fi

if [[ ! -f "${ENTITLEMENTS_PATH}" ]]; then
  echo "release-eng/sign-macos.sh: error: entitlements file not found at ${ENTITLEMENTS_PATH}" >&2
  exit 1
fi

if [[ ! -e "${BUILD_ARTIFACT_PATH}" ]]; then
  echo "release-eng/sign-macos.sh: error: BUILD_ARTIFACT_PATH does not exist: ${BUILD_ARTIFACT_PATH}" >&2
  exit 1
fi

echo "release-eng/sign-macos.sh: signing ${BUILD_ARTIFACT_PATH} with team ${APPLE_TEAM_ID}"
codesign \
  --force \
  --options runtime \
  --timestamp \
  --sign "${MACOS_CERT_NAME}" \
  --entitlements "${ENTITLEMENTS_PATH}" \
  "${BUILD_ARTIFACT_PATH}"
