#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# evidence-tag-lint — enforces EV-EVIDENCE-TAG-COVERAGE per
# docs/43-development-standards.md §15 and the evidence row in
# docs/26-evidence-artifact-registry.md.
#
# Runs as a required CI check (see
# .github/workflows/evidence-tag-lint.yml) from Phase 1 onward.
#
# Scope as of this script version (strict graduation):
#
#   - **Tier-marker parsing (§15.5 line 590):** every `//! Evidence:
#     EV-<ID>` doc comment — column-0 file-level or indented
#     test-module-level — MUST carry a `(T0)`, `(T1)`, or `(T2)` tier
#     token. A tier-less tag fails the lint. Emits a tier-distribution
#     summary for ops visibility. Applying the tier requirement to
#     column-0 tags closes an edit-pattern gap: demoting an indented
#     claim to file-level to escape tier validation would otherwise
#     silently drop coverage.
#   - **Coverage (§15.4 line 582):** every Rust test file under an
#     evidence-producing subtree (`engine/**`, `integrations/**`)
#     that declares a `#[test]` / `#[tokio::test]` function MUST
#     contain at least one `//! Evidence:` declaration. Absent-tag
#     files fail the lint. Per-test granularity (vs per-file) lands
#     with the `#[evidence(...)]` attribute macro; the current
#     per-file check is the Phase-1 transitional form.
#   - **ID validity (§15.4 line 583):** every EV-ID cited in any
#     `//! Evidence:` tag MUST exist in docs/26. Typos and forgotten
#     registry updates fail the lint.
#
# Note on `//! Evidence:` at column 0: file-level narrative that
# discusses evidence without making a claim should omit the
# `Evidence:` prefix (e.g., rephrase as "Produces evidence for
# `EV-<ID>` via ...") so it doesn't trigger the tier requirement.
# If a file-level line is a real claim (not just narrative), tier
# it the same as a module-level declaration.
#
# Evidence-producing subtrees are the first-party engine and
# integration crates: `engine/**` and `integrations/**`. App-shell
# and workspace-hack are infrastructure and are not subject to the
# coverage check. The per-test attribute macro form
# (`#[evidence(...)]`) is reserved in §15.2 but not yet implemented;
# when it ships, the escape hatch form `#[evidence(none = "…")]`
# will be recognized here.
#
# Deferred scope (not an oversight):
#
#   - **Dynamic subtree discovery.** The coverage scope is hardcoded
#     below. The canonical source is `docs/12-work-breakdown-
#     estimates.md` WP-N "Outputs" sections, which enumerate per-
#     work-package evidence-producing subtrees. Driving the list
#     from docs/12 is tracked as a follow-up; until then, adding a
#     new evidence-producing WP requires a paired edit here. An
#     in-body guard comment near the hardcoded paths flags the
#     drift risk.
#   - **TypeScript / JavaScript coverage.** §15.2 line 573 defines a
#     framework-specific tag for TS/JS:
#     `test.concurrent("...", { evidence: ["EV-<ID>"] }, async () => ...)`.
#     This script scans Rust sources only. Phase-1 scope is
#     engine-side; there are no frontend evidence-producing work
#     packages today. TS/JS coverage lands when the first such WP
#     appears (tracked as a follow-up); adding it here before then
#     would be validating an empty set.
#
# No run: step consumes untrusted github.event.* input.

set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"

fail=0
registry="$repo/docs/26-evidence-artifact-registry.md"

# --- Valid-EV-ID set (built from docs/26) ---

if [ ! -f "$registry" ]; then
    printf '[evidence-tag-lint] FAIL: registry missing at %s\n' "$registry"
    exit 1
fi

valid_ids="$(
    grep -oE 'EV-[A-Z][A-Z0-9_-]+' "$registry" \
    | LC_ALL=C sort -u
)"

# --- Collect every `//! Evidence:` line (column-0 or indented) ---

# Both file-level and test-module-level Evidence tags are validated
# for tier-token presence and EV-ID validity. Output format:
# <path>:<line-no>:<line>.
evidence_lines="$(
    find "$repo" \
        -type d \( -name target -o -name node_modules -o -name dist -o -name .git -o -name Contents -o -name chromaticity-planning \) -prune \
        -o -type f -name '*.rs' -print 2>/dev/null \
    | xargs grep -nHE '^[[:space:]]*//! Evidence:' 2>/dev/null \
    || true
)"

# --- Tier-marker parsing (strict, per §15.5 line 590) ---

# Position-anchored tier check. A line satisfies the convention iff
# the `(T<n>)` token sits immediately after the EV-ID (optionally
# backtick-wrapped), separated by whitespace, per §15.5 line 590's
# canonical form. Loose occurrences of `(T0)` buried in narrative
# prose or trailing commentary do not count. Lines that don't match
# the canonical form fall through into `untiered` and fail the lint.
tiered_re='//! Evidence:[[:space:]]+`?EV-[A-Z][A-Z0-9_-]+`?[[:space:]]+\(T[012]\)'
untiered="$(
    printf '%s\n' "$evidence_lines" \
    | grep -v '^$' \
    | grep -v -E "$tiered_re" \
    || true
)"

if [ -n "$untiered" ]; then
    printf '[evidence-tag-lint] FAIL: tier token missing from evidence tag(s):\n'
    printf '%s\n' "$untiered" | while IFS= read -r line; do
        printf '  %s\n' "$line"
    done
    printf '[evidence-tag-lint]        Required form per docs/43 §15.2 / §15.5:\n'
    printf '[evidence-tag-lint]          //! Evidence: EV-<ID> (T<n>) — <one-line rationale>\n'
    printf '[evidence-tag-lint]        where T<n> is one of T0, T1, T2. Applies to both\n'
    printf '[evidence-tag-lint]        file-level (column-0) and module-level (indented) tags.\n'
    fail=1
fi

if [ -n "$evidence_lines" ]; then
    count="$(printf '%s\n' "$evidence_lines" | grep -v '^$' | wc -l | tr -d ' ')"
    printf '[evidence-tag-lint] Tier distribution across %s evidence tag(s):\n' "$count"
    printf '%s\n' "$evidence_lines" \
        | sed -nE 's|^[^:]+:[0-9]+:[[:space:]]*//! Evidence:[[:space:]]+`?([A-Z][A-Z0-9_-]+)`?[[:space:]]+\((T[012])\).*|\1 \2|p' \
        | LC_ALL=C sort | uniq -c \
        | while read -r tally id tier; do
            printf '  %5s  %s %s\n' "$tally" "$id" "$tier"
        done
fi

# --- ID validity (per §15.4 line 583) ---

unknown_refs=""
while IFS= read -r line; do
    [ -z "$line" ] && continue
    # Extract every EV-<ID> reference on this line. A single `//!`
    # tag line may cite multiple IDs (e.g. a two-row aliased claim).
    refs="$(printf '%s\n' "$line" | grep -oE 'EV-[A-Z][A-Z0-9_-]+' || true)"
    while IFS= read -r ref; do
        [ -z "$ref" ] && continue
        if ! printf '%s\n' "$valid_ids" | grep -qxF "$ref"; then
            unknown_refs="$(printf '%s\n  %s  (cites %s)\n' "$unknown_refs" "$line" "$ref")"
        fi
    done <<EOF
$refs
EOF
done <<EOF
$evidence_lines
EOF

if [ -n "$unknown_refs" ]; then
    printf '[evidence-tag-lint] FAIL: evidence tag(s) cite unknown EV-ID(s):\n'
    printf '%s\n' "$unknown_refs" | grep -v '^$'
    printf '[evidence-tag-lint]        Every cited EV-ID must appear in\n'
    printf '[evidence-tag-lint]        docs/26-evidence-artifact-registry.md. Fix the typo\n'
    printf '[evidence-tag-lint]        or add the row to the registry via a docs(policy) PR.\n'
    fail=1
fi

# --- Coverage (per §15.4 line 582) ---

# DRIFT GUARD: The `$repo/engine` and `$repo/integrations` paths
# below are the hardcoded coverage scope. The canonical source for
# "evidence-producing subtree" is docs/12-work-breakdown-estimates.md
# WP-N Outputs. If a new WP adds an evidence-producing subtree
# outside these two trees (or retires one of them), this find's
# root list MUST be updated in the same PR that touches docs/12.
# Drift here silently gaps coverage. Driving this list from
# docs/12 at runtime is a tracked follow-up.
uncovered=""
while IFS= read -r file; do
    [ -z "$file" ] && continue
    if grep -qE '^[[:space:]]*#\[(tokio::)?test\]' "$file" 2>/dev/null; then
        if ! grep -qE '^[[:space:]]*//! Evidence:' "$file" 2>/dev/null; then
            uncovered="$(printf '%s\n  %s\n' "$uncovered" "$file")"
        fi
    fi
done <<EOF
$(find "$repo/engine" "$repo/integrations" \
    -type d \( -name target -o -name node_modules -o -name dist -o -name .git -o -name Contents -o -name chromaticity-planning \) -prune \
    -o -type f -name '*.rs' -print 2>/dev/null)
EOF

if [ -n "$uncovered" ]; then
    printf '[evidence-tag-lint] FAIL: test file(s) without any //! Evidence: tag:\n'
    printf '%s\n' "$uncovered" | grep -v '^$'
    printf '[evidence-tag-lint]        Every Rust source under engine/** or integrations/**\n'
    printf '[evidence-tag-lint]        that declares a #[test] / #[tokio::test] function\n'
    printf '[evidence-tag-lint]        must carry at least one //! Evidence: declaration\n'
    printf '[evidence-tag-lint]        (module-level inside the test module, or file-level).\n'
    fail=1
fi

if [ "$fail" -ne 0 ]; then
    printf '[evidence-tag-lint] FAIL\n'
    exit 1
fi

printf '[evidence-tag-lint] OK\n'
exit 0
