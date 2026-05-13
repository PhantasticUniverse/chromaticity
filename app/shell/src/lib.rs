// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Chromaticity desktop application shell entrypoint.
// Tauri 2 bootstrap that registers the shell-side `AppState` and the
// Phase-1 D2 IPC handlers (`set_control`, `get_control`) so the UI can
// drive the runtime command bus through typed IPC. ts-rs binding
// generation lives in `tests/export_bindings.rs` (test-target export
// per ts-rs 10's conventional pattern).
//
// Wave 1c-α-2 production-bootstrap PR 2 (this commit) extends the
// builder's `setup` callback to spawn the producer-side audio runtime
// (PR 1's `chromaticity_audio::runtime::AudioRuntime`) plus a state-
// commit thread driving `chromaticity_runtime::scheduler::Scheduler::tick`
// at ~60 Hz. The scheduler is configured with the
// `MutexAudioPull` consumer reading the producer's
// `Arc<Mutex<AudioUniformSnapshot>>` AND the Tauri-event
// `TauriIpcEmitter` adapter so the cross-seam audio-uniform IPC bridge
// (ADR-019) flows end-to-end in production. The existing command-handler
// path that calls `bus.commit_one_into` keeps working: the scheduler-tick
// is purely additive — it drains whatever events accumulated since the
// last tick, which is a no-op when no command handler ran in the
// intervening ~16ms.

pub mod commands;
pub mod state;

use crate::state::AppState;
use crate::state::ipc_emit::TauriIpcEmitter;
use chromaticity_audio::runtime::{AudioRuntime, AudioRuntimeConfig};
use chromaticity_runtime::scheduler::Scheduler;
use chromaticity_runtime::scheduler::audio::MutexAudioPull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::Manager;

/// Nominal scheduler-tick cadence. The drift-corrected loop targets
/// 60 Hz (~16.67ms per tick); commands that hold the
/// [`AppState::state`] mutex longer than the tick period cause the
/// loop to fall behind without sleep, recovering on subsequent ticks
/// once the lock is released.
const SCHEDULER_TICK_HZ: f64 = 60.0;

/// File name (relative to `app.path().app_data_dir()`) for the
/// Wave 2.1 Lane C.3 persisted operator settings document.
const SETTINGS_FILE_NAME: &str = "settings.toml";

/// Launch the Tauri application event loop.
///
/// Returns an error if the Tauri runtime fails to start (bad context,
/// capability mismatch, window creation failure, etc.); the binary
/// entry in `src/main.rs` logs the error and exits non-zero.
///
/// ## Bootstrap PR 2 — audio runtime + scheduler-tick wiring
///
/// The `setup` callback (a) constructs the producer-side
/// [`AudioRuntime`] with the default cpal device (falling back to
/// silent on CI / headless / device-less environments per the PR 1
/// fallback contract), captures its
/// `Arc<Mutex<AudioUniformSnapshot>>` shared seam, and spawns the
/// producer thread; (b) constructs a [`Scheduler`] with both
/// [`MutexAudioPull`] (reading the producer's shared snapshot) and
/// [`TauriIpcEmitter`] (emitting the
/// [`chromaticity_audio::AudioUniformSnapshot`] payload onto the
/// Tauri event channel as `audio_uniform_snapshot_changed`); (c)
/// spawns the state-commit thread that drives
/// [`Scheduler::tick`] under the [`AppState::state`] mutex on a
/// drift-corrected ~60 Hz cadence; (d) installs both lifecycle
/// handles onto the managed [`AppState`] via
/// [`AppState::install_audio_and_scheduler`].
///
/// The drift-corrected timer pattern computes each tick's deadline
/// against `t0 + frame_period * tick_count` rather than incrementally
/// adding `frame_period` to the previous wake time. This avoids
/// accumulating per-tick rounding errors over long runs and lets the
/// loop recover its nominal cadence after a slow tick (e.g. a Tauri
/// command handler that held the state mutex longer than ~16ms).
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            // Wave 2.1 Lane C.3 settings persistence — resolve the
            // app data dir, install the path onto AppState, and load
            // the existing document (or fall back to default when
            // no file yet exists). A malformed file is logged + the
            // default is used so a corrupted settings file never
            // blocks the app from launching; the operator's next
            // save overwrites it.
            if let Ok(data_dir) = app.path().app_data_dir() {
                let settings_path = data_dir.join(SETTINGS_FILE_NAME);
                let loaded =
                    match chromaticity_runtime::settings::Settings::load_or_default(&settings_path)
                    {
                        Ok(s) => s,
                        Err(err) => {
                            eprintln!(
                                "chromaticity-app: failed to load settings from {}: {err}; \
                             using defaults",
                                settings_path.display()
                            );
                            chromaticity_runtime::settings::Settings::default()
                        }
                    };
                let bootstrap_state = app.state::<AppState>();
                bootstrap_state.install_settings(loaded);
                bootstrap_state.install_settings_path(settings_path);
            }

            // (a) Producer-side audio runtime. Falls back to silent
            // on CI / headless environments per the PR 1 contract;
            // construction is infallible.
            let runtime = AudioRuntime::new(AudioRuntimeConfig::default());
            let snapshot_handle = runtime.snapshot_handle();
            let audio_handle = runtime.spawn();

            // (b) Scheduler-tick consumer. `MutexAudioPull` reads the
            // producer's shared snapshot once per tick BEFORE state
            // mutation; `TauriIpcEmitter` emits the resulting payload
            // AFTER state mutation, AFTER render dispatch, AFTER
            // observability emit per the
            // `engine/runtime/src/scheduler/ipc.rs` ordering contract.
            let scheduler = Scheduler::new()
                .with_audio_pull(Box::new(MutexAudioPull::new(snapshot_handle)))
                .with_ipc_emit(Box::new(TauriIpcEmitter::new(app.handle().clone())));

            // (c) State-commit thread. Drift-corrected 60 Hz loop;
            // see the function-level prose for the timing rationale.
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_for_thread = Arc::clone(&shutdown);
            let app_handle = app.handle().clone();
            let mut scheduler_owned = scheduler;
            let join = std::thread::Builder::new()
                .name("chromaticity-scheduler-tick".into())
                .spawn(move || {
                    let t0 = Instant::now();
                    let frame_period = Duration::from_secs_f64(1.0 / SCHEDULER_TICK_HZ);
                    let mut tick_count: u64 = 0;
                    while !shutdown_for_thread.load(Ordering::Relaxed) {
                        // Look up the managed AppState lazily on each
                        // tick. Capturing a `tauri::State<'_,
                        // AppState>` outside the loop would borrow
                        // across the await point of
                        // `is_finished()` polling; the per-tick
                        // lookup is O(map-hash) and not on the
                        // frame-budget critical path.
                        let app_state = app_handle.state::<AppState>();
                        let mut state_guard = match app_state.state.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(),
                        };
                        // Swallow the tick result. The only currently-
                        // surfaced error is `FrameOverflow` at
                        // `u64::MAX` ticks, which is unreachable in
                        // normal operation (~9.7 billion years at 60
                        // FPS per the SchedulerError prose).
                        let _ = scheduler_owned.tick(&mut state_guard, &app_state.bus);
                        drop(state_guard);

                        tick_count = tick_count.saturating_add(1);
                        let next_deadline = t0
                            + frame_period
                                .saturating_mul(u32::try_from(tick_count).unwrap_or(u32::MAX));
                        let now = Instant::now();
                        if next_deadline > now {
                            // Sleep to the next tick boundary. If we
                            // overslept (shouldn't happen with `<` /
                            // `>` order above), the next iteration
                            // catches up by setting `next_deadline`
                            // to a past timestamp and skipping the
                            // sleep.
                            std::thread::sleep(next_deadline - now);
                        }
                        // If we're behind (next_deadline <= now), the
                        // next tick fires immediately. Drift
                        // accumulates if Tauri command handlers hold
                        // the state mutex longer than the tick period;
                        // recovery is automatic once the lock is
                        // released and the loop catches up. The
                        // saturating_mul above caps the deadline
                        // computation at u32::MAX seconds (~136
                        // years) which is the practical ceiling.
                    }
                })
                .map_err(|e| -> Box<dyn std::error::Error> {
                    // Bootstrap-time spawn failure is unrecoverable —
                    // the audio-uniform IPC bridge cannot run without
                    // the scheduler-tick thread. Surface the error
                    // through Tauri's setup-callback error path
                    // rather than panicking; the app fails to launch
                    // and the binary entry in `src/main.rs` logs it.
                    Box::<dyn std::error::Error>::from(format!(
                        "failed to spawn chromaticity-scheduler-tick thread: {e}"
                    ))
                })?;

            // (d) Install lifecycle handles onto the managed
            // AppState. The handles' Drop impls fire at app shutdown
            // (Tauri-managed state is dropped at exit) in
            // declaration-reverse order: scheduler thread first
            // (signals shutdown + bounded join), then audio runtime
            // (signals shutdown + bounded join). Field declaration
            // order on AppState pins this contract; see
            // `app/shell/src/state/mod.rs` module prose.
            let scheduler_thread = crate::state::SchedulerThreadHandle::new(join, shutdown);
            let app_state = app.state::<AppState>();
            app_state.install_audio_and_scheduler(audio_handle, scheduler_thread);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audio::list_audio_devices,
            commands::audio::list_audio_output_devices,
            commands::camera::list_camera_devices,
            commands::control::set_control,
            commands::control::get_control,
            commands::snap::recall_snap,
            commands::snap::save_snap,
            commands::snap::clear_snap,
            commands::snap::active_snap_slot,
            commands::snap::snap_slot_saved,
            commands::preset::recall_preset,
            commands::preset::save_preset,
            commands::preset::active_preset,
            commands::playlist::advance_playlist,
            commands::playlist::playlist_position,
            commands::library::list_scene_packages,
            commands::library::list_bundled_scene_packages,
            commands::library::default_scene_root,
            commands::library::bundled_scene_root,
            commands::library_search::search_scene_library,
            commands::library_search::filter_scene_library_by_tag,
            commands::library_search::filter_scene_library_by_category,
            commands::midi::list_midi_devices,
            commands::midi::start_midi_learn,
            commands::midi::stop_midi_learn,
            commands::osc::osc_health_snapshot,
            commands::osc::osc_routes_snapshot,
            commands::osc::bind_osc_route,
            commands::osc::unbind_osc_route,
            commands::picker::pick_folder,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::reset_settings,
            commands::license::install_license,
            commands::license::remove_license,
            commands::license::request_trial,
            commands::syphon::list_syphon_servers,
            commands::syphon::publish_syphon_output,
            commands::syphon::unpublish_syphon_output,
            commands::scene::load_scene,
            commands::scene::loaded_scene,
        ])
        .run(tauri::generate_context!())
}
