// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Live Twitch RTMPS/RTMP broadcast publisher IPC handlers.
//!
//! Exposes [`start_twitch_stream`], [`stop_twitch_stream`], [`get_stream_status`],
//! and [`push_stream_frame`] backed by [`chromaticity_runtime::broadcast_stream`]
//! (`StreamProtocol`, `StreamSecretKey`, `StreamEndpointConfig`,
//! `BroadcastStreamer`, `StreamState`) and an external `ffmpeg` child process
//! managed on [`AppState`] via [`TwitchStreamProcess`].
//!
//! On macOS, hardware-accelerated `h264_videotoolbox` is selected; non-macOS
//! targets fall back to `libx264`. No GPL libraries are linked into the Rust
//! binary. Every error message surfaced to callers is scrubbed through
//! [`StreamSecretKey::redact_in_text`] so the stream key can never leak.

#![forbid(unsafe_code)]

use crate::state::AppState;
use chromaticity_runtime::broadcast_stream::{
    BroadcastStreamer, MIN_ALLOWED_BITRATE_KBPS, StreamEndpointConfig, StreamProtocol,
    StreamSecretKey, StreamState,
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

/// Default Twitch RTMPS ingest URL displayed when idle.
pub const DEFAULT_TWITCH_INGEST_URL: &str = "rtmps://live.twitch.tv/app";

/// Default steady-state broadcast video bitrate in kbps (6 Mbps).
pub const DEFAULT_STREAM_BITRATE_KBPS: u32 = 6_000;

/// Default broadcast output width in pixels.
pub const DEFAULT_STREAM_WIDTH: u32 = 1280;

/// Default broadcast output height in pixels.
pub const DEFAULT_STREAM_HEIGHT: u32 = 720;

/// Default broadcast frame rate in frames per second.
pub const DEFAULT_STREAM_FPS: u32 = 30;

/// Maximum permitted broadcast width in pixels (8K UHD).
const MAX_STREAM_WIDTH: u32 = 7_680;

/// Maximum permitted broadcast height in pixels (8K UHD).
const MAX_STREAM_HEIGHT: u32 = 4_320;

/// Maximum permitted broadcast frame rate.
const MAX_STREAM_FPS: u32 = 120;

/// Maximum bytes retained from `ffmpeg` stderr for redacted error diagnostics.
const MAX_STDERR_CAPTURE_BYTES: usize = 4_096;

/// Maximum permitted length for an optional `avfoundation` audio device identifier.
const MAX_AUDIO_DEVICE_LEN: usize = 64;

/// Serializable live broadcast status snapshot returned across IPC.
///
/// Intentionally omits the stream key so credentials can never cross the IPC
/// boundary back to the UI or diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamStatusDto {
    /// Explicit lifecycle state label (`"offline"`, `"connecting"`, `"live"`,
    /// `"reconnecting"`, or `"failed"`).
    pub state: String,
    /// `true` iff the stream is actively live and accepting frames.
    pub is_live: bool,
    /// Active or configured ingest protocol label (`"rtmps"`, `"rtmp"`, etc.).
    pub protocol: String,
    /// Configured ingest base URL (without the stream key).
    pub ingest_url: String,
    /// Target video bitrate in kbps.
    pub bitrate_kbps: u32,
    /// Active or configured output width in pixels.
    pub width: u32,
    /// Active or configured output height in pixels.
    pub height: u32,
    /// Active or configured output frame rate in FPS.
    pub fps: u32,
    /// Total raw RGBA frames written to the encoder in the current session.
    pub frames_pushed: u64,
    /// Video encoder codec name (`"h264_videotoolbox"` on macOS, `"libx264"` elsewhere).
    pub encoder: String,
    /// Most recent secret-redacted error message, if any.
    pub last_error: Option<String>,
}

/// Returns the platform-appropriate H.264 video encoder name for `ffmpeg`.
#[must_use]
pub const fn selected_video_encoder() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "h264_videotoolbox"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "libx264"
    }
}

/// Resolves the `ffmpeg` executable path.
///
/// Checks `/opt/homebrew/bin/ffmpeg` (Apple Silicon Homebrew),
/// `/usr/local/bin/ffmpeg` (Intel Homebrew / manual install), and then scans
/// every directory on `PATH`.
#[must_use]
pub fn resolve_ffmpeg_binary() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("/opt/homebrew/bin/ffmpeg"),
        PathBuf::from("/usr/local/bin/ffmpeg"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let bin_name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full = dir.join(bin_name);
        if full.is_file() {
            return Some(full);
        }
    }

    None
}

/// Validates an optional audio input device string (e.g. `":0"` or `"BlackHole 2ch"`).
///
/// Returns `Ok(None)` when the trimmed string is empty. Rejects leading `'-'`
/// (preventing ffmpeg option injection), strings longer than 64 characters, and
/// characters outside `[A-Za-z0-9:_\- ]`.
pub fn validate_audio_device(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.len() > MAX_AUDIO_DEVICE_LEN {
        return Err(format!(
            "audio device identifier exceeds maximum length of {MAX_AUDIO_DEVICE_LEN} characters"
        ));
    }
    if trimmed.starts_with('-') {
        return Err("audio device identifier must not start with '-'".to_owned());
    }
    if !trimmed
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_' | b' '))
    {
        return Err(
            "audio device identifier contains unsupported characters (allowed: alphanumeric, ':', '-', '_', space)"
                .to_owned(),
        );
    }
    Ok(Some(trimmed.to_owned()))
}

/// Builds the `ffmpeg` argument list for live Twitch RTMPS/RTMP FLV publishing.
///
/// Configures raw RGBA video on `pipe:0`, either a validated macOS
/// `avfoundation` audio capture input when `audio_device` is supplied on macOS
/// or a silent 44.1 kHz stereo `anullsrc` AAC audio track (required by Twitch
/// ingest), hardware `h264_videotoolbox` on macOS (falling back to `libx264` on
/// non-macOS), and FLV output targeting `{trimmed_ingest}/{stream_key}`.
pub fn build_twitch_ffmpeg_args(
    ingest_url: &str,
    stream_key: &StreamSecretKey,
    bitrate_kbps: u32,
    width: u32,
    height: u32,
    fps: u32,
    audio_device: Option<&str>,
) -> Result<Vec<String>, String> {
    let validated_audio = match audio_device {
        Some(raw) => validate_audio_device(raw).map_err(|err| stream_key.redact_in_text(&err))?,
        None => None,
    };

    let trimmed_ingest = ingest_url.trim().trim_end_matches('/');
    let destination = format!("{trimmed_ingest}/{}", stream_key.expose_secret());
    let video_size = format!("{width}x{height}");
    let fps_str = fps.to_string();
    let gop_str = fps.saturating_mul(2).to_string();
    let bitrate_str = format!("{bitrate_kbps}k");
    let bufsize_str = format!("{}k", bitrate_kbps.saturating_mul(2));

    let mut args = vec![
        "-hide_banner".to_owned(),
        "-loglevel".to_owned(),
        "error".to_owned(),
        "-use_wallclock_as_timestamps".to_owned(),
        "1".to_owned(),
        "-f".to_owned(),
        "rawvideo".to_owned(),
        "-pix_fmt".to_owned(),
        "rgba".to_owned(),
        "-s".to_owned(),
        video_size,
        "-r".to_owned(),
        fps_str,
        "-i".to_owned(),
        "pipe:0".to_owned(),
    ];

    #[cfg(target_os = "macos")]
    {
        if let Some(dev) = validated_audio {
            args.extend([
                "-f".to_owned(),
                "avfoundation".to_owned(),
                "-i".to_owned(),
                dev,
            ]);
        } else {
            args.extend([
                "-f".to_owned(),
                "lavfi".to_owned(),
                "-i".to_owned(),
                "anullsrc=channel_layout=stereo:sample_rate=44100".to_owned(),
            ]);
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = validated_audio;
        args.extend([
            "-f".to_owned(),
            "lavfi".to_owned(),
            "-i".to_owned(),
            "anullsrc=channel_layout=stereo:sample_rate=44100".to_owned(),
        ]);
    }

    #[cfg(target_os = "macos")]
    {
        args.extend([
            "-c:v".to_owned(),
            "h264_videotoolbox".to_owned(),
            "-b:v".to_owned(),
            bitrate_str.clone(),
            "-maxrate".to_owned(),
            bitrate_str,
            "-bufsize".to_owned(),
            bufsize_str,
            "-g".to_owned(),
            gop_str,
            "-pix_fmt".to_owned(),
            "yuv420p".to_owned(),
            "-realtime".to_owned(),
            "true".to_owned(),
        ]);
    }

    #[cfg(not(target_os = "macos"))]
    {
        args.extend([
            "-c:v".to_owned(),
            "libx264".to_owned(),
            "-preset".to_owned(),
            "veryfast".to_owned(),
            "-tune".to_owned(),
            "zerolatency".to_owned(),
            "-b:v".to_owned(),
            bitrate_str.clone(),
            "-maxrate".to_owned(),
            bitrate_str,
            "-bufsize".to_owned(),
            bufsize_str,
            "-g".to_owned(),
            gop_str,
            "-pix_fmt".to_owned(),
            "yuv420p".to_owned(),
        ]);
    }

    args.extend([
        "-c:a".to_owned(),
        "aac".to_owned(),
        "-b:a".to_owned(),
        "160k".to_owned(),
        "-ar".to_owned(),
        "44100".to_owned(),
        "-f".to_owned(),
        "flv".to_owned(),
        destination,
    ]);

    Ok(args)
}

/// Managed `ffmpeg` child process and broadcast state machine stored on [`AppState`].
///
/// Implements [`Drop`] to guarantee that any running `ffmpeg` child process is
/// killed and reaped on shutdown so no zombie encoder can survive app exit.
pub struct TwitchStreamProcess {
    streamer: Option<BroadcastStreamer>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stderr_tail: Arc<Mutex<String>>,
    protocol: StreamProtocol,
    ingest_url: String,
    bitrate_kbps: u32,
    width: u32,
    height: u32,
    fps: u32,
    frames_pushed: u64,
    encoder: String,
    last_error: Option<String>,
}

impl core::fmt::Debug for TwitchStreamProcess {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TwitchStreamProcess")
            .field("state", &self.state_name())
            .field("protocol", &self.protocol)
            .field("ingest_url", &self.ingest_url)
            .field("bitrate_kbps", &self.bitrate_kbps)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("fps", &self.fps)
            .field("frames_pushed", &self.frames_pushed)
            .field("encoder", &self.encoder)
            .field("child_active", &self.child.is_some())
            .field("last_error", &self.last_error)
            .finish()
    }
}

impl Default for TwitchStreamProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitchStreamProcess {
    /// Creates an idle (offline) managed stream process holder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            streamer: None,
            child: None,
            stdin: None,
            stderr_tail: Arc::new(Mutex::new(String::new())),
            protocol: StreamProtocol::Rtmps,
            ingest_url: DEFAULT_TWITCH_INGEST_URL.to_owned(),
            bitrate_kbps: DEFAULT_STREAM_BITRATE_KBPS,
            width: DEFAULT_STREAM_WIDTH,
            height: DEFAULT_STREAM_HEIGHT,
            fps: DEFAULT_STREAM_FPS,
            frames_pushed: 0,
            encoder: selected_video_encoder().to_owned(),
            last_error: None,
        }
    }

    /// Returns the deterministic lowercase state label for diagnostics.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        self.streamer
            .as_ref()
            .map_or("offline", |s| s.state().label())
    }

    /// Closes stdin, kills any running `ffmpeg` child, and reaps its exit status.
    ///
    /// Safe and idempotent to call when no child process is active.
    pub fn terminate_child(&mut self) {
        // Close stdin first so ffmpeg receives EOF cleanly before kill.
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }

    /// Stops any active stream and returns the state machine to `offline`.
    pub fn stop(&mut self) {
        self.terminate_child();
        if let Some(streamer) = self.streamer.as_mut() {
            streamer.stop();
        }
        self.frames_pushed = 0;
    }

    /// Polls the running child process (if any) and transitions state if it exited unexpectedly.
    pub fn poll_child_health(&mut self) {
        let exited_status = match self.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => Some(Ok(status)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            },
            None => None,
        };

        if let Some(exit_result) = exited_status {
            drop(self.stdin.take());
            self.child = None;

            let stderr_excerpt = self.read_stderr_tail();
            let raw_msg = match exit_result {
                Ok(status) => {
                    if stderr_excerpt.is_empty() {
                        format!("ffmpeg exited unexpectedly with status {status}")
                    } else {
                        format!("ffmpeg exited with status {status}: {stderr_excerpt}")
                    }
                }
                Err(err) => format!("failed to poll ffmpeg child process: {err}"),
            };

            let redacted = self.redact_message(&raw_msg);
            self.last_error = Some(redacted.clone());
            if let Some(streamer) = self.streamer.as_mut() {
                let _ = streamer.on_transport_error(current_unix_ms(), &redacted);
            }
        }
    }

    /// Builds a serializable [`StreamStatusDto`] snapshot (never includes the stream key).
    #[must_use]
    pub fn status_dto(&self) -> StreamStatusDto {
        let state = self.state_name();
        let is_live = self.child.is_some()
            && self
                .streamer
                .as_ref()
                .is_some_and(|s| matches!(s.state(), StreamState::Live { .. }));
        StreamStatusDto {
            state: state.to_owned(),
            is_live,
            protocol: self.protocol.label().to_owned(),
            ingest_url: self.ingest_url.clone(),
            bitrate_kbps: self.bitrate_kbps,
            width: self.width,
            height: self.height,
            fps: self.fps,
            frames_pushed: self.frames_pushed,
            encoder: self.encoder.clone(),
            last_error: self.last_error.clone(),
        }
    }

    fn redact_message(&self, msg: &str) -> String {
        match self.streamer.as_ref() {
            Some(streamer) => streamer.config().stream_key.redact_in_text(msg),
            None => msg.to_owned(),
        }
    }

    fn read_stderr_tail(&self) -> String {
        let guard = match self.stderr_tail.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.trim().to_owned()
    }
}

impl Drop for TwitchStreamProcess {
    fn drop(&mut self) {
        self.terminate_child();
    }
}

fn current_unix_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(dur) => u64::try_from(dur.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

fn infer_protocol(ingest_url: &str) -> StreamProtocol {
    if ingest_url.starts_with("rtmp://") {
        StreamProtocol::Rtmp
    } else {
        StreamProtocol::Rtmps
    }
}

fn validate_stream_dimensions(width: u32, height: u32, fps: u32) -> Result<(), String> {
    if width == 0 || width > MAX_STREAM_WIDTH {
        return Err(format!(
            "stream width ({width}) must be within 1..={MAX_STREAM_WIDTH}"
        ));
    }
    if height == 0 || height > MAX_STREAM_HEIGHT {
        return Err(format!(
            "stream height ({height}) must be within 1..={MAX_STREAM_HEIGHT}"
        ));
    }
    if width % 2 != 0 || height % 2 != 0 {
        return Err(format!(
            "stream dimensions ({width}x{height}) must be even for yuv420p encoding"
        ));
    }
    if fps == 0 || fps > MAX_STREAM_FPS {
        return Err(format!(
            "stream fps ({fps}) must be within 1..={MAX_STREAM_FPS}"
        ));
    }
    Ok(())
}

/// Validates and builds a [`StreamEndpointConfig`] for a Twitch RTMPS/RTMP stream.
pub fn build_validated_endpoint_config(
    ingest_url: &str,
    stream_key: StreamSecretKey,
    bitrate_kbps: u32,
) -> Result<StreamEndpointConfig, String> {
    let trimmed_url = ingest_url.trim();
    let protocol = infer_protocol(trimmed_url);
    let min_bitrate_kbps = (bitrate_kbps / 4)
        .max(MIN_ALLOWED_BITRATE_KBPS)
        .min(bitrate_kbps);
    let config = StreamEndpointConfig {
        protocol,
        ingest_url: trimmed_url.to_owned(),
        stream_key: stream_key.clone(),
        target_bitrate_kbps: bitrate_kbps,
        min_bitrate_kbps,
        max_reconnect_attempts: 3,
    };
    config
        .validate()
        .map_err(|err| stream_key.redact_in_text(&err.to_string()))?;
    Ok(config)
}

/// Inner implementation for [`start_twitch_stream`].
#[allow(clippy::too_many_arguments)]
pub fn start_twitch_stream_impl(
    app: &AppState,
    ingest_url: String,
    stream_key: String,
    bitrate_kbps: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<u32>,
    audio_device: Option<String>,
) -> Result<StreamStatusDto, String> {
    let target_bitrate = bitrate_kbps.unwrap_or(DEFAULT_STREAM_BITRATE_KBPS);
    let target_width = width.unwrap_or(DEFAULT_STREAM_WIDTH);
    let target_height = height.unwrap_or(DEFAULT_STREAM_HEIGHT);
    let target_fps = fps.unwrap_or(DEFAULT_STREAM_FPS);

    let secret = StreamSecretKey::new(stream_key);
    validate_stream_dimensions(target_width, target_height, target_fps)
        .map_err(|err| secret.redact_in_text(&err))?;

    let config = build_validated_endpoint_config(&ingest_url, secret.clone(), target_bitrate)?;
    let args = build_twitch_ffmpeg_args(
        &config.ingest_url,
        &config.stream_key,
        target_bitrate,
        target_width,
        target_height,
        target_fps,
        audio_device.as_deref(),
    )?;

    let mut streamer = BroadcastStreamer::new(config.clone())
        .map_err(|err| secret.redact_in_text(&err.to_string()))?;
    streamer
        .start()
        .map_err(|err| secret.redact_in_text(&err.to_string()))?;

    let ffmpeg_path = resolve_ffmpeg_binary().ok_or_else(|| {
        "ffmpeg binary not found (checked /opt/homebrew/bin/ffmpeg, /usr/local/bin/ffmpeg, and PATH)"
            .to_owned()
    })?;

    let mut proc_guard = match app.twitch_stream.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // Always clean up any previous child process before spawning a new one.
    proc_guard.stop();

    let mut child = Command::new(&ffmpeg_path)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            let msg = format!("failed to spawn ffmpeg ({}): {err}", ffmpeg_path.display());
            let redacted = secret.redact_in_text(&msg);
            proc_guard.last_error = Some(redacted.clone());
            redacted
        })?;

    let stdin = child.stdin.take().ok_or_else(|| {
        let _ = child.kill();
        let _ = child.wait();
        let msg = "failed to attach stdin pipe to ffmpeg child process".to_owned();
        proc_guard.last_error = Some(msg.clone());
        msg
    })?;

    let stderr_tail = Arc::new(Mutex::new(String::new()));
    if let Some(mut stderr) = child.stderr.take() {
        let tail_for_thread = Arc::clone(&stderr_tail);
        let secret_for_thread = secret.clone();
        let _ = std::thread::Builder::new()
            .name("chromaticity-ffmpeg-stderr".into())
            .spawn(move || {
                let mut buf = [0u8; 1024];
                loop {
                    match stderr.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let chunk = String::from_utf8_lossy(&buf[..n]);
                            let redacted = secret_for_thread.redact_in_text(&chunk);
                            let mut guard = match tail_for_thread.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            guard.push_str(&redacted);
                            if guard.len() > MAX_STDERR_CAPTURE_BYTES {
                                let mut drain_to = guard.len() - MAX_STDERR_CAPTURE_BYTES;
                                while !guard.is_char_boundary(drain_to) && drain_to < guard.len() {
                                    drain_to += 1;
                                }
                                guard.drain(..drain_to);
                            }
                        }
                    }
                }
            });
    }

    streamer
        .on_connected(current_unix_ms())
        .map_err(|err| secret.redact_in_text(&err.to_string()))?;

    proc_guard.protocol = config.protocol;
    proc_guard.ingest_url = config.ingest_url;
    proc_guard.bitrate_kbps = target_bitrate;
    proc_guard.width = target_width;
    proc_guard.height = target_height;
    proc_guard.fps = target_fps;
    proc_guard.frames_pushed = 0;
    proc_guard.encoder = selected_video_encoder().to_owned();
    proc_guard.last_error = None;
    proc_guard.stderr_tail = stderr_tail;
    proc_guard.stdin = Some(stdin);
    proc_guard.child = Some(child);
    proc_guard.streamer = Some(streamer);

    Ok(proc_guard.status_dto())
}

/// Inner implementation for [`stop_twitch_stream`].
pub fn stop_twitch_stream_impl(app: &AppState) -> Result<StreamStatusDto, String> {
    let mut proc_guard = match app.twitch_stream.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    proc_guard.stop();
    Ok(proc_guard.status_dto())
}

/// Inner implementation for [`get_stream_status`].
pub fn get_stream_status_impl(app: &AppState) -> Result<StreamStatusDto, String> {
    let mut proc_guard = match app.twitch_stream.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    proc_guard.poll_child_health();
    Ok(proc_guard.status_dto())
}

/// Inner implementation for [`push_stream_frame`].
pub fn push_stream_frame_impl(app: &AppState, rgba_bytes: &[u8]) -> Result<u64, String> {
    let mut proc_guard = match app.twitch_stream.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    proc_guard.poll_child_health();

    let expected_len = (proc_guard.width as usize)
        .checked_mul(proc_guard.height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| "configured stream dimensions overflow frame byte size".to_owned())?;

    if rgba_bytes.len() != expected_len {
        return Err(format!(
            "RGBA frame byte length mismatch: expected {expected_len} bytes for {}x{}, got {}",
            proc_guard.width,
            proc_guard.height,
            rgba_bytes.len()
        ));
    }

    let write_result = match proc_guard.stdin.as_mut() {
        Some(stdin) => stdin.write_all(rgba_bytes),
        None => {
            return Err("cannot push stream frame while stream is not live".to_owned());
        }
    };

    if let Err(err) = write_result {
        proc_guard.terminate_child();
        let stderr_excerpt = proc_guard.read_stderr_tail();
        let raw_msg = if stderr_excerpt.is_empty() {
            format!("failed to write RGBA frame to ffmpeg stdin: {err}")
        } else {
            format!("failed to write RGBA frame to ffmpeg stdin: {err} ({stderr_excerpt})")
        };
        let redacted = proc_guard.redact_message(&raw_msg);
        proc_guard.last_error = Some(redacted.clone());
        if let Some(streamer) = proc_guard.streamer.as_mut() {
            let _ = streamer.on_transport_error(current_unix_ms(), &redacted);
        }
        return Err(redacted);
    }

    proc_guard.frames_pushed = proc_guard.frames_pushed.saturating_add(1);
    Ok(proc_guard.frames_pushed)
}

/// Start a live Twitch RTMPS/RTMP broadcast stream via external `ffmpeg`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn start_twitch_stream(
    app_state: State<'_, AppState>,
    ingest_url: String,
    stream_key: String,
    bitrate_kbps: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<u32>,
    audio_device: Option<String>,
) -> Result<StreamStatusDto, String> {
    start_twitch_stream_impl(
        app_state.inner(),
        ingest_url,
        stream_key,
        bitrate_kbps,
        width,
        height,
        fps,
        audio_device,
    )
}

/// Stop the active Twitch broadcast stream and reap any running `ffmpeg` child.
#[tauri::command]
pub fn stop_twitch_stream(app_state: State<'_, AppState>) -> Result<StreamStatusDto, String> {
    stop_twitch_stream_impl(app_state.inner())
}

/// Return the current live broadcast status snapshot (stream key is never included).
#[tauri::command]
pub fn get_stream_status(app_state: State<'_, AppState>) -> Result<StreamStatusDto, String> {
    get_stream_status_impl(app_state.inner())
}

/// Pipe one raw RGBA frame (`width * height * 4` bytes) into the active encoder
/// via Tauri v2 raw binary IPC (`InvokeBody::Raw`), avoiding JSON serialization.
#[tauri::command]
pub fn push_stream_frame(
    app_state: State<'_, AppState>,
    request: tauri::ipc::Request<'_>,
) -> Result<u64, String> {
    match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => push_stream_frame_impl(app_state.inner(), bytes),
        tauri::ipc::InvokeBody::Json(_) => {
            Err("push_stream_frame requires raw binary Uint8Array/ArrayBuffer body".to_owned())
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-NET-HARDENING (T1) — validates Twitch RTMPS endpoint
    //! validation, ffmpeg VideoToolbox/FLV argument construction, stream-key
    //! redaction on error outputs, and safe idle status/stop behavior.

    use super::*;
    use chromaticity_runtime::broadcast_stream::REDACTED_SECRET_PLACEHOLDER;

    #[test]
    fn endpoint_config_rejects_empty_key_and_invalid_scheme_and_accepts_twitch_rtmps() {
        let empty_key_err = build_validated_endpoint_config(
            "rtmps://live.twitch.tv/app",
            StreamSecretKey::new("   "),
            6_000,
        )
        .unwrap_err();
        assert!(
            empty_key_err.contains("stream key"),
            "unexpected error for empty key: {empty_key_err}"
        );

        let bad_scheme_err = build_validated_endpoint_config(
            "http://live.twitch.tv/app",
            StreamSecretKey::new("live_12345_secret"),
            6_000,
        )
        .unwrap_err();
        assert!(
            bad_scheme_err.contains("rtmps://"),
            "unexpected error for bad scheme: {bad_scheme_err}"
        );
        assert!(
            !bad_scheme_err.contains("live_12345_secret"),
            "secret key leaked into validation error"
        );

        let valid = build_validated_endpoint_config(
            "rtmps://live.twitch.tv/app",
            StreamSecretKey::new("live_12345_secret"),
            6_000,
        )
        .unwrap();
        assert_eq!(valid.protocol, StreamProtocol::Rtmps);
        assert_eq!(valid.ingest_url, "rtmps://live.twitch.tv/app");
        assert_eq!(valid.target_bitrate_kbps, 6_000);
        assert_eq!(
            format!("{:?}", valid.stream_key),
            REDACTED_SECRET_PLACEHOLDER
        );
    }

    #[test]
    fn build_twitch_ffmpeg_args_configures_videotoolbox_flv_and_redacts_errors() {
        let secret_raw = "live_987654321_superSecretKey";
        let secret = StreamSecretKey::new(secret_raw);
        let args = build_twitch_ffmpeg_args(
            "rtmps://live.twitch.tv/app/",
            &secret,
            6_000,
            1920,
            1080,
            60,
            None,
        )
        .unwrap();

        assert!(args.windows(2).any(|w| w == ["-f", "rawvideo"]));
        assert!(args.windows(2).any(|w| w == ["-pix_fmt", "rgba"]));
        assert!(args.windows(2).any(|w| w == ["-s", "1920x1080"]));
        assert!(args.windows(2).any(|w| w == ["-r", "60"]));
        assert!(args.windows(2).any(|w| w == ["-i", "pipe:0"]));
        assert!(args.windows(2).any(|w| w == ["-b:v", "6000k"]));
        assert!(args.windows(2).any(|w| w == ["-maxrate", "6000k"]));
        assert!(args.windows(2).any(|w| w == ["-bufsize", "12000k"]));
        assert!(args.windows(2).any(|w| w == ["-g", "120"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|w| w == ["-b:a", "160k"]));
        assert!(args.windows(2).any(|w| w == ["-ar", "44100"]));
        assert!(args.windows(2).any(|w| w == ["-f", "flv"]));

        #[cfg(target_os = "macos")]
        {
            assert!(args.windows(2).any(|w| w == ["-c:v", "h264_videotoolbox"]));
            assert!(args.windows(2).any(|w| w == ["-realtime", "true"]));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        }

        // Trailing slash on ingest URL must be trimmed before appending the key.
        assert_eq!(
            args.last().map(String::as_str),
            Some("rtmps://live.twitch.tv/app/live_987654321_superSecretKey")
        );

        // Simulated ffmpeg error line containing the full destination URL must scrub the key.
        let raw_stderr = format!("rtmps://live.twitch.tv/app/{secret_raw}: Input/output error");
        let scrubbed = secret.redact_in_text(&raw_stderr);
        assert!(!scrubbed.contains(secret_raw));
        assert!(scrubbed.contains(REDACTED_SECRET_PLACEHOLDER));
    }

    #[test]
    fn get_stream_status_and_stop_twitch_stream_are_safe_when_idle() {
        let app = AppState::new();

        let status = get_stream_status_impl(&app).unwrap();
        assert_eq!(status.state, "offline");
        assert!(!status.is_live);
        assert_eq!(status.protocol, "rtmps");
        assert_eq!(status.frames_pushed, 0);
        assert_eq!(status.encoder, selected_video_encoder());
        assert!(status.last_error.is_none());

        let stopped = stop_twitch_stream_impl(&app).unwrap();
        assert_eq!(stopped.state, "offline");
        assert!(!stopped.is_live);
        assert_eq!(stopped.frames_pushed, 0);

        // Pushing a frame while idle fails cleanly without panicking.
        let frame_bytes = vec![0u8; (status.width * status.height * 4) as usize];
        let push_err = push_stream_frame_impl(&app, &frame_bytes).unwrap_err();
        assert!(push_err.contains("not live"));
    }

    #[test]
    fn build_twitch_ffmpeg_args_handles_audio_device_and_rejects_unsafe_values() {
        let secret_raw = "live_audio_test_secret_key";
        let secret = StreamSecretKey::new(secret_raw);

        // None and whitespace-only fall back to silent anullsrc.
        for empty_opt in [None, Some(""), Some("   ")] {
            let args = build_twitch_ffmpeg_args(
                "rtmps://live.twitch.tv/app",
                &secret,
                6_000,
                1280,
                720,
                30,
                empty_opt,
            )
            .unwrap();
            assert!(args.windows(2).any(|w| w == ["-f", "lavfi"]));
            assert!(
                args.windows(2)
                    .any(|w| w == ["-i", "anullsrc=channel_layout=stereo:sample_rate=44100"])
            );
            assert!(!args.iter().any(|a| a == "avfoundation"));
        }

        // Valid audio device selects avfoundation on macOS (and anullsrc elsewhere).
        let with_device = build_twitch_ffmpeg_args(
            "rtmps://live.twitch.tv/app",
            &secret,
            6_000,
            1280,
            720,
            30,
            Some("  :0  "),
        )
        .unwrap();
        #[cfg(target_os = "macos")]
        {
            assert!(with_device.windows(2).any(|w| w == ["-f", "avfoundation"]));
            assert!(with_device.windows(2).any(|w| w == ["-i", ":0"]));
            assert!(!with_device.iter().any(|a| a.contains("anullsrc")));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(with_device.windows(2).any(|w| w == ["-f", "lavfi"]));
        }

        // Named device with spaces, hyphens, and underscores is accepted.
        let named = build_twitch_ffmpeg_args(
            "rtmps://live.twitch.tv/app",
            &secret,
            6_000,
            1280,
            720,
            30,
            Some("BlackHole 2ch_main-1"),
        )
        .unwrap();
        #[cfg(target_os = "macos")]
        {
            assert!(
                named
                    .windows(2)
                    .any(|w| w == ["-i", "BlackHole 2ch_main-1"])
            );
        }

        // Unsafe device strings are rejected cleanly without leaking secrets.
        let too_long = "a".repeat(MAX_AUDIO_DEVICE_LEN + 1);
        for unsafe_dev in ["-f", "-i", "--evil", ":0;rm -rf /", "dev|pipe", &too_long] {
            let err = build_twitch_ffmpeg_args(
                "rtmps://live.twitch.tv/app",
                &secret,
                6_000,
                1280,
                720,
                30,
                Some(unsafe_dev),
            )
            .unwrap_err();
            assert!(!err.contains(secret_raw), "secret leaked into error: {err}");
        }
    }
}
