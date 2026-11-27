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
//! [`AppState`] owns an explicit [`ShellRuntime`] lifecycle state
//! machine:
//!
//! - [`ShellRuntime::Unbootstrapped`] — initial state from
//!   [`AppState::new`], leaving background threads unspawned so
//!   existing tests calling [`AppState::new`] continue to construct a
//!   runtime-free shell.
//! - [`ShellRuntime::Running`] — holds both the spawned
//!   [`SchedulerThreadHandle`] (driving
//!   [`chromaticity_runtime::scheduler::Scheduler::tick`] at ~60 Hz)
//!   and the producer-side [`AudioRuntimeHandle`] (spawned via
//!   [`AudioRuntime::spawn`]). Installed by the Tauri bootstrap path
//!   (`app/shell/src/lib.rs::run`) via
//!   [`AppState::install_audio_and_scheduler`] from inside the Tauri
//!   builder's `setup` callback, after [`AppState::new`] has already
//!   been registered with `manage`.
//! - [`ShellRuntime::Stopped`] — terminal state entered after
//!   explicit shutdown or drop.
//!
//! ## Ordered shutdown contract
//!
//! Teardown ordering at app shutdown is enforced in code inside
//! [`ShellRuntime::shutdown`] (invoked by both [`ShellRuntime::drop`]
//! and [`AppState::drop`]), NOT by struct field declaration order:
//!
//! 1. **Drop `scheduler_thread` first.** The state-commit thread must
//!    stop touching `AppState` before any other field is dropped —
//!    otherwise the thread could observe a half-dropped [`AppState`].
//!    [`SchedulerThreadHandle::drop`] signals shutdown and best-effort
//!    joins on a 2-second timeout.
//! 2. **Drop `audio` second.** The producer thread is independent of
//!    the scheduler; [`AudioRuntimeHandle::drop`] performs its own
//!    best-effort shutdown + 2-second join.

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
use chromaticity_ui_bridge::CrashResumeStatusPayload;
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
/// Managed as part of [`ShellRuntime::Running`], whose explicit
/// [`ShellRuntime::shutdown`] / [`Drop`] tears this handle down
/// BEFORE the audio runtime handle.
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

/// Explicit lifecycle state machine for the shell's background audio
/// producer and scheduler-tick threads.
///
/// Replaces the implicit pair of optional mutexes whose teardown order
/// previously depended on struct field declaration order. Both
/// [`Self::shutdown`] and [`Drop`] explicitly tear down
/// `scheduler_thread` BEFORE `audio` in code.
#[derive(Default)]
pub enum ShellRuntime {
    /// Initial state prior to Tauri bootstrap (or in runtime-free tests).
    #[default]
    Unbootstrapped,
    /// Active production runtime owning both background thread handles.
    Running {
        scheduler_thread: SchedulerThreadHandle,
        audio: AudioRuntimeHandle,
    },
    /// Explicitly shut down or dropped.
    Stopped,
}

impl core::fmt::Debug for ShellRuntime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unbootstrapped => f.write_str("ShellRuntime::Unbootstrapped"),
            Self::Running {
                scheduler_thread, ..
            } => f
                .debug_struct("ShellRuntime::Running")
                .field("scheduler_thread", scheduler_thread)
                .field("audio", &"<AudioRuntimeHandle>")
                .finish(),
            Self::Stopped => f.write_str("ShellRuntime::Stopped"),
        }
    }
}

impl ShellRuntime {
    /// Deterministic state label used by diagnostics and
    /// `inspect_state_machines`.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        match self {
            Self::Unbootstrapped => "unbootstrapped",
            Self::Running { .. } => "running",
            Self::Stopped => "stopped",
        }
    }

    /// Return `true` iff both background handles are currently active.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    /// Transition to [`Self::Stopped`], tearing down `scheduler_thread`
    /// BEFORE `audio` in code.
    ///
    /// Idempotent: calling `shutdown` when already `Stopped` (or
    /// `Unbootstrapped`) leaves the state machine in `Stopped` without
    /// touching any handles.
    pub fn shutdown(&mut self) {
        self.stop_scheduler_first();
        // Dropping the previous `Running` value invokes `Drop`, where
        // `stop_scheduler_first` is a no-op (join already taken) and
        // `audio` tears down second.
        *self = Self::Stopped;
    }

    fn stop_scheduler_first(&mut self) {
        if let Self::Running {
            scheduler_thread, ..
        } = self
        {
            // Explicit ordered teardown in code:
            // 1. Stop and join the scheduler-tick thread first so it
            //    stops accessing AppState before the audio runtime
            //    tears down.
            scheduler_thread.shutdown_inner();
        }
    }
}

impl Drop for ShellRuntime {
    fn drop(&mut self) {
        // 1. Explicitly stop and join `scheduler_thread` BEFORE Rust
        //    drops the variant's fields (including `audio`).
        self.stop_scheduler_first();
        // 2. Rust then drops the variant's fields (`scheduler_thread`
        //    is already joined and a no-op; `audio` tears down second).
    }
}

/// Tauri-managed shell state.
///
/// Background thread teardown ordering is enforced explicitly in
/// [`AppState::drop`] and [`ShellRuntime::shutdown`] (scheduler thread
/// first, audio runtime second) rather than relying on struct field
/// declaration order.
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
    /// Explicit lifecycle state machine owning the background
    /// scheduler-tick thread and audio producer runtime handles.
    /// Starts in [`ShellRuntime::Unbootstrapped`] in [`AppState::new`];
    /// [`AppState::install_audio_and_scheduler`] transitions it to
    /// [`ShellRuntime::Running`]; [`AppState::shutdown_runtime`] and
    /// [`AppState::drop`] transition it to [`ShellRuntime::Stopped`],
    /// explicitly dropping `scheduler_thread` before `audio`.
    pub runtime: Mutex<ShellRuntime>,
    /// Crash-resume recovery status recorded during Tauri bootstrap.
    ///
    /// `Some(payload)` when a saved crash-resume snapshot was found
    /// and restored on launch; `None` when starting fresh or when
    /// recovery failed. Owned directly by [`AppState`] rather than a
    /// process-global static so each instance is isolated and testable.
    pub crash_resume_status: Mutex<Option<CrashResumeStatusPayload>>,
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
    /// Live GPU preview renderer (`wgpu` + `naga` GLSL 4.50).
    pub gpu_renderer: Mutex<crate::render::GpuPreviewEngine>,
    /// Active entitlement token installed via `install_license` (ADR-009).
    pub entitlement_token: Mutex<Option<chromaticity_entitlement::Token>>,
    /// Monotonic process-start timestamp for shader `chroma_Time.x`.
    pub started_at: Instant,
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
    /// [`Self::runtime`] starts in [`ShellRuntime::Unbootstrapped`];
    /// the Tauri bootstrap path transitions it to
    /// [`ShellRuntime::Running`] post-`manage` via
    /// [`Self::install_audio_and_scheduler`]. Existing tests that
    /// exercise command handlers without bootstrapping the runtime
    /// continue to work unchanged.
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
            runtime: Mutex::new(ShellRuntime::Unbootstrapped),
            crash_resume_status: Mutex::new(None),
            midi_learn_handle: Mutex::new(None),
            loaded_scene: Mutex::new(None),
            osc_routes: Mutex::new(OscRouteRegistry::with_default_seed()),
            bundled_scene_root: Mutex::new(None),
            osc_adapter: Mutex::new(osc_adapter),
            osc_sender: Mutex::new(osc_sender),
            osc_dispatch_queue,
            audio_threshold: Mutex::new(0.0_f32),
            gpu_renderer: Mutex::new(crate::render::GpuPreviewEngine::new()),
            entitlement_token: Mutex::new(None),
            started_at: Instant::now(),
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
    /// [`ShellRuntime::Running`] handles by calling
    /// [`ShellRuntime::shutdown`] first (tearing down `scheduler_thread`
    /// before `audio` in code).
    ///
    /// Mutex poison-recovery follows the workspace pattern: read
    /// through `match lock() { Ok(g) => g, Err(p) => p.into_inner() }`
    /// rather than panicking on a poisoned mutex.
    pub fn install_audio_and_scheduler(
        &self,
        audio: AudioRuntimeHandle,
        scheduler_thread: SchedulerThreadHandle,
    ) {
        let mut guard = match self.runtime.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if guard.is_running() {
            guard.shutdown();
        }
        *guard = ShellRuntime::Running {
            scheduler_thread,
            audio,
        };
    }

    /// Explicitly transition [`Self::runtime`] to [`ShellRuntime::Stopped`],
    /// tearing down `scheduler_thread` before `audio`.
    pub fn shutdown_runtime(&self) {
        let mut guard = match self.runtime.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.shutdown();
    }

    /// Return the deterministic state name of [`Self::runtime`]
    /// (`"unbootstrapped"`, `"running"`, or `"stopped"`).
    #[must_use]
    pub fn runtime_state_name(&self) -> &'static str {
        let guard = match self.runtime.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.state_name()
    }

    /// Return `true` iff [`Self::runtime`] is currently in
    /// [`ShellRuntime::Running`].
    #[must_use]
    pub fn is_running(&self) -> bool {
        let guard = match self.runtime.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.is_running()
    }

    /// Record (or clear) the crash-resume recovery status payload.
    pub fn set_crash_resume_status(&self, status: Option<CrashResumeStatusPayload>) {
        match self.crash_resume_status.lock() {
            Ok(mut guard) => {
                *guard = status;
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = status;
            }
        }
    }

    /// Read a clone of the current crash-resume recovery status payload.
    #[must_use]
    pub fn crash_resume_status(&self) -> Option<CrashResumeStatusPayload> {
        match self.crash_resume_status.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Return a deterministic JSON snapshot of every shell state machine.
    #[must_use]
    pub fn inspect_state_machines_snapshot(&self) -> serde_json::Value {
        let runtime = self.runtime_state_name();
        let loaded_scene = {
            let guard = match self.loaded_scene.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            match guard.as_ref() {
                Some(record) => record
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
                None => "none".to_string(),
            }
        };
        let midi_learn = {
            let guard = match self.midi_learn_handle.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if guard.is_some() { "listening" } else { "idle" }
        };
        let osc_sender = {
            let guard = match self.osc_sender.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if guard.is_some() {
                "ready"
            } else {
                "unavailable"
            }
        };
        let crash_resume = {
            let guard = match self.crash_resume_status.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if guard.is_some() { "recovered" } else { "none" }
        };
        let settings_path_installed = {
            let guard = match self.settings_path.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            guard.is_some()
        };
        let gpu_render = {
            let guard = match self.gpu_renderer.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            guard.state_name()
        };
        let entitlement = {
            let guard = match self.entitlement_token.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if guard.is_some() {
                "installed"
            } else {
                "unlicensed"
            }
        };
        serde_json::json!({
            "runtime": runtime,
            "loaded_scene": loaded_scene,
            "gpu_render": gpu_render,
            "entitlement": entitlement,
            "midi_learn": midi_learn,
            "osc_sender": osc_sender,
            "crash_resume": crash_resume,
            "settings_path_installed": settings_path_installed,
        })
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

impl Drop for AppState {
    fn drop(&mut self) {
        // Ensure background threads are torn down (scheduler_thread
        // before audio) BEFORE any other AppState field is dropped.
        self.shutdown_runtime();
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

    use super::{AppState, LoadedSceneRecord, SchedulerThreadHandle, ShellRuntime};
    use chromaticity_audio::runtime::{AudioRuntime, AudioRuntimeConfig};
    use chromaticity_scene::loader::load_chroma_dir;
    use chromaticity_ui_bridge::{CrashResumeStatusPayload, ResumeStatusPayload};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    fn spawn_test_scheduler(name: &str) -> (SchedulerThreadHandle, Arc<AtomicBool>) {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);
        let join = thread::Builder::new()
            .name(name.into())
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
    }

    #[test]
    fn app_state_new_starts_with_no_lifecycle_handles() {
        let app_state = AppState::new();
        assert!(!app_state.is_running());
        assert_eq!(app_state.runtime_state_name(), "unbootstrapped");
        assert!(app_state.crash_resume_status().is_none());
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
        let app_state = AppState::new();

        let (sched_a, flag_a) = spawn_test_scheduler("test-replace-target-a");
        let runtime_a = AudioRuntime::new(AudioRuntimeConfig::default());
        let audio_a = runtime_a.spawn();

        app_state.install_audio_and_scheduler(audio_a, sched_a);
        assert!(app_state.is_running());
        assert_eq!(app_state.runtime_state_name(), "running");
        assert!(!flag_a.load(Ordering::Relaxed));

        let (sched_b, _flag_b) = spawn_test_scheduler("test-replace-target-b");
        let runtime_b = AudioRuntime::new(AudioRuntimeConfig::default());
        let audio_b = runtime_b.spawn();

        app_state.install_audio_and_scheduler(audio_b, sched_b);
        assert!(app_state.is_running());
        assert_eq!(app_state.runtime_state_name(), "running");

        // Replacing the running runtime calls `ShellRuntime::shutdown`,
        // which signals and joins the prior scheduler thread before
        // tearing down the prior audio runtime.
        assert!(
            flag_a.load(Ordering::Relaxed),
            "prior handle's shutdown didn't signal before replacement completed"
        );
    }

    #[test]
    fn shell_runtime_transitions_and_ordered_shutdown() {
        let mut rt = ShellRuntime::Unbootstrapped;
        assert_eq!(rt.state_name(), "unbootstrapped");
        assert!(!rt.is_running());

        let (sched, sched_flag) = spawn_test_scheduler("test-shell-runtime-order");
        let audio = AudioRuntime::new(AudioRuntimeConfig::default()).spawn();
        rt = ShellRuntime::Running {
            scheduler_thread: sched,
            audio,
        };
        assert_eq!(rt.state_name(), "running");
        assert!(rt.is_running());
        assert!(!sched_flag.load(Ordering::Relaxed));

        // Explicit shutdown transitions to Stopped and joins scheduler
        // before dropping audio.
        rt.shutdown();
        assert_eq!(rt.state_name(), "stopped");
        assert!(!rt.is_running());
        assert!(sched_flag.load(Ordering::Relaxed));

        // Idempotent second shutdown stays Stopped.
        rt.shutdown();
        assert_eq!(rt.state_name(), "stopped");
    }

    #[test]
    fn app_state_drop_shuts_down_running_runtime() {
        let (sched, sched_flag) = spawn_test_scheduler("test-app-state-drop");
        let audio = AudioRuntime::new(AudioRuntimeConfig::default()).spawn();
        let app_state = AppState::new();
        app_state.install_audio_and_scheduler(audio, sched);
        assert!(app_state.is_running());
        assert!(!sched_flag.load(Ordering::Relaxed));

        drop(app_state);
        assert!(
            sched_flag.load(Ordering::Relaxed),
            "AppState::drop must signal and join scheduler_thread"
        );
    }

    #[test]
    fn crash_resume_status_is_isolated_per_app_state() {
        let a = AppState::new();
        let b = AppState::new();
        assert!(a.crash_resume_status().is_none());
        assert!(b.crash_resume_status().is_none());

        let payload = CrashResumeStatusPayload {
            show_name: "Recovered runtime state".to_string(),
            controls: ResumeStatusPayload::Restored,
            snaps: ResumeStatusPayload::Restored,
            playlist: ResumeStatusPayload::Restored,
            adapters: ResumeStatusPayload::Restored,
            scene: ResumeStatusPayload::Partial,
            p50: 12,
        };
        a.set_crash_resume_status(Some(payload.clone()));

        assert_eq!(a.crash_resume_status(), Some(payload));
        assert!(
            b.crash_resume_status().is_none(),
            "crash_resume_status must not leak across AppState instances"
        );

        a.set_crash_resume_status(None);
        assert!(a.crash_resume_status().is_none());
    }

    #[test]
    fn inspect_state_machines_snapshot_reflects_all_shell_states() {
        let app_state = AppState::new();
        let initial = app_state.inspect_state_machines_snapshot();
        assert_eq!(initial["runtime"], "unbootstrapped");
        assert_eq!(initial["loaded_scene"], "none");
        assert_eq!(initial["midi_learn"], "idle");
        assert!(
            initial["osc_sender"] == "ready" || initial["osc_sender"] == "unavailable",
            "unexpected osc_sender state: {}",
            initial["osc_sender"]
        );
        assert_eq!(initial["crash_resume"], "none");
        assert_eq!(initial["settings_path_installed"], false);

        // Transition runtime -> running, settings_path -> installed,
        // crash_resume -> recovered, loaded_scene -> demo-kaleido,
        // osc_sender -> unavailable.
        let (sched, _flag) = spawn_test_scheduler("test-inspect-sm");
        let audio = AudioRuntime::new(AudioRuntimeConfig::default()).spawn();
        app_state.install_audio_and_scheduler(audio, sched);
        app_state.install_settings_path(PathBuf::from("/tmp/settings.toml"));
        app_state.set_crash_resume_status(Some(CrashResumeStatusPayload {
            show_name: "Recovered".to_string(),
            controls: ResumeStatusPayload::Restored,
            snaps: ResumeStatusPayload::Restored,
            playlist: ResumeStatusPayload::Restored,
            adapters: ResumeStatusPayload::Restored,
            scene: ResumeStatusPayload::Partial,
            p50: 4,
        }));
        let scene_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../engine/scene/tests/fixtures/scenes/library/demo-kaleido.chroma");
        let scene = load_chroma_dir(&scene_path).unwrap();
        *app_state.loaded_scene.lock().unwrap() = Some(LoadedSceneRecord {
            scene,
            path: scene_path,
        });
        *app_state.osc_sender.lock().unwrap() = None;

        let active = app_state.inspect_state_machines_snapshot();
        assert_eq!(active["runtime"], "running");
        assert_eq!(active["loaded_scene"], "demo-kaleido");
        assert_eq!(active["midi_learn"], "idle");
        assert_eq!(active["osc_sender"], "unavailable");
        assert_eq!(active["crash_resume"], "recovered");
        assert_eq!(active["settings_path_installed"], true);

        // Transition runtime -> stopped.
        app_state.shutdown_runtime();
        let stopped = app_state.inspect_state_machines_snapshot();
        assert_eq!(stopped["runtime"], "stopped");
    }
}
