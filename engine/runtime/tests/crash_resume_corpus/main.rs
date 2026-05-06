// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.3 PR 1 EV-CRASH-RESUME corpus. Tier declaration
//! and evidence tag live on the `fixtures` inner module below,
//! matching the pattern established by
//! `engine/runtime/tests/state_replay_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-CRASH-RESUME (T2) — Phase-4d Wave 4d.3 PR 1
    //! crash-resume class-a + class-b state-restoration corpus.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) builds a [`State`] by dispatching commands
    //! through a fresh [`InMemoryBus`] and committing into the
    //! state, (b) saves the state via a [`CrashResumeAdapter`]
    //! configured with the [`CanonicalBinarySnapshotter`], (c)
    //! drops the original adapter + state (the in-process analogue
    //! of process termination), (d) constructs a fresh adapter
    //! pointing at the same save path plus a fresh [`State::default`]
    //! (the in-process analogue of host re-launch), (e) recovers
    //! the snapshot, (f) projects the recovered values onto the
    //! fresh state via the per-class [`restore_class_a`] /
    //! [`restore_class_b`] helpers (or
    //! [`restore_all_supported_classes`]), and (g) asserts the per-
    //! class observable values match the pre-crash state.
    //!
    //! Fixtures cover the EV-CRASH-RESUME contract per
    //! `docs/26-evidence-artifact-registry.md` row 80, narrowed in
    //! PR 1 to the in-process simulated-crash + class-a + class-b
    //! restoration surface. Real cross-process `SIGKILL` /
    //! `SIGSEGV` / panic-in-render-thread / OOM injection plus the
    //! time-to-resume + visible-artifact-window p50/p95
    //! measurement budgets land in subsequent PRs in this wave
    //! cluster. Fixture set:
    //!
    //! 1. `simulated_crash_recovery_round_trip_preserves_class_a_state`
    //!    — class-a (active scene + preset channel values) survives
    //!    a save / drop / recover / restore cycle.
    //! 2. `simulated_crash_recovery_round_trip_preserves_class_b_state`
    //!    — class-b (Snap slot selection + scene-specific preset
    //!    history) survives the same cycle.
    //! 3. `simulated_crash_recovery_with_three_wallclock_injection_points`
    //!    — drives a 10-tick scripted "show" that mutates state at
    //!    tick 3, tick 6, and tick 9; snapshots after each
    //!    mutation; asserts every snapshot recovers correctly.
    //!    Approximates the row-80 "three wallclock points within a
    //!    10-minute scripted show" at miniature scale.
    //! 4. `recovery_from_corrupt_snapshot_returns_serialization_error_not_panic`
    //!    — synthesises a corrupt blob and asserts the adapter
    //!    surfaces a typed `CrashResumeError::SerializationError`
    //!    rather than panicking.

    use chromaticity_runtime::crash_resume::{
        CanonicalBinarySnapshotter, CrashResumeAdapter, CrashResumeError, SerializationKind,
        StateSnapshotBlob, StateSnapshotter, restore_all_supported_classes, restore_class_a,
        restore_class_b,
    };
    use chromaticity_runtime::state::State;
    use chromaticity_runtime::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, PresetId, SnapSlotId,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Per-fixture unique save path under the OS temp dir. Avoids a
    /// `tempfile` dev-dep — the path is composed from process id,
    /// nanos since epoch, and a per-process counter so concurrent
    /// `cargo test` runs do not collide.
    fn unique_save_path(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-crash-resume-corpus-{label}-{pid}-{nanos}-{n}.bin"
        ));
        path
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
    }

    /// Drive a class-a-rich state mutation through the canonical
    /// command-bus path: two control values + one preset recall.
    fn mutate_class_a(bus: &InMemoryBus, state: &mut State) {
        bus.dispatch(Command::ControlSet {
            id: ControlId(11),
            value: ControlValue(0.25),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::ControlSet {
            id: ControlId(13),
            value: ControlValue(0.875),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::PresetRecall { id: PresetId(5) })
            .expect("preset command must dispatch");
        bus.commit_all_into(state);
    }

    /// Drive a class-b-rich state mutation: save two Snap slots
    /// then recall one.
    fn mutate_class_b(bus: &InMemoryBus, state: &mut State) {
        let slot_a = SnapSlotId::new(2).expect("valid snap slot");
        let slot_b = SnapSlotId::new(7).expect("valid snap slot");
        bus.dispatch(Command::SnapSave { slot: slot_a })
            .expect("snap save command must dispatch");
        bus.dispatch(Command::SnapSave { slot: slot_b })
            .expect("snap save command must dispatch");
        bus.dispatch(Command::SnapRecall { slot: slot_a })
            .expect("snap recall command must dispatch");
        bus.commit_all_into(state);
    }

    #[test]
    fn simulated_crash_recovery_round_trip_preserves_class_a_state() {
        let path = unique_save_path("class-a-round-trip");
        // Pre-crash: build state + snapshot.
        let pre_crash_active_preset;
        let pre_crash_control_11;
        let pre_crash_control_13;
        {
            let bus = InMemoryBus::in_memory();
            let mut pre_crash = State::default();
            mutate_class_a(&bus, &mut pre_crash);
            pre_crash_active_preset = pre_crash.active_preset();
            pre_crash_control_11 = pre_crash.control_value(ControlId(11));
            pre_crash_control_13 = pre_crash.control_value(ControlId(13));
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
            // `pre_crash` and `adapter` drop here — the in-process
            // analogue of a process exit.
        }
        // Post-crash: fresh adapter + fresh state, recover, restore class-a.
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");
        let mut post_crash = State::default();
        restore_class_a(&recovered, &mut post_crash);
        assert_eq!(post_crash.active_preset(), pre_crash_active_preset);
        assert_eq!(
            post_crash.control_value(ControlId(11)),
            pre_crash_control_11
        );
        assert_eq!(
            post_crash.control_value(ControlId(13)),
            pre_crash_control_13
        );
        cleanup(&path);
    }

    #[test]
    fn simulated_crash_recovery_round_trip_preserves_class_b_state() {
        let path = unique_save_path("class-b-round-trip");
        let pre_crash_snap_active;
        let pre_crash_slot_2_saved;
        let pre_crash_slot_7_saved;
        {
            let bus = InMemoryBus::in_memory();
            let mut pre_crash = State::default();
            mutate_class_b(&bus, &mut pre_crash);
            pre_crash_snap_active = pre_crash.active_snap_slot();
            pre_crash_slot_2_saved =
                pre_crash.snap_slot_saved(SnapSlotId::new(2).expect("valid snap slot"));
            pre_crash_slot_7_saved =
                pre_crash.snap_slot_saved(SnapSlotId::new(7).expect("valid snap slot"));
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
        }
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");
        let mut post_crash = State::default();
        restore_class_b(&recovered, &mut post_crash);
        assert_eq!(post_crash.active_snap_slot(), pre_crash_snap_active);
        assert_eq!(
            post_crash.snap_slot_saved(SnapSlotId::new(2).expect("valid snap slot")),
            pre_crash_slot_2_saved,
        );
        assert_eq!(
            post_crash.snap_slot_saved(SnapSlotId::new(7).expect("valid snap slot")),
            pre_crash_slot_7_saved,
        );
        cleanup(&path);
    }

    #[test]
    fn simulated_crash_recovery_with_three_wallclock_injection_points() {
        // Drive a 10-tick scripted show. Mutate state at tick 3,
        // tick 6, tick 9; snapshot after each mutation; assert each
        // snapshot independently recovers to the matching pre-
        // mutation observable values.
        //
        // Approximates the row-80 "three wallclock points within a
        // 10-minute scripted show" at miniature scale. PR 1 ships
        // the structural correctness invariant; the production-
        // wallclock-budget version lands alongside real cross-
        // process injection in a future PR.
        let injection_ticks = [3u32, 6u32, 9u32];
        let mut snapshots: Vec<(PathBuf, Option<PresetId>, Option<SnapSlotId>)> = Vec::new();

        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let mut next_preset: u32 = 1;
        let mut next_snap: u8 = 1;
        let mut next_control_id: u32 = 100;
        for tick in 0..10u32 {
            if injection_ticks.contains(&tick) {
                // Class-a mutation:
                bus.dispatch(Command::ControlSet {
                    id: ControlId(next_control_id),
                    value: ControlValue(0.5),
                })
                .expect("control command must dispatch");
                bus.dispatch(Command::PresetRecall {
                    id: PresetId(next_preset),
                })
                .expect("preset command must dispatch");
                next_control_id += 1;
                next_preset += 1;
                // Class-b mutation:
                let slot = SnapSlotId::new(next_snap).expect("valid snap slot");
                bus.dispatch(Command::SnapSave { slot })
                    .expect("snap save command must dispatch");
                bus.dispatch(Command::SnapRecall { slot })
                    .expect("snap recall command must dispatch");
                next_snap += 1;
                bus.commit_all_into(&mut state);
                // Snapshot at this wallclock point:
                let path = unique_save_path(&format!("three-points-tick-{tick}"));
                let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
                adapter.save(&state).expect("save must succeed");
                snapshots.push((path, state.active_preset(), state.active_snap_slot()));
            } else {
                // No-op tick: drain whatever's pending (nothing).
                bus.commit_all_into(&mut state);
            }
        }
        assert_eq!(
            snapshots.len(),
            3,
            "must capture three injection-point snapshots"
        );

        // Recover each snapshot independently and assert the per-
        // injection-point observable values survive.
        for (path, expected_preset, expected_snap) in &snapshots {
            let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            let recovered = adapter
                .try_recover()
                .expect("recover must succeed")
                .expect("recover must return Some(state)");
            let mut post_crash = State::default();
            restore_all_supported_classes(&recovered, &mut post_crash);
            assert_eq!(
                post_crash.active_preset(),
                *expected_preset,
                "class-a preset must survive recovery from {path:?}",
            );
            assert_eq!(
                post_crash.active_snap_slot(),
                *expected_snap,
                "class-b snap slot must survive recovery from {path:?}",
            );
            cleanup(path);
        }
    }

    #[test]
    fn recovery_from_corrupt_snapshot_returns_serialization_error_not_panic() {
        // Synthesise a corrupt blob: write a byte string that does
        // NOT begin with the V2 envelope's BE-u32(0x00000002) prefix
        // but is long enough that the canonical-body decoder would
        // attempt to parse it. Assert the decoder returns a typed
        // `SerializationError` rather than panicking.
        let snapshotter = CanonicalBinarySnapshotter;
        let corrupt = StateSnapshotBlob(vec![
            0xFF, 0xFF, 0xFF, 0xFF, // unknown version tag
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // would-be frame counter
        ]);
        let outcome = snapshotter.restore(&corrupt);
        match outcome {
            Err(CrashResumeError::SerializationError(SerializationKind::CanonicalBinary(_))) => {
                // Pass: typed error, not a panic.
            }
            Err(other) => panic!("expected SerializationError, got {other:?}"),
            Ok(_) => panic!("corrupt buffer must not decode to Ok(_)"),
        }
    }
}
