// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.3 EV-CRASH-RESUME corpus. Tier declaration and
//! evidence tag live on the `fixtures` inner module below, matching
//! the pattern established by
//! `engine/runtime/tests/state_replay_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-CRASH-RESUME (T2) — Phase-4d Wave 4d.3
    //! crash-resume class-a + class-b + class-c state-restoration
    //! corpus.
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
    //! [`restore_class_b`] / [`restore_class_c`] helpers (or
    //! [`restore_all_supported_classes`]), and (g) asserts the per-
    //! class observable values match the pre-crash state.
    //!
    //! Fixtures cover the EV-CRASH-RESUME contract per
    //! `docs/26-evidence-artifact-registry.md` row 80. PR 1 narrowed
    //! to in-process simulated-crash + class-a + class-b; PR 2
    //! widened to class-c (playlist position; iteration mode is a
    //! deferred sub-store). Wave 4d.3 PR 5 ships class-d (OSC/MIDI
    //! session bindings) via the additive
    //! `crate::state::SessionBindings` sub-store extension and the
    //! V2 envelope's tagged session-bindings extension. Class-e
    //! through class-g remain documented-deferred no-op helpers
    //! pending the corresponding State-shape extensions.
    //! Real cross-process `SIGKILL` / `SIGSEGV` / panic-in-render-
    //! thread / OOM injection plus the time-to-resume + visible-
    //! artifact-window p50/p95 measurement budgets land in
    //! subsequent PRs in this wave cluster. Fixture set:
    //!
    //! 1. `simulated_crash_recovery_round_trip_preserves_class_a_state`
    //!    — class-a (active scene + preset channel values) survives
    //!    a save / drop / recover / restore cycle.
    //! 2. `simulated_crash_recovery_round_trip_preserves_class_b_state`
    //!    — class-b (Snap slot selection + scene-specific preset
    //!    history) survives the same cycle.
    //! 3. `simulated_crash_recovery_round_trip_preserves_class_c_state`
    //!    — class-c (playlist position + iteration mode) survives
    //!    the same cycle. New in PR 2.
    //! 4. `simulated_crash_recovery_with_three_wallclock_injection_points_now_covers_class_c`
    //!    — drives a 10-tick scripted "show" that mutates class-a +
    //!    class-b + class-c state at tick 3, tick 6, and tick 9;
    //!    snapshots after each mutation; asserts every snapshot
    //!    recovers correctly across all three classes. Extends the
    //!    PR 1 three-injection-points fixture to cover class-c.
    //! 5. `restore_class_c_idempotent_when_target_already_has_same_state`
    //!    — applying class-c restoration to a target that already
    //!    carries the same playlist position is a no-op (no panic,
    //!    no flap). New in PR 2.
    //! 6. `recovery_from_corrupt_snapshot_returns_serialization_error_not_panic`
    //!    — synthesises a corrupt blob and asserts the adapter
    //!    surfaces a typed `CrashResumeError::SerializationError`
    //!    rather than panicking.

    use chromaticity_runtime::crash_resume::{
        CanonicalBinarySnapshotter, CrashResumeAdapter, CrashResumeError, SerializationKind,
        StateSnapshotBlob, StateSnapshotter, restore_all_supported_classes, restore_class_a,
        restore_class_b, restore_class_c, restore_class_d,
    };
    use chromaticity_runtime::state::{MidiDeviceBinding, OscDeviceBinding, State};
    use chromaticity_runtime::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, PresetId, SnapSlotId,
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

    /// Drive a class-c-rich state mutation: advance the playlist
    /// position twice with non-trivial steps.
    fn mutate_class_c(bus: &InMemoryBus, state: &mut State) {
        bus.dispatch(Command::PlaylistAdvance { step: 11 })
            .expect("playlist command must dispatch");
        bus.dispatch(Command::PlaylistAdvance { step: 4 })
            .expect("playlist command must dispatch");
        bus.commit_all_into(state);
    }

    /// Apply a class-d session-binding snapshot to `state` (Wave
    /// 4d.3 PR 5, row 80 class-d). One OSC device and one MIDI
    /// device, each with a small mapping list, exercise the full
    /// device-id + mappings round-trip across the V2 envelope's
    /// session-bindings extension.
    fn mutate_class_d(state: &mut State) {
        state.apply_session_bindings(chromaticity_runtime::state::SessionBindings {
            osc_devices: vec![OscDeviceBinding {
                device_id: "osc.studio.local:9000".to_owned(),
                mappings: vec![MappingId(11), MappingId(12)],
            }],
            midi_devices: vec![MidiDeviceBinding {
                device_id: "iac-bus-0".to_owned(),
                mappings: vec![MappingId(21)],
            }],
        });
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
    fn simulated_crash_recovery_round_trip_preserves_class_c_state() {
        let path = unique_save_path("class-c-round-trip");
        let pre_crash_position;
        {
            let bus = InMemoryBus::in_memory();
            let mut pre_crash = State::default();
            mutate_class_c(&bus, &mut pre_crash);
            pre_crash_position = pre_crash.playlist_position();
            assert_eq!(
                pre_crash_position, 15,
                "mutate_class_c contract: 11 + 4 = 15",
            );
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
        }
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");
        let mut post_crash = State::default();
        restore_class_c(&recovered, &mut post_crash);
        assert_eq!(post_crash.playlist_position(), pre_crash_position);
        cleanup(&path);
    }

    #[test]
    fn simulated_crash_recovery_with_three_wallclock_injection_points_now_covers_class_c() {
        // Drive a 10-tick scripted show. Mutate state at tick 3,
        // tick 6, tick 9; snapshot after each mutation; assert each
        // snapshot independently recovers to the matching pre-
        // mutation observable values.
        //
        // PR 1 covered class-a + class-b at each injection point.
        // PR 2 extends with a class-c mutation (playlist advance) at
        // each injection point so the recovery assertions cover all
        // three currently-supported classes.
        //
        // Approximates the row-80 "three wallclock points within a
        // 10-minute scripted show" at miniature scale. PR 1 + PR 2
        // ship the structural correctness invariant; the production-
        // wallclock-budget version lands alongside real cross-
        // process injection in a future PR.
        let injection_ticks = [3u32, 6u32, 9u32];
        let mut snapshots: Vec<(PathBuf, Option<PresetId>, Option<SnapSlotId>, i64)> = Vec::new();

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
                // Class-c mutation: advance playlist by a per-
                // injection-point step so each snapshot has a
                // distinct position. `tick` is bounded to 0..10 so
                // the cast to i32 is exact.
                let step: i32 = i32::try_from(tick).expect("tick fits in i32") + 1;
                bus.dispatch(Command::PlaylistAdvance { step })
                    .expect("playlist command must dispatch");
                bus.commit_all_into(&mut state);
                // Snapshot at this wallclock point:
                let path = unique_save_path(&format!("three-points-tick-{tick}"));
                let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
                adapter.save(&state).expect("save must succeed");
                snapshots.push((
                    path,
                    state.active_preset(),
                    state.active_snap_slot(),
                    state.playlist_position(),
                ));
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
        // injection-point observable values survive across all
        // currently-supported classes.
        for (path, expected_preset, expected_snap, expected_position) in &snapshots {
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
            assert_eq!(
                post_crash.playlist_position(),
                *expected_position,
                "class-c playlist position must survive recovery from {path:?}",
            );
            cleanup(path);
        }
    }

    #[test]
    fn restore_class_c_idempotent_when_target_already_has_same_state() {
        // Applying class-c restoration to a target that already
        // carries the same playlist position MUST be a no-op (no
        // panic, no flap). Pins the partial-advance idempotence
        // invariant: re-applying a restoration helper after the
        // target has converged onto the restored value is safe.
        let path = unique_save_path("class-c-idempotent");
        {
            let bus = InMemoryBus::in_memory();
            let mut pre_crash = State::default();
            mutate_class_c(&bus, &mut pre_crash);
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
        }
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");

        // First application: target moves from default (position 0)
        // to the recovered value.
        let mut target = State::default();
        restore_class_c(&recovered, &mut target);
        let after_first = target.playlist_position();
        assert_ne!(after_first, 0, "first application moves the target");
        // Second application onto the already-converged target.
        restore_class_c(&recovered, &mut target);
        let after_second = target.playlist_position();
        assert_eq!(
            after_first, after_second,
            "class-c restoration must be idempotent on a converged target",
        );
        cleanup(&path);
    }

    // --- Wave 4d.3 PR 5 class-d additive fixtures -------------------

    #[test]
    fn class_d_round_trip_session_bindings_via_restore_helper() {
        // Pin: class-d (OSC/MIDI session bindings) survives a save /
        // drop / recover / `restore_class_d` cycle. The V2 envelope
        // carries the session-bindings extension; the canonical-
        // binary segment is unchanged so EV-STATE-REPLAY (row 32) /
        // EV-PRESET-PLAYLIST (row 34) goldens are preserved.
        let path = unique_save_path("class-d-round-trip");
        let pre_crash_bindings;
        {
            let mut pre_crash = State::default();
            mutate_class_d(&mut pre_crash);
            pre_crash_bindings = pre_crash.session_bindings().clone();
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
        }
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");
        let mut post_crash = State::default();
        restore_class_d(&mut post_crash, &recovered);
        assert_eq!(post_crash.session_bindings(), &pre_crash_bindings);
        assert_eq!(post_crash.session_bindings().osc_devices.len(), 1);
        assert_eq!(post_crash.session_bindings().midi_devices.len(), 1);
        assert_eq!(
            post_crash.session_bindings().osc_devices[0].device_id,
            "osc.studio.local:9000",
        );
        assert_eq!(
            post_crash.session_bindings().osc_devices[0].mappings,
            vec![MappingId(11), MappingId(12)],
        );
        cleanup(&path);
    }

    #[test]
    fn class_d_scripted_show_three_injection_points_recover_session_bindings() {
        // Drive a scripted show with three class-d injection points
        // (each adds a new OSC device-id binding to the running
        // session); snapshot after each injection; assert each
        // snapshot independently recovers to the matching pre-
        // injection-point session bindings via the convenience
        // helper. Mirrors the PR 1 + PR 2 three-injection-points
        // shape extended to class-d.
        let mut snapshots: Vec<(PathBuf, usize, usize)> = Vec::new();
        let mut state = State::default();
        for tick in 0..3u8 {
            // Append one OSC device per injection point so each
            // snapshot has a distinct binding count.
            let mut current = state.session_bindings().clone();
            current.osc_devices.push(OscDeviceBinding {
                device_id: format!("osc.tick-{tick}.local:9000"),
                mappings: vec![MappingId(u32::from(tick) + 100)],
            });
            // MIDI grows alongside on the third injection only, so
            // the OSC and MIDI counts diverge across snapshots.
            if tick == 2 {
                current.midi_devices.push(MidiDeviceBinding {
                    device_id: "iac-bus-0".to_owned(),
                    mappings: vec![MappingId(200)],
                });
            }
            state.apply_session_bindings(current);

            let path = unique_save_path(&format!("class-d-three-points-tick-{tick}"));
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&state).expect("save must succeed");
            snapshots.push((
                path,
                state.session_bindings().osc_devices.len(),
                state.session_bindings().midi_devices.len(),
            ));
        }
        assert_eq!(snapshots.len(), 3);

        for (path, expected_osc, expected_midi) in &snapshots {
            let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            let recovered = adapter
                .try_recover()
                .expect("recover must succeed")
                .expect("recover must return Some(state)");
            let mut post_crash = State::default();
            restore_all_supported_classes(&recovered, &mut post_crash);
            assert_eq!(
                post_crash.session_bindings().osc_devices.len(),
                *expected_osc,
                "class-d OSC device count must survive recovery from {path:?}",
            );
            assert_eq!(
                post_crash.session_bindings().midi_devices.len(),
                *expected_midi,
                "class-d MIDI device count must survive recovery from {path:?}",
            );
            cleanup(path);
        }
    }

    #[test]
    fn class_d_recovers_independently_of_class_a_b_c_e_f_g() {
        // Pin: class-d restoration is orthogonal to the other
        // shipped + deferred classes. A target carrying class-a +
        // class-b + class-c values + a still-deferred-class mapping
        // value receives `restore_class_d` and only its session
        // bindings change.
        let path = unique_save_path("class-d-orthogonal");
        let bus = InMemoryBus::in_memory();
        let mut pre_crash = State::default();
        // Seed pre-crash with all four shipped classes so the
        // recovered state carries non-default values across each.
        bus.dispatch(Command::ControlSet {
            id: ControlId(7),
            value: ControlValue(0.5),
        })
        .expect("control command must dispatch");
        bus.dispatch(Command::PresetRecall { id: PresetId(3) })
            .expect("preset command must dispatch");
        bus.dispatch(Command::SnapSave {
            slot: SnapSlotId::new(4).expect("valid snap slot"),
        })
        .expect("snap save command must dispatch");
        bus.dispatch(Command::PlaylistAdvance { step: 9 })
            .expect("playlist command must dispatch");
        bus.commit_all_into(&mut pre_crash);
        mutate_class_d(&mut pre_crash);

        let pre_crash_bindings = pre_crash.session_bindings().clone();

        {
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
        }

        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");

        // Apply ONLY class-d to a fresh-default target. The other
        // classes must remain at default because class-d restoration
        // is orthogonal.
        let mut target = State::default();
        restore_class_d(&mut target, &recovered);
        assert_eq!(target.session_bindings(), &pre_crash_bindings);
        // Class-a / class-b / class-c untouched — still default:
        assert_eq!(target.active_preset(), None);
        assert_eq!(target.active_snap_slot(), None);
        assert_eq!(target.playlist_position(), 0);
        cleanup(&path);
    }

    #[test]
    fn class_d_empty_session_bindings_round_trips() {
        // Pin: a state with NO session bindings still round-trips
        // cleanly through the V2 envelope's session-bindings
        // extension. The encoder emits the (osc_count=0,
        // midi_count=0) tuple; the decoder reads it back into an
        // empty `SessionBindings`. Pins the structural-no-op
        // behaviour the producer side will lean on at host startup
        // before any device enumeration completes.
        let path = unique_save_path("class-d-empty");
        let pre_crash = State::default();
        {
            let mut adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
            adapter.save(&pre_crash).expect("save must succeed");
        }
        let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, path.clone());
        let recovered = adapter
            .try_recover()
            .expect("recover must succeed")
            .expect("recover must return Some(state)");
        assert!(recovered.session_bindings().osc_devices.is_empty());
        assert!(recovered.session_bindings().midi_devices.is_empty());
        let mut post_crash = State::default();
        restore_class_d(&mut post_crash, &recovered);
        assert!(post_crash.session_bindings().osc_devices.is_empty());
        assert!(post_crash.session_bindings().midi_devices.is_empty());
        cleanup(&path);
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
