# Supportability, Privacy, and Operator Runbook Policy

## Purpose
Define minimum operational support controls for commercial release readiness.

## Support Bundle Policy
- Support bundle export is opt-in per session unless explicit enterprise policy overrides it.
- Bundle minimum contents:
 - runtime version + profile metadata,
 - recent diagnostics and subsystem health events,
 - bounded performance/SLO snapshots,
 - adapter connectivity timeline,
 - anonymized crash identifiers (if present).
- Bundle exclusions by default:
 - raw camera frames,
 - raw audio streams,
 - full scene asset binaries unless explicitly selected.

## Telemetry and Log Privacy Policy
- Collect only telemetry needed for reliability/performance diagnosis.
- Redact or hash user-identifying fields in logs and telemetry.
- Retention policy:
 - local logs default retention: 30 days,
 - exported support bundles: user-controlled retention,
 - crash report payload retention follows legal/privacy policy baseline.
- Any telemetry schema change requires privacy impact checklist update.

## Scheme and Metric Catalog

Authoritative target for the forward reference in `adr/ADR-016-observability-and-telemetry-posture.md` §Health metrics (line 38). This section declares the runtime health-metric schema. Transport for the schema is governed by `ADR-016` and is Phase-4d work; Phase 1 emits into an in-process sink only.

### Scope

- **Phase 1 baseline (this section).** Lifecycle, scene-validate, audio-ingest-state, osc-adapter-state, runtime-bus counters. Emission is in-process only; no network transport is activated.
- **Phase 2 additions (deferred).** Render-pass timing, script execution timing, audio-analysis uniform publication latency.
- **Phase 3 additions (deferred).** Frame-time percentiles, per-pass budget tracking, state-replay and state-migration counters.
- **Phase 4d (deferred).** Transport activation per ADR-016 §Health metrics: Prometheus text-format endpoint on `127.0.0.1:<port>` gated by `telemetry.enabled`. Port chosen per `adr/ADR-013-networking-defaults-and-security.md` bind defaults. No outbound HTTPS telemetry at any phase.

### Naming convention

- Global namespace: `chromaticity_`.
- Form: `chromaticity_<subsystem>_<name>_<unit>`, all lower-case snake_case.
- Unit suffixes match Prometheus convention: `_total` for counters, `_seconds` for durations, `_bytes` for sizes, no suffix for gauges. Histograms carry the base name (e.g. `chromaticity_render_pass_duration_seconds`) with auto-generated `_bucket`, `_sum`, `_count` companions per Prometheus exposition rules.
- Fixed histogram buckets declared per metric; no runtime-configurable bucket boundaries (reproducibility invariant).

### Label policy

- Labels permitted only for bounded, enumerable categories (subsystem name, state name, outcome class). Label value sets are closed at build time.
- Labels **forbidden** on any unbounded value: scene names, node IDs, packet IDs, session IDs, user-provided strings of any kind, file paths, IP addresses, hostnames.
- Label cardinality budget: no metric exceeds 64 active label-value combinations. Transitions between bounded state machines (e.g. `{from, to}` labels over a 5-state enum) are within budget; anything larger is a schema review item.

### Phase-1 baseline catalog

| Metric | Type | Labels | Purpose |
| --- | --- | --- | --- |
| `chromaticity_runtime_lifecycle_transitions_total` | counter | `from`, `to` (runtime lifecycle enum) | Process-lifetime state-machine transitions |
| `chromaticity_runtime_uptime_seconds` | gauge | — | Wall-clock seconds since runtime start |
| `chromaticity_runtime_commands_dispatched_total` | counter | `kind` (command enum discriminant) | Commands accepted through the bus |
| `chromaticity_runtime_events_emitted_total` | counter | `kind` (event enum discriminant) | Events published through the bus |
| `chromaticity_scene_validations_total` | counter | `outcome` (`pass` \| `fail`) | Scene-validator invocations |
| `chromaticity_scene_lint_warnings_total` | counter | `category` (lint category enum) | Schema-lint warnings raised |
| `chromaticity_audio_ingest_state_transitions_total` | counter | `from`, `to` (ingest state enum) | IngestBoundary state transitions |
| `chromaticity_audio_ingest_frames_total` | counter | `status` (`ready` \| `would_block` \| `closed`) | try_pull outcomes |
| `chromaticity_audio_ingest_samples_total` | counter | — | Monotonic total PCM samples delivered |
| `chromaticity_osc_inbound_packets_total` | counter | `outcome` (`accepted` \| `oversized_dropped` \| `allowlist_rejected` \| `rate_limited_dropped` \| `parse_error`) | Inbound OSC packet disposition |
| `chromaticity_osc_adapter_state_transitions_total` | counter | `from`, `to` (adapter state enum per `docs/08-io-integrations.md`) | OSC adapter lifecycle transitions |
| `chromaticity_osc_bind_errors_total` | counter | `reason` (bind-error category enum) | Socket-bind failure categorisation |
| `chromaticity_observability_events_emitted_total` | counter | `subsystem` | Meta-counter of metric emissions by subsystem |
| `chromaticity_observability_schema_version` | gauge | — | Integer-encoded schema version of the running binary |

Phase-1 baseline is 14 metrics. All are emission-ready with or without a transport; Phase-4d merely attaches the Prometheus loopback endpoint to the same in-process emitter.

Runtime `kind` label vocab (closed set for Phase-1):

- `chromaticity_runtime_commands_dispatched_total.kind`:
  `control_set`, `preset_recall`, `preset_save`, `playlist_advance`, `snap_recall`, `snap_save`, `snap_clear`, `mapping_bind`
- `chromaticity_runtime_events_emitted_total.kind`:
  `control_changed`, `preset_recalled`, `preset_saved`, `playlist_advanced`, `snap_recalled`, `snap_saved`, `snap_cleared`, `mapping_bound`

### Privacy posture

No metric in this catalog carries user content, scene content, script content, audio samples, or any derived statistic computable against unbounded input. All values are counts, durations, or bounded-enum state. The `Any telemetry schema change requires privacy impact checklist update` rule above applies to every modification of this catalog.

### Implementation reference

The `engine/observability` crate (introduced in slice-2 per the Phase-1 work plan) carries the Rust type surface implementing this catalog. The crate's module-level `//!` doc comment references this section as the authoritative source. Divergence between the crate types and this catalog is a schema-drift failure caught by `EV-OBSERVABILITY` validation.

### Schema change process

- **Additions.** `docs(policy)` PR amending this section first; implementation PR references the amended catalog. Privacy impact checklist per §Telemetry and Log Privacy Policy line 26 updated in the same or a sibling PR.
- **Removals.** `docs(policy)` PR plus a release-note entry diffing the removed surface. Downstream scrapers are operator-run per ADR-016 so removals affect operator-authored dashboards — name and justify the removal explicitly.
- **Renames.** Treated as a removal plus an addition. Deprecation window of one minor release recommended for any metric that has appeared in a shipped release.
- **Phase-gated expansion.** The Phase 2 / Phase 3 / Phase 4d expansions listed under §Scope above land as successive `docs(policy)` PRs that extend the catalog table. Each expansion PR carries its privacy impact checklist update.

### Evidence binding

- `EV-OBSERVABILITY` (Phase 1 exit per `26-evidence-artifact-registry.md`) validates that every metric in the Phase-1 baseline catalog has a corresponding emission site in the `engine/observability` crate and that the crate's schema types match this section verbatim. `EV-OBSERVABILITY` status transitions from `pending:artifact` to `Pass` only when the T2 conformance harness over this catalog lands.
- `EV-OBSERVABILITY-INVENTORY` (Phase 0 exit, separate track) is unaffected by this section; it governs outbound-endpoint enumeration per ADR-016 §Transparency artifact, not the metric schema.

## Crash Reporting Policy
- Crash reporting is opt-in by default for unmanaged installs.
- Report payload must exclude scene source and media payload content by default.
- Include reproducibility metadata only (build version, subsystem, stack, profile class).

## Operator Runbook Minimum
- Required runbooks:
 - startup and device preflight,
 - reconnect recovery procedure,
 - degraded-mode operation,
 - emergency fallback/black-frame policy,
 - show-state restore and rollback.
- Every runbook must include trigger conditions, step-by-step recovery actions, and abort/escalation rules.

## Evidence and Gate Binding
- `EV-SUPPORT-OPS-READINESS` validates:
 - support bundle policy conformance,
 - telemetry/log redaction checks,
 - crash-report policy verification,
 - runbook completeness checklist.
- `EV-SUPPORT-OPS-READINESS` is required for Phase 5/release gate.
