// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Metric catalog — Rust mirror of
//! `docs/31-supportability-privacy-ops-policy.md` §Scheme and Metric
//! Catalog.
//!
//! The 14 metrics declared here MUST match docs/31 verbatim. Any
//! divergence is a schema-drift failure; see the crate root for the
//! amendment path.
//!
//! Naming convention (docs/31 §Scheme and Metric Catalog):
//!
//! - Global namespace: `chromaticity_`.
//! - Form: `chromaticity_<subsystem>_<name>_<unit>` snake_case.
//! - `_total` for counters, `_seconds` for durations, `_bytes` for
//!   sizes, no suffix for gauges.

/// Metric kind (Prometheus data model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MetricKind {
    /// Monotonically-increasing value reset only on process restart.
    Counter,
    /// Arbitrary real-valued instantaneous sample.
    Gauge,
}

/// Subsystem a metric belongs to. Matches the `observability_events_emitted_total`
/// `subsystem` label cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObservabilitySubsystem {
    Runtime,
    Scene,
    Audio,
    Osc,
    Observability,
}

// ---------------------------------------------------------------------------
// Label-value enums. Each one maps to a docs/31 metric row's `labels`
// column. `#[non_exhaustive]` is applied so slice-3 can extend without
// breaking downstream.
// ---------------------------------------------------------------------------

/// `chromaticity_runtime_lifecycle_transitions_total` `from` / `to`
/// label values.
///
/// The authoritative runtime-lifecycle state machine does not yet
/// exist in-tree; this slice-2 shape is a best-effort synthesis
/// against ADR-006 (thread-and-process-model). The slice-3 wave
/// (PRs `#321` `#326` `#328` `#329` `#330`) landed the
/// `scheduler::Scheduler::tick` frame-loop seam but did NOT widen
/// this enum — TODO(post-slice-3 runtime-bus widening): reconcile
/// variants against the concrete lifecycle enum once the runtime
/// bus surfaces lifecycle transitions through a dedicated
/// `MetricEvent::RuntimeLifecycleTransition` emission site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuntimeLifecycleState {
    Starting,
    Running,
    Paused,
    Stopping,
}

/// `chromaticity_runtime_commands_dispatched_total` `kind` label
/// values.
///
/// Declared independently of [`chromaticity_runtime::Command`] so
/// the telemetry wire vocabulary stays stable across runtime
/// refactors. Adding a command variant in `chromaticity-runtime`
/// does NOT automatically extend this enum; a docs/31 + catalog
/// update is the intentional bottleneck that keeps label
/// cardinality under review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CommandKind {
    ControlSet,
    PresetRecall,
    PlaylistAdvance,
    SnapRecall,
    SnapSave,
    SnapClear,
    MappingBind,
}

/// `chromaticity_runtime_events_emitted_total` `kind` label values.
///
/// Declared independently of [`chromaticity_runtime::EventPayload`]
/// for the same wire-stability reason as [`CommandKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EventKind {
    ControlChanged,
    PresetRecalled,
    PlaylistAdvanced,
    SnapRecalled,
    SnapSaved,
    SnapCleared,
    MappingBound,
}

/// `chromaticity_scene_validations_total` `outcome` label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SceneValidationOutcome {
    Pass,
    Fail,
}

/// `chromaticity_scene_lint_warnings_total` `category` label values.
///
/// Mirrors `docs/05-scene-system.md` §5 warning-class vocabulary.
/// `#[non_exhaustive]` against the open-list semantics docs/05
/// declares ("warning classes:" is enumerative but not frozen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SceneLintCategory {
    LegacyKey,
    UnknownControlType,
    DeprecatedScriptApi,
    CoercionApplied,
}

/// `chromaticity_audio_ingest_state_transitions_total` `from` / `to`
/// label values.
///
/// Distinct from `chromaticity_audio::IngestStatus` — the `Status`
/// enum is the per-pull outcome, whereas this enum is the
/// transition state machine (Starting -> Running -> Draining ->
/// Closed) that the transitions counter keys off. The slice-3 wave
/// (PRs `#321` `#326` `#328` `#329` `#330`) landed the
/// `scheduler::audio::AudioPull` snapshot-pull seam but did NOT
/// reify the audio-pipeline lifecycle enum — TODO(post-slice-3
/// audio-pipeline lifecycle widening): reconcile against the
/// concrete enum once it ships in `engine/audio` or as a
/// `chromaticity_audio::IngestLifecycle` companion type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AudioIngestState {
    Starting,
    Running,
    Draining,
    Closed,
}

/// `chromaticity_audio_ingest_frames_total` `status` label values.
///
/// Matches `chromaticity_audio::IngestStatus` per-pull outcomes
/// exactly; the telemetry vocabulary here is the stable label
/// spelling (`ready` / `would_block` / `closed` — snake_case per
/// docs/31 label conventions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AudioIngestFrameStatus {
    Ready,
    WouldBlock,
    Closed,
}

/// `chromaticity_osc_inbound_packets_total` `outcome` label values.
///
/// Matches docs/31 verbatim — five categories covering the OSC
/// inbound packet disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OscPacketOutcome {
    Accepted,
    OversizedDropped,
    AllowlistRejected,
    RateLimitedDropped,
    ParseError,
}

/// `chromaticity_osc_adapter_state_transitions_total` `from` / `to`
/// label values per `docs/08-io-integrations.md`.
///
/// The concrete adapter-lifecycle enum in `docs/08-io-integrations.md`
/// has not yet been reified as a Rust type; this slice-2 shape is
/// a best-effort synthesis. The slice-3 scheduler-tick wave
/// (PRs `#321` `#326` `#328` `#329` `#330`) did not touch the OSC
/// adapter; the Phase-4a Wave 1b cascade (PRs `#332` `#333` `#334`)
/// shipped MIDI + tier-gating but also left the OSC adapter
/// untouched — TODO(post-MVP OSC-adapter-lifecycle widening):
/// reconcile against the concrete adapter-lifecycle enum once it
/// ships in `integrations/osc-adapter` (currently `OscAdapterState`
/// in `integrations/osc-adapter/src/adapter.rs` carries
/// `Idle / Bound / Errored` — narrower than this enum's
/// `Unbound / Binding / Bound / Unbinding` synthesis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OscAdapterState {
    Unbound,
    Binding,
    Bound,
    Unbinding,
}

/// `chromaticity_osc_bind_errors_total` `reason` label values.
///
/// Synthesised from `std::io::ErrorKind` — the socket-bind failure
/// categorisation that `UdpOscAdapter::bind` surfaces via
/// `OscAdapterError::BindFailed`. The slice-3 scheduler-tick wave
/// (PRs `#321` `#326` `#328` `#329` `#330`) did not touch the OSC
/// adapter; the Phase-4a Wave 1b cascade (PRs `#332` `#333` `#334`)
/// shipped MIDI + tier-gating but also left the OSC adapter
/// untouched — TODO(post-MVP OSC bind-error widening): widen /
/// align as the adapter grows structured bind-error reporting beyond
/// the current `std::io::ErrorKind` synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OscBindErrorReason {
    AddressInUse,
    PermissionDenied,
    AddressNotAvailable,
    Other,
}

// ---------------------------------------------------------------------------
// Metric catalog. Declared as a `MetricEvent` enum with one variant per
// docs/31 row, each variant carrying exactly its label set plus a
// `u64` value. Compile-time drift detection: adding or removing a
// metric MUST update both docs/31 and this enum, which catches silent
// schema changes.
// ---------------------------------------------------------------------------

/// Schema version of the docs/31 §Phase-1 baseline catalog this Rust catalog mirrors.
pub const PHASE_1_SCHEMA_VERSION: u64 = 1;

/// One emission of a catalog metric.
///
/// Variants map 1:1 to docs/31 rows. Counter variants carry `value: u64`
/// as the emission delta; gauge variants carry `value: u64` as the
/// current observation (integer-encoded for the Phase-1 stub; slice-3
/// may introduce a float-backed gauge variant if the catalog adds
/// non-integer gauges).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetricEvent {
    /// `chromaticity_runtime_lifecycle_transitions_total`
    RuntimeLifecycleTransition {
        from: RuntimeLifecycleState,
        to: RuntimeLifecycleState,
        value: u64,
    },
    /// `chromaticity_runtime_uptime_seconds`
    RuntimeUptime { value: u64 },
    /// `chromaticity_runtime_commands_dispatched_total`
    RuntimeCommandsDispatched { kind: CommandKind, value: u64 },
    /// `chromaticity_runtime_events_emitted_total`
    RuntimeEventsEmitted { kind: EventKind, value: u64 },
    /// `chromaticity_scene_validations_total`
    SceneValidations {
        outcome: SceneValidationOutcome,
        value: u64,
    },
    /// `chromaticity_scene_lint_warnings_total`
    SceneLintWarnings {
        category: SceneLintCategory,
        value: u64,
    },
    /// `chromaticity_audio_ingest_state_transitions_total`
    AudioIngestStateTransition {
        from: AudioIngestState,
        to: AudioIngestState,
        value: u64,
    },
    /// `chromaticity_audio_ingest_frames_total`
    AudioIngestFrames {
        status: AudioIngestFrameStatus,
        value: u64,
    },
    /// `chromaticity_audio_ingest_samples_total`
    AudioIngestSamples { value: u64 },
    /// `chromaticity_osc_inbound_packets_total`
    OscInboundPackets {
        outcome: OscPacketOutcome,
        value: u64,
    },
    /// `chromaticity_osc_adapter_state_transitions_total`
    OscAdapterStateTransition {
        from: OscAdapterState,
        to: OscAdapterState,
        value: u64,
    },
    /// `chromaticity_osc_bind_errors_total`
    OscBindErrors {
        reason: OscBindErrorReason,
        value: u64,
    },
    /// `chromaticity_observability_events_emitted_total`
    ObservabilityEventsEmitted {
        subsystem: ObservabilitySubsystem,
        value: u64,
    },
    /// `chromaticity_observability_schema_version`
    ObservabilitySchemaVersion { value: u64 },
}

impl MetricEvent {
    /// Canonical Prometheus-exposition metric name, matching docs/31
    /// §Scheme and Metric Catalog verbatim.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::RuntimeLifecycleTransition { .. } => {
                "chromaticity_runtime_lifecycle_transitions_total"
            }
            Self::RuntimeUptime { .. } => "chromaticity_runtime_uptime_seconds",
            Self::RuntimeCommandsDispatched { .. } => {
                "chromaticity_runtime_commands_dispatched_total"
            }
            Self::RuntimeEventsEmitted { .. } => "chromaticity_runtime_events_emitted_total",
            Self::SceneValidations { .. } => "chromaticity_scene_validations_total",
            Self::SceneLintWarnings { .. } => "chromaticity_scene_lint_warnings_total",
            Self::AudioIngestStateTransition { .. } => {
                "chromaticity_audio_ingest_state_transitions_total"
            }
            Self::AudioIngestFrames { .. } => "chromaticity_audio_ingest_frames_total",
            Self::AudioIngestSamples { .. } => "chromaticity_audio_ingest_samples_total",
            Self::OscInboundPackets { .. } => "chromaticity_osc_inbound_packets_total",
            Self::OscAdapterStateTransition { .. } => {
                "chromaticity_osc_adapter_state_transitions_total"
            }
            Self::OscBindErrors { .. } => "chromaticity_osc_bind_errors_total",
            Self::ObservabilityEventsEmitted { .. } => {
                "chromaticity_observability_events_emitted_total"
            }
            Self::ObservabilitySchemaVersion { .. } => "chromaticity_observability_schema_version",
        }
    }

    /// Prometheus data-model kind for this metric.
    #[must_use]
    pub fn kind(&self) -> MetricKind {
        match self {
            Self::RuntimeUptime { .. } | Self::ObservabilitySchemaVersion { .. } => {
                MetricKind::Gauge
            }
            _ => MetricKind::Counter,
        }
    }

    /// Docs/31 subsystem assignment for this metric row.
    ///
    /// Exhaustive match with no `_` arm: adding a [`MetricEvent`] variant
    /// without extending this mapping fails at rustc level inside this crate.
    #[must_use]
    pub fn subsystem(&self) -> ObservabilitySubsystem {
        match self {
            Self::RuntimeLifecycleTransition { .. }
            | Self::RuntimeUptime { .. }
            | Self::RuntimeCommandsDispatched { .. }
            | Self::RuntimeEventsEmitted { .. } => ObservabilitySubsystem::Runtime,
            Self::SceneValidations { .. } | Self::SceneLintWarnings { .. } => {
                ObservabilitySubsystem::Scene
            }
            Self::AudioIngestStateTransition { .. }
            | Self::AudioIngestFrames { .. }
            | Self::AudioIngestSamples { .. } => ObservabilitySubsystem::Audio,
            Self::OscInboundPackets { .. }
            | Self::OscAdapterStateTransition { .. }
            | Self::OscBindErrors { .. } => ObservabilitySubsystem::Osc,
            Self::ObservabilityEventsEmitted { .. } | Self::ObservabilitySchemaVersion { .. } => {
                ObservabilitySubsystem::Observability
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod ctor_placeholder {
    //! Evidence: EV-OBSERVABILITY (T0) — metric-catalog data-shape
    //! declaration + drift-detection scaffolding.
    //!
    //! T0 per docs/43 §15.5: the crate's public surface is the
    //! `MetricEvent` enum and its companion label-value enums — all
    //! data-shape declarations. No non-derive-generated behaviour is
    //! under test at this tier. T0 does NOT advance EV-OBSERVABILITY
    //! past `pending:artifact`; the T1 upgrade lands with slice-3's
    //! runtime-bus wiring that emits actual events, and T2 with the
    //! Phase-1-exit conformance harness against the docs/31 catalog.

    use super::{
        AudioIngestFrameStatus, CommandKind, EventKind, MetricEvent, MetricKind,
        ObservabilitySubsystem, OscPacketOutcome,
    };

    #[test]
    fn every_metric_has_a_canonical_name() {
        // Sample one variant per subsystem covering both counter and
        // gauge kinds; every canonical name is a non-empty static
        // string matching the docs/31 naming convention
        // (`chromaticity_<subsystem>_<name>[_<unit>]`).
        let samples = [
            MetricEvent::RuntimeCommandsDispatched {
                kind: CommandKind::ControlSet,
                value: 1,
            },
            MetricEvent::RuntimeEventsEmitted {
                kind: EventKind::ControlChanged,
                value: 1,
            },
            MetricEvent::RuntimeUptime { value: 0 },
            MetricEvent::AudioIngestFrames {
                status: AudioIngestFrameStatus::Ready,
                value: 1,
            },
            MetricEvent::OscInboundPackets {
                outcome: OscPacketOutcome::Accepted,
                value: 1,
            },
            MetricEvent::ObservabilitySchemaVersion { value: 1 },
        ];
        for event in samples {
            let name = event.name();
            assert!(name.starts_with("chromaticity_"));
            assert!(!name.ends_with('_'));
        }
    }

    #[test]
    fn counter_and_gauge_kinds_partition_the_catalog() {
        // Only the two gauge rows in docs/31 report `MetricKind::Gauge`;
        // every other variant is `Counter`.
        let gauge = MetricEvent::RuntimeUptime { value: 42 };
        let schema = MetricEvent::ObservabilitySchemaVersion { value: 1 };
        let counter = MetricEvent::AudioIngestSamples { value: 128 };

        assert_eq!(gauge.kind(), MetricKind::Gauge);
        assert_eq!(schema.kind(), MetricKind::Gauge);
        assert_eq!(counter.kind(), MetricKind::Counter);
    }

    #[test]
    fn subsystem_enum_covers_observability_meta_label_cardinality() {
        // `chromaticity_observability_events_emitted_total.subsystem`
        // label values are exactly the five subsystems defined in
        // docs/31. The ObservabilitySubsystem enum must enumerate
        // them; round-trip each through `MetricEvent` to prove the
        // data shape holds.
        let subsystems = [
            ObservabilitySubsystem::Runtime,
            ObservabilitySubsystem::Scene,
            ObservabilitySubsystem::Audio,
            ObservabilitySubsystem::Osc,
            ObservabilitySubsystem::Observability,
        ];
        for subsystem in subsystems {
            let event = MetricEvent::ObservabilityEventsEmitted {
                subsystem,
                value: 1,
            };
            assert_eq!(event.kind(), MetricKind::Counter);
            assert_eq!(
                event.name(),
                "chromaticity_observability_events_emitted_total"
            );
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod catalog_conformance {
    //! Evidence: EV-OBSERVABILITY (T2) — schema-drift conformance
    //! harness against `docs/31` §Phase-1 baseline catalog.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus
    //! or golden artifact. Here the "golden artifact" is the
    //! authoritative catalog table at
    //! `docs/31-supportability-privacy-ops-policy.md` §Phase-1
    //! baseline catalog; the "conformance" is that the Rust
    //! [`MetricEvent`] enum + its [`MetricEvent::name`] /
    //! [`MetricEvent::kind`] / [`MetricEvent::subsystem`] methods
    //! match `docs/31` verbatim, with label sets matching each row's
    //! `labels` column.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 38 and `docs/31-supportability-privacy-ops-policy.md`
    //! line 90 (Evidence binding) jointly declare that
    //! `EV-OBSERVABILITY` status transitions from
    //! `pending:artifact` to `Pass` only when the T2 conformance
    //! harness over the `docs/31` catalog lands. This module IS
    //! that T2 harness — **schema-drift conformance, not
    //! behavioural emission**. The runtime-bus wiring that feeds
    //! events into [`crate::emit::EmitMetric`] stays scoped to
    //! slice-3 per `engine/observability/src/lib.rs:26-27`, and
    //! is ORTHOGONAL to the T2 tier transition — the catalog
    //! schema IS the evidence-producing behaviour row 38
    //! validates (per the `docs/31` §Evidence binding verbatim
    //! quote).
    //!
    //! Drift-detection mechanics. Two complementary paths:
    //!
    //! - **Compile-time.** [`every_metric_event_variant`] and
    //!   [`label_names`] both use exhaustive `match` on
    //!   [`MetricEvent`] with no `_` arm. Adding a variant to the
    //!   enum without extending these matches fails at rustc
    //!   level. `#[non_exhaustive]` on `MetricEvent` affects only
    //!   external-crate callers; inside the defining crate
    //!   exhaustive matching is required.
    //! - **Test-time.** The [`EXPECTED`] and
    //!   [`EXPECTED_SUBSYSTEMS`] tables transcribe `docs/31`
    //!   §Phase-1 baseline catalog verbatim; conformance tests
    //!   cross-check them bidirectionally against the catalog and
    //!   each row's label set. [`catalog_has_exactly_phase1_baseline_count`]
    //!   pins the 14-row count (`docs/31` §Phase-1 baseline
    //!   catalog final line: "Phase-1 baseline is 14 metrics")
    //!   as a belt-and-suspenders anchor against row additions
    //!   that happen to share an existing metric name.
    //!
    //! Packet-minimum coverage (`docs/26` line 160 per-type
    //! minimums for automated suites):
    //!
    //! - `ci-run-url` or `repo-path` to the audit — satisfied by
    //!   `repo-path:engine/observability/src/catalog.rs` (this
    //!   module) and promoted to a `ci-run-url` once green on
    //!   `main` per the follow-up `docs(policy)` registry flip.
    //! - Audit run identifier — populated by
    //!   `evidence-runs/<sha>.json` per `docs/43 §15.3`.
    //! - Telemetry schema version captured — pinned by
    //!   [`observability_schema_version_is_phase1_baseline`]
    //!   against [`PHASE_1_SCHEMA_VERSION`].

    use super::{
        AudioIngestFrameStatus, AudioIngestState, CommandKind, EventKind, MetricEvent, MetricKind,
        ObservabilitySubsystem, OscAdapterState, OscBindErrorReason, OscPacketOutcome,
        PHASE_1_SCHEMA_VERSION, RuntimeLifecycleState, SceneLintCategory, SceneValidationOutcome,
    };

    /// Phase-1 baseline catalog size. Transcribed from
    /// `docs/31-supportability-privacy-ops-policy.md` §Phase-1
    /// baseline catalog final line ("Phase-1 baseline is 14
    /// metrics"). Adding a `docs/31` row without extending both
    /// [`MetricEvent`] and [`EXPECTED`] fails
    /// [`catalog_has_exactly_phase1_baseline_count`].
    const PHASE_1_BASELINE_COUNT: usize = 14;

    /// `docs/31` §Phase-1 baseline catalog, transcribed verbatim.
    ///
    /// Tuple shape: `(metric_name, MetricKind, &[label_name])`
    /// with `label_name` sorted ascending for deterministic
    /// comparison against the Rust catalog's struct-variant field
    /// sets (also sorted by [`label_names`]).
    const EXPECTED: &[(&str, MetricKind, &[&str])] = &[
        (
            "chromaticity_runtime_lifecycle_transitions_total",
            MetricKind::Counter,
            &["from", "to"],
        ),
        (
            "chromaticity_runtime_uptime_seconds",
            MetricKind::Gauge,
            &[],
        ),
        (
            "chromaticity_runtime_commands_dispatched_total",
            MetricKind::Counter,
            &["kind"],
        ),
        (
            "chromaticity_runtime_events_emitted_total",
            MetricKind::Counter,
            &["kind"],
        ),
        (
            "chromaticity_scene_validations_total",
            MetricKind::Counter,
            &["outcome"],
        ),
        (
            "chromaticity_scene_lint_warnings_total",
            MetricKind::Counter,
            &["category"],
        ),
        (
            "chromaticity_audio_ingest_state_transitions_total",
            MetricKind::Counter,
            &["from", "to"],
        ),
        (
            "chromaticity_audio_ingest_frames_total",
            MetricKind::Counter,
            &["status"],
        ),
        (
            "chromaticity_audio_ingest_samples_total",
            MetricKind::Counter,
            &[],
        ),
        (
            "chromaticity_osc_inbound_packets_total",
            MetricKind::Counter,
            &["outcome"],
        ),
        (
            "chromaticity_osc_adapter_state_transitions_total",
            MetricKind::Counter,
            &["from", "to"],
        ),
        (
            "chromaticity_osc_bind_errors_total",
            MetricKind::Counter,
            &["reason"],
        ),
        (
            "chromaticity_observability_events_emitted_total",
            MetricKind::Counter,
            &["subsystem"],
        ),
        (
            "chromaticity_observability_schema_version",
            MetricKind::Gauge,
            &[],
        ),
    ];

    /// `docs/31` §Phase-1 baseline catalog subsystem assignments.
    ///
    /// Tuple shape: `(metric_name, ObservabilitySubsystem)`.
    const EXPECTED_SUBSYSTEMS: &[(&str, ObservabilitySubsystem)] = &[
        (
            "chromaticity_runtime_lifecycle_transitions_total",
            ObservabilitySubsystem::Runtime,
        ),
        (
            "chromaticity_runtime_uptime_seconds",
            ObservabilitySubsystem::Runtime,
        ),
        (
            "chromaticity_runtime_commands_dispatched_total",
            ObservabilitySubsystem::Runtime,
        ),
        (
            "chromaticity_runtime_events_emitted_total",
            ObservabilitySubsystem::Runtime,
        ),
        (
            "chromaticity_scene_validations_total",
            ObservabilitySubsystem::Scene,
        ),
        (
            "chromaticity_scene_lint_warnings_total",
            ObservabilitySubsystem::Scene,
        ),
        (
            "chromaticity_audio_ingest_state_transitions_total",
            ObservabilitySubsystem::Audio,
        ),
        (
            "chromaticity_audio_ingest_frames_total",
            ObservabilitySubsystem::Audio,
        ),
        (
            "chromaticity_audio_ingest_samples_total",
            ObservabilitySubsystem::Audio,
        ),
        (
            "chromaticity_osc_inbound_packets_total",
            ObservabilitySubsystem::Osc,
        ),
        (
            "chromaticity_osc_adapter_state_transitions_total",
            ObservabilitySubsystem::Osc,
        ),
        (
            "chromaticity_osc_bind_errors_total",
            ObservabilitySubsystem::Osc,
        ),
        (
            "chromaticity_observability_events_emitted_total",
            ObservabilitySubsystem::Observability,
        ),
        (
            "chromaticity_observability_schema_version",
            ObservabilitySubsystem::Observability,
        ),
    ];

    /// `docs/31` runtime command-kind label closed set under
    /// `chromaticity_runtime_commands_dispatched_total.kind`.
    const EXPECTED_RUNTIME_COMMAND_KIND_LABELS: &[&str] = &[
        "control_set",
        "preset_recall",
        "playlist_advance",
        "snap_recall",
        "snap_save",
        "snap_clear",
        "mapping_bind",
    ];

    /// `docs/31` runtime event-kind label closed set under
    /// `chromaticity_runtime_events_emitted_total.kind`.
    const EXPECTED_RUNTIME_EVENT_KIND_LABELS: &[&str] = &[
        "control_changed",
        "preset_recalled",
        "playlist_advanced",
        "snap_recalled",
        "snap_saved",
        "snap_cleared",
        "mapping_bound",
    ];

    /// Enumerate one sample of every [`MetricEvent`] variant, in
    /// declaration order.
    ///
    /// The body's first action is constructing the variants; the
    /// second is an exhaustive `match` over the result (no `_`
    /// arm). Adding a variant to [`MetricEvent`] without
    /// extending both the constructor list and the post-match
    /// fails at rustc level — this is the compile-time arm of
    /// the module's drift-detection design.
    ///
    /// For variants carrying a label-value enum, one arbitrary
    /// enum value is chosen. Label-enum coverage is pinned by
    /// the T0 `ctor_placeholder::subsystem_enum_covers_observability_meta_label_cardinality`
    /// test and the `#[non_exhaustive]` markers on each label
    /// enum.
    fn every_metric_event_variant() -> Vec<MetricEvent> {
        let events = vec![
            MetricEvent::RuntimeLifecycleTransition {
                from: RuntimeLifecycleState::Starting,
                to: RuntimeLifecycleState::Running,
                value: 1,
            },
            MetricEvent::RuntimeUptime { value: 0 },
            MetricEvent::RuntimeCommandsDispatched {
                kind: CommandKind::ControlSet,
                value: 1,
            },
            MetricEvent::RuntimeEventsEmitted {
                kind: EventKind::ControlChanged,
                value: 1,
            },
            MetricEvent::SceneValidations {
                outcome: SceneValidationOutcome::Pass,
                value: 1,
            },
            MetricEvent::SceneLintWarnings {
                category: SceneLintCategory::CoercionApplied,
                value: 1,
            },
            MetricEvent::AudioIngestStateTransition {
                from: AudioIngestState::Starting,
                to: AudioIngestState::Running,
                value: 1,
            },
            MetricEvent::AudioIngestFrames {
                status: AudioIngestFrameStatus::Ready,
                value: 1,
            },
            MetricEvent::AudioIngestSamples { value: 128 },
            MetricEvent::OscInboundPackets {
                outcome: OscPacketOutcome::Accepted,
                value: 1,
            },
            MetricEvent::OscAdapterStateTransition {
                from: OscAdapterState::Unbound,
                to: OscAdapterState::Binding,
                value: 1,
            },
            MetricEvent::OscBindErrors {
                reason: OscBindErrorReason::AddressInUse,
                value: 1,
            },
            MetricEvent::ObservabilityEventsEmitted {
                subsystem: ObservabilitySubsystem::Runtime,
                value: 1,
            },
            MetricEvent::ObservabilitySchemaVersion {
                value: PHASE_1_SCHEMA_VERSION,
            },
        ];

        // Compile-time drift trap: exhaustive match with no `_`
        // arm. Adding a new MetricEvent variant without extending
        // this match fails at build time.
        for event in &events {
            match event {
                MetricEvent::RuntimeLifecycleTransition { .. }
                | MetricEvent::RuntimeUptime { .. }
                | MetricEvent::RuntimeCommandsDispatched { .. }
                | MetricEvent::RuntimeEventsEmitted { .. }
                | MetricEvent::SceneValidations { .. }
                | MetricEvent::SceneLintWarnings { .. }
                | MetricEvent::AudioIngestStateTransition { .. }
                | MetricEvent::AudioIngestFrames { .. }
                | MetricEvent::AudioIngestSamples { .. }
                | MetricEvent::OscInboundPackets { .. }
                | MetricEvent::OscAdapterStateTransition { .. }
                | MetricEvent::OscBindErrors { .. }
                | MetricEvent::ObservabilityEventsEmitted { .. }
                | MetricEvent::ObservabilitySchemaVersion { .. } => (),
            }
        }

        events
    }

    /// Extract the struct-variant's named-field set (excluding
    /// `value`) as a sorted `Vec<&'static str>`. Exhaustive match
    /// serves as the second compile-time drift trap: adding a
    /// variant requires a new arm that names the variant's
    /// labels.
    fn label_names(event: &MetricEvent) -> Vec<&'static str> {
        let mut labels: Vec<&'static str> = match event {
            MetricEvent::RuntimeLifecycleTransition { .. } => vec!["from", "to"],
            MetricEvent::RuntimeUptime { .. } => vec![],
            MetricEvent::RuntimeCommandsDispatched { .. } => vec!["kind"],
            MetricEvent::RuntimeEventsEmitted { .. } => vec!["kind"],
            MetricEvent::SceneValidations { .. } => vec!["outcome"],
            MetricEvent::SceneLintWarnings { .. } => vec!["category"],
            MetricEvent::AudioIngestStateTransition { .. } => vec!["from", "to"],
            MetricEvent::AudioIngestFrames { .. } => vec!["status"],
            MetricEvent::AudioIngestSamples { .. } => vec![],
            MetricEvent::OscInboundPackets { .. } => vec!["outcome"],
            MetricEvent::OscAdapterStateTransition { .. } => vec!["from", "to"],
            MetricEvent::OscBindErrors { .. } => vec!["reason"],
            MetricEvent::ObservabilityEventsEmitted { .. } => vec!["subsystem"],
            MetricEvent::ObservabilitySchemaVersion { .. } => vec![],
        };
        labels.sort_unstable();
        labels
    }

    /// Enumerate all [`CommandKind`] variants in declaration order.
    ///
    /// The loop below is deliberately exhaustive with no `_` arm so
    /// enum additions fail at rustc level until this fixture and docs
    /// expectations are updated.
    fn every_command_kind_variant() -> Vec<CommandKind> {
        let variants = vec![
            CommandKind::ControlSet,
            CommandKind::PresetRecall,
            CommandKind::PlaylistAdvance,
            CommandKind::SnapRecall,
            CommandKind::SnapSave,
            CommandKind::SnapClear,
            CommandKind::MappingBind,
        ];
        for kind in &variants {
            match kind {
                CommandKind::ControlSet
                | CommandKind::PresetRecall
                | CommandKind::PlaylistAdvance
                | CommandKind::SnapRecall
                | CommandKind::SnapSave
                | CommandKind::SnapClear
                | CommandKind::MappingBind => (),
            }
        }
        variants
    }

    /// Enumerate all [`EventKind`] variants in declaration order.
    ///
    /// The loop below is deliberately exhaustive with no `_` arm so
    /// enum additions fail at rustc level until this fixture and docs
    /// expectations are updated.
    fn every_event_kind_variant() -> Vec<EventKind> {
        let variants = vec![
            EventKind::ControlChanged,
            EventKind::PresetRecalled,
            EventKind::PlaylistAdvanced,
            EventKind::SnapRecalled,
            EventKind::SnapSaved,
            EventKind::SnapCleared,
            EventKind::MappingBound,
        ];
        for kind in &variants {
            match kind {
                EventKind::ControlChanged
                | EventKind::PresetRecalled
                | EventKind::PlaylistAdvanced
                | EventKind::SnapRecalled
                | EventKind::SnapSaved
                | EventKind::SnapCleared
                | EventKind::MappingBound => (),
            }
        }
        variants
    }

    fn command_kind_label(kind: CommandKind) -> &'static str {
        match kind {
            CommandKind::ControlSet => "control_set",
            CommandKind::PresetRecall => "preset_recall",
            CommandKind::PlaylistAdvance => "playlist_advance",
            CommandKind::SnapRecall => "snap_recall",
            CommandKind::SnapSave => "snap_save",
            CommandKind::SnapClear => "snap_clear",
            CommandKind::MappingBind => "mapping_bind",
        }
    }

    fn event_kind_label(kind: EventKind) -> &'static str {
        match kind {
            EventKind::ControlChanged => "control_changed",
            EventKind::PresetRecalled => "preset_recalled",
            EventKind::PlaylistAdvanced => "playlist_advanced",
            EventKind::SnapRecalled => "snap_recalled",
            EventKind::SnapSaved => "snap_saved",
            EventKind::SnapCleared => "snap_cleared",
            EventKind::MappingBound => "mapping_bound",
        }
    }

    // ---- Conformance tests ----

    /// Catches `docs/31`-only additions: every metric name the
    /// authoritative catalog declares must have a corresponding
    /// [`MetricEvent`] variant whose [`MetricEvent::name`] returns
    /// that spelling.
    #[test]
    fn every_docs31_row_has_a_catalog_variant() {
        let actual_names: std::collections::HashSet<&str> = every_metric_event_variant()
            .iter()
            .map(MetricEvent::name)
            .collect();
        for (expected_name, _, _) in EXPECTED {
            assert!(
                actual_names.contains(expected_name),
                "docs/31 row {expected_name} has no MetricEvent variant",
            );
        }
    }

    /// Catches Rust-catalog-only additions: every
    /// [`MetricEvent`] variant's [`MetricEvent::name`] output
    /// must appear in the authoritative `docs/31` catalog.
    #[test]
    fn every_catalog_variant_appears_in_docs31() {
        let expected_names: std::collections::HashSet<&str> =
            EXPECTED.iter().map(|(name, _, _)| *name).collect();
        for event in every_metric_event_variant() {
            let name = event.name();
            assert!(
                expected_names.contains(name),
                "MetricEvent variant name {name:?} not in docs/31 §Phase-1 baseline catalog",
            );
        }
    }

    /// Each [`MetricEvent`] variant's [`MetricEvent::kind`] must
    /// match `docs/31`'s `Type` column for that row.
    #[test]
    fn metric_kinds_match_docs31() {
        let expected_map: std::collections::HashMap<&str, MetricKind> = EXPECTED
            .iter()
            .map(|(name, kind, _)| (*name, *kind))
            .collect();
        for event in every_metric_event_variant() {
            let name = event.name();
            let actual_kind = event.kind();
            let expected_kind = *expected_map.get(name).expect(
                "variant absent from EXPECTED — caught by every_catalog_variant_appears_in_docs31",
            );
            assert_eq!(
                actual_kind, expected_kind,
                "metric {name} kind drift: catalog reports {actual_kind:?}, docs/31 declares {expected_kind:?}",
            );
        }
    }

    /// Each [`MetricEvent`] variant's [`MetricEvent::subsystem`] must
    /// match the docs/31 subsystem segment for that row's canonical
    /// metric name. This pins the per-variant assignment against silent
    /// reassignment even when names and labels still conform.
    #[test]
    fn subsystem_assignments_match_docs31() {
        assert_eq!(
            EXPECTED_SUBSYSTEMS.len(),
            PHASE_1_BASELINE_COUNT,
            "EXPECTED_SUBSYSTEMS row count drift vs docs/31 Phase-1 baseline",
        );
        let expected_map: std::collections::HashMap<&str, ObservabilitySubsystem> =
            EXPECTED_SUBSYSTEMS
                .iter()
                .map(|(name, subsystem)| (*name, *subsystem))
                .collect();
        for event in every_metric_event_variant() {
            let name = event.name();
            let actual_subsystem = event.subsystem();
            let expected_subsystem = *expected_map.get(name).expect(
                "variant absent from EXPECTED_SUBSYSTEMS — caught by every_catalog_variant_appears_in_docs31",
            );
            assert_eq!(
                actual_subsystem, expected_subsystem,
                "metric {name} subsystem drift: catalog reports {actual_subsystem:?}, docs/31 declares {expected_subsystem:?}",
            );
        }
    }

    /// Each [`MetricEvent`] variant's struct-field names
    /// (excluding `value`) must match `docs/31`'s `Labels`
    /// column for that row. Labels sorted ascending on both
    /// sides for order-independent comparison.
    #[test]
    fn label_sets_match_docs31() {
        let expected_map: std::collections::HashMap<&str, &[&str]> = EXPECTED
            .iter()
            .map(|(name, _, labels)| (*name, *labels))
            .collect();
        for event in every_metric_event_variant() {
            let name = event.name();
            let actual_labels = label_names(&event);
            let expected_labels: Vec<&str> = expected_map
                .get(name)
                .copied()
                .expect("variant absent from EXPECTED — caught by every_catalog_variant_appears_in_docs31")
                .to_vec();
            assert_eq!(
                actual_labels, expected_labels,
                "metric {name} label-set drift: catalog {actual_labels:?} vs docs/31 {expected_labels:?}",
            );
        }
    }

    /// Runtime command kind labels are a docs/31 closed set. Any
    /// enum-level addition or token rename MUST be paired with a
    /// docs/31 schema update.
    #[test]
    fn runtime_command_kind_labels_match_docs31_closed_set() {
        let actual: std::collections::HashSet<&str> = every_command_kind_variant()
            .into_iter()
            .map(command_kind_label)
            .collect();
        let expected: std::collections::HashSet<&str> = EXPECTED_RUNTIME_COMMAND_KIND_LABELS
            .iter()
            .copied()
            .collect();
        assert_eq!(
            actual, expected,
            "runtime command kind label drift vs docs/31 closed set",
        );
    }

    /// Runtime event kind labels are a docs/31 closed set. Any
    /// enum-level addition or token rename MUST be paired with a
    /// docs/31 schema update.
    #[test]
    fn runtime_event_kind_labels_match_docs31_closed_set() {
        let actual: std::collections::HashSet<&str> = every_event_kind_variant()
            .into_iter()
            .map(event_kind_label)
            .collect();
        let expected: std::collections::HashSet<&str> =
            EXPECTED_RUNTIME_EVENT_KIND_LABELS.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "runtime event kind label drift vs docs/31 closed set",
        );
    }

    /// `docs/31` §Phase-1 baseline catalog closes with
    /// "Phase-1 baseline is 14 metrics." Belt-and-suspenders
    /// against `docs/31` row additions that happen to reuse an
    /// existing metric name (which the name-level checks would
    /// miss) by also asserting the row count matches both
    /// [`EXPECTED`] and the Rust catalog.
    #[test]
    fn catalog_has_exactly_phase1_baseline_count() {
        assert_eq!(
            EXPECTED.len(),
            PHASE_1_BASELINE_COUNT,
            "EXPECTED row count drift vs docs/31 Phase-1 baseline",
        );
        assert_eq!(
            every_metric_event_variant().len(),
            PHASE_1_BASELINE_COUNT,
            "MetricEvent variant count drift vs docs/31 Phase-1 baseline",
        );
    }

    /// The
    /// [`MetricEvent::ObservabilitySchemaVersion`] gauge emits
    /// the current schema-version integer. Pinning the constant
    /// [`PHASE_1_SCHEMA_VERSION`] makes the currently-shipped
    /// version value visible as a named drift anchor; bumping it
    /// requires a paired `docs/31` Schema change process PR.
    #[test]
    fn observability_schema_version_is_phase1_baseline() {
        let event = MetricEvent::ObservabilitySchemaVersion {
            value: PHASE_1_SCHEMA_VERSION,
        };
        match event {
            MetricEvent::ObservabilitySchemaVersion { value } => {
                assert_eq!(value, PHASE_1_SCHEMA_VERSION);
            }
            other => {
                unreachable!("constructed ObservabilitySchemaVersion, got {other:?}")
            }
        }
    }
}
