// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-AUDIO-TRACE slice-3b cpal device-capture ingest source.
//!
//! This module activates the cpal `dep-slot` reserved in slice-2 and
//! lands the first real `cpal::Stream` callback in the audio crate. It
//! produces a [`CpalSource`] that implements
//! [`crate::ingest::IngestBoundary`]: the audio-thread callback owned
//! by cpal pushes [`crate::frame::AudioFrame`] values into a bounded
//! `crossbeam-channel`, and the analyser-side `try_pull` consumes from
//! the receiver end.
//!
//! # Standing constraint — FFI-seam panic discipline
//!
//! Slice-3b honours the standing constraint pinned at
//! `engine/audio/src/ingest.rs` lines 20-52 (the `# Standing
//! constraint — cpal FFI-seam panic discipline` section). Every cpal
//! data callback closure constructed in [`try_default_input`] wraps
//! its body in `std::panic::catch_unwind(AssertUnwindSafe(...))` at
//! the exact boundary where Rust control passes to C. A caught panic
//! is coerced to [`crate::ingest::IngestStatus::Closed`] by setting a
//! shared `closed` flag; subsequent [`CpalSource::try_pull`] calls
//! observe the flag and return `Closed` without re-entering the
//! channel. The cpal-supplied error callback follows the same
//! discipline — any [`cpal::StreamError`] also flips the closed flag.
//!
//! # Backpressure policy
//!
//! The bounded channel has a small capacity (see
//! [`CHANNEL_CAPACITY`]). When the analyser is behind, the audio
//! thread MUST NOT block; instead, the data callback drops the new
//! frame and accepts the staleness. This is the §7.11 degraded-mode
//! contract: under-runs are surfaced as `WouldBlock` to the analyser,
//! never as a stalled audio thread. The
//! [`crate::ingest::IngestState`] state machine rolls those
//! `WouldBlock`s into the Reliability/Fallback Policy degraded-mode
//! transition; the
//! [`crate::ingest_negotiation::IngestNegotiator`] (slice-3a) then
//! demotes the host to the silent fallback if the under-runs persist.
//!
//! # Format scope (slice-3b)
//!
//! Slice-3b lands `f32` device support only. cpal can deliver
//! `f32`/`i16`/`u16` (and platform-specific extras) depending on the
//! default-input config the host returns. Non-`f32` configs surface
//! at construction time as
//! [`crate::error::AudioError::CpalDeviceCapture`] with a reason
//! string; the slice intentionally does NOT silently transcode. Multi-
//! format support is a future widening — the cpal sample-format
//! dispatch and any required transcoding live in slice-3c+ and do
//! not block any Phase-2 evidence row this slice closes.
//!
//! # Production vs test construction
//!
//! [`try_default_input`] opens the host default input and is the
//! production entry point. The internal [`CpalSource::from_parts`]
//! constructor takes a pre-built `(consumer, closed, stream-guard)`
//! triple and is exercised by the slice-3b T1 / T2 corpora — the
//! tests inject a fixture sender side over the same bounded channel
//! shape, so the [`crate::ingest::IngestBoundary`] surface is
//! exercised without opening a real audio device.
//!
//! Governing docs: `docs/06-audio-engine.md` "Reliability/Fallback
//! Policy" (fall back to silent on device failure) and the §7.1
//! ingest baseline (canonical 48 kHz mono, hop cadence 375 Hz).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};

use crate::error::AudioError;
use crate::frame::AudioFrame;
use crate::ingest::{IngestBoundary, IngestStatus};

/// Bounded-channel capacity between the cpal data callback and the
/// analyser. Sized for a few hops of headroom at the §7.1 hop cadence
/// (375 Hz) without keeping more buffered audio than the analyser
/// can ever close on. The exact value is not load-bearing; the
/// `WouldBlock` / drop-on-full backpressure policy makes the channel
/// behave correctly across a wide range of capacities.
pub const CHANNEL_CAPACITY: usize = 16;

/// Type-erased `Send` guard whose `Drop` releases the cpal-managed
/// stream resources.
///
/// `cpal::Stream` is `!Send` on some platforms (notably macOS via
/// the CoreAudio backend); the production guard handles that by
/// pinning the stream to a dedicated owner thread (see
/// [`spawn_stream_owner`]) and exposing only `Send`-safe handles
/// (the closed flag, the consumer end of the bounded channel, and
/// the shutdown signal) to the analyser side. The trait keeps
/// [`CpalSource`] decoupled from cpal types so the consumer-side
/// surface tests can substitute a marker guard without naming any
/// cpal types.
pub trait StreamGuard: Send {}

/// Production stream guard: signals the dedicated cpal-owner thread
/// to shut down on drop, then joins it.
///
/// The stream itself lives on the owner thread (see
/// [`spawn_stream_owner`]) so we never have to coerce a `!Send`
/// `cpal::Stream` across a thread boundary. The guard simply drops
/// the shutdown sender (which signals the owner thread to drop the
/// stream and exit) and joins the thread handle — both are `Send`-
/// safe operations the workspace policy permits without `unsafe`.
struct CpalStreamGuard {
    shutdown: Option<Sender<()>>,
    owner: Option<thread::JoinHandle<()>>,
}

impl StreamGuard for CpalStreamGuard {}

impl Drop for CpalStreamGuard {
    fn drop(&mut self) {
        // Close the shutdown channel first; the owner thread sees a
        // Disconnected on its `recv()` and breaks out of its
        // park-loop to drop the stream.
        if let Some(tx) = self.shutdown.take() {
            drop(tx);
        }
        if let Some(handle) = self.owner.take() {
            // Join is best-effort; a failed join (panicked owner
            // thread) is non-fatal here — the closed flag has
            // already been flipped via the FFI-seam coercion path
            // and the analyser is no longer consuming. We
            // deliberately discard the result rather than `.unwrap()`
            // it (workspace policy denies unwrap in non-test code).
            let _ = handle.join();
        }
    }
}

/// Sender half of the bounded channel, plus the closed flag the
/// callback flips on panic / cpal `StreamError` / non-fatal errors.
///
/// Held inside an `Arc` so the data callback, the error callback,
/// and the owner thread share ownership.
struct CallbackState {
    sender: Sender<AudioFrame>,
    closed: Arc<AtomicBool>,
    /// Stream format published by the owner thread once the device's
    /// default input config is resolved. Wrapped in `Mutex` so the
    /// closures can read the latest values without an `unsafe` cell.
    config: std::sync::Mutex<StreamFormat>,
}

#[derive(Clone, Copy)]
struct StreamFormat {
    sample_rate_hz: u32,
    channels: u16,
}

/// cpal-backed device-capture [`IngestBoundary`].
///
/// Slice-3b production entry: [`try_default_input`]. The struct is
/// `Send` (the cpal-stream guard is held behind a `Box<dyn
/// StreamGuard + Send>`); the `try_pull` surface is non-blocking and
/// returns the `IngestStatus::WouldBlock` / `Closed` outcomes
/// expected by [`crate::ingest::IngestState`] without ever touching
/// the platform device API directly.
pub struct CpalSource {
    consumer: Receiver<AudioFrame>,
    closed: Arc<AtomicBool>,
    /// Type-erased lifetime guard for the underlying `cpal::Stream`.
    /// Keeping the stream alive is the entire role of this field;
    /// `Drop` on the guard releases the audio-thread resources.
    _stream: Box<dyn StreamGuard>,
}

impl core::fmt::Debug for CpalSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CpalSource")
            .field("closed", &self.closed.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl CpalSource {
    /// Test-seam constructor used by both the production
    /// [`try_default_input`] path and the slice-3b T2 corpus
    /// (`engine/audio/tests/cpal_source.rs`).
    ///
    /// Tests inject a fixture sender by binding the `consumer` end
    /// of a `crossbeam-channel` plus a marker [`StreamGuard`] in
    /// lieu of a real `cpal::Stream`; production binds the cpal-
    /// built guard. The constructor is `#[doc(hidden)]` because it
    /// is an internal coupling between the production builder and
    /// the integration-test corpus, not a stable public API; slice-
    /// 3c+ may relocate it.
    #[doc(hidden)]
    pub fn from_parts(
        consumer: Receiver<AudioFrame>,
        closed: Arc<AtomicBool>,
        stream: Box<dyn StreamGuard>,
    ) -> Self {
        Self {
            consumer,
            closed,
            _stream: stream,
        }
    }
}

impl IngestBoundary for CpalSource {
    fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
        let status = if self.closed.load(Ordering::Acquire) {
            IngestStatus::Closed
        } else {
            match self.consumer.try_recv() {
                Ok(frame) => {
                    *out = frame;
                    IngestStatus::Ready
                }
                Err(TryRecvError::Empty) => IngestStatus::WouldBlock,
                Err(TryRecvError::Disconnected) => {
                    // Sender side has gone away (callback dropped or
                    // the stream guard's drop ran); treat as closed
                    // and remember so subsequent pulls don't re-enter
                    // the channel after disconnection.
                    self.closed.store(true, Ordering::Release);
                    IngestStatus::Closed
                }
            }
        };
        crate::observability::record_pull_outcome(&status, out.samples().len());
        status
    }
}

/// Open the host default input device and start a `cpal::Stream`
/// feeding [`CpalSource`].
///
/// The construction sequence:
///
/// 1. Resolve the host default input device.
/// 2. Resolve its default input config; reject non-`f32` formats with
///    [`AudioError::CpalDeviceCapture`].
/// 3. Build the bounded channel + closed flag the callback shares
///    with the consumer.
/// 4. Construct the data callback and the error callback under the
///    FFI-seam `catch_unwind` discipline (see module-level prose).
/// 5. Build the cpal stream and start playback.
///
/// Returns [`AudioError::CpalDeviceCapture`] with a human-readable
/// reason on every fallible cpal operation: device-not-found,
/// default-input-config-not-found, unsupported sample format, stream-
/// build-failure, stream-play-failure.
///
/// # Platform note
///
/// `cpal::Stream` is `!Send` on macOS (CoreAudio backend); the
/// workspace `unsafe_code = forbid` rule prevents a manual `Send`
/// implementation. Slice-3b sidesteps the issue by pinning the
/// stream to a dedicated owner thread (see [`spawn_stream_owner`]):
/// the owner thread builds the stream, calls `play()`, and then
/// blocks on the shutdown channel. Dropping [`CpalSource`] drops the
/// shutdown sender, which signals the owner thread to drop the
/// stream and exit. The analyser side never sees a cpal type and
/// never has to send one across threads.
pub fn try_default_input() -> Result<CpalSource, AudioError> {
    let (sender, consumer) = bounded::<AudioFrame>(CHANNEL_CAPACITY);
    let closed = Arc::new(AtomicBool::new(false));

    let (build_tx, build_rx) = bounded::<Result<StreamSetup, AudioError>>(1);
    let (shutdown_tx, shutdown_rx) = bounded::<()>(1);

    let state = Arc::new(CallbackState {
        sender,
        closed: closed.clone(),
        // Sample rate / channel count are filled in by the owner
        // thread once it has resolved the device's default input
        // config. The audio-thread callback doesn't fire until
        // after the owner thread populates these fields and starts
        // the stream, so racy reads are not a concern in practice;
        // the `Mutex` is workspace-policy-clean (no `unsafe`) and
        // its overhead is negligible at the §7.1 hop cadence.
        config: std::sync::Mutex::new(StreamFormat {
            sample_rate_hz: 0,
            channels: 0,
        }),
    });
    let state_for_owner = state.clone();

    let owner = thread::Builder::new()
        .name("chromaticity-cpal-owner".into())
        .spawn(move || {
            run_stream_owner(state_for_owner, build_tx, shutdown_rx);
        })
        .map_err(|e| AudioError::CpalDeviceCapture {
            reason: format!("failed to spawn cpal-owner thread: {e}"),
        })?;

    // Wait for the owner thread to report build success or a
    // structured construction error.
    let setup = match build_rx.recv() {
        Ok(result) => result,
        Err(_) => Err(AudioError::CpalDeviceCapture {
            reason: "cpal-owner thread exited before reporting build outcome".into(),
        }),
    };

    match setup {
        Ok(_) => Ok(CpalSource::from_parts(
            consumer,
            closed,
            Box::new(CpalStreamGuard {
                shutdown: Some(shutdown_tx),
                owner: Some(owner),
            }),
        )),
        Err(err) => {
            // Owner thread already exited (or is exiting); drop the
            // shutdown channel and join.
            drop(shutdown_tx);
            let _ = owner.join();
            Err(err)
        }
    }
}

/// Build-success marker handed back over the build channel. Carries
/// no fields today; future widening can attach the resolved sample
/// rate / channel count for a host-thread-readable surface.
struct StreamSetup;

/// Owner-thread entrypoint. Builds the cpal stream, starts it,
/// reports the outcome, then blocks on the shutdown channel.
fn run_stream_owner(
    state: Arc<CallbackState>,
    build_tx: Sender<Result<StreamSetup, AudioError>>,
    shutdown_rx: Receiver<()>,
) {
    let stream = match build_default_input_stream(&state) {
        Ok(s) => s,
        Err(e) => {
            // Send the error back; ignore send-failure (caller
            // dropped the build_rx, which is recoverable here).
            let _ = build_tx.send(Err(e));
            return;
        }
    };
    if let Err(e) = stream.play() {
        let _ = build_tx.send(Err(AudioError::CpalDeviceCapture {
            reason: format!("stream.play() failed: {e}"),
        }));
        return;
    }
    if build_tx.send(Ok(StreamSetup)).is_err() {
        // Caller dropped the build channel; tear down.
        return;
    }
    // Block until the analyser drops the shutdown sender. The
    // recv() returns Err on Disconnected, which is the signal to
    // drop the stream and exit.
    let _ = shutdown_rx.recv();
    // Belt-and-suspenders pause; `Drop` on cpal::Stream already
    // pauses but explicit pause keeps the teardown deterministic.
    let _ = stream.pause();
    drop(stream);
}

/// Resolve the host default input device + config and build the
/// `f32` input stream, applying the FFI-seam `catch_unwind`
/// discipline to both the data and error callbacks.
fn build_default_input_stream(state: &Arc<CallbackState>) -> Result<cpal::Stream, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AudioError::CpalDeviceCapture {
            reason: "no default input device".into(),
        })?;
    let supported_config =
        device
            .default_input_config()
            .map_err(|e| AudioError::CpalDeviceCapture {
                reason: format!("default_input_config failed: {e}"),
            })?;
    let sample_format = supported_config.sample_format();
    if sample_format != SampleFormat::F32 {
        return Err(AudioError::CpalDeviceCapture {
            reason: format!(
                "unsupported sample format {sample_format:?}; slice-3b is f32-only (multi-format support is slice-3c+ scope)"
            ),
        });
    }
    let config = supported_config.config();
    let sample_rate_hz = config.sample_rate.0;
    let channels = config.channels;
    if let Ok(mut guard) = state.config.lock() {
        guard.sample_rate_hz = sample_rate_hz;
        guard.channels = channels;
    }
    let state_for_data = state.clone();
    let state_for_err = state.clone();
    let data_cb = move |data: &[f32], _info: &cpal::InputCallbackInfo| {
        // FFI-seam panic discipline: a panic crossing this closure
        // boundary into cpal's C audio thread is undefined
        // behaviour. catch_unwind coerces a panic to a `Closed`
        // signal via the shared atomic, after which `try_pull`
        // surfaces `Closed` to the analyser. The source goes
        // permanently degraded per docs/06 Reliability/Fallback
        // Policy and never re-enters the device API.
        let result = catch_unwind(AssertUnwindSafe(|| {
            handle_data_f32(&state_for_data, data);
        }));
        if result.is_err() {
            state_for_data.closed.store(true, Ordering::Release);
        }
    };
    let err_cb = move |_e: cpal::StreamError| {
        // Mirror the data-callback discipline: any cpal-reported
        // stream error coerces to `Closed`, the analyser sees it
        // via the next `try_pull`, and the source does not re-
        // enter the device API.
        state_for_err.closed.store(true, Ordering::Release);
    };
    device
        .build_input_stream(&config, data_cb, err_cb, None)
        .map_err(|e| AudioError::CpalDeviceCapture {
            reason: format!("build_input_stream failed: {e}"),
        })
}

/// Convert a slice of interleaved `f32` samples handed up by cpal
/// into an [`AudioFrame`] and try to send it down the bounded
/// channel. Drops the frame on `TrySendError::Full` (analyser is
/// behind; backpressure surfaces as `WouldBlock` on the consumer
/// side) and flips the closed flag on `TrySendError::Disconnected`
/// (consumer dropped).
fn handle_data_f32(state: &CallbackState, data: &[f32]) {
    if data.is_empty() {
        return;
    }
    let (sample_rate_hz, channels) = match state.config.lock() {
        Ok(guard) => (guard.sample_rate_hz, guard.channels),
        Err(_) => {
            // Mutex poisoned (a prior callback panicked while
            // holding the lock). Coerce to Closed.
            state.closed.store(true, Ordering::Release);
            return;
        }
    };
    if sample_rate_hz == 0 || channels == 0 {
        // Owner thread hasn't published the format yet; drop the
        // pre-format-publish frame rather than emit a malformed
        // AudioFrame. In practice this branch is unreachable —
        // build_default_input_stream populates the format before
        // returning the stream — but covering it keeps the closed
        // flag honest under concurrent races.
        return;
    }
    let frame = AudioFrame::new(sample_rate_hz, channels, data.to_vec());
    match state.sender.try_send(frame) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            // Drop on full: backpressure policy per the §7.11
            // degraded-mode contract.
        }
        Err(TrySendError::Disconnected(_)) => {
            // Consumer dropped; future callbacks would also fail.
            state.closed.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod cpal_source_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the slice-3b
    //! [`CpalSource`] surface.
    //!
    //! T1 per `docs/43 §15.5`: every test pins either a documented
    //! state-machine invariant on [`CpalSource`] (Ready / WouldBlock
    //! / Closed semantics under the bounded-channel + closed-flag
    //! shape) or a documented behaviour of the FFI-seam coercion
    //! (closed flag sticks across subsequent pulls). The cpal-side
    //! `try_default_input` surface is exercised at T2 elsewhere via
    //! a smoke-only test that tolerates either Ok or
    //! `AudioError::CpalDeviceCapture`.

    use super::*;
    use crate::observability;

    /// Test-only stream guard. Never holds a real `cpal::Stream`;
    /// existence + `Drop` semantics are enough for the consumer-side
    /// surface tests.
    struct NoopGuard;
    impl StreamGuard for NoopGuard {}

    #[test]
    fn try_pull_returns_ready_when_channel_has_frame() {
        let _serialize = observability::test_serialize_lock();
        let (tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(false));
        tx.try_send(AudioFrame::new(48_000, 1, vec![0.25; 128]))
            .unwrap();
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
        assert_eq!(out.sample_rate_hz(), 48_000);
        assert_eq!(out.channels(), 1);
        assert_eq!(out.samples().len(), 128);
    }

    #[test]
    fn try_pull_returns_would_block_when_channel_empty() {
        let _serialize = observability::test_serialize_lock();
        let (_tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(false));
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::WouldBlock);
    }

    #[test]
    fn try_pull_returns_closed_when_sender_dropped() {
        let _serialize = observability::test_serialize_lock();
        let (tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(false));
        drop(tx);
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
    }

    #[test]
    fn try_pull_honors_preset_closed_flag() {
        let _serialize = observability::test_serialize_lock();
        // Sender is held alive but the closed flag is pre-set, so
        // try_pull MUST NOT re-enter the channel and MUST surface
        // Closed. Mirrors the FFI-seam coercion contract.
        let (tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(true));
        tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 128]))
            .unwrap();
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
    }

    #[test]
    fn closed_flag_sticks_across_subsequent_pulls() {
        let _serialize = observability::test_serialize_lock();
        let (tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(false));
        drop(tx);
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
        // After the first Disconnected the source MUST keep
        // returning Closed without re-entering the channel.
        assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
        assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
    }

    #[test]
    fn try_pull_emits_observability_outcome_per_call() {
        // Ready outcome advances the frames + samples counters per
        // the silent.rs reference pattern.
        let _serialize = observability::test_serialize_lock();
        use chromaticity_observability::catalog::AudioIngestFrameStatus;
        let before_ready = observability::frames_count(AudioIngestFrameStatus::Ready);
        let before_samples = observability::samples_count();

        let (tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(false));
        tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 128]))
            .unwrap();
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);

        let after_ready = observability::frames_count(AudioIngestFrameStatus::Ready);
        let after_samples = observability::samples_count();
        assert_eq!(after_ready - before_ready, 1);
        assert_eq!(after_samples - before_samples, 128);
    }

    #[test]
    fn would_block_increments_only_would_block_counter() {
        let _serialize = observability::test_serialize_lock();
        use chromaticity_observability::catalog::AudioIngestFrameStatus;
        let before_wb = observability::frames_count(AudioIngestFrameStatus::WouldBlock);
        let before_samples = observability::samples_count();

        let (_tx, rx) = bounded::<AudioFrame>(4);
        let closed = Arc::new(AtomicBool::new(false));
        let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
        let mut out = AudioFrame::default();
        assert_eq!(source.try_pull(&mut out), IngestStatus::WouldBlock);

        let after_wb = observability::frames_count(AudioIngestFrameStatus::WouldBlock);
        let after_samples = observability::samples_count();
        assert_eq!(after_wb - before_wb, 1);
        // WouldBlock MUST NOT advance the samples counter per
        // docs/31 §Phase-1 baseline catalog.
        assert_eq!(after_samples, before_samples);
    }

    #[test]
    fn audio_error_cpal_device_capture_display_renders_reason() {
        let err = AudioError::CpalDeviceCapture {
            reason: "no default input device".into(),
        };
        let s = format!("{err}");
        assert!(s.contains("cpal device-capture"));
        assert!(s.contains("no default input device"));
    }
}
