#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# trailer-check: validate the mandatory trailers required by
# docs/43-development-standards.md Sec.4.5 on the commit referenced by
# $1 (default: HEAD).
#
# Required trailers on every merged commit on `main`:
#   Signed-off-by: <Full Name> <email>
#   Profile-Impact: none | compat-syn-v1-minor | compat-syn-v1-major
#                 | native-chroma-v1-minor | native-chroma-v1-major
#
# Architecture-impacting commits additionally require:
#   ADR: ADR-NNN
#   Evidence: EV-<ID>[, EV-<ID>...]
# (This script does not classify PRs as architecture-impacting; CI does
#  that via labels. Here we only enforce the two always-required trailers.)
#
# Exit code 0 = pass; non-zero = fail with a printed diagnosis.

set -u

ref="${1:-HEAD}"

if ! msg="$(git log -1 --format='%B' "$ref" 2>/dev/null)"; then
  echo "[trailer-check] cannot read commit message for $ref"
  exit 2
fi

fail=0

if ! printf '%s\n' "$msg" | grep -qE '^Signed-off-by: .+ <.+@.+>$'; then
  echo "[trailer-check] $ref: missing Signed-off-by trailer (DCO)"
  fail=1
fi

profile_line="$(printf '%s\n' "$msg" | grep -E '^Profile-Impact:' | head -n1 || true)"
if [ -z "$profile_line" ]; then
  echo "[trailer-check] $ref: missing Profile-Impact trailer"
  fail=1
else
  profile_value="$(printf '%s' "$profile_line" | sed -E 's/^Profile-Impact:[[:space:]]*//; s/[[:space:]]+$//')"
  case "$profile_value" in
    none \
      | compat-syn-v1-minor | compat-syn-v1-major \
      | native-chroma-v1-minor | native-chroma-v1-major) ;;
    *)
      echo "[trailer-check] $ref: Profile-Impact value '$profile_value' not in allowed set"
      echo "              allowed: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}"
      fail=1 ;;
  esac
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "[trailer-check] $ref: OK"
exit 0
