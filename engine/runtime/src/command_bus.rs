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
//!   mapping-bind, and Snap-slot actions).
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
//! - Persistent or cross-process store reconciliation. Slice-3a adds
//!   the in-memory `State` mutation helper used by tests and the future
//!   commit thread.
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

use crate::state::State;
use chromaticity_observability::catalog::{CommandKind, EventKind, RuntimeLifecycleState};
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

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

/// Stable identifier for a Snap / favslot slot.
///
/// The compatibility surface exposes exactly sixteen slots (`0..=15`).
/// The tuple payload stays private so callers construct only valid slots
/// through [`Self::new`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SnapSlotId(u8);

impl SnapSlotId {
    /// Highest valid Snap slot index.
    pub const MAX: u8 = 15;

    /// Construct a Snap slot id when `slot <= 15`.
    #[must_use]
    pub fn new(slot: u8) -> Option<Self> {
        if slot <= Self::MAX {
            Some(Self(slot))
        } else {
            None
        }
    }

    /// Return the zero-based slot index.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

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
    /// Recall a Snap slot for the active scene.
    SnapRecall { slot: SnapSlotId },
    /// Save current state into a Snap slot.
    SnapSave { slot: SnapSlotId },
    /// Clear a Snap slot.
    SnapClear { slot: SnapSlotId },
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
    SnapRecalled {
        slot: SnapSlotId,
    },
    SnapSaved {
        slot: SnapSlotId,
    },
    SnapCleared {
        slot: SnapSlotId,
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
            Command::SnapRecall { slot } => Self::SnapRecalled { slot },
            Command::SnapSave { slot } => Self::SnapSaved { slot },
            Command::SnapClear { slot } => Self::SnapCleared { slot },
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
///
/// Per-kind command-dispatch, event-emit, and lifecycle-transition
/// counters are maintained in-memory alongside a monotonic
/// `start: Instant` for uptime sampling, and exposed via
/// [`InMemoryBus::command_dispatched_count`],
/// [`InMemoryBus::event_emitted_count`],
/// [`InMemoryBus::lifecycle_transition_count`], and
/// [`InMemoryBus::uptime_seconds`]. These are the interim observables
/// for the `chromaticity_runtime_commands_dispatched_total`,
/// `chromaticity_runtime_events_emitted_total`,
/// `chromaticity_runtime_lifecycle_transitions_total`, and
/// `chromaticity_runtime_uptime_seconds` emission sites declared in
/// `docs/31-supportability-privacy-ops-policy.md` §Phase-1 baseline
/// catalog. Slice-3 will route each increment through the runtime bus
/// via
/// `chromaticity_observability::catalog::MetricEvent::RuntimeCommandsDispatched`,
/// `RuntimeEventsEmitted`, `RuntimeLifecycleTransition`, and
/// `RuntimeUptime`, and reduce this bus-local table to a sink-side
/// counter (per `engine/observability/src/lib.rs` lines 26-27 the
/// runtime-bus wiring that feeds `EmitMetric` implementations is
/// slice-3 scope).
///
/// Command / event vocabularies match
/// [`chromaticity_observability::catalog::CommandKind`] and
/// [`chromaticity_observability::catalog::EventKind`]. The runtime
/// command/event taxonomy emits dedicated Snap kinds so observability
/// can distinguish control writes from slot recall/save/clear traffic.
/// The lifecycle counter exposes
/// only `Starting → Running` (at construction) and
/// `Running → Stopping` (in [`Drop`]); further transitions land with
/// the slice-3 scheduler lifecycle.
pub struct InMemoryBus {
    channel: Box<dyn CommandChannel>,
    next_seq: AtomicU64,
    command_counts: CommandKindCounts,
    event_counts: EventKindCounts,
    // `Arc`-wrapped so the drift-pin test for the `Drop` impl can
    // observe the `Running → Stopping` increment after the bus is
    // dropped; the bus itself owns exactly one strong reference and
    // no code outside this module clones it.
    lifecycle_counts: Arc<LifecycleTransitionCounts>,
    start: Instant,
}

/// Per-[`CommandKind`] dispatch counters for [`InMemoryBus`].
///
/// Named `AtomicU64` fields (not a map) so the compiler sees the full
/// key set and external callers cannot depend on the storage shape.
/// `CommandKind` is `#[non_exhaustive]` across the crate boundary, so
/// the getter keeps a wildcard arm for future variants that slice-2
/// has not been taught to produce.
#[derive(Debug, Default)]
struct CommandKindCounts {
    control_set: AtomicU64,
    preset_recall: AtomicU64,
    playlist_advance: AtomicU64,
    snap_recall: AtomicU64,
    snap_save: AtomicU64,
    snap_clear: AtomicU64,
    mapping_bind: AtomicU64,
}

/// Per-[`EventKind`] emission counters for [`InMemoryBus`]. Same
/// storage-shape rationale as [`CommandKindCounts`].
#[derive(Debug, Default)]
struct EventKindCounts {
    control_changed: AtomicU64,
    preset_recalled: AtomicU64,
    playlist_advanced: AtomicU64,
    snap_recalled: AtomicU64,
    snap_saved: AtomicU64,
    snap_cleared: AtomicU64,
    mapping_bound: AtomicU64,
}

/// Lifecycle `(from, to)` transition counters for [`InMemoryBus`].
///
/// Slice-2 only records the two transitions
/// `Starting → Running` and `Running → Stopping`; every other
/// `(from, to)` pair resolves to `0` through
/// [`InMemoryBus::lifecycle_transition_count`]. Additional pairs
/// become wired when the slice-3 scheduler lifecycle lands, per the
/// TODO on `RuntimeLifecycleState` at
/// `engine/observability/src/catalog.rs`.
#[derive(Debug, Default)]
struct LifecycleTransitionCounts {
    starting_to_running: AtomicU64,
    running_to_stopping: AtomicU64,
}

/// Map a slice-2 [`Command`] variant onto
/// [`chromaticity_observability::catalog::CommandKind`].
fn command_kind(command: &Command) -> CommandKind {
    match command {
        Command::ControlSet { .. } => CommandKind::ControlSet,
        Command::PresetRecall { .. } => CommandKind::PresetRecall,
        Command::PlaylistAdvance { .. } => CommandKind::PlaylistAdvance,
        Command::SnapRecall { .. } => CommandKind::SnapRecall,
        Command::SnapSave { .. } => CommandKind::SnapSave,
        Command::SnapClear { .. } => CommandKind::SnapClear,
        Command::MappingBind { .. } => CommandKind::MappingBind,
    }
}

/// Map a slice-2 [`EventPayload`] variant onto
/// [`chromaticity_observability::catalog::EventKind`].
fn event_kind(payload: &EventPayload) -> EventKind {
    match payload {
        EventPayload::ControlChanged { .. } => EventKind::ControlChanged,
        EventPayload::PresetRecalled { .. } => EventKind::PresetRecalled,
        EventPayload::PlaylistAdvanced { .. } => EventKind::PlaylistAdvanced,
        EventPayload::SnapRecalled { .. } => EventKind::SnapRecalled,
        EventPayload::SnapSaved { .. } => EventKind::SnapSaved,
        EventPayload::SnapCleared { .. } => EventKind::SnapCleared,
        EventPayload::MappingBound { .. } => EventKind::MappingBound,
    }
}

impl InMemoryBus {
    /// Construct a bus backed by the given channel.
    ///
    /// Records a `Starting → Running` lifecycle transition at
    /// construction, following the pattern ADR-006 names for the
    /// state-commit thread coming online.
    #[must_use]
    pub fn with_channel(channel: Box<dyn CommandChannel>) -> Self {
        let bus = Self {
            channel,
            next_seq: AtomicU64::new(0),
            command_counts: CommandKindCounts::default(),
            event_counts: EventKindCounts::default(),
            lifecycle_counts: Arc::new(LifecycleTransitionCounts::default()),
            start: Instant::now(),
        };
        bus.record_lifecycle_transition(
            RuntimeLifecycleState::Starting,
            RuntimeLifecycleState::Running,
        );
        bus
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
        self.record_event_emitted(event_kind(&payload));
        Some(Event { seq, payload })
    }

    /// Commit the next pending command and apply the resulting event to
    /// canonical [`State`].
    pub fn commit_one_into(&self, state: &mut State) -> Option<Event> {
        let event = self.commit_one()?;
        state.apply_event(&event);
        Some(event)
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

    /// Commit every pending command into canonical [`State`], returning
    /// the emitted events in commit order.
    pub fn commit_all_into(&self, state: &mut State) -> Vec<Event> {
        let mut events = Vec::new();
        while let Some(event) = self.commit_one_into(state) {
            events.push(event);
        }
        events
    }

    /// Current count of commands mapped to `kind` and dispatched
    /// through this bus since construction.
    ///
    /// The wildcard arm covers future [`CommandKind`] variants —
    /// the enum is `#[non_exhaustive]` across the crate boundary —
    /// which slice-2 has not been taught to produce; any such
    /// variant reads as 0.
    #[must_use]
    pub fn command_dispatched_count(&self, kind: CommandKind) -> u64 {
        match kind {
            CommandKind::ControlSet => self.command_counts.control_set.load(Ordering::Relaxed),
            CommandKind::PresetRecall => self.command_counts.preset_recall.load(Ordering::Relaxed),
            CommandKind::PlaylistAdvance => {
                self.command_counts.playlist_advance.load(Ordering::Relaxed)
            }
            CommandKind::SnapRecall => self.command_counts.snap_recall.load(Ordering::Relaxed),
            CommandKind::SnapSave => self.command_counts.snap_save.load(Ordering::Relaxed),
            CommandKind::SnapClear => self.command_counts.snap_clear.load(Ordering::Relaxed),
            CommandKind::MappingBind => self.command_counts.mapping_bind.load(Ordering::Relaxed),
            _ => 0,
        }
    }

    /// Current count of events of `kind` emitted by
    /// [`InMemoryBus::commit_one`] since construction. Wildcard arm
    /// follows the same `#[non_exhaustive]` rationale as
    /// [`InMemoryBus::command_dispatched_count`].
    #[must_use]
    pub fn event_emitted_count(&self, kind: EventKind) -> u64 {
        match kind {
            EventKind::ControlChanged => self.event_counts.control_changed.load(Ordering::Relaxed),
            EventKind::PresetRecalled => self.event_counts.preset_recalled.load(Ordering::Relaxed),
            EventKind::PlaylistAdvanced => {
                self.event_counts.playlist_advanced.load(Ordering::Relaxed)
            }
            EventKind::SnapRecalled => self.event_counts.snap_recalled.load(Ordering::Relaxed),
            EventKind::SnapSaved => self.event_counts.snap_saved.load(Ordering::Relaxed),
            EventKind::SnapCleared => self.event_counts.snap_cleared.load(Ordering::Relaxed),
            EventKind::MappingBound => self.event_counts.mapping_bound.load(Ordering::Relaxed),
            _ => 0,
        }
    }

    /// Current count of `(from, to)` lifecycle transitions recorded
    /// on this bus. Slice-2 only records
    /// `Starting → Running` (at construction) and
    /// `Running → Stopping` (in [`Drop`]); every other pair reads as
    /// 0.
    #[must_use]
    pub fn lifecycle_transition_count(
        &self,
        from: RuntimeLifecycleState,
        to: RuntimeLifecycleState,
    ) -> u64 {
        match (from, to) {
            (RuntimeLifecycleState::Starting, RuntimeLifecycleState::Running) => self
                .lifecycle_counts
                .starting_to_running
                .load(Ordering::Relaxed),
            (RuntimeLifecycleState::Running, RuntimeLifecycleState::Stopping) => self
                .lifecycle_counts
                .running_to_stopping
                .load(Ordering::Relaxed),
            _ => 0,
        }
    }

    /// Seconds elapsed since this bus was constructed, sampled via
    /// `Instant::elapsed` at call time. Pulled rather than pushed —
    /// no timer thread — so the gauge reads the same value any two
    /// callers in the same second see, and advances monotonically.
    #[must_use]
    pub fn uptime_seconds(&self) -> u64 {
        self.start.elapsed().as_secs()
    }

    /// Increment the counter for `kind` by one. Unknown variants
    /// (future `#[non_exhaustive]` additions to [`CommandKind`]) are
    /// ignored rather than panicked on.
    fn record_command_dispatched(&self, kind: CommandKind) {
        let counter = match kind {
            CommandKind::ControlSet => &self.command_counts.control_set,
            CommandKind::PresetRecall => &self.command_counts.preset_recall,
            CommandKind::PlaylistAdvance => &self.command_counts.playlist_advance,
            CommandKind::SnapRecall => &self.command_counts.snap_recall,
            CommandKind::SnapSave => &self.command_counts.snap_save,
            CommandKind::SnapClear => &self.command_counts.snap_clear,
            CommandKind::MappingBind => &self.command_counts.mapping_bind,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter for `kind` by one. Unknown variants
    /// (future `#[non_exhaustive]` additions to [`EventKind`]) are
    /// ignored rather than panicked on.
    fn record_event_emitted(&self, kind: EventKind) {
        let counter = match kind {
            EventKind::ControlChanged => &self.event_counts.control_changed,
            EventKind::PresetRecalled => &self.event_counts.preset_recalled,
            EventKind::PlaylistAdvanced => &self.event_counts.playlist_advanced,
            EventKind::SnapRecalled => &self.event_counts.snap_recalled,
            EventKind::SnapSaved => &self.event_counts.snap_saved,
            EventKind::SnapCleared => &self.event_counts.snap_cleared,
            EventKind::MappingBound => &self.event_counts.mapping_bound,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter for the `(from, to)` lifecycle pair by
    /// one. Pairs outside the slice-2 recorded set are ignored
    /// rather than panicked on.
    fn record_lifecycle_transition(&self, from: RuntimeLifecycleState, to: RuntimeLifecycleState) {
        let counter = match (from, to) {
            (RuntimeLifecycleState::Starting, RuntimeLifecycleState::Running) => {
                &self.lifecycle_counts.starting_to_running
            }
            (RuntimeLifecycleState::Running, RuntimeLifecycleState::Stopping) => {
                &self.lifecycle_counts.running_to_stopping
            }
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

impl CommandBus for InMemoryBus {
    fn dispatch(&self, command: Command) -> Result<(), CommandBusError> {
        // Map the command to its `CommandKind` at the start of
        // dispatch so slice-3 can lift this straight into a
        // `MetricEvent::RuntimeCommandsDispatched` emission without
        // touching the call site.
        let kind = command_kind(&command);
        self.channel.send(command)?;
        self.record_command_dispatched(kind);
        Ok(())
    }
}

impl Drop for InMemoryBus {
    fn drop(&mut self) {
        // `Running → Stopping` is the one terminal transition the
        // slice-2 runtime emits. Slice-3's scheduler lifecycle will
        // grow additional terminal arcs (`Paused → Stopping`,
        // crash-path transitions) but those land with the concrete
        // scheduler, not this bus.
        self.record_lifecycle_transition(
            RuntimeLifecycleState::Running,
            RuntimeLifecycleState::Stopping,
        );
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
        EventPayload, InMemoryBus, MappingId, MappingSource, PresetId, SnapSlotId,
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
        bus.dispatch(Command::SnapRecall {
            slot: SnapSlotId::new(4).expect("valid snap slot"),
        })
        .unwrap();
        bus.dispatch(Command::SnapSave {
            slot: SnapSlotId::new(5).expect("valid snap slot"),
        })
        .unwrap();
        bus.dispatch(Command::SnapClear {
            slot: SnapSlotId::new(5).expect("valid snap slot"),
        })
        .unwrap();
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
                EventPayload::SnapRecalled {
                    slot: SnapSlotId::new(4).expect("valid snap slot"),
                },
                EventPayload::SnapSaved {
                    slot: SnapSlotId::new(5).expect("valid snap slot"),
                },
                EventPayload::SnapCleared {
                    slot: SnapSlotId::new(5).expect("valid snap slot"),
                },
                EventPayload::MappingBound {
                    id: MappingId(1),
                    source: MappingSource::Midi,
                },
            ]
        );
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod observability_emission {
    //! Evidence: EV-OBSERVABILITY (T1) — runtime emission site against docs/31 §Phase-1 baseline catalog (commands_dispatched, events_emitted, lifecycle_transitions, uptime).
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: each test pins
    //! the slice-2 emission contract for one of the four runtime
    //! metrics the catalog at `engine/observability/src/catalog.rs`
    //! declares. The free-function drift pins in `drift_pins` above
    //! already cover the determinism and channel invariants of
    //! [`InMemoryBus`]; this module is additive and covers the
    //! `dispatch → record_command_dispatched`,
    //! `commit_one → record_event_emitted`,
    //! `with_channel → record_lifecycle_transition(Starting, Running)`,
    //! and `Drop → record_lifecycle_transition(Running, Stopping)`
    //! chains that slice-3 will rebase onto the runtime-bus
    //! `MetricEvent::Runtime*` emissions.
    //!
    //! Coverage is limited to the slice-2 emission set. `RuntimeUptime`
    //! is a sampled gauge rather than a counter — it advances
    //! monotonically across calls (possibly as a no-op inside a single
    //! second); the test pins the no-regress invariant, not a
    //! clock-specific value.
    //!
    //! Dedicated Snap command/event attribution is pinned by
    //! [`snap_variants_increment_dedicated_command_counters`] and
    //! [`snap_events_increment_dedicated_event_counters`].
    //!
    //! The `Running → Stopping` test observes post-`Drop` state via
    //! the `Arc<LifecycleTransitionCounts>` the bus holds internally.
    //! No other module clones the `Arc`; this test is the sole
    //! consumer, and it relies on the same-module access to
    //! `bus.lifecycle_counts` that private-field visibility grants.
    //! Slice-3 replaces this path with a `MetricEvent` emission the
    //! bus-side counter captures directly.

    use super::{
        Command, CommandBus, ControlId, ControlValue, InMemoryBus, LifecycleTransitionCounts,
        MappingId, MappingSource, PresetId, SnapSlotId,
    };
    use chromaticity_observability::catalog::{CommandKind, EventKind, RuntimeLifecycleState};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn control_set_dispatch_increments_control_set_command_counter() {
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::ControlSet {
            id: ControlId(0),
            value: ControlValue(0.0),
        })
        .unwrap();
        assert_eq!(bus.command_dispatched_count(CommandKind::ControlSet), 1);
        assert_eq!(bus.command_dispatched_count(CommandKind::PresetRecall), 0);
        assert_eq!(
            bus.command_dispatched_count(CommandKind::PlaylistAdvance),
            0
        );
        assert_eq!(bus.command_dispatched_count(CommandKind::MappingBind), 0);
    }

    #[test]
    fn preset_recall_dispatch_increments_preset_recall_command_counter() {
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::PresetRecall { id: PresetId(1) })
            .unwrap();
        assert_eq!(bus.command_dispatched_count(CommandKind::PresetRecall), 1);
        assert_eq!(bus.command_dispatched_count(CommandKind::ControlSet), 0);
    }

    #[test]
    fn playlist_advance_dispatch_increments_playlist_advance_command_counter() {
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::PlaylistAdvance { step: 2 }).unwrap();
        bus.dispatch(Command::PlaylistAdvance { step: -1 }).unwrap();
        assert_eq!(
            bus.command_dispatched_count(CommandKind::PlaylistAdvance),
            2
        );
    }

    #[test]
    fn mapping_bind_dispatch_increments_mapping_bind_command_counter() {
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::MappingBind {
            id: MappingId(7),
            source: MappingSource::Osc,
        })
        .unwrap();
        bus.dispatch(Command::MappingBind {
            id: MappingId(8),
            source: MappingSource::Midi,
        })
        .unwrap();
        assert_eq!(bus.command_dispatched_count(CommandKind::MappingBind), 2);
    }

    #[test]
    fn snap_variants_increment_dedicated_command_counters() {
        let bus = InMemoryBus::in_memory();
        let slot = SnapSlotId::new(3).expect("valid snap slot");
        bus.dispatch(Command::SnapRecall { slot }).unwrap();
        bus.dispatch(Command::SnapSave { slot }).unwrap();
        bus.dispatch(Command::SnapClear { slot }).unwrap();
        assert_eq!(bus.command_dispatched_count(CommandKind::SnapRecall), 1);
        assert_eq!(bus.command_dispatched_count(CommandKind::SnapSave), 1);
        assert_eq!(bus.command_dispatched_count(CommandKind::SnapClear), 1);
        assert_eq!(bus.command_dispatched_count(CommandKind::ControlSet), 0);
        assert_eq!(bus.command_dispatched_count(CommandKind::PresetRecall), 0);
        assert_eq!(bus.command_dispatched_count(CommandKind::MappingBind), 0);
    }

    #[test]
    fn commit_one_increments_matching_event_kind_counter() {
        let bus = InMemoryBus::in_memory();
        bus.dispatch(Command::ControlSet {
            id: ControlId(0),
            value: ControlValue(0.0),
        })
        .unwrap();
        bus.dispatch(Command::PresetRecall { id: PresetId(0) })
            .unwrap();
        bus.dispatch(Command::PlaylistAdvance { step: 1 }).unwrap();
        bus.dispatch(Command::MappingBind {
            id: MappingId(1),
            source: MappingSource::Osc,
        })
        .unwrap();

        // Dispatch does NOT advance event counters; only commit does.
        assert_eq!(bus.event_emitted_count(EventKind::ControlChanged), 0);
        assert_eq!(bus.event_emitted_count(EventKind::PresetRecalled), 0);
        assert_eq!(bus.event_emitted_count(EventKind::PlaylistAdvanced), 0);
        assert_eq!(bus.event_emitted_count(EventKind::SnapRecalled), 0);
        assert_eq!(bus.event_emitted_count(EventKind::SnapSaved), 0);
        assert_eq!(bus.event_emitted_count(EventKind::SnapCleared), 0);
        assert_eq!(bus.event_emitted_count(EventKind::MappingBound), 0);

        let events = bus.commit_all();
        assert_eq!(events.len(), 4);
        assert_eq!(bus.event_emitted_count(EventKind::ControlChanged), 1);
        assert_eq!(bus.event_emitted_count(EventKind::PresetRecalled), 1);
        assert_eq!(bus.event_emitted_count(EventKind::PlaylistAdvanced), 1);
        assert_eq!(bus.event_emitted_count(EventKind::MappingBound), 1);
    }

    #[test]
    fn snap_events_increment_dedicated_event_counters() {
        let bus = InMemoryBus::in_memory();
        let slot = SnapSlotId::new(1).expect("valid snap slot");
        bus.dispatch(Command::SnapRecall { slot }).unwrap();
        bus.dispatch(Command::SnapSave { slot }).unwrap();
        bus.dispatch(Command::SnapClear { slot }).unwrap();
        let events = bus.commit_all();
        assert_eq!(events.len(), 3);
        assert_eq!(bus.event_emitted_count(EventKind::SnapRecalled), 1);
        assert_eq!(bus.event_emitted_count(EventKind::SnapSaved), 1);
        assert_eq!(bus.event_emitted_count(EventKind::SnapCleared), 1);
        assert_eq!(bus.event_emitted_count(EventKind::ControlChanged), 0);
        assert_eq!(bus.event_emitted_count(EventKind::PresetRecalled), 0);
    }

    #[test]
    fn bus_construction_records_starting_to_running_lifecycle_transition() {
        let bus = InMemoryBus::in_memory();
        assert_eq!(
            bus.lifecycle_transition_count(
                RuntimeLifecycleState::Starting,
                RuntimeLifecycleState::Running,
            ),
            1
        );
        // Running → Stopping has not fired yet; the bus is still
        // alive.
        assert_eq!(
            bus.lifecycle_transition_count(
                RuntimeLifecycleState::Running,
                RuntimeLifecycleState::Stopping,
            ),
            0
        );
        // Pairs slice-2 does not record read as 0.
        assert_eq!(
            bus.lifecycle_transition_count(
                RuntimeLifecycleState::Running,
                RuntimeLifecycleState::Paused,
            ),
            0
        );
    }

    #[test]
    fn drop_records_running_to_stopping_lifecycle_transition() {
        // Observe the post-drop counter value via an `Arc` clone the
        // test retains. Inside the bus-lifetime block,
        // `running_to_stopping` is still 0. Once the bus is dropped at
        // the end of that block, the `Drop` impl fires and the
        // retained Arc reads 1.
        let retained: Arc<LifecycleTransitionCounts>;
        {
            let bus = InMemoryBus::in_memory();
            retained = Arc::clone(&bus.lifecycle_counts);
            assert_eq!(
                retained.starting_to_running.load(Ordering::Relaxed),
                1,
                "construction should record Starting → Running",
            );
            assert_eq!(
                retained.running_to_stopping.load(Ordering::Relaxed),
                0,
                "bus is still alive; Drop has not run yet",
            );
            // `bus` drops here.
        }
        assert_eq!(
            retained.running_to_stopping.load(Ordering::Relaxed),
            1,
            "Drop impl should record Running → Stopping",
        );
    }

    #[test]
    fn uptime_seconds_is_monotonic() {
        let bus = InMemoryBus::in_memory();
        // First sample bounds the monotonic invariant's baseline.
        let first = bus.uptime_seconds();
        // Two more reads in quick succession must not regress; any
        // forward motion is fine, including staying within the same
        // whole second. A short sleep gives the clock a chance to
        // advance past a second boundary without forcing it to.
        let second = bus.uptime_seconds();
        thread::sleep(Duration::from_millis(5));
        let third = bus.uptime_seconds();
        assert!(second >= first, "uptime regressed: {second} < {first}");
        assert!(third >= second, "uptime regressed: {third} < {second}");
    }

    #[test]
    fn multi_producer_command_counter_is_thread_safe() {
        // Dispatch happens from producer threads; the counter uses
        // `AtomicU64` so independent increments on different threads
        // never lose updates. Pin the total — not the ordering —
        // against a multi-producer fan-in.
        const PRODUCERS: u32 = 4;
        const PER_PRODUCER: u32 = 64;

        let bus = Arc::new(InMemoryBus::in_memory());
        let handles: Vec<_> = (0..PRODUCERS)
            .map(|_| {
                let bus = Arc::clone(&bus);
                thread::spawn(move || {
                    for seq in 0..PER_PRODUCER {
                        bus.dispatch(Command::ControlSet {
                            id: ControlId(seq),
                            value: ControlValue(0.0),
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(
            bus.command_dispatched_count(CommandKind::ControlSet),
            u64::from(PRODUCERS * PER_PRODUCER),
        );
    }
}
