// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine runtime.
//!
//! Exposes the canonical state core ([`state`]) with the ADR-004
//! §Decision sub-stores and the command-bus surface ([`command_bus`]):
//! typed [`Command`] and [`Event`] envelopes, the [`CommandChannel`]
//! pluggable transport trait, a crossbeam-channel-backed
//! [`CrossbeamChannel`] implementation, and the [`InMemoryBus`]
//! `CommandBus` that composes a channel with a monotonic [`EventSeq`]
//! counter. Post-Phase-3 (`ec10ee7`) the crate also carries the
//! deterministic state-replay path ([`state_replay`], Phase-3 wave PR 1
//! / `EV-STATE-REPLAY`), the V1↔V2 forward-migration envelope
//! ([`state_migration`], Phase-3 wave PR 4 / `EV-STATE-MIGRATION`),
//! and the observable-subsystem proxy benchmarks under
//! `engine/runtime/benches/budgets.rs` (Phase-3 wave PR 6 /
//! `EV-PERF-BUDGETS` Path B nightly criterion harness). The
//! render-graph implementation remains reserved as a future module in
//! this crate, to be extracted only when incremental-compile savings
//! or license-boundary concerns justify.
//!
//! Internal state fields and sub-stores remain crate-private
//! (`pub(crate)`) so downstream crates cannot bypass the command bus;
//! only the public command / event / trait surface is exported for
//! sibling crates to consume.
//!
//! Slice-3 (post-Phase-3) adds the [`scheduler`] module: the
//! foundational scheduler-tick path per ADR-001 §Decision and ADR-006
//! §Decision. [`scheduler::Scheduler::tick`] composes
//! [`command_bus::CommandBus::commit_all_into`] with an explicit
//! [`state::State::frame`] increment so the canonical frame counter
//! advances on the state-commit thread alongside the command-bus
//! drain. The scheduler is purely additive: existing callers that
//! drive state mutation through `commit_all_into` directly continue
//! to observe `frame == 0` (the EV-STATE-REPLAY row 32 +
//! EV-PRESET-PLAYLIST row 34 T2 corpora pin this invariant). PR
//! `#321` (`5c1aeb3`) landed the module surface + T1 drift pins;
//! PRs `#326` (`6f994ab`) / `#328` (`6f46a35`) / `#329` (`d888440`) /
//! `#330` (`642bada`) landed the T2 corpus + observability-emit /
//! audio-pull / render-dispatch subsystems; the wave closed
//! 2026-05-01 with `EV-SCHEDULER-TICK` at `Pass` (row 38).
//!
//! Phase-4d Wave 4d.3 (post-Phase-3) adds the [`crash_resume`]
//! module: the `EV-CRASH-RESUME` partial-advance state-restoration
//! adapter (row 80). The adapter composes the existing Phase-3
//! [`state_migration`] V2 envelope with a disk-backed save /
//! recover cycle plus per-class restoration helpers
//! ([`crash_resume::restore_class_a`] through
//! [`crash_resume::restore_class_g`] +
//! [`crash_resume::restore_all_supported_classes`]). The wave
//! cluster closed 2026-05-07 at PR `#378` / `9cbb428` with all
//! seven restoration classes a/b/c/d/e/f/g at `Pass`: class-a +
//! class-b (foundational), class-c (playlist position + iteration
//! mode), class-d (OSC/MIDI session bindings; closes the
//! deferred-pending-state-shape gap by extending [`state::State`]
//! with `SessionBindings` + the V2 envelope
//! `EXT_ID_V2_SESSION_BINDINGS` extension without breaking the
//! V1 → V2 forward-only migration per Phase-3
//! `EV-STATE-MIGRATION`), class-e (audio input device + threshold),
//! class-f (operator meta-controls), and class-g (render
//! resolution + FPS target). The wave is purely additive: existing
//! EV-STATE-REPLAY (row 32), EV-PRESET-PLAYLIST (row 34),
//! EV-STATE-MIGRATION (row 33), and EV-SCHEDULER-TICK (row 38)
//! goldens are preserved verbatim. The full crash-injection drill
//! (controlled `SIGKILL` / `SIGSEGV` / panic-in-render-thread /
//! OOM at three wallclock points in a 10-minute scripted show)
//! plus the time-to-resume + visible-artifact-window p50/p95
//! measurement budgets are deferred to a future-wave full-injection
//! harness composing on top of the synthetic restore-helper
//! apparatus per the Phase-4d cumulative narrowing.
//!
//! Slice-5.3 (Phase-2, behind the `script-bridge` Cargo feature) adds
//! the [`script_bridge`] module: an adapter that implements the
//! slice-5.1 `chromaticity_script::ScriptCommandSender` trait against
//! a host-owned bounded `crossbeam-channel` consumer queue. The
//! adapter forwards [`chromaticity_script::ScriptCommand`] values
//! the script emits (post slice-5.2 catalog routing) onto a host
//! drain the runtime owns. The runtime [`Command`] enum is NOT
//! widened by slice 5.3 — typed reactive writes are not part of the
//! Phase-1 [`Command`] vocabulary, and slice 5.3 keeps that vocabulary
//! frozen by routing the script-emitted values onto a sibling
//! consumer channel rather than into the runtime command bus
//! directly. The host integration that decides whether to translate
//! a `ScriptCommand::TypedReactiveWrite` into a runtime state
//! mutation lives at the call site (typically the script tick loop
//! or a per-frame drain on the commit thread); slice 5.3 keeps the
//! seam minimal so that translation policy stays out-of-crate.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model (`docs/adr/ADR-001-runtime-model.md`).
//! - ADR-004 — canonical state model
//!   (`docs/adr/ADR-004-state-model.md`).
//! - ADR-006 — thread and process model
//!   (`docs/adr/ADR-006-thread-and-process-model.md`).

// Convenience re-exports of the slice-2 public surface. Sibling
// engine crates depend on `chromaticity_runtime::Command` and
// `chromaticity_runtime::Event` shapes from slice-3 onward.
pub use broadcast_stream::{
    AbrAction, AdaptiveBitrateController, BASE_RECONNECT_DELAY_MS, BroadcastStreamError,
    BroadcastStreamer, DEFAULT_ABR_ADDITIVE_INCREASE_KBPS, DEFAULT_ABR_CLEAN_WINDOW_MS,
    DEFAULT_ABR_COOLDOWN_MS, DEFAULT_ABR_DROP_RATE_THRESHOLD, DEFAULT_ABR_RTT_SPIKE_MS, JitterHook,
    MAX_ALLOWED_BITRATE_KBPS, MAX_RECONNECT_DELAY_MS, MIN_ALLOWED_BITRATE_KBPS,
    REDACTED_SECRET_PLACEHOLDER, ReconnectPlan, ReconnectWatchdog, StreamEndpointConfig,
    StreamProtocol, StreamSecretKey, StreamState, StreamTelemetrySample, zero_jitter,
};
pub use clock_sync::{
    AbletonLinkSnapshot, ClockSourceKind, ClockSyncCoordinator, ClockSyncMode, DEFAULT_CLOCK_BPM,
    DEFAULT_CLOCK_TIMEOUT_MS, DEFAULT_MIDI_CLOCK_WINDOW, MAX_CLOCK_BPM, MAX_PHASE_OFFSET_BEATS,
    MIDI_CLOCK_PPQN, MIDI_RT_CONTINUE, MIDI_RT_START, MIDI_RT_STOP, MIDI_RT_TIMING_CLOCK,
    MIN_CLOCK_BPM, MIN_PHASE_OFFSET_BEATS, MidiClockEstimator, ResolvedClock,
};
pub use command_bus::{
    Command, CommandBus, CommandBusError, CommandChannel, ControlId, ControlValue,
    CrossbeamChannel, Event, EventPayload, EventSeq, InMemoryBus, MappingId, MappingSource,
    PresetId, SnapSlotId,
};
pub use cue_runner::{
    ConfigLoadOutcome, CueBpmMode, CueRunnerState, CueTransitionEvent, FootswitchAction,
    SetlistCue, SetlistCueRunner, load_or_self_heal_json,
};
pub use data_ingest::{
    CASSETTE_FORMAT_VERSION, CassetteSample, DATA_UNIFORM_PREFIX, DataCassette, DataIngestEngine,
    DataIngestError, DataIngestState, DataSourceSpec, DataTransport, DataValue,
};
pub use dmx_artnet::{
    ARTNET_DMX_HEADER_LEN, ARTNET_DMX_PACKET_LEN, ARTNET_MAGIC, ARTNET_OPCODE_DMX, ARTNET_PORT,
    ARTNET_PROTOCOL_VERSION, ArtNetDmxPacket, DMX_CHANNEL_COUNT, DMX_NULL_START_CODE, DmxFixture,
    DmxUniverse, FixtureKind, SACN_ACN_PACKET_ID, SACN_DMX_PACKET_LEN, SACN_MAX_PRIORITY,
    SACN_PORT, SACN_VECTOR_DMP_SET_PROPERTY, SACN_VECTOR_E131_DATA_PACKET,
    SACN_VECTOR_ROOT_E131_DATA, SpectralBandSource, SpectralDmxMapper, decode_artnet_dmx,
    encode_artnet_dmx, encode_sacn_dmx, map_audio_to_universe,
};
pub use pro_dj_link_listener::{
    AutoTransitionConfig, AutoTransitionPlanner, DEFAULT_ENERGY_DROP_HIGH_THRESHOLD,
    DEFAULT_ENERGY_DROP_LOW_THRESHOLD, DEFAULT_ENERGY_SURGE_DELTA, DEFAULT_PHRASE_INTERVAL_BARS,
    DEFAULT_PRO_DJ_LINK_TIMEOUT_MS, DEFAULT_TRANSITION_COOLDOWN_MS, ProDjLinkListener,
    ProDjLinkListenerError, ProDjLinkPollResult, ProDjLinkSyncState, TransitionTrigger,
};
pub use recorder::{
    DEFAULT_REPLAY_DURATION_SECS, DEFAULT_REPLAY_MAX_BYTES, DEFAULT_SAMPLE_DURATION_90KHZ,
    EncodedPacket, Fmp4Muxer, Fmp4MuxerConfig, IsoBoxHeader, MAX_REPLAY_DURATION_SECS,
    MIN_REPLAY_DURATION_SECS, RecorderError, RecorderState, ReplayBuffer, TIMESCALE_90KHZ,
    build_fragment, build_init_segment, parse_box_headers,
};
pub use rekordbox_bridge::{
    ChannelPairMeter, ChannelPairSelection, DjDeviceKind, DjDeviceProfile, MultiChannelExtractor,
    PRO_DJ_LINK_BEAT_PACKET_MIN_LEN, PRO_DJ_LINK_BEAT_PACKET_TYPE, PRO_DJ_LINK_BEAT_PORT,
    PRO_DJ_LINK_KEEPALIVE_PORT, PRO_DJ_LINK_MAGIC, ProDjLinkBeat, REKORDBOX_MACOS_BUNDLE_ID,
    REKORDBOX_MACOS_PROCESS_NAME, RekordboxBridgeError, RekordboxDeckState, RekordboxUniforms,
    UNIFORM_DECK_BAR_BEAT, UNIFORM_DECK_BEAT_PHASE, UNIFORM_DECK_BPM, UNIFORM_DECK_CUE_PULSE,
    UNIFORM_DECK_IS_MASTER, UNIFORM_DECK_PHRASE_ENERGY, UNIFORM_DECK_PLAYING,
    parse_pro_dj_link_beat_packet,
};
pub use twitch_bridge::{
    CHAT_ARGS_PLACEHOLDER, TwitchBridgeError, TwitchConnectionState, TwitchEvent, TwitchEventKind,
    TwitchTriggerRule, TwitchVisualAction, TwitchVisualBridge, UNIFORM_TWITCH_BITS,
    UNIFORM_TWITCH_HYPE_LEVEL, UNIFORM_TWITCH_RAID_VIEWERS, UNIFORM_TWITCH_SUB_PULSE,
    parse_eventsub_notification,
};

#[cfg(feature = "script-bridge")]
pub use script_bridge::{
    ScriptBridgeError, ScriptBridgeReceiver, ScriptCommandRouter, script_bridge,
};

pub mod broadcast_stream;
pub mod clock_sync;
pub mod command_bus;
pub mod crash_resume;
pub mod cue_runner;
pub mod data_ingest;
pub mod dmx_artnet;
pub mod pro_dj_link_listener;
pub mod recorder;
pub mod rekordbox_bridge;
pub mod scheduler;
#[cfg(feature = "script-bridge")]
pub mod script_bridge;
pub mod settings;
pub mod state;
pub mod state_migration;
pub mod state_replay;
pub mod twitch_bridge;
