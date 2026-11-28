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
pub mod render;
pub mod script_host;
pub mod state;
pub mod syn_compat;

use crate::state::AppState;
use crate::state::ipc_emit::TauriIpcEmitter;
use chromaticity_audio::runtime::{AudioRuntime, AudioRuntimeConfig};
use chromaticity_runtime::crash_resume::{
    CanonicalBinarySnapshotter, CrashResumeAdapter, restore_all_supported_classes,
};
use chromaticity_runtime::scheduler::Scheduler;
use chromaticity_ui_bridge::{CrashResumeStatusPayload, ResumeStatusPayload};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::Manager;

/// Nominal scheduler-tick cadence. The coalescing loop targets
/// 60 Hz (~16.67ms per tick); commands that hold the
/// [`AppState::state`] mutex longer than the tick period collapse
/// missed ticks into the next pass instead of firing a catch-up
/// burst.
const SCHEDULER_TICK_HZ: f64 = 60.0;

/// File name (relative to `app.path().app_data_dir()`) for the
/// Wave 2.1 Lane C.3 persisted operator settings document.
const SETTINGS_FILE_NAME: &str = "settings.toml";

/// File name (relative to `app.path().app_data_dir()`) for the
/// EV-CRASH-RESUME runtime-state snapshot.
const CRASH_RESUME_FILE_NAME: &str = "crash-resume.bin";

/// Coarse crash-resume snapshot cadence. The scheduler captures a
/// snapshot at most once per second, coalescing missed frame ticks
/// after mutex stalls instead of writing in a burst.
const CRASH_RESUME_SAVE_PERIOD: Duration = Duration::from_secs(1);

pub use commands::inspect_state_machines;

#[tauri::command]
fn get_crash_resume_status(
    app_state: tauri::State<'_, AppState>,
) -> Result<Option<CrashResumeStatusPayload>, String> {
    Ok(app_state.crash_resume_status())
}

fn recover_crash_resume_snapshot(app_state: &AppState, save_path: &Path) {
    let started = Instant::now();
    let adapter = CrashResumeAdapter::new(CanonicalBinarySnapshotter, save_path.to_path_buf());
    match adapter.try_recover() {
        Ok(Some(recovered)) => {
            let mut state_guard = match app_state.state.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            restore_all_supported_classes(&recovered, &mut state_guard);
            drop(state_guard);
            app_state.set_crash_resume_status(Some(CrashResumeStatusPayload {
                show_name: "Recovered runtime state".to_string(),
                controls: ResumeStatusPayload::Restored,
                snaps: ResumeStatusPayload::Restored,
                playlist: ResumeStatusPayload::Restored,
                adapters: ResumeStatusPayload::Restored,
                scene: ResumeStatusPayload::Partial,
                p50: duration_millis_u32(started.elapsed()),
            }));
        }
        Ok(None) => {
            app_state.set_crash_resume_status(None);
        }
        Err(err) => {
            eprintln!(
                "chromaticity-app: failed to recover crash-resume snapshot from {}: {err}",
                save_path.display()
            );
            app_state.set_crash_resume_status(None);
        }
    }
}

fn duration_millis_u32(duration: Duration) -> u32 {
    let millis = duration.as_millis();
    if millis > u128::from(u32::MAX) {
        u32::MAX
    } else {
        millis as u32
    }
}

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
/// recovers any saved crash-resume snapshot before scheduler ticks
/// can mutate state; (d) spawns the state-commit thread that drives
/// [`Scheduler::tick`] under the [`AppState::state`] mutex on a
/// coalescing ~60 Hz cadence while saving crash-resume snapshots on
/// a coarse cadence; (e) installs both lifecycle handles onto the
/// managed [`AppState`] via
/// [`AppState::install_audio_and_scheduler`].
///
/// The scheduler loop carries an explicit next deadline. If a slow
/// tick or state-mutex stall pushes the loop past that deadline, the
/// next deadline is reset to one frame period from "now". That keeps
/// cadence stable without immediate back-to-back catch-up ticks.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            // Wave 2.1 Lane C.3 settings persistence — resolve the
            // app data dir, install the path onto AppState, and load
            // the existing document (or fall back to default when
            // no file yet exists). A malformed file is quarantined
            // by `load_or_default` before defaults are used, so a
            // corrupted settings file never blocks launch and the
            // operator's damaged copy is preserved.
            let mut crash_resume_path = None;
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
                if let Ok(resource_dir) = app.path().resource_dir() {
                    if let Ok(mut gpu) = bootstrap_state.gpu_renderer.lock() {
                        gpu.set_resource_dir(resource_dir.clone());
                    }
                    commands::render::set_thumbnail_resource_dir(resource_dir.clone());
                    bootstrap_state.install_bundled_scene_root(resource_dir.join("scenes"));
                }
                commands::preset::hydrate_presets_from_disk(bootstrap_state.inner(), &data_dir);
                if let Err(err) =
                    commands::osc::configure_live_osc_adapter(bootstrap_state.inner())
                {
                    eprintln!("chromaticity-app: failed to configure OSC adapter: {err}");
                }

                let save_path = data_dir.join(CRASH_RESUME_FILE_NAME);
                recover_crash_resume_snapshot(bootstrap_state.inner(), &save_path);
                crash_resume_path = Some(save_path);
            }

            // (a) Producer-side audio runtime. Falls back to silent
            // on CI / headless environments per the PR 1 contract;
            // construction is infallible.
            //
            // Wave 2.8 Lane AUD.SRC: honour the persisted operator
            // device preference from `settings.audio.input_device`.
            // Unresolvable preferences (operator unplugged BlackHole
            // between sessions, etc.) fall back to the host default
            // with a one-shot stderr warning per `try_input_by_preference`.
            let (preferred_input, input_gain_norm) = {
                let bootstrap_state = app.state::<AppState>();
                match bootstrap_state.settings.lock() {
                    Ok(guard) => (
                        Some(guard.audio.input_device.clone()),
                        guard.audio.input_gain,
                    ),
                    Err(poisoned) => {
                        let inner = poisoned.into_inner();
                        (Some(inner.audio.input_device.clone()), inner.audio.input_gain)
                    }
                }
            };
            // Map normalized [0, 1] input_gain (0.5 == unity, ±18 dB symmetric) to linear gain
            // and install into the lock-free live handle so Settings slider updates apply live.
            let app_state_bootstrap = app.state::<AppState>();
            app_state_bootstrap.set_live_input_gain_norm(input_gain_norm);
            let channel_pair_handle = app_state_bootstrap.live_audio_channel_pair_handle();
            let input_gain_handle = app_state_bootstrap.live_audio_input_gain_handle();
            let audio_config = AudioRuntimeConfig {
                input_gain_linear: input_gain_handle.get_linear(),
                input_gain_handle: Some(input_gain_handle),
                channel_pair: channel_pair_handle,
                ..AudioRuntimeConfig::default()
            };
            let runtime = AudioRuntime::with_preferred_device(
                preferred_input.as_deref(),
                audio_config,
            );
            let snapshot_handle = runtime.snapshot_handle();
            app_state_bootstrap.install_audio_snapshot_handle(snapshot_handle);
            let audio_handle = runtime.spawn();

            // (b) Scheduler-tick consumer. `SwappableAudioPull` reads the
            // active producer's shared snapshot once per tick BEFORE state
            // mutation (supporting zero-restart audio input hot-swap);
            // `TauriIpcEmitter` emits the resulting payload AFTER state
            // mutation, AFTER render dispatch, AFTER observability emit.
            let scheduler = Scheduler::new()
                .with_audio_pull(Box::new(app_state_bootstrap.swappable_audio_pull()))
                .with_ipc_emit(Box::new(TauriIpcEmitter::new(app.handle().clone())));

            // (c) State-commit thread. Coalescing 60 Hz loop; see
            // the function-level prose for the timing rationale.
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_for_thread = Arc::clone(&shutdown);
            let app_handle = app.handle().clone();
            let mut scheduler_owned = scheduler;
            let crash_resume_path_for_thread = crash_resume_path;
            let join = std::thread::Builder::new()
                .name("chromaticity-scheduler-tick".into())
                .spawn(move || {
                    let frame_period = Duration::from_secs_f64(1.0 / SCHEDULER_TICK_HZ);
                    let mut next_deadline = Instant::now() + frame_period;
                    let mut next_crash_resume_save = Instant::now() + CRASH_RESUME_SAVE_PERIOD;
                    let mut crash_resume_adapter = crash_resume_path_for_thread
                        .map(|path| CrashResumeAdapter::new(CanonicalBinarySnapshotter, path));
                    let mut scheduler_ticks: u64 = 0;
                    let mut mixer_out_buf = [0.0_f32; 32];
                    while !shutdown_for_thread.load(Ordering::Acquire) {
                        scheduler_ticks = scheduler_ticks.wrapping_add(1);
                        let tick_outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let should_capture_crash_resume = crash_resume_adapter.is_some()
                                && Instant::now() >= next_crash_resume_save;
                            let app_state = app_handle.state::<AppState>();
                            let mut state_guard = match app_state.state.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            let _ = scheduler_owned.tick(&mut state_guard, &app_state.bus);
                            let crash_resume_blob = if should_capture_crash_resume {
                                crash_resume_adapter
                                    .as_ref()
                                    .map(|adapter| adapter.snapshot(&state_guard))
                            } else {
                                None
                            };
                            drop(state_guard);

                            // Feed live audio energy into BroadcastMixer and ClockSyncCoordinator.
                            let live_audio = {
                                let slot = match app_state.audio_snapshot_slot.lock() {
                                    Ok(g) => Arc::clone(&g),
                                    Err(p) => Arc::clone(&p.into_inner()),
                                };
                                let snap = match slot.lock() {
                                    Ok(g) => *g,
                                    Err(p) => *p.into_inner(),
                                };
                                snap
                            };
                            let master_block = [live_audio.rms; 32];
                            if let Ok(mut mixer) = app_state.broadcast_mixer.lock() {
                                mixer.mix_stereo_block(
                                    &[(chromaticity_audio::mixer::BusId::MasterAudio, &master_block[..])],
                                    &mut mixer_out_buf,
                                );
                            }
                            if live_audio.bpm > 0.0 {
                                if let Ok(mut clock) = app_state.clock_sync.lock() {
                                    let now_ms = app_state.started_at.elapsed().as_millis() as u64;
                                    let phase = ((now_ms as f32 / 1000.0) * (live_audio.bpm / 60.0)).fract();
                                    clock.update_audio_bpm(live_audio.bpm, phase, 0.85, now_ms);
                                }
                            }
                            if scheduler_ticks % 60 == 0 {
                                use chromaticity_observability::emit::EmitMetric;
                                app_state.observability_emitter.emit(
                                    chromaticity_observability::catalog::MetricEvent::RuntimeUptime {
                                        value: app_state.started_at.elapsed().as_secs(),
                                    },
                                );
                            }

                            if let Some(blob_result) = crash_resume_blob {
                                match blob_result {
                                    Ok(blob) => {
                                        if let Some(adapter) = crash_resume_adapter.as_mut() {
                                            if let Err(err) = adapter.save_blob(&blob) {
                                                eprintln!(
                                                    "chromaticity-app: failed to save crash-resume snapshot to {}: {err}",
                                                    adapter.save_path().display()
                                                );
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!(
                                            "chromaticity-app: failed to serialize crash-resume snapshot: {err}"
                                        );
                                    }
                                }
                                next_crash_resume_save = Instant::now() + CRASH_RESUME_SAVE_PERIOD;
                            }

                            if let Err(err) =
                                commands::osc::poll_live_osc_adapter(app_state.inner())
                            {
                                eprintln!("chromaticity-app: OSC poll failed: {err}");
                            }
                            if let Err(err) =
                                commands::osc::drain_queued_osc_actions(app_state.inner())
                            {
                                eprintln!("chromaticity-app: OSC dispatch failed: {err}");
                            }
                        }));
                        if tick_outcome.is_err() {
                            eprintln!("chromaticity-app: recovered from panic in scheduler tick");
                        }

                        let now = Instant::now();
                        if next_deadline > now {
                            std::thread::sleep(next_deadline - now);
                            next_deadline += frame_period;
                        } else {
                            // Coalesce any missed ticks into this
                            // single scheduler pass. If a command
                            // held the state mutex for multiple frame
                            // periods, schedule the next tick one
                            // period from now instead of firing a
                            // burst of immediate catch-up ticks.
                            next_deadline = now + frame_period;
                        }
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
            // AppState. `ShellRuntime::shutdown` (invoked on
            // `AppState::drop` and `ShellRuntime::drop`) explicitly
            // tears down the scheduler thread first (signals shutdown
            // + bounded join) and the audio runtime second.
            let scheduler_thread = crate::state::SchedulerThreadHandle::new(join, shutdown);
            let app_state = app.state::<AppState>();
            app_state.install_audio_and_scheduler(audio_handle, scheduler_thread);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audio::list_audio_devices,
            commands::audio::list_audio_output_devices,
            commands::audio::get_dj_device_profile,
            commands::audio::set_audio_channel_pair,
            commands::audio::get_audio_channel_pair,
            commands::camera::list_camera_devices,
            commands::control::set_control,
            commands::control::get_control,
            commands::control::set_control_2d,
            commands::control::get_control_2d,
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
            commands::library::list_custom_scene_packages,
            commands::library::default_scene_root,
            commands::library::bundled_scene_root,
            commands::library_search::search_scene_library,
            commands::library_search::filter_scene_library_by_tag,
            commands::library_search::filter_scene_library_by_category,
            commands::midi::list_midi_devices,
            commands::midi::start_midi_learn,
            commands::midi::stop_midi_learn,
            commands::midi::start_midi_performance_listener,
            commands::midi::stop_midi_performance_listener,
            commands::osc::osc_health_snapshot,
            commands::osc::osc_routes_snapshot,
            commands::osc::bind_osc_route,
            commands::osc::unbind_osc_route,
            commands::osc::set_audio_threshold,
            commands::picker::pick_folder,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::reset_settings,
            get_crash_resume_status,
            commands::inspect_state_machines,
            commands::license::install_license,
            commands::license::remove_license,
            commands::license::request_trial,
            commands::license::get_entitlement_status,
            commands::syphon::list_syphon_servers,
            commands::syphon::publish_syphon_output,
            commands::syphon::unpublish_syphon_output,
            commands::scene::load_scene,
            commands::scene::loaded_scene,
            commands::render::render_gpu_frame,
            commands::render::gpu_render_status,
            commands::render::compile_and_preview_shader,
            commands::render::render_scene_thumbnail,
            commands::render::set_bpm_override,
            commands::render::get_bpm_override,
            commands::mix::get_gpu_cost_telemetry,
            commands::mix::set_adaptive_scale_enabled,
            commands::mix::get_compositor_capabilities,
            commands::mix::set_mix_layer_stack,
            commands::mix::resolve_youtube_stream,
            commands::feedback::load_scene_feedback,
            commands::feedback::save_scene_feedback,
            commands::author::read_scene_files,
            commands::author::write_scene_file,
            commands::import::import_shadertoy_source,
            commands::import::check_isf_import,
            commands::import::import_syn_scene,
            commands::import::inspect_shadertoy_license,
            commands::output::list_output_monitors,
            commands::output::open_output_window,
            commands::output::close_output_window,
            commands::output::toggle_output_fullscreen,
            commands::output::output_window_status,
            commands::recorder::start_recording,
            commands::recorder::stop_recording,
            commands::recorder::get_recorder_status,
            commands::recorder::push_recorder_frame,
            commands::recorder::push_recorder_jpeg_frame,
            commands::media::push_scene_media_frame,
            commands::media::clear_scene_media,
            commands::recorder::save_replay_buffer,
            commands::stream::start_twitch_stream,
            commands::stream::stop_twitch_stream,
            commands::stream::get_stream_status,
            commands::stream::push_stream_frame,
            commands::stream::list_stream_audio_devices,
            commands::subsystems::get_broadcast_mixer_status,
            commands::subsystems::set_broadcast_bus_config,
            commands::subsystems::get_clock_sync_status,
            commands::subsystems::dispatch_footswitch_action,
            commands::subsystems::get_dmx_universe_status,
            commands::subsystems::set_dmx_channel,
            commands::subsystems::send_dmx_artnet_frame,
            commands::subsystems::get_data_ingest_status,
            commands::subsystems::get_observability_status,
        ])
        .build(tauri::generate_context!())?
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let app_state = app_handle.state::<AppState>();
                app_state.shutdown_runtime();
                let mut stream_guard = match app_state.twitch_stream.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                stream_guard.stop();
                drop(stream_guard);
                commands::mix::shutdown_youtube_proxy();
            }
        });
    Ok(())
}
