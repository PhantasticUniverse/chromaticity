// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Live Pro DJ Link UDP listener and hands-free phrase/drop auto-transition planner.
//!
//! Designed for live multi-deck DJ workflows (3–4 decks on CDJs or Rekordbox) where the
//! operator has both hands on the mixer and needs:
//!
//! 1. **Non-blocking UDP beat reception** ([`ProDjLinkListener`]) on Pro DJ Link beat
//!    port `50001` (or an ephemeral loopback port `127.0.0.1:0` in tests), decoding
//!    `Qspt1WmJOL` (`0x28`) datagrams via [`parse_pro_dj_link_beat_packet`], maintaining
//!    per-deck [`RekordboxDeckState`], resolving the active Master Deck, and falling back
//!    gracefully to audio-derived BPM ([`ProDjLinkSyncState::FallbackToAudio`]) if UDP
//!    traffic drops out for longer than `timeout_ms` (default `3000 ms`).
//! 2. **Hands-free auto-transition planning** ([`AutoTransitionPlanner`]) that evaluates
//!    beat/bar/deck/energy state deterministically to emit [`TransitionTrigger`] events on
//!    configurable phrase boundaries (e.g. 16 or 32 bars), Master Deck handoffs, or
//!    sharp energy drops, protected by a cooldown guard so transitions never fire
//!    back-to-back mid-fill.

#![forbid(unsafe_code)]

use crate::rekordbox_bridge::{
    PRO_DJ_LINK_BEAT_PORT, ProDjLinkBeat, RekordboxDeckState, RekordboxUniforms,
    parse_pro_dj_link_beat_packet,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

/// Default Pro DJ Link packet loss timeout before falling back to live audio BPM (3 seconds).
pub const DEFAULT_PRO_DJ_LINK_TIMEOUT_MS: u64 = 3_000;

/// Default phrase boundary interval in 4/4 bars (16 bars).
pub const DEFAULT_PHRASE_INTERVAL_BARS: u32 = 16;

/// Default cooldown window between consecutive auto-transitions (8 seconds).
pub const DEFAULT_TRANSITION_COOLDOWN_MS: u64 = 8_000;

/// Default high-energy threshold used to arm breakdown / energy-drop detection.
pub const DEFAULT_ENERGY_DROP_HIGH_THRESHOLD: f32 = 0.75;

/// Default low-energy threshold used to confirm a breakdown / energy drop.
pub const DEFAULT_ENERGY_DROP_LOW_THRESHOLD: f32 = 0.35;

/// Default single-step upward energy surge delta for drop-impact detection.
pub const DEFAULT_ENERGY_SURGE_DELTA: f32 = 0.45;

/// Stack buffer size for incoming UDP datagrams (Pro DJ Link beat packets are 96 bytes).
const MAX_UDP_PACKET_BYTES: usize = 512;

/// Upper bound on datagrams drained per `poll_packets` call to prevent frame-loop starvation.
const MAX_DATAGRAMS_PER_POLL: usize = 128;

/// Errors returned when binding or configuring a [`ProDjLinkListener`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProDjLinkListenerError {
    /// Failed to bind the UDP socket to the requested address.
    BindFailed {
        /// Debug representation of the target bind address.
        addr: String,
        /// Underlying OS error description.
        reason: String,
    },
    /// Failed to switch the bound UDP socket into non-blocking mode.
    NonBlockingFailed {
        /// Underlying OS error description.
        reason: String,
    },
    /// Failed to query the bound local socket address (e.g. ephemeral port).
    LocalAddrFailed {
        /// Underlying OS error description.
        reason: String,
    },
}

impl fmt::Display for ProDjLinkListenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BindFailed { addr, reason } => {
                write!(
                    f,
                    "failed to bind Pro DJ Link UDP socket on {addr}: {reason}"
                )
            }
            Self::NonBlockingFailed { reason } => {
                write!(
                    f,
                    "failed to configure non-blocking mode on Pro DJ Link UDP socket: {reason}"
                )
            }
            Self::LocalAddrFailed { reason } => {
                write!(
                    f,
                    "failed to resolve local address for Pro DJ Link UDP socket: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for ProDjLinkListenerError {}

/// Explicit synchronization lifecycle state machine for [`ProDjLinkListener`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ProDjLinkSyncState {
    /// No UDP socket is bound; listener is inactive.
    Disconnected,
    /// Socket is bound and awaiting the first valid Pro DJ Link beat packet.
    Listening {
        /// Bound local UDP port.
        port: u16,
    },
    /// Actively receiving valid Pro DJ Link beat packets from at least one deck.
    Synced {
        /// Currently active tempo master deck ID.
        master_deck: u8,
        /// Effective live BPM of the master deck.
        bpm: f32,
        /// Monotonic timestamp (ms) of the most recent valid beat packet.
        last_packet_ms: u64,
    },
    /// UDP beat stream timed out; visuals should fall back to live audio BPM analysis.
    FallbackToAudio {
        /// Last valid Pro DJ Link BPM observed before packet loss.
        last_known_bpm: f32,
        /// Monotonic timestamp (ms) when the timeout transition occurred.
        lost_at_ms: u64,
    },
}

impl ProDjLinkSyncState {
    /// Returns `true` when actively synced to live Pro DJ Link UDP packets.
    #[must_use]
    pub const fn is_synced(self) -> bool {
        matches!(self, Self::Synced { .. })
    }

    /// Returns `true` when operating in audio-BPM fallback mode after packet loss.
    #[must_use]
    pub const fn is_fallback(self) -> bool {
        matches!(self, Self::FallbackToAudio { .. })
    }

    /// Returns the most recent known BPM (`Some(bpm)` in `Synced` or `FallbackToAudio`).
    #[must_use]
    pub const fn active_bpm(self) -> Option<f32> {
        match self {
            Self::Synced { bpm, .. } => Some(bpm),
            Self::FallbackToAudio { last_known_bpm, .. } => Some(last_known_bpm),
            Self::Disconnected | Self::Listening { .. } => None,
        }
    }
}

/// Summary returned by [`ProDjLinkListener::poll_packets`].
#[derive(Debug, Clone, PartialEq)]
pub struct ProDjLinkPollResult {
    /// Valid Pro DJ Link beat packets decoded during this poll call.
    pub beats_received: Vec<ProDjLinkBeat>,
    /// Number of non-beat or malformed datagrams safely ignored during this poll call.
    pub ignored_packets: usize,
    /// Resulting synchronization state after draining packets and evaluating timeouts.
    pub state: ProDjLinkSyncState,
    /// Present when the active Master Deck changed during this poll (`(previous, new)`).
    pub master_changed: Option<(Option<u8>, u8)>,
}

/// Internal per-deck tracking entry.
#[derive(Debug, Clone)]
struct TrackedDeck {
    state: RekordboxDeckState,
    last_seen_ms: u64,
}

/// Non-blocking Pro DJ Link UDP beat listener with per-deck tracking, Master Deck
/// resolution, and automatic timeout fallback to audio BPM.
#[derive(Debug)]
pub struct ProDjLinkListener {
    socket: Option<UdpSocket>,
    local_addr: Option<SocketAddr>,
    state: ProDjLinkSyncState,
    timeout_ms: u64,
    decks: BTreeMap<u8, TrackedDeck>,
    master_deck: Option<u8>,
    last_known_bpm: f32,
    packets_received: u64,
    packets_ignored: u64,
}

impl ProDjLinkListener {
    /// Creates an inactive listener in [`ProDjLinkSyncState::Disconnected`].
    #[must_use]
    pub const fn disconnected() -> Self {
        Self {
            socket: None,
            local_addr: None,
            state: ProDjLinkSyncState::Disconnected,
            timeout_ms: DEFAULT_PRO_DJ_LINK_TIMEOUT_MS,
            decks: BTreeMap::new(),
            master_deck: None,
            last_known_bpm: 0.0,
            packets_received: 0,
            packets_ignored: 0,
        }
    }

    /// Binds a non-blocking UDP socket on the default Pro DJ Link beat address (`0.0.0.0:50001`).
    pub fn bind_default() -> Result<Self, ProDjLinkListenerError> {
        let addr = SocketAddr::from(([0, 0, 0, 0], PRO_DJ_LINK_BEAT_PORT));
        Self::bind(addr)
    }

    /// Binds a non-blocking UDP socket on `addr` (e.g. `"0.0.0.0:50001"` or `"127.0.0.1:0"`
    /// for ephemeral loopback ports in tests).
    pub fn bind<A: ToSocketAddrs + fmt::Debug>(addr: A) -> Result<Self, ProDjLinkListenerError> {
        let addr_label = format!("{addr:?}");
        let socket = UdpSocket::bind(&addr).map_err(|err| ProDjLinkListenerError::BindFailed {
            addr: addr_label,
            reason: err.to_string(),
        })?;
        socket
            .set_nonblocking(true)
            .map_err(|err| ProDjLinkListenerError::NonBlockingFailed {
                reason: err.to_string(),
            })?;
        let local_addr =
            socket
                .local_addr()
                .map_err(|err| ProDjLinkListenerError::LocalAddrFailed {
                    reason: err.to_string(),
                })?;
        let port = local_addr.port();

        Ok(Self {
            socket: Some(socket),
            local_addr: Some(local_addr),
            state: ProDjLinkSyncState::Listening { port },
            timeout_ms: DEFAULT_PRO_DJ_LINK_TIMEOUT_MS,
            decks: BTreeMap::new(),
            master_deck: None,
            last_known_bpm: 0.0,
            packets_received: 0,
            packets_ignored: 0,
        })
    }

    /// Overrides the packet-loss timeout in milliseconds (clamped to at least `1 ms`).
    #[must_use]
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms.max(1);
        self
    }

    /// Returns the bound local socket address, if currently bound.
    #[must_use]
    pub const fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    /// Returns the current [`ProDjLinkSyncState`].
    #[must_use]
    pub const fn state(&self) -> ProDjLinkSyncState {
        self.state
    }

    /// Returns the configured packet-loss timeout in milliseconds.
    #[must_use]
    pub const fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// Returns the currently resolved Master Deck ID, if any.
    #[must_use]
    pub const fn master_deck_id(&self) -> Option<u8> {
        self.master_deck
    }

    /// Returns a reference to the active Master Deck's [`RekordboxDeckState`], if any.
    #[must_use]
    pub fn master_deck(&self) -> Option<&RekordboxDeckState> {
        let id = self.master_deck?;
        self.decks.get(&id).map(|entry| &entry.state)
    }

    /// Returns a reference to a specific tracked deck by ID.
    #[must_use]
    pub fn deck(&self, deck_id: u8) -> Option<&RekordboxDeckState> {
        self.decks.get(&deck_id).map(|entry| &entry.state)
    }

    /// Returns the number of tracked decks seen on the network.
    #[must_use]
    pub fn deck_count(&self) -> usize {
        self.decks.len()
    }

    /// Returns total valid beat packets received since creation.
    #[must_use]
    pub const fn packets_received(&self) -> u64 {
        self.packets_received
    }

    /// Returns total non-beat or malformed UDP datagrams ignored since creation.
    #[must_use]
    pub const fn packets_ignored(&self) -> u64 {
        self.packets_ignored
    }

    /// Explicitly designates `deck_id` as the active Master Deck (e.g. from operator
    /// selection or external status metadata), updating `is_master` flags across all decks.
    ///
    /// Returns `true` if `deck_id` exists in the tracked deck table and was promoted.
    pub fn set_master_deck(&mut self, deck_id: u8) -> bool {
        if !self.decks.contains_key(&deck_id) {
            return false;
        }
        self.master_deck = Some(deck_id);
        self.sync_master_flags();
        if let Some(entry) = self.decks.get(&deck_id) {
            let bpm = entry.state.bpm;
            if bpm.is_finite() && bpm > 0.0 {
                self.last_known_bpm = bpm;
            }
            if let ProDjLinkSyncState::Synced { last_packet_ms, .. } = self.state {
                self.state = ProDjLinkSyncState::Synced {
                    master_deck: deck_id,
                    bpm,
                    last_packet_ms,
                };
            }
        }
        true
    }

    /// Sets normalized phrase energy (`0.0..=1.0`) on a tracked deck (for example,
    /// from audio envelope or Rekordbox phrase metadata).
    pub fn set_deck_phrase_energy(&mut self, deck_id: u8, energy: f32) -> bool {
        if let Some(entry) = self.decks.get_mut(&deck_id) {
            entry.state.set_phrase_energy(energy);
            true
        } else {
            false
        }
    }

    /// Advances all tracked deck phases and cue pulses smoothly by `dt_secs` between
    /// incoming UDP beat packets.
    pub fn advance(&mut self, dt_secs: f32) {
        for entry in self.decks.values_mut() {
            entry.state.advance(dt_secs);
        }
    }

    /// Closes the UDP socket and transitions cleanly to [`ProDjLinkSyncState::Disconnected`].
    pub fn disconnect(&mut self) {
        self.socket = None;
        self.local_addr = None;
        self.master_deck = None;
        self.sync_master_flags();
        self.state = ProDjLinkSyncState::Disconnected;
    }

    /// Exports the current Master Deck's [`RekordboxUniforms`] snapshot for shader binding.
    ///
    /// - In [`ProDjLinkSyncState::Synced`], returns the active Master Deck's exact uniforms.
    /// - In [`ProDjLinkSyncState::FallbackToAudio`], clears `deck_is_master` / `deck_playing`
    ///   while preserving `last_known_bpm` so downstream audio-BPM fallback has a warm seed.
    /// - In [`ProDjLinkSyncState::Disconnected`] or [`ProDjLinkSyncState::Listening`],
    ///   returns [`RekordboxUniforms::default()`].
    #[must_use]
    pub fn master_uniforms(&self) -> RekordboxUniforms {
        match self.state {
            ProDjLinkSyncState::Synced { .. } => self
                .master_deck()
                .map_or_else(RekordboxUniforms::default, RekordboxDeckState::to_uniforms),
            ProDjLinkSyncState::FallbackToAudio { last_known_bpm, .. } => RekordboxUniforms {
                deck_bpm: if last_known_bpm.is_finite() && last_known_bpm >= 0.0 {
                    last_known_bpm
                } else {
                    0.0
                },
                ..RekordboxUniforms::default()
            },
            ProDjLinkSyncState::Disconnected | ProDjLinkSyncState::Listening { .. } => {
                RekordboxUniforms::default()
            }
        }
    }

    /// Drains pending non-blocking UDP datagrams, parses valid Pro DJ Link beat packets,
    /// updates per-deck state and Master Deck assignment, and transitions to
    /// [`ProDjLinkSyncState::FallbackToAudio`] if no packet has arrived within `timeout_ms`.
    pub fn poll_packets(&mut self, now_ms: u64) -> ProDjLinkPollResult {
        let previous_master = self.master_deck;
        let mut beats_received = Vec::new();
        let mut ignored_packets = 0_usize;

        if let Some(ref socket) = self.socket {
            let mut buf = [0_u8; MAX_UDP_PACKET_BYTES];
            for _ in 0..MAX_DATAGRAMS_PER_POLL {
                match socket.recv_from(&mut buf) {
                    Ok((len, _peer)) => {
                        let datagram = &buf[..len];
                        if let Some(beat) = parse_pro_dj_link_beat_packet(datagram) {
                            beats_received.push(beat);
                        } else {
                            ignored_packets = ignored_packets.saturating_add(1);
                            self.packets_ignored = self.packets_ignored.saturating_add(1);
                        }
                    }
                    Err(err)
                        if err.kind() == ErrorKind::WouldBlock
                            || err.kind() == ErrorKind::Interrupted =>
                    {
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        }

        if !beats_received.is_empty() {
            for beat in &beats_received {
                self.apply_beat_internal(beat, now_ms);
            }
            self.resolve_master_deck(now_ms);
            if let Some(master_id) = self.master_deck {
                let bpm = self
                    .decks
                    .get(&master_id)
                    .map_or(self.last_known_bpm, |entry| entry.state.bpm);
                if bpm.is_finite() && bpm > 0.0 {
                    self.last_known_bpm = bpm;
                }
                self.state = ProDjLinkSyncState::Synced {
                    master_deck: master_id,
                    bpm,
                    last_packet_ms: now_ms,
                };
            }
        } else {
            self.evaluate_timeout(now_ms);
        }

        let master_changed = match self.master_deck {
            Some(new_master) if previous_master != Some(new_master) => {
                Some((previous_master, new_master))
            }
            _ => None,
        };

        ProDjLinkPollResult {
            beats_received,
            ignored_packets,
            state: self.state,
            master_changed,
        }
    }

    /// Feeds a pre-decoded [`ProDjLinkBeat`] directly into the listener state machine
    /// without going through a UDP socket (useful for deterministic replay or testing).
    pub fn ingest_beat(&mut self, beat: &ProDjLinkBeat, now_ms: u64) {
        self.apply_beat_internal(beat, now_ms);
        self.resolve_master_deck(now_ms);
        if let Some(master_id) = self.master_deck {
            let bpm = self
                .decks
                .get(&master_id)
                .map_or(self.last_known_bpm, |entry| entry.state.bpm);
            if bpm.is_finite() && bpm > 0.0 {
                self.last_known_bpm = bpm;
            }
            self.state = ProDjLinkSyncState::Synced {
                master_deck: master_id,
                bpm,
                last_packet_ms: now_ms,
            };
        }
    }

    fn apply_beat_internal(&mut self, beat: &ProDjLinkBeat, now_ms: u64) {
        self.packets_received = self.packets_received.saturating_add(1);
        let entry = self
            .decks
            .entry(beat.player_number)
            .or_insert_with(|| TrackedDeck {
                state: RekordboxDeckState::new(beat.player_number),
                last_seen_ms: now_ms,
            });
        entry.state.apply_beat_packet(beat);
        entry.last_seen_ms = now_ms;
    }

    /// Keeps the current master deck sticky while it remains playing and fresh (avoiding
    /// master thrashing during multi-deck blends), and promotes the freshest playing deck
    /// when no master is set or the previous master stops / times out.
    fn resolve_master_deck(&mut self, now_ms: u64) {
        let current_valid = self.master_deck.is_some_and(|id| {
            self.decks.get(&id).is_some_and(|entry| {
                entry.state.playing && now_ms.saturating_sub(entry.last_seen_ms) < self.timeout_ms
            })
        });

        if !current_valid {
            let best_playing = self
                .decks
                .iter()
                .filter(|(_, entry)| {
                    entry.state.playing
                        && now_ms.saturating_sub(entry.last_seen_ms) < self.timeout_ms
                })
                .max_by(|(id_a, a), (id_b, b)| {
                    a.last_seen_ms
                        .cmp(&b.last_seen_ms)
                        .then_with(|| id_b.cmp(id_a))
                })
                .map(|(&id, _)| id);

            let fallback_any = self
                .decks
                .iter()
                .max_by(|(id_a, a), (id_b, b)| {
                    a.last_seen_ms
                        .cmp(&b.last_seen_ms)
                        .then_with(|| id_b.cmp(id_a))
                })
                .map(|(&id, _)| id);

            self.master_deck = best_playing.or(fallback_any);
        }

        self.sync_master_flags();
    }

    fn sync_master_flags(&mut self) {
        let master = self.master_deck;
        for (&id, entry) in &mut self.decks {
            entry.state.set_master(master == Some(id));
        }
    }

    fn evaluate_timeout(&mut self, now_ms: u64) {
        if let ProDjLinkSyncState::Synced {
            bpm,
            last_packet_ms,
            ..
        } = self.state
        {
            if now_ms.saturating_sub(last_packet_ms) >= self.timeout_ms {
                let last_known_bpm = if bpm.is_finite() && bpm > 0.0 {
                    bpm
                } else {
                    self.last_known_bpm
                };
                self.last_known_bpm = last_known_bpm;
                self.master_deck = None;
                self.sync_master_flags();
                self.state = ProDjLinkSyncState::FallbackToAudio {
                    last_known_bpm,
                    lost_at_ms: now_ms,
                };
            }
        }
    }
}

/// Deterministic auto-transition trigger emitted by [`AutoTransitionPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionTrigger {
    /// No transition should fire on this evaluation.
    None,
    /// Active master deck crossed a configured musical phrase boundary (e.g. 16 or 32 bars).
    PhraseBoundary {
        /// Cumulative completed bar count on the active master deck when the phrase fired.
        bar_count: u32,
    },
    /// Tempo master switched from one deck to another (track-load / mix handoff).
    MasterDeckChanged {
        /// Previously active master deck ID.
        from_deck: u8,
        /// Newly active master deck ID.
        to_deck: u8,
    },
    /// Phrase energy underwent a sharp breakdown drop or drop-impact surge.
    EnergyDrop,
}

/// Configuration for [`AutoTransitionPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutoTransitionConfig {
    /// Number of 4/4 bars per musical phrase boundary (e.g. `16` or `32`; clamped `>= 1`).
    pub phrase_interval_bars: u32,
    /// Minimum milliseconds between consecutive fired transitions so fills never double-fire.
    pub cooldown_ms: u64,
    /// Whether phrase boundary triggers are enabled.
    pub enable_phrase_boundary: bool,
    /// Whether Master Deck handoff triggers are enabled.
    pub enable_master_deck_change: bool,
    /// Whether energy drop / breakdown triggers are enabled.
    pub enable_energy_drop: bool,
    /// High phrase energy threshold (`0.0..=1.0`) that arms a high-to-low energy drop.
    pub energy_drop_high_threshold: f32,
    /// Low phrase energy threshold (`0.0..=1.0`) that confirms a high-to-low energy drop.
    pub energy_drop_low_threshold: f32,
    /// Upward phrase energy jump (`0.0..=1.0`) that confirms a low-to-high drop impact.
    pub energy_surge_delta: f32,
}

impl Default for AutoTransitionConfig {
    fn default() -> Self {
        Self {
            phrase_interval_bars: DEFAULT_PHRASE_INTERVAL_BARS,
            cooldown_ms: DEFAULT_TRANSITION_COOLDOWN_MS,
            enable_phrase_boundary: true,
            enable_master_deck_change: true,
            enable_energy_drop: true,
            energy_drop_high_threshold: DEFAULT_ENERGY_DROP_HIGH_THRESHOLD,
            energy_drop_low_threshold: DEFAULT_ENERGY_DROP_LOW_THRESHOLD,
            energy_surge_delta: DEFAULT_ENERGY_SURGE_DELTA,
        }
    }
}

/// Deterministic hands-free auto-transition planner driven by Pro DJ Link deck state.
#[derive(Debug, Clone)]
pub struct AutoTransitionPlanner {
    config: AutoTransitionConfig,
    last_master_deck: Option<u8>,
    last_bar_beat: u8,
    bar_count: u32,
    last_trigger_ms: Option<u64>,
    previous_phrase_energy: Option<f32>,
}

impl AutoTransitionPlanner {
    /// Creates a new planner with the given [`AutoTransitionConfig`].
    #[must_use]
    pub fn new(mut config: AutoTransitionConfig) -> Self {
        config.phrase_interval_bars = config.phrase_interval_bars.max(1);
        Self {
            config,
            last_master_deck: None,
            last_bar_beat: 0,
            bar_count: 0,
            last_trigger_ms: None,
            previous_phrase_energy: None,
        }
    }

    /// Creates a planner with default settings (16-bar phrases, 8-second cooldown).
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(AutoTransitionConfig::default())
    }

    /// Returns the active [`AutoTransitionConfig`].
    #[must_use]
    pub const fn config(&self) -> AutoTransitionConfig {
        self.config
    }

    /// Updates the phrase boundary interval in bars (clamped to at least `1`).
    pub fn set_phrase_interval_bars(&mut self, bars: u32) {
        self.config.phrase_interval_bars = bars.max(1);
    }

    /// Updates the minimum cooldown window in milliseconds.
    pub const fn set_cooldown_ms(&mut self, cooldown_ms: u64) {
        self.config.cooldown_ms = cooldown_ms;
    }

    /// Returns the cumulative completed bar count on the current Master Deck.
    #[must_use]
    pub const fn bar_count(&self) -> u32 {
        self.bar_count
    }

    /// Returns the timestamp (ms) of the last fired transition, if any.
    #[must_use]
    pub const fn last_trigger_ms(&self) -> Option<u64> {
        self.last_trigger_ms
    }

    /// Returns `true` if a transition fired within `cooldown_ms` of `now_ms`.
    #[must_use]
    pub const fn is_in_cooldown(&self, now_ms: u64) -> bool {
        match self.last_trigger_ms {
            Some(last_ms) => now_ms.saturating_sub(last_ms) < self.config.cooldown_ms,
            None => false,
        }
    }

    /// Resets bar tracking, energy history, and cooldown state.
    pub const fn reset(&mut self) {
        self.last_master_deck = None;
        self.last_bar_beat = 0;
        self.bar_count = 0;
        self.last_trigger_ms = None;
        self.previous_phrase_energy = None;
    }

    /// Convenience helper that evaluates the active Master Deck of a [`ProDjLinkListener`].
    ///
    /// Returns [`TransitionTrigger::None`] when the listener is not in
    /// [`ProDjLinkSyncState::Synced`].
    pub fn evaluate_listener(
        &mut self,
        listener: &ProDjLinkListener,
        now_ms: u64,
    ) -> TransitionTrigger {
        if !listener.state().is_synced() {
            return TransitionTrigger::None;
        }
        match listener.master_deck() {
            Some(deck) => self.evaluate(deck, now_ms),
            None => TransitionTrigger::None,
        }
    }

    /// Deterministically evaluates beat, bar, master-deck, and phrase-energy state to
    /// decide whether an automatic visual transition should fire at `now_ms`.
    ///
    /// Evaluation priority:
    /// 1. [`TransitionTrigger::MasterDeckChanged`] (when tempo master switches decks)
    /// 2. [`TransitionTrigger::PhraseBoundary`] (when completed bars hit `phrase_interval_bars`)
    /// 3. [`TransitionTrigger::EnergyDrop`] (on a sharp breakdown or drop impact)
    ///
    /// All triggers respect the configured `cooldown_ms` guard.
    pub fn evaluate(&mut self, deck: &RekordboxDeckState, now_ms: u64) -> TransitionTrigger {
        if !deck.is_master {
            return TransitionTrigger::None;
        }

        let current_energy = if deck.phrase_energy.is_finite() {
            deck.phrase_energy.clamp(0.0, 1.0)
        } else {
            0.0
        };

        // 1. Track master deck initialization and handoffs.
        match self.last_master_deck {
            None => {
                self.last_master_deck = Some(deck.deck_id);
                self.last_bar_beat = deck.bar_beat;
                self.bar_count = 0;
                self.previous_phrase_energy = Some(current_energy);
                return TransitionTrigger::None;
            }
            Some(prev_deck) if prev_deck != deck.deck_id => {
                self.last_master_deck = Some(deck.deck_id);
                self.last_bar_beat = deck.bar_beat;
                self.bar_count = 0;
                self.previous_phrase_energy = Some(current_energy);
                if self.config.enable_master_deck_change && !self.is_in_cooldown(now_ms) {
                    self.last_trigger_ms = Some(now_ms);
                    return TransitionTrigger::MasterDeckChanged {
                        from_deck: prev_deck,
                        to_deck: deck.deck_id,
                    };
                }
                return TransitionTrigger::None;
            }
            Some(_) => {}
        }

        // 2. Detect 4/4 bar wrap (beats 2..=4 -> beat 1) without double-counting repeated frames.
        let crossed_new_bar = deck.bar_beat == 1 && self.last_bar_beat > 1;
        if deck.bar_beat > 0 {
            self.last_bar_beat = deck.bar_beat;
        }
        if crossed_new_bar {
            self.bar_count = self.bar_count.saturating_add(1);
        }

        // 3. Evaluate energy drop / drop-impact delta before updating previous_phrase_energy.
        let energy_drop_detected = self.previous_phrase_energy.is_some_and(|prev| {
            let breakdown = prev >= self.config.energy_drop_high_threshold
                && current_energy <= self.config.energy_drop_low_threshold;
            let drop_surge = (current_energy - prev) >= self.config.energy_surge_delta;
            breakdown || drop_surge
        });
        self.previous_phrase_energy = Some(current_energy);

        // Priority 2: Phrase boundary trigger.
        if crossed_new_bar
            && self.config.enable_phrase_boundary
            && self.bar_count > 0
            && self.bar_count % self.config.phrase_interval_bars == 0
            && !self.is_in_cooldown(now_ms)
        {
            self.last_trigger_ms = Some(now_ms);
            return TransitionTrigger::PhraseBoundary {
                bar_count: self.bar_count,
            };
        }

        // Priority 3: Energy drop trigger.
        if energy_drop_detected && self.config.enable_energy_drop && !self.is_in_cooldown(now_ms) {
            self.last_trigger_ms = Some(now_ms);
            return TransitionTrigger::EnergyDrop;
        }

        TransitionTrigger::None
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-REKORDBOX-001 (T1) — Live Pro DJ Link UDP loopback listener, Master Deck handoff, phrase/drop auto-transition planner, and 3-second audio-BPM fallback pins.

    use super::*;
    use crate::rekordbox_bridge::{
        PRO_DJ_LINK_BEAT_PACKET_MIN_LEN, PRO_DJ_LINK_BEAT_PACKET_TYPE, PRO_DJ_LINK_MAGIC,
    };
    use std::thread;
    use std::time::Duration;

    /// Builds a valid 96-byte `Qspt1WmJOL` Pro DJ Link beat packet (`0x28`).
    fn build_beat_packet(
        device_name: &str,
        player_number: u8,
        bpm_centi: u16,
        pitch_raw: u32,
        beat_within_bar: u8,
        next_beat_ms: u32,
        next_bar_ms: u32,
    ) -> [u8; PRO_DJ_LINK_BEAT_PACKET_MIN_LEN] {
        let mut pkt = [0_u8; PRO_DJ_LINK_BEAT_PACKET_MIN_LEN];
        pkt[0..10].copy_from_slice(PRO_DJ_LINK_MAGIC);
        pkt[0x0a] = PRO_DJ_LINK_BEAT_PACKET_TYPE;
        let name_bytes = device_name.as_bytes();
        let copy_len = name_bytes.len().min(20);
        pkt[0x0b..0x0b + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        pkt[0x1f] = 0x01;
        pkt[0x21] = player_number;
        pkt[0x22..0x24].copy_from_slice(&60_u16.to_be_bytes());
        pkt[0x24..0x28].copy_from_slice(&next_beat_ms.to_be_bytes());
        pkt[0x2c..0x30].copy_from_slice(&next_bar_ms.to_be_bytes());
        pkt[0x54..0x58].copy_from_slice(&pitch_raw.to_be_bytes());
        pkt[0x58..0x5a].copy_from_slice(&bpm_centi.to_be_bytes());
        pkt[0x5b] = beat_within_bar;
        pkt[0x5f] = player_number;
        pkt
    }

    /// Polls a loopback listener briefly until at least one beat packet arrives or retries exhaust.
    fn poll_until_received(listener: &mut ProDjLinkListener, now_ms: u64) -> ProDjLinkPollResult {
        for _ in 0..20 {
            let result = listener.poll_packets(now_ms);
            if !result.beats_received.is_empty() {
                return result;
            }
            thread::sleep(Duration::from_millis(2));
        }
        listener.poll_packets(now_ms)
    }

    #[test]
    fn live_loopback_udp_beat_packet_syncs_and_emits_exact_uniforms() {
        let mut listener = match ProDjLinkListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(err) => panic!("ephemeral loopback UDP bind must succeed: {err}"),
        };
        let target_addr = match listener.local_addr() {
            Some(addr) => addr,
            None => panic!("bound listener must expose local_addr"),
        };
        assert_eq!(
            listener.state(),
            ProDjLinkSyncState::Listening {
                port: target_addr.port()
            }
        );

        let sender = match UdpSocket::bind("127.0.0.1:0") {
            Ok(s) => s,
            Err(err) => panic!("ephemeral sender bind must succeed: {err}"),
        };

        // Send a non-beat keepalive datagram first to verify malformed/non-beat packets are ignored.
        let mut noise = [0_u8; 32];
        noise[0..10].copy_from_slice(PRO_DJ_LINK_MAGIC);
        noise[0x0a] = 0x06;
        if let Err(err) = sender.send_to(&noise, target_addr) {
            panic!("loopback send of keepalive noise failed: {err}");
        }

        // Send a real 96-byte Pro DJ Link beat packet: CDJ-3000 Deck 1, 128.00 BPM @ 0% pitch, bar downbeat.
        let pkt = build_beat_packet("CDJ-3000", 1, 12_800, 0x0010_0000, 1, 469, 1875);
        if let Err(err) = sender.send_to(&pkt, target_addr) {
            panic!("loopback send of beat packet failed: {err}");
        }

        let poll = poll_until_received(&mut listener, 1_000);
        assert_eq!(poll.beats_received.len(), 1);
        assert_eq!(listener.packets_received(), 1);
        assert_eq!(listener.packets_ignored(), 1);
        assert_eq!(poll.master_changed, Some((None, 1)));

        match listener.state() {
            ProDjLinkSyncState::Synced {
                master_deck,
                bpm,
                last_packet_ms,
            } => {
                assert_eq!(master_deck, 1);
                assert!((bpm - 128.0).abs() < 1e-4);
                assert_eq!(last_packet_ms, 1_000);
            }
            other => panic!("expected Synced state, got {other:?}"),
        }

        assert!(listener.set_deck_phrase_energy(1, 0.8));
        let uniforms = listener.master_uniforms();
        assert!((uniforms.deck_bpm - 128.0).abs() < 1e-4);
        assert_eq!(uniforms.deck_beat_phase, 0.0);
        assert_eq!(uniforms.deck_bar_beat, 1.0);
        assert_eq!(uniforms.deck_is_master, 1.0);
        assert_eq!(uniforms.deck_playing, 1.0);
        assert!((uniforms.deck_phrase_energy - 0.8).abs() < 1e-6);
        assert_eq!(uniforms.deck_cue_pulse, 1.0);
    }

    #[test]
    fn master_deck_switch_triggers_master_deck_changed_transition() {
        let mut listener = match ProDjLinkListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(err) => panic!("loopback bind must succeed: {err}"),
        };
        let target_addr = match listener.local_addr() {
            Some(addr) => addr,
            None => panic!("local_addr must exist"),
        };
        let sender = match UdpSocket::bind("127.0.0.1:0") {
            Ok(s) => s,
            Err(err) => panic!("sender bind must succeed: {err}"),
        };

        let mut planner = AutoTransitionPlanner::with_defaults();

        // Deck 1 starts playing and becomes initial master (no transition on cold start).
        let pkt_deck1 = build_beat_packet("CDJ-3000", 1, 12_600, 0x0010_0000, 1, 476, 1904);
        if let Err(err) = sender.send_to(&pkt_deck1, target_addr) {
            panic!("send deck 1 packet failed: {err}");
        }
        let _ = poll_until_received(&mut listener, 1_000);
        assert_eq!(listener.master_deck_id(), Some(1));
        assert_eq!(
            planner.evaluate_listener(&listener, 1_000),
            TransitionTrigger::None
        );

        // Deck 2 enters the mix while Deck 1 is still active; master stays sticky on Deck 1.
        let pkt_deck2 = build_beat_packet("CDJ-3000", 2, 12_800, 0x0010_0000, 1, 469, 1875);
        if let Err(err) = sender.send_to(&pkt_deck2, target_addr) {
            panic!("send deck 2 packet failed: {err}");
        }
        let _ = poll_until_received(&mut listener, 1_500);
        assert_eq!(listener.master_deck_id(), Some(1));
        assert_eq!(
            planner.evaluate_listener(&listener, 1_500),
            TransitionTrigger::None
        );

        // Operator hands master over to Deck 2 -> MasterDeckChanged fires immediately.
        assert!(listener.set_master_deck(2));
        assert_eq!(listener.master_deck_id(), Some(2));
        assert_eq!(
            planner.evaluate_listener(&listener, 2_000),
            TransitionTrigger::MasterDeckChanged {
                from_deck: 1,
                to_deck: 2,
            }
        );

        // Immediate switch back to Deck 1 inside the 8s cooldown must be suppressed.
        assert!(listener.set_master_deck(1));
        assert_eq!(
            planner.evaluate_listener(&listener, 3_000),
            TransitionTrigger::None
        );
    }

    #[test]
    fn sixteen_bar_phrase_boundary_triggers_with_cooldown_enforcement() {
        let mut planner = AutoTransitionPlanner::new(AutoTransitionConfig {
            phrase_interval_bars: 16,
            cooldown_ms: 8_000,
            ..AutoTransitionConfig::default()
        });

        let mut deck = RekordboxDeckState::new(1);
        deck.set_master(true);
        deck.set_phrase_energy(0.5);

        // Cold-start on bar 1 beat 1.
        let mut now_ms = 0_u64;
        let beat_ms = 500_u64; // 120 BPM = 500 ms per beat, 2000 ms per bar.
        let pkt_init = build_beat_packet("rekordbox", 1, 12_000, 0x0010_0000, 1, 500, 2000);
        if let Some(beat) = parse_pro_dj_link_beat_packet(&pkt_init) {
            deck.apply_beat_packet(&beat);
        }
        assert_eq!(planner.evaluate(&deck, now_ms), TransitionTrigger::None);

        // Advance through 16 full bars (beats 2, 3, 4, then 1 for each bar).
        let mut fired_at = Vec::new();
        for _bar in 1..=16 {
            for beat_in_bar in [2_u8, 3, 4, 1] {
                now_ms += beat_ms;
                let pkt =
                    build_beat_packet("rekordbox", 1, 12_000, 0x0010_0000, beat_in_bar, 500, 2000);
                if let Some(beat) = parse_pro_dj_link_beat_packet(&pkt) {
                    deck.apply_beat_packet(&beat);
                }
                // Repeated evaluation on the same beat must never double-count bars.
                let t1 = planner.evaluate(&deck, now_ms);
                let t2 = planner.evaluate(&deck, now_ms + 16);
                assert_eq!(t2, TransitionTrigger::None);
                if t1 != TransitionTrigger::None {
                    fired_at.push((now_ms, t1));
                }
            }
        }

        assert_eq!(planner.bar_count(), 16);
        assert_eq!(
            fired_at,
            vec![(32_000, TransitionTrigger::PhraseBoundary { bar_count: 16 })]
        );

        // Verify cooldown enforcement: shorten interval to 1 bar and advance 1 bar (2000ms < 8000ms cooldown).
        planner.set_phrase_interval_bars(1);
        for beat_in_bar in [2_u8, 3, 4, 1] {
            now_ms += beat_ms;
            let pkt =
                build_beat_packet("rekordbox", 1, 12_000, 0x0010_0000, beat_in_bar, 500, 2000);
            if let Some(beat) = parse_pro_dj_link_beat_packet(&pkt) {
                deck.apply_beat_packet(&beat);
            }
            assert_eq!(planner.evaluate(&deck, now_ms), TransitionTrigger::None);
        }
        assert_eq!(planner.bar_count(), 17);
        assert!(planner.is_in_cooldown(now_ms));

        // Advance past the 8000ms cooldown (to bar 20, now_ms = 40_000) -> fires again cleanly.
        let mut post_cooldown_trigger = TransitionTrigger::None;
        for _bar in 18..=20 {
            for beat_in_bar in [2_u8, 3, 4, 1] {
                now_ms += beat_ms;
                let pkt =
                    build_beat_packet("rekordbox", 1, 12_000, 0x0010_0000, beat_in_bar, 500, 2000);
                if let Some(beat) = parse_pro_dj_link_beat_packet(&pkt) {
                    deck.apply_beat_packet(&beat);
                }
                let trig = planner.evaluate(&deck, now_ms);
                if trig != TransitionTrigger::None {
                    post_cooldown_trigger = trig;
                }
            }
        }
        assert_eq!(
            post_cooldown_trigger,
            TransitionTrigger::PhraseBoundary { bar_count: 20 }
        );
    }

    #[test]
    fn packet_timeout_transitions_synced_to_fallback_to_audio_and_recovers() {
        let mut listener = match ProDjLinkListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(err) => panic!("loopback bind must succeed: {err}"),
        };
        assert_eq!(listener.timeout_ms(), DEFAULT_PRO_DJ_LINK_TIMEOUT_MS);

        let target_addr = match listener.local_addr() {
            Some(addr) => addr,
            None => panic!("local_addr must exist"),
        };
        let sender = match UdpSocket::bind("127.0.0.1:0") {
            Ok(s) => s,
            Err(err) => panic!("sender bind must succeed: {err}"),
        };

        // Sync at t = 10_000 ms with 132.00 BPM on Deck 3.
        let pkt = build_beat_packet("CDJ-3000", 3, 13_200, 0x0010_0000, 1, 455, 1818);
        if let Err(err) = sender.send_to(&pkt, target_addr) {
            panic!("send packet failed: {err}");
        }
        let _ = poll_until_received(&mut listener, 10_000);
        assert!(listener.state().is_synced());

        // At t = 12_999 ms (2999 ms elapsed < 3000 ms timeout), must still be Synced.
        let before_timeout = listener.poll_packets(12_999);
        assert!(before_timeout.state.is_synced());

        // At t = 13_000 ms (exactly 3000 ms elapsed), must transition cleanly to FallbackToAudio.
        let at_timeout = listener.poll_packets(13_000);
        assert_eq!(
            at_timeout.state,
            ProDjLinkSyncState::FallbackToAudio {
                last_known_bpm: 132.0,
                lost_at_ms: 13_000,
            }
        );
        assert!(listener.state().is_fallback());
        assert_eq!(listener.state().active_bpm(), Some(132.0));

        // Uniforms in fallback mode preserve last_known_bpm while clearing master/playing flags.
        let fallback_uniforms = listener.master_uniforms();
        assert!((fallback_uniforms.deck_bpm - 132.0).abs() < 1e-4);
        assert_eq!(fallback_uniforms.deck_is_master, 0.0);
        assert_eq!(fallback_uniforms.deck_playing, 0.0);

        // When UDP traffic resumes at t = 15_000 ms, listener recovers cleanly to Synced.
        let pkt_resume = build_beat_packet("CDJ-3000", 3, 13_400, 0x0010_0000, 2, 448, 1343);
        if let Err(err) = sender.send_to(&pkt_resume, target_addr) {
            panic!("send resume packet failed: {err}");
        }
        let recovered = poll_until_received(&mut listener, 15_000);
        assert_eq!(
            recovered.state,
            ProDjLinkSyncState::Synced {
                master_deck: 3,
                bpm: 134.0,
                last_packet_ms: 15_000,
            }
        );
    }

    #[test]
    fn energy_drop_trigger_fires_on_breakdown_and_respects_cooldown() {
        let mut planner = AutoTransitionPlanner::with_defaults();
        let mut deck = RekordboxDeckState::new(1);
        deck.set_master(true);
        deck.bar_beat = 1;
        deck.set_phrase_energy(0.85);

        // Cold start seeds energy without firing.
        assert_eq!(planner.evaluate(&deck, 1_000), TransitionTrigger::None);

        // Sharp breakdown drop from 0.85 -> 0.20 fires EnergyDrop.
        deck.bar_beat = 2;
        deck.set_phrase_energy(0.20);
        assert_eq!(
            planner.evaluate(&deck, 2_000),
            TransitionTrigger::EnergyDrop
        );

        // Immediate surge back up to 0.90 inside 8s cooldown is suppressed.
        deck.bar_beat = 3;
        deck.set_phrase_energy(0.90);
        assert_eq!(planner.evaluate(&deck, 3_000), TransitionTrigger::None);
    }
}
