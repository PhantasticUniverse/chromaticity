<!-- SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Closed-set conformance evidence bundle — snap-label surface validation

**Lane:** Team B — Runtime / Observability
**Branch:** `observability/snap-label-surface-validation`
**Anchor commit:** `09a067d2562f614581a8db9f17cb82bcc0a11f3a` (PR #83 merge to `main`)
**Captured:** 2026-04-24

## What this bundle proves

After PR #83 split the runtime Snap command/event telemetry into dedicated observability
kinds, the closed-set conformance harness over `docs/31` §Phase-1 baseline catalog stays
green for both the runtime command-kind label set and the runtime event-kind label set, and
the runtime bus emits dedicated counters for `SnapRecall` / `SnapSave` / `SnapClear` (and
their event mirrors) rather than folding them into `ControlSet` / `ControlChanged`.

Together these pin the post-PR-#83 invariant the lead named: closed-set conformance against
the source-of-truth triad cited below.

## Source-of-truth triad

| Layer                         | File                                           | Anchors                                                                                                                                                                                                            |
| ----------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Catalog (Rust)                | `engine/observability/src/catalog.rs`          | `CommandKind` enum (lines 73-83), `EventKind` enum (lines 89-99), `EXPECTED_RUNTIME_COMMAND_KIND_LABELS` (lines 667-675), `EXPECTED_RUNTIME_EVENT_KIND_LABELS` (lines 679-687), closed-set tests (lines 1004-1038) |
| Runtime bus                   | `engine/runtime/src/command_bus.rs`            | `CommandKindCounts` dedicated fields (lines 361-370), `EventKindCounts` dedicated fields (lines 374-383), `command_kind` / `event_kind` mappers (lines 402-426), Snap emission tests (lines 1011-1023, 1060-1073)  |
| Schema doc (closed-set vocab) | `docs/31-supportability-privacy-ops-policy.md` | §Phase-1 baseline catalog Runtime `kind` label vocab block (lines 73-78) — added by PR #83                                                                                                                         |

The three layers are bidirectionally checked: the catalog conformance harness mirrors
`docs/31` verbatim into the Rust `EXPECTED_*` tables; the runtime bus mappers exhaustively
match each command/event variant to its dedicated `CommandKind` / `EventKind` (no `_` arm
in either direction); `docs/31` §Schema change process names additions/removals/renames as
`docs(policy)` PRs that must round-trip through this harness.

## Cargo invocation

Run from repo root:

```
cargo test -p chromaticity-observability -p chromaticity-runtime --no-fail-fast -- --exact \
  catalog::catalog_conformance::runtime_command_kind_labels_match_docs31_closed_set \
  catalog::catalog_conformance::runtime_event_kind_labels_match_docs31_closed_set \
  command_bus::observability_emission::snap_variants_increment_dedicated_command_counters \
  command_bus::observability_emission::snap_events_increment_dedicated_event_counters
```

## Captured output (2026-04-24, anchor commit `09a067d`)

```
     Running unittests src/lib.rs (target/debug/deps/chromaticity_observability-…)

running 2 tests
test catalog::catalog_conformance::runtime_command_kind_labels_match_docs31_closed_set ... ok
test catalog::catalog_conformance::runtime_event_kind_labels_match_docs31_closed_set ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/chromaticity_runtime-…)

running 2 tests
test command_bus::observability_emission::snap_events_increment_dedicated_event_counters ... ok
test command_bus::observability_emission::snap_variants_increment_dedicated_command_counters ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 0.00s
```

Raw log: `/tmp/snap-label-conformance.log` (working-copy artifact, not committed).

## Supporting harness (full crate run, same anchor commit)

The four lead-named tests are part of a broader catalog-conformance + runtime-emission
harness. The full crate run on the same SHA:

```
cargo test -p chromaticity-observability -p chromaticity-runtime --no-fail-fast
→ chromaticity-observability:  17 passed,  0 failed
→ chromaticity-runtime:         20 passed,  0 failed
→ determinism integration:       6 passed,  0 failed
```

Notable companions in the same harness that backstop the snap-label split:

- `catalog::catalog_conformance::every_docs31_row_has_a_catalog_variant`
- `catalog::catalog_conformance::every_catalog_variant_appears_in_docs31`
- `catalog::catalog_conformance::label_sets_match_docs31`
- `catalog::catalog_conformance::metric_kinds_match_docs31`
- `catalog::catalog_conformance::subsystem_assignments_match_docs31`
- `catalog::catalog_conformance::catalog_has_exactly_phase1_baseline_count`
- `command_bus::observability_emission::commit_one_increments_matching_event_kind_counter`

These cross-pin the row count (14), the bidirectional name set, the kind/subsystem
assignments, and the per-variant counter chain that PR #83 reshaped.

## Drift-detection mechanics that keep this bundle re-runnable

- **Compile-time:** `every_metric_event_variant`, `label_names`, `every_command_kind_variant`,
  and `every_event_kind_variant` (all in `engine/observability/src/catalog.rs`) use exhaustive
  `match` with no `_` arm. Adding a `MetricEvent` / `CommandKind` / `EventKind` variant
  without extending these matches and the corresponding `EXPECTED_*` tables fails at rustc
  level inside the defining crate, before any test even runs.
- **Test-time:** the closed-set tests compare `HashSet<&str>` of label tokens against the
  `EXPECTED_RUNTIME_*_LABELS` constants, which are transcribed from `docs/31` lines 73-78
  verbatim. A docs-side rename or addition without the paired Rust update is a hard test
  failure with a "label drift vs docs/31 closed set" message.

## Re-running on a future commit

To re-confirm conformance against a later anchor SHA, run the cargo invocation above and
attach the captured output plus the current SHA to a refreshed copy of this bundle. The
source-of-truth triad table only needs an update if the file paths change.
