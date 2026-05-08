#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# release-eng/sign-windows.sh
#
# Wave 1e-alpha scaffold stub for the Windows Authenticode signing step of
# the release pipeline per docs/adr/ADR-014-signing-notarization-auto-update.md
# Sec.1 (Windows).
#
# Stub-mode behavior: when WINDOWS_CERT_PATH or BUILD_ARTIFACT_PATH is unset
# the script prints a stub-mode notice and exits 0.
#
# Real-mode behavior: when WINDOWS_CERT_PATH and BUILD_ARTIFACT_PATH are
# set, the script invokes `signtool` with the SHA-256 + timestamp posture
# mandated by ADR-014 Sec.1 Windows. The timestamp server URL is taken from
# WINDOWS_TIMESTAMP_URL with a default of DigiCert's RFC-3161 endpoint.
#
# Production cert procurement (Windows EV code-signing certificate from a
# trusted CA) is AO-gated work happening outside this scaffold; see
# release-eng/README.md.

set -euo pipefail

WINDOWS_TIMESTAMP_URL="${WINDOWS_TIMESTAMP_URL:-http://timestamp.digicert.com}"

if [[ -z "${WINDOWS_CERT_PATH:-}" || -z "${BUILD_ARTIFACT_PATH:-}" ]]; then
  echo "release-eng: stub mode (set WINDOWS_CERT_PATH and BUILD_ARTIFACT_PATH for real signing)"
  exit 0
fi

if [[ ! -e "${BUILD_ARTIFACT_PATH}" ]]; then
  echo "release-eng/sign-windows.sh: error: BUILD_ARTIFACT_PATH does not exist: ${BUILD_ARTIFACT_PATH}" >&2
  exit 1
fi

echo "release-eng/sign-windows.sh: signing ${BUILD_ARTIFACT_PATH} via ${WINDOWS_TIMESTAMP_URL}"
signtool sign \
  /tr "${WINDOWS_TIMESTAMP_URL}" \
  /td sha256 \
  /fd sha256 \
  /a \
  "${BUILD_ARTIFACT_PATH}"
