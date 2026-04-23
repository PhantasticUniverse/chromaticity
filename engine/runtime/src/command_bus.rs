// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Command bus, command / event envelopes, and the in-memory
//! channel-backed implementation Phase-1 slice-2 ships.
//!
//! Per ADR-001 (runtime model) every external and internal state
//! update becomes a typed [`Command`] dispatched across the
//! producer / commit-thread seam before the commit thread applies it
//! to the canonical state. Per ADR-006 (thread and process model)
//! producers run on the scheduler, audio-analysis, and integration IO
//! threads; commit runs on the state-commit thread; this module is
//! the seam.
//!
//! Slice-2 ships:
//!
//! - The [`Command`] and [`Event`] envelope enums covering the
//!   Phase-1 command taxonomy per
//!   `docs/04-runtime-architecture.md` §Command envelope and ADR-004
//!   §Decision (control-set, preset-recall, playlist-advance,
//!   mapping-bind).
//! - The [`CommandChannel`] pluggable transport trait and its
//!   [`CrossbeamChannel`] in-memory implementation backed by
//!   `crossbeam-channel 0.5`. The trait seam exists so slice-3
//!   persistence work can substitute a disk-backed or
//!   reconciliation-layer implementation without touching producer
//!   call sites.
//! - The [`InMemoryBus`] `CommandBus` implementation composing a
//!   channel with a monotonic [`EventSeq`] counter that pins the
//!   slice-2 EV-DETERMINISM T1 invariants.
//!
//! Slice-2 does NOT ship:
//!
//! - The validator path (commands are accepted as handed; no pre-
//!   commit rejection).
//! - Store reconciliation (events are a 1:1 map of their source
//!   commands; `State` stores are placeholder data-shape declarations
//!   only).
//! - A cross-process or persistence-backed channel implementation.
//! - Full replay determinism across multi-producer dispatch
//!   (crossbeam-channel guarantees per-producer FIFO, not a globally
//!   deterministic interleaving). The event-sourced reconciliation
//!   layer in slice-3 lands the full replay semantics.
//!
//! Determinism guarantees (slice-2, EV-DETERMINISM T1 pins):
//!
//! - Per-producer FIFO is preserved through commit. A single producer
//!   dispatching commands in order `A`, `B`, `C` observes events in
//!   the same order when the commit thread drains the bus.
//! - For single-producer dispatch, the emitted event sequence —
//!   including every assigned [`EventSeq`] value — is a pure function
//!   of the input command sequence.

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Stable identifier for a control.
///
/// First-slice shape: a transparent `u32` newtype that reserves the
/// namespace without committing to the concrete control-identifier
/// scheme (hierarchical path, opaque handle, or both). The control
/// store landing in slice-3 decides the canonical form; current
/// payloads round-trip through the bus without inspection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ControlId(pub u32);

/// Stable scalar control value.
///
/// Slice-2 pins the scalar `f32` slot so command and event envelopes
/// have a single payload shape to carry. Slice-3 may widen to a sum
/// type covering the typed control kinds (slider, toggle, color, xy)
/// that `docs/28-implementation-contract-pack.md` CP-01 defines.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ControlValue(pub f32);

/// Stable identifier for a preset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PresetId(pub u32);

/// Stable identifier for a mapping.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MappingId(pub u32);

/// Integration source a mapping binds to.
///
/// Slice-2 declares the two integration kinds ADR-013 names; the
/// route specifics live in each adapter crate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MappingSource {
    Osc,
    Midi,
}

/// Bus-assigned monotonic sequence number for a committed [`Event`].
///
/// Strictly increasing across the lifetime of an [`InMemoryBus`] and
/// pinned by the slice-2 EV-DETERMINISM T1 tests: for a single-
/// producer dispatch sequence the assigned `EventSeq` values are
/// `0, 1, 2, …` in commit order and identical across runs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventSeq(pub u64);

/// Typed command flowing across the producer -> commit-thread seam.
///
/// Variant set matches `docs/04-runtime-architecture.md` §Command
/// envelope and ADR-004 §Decision at the slice-2 scope. Slice-3 will
/// grow the validator path that rejects ill-formed commands before
/// commit; the slice-2 bus accepts every command it is handed.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Command {
    /// Set a control to a specific value.
    ControlSet { id: ControlId, value: ControlValue },
    /// Recall a preset by identifier.
    PresetRecall { id: PresetId },
    /// Advance the active playlist by `step` entries; negative values
    /// move backward.
    PlaylistAdvance { step: i32 },
    /// Bind a mapping from an integration source to a control.
    MappingBind {
        id: MappingId,
        source: MappingSource,
    },
}

/// Event payload mirrored 1:1 from a committed [`Command`].
///
/// Slice-2 emits one event per committed command; slice-3 introduces
/// validator-driven rejection (commands that fail validation emit a
/// rejection event instead) and store-reconciliation side effects.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EventPayload {
    ControlChanged {
        id: ControlId,
        new_value: ControlValue,
    },
    PresetRecalled {
        id: PresetId,
    },
    PlaylistAdvanced {
        step: i32,
    },
    MappingBound {
        id: MappingId,
        source: MappingSource,
    },
}

impl EventPayload {
    fn from_command(command: Command) -> Self {
        match command {
            Command::ControlSet { id, value } => Self::ControlChanged {
                id,
                new_value: value,
            },
            Command::PresetRecall { id } => Self::PresetRecalled { id },
            Command::PlaylistAdvance { step } => Self::PlaylistAdvanced { step },
            Command::MappingBind { id, source } => Self::MappingBound { id, source },
        }
    }
}

/// Committed event with its bus-assigned monotonic sequence number.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub seq: EventSeq,
    pub payload: EventPayload,
}

/// Errors returned from [`CommandBus::dispatch`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandBusError {
    /// The bus has been closed and no longer accepts dispatches.
    Closed,
}

impl fmt::Display for CommandBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, "command bus is closed"),
        }
    }
}

impl std::error::Error for CommandBusError {}

/// Seam between producer subsystems and the state commit thread.
///
/// Implementations MUST uphold ADR-006's thread-affinity rule:
/// `dispatch` may be called from any producer thread, but the commit
/// that applies the command to the canonical state runs on the
/// state-commit thread.
pub trait CommandBus: Send + Sync {
    /// Enqueue `command` for commit. Returns once the bus has
    /// accepted the command; actual commit runs asynchronously on the
    /// commit thread.
    fn dispatch(&self, command: Command) -> Result<(), CommandBusError>;
}

/// Pluggable transport for commands between producer and commit.
///
/// Object-safe so implementations can be substituted at runtime via
/// `Box<dyn CommandChannel>`. Slice-2 ships a single implementation
/// ([`CrossbeamChannel`]); slice-3 persistence work introduces a
/// disk-backed implementation against the same trait without
/// touching producer call sites.
pub trait CommandChannel: Send + Sync {
    /// Send `command` across the channel. Returns
    /// [`CommandBusError::Closed`] if the channel has been closed.
    fn send(&self, command: Command) -> Result<(), CommandBusError>;

    /// Attempt to receive the next command without blocking. Returns
    /// `None` when the channel is empty or the peer has been dropped.
    fn try_recv(&self) -> Option<Command>;
}

/// Unbounded MPMC in-memory channel backed by `crossbeam-channel`.
pub struct CrossbeamChannel {
    tx: Sender<Command>,
    rx: Receiver<Command>,
}

impl CrossbeamChannel {
    /// Construct a new unbounded channel with paired sender and
    /// receiver held internally.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx }
    }
}

impl Default for CrossbeamChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandChannel for CrossbeamChannel {
    fn send(&self, command: Command) -> Result<(), CommandBusError> {
        self.tx.send(command).map_err(|_| CommandBusError::Closed)
    }

    fn try_recv(&self) -> Option<Command> {
        match self.rx.try_recv() {
            Ok(command) => Some(command),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

/// In-memory command bus composing a [`CommandChannel`] with the
/// monotonic [`EventSeq`] counter that pins the slice-2
/// EV-DETERMINISM T1 invariants.
pub struct InMemoryBus {
    channel: Box<dyn CommandChannel>,
    next_seq: AtomicU64,
}

impl InMemoryBus {
    /// Construct a bus backed by the given channel.
    #[must_use]
    pub fn with_channel(channel: Box<dyn CommandChannel>) -> Self {
        Self {
            channel,
            next_seq: AtomicU64::new(0),
        }
    }

    /// Construct a bus backed by a default [`CrossbeamChannel`].
    #[must_use]
    pub fn in_memory() -> Self {
        Self::with_channel(Box::new(CrossbeamChannel::new()))
    }

    /// Commit the next pending command, assigning it a monotonic
    /// [`EventSeq`]. Returns `None` if the channel is empty.
    pub fn commit_one(&self) -> Option<Event> {
        let command = self.channel.try_recv()?;
        let seq = EventSeq(self.next_seq.fetch_add(1, Ordering::SeqCst));
        let payload = EventPayload::from_command(command);
        Some(Event { seq, payload })
    }

    /// Commit every pending command, returning events in commit
    /// order. The returned `EventSeq` values are strictly increasing.
    #[must_use]
    pub fn commit_all(&self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Some(event) = self.commit_one() {
            events.push(event);
        }
        events
    }
}

impl CommandBus for InMemoryBus {
    fn dispatch(&self, command: Command) -> Result<(), CommandBusError> {
        self.channel.send(command)
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-DETERMINISM (T1) — per-producer FIFO preservation
    //! through commit, pure-function determinism for single-producer
    //! dispatch, strictly-monotonic `EventSeq` assignment,
    //! closed-channel failure-mode propagation, empty-bus quiescence,
    //! and the slice-2 one-to-one `Command` / `EventPayload` variant
    //! mirror.
    //!
    //! T1 per docs/43 §15.5: each test pins a state-machine invariant
    //! the [`InMemoryBus`] must honour for slice-3's event-sourced
    //! reconciliation to be a well-defined function of its inputs.
    //! Invariants are stated in the module-level doc comment above;
    //! Phase-1 exit requires T2 conformance over the full determinism
    //! replay corpus.
    //!
    //! Multi-producer total-order-across-producers is NOT pinned —
    //! crossbeam-channel's per-producer FIFO is what slice-2 ships;
    //! cross-producer interleaving is scheduler-dependent and the
    //! determinism invariant is reached by the event-sourced
    //! reconciliation layer in slice-3.

    use super::{
        Command, CommandBus, CommandBusError, CommandChannel, ControlId, ControlValue,
        EventPayload, InMemoryBus, MappingId, MappingSource, PresetId,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::thread;

    /// Hand-rolled seeded generator used as a deterministic test
    /// input source. Not a cryptographic PRNG; the only property the
    /// determinism invariant consumes is `seed -> identical sequence`.
    /// Coefficients are Knuth's `MMIX` LCG constants.
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
        /// Returns a deterministic `f32` in `[0.0, 1.0]` — bounded so
        /// the `PartialEq` on `ControlValue` never sees NaN.
        fn next_f32_unit(&mut self) -> f32 {
            let n = self.next_u32();
            (n as f32) / (u32::MAX as f32)
        }
    }

    fn sample_command(lcg: &mut Lcg) -> Command {
        let kind = lcg.next_u32() % 4;
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
            _ => Command::MappingBind {
                id: MappingId(id_val),
                source: MappingSource::Osc,
            },
        }
    }

    fn run_seeded(seed: u64, count: u32) -> Vec<EventPayload> {
        let bus = InMemoryBus::in_memory();
        let mut lcg = Lcg::new(seed);
        for _ in 0..count {
            bus.dispatch(sample_command(&mut lcg)).unwrap();
        }
        bus.commit_all().into_iter().map(|e| e.payload).collect()
    }

    #[test]
    fn seeded_input_produces_deterministic_event_stream() {
        let first = run_seeded(0xC1D_5EED_u64, 256);
        let second = run_seeded(0xC1D_5EED_u64, 256);
        assert_eq!(first, second);
        assert_eq!(first.len(), 256);

        // Control: a different seed must produce a different stream
        // (the test would pass trivially if the bus dropped inputs).
        let third = run_seeded(0xDEAD_BEEF_u64, 256);
        assert_ne!(first, third);
    }

    #[test]
    fn multi_producer_preserves_per_producer_fifo() {
        const PRODUCERS: u32 = 4;
        const PER_PRODUCER: u32 = 64;

        let bus = Arc::new(InMemoryBus::in_memory());
        let handles: Vec<_> = (0..PRODUCERS)
            .map(|thread_id| {
                let bus = Arc::clone(&bus);
                thread::spawn(move || {
                    for seq in 0..PER_PRODUCER {
                        let cmd = Command::ControlSet {
                            id: ControlId(thread_id * PER_PRODUCER + seq),
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
        assert_eq!(events.len() as u32, PRODUCERS * PER_PRODUCER);

        // Partition events by producer via the ControlId encoding.
        // Each producer's observed sequence MUST be strictly
        // monotonic per crossbeam-channel's per-producer FIFO
        // guarantee — the exact invariant this test pins.
        let mut per_thread: HashMap<u32, Vec<u32>> = HashMap::new();
        for event in events {
            match event.payload {
                EventPayload::ControlChanged { id, .. } => {
                    let thread_id = id.0 / PER_PRODUCER;
                    let seq = id.0 % PER_PRODUCER;
                    per_thread.entry(thread_id).or_default().push(seq);
                }
                _ => unreachable!("test only dispatches ControlSet"),
            }
        }
        assert_eq!(per_thread.len(), PRODUCERS as usize);
        for (thread_id, observed) in per_thread {
            let expected: Vec<u32> = (0..PER_PRODUCER).collect();
            assert_eq!(
                observed, expected,
                "thread {thread_id} FIFO violated: observed {observed:?}"
            );
        }
    }

    #[test]
    fn event_seq_is_strictly_monotonic_from_zero() {
        let bus = InMemoryBus::in_memory();
        const COUNT: u32 = 32;
        for i in 0..COUNT {
            bus.dispatch(Command::PresetRecall { id: PresetId(i) })
                .unwrap();
        }
        let events = bus.commit_all();
        assert_eq!(events.len() as u32, COUNT);
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event.seq.0, index as u64);
        }
    }

    /// Pre-built `CommandChannel` that reports `Closed` on every
    /// `send` and never has a command available. Used to pin the
    /// closed-channel dispatch contract without racing a real
    /// crossbeam channel drop.
    struct ClosedChannel;

    impl CommandChannel for ClosedChannel {
        fn send(&self, _command: Command) -> Result<(), CommandBusError> {
            Err(CommandBusError::Closed)
        }

        fn try_recv(&self) -> Option<Command> {
            None
        }
    }

    #[test]
    fn dispatch_on_closed_channel_returns_closed_error() {
        let bus = InMemoryBus::with_channel(Box::new(ClosedChannel));
        let result = bus.dispatch(Command::PresetRecall { id: PresetId(0) });
        assert_eq!(result, Err(CommandBusError::Closed));
    }

    #[test]
    fn commit_on_empty_bus_yields_no_events() {
        let bus = InMemoryBus::in_memory();
        assert!(bus.commit_one().is_none());
        assert!(bus.commit_all().is_empty());
    }

    #[test]
    fn event_payload_mirrors_command_variants_one_to_one() {
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::ControlSet {
            id: ControlId(7),
            value: ControlValue(0.5),
        })
        .unwrap();
        bus.dispatch(Command::PresetRecall { id: PresetId(3) })
            .unwrap();
        bus.dispatch(Command::PlaylistAdvance { step: -2 }).unwrap();
        bus.dispatch(Command::MappingBind {
            id: MappingId(1),
            source: MappingSource::Midi,
        })
        .unwrap();

        let payloads: Vec<EventPayload> = bus.commit_all().into_iter().map(|e| e.payload).collect();
        assert_eq!(
            payloads,
            vec![
                EventPayload::ControlChanged {
                    id: ControlId(7),
                    new_value: ControlValue(0.5),
                },
                EventPayload::PresetRecalled { id: PresetId(3) },
                EventPayload::PlaylistAdvanced { step: -2 },
                EventPayload::MappingBound {
                    id: MappingId(1),
                    source: MappingSource::Midi,
                },
            ]
        );
    }
}
