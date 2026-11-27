// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Deterministic "Time-Machine" show recording and offline high-resolution
//! (1080p / 4K) GPU replay.
//!
//! Live visual performances can record per-frame control and audio uniform
//! state into a [`ShowRecording`], serialize it to a deterministic,
//! lossless canonical text representation
//! ([`ShowRecording::to_canonical_text`] /
//! [`ShowRecording::from_canonical_text`]), and later replay every recorded
//! frame sequentially on the GPU at arbitrary offline target resolutions
//! (e.g., 1920×1080 or 3840×2160) with zero dropped frames via
//! [`render_offline_recording`].
//!
//! The replay pipeline compiles the fragment shader once through
//! [`MultipassRenderer::compile_cached_single_pass`] using the standard
//! Chromaticity std140 uniform block:
//!
//! - `chroma_Time` (`vec4`: `elapsed_secs`, `1.0 / fps`, `frame`, `aspect`)
//! - `chroma_Resolution` (`vec4`: `width`, `height`, `1.0 / width`, `1.0 / height`)
//! - `syn_BPM` (`vec4`: `bpm`, `0`, `0`, `0`)
//! - `chroma_AudioRMS` (`vec4`: `rms`, `0`, `0`, `0`)
//! - `chroma_AudioPeak` (`vec4`: `peak`, `0`, `0`, `0`)
//! - `chroma_AudioFFT` (`float[8]`: 8 log-spaced FFT bands)
//! - `chroma_Ctrl0` (`vec4`: controls `0..4`)
//! - `chroma_Ctrl1` (`vec4`: controls `4..8`)

use std::fmt::Write as _;
use std::time::Instant;

use thiserror::Error;

use crate::multipass::{MultipassRenderer, UniformInput, UniformValue};
use crate::single_pass::RenderError;

const CANONICAL_HEADER: &str = "chromaticity-show-recording-v1";
const CSV_COLUMNS_HEADER: &str = "frame,elapsed_secs,bpm,rms,peak,fft0,fft1,fft2,fft3,fft4,fft5,fft6,fft7,ctrl0,ctrl1,ctrl2,ctrl3,ctrl4,ctrl5,ctrl6,ctrl7";
const CSV_COLUMN_COUNT: usize = 21;

const FNV1A_64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_64_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Errors returned by show-recording deserialization and offline GPU replay.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OfflineReplayError {
    /// Target render width or height was zero.
    #[error("invalid offline render dimensions: {width}x{height} (both must be > 0)")]
    InvalidDimensions { width: u32, height: u32 },

    /// Recording frame rate was zero.
    #[error("invalid recording fps: {fps} (must be > 0)")]
    InvalidFps { fps: u32 },

    /// Recording contained no frames to render.
    #[error("recording contains no frames")]
    EmptyRecording,

    /// Canonical text representation was malformed.
    #[error("failed to parse canonical show recording at line {line}: {reason}")]
    ParseError { line: usize, reason: String },

    /// Underlying GPU compilation or frame render failed.
    #[error("GPU render error: {0}")]
    Render(#[from] RenderError),
}

/// Deterministic per-frame snapshot captured during a live show.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedFrame {
    /// Monotonic frame index.
    pub frame: u64,
    /// Elapsed show time in seconds.
    pub elapsed_secs: f32,
    /// Tempo estimate in beats per minute.
    pub bpm: f32,
    /// Root-mean-square audio energy.
    pub rms: f32,
    /// Peak audio amplitude.
    pub peak: f32,
    /// Eight log-spaced FFT frequency bands.
    pub fft_bands: [f32; 8],
    /// Eight normalized scene control parameters.
    pub controls: [f32; 8],
}

/// Complete deterministic recording of a scene's per-frame inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct ShowRecording {
    /// Identifier of the scene recorded.
    pub scene_id: String,
    /// Nominal playback / export frames per second.
    pub fps: u32,
    /// Ordered sequence of recorded frames.
    pub frames: Vec<RecordedFrame>,
}

impl ShowRecording {
    /// Serialize this recording into a deterministic, lossless canonical
    /// text/CSV representation.
    ///
    /// Uses `\n` line endings and Rust's shortest round-trip decimal
    /// formatting for every `f32` field so [`Self::from_canonical_text`]
    /// reconstructs bit-identical finite floats with zero external
    /// serialization dependencies.
    #[must_use]
    pub fn to_canonical_text(&self) -> String {
        let mut out = String::with_capacity(128 + self.frames.len().saturating_mul(160));
        out.push_str(CANONICAL_HEADER);
        out.push('\n');
        out.push_str("scene_id=");
        escape_header_value(&self.scene_id, &mut out);
        out.push('\n');
        let _ = writeln!(out, "fps={}", self.fps);
        out.push_str(CSV_COLUMNS_HEADER);
        out.push('\n');

        for f in &self.frames {
            let _ = write!(
                out,
                "{},{},{},{},{}",
                f.frame, f.elapsed_secs, f.bpm, f.rms, f.peak
            );
            for band in f.fft_bands {
                let _ = write!(out, ",{band}");
            }
            for ctrl in f.controls {
                let _ = write!(out, ",{ctrl}");
            }
            out.push('\n');
        }
        out
    }

    /// Deserialize a [`ShowRecording`] produced by [`Self::to_canonical_text`].
    ///
    /// # Errors
    ///
    /// Returns [`OfflineReplayError::ParseError`] if the header, metadata
    /// lines, column count, or any numeric field is invalid.
    pub fn from_canonical_text(input: &str) -> Result<Self, OfflineReplayError> {
        let mut lines = input.lines().enumerate();

        let header = next_required_line(&mut lines, 1, "missing format header")?;
        if header != CANONICAL_HEADER {
            return Err(OfflineReplayError::ParseError {
                line: 1,
                reason: format!("expected header '{CANONICAL_HEADER}', got '{header}'"),
            });
        }

        let scene_line = next_required_line(&mut lines, 2, "missing scene_id metadata line")?;
        let raw_scene_id =
            scene_line
                .strip_prefix("scene_id=")
                .ok_or_else(|| OfflineReplayError::ParseError {
                    line: 2,
                    reason: "expected 'scene_id=<id>'".to_string(),
                })?;
        let scene_id = unescape_header_value(raw_scene_id, 2)?;

        let fps_line = next_required_line(&mut lines, 3, "missing fps metadata line")?;
        let raw_fps =
            fps_line
                .strip_prefix("fps=")
                .ok_or_else(|| OfflineReplayError::ParseError {
                    line: 3,
                    reason: "expected 'fps=<u32>'".to_string(),
                })?;
        let fps = raw_fps
            .parse::<u32>()
            .map_err(|e| OfflineReplayError::ParseError {
                line: 3,
                reason: format!("invalid fps '{raw_fps}': {e}"),
            })?;

        let cols_line = next_required_line(&mut lines, 4, "missing CSV column header")?;
        if cols_line != CSV_COLUMNS_HEADER {
            return Err(OfflineReplayError::ParseError {
                line: 4,
                reason: format!("unexpected CSV header '{cols_line}'"),
            });
        }

        let mut frames = Vec::new();
        for (idx, raw_line) in lines {
            let line_no = idx + 1;
            let line = raw_line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            frames.push(parse_recorded_frame_csv(line, line_no)?);
        }

        Ok(Self {
            scene_id,
            fps,
            frames,
        })
    }
}

/// Target configuration for an offline GPU replay pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineRenderConfig {
    /// Diagnostic label attached to the compiled pipeline.
    pub label: String,
    /// GLSL 4.50 fragment shader source consuming the standard Chromaticity
    /// std140 uniform block.
    pub fragment_glsl: String,
    /// Output frame width in pixels (must be > 0).
    pub width: u32,
    /// Output frame height in pixels (must be > 0).
    pub height: u32,
}

/// Summary report returned after completing an offline GPU replay.
#[derive(Debug, Clone, PartialEq)]
pub struct OfflineRenderReport {
    /// Total number of frames rendered and delivered to the callback.
    pub frames_rendered: usize,
    /// Output frame width in pixels.
    pub width: u32,
    /// Output frame height in pixels.
    pub height: u32,
    /// Deterministic 64-bit FNV-1a hash of each rendered RGBA8 frame buffer.
    pub frame_hashes: Vec<u64>,
    /// Total wall-clock duration of compilation and rendering in milliseconds.
    pub total_ms: f64,
}

/// Compute a deterministic 64-bit FNV-1a hash over a byte slice.
#[must_use]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV1A_64_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV1A_64_PRIME);
    }
    hash
}

/// Build the standard Chromaticity std140 uniform vector for one
/// [`RecordedFrame`] at `target_size` and `fps`.
#[must_use]
pub fn build_recorded_frame_uniforms(
    target_size: (u32, u32),
    fps: u32,
    recorded: &RecordedFrame,
) -> Vec<UniformInput> {
    let w = target_size.0 as f32;
    let h = target_size.1 as f32;
    let aspect = if h > 0.0 { w / h } else { 1.0 };
    let dt = if fps > 0 { 1.0 / (fps as f32) } else { 0.0 };

    vec![
        UniformInput {
            name: String::from("chroma_Time"),
            value: UniformValue::Vec4([recorded.elapsed_secs, dt, recorded.frame as f32, aspect]),
        },
        UniformInput {
            name: String::from("chroma_Resolution"),
            value: UniformValue::Vec4([w, h, 1.0 / w.max(1.0), 1.0 / h.max(1.0)]),
        },
        UniformInput {
            name: String::from("syn_BPM"),
            value: UniformValue::Vec4([recorded.bpm, 0.0, 0.0, 0.0]),
        },
        UniformInput {
            name: String::from("chroma_AudioRMS"),
            value: UniformValue::Vec4([recorded.rms, 0.0, 0.0, 0.0]),
        },
        UniformInput {
            name: String::from("chroma_AudioPeak"),
            value: UniformValue::Vec4([recorded.peak, 0.0, 0.0, 0.0]),
        },
        UniformInput {
            name: String::from("chroma_AudioFFT"),
            value: UniformValue::FloatArray(recorded.fft_bands.to_vec()),
        },
        UniformInput {
            name: String::from("chroma_Ctrl0"),
            value: UniformValue::Vec4([
                recorded.controls[0],
                recorded.controls[1],
                recorded.controls[2],
                recorded.controls[3],
            ]),
        },
        UniformInput {
            name: String::from("chroma_Ctrl1"),
            value: UniformValue::Vec4([
                recorded.controls[4],
                recorded.controls[5],
                recorded.controls[6],
                recorded.controls[7],
            ]),
        },
    ]
}

/// Replay a [`ShowRecording`] offline on the GPU with zero dropped frames.
///
/// Validates `config.width > 0`, `config.height > 0`, `recording.fps > 0`,
/// and `!recording.frames.is_empty()` before issuing any GPU work. Compiles
/// `config.fragment_glsl` once via
/// [`MultipassRenderer::compile_cached_single_pass`], renders every recorded
/// frame sequentially into an offscreen `Rgba8Unorm` target, computes a
/// deterministic 64-bit FNV-1a hash per frame, and invokes
/// `on_frame(frame.frame, &rgba_bytes)`.
///
/// # Errors
///
/// - [`OfflineReplayError::InvalidDimensions`] — `width == 0` or `height == 0`.
/// - [`OfflineReplayError::InvalidFps`] — `recording.fps == 0`.
/// - [`OfflineReplayError::EmptyRecording`] — `recording.frames` is empty.
/// - [`OfflineReplayError::Render`] — shader compilation or GPU render error.
pub fn render_offline_recording<F>(
    renderer: &MultipassRenderer,
    config: &OfflineRenderConfig,
    recording: &ShowRecording,
    mut on_frame: F,
) -> Result<OfflineRenderReport, OfflineReplayError>
where
    F: FnMut(u64, &[u8]),
{
    if config.width == 0 || config.height == 0 {
        return Err(OfflineReplayError::InvalidDimensions {
            width: config.width,
            height: config.height,
        });
    }
    if recording.fps == 0 {
        return Err(OfflineReplayError::InvalidFps { fps: recording.fps });
    }
    let Some(first_frame) = recording.frames.first() else {
        return Err(OfflineReplayError::EmptyRecording);
    };

    let started = Instant::now();
    let target_size = (config.width, config.height);
    let initial_uniforms = build_recorded_frame_uniforms(target_size, recording.fps, first_frame);

    let mut cached = renderer.compile_cached_single_pass(
        &config.label,
        &config.fragment_glsl,
        &initial_uniforms,
        target_size,
    )?;

    let mut frame_hashes = Vec::with_capacity(recording.frames.len());
    for recorded in &recording.frames {
        let uniforms = build_recorded_frame_uniforms(target_size, recording.fps, recorded);
        let rgba = cached.render(renderer, target_size, &uniforms)?;
        let hash = fnv1a_64(&rgba);
        frame_hashes.push(hash);
        on_frame(recorded.frame, &rgba);
    }

    let total_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(OfflineRenderReport {
        frames_rendered: frame_hashes.len(),
        width: config.width,
        height: config.height,
        frame_hashes,
        total_ms,
    })
}

fn next_required_line<'a>(
    lines: &mut std::iter::Enumerate<std::str::Lines<'a>>,
    expected_line: usize,
    reason: &str,
) -> Result<&'a str, OfflineReplayError> {
    lines
        .next()
        .map(|(_, line)| line.trim_end_matches('\r'))
        .ok_or_else(|| OfflineReplayError::ParseError {
            line: expected_line,
            reason: reason.to_string(),
        })
}

fn escape_header_value(raw: &str, out: &mut String) {
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
}

fn unescape_header_value(escaped: &str, line: usize) -> Result<String, OfflineReplayError> {
    let mut out = String::with_capacity(escaped.len());
    let mut chars = escaped.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    return Err(OfflineReplayError::ParseError {
                        line,
                        reason: format!("unknown escape sequence '\\{other}'"),
                    });
                }
                None => {
                    return Err(OfflineReplayError::ParseError {
                        line,
                        reason: "dangling backslash in metadata value".to_string(),
                    });
                }
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn parse_recorded_frame_csv(
    line: &str,
    line_no: usize,
) -> Result<RecordedFrame, OfflineReplayError> {
    let mut cols = line.split(',');
    let mut next_col = |name: &str| -> Result<&str, OfflineReplayError> {
        cols.next().ok_or_else(|| OfflineReplayError::ParseError {
            line: line_no,
            reason: format!("missing column '{name}' (expected {CSV_COLUMN_COUNT} columns)"),
        })
    };

    let frame = parse_u64_col(next_col("frame")?, "frame", line_no)?;
    let elapsed_secs = parse_f32_col(next_col("elapsed_secs")?, "elapsed_secs", line_no)?;
    let bpm = parse_f32_col(next_col("bpm")?, "bpm", line_no)?;
    let rms = parse_f32_col(next_col("rms")?, "rms", line_no)?;
    let peak = parse_f32_col(next_col("peak")?, "peak", line_no)?;

    let mut fft_bands = [0.0_f32; 8];
    for (i, slot) in fft_bands.iter_mut().enumerate() {
        let name = format!("fft{i}");
        *slot = parse_f32_col(next_col(&name)?, &name, line_no)?;
    }

    let mut controls = [0.0_f32; 8];
    for (i, slot) in controls.iter_mut().enumerate() {
        let name = format!("ctrl{i}");
        *slot = parse_f32_col(next_col(&name)?, &name, line_no)?;
    }

    if cols.next().is_some() {
        return Err(OfflineReplayError::ParseError {
            line: line_no,
            reason: format!("too many columns (expected {CSV_COLUMN_COUNT})"),
        });
    }

    Ok(RecordedFrame {
        frame,
        elapsed_secs,
        bpm,
        rms,
        peak,
        fft_bands,
        controls,
    })
}

fn parse_u64_col(raw: &str, name: &str, line: usize) -> Result<u64, OfflineReplayError> {
    raw.parse::<u64>()
        .map_err(|e| OfflineReplayError::ParseError {
            line,
            reason: format!("invalid u64 in column '{name}' ('{raw}'): {e}"),
        })
}

fn parse_f32_col(raw: &str, name: &str, line: usize) -> Result<f32, OfflineReplayError> {
    raw.parse::<f32>()
        .map_err(|e| OfflineReplayError::ParseError {
            line,
            reason: format!("invalid f32 in column '{name}' ('{raw}'): {e}"),
        })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod offline_replay_pins {
    //! Evidence: EV-DETERMINISM (T1) — offline high-res GPU replay determinism pins.
    use super::*;

    const OFFLINE_TEST_SHADER: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 syn_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
};

void main() {
    float r = clamp(uv.x * 0.4 + chroma_AudioRMS.x * 0.4 + chroma_Ctrl0.x * 0.2, 0.0, 1.0);
    float g = clamp(uv.y * 0.4 + chroma_AudioPeak.x * 0.3 + chroma_Ctrl1.y * 0.3, 0.0, 1.0);
    float b = clamp(0.15 + chroma_AudioFFT[0] * 0.35 + chroma_AudioFFT[7] * 0.35 + fract(chroma_Time.x) * 0.15, 0.0, 1.0);
    fragColor = vec4(r, g, b, 1.0);
}
"#;

    fn sample_recording(frame_count: u64) -> ShowRecording {
        let frames = (0..frame_count)
            .map(|i| {
                let t = i as f32 / 60.0;
                RecordedFrame {
                    frame: i,
                    elapsed_secs: t,
                    bpm: 120.0 + (i as f32) * 0.5,
                    rms: 0.1 + (i as f32) * 0.15,
                    peak: 0.2 + (i as f32) * 0.12,
                    fft_bands: [
                        0.05 + (i as f32) * 0.1,
                        0.10,
                        0.15,
                        0.20,
                        0.25,
                        0.30,
                        0.35,
                        0.40 + (i as f32) * 0.08,
                    ],
                    controls: [
                        0.125 * (i as f32),
                        0.25,
                        0.5,
                        0.75,
                        0.1,
                        0.2 + (i as f32) * 0.1,
                        0.6,
                        0.9,
                    ],
                }
            })
            .collect();

        ShowRecording {
            scene_id: String::from("demo/nebula-pulse"),
            fps: 60,
            frames,
        }
    }

    #[test]
    fn canonical_text_round_trip_preserves_exact_fields() {
        let recording = ShowRecording {
            scene_id: String::from("scenes/live\\set\npart-1"),
            fps: 60,
            frames: vec![
                RecordedFrame {
                    frame: 0,
                    elapsed_secs: 0.0,
                    bpm: 128.125,
                    rms: 1.0 / 3.0,
                    peak: 0.999_999_94,
                    fft_bands: [
                        0.0,
                        1.0 / 7.0,
                        0.25,
                        0.333_333_34,
                        0.5,
                        0.666_666_7,
                        0.875,
                        1.0,
                    ],
                    controls: [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7],
                },
                RecordedFrame {
                    frame: 42,
                    elapsed_secs: 42.0 / 60.0,
                    bpm: 140.0,
                    rms: 0.42,
                    peak: 0.88,
                    fft_bands: [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1],
                    controls: [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3],
                },
            ],
        };

        let text_a = recording.to_canonical_text();
        let text_b = recording.to_canonical_text();
        assert_eq!(text_a, text_b, "canonical text must be deterministic");

        let parsed = ShowRecording::from_canonical_text(&text_a).unwrap();
        assert_eq!(parsed, recording);
        assert_eq!(parsed.to_canonical_text(), text_a);
    }

    #[test]
    fn invalid_configs_reject_cleanly_before_gpu_work() {
        let renderer = MultipassRenderer::new().unwrap();
        let valid_recording = sample_recording(2);

        // Zero width
        let cfg_zero_w = OfflineRenderConfig {
            label: String::from("zero-w"),
            fragment_glsl: String::from("not even valid glsl"),
            width: 0,
            height: 64,
        };
        assert!(matches!(
            render_offline_recording(&renderer, &cfg_zero_w, &valid_recording, |_, _| {}),
            Err(OfflineReplayError::InvalidDimensions {
                width: 0,
                height: 64
            })
        ));

        // Zero height
        let cfg_zero_h = OfflineRenderConfig {
            label: String::from("zero-h"),
            fragment_glsl: String::from("not even valid glsl"),
            width: 64,
            height: 0,
        };
        assert!(matches!(
            render_offline_recording(&renderer, &cfg_zero_h, &valid_recording, |_, _| {}),
            Err(OfflineReplayError::InvalidDimensions {
                width: 64,
                height: 0
            })
        ));

        // Zero fps
        let mut zero_fps = valid_recording.clone();
        zero_fps.fps = 0;
        let cfg_valid = OfflineRenderConfig {
            label: String::from("invalid-recording"),
            fragment_glsl: String::from("not even valid glsl"),
            width: 64,
            height: 64,
        };
        assert!(matches!(
            render_offline_recording(&renderer, &cfg_valid, &zero_fps, |_, _| {}),
            Err(OfflineReplayError::InvalidFps { fps: 0 })
        ));

        // Empty frames
        let empty = ShowRecording {
            scene_id: String::from("empty"),
            fps: 60,
            frames: Vec::new(),
        };
        assert!(matches!(
            render_offline_recording(&renderer, &cfg_valid, &empty, |_, _| {}),
            Err(OfflineReplayError::EmptyRecording)
        ));
    }

    #[test]
    fn determinism_gpu_pin_five_frames_bit_identical() {
        let renderer = MultipassRenderer::new().unwrap();
        let recording = sample_recording(5);
        let config = OfflineRenderConfig {
            label: String::from("determinism-pin"),
            fragment_glsl: String::from(OFFLINE_TEST_SHADER),
            width: 128,
            height: 72,
        };

        let mut run_a_frames: Vec<(u64, Vec<u8>)> = Vec::new();
        let report_a = render_offline_recording(&renderer, &config, &recording, |idx, rgba| {
            run_a_frames.push((idx, rgba.to_vec()));
        })
        .unwrap();

        let mut run_b_frames: Vec<(u64, Vec<u8>)> = Vec::new();
        let report_b = render_offline_recording(&renderer, &config, &recording, |idx, rgba| {
            run_b_frames.push((idx, rgba.to_vec()));
        })
        .unwrap();

        assert_eq!(report_a.frames_rendered, 5);
        assert_eq!(report_b.frames_rendered, 5);
        assert_eq!(report_a.width, 128);
        assert_eq!(report_a.height, 72);
        assert_eq!(
            report_a.frame_hashes, report_b.frame_hashes,
            "two offline replays of the same recording must yield identical hashes"
        );
        assert_eq!(
            run_a_frames, run_b_frames,
            "two offline replays of the same recording must yield bit-identical RGBA buffers"
        );

        // Changing audio + control values across the 5 frames must yield distinct hashes.
        let mut unique_hashes = report_a.frame_hashes.clone();
        unique_hashes.sort_unstable();
        unique_hashes.dedup();
        assert_eq!(
            unique_hashes.len(),
            5,
            "frames with changing controls and audio must produce distinct hashes"
        );
    }

    #[test]
    fn high_res_1080p_offline_pin() {
        let renderer = MultipassRenderer::new().unwrap();
        let recording = sample_recording(2);
        let config = OfflineRenderConfig {
            label: String::from("offline-1080p-pin"),
            fragment_glsl: String::from(OFFLINE_TEST_SHADER),
            width: 1920,
            height: 1080,
        };

        let expected_len = 1920 * 1080 * 4;
        let mut seen_frames = Vec::new();
        let report = render_offline_recording(&renderer, &config, &recording, |idx, rgba| {
            assert_eq!(rgba.len(), expected_len);
            assert!(
                rgba.iter().any(|&b| b != 0),
                "1080p frame {idx} must be non-black"
            );
            seen_frames.push(idx);
        })
        .unwrap();

        assert_eq!(report.frames_rendered, 2);
        assert_eq!(report.width, 1920);
        assert_eq!(report.height, 1080);
        assert_eq!(seen_frames, vec![0, 1]);
        assert_eq!(report.frame_hashes.len(), 2);
        assert_ne!(report.frame_hashes[0], report.frame_hashes[1]);
    }
}
