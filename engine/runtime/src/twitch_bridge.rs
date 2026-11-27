// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Native Twitch EventSub & Chat → live visuals bridge (Phase 7).
//!
//! Translates Twitch EventSub WebSocket `notification` envelopes and
//! chat commands into:
//!
//! - Reactive ADR-017 `chroma_Data_*` scalar uniform pulses
//!   ([`UNIFORM_TWITCH_BITS`], [`UNIFORM_TWITCH_SUB_PULSE`],
//!   [`UNIFORM_TWITCH_RAID_VIEWERS`], [`UNIFORM_TWITCH_HYPE_LEVEL`],
//!   plus custom `chroma_Data_*` pulses) with smooth exponential decay
//!   via [`TwitchVisualBridge::tick`].
//! - Deterministic [`TwitchVisualAction`] triggers (control overrides,
//!   snap recalls, scene switches, and custom uniform pulses) driven by
//!   declarative [`TwitchTriggerRule`] bindings.
//!
//! Zero network I/O and zero external JSON crate dependencies: EventSub
//! envelopes are parsed with a deterministic, depth-bounded RFC 8259
//! parser matching `data_ingest`, and connection lifecycle state is
//! tracked explicitly via [`TwitchConnectionState`].

use crate::data_ingest::{DATA_UNIFORM_PREFIX, DataValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Reactive uniform for normalized Twitch cheer bit bursts (`0.0..=1.0`).
pub const UNIFORM_TWITCH_BITS: &str = "chroma_Data_TwitchBits";

/// Reactive uniform for Twitch subscription pulses (`0.0..=1.0`).
pub const UNIFORM_TWITCH_SUB_PULSE: &str = "chroma_Data_TwitchSubPulse";

/// Reactive uniform for normalized Twitch raid viewer bursts (`0.0..=1.0`).
pub const UNIFORM_TWITCH_RAID_VIEWERS: &str = "chroma_Data_TwitchRaidViewers";

/// Reactive uniform for normalized Twitch Hype Train intensity (`0.0..=1.0`).
pub const UNIFORM_TWITCH_HYPE_LEVEL: &str = "chroma_Data_TwitchHypeLevel";

/// Placeholder token in [`TwitchVisualAction::SwitchScene::scene_id`]
/// replaced with the trimmed arguments of a matching
/// [`TwitchEvent::ChatCommand`].
pub const CHAT_ARGS_PLACEHOLDER: &str = "{args}";

/// Default exponential decay time constant (seconds) for cheer bit bursts.
const DEFAULT_BITS_DECAY_SECS: f32 = 4.0;

/// Default exponential decay time constant (seconds) for subscription pulses.
const DEFAULT_SUB_DECAY_SECS: f32 = 3.5;

/// Default exponential decay time constant (seconds) for raid viewer bursts.
const DEFAULT_RAID_DECAY_SECS: f32 = 8.0;

/// Default exponential decay time constant (seconds) for Hype Train level.
const DEFAULT_HYPE_DECAY_SECS: f32 = 15.0;

/// Normalization denominator for cheer bits (`1000` bits → `1.0`).
const BITS_NORMALIZATION_SCALE: f32 = 1000.0;

/// Normalization denominator for raid viewers (`100` viewers → `1.0`).
const RAID_VIEWERS_NORMALIZATION_SCALE: f32 = 100.0;

/// Floor below which decayed pulse values snap cleanly to `0.0`.
const PULSE_ZERO_EPSILON: f32 = 1e-6;

/// Maximum JSON nesting depth accepted by the EventSub parser.
const MAX_JSON_DEPTH: usize = 64;

/// Normalized Twitch event produced from an EventSub WebSocket
/// notification or an IRC/chat command parser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TwitchEvent {
    /// A viewer followed the broadcaster (`channel.follow`).
    Follow {
        /// Display name of the following user.
        user_name: String,
    },
    /// A viewer subscribed to the channel (`channel.subscribe`).
    Subscribe {
        /// Display name of the subscriber.
        user_name: String,
        /// Subscription tier (`1`, `2`, or `3`).
        tier: u8,
        /// Whether the subscription was gifted.
        is_gift: bool,
    },
    /// A viewer cheered bits in chat (`channel.cheer`).
    Cheer {
        /// Display name of the cheering user (`"Anonymous"` if anonymous).
        user_name: String,
        /// Number of bits cheered.
        bits: u32,
        /// Cheer message text.
        message: String,
    },
    /// Another broadcaster raided the channel (`channel.raid`).
    Raid {
        /// Display name of the raiding broadcaster.
        from_broadcaster: String,
        /// Number of viewers arriving in the raid.
        viewers: u32,
    },
    /// A viewer redeemed a custom Channel Points reward
    /// (`channel.channel_points_custom_reward_redemption.add`).
    ChannelPointRedemption {
        /// Display name of the redeeming user.
        user_name: String,
        /// Twitch reward identifier.
        reward_id: String,
        /// Human-readable reward title.
        reward_title: String,
        /// Optional text input supplied by the viewer.
        user_input: String,
    },
    /// A Hype Train progressed on the channel (`channel.hype_train.progress`).
    HypeTrainProgress {
        /// Current Hype Train level (typically `1..=5+`).
        level: u32,
        /// Current progress points toward `goal`.
        progress: u32,
        /// Target points required to complete the current level.
        goal: u32,
    },
    /// A chat command prefixed with `!` (e.g. `!scene neon_grid`).
    ChatCommand {
        /// Display name of the chatter.
        user_name: String,
        /// Command token including leading `!` (e.g. `"!scene"`).
        command: String,
        /// Remaining trimmed argument string after the command token.
        args: String,
    },
}

impl TwitchEvent {
    /// Returns the coarse [`TwitchEventKind`] discriminant for rule matching.
    #[must_use]
    pub const fn kind(&self) -> TwitchEventKind {
        match self {
            Self::Follow { .. } => TwitchEventKind::Follow,
            Self::Subscribe { .. } => TwitchEventKind::Subscribe,
            Self::Cheer { .. } => TwitchEventKind::Cheer,
            Self::Raid { .. } => TwitchEventKind::Raid,
            Self::ChannelPointRedemption { .. } => TwitchEventKind::ChannelPointRedemption,
            Self::HypeTrainProgress { .. } => TwitchEventKind::HypeTrainProgress,
            Self::ChatCommand { .. } => TwitchEventKind::ChatCommand,
        }
    }

    /// Parses a raw chat message into a [`TwitchEvent::ChatCommand`] if it
    /// starts with `!` followed by at least one non-whitespace character.
    #[must_use]
    pub fn parse_chat_message(user_name: impl Into<String>, message: &str) -> Option<Self> {
        let trimmed = message.trim();
        if !trimmed.starts_with('!') {
            return None;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let cmd = parts.next()?;
        if cmd.len() <= 1 {
            return None;
        }
        let args = parts.next().map_or("", str::trim);
        Some(Self::ChatCommand {
            user_name: user_name.into(),
            command: cmd.to_owned(),
            args: args.to_owned(),
        })
    }
}

/// Discriminant enum matching [`TwitchEvent`] variants for
/// [`TwitchTriggerRule`] definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TwitchEventKind {
    /// Matches [`TwitchEvent::Follow`].
    Follow,
    /// Matches [`TwitchEvent::Subscribe`].
    Subscribe,
    /// Matches [`TwitchEvent::Cheer`].
    Cheer,
    /// Matches [`TwitchEvent::Raid`].
    Raid,
    /// Matches [`TwitchEvent::ChannelPointRedemption`].
    ChannelPointRedemption,
    /// Matches [`TwitchEvent::HypeTrainProgress`].
    HypeTrainProgress,
    /// Matches [`TwitchEvent::ChatCommand`].
    ChatCommand,
}

/// Visual action emitted by [`TwitchVisualBridge::ingest_event`] when a
/// [`TwitchTriggerRule`] matches an incoming [`TwitchEvent`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TwitchVisualAction {
    /// Set a scene control parameter to `value`.
    SetControl {
        /// Zero-based scene control index.
        control_index: u32,
        /// Target control value.
        value: f32,
    },
    /// Recall a performance snap slot.
    TriggerSnap {
        /// Target snap slot index.
        slot: u8,
    },
    /// Switch the active scene to `scene_id`.
    SwitchScene {
        /// Target scene identifier.
        scene_id: String,
    },
    /// Trigger or boost a decaying `chroma_Data_*` scalar uniform pulse.
    PulseUniform {
        /// Target `chroma_Data_*` uniform name.
        uniform_name: String,
        /// Peak pulse value (non-negative finite `f32`).
        peak: f32,
        /// Exponential decay time constant in seconds (`> 0.0`).
        decay_secs: f32,
    },
}

/// Declarative rule mapping a [`TwitchEvent`] (with optional reward title
/// or chat command filter) to a [`TwitchVisualAction`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitchTriggerRule {
    /// Event variant required to trigger this rule.
    pub event_kind: TwitchEventKind,
    /// Optional case-insensitive reward title filter (for
    /// [`TwitchEventKind::ChannelPointRedemption`]).
    pub reward_title: Option<String>,
    /// Optional case-insensitive chat command filter (for
    /// [`TwitchEventKind::ChatCommand`], e.g. `"!scene"`).
    pub chat_command: Option<String>,
    /// Action emitted when this rule matches.
    pub action: TwitchVisualAction,
}

impl TwitchTriggerRule {
    /// Evaluates whether this rule matches `event` and, if so, resolves
    /// any dynamic action parameters (such as `{args}` in `SwitchScene`).
    #[must_use]
    pub fn evaluate(&self, event: &TwitchEvent) -> Option<TwitchVisualAction> {
        if self.event_kind != event.kind() {
            return None;
        }

        if let Some(expected_reward) = &self.reward_title {
            match event {
                TwitchEvent::ChannelPointRedemption { reward_title, .. }
                    if reward_title.eq_ignore_ascii_case(expected_reward.trim()) => {}
                _ => return None,
            }
        }

        if let Some(expected_cmd) = &self.chat_command {
            match event {
                TwitchEvent::ChatCommand { command, .. }
                    if normalize_chat_command(command)
                        .eq_ignore_ascii_case(normalize_chat_command(expected_cmd)) => {}
                _ => return None,
            }
        }

        match (&self.action, event) {
            (
                TwitchVisualAction::SwitchScene { scene_id },
                TwitchEvent::ChatCommand { args, .. },
            ) if scene_id.is_empty() || scene_id == CHAT_ARGS_PLACEHOLDER => {
                let target = args.trim();
                if target.is_empty() {
                    return None;
                }
                Some(TwitchVisualAction::SwitchScene {
                    scene_id: target.to_owned(),
                })
            }
            (action, _) => Some(action.clone()),
        }
    }
}

/// Explicit connection lifecycle state for [`TwitchVisualBridge`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TwitchConnectionState {
    /// No live connection or rehearsal cassette is active.
    #[default]
    Disconnected,
    /// Replaying recorded Twitch events from an offline rehearsal cassette.
    RehearsalCassette,
    /// Connected to a live Twitch channel EventSub session.
    Connected {
        /// Broadcaster channel login name.
        channel: String,
    },
}

/// Errors returned by [`parse_eventsub_notification`] and
/// [`TwitchVisualBridge`] validation helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwitchBridgeError {
    /// Malformed RFC 8259 JSON input.
    JsonError {
        /// Human-readable parse diagnostic.
        message: String,
    },
    /// Required EventSub field was absent.
    MissingField {
        /// Dotted path of the missing field.
        field: &'static str,
    },
    /// EventSub field carried an unexpected JSON type or out-of-range value.
    InvalidFieldType {
        /// Dotted path of the invalid field.
        field: &'static str,
        /// Expected type description.
        expected: &'static str,
    },
    /// Subscription tier string was not a recognized Twitch tier.
    InvalidTier {
        /// Raw tier value received.
        raw: String,
    },
    /// Custom uniform name violated the `chroma_Data_*` ASCII identifier rule.
    InvalidUniformName {
        /// Rejected uniform name.
        name: String,
    },
    /// Non-finite or out-of-domain floating-point pulse parameter.
    NonFiniteParameter {
        /// Name of the invalid parameter.
        name: &'static str,
    },
}

impl fmt::Display for TwitchBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonError { message } => write!(f, "twitch bridge JSON error: {message}"),
            Self::MissingField { field } => {
                write!(f, "missing required Twitch EventSub field '{field}'")
            }
            Self::InvalidFieldType { field, expected } => {
                write!(
                    f,
                    "invalid Twitch EventSub field '{field}': expected {expected}"
                )
            }
            Self::InvalidTier { raw } => {
                write!(
                    f,
                    "unrecognized Twitch subscription tier '{raw}' (expected 1000/2000/3000 or 1/2/3)"
                )
            }
            Self::InvalidUniformName { name } => write!(
                f,
                "invalid Twitch uniform name '{name}': must start with '{DATA_UNIFORM_PREFIX}' followed by ASCII identifier characters"
            ),
            Self::NonFiniteParameter { name } => {
                write!(f, "invalid non-finite or out-of-range parameter '{name}'")
            }
        }
    }
}

impl std::error::Error for TwitchBridgeError {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ActivePulse {
    value: f32,
    decay_secs: f32,
}

/// Stateful Twitch EventSub & Chat → live visual bridge.
///
/// Owns explicit [`TwitchConnectionState`], declarative
/// [`TwitchTriggerRule`] bindings, and smoothly decaying `chroma_Data_*`
/// scalar uniform pulses.
#[derive(Debug, Clone)]
pub struct TwitchVisualBridge {
    connection_state: TwitchConnectionState,
    rules: Vec<TwitchTriggerRule>,
    pulses: BTreeMap<String, ActivePulse>,
}

impl Default for TwitchVisualBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitchVisualBridge {
    /// Creates a new [`TwitchVisualBridge`] in
    /// [`TwitchConnectionState::Disconnected`] with the four built-in
    /// Twitch uniform pulses initialized to `0.0`.
    #[must_use]
    pub fn new() -> Self {
        let mut pulses = BTreeMap::new();
        pulses.insert(
            UNIFORM_TWITCH_BITS.to_owned(),
            ActivePulse {
                value: 0.0,
                decay_secs: DEFAULT_BITS_DECAY_SECS,
            },
        );
        pulses.insert(
            UNIFORM_TWITCH_SUB_PULSE.to_owned(),
            ActivePulse {
                value: 0.0,
                decay_secs: DEFAULT_SUB_DECAY_SECS,
            },
        );
        pulses.insert(
            UNIFORM_TWITCH_RAID_VIEWERS.to_owned(),
            ActivePulse {
                value: 0.0,
                decay_secs: DEFAULT_RAID_DECAY_SECS,
            },
        );
        pulses.insert(
            UNIFORM_TWITCH_HYPE_LEVEL.to_owned(),
            ActivePulse {
                value: 0.0,
                decay_secs: DEFAULT_HYPE_DECAY_SECS,
            },
        );

        Self {
            connection_state: TwitchConnectionState::Disconnected,
            rules: Vec::new(),
            pulses,
        }
    }

    /// Creates a new [`TwitchVisualBridge`] pre-configured with `rules`.
    #[must_use]
    pub fn with_rules(rules: Vec<TwitchTriggerRule>) -> Self {
        let mut bridge = Self::new();
        bridge.rules = rules;
        bridge
    }

    /// Returns the current [`TwitchConnectionState`].
    #[must_use]
    pub fn connection_state(&self) -> &TwitchConnectionState {
        &self.connection_state
    }

    /// Transitions the explicit [`TwitchConnectionState`].
    pub fn set_connection_state(&mut self, state: TwitchConnectionState) {
        self.connection_state = state;
    }

    /// Returns the currently configured [`TwitchTriggerRule`] slice.
    #[must_use]
    pub fn rules(&self) -> &[TwitchTriggerRule] {
        &self.rules
    }

    /// Appends a [`TwitchTriggerRule`] to this bridge.
    pub fn add_rule(&mut self, rule: TwitchTriggerRule) {
        self.rules.push(rule);
    }

    /// Removes all configured [`TwitchTriggerRule`] entries.
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Returns a snapshot of all current `chroma_Data_*` uniform values
    /// without advancing decay timers.
    #[must_use]
    pub fn current_uniforms(&self) -> BTreeMap<String, DataValue> {
        self.pulses
            .iter()
            .map(|(name, pulse)| (name.clone(), DataValue::Scalar(pulse.value)))
            .collect()
    }

    /// Explicitly triggers or raises a custom `chroma_Data_*` uniform pulse.
    ///
    /// # Errors
    ///
    /// Returns [`TwitchBridgeError::InvalidUniformName`] if `uniform_name`
    /// does not follow the `chroma_Data_[A-Za-z0-9_]+` convention, or
    /// [`TwitchBridgeError::NonFiniteParameter`] if `peak` is non-finite or
    /// negative, or `decay_secs` is non-finite or `<= 0.0`.
    pub fn trigger_pulse(
        &mut self,
        uniform_name: impl Into<String>,
        peak: f32,
        decay_secs: f32,
    ) -> Result<(), TwitchBridgeError> {
        let uniform_name = uniform_name.into();
        validate_uniform_name(&uniform_name)?;
        if !peak.is_finite() || peak < 0.0 {
            return Err(TwitchBridgeError::NonFiniteParameter { name: "peak" });
        }
        if !decay_secs.is_finite() || decay_secs <= 0.0 {
            return Err(TwitchBridgeError::NonFiniteParameter { name: "decay_secs" });
        }

        self.apply_pulse_max(&uniform_name, peak, decay_secs);
        Ok(())
    }

    /// Ingests a [`TwitchEvent`], updates built-in reactive uniform pulses,
    /// evaluates all configured [`TwitchTriggerRule`] entries, and returns
    /// every triggered [`TwitchVisualAction`] in rule order.
    pub fn ingest_event(&mut self, event: TwitchEvent) -> Vec<TwitchVisualAction> {
        // 1. Update built-in reactive uniform pulses.
        match &event {
            TwitchEvent::Cheer { bits, .. } => {
                let burst = (*bits as f32 / BITS_NORMALIZATION_SCALE).clamp(0.0, 1.0);
                let current = self
                    .pulses
                    .get(UNIFORM_TWITCH_BITS)
                    .map_or(0.0, |p| p.value);
                let next = (current + burst).clamp(0.0, 1.0);
                self.pulses.insert(
                    UNIFORM_TWITCH_BITS.to_owned(),
                    ActivePulse {
                        value: next,
                        decay_secs: DEFAULT_BITS_DECAY_SECS,
                    },
                );
            }
            TwitchEvent::Subscribe { .. } => {
                self.apply_pulse_max(UNIFORM_TWITCH_SUB_PULSE, 1.0, DEFAULT_SUB_DECAY_SECS);
            }
            TwitchEvent::Raid { viewers, .. } => {
                let burst = (*viewers as f32 / RAID_VIEWERS_NORMALIZATION_SCALE).clamp(0.0, 1.0);
                self.apply_pulse_max(UNIFORM_TWITCH_RAID_VIEWERS, burst, DEFAULT_RAID_DECAY_SECS);
            }
            TwitchEvent::HypeTrainProgress {
                level,
                progress,
                goal,
            } => {
                let level_norm = normalize_hype_train(*level, *progress, *goal);
                self.pulses.insert(
                    UNIFORM_TWITCH_HYPE_LEVEL.to_owned(),
                    ActivePulse {
                        value: level_norm,
                        decay_secs: DEFAULT_HYPE_DECAY_SECS,
                    },
                );
            }
            TwitchEvent::Follow { .. }
            | TwitchEvent::ChannelPointRedemption { .. }
            | TwitchEvent::ChatCommand { .. } => {}
        }

        // 2. Evaluate trigger rules in deterministic order.
        let mut actions = Vec::new();
        for rule in &self.rules {
            if let Some(action) = rule.evaluate(&event) {
                actions.push(action);
            }
        }

        // 3. Apply any PulseUniform actions to the active pulse map.
        for action in &actions {
            if let TwitchVisualAction::PulseUniform {
                uniform_name,
                peak,
                decay_secs,
            } = action
            {
                if validate_uniform_name(uniform_name).is_ok()
                    && peak.is_finite()
                    && *peak >= 0.0
                    && decay_secs.is_finite()
                    && *decay_secs > 0.0
                {
                    self.apply_pulse_max(uniform_name, *peak, *decay_secs);
                }
            }
        }

        actions
    }

    /// Advances all active uniform pulses by `dt_secs` using smooth
    /// exponential decay (`value * exp(-dt / decay_secs)`) and returns the
    /// updated `chroma_Data_*` uniform map ready for [`crate::DataIngestEngine`]
    /// or shader uniform upload.
    ///
    /// Non-finite or non-positive `dt_secs` values leave pulse levels
    /// unchanged and still return the current uniform snapshot.
    pub fn tick(&mut self, dt_secs: f32) -> BTreeMap<String, DataValue> {
        if dt_secs.is_finite() && dt_secs > 0.0 {
            for pulse in self.pulses.values_mut() {
                if pulse.value > 0.0 && pulse.decay_secs > 0.0 {
                    let decayed = pulse.value * (-dt_secs / pulse.decay_secs).exp();
                    pulse.value = if decayed < PULSE_ZERO_EPSILON || !decayed.is_finite() {
                        0.0
                    } else {
                        decayed
                    };
                }
            }
        }

        self.current_uniforms()
    }

    fn apply_pulse_max(&mut self, uniform_name: &str, peak: f32, decay_secs: f32) {
        let next_value = self
            .pulses
            .get(uniform_name)
            .map_or(peak, |existing| existing.value.max(peak));
        self.pulses.insert(
            uniform_name.to_owned(),
            ActivePulse {
                value: next_value,
                decay_secs,
            },
        );
    }
}

/// Parses a standard Twitch EventSub WebSocket JSON envelope into a
/// [`TwitchEvent`].
///
/// Returns:
/// - `Ok(Some(event))` for supported `notification` messages
///   (`channel.follow`, `channel.subscribe`, `channel.cheer`,
///   `channel.raid`, `channel.channel_points_custom_reward_redemption.add`,
///   `channel.hype_train.progress`).
/// - `Ok(None)` for non-notification lifecycle messages (such as
///   `session_welcome`, `session_keepalive`, `session_reconnect`,
///   `revocation`) or unrecognized `subscription_type` values.
/// - `Err(TwitchBridgeError)` for malformed JSON or schema violations in a
///   supported notification payload.
///
/// # Errors
///
/// Returns [`TwitchBridgeError::JsonError`], [`TwitchBridgeError::MissingField`],
/// [`TwitchBridgeError::InvalidFieldType`], or [`TwitchBridgeError::InvalidTier`].
pub fn parse_eventsub_notification(json: &str) -> Result<Option<TwitchEvent>, TwitchBridgeError> {
    let root = parse_json(json)?;
    let JsonValue::Object(root_obj) = root else {
        return Err(TwitchBridgeError::JsonError {
            message: "EventSub envelope root must be a JSON object".to_owned(),
        });
    };

    let metadata = get_required_object(&root_obj, "metadata")?;
    let message_type = get_required_string(metadata, "message_type", "metadata.message_type")?;
    if message_type != "notification" {
        return Ok(None);
    }

    let payload = get_required_object(&root_obj, "payload")?;
    let subscription_type = match metadata.get("subscription_type") {
        Some(JsonValue::String(s)) => s.as_str(),
        Some(_) => {
            return Err(TwitchBridgeError::InvalidFieldType {
                field: "metadata.subscription_type",
                expected: "string",
            });
        }
        None => {
            let subscription = get_required_object(payload, "subscription")?;
            get_required_string(subscription, "type", "payload.subscription.type")?
        }
    };

    let event = get_required_object(payload, "event")?;

    match subscription_type {
        "channel.follow" => {
            let user_name = get_optional_string(event, "user_name", "payload.event.user_name")?
                .or(get_optional_string(
                    event,
                    "user_login",
                    "payload.event.user_login",
                )?)
                .ok_or(TwitchBridgeError::MissingField {
                    field: "payload.event.user_name",
                })?;
            Ok(Some(TwitchEvent::Follow {
                user_name: user_name.to_owned(),
            }))
        }
        "channel.subscribe" => {
            let user_name =
                get_required_string(event, "user_name", "payload.event.user_name")?.to_owned();
            let tier = parse_subscription_tier(event)?;
            let is_gift =
                get_optional_bool(event, "is_gift", "payload.event.is_gift")?.unwrap_or(false);
            Ok(Some(TwitchEvent::Subscribe {
                user_name,
                tier,
                is_gift,
            }))
        }
        "channel.cheer" => {
            let user_name = get_optional_string(event, "user_name", "payload.event.user_name")?
                .unwrap_or("Anonymous")
                .to_owned();
            let bits = get_required_u32(event, "bits", "payload.event.bits")?;
            let message = get_optional_string(event, "message", "payload.event.message")?
                .unwrap_or("")
                .to_owned();
            Ok(Some(TwitchEvent::Cheer {
                user_name,
                bits,
                message,
            }))
        }
        "channel.raid" => {
            let from_broadcaster = get_optional_string(
                event,
                "from_broadcaster_user_name",
                "payload.event.from_broadcaster_user_name",
            )?
            .or(get_optional_string(
                event,
                "from_broadcaster_user_login",
                "payload.event.from_broadcaster_user_login",
            )?)
            .ok_or(TwitchBridgeError::MissingField {
                field: "payload.event.from_broadcaster_user_name",
            })?
            .to_owned();
            let viewers = get_required_u32(event, "viewers", "payload.event.viewers")?;
            Ok(Some(TwitchEvent::Raid {
                from_broadcaster,
                viewers,
            }))
        }
        "channel.channel_points_custom_reward_redemption.add" => {
            let user_name =
                get_required_string(event, "user_name", "payload.event.user_name")?.to_owned();
            let reward = get_required_object(event, "reward")?;
            let reward_id =
                get_required_string(reward, "id", "payload.event.reward.id")?.to_owned();
            let reward_title =
                get_required_string(reward, "title", "payload.event.reward.title")?.to_owned();
            let user_input = get_optional_string(event, "user_input", "payload.event.user_input")?
                .unwrap_or("")
                .to_owned();
            Ok(Some(TwitchEvent::ChannelPointRedemption {
                user_name,
                reward_id,
                reward_title,
                user_input,
            }))
        }
        "channel.hype_train.progress" => {
            let level = get_required_u32(event, "level", "payload.event.level")?;
            let progress = get_required_u32(event, "progress", "payload.event.progress")?;
            let goal = get_required_u32(event, "goal", "payload.event.goal")?;
            Ok(Some(TwitchEvent::HypeTrainProgress {
                level,
                progress,
                goal,
            }))
        }
        _ => Ok(None),
    }
}

fn normalize_hype_train(level: u32, progress: u32, goal: u32) -> f32 {
    if level == 0 {
        return 0.0;
    }
    let frac = if goal > 0 {
        (progress as f32 / goal as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((level as f32 + frac) / 6.0).clamp(0.0, 1.0)
}

fn normalize_chat_command(cmd: &str) -> &str {
    let trimmed = cmd.trim();
    trimmed.strip_prefix('!').unwrap_or(trimmed)
}

fn validate_uniform_name(name: &str) -> Result<(), TwitchBridgeError> {
    let Some(suffix) = name.strip_prefix(DATA_UNIFORM_PREFIX) else {
        return Err(TwitchBridgeError::InvalidUniformName {
            name: name.to_owned(),
        });
    };
    if suffix.is_empty()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(TwitchBridgeError::InvalidUniformName {
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn parse_subscription_tier(event: &BTreeMap<String, JsonValue>) -> Result<u8, TwitchBridgeError> {
    let Some(raw) = event.get("tier") else {
        return Err(TwitchBridgeError::MissingField {
            field: "payload.event.tier",
        });
    };
    match raw {
        JsonValue::String(s) => match s.as_str() {
            "1000" | "1" => Ok(1),
            "2000" | "2" => Ok(2),
            "3000" | "3" => Ok(3),
            other => Err(TwitchBridgeError::InvalidTier {
                raw: other.to_owned(),
            }),
        },
        JsonValue::Number(n) => {
            let n_u32 = json_f64_to_u32(*n, "payload.event.tier")?;
            match n_u32 {
                1000 | 1 => Ok(1),
                2000 | 2 => Ok(2),
                3000 | 3 => Ok(3),
                other => Err(TwitchBridgeError::InvalidTier {
                    raw: other.to_string(),
                }),
            }
        }
        _ => Err(TwitchBridgeError::InvalidFieldType {
            field: "payload.event.tier",
            expected: "string or integer tier",
        }),
    }
}

fn get_required_object<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &'static str,
) -> Result<&'a BTreeMap<String, JsonValue>, TwitchBridgeError> {
    match obj.get(key) {
        Some(JsonValue::Object(map)) => Ok(map),
        Some(_) => Err(TwitchBridgeError::InvalidFieldType {
            field: key,
            expected: "object",
        }),
        None => Err(TwitchBridgeError::MissingField { field: key }),
    }
}

fn get_required_string<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &str,
    field: &'static str,
) -> Result<&'a str, TwitchBridgeError> {
    match obj.get(key) {
        Some(JsonValue::String(s)) => Ok(s.as_str()),
        Some(_) => Err(TwitchBridgeError::InvalidFieldType {
            field,
            expected: "string",
        }),
        None => Err(TwitchBridgeError::MissingField { field }),
    }
}

fn get_optional_string<'a>(
    obj: &'a BTreeMap<String, JsonValue>,
    key: &str,
    field: &'static str,
) -> Result<Option<&'a str>, TwitchBridgeError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(TwitchBridgeError::InvalidFieldType {
            field,
            expected: "string or null",
        }),
    }
}

fn get_optional_bool(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
    field: &'static str,
) -> Result<Option<bool>, TwitchBridgeError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(TwitchBridgeError::InvalidFieldType {
            field,
            expected: "boolean",
        }),
    }
}

fn get_required_u32(
    obj: &BTreeMap<String, JsonValue>,
    key: &str,
    field: &'static str,
) -> Result<u32, TwitchBridgeError> {
    match obj.get(key) {
        Some(JsonValue::Number(n)) => json_f64_to_u32(*n, field),
        Some(_) => Err(TwitchBridgeError::InvalidFieldType {
            field,
            expected: "unsigned 32-bit integer",
        }),
        None => Err(TwitchBridgeError::MissingField { field }),
    }
}

fn json_f64_to_u32(n: f64, field: &'static str) -> Result<u32, TwitchBridgeError> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > f64::from(u32::MAX) {
        return Err(TwitchBridgeError::InvalidFieldType {
            field,
            expected: "unsigned 32-bit integer",
        });
    }
    Ok(n as u32)
}

// ---------------------------------------------------------------------------
// Minimal deterministic RFC 8259 JSON parser (zero external dependencies)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

fn parse_json(input: &str) -> Result<JsonValue, TwitchBridgeError> {
    let mut parser = JsonParser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let val = parser.parse_value(0)?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(TwitchBridgeError::JsonError {
            message: format!("unexpected trailing characters at byte {}", parser.pos),
        });
    }
    Ok(val)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl JsonParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, TwitchBridgeError> {
        if depth > MAX_JSON_DEPTH {
            return Err(TwitchBridgeError::JsonError {
                message: "JSON nesting depth exceeded".to_owned(),
            });
        }
        self.skip_ws();
        let Some(b) = self.peek() else {
            return Err(TwitchBridgeError::JsonError {
                message: "unexpected end of JSON input".to_owned(),
            });
        };

        match b {
            b'{' => self.parse_object(depth + 1),
            b'[' => self.parse_array(depth + 1),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' => self.parse_literal(b"true", JsonValue::Bool(true)),
            b'f' => self.parse_literal(b"false", JsonValue::Bool(false)),
            b'n' => self.parse_literal(b"null", JsonValue::Null),
            b'-' | b'0'..=b'9' => self.parse_number().map(JsonValue::Number),
            other => Err(TwitchBridgeError::JsonError {
                message: format!("unexpected byte 0x{other:02x} at position {}", self.pos),
            }),
        }
    }

    fn parse_literal(
        &mut self,
        expected: &[u8],
        val: JsonValue,
    ) -> Result<JsonValue, TwitchBridgeError> {
        let end = self.pos + expected.len();
        if self.bytes.get(self.pos..end) == Some(expected) {
            self.pos = end;
            Ok(val)
        } else {
            Err(TwitchBridgeError::JsonError {
                message: format!("invalid JSON literal at byte {}", self.pos),
            })
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, TwitchBridgeError> {
        self.pos += 1; // consume '{'
        self.skip_ws();
        let mut map = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(TwitchBridgeError::JsonError {
                    message: format!("expected object key string at byte {}", self.pos),
                });
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(TwitchBridgeError::JsonError {
                    message: format!("expected ':' after object key at byte {}", self.pos),
                });
            }
            self.pos += 1;
            let value = self.parse_value(depth)?;
            map.insert(key, value);

            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(map));
                }
                _ => {
                    return Err(TwitchBridgeError::JsonError {
                        message: format!("expected ',' or '}}' in object at byte {}", self.pos),
                    });
                }
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, TwitchBridgeError> {
        self.pos += 1; // consume '['
        self.skip_ws();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }

        loop {
            let value = self.parse_value(depth)?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => {
                    return Err(TwitchBridgeError::JsonError {
                        message: format!("expected ',' or ']' in array at byte {}", self.pos),
                    });
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, TwitchBridgeError> {
        self.pos += 1; // consume opening '"'
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            match b {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(esc) = self.peek() else {
                        return Err(TwitchBridgeError::JsonError {
                            message: "unterminated escape sequence in string".to_owned(),
                        });
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\x08'),
                        b'f' => out.push('\x0c'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.parse_hex4()?;
                            if (0xD800..=0xDBFF).contains(&code) {
                                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                    return Err(TwitchBridgeError::JsonError {
                                        message: "unpaired UTF-16 high surrogate".to_owned(),
                                    });
                                }
                                self.pos += 2;
                                let low = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(TwitchBridgeError::JsonError {
                                        message: "invalid UTF-16 low surrogate".to_owned(),
                                    });
                                }
                                let combined = 0x10000
                                    + ((u32::from(code) - 0xD800) << 10)
                                    + (u32::from(low) - 0xDC00);
                                let Some(ch) = char::from_u32(combined) else {
                                    return Err(TwitchBridgeError::JsonError {
                                        message: "invalid Unicode scalar value".to_owned(),
                                    });
                                };
                                out.push(ch);
                            } else {
                                let Some(ch) = char::from_u32(u32::from(code)) else {
                                    return Err(TwitchBridgeError::JsonError {
                                        message: "invalid Unicode scalar escape".to_owned(),
                                    });
                                };
                                out.push(ch);
                            }
                        }
                        _ => {
                            return Err(TwitchBridgeError::JsonError {
                                message: format!("invalid string escape at byte {}", self.pos),
                            });
                        }
                    }
                }
                0x00..=0x1F => {
                    return Err(TwitchBridgeError::JsonError {
                        message: format!("unescaped control character at byte {}", self.pos),
                    });
                }
                _ => {
                    let rest = std::str::from_utf8(&self.bytes[self.pos..]).map_err(|_| {
                        TwitchBridgeError::JsonError {
                            message: "invalid UTF-8 in JSON string".to_owned(),
                        }
                    })?;
                    let Some(ch) = rest.chars().next() else {
                        return Err(TwitchBridgeError::JsonError {
                            message: "unexpected end of string".to_owned(),
                        });
                    };
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err(TwitchBridgeError::JsonError {
            message: "unterminated JSON string".to_owned(),
        })
    }

    fn parse_hex4(&mut self) -> Result<u16, TwitchBridgeError> {
        let Some(slice) = self.bytes.get(self.pos..self.pos + 4) else {
            return Err(TwitchBridgeError::JsonError {
                message: "truncated \\uXXXX escape".to_owned(),
            });
        };
        let hex_str = std::str::from_utf8(slice).map_err(|_| TwitchBridgeError::JsonError {
            message: "invalid UTF-8 in \\uXXXX escape".to_owned(),
        })?;
        let val = u16::from_str_radix(hex_str, 16).map_err(|_| TwitchBridgeError::JsonError {
            message: format!("invalid hex digits '{hex_str}' in \\u escape"),
        })?;
        self.pos += 4;
        Ok(val)
    }

    fn parse_number(&mut self) -> Result<f64, TwitchBridgeError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
            }
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => {
                return Err(TwitchBridgeError::JsonError {
                    message: format!("invalid number at byte {start}"),
                });
            }
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(TwitchBridgeError::JsonError {
                    message: format!("missing fractional digits in number at byte {start}"),
                });
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(TwitchBridgeError::JsonError {
                    message: format!("missing exponent digits in number at byte {start}"),
                });
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let num_str = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| {
            TwitchBridgeError::JsonError {
                message: format!("invalid UTF-8 in number at byte {start}"),
            }
        })?;
        num_str
            .parse::<f64>()
            .map_err(|_| TwitchBridgeError::JsonError {
                message: format!("failed to parse number '{num_str}'"),
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — Twitch EventSub/Chat visual bridge pins.
    use super::*;
    use crate::data_ingest::{DataIngestEngine, DataSourceSpec, DataTransport};

    fn scalar(uniforms: &BTreeMap<String, DataValue>, key: &str) -> f32 {
        match uniforms.get(key) {
            Some(DataValue::Scalar(v)) => *v,
            other => panic!("expected scalar for '{key}', got {other:?}"),
        }
    }

    #[test]
    fn parse_eventsub_follow() {
        let json = r#"{
            "metadata": {
                "message_id": "msg-follow-1",
                "message_type": "notification",
                "message_timestamp": "2026-11-27T05:00:00Z",
                "subscription_type": "channel.follow",
                "subscription_version": "2"
            },
            "payload": {
                "subscription": { "type": "channel.follow", "version": "2" },
                "event": {
                    "user_id": "1234",
                    "user_login": "neon_fan",
                    "user_name": "NeonFan"
                }
            }
        }"#;

        let parsed = parse_eventsub_notification(json).unwrap();
        assert_eq!(
            parsed,
            Some(TwitchEvent::Follow {
                user_name: "NeonFan".to_owned(),
            })
        );
    }

    #[test]
    fn parse_eventsub_subscribe_tiers_and_gift() {
        let json = r#"{
            "metadata": {
                "message_id": "msg-sub-1",
                "message_type": "notification",
                "subscription_type": "channel.subscribe"
            },
            "payload": {
                "subscription": { "type": "channel.subscribe" },
                "event": {
                    "user_name": "SynthLover",
                    "tier": "2000",
                    "is_gift": true
                }
            }
        }"#;

        let parsed = parse_eventsub_notification(json).unwrap();
        assert_eq!(
            parsed,
            Some(TwitchEvent::Subscribe {
                user_name: "SynthLover".to_owned(),
                tier: 2,
                is_gift: true,
            })
        );
    }

    #[test]
    fn parse_eventsub_cheer_and_anonymous() {
        let json = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.cheer"
            },
            "payload": {
                "event": {
                    "is_anonymous": false,
                    "user_name": "BitWizard",
                    "bits": 500,
                    "message": "Cheer500 strobe it!"
                }
            }
        }"#;

        assert_eq!(
            parse_eventsub_notification(json).unwrap(),
            Some(TwitchEvent::Cheer {
                user_name: "BitWizard".to_owned(),
                bits: 500,
                message: "Cheer500 strobe it!".to_owned(),
            })
        );

        let anon_json = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.cheer"
            },
            "payload": {
                "event": {
                    "is_anonymous": true,
                    "user_name": null,
                    "bits": 100,
                    "message": "Cheer100"
                }
            }
        }"#;

        assert_eq!(
            parse_eventsub_notification(anon_json).unwrap(),
            Some(TwitchEvent::Cheer {
                user_name: "Anonymous".to_owned(),
                bits: 100,
                message: "Cheer100".to_owned(),
            })
        );
    }

    #[test]
    fn parse_eventsub_raid() {
        let json = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.raid"
            },
            "payload": {
                "event": {
                    "from_broadcaster_user_name": "LaserDJ",
                    "viewers": 75
                }
            }
        }"#;

        assert_eq!(
            parse_eventsub_notification(json).unwrap(),
            Some(TwitchEvent::Raid {
                from_broadcaster: "LaserDJ".to_owned(),
                viewers: 75,
            })
        );
    }

    #[test]
    fn parse_eventsub_channel_point_redemption() {
        let json = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.channel_points_custom_reward_redemption.add"
            },
            "payload": {
                "event": {
                    "user_name": "PrismViewer",
                    "user_input": "Make it ultraviolet!",
                    "reward": {
                        "id": "reward-99",
                        "title": "Invert Palette"
                    }
                }
            }
        }"#;

        assert_eq!(
            parse_eventsub_notification(json).unwrap(),
            Some(TwitchEvent::ChannelPointRedemption {
                user_name: "PrismViewer".to_owned(),
                reward_id: "reward-99".to_owned(),
                reward_title: "Invert Palette".to_owned(),
                user_input: "Make it ultraviolet!".to_owned(),
            })
        );
    }

    #[test]
    fn parse_eventsub_hype_train_progress() {
        let json = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.hype_train.progress"
            },
            "payload": {
                "event": {
                    "level": 3,
                    "progress": 600,
                    "goal": 1200
                }
            }
        }"#;

        assert_eq!(
            parse_eventsub_notification(json).unwrap(),
            Some(TwitchEvent::HypeTrainProgress {
                level: 3,
                progress: 600,
                goal: 1200,
            })
        );
    }

    #[test]
    fn parse_eventsub_ignores_keepalive_and_unknown_subscription() {
        let keepalive = r#"{
            "metadata": {
                "message_type": "session_keepalive"
            },
            "payload": {}
        }"#;
        assert_eq!(parse_eventsub_notification(keepalive).unwrap(), None);

        let unknown = r#"{
            "metadata": {
                "message_type": "notification",
                "subscription_type": "channel.poll.begin"
            },
            "payload": { "event": {} }
        }"#;
        assert_eq!(parse_eventsub_notification(unknown).unwrap(), None);
    }

    #[test]
    fn cheer_sub_raid_hype_uniform_pulses_and_exponential_decay() {
        let mut bridge = TwitchVisualBridge::new();
        assert_eq!(
            *bridge.connection_state(),
            TwitchConnectionState::Disconnected
        );
        bridge.set_connection_state(TwitchConnectionState::Connected {
            channel: "chromaticity_live".to_owned(),
        });

        // Cheer 500 bits -> 0.5 burst, then another 250 bits -> 0.75.
        bridge.ingest_event(TwitchEvent::Cheer {
            user_name: "Alice".to_owned(),
            bits: 500,
            message: "Cheer500".to_owned(),
        });
        bridge.ingest_event(TwitchEvent::Cheer {
            user_name: "Bob".to_owned(),
            bits: 250,
            message: "Cheer250".to_owned(),
        });
        // Subscribe -> 1.0 pulse.
        bridge.ingest_event(TwitchEvent::Subscribe {
            user_name: "Carol".to_owned(),
            tier: 1,
            is_gift: false,
        });
        // Raid 50 viewers -> 0.5 burst.
        bridge.ingest_event(TwitchEvent::Raid {
            from_broadcaster: "DJRaider".to_owned(),
            viewers: 50,
        });
        // Hype train level 3, 50% progress -> (3.5 / 6.0).
        bridge.ingest_event(TwitchEvent::HypeTrainProgress {
            level: 3,
            progress: 500,
            goal: 1000,
        });

        let t0 = bridge.current_uniforms();
        assert!((scalar(&t0, UNIFORM_TWITCH_BITS) - 0.75).abs() < 1e-6);
        assert!((scalar(&t0, UNIFORM_TWITCH_SUB_PULSE) - 1.0).abs() < 1e-6);
        assert!((scalar(&t0, UNIFORM_TWITCH_RAID_VIEWERS) - 0.5).abs() < 1e-6);
        let expected_hype = 3.5_f32 / 6.0_f32;
        assert!((scalar(&t0, UNIFORM_TWITCH_HYPE_LEVEL) - expected_hype).abs() < 1e-6);

        // Advance by 2.0 seconds and verify exact exp(-dt / decay_secs) decay.
        let dt = 2.0_f32;
        let t1 = bridge.tick(dt);
        assert!(
            (scalar(&t1, UNIFORM_TWITCH_BITS) - 0.75 * (-dt / DEFAULT_BITS_DECAY_SECS).exp()).abs()
                < 1e-5
        );
        assert!(
            (scalar(&t1, UNIFORM_TWITCH_SUB_PULSE) - (-dt / DEFAULT_SUB_DECAY_SECS).exp()).abs()
                < 1e-5
        );
        assert!(
            (scalar(&t1, UNIFORM_TWITCH_RAID_VIEWERS)
                - 0.5 * (-dt / DEFAULT_RAID_DECAY_SECS).exp())
            .abs()
                < 1e-5
        );
        assert!(
            (scalar(&t1, UNIFORM_TWITCH_HYPE_LEVEL)
                - expected_hype * (-dt / DEFAULT_HYPE_DECAY_SECS).exp())
            .abs()
                < 1e-5
        );

        // Large elapsed time decays cleanly to 0.0 (no denormals / NaNs).
        let t_end = bridge.tick(500.0);
        assert_eq!(scalar(&t_end, UNIFORM_TWITCH_BITS), 0.0);
        assert_eq!(scalar(&t_end, UNIFORM_TWITCH_SUB_PULSE), 0.0);
        assert_eq!(scalar(&t_end, UNIFORM_TWITCH_RAID_VIEWERS), 0.0);
        assert_eq!(scalar(&t_end, UNIFORM_TWITCH_HYPE_LEVEL), 0.0);
    }

    #[test]
    fn channel_point_redemption_and_chat_command_trigger_rules() {
        let mut bridge = TwitchVisualBridge::with_rules(vec![
            TwitchTriggerRule {
                event_kind: TwitchEventKind::ChatCommand,
                reward_title: None,
                chat_command: Some("!scene".to_owned()),
                action: TwitchVisualAction::SwitchScene {
                    scene_id: CHAT_ARGS_PLACEHOLDER.to_owned(),
                },
            },
            TwitchTriggerRule {
                event_kind: TwitchEventKind::ChannelPointRedemption,
                reward_title: Some("Glitch Drop".to_owned()),
                chat_command: None,
                action: TwitchVisualAction::TriggerSnap { slot: 3 },
            },
            TwitchTriggerRule {
                event_kind: TwitchEventKind::ChannelPointRedemption,
                reward_title: Some("Glitch Drop".to_owned()),
                chat_command: None,
                action: TwitchVisualAction::SetControl {
                    control_index: 2,
                    value: 0.95,
                },
            },
            TwitchTriggerRule {
                event_kind: TwitchEventKind::ChannelPointRedemption,
                reward_title: Some("Glitch Drop".to_owned()),
                chat_command: None,
                action: TwitchVisualAction::PulseUniform {
                    uniform_name: "chroma_Data_GlitchDrop".to_owned(),
                    peak: 1.0,
                    decay_secs: 2.0,
                },
            },
        ]);

        // Chat command `!scene neon_tunnel` triggers SwitchScene { scene_id: "neon_tunnel" }.
        let chat_evt = TwitchEvent::parse_chat_message("ModUser", "!SCENE neon_tunnel").unwrap();
        let chat_actions = bridge.ingest_event(chat_evt);
        assert_eq!(
            chat_actions,
            vec![TwitchVisualAction::SwitchScene {
                scene_id: "neon_tunnel".to_owned(),
            }]
        );

        // Non-matching chat command produces no actions.
        let other_chat = TwitchEvent::parse_chat_message("Viewer", "!uptime").unwrap();
        assert!(bridge.ingest_event(other_chat).is_empty());

        // Channel Point Redemption matches case-insensitively and fires all 3 actions.
        let redeem_actions = bridge.ingest_event(TwitchEvent::ChannelPointRedemption {
            user_name: "VIP".to_owned(),
            reward_id: "rw-1".to_owned(),
            reward_title: "glitch drop".to_owned(),
            user_input: String::new(),
        });
        assert_eq!(
            redeem_actions,
            vec![
                TwitchVisualAction::TriggerSnap { slot: 3 },
                TwitchVisualAction::SetControl {
                    control_index: 2,
                    value: 0.95,
                },
                TwitchVisualAction::PulseUniform {
                    uniform_name: "chroma_Data_GlitchDrop".to_owned(),
                    peak: 1.0,
                    decay_secs: 2.0,
                },
            ]
        );

        // Custom PulseUniform is also tracked and decayed by tick().
        let after_1s = bridge.tick(1.0);
        assert!((scalar(&after_1s, "chroma_Data_GlitchDrop") - (-0.5_f32).exp()).abs() < 1e-5);
    }

    #[test]
    fn bridge_uniforms_feed_directly_into_data_ingest_engine() {
        let mut engine = DataIngestEngine::new();
        engine
            .register_source(DataSourceSpec {
                id: "twitch_sub".to_owned(),
                uniform_name: UNIFORM_TWITCH_SUB_PULSE.to_owned(),
                transport: DataTransport::WebSocket,
                opt_in: true,
            })
            .unwrap();

        let mut bridge = TwitchVisualBridge::new();
        bridge.set_connection_state(TwitchConnectionState::RehearsalCassette);
        bridge.ingest_event(TwitchEvent::Subscribe {
            user_name: "DemoSub".to_owned(),
            tier: 3,
            is_gift: false,
        });

        let uniforms = bridge.tick(0.5);
        let sub_val = uniforms.get(UNIFORM_TWITCH_SUB_PULSE).unwrap().clone();
        engine.publish_sample(1, "twitch_sub", sub_val).unwrap();
        assert!(scalar(engine.current_uniforms(), UNIFORM_TWITCH_SUB_PULSE) > 0.8);
    }

    #[test]
    fn trigger_pulse_validates_uniform_name_and_finite_params() {
        let mut bridge = TwitchVisualBridge::new();
        assert!(matches!(
            bridge.trigger_pulse("invalid_prefix", 1.0, 2.0),
            Err(TwitchBridgeError::InvalidUniformName { .. })
        ));
        assert!(matches!(
            bridge.trigger_pulse("chroma_Data_Good", f32::NAN, 2.0),
            Err(TwitchBridgeError::NonFiniteParameter { name: "peak" })
        ));
        assert!(matches!(
            bridge.trigger_pulse("chroma_Data_Good", 1.0, 0.0),
            Err(TwitchBridgeError::NonFiniteParameter { name: "decay_secs" })
        ));
        bridge.trigger_pulse("chroma_Data_Good", 0.8, 2.0).unwrap();
        assert!((scalar(&bridge.current_uniforms(), "chroma_Data_Good") - 0.8).abs() < 1e-6);
    }
}
