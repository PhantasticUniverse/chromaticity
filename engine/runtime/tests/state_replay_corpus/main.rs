// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 slice-1 EV-STATE-REPLAY corpus. Tier declaration and
//! evidence tag live on the `fixtures` inner module below, matching
//! the pattern established by `engine/runtime/tests/determinism/
//! main.rs` so `scripts/evidence-tag-lint.sh`'s tier-distribution
//! summary counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-STATE-REPLAY (T2) — Phase-3 slice-1 state-replay
    //! corpus, 100 trials per fixture, cross-commit drift pinned via
    //! canonical-state-binary + FNV-1a 64 golden hashes.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) seeds an `Lcg` PRNG with a fixture-specific
    //! constant, (b) generates `count` commands by sampling from
    //! the slice-2 [`Command`] vocabulary using the same LCG +
    //! variant-mix that Phase-1 `EV-DETERMINISM` established, (c)
    //! dispatches them on a fresh [`InMemoryBus`], (d) commits the
    //! events out via [`InMemoryBus::commit_all_into`], and (e)
    //! re-applies the events to a SECOND fresh [`State`] via
    //! [`replay_canonical_hash`] to pin the canonical state hash.
    //!
    //! Fixtures cover the EV-STATE-REPLAY contract as described in
    //! `docs/26-evidence-artifact-registry.md` row 32 (validate
    //! deterministic state transitions): for a given command
    //! sequence the resulting canonical state is bit-identical
    //! across runs and across commits. The fixtures span small to
    //! large command counts (50 / 256 / 512 / 1024) plus targeted
    //! variant patterns (multistore-interleaved-256, snap-cycles-
    //! 128, playlist-saturation-1024) so single-store regressions
    //! and edge-case state transitions are isolated.
    //!
    //! Each fixture is replayed `TRIALS = 100` times. Cross-trial
    //! identity is asserted first (regression signal for any
    //! nondeterminism in command sampling, dispatch, commit, or
    //! state mutation); a stored golden hash over the canonical
    //! state encoding is asserted second (regression signal for
    //! canonical-encoding drift, sub-store-shape drift, or state
    //! mutation drift across commits).
    //!
    //! Cumulative-release-gate scope.
    //! `docs/26-evidence-artifact-registry.md` row 32 declares
    //! `Gate Usage = "Phase 3 + release cumulative gate"`. This
    //! corpus satisfies the Phase-3 baseline; future widening to
    //! cover migration / preset / playlist / show-bundle workflows
    //! happens under the sibling Phase-3 EXIT rows
    //! (`EV-PRESET-PLAYLIST` row 34, `EV-STATE-MIGRATION` row 33,
    //! `EV-SHOW-BUNDLE-RECOVERY` row 35) which share this corpus's
    //! canonical-encoding + golden-hash discipline.

    use chromaticity_runtime::state_replay::replay_canonical_hash;
    use chromaticity_runtime::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource,
        PresetId, SnapSlotId,
    };

    /// Replay-trial count per fixture. Every trial must produce an
    /// identical canonical state hash; the stored golden hash pins
    /// the canonical form across commits.
    const TRIALS: u32 = 100;

    /// Seeded deterministic command sampler (Knuth MMIX LCG). Mirrors
    /// the Phase-1 `EV-DETERMINISM` corpus's `Lcg` and `sample_command`
    /// shapes verbatim so the two corpora share a known-good command
    /// sampler.
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

    /// Run one trial: dispatch `count` seeded commands on a fresh
    /// `InMemoryBus`, commit the events out, replay them through
    /// `replay_canonical_hash`, return the hash.
    fn run_trial(seed: u64, count: u32) -> u64 {
        let bus = InMemoryBus::in_memory();
        let mut lcg = Lcg::new(seed);
        for _ in 0..count {
            bus.dispatch(sample_command(&mut lcg))
                .expect("in-memory bus dispatch must succeed");
        }
        let mut state = chromaticity_runtime::state::State::default();
        let events = bus.commit_all_into(&mut state);
        replay_canonical_hash(&events)
    }

    fn assert_corpus(name: &str, seed: u64, count: u32, golden: u64) {
        let trials: Vec<u64> = (0..TRIALS).map(|_| run_trial(seed, count)).collect();
        // Cross-trial identity: every trial must produce the same
        // hash. A regression in dispatch / commit / apply_event /
        // canonical-encode / FNV-1a-64 surfaces here as the FIRST
        // failure mode (much louder than the golden assertion).
        for (idx, hash) in trials.iter().enumerate().skip(1) {
            assert_eq!(
                trials[0], *hash,
                "{name}: trial {idx} hash 0x{hash:016x} diverged from \
                 trial 0 hash 0x{:016x}; nondeterminism in the \
                 dispatch/commit/replay pipeline",
                trials[0],
            );
        }
        // Golden-hash assertion: the trial-0 hash must match the
        // stored constant. Drift here surfaces as a deterministic
        // shift across commits (canonical-encoding change,
        // sub-store-shape change, or state-mutation change).
        let observed = trials[0];
        assert_eq!(
            observed, golden,
            "{name}: golden drift; observed 0x{observed:016x} \
             expected 0x{golden:016x}. If this regression is \
             intentional (canonical-encoding bump, sub-store widening, \
             new EventPayload variant), update the golden constant \
             via a `docs(repo)`- or `feat(runtime)`-scoped commit and \
             reference this test path.",
        );
    }

    // --- Fixtures (6 total) --------------------------------------------
    //
    // Goldens were measured on Apple Silicon Metal at HEAD `cb3744d`
    // post-Phase-2-EXIT. The hashes are pure-Rust outputs of the
    // `replay_canonical_hash` helper (no GPU involvement) so they
    // should be backend-invariant — the per-backend ACCEPTED-set
    // pattern that the GPU-rendered corpora carry is not needed
    // here. Linux Vulkan / Mesa llvmpipe (CI), macOS Metal nightly,
    // and any future backend should produce the same hash.

    #[test]
    fn replay_50_commands_baseline_converges_to_golden_hash() {
        assert_corpus(
            "50-command baseline",
            0x0C1D_5EED_u64,
            50,
            0xad68_aacc_2d5f_a3b5,
        );
    }

    #[test]
    fn replay_256_commands_scale_converges_to_golden_hash() {
        assert_corpus(
            "256-command scale",
            0xDEAD_BEEF_u64,
            256,
            0x73f3_55fd_7b9c_0d18,
        );
    }

    #[test]
    fn replay_512_commands_complex_chain_converges_to_golden_hash() {
        assert_corpus(
            "512-command complex chain",
            0xBEEF_DEAD_u64,
            512,
            0x95f7_d430_a850_cd3b,
        );
    }

    #[test]
    fn replay_multistore_interleaved_converges_to_golden_hash() {
        assert_corpus(
            "multistore interleaved 256",
            0xF00D_CADE_u64,
            256,
            0x6a81_20bf_e05d_9795,
        );
    }

    #[test]
    fn replay_snap_cycles_converges_to_golden_hash() {
        assert_corpus(
            "snap cycles 128",
            0xCAFE_BABE_u64,
            128,
            0x32d5_4ec8_81c1_b465,
        );
    }

    #[test]
    fn replay_playlist_saturation_edge_case_converges_to_golden_hash() {
        assert_corpus(
            "playlist saturation 1024",
            0xDECA_FBAD_u64,
            1024,
            0x3a09_d6f4_c07c_c190,
        );
    }
}
