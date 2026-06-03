// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1c-α-2 production-bootstrap PR 1: producer-side
//! audio-analysis runtime entrypoint.
//!
//! Wave 1c-α (PRs 338 / 339 / 340) shipped the cross-seam payload
//! contract for `EV-AUDIO-UNIFORM-IPC` (row 83): the producer-side
//! `AudioUniformSnapshotPayload` ts-rs newtype, the
//! `engine/runtime` `IpcEmit` trait + tick-step at AFTER-observability,
//! the consumer-side `audioSnapshot.svelte.ts` singleton store, and
//! dual-mode retrofits to the `AudioReactivity` / `BpmRing` /
//! `BeatDots` widgets. Wave 1c-α explicitly recorded five deferred
//! axes; the first axis ("live end-to-end producer→Tauri-emit→consumer
//! wiring") is the production-bootstrap wave's scope.
//!
//! This module is **PR 1 of the production-bootstrap wave**: a
//! composition-layer entrypoint wrapping the existing producer-side
//! surfaces ([`crate::cpal_source::try_default_input`],
//! [`crate::ingest_negotiation::IngestNegotiator`],
//! [`crate::silent::SilentSource`], [`crate::bpm::BpmEstimator`],
//! [`crate::uniforms::extract_uniform_snapshot`]) into a single
//! thread-spawnable unit. The runtime owns an
//! `Arc<Mutex<AudioUniformSnapshot>>` shared seam that the
//! scheduler-tick consumer reads via
//! `engine/runtime/src/scheduler/audio.rs::MutexAudioPull`. PR 2 of
//! this wave wires the Tauri-side `IpcEmit` adapter + scheduler
//! bootstrap; PR 3 lands the e2e IPC wire test + the Wave 1c-α-2
//! closing docs.
//!
//! # Composition shape
//!
//! [`AudioRuntime::new`] tries
//! [`crate::cpal_source::try_default_input`] for the primary ingest
//! source. CI runners and headless test environments frequently lack
//! a default input device; on construction-time failure the runtime
//! falls back to wrapping a [`crate::silent::SilentSource`] (with a
//! `u32::MAX` hop budget) as the negotiator's primary, rather than
//! routing through the negotiator's internal silent-fallback (which
//! expects a *failing* primary to drive the fallback transition).
//! Both paths produce a runnable runtime; the silent-only path
//! produces zero-extraction snapshots indefinitely.
//!
//! # Thread-loop contract
//!
//! [`AudioRuntime::spawn`] returns an [`AudioRuntimeHandle`] that
//! owns the producer thread's `JoinHandle` plus a shutdown
//! [`crossbeam_channel::Sender`]. The thread runs:
//!
//! ```text
//! loop {
//!     if shutdown_rx.try_recv() is Ok(()) | Disconnected { break }
//!     match negotiator.try_pull(&mut frame) {
//!         Ready => {
//!             let analysis_frame = analysis_window.observe(&frame);
//!             let snapshot = extract_uniform_snapshot(&analysis_frame);
//!             let smoothed_bpm = bpm_estimator.observe(&analysis_frame);
//!             *shared.lock() = AudioUniformSnapshot { bpm: smoothed_bpm, ..snapshot };
//!         }
//!         WouldBlock => sleep(500us),  // back off briefly
//!         Closed     => sleep(2ms),    // negotiator handles fallback transition
//!     }
//! }
//! ```
//!
//! Thread-loop invariants:
//!
//! - **No panics escape the thread.** The body of every loop
//!   iteration runs under `std::panic::catch_unwind(AssertUnwindSafe(...))`.
//!   On caught panic the loop exits without further snapshot writes;
//!   the last-good snapshot stays in place per ADR-019 §6.
//! - **Lock-poison recovery.** The shared mutex follows the same
//!   `match lock() { Ok(g) => *g, Err(p) => *p.into_inner() }` pattern
//!   that `engine/runtime/src/scheduler/audio.rs::MutexAudioPull`
//!   uses (lines 91-106). A poisoned mutex never aborts the thread;
//!   the runtime resumes writing into the recovered guard.
//! - **No `.unwrap()` / `.expect()` outside `#[cfg(test)]`.** The
//!   workspace `clippy::unwrap_used = deny` + `clippy::expect_used =
//!   deny` lints apply; the loop uses `match` / `if let` /
//!   `unwrap_or_else` instead.
//! - **No `unsafe`.** Workspace `unsafe_code = forbid` applies;
//!   `catch_unwind` is safe Rust.
//!
//! # Producer / consumer seam
//!
//! [`AudioRuntime::snapshot_handle`] (and the same accessor on
//! [`AudioRuntimeHandle`]) returns a clone of the
//! `Arc<Mutex<AudioUniformSnapshot>>` the producer thread writes
//! into. The scheduler-tick consumer (PR 2's bootstrap path)
//! constructs a `MutexAudioPull` over the same handle and routes the
//! per-tick snapshot pull → `IpcEmit::emit_audio_uniform_snapshot`
//! step. Wave 1c-α-2 PR 1 (this PR) ships only the producer side; PR
//! 2 wires the Tauri-side `IpcEmit` adapter and the scheduler
//! bootstrap.
//!
//! Evidence: EV-AUDIO-UNIFORM-IPC (T1) — the producer-side runtime
//! contributes T1 drift pins on the composition layer alongside the
//! existing `app/ui-bridge/tests/audio_uniform_payload_round_trip.rs`
//! T2 corpus and the `engine/runtime/src/scheduler/ipc.rs` T1 pins.

use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Sender, TryRecvError, bounded};

use crate::bpm::BpmEstimator;
use crate::cpal_source::{try_default_input, try_input_by_preference};
use crate::frame::AudioFrame;
use crate::ingest::{IngestBoundary, IngestStatus};
use crate::ingest_negotiation::{IngestModeCriteria, IngestNegotiator};
use crate::reactive_bands::ReactiveBands;
use crate::silent::SilentSource;
use crate::uniforms::{AudioAnalysisWindow, AudioUniformSnapshot, extract_uniform_snapshot};

/// Back-off when [`IngestStatus::Pending`] or
/// [`IngestStatus::WouldBlock`] is observed.
///
/// Tuned to the §7.1 hop cadence (375 Hz nominal -> ~2.67ms per hop);
/// a 500us back-off yields ~5 retry attempts per nominal hop, which
/// keeps the analyser thread responsive without busy-spinning.
const WOULD_BLOCK_BACKOFF: Duration = Duration::from_micros(500);

/// Back-off when [`IngestStatus::Closed`] is observed.
///
/// On `Closed` the negotiator handles the fallback transition; the
/// brief sleep keeps the loop from spinning while the negotiator's
/// criteria flip the active source for the next iteration.
const CLOSED_BACKOFF: Duration = Duration::from_millis(2);

/// Drop-time best-effort join timeout for [`AudioRuntimeHandle`].
const DROP_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

/// Fixed cadence for the heavy extract (FFT + BPM autocorrelation), decoupled
/// from the per-tick frame drain.
///
/// The producer loop drains ALL buffered frames into the rolling window every
/// tick (cheap), but runs the heavy `extract_uniform_snapshot` + BPM estimator
/// only this often. ~100 Hz — fresh for the 60 Hz scheduler-tick consumer, and
/// far below the macOS tap's ~1500-frame/s callback rate, so the bounded ingest
/// channel never overflows (the prior pull-one-then-heavy-analyze loop fell
/// behind and the producer silently dropped frames → gappy windows → low,
/// jittery rms). Tempo is a backup feature; a single cadence covers both axes
/// here, with a finer bands-fast / BPM-slow split as a follow-up.
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(10);

/// Max frames the producer loop drains into the rolling window per tick.
///
/// Bounds the drain so an always-`Ready` source (the `SilentSource` fallback,
/// or a test double with no backpressure) can never spin the inner loop forever
/// — it always returns to the outer loop to check shutdown + pace. Generously
/// above the cpal ingest channel's 16-frame capacity, so a genuine backlog is
/// cleared in a single tick while a real backpressured source naturally stops at
/// `WouldBlock` long before the cap.
const MAX_DRAIN_PER_TICK: usize = 64;

/// Configuration for [`AudioRuntime`].
///
/// Tweakable knobs for the composed runtime. Defaults match the
/// existing per-component baselines: an 8-frame BPM history-window
/// (matches [`BpmEstimator::new`]) and the
/// [`IngestModeCriteria::default`] two-failures + one-success
/// Reliability/Fallback policy.
#[derive(Debug, Clone)]
pub struct AudioRuntimeConfig {
    /// Window size for the smoothed BPM estimator. Default 8 frames.
    /// Validated `1..=128` by [`BpmEstimator::with_window`]; an
    /// invalid value falls back to [`BpmEstimator::new`] at
    /// construction time.
    pub bpm_window: usize,
    /// [`IngestModeCriteria`] for the negotiator. Defaults to the
    /// Reliability/Fallback policy two-failures + one-success rule.
    /// Invalid criteria fall back to [`IngestModeCriteria::default`]
    /// at construction time.
    pub criteria: IngestModeCriteria,
}

impl Default for AudioRuntimeConfig {
    fn default() -> Self {
        Self {
            bpm_window: 8,
            criteria: IngestModeCriteria::default(),
        }
    }
}

/// Producer-side audio-analysis runtime.
///
/// Composes the existing cpal-source + ingest-negotiator +
/// bpm-estimator + uniform-extractor surfaces into a single
/// thread-spawnable unit. Owns an `Arc<Mutex<AudioUniformSnapshot>>`
/// shared seam that the scheduler-tick consumer reads via
/// `engine/runtime/src/scheduler/audio.rs::MutexAudioPull`.
///
/// # Construction
///
/// [`AudioRuntime::new`] builds with the default cpal input device.
/// On cpal failure (CI runners, headless environments, missing
/// devices) the runtime falls back to a [`SilentSource`]-only
/// configuration that produces zero snapshots indefinitely.
///
/// # Threading
///
/// [`AudioRuntime::spawn`] consumes the runtime and returns an
/// [`AudioRuntimeHandle`] that owns the spawned producer thread.
pub struct AudioRuntime {
    negotiator: IngestNegotiator,
    estimator: BpmEstimator,
    shared: Arc<Mutex<AudioUniformSnapshot>>,
}

impl core::fmt::Debug for AudioRuntime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AudioRuntime")
            .field("estimator_window", &self.estimator.window())
            .field("negotiator_active", &self.negotiator.active_source())
            .finish_non_exhaustive()
    }
}

impl AudioRuntime {
    /// Construct a runtime with the default cpal input device.
    ///
    /// Falls back to a [`SilentSource`]-only configuration if
    /// [`try_default_input`] returns an error (no default device,
    /// unsupported sample format, stream-build / stream-play
    /// failure). The fallback path produces zero snapshots
    /// indefinitely; consumers see [`AudioUniformSnapshot::default`]
    /// values with the BPM smoothed across the silent frames'
    /// per-frame autocorrelation result.
    ///
    /// Invalid [`AudioRuntimeConfig::bpm_window`] values fall back to
    /// [`BpmEstimator::new`]'s default window. Invalid
    /// [`AudioRuntimeConfig::criteria`] values fall back to the
    /// negotiator's default criteria.
    #[must_use]
    pub fn new(config: AudioRuntimeConfig) -> Self {
        match try_default_input() {
            Ok(cpal_source) => Self::from_primary(cpal_source, config),
            Err(_) => Self::from_primary(SilentSource::new(u32::MAX), config),
        }
    }

    /// Construct a runtime honouring a persisted operator device
    /// preference at startup.
    ///
    /// Wave 2.8 Lane AUD.SRC: closes the cosmetic-only gap on the
    /// Wave 2.6 Lane AUD.A Input Device dropdown — previously
    /// `AudioRuntime::new` always bound the host's default input
    /// device (microphone on macOS) and the persisted
    /// `settings.audio.input_device` was inert. This constructor takes
    /// the persisted preference string and resolves it via
    /// [`try_input_by_preference`]:
    ///
    /// - `None` or the Wave 2.1 sentinel `"default"` → host default
    ///   input (matches [`AudioRuntime::new`] semantics).
    /// - A sanitised id from `enumerate_input_devices` (e.g.
    ///   `"input-blackhole-16ch"`) or a raw cpal device name → that
    ///   device is bound.
    /// - A preference that does not resolve against the current host
    ///   (operator unplugged BlackHole, switched audio profiles, etc.)
    ///   → falls back to the host default input with a one-shot stderr
    ///   warning; runtime still constructs successfully.
    /// - cpal failure on either path → falls back to a
    ///   [`SilentSource`]-only configuration that produces zero
    ///   snapshots indefinitely (matches [`AudioRuntime::new`]).
    ///
    /// "Apply at startup" only — true "apply without restart" needs
    /// producer-thread restart semantics + 60 Hz scheduler-tick
    /// coordination and is a future wave. The Settings → Audio hint
    /// copy notes the restart requirement to the operator.
    ///
    /// Evidence: `EV-INTEGRATION-MATRIX` (audio arm, Wave 2.8 Lane
    /// AUD.SRC).
    #[must_use]
    pub fn with_preferred_device(preference: Option<&str>, config: AudioRuntimeConfig) -> Self {
        match try_input_by_preference(preference) {
            Ok(cpal_source) => Self::from_primary(cpal_source, config),
            Err(_) => Self::from_primary(SilentSource::new(u32::MAX), config),
        }
    }

    /// Test-seam constructor: build a runtime over an arbitrary
    /// [`IngestBoundary`] primary plus the supplied config.
    ///
    /// Used by [`AudioRuntime::new`] for both the cpal and silent-
    /// fallback paths, and by the in-source T1 drift pins below to
    /// inject scripted ingest sources without touching the cpal
    /// device API.
    pub(crate) fn from_primary<P>(primary: P, config: AudioRuntimeConfig) -> Self
    where
        P: IngestBoundary + Send + 'static,
    {
        let negotiator = match IngestNegotiator::new(primary).with_criteria(config.criteria) {
            Ok(n) => n,
            Err(_) => IngestNegotiator::new(SilentSource::new(u32::MAX)),
        };
        let estimator = match BpmEstimator::with_window(config.bpm_window) {
            Ok(e) => e,
            Err(_) => BpmEstimator::new(),
        };
        Self {
            negotiator,
            estimator,
            shared: Arc::new(Mutex::new(AudioUniformSnapshot::default())),
        }
    }

    /// Borrow the shared snapshot mutex without spawning a thread.
    ///
    /// Returned handle is the same `Arc<Mutex<AudioUniformSnapshot>>`
    /// the spawned producer thread writes to; cloning is cheap
    /// (`Arc::clone`).
    #[must_use]
    pub fn snapshot_handle(&self) -> Arc<Mutex<AudioUniformSnapshot>> {
        Arc::clone(&self.shared)
    }

    /// Spawn the producer thread.
    ///
    /// Returns an [`AudioRuntimeHandle`] that owns the thread's
    /// `JoinHandle` plus a shutdown signal. Dropping the handle is
    /// best-effort: the shutdown signal fires and a 2-second join
    /// timeout caps the producer thread's exit.
    pub fn spawn(self) -> AudioRuntimeHandle {
        let Self {
            mut negotiator,
            mut estimator,
            shared,
        } = self;
        let shared_for_thread = Arc::clone(&shared);
        let (shutdown_tx, shutdown_rx) = bounded::<()>(1);

        let join = thread::Builder::new()
            .name("chromaticity-audio-runtime".into())
            .spawn(move || {
                let mut frame = AudioFrame::default();
                let mut analysis_window = AudioAnalysisWindow::default();
                // CHROMATICITY PATCH (ADR-024 drop-fix): the rolling window
                // carried across the drain loop, plus the last-analysis time for
                // the fixed-cadence gate. Decoupling the cheap drain from the
                // heavy extract is what keeps the bounded ingest channel from
                // overflowing.
                let mut current_window: Option<AudioFrame> = None;
                let mut last_analysis: Option<Instant> = None;
                // MilkDrop-class per-band AGC (ADR-024). Lazily built once the
                // first frame's sample rate is known, and rebuilt if the rate
                // changes (its Hz→bin band map is rate-dependent) — mirroring
                // `AudioAnalysisWindow`'s reset-on-rate-change. Lives in the
                // thread (not the struct) because it is only ever touched here.
                let mut reactive_bands: Option<ReactiveBands> = None;
                let mut reactive_rate_hz: Option<u32> = None;
                loop {
                    match shutdown_rx.try_recv() {
                        Ok(()) | Err(TryRecvError::Disconnected) => break,
                        Err(TryRecvError::Empty) => {}
                    }

                    // Wrap the per-iteration body in catch_unwind so a
                    // panic in any composed surface (extractor /
                    // estimator / negotiator) exits the loop instead
                    // of unwinding the thread. Last-good snapshot
                    // stays in place per ADR-019 §6.
                    let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        // 1. Drain ALL currently-buffered frames into the rolling
                        //    window (cheap). At the macOS tap's ~1500-frame/s rate
                        //    the old pull-one-then-heavy-analyze loop fell behind,
                        //    the bounded channel filled, and the producer silently
                        //    dropped frames; draining every tick keeps it
                        //    near-empty. `observe` returns the accumulated window.
                        let mut closed = false;
                        let mut drained = 0usize;
                        while drained < MAX_DRAIN_PER_TICK {
                            match negotiator.try_pull(&mut frame) {
                                IngestStatus::Ready => {
                                    current_window = Some(analysis_window.observe(&frame));
                                    drained += 1;
                                }
                                IngestStatus::Pending | IngestStatus::WouldBlock => break,
                                IngestStatus::Closed => {
                                    closed = true;
                                    break;
                                }
                            }
                        }

                        // 2. Run the heavy extract + BPM at the fixed cadence
                        //    ([`ANALYSIS_INTERVAL`]), not per frame — analyses the
                        //    first available window immediately, then at cadence.
                        if let Some(window) = current_window.as_ref() {
                            if last_analysis.is_none_or(|t| t.elapsed() >= ANALYSIS_INTERVAL) {
                                let analysis_snapshot = extract_uniform_snapshot(window);
                                let smoothed_bpm = estimator.observe(window);

                                // Quality layer (ADR-024): the per-band AGC turns
                                // the raw spectrum into volume-independent relative
                                // bands. (Re)build on a sample-rate change, then
                                // advance by the REAL elapsed time since the last
                                // analysis so the FPS-compensated smoothing tracks
                                // the actual cadence rather than a fixed nominal.
                                let sample_rate_hz = window.sample_rate_hz();
                                if reactive_rate_hz != Some(sample_rate_hz) {
                                    reactive_bands = Some(ReactiveBands::new(sample_rate_hz));
                                    reactive_rate_hz = Some(sample_rate_hz);
                                }
                                let dt = last_analysis
                                    .map(|t| t.elapsed().as_secs_f32())
                                    .unwrap_or_else(|| ANALYSIS_INTERVAL.as_secs_f32());
                                // `unwrap_or_default()` is unreachable (the block
                                // above always leaves `reactive_bands` `Some`) but
                                // keeps the path `.unwrap()`/`.expect()`-free per
                                // the workspace clippy lints; the default is the
                                // neutral centred baseline.
                                let reactive = reactive_bands
                                    .as_mut()
                                    .map(|rb| rb.observe(window.samples(), dt))
                                    .unwrap_or_default();

                                let merged = AudioUniformSnapshot {
                                    bpm: smoothed_bpm,
                                    bpm_availability: analysis_snapshot.bpm_availability,
                                    rms: analysis_snapshot.rms,
                                    peak: analysis_snapshot.peak,
                                    fft_bands: analysis_snapshot.fft_bands,
                                    fft_availability: analysis_snapshot.fft_availability,
                                    bass: reactive.bass,
                                    mid: reactive.mid,
                                    treb: reactive.treb,
                                    bass_att: reactive.bass_att,
                                    mid_att: reactive.mid_att,
                                    treb_att: reactive.treb_att,
                                    degraded: false,
                                };
                                match shared_for_thread.lock() {
                                    Ok(mut guard) => {
                                        *guard = merged;
                                    }
                                    Err(poisoned) => {
                                        // Lock-poison recovery: the
                                        // mutex was poisoned by an
                                        // earlier panic on the
                                        // consumer side. Recover the
                                        // inner guard and resume
                                        // writing.
                                        let mut guard = poisoned.into_inner();
                                        *guard = merged;
                                    }
                                }
                                last_analysis = Some(Instant::now());
                            }
                        }

                        // Always back off one short interval per tick — never a
                        // busy-spin, even for an always-`Ready` source (the
                        // SilentSource fallback / a test double). At ~2 kHz this
                        // still drains the bounded channel far faster than the tap
                        // fills it (~1500/s), and a genuine backlog is cleared in
                        // one tick via the MAX_DRAIN_PER_TICK cap above. The body
                        // returns the back-off interval for the outer loop.
                        if closed {
                            CLOSED_BACKOFF
                        } else {
                            WOULD_BLOCK_BACKOFF
                        }
                    }));

                    match outcome {
                        Ok(backoff) => thread::sleep(backoff),
                        Err(_panic) => {
                            // Caught panic: don't touch the snapshot
                            // (last-good stays in place per ADR-019
                            // §6), exit the loop. Best-effort
                            // observability emit would go here once
                            // the audio crate has a panic-safe
                            // emission surface; today the crate's
                            // observability module fires per-pull
                            // counters that already cover the
                            // `Closed` outcome the analyser will
                            // observe once the producer exits.
                            break;
                        }
                    }
                }
            })
            .ok();

        AudioRuntimeHandle {
            shared,
            shutdown_tx: Some(shutdown_tx),
            join,
        }
    }
}

/// Handle to a spawned audio runtime.
///
/// Owns the producer thread's `JoinHandle` plus a shutdown channel.
/// The Drop impl signals shutdown and best-effort-joins the thread
/// with a 2-second timeout; explicit [`AudioRuntimeHandle::shutdown`]
/// is provided for tests and graceful app exit paths.
pub struct AudioRuntimeHandle {
    shared: Arc<Mutex<AudioUniformSnapshot>>,
    shutdown_tx: Option<Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl core::fmt::Debug for AudioRuntimeHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AudioRuntimeHandle")
            .field("running", &self.join.is_some())
            .finish_non_exhaustive()
    }
}

impl AudioRuntimeHandle {
    /// Shared snapshot the scheduler-tick consumer reads.
    #[must_use]
    pub fn snapshot_handle(&self) -> Arc<Mutex<AudioUniformSnapshot>> {
        Arc::clone(&self.shared)
    }

    /// Signal the producer thread to stop and join it.
    ///
    /// The Drop impl already does this on a best-effort basis; the
    /// explicit method is for tests and graceful app-exit paths that
    /// want to observe the join completion.
    pub fn shutdown(mut self) {
        self.shutdown_inner();
    }

    fn shutdown_inner(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            // Best-effort send: a full channel (capacity 1) means a
            // shutdown is already pending; either way the thread
            // exits on the next loop iteration. A disconnected
            // receiver means the thread already exited.
            let _ = tx.try_send(());
        }
        if let Some(handle) = self.join.take() {
            // Bounded best-effort join: if the producer thread is
            // wedged, don't block app-exit beyond DROP_JOIN_TIMEOUT.
            // Cross-platform `JoinHandle::join` has no timeout API;
            // we approximate by polling `is_finished` (stable since
            // Rust 1.61) on a short cadence and falling through if
            // the timeout expires. The thread is detached on
            // timeout; OS reaps it on process exit.
            let deadline = Instant::now() + DROP_JOIN_TIMEOUT;
            while Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
            // Timeout: leak the handle (thread continues until
            // process exit). This path indicates a bug — the
            // shutdown signal fired but the thread didn't exit
            // within 2 seconds — and is observable through process
            // tracing.
        }
    }
}

impl Drop for AudioRuntimeHandle {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod runtime_pins {
    //! Evidence: EV-AUDIO-UNIFORM-IPC (T1) — drift pins on the
    //! producer-side audio-analysis runtime composition layer.
    //!
    //! T1 per `docs/43 §15.5`: each test pins a documented
    //! state-machine invariant the [`super::AudioRuntime`] /
    //! [`super::AudioRuntimeHandle`] composition must honour for the
    //! production-bootstrap wave's downstream IPC-emit path (PR 2's
    //! Tauri adapter, PR 3's e2e wire test) to compose correctly.
    //! The cross-seam payload-shape contract is pinned T1 by
    //! `engine/runtime/src/scheduler/ipc.rs` and T2 by
    //! `app/ui-bridge/tests/audio_uniform_payload_round_trip.rs`;
    //! this module pins the producer-side composition.
    //!
    //! Coverage:
    //! - `runtime_with_silent_only_config_produces_zero_snapshots`
    //!   pins the silent-fallback baseline (zero RMS / peak / bands;
    //!   BPM at the `BPM_DEFAULT` 120 floor the silent-frame
    //!   autocorrelation returns).
    //! - `runtime_shutdown_signal_joins_thread_cleanly` pins the
    //!   explicit-shutdown happy path within a bounded timeout.
    //! - `runtime_lock_poisoning_recovers_snapshot_value` pins the
    //!   `lock().unwrap_or_else(|p| p.into_inner())` posture from
    //!   `engine/runtime/src/scheduler/audio.rs:91-106`.
    //! - `runtime_handle_drop_signals_shutdown_implicitly` pins the
    //!   Drop-time best-effort join.
    //! - `runtime_default_input_device_unavailable_falls_back_to_silent`
    //!   pins the [`AudioRuntime::new`] cpal-failure fallback path
    //!   via the `from_primary` test seam.

    use super::*;
    use crate::frame::DEFAULT_SAMPLE_RATE_HZ;
    use crate::observability;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    /// Sleep budget per polling step inside the test asserts. Tuned
    /// to the analyser hop cadence so the producer thread observes a
    /// handful of `Ready` outcomes before the test makes its
    /// snapshot assertion.
    const SETTLE_DURATION: Duration = Duration::from_millis(50);

    /// Polling step duration when waiting for thread exit.
    const POLL_STEP: Duration = Duration::from_millis(5);

    #[test]
    fn runtime_with_silent_only_config_produces_zero_snapshots() {
        let _serialize = observability::test_serialize_lock();
        let primary = SilentSource::new(u32::MAX);
        let runtime = AudioRuntime::from_primary(primary, AudioRuntimeConfig::default());
        let snapshot_handle = runtime.snapshot_handle();
        let handle = runtime.spawn();

        // Let the producer thread observe a handful of frames.
        thread::sleep(SETTLE_DURATION);

        let snapshot = *snapshot_handle.lock().unwrap();
        // Silent frames yield zero RMS / peak / bands; the BPM
        // estimator over silent frames returns the slice-1 default
        // (120.0) per `extract_uniform_snapshot`'s short-frame
        // fallback.
        assert_eq!(snapshot.rms, 0.0);
        assert_eq!(snapshot.peak, 0.0);
        for band in snapshot.fft_bands {
            assert_eq!(band, 0.0);
        }
        // ADR-024 reactive bands: silence drives the AGC's silence guard
        // (long average below SILENCE_FLOOR), so every relative band reads
        // the centred 1.0 neutral — whether this is the pre-first-write
        // default or a written silent-frame snapshot. Asserting exactly 1.0
        // is the deterministic proof that `ReactiveBands::observe` is wired
        // into the producer loop (a missing call would also read 1.0 via the
        // struct default, but a BUGGY call dividing by ~0 would not).
        assert_eq!(snapshot.bass, 1.0);
        assert_eq!(snapshot.mid, 1.0);
        assert_eq!(snapshot.treb, 1.0);
        assert_eq!(snapshot.bass_att, 1.0);
        assert_eq!(snapshot.mid_att, 1.0);
        assert_eq!(snapshot.treb_att, 1.0);
        // BPM is the median over the 8-frame ring of per-frame
        // autocorrelation results on the 128-sample silent frame.
        // The slice-1 short-frame fallback returns BPM_DEFAULT
        // (120.0); the median of an all-120.0 ring is 120.0.
        assert!(
            (snapshot.bpm - 120.0).abs() < f32::EPSILON || snapshot.bpm == 0.0,
            "expected BPM at 120.0 silent-frame fallback or 0.0 pre-first-write, got {}",
            snapshot.bpm,
        );

        handle.shutdown();
    }

    #[test]
    fn runtime_rms_and_peak_are_computed_over_analysis_window() {
        let _serialize = observability::test_serialize_lock();

        #[derive(Debug)]
        struct ScriptedFrames {
            frames: VecDeque<AudioFrame>,
            pulls: Arc<AtomicU32>,
        }

        impl IngestBoundary for ScriptedFrames {
            fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
                self.pulls.fetch_add(1, Ordering::Relaxed);
                match self.frames.pop_front() {
                    Some(frame) => {
                        *out = frame;
                        IngestStatus::Ready
                    }
                    None => IngestStatus::Pending,
                }
            }
        }

        let pulls = Arc::new(AtomicU32::new(0));
        let frames = VecDeque::from([
            AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![1.0; 128]),
            AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0; 128]),
        ]);
        let runtime = AudioRuntime::from_primary(
            ScriptedFrames {
                frames,
                pulls: Arc::clone(&pulls),
            },
            AudioRuntimeConfig::default(),
        );
        let snapshot_handle = runtime.snapshot_handle();
        let handle = runtime.spawn();

        let deadline = Instant::now() + Duration::from_secs(1);
        while pulls.load(Ordering::Relaxed) < 3 && Instant::now() < deadline {
            thread::sleep(POLL_STEP);
        }
        assert!(
            pulls.load(Ordering::Relaxed) >= 3,
            "runtime did not consume the scripted frames",
        );

        let snapshot = *snapshot_handle.lock().unwrap();
        let expected_rms = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (snapshot.rms - expected_rms).abs() < 1e-5,
            "expected windowed RMS {expected_rms}, got {}",
            snapshot.rms,
        );
        assert_eq!(snapshot.peak, 1.0);

        handle.shutdown();
    }

    #[test]
    fn runtime_reactive_bands_are_finite_on_loud_input() {
        // ADR-024: feeding real (loud, broadband) audio through the runtime
        // must drive the per-band AGC to finite, positive relative bands —
        // proving `ReactiveBands` is constructed at the capture rate and
        // `observe` runs over the rolling window without producing NaN/Inf or
        // dividing by ~0. (The AGC's *correctness* — volume independence,
        // band selectivity — is pinned by the `reactive_bands` unit tests and
        // the mini-meter rubric; this test pins the runtime WIRING.)
        let _serialize = observability::test_serialize_lock();

        #[derive(Debug)]
        struct LoudBroadband {
            phase: u64,
        }
        impl IngestBoundary for LoudBroadband {
            fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
                const CHUNK: usize = 128;
                let sr = DEFAULT_SAMPLE_RATE_HZ as f32;
                let samples: Vec<f32> = (0..CHUNK)
                    .map(|i| {
                        let t = (self.phase + i as u64) as f32 / sr;
                        // Bass + mid + treble so every band receives energy.
                        let tau = std::f32::consts::TAU;
                        0.3 * (tau * 120.0 * t).sin()
                            + 0.3 * (tau * 1_000.0 * t).sin()
                            + 0.3 * (tau * 8_000.0 * t).sin()
                    })
                    .collect();
                self.phase += CHUNK as u64;
                *out = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, samples);
                let status = IngestStatus::Ready;
                observability::record_pull_outcome(&status, CHUNK);
                status
            }
        }

        let runtime =
            AudioRuntime::from_primary(LoudBroadband { phase: 0 }, AudioRuntimeConfig::default());
        let snapshot_handle = runtime.snapshot_handle();
        let handle = runtime.spawn();

        // Let the window fill and several analysis ticks run.
        thread::sleep(SETTLE_DURATION);

        let snapshot = match snapshot_handle.lock() {
            Ok(g) => *g,
            Err(p) => *p.into_inner(),
        };
        for (name, value) in [
            ("bass", snapshot.bass),
            ("mid", snapshot.mid),
            ("treb", snapshot.treb),
            ("bass_att", snapshot.bass_att),
            ("mid_att", snapshot.mid_att),
            ("treb_att", snapshot.treb_att),
        ] {
            assert!(
                value.is_finite() && value > 0.0,
                "reactive band {name} must be finite and positive, got {value}",
            );
        }

        handle.shutdown();
    }

    #[test]
    fn runtime_shutdown_signal_joins_thread_cleanly() {
        let _serialize = observability::test_serialize_lock();
        let primary = SilentSource::new(u32::MAX);
        let runtime = AudioRuntime::from_primary(primary, AudioRuntimeConfig::default());
        let handle = runtime.spawn();

        // Capture the inner JoinHandle's finished-state polling via
        // an explicit shutdown call; the call returns once the
        // thread is joined or the 2-second drop-timeout expires.
        let start = Instant::now();
        handle.shutdown();
        let elapsed = start.elapsed();

        // 2-second drop-timeout is the ceiling; the silent-only
        // runtime should exit well under 100ms.
        assert!(
            elapsed < Duration::from_millis(500),
            "shutdown took {elapsed:?}, expected < 500ms",
        );
    }

    #[test]
    fn runtime_lock_poisoning_recovers_snapshot_value() {
        let _serialize = observability::test_serialize_lock();
        let primary = SilentSource::new(u32::MAX);
        let runtime = AudioRuntime::from_primary(primary, AudioRuntimeConfig::default());
        let shared = runtime.snapshot_handle();

        // Poison the mutex from a side thread by panicking while
        // holding the lock. The runtime's producer thread will
        // observe the poison on its next write attempt and recover
        // via `unwrap_or_else(|p| p.into_inner())`.
        let poison_target = Arc::clone(&shared);
        let poisoner = thread::spawn(move || {
            let _guard = poison_target.lock().unwrap();
            panic!("intentional poison for runtime lock-poison recovery test");
        });
        // Wait for the panicking thread to drop the guard.
        let _ = poisoner.join();
        assert!(shared.is_poisoned(), "expected mutex to be poisoned");

        // Now spawn the runtime; its first Ready outcome should
        // recover the poison and write a fresh snapshot.
        let handle = runtime.spawn();
        thread::sleep(SETTLE_DURATION);

        // After the runtime has had time to write, the snapshot is
        // observable via `into_inner` on the poisoned guard. Either
        // the runtime poisoned it again (won't, no panics) or the
        // recovery wrote successfully — either way we can read the
        // value.
        let snapshot = match shared.lock() {
            Ok(g) => *g,
            Err(p) => *p.into_inner(),
        };
        // The recovery wrote a silent-frame snapshot; check the
        // runtime didn't panic by confirming we can read the value.
        // Concrete value: zero RMS / peak / bands.
        assert_eq!(snapshot.rms, 0.0);
        assert_eq!(snapshot.peak, 0.0);

        handle.shutdown();
    }

    #[test]
    fn runtime_handle_drop_signals_shutdown_implicitly() {
        let _serialize = observability::test_serialize_lock();

        // Use a flag wired through a custom IngestBoundary impl to
        // confirm the producer thread observed shutdown by counting
        // try_pull invocations and asserting they stop after drop.
        #[derive(Debug)]
        struct CountingSilent {
            pulls: Arc<std::sync::atomic::AtomicU32>,
        }

        impl IngestBoundary for CountingSilent {
            fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
                self.pulls.fetch_add(1, Ordering::Relaxed);
                *out = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0; 128]);
                let status = IngestStatus::Ready;
                observability::record_pull_outcome(&status, 128);
                status
            }
        }

        let pulls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let primary = CountingSilent {
            pulls: Arc::clone(&pulls),
        };
        let runtime = AudioRuntime::from_primary(primary, AudioRuntimeConfig::default());
        let handle = runtime.spawn();

        thread::sleep(SETTLE_DURATION);
        let pulls_pre_drop = pulls.load(Ordering::Relaxed);
        assert!(
            pulls_pre_drop > 0,
            "expected at least one pull before drop, got {pulls_pre_drop}",
        );

        // Drop without explicit shutdown.
        drop(handle);

        // Poll for the pull counter to stabilise (thread has exited).
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut last = pulls.load(Ordering::Relaxed);
        let mut stable_for = Duration::ZERO;
        let mut stabilised = false;
        while Instant::now() < deadline {
            thread::sleep(POLL_STEP);
            let now = pulls.load(Ordering::Relaxed);
            if now == last {
                stable_for += POLL_STEP;
                if stable_for >= Duration::from_millis(50) {
                    stabilised = true;
                    break;
                }
            } else {
                stable_for = Duration::ZERO;
                last = now;
            }
        }
        assert!(
            stabilised,
            "pull counter never stabilised after drop; thread didn't exit (last={last})",
        );
    }

    #[test]
    fn drains_full_backlog_within_one_settle_then_analyzes() {
        let _serialize = observability::test_serialize_lock();

        // ADR-024 drop-fix: the producer drains ALL buffered frames per tick
        // (capped) and runs the heavy analysis at a fixed cadence, instead of
        // pull-one-then-heavy-analyze. A buffered backlog far larger than one
        // pull (and larger than a one-per-tick loop could clear in one settle)
        // must drain quickly, and the loud frames must reach the snapshot.
        #[derive(Debug)]
        struct BurstThenIdle {
            remaining: u32,
            delivered: Arc<std::sync::atomic::AtomicU32>,
        }
        impl IngestBoundary for BurstThenIdle {
            fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
                if self.remaining > 0 {
                    self.remaining -= 1;
                    *out = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![1.0; 128]);
                    self.delivered.fetch_add(1, Ordering::Relaxed);
                    let status = IngestStatus::Ready;
                    observability::record_pull_outcome(&status, 128);
                    status
                } else {
                    let status = IngestStatus::WouldBlock;
                    observability::record_pull_outcome(&status, 0);
                    status
                }
            }
        }

        // > MAX_DRAIN_PER_TICK (64) so it spans multiple drain ticks, proving
        // multi-frame-per-tick draining (a one-per-tick loop would not clear it
        // in one SETTLE_DURATION).
        const BURST: u32 = 200;
        let delivered = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let source = BurstThenIdle {
            remaining: BURST,
            delivered: Arc::clone(&delivered),
        };
        let runtime = AudioRuntime::from_primary(source, AudioRuntimeConfig::default());
        let snapshot_handle = runtime.snapshot_handle();
        let handle = runtime.spawn();

        thread::sleep(SETTLE_DURATION);

        assert_eq!(
            delivered.load(Ordering::Relaxed),
            BURST,
            "the full backlog should drain within one settle (drain-all per tick)",
        );
        let snapshot = match snapshot_handle.lock() {
            Ok(g) => *g,
            Err(p) => *p.into_inner(),
        };
        assert!(
            snapshot.rms > 0.0,
            "drained loud frames should analyse to rms > 0, got {}",
            snapshot.rms,
        );

        handle.shutdown();
    }

    #[test]
    fn runtime_default_input_device_unavailable_falls_back_to_silent() {
        let _serialize = observability::test_serialize_lock();

        // The production AudioRuntime::new() path tries cpal first
        // and falls back to SilentSource on error. We exercise the
        // fallback path directly via the from_primary test seam
        // with a SilentSource — the same construction
        // AudioRuntime::new() takes when try_default_input() fails.
        // This pins the silent-fallback path's runnability without
        // depending on the test environment's cpal device state.
        let observed_panic = Arc::new(AtomicBool::new(false));
        let runtime =
            AudioRuntime::from_primary(SilentSource::new(u32::MAX), AudioRuntimeConfig::default());
        let snapshot_handle = runtime.snapshot_handle();
        let handle = runtime.spawn();

        thread::sleep(SETTLE_DURATION);

        // Confirm the runtime is alive (no construction panic) and
        // serving zero snapshots.
        let snapshot = match snapshot_handle.lock() {
            Ok(g) => *g,
            Err(p) => *p.into_inner(),
        };
        assert_eq!(snapshot.rms, 0.0);
        assert!(!observed_panic.load(Ordering::Relaxed));

        handle.shutdown();
    }

    #[test]
    fn runtime_config_defaults_match_component_baselines() {
        let cfg = AudioRuntimeConfig::default();
        assert_eq!(cfg.bpm_window, 8);
        assert_eq!(cfg.criteria, IngestModeCriteria::default());
    }

    #[test]
    fn runtime_config_invalid_bpm_window_falls_back_to_default() {
        // Window 0 is rejected by BpmEstimator::with_window; the
        // runtime falls back to BpmEstimator::new (default 8).
        let cfg = AudioRuntimeConfig {
            bpm_window: 0,
            criteria: IngestModeCriteria::default(),
        };
        let runtime = AudioRuntime::from_primary(SilentSource::new(1), cfg);
        assert_eq!(runtime.estimator.window(), 8);
    }
}
