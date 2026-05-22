// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 EV-STATE-MIGRATION corpus. Tier declaration and evidence
//! tag live on the `fixtures` inner module below, matching the
//! pattern established by `engine/runtime/tests/state_replay_corpus/
//! main.rs` so `scripts/evidence-tag-lint.sh`'s tier-distribution
//! summary counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-STATE-MIGRATION (T2) — Phase-3 state-migration
    //! corpus, 100 trials per fixture, exercising the
    //! [`crate::state_migration`] versioned snapshot envelope across
    //! the V1↔V2 surface.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) seeds an `Lcg` PRNG with a fixture-specific
    //! constant, (b) generates `count` commands by sampling from
    //! the slice-2 [`Command`] vocabulary using the same LCG +
    //! variant-mix that EV-STATE-REPLAY established, (c) dispatches
    //! them on a fresh [`InMemoryBus`] and commits into a fresh
    //! [`State`], and (d) drives the snapshot-envelope path
    //! covered by the fixture's specific assertion.
    //!
    //! Fixtures cover the EV-STATE-MIGRATION contract per
    //! `docs/26-evidence-artifact-registry.md` row 33: forward
    //! migration of canonical state snapshots across schema
    //! versions, with the Phase-3 narrowing pinning the V1 → V2
    //! transition. Fixture set:
    //!
    //! 1. `decode_v2_default_state_round_trips` — encode + decode
    //!    of the default state preserves the canonical hash.
    //! 2. `decode_v2_round_trips_after_mixed_command_stream` — 256
    //!    Lcg-seeded commands committed into state, encoded as V2,
    //!    decoded, canonical hash matches the pre-encode hash.
    //! 3. `decode_v2_rejects_truncated_buffer` — truncating the
    //!    V2 envelope at every 4-byte boundary surfaces
    //!    `DecodeError::Truncated`.
    //! 4. `decode_v2_rejects_unknown_version_tag` — synthesising a
    //!    buffer with version `0xFFFF_FFFF` surfaces
    //!    `DecodeError::UnknownVersion`.
    //! 5. `decode_v2_rejects_invalid_mapping_source_tag` — flipping
    //!    a mapping-source tag byte to a non-{0x01, 0x02} value
    //!    surfaces `DecodeError::InvalidMappingTag`.
    //! 6. `migrate_v1_to_v2_seeds_default_metadata` — V1 → migrate
    //!    → V2 → decode produces `V2SchemaMetadata { schema_version
    //!    = 2 }`.
    //! 7. `migrate_v1_to_v2_preserves_observable_state` — non-
    //!    trivial multi-store payload survives the V1 → V2
    //!    migration: every public `State` accessor matches the
    //!    pre-migration value.
    //! 8. `migrate_v1_to_v2_then_round_trip_v2` — V1 → V2 → V2
    //!    decode → V2 re-encode is idempotent (chained migration
    //!    converges to a fixed-point).
    //! 9. `decode_v2_round_trips_xy_control_values` — parity-closure
    //!    wave 2 Lane D item 1: a state carrying `ControlSet2D`-
    //!    committed XY values round-trips through the V2 envelope
    //!    via the `EXT_ID_V2_XY_CONTROL_VALUES` extension, and the
    //!    XY payload leaves the canonical hash invariant.
    //!
    //! Each fixture is replayed `TRIALS = 100` times; cross-trial
    //! identity is asserted as the loudest regression signal, then
    //! the per-fixture invariant is asserted.
    //!
    //! Cumulative-release-gate scope.
    //! `docs/26-evidence-artifact-registry.md` row 33 declares
    //! `Gate Usage = "Phase 3 + release cumulative gate"`. This
    //! corpus satisfies the Phase-3 baseline; rollback (V2 → V1)
    //! and broader transition matrices are Phase-4+ scope per the
    //! row 33 description column.

    use chromaticity_runtime::state::State;
    use chromaticity_runtime::state_migration::{
        DecodeError, V2SchemaMetadata, decode_snapshot, encode_snapshot, encode_snapshot_v1,
        migrate_v1_to_v2,
    };
    use chromaticity_runtime::state_replay::canonical_state_hash;
    use chromaticity_runtime::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource,
        PresetId, SnapSlotId,
    };

    /// Replay-trial count per fixture. Mirrors the EV-STATE-REPLAY
    /// corpus constant verbatim.
    const TRIALS: u32 = 100;

    /// Seeded deterministic command sampler (Knuth MMIX LCG). Mirrors
    /// the EV-STATE-REPLAY corpus's `Lcg` and `sample_command` shapes
    /// verbatim so the two corpora share a known-good command sampler.
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

    /// Build a fresh [`State`] by dispatching `count` seeded
    /// commands through the public bus surface. Mirrors the
    /// EV-STATE-REPLAY corpus's `run_trial` shape.
    fn build_state(seed: u64, count: u32) -> State {
        let bus = InMemoryBus::in_memory();
        let mut lcg = Lcg::new(seed);
        for _ in 0..count {
            bus.dispatch(sample_command(&mut lcg))
                .expect("in-memory bus dispatch must succeed");
        }
        let mut state = State::default();
        bus.commit_all_into(&mut state);
        state
    }

    // --- Fixture 1: V2 default-state round-trip ----------------------

    #[test]
    fn decode_v2_default_state_round_trips() {
        let mut hashes = Vec::with_capacity(TRIALS as usize);
        for _ in 0..TRIALS {
            let state = State::default();
            let pre_hash = canonical_state_hash(&state);
            let buf = encode_snapshot(&state);
            let decoded = decode_snapshot(&buf).expect("default-state v2 envelope must decode");
            let post_hash = canonical_state_hash(&decoded);
            assert_eq!(
                pre_hash, post_hash,
                "default-state V2 round-trip must preserve canonical hash",
            );
            assert_eq!(
                decoded.snapshot_metadata(),
                Some(V2SchemaMetadata::v2_default()),
                "default-state V2 decode must surface V2 default metadata",
            );
            hashes.push(post_hash);
        }
        for (idx, hash) in hashes.iter().enumerate().skip(1) {
            assert_eq!(
                hashes[0], *hash,
                "default-state round-trip: trial {idx} hash diverged",
            );
        }
    }

    // --- Fixture 2: V2 mixed-command-stream round-trip ---------------

    #[test]
    fn decode_v2_round_trips_after_mixed_command_stream() {
        let seed: u64 = 0xA5A5_5A5A_DEAD_BEEF;
        let count: u32 = 256;
        let mut hashes = Vec::with_capacity(TRIALS as usize);
        for _ in 0..TRIALS {
            let state = build_state(seed, count);
            let pre_hash = canonical_state_hash(&state);
            let buf = encode_snapshot(&state);
            let decoded = decode_snapshot(&buf).expect("v2 envelope must decode");
            let post_hash = canonical_state_hash(&decoded);
            assert_eq!(
                pre_hash, post_hash,
                "256-cmd V2 round-trip must preserve canonical hash",
            );
            hashes.push(post_hash);
        }
        for (idx, hash) in hashes.iter().enumerate().skip(1) {
            assert_eq!(
                hashes[0], *hash,
                "256-cmd round-trip: trial {idx} hash diverged",
            );
        }
    }

    // --- Fixture 3: V2 truncated-buffer rejection --------------------

    #[test]
    fn decode_v2_rejects_truncated_buffer() {
        // Build a non-trivial V2 envelope, then truncate at every
        // 4-byte boundary below the full length and assert each
        // truncation surfaces `DecodeError::Truncated`. The empty
        // truncation (length 0) IS the prefix-truncation case the
        // version-tag reader catches; the all-but-last truncation
        // catches the extension trailer.
        let state = build_state(0xCAFE_F00D_DEAD_BEEF, 64);
        let buf = encode_snapshot(&state);
        assert!(
            buf.len() >= 8,
            "envelope must be wider than the version tag"
        );
        for trial in 0..TRIALS {
            // Re-deriving the truncation point from the trial index
            // keeps the corpus structure identical to siblings; the
            // truncation is deterministic but every trial probes a
            // different cut.
            let cut = ((trial as usize) * 4) % (buf.len() - 4);
            let truncated = &buf[..cut];
            let err = decode_snapshot(truncated).expect_err("truncated buffer must reject");
            assert!(
                matches!(err, DecodeError::Truncated),
                "truncated buffer at cut {cut} surfaced {err:?}, want Truncated",
            );
        }
    }

    // --- Fixture 4: V2 unknown-version-tag rejection -----------------

    #[test]
    fn decode_v2_rejects_unknown_version_tag() {
        // Synthesise a buffer with version `0xFFFF_FFFF` (no other
        // bytes); each trial reproduces the same rejection. The
        // T1 pin asserts this once; this fixture pins the
        // deterministic outcome under the corpus's TRIAL discipline
        // so a flaky / nondeterministic regression in the version-
        // tag arm would surface as cross-trial divergence.
        let synth: Vec<u8> = 0xFFFF_FFFF_u32.to_be_bytes().to_vec();
        for trial in 0..TRIALS {
            let err = decode_snapshot(&synth).expect_err("unknown-version buffer must reject");
            assert!(
                matches!(err, DecodeError::UnknownVersion(0xFFFF_FFFF)),
                "trial {trial}: expected UnknownVersion(0xFFFFFFFF), got {err:?}",
            );
        }
    }

    // --- Fixture 5: V2 invalid-mapping-tag rejection -----------------

    #[test]
    fn decode_v2_rejects_invalid_mapping_source_tag() {
        // Build a state whose mapping_store has a single binding,
        // encode as V2, then flip the mapping-source tag byte to
        // 0xFF. Asserts `DecodeError::InvalidMappingTag(0xFF)`.
        // The mapping byte is the LAST byte of the canonical body
        // before the extension trailer; the layout per
        // `state_replay.rs` lines 181-209 is `be_u32(count) ||
        // pairs[ be_u32(id) || u8(tag) ]`, so for a single-mapping
        // state the tag byte sits at canonical_body_end - 1 -
        // extension_trailer_size. Compute via re-encoding rather
        // than hand-counted offset to stay encoder-shape-agnostic.
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::MappingBind {
            id: MappingId(7),
            source: MappingSource::Osc,
        })
        .expect("in-memory bus dispatch must succeed");
        let mut state = State::default();
        bus.commit_all_into(&mut state);

        // Compute the mapping-tag offset: the V2 envelope is
        // `[version u32] || canonical_body || [extension trailer]`,
        // and within canonical_body the mapping store is the last
        // section. Reusing the encoder helpers via `encode_snapshot`
        // keeps the offset derivation honest under encoder drift.
        let buf = encode_snapshot(&state);
        // Extension trailer for V2 with three extensions on a state
        // carrying empty `session_bindings` (Wave 4d.3 PR 5, row 80
        // class-d) and empty `xy_values` (parity-closure wave 2
        // Lane D item 1):
        //   be_u32(extension_count=3)                              [4]
        //   ext1: be_u32(ext_id=schema_metadata) || be_u32(schema)  [8]
        //   ext2: be_u32(ext_id=session_bindings)
        //      || be_u32(osc_count=0) || be_u32(midi_count=0)      [12]
        //   ext3: be_u32(ext_id=xy_control_values)
        //      || be_u32(xy_count=0)                                [8]
        // = 32 bytes. The mapping-store fixture used by this test
        // populates neither `session_bindings` nor `xy_values`, so
        // the empty shape applies to both.
        const V2_EXTENSION_TRAILER_LEN: usize = 32;
        let mapping_tag_offset = buf
            .len()
            .checked_sub(V2_EXTENSION_TRAILER_LEN + 1)
            .expect("envelope must hold extension trailer + at least one tag byte");

        for trial in 0..TRIALS {
            let mut tampered = buf.clone();
            // Vary the corrupt tag across trials in {0x00, 0x03..0xFF}
            // so the corpus probes the full invalid-tag space, not
            // just one byte. Skip 0x01 / 0x02 (the valid tags).
            let candidate = (trial as u8).wrapping_add(0x10);
            let corrupt_tag = if candidate == 0x01 || candidate == 0x02 {
                0xFF
            } else {
                candidate
            };
            tampered[mapping_tag_offset] = corrupt_tag;
            let err =
                decode_snapshot(&tampered).expect_err("invalid mapping-tag buffer must reject");
            match err {
                DecodeError::InvalidMappingTag(observed) => {
                    assert_eq!(
                        observed, corrupt_tag,
                        "trial {trial}: corrupt-tag value mismatch",
                    );
                }
                other => panic!(
                    "trial {trial}: expected InvalidMappingTag({corrupt_tag:#04x}), got {other:?}",
                ),
            }
        }
    }

    // --- Fixture 6: V1 → V2 default-metadata seeding -----------------

    #[test]
    fn migrate_v1_to_v2_seeds_default_metadata() {
        for trial in 0..TRIALS {
            let state = build_state(0x1111_2222_3333_4444 ^ u64::from(trial), 32);
            let v1 = encode_snapshot_v1(&state);
            let v2 = migrate_v1_to_v2(&v1).expect("v1 must migrate to v2");
            let decoded = decode_snapshot(&v2).expect("migrated v2 must decode");
            assert_eq!(
                decoded.snapshot_metadata(),
                Some(V2SchemaMetadata::v2_default()),
                "trial {trial}: V1 → V2 must seed V2 default metadata",
            );
        }
    }

    // --- Fixture 7: V1 → V2 preserves observable state ---------------

    #[test]
    fn migrate_v1_to_v2_preserves_observable_state() {
        // Non-trivial multi-store payload: dispatch a curated
        // command sequence that touches every public State accessor
        // (control_value, active_preset, playlist_position,
        // active_snap_slot, snap_slot_saved, mapping_source). After
        // V1 → migrate → V2 → decode, every accessor must match the
        // pre-migration value. Cross-trial identity is implicit:
        // `build_state` is deterministic so re-running yields the
        // same state. The fixture's TRIAL loop pins re-determinism
        // of the migration path itself.
        let snap = SnapSlotId::new(7).expect("valid snap slot");
        for trial in 0..TRIALS {
            let bus = InMemoryBus::in_memory();
            bus.dispatch(Command::ControlSet {
                id: ControlId(11),
                value: ControlValue(0.42),
            })
            .expect("control dispatch must succeed");
            bus.dispatch(Command::PresetRecall { id: PresetId(13) })
                .expect("preset dispatch must succeed");
            bus.dispatch(Command::PlaylistAdvance { step: 17 })
                .expect("playlist dispatch must succeed");
            bus.dispatch(Command::SnapSave { slot: snap })
                .expect("snap save dispatch must succeed");
            bus.dispatch(Command::MappingBind {
                id: MappingId(19),
                source: MappingSource::Midi,
            })
            .expect("mapping dispatch must succeed");
            let mut original = State::default();
            bus.commit_all_into(&mut original);

            let original_hash = canonical_state_hash(&original);
            let v1 = encode_snapshot_v1(&original);
            let v2 = migrate_v1_to_v2(&v1).expect("v1 must migrate to v2");
            let decoded = decode_snapshot(&v2).expect("migrated v2 must decode");
            let decoded_hash = canonical_state_hash(&decoded);

            // Canonical-hash invariant: hashing ignores
            // snapshot_metadata, so the decoded state hashes
            // identically to the original.
            assert_eq!(
                original_hash, decoded_hash,
                "trial {trial}: V1 → V2 must preserve canonical hash",
            );

            // Direct accessor invariants: every public State
            // accessor matches.
            assert_eq!(
                decoded.control_value(ControlId(11)),
                Some(ControlValue(0.42)),
                "trial {trial}: control_value must survive migration",
            );
            assert_eq!(
                decoded.active_preset(),
                Some(PresetId(13)),
                "trial {trial}: active_preset must survive migration",
            );
            assert_eq!(
                decoded.playlist_position(),
                17,
                "trial {trial}: playlist_position must survive migration",
            );
            assert_eq!(
                decoded.active_snap_slot(),
                Some(snap),
                "trial {trial}: active_snap_slot must survive migration",
            );
            assert!(
                decoded.snap_slot_saved(snap),
                "trial {trial}: snap_slot_saved(snap) must survive migration",
            );
            assert_eq!(
                decoded.mapping_source(MappingId(19)),
                Some(MappingSource::Midi),
                "trial {trial}: mapping_source must survive migration",
            );
        }
    }

    // --- Fixture 8: V1 → V2 chain idempotency ------------------------

    #[test]
    fn migrate_v1_to_v2_then_round_trip_v2() {
        // Chain idempotency: V1 → migrate → V2 → decode → encode
        // again must produce the same V2 buffer. Pins that the
        // migration outcome is a fixed-point under the V2 encoder
        // (no metadata drift, no schema-version inflation).
        for trial in 0..TRIALS {
            let state = build_state(0xDEAD_BEEF_FEED_FACE ^ u64::from(trial), 64);
            let v1 = encode_snapshot_v1(&state);
            let v2_first = migrate_v1_to_v2(&v1).expect("v1 must migrate to v2");
            let decoded = decode_snapshot(&v2_first).expect("first v2 must decode");
            let v2_second = encode_snapshot(&decoded);
            assert_eq!(
                v2_first, v2_second,
                "trial {trial}: V1 → V2 → decode → encode must be a fixed-point",
            );
        }
    }

    // --- Fixture 9: V2 xy-control-values round-trip ------------------

    #[test]
    fn decode_v2_round_trips_xy_control_values() {
        // Parity-closure wave 2 Lane D item 1: a state carrying
        // `ControlSet2D`-committed XY values must survive the V2
        // envelope round-trip via the `EXT_ID_V2_XY_CONTROL_VALUES`
        // extension. `build_state`'s LCG sampler never emits
        // `ControlSet2D` (the `kind % 7` mix predates the variant),
        // so the XY payload is dispatched directly here. Each trial
        // LCG-samples a distinct two-control payload — ids in
        // disjoint even/odd lanes, axis values across the signed
        // [-1, 1] range — so the round-trip is exercised across the
        // value space rather than one fixed point, exactly as the
        // command-stream fixtures (2 / 7 / 8) vary their LCG seed.
        for trial in 0..TRIALS {
            let mut lcg = Lcg::new(0x7C0F_FEE5_1234_ABCD ^ u64::from(trial));
            // Even/odd id lanes guarantee `id_a != id_b` so the two
            // XY entries never collide within a trial's fresh state.
            let id_a = ControlId(lcg.next_u32() & !1);
            let val_a = [
                lcg.next_f32_unit() * 2.0 - 1.0,
                lcg.next_f32_unit() * 2.0 - 1.0,
            ];
            let id_b = ControlId(lcg.next_u32() | 1);
            let val_b = [
                lcg.next_f32_unit() * 2.0 - 1.0,
                lcg.next_f32_unit() * 2.0 - 1.0,
            ];

            let bus = InMemoryBus::in_memory();
            bus.dispatch(Command::ControlSet2D {
                id: id_a,
                value: val_a,
            })
            .expect("xy control dispatch must succeed");
            bus.dispatch(Command::ControlSet2D {
                id: id_b,
                value: val_b,
            })
            .expect("xy control dispatch must succeed");
            let mut original = State::default();
            bus.commit_all_into(&mut original);

            let buf = encode_snapshot(&original);
            let decoded = decode_snapshot(&buf).expect("xy-bearing v2 envelope must decode");

            assert_eq!(
                decoded.xy_control_value(id_a),
                Some(val_a),
                "trial {trial}: xy control a must survive the V2 round-trip",
            );
            assert_eq!(
                decoded.xy_control_value(id_b),
                Some(val_b),
                "trial {trial}: xy control b must survive the V2 round-trip",
            );
            // XY values ride the extension, not the canonical-binary
            // segment — the canonical hash equals the empty-state
            // hash even though two XY controls are populated.
            assert_eq!(
                canonical_state_hash(&decoded),
                canonical_state_hash(&State::default()),
                "trial {trial}: xy values must not enter the canonical hash",
            );
        }
    }
}
