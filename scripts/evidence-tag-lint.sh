#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# evidence-tag-lint — enforces EV-EVIDENCE-TAG-COVERAGE per
# docs/43-development-standards.md §15 and the evidence row in
# docs/26-evidence-artifact-registry.md.
#
# Validates that every test in an evidence-producing subtree carries
# an `#[evidence(...)]` tag referencing an EV-ID present in the
# evidence registry. Runs as a required CI check (see
# .github/workflows/evidence-tag-lint.yml) so the branch-protection
# slot exists from Phase 1 onward.
#
# Scope as of this script version:
#
#   - **Tier-marker parsing (strict, per §15.5 line 590):** every
#     module-level `//! Evidence: EV-<ID>` doc comment MUST carry a
#     `(T0)`, `(T1)`, or `(T2)` tier token. A tier-less tag fails
#     the lint. Also emits a tier-distribution summary (count by
#     EV-ID × tier) for ops visibility.
#   - **Coverage (advisory stub):** the "every test in an evidence-
#     producing subtree carries an evidence tag referencing a live
#     EV-ID" check per §15.4 lines 582–583 lands with a subsequent
#     script version (evidence-tag-lint graduation work package per
#     docs/12 WP-N Outputs). Until then, the coverage body counts
#     test sources and emits an advisory marker pointing at the
#     graduation work.
#
# "Module-level" is identified heuristically by the doc-comment's
# leading indentation: `//!` at column 0 is file-level narrative
# describing the crate's evidence landscape and is NOT validated
# (no tier expected); an indented `//!` sits inside a `mod { }`
# block and is treated as the enclosing test module's evidence
# declaration (tier required).
#
# No run: step consumes untrusted github.event.* input.

set -eu

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"

fail=0

# --- Tier-marker parsing (strict) ---

# Collect every indented `//! Evidence:` line across Rust sources,
# skipping build / tooling dirs. Output format: <path>:<line-no>:<line>.
module_evidence_lines="$(
    find "$repo" \
        -type d \( -name target -o -name node_modules -o -name dist -o -name .git \) -prune \
        -o -type f -name '*.rs' -print 2>/dev/null \
    | xargs grep -nHE '^[[:space:]]+//! Evidence:' 2>/dev/null \
    || true
)"

# Tier-presence validation. An indented `//! Evidence:` line without a
# `(T0)` / `(T1)` / `(T2)` token fails per §15.5 line 590.
untiered="$(
    printf '%s\n' "$module_evidence_lines" \
    | grep -v '^$' \
    | grep -v -E '\(T[012]\)' \
    || true
)"

if [ -n "$untiered" ]; then
    printf '[evidence-tag-lint] FAIL: tier token missing from module-level evidence tag(s):\n'
    printf '%s\n' "$untiered" | while IFS= read -r line; do
        printf '  %s\n' "$line"
    done
    printf '[evidence-tag-lint]        Required form per docs/43 §15.2 / §15.5:\n'
    printf '[evidence-tag-lint]          //! Evidence: EV-<ID> (T<n>) — <one-line rationale>\n'
    printf '[evidence-tag-lint]        where T<n> is one of T0, T1, T2.\n'
    fail=1
fi

# Tier-distribution metadata. Emits a (count, EV-ID, tier) rollup so
# ops can see at a glance which rows are claimed at which tiers.
if [ -n "$module_evidence_lines" ]; then
    count="$(printf '%s\n' "$module_evidence_lines" | grep -v '^$' | wc -l | tr -d ' ')"
    printf '[evidence-tag-lint] Tier distribution across %s module-level evidence tag(s):\n' "$count"
    printf '%s\n' "$module_evidence_lines" \
        | sed -nE 's|^[^:]+:[0-9]+:[[:space:]]*//! Evidence:[[:space:]]+`?([A-Z][A-Z0-9_-]+)`?[[:space:]]+\((T[012])\).*|\1 \2|p' \
        | LC_ALL=C sort | uniq -c \
        | while read -r tally id tier; do
            printf '  %5s  %s %s\n' "$tally" "$id" "$tier"
        done
fi

# --- Coverage (advisory stub — graduation work per docs/12 WP-N) ---

rust_tests="$(
    find "$repo" \
        -type d \( -name target -o -name node_modules -o -name dist -o -name .git \) -prune \
        -o -type f -name '*.rs' -print 2>/dev/null \
    | xargs grep -lE '^[[:space:]]*#\[(tokio::)?test\]' 2>/dev/null \
    | wc -l \
    | tr -d ' '
)"

ts_tests="$(
    find "$repo" \
        -type d \( -name target -o -name node_modules -o -name dist -o -name .git \) -prune \
        -o -type f \( -name '*.test.ts' -o -name '*.spec.ts' \) -print 2>/dev/null \
    | wc -l \
    | tr -d ' '
)"

total=$(( rust_tests + ts_tests ))

if [ "$total" -eq 0 ]; then
    printf '[evidence-tag-lint] Coverage stub: no test sources yet (rust=%s, ts=%s).\n' \
        "$rust_tests" "$ts_tests"
else
    printf '[evidence-tag-lint] Coverage stub: %s test file(s) present (rust=%s, ts=%s);\n' \
        "$total" "$rust_tests" "$ts_tests"
    printf '[evidence-tag-lint]                strict #[evidence(...)] coverage check pending\n'
    printf '[evidence-tag-lint]                per docs/43 §15.4 + docs/26 EV-EVIDENCE-TAG-COVERAGE.\n'
fi

if [ "$fail" -ne 0 ]; then
    printf '[evidence-tag-lint] FAIL\n'
    exit 1
fi

printf '[evidence-tag-lint] OK\n'
exit 0
