#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# succession-waiver-check: enforce docs/43-development-standards.md
# Sec.7.6 on solo-merge events.
#
# Solo-merge intent is signaled directly by the trailer
#   Succession-Waiver: T1 | T2
# (case-sensitive). There is no `solo-merge` GitHub label — the trailer
# presence itself is the gate signal. This script runs unconditionally
# on every PR / merge_group event; on the 99% of PRs without a
# Succession-Waiver trailer, it is a no-op (exits 0 immediately).
#
# When the trailer IS present, the value MUST be exactly `T1` or `T2`.
# Any other value (T3, lowercase, missing value, multiple tokens) is a
# hard fail.
#
# Modes
# -----
#   --stdin         read commit message / synthesised PR body from stdin
#                   (CI invocation).
#   <commit-ish>    check the named commit (local invocation).
#   (no argument)   check HEAD (local invocation).
#
# Body extraction
# ---------------
# Reuses the backward-walking awk parser from scripts/trailer-check.sh
# (lines 105-141). See that script's prose for the full rationale.
#
# Exit codes
#   0 = pass (trailer absent OR value is T1 / T2)
#   1 = trailer present with invalid value
#   2 = invocation error

set -uo pipefail

mode_stdin=0
ref=""

case "${1:-}" in
  --stdin)
    mode_stdin=1
    ;;
  -h|--help)
    cat <<'USAGE'
usage: succession-waiver-check.sh [<commit-ish> | --stdin]
  (no argument)   check HEAD
  <commit-ish>    check the named commit
  --stdin         read commit message from stdin (CI invocation)
USAGE
    exit 0
    ;;
  "")
    ref="HEAD"
    ;;
  *)
    ref="$1"
    ;;
esac

if [ "$mode_stdin" -eq 1 ]; then
  msg="$(cat)"
  label="stdin"
else
  if ! msg="$(git log -1 --format='%B' "$ref" 2>/dev/null)"; then
    echo "[succession-waiver-check] cannot read commit message for $ref" >&2
    exit 2
  fi
  label="$ref"
fi

# --- Body extraction (mirrors scripts/trailer-check.sh lines 105-141) ---
zone="$(printf '%s\n' "$msg" | awk '
  { lines[NR] = $0 }
  END {
    rc = 0
    for (i = NR; i >= 1; i--) {
      line = lines[i]
      if (line ~ /^[[:space:]]*$/) continue
      if (line ~ /^-{3,}[[:space:]]*$/) continue
      if (line ~ /^={3,}[[:space:]]*$/) continue
      if (line ~ /^[A-Za-z][A-Za-z0-9_-]*:( |$)/) {
        result[++rc] = line
        continue
      }
      break
    }
    for (i = rc; i >= 1; i--) print result[i]
  }
')"

if [ -n "$zone" ]; then
  trailers="$( { printf 'succession-waiver-check-synthetic-subject\n\n'; printf '%s\n' "$zone"; } | git interpret-trailers --parse)"
else
  trailers=""
fi

waiver_line="$(printf '%s\n' "$trailers" | grep -E '^Succession-Waiver:' | tail -n1 || true)"

if [ -z "$waiver_line" ]; then
  # No solo-merge intent declared; this is the 99% path.
  exit 0
fi

# Strip the trailer key but PRESERVE leading whitespace boundaries so a
# valueless `Succession-Waiver:` (no value) is detected as the empty
# string rather than silently coerced.
value="$(printf '%s' "$waiver_line" | sed -E 's/^Succession-Waiver:[[:space:]]*//; s/[[:space:]]+$//')"

case "$value" in
  T1|T2)
    echo "[succession-waiver-check] $label: OK (Succession-Waiver=$value)"
    exit 0
    ;;
  "")
    echo "[succession-waiver-check] $label: Succession-Waiver trailer present but value is empty (must be T1 or T2)" >&2
    exit 1
    ;;
  *)
    echo "[succession-waiver-check] $label: Succession-Waiver value '$value' not in allowed set (must be T1 or T2 per docs/43 Sec.7.6)" >&2
    exit 1
    ;;
esac
