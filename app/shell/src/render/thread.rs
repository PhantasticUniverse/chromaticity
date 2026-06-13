// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Dedicated render thread for the ADR-011 Phase-2 Slice R-1a output
//! window.
//!
//! ## The macOS main-thread handoff (load-bearing)
//!
//! On macOS / Metal, `wgpu::Instance::create_surface` and window
//! creation MUST happen on the main thread — calling them off-thread is
//! a PANIC, not a recoverable error (see the `wgpu` 26
//! `SurfaceTarget::Window` doc: "On macOS/Metal: will panic if not
//! called on the main thread"). The Tauri `setup` callback runs ON the
//! main thread, so [`crate::render::output_window`] creates the window,
//! the surface, and acquires the adapter / device there. The resulting
//! `wgpu::Surface<'static>` is `Send + Sync` (it owns the
//! `Arc<Window>` handle), so this module's [`spawn`] MOVES it —
//! together with the already-built [`MultipassRenderer`] and
//! [`BlitPass`] — into a dedicated render thread. Nothing in this
//! module ever calls `create_surface` or touches the window builder;
//! that boundary is the whole point of the slice.
//!
//! ## The loop
//!
//! Each iteration:
//! 1. checks the shutdown flag (set by [`RenderThreadHandle::drop`]),
//! 2. reads the window's current physical size and reconfigures the
//!    surface if it changed (resize handling),
//! 3. acquires the swapchain texture (reconfiguring on
//!    `Lost` / `Outdated`, retrying next frame on `Timeout`, exiting on
//!    `OutOfMemory` / `Other`),
//! 4. builds the live output scene's frame spec (Slice R-1b — via
//!    `from_scene_live`, or the R-1a test pattern as fallback) — driven
//!    by the LIVE audio snapshot so the scene reacts to sound (Slice
//!    R-1c; silent only when no audio source is present). The audio
//!    uniforms are ballistically smoothed per frame
//!    (instant-attack/slow-release PPM one-pole, dt-aware — see
//!    [`crate::render::audio_smooth`]) so reactive motion lands the
//!    same frame on transients and decays smoothly — then renders it
//!    via [`MultipassRenderer::render_frame_live`],
//! 5. blits that texture onto the swapchain view, and
//! 6. presents, then sleeps to pace at ~60 fps.
//!
//! The loop NEVER panics on a surface error — every error path is
//! handled (reconfigure / skip / graceful exit).
//!
//! ## No `unsafe`
//!
//! All `wgpu` 26 + `std::thread` API used here is safe. The workspace
//! `unsafe_code = "forbid"` lint is inherited.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use chromaticity_audio::AudioUniformSnapshot;
use chromaticity_render::scene_adapter::LiveUniforms;
use chromaticity_render::{MultipassRenderer, live_feedback_spec_key};
use tauri::Runtime;
use tauri::window::Window;

use crate::render::audio_smooth::AudioSmoother;
use crate::render::blit::BlitPass;
use crate::render::pattern::{PATTERN_SCENE_ID, test_pattern_spec};
use crate::render::scene_source::{LoadedScene, OUTPUT_SCENE_SPEC_ID, live_uniforms_from_audio};

/// Target frame period for the present loop (~60 fps).
const FRAME_PERIOD: Duration = Duration::from_micros(16_667);

/// Best-effort join timeout for the render thread on drop. Mirrors the
/// [`crate::state::SchedulerThreadHandle`] bounded-join shape: poll
/// [`JoinHandle::is_finished`] up to a 2-second deadline, then leak the
/// handle (the OS reaps the thread on process exit).
const RENDER_DROP_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

/// Polling cadence inside the drop-join loop.
const RENDER_JOIN_POLL_STEP: Duration = Duration::from_millis(10);

/// Everything the render thread needs, assembled on the main thread and
/// MOVED into [`spawn`].
///
/// The fields are constructed in [`crate::render::output_window`] on
/// the Tauri main thread (the macOS main-thread constraint applies to
/// the surface / adapter / device acquisition, all of which already
/// happened by the time this struct is built). `surface` is
/// `Surface<'static>` because it owns the `Arc<Window>` handle,
/// making the whole bundle `Send`.
pub struct RenderThreadResources<R: Runtime> {
    /// The presentation surface (owns the `Arc<Window>` handle).
    pub surface: wgpu::Surface<'static>,
    /// The surface configuration to apply on first frame + after resize.
    pub config: wgpu::SurfaceConfiguration,
    /// The surface-compatible renderer built via
    /// [`MultipassRenderer::new_with_surface_hint`].
    pub renderer: MultipassRenderer,
    /// The swapchain blit pipeline (color target = `config.format`).
    pub blit: BlitPass,
    /// The output window, kept alive for the loop's lifetime so we can
    /// read its physical size each frame for resize handling. The
    /// surface ALSO holds an `Arc` clone of this window internally.
    pub window: Arc<Window<R>>,
    /// The live output scene resolved per frame via `from_scene_live`
    /// (Slice R-1b), loaded on the main thread. `None` falls the loop back
    /// to the R-1a test pattern.
    pub scene: Option<LoadedScene>,
    /// The shared LIVE audio snapshot produced by `AudioRuntime`, read
    /// (cloned under the lock) once per frame to build the scene's
    /// [`LiveUniforms`] (Slice R-1c). `None` => the scene renders at
    /// silent audio.
    pub audio: Option<Arc<Mutex<AudioUniformSnapshot>>>,
}

/// Owns the spawned render thread + its shutdown signal.
///
/// Stored on [`crate::state::AppState`] so the thread is torn down
/// deterministically at app shutdown. The [`Drop`] impl signals
/// shutdown and best-effort joins (bounded, same posture as
/// [`crate::state::SchedulerThreadHandle`]).
pub struct RenderThreadHandle {
    join: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl core::fmt::Debug for RenderThreadHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RenderThreadHandle")
            .field("running", &self.join.is_some())
            .field("shutdown_signalled", &self.shutdown.load(Ordering::Relaxed))
            .finish()
    }
}

impl RenderThreadHandle {
    /// Borrow the shutdown flag (test/diagnostic accessor).
    #[must_use]
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    fn shutdown_inner(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            let deadline = Instant::now() + RENDER_DROP_JOIN_TIMEOUT;
            while Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    return;
                }
                std::thread::sleep(RENDER_JOIN_POLL_STEP);
            }
            // Timeout: leak the handle. The thread continues until
            // process exit; the OS reaps it. Indicates a bug (the
            // shutdown flag fired but the loop didn't exit in 2s).
        }
    }
}

impl Drop for RenderThreadHandle {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

/// Spawn the render thread, MOVING `resources` into it.
///
/// MUST be called from the main thread (it is — the Tauri `setup`
/// callback runs there) AFTER the surface / adapter / device have
/// already been acquired on the main thread. The returned handle drives
/// ordered shutdown.
///
/// # Errors
///
/// Returns the [`std::io::Error`] from [`std::thread::Builder::spawn`]
/// if the OS refuses to create the thread (resource exhaustion).
pub fn spawn<R: Runtime>(
    resources: RenderThreadResources<R>,
) -> std::io::Result<RenderThreadHandle> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_for_thread = Arc::clone(&shutdown);
    let join = std::thread::Builder::new()
        .name("chromaticity-render-output".into())
        .spawn(move || run_loop(resources, &shutdown_for_thread))?;
    Ok(RenderThreadHandle {
        join: Some(join),
        shutdown,
    })
}

/// The render-thread body. Owns all GPU resources; returns when the
/// shutdown flag fires or an unrecoverable surface error occurs.
fn run_loop<R: Runtime>(mut res: RenderThreadResources<R>, shutdown: &AtomicBool) {
    // Configure the surface once before the first frame. The config
    // size came from the window's physical size at construction.
    res.surface.configure(res.renderer.device(), &res.config);

    let started = Instant::now();
    let mut frame_index: u64 = 0;
    let mut last_time_secs = 0.0f32;
    let mut next_deadline = Instant::now() + FRAME_PERIOD;
    // dt-aware fast-attack/slow-release ballistics for the live audio
    // uniforms; carries follower state across frames (see
    // [`crate::render::audio_smooth`]).
    let mut audio_smoother = AudioSmoother::new();

    while !shutdown.load(Ordering::Relaxed) {
        // Resize handling: read the window's current physical size and
        // reconfigure if it diverged. A zero dimension (minimised /
        // not-yet-mapped window) is skipped — wgpu rejects a zero-sized
        // surface configuration.
        if let Ok(size) = res.window.inner_size() {
            if size.width > 0
                && size.height > 0
                && (size.width != res.config.width || size.height != res.config.height)
            {
                res.config.width = size.width;
                res.config.height = size.height;
                res.surface.configure(res.renderer.device(), &res.config);
            }
        }

        // Acquire the swapchain texture, handling every error variant.
        let frame = match res.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Timeout) => {
                // Transient — try again next iteration.
                pace(&mut next_deadline);
                continue;
            }
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                // Reconfigure and retry next iteration.
                res.surface.configure(res.renderer.device(), &res.config);
                pace(&mut next_deadline);
                continue;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                eprintln!(
                    "chromaticity-app: render output surface out of memory; stopping render thread"
                );
                return;
            }
            Err(wgpu::SurfaceError::Other) => {
                eprintln!(
                    "chromaticity-app: render output surface acquire failed (Other); \
                     stopping render thread"
                );
                return;
            }
        };

        // Build one live frame: the loaded output scene resolved through
        // `from_scene_live` (Slice R-1b) at the current swapchain size +
        // elapsed time, driven by the LIVE audio snapshot (Slice R-1c —
        // silent only when no audio source is present). The audio lock is
        // held only long enough to CLONE the small snapshot, then released
        // before the spec build. Fall back to the R-1a test pattern when no
        // scene loaded, or if a frame fails to resolve (validated at load,
        // so the Err arm is a safety net).
        let time_secs = started.elapsed().as_secs_f32();
        let time_delta = (time_secs - last_time_secs).max(0.0);
        last_time_secs = time_secs;
        let (width, height) = (res.config.width, res.config.height);
        let live = match &res.audio {
            Some(audio) => {
                // `AudioUniformSnapshot` is `Copy`; deref-copy out of the
                // guard so the lock is released immediately (before the
                // ballistics + spec build) and we never hold it across
                // `from_scene_live`.
                let raw_snapshot: AudioUniformSnapshot = match audio.lock() {
                    Ok(guard) => *guard,
                    Err(poisoned) => *poisoned.into_inner(),
                };
                // Ballistics run OUTSIDE the lock: apply per-channel
                // instant-attack/slow-release (PPM) smoothing so transients
                // land the same frame and decay smoothly (kills the raw
                // snapshot's frame-to-frame jitter without adding lag).
                let snapshot = audio_smoother.smooth(&raw_snapshot, time_delta);
                live_uniforms_from_audio(
                    time_secs,
                    time_delta,
                    (width as f32, height as f32),
                    &snapshot,
                )
            }
            None => LiveUniforms::silent(time_secs, (width as f32, height as f32)),
        };
        let (spec, scene_id) = match &res.scene {
            Some(loaded) => match loaded.frame_spec(width, height, &live) {
                Ok(spec) => (spec, OUTPUT_SCENE_SPEC_ID),
                Err(_) => (
                    test_pattern_spec(width, height, time_secs),
                    PATTERN_SCENE_ID,
                ),
            },
            None => (
                test_pattern_spec(width, height, time_secs),
                PATTERN_SCENE_ID,
            ),
        };
        let spec_key = live_feedback_spec_key(scene_id, spec.target_size);
        let rendered = match res.renderer.render_frame_live(&spec, frame_index, spec_key) {
            Ok(texture) => texture,
            Err(err) => {
                // A render error here is a programming / driver fault,
                // not a transient surface event. Log once per frame and
                // skip presenting this frame rather than panicking; if
                // it persists it stays visible in the log.
                eprintln!("chromaticity-app: render_frame_live failed: {err}");
                // Drop the unpresented swapchain texture (its Drop
                // discards the frame) and pace.
                drop(frame);
                pace(&mut next_deadline);
                continue;
            }
        };

        let source_view = rendered.create_view(&wgpu::TextureViewDescriptor::default());
        let swapchain_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        res.blit.blit(
            res.renderer.device(),
            res.renderer.queue(),
            &source_view,
            &swapchain_view,
        );
        frame.present();

        frame_index = frame_index.wrapping_add(1);
        pace(&mut next_deadline);
    }
}

/// Sleep until `next_deadline`, then advance it by one frame period.
/// If the loop already ran past the deadline (slow frame), reset the
/// deadline to one period from now instead of bursting catch-up frames
/// — mirrors the scheduler-tick coalescing posture in
/// `app/shell/src/lib.rs`.
fn pace(next_deadline: &mut Instant) {
    let now = Instant::now();
    if *next_deadline > now {
        std::thread::sleep(*next_deadline - now);
        *next_deadline += FRAME_PERIOD;
    } else {
        *next_deadline = now + FRAME_PERIOD;
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Slice R-1a pins for the frame pacer. The loop body needs a GPU
    //! surface (orchestrator-verified by running the app); the pacing
    //! arithmetic is pure and pinned here.

    use super::{FRAME_PERIOD, pace};
    use std::time::Instant;

    #[test]
    fn pace_advances_deadline_by_one_period_when_ahead() {
        // Deadline far in the future: pace sleeps (briefly, since the
        // delta is tiny here) and advances by exactly one period.
        let base = Instant::now();
        let mut deadline = base + FRAME_PERIOD;
        let before = deadline;
        pace(&mut deadline);
        assert_eq!(
            deadline,
            before + FRAME_PERIOD,
            "when the loop is on-cadence the deadline advances by one frame period",
        );
    }

    #[test]
    fn pace_resets_deadline_when_behind() {
        // Deadline in the past (a stalled frame): pace must NOT burst
        // catch-up; it resets the deadline to ~one period from now.
        let mut deadline = Instant::now() - FRAME_PERIOD * 10;
        pace(&mut deadline);
        let now = Instant::now();
        // The new deadline is in the future, within ~one frame period.
        assert!(
            deadline > now,
            "behind-cadence pace must push the deadline forward"
        );
        assert!(
            deadline <= now + FRAME_PERIOD + FRAME_PERIOD,
            "the reset deadline stays bounded near one frame period ahead",
        );
    }
}
