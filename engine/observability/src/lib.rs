// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine observability.
//!
//! Surface (Phase-1 slice-2 baseline widened through Phase-4a
//! `EV-HEALTH-METRICS-LOOPBACK-ONLY` and Phase-4d Wave 4d.1 / 4d.2 /
//! 4d.3 lanes):
//!
//! - [`catalog`] — Rust type mirror of the metric catalog at
//!   `docs/31-supportability-privacy-ops-policy.md` §Scheme and
//!   Metric Catalog (landed in PR number 19). 14 metrics across
//!   5 subsystems (runtime, scene, audio, osc, observability-meta)
//!   plus the label-value enums each metric's `labels` column
//!   references.
//! - [`emit`] — [`emit::EmitMetric`] object-safe trait plus an
//!   [`emit::InMemoryEmitter`] stub that round-trips events through
//!   an internal `Mutex<Vec<_>>` without network or filesystem I/O.
//! - [`analytics_audit`] — Wave 4d.1 zero-by-default analytics-event
//!   audit scaffolding for `EV-ANALYTICS-ZERO-DEFAULT` (row 92).
//! - [`chaos`] — Wave 4d.2 chaos-injection harness producing
//!   stability-failure events the soak runner consumes.
//! - [`crash_redaction`] — Wave 4d.1 Lane A primitives encoding the
//!   ADR-016 §Crash reporting redaction contract (`main.glsl` /
//!   `script.js` / media buffers / operator free-text / license-token
//!   bytes excluded from any future crash-report payload).
//! - [`crash_harness`] — synthetic crash-payload construction +
//!   validation harness composing on top of [`crash_redaction`]
//!   (PR 2 of the row 93 sequence). Builds shape-realistic payloads
//!   across four simulated crash classes (SIGSEGV,
//!   panic-in-render-thread, OOM, assertion-failure), runs them
//!   through [`crash_redaction::CrashPayloadRedactor`], and asserts
//!   no sentinel value from the sensitive-field set survives the
//!   redaction transform. Registry row 93
//!   (`EV-CRASH-PAYLOAD-REDACTION`) is scope-narrowed to the
//!   synthetic-harness path; the row remains `Planned` pending a
//!   later wave PR.
//! - [`soak`] — Wave 4d.3 long-running soak runner for `EV-SOAK-CHAOS`.
//!
//! Divergence between the Rust types in [`catalog`] and docs/31 is a
//! **schema-drift failure**. If the catalog itself needs amendment,
//! raise a `docs(policy)` PR against docs/31 first (NON-DELEGABLE
//! surface, AO-authored), then update the Rust types to match the
//! amended baseline.
//!
//! This crate does NOT ship a network transport. The Prometheus
//! text-format endpoint (loopback-only, opt-in via Settings) is
//! covered by `EV-HEALTH-METRICS-LOOPBACK-ONLY` per ADR-016 and
//! lives outside this crate.
//!
//! ADR-016 posture constraints this crate honours:
//!
//! - **No network I/O by default.** Absolutely none at Phase 1.
//! - **No payload logging.** No scene source, no script source, no
//!   media payloads, no audio buffers, no operator-entered text.
//! - **No outbound HTTPS telemetry** at any phase except the
//!   Phase 4d loopback-only health endpoint (not in this crate).
//! - **Opt-in per-Settings-toggle**; consent surfaces live in the
//!   app shell, not in this library.
//!
//! Governing documents:
//!
//! - `docs/31-supportability-privacy-ops-policy.md` — metric catalog
//!   (authoritative).
//! - `docs/adr/ADR-016-observability-and-telemetry-posture.md` —
//!   posture constraints.
//! - `docs/26-evidence-artifact-registry.md` — `EV-OBSERVABILITY`
//!   row.
//! - `docs/12-work-breakdown-estimates.md` — WP-8 ownership and
//!   gate outputs.

pub mod analytics_audit;
pub mod catalog;
pub mod chaos;
pub mod crash_harness;
pub mod crash_redaction;
pub mod emit;
pub mod soak;

pub use chaos::{
    ChaosInjectionError, ChaosInjectionOutcome, ChaosPlan, ChaosReport, ChaosScheduleEntry,
    ChaosWorkload, FaultEvent, FiredFaultEvent, StabilityFailureChaosClass, run_chaos_soak,
};
pub use crash_harness::{
    CrashPayloadValidationReport, SimulatedCrashClass, SyntheticCrashPayload,
    validate_synthetic_crash_payload,
};
pub use soak::{
    SoakConfig, SoakHarnessError, SoakReport, SoakWorkload, SoakWorkloadError, StabilityFailure,
    StabilityFailureKind, run_soak,
};

/// Construct the current observability schema-version metric event.
///
/// This free function is the Phase-1 emission site for
/// `chromaticity_observability_schema_version`: callers consume the
/// crate-root API and receive the catalog-typed gauge event for the
/// running binary.
#[must_use]
pub fn current_schema_version() -> catalog::MetricEvent {
    catalog::MetricEvent::ObservabilitySchemaVersion {
        value: catalog::PHASE_1_SCHEMA_VERSION,
    }
}
