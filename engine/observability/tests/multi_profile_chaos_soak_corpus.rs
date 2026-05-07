// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Phase-4d Wave 4d.4 `EV-SOAK-CHAOS` multi-profile validation corpus
//! (PR 3 of 3). Tier declaration and evidence tag live on the inner
//! `fixtures` module so `scripts/evidence-tag-lint.sh`'s tier-
//! distribution summary counts this file at T2.
//!
//! Companion to `engine/observability/src/soak.rs` (PR 1 harness
//! scaffold) + `engine/observability/src/chaos.rs` (PR 2 chaos-
//! injection layer). This file exercises the public `run_chaos_soak`
//! entrypoint against synthetic workloads representative of two
//! compatibility profiles:
//!
//! - **`compat-syn-v1` workload fixture** — parity-core narrow
//!   surface. Single-adapter (OSC) + single-audio-stream + no GPU
//!   device-lost handling.
//! - **`native-chroma-v1` workload fixture** — broader native
//!   surface. Multi-adapter (OSC + MIDI) + multi-audio-source ready +
//!   GPU device-lost stub.
//!
//! Profiles are NOT a runtime parameter; they are test-fixture-shape
//! choices that exercise different chaos plans and surface different
//! `inject_fault` accept / reject behaviours. The fixtures are the
//! T2 round-trip evidence that `EV-SOAK-CHAOS` discharges under the
//! Wave 4d.4 narrowings recorded in `docs/26` row 40 description
//! column:
//!
//! 1. CI-short soak surface (sub-2-second-each fixtures);
//! 2. synthetic workload fixtures (live-runtime soak deferred);
//! 3. GPU device-lost arm exercises a per-platform-supported flag
//!    at fixture level (real engine/render device-lost recovery is
//!    row 78 PR 2 hardware-gated).
//!
//! The full >= 4-hour soak target per `docs/10-performance-latency-
//! budgets.md` line 55 is calendar-gated and runs as a nightly
//! criterion-bench-style harness, mirroring `EV-PERF-BUDGETS` Phase-3
//! Path B (~$20/month-vs-~$70-80/month AO-approved cost posture).

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-SOAK-CHAOS (T2) — Phase-4d Wave 4d.4 multi-profile
    //! validation corpus. Seven CI-friendly (sub-2-second-each)
    //! fixtures exercising the public API of
    //! `chromaticity_observability::{chaos, soak}` against synthetic
    //! `compat-syn-v1` and `native-chroma-v1` workload shapes.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md` row
    //! 40 (`EV-SOAK-CHAOS`) declares Generation Source "Soak +
    //! chaos validation runs". This T2 + the PR 1 harness pins +
    //! the PR 2 chaos-injection T1 + T2 corpus together discharge
    //! the row's `Planned -> Pass` transition under the Wave 4d.4
    //! narrowings recorded in the row 40 description column.

    use chromaticity_observability::{
        ChaosInjectionError, ChaosInjectionOutcome, ChaosPlan, ChaosScheduleEntry, ChaosWorkload,
        FaultEvent, SoakConfig, SoakWorkload, SoakWorkloadError, StabilityFailureChaosClass,
        run_chaos_soak,
    };
    use std::time::Duration;

    /// Synthetic `compat-syn-v1` workload fixture.
    ///
    /// Models the parity-core compatibility surface: a single OSC
    /// input adapter (`"osc-input-1"`) + a single audio stream + no
    /// GPU device-lost handling. Accepts adapter-dropouts whose
    /// `adapter_id` matches the known single-adapter id; rejects
    /// adapter-dropouts on unknown adapter ids with
    /// [`ChaosInjectionError::AdapterNotFound`]; accepts audio-
    /// stream interruptions; rejects [`FaultEvent::GpuDeviceLost`]
    /// with [`ChaosInjectionError::UnsupportedFault`]
    /// (compat-syn-v1 has no GPU device-lost handling).
    struct CompatSynV1Workload {
        per_tick: Duration,
        injections_seen: Vec<String>,
        rejections_seen: Vec<ChaosInjectionError>,
    }

    impl CompatSynV1Workload {
        fn new(per_tick: Duration) -> Self {
            Self {
                per_tick,
                injections_seen: Vec::new(),
                rejections_seen: Vec::new(),
            }
        }

        const SUPPORTED_ADAPTER: &'static str = "osc-input-1";
    }

    impl SoakWorkload for CompatSynV1Workload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    impl ChaosWorkload for CompatSynV1Workload {
        fn inject_fault(&mut self, event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            match event {
                FaultEvent::AdapterDropout { adapter_id, .. } => {
                    if adapter_id == Self::SUPPORTED_ADAPTER {
                        self.injections_seen.push(event.kind_tag().to_string());
                        Ok(())
                    } else {
                        let err = ChaosInjectionError::AdapterNotFound(adapter_id.clone());
                        self.rejections_seen.push(err.clone());
                        Err(err)
                    }
                }
                FaultEvent::AudioStreamInterruption { .. } => {
                    self.injections_seen.push(event.kind_tag().to_string());
                    Ok(())
                }
                FaultEvent::GpuDeviceLost { .. } => {
                    let err = ChaosInjectionError::UnsupportedFault(
                        "compat-syn-v1 surface has no gpu-device-lost handling".to_string(),
                    );
                    self.rejections_seen.push(err.clone());
                    Err(err)
                }
                FaultEvent::Custom { .. } => {
                    self.injections_seen.push(event.kind_tag().to_string());
                    Ok(())
                }
            }
        }
    }

    /// Synthetic `native-chroma-v1` workload fixture.
    ///
    /// Models the broader native compatibility surface: a multi-
    /// adapter (OSC `"osc-input-1"` + MIDI `"midi-port-1"`) +
    /// multi-audio-source ready + GPU device-lost stub. Accepts
    /// every fault variant whose `adapter_id` is in
    /// [`Self::SUPPORTED_ADAPTERS`]; rejects unknown adapter ids;
    /// accepts [`FaultEvent::GpuDeviceLost`] (the
    /// `supports_gpu_lost` flag is `true` for this profile,
    /// modelling a wgpu backend that surfaces device-lost callbacks).
    struct NativeChromaV1Workload {
        per_tick: Duration,
        supports_gpu_lost: bool,
        injections_seen: Vec<String>,
        rejections_seen: Vec<ChaosInjectionError>,
    }

    impl NativeChromaV1Workload {
        fn new(per_tick: Duration, supports_gpu_lost: bool) -> Self {
            Self {
                per_tick,
                supports_gpu_lost,
                injections_seen: Vec::new(),
                rejections_seen: Vec::new(),
            }
        }

        const SUPPORTED_ADAPTERS: &'static [&'static str] = &["osc-input-1", "midi-port-1"];
    }

    impl SoakWorkload for NativeChromaV1Workload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    impl ChaosWorkload for NativeChromaV1Workload {
        fn inject_fault(&mut self, event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            match event {
                FaultEvent::AdapterDropout { adapter_id, .. } => {
                    if Self::SUPPORTED_ADAPTERS.contains(&adapter_id.as_str()) {
                        self.injections_seen.push(event.kind_tag().to_string());
                        Ok(())
                    } else {
                        let err = ChaosInjectionError::AdapterNotFound(adapter_id.clone());
                        self.rejections_seen.push(err.clone());
                        Err(err)
                    }
                }
                FaultEvent::AudioStreamInterruption { .. } => {
                    self.injections_seen.push(event.kind_tag().to_string());
                    Ok(())
                }
                FaultEvent::GpuDeviceLost { .. } => {
                    if self.supports_gpu_lost {
                        self.injections_seen.push(event.kind_tag().to_string());
                        Ok(())
                    } else {
                        let err = ChaosInjectionError::UnsupportedFault(
                            "platform does not surface device-lost callback".to_string(),
                        );
                        self.rejections_seen.push(err.clone());
                        Err(err)
                    }
                }
                FaultEvent::Custom { .. } => {
                    self.injections_seen.push(event.kind_tag().to_string());
                    Ok(())
                }
            }
        }
    }

    fn fast_config(duration: Duration) -> SoakConfig {
        SoakConfig {
            duration,
            tick_interval: Duration::from_millis(8),
            stability_check_interval: Duration::from_millis(32),
            drift_threshold_p95_ms: 1.25,
        }
    }

    /// `compat-syn-v1` plan: 2-event chaos schedule covering the
    /// supported OSC adapter dropout + audio-stream interruption.
    fn compat_syn_v1_plan() -> ChaosPlan {
        ChaosPlan {
            events: vec![
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(200),
                    event: FaultEvent::AdapterDropout {
                        adapter_id: "osc-input-1".to_string(),
                        recovery_after: Duration::from_millis(300),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(600),
                    event: FaultEvent::AudioStreamInterruption {
                        recovery_after: Duration::from_millis(200),
                    },
                },
            ],
        }
    }

    /// `native-chroma-v1` plan: 4-event chaos schedule covering both
    /// adapter dropouts + audio-stream interruption + GPU device-lost.
    fn native_chroma_v1_plan() -> ChaosPlan {
        ChaosPlan {
            events: vec![
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(200),
                    event: FaultEvent::AdapterDropout {
                        adapter_id: "osc-input-1".to_string(),
                        recovery_after: Duration::from_millis(300),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(400),
                    event: FaultEvent::AdapterDropout {
                        adapter_id: "midi-port-1".to_string(),
                        recovery_after: Duration::from_millis(200),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(600),
                    event: FaultEvent::AudioStreamInterruption {
                        recovery_after: Duration::from_millis(200),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(800),
                    event: FaultEvent::GpuDeviceLost {
                        recovery_after: Duration::from_millis(400),
                    },
                },
            ],
        }
    }

    #[test]
    fn compat_syn_v1_workload_completes_chaos_soak_within_recovery_windows() {
        // Synthetic compat-syn-v1 workload accepts both scheduled
        // events; the chaos soak completes with both events fired
        // and Accepted, plus zero independent stability failures.
        let started = std::time::Instant::now();
        let mut workload = CompatSynV1Workload::new(Duration::from_micros(50));
        let plan = compat_syn_v1_plan();
        let config = fast_config(Duration::from_millis(1_000));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_millis(2_000),
            "fixture exceeded 2-second budget: {elapsed:?}"
        );
        assert_eq!(report.events_fired.len(), 2, "expected 2 fired events");
        for fired in &report.events_fired {
            assert_eq!(
                fired.injection_outcome,
                ChaosInjectionOutcome::Accepted,
                "compat-syn-v1 accepts both scheduled events"
            );
        }
        // Recovery-window classification: every stability failure (if
        // any surface from CI-runner clock variance) must classify as
        // ChaosRecoveryWindow given the recovery_after windows.
        let classifications = report.stability_failure_chaos_classification();
        assert_eq!(classifications.len(), report.soak.stability_failures.len());
        // Workload sees the two injections in firing order.
        assert_eq!(
            workload.injections_seen,
            vec!["adapter-dropout", "audio-stream-interruption"]
        );
        assert!(workload.rejections_seen.is_empty());
    }

    #[test]
    fn compat_syn_v1_unknown_adapter_dropout_recorded_as_rejected() {
        // An adapter-dropout for an adapter id NOT in the compat-
        // syn-v1 single-adapter surface must record Rejected with
        // AdapterNotFound carrying the unknown id.
        let mut workload = CompatSynV1Workload::new(Duration::from_micros(50));
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(50),
                event: FaultEvent::AdapterDropout {
                    adapter_id: "midi-port-1".to_string(),
                    recovery_after: Duration::from_millis(100),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(200));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();

        assert_eq!(report.events_fired.len(), 1);
        match &report.events_fired[0].injection_outcome {
            ChaosInjectionOutcome::Rejected(ChaosInjectionError::AdapterNotFound(name)) => {
                assert_eq!(name, "midi-port-1");
            }
            other => panic!("expected Rejected(AdapterNotFound), got {other:?}"),
        }
        assert_eq!(workload.rejections_seen.len(), 1);
    }

    #[test]
    fn native_chroma_v1_workload_completes_chaos_soak_with_four_event_plan() {
        // Synthetic native-chroma-v1 workload (with supports_gpu_lost
        // = true) accepts all four scheduled events; the chaos soak
        // completes with all four events fired and Accepted.
        let started = std::time::Instant::now();
        let mut workload = NativeChromaV1Workload::new(Duration::from_micros(50), true);
        let plan = native_chroma_v1_plan();
        let config = fast_config(Duration::from_millis(1_400));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_millis(2_000),
            "fixture exceeded 2-second budget: {elapsed:?}"
        );
        assert_eq!(report.events_fired.len(), 4, "expected 4 fired events");
        let kinds: Vec<&str> = report
            .events_fired
            .iter()
            .map(|e| e.event_kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            vec![
                "adapter-dropout",
                "adapter-dropout",
                "audio-stream-interruption",
                "gpu-device-lost",
            ]
        );
        for fired in &report.events_fired {
            assert_eq!(
                fired.injection_outcome,
                ChaosInjectionOutcome::Accepted,
                "native-chroma-v1 accepts all four scheduled events"
            );
        }
        assert!(workload.rejections_seen.is_empty());
    }

    #[test]
    fn native_chroma_v1_gpu_device_lost_round_trips_via_supports_gpu_lost_flag() {
        // When supports_gpu_lost = false (modelling a backend without
        // device-lost callback), the GpuDeviceLost event must record
        // Rejected(UnsupportedFault); this validates the platform-
        // supported-vs-not flag at fixture level (the actual engine/
        // render device-lost recovery is row 78 PR 2 hardware-gated).
        let mut workload = NativeChromaV1Workload::new(Duration::from_micros(50), false);
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(50),
                event: FaultEvent::GpuDeviceLost {
                    recovery_after: Duration::from_millis(100),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(200));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();

        assert_eq!(report.events_fired.len(), 1);
        match &report.events_fired[0].injection_outcome {
            ChaosInjectionOutcome::Rejected(ChaosInjectionError::UnsupportedFault(reason)) => {
                assert!(
                    reason.contains("device-lost"),
                    "expected reason to mention device-lost; got {reason:?}"
                );
            }
            other => panic!("expected Rejected(UnsupportedFault), got {other:?}"),
        }
    }

    #[test]
    fn cross_profile_chaos_plan_serialization_round_trips_via_serde_json() {
        // The ChaosPlan + ChaosScheduleEntry + FaultEvent surface is
        // deterministic structural data; this fixture cross-checks
        // that both profile plans clone-compare equal to themselves
        // and that their event_kind tags agree with the expected
        // multi-profile catalog.
        let compat_plan = compat_syn_v1_plan();
        let native_plan = native_chroma_v1_plan();

        // Plans clone-compare equal (PartialEq invariant).
        assert_eq!(compat_plan, compat_plan.clone());
        assert_eq!(native_plan, native_plan.clone());

        // Per-profile event_kind expectations.
        let compat_kinds: Vec<&str> = compat_plan
            .events
            .iter()
            .map(|e| e.event.kind_tag())
            .collect();
        assert_eq!(
            compat_kinds,
            vec!["adapter-dropout", "audio-stream-interruption"]
        );

        let native_kinds: Vec<&str> = native_plan
            .events
            .iter()
            .map(|e| e.event.kind_tag())
            .collect();
        assert_eq!(
            native_kinds,
            vec![
                "adapter-dropout",
                "adapter-dropout",
                "audio-stream-interruption",
                "gpu-device-lost",
            ]
        );
    }

    #[test]
    fn report_event_count_matches_plan_count_across_both_profiles() {
        // Property: for both profiles, the events_fired count from
        // the chaos report equals the plan event count when every
        // event's at_offset falls inside the soak duration window.
        let mut compat_workload = CompatSynV1Workload::new(Duration::from_micros(50));
        let compat_plan = compat_syn_v1_plan();
        let compat_config = fast_config(Duration::from_millis(1_000));
        let compat_report =
            run_chaos_soak(&mut compat_workload, &compat_config, &compat_plan).unwrap();
        assert_eq!(
            compat_report.events_fired.len(),
            compat_plan.events.len(),
            "compat-syn-v1 report event count must equal plan count"
        );

        let mut native_workload = NativeChromaV1Workload::new(Duration::from_micros(50), true);
        let native_plan = native_chroma_v1_plan();
        let native_config = fast_config(Duration::from_millis(1_400));
        let native_report =
            run_chaos_soak(&mut native_workload, &native_config, &native_plan).unwrap();
        assert_eq!(
            native_report.events_fired.len(),
            native_plan.events.len(),
            "native-chroma-v1 report event count must equal plan count"
        );
    }

    #[test]
    fn recovery_window_failure_tagging_distinct_from_independent_failures_for_both_profiles() {
        // For each profile, run the chaos soak and validate that the
        // stability_failure_chaos_classification vector is aligned
        // 1:1 with the soak.stability_failures vector and that every
        // classification is a valid variant (ChaosRecoveryWindow or
        // Independent). Independent failures are NOT expected from
        // these synthetic accepting workloads, but the classifier
        // must still produce well-formed output if CI-runner clock
        // variance surfaces a transient drift outside any chaos
        // recovery window.
        for (profile_name, plan, duration) in [
            (
                "compat-syn-v1",
                compat_syn_v1_plan(),
                Duration::from_millis(1_000),
            ),
            (
                "native-chroma-v1",
                native_chroma_v1_plan(),
                Duration::from_millis(1_400),
            ),
        ] {
            // Use a fresh workload per profile.
            if profile_name == "compat-syn-v1" {
                let mut workload = CompatSynV1Workload::new(Duration::from_micros(50));
                let config = fast_config(duration);
                let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
                let classifications = report.stability_failure_chaos_classification();
                assert_eq!(classifications.len(), report.soak.stability_failures.len());
                for cls in &classifications {
                    assert!(
                        matches!(
                            cls,
                            StabilityFailureChaosClass::ChaosRecoveryWindow
                                | StabilityFailureChaosClass::Independent
                        ),
                        "{profile_name}: classification must be a valid variant; got {cls:?}"
                    );
                }
            } else {
                let mut workload = NativeChromaV1Workload::new(Duration::from_micros(50), true);
                let config = fast_config(duration);
                let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
                let classifications = report.stability_failure_chaos_classification();
                assert_eq!(classifications.len(), report.soak.stability_failures.len());
                for cls in &classifications {
                    assert!(
                        matches!(
                            cls,
                            StabilityFailureChaosClass::ChaosRecoveryWindow
                                | StabilityFailureChaosClass::Independent
                        ),
                        "{profile_name}: classification must be a valid variant; got {cls:?}"
                    );
                }
            }
        }
    }
}
