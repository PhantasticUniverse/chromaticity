// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Broadcast streamer watchdog, secret redaction, and adaptive bitrate controller (Phase 7b).
//!
//! Provides a pure-Rust, deterministic control plane for live broadcast outputs
//! (`RTMP`, `RTMPS`, and `WHIP`) with:
//!
//! 1. **Secret-safe credentials** ([`StreamSecretKey`]) whose [`fmt::Debug`] and
//!    [`fmt::Display`] implementations unconditionally emit `"[REDACTED]"` and
//!    which can scrub leaked tokens out of transport error strings before they
//!    reach logs, diagnostics, or UI state.
//! 2. **Strict endpoint configuration validation** ([`StreamEndpointConfig`])
//!    enforcing protocol-matched URL schemes (`rtmp://`, `rtmps://`, `https://`),
//!    non-empty hosts and keys, and sane bitrate bounds.
//! 3. **Explicit lifecycle state machine** ([`StreamState`] driven by
//!    [`BroadcastStreamer`]) covering `Offline`, `Connecting`, `Live`,
//!    `Reconnecting`, and `Failed`.
//! 4. **Exponential-backoff reconnect watchdog** ([`ReconnectWatchdog`]) scaling
//!    `1s -> 2s -> 4s -> 8s -> 16s -> 30s` with a deterministic jitter hook and
//!    `max_reconnect_attempts` enforcement.
//! 5. **AIMD adaptive bitrate controller** ([`AdaptiveBitrateController`]) that
//!    steps bitrate down by 20% down to `min_bitrate_kbps` on frame-drop or RTT
//!    spikes, holds a cooldown window, and gently probes upward by `+250 kbps`
//!    on sustained clean windows back up to `target_bitrate_kbps`.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Sentinel string emitted whenever a [`StreamSecretKey`] is formatted or redacted.
pub const REDACTED_SECRET_PLACEHOLDER: &str = "[REDACTED]";

/// Minimum permitted broadcast video bitrate in kbps (300 kbps).
pub const MIN_ALLOWED_BITRATE_KBPS: u32 = 300;

/// Maximum permitted broadcast video bitrate in kbps (50 Mbps).
pub const MAX_ALLOWED_BITRATE_KBPS: u32 = 50_000;

/// Initial reconnect backoff delay in milliseconds (1 second).
pub const BASE_RECONNECT_DELAY_MS: u64 = 1_000;

/// Maximum reconnect backoff delay cap in milliseconds (30 seconds).
pub const MAX_RECONNECT_DELAY_MS: u64 = 30_000;

/// Default frame-drop fraction threshold (2%) that triggers an AIMD step-down.
pub const DEFAULT_ABR_DROP_RATE_THRESHOLD: f32 = 0.02;

/// Default RTT threshold in milliseconds (250 ms) that triggers an AIMD step-down.
pub const DEFAULT_ABR_RTT_SPIKE_MS: u32 = 250;

/// Default AIMD additive upward probe step in kbps (+250 kbps).
pub const DEFAULT_ABR_ADDITIVE_INCREASE_KBPS: u32 = 250;

/// Default AIMD cooldown duration in milliseconds after a congestion step-down (5 seconds).
pub const DEFAULT_ABR_COOLDOWN_MS: u64 = 5_000;

/// Default sustained clean window in milliseconds required before each upward probe (3 seconds).
pub const DEFAULT_ABR_CLEAN_WINDOW_MS: u64 = 3_000;

/// Supported live broadcast ingest protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamProtocol {
    /// Unencrypted Real-Time Messaging Protocol (`rtmp://`).
    Rtmp,
    /// TLS-encrypted Real-Time Messaging Protocol (`rtmps://`).
    Rtmps,
    /// WebRTC-HTTP Ingestion Protocol (`https://`).
    Whip,
}

impl StreamProtocol {
    /// Returns the exact URL scheme prefix required for this protocol.
    #[must_use]
    pub const fn required_scheme(self) -> &'static str {
        match self {
            Self::Rtmp => "rtmp://",
            Self::Rtmps => "rtmps://",
            Self::Whip => "https://",
        }
    }

    /// Returns a stable lowercase diagnostic label for this protocol.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rtmp => "rtmp",
            Self::Rtmps => "rtmps",
            Self::Whip => "whip",
        }
    }
}

impl fmt::Display for StreamProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Wrapper for broadcast stream keys and WHIP bearer tokens.
///
/// Both [`fmt::Debug`] and [`fmt::Display`] unconditionally format as
/// `"[REDACTED]"` so credentials can never leak into logs, traces, or panic
/// messages. Intentionally does **not** implement `serde::Serialize` so keys
/// cannot accidentally be serialized into diagnostic dumps.
#[derive(Clone)]
pub struct StreamSecretKey(String);

impl StreamSecretKey {
    /// Wraps a raw stream key or bearer token.
    #[must_use]
    pub fn new(secret: impl Into<String>) -> Self {
        Self(secret.into())
    }

    /// Explicitly exposes the raw secret string for transport authentication.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Returns `true` if the underlying secret string has zero length.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the underlying secret is empty or whitespace-only.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }

    /// Returns the byte length of the underlying secret.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Replaces any occurrence of the secret inside `text` with `"[REDACTED]"`.
    #[must_use]
    pub fn redact_in_text(&self, text: &str) -> String {
        if self.0.is_empty() {
            return text.to_owned();
        }
        text.replace(&self.0, REDACTED_SECRET_PLACEHOLDER)
    }
}

impl PartialEq for StreamSecretKey {
    fn eq(&self, other: &Self) -> bool {
        let a = self.0.as_bytes();
        let b = other.0.as_bytes();
        if a.len() != b.len() {
            return false;
        }
        let mut diff = 0u8;
        for (&x, &y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }
}

impl Eq for StreamSecretKey {}

impl fmt::Debug for StreamSecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED_SECRET_PLACEHOLDER)
    }
}

impl fmt::Display for StreamSecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED_SECRET_PLACEHOLDER)
    }
}

impl From<String> for StreamSecretKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for StreamSecretKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Errors returned by endpoint validation, ABR construction, or streamer state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BroadcastStreamError {
    /// The configured ingest URL was empty or whitespace-only.
    EmptyIngestUrl,
    /// The configured ingest URL contained whitespace or ASCII control characters.
    InvalidUrlCharacters,
    /// The ingest URL scheme did not match the selected [`StreamProtocol`].
    ProtocolSchemeMismatch {
        /// Selected broadcast protocol.
        protocol: StreamProtocol,
        /// Exact scheme prefix required for `protocol`.
        expected_scheme: &'static str,
    },
    /// The ingest URL had no host component after its scheme prefix.
    MissingIngestHost,
    /// The stream key or bearer token was empty or whitespace-only.
    EmptyStreamKey,
    /// The stream key or bearer token contained forbidden ASCII control characters.
    InvalidStreamKeyCharacters,
    /// One or both bitrate bounds fell outside `[MIN_ALLOWED_BITRATE_KBPS, MAX_ALLOWED_BITRATE_KBPS]`.
    BitrateOutOfBounds {
        /// Requested minimum bitrate in kbps.
        min_bitrate_kbps: u32,
        /// Requested target bitrate in kbps.
        target_bitrate_kbps: u32,
        /// Lower bound permitted by runtime policy.
        allowed_min_kbps: u32,
        /// Upper bound permitted by runtime policy.
        allowed_max_kbps: u32,
    },
    /// `min_bitrate_kbps` exceeded `target_bitrate_kbps`.
    InvalidBitrateOrder {
        /// Requested minimum bitrate in kbps.
        min_bitrate_kbps: u32,
        /// Requested target bitrate in kbps.
        target_bitrate_kbps: u32,
    },
    /// A lifecycle transition was requested in a [`StreamState`] that forbids it.
    InvalidStateTransition {
        /// Label of the current [`StreamState`].
        from: &'static str,
        /// Name of the attempted action.
        action: &'static str,
    },
}

impl fmt::Display for BroadcastStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIngestUrl => f.write_str("broadcast ingest URL must not be empty"),
            Self::InvalidUrlCharacters => {
                f.write_str("broadcast ingest URL must not contain whitespace or control chars")
            }
            Self::ProtocolSchemeMismatch {
                protocol,
                expected_scheme,
            } => write!(
                f,
                "ingest URL scheme does not match protocol {protocol} (expected {expected_scheme})"
            ),
            Self::MissingIngestHost => {
                f.write_str("broadcast ingest URL must include a non-empty host")
            }
            Self::EmptyStreamKey => f.write_str("broadcast stream key must not be empty"),
            Self::InvalidStreamKeyCharacters => {
                f.write_str("broadcast stream key must not contain control characters")
            }
            Self::BitrateOutOfBounds {
                min_bitrate_kbps,
                target_bitrate_kbps,
                allowed_min_kbps,
                allowed_max_kbps,
            } => write!(
                f,
                "bitrate bounds ({min_bitrate_kbps}..={target_bitrate_kbps} kbps) outside allowed range {allowed_min_kbps}..={allowed_max_kbps} kbps"
            ),
            Self::InvalidBitrateOrder {
                min_bitrate_kbps,
                target_bitrate_kbps,
            } => write!(
                f,
                "min_bitrate_kbps ({min_bitrate_kbps}) must be <= target_bitrate_kbps ({target_bitrate_kbps})"
            ),
            Self::InvalidStateTransition { from, action } => {
                write!(
                    f,
                    "cannot perform `{action}` while stream state is `{from}`"
                )
            }
        }
    }
}

impl std::error::Error for BroadcastStreamError {}

/// Validated configuration for a live broadcast ingest destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEndpointConfig {
    /// Transport protocol (`RTMP`, `RTMPS`, or `WHIP`).
    pub protocol: StreamProtocol,
    /// Full ingest URL (must match [`StreamProtocol::required_scheme`]).
    pub ingest_url: String,
    /// Redacted stream key or WHIP bearer token.
    pub stream_key: StreamSecretKey,
    /// Target steady-state video bitrate in kbps.
    pub target_bitrate_kbps: u32,
    /// Floor video bitrate in kbps during congestion step-down.
    pub min_bitrate_kbps: u32,
    /// Maximum automatic reconnect attempts before entering [`StreamState::Failed`].
    pub max_reconnect_attempts: u32,
}

impl StreamEndpointConfig {
    /// Validates protocol/URL scheme agreement, host presence, secret key hygiene,
    /// and bitrate bounds.
    pub fn validate(&self) -> Result<(), BroadcastStreamError> {
        if self.ingest_url.trim().is_empty() {
            return Err(BroadcastStreamError::EmptyIngestUrl);
        }
        if self
            .ingest_url
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
        {
            return Err(BroadcastStreamError::InvalidUrlCharacters);
        }

        let expected_scheme = self.protocol.required_scheme();
        let Some(after_scheme) = self.ingest_url.strip_prefix(expected_scheme) else {
            return Err(BroadcastStreamError::ProtocolSchemeMismatch {
                protocol: self.protocol,
                expected_scheme,
            });
        };

        let host_segment = after_scheme
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default();
        let host_only = host_segment.split(':').next().unwrap_or_default();
        if host_only.is_empty() {
            return Err(BroadcastStreamError::MissingIngestHost);
        }

        if self.stream_key.is_blank() {
            return Err(BroadcastStreamError::EmptyStreamKey);
        }
        if self
            .stream_key
            .expose_secret()
            .chars()
            .any(char::is_control)
        {
            return Err(BroadcastStreamError::InvalidStreamKeyCharacters);
        }

        validate_bitrate_bounds(self.min_bitrate_kbps, self.target_bitrate_kbps)?;

        Ok(())
    }
}

/// Validates minimum and target bitrate bounds against runtime policy.
fn validate_bitrate_bounds(
    min_bitrate_kbps: u32,
    target_bitrate_kbps: u32,
) -> Result<(), BroadcastStreamError> {
    if min_bitrate_kbps > target_bitrate_kbps {
        return Err(BroadcastStreamError::InvalidBitrateOrder {
            min_bitrate_kbps,
            target_bitrate_kbps,
        });
    }
    if min_bitrate_kbps < MIN_ALLOWED_BITRATE_KBPS || target_bitrate_kbps > MAX_ALLOWED_BITRATE_KBPS
    {
        return Err(BroadcastStreamError::BitrateOutOfBounds {
            min_bitrate_kbps,
            target_bitrate_kbps,
            allowed_min_kbps: MIN_ALLOWED_BITRATE_KBPS,
            allowed_max_kbps: MAX_ALLOWED_BITRATE_KBPS,
        });
    }
    Ok(())
}

/// Explicit lifecycle state machine for a live broadcast stream.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StreamState {
    /// Streamer is idle and not connected to an ingest server.
    #[default]
    Offline,
    /// Transport handshake is in progress.
    Connecting {
        /// Active ingest protocol.
        protocol: StreamProtocol,
        /// 1-based connection attempt counter.
        attempt: u32,
    },
    /// Actively pushing media frames to the ingest endpoint.
    Live {
        /// Monotonic timestamp (ms) when the session entered `Live`.
        started_ms: u64,
        /// Cumulative bytes transmitted during the current `Live` session.
        bytes_sent: u64,
        /// Cumulative dropped frames observed during the current `Live` session.
        dropped_frames: u64,
        /// Current encoder target bitrate in kbps selected by AIMD.
        current_bitrate_kbps: u32,
        /// Most recently measured transport round-trip time in milliseconds.
        rtt_ms: u32,
    },
    /// Waiting for exponential backoff timer before the next reconnect attempt.
    Reconnecting {
        /// 1-based reconnect retry counter (`1..=max_reconnect_attempts`).
        attempt: u32,
        /// Monotonic timestamp (ms) when the next reconnect attempt is due.
        next_retry_ms: u64,
        /// Secret-redacted summary of the most recent transport error.
        last_error: String,
    },
    /// Terminal failure state reached after exhausting `max_reconnect_attempts`.
    Failed {
        /// Secret-redacted failure explanation.
        reason: String,
    },
}

impl StreamState {
    /// Returns a stable lowercase diagnostic label for the current state.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Connecting { .. } => "connecting",
            Self::Live { .. } => "live",
            Self::Reconnecting { .. } => "reconnecting",
            Self::Failed { .. } => "failed",
        }
    }

    /// Returns `true` when the streamer is connecting, live, or reconnecting.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Connecting { .. } | Self::Live { .. } | Self::Reconnecting { .. }
        )
    }
}

/// Deterministic jitter hook invoked by [`ReconnectWatchdog`].
///
/// Receives `(attempt, base_delay_ms)` and returns a signed millisecond delta
/// applied to `base_delay_ms` (clamped to `0..=MAX_RECONNECT_DELAY_MS`).
pub type JitterHook = fn(u32, u64) -> i64;

/// Default zero-jitter hook returning `0 ms` offset.
#[must_use]
pub fn zero_jitter(_attempt: u32, _base_delay_ms: u64) -> i64 {
    0
}

/// Scheduled reconnect plan emitted by [`ReconnectWatchdog::schedule_retry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconnectPlan {
    /// 1-based reconnect attempt number (`1..=max_reconnect_attempts`).
    pub attempt: u32,
    /// Computed backoff delay in milliseconds (including jitter, capped at 30s).
    pub delay_ms: u64,
    /// Monotonic deadline timestamp (`now_ms + delay_ms`).
    pub next_retry_ms: u64,
}

/// Exponential-backoff reconnect watchdog with deterministic jitter and retry cap.
///
/// Base backoff schedule: `1s -> 2s -> 4s -> 8s -> 16s -> 30s` (capped at
/// [`MAX_RECONNECT_DELAY_MS`]).
#[derive(Debug, Clone, Copy)]
pub struct ReconnectWatchdog {
    max_reconnect_attempts: u32,
    attempts: u32,
    jitter_hook: JitterHook,
}

impl ReconnectWatchdog {
    /// Creates a watchdog with zero jitter and the given maximum reconnect attempts.
    #[must_use]
    pub const fn new(max_reconnect_attempts: u32) -> Self {
        Self {
            max_reconnect_attempts,
            attempts: 0,
            jitter_hook: zero_jitter,
        }
    }

    /// Creates a watchdog with a caller-supplied deterministic jitter hook.
    #[must_use]
    pub const fn with_jitter_hook(max_reconnect_attempts: u32, jitter_hook: JitterHook) -> Self {
        Self {
            max_reconnect_attempts,
            attempts: 0,
            jitter_hook,
        }
    }

    /// Returns the pure exponential backoff delay before jitter for a 1-based `attempt`.
    ///
    /// Schedule: attempt 1 -> `1_000 ms`, 2 -> `2_000 ms`, 3 -> `4_000 ms`,
    /// 4 -> `8_000 ms`, 5 -> `16_000 ms`, 6+ -> `30_000 ms`.
    #[must_use]
    pub const fn base_delay_ms(attempt: u32) -> u64 {
        let shift = attempt.saturating_sub(1);
        if shift >= 5 {
            return MAX_RECONNECT_DELAY_MS;
        }
        let raw = BASE_RECONNECT_DELAY_MS.saturating_mul(1u64 << shift);
        if raw > MAX_RECONNECT_DELAY_MS {
            MAX_RECONNECT_DELAY_MS
        } else {
            raw
        }
    }

    /// Returns the number of reconnect attempts scheduled since the last reset.
    #[must_use]
    pub const fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Returns the configured maximum number of reconnect attempts.
    #[must_use]
    pub const fn max_attempts(&self) -> u32 {
        self.max_reconnect_attempts
    }

    /// Resets the attempt counter back to `0` (e.g., once the stream reaches `Live`).
    pub fn reset(&mut self) {
        self.attempts = 0;
    }

    /// Schedules the next reconnect attempt at `now_ms`, or returns `None` if
    /// `max_reconnect_attempts` has been exhausted.
    pub fn schedule_retry(&mut self, now_ms: u64) -> Option<ReconnectPlan> {
        if self.attempts >= self.max_reconnect_attempts {
            return None;
        }
        self.attempts = self.attempts.saturating_add(1);
        let base_delay = Self::base_delay_ms(self.attempts);
        let jitter_delta = (self.jitter_hook)(self.attempts, base_delay);
        let delay_ms = if jitter_delta >= 0 {
            base_delay
                .saturating_add(jitter_delta.unsigned_abs())
                .min(MAX_RECONNECT_DELAY_MS)
        } else {
            base_delay.saturating_sub(jitter_delta.unsigned_abs())
        };
        Some(ReconnectPlan {
            attempt: self.attempts,
            delay_ms,
            next_retry_ms: now_ms.saturating_add(delay_ms),
        })
    }
}

/// Per-interval transport telemetry sample fed into [`AdaptiveBitrateController`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamTelemetrySample {
    /// Monotonic sample timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Number of frames transmitted during this sampling interval.
    pub frames_sent: u64,
    /// Number of frames dropped during this sampling interval.
    pub frames_dropped: u64,
    /// Observed round-trip time in milliseconds.
    pub rtt_ms: u32,
}

impl StreamTelemetrySample {
    /// Computes the frame drop fraction in `[0.0, 1.0]` for this interval.
    #[must_use]
    pub fn drop_rate(&self) -> f32 {
        let total = self.frames_sent.saturating_add(self.frames_dropped);
        if total == 0 {
            return 0.0;
        }
        (self.frames_dropped as f64 / total as f64) as f32
    }
}

/// Bitrate adjustment decision emitted by [`AdaptiveBitrateController::evaluate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbrAction {
    /// Maintain the current bitrate.
    Hold,
    /// Multiplicative step-down triggered by frame drops or RTT spike.
    Decrease {
        /// Bitrate before the step-down (kbps).
        from_kbps: u32,
        /// New lower bitrate (kbps), floored at `min_bitrate_kbps`.
        to_kbps: u32,
    },
    /// Additive upward probe after cooldown and sustained clean frames.
    Increase {
        /// Bitrate before the upward probe (kbps).
        from_kbps: u32,
        /// New higher bitrate (kbps), capped at `target_bitrate_kbps`.
        to_kbps: u32,
    },
}

/// Additive-Increase / Multiplicative-Decrease (AIMD) adaptive bitrate controller.
///
/// - **Multiplicative decrease:** drops bitrate by 20% (`* 4 / 5`) down to
///   `min_bitrate_kbps` whenever `drop_rate >= drop_rate_threshold` or
///   `rtt_ms >= rtt_spike_ms`, and enters a cooldown window.
/// - **Cooldown hold:** suppresses upward probes until `cooldown_ms` has elapsed
///   since the last congestion event.
/// - **Additive increase:** once cooldown has expired and frames remain clean
///   across `clean_window_ms`, probes upward by `+250 kbps` (configurable) up to
///   `target_bitrate_kbps`.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveBitrateController {
    min_bitrate_kbps: u32,
    target_bitrate_kbps: u32,
    current_bitrate_kbps: u32,
    drop_rate_threshold: f32,
    rtt_spike_ms: u32,
    probe_step_kbps: u32,
    cooldown_ms: u64,
    clean_window_ms: u64,
    cooldown_until_ms: Option<u64>,
    clean_since_ms: Option<u64>,
}

impl AdaptiveBitrateController {
    /// Creates an AIMD controller starting at `target_bitrate_kbps`.
    pub fn new(
        min_bitrate_kbps: u32,
        target_bitrate_kbps: u32,
    ) -> Result<Self, BroadcastStreamError> {
        validate_bitrate_bounds(min_bitrate_kbps, target_bitrate_kbps)?;
        Ok(Self {
            min_bitrate_kbps,
            target_bitrate_kbps,
            current_bitrate_kbps: target_bitrate_kbps,
            drop_rate_threshold: DEFAULT_ABR_DROP_RATE_THRESHOLD,
            rtt_spike_ms: DEFAULT_ABR_RTT_SPIKE_MS,
            probe_step_kbps: DEFAULT_ABR_ADDITIVE_INCREASE_KBPS,
            cooldown_ms: DEFAULT_ABR_COOLDOWN_MS,
            clean_window_ms: DEFAULT_ABR_CLEAN_WINDOW_MS,
            cooldown_until_ms: None,
            clean_since_ms: None,
        })
    }

    /// Overrides the drop-rate and RTT congestion thresholds.
    #[must_use]
    pub const fn with_thresholds(mut self, drop_rate_threshold: f32, rtt_spike_ms: u32) -> Self {
        self.drop_rate_threshold = drop_rate_threshold;
        self.rtt_spike_ms = rtt_spike_ms;
        self
    }

    /// Overrides the post-congestion cooldown and sustained clean-probe window durations.
    #[must_use]
    pub const fn with_windows(mut self, cooldown_ms: u64, clean_window_ms: u64) -> Self {
        self.cooldown_ms = cooldown_ms;
        self.clean_window_ms = clean_window_ms;
        self
    }

    /// Returns the active encoder bitrate in kbps.
    #[must_use]
    pub const fn current_bitrate_kbps(&self) -> u32 {
        self.current_bitrate_kbps
    }

    /// Returns the configured floor bitrate in kbps.
    #[must_use]
    pub const fn min_bitrate_kbps(&self) -> u32 {
        self.min_bitrate_kbps
    }

    /// Returns the configured target ceiling bitrate in kbps.
    #[must_use]
    pub const fn target_bitrate_kbps(&self) -> u32 {
        self.target_bitrate_kbps
    }

    /// Returns the monotonic timestamp (ms) until which upward probing is suppressed.
    #[must_use]
    pub const fn cooldown_until_ms(&self) -> Option<u64> {
        self.cooldown_until_ms
    }

    /// Resets bitrate to `target_bitrate_kbps` and clears cooldown/clean tracking.
    pub fn reset(&mut self) {
        self.current_bitrate_kbps = self.target_bitrate_kbps;
        self.cooldown_until_ms = None;
        self.clean_since_ms = None;
    }

    /// Evaluates a telemetry sample and returns the resulting [`AbrAction`].
    pub fn evaluate(&mut self, sample: StreamTelemetrySample) -> AbrAction {
        let congested =
            sample.drop_rate() >= self.drop_rate_threshold || sample.rtt_ms >= self.rtt_spike_ms;

        if congested {
            self.clean_since_ms = None;
            self.cooldown_until_ms = Some(sample.timestamp_ms.saturating_add(self.cooldown_ms));

            // 20% multiplicative decrease using exact integer arithmetic (`* 4 / 5`).
            let reduced = ((u64::from(self.current_bitrate_kbps) * 4) / 5) as u32;
            let next = reduced.max(self.min_bitrate_kbps);
            if next < self.current_bitrate_kbps {
                let from_kbps = self.current_bitrate_kbps;
                self.current_bitrate_kbps = next;
                return AbrAction::Decrease {
                    from_kbps,
                    to_kbps: next,
                };
            }
            return AbrAction::Hold;
        }

        if self.current_bitrate_kbps >= self.target_bitrate_kbps {
            self.clean_since_ms = None;
            return AbrAction::Hold;
        }

        if let Some(until_ms) = self.cooldown_until_ms {
            if sample.timestamp_ms < until_ms {
                self.clean_since_ms = None;
                return AbrAction::Hold;
            }
            self.cooldown_until_ms = None;
        }

        let Some(since_ms) = self.clean_since_ms else {
            self.clean_since_ms = Some(sample.timestamp_ms);
            if self.clean_window_ms > 0 {
                return AbrAction::Hold;
            }
            let from_kbps = self.current_bitrate_kbps;
            let next = from_kbps
                .saturating_add(self.probe_step_kbps)
                .min(self.target_bitrate_kbps);
            self.current_bitrate_kbps = next;
            return if next > from_kbps {
                AbrAction::Increase {
                    from_kbps,
                    to_kbps: next,
                }
            } else {
                AbrAction::Hold
            };
        };

        if sample.timestamp_ms.saturating_sub(since_ms) >= self.clean_window_ms {
            let from_kbps = self.current_bitrate_kbps;
            let next = from_kbps
                .saturating_add(self.probe_step_kbps)
                .min(self.target_bitrate_kbps);
            self.clean_since_ms = Some(sample.timestamp_ms);
            if next > from_kbps {
                self.current_bitrate_kbps = next;
                return AbrAction::Increase {
                    from_kbps,
                    to_kbps: next,
                };
            }
        }

        AbrAction::Hold
    }
}

/// Stateful broadcast streamer coordinating [`StreamState`], [`ReconnectWatchdog`],
/// and [`AdaptiveBitrateController`].
#[derive(Debug, Clone)]
pub struct BroadcastStreamer {
    config: StreamEndpointConfig,
    state: StreamState,
    watchdog: ReconnectWatchdog,
    abr: AdaptiveBitrateController,
}

impl BroadcastStreamer {
    /// Creates a new streamer after validating `config`.
    pub fn new(config: StreamEndpointConfig) -> Result<Self, BroadcastStreamError> {
        Self::with_jitter_hook(config, zero_jitter)
    }

    /// Creates a new streamer with a custom deterministic reconnect jitter hook.
    pub fn with_jitter_hook(
        config: StreamEndpointConfig,
        jitter_hook: JitterHook,
    ) -> Result<Self, BroadcastStreamError> {
        config.validate()?;
        let abr =
            AdaptiveBitrateController::new(config.min_bitrate_kbps, config.target_bitrate_kbps)?;
        let watchdog =
            ReconnectWatchdog::with_jitter_hook(config.max_reconnect_attempts, jitter_hook);
        Ok(Self {
            config,
            state: StreamState::Offline,
            watchdog,
            abr,
        })
    }

    /// Returns the validated endpoint configuration.
    #[must_use]
    pub const fn config(&self) -> &StreamEndpointConfig {
        &self.config
    }

    /// Returns the current [`StreamState`].
    #[must_use]
    pub const fn state(&self) -> &StreamState {
        &self.state
    }

    /// Returns a read-only view of the [`ReconnectWatchdog`].
    #[must_use]
    pub const fn watchdog(&self) -> &ReconnectWatchdog {
        &self.watchdog
    }

    /// Returns a read-only view of the [`AdaptiveBitrateController`].
    #[must_use]
    pub const fn abr(&self) -> &AdaptiveBitrateController {
        &self.abr
    }

    /// Returns a mutable handle to the [`AdaptiveBitrateController`].
    #[must_use]
    pub fn abr_mut(&mut self) -> &mut AdaptiveBitrateController {
        &mut self.abr
    }

    /// Initiates a connection from [`StreamState::Offline`] or [`StreamState::Failed`].
    pub fn start(&mut self) -> Result<&StreamState, BroadcastStreamError> {
        match self.state {
            StreamState::Offline | StreamState::Failed { .. } => {
                self.watchdog.reset();
                self.abr.reset();
                self.state = StreamState::Connecting {
                    protocol: self.config.protocol,
                    attempt: 1,
                };
                Ok(&self.state)
            }
            _ => Err(BroadcastStreamError::InvalidStateTransition {
                from: self.state.label(),
                action: "start",
            }),
        }
    }

    /// Marks the active handshake as connected and transitions into [`StreamState::Live`].
    pub fn on_connected(&mut self, now_ms: u64) -> Result<&StreamState, BroadcastStreamError> {
        match self.state {
            StreamState::Connecting { .. } => {
                self.watchdog.reset();
                self.state = StreamState::Live {
                    started_ms: now_ms,
                    bytes_sent: 0,
                    dropped_frames: 0,
                    current_bitrate_kbps: self.abr.current_bitrate_kbps(),
                    rtt_ms: 0,
                };
                Ok(&self.state)
            }
            _ => Err(BroadcastStreamError::InvalidStateTransition {
                from: self.state.label(),
                action: "on_connected",
            }),
        }
    }

    /// Processes live telemetry, updates AIMD bitrate, and refreshes [`StreamState::Live`] counters.
    pub fn on_telemetry(
        &mut self,
        sample: StreamTelemetrySample,
        bytes_sent_delta: u64,
    ) -> Result<AbrAction, BroadcastStreamError> {
        let StreamState::Live {
            bytes_sent,
            dropped_frames,
            current_bitrate_kbps,
            rtt_ms,
            ..
        } = &mut self.state
        else {
            return Err(BroadcastStreamError::InvalidStateTransition {
                from: self.state.label(),
                action: "on_telemetry",
            });
        };

        let action = self.abr.evaluate(sample);
        *bytes_sent = bytes_sent.saturating_add(bytes_sent_delta);
        *dropped_frames = dropped_frames.saturating_add(sample.frames_dropped);
        *current_bitrate_kbps = self.abr.current_bitrate_kbps();
        *rtt_ms = sample.rtt_ms;
        Ok(action)
    }

    /// Handles a transport error during [`StreamState::Connecting`] or [`StreamState::Live`],
    /// scrubbing any leaked secret key from `error` and scheduling exponential backoff or
    /// transitioning to [`StreamState::Failed`] when retries are exhausted.
    pub fn on_transport_error(
        &mut self,
        now_ms: u64,
        error: impl AsRef<str>,
    ) -> Result<&StreamState, BroadcastStreamError> {
        match self.state {
            StreamState::Connecting { .. } | StreamState::Live { .. } => {
                let redacted_error = self.config.stream_key.redact_in_text(error.as_ref());
                if let Some(plan) = self.watchdog.schedule_retry(now_ms) {
                    self.state = StreamState::Reconnecting {
                        attempt: plan.attempt,
                        next_retry_ms: plan.next_retry_ms,
                        last_error: redacted_error,
                    };
                } else {
                    self.state = StreamState::Failed {
                        reason: format!(
                            "exhausted {} reconnect attempts: {redacted_error}",
                            self.watchdog.max_attempts()
                        ),
                    };
                }
                Ok(&self.state)
            }
            _ => Err(BroadcastStreamError::InvalidStateTransition {
                from: self.state.label(),
                action: "on_transport_error",
            }),
        }
    }

    /// Advances from [`StreamState::Reconnecting`] to [`StreamState::Connecting`] once
    /// `now_ms >= next_retry_ms`. Returns `Ok(true)` if the reconnect fired, or `Ok(false)`
    /// if the backoff timer has not elapsed yet.
    pub fn poll_reconnect(&mut self, now_ms: u64) -> Result<bool, BroadcastStreamError> {
        let StreamState::Reconnecting {
            attempt,
            next_retry_ms,
            ..
        } = self.state
        else {
            return Err(BroadcastStreamError::InvalidStateTransition {
                from: self.state.label(),
                action: "poll_reconnect",
            });
        };

        if now_ms < next_retry_ms {
            return Ok(false);
        }

        self.state = StreamState::Connecting {
            protocol: self.config.protocol,
            attempt: attempt.saturating_add(1),
        };
        Ok(true)
    }

    /// Gracefully stops the stream from any state and returns to [`StreamState::Offline`].
    pub fn stop(&mut self) -> &StreamState {
        self.watchdog.reset();
        self.abr.reset();
        self.state = StreamState::Offline;
        &self.state
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-NET-HARDENING (T1) — validates broadcast stream secret redaction,
    //! protocol/URL scheme enforcement, reconnect watchdog backoff bounds, and AIMD
    //! congestion control invariants.

    use super::*;

    fn valid_rtmps_config() -> StreamEndpointConfig {
        StreamEndpointConfig {
            protocol: StreamProtocol::Rtmps,
            ingest_url: "rtmps://live.twitch.tv/app".to_owned(),
            stream_key: StreamSecretKey::new("live_99887766_superSecretToken"),
            target_bitrate_kbps: 6_000,
            min_bitrate_kbps: 1_500,
            max_reconnect_attempts: 6,
        }
    }

    fn fixed_jitter_plus_250(_attempt: u32, _base_delay_ms: u64) -> i64 {
        250
    }

    fn fixed_jitter_minus_200(_attempt: u32, _base_delay_ms: u64) -> i64 {
        -200
    }

    #[test]
    fn secret_key_debug_and_display_always_redacted() {
        let raw = "live_42_do_not_ever_log_this_token";
        let key = StreamSecretKey::new(raw);

        assert_eq!(format!("{key}"), "[REDACTED]");
        assert_eq!(format!("{key:?}"), "[REDACTED]");
        assert_eq!(format!("{key:#?}"), "[REDACTED]");
        assert!(!format!("{key:?}").contains(raw));
        assert!(!format!("{key}").contains(raw));
        assert_eq!(key.expose_secret(), raw);
        assert_eq!(key.len(), raw.len());
        assert!(!key.is_empty());

        let config = StreamEndpointConfig {
            protocol: StreamProtocol::Whip,
            ingest_url: "https://ingest.example.com/whip".to_owned(),
            stream_key: key,
            target_bitrate_kbps: 8_000,
            min_bitrate_kbps: 2_000,
            max_reconnect_attempts: 5,
        };

        let config_debug = format!("{config:?}");
        let config_pretty = format!("{config:#?}");
        assert!(config_debug.contains("[REDACTED]"));
        assert!(config_pretty.contains("[REDACTED]"));
        assert!(!config_debug.contains(raw));
        assert!(!config_pretty.contains(raw));
    }

    #[test]
    fn endpoint_config_validation_enforces_protocol_url_key_and_bitrates() {
        let rtmp_ok = StreamEndpointConfig {
            protocol: StreamProtocol::Rtmp,
            ingest_url: "rtmp://ingest.example.com:1935/live".to_owned(),
            stream_key: StreamSecretKey::from("stream-key-1"),
            target_bitrate_kbps: 4_500,
            min_bitrate_kbps: 1_000,
            max_reconnect_attempts: 3,
        };
        assert_eq!(rtmp_ok.validate(), Ok(()));

        let rtmps_ok = valid_rtmps_config();
        assert_eq!(rtmps_ok.validate(), Ok(()));

        let whip_ok = StreamEndpointConfig {
            protocol: StreamProtocol::Whip,
            ingest_url: "https://global.whip.example.com/v1/ingest".to_owned(),
            stream_key: StreamSecretKey::from("whip-bearer-token"),
            target_bitrate_kbps: 10_000,
            min_bitrate_kbps: 2_500,
            max_reconnect_attempts: 4,
        };
        assert_eq!(whip_ok.validate(), Ok(()));

        // Protocol/scheme mismatches.
        let mut wrong_scheme = valid_rtmps_config();
        wrong_scheme.ingest_url = "rtmp://live.twitch.tv/app".to_owned();
        assert_eq!(
            wrong_scheme.validate(),
            Err(BroadcastStreamError::ProtocolSchemeMismatch {
                protocol: StreamProtocol::Rtmps,
                expected_scheme: "rtmps://",
            })
        );

        let mut insecure_whip = whip_ok.clone();
        insecure_whip.ingest_url = "http://global.whip.example.com/v1/ingest".to_owned();
        assert_eq!(
            insecure_whip.validate(),
            Err(BroadcastStreamError::ProtocolSchemeMismatch {
                protocol: StreamProtocol::Whip,
                expected_scheme: "https://",
            })
        );

        // Missing host or whitespace.
        let mut missing_host = valid_rtmps_config();
        missing_host.ingest_url = "rtmps:///app".to_owned();
        assert_eq!(
            missing_host.validate(),
            Err(BroadcastStreamError::MissingIngestHost)
        );

        let mut space_in_url = valid_rtmps_config();
        space_in_url.ingest_url = "rtmps://live.twitch.tv/app bad".to_owned();
        assert_eq!(
            space_in_url.validate(),
            Err(BroadcastStreamError::InvalidUrlCharacters)
        );

        // Empty or control-character stream key.
        let mut blank_key = valid_rtmps_config();
        blank_key.stream_key = StreamSecretKey::new("   ");
        assert_eq!(
            blank_key.validate(),
            Err(BroadcastStreamError::EmptyStreamKey)
        );

        let mut newline_key = valid_rtmps_config();
        newline_key.stream_key = StreamSecretKey::new("secret\r\ninjection");
        assert_eq!(
            newline_key.validate(),
            Err(BroadcastStreamError::InvalidStreamKeyCharacters)
        );

        // Bitrate ordering and policy bounds.
        let mut inverted_bitrate = valid_rtmps_config();
        inverted_bitrate.min_bitrate_kbps = 7_000;
        inverted_bitrate.target_bitrate_kbps = 6_000;
        assert_eq!(
            inverted_bitrate.validate(),
            Err(BroadcastStreamError::InvalidBitrateOrder {
                min_bitrate_kbps: 7_000,
                target_bitrate_kbps: 6_000,
            })
        );

        let mut below_floor = valid_rtmps_config();
        below_floor.min_bitrate_kbps = 100;
        assert_eq!(
            below_floor.validate(),
            Err(BroadcastStreamError::BitrateOutOfBounds {
                min_bitrate_kbps: 100,
                target_bitrate_kbps: 6_000,
                allowed_min_kbps: MIN_ALLOWED_BITRATE_KBPS,
                allowed_max_kbps: MAX_ALLOWED_BITRATE_KBPS,
            })
        );

        let mut above_ceiling = valid_rtmps_config();
        above_ceiling.target_bitrate_kbps = 100_000;
        assert_eq!(
            above_ceiling.validate(),
            Err(BroadcastStreamError::BitrateOutOfBounds {
                min_bitrate_kbps: 1_500,
                target_bitrate_kbps: 100_000,
                allowed_min_kbps: MIN_ALLOWED_BITRATE_KBPS,
                allowed_max_kbps: MAX_ALLOWED_BITRATE_KBPS,
            })
        );
    }

    #[test]
    fn reconnect_watchdog_exponential_backoff_cap_jitter_and_limit() {
        let mut watchdog = ReconnectWatchdog::new(7);
        let expected_delays = [1_000u64, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000];

        let mut now = 10_000u64;
        for (idx, &expected_delay) in expected_delays.iter().enumerate() {
            let Some(plan) = watchdog.schedule_retry(now) else {
                panic!("expected retry plan for attempt {}", idx + 1);
            };
            assert_eq!(plan.attempt, (idx + 1) as u32);
            assert_eq!(plan.delay_ms, expected_delay);
            assert_eq!(plan.next_retry_ms, now + expected_delay);
            now = plan.next_retry_ms;
        }

        // Next retry exceeds max_reconnect_attempts (7).
        assert_eq!(watchdog.schedule_retry(now), None);
        assert_eq!(watchdog.attempts(), 7);

        // Reset restores the backoff sequence.
        watchdog.reset();
        assert_eq!(watchdog.attempts(), 0);
        let Some(first_after_reset) = watchdog.schedule_retry(50_000) else {
            panic!("expected retry plan after reset");
        };
        assert_eq!(first_after_reset.attempt, 1);
        assert_eq!(first_after_reset.delay_ms, 1_000);

        // Deterministic positive jitter respects the 30s cap.
        let mut plus_jitter = ReconnectWatchdog::with_jitter_hook(6, fixed_jitter_plus_250);
        let Some(p1) = plus_jitter.schedule_retry(0) else {
            panic!("expected plan 1");
        };
        assert_eq!(p1.delay_ms, 1_250);
        for _ in 0..4 {
            assert!(plus_jitter.schedule_retry(0).is_some());
        }
        let Some(p6) = plus_jitter.schedule_retry(0) else {
            panic!("expected plan 6");
        };
        assert_eq!(p6.delay_ms, MAX_RECONNECT_DELAY_MS);

        // Deterministic negative jitter reduces delay without underflow.
        let mut minus_jitter = ReconnectWatchdog::with_jitter_hook(2, fixed_jitter_minus_200);
        let Some(m1) = minus_jitter.schedule_retry(1_000) else {
            panic!("expected plan 1");
        };
        assert_eq!(m1.delay_ms, 800);
        assert_eq!(m1.next_retry_ms, 1_800);
    }

    #[test]
    fn full_streamer_lifecycle_and_secret_scrubbing_in_errors() {
        let mut config = valid_rtmps_config();
        config.max_reconnect_attempts = 2;
        let raw_secret = config.stream_key.expose_secret().to_owned();

        let Ok(mut streamer) = BroadcastStreamer::new(config) else {
            panic!("valid config should construct BroadcastStreamer");
        };
        assert_eq!(*streamer.state(), StreamState::Offline);

        // Start -> Connecting(attempt 1).
        let Ok(state) = streamer.start() else {
            panic!("start from Offline should succeed");
        };
        assert_eq!(
            *state,
            StreamState::Connecting {
                protocol: StreamProtocol::Rtmps,
                attempt: 1,
            }
        );

        // Connected -> Live.
        let Ok(state) = streamer.on_connected(1_000) else {
            panic!("on_connected from Connecting should succeed");
        };
        assert_eq!(
            *state,
            StreamState::Live {
                started_ms: 1_000,
                bytes_sent: 0,
                dropped_frames: 0,
                current_bitrate_kbps: 6_000,
                rtt_ms: 0,
            }
        );

        // Live telemetry updates counters.
        let Ok(action) = streamer.on_telemetry(
            StreamTelemetrySample {
                timestamp_ms: 2_000,
                frames_sent: 60,
                frames_dropped: 0,
                rtt_ms: 42,
            },
            750_000,
        ) else {
            panic!("on_telemetry in Live should succeed");
        };
        assert_eq!(action, AbrAction::Hold);
        assert_eq!(
            *streamer.state(),
            StreamState::Live {
                started_ms: 1_000,
                bytes_sent: 750_000,
                dropped_frames: 0,
                current_bitrate_kbps: 6_000,
                rtt_ms: 42,
            }
        );

        // First transport error (with leaked secret in error text) -> Reconnecting(attempt 1, +1000ms).
        let leaked_msg =
            format!("rtmps publish failed for rtmps://live.twitch.tv/app/{raw_secret}");
        let Ok(state) = streamer.on_transport_error(5_000, &leaked_msg) else {
            panic!("on_transport_error in Live should succeed");
        };
        match state {
            StreamState::Reconnecting {
                attempt,
                next_retry_ms,
                last_error,
            } => {
                assert_eq!(*attempt, 1);
                assert_eq!(*next_retry_ms, 6_000);
                assert!(last_error.contains("[REDACTED]"));
                assert!(!last_error.contains(&raw_secret));
            }
            other => panic!("expected Reconnecting, got {other:?}"),
        }

        // Poll before deadline -> stays Reconnecting.
        assert_eq!(streamer.poll_reconnect(5_999), Ok(false));
        // Poll at deadline -> Connecting(attempt 2).
        assert_eq!(streamer.poll_reconnect(6_000), Ok(true));
        assert_eq!(
            *streamer.state(),
            StreamState::Connecting {
                protocol: StreamProtocol::Rtmps,
                attempt: 2,
            }
        );

        // Second error -> Reconnecting(attempt 2, +2000ms).
        let Ok(state) = streamer.on_transport_error(6_100, "tls reset") else {
            panic!("second transport error should schedule retry 2");
        };
        assert_eq!(
            *state,
            StreamState::Reconnecting {
                attempt: 2,
                next_retry_ms: 8_100,
                last_error: "tls reset".to_owned(),
            }
        );

        assert_eq!(streamer.poll_reconnect(8_100), Ok(true));

        // Third error exhausts max_reconnect_attempts (2) -> Failed (also scrubbed).
        let Ok(state) = streamer.on_transport_error(8_200, &leaked_msg) else {
            panic!("exhausted transport error should transition to Failed");
        };
        match state {
            StreamState::Failed { reason } => {
                assert!(reason.contains("exhausted 2 reconnect attempts"));
                assert!(reason.contains("[REDACTED]"));
                assert!(!reason.contains(&raw_secret));
            }
            other => panic!("expected Failed, got {other:?}"),
        }

        // Stop from Failed -> Offline.
        assert_eq!(*streamer.stop(), StreamState::Offline);
    }

    #[test]
    fn aimd_controller_step_down_cooldown_and_upward_recovery() {
        let Ok(mut abr) = AdaptiveBitrateController::new(2_000, 6_000) else {
            panic!("valid bitrate bounds should construct AdaptiveBitrateController");
        };
        assert_eq!(abr.current_bitrate_kbps(), 6_000);

        // Clean traffic at target holds steady.
        let clean = StreamTelemetrySample {
            timestamp_ms: 1_000,
            frames_sent: 60,
            frames_dropped: 0,
            rtt_ms: 35,
        };
        assert_eq!(abr.evaluate(clean), AbrAction::Hold);
        assert_eq!(abr.current_bitrate_kbps(), 6_000);

        // Drop-rate spike (5% >= 2%) -> 20% decrease: 6000 -> 4800, cooldown until 7000ms.
        let drop_spike = StreamTelemetrySample {
            timestamp_ms: 2_000,
            frames_sent: 57,
            frames_dropped: 3,
            rtt_ms: 40,
        };
        assert_eq!(
            abr.evaluate(drop_spike),
            AbrAction::Decrease {
                from_kbps: 6_000,
                to_kbps: 4_800,
            }
        );
        assert_eq!(abr.cooldown_until_ms(), Some(7_000));

        // Clean frames during cooldown must NOT probe upward.
        let during_cooldown = StreamTelemetrySample {
            timestamp_ms: 6_500,
            frames_sent: 60,
            frames_dropped: 0,
            rtt_ms: 30,
        };
        assert_eq!(abr.evaluate(during_cooldown), AbrAction::Hold);
        assert_eq!(abr.current_bitrate_kbps(), 4_800);

        // RTT spike (>= 250 ms) triggers another 20% decrease: 4800 -> 3840, cooldown until 12000ms.
        let rtt_spike = StreamTelemetrySample {
            timestamp_ms: 7_000,
            frames_sent: 60,
            frames_dropped: 0,
            rtt_ms: 310,
        };
        assert_eq!(
            abr.evaluate(rtt_spike),
            AbrAction::Decrease {
                from_kbps: 4_800,
                to_kbps: 3_840,
            }
        );
        assert_eq!(abr.cooldown_until_ms(), Some(12_000));

        // Repeated congestion floors strictly at min_bitrate_kbps (2000).
        let mut t = 7_500u64;
        for _ in 0..8 {
            let _ = abr.evaluate(StreamTelemetrySample {
                timestamp_ms: t,
                frames_sent: 50,
                frames_dropped: 10,
                rtt_ms: 400,
            });
            t += 500;
        }
        assert_eq!(abr.current_bitrate_kbps(), 2_000);
        // Further congestion at floor returns Hold.
        assert_eq!(
            abr.evaluate(StreamTelemetrySample {
                timestamp_ms: t,
                frames_sent: 50,
                frames_dropped: 10,
                rtt_ms: 400,
            }),
            AbrAction::Hold
        );
        let cooldown_end = t + DEFAULT_ABR_COOLDOWN_MS;
        assert_eq!(abr.cooldown_until_ms(), Some(cooldown_end));

        // First clean sample after cooldown starts the sustained clean window.
        assert_eq!(
            abr.evaluate(StreamTelemetrySample {
                timestamp_ms: cooldown_end,
                frames_sent: 60,
                frames_dropped: 0,
                rtt_ms: 30,
            }),
            AbrAction::Hold
        );

        // Before clean_window_ms elapses -> Hold.
        assert_eq!(
            abr.evaluate(StreamTelemetrySample {
                timestamp_ms: cooldown_end + DEFAULT_ABR_CLEAN_WINDOW_MS - 1,
                frames_sent: 60,
                frames_dropped: 0,
                rtt_ms: 30,
            }),
            AbrAction::Hold
        );

        // At clean_window_ms -> gentle upward probe +250 kbps (2000 -> 2250).
        let first_probe_ts = cooldown_end + DEFAULT_ABR_CLEAN_WINDOW_MS;
        assert_eq!(
            abr.evaluate(StreamTelemetrySample {
                timestamp_ms: first_probe_ts,
                frames_sent: 60,
                frames_dropped: 0,
                rtt_ms: 30,
            }),
            AbrAction::Increase {
                from_kbps: 2_000,
                to_kbps: 2_250,
            }
        );

        // Sustained clean windows climb +250 kbps per window and cap cleanly at target (6000).
        let mut probe_ts = first_probe_ts;
        for _ in 0..20 {
            probe_ts += DEFAULT_ABR_CLEAN_WINDOW_MS;
            let _ = abr.evaluate(StreamTelemetrySample {
                timestamp_ms: probe_ts,
                frames_sent: 60,
                frames_dropped: 0,
                rtt_ms: 28,
            });
        }
        assert_eq!(abr.current_bitrate_kbps(), 6_000);
    }
}
