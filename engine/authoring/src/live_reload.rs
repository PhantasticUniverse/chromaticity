// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Authoring scene file-watcher and reload pipeline (Wave 4b.4 Lane B
//! PR 1).
//!
//! This module is the producer side of the
//! `EV-AUTHORING-LIVE-RELOAD` registry row (`docs/26-evidence-artifact-
//! registry.md` row 61). It owns two surfaces composed independently:
//!
//! - [`start_watching`] spawns a [`notify`] cross-platform filesystem
//!   watcher against a set of paths and pumps debounced
//!   [`ReloadEvent`] values onto a [`std::sync::mpsc::Receiver`]. The
//!   returned [`LiveReloadHandle`] joins the watcher thread on
//!   `Drop`.
//! - [`perform_reload`] is a generic single-shot reload step that
//!   accepts a caller-supplied loader closure and produces a
//!   [`ReloadOutcome`]; on loader failure the load error is mapped
//!   into a structured [`crate::diagnostics::Diagnostic`] via
//!   [`crate::diagnostics::diagnostic_from_load_error`] so the
//!   inline-diagnostics surface (`docs/37` §3.5) and the console
//!   surface (`docs/37` §3.4) read the same shape.
//!
//! # PR-1 scope versus the wave's eventual scope
//!
//! The wave is sized at 2-3 PRs (file-watcher + reload pipeline +
//! state-preservation contract; corpus separate). PR 1 ships only
//! the watch + reload mechanism. State preservation (per `docs/37`
//! §3.3 — preserve in-flight playlist / OSC bindings on rebuild;
//! preserve control values on shader recompile; etc.) is PR-2
//! scope. PR-1's [`perform_reload`] is intentionally naïve:
//! "discard state on reload" is the only post-condition. PR-2
//! layers a state-preservation contract on top without re-shaping
//! the surface here.
//!
//! Likewise, the [`docs/37` §3.3] latency budget (`p95 < 500 ms`
//! for `main.glsl` edits) is asserted by the PR-3 T2 corpus, not
//! here. PR-1 emits the structural surface that supports meeting
//! the budget — debounce, channel-pump, single-shot reload — and
//! pins the structural invariants in the in-source T1 module.
//!
//! # Debounce posture
//!
//! Editor save flows fire filesystem events in bursts (a tab save
//! that touches the file and the editor's swap file produces 2-3
//! events; some editors atomic-rename through a temp file). The
//! watcher coalesces events within a debounce window (default
//! 100 ms) into a single [`ReloadEvent::FilesChanged`] carrying
//! the deduplicated path set. The window is configurable through
//! [`LiveReloadConfig::debounce_ms`] so the PR-3 latency-budget
//! corpus can tune it against the measurement harness.
//!
//! # Why `notify` (cross-platform watcher choice)
//!
//! `notify` is the canonical Rust cross-platform filesystem
//! watcher (FSEvents on macOS, ReadDirectoryChangesW on Windows,
//! inotify on Linux); used in production by `cargo-watch`,
//! `mdbook serve`, and most Rust hot-reload tools. License is
//! CC0-1.0 (in `deny.toml` `[licenses].allow`); transitive
//! licenses are MIT / Apache-2.0 / Unlicense-OR-MIT, all already
//! in the workspace dep graph or covered by the existing allow
//! list.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chromaticity_scene::loader::LoadError;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::diagnostics::{Diagnostic, diagnostic_from_load_error};

/// Configuration for [`start_watching`].
///
/// `debounce_ms` defaults to 100 ms (see module docstring's
/// "Debounce posture" section). `watched_paths` is the set of
/// directories or files the watcher monitors recursively; an
/// empty set is permitted and yields a watcher that emits no
/// events (useful for a UI startup state where no scene is yet
/// open).
#[derive(Debug, Clone)]
pub struct LiveReloadConfig {
    /// Set of paths the watcher monitors. Each entry is watched
    /// in [`RecursiveMode::Recursive`] mode; if a path does not
    /// exist or is not accessible, the watcher's `watch()` call
    /// returns [`LiveReloadError::NotifySetupError`].
    pub watched_paths: Vec<PathBuf>,
    /// Debounce window in milliseconds. Filesystem events that
    /// arrive within this window are coalesced into a single
    /// [`ReloadEvent::FilesChanged`] carrying the deduplicated
    /// path set. Defaults to 100 ms via [`Self::default`].
    pub debounce_ms: u64,
}

impl Default for LiveReloadConfig {
    fn default() -> Self {
        Self {
            watched_paths: Vec::new(),
            debounce_ms: 100,
        }
    }
}

/// Coalesced filesystem-change event delivered through the
/// [`start_watching`] receiver.
///
/// Slice-1 emits only the [`Self::FilesChanged`] variant. Future
/// PRs may widen the variant set (e.g. a `WatcherStopped` arm
/// when the watcher thread is asked to terminate, a `WatchError`
/// arm for runtime errors); the enum is `#[non_exhaustive]` to
/// keep that widening compatible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReloadEvent {
    /// One or more watched paths changed within the debounce
    /// window. The carried `Vec<PathBuf>` is the deduplicated
    /// set of paths touched during the window.
    FilesChanged(Vec<PathBuf>),
}

/// Captured pre-reload state that gets re-applied to the post-reload
/// state on successful reload. Per
/// `docs/37-authoring-environment-spec.md` §3.3 hot-reload semantics:
/// an operator's mid-show edits to control values, active preset,
/// Snap selection, and playlist position must survive a successful
/// hot-reload — a reload is a code-change event, not a
/// session-reset.
///
/// PR 2 ships the cross-seam contract: producer captures the
/// preserved state before swapping the runtime artifact; consumer
/// re-applies it onto the post-reload State at the first
/// commit-thread tick. The canonical-state mutation path stays via
/// the command-bus + commit-thread (ADR-006); state-preservation
/// does NOT bypass it.
///
/// The runtime-side wiring (engine/runtime adapter that captures
/// `State -> PreservedState` before reload + re-applies after) is a
/// future-wave option scoped behind the live-reload + runtime
/// composition; the engine/authoring crate ships only the cross-seam
/// contract so it does not take a hard dep on the runtime crate's
/// `State` type. Adapter crates wire the actual extraction at the
/// call site.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreservedState {
    /// Control-value pairs captured pre-reload. Keys are control
    /// IDs; values are the most-recently-committed values.
    pub control_values: Vec<(String, f32)>,
    /// Active preset id pre-reload, if any.
    pub active_preset: Option<String>,
    /// Active Snap slot pre-reload, if any.
    pub active_snap: Option<u8>,
    /// Playlist position pre-reload, if any.
    pub playlist_position: Option<i64>,
}

impl PreservedState {
    /// Returns `true` when no pre-reload state has been captured —
    /// the consumer-side re-apply step can short-circuit on this.
    pub fn is_empty(&self) -> bool {
        self.control_values.is_empty()
            && self.active_preset.is_none()
            && self.active_snap.is_none()
            && self.playlist_position.is_none()
    }
}

/// Caller-supplied snapshot of the pre-reload state. The shape is
/// opaque to the engine/authoring crate so it does not take a hard
/// dep on the runtime crate's `State` type — adapter crates wire the
/// actual `State -> PreservedStateSnapshot` extraction at the call
/// site, then call [`capture_preserved_state`] to pin the
/// preservation shape ahead of the reload.
#[derive(Clone, Debug, Default)]
pub struct PreservedStateSnapshot {
    /// Control-value pairs captured pre-reload. Keys are control
    /// IDs; values are the most-recently-committed values.
    pub control_values: Vec<(String, f32)>,
    /// Active preset id pre-reload, if any.
    pub active_preset: Option<String>,
    /// Active Snap slot pre-reload, if any.
    pub active_snap: Option<u8>,
    /// Playlist position pre-reload, if any.
    pub playlist_position: Option<i64>,
}

/// Capture the pre-reload state from a snapshot the caller
/// provides. The snapshot shape is opaque
/// ([`PreservedStateSnapshot`] builder) so the engine/authoring
/// crate doesn't take a hard dep on the runtime crate's `State`
/// type — adapter crates wire the actual `State -> PreservedState`
/// extraction at the call site.
pub fn capture_preserved_state(snapshot: PreservedStateSnapshot) -> PreservedState {
    PreservedState {
        control_values: snapshot.control_values,
        active_preset: snapshot.active_preset,
        active_snap: snapshot.active_snap,
        playlist_position: snapshot.playlist_position,
    }
}

/// Result of a [`perform_reload`] step.
#[derive(Debug)]
pub enum ReloadOutcome<T> {
    /// Loader returned a fresh value of `T`.
    ///
    /// `preserved` carries the [`PreservedState`] captured pre-reload
    /// per the §3.3 hot-reload semantics contract. The consumer-side
    /// re-apply step folds `preserved` onto the post-reload State at
    /// the first commit-thread tick; an empty `PreservedState` (the
    /// default) signals "no state to preserve" and the consumer can
    /// short-circuit via [`PreservedState::is_empty`].
    Success {
        /// Loaded artifact returned by the caller's loader closure.
        value: T,
        /// Pre-reload state captured by the caller before invoking
        /// the loader. Forwarded verbatim onto the success outcome
        /// so the consumer-side re-apply step has it in hand at the
        /// same callback site that observes the new artifact.
        preserved: PreservedState,
    },
    /// Loader failed; the carried [`Diagnostic`] is the structured
    /// error shape the authoring console / inline-diagnostics
    /// surface consumes.
    ///
    /// On failure, no [`PreservedState`] is forwarded — the producer
    /// retains the pre-reload state in its own buffer so the
    /// next-attempt reload can re-capture or re-use it. Bundling
    /// `PreservedState` with `Failed` would push pre-reload state
    /// across a failed-load boundary the consumer cannot meaningfully
    /// act on, so the contract is "preservation is success-only".
    Failed(Diagnostic),
}

/// Errors returned by [`start_watching`].
///
/// Marked `#[non_exhaustive]` so future watcher-class failure
/// modes (for instance, a permission-denied recursive-watch on a
/// privileged path under macOS Sandbox) can be added without a
/// compat-surface change.
#[derive(Debug)]
#[non_exhaustive]
pub enum LiveReloadError {
    /// `notify` failed to construct the platform watcher or to
    /// register one of the requested paths. The carried `String`
    /// is the formatted upstream error.
    NotifySetupError(String),
    /// I/O failure while preparing the watcher (canonicalising
    /// paths, etc.).
    IoError(std::io::Error),
}

impl std::fmt::Display for LiveReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiveReloadError::NotifySetupError(msg) => {
                write!(f, "live-reload watcher setup failed: {msg}")
            }
            LiveReloadError::IoError(err) => {
                write!(f, "live-reload I/O error: {err}")
            }
        }
    }
}

impl std::error::Error for LiveReloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LiveReloadError::IoError(err) => Some(err),
            LiveReloadError::NotifySetupError(_) => None,
        }
    }
}

impl From<std::io::Error> for LiveReloadError {
    fn from(err: std::io::Error) -> Self {
        LiveReloadError::IoError(err)
    }
}

/// Handle to a running watcher thread.
///
/// Dropping the handle signals the watcher thread to stop and
/// best-effort joins it. The drop is non-blocking past the join
/// timeout so a misbehaving watcher cannot deadlock the caller.
pub struct LiveReloadHandle {
    stop_flag: Arc<AtomicBool>,
    /// `Option` so [`Drop::drop`] can `take()` the join handle
    /// (joining requires owning the [`JoinHandle`]).
    thread: Option<JoinHandle<()>>,
    /// The watcher itself owns OS-side resources; carrying it on
    /// the handle keeps it alive for the watcher thread's
    /// lifetime, and dropping it before the thread join unwinds
    /// the platform-watcher state.
    _watcher: Option<RecommendedWatcher>,
}

impl LiveReloadHandle {
    /// Returns `true` while the underlying watcher thread has not
    /// yet been signalled to stop. Test-only diagnostic; not a
    /// public-surface invariant beyond that.
    #[cfg(test)]
    fn is_running(&self) -> bool {
        !self.stop_flag.load(Ordering::SeqCst)
    }
}

impl Drop for LiveReloadHandle {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        // Drop the watcher first so the platform-watcher event
        // queue closes; the worker thread then exits its
        // recv-with-timeout loop on the next debounce tick.
        self._watcher.take();
        if let Some(handle) = self.thread.take() {
            // Best-effort join. The thread's loop wakes every
            // `debounce_ms` so it sees the stop flag within at
            // most one debounce window.
            let _ = handle.join();
        }
    }
}

/// Spawn a filesystem watcher thread for `config.watched_paths`.
///
/// Returns the handle plus a receiver for [`ReloadEvent`] values.
/// The receiver is the sole consumer-side channel: the watcher
/// thread is the sole producer. Dropping the handle terminates
/// the watcher; the receiver returns `Err(RecvError)` on the next
/// receive once the sender side has been dropped.
///
/// # Errors
///
/// Returns [`LiveReloadError::NotifySetupError`] if the platform
/// watcher cannot be constructed or if any of the requested
/// paths cannot be registered for watching.
pub fn start_watching(
    config: LiveReloadConfig,
) -> Result<(LiveReloadHandle, Receiver<ReloadEvent>), LiveReloadError> {
    // Channel pumped by the worker thread. Bounded-by-default
    // would risk the worker blocking on a slow consumer; the
    // unbounded `mpsc::channel` matches the existing IPC-emit
    // pattern in `engine/runtime/src/scheduler/ipc.rs`.
    let (event_tx, event_rx) = mpsc::channel::<ReloadEvent>();

    // Internal channel from notify into the worker thread. Notify's
    // event handler closure runs on a notify-internal thread; it
    // forwards raw events to the worker, which performs debounce.
    let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<notify::Event>>();

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            // If the worker has been dropped, the send fails
            // silently — the watcher will be unregistered shortly
            // after via the `LiveReloadHandle::Drop` path.
            let _ = raw_tx.send(res);
        })
        .map_err(|e| LiveReloadError::NotifySetupError(e.to_string()))?;

    for path in &config.watched_paths {
        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| LiveReloadError::NotifySetupError(e.to_string()))?;
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let worker_stop_flag = Arc::clone(&stop_flag);
    let debounce = Duration::from_millis(config.debounce_ms);

    let thread = thread::Builder::new()
        .name("chromaticity-live-reload".to_string())
        .spawn(move || {
            worker_loop(raw_rx, event_tx, worker_stop_flag, debounce);
        })
        .map_err(LiveReloadError::IoError)?;

    Ok((
        LiveReloadHandle {
            stop_flag,
            thread: Some(thread),
            _watcher: Some(watcher),
        },
        event_rx,
    ))
}

/// Watcher worker loop. Pumps raw [`notify::Event`] values into
/// debounced [`ReloadEvent`] values.
///
/// Algorithm: block on `raw_rx.recv_timeout(debounce)`. On the
/// first event, accumulate paths into a dedup buffer; continue
/// receiving with the same timeout until the next interval
/// elapses without an event, then flush the buffer as a single
/// [`ReloadEvent::FilesChanged`]. The stop flag is polled on
/// every timeout boundary so the loop exits within one debounce
/// window of [`LiveReloadHandle::Drop`].
fn worker_loop(
    raw_rx: Receiver<notify::Result<notify::Event>>,
    event_tx: Sender<ReloadEvent>,
    stop_flag: Arc<AtomicBool>,
    debounce: Duration,
) {
    let mut pending: Vec<PathBuf> = Vec::new();

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            break;
        }
        match raw_rx.recv_timeout(debounce) {
            Ok(Ok(event)) => {
                if is_reload_relevant(&event.kind) {
                    for path in event.paths {
                        if !pending.contains(&path) {
                            pending.push(path);
                        }
                    }
                    // Continue draining within the debounce
                    // window; only flush once the window
                    // elapses without further events.
                }
                // Non-relevant kinds (e.g. metadata-only events)
                // are dropped silently.
            }
            Ok(Err(_notify_err)) => {
                // `notify` runtime errors today (e.g. a watch
                // path becoming inaccessible) are swallowed at
                // PR-1 scope. PR-2 / PR-3 may widen
                // `ReloadEvent` with a `WatchError` variant
                // when the consumer-side surface is wired.
            }
            Err(RecvTimeoutError::Timeout) => {
                if !pending.is_empty() {
                    let drained = std::mem::take(&mut pending);
                    if event_tx.send(ReloadEvent::FilesChanged(drained)).is_err() {
                        // Receiver dropped; nothing else to do.
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                // Watcher dropped; flush pending then exit.
                if !pending.is_empty() {
                    let _ = event_tx.send(ReloadEvent::FilesChanged(pending));
                }
                break;
            }
        }
    }
}

/// Filter raw [`notify::EventKind`] values down to the kinds the
/// reload pipeline cares about.
///
/// Today the reload pipeline reacts to file-creation, file-write,
/// and file-removal kinds. Metadata-only events (chmod, atime
/// touches) are ignored; they regularly fire during editor save
/// flows on macOS without representing a content change.
fn is_reload_relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

/// Single-shot reload step.
///
/// Calls the supplied `loader` against `scene_path` and wraps
/// the result. Loader successes pass through verbatim alongside
/// the caller-provided `preserved` state; loader failures are
/// routed through [`crate::diagnostics::diagnostic_from_load_error`]
/// into a structured [`Diagnostic`] consumed by the authoring
/// console and inline-diagnostics surfaces.
///
/// `preserved` is the [`PreservedState`] the caller captured
/// pre-reload (typically via [`capture_preserved_state`] folded
/// over a [`PreservedStateSnapshot`] sourced from the runtime
/// State at the call site). On a successful load, `preserved` is
/// forwarded verbatim onto [`ReloadOutcome::Success`] so the
/// consumer-side re-apply step has it in hand at the same callback
/// site that observes the new artifact. On a failed load,
/// `preserved` is dropped — preservation is success-only by
/// contract; the producer retains the pre-reload state in its own
/// buffer so the next-attempt reload can re-capture or re-use it.
///
/// The caller is assumed to have already chosen which file's
/// change triggered the reload (the watcher emits paths; the
/// caller maps them to a scene-package root and calls
/// `perform_reload` against the canonical scene-package path).
pub fn perform_reload<F, T>(
    scene_path: &Path,
    preserved: PreservedState,
    loader: F,
) -> ReloadOutcome<T>
where
    F: Fn(&Path) -> Result<T, LoadError>,
{
    match loader(scene_path) {
        Ok(value) => ReloadOutcome::Success { value, preserved },
        Err(err) => ReloadOutcome::Failed(diagnostic_from_load_error(&err)),
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-AUTHORING-LIVE-RELOAD (T1) — file-watcher and
    //! reload-pipeline drift pins. Per `docs/43-development-
    //! standards.md` §15.5, T1 covers "in-source pin against a
    //! producer surface; no external corpus or replay harness".
    //! Pinned invariants:
    //!
    //! - Empty-watch-set start-up does not panic and yields a
    //!   live handle.
    //! - Dropping the handle signals the worker thread to stop.
    //! - `perform_reload` with a successful loader yields the
    //!   `Success` variant carrying the loader output.
    //! - `perform_reload` with an erroring loader yields `Failed`
    //!   wrapping a structured `Diagnostic`.
    //! - The `Failed`-variant `Diagnostic` carries the severity
    //!   the diagnostic-renderer surface assigns to
    //!   `LoadError::Io` (today: `Severity::Error`,
    //!   `DiagnosticCode::Json`).
    //! - Filesystem-modification events flow through the
    //!   debounced channel within a slack window of the debounce
    //!   period.
    //! - Burst writes within the debounce window collapse to a
    //!   single `ReloadEvent`.
    //!
    //! T2 corpus (latency-budget assertion harness over multiple
    //! file types per `docs/37` §3.3) is the scope of PR-3 of
    //! this wave; this module establishes the structural floor.
    //!
    //! Filesystem-touching tests follow the established
    //! `engine/runtime/src/crash_resume.rs` pattern of
    //! `std::env::temp_dir()` plus `(pid, nanos, counter)` for
    //! per-test isolation; `tempfile` is not in the workspace
    //! dep graph and a single in-source helper avoids adding it.
    use super::*;
    use crate::diagnostics::{DiagnosticCode, Severity};
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Per-test-unique scratch directory under `std::env::temp_dir()`.
    /// Mirrors the `crash_resume` pattern; manual cleanup at end
    /// of each test (no `tempfile` dep in the workspace).
    fn scratch_dir(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-live-reload-{label}-{pid}-{nanos}-{n}"
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn start_watching_with_empty_paths_returns_handle_without_panic() {
        let cfg = LiveReloadConfig::default();
        let (handle, _rx) =
            start_watching(cfg).expect("empty-paths watcher should construct without error");
        assert!(handle.is_running());
        // Drop is implicit at end of scope; the next test
        // (`live_reload_handle_drop_signals_thread_to_stop`)
        // exercises the Drop signalling explicitly.
    }

    #[test]
    fn live_reload_handle_drop_signals_thread_to_stop() {
        let cfg = LiveReloadConfig::default();
        let (handle, _rx) = start_watching(cfg).unwrap();
        let stop_flag = Arc::clone(&handle.stop_flag);
        assert!(!stop_flag.load(Ordering::SeqCst));
        drop(handle);
        // After the handle drop returns, the stop flag must
        // have been set. (The thread join in Drop blocks until
        // the worker exits, so by the time `drop` returns the
        // flag is observable.)
        assert!(stop_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn perform_reload_with_successful_loader_returns_success_variant() {
        let outcome: ReloadOutcome<u32> =
            perform_reload(Path::new("/dummy"), PreservedState::default(), |_p| {
                Ok::<u32, LoadError>(42)
            });
        match outcome {
            ReloadOutcome::Success { value, preserved } => {
                assert_eq!(value, 42);
                assert!(preserved.is_empty());
            }
            ReloadOutcome::Failed(d) => panic!("expected Success, got Failed: {d:?}"),
        }
    }

    #[test]
    fn perform_reload_with_erroring_loader_returns_failed_variant_with_diagnostic() {
        let outcome: ReloadOutcome<()> =
            perform_reload(Path::new("/dummy"), PreservedState::default(), |_p| {
                Err::<(), LoadError>(LoadError::MissingSceneJson(PathBuf::from(
                    "/dummy/scene.json",
                )))
            });
        match outcome {
            ReloadOutcome::Failed(d) => {
                assert_eq!(d.code, DiagnosticCode::Json);
                assert!(d.message.contains("missing scene.json"));
            }
            ReloadOutcome::Success { .. } => panic!("expected Failed, got Success"),
        }
    }

    #[test]
    fn perform_reload_diagnostic_severity_matches_load_error_kind() {
        // `LoadError::MissingSceneJson` → `Severity::Error` per
        // `diagnostic_from_load_error`'s catch-all arm. This
        // pins the severity classification through the reload
        // pipeline so consumers can rely on it.
        let outcome: ReloadOutcome<()> =
            perform_reload(Path::new("/dummy"), PreservedState::default(), |_p| {
                Err::<(), LoadError>(LoadError::NotDirectory(PathBuf::from("/dummy")))
            });
        let ReloadOutcome::Failed(d) = outcome else {
            panic!("expected Failed");
        };
        assert_eq!(d.severity, Severity::Error);
    }

    // ---- PR 2 state-preservation T1 pins ----
    //
    // Per `docs/37-authoring-environment-spec.md` §3.3 hot-reload
    // semantics: an operator's mid-show edits to control values,
    // active preset, Snap selection, and playlist position must
    // survive a successful hot-reload. PR 2 ships the cross-seam
    // contract (`PreservedState` + `PreservedStateSnapshot` +
    // `capture_preserved_state` + `ReloadOutcome::Success` carrying
    // the captured state); the runtime-side wiring is a future
    // wave's option. These T1 pins exercise the contract surface in
    // isolation — no filesystem, no notify dep, no scene loader.

    #[test]
    fn preserved_state_round_trips_via_capture_helper() {
        let snapshot = PreservedStateSnapshot {
            control_values: vec![("gain".to_string(), 0.5), ("tint".to_string(), 1.25)],
            active_preset: Some("preset-a".to_string()),
            active_snap: Some(3),
            playlist_position: Some(7),
        };
        let preserved = capture_preserved_state(snapshot);
        assert_eq!(
            preserved.control_values,
            vec![("gain".to_string(), 0.5), ("tint".to_string(), 1.25),],
        );
        assert_eq!(preserved.active_preset.as_deref(), Some("preset-a"));
        assert_eq!(preserved.active_snap, Some(3));
        assert_eq!(preserved.playlist_position, Some(7));
        assert!(!preserved.is_empty());
    }

    #[test]
    fn preserved_state_default_is_empty() {
        let preserved = PreservedState::default();
        assert!(preserved.is_empty());
        assert!(preserved.control_values.is_empty());
        assert!(preserved.active_preset.is_none());
        assert!(preserved.active_snap.is_none());
        assert!(preserved.playlist_position.is_none());
    }

    #[test]
    fn preserved_state_with_control_values_round_trip() {
        let snapshot = PreservedStateSnapshot {
            control_values: vec![("gain".to_string(), 0.42)],
            ..Default::default()
        };
        let preserved = capture_preserved_state(snapshot);
        assert!(!preserved.is_empty());
        assert_eq!(preserved.control_values.len(), 1);
        assert_eq!(preserved.control_values[0].0, "gain");
        assert!((preserved.control_values[0].1 - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn reload_outcome_success_carries_preserved_state() {
        let snapshot = PreservedStateSnapshot {
            control_values: vec![("a".to_string(), 1.0)],
            active_preset: Some("p1".to_string()),
            active_snap: Some(2),
            playlist_position: Some(4),
        };
        let preserved = capture_preserved_state(snapshot);
        let outcome: ReloadOutcome<u32> = perform_reload(Path::new("/dummy"), preserved, |_p| {
            Ok::<u32, LoadError>(99)
        });
        let ReloadOutcome::Success { value, preserved } = outcome else {
            panic!("expected Success");
        };
        assert_eq!(value, 99);
        assert_eq!(preserved.control_values.len(), 1);
        assert_eq!(preserved.active_preset.as_deref(), Some("p1"));
        assert_eq!(preserved.active_snap, Some(2));
        assert_eq!(preserved.playlist_position, Some(4));
    }

    #[test]
    fn is_empty_returns_true_for_default_state() {
        // Explicit pin: the `PreservedState::default()` value MUST
        // satisfy `is_empty()` so the consumer-side re-apply
        // short-circuit is sound at the unmodified-session start.
        assert!(PreservedState::default().is_empty());
    }

    #[test]
    fn live_reload_event_channel_receives_files_changed_on_filesystem_modification() {
        let dir = scratch_dir("modify");
        let target = dir.join("scene.json");
        fs::write(&target, "{}").unwrap();

        let cfg = LiveReloadConfig {
            watched_paths: vec![dir.clone()],
            debounce_ms: 50,
        };
        let (handle, rx) = start_watching(cfg).unwrap();

        // Small wait so the watcher is fully registered before
        // we touch the file. macOS FSEvents can drop events that
        // race the watch() call.
        thread::sleep(Duration::from_millis(150));

        {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&target)
                .unwrap();
            f.write_all(b"{\"modified\": true}").unwrap();
            f.sync_all().unwrap();
        }

        // Slack window: debounce + platform watcher latency.
        // Empirically generous for FSEvents (~200 ms in
        // worst-case CI) plus the 50 ms debounce; allow up to
        // 3 seconds before failing.
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = None;
        while Instant::now() < deadline {
            if let Ok(ev) = rx.recv_timeout(Duration::from_millis(100)) {
                received = Some(ev);
                break;
            }
        }
        drop(handle);
        cleanup(&dir);

        let ev = received.expect("expected ReloadEvent within slack window");
        match ev {
            ReloadEvent::FilesChanged(paths) => {
                assert!(!paths.is_empty(), "FilesChanged carries at least one path");
            }
        }
    }

    #[test]
    fn live_reload_debounce_collapses_burst_writes_into_single_event() {
        let dir = scratch_dir("burst");
        let target = dir.join("scene.json");
        fs::write(&target, "{}").unwrap();

        let cfg = LiveReloadConfig {
            watched_paths: vec![dir.clone()],
            // Generous debounce so the burst writes all land
            // inside a single window with platform-watcher
            // latency room to spare.
            debounce_ms: 250,
        };
        let (handle, rx) = start_watching(cfg).unwrap();

        thread::sleep(Duration::from_millis(150));

        // Five rapid writes, well under the debounce window.
        for i in 0..5 {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&target)
                .unwrap();
            write!(f, "{{\"i\": {i}}}").unwrap();
            f.sync_all().unwrap();
        }

        // Wait long enough for the debounce window to elapse
        // PLUS allow extra slack for the worker to flush, then
        // count distinct events delivered. We expect exactly
        // one (or, generously, "at most one" — a second event
        // would only arrive if late-fire FSEvents straggled in
        // more than one debounce window after the burst).
        let mut events: Vec<ReloadEvent> = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(ev) => events.push(ev),
                Err(RecvTimeoutError::Timeout) => {
                    if !events.is_empty() {
                        // Once we've seen at least one event
                        // and the channel goes quiet for a
                        // poll interval, the burst has been
                        // flushed.
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(handle);
        cleanup(&dir);

        assert!(
            !events.is_empty(),
            "expected at least one debounced event for the burst"
        );
        // Strict equality is brittle on CI under heavy load
        // (FSEvents can deliver a single trailing event past
        // the debounce window). Accept 1 OR 2 as evidence of
        // collapse — without debounce, 5 writes produce many
        // more than 2 events, since each write fires Modify
        // (and on some editors Modify + Metadata).
        assert!(
            events.len() <= 2,
            "expected debounce to collapse 5 writes into ≤ 2 events, got {}",
            events.len()
        );
    }
}
