// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Native GPU output-window pipeline for ADR-011 Phase-2 Slices R-1a/R-1b.
//!
//! R-1a proved the `wgpu` ↔ Tauri surface pipeline + the macOS
//! main-thread handoff in isolation (a titled output window presenting a
//! time-animated test pattern). R-1b builds on it: the window now renders
//! a real `.chroma` SCENE — loaded on the main thread
//! ([`scene_source::LoadedScene`]) and resolved per frame through the
//! engine's `from_scene_live` resolver — blitted to the swapchain and
//! presented in a dedicated render-thread loop, with the R-1a test
//! pattern as the fallback. R-1c threads the LIVE audio snapshot
//! (`AudioRuntime`'s shared `Arc<Mutex<AudioUniformSnapshot>>`) into the
//! render thread, so the scene now REACTS to sound (it renders at silent
//! audio only when no audio source is present). Scheduler wiring stays
//! out of this path — the render thread reads the audio snapshot
//! directly.
//!
//! ## Opt-in via `CHROMA_RENDER_OUTPUT`
//!
//! [`maybe_spawn_output_window`] is a no-op unless the
//! `CHROMA_RENDER_OUTPUT` environment variable is set to a truthy value
//! (`1` / `true` / `yes` / `on`, case-insensitive). Default launches
//! are completely unchanged — the env-gate keeps R-1a low-blast-radius.
//!
//! ## Thread boundary (load-bearing on macOS)
//!
//! - [`output_window`] runs on the MAIN THREAD (called from the Tauri
//!   `setup` callback) and creates the window + surface + adapter /
//!   device. On macOS / Metal these are main-thread-only — off-thread
//!   is a PANIC.
//! - [`thread`] receives the resulting `Send` bundle and MOVES it onto
//!   a dedicated render thread that runs the present loop. It never
//!   creates a surface or window.

pub mod blit;
pub mod output_window;
pub mod pattern;
pub mod scene_source;
pub mod thread;

use std::sync::Arc;
use std::sync::Mutex;

use chromaticity_audio::AudioUniformSnapshot;
use tauri::{Manager, Runtime};

use crate::render::thread::RenderThreadHandle;

/// Spawn the R-1a output window + render thread IFF
/// `CHROMA_RENDER_OUTPUT` is truthy.
///
/// `audio` is the shared LIVE audio snapshot produced by `AudioRuntime`,
/// threaded through to the render thread so the scene reacts to sound
/// (Slice R-1c); `None` => the scene renders at silent audio.
///
/// Returns `Ok(Some(handle))` when the output window was built and the
/// render thread spawned; `Ok(None)` when the env-gate is unset (the
/// common default-launch path); `Err` only when the gate IS set but
/// bootstrap failed (so the caller can log it — a failed opt-in render
/// window must not crash the app).
///
/// MUST be called on the main thread (the Tauri `setup` callback).
///
/// # Errors
///
/// See [`output_window::OutputWindowError`] plus a thread-spawn
/// [`std::io::Error`] surfaced as a boxed error.
pub fn maybe_spawn_output_window<R, M>(
    app: &M,
    audio: Option<Arc<Mutex<AudioUniformSnapshot>>>,
) -> Result<Option<RenderThreadHandle>, Box<dyn std::error::Error>>
where
    R: Runtime,
    M: Manager<R>,
{
    if !output_enabled() {
        return Ok(None);
    }

    // Main-thread: build the window + surface + renderer + blit.
    let resources = output_window::build_output_window_resources::<R, M>(app, audio)?;
    // Hand the Send bundle to the render thread (it MOVES it).
    let handle = thread::spawn(resources)?;
    Ok(Some(handle))
}

/// Name of the env var that opt-in-enables the R-1a output window.
const OUTPUT_ENV_VAR: &str = "CHROMA_RENDER_OUTPUT";

/// `true` iff `CHROMA_RENDER_OUTPUT` is set to a truthy value.
fn output_enabled() -> bool {
    is_truthy(std::env::var(OUTPUT_ENV_VAR).ok().as_deref())
}

/// Pure parse of the env-gate value (split out so it is testable
/// without touching the process-global environment — `std::env::set_var`
/// is `unsafe` under the 2024 edition and the workspace forbids
/// `unsafe`). `None` (unset) is falsy.
fn is_truthy(value: Option<&str>) -> bool {
    match value {
        Some(raw) => {
            let v = raw.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        None => false,
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! R-1a env-gate pins. The window / surface path needs a GUI +
    //! GPU (orchestrator-verified by running the app); the env parsing
    //! is pure and pinned here against the parse helper directly so the
    //! test never mutates the process-global environment (which would
    //! require `unsafe` under the 2024 edition).

    use super::is_truthy;

    #[test]
    fn truthy_values_enable_output() {
        for truthy in ["1", "true", "TRUE", "Yes", "on", " on "] {
            assert!(is_truthy(Some(truthy)), "{truthy:?} should enable output");
        }
    }

    #[test]
    fn falsy_and_unset_values_disable_output() {
        for falsy in ["0", "false", "no", "off", "", "  "] {
            assert!(
                !is_truthy(Some(falsy)),
                "{falsy:?} should NOT enable output"
            );
        }
        assert!(!is_truthy(None), "an unset var should NOT enable output");
    }
}
