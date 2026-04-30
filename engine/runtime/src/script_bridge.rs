// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: (c) 2026 Xavier Butler

//! Slice-5.3 — engine/script command-bus adapter.
//!
//! Bridges the slice-5.1 [`chromaticity_script::ScriptCommandSender`]
//! producer-side trait onto a runtime-owned bounded
//! `crossbeam-channel` consumer queue. Closes the script lane's
//! EXIT-quality core surface: a [`ScriptRuntime::with_command_sender`](
//! chromaticity_script::ScriptRuntime::with_command_sender)-configured
//! script can emit reactive writes (slice 5.1 string-keyed +
//! slice 5.2 typed-catalog-resolved) and the runtime side drains the
//! consumer to act on them.
//!
//! # Where this lives
//!
//! Three placements were considered in the slice 5.3 scoping:
//!
//! - **engine/runtime-side (chosen).** The adapter lives here behind
//!   the optional `script-bridge` Cargo feature, with engine/runtime
//!   taking an optional `chromaticity-script` dep gated on the same
//!   feature. The runtime is the natural consumer of script-emitted
//!   reactive writes (the runtime owns the canonical state
//!   per ADR-004), and slice 5.1 designed
//!   [`chromaticity_script::ScriptCommandSender`] specifically as a
//!   producer-side trait so the consumer-side adapter could land
//!   without engine/script taking a runtime dep. This placement
//!   honours that direction: engine/runtime imports
//!   `chromaticity_script::*` types; engine/script does NOT import
//!   `chromaticity_runtime::*`. Default-off feature keeps the
//!   runtime crate dep-free for call sites that don't link the
//!   script lane (e.g. headless-render integration tests).
//! - engine/script-side feature-gated module. Would couple engine/
//!   script's "core sandbox" surface to runtime types, breaking the
//!   slice 5.1 invariant that engine/script does NOT depend on
//!   `chromaticity-runtime`.
//! - Shared crate (e.g. `chromaticity-script-bridge`). Adds a
//!   workspace-members edit (CLAUDE.md serial-list) for no concrete
//!   benefit at slice 5.3 scope — neither crate has multiple
//!   consumers of the bridge surface today.
//!
//! engine/runtime-side wins on dep-direction cleanness, on minimal
//! workspace churn, and on natural consumer placement.
//!
//! # The runtime [`Command`] enum is NOT widened
//!
//! Slice 5.3 deliberately routes [`chromaticity_script::ScriptCommand`]
//! values onto a sibling consumer channel rather than translating
//! them into [`crate::Command`] variants. Two reasons:
//!
//! - The Phase-1 [`crate::Command`] vocabulary covers control writes,
//!   preset recall, playlist advance, Snap slot operations, and
//!   mapping bind. Reactive symbol writes (script-emitted `syn_*` or
//!   `chroma_*` value updates per `docs/36` v1.1.0) are a different
//!   semantic axis: they're per-frame data updates, not user-action
//!   commands. Folding them into [`crate::Command`] would conflate
//!   two distinct dispatch paths and force the slice-2 monotonic
//!   `EventSeq` counter to absorb high-frequency frame data.
//! - The host integration that decides whether to translate a
//!   `ScriptCommand::TypedReactiveWrite` into a runtime state
//!   mutation depends on per-symbol semantics (clamping, exponential
//!   curves, beat-aligned phase) governed by `docs/36` per-symbol
//!   columns. Encoding those in [`crate::Command`] would either
//!   leak the reactive-symbol catalog into the runtime crate or
//!   require a generic `SetReactive { symbol, value }` variant the
//!   commit thread could not validate without growing a catalog
//!   dep itself. Slice 5.3 keeps the policy at the call site by
//!   leaving translation to the host (typically a per-frame drain
//!   on the script tick loop).
//!
//! # Surface
//!
//! - [`ScriptCommandRouter`] — implements
//!   [`chromaticity_script::ScriptCommandSender`]. Stores a
//!   `crossbeam_channel::Sender<ScriptCommand>` and a non-blocking
//!   `try_send` that maps a full / disconnected channel to the
//!   typed [`chromaticity_script::ScriptCommandError`] surface.
//! - [`ScriptBridgeReceiver`] — owns the receive half of the bounded
//!   channel. The host drains this on the consumer side (typically
//!   the commit thread's per-frame tick).
//! - [`script_bridge`] — constructor returning a paired
//!   `(ScriptCommandRouter, ScriptBridgeReceiver)`. Mirrors the
//!   `crossbeam_channel::bounded` shape so the producer + consumer
//!   stay paired by construction.
//! - [`ScriptBridgeError`] — receiver-side errors when draining.
//!
//! # Error mapping
//!
//! `crossbeam_channel::TrySendError`:
//! - `Full(_)` →
//!   [`chromaticity_script::ScriptCommandError::Backpressure`]. The
//!   queue depth is bounded; producers are expected to honour the
//!   shed signal (drop or retry per host policy).
//! - `Disconnected(_)` →
//!   [`chromaticity_script::ScriptCommandError::Disconnected`]. The
//!   receive half has been dropped; the host sink is gone.
//!
//! Mirrors the [`crate::CrossbeamChannel`] error mapping so the
//! slice-5.3 adapter and the slice-2 in-memory bus surface the same
//! shape on the same underlying failure modes.
//!
//! # Threading
//!
//! [`ScriptCommandRouter`] is `Send + Sync` (the
//! `crossbeam_channel::Sender<ScriptCommand>` is). The trait it
//! implements is also `Send + Sync` (slice 5.1), so the producer
//! handle the host installs on
//! [`chromaticity_script::ScriptRuntime::with_command_sender`] is a
//! valid `Arc<dyn ScriptCommandSender>` clone-target for any
//! producer thread.
//!
//! Per ADR-006, the script `execute` call always runs on the script
//! runtime thread, but `crossbeam_channel::Sender::try_send` is
//! lock-free MPSC so the actual `send` call does not need a
//! particular thread.
//!
//! # Evidence
//!
//! Widens `docs/26-evidence-artifact-registry.md` row 29
//! (`EV-SCRIPT-CONTRACT`) past slice 5.2's typed-catalog binding
//! with the engine/runtime consumer-side adapter. T1 algorithm-drift
//! pins inline below ([`script_bridge_pins`]); T2 corpus in
//! `engine/runtime/tests/script_bridge.rs`.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model (commands are the inter-thread seam).
//! - ADR-003 — script runtime sandbox (sandbox surface this bridge
//!   crosses; engine choice + budgets superseded by ADR-012).
//! - ADR-004 — canonical state model.
//! - ADR-006 — thread + process model.
//! - ADR-012 — script sandbox engine and budgets.

use std::fmt;

use chromaticity_script::{ScriptCommand, ScriptCommandError, ScriptCommandSender};
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};

/// Producer-side adapter implementing
/// [`chromaticity_script::ScriptCommandSender`] over a runtime-owned
/// bounded `crossbeam-channel`.
///
/// Construct via [`script_bridge`] which returns a paired
/// `(ScriptCommandRouter, ScriptBridgeReceiver)`.
///
/// # Cloning
///
/// `Clone` is provided so the host can hand multiple
/// [`std::sync::Arc<dyn ScriptCommandSender>`] clones to different
/// scripts; clones share the same underlying channel send half.
/// Mirrors `crossbeam_channel::Sender<T>: Clone` semantics.
#[derive(Clone)]
pub struct ScriptCommandRouter {
    tx: Sender<ScriptCommand>,
}

impl fmt::Debug for ScriptCommandRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The `crossbeam_channel::Sender` itself is opaque; expose
        // the queue length as the only structurally-useful field a
        // diagnostic caller can act on.
        f.debug_struct("ScriptCommandRouter")
            .field("queued", &self.tx.len())
            .finish()
    }
}

impl ScriptCommandSender for ScriptCommandRouter {
    fn send(&self, command: ScriptCommand) -> Result<(), ScriptCommandError> {
        // Non-blocking try_send so a full queue surfaces as the
        // typed `Backpressure` rather than blocking the script
        // runtime thread. Mirrors the slice-2
        // `crate::CrossbeamChannel::send` shape: the only structural
        // difference is the source / destination type.
        self.tx.try_send(command).map_err(|err| match err {
            TrySendError::Full(_) => ScriptCommandError::Backpressure,
            TrySendError::Disconnected(_) => ScriptCommandError::Disconnected,
        })
    }
}

/// Consumer-side handle the host drains.
///
/// Holds the receive half of the bounded channel paired with
/// [`ScriptCommandRouter`]. The host typically owns one of these on
/// the commit thread and drains it once per frame via [`Self::try_recv`]
/// or [`Self::drain`].
///
/// Not `Clone` — `crossbeam_channel::Receiver<T>` is `Clone` (MPMC)
/// but the slice 5.3 design assumes a single consumer per channel
/// (host owns the drain). If multi-consumer fan-out is needed in a
/// later slice, expose [`Self::receiver`] (currently crate-private)
/// or relax the bound.
pub struct ScriptBridgeReceiver {
    rx: Receiver<ScriptCommand>,
}

impl fmt::Debug for ScriptBridgeReceiver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptBridgeReceiver")
            .field("queued", &self.rx.len())
            .finish()
    }
}

impl ScriptBridgeReceiver {
    /// Try to receive the next pending [`ScriptCommand`] without
    /// blocking.
    ///
    /// Returns:
    /// - `Ok(Some(cmd))` — a command was waiting.
    /// - `Ok(None)` — no command pending; queue is empty but the
    ///   send half is still alive.
    /// - `Err(ScriptBridgeError::Disconnected)` — the send half
    ///   has been dropped and no further commands can arrive.
    pub fn try_recv(&self) -> Result<Option<ScriptCommand>, ScriptBridgeError> {
        match self.rx.try_recv() {
            Ok(cmd) => Ok(Some(cmd)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(ScriptBridgeError::Disconnected),
        }
    }

    /// Drain every pending [`ScriptCommand`] in FIFO order.
    ///
    /// Stops when the queue is empty OR the send half has been
    /// dropped; in either case the returned `Vec` contains every
    /// command that was queued. Use [`Self::try_recv`] when the
    /// caller needs to distinguish "empty" from "disconnected".
    #[must_use]
    pub fn drain(&self) -> Vec<ScriptCommand> {
        let mut out = Vec::new();
        // `while let` rather than `loop+match` per clippy
        // `while_let_loop`: both `Empty` and `Disconnected` arms
        // terminate, so any non-`Ok` collapses to loop exit. The
        // alive-vs-disconnected distinction is the
        // [`Self::try_recv`] caller's concern.
        while let Ok(cmd) = self.rx.try_recv() {
            out.push(cmd);
        }
        out
    }

    /// Current queued-command count.
    ///
    /// Sampled via `crossbeam_channel::Receiver::len`. Useful for
    /// per-frame backpressure observability (compare against
    /// [`script_bridge`]'s `capacity` argument). Does not snapshot
    /// across threads — the value may change between this call and
    /// the next [`Self::try_recv`].
    #[must_use]
    pub fn queued(&self) -> usize {
        self.rx.len()
    }
}

/// Receiver-side errors for [`ScriptBridgeReceiver::try_recv`].
///
/// `#[non_exhaustive]` so future widening (e.g. timeout-aware drain)
/// stays additive on the public surface. Mirrors the `Disconnected`
/// arm of [`chromaticity_script::ScriptCommandError`] for symmetry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScriptBridgeError {
    /// The send half ([`ScriptCommandRouter`]) has been dropped and
    /// no further commands can arrive.
    Disconnected,
}

impl fmt::Display for ScriptBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "script bridge sender disconnected"),
        }
    }
}

impl std::error::Error for ScriptBridgeError {}

/// Construct a paired [`ScriptCommandRouter`] /
/// [`ScriptBridgeReceiver`] over a bounded `crossbeam-channel` of
/// the given `capacity`.
///
/// Panics if `capacity == 0`. Bounded `0` would mean a rendezvous
/// channel — both sides must synchronize on every send — which the
/// slice 5.3 producer-side cannot honour because the script
/// runtime thread is `!Send` and cannot block on a runtime-thread
/// drain. Callers MUST pass `capacity >= 1`; the typical Phase-2
/// budget is the same `1024` the slice-2
/// [`crate::CrossbeamChannel::CAPACITY`] uses for the runtime bus.
///
/// # Cross-crate dep direction
///
/// This is the only construction path. Engine/script callers
/// install the returned router via
/// [`chromaticity_script::ScriptRuntime::with_command_sender`] —
/// engine/script imports the slice-5.1
/// [`chromaticity_script::ScriptCommandSender`] trait, NOT this
/// module's concrete type, which keeps engine/script independent of
/// runtime types as slice 5.1 designed.
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
/// use chromaticity_runtime::script_bridge;
/// use chromaticity_script::ScriptRuntime;
///
/// let (router, drain) = script_bridge(1024);
/// let runtime = ScriptRuntime::new()?
///     .with_command_sender(Arc::new(router))?;
/// // ... runtime.execute("__chromaticityEmit('syn_BPM', 120);") ...
/// // Host-side per-frame drain:
/// for cmd in drain.drain() {
///     // route the command per host policy.
/// }
/// ```
#[must_use]
pub fn script_bridge(capacity: usize) -> (ScriptCommandRouter, ScriptBridgeReceiver) {
    assert!(
        capacity > 0,
        "script_bridge: capacity must be >= 1 (rendezvous channels not supported)"
    );
    let (tx, rx) = bounded(capacity);
    (ScriptCommandRouter { tx }, ScriptBridgeReceiver { rx })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod script_bridge_pins {
    //! Evidence: EV-SCRIPT-CONTRACT (T1) — slice-5.3 engine/runtime
    //! adapter data-shape contract.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! [`ScriptCommandRouter`] / [`ScriptBridgeReceiver`] data-shape
    //! contract that the host
    //! [`chromaticity_script::ScriptRuntime::with_command_sender`]
    //! integration depends on. End-to-end T2 corpus (running a
    //! `ScriptRuntime` against the router) lives in
    //! `engine/runtime/tests/script_bridge.rs`.
    //!
    //! These pins do NOT exercise QuickJS — they only assert the
    //! Rust-side surface: error mapping (Full → Backpressure,
    //! Disconnected → Disconnected), sender trait Send+Sync,
    //! consumer drain semantics, paired-construction.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 29 —
    //!   `EV-SCRIPT-CONTRACT`.
    //! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

    use super::{
        ScriptBridgeError, ScriptBridgeReceiver, ScriptCommandRouter, ScriptCommandSender,
        script_bridge,
    };
    use chromaticity_script::{ReactiveSymbol, ScriptCommand, ScriptCommandError, ScriptValue};
    use std::sync::Arc;

    /// A minimal valid [`ScriptCommand`] for tests that only need
    /// "some command flows through".
    fn sample_command() -> ScriptCommand {
        ScriptCommand::ReactiveWrite {
            name: "syn_BPM".to_string(),
            value: ScriptValue::Number(120.0),
        }
    }

    /// A typed [`ScriptCommand::TypedReactiveWrite`] for pins that
    /// exercise the slice-5.2 typed variant traversal through the
    /// bridge.
    fn typed_sample_command() -> ScriptCommand {
        ScriptCommand::TypedReactiveWrite {
            symbol: ReactiveSymbol::Bpm,
            value: ScriptValue::Number(120.0),
        }
    }

    #[test]
    fn paired_construction_round_trips_a_command() {
        let (router, drain) = script_bridge(8);
        router.send(sample_command()).expect("router accepts");
        let recv = drain.try_recv().expect("queue alive").expect("had cmd");
        assert_eq!(recv, sample_command());
        // Queue is now drained; next try_recv is Ok(None).
        assert_eq!(drain.try_recv(), Ok(None));
    }

    #[test]
    fn typed_reactive_write_passes_through_unchanged() {
        // Drift pin: the bridge does NOT inspect the
        // [`ScriptCommand`] variant. A slice-5.2
        // `TypedReactiveWrite` flows through identically to a
        // slice-5.1 `ReactiveWrite`. The host-side drain receives
        // whatever the producer routed; per-symbol policy stays at
        // the call site, not in the bridge.
        let (router, drain) = script_bridge(4);
        let cmd = typed_sample_command();
        router.send(cmd.clone()).unwrap();
        let recv = drain.try_recv().unwrap().unwrap();
        assert_eq!(recv, cmd);
    }

    #[test]
    fn full_queue_returns_backpressure() {
        // Drift pin: a full bounded queue surfaces
        // `ScriptCommandError::Backpressure`, NOT `Disconnected`.
        // Mirrors the slice-2 CrossbeamChannel mapping shape; if
        // these diverge the script runtime's post-execute
        // canonical-message remap stops recognising the typed
        // failure mode.
        let (router, _drain) = script_bridge(2);
        router.send(sample_command()).unwrap();
        router.send(sample_command()).unwrap();
        let err = router
            .send(sample_command())
            .expect_err("third send must fail on cap=2");
        assert_eq!(err, ScriptCommandError::Backpressure);
    }

    #[test]
    fn disconnected_drain_returns_disconnected() {
        // Drift pin: dropping the receiver MUST surface as
        // `ScriptCommandError::Disconnected` on the next send.
        // Symmetric to slice-2 `CrossbeamChannel` shape.
        let (router, drain) = script_bridge(2);
        drop(drain);
        let err = router
            .send(sample_command())
            .expect_err("send must fail after drain dropped");
        assert_eq!(err, ScriptCommandError::Disconnected);
    }

    #[test]
    fn drain_yields_commands_in_fifo_order() {
        let (router, drain) = script_bridge(8);
        for i in 0..5_u32 {
            router
                .send(ScriptCommand::ReactiveWrite {
                    name: format!("n{i}"),
                    value: ScriptValue::Number(i as f64),
                })
                .unwrap();
        }
        let drained = drain.drain();
        assert_eq!(drained.len(), 5);
        for (i, cmd) in drained.iter().enumerate() {
            match cmd {
                ScriptCommand::ReactiveWrite { name, value } => {
                    assert_eq!(name, &format!("n{i}"));
                    assert_eq!(*value, ScriptValue::Number(i as f64));
                }
                // `ScriptCommand` is `#[non_exhaustive]`; this fifo
                // pin emits only `ReactiveWrite` variants so any
                // other variant is a test-data programming error.
                other => panic!("unexpected variant in fifo pin: {other:?}"),
            }
        }
    }

    #[test]
    fn drain_after_disconnect_returns_already_queued_commands() {
        // Drift pin: dropping the router MUST NOT lose commands
        // already in flight. The drain returns every queued
        // command and STOPS at the disconnected boundary; the
        // host sees the full FIFO, then no more.
        let (router, drain) = script_bridge(8);
        router.send(sample_command()).unwrap();
        router.send(sample_command()).unwrap();
        drop(router);
        let drained = drain.drain();
        assert_eq!(drained.len(), 2);
    }

    #[test]
    fn try_recv_disconnect_distinct_from_empty() {
        // Drift pin: an alive-but-empty queue and a dropped-sender
        // queue are NOT the same. The receiver-side error type
        // distinguishes them; a host that wants to retry "empty"
        // must not retry "disconnected".
        let (router, drain) = script_bridge(2);
        // Alive + empty.
        assert_eq!(drain.try_recv(), Ok(None));
        drop(router);
        // Dropped + empty.
        assert_eq!(drain.try_recv(), Err(ScriptBridgeError::Disconnected));
    }

    #[test]
    fn router_trait_object_is_send_sync() {
        // Compile-time pin: the router must be `Send + Sync` so
        // `Arc<dyn ScriptCommandSender>` clones across threads.
        // Trips at compile time if the bound drifts.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn ScriptCommandSender>>();
        assert_send_sync::<ScriptCommandRouter>();
        assert_send_sync::<ScriptBridgeReceiver>();
    }

    #[test]
    fn router_clone_shares_underlying_queue() {
        // Drift pin: cloning the router shares the underlying
        // `crossbeam_channel::Sender` (MPSC fan-in semantics).
        // Clones are interchangeable; either may emit, the drain
        // observes one combined FIFO. Mirrors the standard
        // crossbeam-channel sender semantics.
        let (router, drain) = script_bridge(8);
        let router2 = router.clone();
        router.send(sample_command()).unwrap();
        router2.send(sample_command()).unwrap();
        let drained = drain.drain();
        assert_eq!(drained.len(), 2);
    }

    #[test]
    fn queued_count_matches_drain_size() {
        let (router, drain) = script_bridge(8);
        assert_eq!(drain.queued(), 0);
        router.send(sample_command()).unwrap();
        router.send(sample_command()).unwrap();
        router.send(sample_command()).unwrap();
        assert_eq!(drain.queued(), 3);
        let drained = drain.drain();
        assert_eq!(drained.len(), 3);
        assert_eq!(drain.queued(), 0);
    }

    #[test]
    #[should_panic(expected = "script_bridge: capacity must be >= 1")]
    fn zero_capacity_panics() {
        // Drift pin: rendezvous channels are not supported because
        // the script runtime thread is `!Send` and cannot block on
        // a runtime-thread drain. The constructor panics rather
        // than handing back a router that would deadlock on first
        // send.
        let (_router, _drain) = script_bridge(0);
    }
}
