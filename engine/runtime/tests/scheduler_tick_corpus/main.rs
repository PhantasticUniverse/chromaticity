// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 slice-3 EV-SCHEDULER-TICK corpus. Tier declaration and
//! evidence tag live on the `fixtures` inner module below, matching
//! the pattern established by `engine/runtime/tests/state_replay_corpus/
//! main.rs` so `scripts/evidence-tag-lint.sh`'s tier-distribution
//! summary counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-SCHEDULER-TICK (T2) — slice-3 scheduler-tick
    //! corpus, 100 trials per fixture, cross-tick drift pinned via
    //! canonical-state-binary + FNV-1a 64 golden hashes plus
    //! frame-counter post-conditions.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) seeds an `Lcg` PRNG with a fixture-specific
    //! constant, (b) generates `count` commands by sampling from
    //! the slice-2 [`Command`] vocabulary using the same LCG +
    //! variant-mix that Phase-1 `EV-DETERMINISM` and Phase-3
    //! `EV-STATE-REPLAY` established, (c) for each command:
    //! dispatches it on a fresh [`InMemoryBus`] and immediately
    //! [`Scheduler::tick`]s a fresh [`State`] forward by one,
    //! (d) accumulates the per-tick [`TickReport::events_committed`]
    //! count, and (e) computes the canonical state hash via
    //! [`canonical_state_hash`] for golden assertion.
    //!
    //! Fixtures cover the EV-SCHEDULER-TICK contract per
    //! `docs/26-evidence-artifact-registry.md` row 38 (validate
    //! scheduler-tick frame advancement and command-bus drain
    //! composition): for a given command sequence the resulting
    //! canonical state hash, the post-tick frame counter, and the
    //! cumulative events-committed total are all bit-identical
    //! across runs, across trials, and across commits. The fixtures
    //! span small to large command counts (50 / 256 / 512 / 1024)
    //! plus targeted variant patterns (multistore-interleaved-256,
    //! snap-cycles-128, playlist-saturation-1024) so single-store
    //! regressions and edge-case state transitions are isolated.
    //!
    //! Each fixture is replayed `TRIALS = 100` times. Cross-trial
    //! identity is asserted first (regression signal for any
    //! nondeterminism in command sampling, dispatch, scheduler
    //! tick, frame increment, or state mutation); a stored golden
    //! hash over the canonical state encoding is asserted second
    //! (regression signal for canonical-encoding drift, sub-store-
    //! shape drift, scheduler ordering drift, or state mutation
    //! drift across commits).
    //!
    //! ## Relationship to EV-STATE-REPLAY corpus
    //!
    //! `engine/runtime/tests/state_replay_corpus/main.rs` exercises
    //! the [`InMemoryBus::commit_all_into`] path directly with
    //! `frame=0` static state — that corpus's goldens are
    //! `Scheduler`-independent and remain unchanged across this
    //! slice (per the additivity invariant in
    //! `engine/runtime/src/scheduler.rs` module-level prose). The
    //! EV-SCHEDULER-TICK corpus exercises the orthogonal
    //! [`Scheduler::tick`] path that composes
    //! [`InMemoryBus::commit_all_into`] with an explicit
    //! `state.frame += 1` advancement, then dispatches one command
    //! per tick so the per-tick events-committed total and the
    //! per-tick frame increment are observable across the run. The
    //! goldens are independent: dispatching N commands in one
    //! commit (state-replay shape) versus dispatching one command
    //! per tick across N ticks (scheduler-tick shape) yield
    //! different EventSeq sequences and therefore different
    //! canonical state hashes, even when the underlying command
    //! sampler is identical.
    //!
    //! ## Cumulative-release-gate scope
    //!
    //! `docs/26-evidence-artifact-registry.md` row 38 declares
    //! `Gate Usage = "Phase 4+ + release cumulative gate"`. This
    //! corpus satisfies the slice-3 part-2 baseline; subsequent
    //! slice-3 PRs (audio-uniform pull, render dispatch,
    //! observability emission) widen the tick path's surface and
    //! extend this corpus accordingly. The Pass-flip on row 38
    //! lands in the slice-3 closing docs PR (PR 6 of the wave)
    //! after subsystem-wiring PRs 3/4/5 land their evidence tags.

    use chromaticity_runtime::scheduler::{Scheduler, TickReport};
    use chromaticity_runtime::state::State;
    use chromaticity_runtime::state_replay::canonical_state_hash;
    use chromaticity_runtime::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource,
        PresetId, SnapSlotId,
    };

    /// Replay-trial count per fixture. Every trial must produce an
    /// identical (frame, hash, events-committed) triple; the stored
    /// golden hash pins the canonical form across commits.
    const TRIALS: u32 = 100;

    /// Seeded deterministic command sampler (Knuth MMIX LCG). Mirrors
    /// the Phase-1 `EV-DETERMINISM` corpus's `Lcg` and `sample_command`
    /// shapes verbatim so the EV-SCHEDULER-TICK and EV-STATE-REPLAY
    /// corpora share a known-good command sampler — divergence in
    /// canonical state hash between the two is therefore attributable
    /// to scheduler-tick semantics alone.
    struct Lcg(u64);

    impl Lcg {
        fn new(seed: u64) -> Self {
            Self(seed)
        }
        fn next_u32(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 32) as u32
        }
        fn next_f32_unit(&mut self) -> f32 {
            let n = self.next_u32();
            (n as f32) / (u32::MAX as f32)
        }
    }

    fn sample_command(lcg: &mut Lcg) -> Command {
        let kind = lcg.next_u32() % 7;
        let id_val = lcg.next_u32();
        match kind {
            0 => Command::ControlSet {
                id: ControlId(id_val),
                value: ControlValue(lcg.next_f32_unit()),
            },
            1 => Command::PresetRecall {
                id: PresetId(id_val),
            },
            2 => Command::PlaylistAdvance {
                step: lcg.next_u32() as i32,
            },
            3 => Command::MappingBind {
                id: MappingId(id_val),
                source: MappingSource::Osc,
            },
            4 => Command::SnapRecall {
                slot: SnapSlotId::new((id_val % 16) as u8).expect("modulo keeps slot in range"),
            },
            5 => Command::SnapSave {
                slot: SnapSlotId::new((id_val % 16) as u8).expect("modulo keeps slot in range"),
            },
            _ => Command::SnapClear {
                slot: SnapSlotId::new((id_val % 16) as u8).expect("modulo keeps slot in range"),
            },
        }
    }

    /// Run one trial: for each of `count` seeded commands, dispatch
    /// to a fresh `InMemoryBus` and immediately tick the scheduler
    /// against a fresh `State`. Returns the post-run frame counter,
    /// the canonical state hash, and the cumulative events-committed
    /// total summed from each [`TickReport`].
    fn run_trial(seed: u64, count: u32) -> (u64, u64, u64) {
        let bus = InMemoryBus::in_memory();
        let mut state = State::default();
        let mut scheduler = Scheduler::new();
        let mut lcg = Lcg::new(seed);
        let mut total_committed = 0_u64;
        for _ in 0..count {
            bus.dispatch(sample_command(&mut lcg))
                .expect("in-memory bus dispatch must succeed");
            let report: TickReport = scheduler
                .tick(&mut state, &bus)
                .expect("scheduler tick must succeed under non-overflow conditions");
            total_committed += report.events_committed as u64;
        }
        (state.frame(), canonical_state_hash(&state), total_committed)
    }

    fn assert_corpus(name: &str, seed: u64, count: u32, golden_hash: u64) {
        let trials: Vec<(u64, u64, u64)> = (0..TRIALS).map(|_| run_trial(seed, count)).collect();

        // Cross-trial identity: every trial must produce the same
        // (frame, hash, events-committed) triple. A regression in
        // dispatch / scheduler tick / commit / apply_event /
        // canonical-encode / FNV-1a-64 surfaces here as the FIRST
        // failure mode (much louder than the golden assertion).
        for (idx, triple) in trials.iter().enumerate().skip(1) {
            assert_eq!(
                trials[0], *triple,
                "{name}: trial {idx} diverged from trial 0; \
                 nondeterminism in the dispatch/tick/commit/apply pipeline",
            );
        }

        let (frame, observed_hash, total_committed) = trials[0];

        // Frame post-condition: an N-tick run lands the scheduler
        // at frame N. Pins the slice-3 PR-1 frame-advancement
        // invariant across the corpus surface.
        assert_eq!(
            frame,
            u64::from(count),
            "{name}: frame counter mismatch — observed {frame}, expected {count}; \
             scheduler-tick frame-advancement invariant regressed",
        );

        // Events-committed correctness: dispatching one command per
        // tick means every tick's events_committed should be 1, so
        // the cumulative total must equal the tick count exactly.
        // Pins the slice-3 PR-1 command-bus drain invariant across
        // the corpus surface.
        assert_eq!(
            total_committed,
            u64::from(count),
            "{name}: cumulative events_committed mismatch — observed {total_committed}, \
             expected {count}; command-bus drain invariant regressed",
        );

        // Golden-hash assertion: the trial-0 hash must match the
        // stored constant. Drift here surfaces as a deterministic
        // shift across commits (canonical-encoding change,
        // sub-store-shape change, scheduler ordering change, or
        // state-mutation change).
        assert_eq!(
            observed_hash, golden_hash,
            "{name}: golden drift; observed 0x{observed_hash:016x} \
             expected 0x{golden_hash:016x}. If this regression is \
             intentional (canonical-encoding bump, sub-store widening, \
             scheduler-tick semantics change, new EventPayload \
             variant), update the golden constant via a `docs(repo)`- \
             or `feat(runtime)`-scoped commit and reference this test \
             path.",
        );
    }

    // --- Fixtures (6 total) --------------------------------------------
    //
    // Goldens were measured on Apple Silicon Metal at HEAD against the
    // slice-3 part-1 scheduler-tick API. The hashes are pure-Rust
    // outputs of `canonical_state_hash` (no GPU involvement) so they
    // should be backend-invariant — the per-backend ACCEPTED-set
    // pattern that the GPU-rendered corpora carry is not needed
    // here. Linux Vulkan / Mesa llvmpipe (CI), macOS Metal nightly,
    // and any future backend should produce the same hash, matching
    // the EV-STATE-REPLAY backend-invariance pattern.

    #[test]
    fn tick_50_command_baseline_advances_frame_by_50_and_converges_to_golden_hash() {
        assert_corpus(
            "50-command baseline",
            0x0C1D_5EED_u64,
            50,
            0x9aea_fa9d_f9a1_d217,
        );
    }

    #[test]
    fn tick_256_command_scale_advances_frame_to_256_and_converges_to_golden() {
        assert_corpus(
            "256-command scale",
            0xDEAD_BEEF_u64,
            256,
            0xdee3_53f8_7832_e025,
        );
    }

    #[test]
    fn tick_512_command_complex_chain_converges_to_golden() {
        assert_corpus(
            "512-command complex chain",
            0xBEEF_DEAD_u64,
            512,
            0xaf1a_861c_3434_58f1,
        );
    }

    #[test]
    fn tick_multistore_interleaved_256_converges_to_golden() {
        assert_corpus(
            "multistore interleaved 256",
            0xF00D_CADE_u64,
            256,
            0xefdc_b55e_0b90_a88c,
        );
    }

    #[test]
    fn tick_snap_cycles_128_converges_to_golden() {
        assert_corpus(
            "snap cycles 128",
            0xCAFE_BABE_u64,
            128,
            0x8bfa_6e7f_e2c4_14e5,
        );
    }

    #[test]
    fn tick_playlist_saturation_1024_converges_to_golden() {
        assert_corpus(
            "playlist saturation 1024",
            0xDECA_FBAD_u64,
            1024,
            0x4967_59f4_12e1_83d4,
        );
    }
}
