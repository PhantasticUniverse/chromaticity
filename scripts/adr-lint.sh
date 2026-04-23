#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# adr-lint: enforce docs/43-development-standards.md Sec.14.3 on every
# ADR file passed on the command line. Invoked by lefthook pre-commit
# (on staged docs/adr/**/*.md) and by the CI policy workflow.
#
# Checks:
#   1. Filename matches ADR-<NNN>-<kebab-slug>.md (or is TEMPLATE.md).
#   2. A Status is declared, either inline (`**Status:** <value>`,
#      `- **Status:** <value>`, or `Status: <value>`) or as a
#      section-header form (`## Status\n<value>`). Value must be one of
#      Proposed | Accepted | Rejected | Deprecated | Superseded by ADR-NNN.
#      A parenthesized qualifier (e.g. "Accepted (planning baseline)") is
#      tolerated to keep the grandfathered ADR-001..018 valid.
#   3. `## Evidence` section header exists.
#   4. `## Traceability` section header exists.
#   5. If Status is "Superseded by ADR-NNN", the referenced ADR file exists.
#
# Phase-0 grace mode:
#   Checks 3 and 4 default to **warnings** (printed to stderr, do not
#   fail the script) to match the Phase-0 waiver in
#   docs/43-development-standards.md Sec.7.3. Export
#   ADR_LINT_REQUIRE_EV_TRACE=1 to promote them to errors. Phase-1 CI
#   sets this variable; the backfill of legacy ADRs is tracked as a
#   Phase-0 work-package item.
#
# Architecture-review gate (opt-in, GITHUB_EVENT_PATH-driven):
#   When ADR_LINT_REQUIRE_ARCH_REVIEW=1 AND the script runs in a CI
#   PR context (PR_NUMBER + GITHUB_REPOSITORY set), the gate checks
#   whether the PR carries the `architecture` label per
#   docs/43-development-standards.md Sec.8.2 (label taxonomy). If
#   the label is present, at least one submitted review on the PR
#   MUST carry `[INDEPENDENT-ARCH-REVIEW]` verbatim in its body per
#   Sec.7.4 to satisfy the independent-reviewer requirement. The
#   gate silently no-ops for PRs without the `architecture` label
#   and when the env var is unset, so local invocations and
#   non-PR events don't touch the network. `gh` CLI is required
#   when the gate fires; CI installs it by default on
#   `ubuntu-latest`.

set -uo pipefail

fail=0
warn=0
require_ev_trace="${ADR_LINT_REQUIRE_EV_TRACE:-0}"
require_arch_review="${ADR_LINT_REQUIRE_ARCH_REVIEW:-0}"

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
adr_dir="$repo/docs/adr"

allowed_statuses='Proposed|Accepted|Rejected|Deprecated|Superseded by ADR-[0-9]{3}'

if [ "$#" -eq 0 ]; then
  set -- "$adr_dir"/*.md
fi

emit_warn_or_error() {
  local msg="$1"
  if [ "$require_ev_trace" = "1" ]; then
    printf '%s\n' "$msg" >&2
    fail=1
  else
    printf '[adr-lint] WARN (Phase-0 grace): %s\n' "${msg#'[adr-lint] '}" >&2
    warn=$((warn + 1))
  fi
}

extract_status_value() {
  # $1 is the path to the ADR file. Emit the Status value to stdout, or
  # nothing if the ADR has no recognizable Status declaration.
  local f="$1"
  local line value

  # Inline forms: **Status:**, - **Status:**, Status:.
  line="$(grep -iE '^\*\*Status:|^[-*] +\*\*Status:|^Status:' "$f" | head -n1 || true)"
  if [ -n "$line" ]; then
    value="$(printf '%s' "$line" \
      | sed -E 's/^[-*] +//; s/^\*\*//; s/Status:\*\*//; s/^Status://; s/^[[:space:]]+//; s/[[:space:]]+$//; s/\*+$//')"
    printf '%s' "$value"
    return 0
  fi

  # Section-header form: `## Status` followed on a subsequent non-empty
  # line by the value. Grab the first non-empty, non-comment line after
  # the header.
  value="$(awk '
    /^## +Status([[:space:]]|$)/ { capturing=1; next }
    capturing && /^## +/ { exit }
    capturing && NF > 0 && !/^<!--/ {
      print
      exit
    }
  ' "$f")"
  if [ -n "$value" ]; then
    value="$(printf '%s' "$value" | sed -E 's/^[-*] +//; s/^[[:space:]]+//; s/[[:space:]]+$//')"
    printf '%s' "$value"
    return 0
  fi

  return 0
}

for f in "$@"; do
  [ -f "$f" ] || continue

  base="$(basename "$f")"

  if [ "$base" = "TEMPLATE.md" ]; then
    # Template must itself be a well-formed MADR starter.
    if ! grep -qE '^## *Evidence' "$f"; then
      printf '[adr-lint] %s: missing "## Evidence" section\n' "$f" >&2
      fail=1
    fi
    if ! grep -qE '^## *Traceability' "$f"; then
      printf '[adr-lint] %s: missing "## Traceability" section\n' "$f" >&2
      fail=1
    fi
    continue
  fi

  # 1. Filename.
  if ! printf '%s' "$base" | grep -qE '^ADR-[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*\.md$'; then
    printf '[adr-lint] %s: filename must match ADR-NNN-kebab-slug.md\n' "$f" >&2
    fail=1
  fi

  # 2. Status.
  status_value="$(extract_status_value "$f")"
  if [ -z "$status_value" ]; then
    printf '[adr-lint] %s: missing Status declaration\n' "$f" >&2
    fail=1
  else
    # Strip a trailing parenthesized qualifier such as "(planning baseline)"
    # so grandfathered ADR-001..018 pass without rewriting.
    normalized="$(printf '%s' "$status_value" | sed -E 's/[[:space:]]*\(.*\)$//')"
    if ! printf '%s' "$normalized" | grep -qE "^($allowed_statuses)$"; then
      printf '[adr-lint] %s: Status "%s" not in allowed set\n' "$f" "$status_value" >&2
      printf '           allowed: Proposed | Accepted | Rejected | Deprecated | Superseded by ADR-NNN\n' >&2
      fail=1
    fi

    # 5. Superseded-by target exists.
    if printf '%s' "$normalized" | grep -qE '^Superseded by ADR-[0-9]{3}$'; then
      target_num="$(printf '%s' "$normalized" | sed -E 's/^Superseded by ADR-([0-9]{3})$/\1/')"
      if ! ls "$adr_dir"/ADR-"$target_num"-*.md >/dev/null 2>&1; then
        printf '[adr-lint] %s: Status references ADR-%s but no such ADR exists\n' "$f" "$target_num" >&2
        fail=1
      fi
    fi
  fi

  # 3. Evidence.
  if ! grep -qE '^## *Evidence' "$f"; then
    emit_warn_or_error "[adr-lint] $f: missing '## Evidence' section"
  fi

  # 4. Traceability.
  if ! grep -qE '^## *Traceability' "$f"; then
    emit_warn_or_error "[adr-lint] $f: missing '## Traceability' section"
  fi
done

# --- Architecture-review gate (opt-in) ---
#
# Runs AFTER the per-file ADR checks so a missing arch-review on an
# architecture-labeled PR fails alongside any ADR-file issues, not
# instead of them. Triple-gated to stay a no-op locally:
#
#   1. ADR_LINT_REQUIRE_ARCH_REVIEW != 1          -> skip
#   2. PR_NUMBER or GITHUB_REPOSITORY unset        -> skip (not in CI PR context)
#   3. PR doesn't carry the `architecture` label   -> skip
#
# When the gate fires, it queries the PR's reviews via `gh api` and
# fails if no submitted review body contains `[INDEPENDENT-ARCH-REVIEW]`.
# APPROVED reviews are preferred but the substring match applies to
# any review state — the tag signals the reviewer claims the
# independence contract, and downstream branch-protection handles
# whether the review is sufficient to merge.

if [ "$require_arch_review" = "1" ]; then
  pr_num="${PR_NUMBER:-}"
  repo_slug="${GITHUB_REPOSITORY:-}"

  if [ -z "$pr_num" ] || [ -z "$repo_slug" ]; then
    printf '[adr-lint] ARCH_REVIEW gate requested but PR_NUMBER or GITHUB_REPOSITORY is unset; skipping.\n' >&2
  elif ! command -v gh >/dev/null 2>&1; then
    printf '[adr-lint] ARCH_REVIEW gate requires the gh CLI; not installed on this runner.\n' >&2
    fail=1
  else
    labels="$(gh api "/repos/$repo_slug/pulls/$pr_num" --jq '[.labels[].name] | join(",")' 2>/dev/null || printf '')"
    if printf '%s' ",$labels," | grep -qE ',architecture,'; then
      printf '[adr-lint] architecture label present on PR %s; checking for [INDEPENDENT-ARCH-REVIEW] tag\n' "$pr_num"
      tag_count="$(gh api "/repos/$repo_slug/pulls/$pr_num/reviews" --jq '[.[] | select(.body // "" | contains("[INDEPENDENT-ARCH-REVIEW]"))] | length' 2>/dev/null || printf '0')"
      if [ -z "$tag_count" ] || [ "$tag_count" = "0" ]; then
        printf '[adr-lint] FAIL: architecture-labeled PR %s has no review carrying [INDEPENDENT-ARCH-REVIEW] per docs/43 Sec.7.4\n' "$pr_num" >&2
        fail=1
      else
        printf '[adr-lint] architecture-review tag present on %s review(s)\n' "$tag_count"
      fi
    fi
  fi
fi

if [ "$fail" -ne 0 ]; then
  printf '[adr-lint] one or more ADR lint checks failed.\n' >&2
  exit 1
fi

if [ "$warn" -gt 0 ]; then
  printf '[adr-lint] OK with %d Phase-0 grace warning(s). Promote to errors by setting ADR_LINT_REQUIRE_EV_TRACE=1.\n' "$warn"
else
  printf '[adr-lint] OK\n'
fi
exit 0
