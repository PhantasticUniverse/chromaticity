// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#![forbid(unsafe_code)]

//! Phase 8 — Setlist Cue-List Runner, 2-Button Footswitch Controller, and
//! Self-Healing JSON Config Guard.
//!
//! Built for live analog bands and hybrid stage rigs:
//!
//! - [`SetlistCueRunner`] manages an ordered setlist of [`SetlistCue`] items
//!   driven by an explicit [`CueRunnerState`] finite state machine and
//!   2-button footswitch actions ([`FootswitchAction`]: next song with smooth
//!   crossfade, previous song, blackout toggle between songs, and direct cue
//!   jumps).
//! - [`load_or_self_heal_json`] validates JSON configuration files on load,
//!   quarantines corrupted, truncated, or non-UTF-8 files to
//!   `<filename>.corrupt-<epoch>.bak` without losing user data, and falls back
//!   cleanly to `T::default()`.

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Maximum JSON nesting depth accepted by the self-healing config parser.
const MAX_JSON_DEPTH: usize = 64;

/// Per-song tempo tracking mode for a [`SetlistCue`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum CueBpmMode {
    /// Disable BPM tracking (ideal for free-tempo / acoustic songs).
    #[default]
    Off,
    /// Track tempo automatically from live audio analysis.
    Auto,
    /// Lock tempo to an explicit BPM value.
    Manual(f32),
}

/// A single song / visual cue entry in a live performance setlist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetlistCue {
    /// Stable unique identifier for this cue.
    pub id: String,
    /// Human-readable song title displayed on stage.
    pub song_title: String,
    /// Target scene identifier to load for this song.
    pub scene_id: String,
    /// Optional preset name within `scene_id`.
    pub preset_name: Option<String>,
    /// Per-song BPM tracking mode.
    pub bpm_mode: CueBpmMode,
    /// Per-song audio input gain trim in decibels.
    pub input_gain_db: f32,
    /// Crossfade duration in milliseconds when entering this cue.
    pub crossfade_ms: u32,
}

/// Explicit finite state machine for [`SetlistCueRunner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CueRunnerState {
    /// No cues are loaded in the setlist.
    Empty,
    /// Settled on `active_index` with optional stage blackout.
    Ready {
        /// Index of the active cue in the setlist.
        active_index: usize,
        /// Whether stage output is currently blacked out.
        blackout: bool,
    },
    /// Actively crossfading between two cues.
    Transitioning {
        /// Index of the cue being faded out.
        from_index: usize,
        /// Index of the cue being faded in.
        to_index: usize,
        /// Remaining crossfade time in milliseconds.
        remaining_ms: u32,
        /// Whether stage output is currently blacked out.
        blackout: bool,
    },
}

/// Operator or 2-button footswitch command routed to [`SetlistCueRunner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootswitchAction {
    /// Advance to the next song in the setlist (clamped at the final cue).
    NextCue,
    /// Return to the previous song in the setlist (clamped at cue `0`).
    PrevCue,
    /// Toggle stage blackout while preserving the active cue index.
    ToggleBlackout,
    /// Jump directly to a specific zero-based cue index.
    JumpTo(usize),
}

/// Event emitted synchronously by [`SetlistCueRunner::dispatch`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CueTransitionEvent {
    /// Action produced no state change (e.g. empty setlist or out-of-bounds).
    NoOp {
        /// Unchanged runner state.
        state: CueRunnerState,
    },
    /// Switched immediately because the target cue has `crossfade_ms == 0`.
    SwitchedImmediate {
        /// Index of the previous cue.
        from_index: usize,
        /// Index of the newly active cue.
        to_index: usize,
        /// Active blackout flag.
        blackout: bool,
    },
    /// Initiated a timed crossfade transition to the target cue.
    CrossfadeStarted {
        /// Index of the cue being faded out.
        from_index: usize,
        /// Index of the cue being faded in.
        to_index: usize,
        /// Total crossfade duration in milliseconds.
        duration_ms: u32,
        /// Active blackout flag.
        blackout: bool,
    },
    /// Toggled the stage blackout state without losing cue position.
    BlackoutToggled {
        /// New blackout state (`true` = blacked out).
        blackout: bool,
        /// Preserved active (or transition target) cue index.
        active_index: usize,
    },
}

/// Deterministic setlist cue-list runner with 2-button footswitch navigation
/// and smooth crossfade progression.
#[derive(Debug, Clone, PartialEq)]
pub struct SetlistCueRunner {
    cues: Vec<SetlistCue>,
    state: CueRunnerState,
}

impl SetlistCueRunner {
    /// Constructs a runner for the given ordered setlist.
    ///
    /// An empty cue list starts in [`CueRunnerState::Empty`]; a non-empty list
    /// starts in [`CueRunnerState::Ready`] at index `0` with `blackout: false`.
    #[must_use]
    pub fn new(cues: Vec<SetlistCue>) -> Self {
        let state = if cues.is_empty() {
            CueRunnerState::Empty
        } else {
            CueRunnerState::Ready {
                active_index: 0,
                blackout: false,
            }
        };
        Self { cues, state }
    }

    /// Dispatches a [`FootswitchAction`] and returns the resulting
    /// [`CueTransitionEvent`].
    pub fn dispatch(&mut self, action: FootswitchAction) -> CueTransitionEvent {
        let (current_index, blackout) = match self.state {
            CueRunnerState::Empty => {
                return CueTransitionEvent::NoOp { state: self.state };
            }
            CueRunnerState::Ready {
                active_index,
                blackout,
            } => (active_index, blackout),
            CueRunnerState::Transitioning {
                to_index, blackout, ..
            } => (to_index, blackout),
        };

        match action {
            FootswitchAction::ToggleBlackout => {
                let next_blackout = !blackout;
                self.state = match self.state {
                    CueRunnerState::Empty => CueRunnerState::Empty,
                    CueRunnerState::Ready { active_index, .. } => CueRunnerState::Ready {
                        active_index,
                        blackout: next_blackout,
                    },
                    CueRunnerState::Transitioning {
                        from_index,
                        to_index,
                        remaining_ms,
                        ..
                    } => CueRunnerState::Transitioning {
                        from_index,
                        to_index,
                        remaining_ms,
                        blackout: next_blackout,
                    },
                };
                CueTransitionEvent::BlackoutToggled {
                    blackout: next_blackout,
                    active_index: current_index,
                }
            }
            FootswitchAction::NextCue => {
                let Some(next_index) = current_index.checked_add(1) else {
                    return CueTransitionEvent::NoOp { state: self.state };
                };
                if next_index >= self.cues.len() {
                    return CueTransitionEvent::NoOp { state: self.state };
                }
                self.transition_to(current_index, next_index, blackout)
            }
            FootswitchAction::PrevCue => {
                let Some(prev_index) = current_index.checked_sub(1) else {
                    return CueTransitionEvent::NoOp { state: self.state };
                };
                self.transition_to(current_index, prev_index, blackout)
            }
            FootswitchAction::JumpTo(target_index) => {
                if target_index >= self.cues.len() {
                    return CueTransitionEvent::NoOp { state: self.state };
                }
                if matches!(
                    self.state,
                    CueRunnerState::Ready { active_index, .. } if active_index == target_index
                ) {
                    return CueTransitionEvent::NoOp { state: self.state };
                }
                self.transition_to(current_index, target_index, blackout)
            }
        }
    }

    /// Advances any active crossfade by `dt_ms` milliseconds.
    ///
    /// When `remaining_ms` reaches `0`, the runner transitions cleanly from
    /// [`CueRunnerState::Transitioning`] to [`CueRunnerState::Ready`] at
    /// `to_index` while preserving the `blackout` flag.
    pub fn tick(&mut self, dt_ms: u32) -> CueRunnerState {
        if let CueRunnerState::Transitioning {
            from_index,
            to_index,
            remaining_ms,
            blackout,
        } = self.state
        {
            let next_remaining = remaining_ms.saturating_sub(dt_ms);
            if next_remaining == 0 {
                self.state = CueRunnerState::Ready {
                    active_index: to_index,
                    blackout,
                };
            } else {
                self.state = CueRunnerState::Transitioning {
                    from_index,
                    to_index,
                    remaining_ms: next_remaining,
                    blackout,
                };
            }
        }
        self.state
    }

    /// Returns the currently active cue (or the incoming target cue while
    /// transitioning), or `None` if the setlist is empty.
    #[must_use]
    pub fn active_cue(&self) -> Option<&SetlistCue> {
        match self.state {
            CueRunnerState::Empty => None,
            CueRunnerState::Ready { active_index, .. } => self.cues.get(active_index),
            CueRunnerState::Transitioning { to_index, .. } => self.cues.get(to_index),
        }
    }

    /// Returns the current [`CueRunnerState`].
    #[must_use]
    pub fn state(&self) -> CueRunnerState {
        self.state
    }

    /// Returns `true` when stage blackout is engaged.
    #[must_use]
    pub fn is_blackout(&self) -> bool {
        match self.state {
            CueRunnerState::Empty => false,
            CueRunnerState::Ready { blackout, .. }
            | CueRunnerState::Transitioning { blackout, .. } => blackout,
        }
    }

    /// Returns the loaded setlist cues in order.
    #[must_use]
    pub fn cues(&self) -> &[SetlistCue] {
        &self.cues
    }

    fn transition_to(
        &mut self,
        from_index: usize,
        to_index: usize,
        blackout: bool,
    ) -> CueTransitionEvent {
        let Some(target) = self.cues.get(to_index) else {
            return CueTransitionEvent::NoOp { state: self.state };
        };
        let duration_ms = target.crossfade_ms;
        if duration_ms == 0 {
            self.state = CueRunnerState::Ready {
                active_index: to_index,
                blackout,
            };
            CueTransitionEvent::SwitchedImmediate {
                from_index,
                to_index,
                blackout,
            }
        } else {
            self.state = CueRunnerState::Transitioning {
                from_index,
                to_index,
                remaining_ms: duration_ms,
                blackout,
            };
            CueTransitionEvent::CrossfadeStarted {
                from_index,
                to_index,
                duration_ms,
                blackout,
            }
        }
    }
}

/// Outcome of loading a JSON config file through [`load_or_self_heal_json`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigLoadOutcome<T> {
    /// Existing config file parsed and validated cleanly.
    Loaded(T),
    /// Existing config file was corrupted, truncated, or invalid UTF-8; it was
    /// quarantined to `quarantine_path` and `value` holds `T::default()`.
    RecoveredFromCorruption {
        /// Clean default configuration value.
        value: T,
        /// Path where the original damaged file was preserved.
        quarantine_path: PathBuf,
        /// Human-readable parse or UTF-8 validation error.
        reason: String,
    },
    /// Config file did not exist yet; `T::default()` was returned.
    CreatedDefault(T),
}

/// Loads and deserializes a JSON configuration file, self-healing if damaged.
///
/// Behavior:
/// - If `path` does not exist (`ErrorKind::NotFound`), returns
///   [`ConfigLoadOutcome::CreatedDefault`] with `T::default()`.
/// - If `path` exists and parses cleanly as RFC 8259 JSON into `T`, returns
///   [`ConfigLoadOutcome::Loaded`].
/// - If `path` contains invalid UTF-8, truncated JSON, trailing garbage, or
///   schema-mismatched JSON, renames `path` to
///   `<filename>.corrupt-<now_epoch_secs>.bak` in the same directory and
///   returns [`ConfigLoadOutcome::RecoveredFromCorruption`] with
///   `T::default()`.
/// - Propagates unexpected filesystem I/O errors (e.g. permission denied or
///   quarantine rename failure).
pub fn load_or_self_heal_json<T: DeserializeOwned + Default>(
    path: &Path,
    now_epoch_secs: u64,
) -> Result<ConfigLoadOutcome<T>, io::Error> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(ConfigLoadOutcome::CreatedDefault(T::default()));
        }
        Err(err) => return Err(err),
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(err) => {
            let quarantine_path = quarantine_corrupt_file(path, now_epoch_secs)?;
            return Ok(ConfigLoadOutcome::RecoveredFromCorruption {
                value: T::default(),
                quarantine_path,
                reason: format!("invalid UTF-8 in config file: {err}"),
            });
        }
    };

    match from_json_str::<T>(text) {
        Ok(value) => Ok(ConfigLoadOutcome::Loaded(value)),
        Err(err) => {
            let quarantine_path = quarantine_corrupt_file(path, now_epoch_secs)?;
            Ok(ConfigLoadOutcome::RecoveredFromCorruption {
                value: T::default(),
                quarantine_path,
                reason: err.to_string(),
            })
        }
    }
}

fn quarantine_corrupt_file(path: &Path, now_epoch_secs: u64) -> Result<PathBuf, io::Error> {
    let mut file_name = path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("config.json"));
    file_name.push(format!(".corrupt-{now_epoch_secs}.bak"));
    let quarantine_path = path.with_file_name(file_name);
    fs::rename(path, &quarantine_path)?;
    Ok(quarantine_path)
}

// ---------------------------------------------------------------------------
// Zero-dependency, depth-bounded RFC 8259 JSON Serde Deserializer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonError {
    message: String,
}

impl JsonError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsonError {}

impl de::Error for JsonError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

fn from_json_str<T: DeserializeOwned>(input: &str) -> Result<T, JsonError> {
    let mut deserializer = JsonDeserializer::new(input);
    let value = T::deserialize(&mut deserializer)?;
    deserializer.skip_whitespace();
    if deserializer.pos < deserializer.input.len() {
        return Err(JsonError::new(format!(
            "trailing characters at byte offset {}",
            deserializer.pos
        )));
    }
    Ok(value)
}

struct JsonDeserializer<'de> {
    input: &'de str,
    pos: usize,
    depth: usize,
}

impl<'de> JsonDeserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self {
            input,
            pos: 0,
            depth: 0,
        }
    }

    fn remaining(&self) -> &'de str {
        self.input.get(self.pos..).unwrap_or("")
    }

    fn skip_whitespace(&mut self) {
        let bytes = self.input.as_bytes();
        while let Some(&b) = bytes.get(self.pos) {
            if matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek_byte(&mut self) -> Result<u8, JsonError> {
        self.skip_whitespace();
        self.input
            .as_bytes()
            .get(self.pos)
            .copied()
            .ok_or_else(|| JsonError::new("unexpected end of JSON input"))
    }

    fn next_byte(&mut self) -> Result<u8, JsonError> {
        let b = self
            .input
            .as_bytes()
            .get(self.pos)
            .copied()
            .ok_or_else(|| JsonError::new("unexpected end of JSON input"))?;
        self.pos += 1;
        Ok(b)
    }

    fn enter_container(&mut self) -> Result<(), JsonError> {
        self.depth += 1;
        if self.depth > MAX_JSON_DEPTH {
            return Err(JsonError::new(format!(
                "JSON nesting depth exceeded maximum ({MAX_JSON_DEPTH})"
            )));
        }
        Ok(())
    }

    fn exit_container(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn parse_literal(&mut self, expected: &str) -> Result<(), JsonError> {
        self.skip_whitespace();
        if self.remaining().starts_with(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            Err(JsonError::new(format!(
                "expected `{expected}` at byte offset {}",
                self.pos
            )))
        }
    }

    fn parse_bool(&mut self) -> Result<bool, JsonError> {
        match self.peek_byte()? {
            b't' => {
                self.parse_literal("true")?;
                Ok(true)
            }
            b'f' => {
                self.parse_literal("false")?;
                Ok(false)
            }
            _ => Err(JsonError::new(format!(
                "expected boolean at byte offset {}",
                self.pos
            ))),
        }
    }

    fn parse_number_slice(&mut self) -> Result<&'de str, JsonError> {
        self.skip_whitespace();
        let start = self.pos;
        let bytes = self.input.as_bytes();

        if bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }

        match bytes.get(self.pos).copied() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                    return Err(JsonError::new(format!(
                        "leading zero in number at byte offset {start}"
                    )));
                }
            }
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => {
                return Err(JsonError::new(format!(
                    "invalid number at byte offset {start}"
                )));
            }
        }

        if bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            let frac_start = self.pos;
            while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == frac_start {
                return Err(JsonError::new(format!(
                    "missing fractional digits at byte offset {start}"
                )));
            }
        }

        if matches!(bytes.get(self.pos), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(bytes.get(self.pos), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == exp_start {
                return Err(JsonError::new(format!(
                    "missing exponent digits at byte offset {start}"
                )));
            }
        }

        self.input
            .get(start..self.pos)
            .ok_or_else(|| JsonError::new("number slice out of bounds"))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.skip_whitespace();
        if self.next_byte()? != b'"' {
            return Err(JsonError::new(format!(
                "expected string opening quote at byte offset {}",
                self.pos.saturating_sub(1)
            )));
        }

        let mut out = String::new();
        loop {
            let rest = self.remaining();
            let mut chars = rest.chars();
            let Some(ch) = chars.next() else {
                return Err(JsonError::new("unterminated JSON string"));
            };
            self.pos += ch.len_utf8();

            match ch {
                '"' => return Ok(out),
                '\\' => {
                    let esc = self.next_byte()?;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code_unit = self.parse_hex4()?;
                            if (0xD800..=0xDBFF).contains(&code_unit) {
                                if self.next_byte()? != b'\\' || self.next_byte()? != b'u' {
                                    return Err(JsonError::new("unpaired UTF-16 high surrogate"));
                                }
                                let low = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(JsonError::new("invalid UTF-16 low surrogate"));
                                }
                                let combined = 0x10000
                                    + (((u32::from(code_unit) - 0xD800) << 10)
                                        | (u32::from(low) - 0xDC00));
                                let decoded = char::from_u32(combined)
                                    .ok_or_else(|| JsonError::new("invalid Unicode scalar"))?;
                                out.push(decoded);
                            } else if (0xDC00..=0xDFFF).contains(&code_unit) {
                                return Err(JsonError::new("unexpected UTF-16 low surrogate"));
                            } else {
                                let decoded = char::from_u32(u32::from(code_unit))
                                    .ok_or_else(|| JsonError::new("invalid Unicode scalar"))?;
                                out.push(decoded);
                            }
                        }
                        other => {
                            return Err(JsonError::new(format!(
                                "invalid escape sequence `\\{}`",
                                char::from(other)
                            )));
                        }
                    }
                }
                c if (c as u32) < 0x20 => {
                    return Err(JsonError::new("unescaped control character in JSON string"));
                }
                c => out.push(c),
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u16, JsonError> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let b = self.next_byte()?;
            let digit = match b {
                b'0'..=b'9' => u16::from(b - b'0'),
                b'a'..=b'f' => u16::from(b - b'a' + 10),
                b'A'..=b'F' => u16::from(b - b'A' + 10),
                _ => return Err(JsonError::new("invalid hex digit in \\u escape")),
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }
}

impl<'de> de::Deserializer<'de> for &mut JsonDeserializer<'de> {
    type Error = JsonError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.peek_byte()? {
            b'n' => self.deserialize_unit(visitor),
            b't' | b'f' => self.deserialize_bool(visitor),
            b'"' => self.deserialize_string(visitor),
            b'[' => self.deserialize_seq(visitor),
            b'{' => self.deserialize_map(visitor),
            b'-' | b'0'..=b'9' => {
                let raw = self.parse_number_slice()?;
                if raw.contains(['.', 'e', 'E']) {
                    let parsed = raw
                        .parse::<f64>()
                        .map_err(|err| JsonError::new(format!("invalid float `{raw}`: {err}")))?;
                    if !parsed.is_finite() {
                        return Err(JsonError::new("non-finite float in JSON"));
                    }
                    visitor.visit_f64(parsed)
                } else if raw.starts_with('-') {
                    let parsed = raw
                        .parse::<i64>()
                        .map_err(|err| JsonError::new(format!("invalid int `{raw}`: {err}")))?;
                    visitor.visit_i64(parsed)
                } else {
                    let parsed = raw
                        .parse::<u64>()
                        .map_err(|err| JsonError::new(format!("invalid uint `{raw}`: {err}")))?;
                    visitor.visit_u64(parsed)
                }
            }
            other => Err(JsonError::new(format!(
                "unexpected token `{}` at byte offset {}",
                char::from(other),
                self.pos
            ))),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<i8>()
            .map_err(|err| JsonError::new(format!("invalid i8 `{raw}`: {err}")))?;
        visitor.visit_i8(val)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<i16>()
            .map_err(|err| JsonError::new(format!("invalid i16 `{raw}`: {err}")))?;
        visitor.visit_i16(val)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<i32>()
            .map_err(|err| JsonError::new(format!("invalid i32 `{raw}`: {err}")))?;
        visitor.visit_i32(val)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<i64>()
            .map_err(|err| JsonError::new(format!("invalid i64 `{raw}`: {err}")))?;
        visitor.visit_i64(val)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<u8>()
            .map_err(|err| JsonError::new(format!("invalid u8 `{raw}`: {err}")))?;
        visitor.visit_u8(val)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<u16>()
            .map_err(|err| JsonError::new(format!("invalid u16 `{raw}`: {err}")))?;
        visitor.visit_u16(val)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<u32>()
            .map_err(|err| JsonError::new(format!("invalid u32 `{raw}`: {err}")))?;
        visitor.visit_u32(val)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<u64>()
            .map_err(|err| JsonError::new(format!("invalid u64 `{raw}`: {err}")))?;
        visitor.visit_u64(val)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<f32>()
            .map_err(|err| JsonError::new(format!("invalid f32 `{raw}`: {err}")))?;
        if !val.is_finite() {
            return Err(JsonError::new("non-finite f32 in JSON"));
        }
        visitor.visit_f32(val)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let raw = self.parse_number_slice()?;
        let val = raw
            .parse::<f64>()
            .map_err(|err| JsonError::new(format!("invalid f64 `{raw}`: {err}")))?;
        if !val.is_finite() {
            return Err(JsonError::new("non-finite f64 in JSON"));
        }
        visitor.visit_f64(val)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.parse_string()?;
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(JsonError::new("expected single-character JSON string")),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.peek_byte()? == b'n' {
            self.parse_literal("null")?;
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.parse_literal("null")?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.peek_byte()? != b'[' {
            return Err(JsonError::new(format!(
                "expected `[` at byte offset {}",
                self.pos
            )));
        }
        self.pos += 1;
        self.enter_container()?;
        let result = visitor.visit_seq(JsonSeqAccess {
            de: self,
            first: true,
        });
        self.exit_container();
        let value = result?;
        if self.peek_byte()? != b']' {
            return Err(JsonError::new(format!(
                "expected `]` at byte offset {}",
                self.pos
            )));
        }
        self.pos += 1;
        Ok(value)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.peek_byte()? != b'{' {
            return Err(JsonError::new(format!(
                "expected `{{` at byte offset {}",
                self.pos
            )));
        }
        self.pos += 1;
        self.enter_container()?;
        let result = visitor.visit_map(JsonMapAccess {
            de: self,
            first: true,
        });
        self.exit_container();
        let value = result?;
        if self.peek_byte()? != b'}' {
            return Err(JsonError::new(format!(
                "expected `}}` at byte offset {}",
                self.pos
            )));
        }
        self.pos += 1;
        Ok(value)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.peek_byte()? {
            b'"' => visitor.visit_enum(JsonUnitEnumAccess { de: self }),
            b'{' => {
                self.pos += 1;
                self.enter_container()?;
                let result = visitor.visit_enum(JsonMapEnumAccess { de: self });
                self.exit_container();
                let value = result?;
                if self.peek_byte()? != b'}' {
                    return Err(JsonError::new(format!(
                        "expected `}}` after enum variant at byte offset {}",
                        self.pos
                    )));
                }
                self.pos += 1;
                Ok(value)
            }
            _ => Err(JsonError::new(format!(
                "expected enum string or object at byte offset {}",
                self.pos
            ))),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct JsonSeqAccess<'a, 'de> {
    de: &'a mut JsonDeserializer<'de>,
    first: bool,
}

impl<'de> SeqAccess<'de> for JsonSeqAccess<'_, 'de> {
    type Error = JsonError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b']' {
            return Ok(None);
        }
        if !self.first {
            if self.de.peek_byte()? != b',' {
                return Err(JsonError::new(format!(
                    "expected `,` in array at byte offset {}",
                    self.de.pos
                )));
            }
            self.de.pos += 1;
            if self.de.peek_byte()? == b']' {
                return Err(JsonError::new("trailing comma in JSON array"));
            }
        }
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct JsonMapAccess<'a, 'de> {
    de: &'a mut JsonDeserializer<'de>,
    first: bool,
}

impl<'de> MapAccess<'de> for JsonMapAccess<'_, 'de> {
    type Error = JsonError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'}' {
            return Ok(None);
        }
        if !self.first {
            if self.de.peek_byte()? != b',' {
                return Err(JsonError::new(format!(
                    "expected `,` in object at byte offset {}",
                    self.de.pos
                )));
            }
            self.de.pos += 1;
            if self.de.peek_byte()? == b'}' {
                return Err(JsonError::new("trailing comma in JSON object"));
            }
        }
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? != b':' {
            return Err(JsonError::new(format!(
                "expected `:` in object at byte offset {}",
                self.de.pos
            )));
        }
        self.de.pos += 1;
        seed.deserialize(&mut *self.de)
    }
}

struct JsonUnitEnumAccess<'a, 'de> {
    de: &'a mut JsonDeserializer<'de>,
}

impl<'de> EnumAccess<'de> for JsonUnitEnumAccess<'_, 'de> {
    type Error = JsonError;
    type Variant = JsonUnitVariantAccess;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = seed.deserialize(&mut *self.de)?;
        Ok((value, JsonUnitVariantAccess))
    }
}

struct JsonUnitVariantAccess;

impl<'de> VariantAccess<'de> for JsonUnitVariantAccess {
    type Error = JsonError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Err(JsonError::new("expected object for newtype enum variant"))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(JsonError::new("expected object for tuple enum variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(JsonError::new("expected object for struct enum variant"))
    }
}

struct JsonMapEnumAccess<'a, 'de> {
    de: &'a mut JsonDeserializer<'de>,
}

impl<'de> EnumAccess<'de> for JsonMapEnumAccess<'_, 'de> {
    type Error = JsonError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = seed.deserialize(&mut *self.de)?;
        if self.de.peek_byte()? != b':' {
            return Err(JsonError::new(format!(
                "expected `:` after enum variant key at byte offset {}",
                self.de.pos
            )));
        }
        self.de.pos += 1;
        Ok((value, self))
    }
}

impl<'de> VariantAccess<'de> for JsonMapEnumAccess<'_, 'de> {
    type Error = JsonError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        de::Deserialize::deserialize(&mut *self.de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(&mut *self.de, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(&mut *self.de, visitor)
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-CRASH-RESUME (T1) — Setlist cue-list footswitch state machine
    //! and self-healing JSON config quarantine recovery.

    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct SampleSetlistConfig {
        band_name: String,
        cues: Vec<SetlistCue>,
    }

    impl Default for SampleSetlistConfig {
        fn default() -> Self {
            Self {
                band_name: "Default Band".to_owned(),
                cues: Vec::new(),
            }
        }
    }

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(label: &str) -> io::Result<Self> {
            let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let pid = std::process::id();
            let path =
                std::env::temp_dir().join(format!("chromaticity_cue_runner_{label}_{pid}_{seq}"));
            fs::create_dir_all(&path)?;
            Ok(Self { path })
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_cues() -> Vec<SetlistCue> {
        vec![
            SetlistCue {
                id: "cue-1".to_owned(),
                song_title: "Intro Drone".to_owned(),
                scene_id: "scene_nebula".to_owned(),
                preset_name: Some("Warm Amber".to_owned()),
                bpm_mode: CueBpmMode::Off,
                input_gain_db: -3.0,
                crossfade_ms: 0,
            },
            SetlistCue {
                id: "cue-2".to_owned(),
                song_title: "Drive Anthem".to_owned(),
                scene_id: "scene_strobe".to_owned(),
                preset_name: None,
                bpm_mode: CueBpmMode::Auto,
                input_gain_db: 2.5,
                crossfade_ms: 1200,
            },
            SetlistCue {
                id: "cue-3".to_owned(),
                song_title: "Encore Pulse".to_owned(),
                scene_id: "scene_tunnel".to_owned(),
                preset_name: Some("Cyan Finale".to_owned()),
                bpm_mode: CueBpmMode::Manual(128.0),
                input_gain_db: 0.0,
                crossfade_ms: 500,
            },
        ]
    }

    #[test]
    fn empty_setlist_stays_empty_on_all_actions() {
        let mut runner = SetlistCueRunner::new(Vec::new());
        assert_eq!(runner.state(), CueRunnerState::Empty);
        assert!(runner.active_cue().is_none());
        assert!(!runner.is_blackout());

        for action in [
            FootswitchAction::NextCue,
            FootswitchAction::PrevCue,
            FootswitchAction::ToggleBlackout,
            FootswitchAction::JumpTo(0),
        ] {
            let event = runner.dispatch(action);
            assert_eq!(
                event,
                CueTransitionEvent::NoOp {
                    state: CueRunnerState::Empty
                }
            );
        }
        assert_eq!(runner.tick(500), CueRunnerState::Empty);
    }

    #[test]
    fn footswitch_progression_bounds_and_smooth_crossfade_ticks() {
        let mut runner = SetlistCueRunner::new(sample_cues());
        assert_eq!(
            runner.state(),
            CueRunnerState::Ready {
                active_index: 0,
                blackout: false,
            }
        );
        assert_eq!(
            runner.active_cue().map(|c| c.song_title.as_str()),
            Some("Intro Drone")
        );

        // PrevCue at cue 0 is clamped (NoOp).
        assert_eq!(
            runner.dispatch(FootswitchAction::PrevCue),
            CueTransitionEvent::NoOp {
                state: CueRunnerState::Ready {
                    active_index: 0,
                    blackout: false,
                }
            }
        );

        // NextCue enters cue 1 with 1200 ms crossfade.
        assert_eq!(
            runner.dispatch(FootswitchAction::NextCue),
            CueTransitionEvent::CrossfadeStarted {
                from_index: 0,
                to_index: 1,
                duration_ms: 1200,
                blackout: false,
            }
        );
        assert_eq!(
            runner.state(),
            CueRunnerState::Transitioning {
                from_index: 0,
                to_index: 1,
                remaining_ms: 1200,
                blackout: false,
            }
        );
        assert_eq!(runner.active_cue().map(|c| c.id.as_str()), Some("cue-2"));

        // Partial ticks count down smoothly.
        assert_eq!(
            runner.tick(450),
            CueRunnerState::Transitioning {
                from_index: 0,
                to_index: 1,
                remaining_ms: 750,
                blackout: false,
            }
        );

        // Final tick completes Transitioning -> Ready.
        assert_eq!(
            runner.tick(800),
            CueRunnerState::Ready {
                active_index: 1,
                blackout: false,
            }
        );

        // Advance to final cue (index 2).
        assert_eq!(
            runner.dispatch(FootswitchAction::NextCue),
            CueTransitionEvent::CrossfadeStarted {
                from_index: 1,
                to_index: 2,
                duration_ms: 500,
                blackout: false,
            }
        );
        assert_eq!(
            runner.tick(500),
            CueRunnerState::Ready {
                active_index: 2,
                blackout: false,
            }
        );

        // NextCue at last cue is clamped (NoOp).
        assert_eq!(
            runner.dispatch(FootswitchAction::NextCue),
            CueTransitionEvent::NoOp {
                state: CueRunnerState::Ready {
                    active_index: 2,
                    blackout: false,
                }
            }
        );

        // JumpTo(0) has crossfade_ms == 0 -> immediate switch.
        assert_eq!(
            runner.dispatch(FootswitchAction::JumpTo(0)),
            CueTransitionEvent::SwitchedImmediate {
                from_index: 2,
                to_index: 0,
                blackout: false,
            }
        );
        assert_eq!(
            runner.state(),
            CueRunnerState::Ready {
                active_index: 0,
                blackout: false,
            }
        );

        // Out-of-bounds JumpTo is a NoOp.
        assert_eq!(
            runner.dispatch(FootswitchAction::JumpTo(99)),
            CueTransitionEvent::NoOp {
                state: CueRunnerState::Ready {
                    active_index: 0,
                    blackout: false,
                }
            }
        );
    }

    #[test]
    fn toggle_blackout_preserves_active_cue_and_transition() {
        let mut runner = SetlistCueRunner::new(sample_cues());
        let _ = runner.dispatch(FootswitchAction::NextCue);
        assert_eq!(
            runner.state(),
            CueRunnerState::Transitioning {
                from_index: 0,
                to_index: 1,
                remaining_ms: 1200,
                blackout: false,
            }
        );

        // Engage blackout mid-crossfade: preserves indices & remaining_ms.
        assert_eq!(
            runner.dispatch(FootswitchAction::ToggleBlackout),
            CueTransitionEvent::BlackoutToggled {
                blackout: true,
                active_index: 1,
            }
        );
        assert!(runner.is_blackout());
        assert_eq!(
            runner.state(),
            CueRunnerState::Transitioning {
                from_index: 0,
                to_index: 1,
                remaining_ms: 1200,
                blackout: true,
            }
        );

        // Completing crossfade preserves blackout.
        assert_eq!(
            runner.tick(1200),
            CueRunnerState::Ready {
                active_index: 1,
                blackout: true,
            }
        );
        assert!(runner.is_blackout());
        assert_eq!(runner.active_cue().map(|c| c.id.as_str()), Some("cue-2"));

        // Disengaging blackout preserves active cue index 1.
        assert_eq!(
            runner.dispatch(FootswitchAction::ToggleBlackout),
            CueTransitionEvent::BlackoutToggled {
                blackout: false,
                active_index: 1,
            }
        );
        assert!(!runner.is_blackout());
        assert_eq!(
            runner.state(),
            CueRunnerState::Ready {
                active_index: 1,
                blackout: false,
            }
        );
    }

    #[test]
    fn load_or_self_heal_json_handles_valid_missing_and_corrupted_files() {
        let Ok(tmp) = TempDirGuard::new("self_heal") else {
            panic!("failed to create temp test directory");
        };

        // 1. Missing file -> CreatedDefault.
        let missing_path = tmp.path.join("missing-setlist.json");
        let missing_outcome =
            load_or_self_heal_json::<SampleSetlistConfig>(&missing_path, 1700000001);
        match missing_outcome {
            Ok(ConfigLoadOutcome::CreatedDefault(cfg)) => {
                assert_eq!(cfg, SampleSetlistConfig::default());
            }
            other => panic!("expected CreatedDefault, got {other:?}"),
        }

        // 2. Valid JSON -> Loaded with full fidelity (including enum variants).
        let valid_path = tmp.path.join("setlist.json");
        let valid_json = r#"{
            "band_name": "The Analogues",
            "cues": [
                {
                    "id": "cue-1",
                    "song_title": "Opener",
                    "scene_id": "scene_a",
                    "preset_name": "Amber",
                    "bpm_mode": "Off",
                    "input_gain_db": -1.5,
                    "crossfade_ms": 800
                },
                {
                    "id": "cue-2",
                    "song_title": "Closer",
                    "scene_id": "scene_b",
                    "preset_name": null,
                    "bpm_mode": { "Manual": 132.5 },
                    "input_gain_db": 3.0,
                    "crossfade_ms": 0
                }
            ]
        }"#;
        if let Err(err) = fs::write(&valid_path, valid_json) {
            panic!("failed to write valid json fixture: {err}");
        }

        let loaded_outcome = load_or_self_heal_json::<SampleSetlistConfig>(&valid_path, 1700000002);
        match loaded_outcome {
            Ok(ConfigLoadOutcome::Loaded(cfg)) => {
                assert_eq!(cfg.band_name, "The Analogues");
                assert_eq!(cfg.cues.len(), 2);
                assert_eq!(cfg.cues[0].bpm_mode, CueBpmMode::Off);
                assert_eq!(cfg.cues[1].bpm_mode, CueBpmMode::Manual(132.5));
                assert_eq!(cfg.cues[1].preset_name, None);
            }
            other => panic!("expected Loaded, got {other:?}"),
        }

        // 3. Truncated / corrupted JSON -> quarantined to .corrupt-<epoch>.bak & defaults returned.
        let corrupt_path = tmp.path.join("broken-setlist.json");
        let corrupt_payload = r#"{"band_name": "Mid-write crash", "cues": [{"id": "c1""#;
        if let Err(err) = fs::write(&corrupt_path, corrupt_payload) {
            panic!("failed to write corrupt json fixture: {err}");
        }

        let epoch = 1_765_432_100_u64;
        let recovered = load_or_self_heal_json::<SampleSetlistConfig>(&corrupt_path, epoch);
        match recovered {
            Ok(ConfigLoadOutcome::RecoveredFromCorruption {
                value,
                quarantine_path,
                reason,
            }) => {
                assert_eq!(value, SampleSetlistConfig::default());
                assert!(!reason.is_empty());
                assert_eq!(
                    quarantine_path,
                    tmp.path.join("broken-setlist.json.corrupt-1765432100.bak")
                );
                assert!(
                    !corrupt_path.exists(),
                    "original corrupt path should be moved"
                );
                assert!(quarantine_path.exists(), "quarantine backup must exist");
                let preserved = fs::read_to_string(&quarantine_path);
                match preserved {
                    Ok(text) => assert_eq!(text, corrupt_payload),
                    Err(err) => panic!("failed to read quarantine backup: {err}"),
                }
            }
            other => panic!("expected RecoveredFromCorruption, got {other:?}"),
        }

        // 4. Invalid UTF-8 bytes -> quarantined & defaults returned without panicking.
        let bad_utf8_path = tmp.path.join("binary-garbage.json");
        let raw_bytes: [u8; 4] = [0xFF, 0xFE, 0x00, 0x80];
        if let Err(err) = fs::write(&bad_utf8_path, raw_bytes) {
            panic!("failed to write invalid UTF-8 fixture: {err}");
        }

        let utf8_recovered =
            load_or_self_heal_json::<SampleSetlistConfig>(&bad_utf8_path, 1_765_432_200);
        match utf8_recovered {
            Ok(ConfigLoadOutcome::RecoveredFromCorruption {
                value,
                quarantine_path,
                reason,
            }) => {
                assert_eq!(value, SampleSetlistConfig::default());
                assert!(reason.contains("UTF-8"));
                assert_eq!(
                    quarantine_path,
                    tmp.path.join("binary-garbage.json.corrupt-1765432200.bak")
                );
                assert!(quarantine_path.exists());
            }
            other => panic!("expected RecoveredFromCorruption for bad UTF-8, got {other:?}"),
        }
    }
}
