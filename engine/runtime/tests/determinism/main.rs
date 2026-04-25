// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-1 slice-2 EV-DETERMINISM replay corpus. Tier declaration
//! and evidence tag live on the `fixtures` inner module below,
//! matching the pattern established by `command_bus::drift_pins`
//! and `state::tests` so `scripts/evidence-tag-lint.sh`'s
//! tier-distribution summary counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-DETERMINISM (T2) — Phase-1 slice-2 determinism
    //! replay corpus, 100 trials per fixture, cross-commit drift
    //! pinned via canonical-binary + FNV-1a 64 golden hashes.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Fixtures cover the slice-2 determinism contract as pinned
    //! in `engine/runtime/src/command_bus.rs` module docs
    //! §"Determinism guarantees": for single-producer dispatch the
    //! emitted event sequence (including every assigned `EventSeq`)
    //! is a pure function of the input command sequence, and
    //! per-producer FIFO survives commit across multi-producer
    //! dispatch.
    //!
    //! Each fixture is replayed `TRIALS = 100` times under a fresh
    //! bus. Cross-trial identity is asserted first (regression
    //! signal for nondeterminism introduced to the pipeline); a
    //! stored golden hash over a canonical binary encoding (not
    //! `Debug`) is asserted second (regression signal for
    //! canonical-encoding drift or payload-shape drift across
    //! commits).
    //!
    //! Cumulative-release-gate scope.
    //! `docs/26-evidence-artifact-registry.md` row `EV-DETERMINISM`
    //! declares `Gate Usage = "Phase 1 + release cumulative gate"`.
    //! This corpus satisfies the Phase-1 baseline; re-validation
    //! against the full slice-3+ state-reconciliation corpus (once
    //! the event-sourced reconciliation layer in
    //! `chromaticity_runtime::state` lands and commands mutate the
    //! canonical `State`) is the cumulative-release revalidation
    //! and is explicitly deferred to slice-3.

    use chromaticity_runtime::{
        Command, CommandBus, CommandBusError, CommandChannel, ControlId, ControlValue, Event,
        EventPayload, EventSeq, InMemoryBus, MappingId, MappingSource, PresetId, SnapSlotId,
    };
    use std::sync::Arc;
    use std::thread;

    /// Replay-trial count per fixture. Every trial must produce an
    /// identical canonical event stream; the stored golden hash pins
    /// the canonical form across commits.
    const TRIALS: u32 = 100;

    // --- Canonical binary encoding ----------------------------------
    //
    // Lead refinement 3: golden hashes are computed over a canonical
    // binary encoding, not `Debug` output. Format:
    //
    //   event   = be_u64(seq) || payload
    //   payload = tag_u8 || payload_fields
    //
    //   tag 0x01 ControlChanged { id, new_value }
    //             = be_u32(id.0) || be_u32(new_value.0.to_bits())
    //   tag 0x02 PresetRecalled { id }
    //             = be_u32(id.0)
    //   tag 0x03 PlaylistAdvanced { step }
    //             = be_i32(step)
    //   tag 0x04 MappingBound { id, source }
    //             = be_u32(id.0) || source_tag_u8
    //     source_tag 0x01 Osc, 0x02 Midi
    //   tag 0x05 SnapRecalled { slot } = u8(slot.get())
    //   tag 0x06 SnapSaved    { slot } = u8(slot.get())
    //   tag 0x07 SnapCleared  { slot } = u8(slot.get())
    //
    // New `Command` / `EventPayload` variants fail the match's
    // exhaustiveness check at compile time OR hit the wildcard
    // panic arm at runtime; a tag-byte assignment for any new
    // variant is a required PR edit. Tag bytes deliberately
    // non-zero so an empty event stream never aliases with a
    // single-byte payload.

    fn encode_event(event: &Event, out: &mut Vec<u8>) {
        out.extend_from_slice(&event.seq.0.to_be_bytes());
        encode_payload(&event.payload, out);
    }

    fn encode_payload(payload: &EventPayload, out: &mut Vec<u8>) {
        match payload {
            EventPayload::ControlChanged { id, new_value } => {
                out.push(0x01);
                out.extend_from_slice(&id.0.to_be_bytes());
                out.extend_from_slice(&new_value.0.to_bits().to_be_bytes());
            }
            EventPayload::PresetRecalled { id } => {
                out.push(0x02);
                out.extend_from_slice(&id.0.to_be_bytes());
            }
            EventPayload::PlaylistAdvanced { step } => {
                out.push(0x03);
                out.extend_from_slice(&step.to_be_bytes());
            }
            EventPayload::MappingBound { id, source } => {
                out.push(0x04);
                out.extend_from_slice(&id.0.to_be_bytes());
                let source_tag: u8 = match source {
                    MappingSource::Osc => 0x01,
                    MappingSource::Midi => 0x02,
                    other => panic!(
                        "determinism corpus: unencoded MappingSource variant {other:?}; \
                         assign a source-tag byte in encode_payload and regenerate goldens",
                    ),
                };
                out.push(source_tag);
            }
            EventPayload::SnapRecalled { slot } => {
                out.push(0x05);
                out.push(slot.get());
            }
            EventPayload::SnapSaved { slot } => {
                out.push(0x06);
                out.push(slot.get());
            }
            EventPayload::SnapCleared { slot } => {
                out.push(0x07);
                out.push(slot.get());
            }
            other => panic!(
                "determinism corpus: unencoded EventPayload variant {other:?}; \
                 assign a tag byte in encode_payload and regenerate goldens",
            ),
        }
    }

    // --- FNV-1a 64-bit ----------------------------------------------
    //
    // No cryptographic claim; the only property consumed is
    // `(input bytes) -> stable u64`. No crate dependency; well-known
    // algorithm; hex-rendered in golden constants for
    // review-friendly git-blame on drift.

    const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

    fn fnv1a_64(bytes: &[u8]) -> u64 {
        let mut hash = FNV_OFFSET;
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    fn hash_event_stream(events: &[Event]) -> u64 {
        let mut buf = Vec::with_capacity(events.len() * 16);
        buf.extend_from_slice(&(events.len() as u64).to_be_bytes());
        for event in events {
            encode_event(event, &mut buf);
        }
        fnv1a_64(&buf)
    }

    // --- Seeded deterministic command sampler -----------------------
    //
    // Same Knuth MMIX LCG + variant-mix used by the T1 drift pins in
    // `command_bus::drift_pins` so corpus goldens compose with the
    // existing single-producer invariant that module already
    // established.

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

    // --- ClosedChannel test double ----------------------------------
    //
    // Mirrors the T1 `ClosedChannel` double defined crate-privately
    // in `command_bus::drift_pins`; integration tests keep their own
    // copy.

    struct ClosedChannel;

    impl CommandChannel for ClosedChannel {
        fn send(&self, _command: Command) -> Result<(), CommandBusError> {
            Err(CommandBusError::Closed)
        }
        fn try_recv(&self) -> Option<Command> {
            None
        }
    }

    // --- Shared replay helpers --------------------------------------

    fn replay<F, T>(f: F) -> Vec<T>
    where
        F: Fn() -> T,
    {
        (0..TRIALS).map(|_| f()).collect()
    }

    fn assert_all_identical<T: PartialEq + std::fmt::Debug>(name: &str, trials: &[T]) {
        for (idx, trial) in trials.iter().enumerate().skip(1) {
            assert_eq!(
                trial, &trials[0],
                "fixture {name}: trial {idx} diverged from trial 0",
            );
        }
    }

    // --- Corpus fixtures --------------------------------------------

    #[test]
    fn seeded_single_producer_50_cmd() {
        const NAME: &str = "seeded-single-producer-50";
        const SEED: u64 = 0x0C1D_5EED;
        const COUNT: u32 = 50;
        const EXPECTED_LEN: usize = 50;
        const GOLDEN_HASH: u64 = 0xD920_9A03_881F_3F36;

        let trials = replay(|| {
            let bus = InMemoryBus::in_memory();
            let mut lcg = Lcg::new(SEED);
            for _ in 0..COUNT {
                bus.dispatch(sample_command(&mut lcg)).unwrap();
            }
            bus.commit_all()
        });
        assert_all_identical(NAME, &trials);
        assert_eq!(trials[0].len(), EXPECTED_LEN, "{NAME}");
        let hash = hash_event_stream(&trials[0]);
        assert_eq!(hash, GOLDEN_HASH, "{NAME} golden drift: got {hash:#018x}");
    }

    #[test]
    fn seeded_single_producer_256_cmd() {
        const NAME: &str = "seeded-single-producer-256";
        const SEED: u64 = 0x0C1D_5EED;
        const COUNT: u32 = 256;
        const EXPECTED_LEN: usize = 256;
        const GOLDEN_HASH: u64 = 0x94BE_433C_49F5_53A0;

        let trials = replay(|| {
            let bus = InMemoryBus::in_memory();
            let mut lcg = Lcg::new(SEED);
            for _ in 0..COUNT {
                bus.dispatch(sample_command(&mut lcg)).unwrap();
            }
            bus.commit_all()
        });
        assert_all_identical(NAME, &trials);
        assert_eq!(trials[0].len(), EXPECTED_LEN, "{NAME}");
        let hash = hash_event_stream(&trials[0]);
        assert_eq!(hash, GOLDEN_HASH, "{NAME} golden drift: got {hash:#018x}");
    }

    #[test]
    fn multi_producer_per_producer_fifo() {
        const NAME: &str = "multi-producer-per-producer-fifo";
        const PRODUCERS: u32 = 4;
        const PER_PRODUCER: u32 = 64;
        const TOTAL: usize = (PRODUCERS * PER_PRODUCER) as usize;
        const GOLDEN_HASH: u64 = 0x2B67_411F_BA87_DBDC;

        // Refinement 1: per-producer FIFO is asserted; cross-producer
        // global interleaving is scheduler-dependent at slice-2
        // (crossbeam-channel guarantees per-producer FIFO only) and
        // is NOT pinned here. The canonical form used for the golden
        // hash is the events sorted by `ControlId` (unique across
        // producers) with `EventSeq` renumbered 0..N so the golden
        // is stable despite the nondeterministic global interleaving.
        let trials = replay(|| {
            let bus = Arc::new(InMemoryBus::in_memory());
            let handles: Vec<_> = (0..PRODUCERS)
                .map(|tid| {
                    let bus = Arc::clone(&bus);
                    thread::spawn(move || {
                        for seq in 0..PER_PRODUCER {
                            let cmd = Command::ControlSet {
                                id: ControlId(tid * PER_PRODUCER + seq),
                                value: ControlValue(seq as f32),
                            };
                            bus.dispatch(cmd).unwrap();
                        }
                    })
                })
                .collect();
            for h in handles {
                h.join().unwrap();
            }
            let events = bus.commit_all();
            assert_per_producer_fifo(&events, PRODUCERS, PER_PRODUCER);
            canonicalize_multi_producer(events)
        });
        assert_all_identical(NAME, &trials);
        assert_eq!(trials[0].len(), TOTAL, "{NAME}");
        let hash = hash_event_stream(&trials[0]);
        assert_eq!(hash, GOLDEN_HASH, "{NAME} golden drift: got {hash:#018x}");
    }

    fn assert_per_producer_fifo(events: &[Event], producers: u32, per_producer: u32) {
        use std::collections::HashMap;
        let mut per_thread: HashMap<u32, Vec<u32>> = HashMap::new();
        for event in events {
            match &event.payload {
                EventPayload::ControlChanged { id, .. } => {
                    let tid = id.0 / per_producer;
                    let seq = id.0 % per_producer;
                    per_thread.entry(tid).or_default().push(seq);
                }
                other => panic!("multi-producer fixture saw unexpected payload {other:?}"),
            }
        }
        assert_eq!(
            per_thread.len(),
            producers as usize,
            "expected {producers} producers in event stream",
        );
        let expected: Vec<u32> = (0..per_producer).collect();
        for (tid, observed) in per_thread {
            assert_eq!(
                observed, expected,
                "producer {tid} violated per-producer FIFO",
            );
        }
    }

    fn canonicalize_multi_producer(mut events: Vec<Event>) -> Vec<Event> {
        events.sort_by_key(|e| match &e.payload {
            EventPayload::ControlChanged { id, .. } => id.0,
            _ => unreachable!("multi-producer fixture only dispatches ControlSet"),
        });
        for (idx, event) in events.iter_mut().enumerate() {
            event.seq = EventSeq(idx as u64);
        }
        events
    }

    #[test]
    fn closed_channel_rejects_all_dispatches() {
        const NAME: &str = "closed-channel-rejection";
        const DISPATCH_COUNT: usize = 4;
        const GOLDEN_HASH: u64 = 0x189B_3EB6_9EFD_E68A;

        // Refinement 2: assert the specific `CommandBusError::Closed`
        // variant, not `Err(_)`.
        let trials = replay(|| {
            let bus = InMemoryBus::with_channel(Box::new(ClosedChannel));
            let mut results: Vec<Result<(), CommandBusError>> = Vec::with_capacity(DISPATCH_COUNT);
            for i in 0..DISPATCH_COUNT as u32 {
                results.push(bus.dispatch(Command::PresetRecall { id: PresetId(i) }));
            }
            let events = bus.commit_all();
            (results, events)
        });
        assert_all_identical(NAME, &trials);
        let (results, events) = &trials[0];
        assert_eq!(results.len(), DISPATCH_COUNT);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(
                *result,
                Err(CommandBusError::Closed),
                "{NAME}: dispatch {i} returned {result:?}, want Err(Closed)",
            );
        }
        assert!(events.is_empty(), "{NAME}: closed channel emitted events");

        let mut buf = Vec::new();
        buf.extend_from_slice(&(results.len() as u64).to_be_bytes());
        for r in results {
            buf.push(match r {
                Err(CommandBusError::Closed) => 0x01,
                Ok(()) => 0x00,
                Err(other) => unreachable!(
                    "closed-channel fixture saw non-Closed error {other:?} despite \
                     assertion above",
                ),
            });
        }
        buf.extend_from_slice(&hash_event_stream(events).to_be_bytes());
        let hash = fnv1a_64(&buf);
        assert_eq!(hash, GOLDEN_HASH, "{NAME} golden drift: got {hash:#018x}");
    }

    #[test]
    fn empty_bus_commits_nothing() {
        const NAME: &str = "empty-bus-quiescence";
        const GOLDEN_HASH: u64 = 0xA8C7_F832_281A_39C5;

        // Refinement 2 note: the slice-2 public `CommandChannel`
        // trait collapses the underlying crossbeam
        // `TryRecvError::Empty` and `Disconnected` to
        // `Option<Command>::None`. This fixture pins the empty-case
        // path specifically — a bus constructed with an open channel
        // and zero dispatches returns `None` from `commit_one` and
        // an empty `Vec<Event>` from `commit_all`. The literal
        // `TryRecvError::Empty` assertion belongs with the slice-3
        // `CommandChannel` surface that distinguishes empty vs.
        // disconnected explicitly; this fixture is the anchor that
        // the slice-3 refinement will upgrade.
        let trials = replay(|| {
            let bus = InMemoryBus::in_memory();
            let first = bus.commit_one();
            let second = bus.commit_one();
            let rest = bus.commit_all();
            (first.is_none(), second.is_none(), rest)
        });
        assert_all_identical(NAME, &trials);
        let (first_none, second_none, rest) = &trials[0];
        assert!(*first_none, "{NAME}: commit_one on fresh bus returned Some");
        assert!(*second_none, "{NAME}: re-commit on empty bus returned Some");
        assert!(
            rest.is_empty(),
            "{NAME}: commit_all on empty bus returned events",
        );
        let hash = hash_event_stream(rest);
        assert_eq!(hash, GOLDEN_HASH, "{NAME} golden drift: got {hash:#018x}");
    }

    #[test]
    fn full_payload_mirror_round_trip() {
        const NAME: &str = "full-payload-mirror";
        const EXPECTED_LEN: usize = 7;
        const GOLDEN_HASH: u64 = 0x1395_DCA7_C173_F216;

        // Exercises all seven `Command` variants plus both
        // `MappingSource` tags in a fixed single-producer order.
        // Content is chosen to populate distinct tag bytes and
        // distinct non-zero ids / slots so the canonical encoding
        // varies across every byte of the 7-event stream.
        let commands: Vec<Command> = vec![
            Command::ControlSet {
                id: ControlId(7),
                value: ControlValue(0.5),
            },
            Command::PresetRecall { id: PresetId(3) },
            Command::PlaylistAdvance { step: -2 },
            Command::MappingBind {
                id: MappingId(1),
                source: MappingSource::Midi,
            },
            Command::SnapRecall {
                slot: SnapSlotId::new(2).expect("valid snap slot"),
            },
            Command::SnapSave {
                slot: SnapSlotId::new(11).expect("valid snap slot"),
            },
            Command::SnapClear {
                slot: SnapSlotId::new(15).expect("valid snap slot"),
            },
        ];

        let trials = replay(|| {
            let bus = InMemoryBus::in_memory();
            for cmd in &commands {
                bus.dispatch(cmd.clone()).unwrap();
            }
            bus.commit_all()
        });
        assert_all_identical(NAME, &trials);
        assert_eq!(trials[0].len(), EXPECTED_LEN);
        let hash = hash_event_stream(&trials[0]);
        assert_eq!(hash, GOLDEN_HASH, "{NAME} golden drift: got {hash:#018x}");
    }
}
