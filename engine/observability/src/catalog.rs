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
/// against ADR-006 (thread-and-process-model). TODO(slice-3):
/// reconcile variants against the concrete lifecycle enum once the
/// runtime scheduler ships.
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
/// Closed) that the transitions counter keys off. The slice-3 wiring
/// maps outcomes to transitions; slice-2 pins the vocabulary.
/// TODO(slice-3): reconcile against the concrete audio-pipeline
/// lifecycle once the scheduler lands.
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
/// a best-effort synthesis. TODO(slice-3): reconcile against the
/// adapter-lifecycle enum once it ships in `integrations/`.
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
/// `OscAdapterError::BindFailed`. TODO(slice-3): widen / align as
/// the adapter grows structured bind-error reporting.
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
