// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! ADR-017 scene-declarative external data ingestion & deterministic
//! cassette replay substrate.
//!
//! Provides the foundational Phase-6 differentiator data plane inside
//! `chromaticity-runtime`:
//!
//! - Scene-declared [`DataSourceSpec`] descriptors enforcing the
//!   Chromaticity-native `chroma_Data_*` uniform namespace
//!   ([`DATA_UNIFORM_PREFIX`]).
//! - Explicit ADR-016 opt-in gating: every data source must declare
//!   `opt_in: true` before samples are accepted or replayed into the
//!   uniform map.
//! - Deterministic [`DataCassette`] recording, canonical JSON
//!   serialization/deserialization, and frame-indexed replay producing
//!   a deterministic `BTreeMap<String, DataValue>` keyed by
//!   `chroma_Data_*` uniform name.
//! - Zero-network local JSON file polling via
//!   [`DataIngestEngine::poll_local_json_file`].
//! - Explicit [`DataIngestState`] state machine with no implicit
//!   background threads or hidden global state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

/// Required prefix for all external data uniform names in Chromaticity
/// shaders and scene declarations.
pub const DATA_UNIFORM_PREFIX: &str = "chroma_Data_";

/// Current canonical [`DataCassette`] JSON format version.
pub const CASSETTE_FORMAT_VERSION: u32 = 1;

/// Maximum JSON nesting depth accepted by the built-in RFC 8259 parser.
const MAX_JSON_DEPTH: usize = 64;

/// Typed payload value bound to a `chroma_Data_*` shader uniform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataValue {
    /// Single 32-bit floating-point scalar uniform.
    Scalar(f32),
    /// Three-component vector uniform.
    Vec3([f32; 3]),
    /// Variable-length floating-point array uniform.
    Array(Vec<f32>),
}

impl DataValue {
    /// Returns `true` if every floating-point component in this value
    /// is finite (neither `NaN` nor `±inf`).
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Scalar(v) => v.is_finite(),
            Self::Vec3(v) => v.iter().all(|c| c.is_finite()),
            Self::Array(vs) => vs.iter().all(|c| c.is_finite()),
        }
    }
}

/// Transport mechanism declared by a [`DataSourceSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataTransport {
    /// Deterministic recorded cassette playback.
    Cassette,
    /// Local filesystem JSON polling (zero network I/O).
    LocalFile,
    /// Periodic HTTP polling transport (future live adapter).
    HttpPoll,
    /// Streaming WebSocket transport (future live adapter).
    WebSocket,
}

/// Scene-declarative specification for an external data source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSourceSpec {
    /// Unique identifier for this source within a scene.
    pub id: String,
    /// Target shader uniform name; must start with `chroma_Data_` and
    /// consist solely of ASCII identifier characters (`[A-Za-z0-9_]`).
    pub uniform_name: String,
    /// Declared transport for this data source.
    pub transport: DataTransport,
    /// ADR-016 explicit opt-in flag. Sources with `opt_in == false`
    /// cannot publish or replay samples.
    pub opt_in: bool,
}

impl DataSourceSpec {
    /// Validates `id` and `uniform_name` against Chromaticity naming
    /// invariants.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::EmptySourceId`] if `id` is empty or
    /// whitespace-only, or [`DataIngestError::InvalidUniformName`] if
    /// `uniform_name` does not start with `chroma_Data_`, has an empty
    /// suffix, or contains non-ASCII-identifier characters.
    pub fn validate(&self) -> Result<(), DataIngestError> {
        if self.id.trim().is_empty() {
            return Err(DataIngestError::EmptySourceId);
        }

        let Some(suffix) = self.uniform_name.strip_prefix(DATA_UNIFORM_PREFIX) else {
            return Err(DataIngestError::InvalidUniformName {
                name: self.uniform_name.clone(),
            });
        };

        if suffix.is_empty()
            || !suffix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(DataIngestError::InvalidUniformName {
                name: self.uniform_name.clone(),
            });
        }

        Ok(())
    }
}

/// A single frame-stamped sample recorded in a [`DataCassette`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CassetteSample {
    /// Canonical runtime frame index at which this sample applies.
    pub frame: u64,
    /// Source identifier matching a registered [`DataSourceSpec::id`].
    pub source_id: String,
    /// Sample payload value.
    pub value: DataValue,
}

/// Deterministic, versioned cassette of frame-stamped external data
/// samples for offline replay and regression testing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataCassette {
    /// Cassette schema version (currently [`CASSETTE_FORMAT_VERSION`]).
    pub version: u32,
    /// Ordered list of recorded samples.
    pub samples: Vec<CassetteSample>,
}

impl DataCassette {
    /// Creates a new [`DataCassette`] tagged with
    /// [`CASSETTE_FORMAT_VERSION`].
    #[must_use]
    pub fn new(samples: Vec<CassetteSample>) -> Self {
        Self {
            version: CASSETTE_FORMAT_VERSION,
            samples,
        }
    }

    /// Returns all samples stamped for `frame` in deterministic slice
    /// order.
    #[must_use]
    pub fn samples_for_frame(&self, frame: u64) -> Vec<&CassetteSample> {
        self.samples.iter().filter(|s| s.frame == frame).collect()
    }

    /// Serializes this cassette to deterministic canonical JSON
    /// (RFC 8259) compatible with standard `serde` JSON schemas.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::NonFiniteValue`] if any sample
    /// contains a `NaN` or infinite `f32`.
    pub fn to_json(&self) -> Result<String, DataIngestError> {
        let mut out = String::with_capacity(64 + self.samples.len() * 64);
        out.push_str("{\"version\":");
        let _ = write!(out, "{}", self.version);
        out.push_str(",\"samples\":[");
        for (idx, sample) in self.samples.iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push_str("{\"frame\":");
            let _ = write!(out, "{}", sample.frame);
            out.push_str(",\"source_id\":");
            write_json_string(&mut out, &sample.source_id);
            out.push_str(",\"value\":");
            write_data_value_json(&mut out, &sample.source_id, &sample.value)?;
            out.push('}');
        }
        out.push_str("]}");
        Ok(out)
    }

    /// Deserializes a [`DataCassette`] from canonical JSON.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::JsonError`] on malformed JSON or
    /// schema mismatches, [`DataIngestError::UnsupportedCassetteVersion`]
    /// for unknown format versions, or
    /// [`DataIngestError::NonFiniteValue`] if a decoded float overflows
    /// `f32`.
    pub fn from_json(input: &str) -> Result<Self, DataIngestError> {
        let root = parse_json(input)?;
        let JsonValue::Object(mut obj) = root else {
            return Err(DataIngestError::JsonError {
                message: "cassette root must be a JSON object".to_owned(),
            });
        };

        let version = match obj.remove("version") {
            Some(JsonValue::Number(n)) => parse_json_u32(n, "version")?,
            _ => {
                return Err(DataIngestError::JsonError {
                    message: "missing or invalid integer field 'version'".to_owned(),
                });
            }
        };

        if version != CASSETTE_FORMAT_VERSION {
            return Err(DataIngestError::UnsupportedCassetteVersion { version });
        }

        let Some(JsonValue::Array(raw_samples)) = obj.remove("samples") else {
            return Err(DataIngestError::JsonError {
                message: "missing or invalid array field 'samples'".to_owned(),
            });
        };

        let mut samples = Vec::with_capacity(raw_samples.len());
        for raw_sample in raw_samples {
            let JsonValue::Object(mut sample_obj) = raw_sample else {
                return Err(DataIngestError::JsonError {
                    message: "cassette sample must be a JSON object".to_owned(),
                });
            };

            let frame = match sample_obj.remove("frame") {
                Some(JsonValue::Number(n)) => parse_json_u64(n, "frame")?,
                _ => {
                    return Err(DataIngestError::JsonError {
                        message: "missing or invalid integer field 'frame'".to_owned(),
                    });
                }
            };

            let Some(JsonValue::String(source_id)) = sample_obj.remove("source_id") else {
                return Err(DataIngestError::JsonError {
                    message: "missing or invalid string field 'source_id'".to_owned(),
                });
            };

            let Some(raw_value) = sample_obj.remove("value") else {
                return Err(DataIngestError::JsonError {
                    message: "missing field 'value'".to_owned(),
                });
            };

            let value = decode_data_value_json(raw_value, &source_id)?;
            samples.push(CassetteSample {
                frame,
                source_id,
                value,
            });
        }

        Ok(Self { version, samples })
    }
}

/// Explicit lifecycle state machine for [`DataIngestEngine`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DataIngestState {
    /// Ingestion is idle; no cassette replay, recording, or active file
    /// poll is running.
    #[default]
    Disabled,
    /// Deterministic cassette replay is active.
    Replaying {
        /// Total number of samples in the loaded cassette.
        cassette_len: usize,
        /// Most recent frame advanced by the engine.
        cursor_frame: u64,
    },
    /// Recording published samples into an in-memory cassette buffer.
    Recording {
        /// Count of samples recorded so far.
        recorded_samples: usize,
    },
    /// Local JSON file polling has most recently updated the engine.
    LocalFileActive {
        /// Path of the most recently polled local JSON file.
        path: PathBuf,
    },
}

impl DataIngestState {
    #[must_use]
    fn label(&self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Replaying { .. } => "Replaying",
            Self::Recording { .. } => "Recording",
            Self::LocalFileActive { .. } => "LocalFileActive",
        }
    }
}

/// Errors returned by [`DataSourceSpec::validate`], [`DataCassette`],
/// and [`DataIngestEngine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataIngestError {
    /// A [`DataSourceSpec`] was registered with an empty `id`.
    EmptySourceId,
    /// A [`DataSourceSpec`] uniform name violated the `chroma_Data_*`
    /// ASCII identifier rule.
    InvalidUniformName {
        /// The rejected uniform name.
        name: String,
    },
    /// Attempted to register a duplicate `source.id`.
    DuplicateSourceId {
        /// The conflicting source id.
        id: String,
    },
    /// Attempted to register a second source targeting the same
    /// `chroma_Data_*` uniform name.
    DuplicateUniformName {
        /// The conflicting uniform name.
        uniform_name: String,
    },
    /// Sample or poll referenced an unregistered source id.
    UnknownSource {
        /// The unknown source id.
        id: String,
    },
    /// ADR-016 opt-in gate rejected a source with `opt_in == false`.
    SourceNotOptedIn {
        /// The non-opted-in source id.
        id: String,
    },
    /// A sample contained a `NaN` or infinite `f32` component.
    NonFiniteValue {
        /// The source id that produced the non-finite value.
        source_id: String,
    },
    /// Operation is invalid in the current [`DataIngestState`].
    InvalidStateTransition {
        /// Name of the current state.
        from: &'static str,
        /// Name of the attempted action.
        action: &'static str,
    },
    /// Cassette JSON carried an unsupported format version.
    UnsupportedCassetteVersion {
        /// The unsupported version number.
        version: u32,
    },
    /// JSON parsing or schema decoding failed.
    JsonError {
        /// Human-readable parse/schema diagnostic.
        message: String,
    },
    /// Local file I/O failed.
    IoError {
        /// Path that failed to read.
        path: PathBuf,
        /// OS error description.
        message: String,
    },
}

impl fmt::Display for DataIngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySourceId => write!(f, "data source id must not be empty"),
            Self::InvalidUniformName { name } => write!(
                f,
                "invalid data uniform name '{name}': must start with '{DATA_UNIFORM_PREFIX}' followed by ASCII identifier characters"
            ),
            Self::DuplicateSourceId { id } => {
                write!(f, "duplicate data source id '{id}'")
            }
            Self::DuplicateUniformName { uniform_name } => {
                write!(f, "duplicate data uniform name '{uniform_name}'")
            }
            Self::UnknownSource { id } => write!(f, "unknown data source '{id}'"),
            Self::SourceNotOptedIn { id } => {
                write!(f, "data source '{id}' is not opted in (ADR-016)")
            }
            Self::NonFiniteValue { source_id } => {
                write!(f, "non-finite float value for data source '{source_id}'")
            }
            Self::InvalidStateTransition { from, action } => {
                write!(f, "invalid data ingest action '{action}' in state '{from}'")
            }
            Self::UnsupportedCassetteVersion { version } => {
                write!(
                    f,
                    "unsupported cassette format version {version} (expected {CASSETTE_FORMAT_VERSION})"
                )
            }
            Self::JsonError { message } => write!(f, "data ingest JSON error: {message}"),
            Self::IoError { path, message } => {
                write!(
                    f,
                    "data ingest I/O error for '{}': {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DataIngestError {}

/// Stateful ADR-017 external data ingestion and deterministic cassette
/// replay engine.
#[derive(Debug, Clone, Default)]
pub struct DataIngestEngine {
    sources: BTreeMap<String, DataSourceSpec>,
    uniform_to_source: BTreeMap<String, String>,
    state: DataIngestState,
    active_cassette: Option<DataCassette>,
    recording_buffer: Vec<CassetteSample>,
    latest_values: BTreeMap<String, DataValue>,
}

impl DataIngestEngine {
    /// Constructs a new [`DataIngestEngine`] in
    /// [`DataIngestState::Disabled`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current [`DataIngestState`].
    #[must_use]
    pub fn state(&self) -> &DataIngestState {
        &self.state
    }

    /// Registers a scene-declared [`DataSourceSpec`].
    ///
    /// Validates `spec` and rejects duplicate source IDs or duplicate
    /// uniform names.
    ///
    /// # Errors
    ///
    /// Returns validation errors from [`DataSourceSpec::validate`],
    /// [`DataIngestError::DuplicateSourceId`], or
    /// [`DataIngestError::DuplicateUniformName`].
    pub fn register_source(&mut self, spec: DataSourceSpec) -> Result<(), DataIngestError> {
        spec.validate()?;

        if self.sources.contains_key(&spec.id) {
            return Err(DataIngestError::DuplicateSourceId { id: spec.id });
        }

        if self.uniform_to_source.contains_key(&spec.uniform_name) {
            return Err(DataIngestError::DuplicateUniformName {
                uniform_name: spec.uniform_name,
            });
        }

        self.uniform_to_source
            .insert(spec.uniform_name.clone(), spec.id.clone());
        self.sources.insert(spec.id.clone(), spec);
        Ok(())
    }

    /// Looks up a registered [`DataSourceSpec`] by `id`.
    #[must_use]
    pub fn source(&self, id: &str) -> Option<&DataSourceSpec> {
        self.sources.get(id)
    }

    /// Returns all registered [`DataSourceSpec`] entries keyed by
    /// source id in deterministic order.
    #[must_use]
    pub fn sources(&self) -> &BTreeMap<String, DataSourceSpec> {
        &self.sources
    }

    /// Returns the current uniform snapshot keyed by `chroma_Data_*`
    /// uniform name.
    #[must_use]
    pub fn current_uniforms(&self) -> &BTreeMap<String, DataValue> {
        &self.latest_values
    }

    /// Publishes a sample for `source_id` at `frame`.
    ///
    /// Enforces the ADR-016 opt-in posture: unknown sources and sources
    /// with `opt_in == false` are rejected. When the engine is in
    /// [`DataIngestState::Recording`], the sample is appended to the
    /// recording buffer.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::UnknownSource`],
    /// [`DataIngestError::SourceNotOptedIn`], or
    /// [`DataIngestError::NonFiniteValue`].
    pub fn publish_sample(
        &mut self,
        frame: u64,
        source_id: &str,
        value: DataValue,
    ) -> Result<(), DataIngestError> {
        let uniform_name = self.require_opted_in_uniform(source_id)?;

        if !value.is_finite() {
            return Err(DataIngestError::NonFiniteValue {
                source_id: source_id.to_owned(),
            });
        }

        self.latest_values.insert(uniform_name, value.clone());

        if let DataIngestState::Recording { recorded_samples } = &mut self.state {
            self.recording_buffer.push(CassetteSample {
                frame,
                source_id: source_id.to_owned(),
                value,
            });
            *recorded_samples = self.recording_buffer.len();
        }

        Ok(())
    }

    /// Transitions the engine into [`DataIngestState::Recording`] with
    /// an empty sample buffer.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::InvalidStateTransition`] if recording
    /// is already in progress.
    pub fn begin_recording(&mut self) -> Result<(), DataIngestError> {
        if matches!(self.state, DataIngestState::Recording { .. }) {
            return Err(DataIngestError::InvalidStateTransition {
                from: self.state.label(),
                action: "begin_recording",
            });
        }

        self.active_cassette = None;
        self.recording_buffer.clear();
        self.state = DataIngestState::Recording {
            recorded_samples: 0,
        };
        Ok(())
    }

    /// Completes recording, returning a deterministically frame-ordered
    /// [`DataCassette`] and transitioning back to
    /// [`DataIngestState::Disabled`].
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::InvalidStateTransition`] if the
    /// engine is not currently in [`DataIngestState::Recording`].
    pub fn finish_recording(&mut self) -> Result<DataCassette, DataIngestError> {
        if !matches!(self.state, DataIngestState::Recording { .. }) {
            return Err(DataIngestError::InvalidStateTransition {
                from: self.state.label(),
                action: "finish_recording",
            });
        }

        let mut samples = std::mem::take(&mut self.recording_buffer);
        // Stable sort by frame preserves within-frame publish order
        // while guaranteeing monotonic frame ordering in the cassette.
        samples.sort_by_key(|sample| sample.frame);
        self.state = DataIngestState::Disabled;
        Ok(DataCassette::new(samples))
    }

    /// Loads a [`DataCassette`] for deterministic replay and
    /// transitions to [`DataIngestState::Replaying`].
    ///
    /// Clears any previously latched uniform values so replay begins
    /// from a clean, reproducible initial state.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::UnsupportedCassetteVersion`] or
    /// [`DataIngestError::NonFiniteValue`].
    pub fn load_cassette(&mut self, cassette: DataCassette) -> Result<(), DataIngestError> {
        if cassette.version != CASSETTE_FORMAT_VERSION {
            return Err(DataIngestError::UnsupportedCassetteVersion {
                version: cassette.version,
            });
        }

        for sample in &cassette.samples {
            if !sample.value.is_finite() {
                return Err(DataIngestError::NonFiniteValue {
                    source_id: sample.source_id.clone(),
                });
            }
        }

        let cassette_len = cassette.samples.len();
        self.recording_buffer.clear();
        self.latest_values.clear();
        self.active_cassette = Some(cassette);
        self.state = DataIngestState::Replaying {
            cassette_len,
            cursor_frame: 0,
        };
        Ok(())
    }

    /// Advances cassette replay to `frame`, applying all cassette
    /// samples stamped for `frame` and returning the deterministic
    /// uniform snapshot keyed by `chroma_Data_*` uniform name.
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::InvalidStateTransition`] if no
    /// cassette is loaded, or [`DataIngestError::UnknownSource`] /
    /// [`DataIngestError::SourceNotOptedIn`] if a cassette sample
    /// targets an unregistered or non-opted-in source.
    pub fn advance_frame(
        &mut self,
        frame: u64,
    ) -> Result<BTreeMap<String, DataValue>, DataIngestError> {
        let Some(cassette) = &self.active_cassette else {
            return Err(DataIngestError::InvalidStateTransition {
                from: self.state.label(),
                action: "advance_frame",
            });
        };

        let cassette_len = cassette.samples.len();
        let frame_samples: Vec<(String, DataValue)> = cassette
            .samples_for_frame(frame)
            .into_iter()
            .map(|sample| (sample.source_id.clone(), sample.value.clone()))
            .collect();

        for (source_id, value) in frame_samples {
            let uniform_name = self.require_opted_in_uniform(&source_id)?;
            if !value.is_finite() {
                return Err(DataIngestError::NonFiniteValue { source_id });
            }
            self.latest_values.insert(uniform_name, value);
        }

        self.state = DataIngestState::Replaying {
            cassette_len,
            cursor_frame: frame,
        };

        Ok(self.latest_values.clone())
    }

    /// Polls a local JSON file at `path` (zero network I/O), decodes a
    /// numeric scalar, `{ "key": number }` object, or JSON array of
    /// numbers into a [`DataValue`], and publishes it for `source_id`
    /// at `frame`.
    ///
    /// Decoding rules:
    /// - JSON number -> [`DataValue::Scalar`]
    /// - Single-entry JSON object `{ "key": number }` ->
    ///   [`DataValue::Scalar`]
    /// - Multi-entry JSON object `{ "a": n1, "b": n2, ... }` ->
    ///   [`DataValue::Array`] ordered deterministically by key
    /// - JSON array of numbers `[n0, n1, ...]` -> [`DataValue::Array`]
    ///
    /// Unless the engine is currently in
    /// [`DataIngestState::Recording`], transitions state to
    /// [`DataIngestState::LocalFileActive`].
    ///
    /// # Errors
    ///
    /// Returns [`DataIngestError::UnknownSource`],
    /// [`DataIngestError::SourceNotOptedIn`],
    /// [`DataIngestError::IoError`], [`DataIngestError::JsonError`],
    /// or [`DataIngestError::NonFiniteValue`].
    pub fn poll_local_json_file(
        &mut self,
        frame: u64,
        source_id: &str,
        path: &Path,
    ) -> Result<DataValue, DataIngestError> {
        let _ = self.require_opted_in_uniform(source_id)?;

        let content = std::fs::read_to_string(path).map_err(|err| DataIngestError::IoError {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;

        let value = decode_local_json_payload(&content, source_id)?;
        self.publish_sample(frame, source_id, value.clone())?;

        if !matches!(self.state, DataIngestState::Recording { .. }) {
            self.active_cassette = None;
            self.state = DataIngestState::LocalFileActive {
                path: path.to_path_buf(),
            };
        }

        Ok(value)
    }

    fn require_opted_in_uniform(&self, source_id: &str) -> Result<String, DataIngestError> {
        let Some(spec) = self.sources.get(source_id) else {
            return Err(DataIngestError::UnknownSource {
                id: source_id.to_owned(),
            });
        };

        if !spec.opt_in {
            return Err(DataIngestError::SourceNotOptedIn {
                id: source_id.to_owned(),
            });
        }

        Ok(spec.uniform_name.clone())
    }
}

// ---------------------------------------------------------------------------
// Minimal deterministic RFC 8259 JSON codec (zero external dependencies)
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

fn write_json_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_json_f32(out: &mut String, source_id: &str, v: f32) -> Result<(), DataIngestError> {
    if !v.is_finite() {
        return Err(DataIngestError::NonFiniteValue {
            source_id: source_id.to_owned(),
        });
    }
    let _ = write!(out, "{v}");
    Ok(())
}

fn write_data_value_json(
    out: &mut String,
    source_id: &str,
    value: &DataValue,
) -> Result<(), DataIngestError> {
    match value {
        DataValue::Scalar(v) => {
            out.push_str("{\"Scalar\":");
            write_json_f32(out, source_id, *v)?;
            out.push('}');
        }
        DataValue::Vec3(v) => {
            out.push_str("{\"Vec3\":[");
            for (idx, comp) in v.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_json_f32(out, source_id, *comp)?;
            }
            out.push_str("]}");
        }
        DataValue::Array(vs) => {
            out.push_str("{\"Array\":[");
            for (idx, comp) in vs.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_json_f32(out, source_id, *comp)?;
            }
            out.push_str("]}");
        }
    }
    Ok(())
}

fn decode_data_value_json(raw: JsonValue, source_id: &str) -> Result<DataValue, DataIngestError> {
    let JsonValue::Object(mut obj) = raw else {
        return Err(DataIngestError::JsonError {
            message: "DataValue must be a tagged JSON object".to_owned(),
        });
    };

    if obj.len() != 1 {
        return Err(DataIngestError::JsonError {
            message: "DataValue object must contain exactly one variant tag".to_owned(),
        });
    }

    if let Some(val) = obj.remove("Scalar") {
        let JsonValue::Number(n) = val else {
            return Err(DataIngestError::JsonError {
                message: "Scalar payload must be a JSON number".to_owned(),
            });
        };
        return Ok(DataValue::Scalar(narrow_f64_to_f32(n, source_id)?));
    }

    if let Some(val) = obj.remove("Vec3") {
        let JsonValue::Array(items) = val else {
            return Err(DataIngestError::JsonError {
                message: "Vec3 payload must be a JSON array".to_owned(),
            });
        };
        if items.len() != 3 {
            return Err(DataIngestError::JsonError {
                message: format!("Vec3 payload must have 3 elements, got {}", items.len()),
            });
        }
        let mut out = [0.0_f32; 3];
        for (slot, item) in out.iter_mut().zip(items.into_iter()) {
            let JsonValue::Number(n) = item else {
                return Err(DataIngestError::JsonError {
                    message: "Vec3 elements must be JSON numbers".to_owned(),
                });
            };
            *slot = narrow_f64_to_f32(n, source_id)?;
        }
        return Ok(DataValue::Vec3(out));
    }

    if let Some(val) = obj.remove("Array") {
        let JsonValue::Array(items) = val else {
            return Err(DataIngestError::JsonError {
                message: "Array payload must be a JSON array".to_owned(),
            });
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let JsonValue::Number(n) = item else {
                return Err(DataIngestError::JsonError {
                    message: "Array elements must be JSON numbers".to_owned(),
                });
            };
            out.push(narrow_f64_to_f32(n, source_id)?);
        }
        return Ok(DataValue::Array(out));
    }

    Err(DataIngestError::JsonError {
        message: "unknown DataValue variant tag".to_owned(),
    })
}

fn decode_local_json_payload(content: &str, source_id: &str) -> Result<DataValue, DataIngestError> {
    let root = parse_json(content)?;
    match root {
        JsonValue::Number(n) => Ok(DataValue::Scalar(narrow_f64_to_f32(n, source_id)?)),
        JsonValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let JsonValue::Number(n) = item else {
                    return Err(DataIngestError::JsonError {
                        message: "local JSON array must contain only numbers".to_owned(),
                    });
                };
                out.push(narrow_f64_to_f32(n, source_id)?);
            }
            Ok(DataValue::Array(out))
        }
        JsonValue::Object(map) => {
            if map.is_empty() {
                return Err(DataIngestError::JsonError {
                    message: "local JSON object must contain at least one numeric field".to_owned(),
                });
            }
            if map.len() == 1 {
                if let Some((_, JsonValue::Number(n))) = map.into_iter().next() {
                    return Ok(DataValue::Scalar(narrow_f64_to_f32(n, source_id)?));
                }
                return Err(DataIngestError::JsonError {
                    message: "local JSON object values must be numbers".to_owned(),
                });
            }
            let mut out = Vec::with_capacity(map.len());
            for (_, val) in map {
                let JsonValue::Number(n) = val else {
                    return Err(DataIngestError::JsonError {
                        message: "local JSON object values must be numbers".to_owned(),
                    });
                };
                out.push(narrow_f64_to_f32(n, source_id)?);
            }
            Ok(DataValue::Array(out))
        }
        _ => Err(DataIngestError::JsonError {
            message: "local JSON root must be a number, object of numbers, or array of numbers"
                .to_owned(),
        }),
    }
}

fn narrow_f64_to_f32(n: f64, source_id: &str) -> Result<f32, DataIngestError> {
    let narrowed = n as f32;
    if !narrowed.is_finite() {
        return Err(DataIngestError::NonFiniteValue {
            source_id: source_id.to_owned(),
        });
    }
    Ok(narrowed)
}

fn parse_json_u32(n: f64, field: &str) -> Result<u32, DataIngestError> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > f64::from(u32::MAX) {
        return Err(DataIngestError::JsonError {
            message: format!("field '{field}' must be an unsigned 32-bit integer"),
        });
    }
    Ok(n as u32)
}

fn parse_json_u64(n: f64, field: &str) -> Result<u64, DataIngestError> {
    // 2^53 is the maximum exact integer representable in f64.
    const MAX_EXACT_F64_INT: f64 = 9_007_199_254_740_992.0;
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > MAX_EXACT_F64_INT {
        return Err(DataIngestError::JsonError {
            message: format!("field '{field}' must be a non-negative integer"),
        });
    }
    Ok(n as u64)
}

fn parse_json(input: &str) -> Result<JsonValue, DataIngestError> {
    let mut parser = JsonParser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let val = parser.parse_value(0)?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(DataIngestError::JsonError {
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

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, DataIngestError> {
        if depth > MAX_JSON_DEPTH {
            return Err(DataIngestError::JsonError {
                message: "JSON nesting depth exceeded".to_owned(),
            });
        }
        self.skip_ws();
        let Some(b) = self.peek() else {
            return Err(DataIngestError::JsonError {
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
            other => Err(DataIngestError::JsonError {
                message: format!("unexpected byte 0x{other:02x} at position {}", self.pos),
            }),
        }
    }

    fn parse_literal(
        &mut self,
        expected: &[u8],
        val: JsonValue,
    ) -> Result<JsonValue, DataIngestError> {
        let end = self.pos + expected.len();
        if self.bytes.get(self.pos..end) == Some(expected) {
            self.pos = end;
            Ok(val)
        } else {
            Err(DataIngestError::JsonError {
                message: format!("invalid JSON literal at byte {}", self.pos),
            })
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, DataIngestError> {
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
                return Err(DataIngestError::JsonError {
                    message: format!("expected object key string at byte {}", self.pos),
                });
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(DataIngestError::JsonError {
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
                    return Err(DataIngestError::JsonError {
                        message: format!("expected ',' or '}}' in object at byte {}", self.pos),
                    });
                }
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, DataIngestError> {
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
                    return Err(DataIngestError::JsonError {
                        message: format!("expected ',' or ']' in array at byte {}", self.pos),
                    });
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, DataIngestError> {
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
                        return Err(DataIngestError::JsonError {
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
                                // High surrogate: require trailing \uXXXX low surrogate.
                                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                    return Err(DataIngestError::JsonError {
                                        message: "unpaired UTF-16 high surrogate".to_owned(),
                                    });
                                }
                                self.pos += 2;
                                let low = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(DataIngestError::JsonError {
                                        message: "invalid UTF-16 low surrogate".to_owned(),
                                    });
                                }
                                let combined = 0x10000
                                    + ((u32::from(code) - 0xD800) << 10)
                                    + (u32::from(low) - 0xDC00);
                                let Some(ch) = char::from_u32(combined) else {
                                    return Err(DataIngestError::JsonError {
                                        message: "invalid Unicode scalar value".to_owned(),
                                    });
                                };
                                out.push(ch);
                            } else {
                                let Some(ch) = char::from_u32(u32::from(code)) else {
                                    return Err(DataIngestError::JsonError {
                                        message: "invalid Unicode scalar escape".to_owned(),
                                    });
                                };
                                out.push(ch);
                            }
                        }
                        _ => {
                            return Err(DataIngestError::JsonError {
                                message: format!("invalid string escape at byte {}", self.pos),
                            });
                        }
                    }
                }
                0x00..=0x1F => {
                    return Err(DataIngestError::JsonError {
                        message: format!("unescaped control character at byte {}", self.pos),
                    });
                }
                _ => {
                    // Decode valid UTF-8 character slice from remaining bytes.
                    let rest = std::str::from_utf8(&self.bytes[self.pos..]).map_err(|_| {
                        DataIngestError::JsonError {
                            message: "invalid UTF-8 in JSON string".to_owned(),
                        }
                    })?;
                    let Some(ch) = rest.chars().next() else {
                        return Err(DataIngestError::JsonError {
                            message: "unexpected end of string".to_owned(),
                        });
                    };
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err(DataIngestError::JsonError {
            message: "unterminated JSON string".to_owned(),
        })
    }

    fn parse_hex4(&mut self) -> Result<u16, DataIngestError> {
        let Some(slice) = self.bytes.get(self.pos..self.pos + 4) else {
            return Err(DataIngestError::JsonError {
                message: "truncated \\uXXXX escape".to_owned(),
            });
        };
        let hex_str = std::str::from_utf8(slice).map_err(|_| DataIngestError::JsonError {
            message: "invalid UTF-8 in \\uXXXX escape".to_owned(),
        })?;
        let val = u16::from_str_radix(hex_str, 16).map_err(|_| DataIngestError::JsonError {
            message: format!("invalid hex digits '{hex_str}' in \\u escape"),
        })?;
        self.pos += 4;
        Ok(val)
    }

    fn parse_number(&mut self) -> Result<f64, DataIngestError> {
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
                return Err(DataIngestError::JsonError {
                    message: format!("invalid number at byte {start}"),
                });
            }
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(DataIngestError::JsonError {
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
                return Err(DataIngestError::JsonError {
                    message: format!("missing exponent digits in number at byte {start}"),
                });
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let num_str = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| {
            DataIngestError::JsonError {
                message: "invalid UTF-8 in number".to_owned(),
            }
        })?;
        let parsed = num_str
            .parse::<f64>()
            .map_err(|_| DataIngestError::JsonError {
                message: format!("failed to parse JSON number '{num_str}'"),
            })?;
        if !parsed.is_finite() {
            return Err(DataIngestError::JsonError {
                message: format!("JSON number '{num_str}' is out of finite f64 range"),
            });
        }
        Ok(parsed)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_spec(
        id: &str,
        uniform_name: &str,
        transport: DataTransport,
        opt_in: bool,
    ) -> DataSourceSpec {
        DataSourceSpec {
            id: id.to_owned(),
            uniform_name: uniform_name.to_owned(),
            transport,
            opt_in,
        }
    }

    fn temp_json_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "chromaticity_data_ingest_{tag}_{}_{nanos}.json",
            std::process::id()
        ))
    }

    fn assert_uniform_maps_bit_identical(
        a: &BTreeMap<String, DataValue>,
        b: &BTreeMap<String, DataValue>,
    ) {
        assert_eq!(a.len(), b.len());
        for ((ka, va), (kb, vb)) in a.iter().zip(b.iter()) {
            assert_eq!(ka, kb);
            match (va, vb) {
                (DataValue::Scalar(x), DataValue::Scalar(y)) => {
                    assert_eq!(x.to_bits(), y.to_bits(), "scalar bit mismatch for {ka}");
                }
                (DataValue::Vec3(x), DataValue::Vec3(y)) => {
                    let bx: [u32; 3] = x.map(f32::to_bits);
                    let by: [u32; 3] = y.map(f32::to_bits);
                    assert_eq!(bx, by, "vec3 bit mismatch for {ka}");
                }
                (DataValue::Array(x), DataValue::Array(y)) => {
                    let bx: Vec<u32> = x.iter().copied().map(f32::to_bits).collect();
                    let by: Vec<u32> = y.iter().copied().map(f32::to_bits).collect();
                    assert_eq!(bx, by, "array bit mismatch for {ka}");
                }
                _ => panic!("variant mismatch for {ka}: {va:?} vs {vb:?}"),
            }
        }
    }

    #[test]
    fn spec_validation_accepts_valid_chroma_data_names() {
        let valid = [
            make_spec(
                "temp",
                "chroma_Data_temperature",
                DataTransport::Cassette,
                true,
            ),
            make_spec(
                "wind",
                "chroma_Data_Wind_Vec3",
                DataTransport::LocalFile,
                false,
            ),
            make_spec("fft_0", "chroma_Data_0_bins", DataTransport::HttpPoll, true),
        ];

        for spec in valid {
            assert_eq!(spec.validate(), Ok(()));
        }
    }

    #[test]
    fn spec_validation_rejects_legacy_and_malformed_names() {
        let legacy = make_spec(
            "legacy",
            "syn_Data_temperature",
            DataTransport::Cassette,
            true,
        );
        assert_eq!(
            legacy.validate(),
            Err(DataIngestError::InvalidUniformName {
                name: "syn_Data_temperature".to_owned(),
            })
        );

        let bare_prefix = make_spec("bare", "chroma_Data_", DataTransport::Cassette, true);
        assert!(matches!(
            bare_prefix.validate(),
            Err(DataIngestError::InvalidUniformName { .. })
        ));

        let hyphen = make_spec(
            "bad_char",
            "chroma_Data_wind-speed",
            DataTransport::HttpPoll,
            true,
        );
        assert!(matches!(
            hyphen.validate(),
            Err(DataIngestError::InvalidUniformName { .. })
        ));

        let space = make_spec(
            "space",
            "chroma_Data_wind speed",
            DataTransport::WebSocket,
            true,
        );
        assert!(matches!(
            space.validate(),
            Err(DataIngestError::InvalidUniformName { .. })
        ));

        let empty_id = make_spec("", "chroma_Data_ok", DataTransport::Cassette, true);
        assert_eq!(empty_id.validate(), Err(DataIngestError::EmptySourceId));
    }

    #[test]
    fn register_source_rejects_duplicates() {
        let mut engine = DataIngestEngine::new();
        engine
            .register_source(make_spec(
                "weather",
                "chroma_Data_temp",
                DataTransport::Cassette,
                true,
            ))
            .unwrap();

        let dup_id = engine.register_source(make_spec(
            "weather",
            "chroma_Data_pressure",
            DataTransport::Cassette,
            true,
        ));
        assert_eq!(
            dup_id,
            Err(DataIngestError::DuplicateSourceId {
                id: "weather".to_owned(),
            })
        );

        let dup_uniform = engine.register_source(make_spec(
            "weather_2",
            "chroma_Data_temp",
            DataTransport::Cassette,
            true,
        ));
        assert_eq!(
            dup_uniform,
            Err(DataIngestError::DuplicateUniformName {
                uniform_name: "chroma_Data_temp".to_owned(),
            })
        );
    }

    #[test]
    fn opt_in_gate_rejects_non_opted_in_and_unknown_sources() {
        let mut engine = DataIngestEngine::new();
        engine
            .register_source(make_spec(
                "disabled_src",
                "chroma_Data_disabled",
                DataTransport::HttpPoll,
                false,
            ))
            .unwrap();

        let err = engine.publish_sample(0, "disabled_src", DataValue::Scalar(1.0));
        assert_eq!(
            err,
            Err(DataIngestError::SourceNotOptedIn {
                id: "disabled_src".to_owned(),
            })
        );

        let unknown = engine.publish_sample(0, "missing_src", DataValue::Scalar(1.0));
        assert_eq!(
            unknown,
            Err(DataIngestError::UnknownSource {
                id: "missing_src".to_owned(),
            })
        );

        assert!(engine.current_uniforms().is_empty());
    }

    #[test]
    fn record_serialize_deserialize_replay_is_bit_identical() {
        let mut recorder = DataIngestEngine::new();
        let specs = [
            make_spec("temp", "chroma_Data_temp", DataTransport::Cassette, true),
            make_spec("wind", "chroma_Data_wind", DataTransport::Cassette, true),
            make_spec(
                "spectrum",
                "chroma_Data_spectrum",
                DataTransport::Cassette,
                true,
            ),
        ];
        for spec in &specs {
            recorder.register_source(spec.clone()).unwrap();
        }

        recorder.begin_recording().unwrap();
        assert_eq!(
            recorder.state(),
            &DataIngestState::Recording {
                recorded_samples: 0,
            }
        );

        let mut expected_snapshots: Vec<(u64, BTreeMap<String, DataValue>)> = Vec::new();

        // Frame 0: scalar + vec3
        recorder
            .publish_sample(0, "temp", DataValue::Scalar(21.375))
            .unwrap();
        recorder
            .publish_sample(
                0,
                "wind",
                DataValue::Vec3([0.125, -0.0, std::f32::consts::PI]),
            )
            .unwrap();
        expected_snapshots.push((0, recorder.current_uniforms().clone()));

        // Frame 1: no new samples (latched values persist)
        expected_snapshots.push((1, recorder.current_uniforms().clone()));

        // Frame 2: array + updated scalar
        recorder
            .publish_sample(
                2,
                "spectrum",
                DataValue::Array(vec![0.0, 0.25, 0.5, 0.75, 1.0]),
            )
            .unwrap();
        recorder
            .publish_sample(2, "temp", DataValue::Scalar(-4.125))
            .unwrap();
        expected_snapshots.push((2, recorder.current_uniforms().clone()));

        assert_eq!(
            recorder.state(),
            &DataIngestState::Recording {
                recorded_samples: 4,
            }
        );

        let cassette = recorder.finish_recording().unwrap();
        assert_eq!(recorder.state(), &DataIngestState::Disabled);
        assert_eq!(cassette.version, CASSETTE_FORMAT_VERSION);
        assert_eq!(cassette.samples_for_frame(0).len(), 2);
        assert_eq!(cassette.samples_for_frame(1).len(), 0);
        assert_eq!(cassette.samples_for_frame(2).len(), 2);

        // Deterministic JSON serialization round-trip.
        let json_a = cassette.to_json().unwrap();
        let json_b = cassette.to_json().unwrap();
        assert_eq!(json_a, json_b);

        let decoded = DataCassette::from_json(&json_a).unwrap();
        assert_eq!(decoded, cassette);

        // Replay into a fresh engine and verify bit-identical snapshots per frame.
        let mut replayer = DataIngestEngine::new();
        for spec in specs {
            replayer.register_source(spec).unwrap();
        }
        replayer.load_cassette(decoded).unwrap();
        assert_eq!(
            replayer.state(),
            &DataIngestState::Replaying {
                cassette_len: 4,
                cursor_frame: 0,
            }
        );

        for (frame, expected) in &expected_snapshots {
            let actual = replayer.advance_frame(*frame).unwrap();
            assert_uniform_maps_bit_identical(&actual, expected);
            assert_eq!(
                replayer.state(),
                &DataIngestState::Replaying {
                    cassette_len: 4,
                    cursor_frame: *frame,
                }
            );
        }
    }

    #[test]
    fn poll_local_json_file_updates_scalar_and_array_uniforms() {
        let mut engine = DataIngestEngine::new();
        engine
            .register_source(make_spec(
                "sensor_scalar",
                "chroma_Data_sensor_temp",
                DataTransport::LocalFile,
                true,
            ))
            .unwrap();
        engine
            .register_source(make_spec(
                "sensor_array",
                "chroma_Data_sensor_bins",
                DataTransport::LocalFile,
                true,
            ))
            .unwrap();
        engine
            .register_source(make_spec(
                "sensor_blocked",
                "chroma_Data_blocked",
                DataTransport::LocalFile,
                false,
            ))
            .unwrap();

        let scalar_path = temp_json_path("scalar");
        let array_path = temp_json_path("array");

        std::fs::write(&scalar_path, r#"{"temperature_c": 23.5}"#).unwrap();
        std::fs::write(&array_path, r#"[0.1, 0.2, 0.3, 0.4]"#).unwrap();

        let scalar_val = engine
            .poll_local_json_file(10, "sensor_scalar", &scalar_path)
            .unwrap();
        assert_eq!(scalar_val, DataValue::Scalar(23.5));
        assert_eq!(
            engine.state(),
            &DataIngestState::LocalFileActive {
                path: scalar_path.clone(),
            }
        );

        let array_val = engine
            .poll_local_json_file(11, "sensor_array", &array_path)
            .unwrap();
        assert_eq!(array_val, DataValue::Array(vec![0.1, 0.2, 0.3, 0.4]));
        assert_eq!(
            engine.state(),
            &DataIngestState::LocalFileActive {
                path: array_path.clone(),
            }
        );

        assert_eq!(
            engine.current_uniforms().get("chroma_Data_sensor_temp"),
            Some(&DataValue::Scalar(23.5))
        );
        assert_eq!(
            engine.current_uniforms().get("chroma_Data_sensor_bins"),
            Some(&DataValue::Array(vec![0.1, 0.2, 0.3, 0.4]))
        );

        // Opt-in gate also applies to local file polling.
        let blocked = engine.poll_local_json_file(12, "sensor_blocked", &scalar_path);
        assert_eq!(
            blocked,
            Err(DataIngestError::SourceNotOptedIn {
                id: "sensor_blocked".to_owned(),
            })
        );

        let _ = std::fs::remove_file(&scalar_path);
        let _ = std::fs::remove_file(&array_path);
    }

    #[test]
    fn rejects_non_finite_values() {
        let mut engine = DataIngestEngine::new();
        engine
            .register_source(make_spec(
                "sig",
                "chroma_Data_signal",
                DataTransport::Cassette,
                true,
            ))
            .unwrap();

        assert_eq!(
            engine.publish_sample(0, "sig", DataValue::Scalar(f32::NAN)),
            Err(DataIngestError::NonFiniteValue {
                source_id: "sig".to_owned(),
            })
        );
        assert_eq!(
            engine.publish_sample(0, "sig", DataValue::Vec3([1.0, f32::INFINITY, 0.0])),
            Err(DataIngestError::NonFiniteValue {
                source_id: "sig".to_owned(),
            })
        );
    }
}
