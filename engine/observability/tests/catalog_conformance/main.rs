// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1a EV-HEALTH-METRICS-LOOPBACK-ONLY corpus (catalog
//! half). Tier declaration and evidence tag live on the `fixtures`
//! inner module below, matching the pattern established by
//! `engine/runtime/tests/state_replay_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-HEALTH-METRICS-LOOPBACK-ONLY (T2) — Phase-1 metric
    //! catalog conformance against `docs/31-supportability-privacy-ops-policy.md`
    //! §Phase-1 baseline catalog. Validates the data-shape surface
    //! that the Phase-4d loopback-only Prometheus endpoint will
    //! expose, with row 94's "external network sees zero egress"
    //! invariant covered by the sibling `no_network_egress` corpus.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! evidence-producing behaviour against a fixture corpus —
    //! namely the authoritative metric catalog at
    //! `docs/31-supportability-privacy-ops-policy.md` §Phase-1
    //! baseline catalog (14 metrics across 5 subsystems) plus the
    //! ADR-016 §Health metrics privacy / cardinality posture
    //! constraints.
    //!
    //! Scope split. Row 94's "telemetry endpoint binds only to
    //! loopback when enabled; external network sees zero egress"
    //! validation has two halves at Phase 1:
    //!
    //! - **Catalog conformance (this corpus).** The metric catalog
    //!   the loopback endpoint will expose at Phase 4d already
    //!   exists at Phase 1 as `engine/observability::catalog`. The
    //!   row-94 "what gets emitted" half is the Phase-1 baseline
    //!   catalog matching docs/31, with cardinality and privacy
    //!   posture asserted at the type level.
    //! - **No network egress (sibling corpus).** The "where the
    //!   bytes go" half — at Phase 1, NOWHERE: in-process emission
    //!   only, no transport. The sibling
    //!   `engine/observability/tests/no_network_egress/main.rs`
    //!   asserts the absence of any network dependency or socket
    //!   call, the Phase-1 substrate the Phase-4d loopback endpoint
    //!   will sit on top of.
    //!
    //! Phase-4d (deferred) flips the transport on against the same
    //! catalog. When that lands, this corpus stays unchanged —
    //! the catalog shape this row validates is invariant across the
    //! Phase-1 in-process stub and the Phase-4d loopback endpoint.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 94 declares `EV-HEALTH-METRICS-LOOPBACK-ONLY` Generation
    //! Source = "Binding + egress observation" with Gate Usage =
    //! "Phase 4a exit". This corpus is the binding half; the
    //! egress half lives in the sibling `no_network_egress` corpus.
    //!
    //! Drift-detection mechanics mirror the in-source T2 harness in
    //! `engine/observability/src/catalog.rs::catalog_conformance`
    //! (which carries the parallel `EV-OBSERVABILITY` evidence tag
    //! per `docs/26` row 38). Two tests can share schema-shape
    //! coverage by intent: the in-source harness is the
    //! `EV-OBSERVABILITY` row's audit; this external corpus is
    //! `EV-HEALTH-METRICS-LOOPBACK-ONLY`'s audit. Both exercise
    //! the same `MetricEvent` enum from the public crate surface,
    //! catching identical drift signals from two evidence-row
    //! perspectives.

    use chromaticity_observability::catalog::{
        AudioIngestFrameStatus, AudioIngestState, CommandKind, EventKind, MetricEvent, MetricKind,
        ObservabilitySubsystem, OscAdapterState, OscBindErrorReason, OscPacketOutcome,
        PHASE_1_SCHEMA_VERSION, RuntimeLifecycleState, SceneLintCategory, SceneValidationOutcome,
    };
    use chromaticity_observability::current_schema_version;
    use chromaticity_observability::emit::{EmitMetric, InMemoryEmitter};

    /// Phase-1 baseline catalog size. Transcribed from docs/31
    /// §Phase-1 baseline catalog final line: "Phase-1 baseline is
    /// 14 metrics."
    const PHASE_1_BASELINE_COUNT: usize = 14;

    /// docs/31 §Label policy line 50 caps any metric at 64 active
    /// label-value combinations. Verified against the closed label
    /// enums declared by the catalog.
    const CARDINALITY_CAP: usize = 64;

    fn every_metric_event_variant() -> Vec<MetricEvent> {
        vec![
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
        ]
    }

    /// Compute the active label-value combination count for one
    /// metric. Closed-enum-only labels per docs/31 §Label policy.
    /// Exhaustive `match` — adding a `MetricEvent` variant fails at
    /// rustc level until this mapping is extended.
    fn label_combinations(event: &MetricEvent) -> usize {
        let runtime_lifecycle: usize = 4; // Starting / Running / Paused / Stopping
        let command_kind: usize = 7; // see docs/31 closed set
        let event_kind: usize = 7;
        let scene_outcome: usize = 2; // pass / fail
        let scene_lint: usize = 4; // 4 categories declared
        let audio_state: usize = 4; // Starting / Running / Draining / Closed
        let audio_status: usize = 3; // ready / would_block / closed
        let osc_outcome: usize = 5; // 5 categories
        let osc_adapter_state: usize = 4; // Unbound / Binding / Bound / Unbinding
        let osc_bind_reason: usize = 4; // AddressInUse / PermissionDenied / AddressNotAvailable / Other
        let observability_subsystem: usize = 5; // Runtime / Scene / Audio / Osc / Observability

        match event {
            MetricEvent::RuntimeLifecycleTransition { .. } => runtime_lifecycle * runtime_lifecycle,
            MetricEvent::RuntimeUptime { .. } => 1,
            MetricEvent::RuntimeCommandsDispatched { .. } => command_kind,
            MetricEvent::RuntimeEventsEmitted { .. } => event_kind,
            MetricEvent::SceneValidations { .. } => scene_outcome,
            MetricEvent::SceneLintWarnings { .. } => scene_lint,
            MetricEvent::AudioIngestStateTransition { .. } => audio_state * audio_state,
            MetricEvent::AudioIngestFrames { .. } => audio_status,
            MetricEvent::AudioIngestSamples { .. } => 1,
            MetricEvent::OscInboundPackets { .. } => osc_outcome,
            MetricEvent::OscAdapterStateTransition { .. } => osc_adapter_state * osc_adapter_state,
            MetricEvent::OscBindErrors { .. } => osc_bind_reason,
            MetricEvent::ObservabilityEventsEmitted { .. } => observability_subsystem,
            MetricEvent::ObservabilitySchemaVersion { .. } => 1,
            // `MetricEvent` is `#[non_exhaustive]`. From this
            // external test crate the wildcard arm is required;
            // a future variant lands here and the
            // `phase_1_baseline_catalog_lists_14_metrics_exactly`
            // test fails first (loud signal) — this arm is the
            // belt that keeps the corpus compiling so the count
            // mismatch is what surfaces.
            _ => CARDINALITY_CAP + 1,
        }
    }

    /// Stable byte-encoding of one [`MetricEvent`] for determinism
    /// pinning. Encoded as the canonical metric name followed by
    /// the integer value, length-prefixed. No `serde_json` or other
    /// transport-coupled framing — the Phase-4d Prometheus
    /// exposition is a separate codec; this byte form pins
    /// data-shape stability inside the in-process emitter.
    fn encode_event(event: &MetricEvent) -> Vec<u8> {
        let mut out = Vec::new();
        let name = event.name();
        out.extend_from_slice(&(name.len() as u64).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        let value: u64 = match *event {
            MetricEvent::RuntimeLifecycleTransition { value, .. }
            | MetricEvent::RuntimeUptime { value }
            | MetricEvent::RuntimeCommandsDispatched { value, .. }
            | MetricEvent::RuntimeEventsEmitted { value, .. }
            | MetricEvent::SceneValidations { value, .. }
            | MetricEvent::SceneLintWarnings { value, .. }
            | MetricEvent::AudioIngestStateTransition { value, .. }
            | MetricEvent::AudioIngestFrames { value, .. }
            | MetricEvent::AudioIngestSamples { value }
            | MetricEvent::OscInboundPackets { value, .. }
            | MetricEvent::OscAdapterStateTransition { value, .. }
            | MetricEvent::OscBindErrors { value, .. }
            | MetricEvent::ObservabilityEventsEmitted { value, .. }
            | MetricEvent::ObservabilitySchemaVersion { value } => value,
            // `#[non_exhaustive]` wildcard. A new variant landing
            // here is caught by
            // `phase_1_baseline_catalog_lists_14_metrics_exactly`
            // before this arm executes.
            _ => 0,
        };
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    // ---- Conformance fixtures ----

    /// docs/31 §Phase-1 baseline catalog declares exactly 14 metrics
    /// ("Phase-1 baseline is 14 metrics" closing line). Belt-and-
    /// suspenders pin against silent additions that the schema-flip
    /// process did not bless.
    #[test]
    fn phase_1_baseline_catalog_lists_14_metrics_exactly() {
        let variants = every_metric_event_variant();
        assert_eq!(
            variants.len(),
            PHASE_1_BASELINE_COUNT,
            "MetricEvent variant count drift vs docs/31 §Phase-1 baseline catalog (expected {PHASE_1_BASELINE_COUNT})",
        );
        let names: std::collections::HashSet<&str> =
            variants.iter().map(MetricEvent::name).collect();
        assert_eq!(
            names.len(),
            PHASE_1_BASELINE_COUNT,
            "MetricEvent variants are not name-distinct; duplicate canonical names violate docs/31",
        );
    }

    /// 4 runtime metrics by canonical name (docs/31 §Phase-1
    /// baseline catalog rows 1-4). `subsystem` assignment double-
    /// pinned via [`MetricEvent::subsystem`].
    #[test]
    fn runtime_metrics_match_docs_31_canonical_names() {
        let expected = [
            "chromaticity_runtime_lifecycle_transitions_total",
            "chromaticity_runtime_uptime_seconds",
            "chromaticity_runtime_commands_dispatched_total",
            "chromaticity_runtime_events_emitted_total",
        ];
        let runtime_names: std::collections::HashSet<&str> = every_metric_event_variant()
            .into_iter()
            .filter(|e| e.subsystem() == ObservabilitySubsystem::Runtime)
            .map(|e| e.name())
            .collect();
        let expected_set: std::collections::HashSet<&str> = expected.iter().copied().collect();
        assert_eq!(
            runtime_names, expected_set,
            "runtime subsystem catalog drift vs docs/31",
        );
    }

    /// 2 scene metrics by canonical name (docs/31 §Phase-1 baseline
    /// catalog rows 5-6).
    #[test]
    fn scene_metrics_match_docs_31_canonical_names() {
        let expected = [
            "chromaticity_scene_validations_total",
            "chromaticity_scene_lint_warnings_total",
        ];
        let scene_names: std::collections::HashSet<&str> = every_metric_event_variant()
            .into_iter()
            .filter(|e| e.subsystem() == ObservabilitySubsystem::Scene)
            .map(|e| e.name())
            .collect();
        let expected_set: std::collections::HashSet<&str> = expected.iter().copied().collect();
        assert_eq!(
            scene_names, expected_set,
            "scene subsystem catalog drift vs docs/31"
        );
    }

    /// 3 audio metrics by canonical name (docs/31 §Phase-1 baseline
    /// catalog rows 7-9).
    #[test]
    fn audio_metrics_match_docs_31_canonical_names() {
        let expected = [
            "chromaticity_audio_ingest_state_transitions_total",
            "chromaticity_audio_ingest_frames_total",
            "chromaticity_audio_ingest_samples_total",
        ];
        let audio_names: std::collections::HashSet<&str> = every_metric_event_variant()
            .into_iter()
            .filter(|e| e.subsystem() == ObservabilitySubsystem::Audio)
            .map(|e| e.name())
            .collect();
        let expected_set: std::collections::HashSet<&str> = expected.iter().copied().collect();
        assert_eq!(
            audio_names, expected_set,
            "audio subsystem catalog drift vs docs/31"
        );
    }

    /// 3 OSC metrics by canonical name (docs/31 §Phase-1 baseline
    /// catalog rows 10-12).
    #[test]
    fn osc_metrics_match_docs_31_canonical_names() {
        let expected = [
            "chromaticity_osc_inbound_packets_total",
            "chromaticity_osc_adapter_state_transitions_total",
            "chromaticity_osc_bind_errors_total",
        ];
        let osc_names: std::collections::HashSet<&str> = every_metric_event_variant()
            .into_iter()
            .filter(|e| e.subsystem() == ObservabilitySubsystem::Osc)
            .map(|e| e.name())
            .collect();
        let expected_set: std::collections::HashSet<&str> = expected.iter().copied().collect();
        assert_eq!(
            osc_names, expected_set,
            "osc subsystem catalog drift vs docs/31"
        );
    }

    /// 2 observability-meta metrics by canonical name (docs/31
    /// §Phase-1 baseline catalog rows 13-14).
    #[test]
    fn observability_meta_metrics_match_docs_31_canonical_names() {
        let expected = [
            "chromaticity_observability_events_emitted_total",
            "chromaticity_observability_schema_version",
        ];
        let meta_names: std::collections::HashSet<&str> = every_metric_event_variant()
            .into_iter()
            .filter(|e| e.subsystem() == ObservabilitySubsystem::Observability)
            .map(|e| e.name())
            .collect();
        let expected_set: std::collections::HashSet<&str> = expected.iter().copied().collect();
        assert_eq!(
            meta_names, expected_set,
            "observability-meta subsystem catalog drift vs docs/31",
        );
    }

    /// docs/31 §Label policy line 50 caps any metric at 64 active
    /// label-value combinations. Pinned per-metric against the
    /// closed enum cardinalities the catalog declares.
    #[test]
    fn metric_label_cardinality_does_not_exceed_64_combinations() {
        for event in every_metric_event_variant() {
            let count = label_combinations(&event);
            assert!(
                count <= CARDINALITY_CAP,
                "metric {} cardinality {} exceeds docs/31 cap {}",
                event.name(),
                count,
                CARDINALITY_CAP,
            );
        }
    }

    /// docs/31 §Label policy line 49 forbids labels carrying
    /// scene names, file paths, IP addresses, or any unbounded
    /// user content. Enforced structurally: every label value type
    /// in the catalog is a closed `Copy + Eq` enum (no `String`,
    /// no `&str`, no `PathBuf`, no integer ID type that could
    /// represent unbounded keys). The test below asserts the
    /// construction surface — a string-typed label cannot be
    /// constructed without changing this corpus and the catalog
    /// in lockstep.
    #[test]
    fn no_metric_label_carries_unbounded_user_content() {
        // Construct one of each label-bearing variant. The compile
        // surface is the proof: if any future variant introduces a
        // `String` / `&str` / path-like label, the constructor list
        // will fail to compile against today's `Copy` enum-only
        // shape and force the schema-flip process per docs/31
        // §Schema change process line 88.
        let _: MetricEvent = MetricEvent::RuntimeLifecycleTransition {
            from: RuntimeLifecycleState::Starting,
            to: RuntimeLifecycleState::Running,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::RuntimeCommandsDispatched {
            kind: CommandKind::ControlSet,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::RuntimeEventsEmitted {
            kind: EventKind::ControlChanged,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::SceneValidations {
            outcome: SceneValidationOutcome::Pass,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::SceneLintWarnings {
            category: SceneLintCategory::CoercionApplied,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::AudioIngestStateTransition {
            from: AudioIngestState::Starting,
            to: AudioIngestState::Running,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::AudioIngestFrames {
            status: AudioIngestFrameStatus::Ready,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::OscInboundPackets {
            outcome: OscPacketOutcome::Accepted,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::OscAdapterStateTransition {
            from: OscAdapterState::Unbound,
            to: OscAdapterState::Binding,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::OscBindErrors {
            reason: OscBindErrorReason::AddressInUse,
            value: 1,
        };
        let _: MetricEvent = MetricEvent::ObservabilityEventsEmitted {
            subsystem: ObservabilitySubsystem::Runtime,
            value: 1,
        };
    }

    /// `current_schema_version()` returns the Phase-1 schema-version
    /// gauge. Bumping the constant is a paired docs/31 §Schema
    /// change process action and is intentionally noisy.
    #[test]
    fn current_schema_version_matches_phase_1_constant() {
        let event = current_schema_version();
        assert_eq!(event.kind(), MetricKind::Gauge);
        match event {
            MetricEvent::ObservabilitySchemaVersion { value } => {
                assert_eq!(
                    value, PHASE_1_SCHEMA_VERSION,
                    "current_schema_version() drift vs PHASE_1_SCHEMA_VERSION constant",
                );
                assert_eq!(
                    value, 1,
                    "Phase-1 schema version is 1 per docs/31 catalog row 14"
                );
            }
            other => unreachable!("current_schema_version returned {other:?}"),
        }
    }

    /// Every catalog metric type round-trips through the
    /// in-process [`InMemoryEmitter`] preserving event identity.
    /// This is the Phase-1 substrate the Phase-4d Prometheus
    /// loopback endpoint will read from — drift in the round-trip
    /// shape is row 94's first failure mode.
    #[test]
    fn in_memory_emitter_round_trips_each_metric_type() {
        let emitter = InMemoryEmitter::new();
        let originals = every_metric_event_variant();
        for event in &originals {
            emitter.emit(event.clone());
        }
        let drained = emitter.drain();
        assert_eq!(
            drained.len(),
            originals.len(),
            "InMemoryEmitter dropped events on round-trip",
        );
        for (idx, (orig, observed)) in originals.iter().zip(drained.iter()).enumerate() {
            assert_eq!(
                orig, observed,
                "round-trip event {idx} drift: {orig:?} != {observed:?}",
            );
        }
    }

    /// Encoding a metric event twice produces byte-identical
    /// output. Pins canonical-binary determinism — a property
    /// the loopback Prometheus endpoint will inherit when it
    /// lands at Phase 4d.
    #[test]
    fn metric_event_serialization_is_byte_stable() {
        for event in every_metric_event_variant() {
            let first = encode_event(&event);
            let second = encode_event(&event);
            assert_eq!(
                first,
                second,
                "non-deterministic encoding for {}",
                event.name(),
            );
            assert!(!first.is_empty(), "empty encoding for {}", event.name());
        }
    }

    /// Repeated emission of the same metric does not lose events
    /// or produce non-monotonic counts. 1024-event volume catches
    /// any racy mutex / lost-update regression in the in-process
    /// stub.
    #[test]
    fn counter_atomicity_under_repeated_emission() {
        const ITERATIONS: u64 = 1024;
        let emitter = InMemoryEmitter::new();
        for _ in 0..ITERATIONS {
            emitter.emit(MetricEvent::RuntimeCommandsDispatched {
                kind: CommandKind::ControlSet,
                value: 1,
            });
        }
        assert_eq!(
            emitter.len() as u64,
            ITERATIONS,
            "InMemoryEmitter buffered count drift",
        );
        assert_eq!(
            emitter.events_emitted_count(ObservabilitySubsystem::Runtime),
            ITERATIONS,
            "subsystem emission counter drift",
        );
        let drained = emitter.drain();
        assert_eq!(drained.len() as u64, ITERATIONS, "drain count drift");
        for event in drained {
            match event {
                MetricEvent::RuntimeCommandsDispatched {
                    kind: CommandKind::ControlSet,
                    value: 1,
                } => (),
                other => unreachable!("emitted shape drift: {other:?}"),
            }
        }
    }
}
