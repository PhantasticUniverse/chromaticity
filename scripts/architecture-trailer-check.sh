#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# architecture-trailer-check: enforce docs/43-development-standards.md
# Sec.4.6 on architecture-labeled PRs.
#
# Architecture-impacting commits MUST carry both:
#   ADR: ADR-NNN
#   Evidence: EV-<ID>[, EV-<ID>...]
# in their trailer block, where each ADR-NNN must resolve to a file
# under docs/adr/ADR-NNN-*.md and each EV-<ID> must resolve to a row
# in the docs/26-evidence-artifact-registry.md table.
#
# Modes
# -----
#   --stdin         read commit message / synthesised PR body from stdin
#                   (CI invocation; PR_NUMBER + GITHUB_REPOSITORY env
#                   vars are consulted for the architecture-label gate).
#   <commit-ish>    check the named commit (local invocation; no label
#                   gate, validates trailers unconditionally).
#   (no argument)   check HEAD (local invocation; no label gate).
#
# Label gate
# ----------
# In --stdin mode, the label gate fires when both PR_NUMBER and
# GITHUB_REPOSITORY are set. The gate queries
# /repos/$GITHUB_REPOSITORY/pulls/$PR_NUMBER/labels and exits 0 with an
# informational stderr message if the `architecture` label is absent —
# the canonical label is `architecture` per .github/labels.yml line 106
# (there is no `solo-merge` label; succession-waiver uses a different
# trailer-presence gate in scripts/succession-waiver-check.sh). When
# the env vars are absent (local mode), the gate is skipped and trailer
# validation runs unconditionally for testability.
#
# Body extraction
# ---------------
# Reuses the backward-walking awk parser from scripts/trailer-check.sh
# (lines 105-141): collects contiguous trailer-shaped lines from the
# tail of the message, skipping blank lines and `^-{3,}` / `^={3,}`
# dividers, stopping at the first prose line. Then pipes through
# `git interpret-trailers --parse` for canonical key/value extraction.
# See trailer-check.sh's prose for the full rationale (squash-merge
# divider handling, prose-collision defense, URL exclusion).
#
# ADR validation
# --------------
# For each `ADR: ADR-NNN` trailer, the script extracts NNN and checks
# that `docs/adr/ADR-NNN-*.md` resolves to at least one file via the
# same `ls` glob pattern adr-lint.sh uses (lines 145-152). Multiple
# ADR: lines are allowed.
#
# Evidence validation
# -------------------
# For each `Evidence: EV-XX[, EV-YY...]` trailer, the value is split on
# commas and each EV-ID is validated against the row-anchored valid_ids
# set extracted from docs/26-evidence-artifact-registry.md per the
# evidence-tag-lint.sh pattern (line 93). Multiple Evidence: lines are
# allowed; comma-separated lists within one line are allowed.
#
# Required-trailer presence
# -------------------------
# When the architecture label is present (or in local mode), AT LEAST
# ONE `ADR:` AND AT LEAST ONE `Evidence:` trailer is required. Missing
# either trailer entirely is a hard fail.
#
# Exit codes
#   0 = pass (label absent OR all trailers valid)
#   1 = label present + missing/malformed trailer
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
usage: architecture-trailer-check.sh [<commit-ish> | --stdin]
  (no argument)   check HEAD (local mode; no label gate)
  <commit-ish>    check the named commit (local mode; no label gate)
  --stdin         read commit message from stdin (CI mode; consults
                  PR_NUMBER + GITHUB_REPOSITORY for label gate)
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

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
adr_dir="$repo/docs/adr"
registry="$repo/docs/26-evidence-artifact-registry.md"

# --- Label gate (CI mode only) ---
if [ "$mode_stdin" -eq 1 ]; then
  pr_num="${PR_NUMBER:-}"
  repo_slug="${GITHUB_REPOSITORY:-}"
  if [ -n "$pr_num" ] && [ -n "$repo_slug" ]; then
    if ! command -v gh >/dev/null 2>&1; then
      echo "[architecture-trailer-check] gh CLI required for label gate; not installed." >&2
      exit 2
    fi
    labels="$(gh api "repos/$repo_slug/pulls/$pr_num/labels" --jq '.[].name' 2>/dev/null || printf '')"
    if ! printf '%s\n' "$labels" | grep -qx 'architecture'; then
      echo "[architecture-trailer-check] no architecture label on PR $pr_num; skipping." >&2
      # Drain stdin so callers piping to us don't get SIGPIPE.
      cat >/dev/null
      exit 0
    fi
    echo "[architecture-trailer-check] architecture label present on PR $pr_num; validating trailers." >&2
  else
    echo "[architecture-trailer-check] PR_NUMBER or GITHUB_REPOSITORY unset; running in local mode (no label gate)." >&2
  fi
fi

# --- Read message ---
if [ "$mode_stdin" -eq 1 ]; then
  msg="$(cat)"
  label="stdin"
else
  if ! msg="$(git log -1 --format='%B' "$ref" 2>/dev/null)"; then
    echo "[architecture-trailer-check] cannot read commit message for $ref" >&2
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
  trailers="$( { printf 'architecture-trailer-check-synthetic-subject\n\n'; printf '%s\n' "$zone"; } | git interpret-trailers --parse)"
else
  trailers=""
fi

fail=0

# --- Build the valid EV-ID set from docs/26 (mirrors evidence-tag-lint.sh line 93) ---
if [ ! -f "$registry" ]; then
  echo "[architecture-trailer-check] $label: registry missing at $registry" >&2
  exit 2
fi
valid_ids="$(
  grep -oE '^\| `EV-[A-Z][A-Z0-9_-]+`' "$registry" \
    | grep -oE 'EV-[A-Z][A-Z0-9_-]+' \
    | LC_ALL=C sort -u
)"

# --- ADR: trailers ---
adr_lines="$(printf '%s\n' "$trailers" | grep -E '^ADR:' || true)"
adr_count=0
if [ -n "$adr_lines" ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    value="$(printf '%s' "$line" | sed -E 's/^ADR:[[:space:]]*//; s/[[:space:]]+$//')"
    if ! printf '%s' "$value" | grep -qE '^ADR-[0-9]{3}$'; then
      echo "[architecture-trailer-check] $label: malformed ADR trailer value '$value' (expected ADR-NNN)" >&2
      fail=1
      continue
    fi
    target_num="$(printf '%s' "$value" | sed -E 's/^ADR-([0-9]{3})$/\1/')"
    if ! ls "$adr_dir"/ADR-"$target_num"-*.md >/dev/null 2>&1; then
      echo "[architecture-trailer-check] $label: ADR trailer references $value but no docs/adr/$value-*.md file exists" >&2
      fail=1
      continue
    fi
    adr_count=$((adr_count + 1))
  done <<EOF
$adr_lines
EOF
fi

# --- Evidence: trailers ---
ev_lines="$(printf '%s\n' "$trailers" | grep -E '^Evidence:' || true)"
ev_count=0
if [ -n "$ev_lines" ]; then
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    value="$(printf '%s' "$line" | sed -E 's/^Evidence:[[:space:]]*//; s/[[:space:]]+$//')"
    # Split on commas; trim whitespace per item.
    IFS=',' read -r -a items <<<"$value"
    for raw in "${items[@]}"; do
      ev_id="$(printf '%s' "$raw" | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')"
      [ -z "$ev_id" ] && continue
      if ! printf '%s' "$ev_id" | grep -qE '^EV-[A-Z][A-Z0-9_-]+$'; then
        echo "[architecture-trailer-check] $label: malformed EV-ID '$ev_id' (expected EV-<ID>)" >&2
        fail=1
        continue
      fi
      if ! printf '%s\n' "$valid_ids" | grep -qx "$ev_id"; then
        echo "[architecture-trailer-check] $label: Evidence trailer references $ev_id but no such row in $registry" >&2
        fail=1
        continue
      fi
      ev_count=$((ev_count + 1))
    done
  done <<EOF
$ev_lines
EOF
fi

# --- Required-trailer presence ---
if [ "$adr_count" -eq 0 ]; then
  echo "[architecture-trailer-check] $label: missing 'ADR: ADR-NNN' trailer (required for architecture-labeled PRs per docs/43 Sec.4.6)" >&2
  fail=1
fi
if [ "$ev_count" -eq 0 ]; then
  echo "[architecture-trailer-check] $label: missing 'Evidence: EV-<ID>' trailer (required for architecture-labeled PRs per docs/43 Sec.4.6)" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "[architecture-trailer-check] $label: OK (ADR refs=$adr_count, Evidence refs=$ev_count)"
exit 0
