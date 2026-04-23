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
# STATUS: Phase-1 entry stub. The CI gate is wired so the required-
# check slot exists; the strict enforcement body is implemented by
# the evidence-tag-lint work package in
# docs/12-work-breakdown-estimates.md once the first test arrives.
# In the interim we scan for test attributes and pass trivially
# when none are present (Phase-1 scaffold state) or emit a pointer
# to the work package when any are present.

set -eu

repo="$(cd "$(dirname "$0")/.." && pwd)"

# Count Rust test attributes, skipping target/ + node_modules/ + dist/.
rust_tests="$(
    find "$repo" \
        -type d \( -name target -o -name node_modules -o -name dist -o -name .git \) -prune \
        -o -type f -name '*.rs' -print 2>/dev/null \
    | xargs grep -lE '^[[:space:]]*#\[(tokio::)?test\]' 2>/dev/null \
    | wc -l \
    | tr -d ' '
)"

# Count TS/JS test files by convention.
ts_tests="$(
    find "$repo" \
        -type d \( -name target -o -name node_modules -o -name dist -o -name .git \) -prune \
        -o -type f \( -name '*.test.ts' -o -name '*.spec.ts' \) -print 2>/dev/null \
    | wc -l \
    | tr -d ' '
)"

total=$(( rust_tests + ts_tests ))

if [ "$total" -eq 0 ]; then
    printf '[evidence-tag-lint] Phase-1 entry stub: no test sources yet (rust=%s, ts=%s).\n' \
        "$rust_tests" "$ts_tests"
    printf '[evidence-tag-lint] Strict #[evidence(...)] coverage enforcement wires with the\n'
    printf '                   first test per docs/12-work-breakdown-estimates.md.\n'
    printf '[evidence-tag-lint] OK\n'
    exit 0
fi

printf '[evidence-tag-lint] WARN: %s test file(s) present (rust=%s, ts=%s); strict\n' \
    "$total" "$rust_tests" "$ts_tests"
printf '[evidence-tag-lint]       #[evidence(...)] coverage check pending implementation\n'
printf '[evidence-tag-lint]       per docs/43-development-standards.md §15 and\n'
printf '[evidence-tag-lint]       docs/26-evidence-artifact-registry.md EV-EVIDENCE-TAG-COVERAGE.\n'
printf '[evidence-tag-lint] OK (advisory)\n'
exit 0
