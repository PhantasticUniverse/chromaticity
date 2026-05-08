#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# release-eng/stage-appcast.sh
#
# Wave 1e-alpha scaffold stub for the Sparkle 2 appcast staging step per
# docs/adr/ADR-014-signing-notarization-auto-update.md Sec.2 (macOS) +
# Sec.5 step 6.
#
# Stub-mode behavior: when APPCAST_FEED_URL, RELEASE_VERSION, or
# BUILD_ARTIFACT_PATH is unset the script prints a stub-mode notice and
# exits 0.
#
# Real-mode behavior (target shape): when all three variables are set, the
# script will compute the EdDSA Sparkle signature of the artifact (via
# Sparkle's `sign_update` tool resolved through SPARKLE_TOOLS_PATH or PATH),
# emit an appcast.xml entry pointing at APPCAST_FEED_URL with the new
# version + signature, and exit 0. Real-mode wiring is intentionally minimal
# at this scaffold revision; the EdDSA signing-key surface lives behind the
# AO-gated key-rotation procedure in the security runbook (ADR-014 Sec.4).
#
# EdDSA-only posture per ADR-014 Sec.2: no DSA fallback chain ever ships.

set -euo pipefail

if [[ -z "${APPCAST_FEED_URL:-}" || -z "${RELEASE_VERSION:-}" || -z "${BUILD_ARTIFACT_PATH:-}" ]]; then
  echo "release-eng: stub mode (set APPCAST_FEED_URL, RELEASE_VERSION, BUILD_ARTIFACT_PATH for real appcast staging)"
  exit 0
fi

if [[ ! -e "${BUILD_ARTIFACT_PATH}" ]]; then
  echo "release-eng/stage-appcast.sh: error: BUILD_ARTIFACT_PATH does not exist: ${BUILD_ARTIFACT_PATH}" >&2
  exit 1
fi

echo "release-eng/stage-appcast.sh: staging appcast entry for ${RELEASE_VERSION} -> ${APPCAST_FEED_URL}"
echo "release-eng/stage-appcast.sh: real-mode EdDSA signing + XML emission deferred to AO-gated production wiring (ADR-014 Sec.4)"
exit 0
