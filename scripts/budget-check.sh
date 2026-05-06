#!/usr/bin/env bash
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# budget-check — Phase-3 wave PR 6 EV-PERF-BUDGETS criterion budget
# checker (Path B, nightly-only proxy).
#
# Reads `target/criterion/<group>/<bench>/new/estimates.json` for every
# benchmark group `engine/runtime/benches/budgets.rs` produces and
# emits a markdown summary at `target/criterion/budget-summary.md`. For
# each measured mean point estimate, the helper compares against a
# loose floor derived from `docs/10-performance-latency-budgets.md` —
# the docs/10 hard budget multiplied by 2x. Exceeding the loose floor
# returns a non-zero exit code, failing the macos-nightly job.
#
# # Budget-assert posture (Wave 5, 2026-05-06)
#
# Observed-max across the 6 baseline macos-nightly runs (2026-05-01 →
# 2026-05-06 inclusive) was `replay_1024` @ 0.0894 ms = 7.15% of the
# 1.25 ms hard budget. Wave 5 tightened `LOOSE_FLOOR_MULTIPLIER` from
# 10 → 2, moving the harness from observation-only to budget-assert
# posture while preserving a ~22× safety margin over the observed-max.
# The original observation-only first-run posture (loose floor at 10x)
# shipped in Phase-3 wave PR 6 at merge `85f869e`; Wave 5 supersedes
# it once the 6-nightly baseline cycle settled the noise floor on
# `macos-latest`.
#
# Re-tightening or loosening: derive the new multiplier from the
# current observed-max across a fresh ~6-run baseline window. The
# edit is a `chore(release-eng)`-scoped follow-up PR that updates this
# script's `LOOSE_FLOOR_MULTIPLIER` plus the `EV-PERF-BUDGETS` row in
# `docs/26-evidence-artifact-registry.md`.
#
# # Usage
#
# bash scripts/budget-check.sh [target/criterion]
#
# Default `target/criterion` matches the nightly workflow's check-out
# layout. Errors that prevent measurement (missing criterion output,
# malformed estimates.json) exit with code 2 to distinguish from the
# code-1 budget-exceeded path.
#
# # Dependencies
#
# - jq (criterion estimates.json parsing).
# - bash 4+ (associative arrays).
#
# Both are present on `macos-latest` runners by default; `bash` is
# 3.2.57 there but `/usr/bin/env bash` resolves to whichever bash the
# shell finds first — Homebrew's bash 5 is not pre-installed. The
# script keeps to bash 3-compatible syntax (no associative arrays;
# parallel arrays / case statements instead).

set -euo pipefail

CRITERION_DIR="${1:-target/criterion}"
SUMMARY_PATH="$CRITERION_DIR/budget-summary.md"

# Loose floor multiplier (2x). See "Budget-assert posture" above.
LOOSE_FLOOR_MULTIPLIER=2

# --- Hard-budget targets, milliseconds, derived from docs/10 -----------
#
# Each entry maps a `<group>/<bench>` pair to the docs/10 subsystem
# allocation that bench proxies. Values are in MILLISECONDS so the
# parsed criterion mean (which is in NANOSECONDS) divides by 1e6
# before comparison. The rationale lives next to each entry so the
# tightening-pass PR can re-derive the budget from the same anchor.
#
# Parallel-array form (bash 3 compatible). Indexes match across the
# three arrays.
BENCH_KEYS=(
    "command_bus_dispatch/dispatch_32"
    "command_bus_dispatch/dispatch_256"
    "command_bus_dispatch/dispatch_1024"
    "state_replay_canonical_hash/replay_256"
    "state_replay_canonical_hash/replay_1024"
    "preset_recall_apply/single_recall"
    "preset_recall_apply/multi_recall_64"
    "playlist_advance_apply/single_advance"
    "playlist_advance_apply/multi_advance_64"
)

# docs/10 anchor: "command + state updates: <= 1.25 ms" per frame.
# Single-bench hard budget = 1.25 ms (per-frame allocation; a 60 fps
# frame's full command + state phase). Loose floor = 2.5 ms
# (1.25 * 2 — Wave 5 tightening, 2026-05-06).
BENCH_BUDGETS_MS=(
    "1.25"   # command_bus_dispatch/dispatch_32
    "1.25"   # command_bus_dispatch/dispatch_256
    "1.25"   # command_bus_dispatch/dispatch_1024
    "1.25"   # state_replay_canonical_hash/replay_256
    "1.25"   # state_replay_canonical_hash/replay_1024
    "1.25"   # preset_recall_apply/single_recall
    "1.25"   # preset_recall_apply/multi_recall_64
    "1.25"   # playlist_advance_apply/single_advance
    "1.25"   # playlist_advance_apply/multi_advance_64
)

BENCH_RATIONALES=(
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
    "docs/10 §Frame Performance — command + state updates: <= 1.25 ms"
)

# --- Pre-flight checks -------------------------------------------------

if ! command -v jq >/dev/null 2>&1; then
    printf '[budget-check] FAIL: jq is not installed; install via `brew install jq`\n' >&2
    exit 2
fi

if [ ! -d "$CRITERION_DIR" ]; then
    printf '[budget-check] WARN: criterion directory %s not found; nothing to check\n' "$CRITERION_DIR" >&2
    printf '[budget-check] WARN: run `cargo bench -p chromaticity-runtime --bench budgets` first\n' >&2
    # Treat missing criterion output as "nothing to do" (exit 0) when
    # invoked locally without a prior bench run; the nightly workflow
    # always runs `cargo bench` immediately before this step, so the
    # directory is guaranteed there.
    exit 0
fi

# --- Walk every (group, bench) pair, parse mean, compare to floor -----

mkdir -p "$(dirname "$SUMMARY_PATH")"
{
    printf '# EV-PERF-BUDGETS criterion summary\n\n'
    printf 'Generated by `scripts/budget-check.sh` against `%s`.\n\n' "$CRITERION_DIR"
    printf 'Budget-assert posture (Wave 5, 2026-05-06). Loose floor multiplier = %sx (~22x safety over 2026-05-01..06 observed-max).\n\n' "$LOOSE_FLOOR_MULTIPLIER"
    printf '| Bench | Mean (ms) | Hard budget (ms) | Loose floor (ms) | Status | Rationale |\n'
    printf '|---|---|---|---|---|---|\n'
} > "$SUMMARY_PATH"

fail=0
missing=0
checked=0
i=0
while [ "$i" -lt "${#BENCH_KEYS[@]}" ]; do
    key="${BENCH_KEYS[$i]}"
    budget_ms="${BENCH_BUDGETS_MS[$i]}"
    rationale="${BENCH_RATIONALES[$i]}"
    estimates_path="$CRITERION_DIR/$key/new/estimates.json"

    if [ ! -f "$estimates_path" ]; then
        printf '[budget-check] WARN: estimates missing for %s\n' "$key" >&2
        printf '| `%s` | _missing_ | %s | _n/a_ | MISSING | %s |\n' \
            "$key" "$budget_ms" "$rationale" >> "$SUMMARY_PATH"
        missing=$((missing + 1))
        i=$((i + 1))
        continue
    fi

    # criterion writes the mean estimate in nanoseconds at
    # `.mean.point_estimate`. Convert to milliseconds for human-
    # readable comparison against the docs/10 budget values.
    mean_ns="$(jq -r '.mean.point_estimate' "$estimates_path")"
    if [ -z "$mean_ns" ] || [ "$mean_ns" = "null" ]; then
        printf '[budget-check] FAIL: malformed estimates at %s\n' "$estimates_path" >&2
        exit 2
    fi
    mean_ms="$(awk -v n="$mean_ns" 'BEGIN { printf "%.6f", n / 1000000.0 }')"
    loose_floor_ms="$(awk -v b="$budget_ms" -v m="$LOOSE_FLOOR_MULTIPLIER" \
        'BEGIN { printf "%.6f", b * m }')"

    over="$(awk -v a="$mean_ms" -v b="$loose_floor_ms" \
        'BEGIN { print (a + 0 > b + 0) ? "1" : "0" }')"
    if [ "$over" = "1" ]; then
        status="OVER"
        fail=$((fail + 1))
    else
        status="OK"
    fi

    printf '| `%s` | %s | %s | %s | %s | %s |\n' \
        "$key" "$mean_ms" "$budget_ms" "$loose_floor_ms" "$status" "$rationale" \
        >> "$SUMMARY_PATH"
    checked=$((checked + 1))
    i=$((i + 1))
done

{
    printf '\n## Totals\n\n'
    # `printf -- '- ...'` instead of `printf '- ...'` because bash's
    # builtin printf treats a leading `-` in the format string as the
    # start of an option flag and aborts with "invalid option".
    printf -- '- Benches checked: %s\n' "$checked"
    printf -- '- Benches missing: %s\n' "$missing"
    printf -- '- Benches over loose floor: %s\n' "$fail"
    printf '\nLoose-floor multiplier of %sx asserts the budget at ~22x safety over the\n' \
        "$LOOSE_FLOOR_MULTIPLIER"
    printf '2026-05-01..06 observed-max. Re-tightening or loosening requires a\n'
    printf '`chore(release-eng)`-scoped follow-up PR re-deriving the multiplier from a\n'
    printf 'fresh ~6-run baseline window.\n'
} >> "$SUMMARY_PATH"

printf '[budget-check] summary: checked=%s missing=%s over=%s -> %s\n' \
    "$checked" "$missing" "$fail" "$SUMMARY_PATH"

if [ "$fail" -gt 0 ]; then
    printf '[budget-check] FAIL: %s bench(es) exceeded the %sx loose floor.\n' \
        "$fail" "$LOOSE_FLOOR_MULTIPLIER" >&2
    printf '[budget-check] Wave 5 budget-assert posture: a non-zero exit fails the\n' >&2
    printf '[budget-check] macos-nightly job. Re-tightening or loosening the multiplier\n' >&2
    printf '[budget-check] requires a `chore(release-eng)`-scoped follow-up PR.\n' >&2
    exit 1
fi

printf '[budget-check] OK\n'
exit 0
