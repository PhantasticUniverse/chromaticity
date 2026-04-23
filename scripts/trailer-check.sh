#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# trailer-check: validate the mandatory trailers required by
# docs/43-development-standards.md Sec.4.5 on the commit referenced
# by $1 (default: HEAD), or on a commit message read from stdin when
# invoked as `trailer-check.sh --stdin`.
#
# Required trailers on every merged commit on `main`:
#   Signed-off-by: <Full Name> <email>
#   Profile-Impact: none | compat-syn-v1-minor | compat-syn-v1-major
#                 | native-chroma-v1-minor | native-chroma-v1-major
#
# Architecture-impacting commits additionally require:
#   ADR: ADR-NNN
#   Evidence: EV-<ID>[, EV-<ID>...]
# (This script does not classify PRs as architecture-impacting; CI
# does that via labels. Here we only enforce the two always-required
# trailers.)
#
# Parsing strategy
# ----------------
# The trailer zone (the tail of the commit message) may span multiple
# paragraphs separated by blank lines. This is common on squash-merge
# commits where GitHub appends its own Signed-off-by trailer below a
# blank line, leaving Profile-Impact stranded in the preceding
# paragraph. `git interpret-trailers --parse` strictly treats only
# the final contiguous non-blank paragraph as the trailer block and
# drops the Profile-Impact paragraph as body prose, which regresses
# every existing merge commit on main.
#
# This script pre-processes the message in awk first: walking from
# the last line upward, it collects contiguous trailer-shaped lines
# into a trailer zone, skipping blank lines and markdown horizontal-
# rule dividers (`^-{3,}$`, `^={3,}$`), and stopping at the first
# prose line (any non-blank, non-divider line that is not trailer-
# shaped). The collected trailer-shaped lines are emitted as a single
# coalesced paragraph and handed to `git interpret-trailers --parse`
# for canonical key/value extraction.
#
# Divider handling is load-bearing for multi-commit squash-merge
# commits under the repo's "default to PR title and description"
# squash template. GitHub appends a cumulative Signed-off-by trailer
# to the merge commit body on multi-commit PRs, separated from the
# PR-body's own trailer block by a 9-dash `---------` divider. The
# walker must treat that divider as blank-equivalent so both trailer
# blocks coalesce into a single parseable zone; otherwise the walker
# breaks at the divider and Profile-Impact (living in the upper
# block) is silently dropped. f6de2f5 was the first main commit to
# hit this shape and is the regression fixture's motivating case.
#
# The prose-collision defense remains intact: any prose line above
# the trailer zone is a hard boundary, so body text containing
# `Profile-Impact: X` or `Signed-off-by: X` cannot leak into the
# parsed trailer set. The regression fixture corpus under
# tests/fixtures/trailer-check/ exercises the edge cases; the sibling
# scripts/test-trailer-check.sh harness drives it.
#
# A line is treated as trailer-shaped if it matches
# `^[A-Za-z][A-Za-z0-9_-]*:( |$)` — a token followed by a colon and
# then whitespace or end-of-line. This is the same shape
# `git interpret-trailers` recognises and correctly excludes URLs
# (`http://…` fails the post-colon requirement of whitespace).
#
# Exit code 0 = pass; non-zero = fail with a printed diagnosis.

set -uo pipefail

mode_stdin=0
ref=""

case "${1:-}" in
  --stdin)
    mode_stdin=1
    ;;
  -h|--help)
    cat <<'USAGE'
usage: trailer-check.sh [<commit-ish> | --stdin]
  (no argument)   check HEAD
  <commit-ish>    check the named commit
  --stdin         read commit message from stdin (used by test harness)
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
    echo "[trailer-check] cannot read commit message for $ref"
    exit 2
  fi
  label="$ref"
fi

# Step 1: extract the trailer zone. Walk from end of message upward;
# skip blank lines; collect trailer-shaped lines; stop at the first
# prose line. Emit collected lines in their original order (no blanks).
zone="$(printf '%s\n' "$msg" | awk '
  { lines[NR] = $0 }
  END {
    rc = 0
    for (i = NR; i >= 1; i--) {
      line = lines[i]
      if (line ~ /^[[:space:]]*$/) continue
      # Markdown horizontal-rule dividers (---, ===, and longer runs)
      # are skipped the same as blanks. GitHub inserts a 9-dash
      # divider between the PR-body trailers and its auto-injected
      # cumulative Signed-off-by on multi-commit squash merges;
      # without this skip, the walker breaks at the divider and the
      # upper trailer block (Profile-Impact, author Signed-off-by)
      # is dropped.
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

# Step 2: canonicalise via git interpret-trailers --parse. The parser
# requires a subject line plus a blank line before trailers, so
# fabricate a minimal header; the subject content is discarded.
if [ -n "$zone" ]; then
  trailers="$( { printf 'trailer-check-synthetic-subject\n\n'; printf '%s\n' "$zone"; } | git interpret-trailers --parse)"
else
  trailers=""
fi

fail=0

signed_off_line="$(printf '%s\n' "$trailers" | grep -E '^Signed-off-by: .+ <.+@.+>$' | tail -n1 || true)"
if [ -z "$signed_off_line" ]; then
  echo "[trailer-check] $label: missing Signed-off-by trailer (DCO)"
  fail=1
fi

profile_line="$(printf '%s\n' "$trailers" | grep -E '^Profile-Impact:' | tail -n1 || true)"
profile_value=""
if [ -z "$profile_line" ]; then
  echo "[trailer-check] $label: missing Profile-Impact trailer"
  fail=1
else
  profile_value="$(printf '%s' "$profile_line" | sed -E 's/^Profile-Impact:[[:space:]]*//; s/[[:space:]]+$//')"
  case "$profile_value" in
    none \
      | compat-syn-v1-minor | compat-syn-v1-major \
      | native-chroma-v1-minor | native-chroma-v1-major) ;;
    *)
      echo "[trailer-check] $label: Profile-Impact value '$profile_value' not in allowed set"
      echo "              allowed: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}"
      fail=1 ;;
  esac
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

# Success diagnostics include the resolved Profile-Impact value so
# the test harness can assert the trailer-block value was selected
# over any prose-collision mention.
echo "[trailer-check] $label: OK (Profile-Impact=$profile_value)"
exit 0
