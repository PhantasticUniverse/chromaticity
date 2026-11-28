// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase 7a crash-safe fMP4 recorder and rolling replay buffer IPC handlers.
//!
//! Exposes [`start_recording`], [`stop_recording`], [`get_recorder_status`], and
//! [`save_replay_buffer`] backed by [`chromaticity_runtime::recorder`]
//! (`Fmp4Muxer`, `Fmp4MuxerConfig`, `ReplayBuffer`, `RecorderState`) managed on
//! [`AppState`].

#![forbid(unsafe_code)]

use crate::state::AppState;
use chromaticity_runtime::recorder::{
    DEFAULT_SAMPLE_DURATION_90KHZ, EncodedPacket, Fmp4Muxer, Fmp4MuxerConfig, RecorderState,
    ReplayBuffer, TIMESCALE_90KHZ,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};

/// Subdirectory under `app.path().app_data_dir()` where default replay files are saved.
const RECORDINGS_SUBDIR: &str = "recordings";

/// Open on-disk fMP4 file backing an active recording session.
#[derive(Debug)]
pub struct ActiveRecordingFile {
    pub path: PathBuf,
    pub file: std::fs::File,
}

/// Monotonic counter used to disambiguate atomic temporary files within a process.
static TMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Serializable recorder and rolling replay-buffer status snapshot returned across IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecorderStatusDto {
    /// Explicit lifecycle state label: `"idle"`, `"recording"`, or `"stopping"`.
    pub state: String,
    /// `true` iff the recorder is actively in the `"recording"` state.
    pub is_recording: bool,
    /// Active or configured video width in pixels.
    pub width: u16,
    /// Active or configured video height in pixels.
    pub height: u16,
    /// Recording session start timestamp in Unix milliseconds (`None` when not recording).
    pub started_ms: Option<u64>,
    /// Complete `moof` + `mdat` fragments written in the active session.
    pub fragments_written: u64,
    /// Total fMP4 bytes written in the active session (init segment + fragments).
    pub bytes_written: u64,
    /// Number of keyframe-aligned GOP segments currently retained in the replay buffer.
    pub replay_gop_count: usize,
    /// Total encoded packets currently retained in the replay buffer.
    pub replay_packet_count: usize,
    /// Total compressed payload bytes currently retained in the replay buffer.
    pub replay_retained_bytes: usize,
    /// Retained replay buffer duration in whole seconds.
    pub replay_retained_secs: u64,
    /// Configured rolling replay buffer duration ceiling in seconds.
    pub replay_max_duration_secs: u32,
}

/// Serializable result returned by [`save_replay_buffer`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaySaveResultDto {
    /// Absolute filesystem path of the written standalone fMP4 file.
    pub path: String,
    /// Total bytes written to disk (`ftyp` + `moov` + `moof`/`mdat` fragments).
    pub bytes_written: usize,
    /// Number of GOP segments flushed into the file.
    pub gop_count: usize,
    /// Number of encoded video packets flushed into the file.
    pub packet_count: usize,
    /// Total retained duration of the saved replay in 90 kHz ticks.
    pub duration_90khz: u64,
}

fn current_unix_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(dur) => u64::try_from(dur.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

fn current_unix_secs() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(dur) => dur.as_secs(),
        Err(_) => 0,
    }
}

fn validate_dimension(label: &str, value: Option<u32>, default_val: u16) -> Result<u16, String> {
    match value {
        None => Ok(default_val),
        Some(0) => Err(format!("recorder {label} must be greater than zero")),
        Some(v) => u16::try_from(v).map_err(|_| {
            format!(
                "recorder {label} ({v}) exceeds ISO BMFF maximum ({})",
                u16::MAX
            )
        }),
    }
}

fn build_status_dto(muxer: Option<&Fmp4Muxer>, replay: &ReplayBuffer) -> RecorderStatusDto {
    let default_cfg = Fmp4MuxerConfig::default();
    let (state, width, height) = match muxer {
        Some(m) => (m.state(), m.config().width, m.config().height),
        None => (RecorderState::Idle, default_cfg.width, default_cfg.height),
    };

    let (started_ms, fragments_written, bytes_written) = match state {
        RecorderState::Recording {
            started_ms,
            fragments_written,
            bytes_written,
        } => (Some(started_ms), fragments_written, bytes_written),
        RecorderState::Idle | RecorderState::Stopping => (None, 0, 0),
    };

    RecorderStatusDto {
        state: state.label().to_string(),
        is_recording: matches!(state, RecorderState::Recording { .. }),
        width,
        height,
        started_ms,
        fragments_written,
        bytes_written,
        replay_gop_count: replay.gop_count(),
        replay_packet_count: replay.packet_count(),
        replay_retained_bytes: replay.retained_bytes(),
        replay_retained_secs: replay.retained_duration_90khz() / u64::from(TIMESCALE_90KHZ),
        replay_max_duration_secs: replay.max_duration_secs(),
    }
}

/// Validate a caller-supplied output path or construct the default
/// `<default_data_dir>/recordings/replay-<ts>.mp4` destination.
fn resolve_replay_output_path(
    default_data_dir: &Path,
    output_path: Option<String>,
) -> Result<PathBuf, String> {
    let Some(raw) = output_path else {
        let ts = current_unix_secs();
        let filename = format!("replay-{ts}.mp4");
        return Ok(default_data_dir.join(RECORDINGS_SUBDIR).join(filename));
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("replay output path must not be empty".to_string());
    }
    if trimmed.contains('\0') {
        return Err("replay output path must not contain NUL bytes".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("replay output path must be an absolute path".to_string());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("replay output path must not contain parent-directory traversal (`..`)".into());
    }

    let is_mp4 = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"));
    if !is_mp4 || path.file_name().is_none() {
        return Err("replay output path must specify a `.mp4` filename".to_string());
    }

    Ok(path)
}

fn write_bytes_atomically(target_path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = target_path.parent().ok_or_else(|| {
        format!(
            "replay output path {} has no parent directory",
            target_path.display()
        )
    })?;

    std::fs::create_dir_all(parent).map_err(|err| {
        format!(
            "failed to create recordings directory {}: {err}",
            parent.display()
        )
    })?;

    let seq = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_name = format!(".replay.{}.{seq}.tmp", std::process::id());
    let tmp_path = parent.join(tmp_name);

    let write_result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "failed to write temporary replay file {}: {err}",
            tmp_path.display()
        ));
    }

    if let Err(err) = std::fs::rename(&tmp_path, target_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "failed to commit replay file {}: {err}",
            target_path.display()
        ));
    }

    Ok(())
}

/// Inner implementation for [`start_recording`], optionally backing the session
/// with a real on-disk `.mp4` file under `data_dir/recordings/`.
pub fn start_recording_to_dir_impl(
    app: &AppState,
    data_dir: Option<&Path>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<RecorderStatusDto, String> {
    let default_cfg = Fmp4MuxerConfig::default();
    let resolved_width = validate_dimension("width", width, default_cfg.width)?;
    let resolved_height = validate_dimension("height", height, default_cfg.height)?;

    let mut muxer_guard = match app.recorder_muxer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    if let Some(existing) = muxer_guard.as_ref() {
        if !matches!(existing.state(), RecorderState::Idle) {
            return Err(format!(
                "invalid recorder state transition: cannot `start` from `{}`",
                existing.state().label()
            ));
        }
    }

    let config = Fmp4MuxerConfig {
        width: resolved_width,
        height: resolved_height,
        ..default_cfg
    };
    let mut muxer = Fmp4Muxer::new(config).map_err(|err| err.to_string())?;
    let init_segment = muxer
        .start(current_unix_ms())
        .map_err(|err| err.to_string())?;

    if let Some(base_dir) = data_dir {
        let dir = base_dir.join(RECORDINGS_SUBDIR);
        std::fs::create_dir_all(&dir).map_err(|err| {
            format!(
                "failed to create recordings directory {}: {err}",
                dir.display()
            )
        })?;
        let seq = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let target_path = dir.join(format!("recording-{}-{seq}.mp4", current_unix_secs()));
        let mut file = std::fs::File::create(&target_path).map_err(|err| {
            format!(
                "failed to create recording file {}: {err}",
                target_path.display()
            )
        })?;
        file.write_all(&init_segment).map_err(|err| {
            format!(
                "failed to write fMP4 init segment to {}: {err}",
                target_path.display()
            )
        })?;
        file.flush().map_err(|err| {
            format!(
                "failed to flush fMP4 init segment to {}: {err}",
                target_path.display()
            )
        })?;
        let mut file_guard = match app.recorder_output_file.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *file_guard = Some(ActiveRecordingFile {
            path: target_path,
            file,
        });
    }

    *muxer_guard = Some(muxer);

    let replay_guard = match app.replay_buffer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(build_status_dto(muxer_guard.as_ref(), &replay_guard))
}

/// Inner implementation for [`start_recording`] (in-memory muxer without disk path).
pub fn start_recording_impl(
    app: &AppState,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<RecorderStatusDto, String> {
    start_recording_to_dir_impl(app, None, width, height)
}

/// Inner implementation for [`stop_recording`].
pub fn stop_recording_impl(app: &AppState) -> Result<RecorderStatusDto, String> {
    let mut muxer_guard = match app.recorder_muxer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    let muxer = muxer_guard.as_mut().ok_or_else(|| {
        "invalid recorder state transition: cannot `stop` from `idle`".to_string()
    })?;
    muxer.begin_stop().map_err(|err| err.to_string())?;
    let final_fragment = muxer.finish_stop().map_err(|err| err.to_string())?;

    let mut file_guard = match app.recorder_output_file.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(mut active_file) = file_guard.take() {
        if let Some(fragment_bytes) = final_fragment {
            if !fragment_bytes.is_empty() {
                active_file.file.write_all(&fragment_bytes).map_err(|err| {
                    format!(
                        "failed to write final fMP4 fragment to {}: {err}",
                        active_file.path.display()
                    )
                })?;
            }
        }
        let _ = active_file.file.sync_all();
    }

    let replay_guard = match app.replay_buffer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(build_status_dto(muxer_guard.as_ref(), &replay_guard))
}

/// Inner implementation for [`get_recorder_status`].
pub fn get_recorder_status_impl(app: &AppState) -> Result<RecorderStatusDto, String> {
    let muxer_guard = match app.recorder_muxer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let replay_guard = match app.replay_buffer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(build_status_dto(muxer_guard.as_ref(), &replay_guard))
}

/// Inner implementation for [`save_replay_buffer`].
pub fn save_replay_buffer_impl(
    app: &AppState,
    default_data_dir: &Path,
    output_path: Option<String>,
) -> Result<ReplaySaveResultDto, String> {
    let target_path = resolve_replay_output_path(default_data_dir, output_path)?;

    let muxer_config = {
        let muxer_guard = match app.recorder_muxer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        muxer_guard
            .as_ref()
            .map_or_else(Fmp4MuxerConfig::default, |m| m.config().clone())
    };

    let (fmp4_bytes, gop_count, packet_count, duration_90khz) = {
        let replay_guard = match app.replay_buffer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let bytes = replay_guard.flush_to_fmp4(&muxer_config);
        (
            bytes,
            replay_guard.gop_count(),
            replay_guard.packet_count(),
            replay_guard.retained_duration_90khz(),
        )
    };

    if fmp4_bytes.is_empty() {
        return Err("replay buffer is empty: no keyframes available to save".to_string());
    }

    write_bytes_atomically(&target_path, &fmp4_bytes)?;

    Ok(ReplaySaveResultDto {
        path: target_path.to_string_lossy().into_owned(),
        bytes_written: fmp4_bytes.len(),
        gop_count,
        packet_count,
        duration_90khz,
    })
}

/// Start a crash-safe Fragmented MP4 recording session.
#[tauri::command]
pub fn start_recording(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<RecorderStatusDto, String> {
    let data_dir = app.path().app_data_dir().ok();
    start_recording_to_dir_impl(app_state.inner(), data_dir.as_deref(), width, height)
}

/// Stop the active Fragmented MP4 recording session and flush any buffered GOP.
#[tauri::command]
pub fn stop_recording(app_state: State<'_, AppState>) -> Result<RecorderStatusDto, String> {
    stop_recording_impl(app_state.inner())
}

/// Return the current recorder lifecycle and rolling replay-buffer status snapshot.
#[tauri::command]
pub fn get_recorder_status(app_state: State<'_, AppState>) -> Result<RecorderStatusDto, String> {
    get_recorder_status_impl(app_state.inner())
}

/// Push one encoded frame payload into the rolling replay buffer and, when
/// actively recording, into the fMP4 muxer (flushing completed `moof` + `mdat`
/// fragments straight to the active on-disk `.mp4` file).
pub fn push_recorder_packet_impl(
    app: &AppState,
    payload: Vec<u8>,
    is_keyframe: bool,
    pts_90khz: u64,
) -> Result<RecorderStatusDto, String> {
    if payload.is_empty() {
        return Err("recorder packet payload must not be empty".into());
    }
    if payload.len() >= 2 && payload[0] == 0xFF && payload[1] == 0xD8 {
        return Err(
            "recorder requires H.264 NAL units (avc1); raw JPEG frames cannot be muxed into fMP4"
                .into(),
        );
    }
    let packet = EncodedPacket {
        pts_90khz,
        dts_90khz: pts_90khz,
        duration_90khz: DEFAULT_SAMPLE_DURATION_90KHZ,
        is_keyframe,
        data: payload,
    };

    let mut replay_guard = match app.replay_buffer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    replay_guard.push_packet(packet.clone());

    let mut muxer_guard = match app.recorder_muxer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(muxer) = muxer_guard.as_mut() {
        if matches!(muxer.state(), RecorderState::Recording { .. }) {
            if let Some(fragment) = muxer.push_packet(packet).map_err(|err| err.to_string())? {
                let mut file_guard = match app.recorder_output_file.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                if let Some(active) = file_guard.as_mut() {
                    active.file.write_all(&fragment).map_err(|err| {
                        format!(
                            "failed to write fMP4 fragment to {}: {err}",
                            active.path.display()
                        )
                    })?;
                    active.file.flush().map_err(|err| {
                        format!(
                            "failed to flush fMP4 fragment to {}: {err}",
                            active.path.display()
                        )
                    })?;
                }
            }
        }
    }

    Ok(build_status_dto(muxer_guard.as_ref(), &replay_guard))
}

/// Push an encoded video frame into the live fMP4 recorder and rolling replay buffer.
#[tauri::command]
pub fn push_recorder_frame(
    app_state: State<'_, AppState>,
    payload: Vec<u8>,
    is_keyframe: bool,
    pts_90khz: u64,
) -> Result<RecorderStatusDto, String> {
    push_recorder_packet_impl(app_state.inner(), payload, is_keyframe, pts_90khz)
}

/// Flush the rolling replay buffer into a standalone playable Fragmented MP4 file.
///
/// Writes to `<app_data_dir>/recordings/replay-<ts>.mp4` when `output_path` is `None`,
/// or validates and atomically writes to the caller-supplied absolute `.mp4` path.
#[tauri::command]
pub fn save_replay_buffer(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    output_path: Option<String>,
) -> Result<ReplaySaveResultDto, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    save_replay_buffer_impl(app_state.inner(), &data_dir, output_path)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-DETERMINISM (T1) — Verifies the Phase 7a shell recorder
    //! lifecycle (`start_recording -> get_recorder_status -> stop_recording`),
    //! safe path validation, and standalone fMP4 replay-buffer disk flush.

    use super::*;
    use chromaticity_runtime::recorder::{
        DEFAULT_SAMPLE_DURATION_90KHZ, EncodedPacket, parse_box_headers,
    };

    fn make_temp_dir(label: &str) -> PathBuf {
        let seq = TMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "chromaticity-recorder-test-{label}-{}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn seed_replay_buffer(app: &AppState) {
        let mut replay = app.replay_buffer.lock().expect("lock replay buffer");
        replay.push_packet(EncodedPacket {
            pts_90khz: 90_000,
            dts_90khz: 90_000,
            duration_90khz: DEFAULT_SAMPLE_DURATION_90KHZ,
            is_keyframe: true,
            data: vec![0x00, 0x00, 0x00, 0x04, 0x65, 0x88, 0x84, 0x21],
        });
        replay.push_packet(EncodedPacket {
            pts_90khz: 91_500,
            dts_90khz: 91_500,
            duration_90khz: DEFAULT_SAMPLE_DURATION_90KHZ,
            is_keyframe: false,
            data: vec![0x00, 0x00, 0x00, 0x03, 0x41, 0x9A, 0x01],
        });
        replay.push_packet(EncodedPacket {
            pts_90khz: 93_000,
            dts_90khz: 93_000,
            duration_90khz: DEFAULT_SAMPLE_DURATION_90KHZ,
            is_keyframe: true,
            data: vec![0x00, 0x00, 0x00, 0x04, 0x65, 0x88, 0x99, 0x10],
        });
    }

    #[test]
    fn start_status_stop_recording_lifecycle_round_trip() {
        let app = AppState::new();

        // Initially idle with default 1920x1080 dimensions.
        let idle = get_recorder_status_impl(&app).expect("initial status");
        assert_eq!(idle.state, "idle");
        assert!(!idle.is_recording);
        assert_eq!(idle.width, 1920);
        assert_eq!(idle.height, 1080);
        assert!(idle.started_ms.is_none());
        assert_eq!(idle.bytes_written, 0);

        // Stopping while idle must error.
        let stop_err = stop_recording_impl(&app).expect_err("stop while idle must fail");
        assert!(stop_err.contains("begin_stop"));

        // Invalid dimensions must be rejected cleanly before mutating state.
        let zero_err = start_recording_impl(&app, Some(0), Some(720))
            .expect_err("zero width must be rejected");
        assert!(zero_err.contains("greater than zero"));
        let huge_err = start_recording_impl(&app, Some(100_000), Some(720))
            .expect_err("overflow width must be rejected");
        assert!(huge_err.contains("exceeds ISO BMFF maximum"));

        // Start recording at 1280x720.
        let started = start_recording_impl(&app, Some(1280), Some(720)).expect("start recording");
        assert_eq!(started.state, "recording");
        assert!(started.is_recording);
        assert_eq!(started.width, 1280);
        assert_eq!(started.height, 720);
        assert!(started.started_ms.is_some());
        assert!(
            started.bytes_written > 0,
            "init segment bytes must be accounted for"
        );

        // get_recorder_status reflects the active recording session.
        let active = get_recorder_status_impl(&app).expect("active status");
        assert_eq!(active, started);
        assert_eq!(
            app.inspect_state_machines_snapshot()["recorder"],
            "recording"
        );

        // Starting again while already recording must fail.
        let double_start =
            start_recording_impl(&app, None, None).expect_err("double start must fail");
        assert!(double_start.contains("cannot `start` from `recording`"));

        // Stop recording transitions back to idle.
        let stopped = stop_recording_impl(&app).expect("stop recording");
        assert_eq!(stopped.state, "idle");
        assert!(!stopped.is_recording);
        assert_eq!(stopped.width, 1280);
        assert_eq!(stopped.height, 720);
        assert!(stopped.started_ms.is_none());
        assert_eq!(app.inspect_state_machines_snapshot()["recorder"], "idle");
    }

    #[test]
    fn save_replay_buffer_writes_standalone_fmp4_and_validates_paths() {
        let app = AppState::new();
        let dir = make_temp_dir("replay");

        // Empty buffer returns a clear error without creating a file.
        let empty_err =
            save_replay_buffer_impl(&app, &dir, None).expect_err("empty buffer must fail");
        assert!(empty_err.contains("replay buffer is empty"));
        assert!(!dir.join(RECORDINGS_SUBDIR).exists());

        // Seed two GOPs (3 packets) into the rolling replay buffer.
        seed_replay_buffer(&app);

        // Reject unsafe or malformed caller-supplied paths.
        assert!(
            save_replay_buffer_impl(&app, &dir, Some("relative/replay.mp4".into())).is_err(),
            "relative path must be rejected"
        );
        assert!(
            save_replay_buffer_impl(
                &app,
                &dir,
                Some(dir.join("../escape.mp4").to_string_lossy().into_owned()),
            )
            .is_err(),
            "parent traversal must be rejected"
        );
        assert!(
            save_replay_buffer_impl(
                &app,
                &dir,
                Some(dir.join("not-video.txt").to_string_lossy().into_owned()),
            )
            .is_err(),
            "non-mp4 extension must be rejected"
        );

        // Default path: `<app_data_dir>/recordings/replay-<ts>.mp4`.
        let default_res =
            save_replay_buffer_impl(&app, &dir, None).expect("save replay to default dir");
        let default_path = PathBuf::from(&default_res.path);
        assert!(default_path.is_file());
        assert_eq!(
            default_path.parent(),
            Some(dir.join(RECORDINGS_SUBDIR).as_path())
        );
        let file_name = default_path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 filename");
        assert!(file_name.starts_with("replay-") && file_name.ends_with(".mp4"));
        assert_eq!(default_res.gop_count, 2);
        assert_eq!(default_res.packet_count, 3);
        assert_eq!(
            default_res.duration_90khz,
            u64::from(DEFAULT_SAMPLE_DURATION_90KHZ) * 3
        );

        // Verify the saved file is a valid standalone ISO BMFF fMP4 (`ftyp`, `moov`, `moof`, `mdat`, ...).
        let bytes = std::fs::read(&default_path).expect("read saved replay");
        assert_eq!(bytes.len(), default_res.bytes_written);
        let boxes = parse_box_headers(&bytes).expect("valid ISO BMFF boxes");
        let box_types: Vec<[u8; 4]> = boxes.iter().map(|b| b.box_type).collect();
        assert_eq!(
            box_types,
            vec![*b"ftyp", *b"moov", *b"moof", *b"mdat", *b"moof", *b"mdat"]
        );

        // Explicit safe absolute `.mp4` path also succeeds.
        let explicit_target = dir.join("custom").join("drop-moment.mp4");
        let explicit_res = save_replay_buffer_impl(
            &app,
            &dir,
            Some(explicit_target.to_string_lossy().into_owned()),
        )
        .expect("save replay to explicit path");
        assert_eq!(PathBuf::from(&explicit_res.path), explicit_target);
        assert!(explicit_target.is_file());
        assert_eq!(explicit_res.bytes_written, default_res.bytes_written);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
