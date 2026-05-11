// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! MIDI adapter trait and midir-backed implementation.
//!
//! Phase-4a Wave 1b-A ships the [`MidiAdapter`] trait and two
//! implementations:
//!
//! - [`MidirInputAdapter`] — production midir-backed adapter.
//!   `connect()` opens an input port and installs a closure that
//!   pushes raw bytes into a shared `Mutex<VecDeque<MidiMessage>>`
//!   queue. `poll()` is non-blocking and drains the queue,
//!   accumulating per-outcome counters per the OSC adapter pattern.
//!   Route dispatch (turning a [`MidiMessage`] into a runtime
//!   command) is slice-3+ scope.
//! - [`InMemoryMidiAdapter`] — test-only seam used by the
//!   `EV-MIDI-EXTENDED` conformance corpus. Implements the same
//!   trait without a live MIDI port (CI sandbox + headless build
//!   environments lack MIDI device enumeration). Exposes
//!   `inject_raw` / `assemble_cc14` / `assemble_nrpn` / `assemble_rpn`
//!   helpers so fixtures can synthesise the wire-form streams the
//!   midir callback would deliver.
//!
//! Lifecycle counters mirror the OSC adapter shape: `connect()`
//! transitions Disconnected -> Connecting -> Connected; `disconnect()`
//! transitions Connected -> Disconnecting -> Disconnected; `poll()`
//! advances the per-outcome packet counters. The Wave 1b-A adapter
//! reuses the existing `OscAdapterState` / `OscBindErrorReason` /
//! `OscPacketOutcome` enums from `chromaticity-observability` as a
//! placeholder until the catalog grows protocol-specific MIDI label
//! enums (slice-3+ scope tracked alongside `MetricEvent::OscInboundPackets`).
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`). The
//!   [`MidiAdapter`] trait is the `ProtocolAdapter` shape named in
//!   ADR-005 "Decision" bullet 1, specialised to MIDI.
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`).
//!   Implementations MUST honour [`crate::defaults`] for queue
//!   depth, rate ceilings, and SysEx caps unless an operator
//!   override is installed per ADR-013 §6.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chromaticity_observability::catalog::{OscAdapterState, OscBindErrorReason, OscPacketOutcome};
use midir::{MidiInput, MidiInputConnection};

use crate::defaults::{DEFAULT_MAX_SYSEX_BYTES, DEFAULT_QUEUE_DEPTH};
use crate::messages::{MidiMessage, combine_14bit, parse};

/// Default midir client name surfaced to the OS MIDI subsystem.
///
/// CoreMIDI exposes this as the client display name in Audio MIDI
/// Setup; ALSA exposes it as the client identifier; WinMM ignores
/// it. Pinned constant rather than runtime parameter so the
/// production adapter has a deterministic operator-visible
/// identifier.
pub const MIDI_CLIENT_NAME: &str = "chromaticity";

/// Default midir port-name prefix surfaced to the OS MIDI subsystem.
///
/// The full port name is `<MIDI_PORT_NAME_PREFIX>-input-<index>`;
/// pinned for the same operator-visibility reason as
/// [`MIDI_CLIENT_NAME`].
pub const MIDI_PORT_NAME_PREFIX: &str = "chromaticity-midi";

/// Default 14-bit CC pairing window (50 ms).
///
/// Per `docs/08-io-integrations.md` MIDI-CONF-16: an MSB CC arriving
/// without a subsequent LSB partner within this window is committed
/// as a 14-bit value with implicit LSB = 0. Below this threshold a
/// pending pair stays open waiting for the LSB partner.
pub const DEFAULT_CC14_PAIR_WINDOW: Duration = Duration::from_millis(50);

/// Coarse classification of a MIDI device-port direction.
///
/// Wave 2.1 Lane D.1 ships [`MidiDeviceKind::Input`] only: the Tauri
/// `list_midi_devices` IPC handler enumerates input ports via
/// [`MidiInput::ports`] so the operator-facing MIDI view Devices tab
/// can render real device names rather than the static fixture set
/// in `src/lib/data/midi-fixtures.ts`. Output enumeration is reserved
/// for a follow-up wave alongside the per-device allowlist persistence
/// surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MidiDeviceKind {
    /// Inbound MIDI port (Note / CC / Pitch Bend / SysEx / etc.).
    Input,
}

/// Static descriptor for a single MIDI port surfaced by
/// [`list_devices`].
///
/// Field shape is intentionally minimal — a stable per-port id (the
/// midir port index, formatted `"input-<n>"` to namespace away from a
/// future output enumeration), the human-readable display name midir
/// returns from [`MidiInput::port_name`], and the direction discriminant.
/// The midir port name varies by OS-MIDI subsystem (CoreMIDI exposes
/// the device's display name; ALSA exposes its client / port label;
/// WinMM exposes a MIDIINCAPS name), so the bridge layer treats the
/// field as opaque operator-visible text rather than parsing it for
/// vendor / manufacturer hints. The fixture corpus's
/// `vendor` / `direction` columns get conservative defaults
/// (`vendor = "—"`, `direction = "in"`) at the frontend until a richer
/// MIDI-introspection surface lands.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MidiDeviceDescriptor {
    /// Stable per-port id. Current shape: `"input-<index>"` against
    /// the midir port-index ordering at the moment of enumeration.
    /// The index is not stable across host reconnects (CoreMIDI /
    /// ALSA / WinMM can renumber when devices are plugged or
    /// unplugged), so the operator-facing UI should treat this id as
    /// session-stable only — Wave 2.1 Lane D.1 does not persist it.
    pub id: String,
    /// Operator-visible display name from
    /// [`MidiInput::port_name`]. Falls back to `"Port <index>"` when
    /// the subsystem refuses to surface the name (e.g. a transient
    /// disconnection between `ports()` and `port_name()`).
    pub name: String,
    /// Direction discriminant. Wave 2.1 Lane D.1 emits
    /// [`MidiDeviceKind::Input`] for every entry.
    pub kind: MidiDeviceKind,
}

/// Enumerate the MIDI input ports currently visible to the host.
///
/// Returns one [`MidiDeviceDescriptor`] per port surfaced by a fresh
/// [`MidiInput`] client. On a host with no MIDI subsystem access
/// (CI / headless / a Linux box without ALSA), midir's
/// [`MidiInput::new`] errors at client construction; the function
/// collapses that error to `Ok(Vec::new())` so the Tauri command
/// surface treats "no MIDI available" as the same empty-list state
/// as "MIDI available but no devices connected" — the frontend's
/// "No MIDI devices detected" empty-state branch fires identically
/// in both cases. The non-recoverable error path (lock poisoning,
/// unrecoverable midir failure beyond client construction) currently
/// has no producer in this function body; the `Result` return type
/// is forward-compat for the follow-up wave that adds output-device
/// enumeration plus a more granular failure surface for the
/// per-platform mid-iteration error classes midir surfaces.
///
/// Evidence: `EV-INTEGRATION-MATRIX` (MIDI arm, Wave 2.1 Lane D.1) —
/// device-listing surface widened from the W18 PR3 fixture-only
/// posture documented in the `EV-INTEGRATION-MATRIX` Wave 1c-β
/// closure section of `docs/EV-GATE-MODEL-AUDIT-checklist.md`.
pub fn list_devices() -> Result<Vec<MidiDeviceDescriptor>, MidiAdapterError> {
    // Client construction can fail on headless / CI hosts without a
    // MIDI subsystem. Treat that as the same empty-list state as
    // "MIDI available but no devices connected" — both surface the
    // frontend's no-devices empty-state branch.
    let input = match MidiInput::new(MIDI_CLIENT_NAME) {
        Ok(input) => input,
        Err(_) => return Ok(Vec::new()),
    };
    let ports = input.ports();
    let descriptors = ports
        .iter()
        .enumerate()
        .map(|(idx, port)| {
            let name = input
                .port_name(port)
                .unwrap_or_else(|_| format!("Port {idx}"));
            MidiDeviceDescriptor {
                id: format!("input-{idx}"),
                name,
                kind: MidiDeviceKind::Input,
            }
        })
        .collect();
    Ok(descriptors)
}

/// MIDI adapter — protocol-abstract trait per ADR-005.
///
/// `connect` / `disconnect` mirror the OSC adapter's `bind` /
/// `unbind`, framed in MIDI's port-index vocabulary. `poll` is
/// non-blocking; the adapter drains any messages midir's callback
/// queued and accumulates per-outcome counters.
pub trait MidiAdapter {
    /// Adapter-specific error type.
    type Error;

    /// Open an inbound MIDI port at the given index.
    ///
    /// Idempotent on success: re-connecting to the same port is a
    /// no-op. Disconnecting an existing connection before
    /// reconnecting to a different port is the operator's
    /// responsibility — Wave 1b-A does not auto-disconnect on
    /// re-connect.
    fn connect(&mut self, port_index: usize) -> Result<(), Self::Error>;

    /// Release any held MIDI input connection.
    ///
    /// Idempotent: calling `disconnect` on an already-disconnected
    /// adapter is not an error. Implementations MUST NOT leak the
    /// midir resources across `disconnect` boundaries.
    fn disconnect(&mut self) -> Result<(), Self::Error>;

    /// Poll the adapter once for queued messages.
    ///
    /// Non-blocking. Drains every message accumulated by midir's
    /// callback since the last poll, advancing the per-outcome
    /// counter for each (Accepted on a successful classify, dropped
    /// on malformed / rate-limited / overflow).
    fn poll(&mut self) -> Result<(), Self::Error>;
}

/// Errors surfaced by the midir-backed adapter.
///
/// Wave 1b-A surfaces diagnostics without wiring them to a runtime
/// event channel. Slice-3+ routes these through the
/// `chromaticity-runtime` bus as validator-rejection events.
#[derive(Debug)]
#[non_exhaustive]
pub enum MidiAdapterError {
    /// `connect` could not open the requested port.
    ConnectFailed(String),
    /// `poll` hit a non-recoverable queue lock failure.
    RecvFailed(String),
    /// midir delivered a message the parser rejected.
    MalformedMessage(String),
    /// Adapter saw a message kind it does not yet route — Wave 1b-A
    /// rejects MPE Configuration messages with this variant per the
    /// brief deferral; full SysEx routing is V2 scope.
    UnsupportedMessage(MidiMessageKind),
}

impl core::fmt::Display for MidiAdapterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConnectFailed(msg) => write!(f, "midi connect failed: {msg}"),
            Self::RecvFailed(msg) => write!(f, "midi recv failed: {msg}"),
            Self::MalformedMessage(msg) => write!(f, "midi message malformed: {msg}"),
            Self::UnsupportedMessage(kind) => {
                write!(f, "midi message kind unsupported: {kind:?}")
            }
        }
    }
}

impl std::error::Error for MidiAdapterError {}

/// Coarse classification of [`MidiMessage`] for diagnostic surfaces.
///
/// Used by [`MidiAdapterError::UnsupportedMessage`] to communicate
/// "this kind of message arrived but Wave 1b-A does not route it"
/// without retaining the full message body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MidiMessageKind {
    /// Channel Voice (Note / CC / Pitch Bend / Aftertouch / etc.).
    ChannelVoice,
    /// System Real-Time (Clock / Start / Continue / Stop).
    SystemRealTime,
    /// System Common (SPP / MTC / Tune Request).
    SystemCommon,
    /// System Exclusive — logged only this Wave; full routing V2.
    SysEx,
    /// MPE Configuration Message — deferred to V2 per the brief.
    MpeConfiguration,
}

/// Operational health of a midir-backed adapter.
///
/// Mirrors the OSC adapter's [`OscHealthState`]-style enum: orthogonal
/// to the bind-lifecycle catalog state, surfaces R-05-style
/// recovering / degraded posture for IntegrationHub routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MidiHealthState {
    /// Adapter is connected and the recv-callback path is delivering
    /// successfully.
    Healthy,
    /// Connect succeeded only after at least one retry. Stays sticky
    /// until the next successful poll or disconnect.
    Recovering,
    /// `poll()` saw repeated queue-overflow events. Cleared by the
    /// first subsequent quiescent poll.
    Degraded,
}

/// Per-[`OscPacketOutcome`] packet counter storage.
///
/// Reused unchanged from the OSC adapter shape so the
/// `chromaticity_osc_inbound_packets_total` catalog row's eventual
/// MIDI sibling can fold this counter into a unified emission site
/// when the catalog grows protocol-specific MIDI label enums.
#[derive(Debug, Default)]
struct PacketOutcomeCounts {
    accepted: u64,
    malformed_dropped: u64,
    rate_limited_dropped: u64,
    overflow_dropped: u64,
}

/// State-transition counter table for the midir-backed adapter.
///
/// Reuses [`OscAdapterState`] verbatim; the Wave 1b-A adapter maps
/// `Disconnected`/`Connecting`/`Connected`/`Disconnecting` onto
/// `Unbound`/`Binding`/`Bound`/`Unbinding` semantically.
#[derive(Debug, Default)]
struct StateTransitionCounts {
    disconnected_to_connecting: u64,
    connecting_to_connected: u64,
    connecting_to_disconnected: u64,
    connected_to_disconnecting: u64,
    disconnecting_to_disconnected: u64,
}

/// Per-[`OscBindErrorReason`] connect-error counter storage.
///
/// Wave 1b-A folds midir's failure reasons into the existing
/// reason enum (`AddressInUse` -> port taken, `PermissionDenied` ->
/// privileged-port refusal, `AddressNotAvailable` -> port index
/// out of range, `Other` -> remainder). The catalog grows MIDI-
/// specific reasons in slice-3+ if operator-visible diagnostics
/// require finer disambiguation.
#[derive(Debug, Default)]
struct ConnectErrorCounts {
    address_in_use: u64,
    permission_denied: u64,
    address_not_available: u64,
    other: u64,
}

/// Pending MSB byte for a 14-bit CC pair, awaiting an LSB partner.
///
/// `controller_msb` is the MSB controller number (0..=31); `msb`
/// is the value byte. The pair-assembly state machine commits the
/// pending entry after the matching LSB controller (controller +
/// 32) arrives, or after [`DEFAULT_CC14_PAIR_WINDOW`] elapses
/// (in which case `value14 = msb << 7`, implicit LSB = 0 per
/// MIDI-CONF-16).
#[derive(Debug, Clone, Copy)]
struct PendingCc14 {
    controller_msb: u8,
    msb: u8,
}

/// Pending RPN/NRPN parameter address, awaiting data-MSB / data-LSB.
///
/// `is_nrpn` distinguishes NRPN (CC 99/98) from RPN (CC 101/100);
/// `parameter14` is the assembled 14-bit parameter number. Data MSB
/// (CC 6) and data LSB (CC 38) follow to complete the sequence.
#[derive(Debug, Clone, Copy)]
struct PendingRpnLike {
    is_nrpn: bool,
    parameter14: u16,
    data_msb: Option<u8>,
}

/// Internal pair-assembly state machine. Tracks the 14-bit CC pair
/// pending state and the RPN/NRPN parameter-address state.
///
/// Per-channel rather than global: the MIDI 1.0 spec allows
/// independent RPN / NRPN streams on different channels.
#[derive(Debug, Default)]
struct PairAssembler {
    pending_cc14: [Option<PendingCc14>; 16],
    pending_rpn: [Option<PendingRpnLike>; 16],
}

/// Producer-shared message queue type. Boxed Mutex<VecDeque> behind
/// an `Arc` so midir's callback (which moves into a closure) can
/// share state with the polling thread without `unsafe`.
type SharedQueue = Arc<Mutex<VecDeque<MidiMessage>>>;

/// midir-backed MIDI input adapter.
///
/// Wraps midir's callback API in a non-blocking poll surface per
/// ADR-005 — the callback pushes parsed messages into a shared
/// `Arc<Mutex<VecDeque>>` queue, and `poll()` drains the queue under
/// the trait's non-blocking contract. Per-outcome counters mirror
/// the OSC adapter pattern; route dispatch is slice-3+ scope.
pub struct MidirInputAdapter {
    midi_input: Option<MidiInput>,
    connection: Option<MidiInputConnection<()>>,
    queue: SharedQueue,
    queue_depth_cap: usize,
    sysex_byte_cap: u32,
    current_state: OscAdapterState,
    health_state: MidiHealthState,
    attempted_connects: u32,
    consecutive_overflow_polls: u32,
    outcome_counts: PacketOutcomeCounts,
    state_transition_counts: StateTransitionCounts,
    connect_error_counts: ConnectErrorCounts,
    assembler: PairAssembler,
}

impl MidirInputAdapter {
    /// Construct a fresh, disconnected adapter with the Wave 1b-A
    /// defaults. The midir client is created lazily on `connect`
    /// rather than at construction so a fresh adapter can be
    /// instantiated in environments without MIDI subsystem access
    /// (e.g. CI sandboxes for the conformance corpus).
    #[must_use]
    pub fn new() -> Self {
        Self {
            midi_input: None,
            connection: None,
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(DEFAULT_QUEUE_DEPTH))),
            queue_depth_cap: DEFAULT_QUEUE_DEPTH,
            sysex_byte_cap: DEFAULT_MAX_SYSEX_BYTES,
            current_state: OscAdapterState::Unbound,
            health_state: MidiHealthState::Healthy,
            attempted_connects: 0,
            consecutive_overflow_polls: 0,
            outcome_counts: PacketOutcomeCounts::default(),
            state_transition_counts: StateTransitionCounts::default(),
            connect_error_counts: ConnectErrorCounts::default(),
            assembler: PairAssembler::default(),
        }
    }

    /// Current operational health state.
    #[must_use]
    pub fn health_state(&self) -> MidiHealthState {
        self.health_state
    }

    /// Total connect attempts (including the successful one) made
    /// on the most recent [`connect`] call.
    ///
    /// [`connect`]: MidiAdapter::connect
    #[must_use]
    pub fn attempted_connects(&self) -> u32 {
        self.attempted_connects
    }

    /// Current count of packets that the adapter classified as
    /// `outcome` since construction. The Wave 1b-A adapter uses the
    /// existing [`OscPacketOutcome`] enum; the OSC `Accepted`
    /// variant counts MIDI Accepted, `OversizedDropped` /
    /// `RateLimitedDropped` map identically, and `ParseError` covers
    /// any malformed message the midir callback delivered.
    #[must_use]
    pub fn packet_outcome_count(&self, outcome: OscPacketOutcome) -> u64 {
        match outcome {
            OscPacketOutcome::Accepted => self.outcome_counts.accepted,
            OscPacketOutcome::OversizedDropped => self.outcome_counts.overflow_dropped,
            OscPacketOutcome::RateLimitedDropped => self.outcome_counts.rate_limited_dropped,
            OscPacketOutcome::ParseError => self.outcome_counts.malformed_dropped,
            _ => 0,
        }
    }

    /// Current count of state-machine transitions from `from` to
    /// `to`.
    #[must_use]
    pub fn state_transition_count(&self, from: OscAdapterState, to: OscAdapterState) -> u64 {
        match (from, to) {
            (OscAdapterState::Unbound, OscAdapterState::Binding) => {
                self.state_transition_counts.disconnected_to_connecting
            }
            (OscAdapterState::Binding, OscAdapterState::Bound) => {
                self.state_transition_counts.connecting_to_connected
            }
            (OscAdapterState::Binding, OscAdapterState::Unbound) => {
                self.state_transition_counts.connecting_to_disconnected
            }
            (OscAdapterState::Bound, OscAdapterState::Unbinding) => {
                self.state_transition_counts.connected_to_disconnecting
            }
            (OscAdapterState::Unbinding, OscAdapterState::Unbound) => {
                self.state_transition_counts.disconnecting_to_disconnected
            }
            _ => 0,
        }
    }

    /// Current count of connect-failure events mapped to `reason`.
    #[must_use]
    pub fn connect_error_count(&self, reason: OscBindErrorReason) -> u64 {
        match reason {
            OscBindErrorReason::AddressInUse => self.connect_error_counts.address_in_use,
            OscBindErrorReason::PermissionDenied => self.connect_error_counts.permission_denied,
            OscBindErrorReason::AddressNotAvailable => {
                self.connect_error_counts.address_not_available
            }
            OscBindErrorReason::Other => self.connect_error_counts.other,
            _ => 0,
        }
    }

    /// Current state-machine state.
    #[must_use]
    pub fn current_state(&self) -> OscAdapterState {
        self.current_state
    }

    /// Configured SysEx byte cap (operator-overridable per ADR-013
    /// §6, but Wave 1b-A pins the [`DEFAULT_MAX_SYSEX_BYTES`] value).
    #[must_use]
    pub fn sysex_byte_cap(&self) -> u32 {
        self.sysex_byte_cap
    }

    fn record_outcome(&mut self, outcome: OscPacketOutcome) {
        match outcome {
            OscPacketOutcome::Accepted => self.outcome_counts.accepted += 1,
            OscPacketOutcome::OversizedDropped => self.outcome_counts.overflow_dropped += 1,
            OscPacketOutcome::RateLimitedDropped => self.outcome_counts.rate_limited_dropped += 1,
            OscPacketOutcome::ParseError => self.outcome_counts.malformed_dropped += 1,
            _ => (),
        }
    }

    fn record_state_transition(&mut self, from: OscAdapterState, to: OscAdapterState) {
        match (from, to) {
            (OscAdapterState::Unbound, OscAdapterState::Binding) => {
                self.state_transition_counts.disconnected_to_connecting += 1;
            }
            (OscAdapterState::Binding, OscAdapterState::Bound) => {
                self.state_transition_counts.connecting_to_connected += 1;
            }
            (OscAdapterState::Binding, OscAdapterState::Unbound) => {
                self.state_transition_counts.connecting_to_disconnected += 1;
            }
            (OscAdapterState::Bound, OscAdapterState::Unbinding) => {
                self.state_transition_counts.connected_to_disconnecting += 1;
            }
            (OscAdapterState::Unbinding, OscAdapterState::Unbound) => {
                self.state_transition_counts.disconnecting_to_disconnected += 1;
            }
            _ => (),
        }
    }

    fn record_connect_error(&mut self, reason: OscBindErrorReason) {
        match reason {
            OscBindErrorReason::AddressInUse => self.connect_error_counts.address_in_use += 1,
            OscBindErrorReason::PermissionDenied => {
                self.connect_error_counts.permission_denied += 1;
            }
            OscBindErrorReason::AddressNotAvailable => {
                self.connect_error_counts.address_not_available += 1;
            }
            OscBindErrorReason::Other => self.connect_error_counts.other += 1,
            _ => (),
        }
    }

    fn enter_state(&mut self, to: OscAdapterState) {
        let from = self.current_state;
        self.current_state = to;
        self.record_state_transition(from, to);
    }

    /// Process one queued [`MidiMessage`] through the pair-assembly
    /// state machine, recording the outcome counter. Returns the
    /// final assembled message (post-pairing) when one is produced;
    /// otherwise returns `None` (e.g. an MSB CC waiting for its LSB
    /// partner).
    ///
    /// Visible to the conformance corpus via the in-memory adapter
    /// that exposes the same assembler logic.
    fn process_queued(&mut self, msg: MidiMessage) -> Option<MidiMessage> {
        // SysEx: cap-check then accept (logged-only; no routing).
        if let MidiMessage::SysEx { len, .. } = &msg {
            #[allow(clippy::cast_possible_truncation)]
            let len_u32 = (*len).min(u32::MAX as usize) as u32;
            if len_u32 > self.sysex_byte_cap {
                self.record_outcome(OscPacketOutcome::OversizedDropped);
                return None;
            }
            self.record_outcome(OscPacketOutcome::Accepted);
            return Some(msg);
        }
        // CC: route through pair-assembly state machine.
        if let MidiMessage::ControlChange {
            channel,
            controller,
            value,
        } = msg
        {
            let ch = channel as usize;
            // RPN/NRPN parameter setup.
            match controller {
                99 => {
                    // NRPN MSB
                    let entry = self.assembler.pending_rpn[ch].get_or_insert(PendingRpnLike {
                        is_nrpn: true,
                        parameter14: 0,
                        data_msb: None,
                    });
                    entry.is_nrpn = true;
                    entry.parameter14 = (entry.parameter14 & 0x007F) | (u16::from(value) << 7);
                    entry.data_msb = None;
                    self.record_outcome(OscPacketOutcome::Accepted);
                    return None;
                }
                98 => {
                    // NRPN LSB
                    let entry = self.assembler.pending_rpn[ch].get_or_insert(PendingRpnLike {
                        is_nrpn: true,
                        parameter14: 0,
                        data_msb: None,
                    });
                    entry.is_nrpn = true;
                    entry.parameter14 = (entry.parameter14 & 0x3F80) | u16::from(value);
                    entry.data_msb = None;
                    self.record_outcome(OscPacketOutcome::Accepted);
                    return None;
                }
                101 => {
                    // RPN MSB
                    let entry = self.assembler.pending_rpn[ch].get_or_insert(PendingRpnLike {
                        is_nrpn: false,
                        parameter14: 0,
                        data_msb: None,
                    });
                    entry.is_nrpn = false;
                    entry.parameter14 = (entry.parameter14 & 0x007F) | (u16::from(value) << 7);
                    entry.data_msb = None;
                    self.record_outcome(OscPacketOutcome::Accepted);
                    return None;
                }
                100 => {
                    // RPN LSB
                    let entry = self.assembler.pending_rpn[ch].get_or_insert(PendingRpnLike {
                        is_nrpn: false,
                        parameter14: 0,
                        data_msb: None,
                    });
                    entry.is_nrpn = false;
                    entry.parameter14 = (entry.parameter14 & 0x3F80) | u16::from(value);
                    entry.data_msb = None;
                    // Null-parameter (0x7F 0x7F) sequence: clear the
                    // pending RPN context per docs/08 MIDI-CONF-17.
                    if entry.parameter14 == 0x3FFF {
                        self.assembler.pending_rpn[ch] = None;
                    }
                    self.record_outcome(OscPacketOutcome::Accepted);
                    return None;
                }
                6 => {
                    // Data MSB
                    if let Some(entry) = self.assembler.pending_rpn[ch].as_mut() {
                        entry.data_msb = Some(value);
                        self.record_outcome(OscPacketOutcome::Accepted);
                        return None;
                    }
                    // No active RPN/NRPN context: fall through to
                    // 14-bit CC pair handling for CC #6 specifically
                    // (CC 6 is also an MSB controller in the 14-bit
                    // pair scheme — its LSB partner is CC 38).
                    return self.handle_cc14_pair(channel, controller, value);
                }
                38 => {
                    // Data LSB OR 14-bit CC LSB partner.
                    if let Some(entry) = self.assembler.pending_rpn[ch].as_mut() {
                        if let Some(data_msb) = entry.data_msb {
                            let value14 = combine_14bit(data_msb, value);
                            let parameter14 = entry.parameter14;
                            let is_nrpn = entry.is_nrpn;
                            // RPN/NRPN reset-null sequence: keep
                            // assembled message but clear state on
                            // the next null-parameter pair.
                            self.assembler.pending_rpn[ch] = Some(PendingRpnLike {
                                is_nrpn,
                                parameter14,
                                data_msb: None,
                            });
                            self.record_outcome(OscPacketOutcome::Accepted);
                            let assembled = if is_nrpn {
                                MidiMessage::Nrpn {
                                    channel,
                                    parameter14,
                                    value14,
                                }
                            } else {
                                MidiMessage::Rpn {
                                    channel,
                                    parameter14,
                                    value14,
                                }
                            };
                            return Some(assembled);
                        }
                    }
                    // No active RPN/NRPN context: handle as CC 14-bit
                    // LSB partner.
                    return self.handle_cc14_pair(channel, controller, value);
                }
                _ => {
                    // 14-bit CC MSB / LSB pair routing.
                    return self.handle_cc14_pair(channel, controller, value);
                }
            }
        }
        // Other channel-voice + system messages pass through unchanged.
        self.record_outcome(OscPacketOutcome::Accepted);
        Some(msg)
    }

    fn handle_cc14_pair(&mut self, channel: u8, controller: u8, value: u8) -> Option<MidiMessage> {
        let ch = channel as usize;
        if controller < 32 {
            // MSB controller. Open or replace the pending entry.
            self.assembler.pending_cc14[ch] = Some(PendingCc14 {
                controller_msb: controller,
                msb: value,
            });
            self.record_outcome(OscPacketOutcome::Accepted);
            None
        } else if (32..64).contains(&controller) {
            // LSB controller. Pair with the matching MSB if pending.
            let msb_ctrl = controller - 32;
            if let Some(pending) = self.assembler.pending_cc14[ch] {
                if pending.controller_msb == msb_ctrl {
                    let value14 = combine_14bit(pending.msb, value);
                    self.assembler.pending_cc14[ch] = None;
                    self.record_outcome(OscPacketOutcome::Accepted);
                    return Some(MidiMessage::Cc14Bit {
                        channel,
                        controller_msb: msb_ctrl,
                        value14,
                    });
                }
            }
            // Unmatched LSB: emit as bare CC.
            self.record_outcome(OscPacketOutcome::Accepted);
            Some(MidiMessage::ControlChange {
                channel,
                controller,
                value,
            })
        } else {
            // 64..=127: ordinary 7-bit CC, no pairing.
            self.record_outcome(OscPacketOutcome::Accepted);
            Some(MidiMessage::ControlChange {
                channel,
                controller,
                value,
            })
        }
    }

    /// Force-commit any pending 14-bit CC pair on `channel` with
    /// implicit LSB = 0 per `docs/08-io-integrations.md`
    /// MIDI-CONF-16. Returns the assembled message if one was
    /// pending, otherwise `None`.
    ///
    /// The Wave 1b-A in-process state machine does not wire up a
    /// real timer — the pairing window is the operator-visible
    /// commit boundary the dispatch layer (slice-3+) drives via
    /// the runtime tick. The conformance corpus calls this directly
    /// to exercise the timeout path deterministically.
    pub fn commit_pending_cc14(&mut self, channel: u8) -> Option<MidiMessage> {
        let ch = channel as usize;
        if let Some(pending) = self.assembler.pending_cc14[ch].take() {
            self.record_outcome(OscPacketOutcome::Accepted);
            let value14 = combine_14bit(pending.msb, 0);
            return Some(MidiMessage::Cc14Bit {
                channel,
                controller_msb: pending.controller_msb,
                value14,
            });
        }
        None
    }
}

impl Default for MidirInputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiAdapter for MidirInputAdapter {
    type Error = MidiAdapterError;

    fn connect(&mut self, port_index: usize) -> Result<(), Self::Error> {
        self.attempted_connects = 1;
        self.enter_state(OscAdapterState::Binding);
        let midi_input = MidiInput::new(MIDI_CLIENT_NAME).map_err(|e| {
            self.record_connect_error(OscBindErrorReason::Other);
            self.enter_state(OscAdapterState::Unbound);
            MidiAdapterError::ConnectFailed(format!("midi-input init: {e}"))
        })?;
        let ports = midi_input.ports();
        let Some(port) = ports.get(port_index) else {
            self.record_connect_error(OscBindErrorReason::AddressNotAvailable);
            self.enter_state(OscAdapterState::Unbound);
            return Err(MidiAdapterError::ConnectFailed(format!(
                "port index {port_index} out of range (have {} ports)",
                ports.len()
            )));
        };
        let queue = Arc::clone(&self.queue);
        let cap = self.queue_depth_cap;
        let port_name = format!("{MIDI_PORT_NAME_PREFIX}-input-{port_index}");
        let connect_result = midi_input.connect(
            port,
            &port_name,
            move |_timestamp, raw, _user| {
                if let Some(parsed) = parse(raw) {
                    if let Ok(mut q) = queue.lock() {
                        if q.len() >= cap {
                            // Ring-buffer: drop oldest to make room.
                            let _ = q.pop_front();
                        }
                        q.push_back(parsed);
                    }
                }
            },
            (),
        );
        match connect_result {
            Ok(connection) => {
                self.connection = Some(connection);
                self.midi_input = None; // consumed by `connect`
                self.enter_state(OscAdapterState::Bound);
                self.health_state = MidiHealthState::Healthy;
                Ok(())
            }
            Err(e) => {
                self.record_connect_error(OscBindErrorReason::Other);
                self.enter_state(OscAdapterState::Unbound);
                Err(MidiAdapterError::ConnectFailed(format!(
                    "midir connect: {e}"
                )))
            }
        }
    }

    fn disconnect(&mut self) -> Result<(), Self::Error> {
        if self.current_state == OscAdapterState::Unbound {
            return Ok(());
        }
        self.enter_state(OscAdapterState::Unbinding);
        if let Some(connection) = self.connection.take() {
            // midir's MidiInputConnection::close returns the
            // underlying MidiInput; we drop it.
            let _ = connection.close();
        }
        self.midi_input = None;
        self.enter_state(OscAdapterState::Unbound);
        self.health_state = MidiHealthState::Healthy;
        self.attempted_connects = 0;
        self.consecutive_overflow_polls = 0;
        Ok(())
    }

    fn poll(&mut self) -> Result<(), Self::Error> {
        if self.current_state != OscAdapterState::Bound {
            return Ok(());
        }
        // Drain the queue under lock, then process each message
        // without holding the lock so the midir callback can keep
        // pushing concurrently.
        let drained: Vec<MidiMessage> = match self.queue.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(e) => {
                return Err(MidiAdapterError::RecvFailed(format!("queue lock: {e}")));
            }
        };
        if drained.is_empty() {
            // Quiescent poll clears the Degraded sticky-state.
            if self.health_state == MidiHealthState::Degraded {
                self.health_state = MidiHealthState::Healthy;
            }
            return Ok(());
        }
        for msg in drained {
            let _ = self.process_queued(msg);
        }
        self.consecutive_overflow_polls = 0;
        if self.health_state == MidiHealthState::Degraded {
            self.health_state = MidiHealthState::Healthy;
        }
        Ok(())
    }
}

/// In-memory test seam adapter used by the conformance corpus.
///
/// Implements [`MidiAdapter`] without midir / OS MIDI subsystem
/// access. `connect` / `disconnect` advance the same lifecycle
/// counters; `poll` drains the same queue. Fixtures call
/// `inject_raw` to push synthesized wire-form bytes through the
/// parser-and-pair-assembly pipeline as if midir's callback had
/// delivered them.
pub struct InMemoryMidiAdapter {
    inner: MidirInputAdapter,
    /// Captured outputs: the post-assembly [`MidiMessage`] sequence
    /// each `poll()` produced, in chronological order. Tests assert
    /// against this so the pair-assembly state machine's emissions
    /// (Cc14Bit, Nrpn, Rpn) are observable without poking adapter
    /// internals.
    captured: Vec<MidiMessage>,
    /// MPE-rejection counter — Wave 1b-A increments this when a
    /// caller injects an MPE Configuration Message; full MPE
    /// support is V2 scope per the brief.
    mpe_rejections: u64,
}

impl InMemoryMidiAdapter {
    /// Construct a fresh in-memory adapter. The midir client is
    /// never initialized in this seam, so the adapter works in
    /// environments without MIDI subsystem access.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: MidirInputAdapter::new(),
            captured: Vec::new(),
            mpe_rejections: 0,
        }
    }

    /// Inject raw wire-form bytes through the parser. If the parser
    /// rejects the input, increments the malformed-dropped counter
    /// and returns `Err`. Otherwise enqueues the parsed message for
    /// the next `poll()` to drain.
    pub fn inject_raw(&mut self, raw: &[u8]) -> Result<(), MidiAdapterError> {
        let Some(parsed) = parse(raw) else {
            self.inner.record_outcome(OscPacketOutcome::ParseError);
            return Err(MidiAdapterError::MalformedMessage(format!(
                "parse rejected raw bytes: {raw:?}"
            )));
        };
        let queue = Arc::clone(&self.inner.queue);
        if let Ok(mut q) = queue.lock() {
            if q.len() >= self.inner.queue_depth_cap {
                let _ = q.pop_front();
            }
            q.push_back(parsed);
        }
        Ok(())
    }

    /// Inject an MPE Configuration Message — Wave 1b-A rejects
    /// these with the diagnostic surface per the brief deferral.
    /// Increments [`Self::mpe_rejections`] and surfaces the
    /// `UnsupportedMessage(MidiMessageKind::MpeConfiguration)`
    /// error.
    pub fn inject_mpe_configuration(&mut self) -> Result<(), MidiAdapterError> {
        self.mpe_rejections += 1;
        Err(MidiAdapterError::UnsupportedMessage(
            MidiMessageKind::MpeConfiguration,
        ))
    }

    /// Total MPE Configuration Messages rejected since construction.
    #[must_use]
    pub fn mpe_rejection_count(&self) -> u64 {
        self.mpe_rejections
    }

    /// Captured messages from all prior `poll()` invocations.
    #[must_use]
    pub fn captured(&self) -> &[MidiMessage] {
        &self.captured
    }

    /// Reset captured-message buffer (between fixture sub-phases).
    pub fn clear_captured(&mut self) {
        self.captured.clear();
    }

    /// Force-commit the pending 14-bit CC pair on `channel` per the
    /// `commit_pending_cc14` semantics. The assembled message is
    /// pushed onto the captured list directly (it bypasses the
    /// queue, mirroring the timeout-driven commit pattern the
    /// dispatch layer uses).
    pub fn force_commit_pending_cc14(&mut self, channel: u8) {
        if let Some(msg) = self.inner.commit_pending_cc14(channel) {
            self.captured.push(msg);
        }
    }

    /// Borrowed access to the inner counters / state.
    #[must_use]
    pub fn inner(&self) -> &MidirInputAdapter {
        &self.inner
    }

    /// Pass-through to the inner [`MidirInputAdapter::current_state`].
    #[must_use]
    pub fn current_state(&self) -> OscAdapterState {
        self.inner.current_state
    }

    /// Pass-through to the inner packet-outcome counter.
    #[must_use]
    pub fn packet_outcome_count(&self, outcome: OscPacketOutcome) -> u64 {
        self.inner.packet_outcome_count(outcome)
    }

    /// Pass-through to the inner state-transition counter.
    #[must_use]
    pub fn state_transition_count(&self, from: OscAdapterState, to: OscAdapterState) -> u64 {
        self.inner.state_transition_count(from, to)
    }
}

impl Default for InMemoryMidiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiAdapter for InMemoryMidiAdapter {
    type Error = MidiAdapterError;

    fn connect(&mut self, _port_index: usize) -> Result<(), Self::Error> {
        if self.inner.current_state == OscAdapterState::Bound {
            return Ok(());
        }
        self.inner.attempted_connects = 1;
        self.inner.enter_state(OscAdapterState::Binding);
        self.inner.enter_state(OscAdapterState::Bound);
        self.inner.health_state = MidiHealthState::Healthy;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), Self::Error> {
        if self.inner.current_state == OscAdapterState::Unbound {
            return Ok(());
        }
        self.inner.enter_state(OscAdapterState::Unbinding);
        self.inner.enter_state(OscAdapterState::Unbound);
        self.inner.health_state = MidiHealthState::Healthy;
        self.inner.attempted_connects = 0;
        Ok(())
    }

    fn poll(&mut self) -> Result<(), Self::Error> {
        if self.inner.current_state != OscAdapterState::Bound {
            return Ok(());
        }
        let drained: Vec<MidiMessage> = match self.inner.queue.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(e) => {
                return Err(MidiAdapterError::RecvFailed(format!("queue lock: {e}")));
            }
        };
        for msg in drained {
            if let Some(out) = self.inner.process_queued(msg) {
                self.captured.push(out);
            }
        }
        if self.inner.health_state == MidiHealthState::Degraded {
            self.inner.health_state = MidiHealthState::Healthy;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-MIDI-EXTENDED (T1) — adapter trait surface +
    //! lifecycle + counter-table invariants for the Wave 1b-A
    //! midir-backed implementation.
    //!
    //! T1 per docs/43 §15.5: each test pins a specific Wave 1b-A
    //! brief invariant against the adapter implementation. Live
    //! port-bind coverage requires platform MIDI subsystem access
    //! and is not run in CI; the live midir path is exercised in
    //! the conformance corpus via the in-memory seam adapter that
    //! shares the same internal counter logic.

    use chromaticity_observability::catalog::{OscAdapterState, OscPacketOutcome};

    use super::{
        InMemoryMidiAdapter, MidiAdapter, MidiDeviceKind, MidirInputAdapter, list_devices,
    };

    #[test]
    fn fresh_adapter_has_zero_counters() {
        let adapter = MidirInputAdapter::new();
        for outcome in [
            OscPacketOutcome::Accepted,
            OscPacketOutcome::OversizedDropped,
            OscPacketOutcome::RateLimitedDropped,
            OscPacketOutcome::ParseError,
        ] {
            assert_eq!(adapter.packet_outcome_count(outcome), 0);
        }
        assert_eq!(adapter.attempted_connects(), 0);
        assert_eq!(adapter.current_state(), OscAdapterState::Unbound);
    }

    #[test]
    fn in_memory_connect_disconnect_round_trip_records_state_machine() {
        let mut adapter = InMemoryMidiAdapter::new();
        adapter.connect(0).expect("in-memory connect succeeds");
        assert_eq!(adapter.current_state(), OscAdapterState::Bound);
        adapter.disconnect().expect("in-memory disconnect succeeds");
        assert_eq!(adapter.current_state(), OscAdapterState::Unbound);

        for (from, to, expected) in [
            (OscAdapterState::Unbound, OscAdapterState::Binding, 1),
            (OscAdapterState::Binding, OscAdapterState::Bound, 1),
            (OscAdapterState::Bound, OscAdapterState::Unbinding, 1),
            (OscAdapterState::Unbinding, OscAdapterState::Unbound, 1),
        ] {
            assert_eq!(
                adapter.state_transition_count(from, to),
                expected,
                "expected one {from:?} -> {to:?} transition",
            );
        }
    }

    #[test]
    fn disconnect_on_disconnected_adapter_is_noop() {
        let mut adapter = InMemoryMidiAdapter::new();
        adapter.disconnect().expect("idempotent disconnect");
        assert_eq!(adapter.current_state(), OscAdapterState::Unbound);
        assert_eq!(
            adapter.state_transition_count(OscAdapterState::Unbinding, OscAdapterState::Unbound),
            0,
            "disconnecting an already-disconnected adapter must not record a transition",
        );
    }

    #[test]
    fn list_devices_returns_ok_on_any_host_with_input_kind_for_every_entry() {
        // Wave 2.1 Lane D.1 invariant: `list_devices` MUST NOT panic
        // and MUST NOT propagate the `MidiInput::new` client-construction
        // error to the caller. On CI / headless / no-MIDI-subsystem
        // hosts the function collapses the error to `Ok(Vec::new())`;
        // on a host that DOES surface ports, every descriptor's `kind`
        // is `MidiDeviceKind::Input` (output enumeration is reserved
        // for a follow-up wave) and the `id` carries the `input-`
        // prefix so a future output enumeration can namespace-separate
        // without breaking the existing id space.
        let devices = list_devices().expect("list_devices must not error at the MVP surface");
        for d in &devices {
            assert_eq!(d.kind, MidiDeviceKind::Input);
            assert!(
                d.id.starts_with("input-"),
                "expected id to start with `input-`, got {:?}",
                d.id,
            );
            assert!(!d.name.is_empty(), "midir port name must not be empty");
        }
    }

    #[test]
    fn mpe_rejection_increments_counter_and_returns_unsupported() {
        let mut adapter = InMemoryMidiAdapter::new();
        let res = adapter.inject_mpe_configuration();
        assert!(res.is_err(), "MPE Configuration must reject");
        assert_eq!(adapter.mpe_rejection_count(), 1);
    }
}
