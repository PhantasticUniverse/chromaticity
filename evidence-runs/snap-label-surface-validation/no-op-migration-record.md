<!-- SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# No-op migration record — snap-label surface validation

**Lane:** Team B — Runtime / Observability
**Branch:** `observability/snap-label-surface-validation`
**Anchor commit:** `09a067d2562f614581a8db9f17cb82bcc0a11f3a` (PR #83 merge to `main`)
**Captured:** 2026-04-24

## Conclusion

There is no operator-facing query pack, dashboard, alerting rule, or scraper configuration
in this repository that consumes the runtime Snap command/event labels. PR #83 completed
the only source-of-truth migration this repository can support today: the catalog enums,
the runtime-bus counter shape, and the `docs/31` closed-set vocab block all enumerate the
six dedicated snap labels (`snap_recall` / `snap_save` / `snap_clear` and their event
mirrors). Migration of consumer artifacts is a no-op because no consumer artifact exists
to migrate.

The `phase-4d-handoff-checklist.md` sibling artifact in this directory governs the first
future PR that introduces an operator-facing consumer.

## Exhaustive search ran

```
rg -l --hidden \
  -g '!Contents/**' -g '!chromaticity-planning/**' \
  -g '!target/**' -g '!node_modules/**' -g '!.git/**' \
  -e 'control_set|control_changed|chromaticity_runtime_commands_dispatched|chromaticity_runtime_events_emitted|snap_recall|snap_save|snap_clear|snap_recalled|snap_saved|snap_cleared|alert|alerts|dashboard|query.?pack|prom(ql|etheus)' \
  .
```

Returned: `docs/49`, `docs/12`, `engine/runtime/src/state.rs` (snap_clear test name only),
`docs/20`, `docs/31`, `engine/runtime/src/command_bus.rs`, `engine/observability/src/catalog.rs`,
`docs/13`. Inspection of every match confirmed they are either source-of-truth files
(catalog, runtime bus, schema doc) or narrative references in forward-looking planning
docs (`role="alert"` ARIA usage in `docs/49`, future "budget alerting hooks" line in
`docs/12`, "Telemetry schema and dashboard checks" cell in the traceability matrix
`docs/20`, "operator health alerts with fallback recommendations" line in the risk
register `docs/13`). None of these are load-bearing artifacts that consume catalog
labels.

```
find . -path ./node_modules -prune -o -path ./target -prune \
       -o -path ./Contents -prune -o -path ./chromaticity-planning -prune \
       -o -path ./.git -prune -o -type f \
       \( -iname '*alert*' -o -iname '*dashboard*' -o -iname '*query*pack*' \
          -o -iname '*.promql' -o -iname '*.rules.yml' -o -iname '*.rules.yaml' \
          -o -iname 'grafana*' -o -iname 'prom*.yml' -o -iname 'prom*.yaml' \) \
       -print
```

Returned: zero files.

## Operator-facing directories absent

Confirmed missing from the repo root:

- `monitoring/`
- `observability/` (operator-facing, top-level)
- `dashboards/`
- `alerts/`
- `ops/`
- `runbooks/`

Adjacent existing trees and what they contain:

- `engine/observability/` — Rust crate only: `catalog.rs`, `emit.rs`, `lib.rs`. In-process
  schema and `InMemoryEmitter` stub. No transport, no operator surface.
- `release-eng/` — empty top-level directory; per `CLAUDE.md` reserved for signing /
  notarization / updater. Not the natural home for observability consumer artifacts.
- `integrations/osc-adapter/` — input-side adapter. Not an observability consumer.
- `app/shell/` — Tauri shell. Carries the future opt-in consent surface but no scraper
  config today.

## Why this is the correct posture (not a gap)

Phase 1 is in-process only by explicit policy:

- `docs/adr/ADR-016-observability-and-telemetry-posture.md` lines 36-37:

  > "Vendor: Prometheus exposition over loopback only. When operators opt in, the runtime
  > exposes a local Prometheus text endpoint on `127.0.0.1:<port>` (port chosen per
  > `adr/ADR-013-networking-defaults-and-security.md` bind defaults). Operators who want
  > to aggregate across machines run their own scraper."
  > "No outbound HTTPS telemetry at any point. Health metrics never leave the local
  > machine without an explicit operator-configured scraper pointed at the loopback
  > endpoint."

- `engine/observability/src/lib.rs` lines 24-27:

  > "Slice-2 does NOT ship any transport. The Prometheus text-format endpoint
  > (loopback-only, opt-in via Settings) is a Phase 4d item per ADR-016. The runtime-bus
  > wiring that feeds actual events into `emit::EmitMetric` implementations is slice-3."

- `docs/31-supportability-privacy-ops-policy.md` §Scope lines 33-37 list the Phase-1
  baseline as "in-process only; no network transport is activated" and reserve
  "Phase 4d: transport activation per ADR-016 §Health metrics".

The "control-folding" PR #83 unwound was a **bus-local in-memory counter shape** — never
reachable by any external scraper, dashboard, or alerting rule, because the transport
that would expose it has not been activated and no operator-authored query consumer
exists in this tree. The folding lived on a non-shipped Phase-1 in-process emitter for
≤ 1 commit before PR #83 split it.

## What changed in `docs/31` itself (operator-facing communication)

PR #83's `docs/31` edit (lines 73-78) is the single operator-facing artifact in-tree
that the snap-label split affects. It declares the Phase-1 closed set verbatim:

- `chromaticity_runtime_commands_dispatched_total.kind`:
  `control_set`, `preset_recall`, `playlist_advance`, `snap_recall`, `snap_save`,
  `snap_clear`, `mapping_bind`
- `chromaticity_runtime_events_emitted_total.kind`:
  `control_changed`, `preset_recalled`, `playlist_advanced`, `snap_recalled`,
  `snap_saved`, `snap_cleared`, `mapping_bound`

Any future operator-authored query consumer reads `docs/31` as authoritative; the closed
set is the contract the catalog conformance harness pins against (see the bundle in
`closed-set-conformance-bundle.md` for the Rust mirror).

## Scope respected

This lane did not touch:

- `.github/workflows/policy.yml`
- `CODEOWNERS`
- `docs/26-evidence-artifact-registry.md` (downstream wording is Team C's lane)
- any consumer-artifact file (none was invented)
