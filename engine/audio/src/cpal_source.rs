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
//! # Format scope
//!
//! [`try_default_input`] dispatches over `cpal::SampleFormat` at
//! stream-build time and supports the three formats cpal commonly
//! delivers across the supported hosts: `f32` (CoreAudio default on
//! macOS, often WASAPI on Windows), `i16` (frequent ALSA default on
//! Linux), and `u16` (occasional WASAPI default on Windows). All
//! three formats are converted to `f32` inside the cpal callback
//! before the frame is pushed into the bounded channel; the
//! analyser-side [`crate::ingest::IngestBoundary`] surface stays
//! `f32` regardless of the device-side format. Other cpal-reported
//! sample formats (`i8` / `u8` / `i32` / `u32` / `i64` / `u64` /
//! `f64`, plus any future additions) surface at construction time
//! as [`crate::error::AudioError::CpalDeviceCapture`] with a reason
//! string; the dispatch intentionally does NOT silently transcode
//! formats outside the supported set.
//!
//! Slice-3b shipped `f32`-only support; the multi-format widening is
//! strictly additive cross-platform robustness and does not flip the
//! state of any `EV-AUDIO-TRACE` row in `docs/26`. The R-37 Tier-8
//! narrative explicitly frames multi-format support as "future
//! widening, NOT a named deferred slice".
//!
//! # Conversion math
//!
//! - `f32` is passthrough (no allocation beyond the eventual
//!   `Vec<f32>` copy into the channel-published frame).
//! - `i16` -> `f32`: `(s as f32) / (i16::MAX as f32)`. Yields `+1.0`
//!   at `i16::MAX` and `-1.00003...` at `i16::MIN` — the asymmetry
//!   of two's complement spans one extra integer step on the
//!   negative side; the analyser tolerates the `< -1.0` minority
//!   case via §7.4 clamp.
//! - `u16` -> `f32`: `((s as f32) - 32768.0) / 32768.0`. Yields
//!   `-1.0` at `0`, `0.0` at `32768`, and `+0.99997...` at
//!   `u16::MAX = 65535`. The asymmetry mirrors the `i16` case (one
//!   extra integer step on the negative side).
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

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};

use crate::error::AudioError;
use crate::frame::AudioFrame;
use crate::ingest::{IngestBoundary, IngestStatus};

/// Operator-visible descriptor for a single cpal input device,
/// surfaced by [`enumerate_input_devices`].
///
/// Wave 2.6 Lane AUD.A — backs the `list_audio_devices` Tauri command
/// the operator-facing Settings → Audio Device → Input Device dropdown
/// consumes via the bridge [`AudioDevicePayload`] mirror. Field shape is
/// intentionally minimal — a stable per-device id (the cpal-reported
/// device name today, namespaced under the `input-` prefix to match
/// the MIDI Lane D.1 precedent and reserve room for an `output-`
/// follow-up) plus the operator-visible display name.
///
/// The cpal device name varies by host backend (CoreAudio exposes the
/// device's display name; ALSA exposes its client / port label; WASAPI
/// exposes the WAVEOUTCAPS friendly name), so the bridge layer treats
/// the field as opaque operator-visible text rather than parsing it
/// for vendor / manufacturer hints.
///
/// [`AudioDevicePayload`]: chromaticity_ui_bridge::AudioDevicePayload
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioDeviceInfo {
    /// Stable per-device id. Current shape: `"input-<sanitized-name>"`
    /// against the cpal-reported device name. The id is not stable
    /// across host reconnects (CoreAudio / ALSA / WASAPI can rename or
    /// renumber when devices are plugged or unplugged), so the
    /// operator-facing UI should treat this id as session-stable only.
    pub id: String,
    /// Operator-visible display name from
    /// [`cpal::Device::name`]. Falls back to `"Input <index>"` when the
    /// backend refuses to surface the name (e.g. a transient
    /// disconnection between `input_devices()` and `name()`).
    pub name: String,
}

/// Coarse classification of a cpal audio device direction.
///
/// Wave 2.6 Lane AUD.A shipped [`AudioDeviceKind::Input`]; Wave 2.7
/// Lane SET.C widens the enum with [`AudioDeviceKind::Output`] so the
/// `list_audio_output_devices` Tauri command can enumerate output
/// devices via [`cpal::Host::output_devices`] for the operator-facing
/// Settings → Audio Device → Output Device dropdown. `#[non_exhaustive]`
/// stays in place so future variants (e.g. a loopback discriminant)
/// remain a non-breaking widening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AudioDeviceKind {
    /// Inbound audio device (cpal `input_devices()` enumeration).
    Input,
    /// Outbound audio device (cpal `output_devices()` enumeration).
    Output,
}

/// Enumerate the cpal input devices currently visible to the host.
///
/// Returns one [`AudioDeviceInfo`] per device surfaced by a fresh
/// [`cpal::Host::input_devices`] call. On a host with no audio
/// subsystem access (CI / headless / a Linux box without ALSA), cpal's
/// `input_devices()` errors at iteration construction; the function
/// collapses that error to `Ok(Vec::new())` so the Tauri command
/// surface treats "no audio available" as the same empty-list state
/// as "audio available but no devices connected" — the frontend's
/// fixture-fallback branch fires identically in both cases. Per-device
/// name-retrieval failures (transient disconnection) fall back to
/// `"Input <index>"` rather than skipping the device entirely.
///
/// Mirrors the [`crate::cpal_source::try_default_input`] error model:
/// returns [`AudioError::CpalDeviceCapture`] for the genuinely
/// unrecoverable error class. There are no producers of that branch
/// in Wave 2.6 Lane AUD.A's MVP body (the no-subsystem / no-devices
/// branches both collapse to `Ok(Vec::new())`), but the return type
/// stays `Result<_, AudioError>` to match the crate's existing audio
/// surface and so the follow-up output-enumeration wave can introduce
/// real error producers without churning the API contract.
///
/// Evidence: `EV-INTEGRATION-MATRIX` (audio arm, Wave 2.6 Lane AUD.A)
/// — device-listing surface widened from the W21 PR1 fixture-only
/// posture documented in `src/lib/data/settings-fixtures.ts`.
pub fn enumerate_input_devices() -> Result<Vec<AudioDeviceInfo>, AudioError> {
    let host = cpal::default_host();
    let devices = match host.input_devices() {
        Ok(d) => d,
        // CI / headless / no-audio-subsystem hosts: collapse to empty
        // vec rather than erroring. Mirrors the
        // `chromaticity_midi_adapter::list_devices` posture for the
        // parallel MIDI enumeration surface.
        Err(_) => return Ok(Vec::new()),
    };
    let infos = devices
        .enumerate()
        .map(|(idx, device)| {
            let name = device.name().unwrap_or_else(|_| format!("Input {idx}"));
            // Namespace the id under `input-` to match the MIDI Lane
            // D.1 precedent and reserve room for an `output-` follow-up.
            // Sanitisation strips whitespace runs to a single dash so
            // the id round-trips through a URL / DOM selector if the
            // operator-facing UI ever uses it as a CSS hook.
            let sanitized = sanitize_device_id_segment(&name);
            AudioDeviceInfo {
                id: format!("input-{sanitized}"),
                name,
            }
        })
        .collect();
    Ok(infos)
}

/// Enumerate the cpal output devices currently visible to the host.
///
/// Returns one [`AudioDeviceInfo`] per device surfaced by a fresh
/// [`cpal::Host::output_devices`] call. Mirrors the
/// [`enumerate_input_devices`] empty-vec graceful-fallback posture: on
/// a host with no audio subsystem access (CI / headless / a Linux box
/// without ALSA), cpal's `output_devices()` errors at iteration
/// construction; the function collapses that error to `Ok(Vec::new())`
/// so the Tauri command surface treats "no audio available" as the
/// same empty-list state as "audio available but no devices
/// connected" — the frontend's fixture-fallback branch fires
/// identically in both cases. Per-device name-retrieval failures
/// (transient disconnection) fall back to `"Output <index>"` rather
/// than skipping the device entirely.
///
/// The id is namespaced under the `output-` prefix to match the
/// [`enumerate_input_devices`] `input-` precedent and so a payload
/// id alone is sufficient to disambiguate the device direction without
/// inspecting the surrounding [`AudioDeviceKind`] discriminant.
///
/// Evidence: `EV-INTEGRATION-MATRIX` (audio arm, Wave 2.7 Lane SET.C)
/// — output device-listing surface widened from the W21 PR1
/// fixture-only posture documented in
/// `src/lib/data/settings-fixtures.ts`.
pub fn enumerate_output_devices() -> Result<Vec<AudioDeviceInfo>, AudioError> {
    let host = cpal::default_host();
    let devices = match host.output_devices() {
        Ok(d) => d,
        // CI / headless / no-audio-subsystem hosts: collapse to empty
        // vec rather than erroring. Mirrors the Wave 2.6 Lane AUD.A
        // `enumerate_input_devices` posture.
        Err(_) => return Ok(Vec::new()),
    };
    let infos = devices
        .enumerate()
        .map(|(idx, device)| {
            let name = device.name().unwrap_or_else(|_| format!("Output {idx}"));
            let sanitized = sanitize_device_id_segment(&name);
            AudioDeviceInfo {
                id: format!("output-{sanitized}"),
                name,
            }
        })
        .collect();
    Ok(infos)
}

/// Sanitise a cpal device name into a single id segment.
///
/// Replaces whitespace runs with a single ASCII dash and lowercases
/// the result. Non-ASCII characters pass through unchanged so the id
/// stays a stable round-trip of the operator-visible name without
/// dropping locale-specific characters.
fn sanitize_device_id_segment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_was_space = false;
    for ch in name.chars() {
        if ch.is_whitespace() {
            if !prev_was_space {
                out.push('-');
                prev_was_space = true;
            }
        } else {
            out.extend(ch.to_lowercase());
            prev_was_space = false;
        }
    }
    out.trim_matches('-').to_string()
}

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
/// 2. Resolve its default input config; dispatch over
///    `cpal::SampleFormat` to build a per-format input stream
///    (`f32` passthrough, `i16` -> `f32` normalisation, `u16` ->
///    `f32` normalisation). Other formats are rejected with
///    [`AudioError::CpalDeviceCapture`].
/// 3. Build the bounded channel + closed flag the callback shares
///    with the consumer.
/// 4. Construct the data callback and the error callback under the
///    FFI-seam `catch_unwind` discipline (see module-level prose).
///    The discipline is preserved at every per-format closure seam.
/// 5. Build the cpal stream and start playback.
///
/// Returns [`AudioError::CpalDeviceCapture`] with a human-readable
/// reason on every fallible cpal operation: device-not-found,
/// default-input-config-not-found, unsupported sample format
/// (anything other than `f32` / `i16` / `u16`), stream-build-failure,
/// stream-play-failure.
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
/// per-format input stream, applying the FFI-seam `catch_unwind`
/// discipline to both the data and error callbacks at every
/// format-arm closure seam.
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
    let config = supported_config.config();
    let sample_rate_hz = config.sample_rate.0;
    let channels = config.channels;
    if let Ok(mut guard) = state.config.lock() {
        guard.sample_rate_hz = sample_rate_hz;
        guard.channels = channels;
    }
    let state_for_err = state.clone();
    let err_cb = move |_e: cpal::StreamError| {
        // Mirror the data-callback discipline: any cpal-reported
        // stream error coerces to `Closed`, the analyser sees it
        // via the next `try_pull`, and the source does not re-
        // enter the device API.
        state_for_err.closed.store(true, Ordering::Release);
    };
    // Per-format dispatch. Every arm wraps its conversion + frame-
    // publish work in catch_unwind at the FFI seam — a panic in any
    // arm flips the AtomicBool closed flag and the source stops
    // re-entering the device API. Conversion math lives in pure
    // helpers (`convert_i16_sample` / `convert_u16_sample`) so the
    // T1 drift pins can exercise it without a real cpal stream.
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let state_for_data = state.clone();
            let data_cb = move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    publish_f32_frame(&state_for_data, data);
                }));
                if result.is_err() {
                    state_for_data.closed.store(true, Ordering::Release);
                }
            };
            device.build_input_stream(&config, data_cb, err_cb, None)
        }
        cpal::SampleFormat::I16 => {
            let state_for_data = state.clone();
            let data_cb = move |data: &[i16], _info: &cpal::InputCallbackInfo| {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    // Per-callback Vec allocation; small (typically
                    // 128-1024 samples) and acceptable on the audio
                    // thread for first-cut multi-format support.
                    // Future widening could reuse a thread-local
                    // scratch buffer if profiling shows pressure.
                    let f32_samples: Vec<f32> =
                        data.iter().copied().map(convert_i16_sample).collect();
                    publish_f32_frame(&state_for_data, &f32_samples);
                }));
                if result.is_err() {
                    state_for_data.closed.store(true, Ordering::Release);
                }
            };
            device.build_input_stream(&config, data_cb, err_cb, None)
        }
        cpal::SampleFormat::U16 => {
            let state_for_data = state.clone();
            let data_cb = move |data: &[u16], _info: &cpal::InputCallbackInfo| {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let f32_samples: Vec<f32> =
                        data.iter().copied().map(convert_u16_sample).collect();
                    publish_f32_frame(&state_for_data, &f32_samples);
                }));
                if result.is_err() {
                    state_for_data.closed.store(true, Ordering::Release);
                }
            };
            device.build_input_stream(&config, data_cb, err_cb, None)
        }
        other => {
            return Err(AudioError::CpalDeviceCapture {
                reason: format!("unsupported sample format {other:?}; supported: f32, i16, u16"),
            });
        }
    };
    stream.map_err(|e| AudioError::CpalDeviceCapture {
        reason: format!("build_input_stream failed: {e}"),
    })
}

/// Convert a single `i16` PCM sample to `f32` in the nominal
/// `[-1.0, 1.0]` range. `i16::MAX` maps to exactly `+1.0`;
/// `i16::MIN` maps to slightly below `-1.0` (`-1.00003...`) per the
/// asymmetry of two's complement. The analyser tolerates the
/// `< -1.0` minority case via the §7.4 clamp.
#[inline]
pub(crate) fn convert_i16_sample(s: i16) -> f32 {
    f32::from(s) / f32::from(i16::MAX)
}

/// Convert a single `u16` PCM sample to `f32` in the nominal
/// `[-1.0, 1.0]` range. `0` maps to `-1.0`, `32768` maps to `0.0`,
/// `u16::MAX = 65535` maps to `+0.99997...`. The asymmetry mirrors
/// the `i16` case (one extra integer step on the negative side).
#[inline]
pub(crate) fn convert_u16_sample(s: u16) -> f32 {
    (f32::from(s) - 32768.0) / 32768.0
}

/// Convert a slice of interleaved `f32` samples (already-converted
/// by the per-format closure if the device delivered `i16` / `u16`)
/// into an [`AudioFrame`] and try to send it down the bounded
/// channel. Drops the frame on `TrySendError::Full` (analyser is
/// behind; backpressure surfaces as `WouldBlock` on the consumer
/// side) and flips the closed flag on `TrySendError::Disconnected`
/// (consumer dropped).
fn publish_f32_frame(state: &CallbackState, data: &[f32]) {
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

    // ---- Multi-format dispatch widening pins ----
    //
    // T1 per docs/43 §15.5: each pin asserts a documented invariant
    // of the per-format conversion math (i16 / u16 -> f32). The
    // module-level `# Conversion math` prose is the contract these
    // pins protect.

    /// Tolerance for f32 conversion-math pins. The conversions are
    /// exact-rational divisions by integer constants; this gives
    /// margin for the inherent rounding of f32 multiply / divide.
    const CONVERSION_TOLERANCE: f32 = 1.0e-6;

    #[test]
    fn i16_to_f32_conversion_at_max_yields_unit_amplitude() {
        // Documented invariant: i16::MAX maps to exactly +1.0.
        let v = convert_i16_sample(i16::MAX);
        assert!(
            (v - 1.0).abs() < CONVERSION_TOLERANCE,
            "i16::MAX -> {v}, expected +1.0",
        );
    }

    #[test]
    fn i16_to_f32_conversion_at_min_yields_negative_unit() {
        // Documented invariant: i16::MIN maps to slightly below -1.0
        // (-32768 / 32767 = -1.00003...). The asymmetry of two's
        // complement gives one extra integer step on the negative
        // side; the analyser tolerates the `< -1.0` minority case
        // via the §7.4 clamp.
        let v = convert_i16_sample(i16::MIN);
        let expected: f32 = -32768.0 / 32767.0;
        assert!(
            (v - expected).abs() < CONVERSION_TOLERANCE,
            "i16::MIN -> {v}, expected {expected}",
        );
        // Pin the < -1.0 invariant explicitly so a future "shift
        // + scale" rewrite that lands at exactly -1.0 surfaces here.
        assert!(v < -1.0, "i16::MIN -> {v}, must be < -1.0");
    }

    #[test]
    fn i16_to_f32_conversion_at_zero_yields_zero() {
        // Documented invariant: 0 maps to exactly 0.0.
        let v = convert_i16_sample(0);
        assert!(
            v.abs() < CONVERSION_TOLERANCE,
            "i16::0 -> {v}, expected 0.0",
        );
    }

    #[test]
    fn u16_to_f32_conversion_at_zero_yields_negative_one() {
        // Documented invariant: 0 maps to exactly -1.0 (offset-binary
        // origin).
        let v = convert_u16_sample(0);
        assert!(
            (v - (-1.0)).abs() < CONVERSION_TOLERANCE,
            "u16::0 -> {v}, expected -1.0",
        );
    }

    #[test]
    fn u16_to_f32_conversion_at_max_yields_unit_amplitude() {
        // Documented invariant: u16::MAX = 65535 maps to slightly
        // below +1.0 (32767 / 32768 = +0.99997...). Mirrors the i16
        // asymmetry on the positive side.
        let v = convert_u16_sample(u16::MAX);
        let expected: f32 = 32767.0 / 32768.0;
        assert!(
            (v - expected).abs() < CONVERSION_TOLERANCE,
            "u16::MAX -> {v}, expected {expected}",
        );
        assert!(v < 1.0, "u16::MAX -> {v}, must be < 1.0");
        assert!(v > 0.0, "u16::MAX -> {v}, must be > 0.0");
    }

    #[test]
    fn u16_to_f32_conversion_at_center_yields_zero() {
        // Documented invariant: 32768 (offset-binary origin) maps to
        // exactly 0.0.
        let v = convert_u16_sample(32768);
        assert!(
            v.abs() < CONVERSION_TOLERANCE,
            "u16::32768 -> {v}, expected 0.0",
        );
    }

    #[test]
    fn i16_slice_conversion_round_trips_through_publish_helper_shape() {
        // Pin: a slice of i16 samples converted via `convert_i16_sample`
        // produces a Vec<f32> the publish helper would accept (length
        // preserved, values in the documented range modulo the i16::MIN
        // negative overshoot).
        let raw: [i16; 5] = [i16::MIN, -16384, 0, 16384, i16::MAX];
        let f: Vec<f32> = raw.iter().copied().map(convert_i16_sample).collect();
        assert_eq!(f.len(), raw.len());
        // -16384 / 32767 ≈ -0.5
        assert!((f[1] - (-16384.0 / 32767.0)).abs() < CONVERSION_TOLERANCE);
        assert!(f[2].abs() < CONVERSION_TOLERANCE);
        // 16384 / 32767 ≈ +0.5
        assert!((f[3] - (16384.0 / 32767.0)).abs() < CONVERSION_TOLERANCE);
    }

    #[test]
    fn u16_slice_conversion_round_trips_through_publish_helper_shape() {
        let raw: [u16; 5] = [0, 16384, 32768, 49152, u16::MAX];
        let f: Vec<f32> = raw.iter().copied().map(convert_u16_sample).collect();
        assert_eq!(f.len(), raw.len());
        // 16384 -> (16384 - 32768) / 32768 = -0.5
        assert!((f[1] - (-0.5)).abs() < CONVERSION_TOLERANCE);
        assert!(f[2].abs() < CONVERSION_TOLERANCE);
        // 49152 -> (49152 - 32768) / 32768 = +0.5
        assert!((f[3] - 0.5).abs() < CONVERSION_TOLERANCE);
    }

    // ---- Wave 2.6 Lane AUD.A device-enumeration pins ----
    //
    // T1 per docs/43 §15.5: pins the documented shape contract for
    // `enumerate_input_devices`. The function MUST NOT error on any
    // host (CI / headless / no-audio-subsystem branches collapse to
    // `Ok(Vec::new())` per the parallel `chromaticity_midi_adapter::list_devices`
    // posture); every returned descriptor MUST carry a non-empty id +
    // name; ids MUST be namespaced under the `input-` prefix to match
    // the MIDI Lane D.1 precedent. Evidence: `EV-INTEGRATION-MATRIX`
    // (audio arm, Wave 2.6 Lane AUD.A).

    #[test]
    fn enumerate_input_devices_does_not_error_on_any_host() {
        // The enumeration MUST NOT propagate the cpal host-init
        // failure to the caller — CI runners typically have no
        // input device available. The empty vec under that condition
        // is the contract; on a developer host with audio subsystem
        // access AND zero connected devices, the result is also
        // empty; on a host WITH connected devices the result is
        // non-empty. All three cases pass this assertion.
        let result = enumerate_input_devices()
            .expect("enumerate_input_devices must not error at MVP surface");
        for info in &result {
            assert!(
                info.id.starts_with("input-"),
                "expected id to start with `input-`, got {:?}",
                info.id,
            );
            assert!(!info.name.is_empty(), "cpal device name must not be empty");
            // The id segment after the `input-` prefix MUST NOT be
            // empty either — `sanitize_device_id_segment` strips
            // trailing dashes after whitespace collapse, so a
            // pathological all-whitespace name would surface here.
            let suffix = info.id.strip_prefix("input-").unwrap_or("");
            assert!(
                !suffix.is_empty(),
                "id suffix must not be empty, got {:?}",
                info.id,
            );
        }
    }

    #[test]
    fn sanitize_device_id_segment_collapses_whitespace_runs() {
        // Documented invariant: whitespace runs collapse to a single
        // dash; the result is lowercased; leading / trailing dashes
        // strip.
        assert_eq!(
            sanitize_device_id_segment("Built-in Microphone"),
            "built-in-microphone"
        );
        assert_eq!(sanitize_device_id_segment("Yeti  Classic"), "yeti-classic");
        assert_eq!(sanitize_device_id_segment("  Padded  "), "padded");
        assert_eq!(
            sanitize_device_id_segment("BlackHole 16ch"),
            "blackhole-16ch"
        );
    }

    #[test]
    fn audio_device_kind_input_variant_is_construct_and_clone_safe() {
        // Pin the Wave 2.6 Lane AUD.A variant; Wave 2.7 Lane SET.C
        // widens with `Output` (covered in the sibling pin below).
        let kind = AudioDeviceKind::Input;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    // ---- Wave 2.7 Lane SET.C output-enumeration pins ----
    //
    // T1 per docs/43 §15.5: pins the documented shape contract for
    // `enumerate_output_devices`. The function MUST NOT error on any
    // host (CI / headless / no-audio-subsystem branches collapse to
    // `Ok(Vec::new())` per the Wave 2.6 Lane AUD.A `enumerate_input_devices`
    // precedent); every returned descriptor MUST carry a non-empty id +
    // name; ids MUST be namespaced under the `output-` prefix to match
    // the `input-` precedent and so the operator-facing payload id
    // alone is sufficient to disambiguate the device direction.
    // Evidence: `EV-INTEGRATION-MATRIX` (audio arm, Wave 2.7 Lane SET.C).

    #[test]
    fn enumerate_output_devices_does_not_error_on_any_host() {
        // Same three-case coverage as the Wave 2.6 Lane AUD.A input
        // pin: no-audio-subsystem hosts collapse to `Ok(Vec::new())`;
        // hosts with audio access AND zero output devices return
        // empty; hosts WITH connected output devices return non-empty.
        // All three cases pass this assertion.
        let result = enumerate_output_devices()
            .expect("enumerate_output_devices must not error at MVP surface");
        for info in &result {
            assert!(
                info.id.starts_with("output-"),
                "expected id to start with `output-`, got {:?}",
                info.id,
            );
            assert!(!info.name.is_empty(), "cpal device name must not be empty");
            let suffix = info.id.strip_prefix("output-").unwrap_or("");
            assert!(
                !suffix.is_empty(),
                "id suffix must not be empty, got {:?}",
                info.id,
            );
        }
    }

    #[test]
    fn audio_device_kind_output_variant_is_construct_and_clone_safe() {
        // Wave 2.7 Lane SET.C widens the `#[non_exhaustive]` enum
        // with the `Output` variant; the bridge mirror's `Output`
        // variant pre-existed as a scaffold (Wave 2.6 Lane AUD.A)
        // so the wire shape is unchanged.
        let kind = AudioDeviceKind::Output;
        let cloned = kind;
        assert_eq!(kind, cloned);
        assert_ne!(kind, AudioDeviceKind::Input);
    }
}
