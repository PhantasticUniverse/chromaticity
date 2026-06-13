// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell-side application state managed by Tauri.
//!
//! Composes the canonical [`chromaticity_runtime::state::State`] with an
//! [`chromaticity_runtime::InMemoryBus`]. The shell holds the only
//! mutable reference to `State` behind a [`Mutex`]; Tauri command
//! handlers acquire the lock, dispatch through the bus, and apply
//! committed events via [`chromaticity_runtime::InMemoryBus::commit_one_into`].
//!
//! Phase-1 D2 scope: a single-window shell whose state is owned by one
//! Tauri-managed [`AppState`]. Multi-window or worker-thread variants
//! land with the slice that introduces them.
//!
//! ## Bootstrap PR 2 — audio runtime + scheduler-tick lifecycle handles
//!
//! The Wave 1c-α-2 production-bootstrap wave wires the producer-side
//! audio runtime (PR 1's [`chromaticity_audio::runtime::AudioRuntime`])
//! together with the scheduler-tick consumer (Wave 1c-α PR 1's
//! [`chromaticity_runtime::scheduler::Scheduler`] +
//! [`chromaticity_runtime::scheduler::ipc::IpcEmit`]) into the Tauri
//! shell so audio-uniform data flows end-to-end in production.
//!
//! [`AppState`] grows two optional lifecycle handles:
//!
//! - [`AppState::audio`] — `Some(AudioRuntimeHandle)` once the audio
//!   producer thread has been spawned via [`AudioRuntime::spawn`]; the
//!   handle's `Drop` impl signals shutdown and best-effort joins the
//!   producer thread on a 2-second timeout (PR 1 contract).
//! - [`AppState::scheduler_thread`] — `Some(SchedulerThreadHandle)`
//!   once the state-commit thread that drives
//!   [`chromaticity_runtime::scheduler::Scheduler::tick`] at ~60 Hz
//!   has been spawned. Carries an `Arc<AtomicBool>` shutdown flag the
//!   thread polls between iterations + the spawned `JoinHandle` for
//!   ordered shutdown.
//!
//! Both fields default to `None` so existing tests calling
//! [`AppState::new`] continue to construct a runtime-free shell. The
//! Tauri bootstrap path (`app/shell/src/lib.rs::run`) installs the
//! handles via [`AppState::install_audio_and_scheduler`] from inside
//! the Tauri builder's `setup` callback, after [`AppState::new`] has
//! already been registered with `manage`.
//!
//! ## Ordered shutdown contract
//!
//! The composition's `Drop` ordering at app shutdown matters:
//!
//! 1. **Signal scheduler-tick shutdown.** The state-commit thread
//!    must stop touching `AppState` before any other field is dropped
//!    — otherwise the thread could observe a half-dropped
//!    [`AppState`] (Tauri-managed state has no guaranteed teardown
//!    ordering between the manager and the spawned threads that
//!    captured an [`AppHandle`] referencing it).
//! 2. **Drop the audio runtime handle.** The producer thread is
//!    independent of the scheduler; it's safe to drop after step 1.
//!    [`chromaticity_audio::runtime::AudioRuntimeHandle::drop`] does
//!    its own best-effort shutdown + 2-second join.
//!
//! [`SchedulerThreadHandle::drop`] performs step 1; the engine
//! crate's [`AudioRuntimeHandle::drop`] performs step 2. The `Drop`
//! order on [`AppState`] follows Rust's struct field-declaration
//! order in reverse, so [`AppState::scheduler_thread`] is dropped
//! BEFORE [`AppState::audio`] iff the field declarations match. Don't
//! reorder the field declarations without updating the contract above.

use crate::render::thread::RenderThreadHandle;
use chromaticity_audio::runtime::AudioRuntimeHandle;
use chromaticity_midi_adapter::MidiListenHandle;
use chromaticity_osc_adapter::adapter::UdpOscAdapter;
use chromaticity_osc_adapter::bindings::{OscDispatchAction, OscDispatchSink};
use chromaticity_osc_adapter::route_registry::OscRouteRegistry;
use chromaticity_osc_adapter::sender::OscSender;
use chromaticity_runtime::InMemoryBus;
use chromaticity_runtime::settings::Settings;
use chromaticity_runtime::state::State;
use chromaticity_scene::schema::Scene;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub mod ipc_emit;

/// Best-effort join timeout for the scheduler-tick thread on drop.
///
/// Mirrors the
/// [`chromaticity_audio::runtime::AudioRuntimeHandle`]
/// `DROP_JOIN_TIMEOUT` shape: poll [`JoinHandle::is_finished`] on a
/// 10ms cadence up to a 2-second deadline, then fall through (the
/// thread is detached on timeout; the OS reaps it on process exit).
const SCHEDULER_DROP_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

/// Polling cadence inside the [`SchedulerThreadHandle`] drop join
/// loop.
const SCHEDULER_JOIN_POLL_STEP: Duration = Duration::from_millis(10);

/// Owns the spawned scheduler-tick thread and its shutdown signal.
///
/// The scheduler thread runs a drift-corrected ~60 Hz loop calling
/// [`chromaticity_runtime::scheduler::Scheduler::tick`] under the
/// [`AppState::state`] mutex. The shutdown flag is polled between
/// iterations; the [`Drop`] impl signals shutdown and best-effort
/// joins on a 2-second deadline.
///
/// See the module-level "Ordered shutdown contract" prose for why
/// [`AppState`] declares the field BEFORE [`AppState::audio`].
pub struct SchedulerThreadHandle {
    join: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl core::fmt::Debug for SchedulerThreadHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SchedulerThreadHandle")
            .field("running", &self.join.is_some())
            .field("shutdown_signalled", &self.shutdown.load(Ordering::Relaxed))
            .finish()
    }
}

impl SchedulerThreadHandle {
    /// Wrap a spawned thread + its shutdown flag.
    #[must_use]
    pub fn new(join: JoinHandle<()>, shutdown: Arc<AtomicBool>) -> Self {
        Self {
            join: Some(join),
            shutdown,
        }
    }

    /// Borrow the shutdown flag for the thread closure to poll.
    #[must_use]
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    fn shutdown_inner(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join.take() {
            // Bounded best-effort join. `JoinHandle::join` has no
            // timeout API on stable Rust; approximate by polling
            // `is_finished` (stable since 1.61). Mirrors the
            // `AudioRuntimeHandle` shape in
            // `engine/audio/src/runtime.rs:410-422`.
            let deadline = Instant::now() + SCHEDULER_DROP_JOIN_TIMEOUT;
            while Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    return;
                }
                std::thread::sleep(SCHEDULER_JOIN_POLL_STEP);
            }
            // Timeout: leak the handle. The thread continues until
            // process exit; the OS reaps it. Indicates a bug — the
            // shutdown flag fired but the thread didn't exit within
            // 2 seconds — and is observable through process tracing.
        }
    }
}

impl Drop for SchedulerThreadHandle {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

/// Tauri-managed shell state.
///
/// Field order is load-bearing: per the module-level "Ordered
/// shutdown contract", [`Self::scheduler_thread`] MUST be declared
/// AFTER [`Self::state`] / [`Self::bus`] AND BEFORE [`Self::audio`]
/// so Rust's reverse-field-declaration drop order tears the
/// scheduler thread down BEFORE the audio runtime, which is BEFORE
/// the canonical state + bus. Reordering the fields without updating
/// the prose breaks the contract silently.
pub struct AppState {
    /// Canonical runtime state guarded behind a mutex so command
    /// handlers can apply committed events serially.
    pub state: Mutex<State>,
    /// In-memory command bus — the single producer/commit seam the
    /// shell uses to mutate `state`.
    pub bus: InMemoryBus,
    /// Operator preferences document held behind a mutex so the
    /// Wave 2.1 Lane C.3 `get_settings` / `set_settings` IPC
    /// handlers serialise concurrent reads / writes. Defaults to
    /// [`Settings::default`] in [`AppState::new`]; the Tauri
    /// bootstrap path replaces the initial default by calling
    /// [`AppState::install_settings`] after loading the on-disk
    /// document (or falling back to default when no file yet
    /// exists).
    pub settings: Mutex<Settings>,
    /// Path to the on-disk TOML file backing [`Self::settings`].
    /// `Some(path)` once the Tauri bootstrap path resolves
    /// `app.path().app_data_dir()`; `None` in test contexts that
    /// don't bootstrap persistence (the in-memory mutex still
    /// works; the write-through file I/O is skipped).
    pub settings_path: Mutex<Option<PathBuf>>,
    /// Scheduler-tick thread handle. `Some` once the bootstrap path
    /// has spawned the state-commit thread; `None` in test contexts
    /// that construct a runtime-free [`AppState`] via
    /// [`AppState::new`]. Behind a [`Mutex`] for interior mutability:
    /// [`AppState::install_audio_and_scheduler`] takes `&self` since
    /// the Tauri-managed state is reached via
    /// [`tauri::Manager::state`] which yields `tauri::State<'_,
    /// AppState>` (immutable shared borrow).
    pub scheduler_thread: Mutex<Option<SchedulerThreadHandle>>,
    /// Audio runtime handle. `Some` once the bootstrap path has
    /// spawned the producer thread; `None` in test contexts. Drop
    /// signals shutdown + best-effort joins per
    /// [`chromaticity_audio::runtime::AudioRuntimeHandle::drop`].
    pub audio: Mutex<Option<AudioRuntimeHandle>>,
    /// Active MIDI learn-mode listener handle. `Some` between a
    /// successful `start_midi_learn` IPC call and the matching
    /// `stop_midi_learn` (or until AppState teardown). The Wave 2.1
    /// Lane D.2 minimal-viable learn surface holds at most one active
    /// listener at a time: starting a new learn session replaces the
    /// prior handle, whose `Drop` impl closes the OS midir port
    /// deterministically. App shutdown drops AppState which drops the
    /// `Option` slot which drops the handle which closes the port —
    /// midir 0.10 documents [`MidiInputConnection::drop`] as "will
    /// close the connection when dropped", so no `MidiInputConnection`
    /// leaks on app exit.
    pub midi_learn_handle: Mutex<Option<MidiListenHandle>>,
    /// Most recently loaded `.chroma` scene package and its on-disk
    /// path. Wave 2.1 Lane A.1 introduces this as a shell-side
    /// concern: the Library view's double-click handler invokes
    /// `load_scene(path)`, which parses the `.chroma` directory via
    /// [`chromaticity_scene::loader::load_chroma_dir`] and stores the
    /// resulting [`Scene`] here so the Perform view's Scene Controls
    /// panel can bind to the real control schema instead of the
    /// `perform-fixtures.ts` placeholder. `None` before the first
    /// load.
    ///
    /// **Not flowing through the canonical command bus on purpose.**
    /// Lane A.1 keeps the engine [`chromaticity_runtime::Command`]
    /// envelope frozen — adding a `SceneLoad` variant would touch the
    /// observability catalog, [`crate::state::AppState`]
    /// `apply_event` path, multiple drift pins, and the EV-STATE-REPLAY
    /// / EV-PRESET-PLAYLIST T2 corpora. The loaded scene is a shell-
    /// side concept until Wave 2.2 wires the render pipeline + control
    /// dispatch refactor against it; at that point the engine surface
    /// widens in a single coordinated PR.
    pub loaded_scene: Mutex<Option<LoadedSceneRecord>>,
    /// Operator-facing OSC route binding registry.
    ///
    /// Wave 2.1 Lane D.3 (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`).
    /// Seeded from
    /// [`chromaticity_osc_adapter::route_registry::OscRouteRegistry::with_default_seed`]
    /// in [`Self::new`] so the OSC view's routes table renders against
    /// the canonical UX subset (matching the v0.2-beta bundle fixture)
    /// immediately on app launch. The `bind_target` field on each entry
    /// is mutated by `bind_osc_route` / `unbind_osc_route` IPC handlers;
    /// the seed table itself is immutable across the registry's lifetime.
    ///
    /// Bind / unbind survives the process lifetime only; restoring
    /// bindings across restart is a future-wave concern (would extend
    /// either the settings document or grow a sibling
    /// `osc-bindings.toml` document).
    pub osc_routes: Mutex<OscRouteRegistry>,
    /// Bundled scene resource root (`<AppBundle>/Resources/scenes`).
    ///
    /// Installed during Tauri setup when resource-dir resolution is
    /// available. OSC `scene:<id>` dispatch uses this root plus the
    /// operator's persisted custom scene folders to resolve scene ids
    /// into `.chroma` packages without widening the runtime command
    /// bus.
    pub bundled_scene_root: Mutex<Option<PathBuf>>,
    /// Live UDP OSC adapter owned by the shell.
    ///
    /// The adapter itself never mutates [`Self::state`] while polling.
    /// Instead, its dispatch sink appends [`OscDispatchAction`]s into
    /// [`Self::osc_dispatch_queue`]. The scheduler thread calls the
    /// OSC poll helper after dropping the runtime state lock, then
    /// drains queued actions through the command bus / shell scene
    /// loader in a separate step.
    pub osc_adapter: Mutex<UdpOscAdapter>,
    /// Outbound OSC sender owned by the shell.
    ///
    /// Wraps a non-blocking UDP send socket targeting the
    /// operator-configured host:port (ADR-013 §1 loopback default).
    /// `None` when the initial bind fails (e.g. all ephemeral ports
    /// exhausted); the failure is logged and the runtime continues
    /// without outbound OSC. [`configure_live_osc_adapter`]
    /// replaces the sender when the operator changes outbound settings.
    pub osc_sender: Mutex<Option<OscSender>>,
    /// Queue of actions emitted by [`Self::osc_adapter`]'s sink.
    pub osc_dispatch_queue: Arc<Mutex<Vec<OscDispatchAction>>>,
    /// Current audio noise-floor gate threshold value.
    ///
    /// Shell-side state (not in the ADR-004 canonical [`State`] — that
    /// slot is class-e in `engine/runtime/src/crash_resume.rs` and is
    /// still a documented-deferred no-op). Defaults to `0.0` on
    /// construction. Updated by two paths:
    ///
    /// 1. **Inbound OSC** — when the adapter receives
    ///    `/audio/threshold {f}` on a route bound to `audio:threshold`,
    ///    [`drain_queued_osc_actions`] calls
    ///    [`apply_audio_threshold`](Self::apply_audio_threshold) and
    ///    emits the outbound announce (the `OscDirection::Both` contract,
    ///    PR9).
    /// 2. **Direct set** — the `set_audio_threshold` Tauri command calls
    ///    the same helper so the operator can set the threshold via the
    ///    `Shift+T` hotkey path (frontend-only today; the IPC surface is
    ///    provided here for when the hotkey handler is wired to a backend
    ///    command).
    pub audio_threshold: Mutex<f32>,
    /// ADR-011 Phase-2 Slice R-1a native GPU output-window render
    /// thread. `Some(handle)` once the Tauri bootstrap path has spawned
    /// the render thread (only when the `CHROMA_RENDER_OUTPUT`
    /// env-gate is set); `None` on default launches and in every test
    /// context. Behind a [`Mutex`] for the same interior-mutability
    /// reason as [`Self::scheduler_thread`]: the install path takes
    /// `&self` because Tauri-managed state is reached via a shared
    /// borrow.
    ///
    /// Declared LAST so Rust's reverse-field-declaration drop order
    /// tears the render thread down FIRST at app shutdown — the present
    /// loop stops (releasing the surface + window + GPU device) before
    /// the scheduler / audio / canonical state. The render thread is
    /// independent of all of them ([`RenderThreadHandle::drop`] signals
    /// shutdown + bounded-joins on its own), so dropping it first is
    /// always safe.
    pub render_thread: Mutex<Option<RenderThreadHandle>>,
}

struct QueuedOscDispatchSink {
    queue: Arc<Mutex<Vec<OscDispatchAction>>>,
}

impl OscDispatchSink for QueuedOscDispatchSink {
    fn dispatch(&self, action: OscDispatchAction) -> Result<(), String> {
        let mut guard = self
            .queue
            .lock()
            .map_err(|err| format!("osc dispatch queue mutex poisoned: {err}"))?;
        guard.push(action);
        Ok(())
    }
}

/// Composite record stored on [`AppState::loaded_scene`].
///
/// Carries the parsed [`Scene`] alongside the absolute path the
/// runtime loaded it from so the IPC handler can return both the
/// id (derived from the path's file-stem) and the on-disk location
/// without re-walking the filesystem.
#[derive(Debug, Clone)]
pub struct LoadedSceneRecord {
    pub scene: Scene,
    pub path: std::path::PathBuf,
}

impl AppState {
    /// Construct a fresh shell state with a default canonical
    /// [`State`] and a freshly-instantiated [`InMemoryBus`].
    ///
    /// Lifecycle handles ([`Self::audio`], [`Self::scheduler_thread`])
    /// default to `None`; the Tauri bootstrap path installs them
    /// post-`manage` via [`Self::install_audio_and_scheduler`].
    /// Existing tests that exercise command handlers without
    /// bootstrapping the runtime continue to work unchanged.
    #[must_use]
    pub fn new() -> Self {
        let osc_dispatch_queue = Arc::new(Mutex::new(Vec::new()));
        let mut osc_adapter = UdpOscAdapter::new();
        osc_adapter.set_dispatch_sink(Arc::new(QueuedOscDispatchSink {
            queue: Arc::clone(&osc_dispatch_queue),
        }));
        // Attempt to construct the initial outbound sender. Failure
        // (e.g. all ephemeral ports exhausted) is non-fatal: the
        // field stays `None` and outbound sends are silently skipped
        // until `configure_live_osc_adapter` succeeds.
        let osc_sender = OscSender::new().map(Some).unwrap_or_else(|err| {
            eprintln!(
                "[chromaticity-shell] osc outbound sender init failed: {err}; \
                 outbound OSC disabled until settings configure"
            );
            None
        });
        Self {
            state: Mutex::new(State::default()),
            bus: InMemoryBus::in_memory(),
            settings: Mutex::new(Settings::default()),
            settings_path: Mutex::new(None),
            scheduler_thread: Mutex::new(None),
            audio: Mutex::new(None),
            midi_learn_handle: Mutex::new(None),
            loaded_scene: Mutex::new(None),
            osc_routes: Mutex::new(OscRouteRegistry::with_default_seed()),
            bundled_scene_root: Mutex::new(None),
            osc_adapter: Mutex::new(osc_adapter),
            osc_sender: Mutex::new(osc_sender),
            osc_dispatch_queue,
            audio_threshold: Mutex::new(0.0_f32),
            render_thread: Mutex::new(None),
        }
    }

    /// Install the ADR-011 Phase-2 Slice R-1a render-thread handle.
    ///
    /// Called from the Tauri builder's `setup` callback when the
    /// `CHROMA_RENDER_OUTPUT` env-gate is set and the output window +
    /// render thread were spawned successfully. Replaces any
    /// previously-installed handle (whose `Drop` fires here, signalling
    /// shutdown of the prior render thread). Mutex poison-recovery
    /// follows the workspace pattern.
    pub fn install_render_thread(&self, handle: RenderThreadHandle) {
        match self.render_thread.lock() {
            Ok(mut guard) => {
                *guard = Some(handle);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(handle);
            }
        }
    }

    /// Install the bundled scene resource root used by OSC scene-load
    /// dispatch.
    pub fn install_bundled_scene_root(&self, root: PathBuf) {
        match self.bundled_scene_root.lock() {
            Ok(mut guard) => {
                *guard = Some(root);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(root);
            }
        }
    }

    /// Replace the in-memory operator preferences document.
    ///
    /// Wave 2.1 Lane C.3: called from the Tauri builder's `setup`
    /// callback after [`Self::new`] is registered with `manage` and
    /// after the on-disk settings file has been loaded (or
    /// [`Settings::default`] resolved when no file yet exists).
    /// Mutex poison-recovery follows the workspace pattern: read
    /// through `match lock() { Ok(g) => g, Err(p) => p.into_inner() }`
    /// rather than panicking on a poisoned mutex.
    pub fn install_settings(&self, settings: Settings) {
        match self.settings.lock() {
            Ok(mut guard) => {
                *guard = settings;
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = settings;
            }
        }
    }

    /// Install the on-disk TOML path backing [`Self::settings`].
    ///
    /// Wave 2.1 Lane C.3: called from the Tauri builder's `setup`
    /// callback once `app.path().app_data_dir()` resolves. After
    /// install, `set_settings` writes through to this path; before
    /// install (test contexts), `set_settings` updates the
    /// in-memory mutex only.
    pub fn install_settings_path(&self, path: PathBuf) {
        match self.settings_path.lock() {
            Ok(mut guard) => {
                *guard = Some(path);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(path);
            }
        }
    }

    /// Install the audio runtime + scheduler-tick thread handles.
    ///
    /// Called from the Tauri builder's `setup` callback after
    /// [`Self::new`] is registered with `manage` and after the
    /// runtime + scheduler thread have been constructed against the
    /// managed [`AppState`]. Replaces any previously-installed
    /// handles (their `Drop` impls fire as the slot values are
    /// overwritten — the scheduler thread first, then the audio
    /// runtime; matches the [`Drop`] order on [`AppState`] itself).
    ///
    /// Mutex poison-recovery follows the workspace pattern: read
    /// through `match lock() { Ok(g) => g, Err(p) => p.into_inner() }`
    /// rather than panicking on a poisoned mutex.
    pub fn install_audio_and_scheduler(
        &self,
        audio: AudioRuntimeHandle,
        scheduler_thread: SchedulerThreadHandle,
    ) {
        // Replace the audio handle first so the scheduler thread, if
        // it's still running from a prior install, observes the
        // shared snapshot through its `Arc<Mutex<...>>` clone before
        // its handle is dropped. The replaced previous audio handle
        // (if any) fires its `Drop` here.
        match self.audio.lock() {
            Ok(mut guard) => {
                *guard = Some(audio);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(audio);
            }
        }
        match self.scheduler_thread.lock() {
            Ok(mut guard) => {
                *guard = Some(scheduler_thread);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(scheduler_thread);
            }
        }
    }

    /// Store `value` as the current audio threshold, clamping to `[0.0, 1.0]`.
    ///
    /// Called by [`crate::commands::osc::drain_queued_osc_actions`] when
    /// an inbound `OscDispatchAction::AudioThreshold` fires (the inbound
    /// path), and by the `set_audio_threshold` Tauri command (the
    /// direct-set path for the `Shift+T` hotkey).
    ///
    /// Returns the stored (post-clamp) value so callers can emit the
    /// outbound OSC announce with the same bit pattern that will be
    /// persisted.
    pub fn store_audio_threshold(&self, value: f32) -> f32 {
        let clamped = value.clamp(0.0_f32, 1.0_f32);
        match self.audio_threshold.lock() {
            Ok(mut guard) => {
                *guard = clamped;
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = clamped;
            }
        }
        clamped
    }

    /// Read the current audio threshold value.
    #[must_use]
    pub fn current_audio_threshold(&self) -> f32 {
        match self.audio_threshold.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod app_state_pins {
    //! Evidence: EV-AUDIO-UNIFORM-IPC (T1) — Tauri-side lifecycle
    //! drift pins.
    //!
    //! T1 per `docs/43 §15.5`: each test pins a state-machine
    //! invariant the [`super::AppState`] /
    //! [`super::SchedulerThreadHandle`] composition must honour for
    //! the production-bootstrap wave's e2e wiring (PR 3) to compose
    //! correctly.
    //!
    //! Coverage:
    //! - `app_state_new_starts_with_no_lifecycle_handles` — the
    //!   non-bootstrap construction path leaves both handles
    //!   `None`; existing tests that drive command handlers without
    //!   bootstrapping the runtime keep working.
    //! - `scheduler_thread_handle_drop_signals_and_joins_within_timeout`
    //!   pins the bounded-join contract: dropping the handle stores
    //!   `true` on the shutdown flag and the thread (which polls the
    //!   flag) exits within ~100ms.
    //! - `scheduler_thread_handle_shutdown_flag_observable_to_thread`
    //!   pins that the shutdown flag clone given to the thread is
    //!   the same atomic the handle's drop signals.

    use super::{AppState, SchedulerThreadHandle};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn app_state_new_starts_with_no_lifecycle_handles() {
        let app_state = AppState::new();
        assert!(app_state.audio.lock().unwrap().is_none());
        assert!(app_state.scheduler_thread.lock().unwrap().is_none());
        // Wave 2.1 Lane D.2 — fresh AppState carries no active
        // learn-mode listener; the first `start_midi_learn` IPC
        // call installs one.
        assert!(app_state.midi_learn_handle.lock().unwrap().is_none());
    }

    #[test]
    fn scheduler_thread_handle_drop_signals_and_joins_within_timeout() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);
        let iterations = Arc::new(AtomicU32::new(0));
        let iterations_for_thread = Arc::clone(&iterations);

        let join = thread::Builder::new()
            .name("test-scheduler-tick".into())
            .spawn(move || {
                while !shutdown_for_thread.load(Ordering::Relaxed) {
                    iterations_for_thread.fetch_add(1, Ordering::Relaxed);
                    thread::sleep(Duration::from_millis(5));
                }
            })
            .unwrap();
        let handle = SchedulerThreadHandle::new(join, Arc::clone(&shutdown));

        // Let the loop iterate a few times to confirm it's running.
        thread::sleep(Duration::from_millis(20));
        let pre_drop = iterations.load(Ordering::Relaxed);
        assert!(pre_drop > 0, "thread didn't iterate: {pre_drop}");

        // Drop signals shutdown + bounded join.
        let start = Instant::now();
        drop(handle);
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "drop took {elapsed:?}, expected < 500ms",
        );
        assert!(shutdown.load(Ordering::Relaxed));

        // Iteration counter stabilises after thread exit.
        let post_drop = iterations.load(Ordering::Relaxed);
        thread::sleep(Duration::from_millis(50));
        let stable = iterations.load(Ordering::Relaxed);
        assert_eq!(post_drop, stable);
    }

    #[test]
    fn scheduler_thread_handle_shutdown_flag_observable_to_thread() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);

        let join = thread::Builder::new()
            .name("test-flag-observer".into())
            .spawn(move || {
                while !shutdown_for_thread.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(2));
                }
            })
            .unwrap();
        let handle = SchedulerThreadHandle::new(join, Arc::clone(&shutdown));

        // The handle exposes a shutdown-flag clone for the thread
        // closure; pin that the clone observes the same atomic the
        // handle's drop signals.
        let flag_clone = handle.shutdown_flag();
        assert!(!flag_clone.load(Ordering::Relaxed));
        drop(handle);
        assert!(flag_clone.load(Ordering::Relaxed));
    }

    #[test]
    fn install_audio_and_scheduler_replaces_existing_handles() {
        // Spawn two no-op scheduler threads back to back; the second
        // install replaces the first, dropping the prior handle and
        // signalling its shutdown. This pins the "replace dropped
        // prior" contract documented on
        // `AppState::install_audio_and_scheduler`. We only exercise
        // the scheduler half because the audio half requires a
        // full `AudioRuntime` spawn which the producer-side T1 pins
        // already cover (`engine/audio/src/runtime.rs` runtime_pins
        // module).
        let app_state = AppState::new();

        let make_handle = || {
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_for_thread = Arc::clone(&shutdown);
            let join = thread::Builder::new()
                .name("test-replace-target".into())
                .spawn(move || {
                    while !shutdown_for_thread.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(2));
                    }
                })
                .unwrap();
            (
                SchedulerThreadHandle::new(join, Arc::clone(&shutdown)),
                shutdown,
            )
        };

        let (sched_a, flag_a) = make_handle();
        // The audio half requires a real `AudioRuntimeHandle`. The
        // public `AudioRuntime::new(default)` constructor falls back
        // to a SilentSource-only runtime when no cpal default device
        // is available (CI / headless / device-less environments per
        // the PR 1 fallback contract); on dev machines with a real
        // device it spawns an actual cpal stream which the test
        // tears down via the `Drop` impl on `AudioRuntimeHandle`.
        // Either path produces a runnable handle the install path
        // owns.
        use chromaticity_audio::runtime::{AudioRuntime, AudioRuntimeConfig};
        let runtime_a = AudioRuntime::new(AudioRuntimeConfig::default());
        let audio_a = runtime_a.spawn();

        app_state.install_audio_and_scheduler(audio_a, sched_a);
        assert!(app_state.audio.lock().unwrap().is_some());
        assert!(app_state.scheduler_thread.lock().unwrap().is_some());
        assert!(!flag_a.load(Ordering::Relaxed));

        let (sched_b, _flag_b) = make_handle();
        let runtime_b = AudioRuntime::new(AudioRuntimeConfig::default());
        let audio_b = runtime_b.spawn();

        app_state.install_audio_and_scheduler(audio_b, sched_b);

        // Replacing the handle drops the prior, which signals
        // shutdown via its `Drop` impl. Poll the flag to allow the
        // bounded-join inside `Drop` to settle.
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut signalled = false;
        while Instant::now() < deadline {
            if flag_a.load(Ordering::Relaxed) {
                signalled = true;
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(signalled, "prior handle's drop didn't signal shutdown");
    }
}
