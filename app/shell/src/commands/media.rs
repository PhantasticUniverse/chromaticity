// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene media (`chroma_Media`) ingest IPC.
//!
//! The webview samples the selected media source (webcam, screen, local
//! file, test card) to RGBA8 and pushes it here with
//! [`push_scene_media_frame`]. The command only swaps the frame into
//! [`crate::state::AppState::scene_media`]; the GPU upload happens on the
//! render path (`render_gpu_frame` → `GpuPreviewEngine::sync_media`) so
//! IPC never blocks on the GPU.
//!
//! Wire format (raw `InvokeBody`): 12-byte little-endian header
//! `[width u32][height u32][kind u32]` (`kind`: 1 image, 2 video, 3 webcam)
//! followed by exactly `width * height * 4` RGBA8 bytes, row 0 = top.

use std::sync::Arc;

use chromaticity_render::MediaKind;
use tauri::State;

use crate::render::SceneMediaFrame;
use crate::state::AppState;

/// Largest accepted scene-media frame (pixels per edge). The webview
/// producer downsamples to at most 640x360; this bounds a hostile push.
pub const MAX_SCENE_MEDIA_WIDTH: u32 = 1920;
/// See [`MAX_SCENE_MEDIA_WIDTH`].
pub const MAX_SCENE_MEDIA_HEIGHT: u32 = 1080;

const HEADER_LEN: usize = 12;

/// Latest scene-media frame plus a version counter bumped on every push
/// and clear, so the renderer knows when to re-upload.
#[derive(Debug, Default)]
pub struct SceneMediaSlot {
    /// Monotonic version (0 = never touched).
    pub seq: u64,
    /// Current frame, `None` when no media is selected.
    pub frame: Option<Arc<SceneMediaFrame>>,
}

fn parse_frame(body: &[u8]) -> Result<SceneMediaFrame, String> {
    if body.len() < HEADER_LEN {
        return Err("scene media payload shorter than its 12-byte header".into());
    }
    let word = |i: usize| -> u32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&body[i * 4..i * 4 + 4]);
        u32::from_le_bytes(b)
    };
    let (width, height, kind_code) = (word(0), word(1), word(2));
    if width == 0 || height == 0 || width > MAX_SCENE_MEDIA_WIDTH || height > MAX_SCENE_MEDIA_HEIGHT
    {
        return Err(format!(
            "scene media frame {width}x{height} outside 1..=1920x1080"
        ));
    }
    let kind = match kind_code {
        1 => MediaKind::Image,
        2 => MediaKind::Video,
        3 => MediaKind::Webcam,
        other => return Err(format!("unknown scene media kind {other}")),
    };
    let expected = width as usize * height as usize * 4;
    let rgba = &body[HEADER_LEN..];
    if rgba.len() != expected {
        return Err(format!(
            "scene media frame {width}x{height} needs {expected} RGBA bytes, got {}",
            rgba.len()
        ));
    }
    Ok(SceneMediaFrame {
        width,
        height,
        kind,
        rgba: rgba.to_vec(),
    })
}

/// Store a pushed frame as the current scene media; returns the new slot version.
///
/// # Errors
///
/// Malformed header, out-of-range size, unknown kind, or length mismatch.
pub fn push_scene_media_frame_impl(app: &AppState, body: &[u8]) -> Result<u64, String> {
    let frame = Arc::new(parse_frame(body)?);
    let mut slot = match app.scene_media.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    slot.seq = slot.seq.wrapping_add(1).max(1);
    slot.frame = Some(frame);
    Ok(slot.seq)
}

/// Clear the scene media (shaders see `_isMediaActive() == false`).
pub fn clear_scene_media_impl(app: &AppState) -> u64 {
    let mut slot = match app.scene_media.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    slot.seq = slot.seq.wrapping_add(1).max(1);
    slot.frame = None;
    slot.seq
}

/// Snapshot `(seq, frame)` for the render path.
pub fn scene_media_snapshot(app: &AppState) -> (u64, Option<Arc<SceneMediaFrame>>) {
    let slot = match app.scene_media.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    (slot.seq, slot.frame.clone())
}

/// Push one RGBA8 scene-media frame (raw body; see module docs).
#[tauri::command]
pub fn push_scene_media_frame(
    app_state: State<'_, AppState>,
    request: tauri::ipc::Request<'_>,
) -> Result<u64, String> {
    match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => push_scene_media_frame_impl(app_state.inner(), bytes),
        tauri::ipc::InvokeBody::Json(_) => {
            Err("push_scene_media_frame expects a raw binary body".into())
        }
    }
}

/// Clear the scene media.
#[tauri::command]
pub fn clear_scene_media(app_state: State<'_, AppState>) -> u64 {
    clear_scene_media_impl(app_state.inner())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    use super::*;

    fn payload(w: u32, h: u32, kind: u32, rgba: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&w.to_le_bytes());
        v.extend_from_slice(&h.to_le_bytes());
        v.extend_from_slice(&kind.to_le_bytes());
        v.extend_from_slice(rgba);
        v
    }

    #[test]
    fn parse_accepts_exact_frames_and_rejects_malformed_ones() {
        let ok = parse_frame(&payload(2, 1, 3, &[1, 2, 3, 4, 5, 6, 7, 8])).expect("valid");
        assert_eq!((ok.width, ok.height, ok.kind), (2, 1, MediaKind::Webcam));
        assert_eq!(ok.rgba, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(parse_frame(&[0; 8]).is_err(), "short header");
        assert!(
            parse_frame(&payload(2, 1, 3, &[0; 7])).is_err(),
            "length mismatch"
        );
        assert!(parse_frame(&payload(0, 1, 1, &[])).is_err(), "zero width");
        assert!(
            parse_frame(&payload(1921, 1, 1, &vec![0; 1921 * 4])).is_err(),
            "too wide"
        );
        assert!(
            parse_frame(&payload(1, 1, 9, &[0; 4])).is_err(),
            "unknown kind"
        );
    }

    #[test]
    fn pushed_frame_reaches_the_gpu_engine_via_the_render_path_snapshot() {
        let app = AppState::new();
        assert_eq!(scene_media_snapshot(&app).0, 0, "untouched slot");
        let seq =
            push_scene_media_frame_impl(&app, &payload(1, 1, 2, &[10, 20, 30, 255])).expect("push");
        assert!(push_scene_media_frame_impl(&app, &payload(1, 1, 2, &[0; 3])).is_err());

        // Same sequence render_gpu_frame runs: snapshot, sync, render.
        let mut engine = crate::render::GpuPreviewEngine::new();
        engine.set_inline_shader(
            "media".into(),
            "#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0, std140) uniform U { vec4 chroma_Time; };
void main() { fragColor = _textureMedia(uv) + 0.0 * chroma_Time; }
"
            .into(),
        );
        let (snap_seq, frame) = scene_media_snapshot(&app);
        assert_eq!(snap_seq, seq);
        engine.sync_media(snap_seq, frame.as_deref()).expect("sync");
        let audio = chromaticity_audio::AudioUniformSnapshot::default();
        let bytes = engine
            .render_frame(16, 16, 0.0, 1, &audio, [0.5; 16])
            .expect("render");
        assert_eq!(&bytes[0..4], &[10, 20, 30, 255]);

        let cleared = clear_scene_media_impl(&app);
        assert!(cleared > seq);
        let (snap_seq, frame) = scene_media_snapshot(&app);
        assert!(frame.is_none());
        engine
            .sync_media(snap_seq, frame.as_deref())
            .expect("clear");
        let bytes = engine
            .render_frame(16, 16, 0.0, 2, &audio, [0.5; 16])
            .expect("render");
        assert_eq!(&bytes[0..4], &[0, 0, 0, 255]);
    }
}
