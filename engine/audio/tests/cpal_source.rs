// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-AUDIO-TRACE (T2) — slice-3b cpal device-capture
//! corpus.
//!
//! T2 per `docs/43 §15.5`: end-to-end exercise of
//! [`chromaticity_audio::cpal_source::CpalSource`] over fixture-
//! substituted producer sides. The cpal-owned data callback is
//! replaced in tests by direct sends into the bounded channel that
//! the production constructor binds via
//! [`chromaticity_audio::cpal_source::CpalSource::from_parts`]; this
//! exercises the [`chromaticity_audio::ingest::IngestBoundary`]
//! surface (Ready / WouldBlock / Closed semantics, sample-rate /
//! channels preservation, observability counter routing, FFI-seam
//! coercion sticking the closed flag) without opening a real audio
//! device.
//!
//! The cpal-side `try_default_input` production path is exercised
//! only by the `#[ignore]`-tagged smoke test below — host devices
//! are environmental and CI-runner-dependent, so that test tolerates
//! either Ok (when a device is present) or `CpalDeviceCapture` (when
//! not).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::bounded;

use chromaticity_audio::cpal_source::{
    CHANNEL_CAPACITY, CpalSource, StreamGuard, try_default_input,
};
use chromaticity_audio::error::AudioError;
use chromaticity_audio::frame::AudioFrame;
use chromaticity_audio::ingest::{IngestBoundary, IngestStatus};

/// Test-only stream guard. Existence + `Drop` semantics are
/// sufficient for the consumer-side surface tests.
struct NoopGuard;
impl StreamGuard for NoopGuard {}

/// Drop-tracking stream guard. Confirms the `CpalSource` releases
/// its stream resources on drop in the same order the production
/// path would.
struct TrackedGuard {
    dropped: Arc<AtomicBool>,
}
impl StreamGuard for TrackedGuard {}
impl Drop for TrackedGuard {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

#[test]
fn pipeline_preserves_sample_rate_and_channels_across_frames() {
    let (tx, rx) = bounded::<AudioFrame>(CHANNEL_CAPACITY);
    let closed = Arc::new(AtomicBool::new(false));
    let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));

    // Pre-canned sequence of distinct frames; the consumer-side pull
    // MUST observe them in FIFO order with the published metadata
    // intact.
    let inputs = [
        AudioFrame::new(48_000, 2, vec![0.10; 256]),
        AudioFrame::new(48_000, 2, vec![0.20; 256]),
        AudioFrame::new(48_000, 2, vec![0.30; 256]),
    ];
    for frame in &inputs {
        tx.try_send(frame.clone()).unwrap();
    }

    let mut out = AudioFrame::default();
    for expected in &inputs {
        assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
        assert_eq!(out.sample_rate_hz(), expected.sample_rate_hz());
        assert_eq!(out.channels(), expected.channels());
        assert_eq!(out.samples().len(), expected.samples().len());
        assert_eq!(out.samples()[0], expected.samples()[0]);
    }

    // Channel drained while sender is still alive: WouldBlock.
    assert_eq!(source.try_pull(&mut out), IngestStatus::WouldBlock);
    drop(tx);
    // Sender dropped: Closed, and Closed sticks.
    assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
    assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
}

#[test]
fn channel_full_does_not_block_producer_side() {
    // Capacity 2 + 3 sends. The third send observes Full; in the
    // production callback that branch DROPS the frame (backpressure
    // policy) and the analyser then sees WouldBlock once the queued
    // frames are drained. Mirror the same shape here using direct
    // try_send to confirm the consumer-side semantics.
    let (tx, rx) = bounded::<AudioFrame>(2);
    let closed = Arc::new(AtomicBool::new(false));
    let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));

    tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 128]))
        .unwrap();
    tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 128]))
        .unwrap();
    let third = tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 128]));
    assert!(third.is_err(), "third send must fail with Full");

    let mut out = AudioFrame::default();
    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(source.try_pull(&mut out), IngestStatus::WouldBlock);
}

#[test]
fn ffi_seam_coerce_to_closed_makes_subsequent_pulls_closed() {
    // Simulate the post-catch-unwind state: closed flag is true,
    // sender is still alive (callback closure hasn't been dropped),
    // but try_pull MUST return Closed and MUST NOT consume from the
    // channel.
    let (tx, rx) = bounded::<AudioFrame>(4);
    let closed = Arc::new(AtomicBool::new(false));
    tx.try_send(AudioFrame::new(48_000, 1, vec![0.5; 128]))
        .unwrap();
    let mut source = CpalSource::from_parts(rx.clone(), closed.clone(), Box::new(NoopGuard));

    // Prior to the panic, a Ready pull works.
    let mut out = AudioFrame::default();
    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(out.samples().len(), 128);

    // Simulate the FFI-seam coercion (catch_unwind path or
    // cpal::StreamError path) flipping the flag.
    closed.store(true, Ordering::Release);

    // Stage another frame on the still-alive sender; the source
    // MUST NOT consume it.
    tx.try_send(AudioFrame::new(48_000, 1, vec![0.99; 128]))
        .unwrap();
    assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
    // The unconsumed frame remains in the channel, demonstrating
    // the source did not re-enter after Closed.
    assert_eq!(rx.len(), 1);
    // Subsequent pulls also return Closed (sticky).
    assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
    assert_eq!(source.try_pull(&mut out), IngestStatus::Closed);
}

#[test]
fn dropping_source_releases_stream_guard() {
    let (_tx, rx) = bounded::<AudioFrame>(4);
    let closed = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let guard = TrackedGuard {
        dropped: dropped.clone(),
    };
    let source = CpalSource::from_parts(rx, closed, Box::new(guard));
    assert!(!dropped.load(Ordering::Acquire));
    drop(source);
    assert!(
        dropped.load(Ordering::Acquire),
        "Dropping CpalSource MUST drop the underlying stream guard",
    );
}

#[test]
fn round_trips_through_dyn_ingest_boundary() {
    // CpalSource MUST be usable as `Box<dyn IngestBoundary + Send>`
    // — that's the analyser's storage shape per the slice-3a
    // negotiator API.
    let (tx, rx) = bounded::<AudioFrame>(4);
    let closed = Arc::new(AtomicBool::new(false));
    tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 128]))
        .unwrap();
    let mut boxed: Box<dyn IngestBoundary + Send> =
        Box::new(CpalSource::from_parts(rx, closed, Box::new(NoopGuard)));
    let mut out = AudioFrame::default();
    assert_eq!(boxed.try_pull(&mut out), IngestStatus::Ready);
}

#[test]
fn audio_error_cpal_device_capture_display_renders_reason() {
    let err = AudioError::CpalDeviceCapture {
        reason: "build_input_stream failed: ...".into(),
    };
    let s = format!("{err}");
    assert!(s.contains("cpal device-capture"));
    assert!(s.contains("build_input_stream"));
}

#[test]
fn observability_routing_accumulates_across_pipeline_frames() {
    use chromaticity_audio::observability::{frames_count, samples_count};
    use chromaticity_observability::catalog::AudioIngestFrameStatus;

    // Count both Ready and WouldBlock outcomes against the process-
    // wide counters. We can't take the test_serialize_lock guard
    // from here (it's pub(crate)), so we use delta semantics with a
    // tolerance — other tests running in parallel may also bump the
    // counters, but the delta MUST be at least as large as the
    // outcomes this test produced.
    let before_ready = frames_count(AudioIngestFrameStatus::Ready);
    let before_wb = frames_count(AudioIngestFrameStatus::WouldBlock);
    let before_samples = samples_count();

    let (tx, rx) = bounded::<AudioFrame>(4);
    let closed = Arc::new(AtomicBool::new(false));
    tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 64]))
        .unwrap();
    tx.try_send(AudioFrame::new(48_000, 1, vec![0.0; 64]))
        .unwrap();
    let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));
    let mut out = AudioFrame::default();
    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(source.try_pull(&mut out), IngestStatus::WouldBlock);

    let after_ready = frames_count(AudioIngestFrameStatus::Ready);
    let after_wb = frames_count(AudioIngestFrameStatus::WouldBlock);
    let after_samples = samples_count();
    assert!(after_ready >= before_ready + 2);
    assert!(after_wb > before_wb);
    assert!(after_samples >= before_samples + 128);
}

#[test]
fn pipeline_accepts_post_conversion_f32_frame_as_if_from_i16_or_u16_arm() {
    // Multi-format dispatch widening: the cpal callback for the
    // i16 / u16 sample-format arms converts each sample to f32
    // before pushing into the bounded channel. The consumer-side
    // surface (CpalSource::try_pull -> AudioFrame) sees an f32
    // payload regardless of the device-side format. This T2 pin
    // exercises that invariant by feeding a Vec<f32> shaped like
    // the i16 / u16 callback output (values in `[-1.0, 1.0]` after
    // the documented per-format conversion) and confirming the
    // pipeline preserves the buffer verbatim with the AudioFrame
    // shape contract intact.
    let (tx, rx) = bounded::<AudioFrame>(CHANNEL_CAPACITY);
    let closed = Arc::new(AtomicBool::new(false));
    let mut source = CpalSource::from_parts(rx, closed, Box::new(NoopGuard));

    // Synthesise two post-conversion buffers: one shaped like the
    // i16 arm output (full positive / negative excursions, zero,
    // half-amplitude) and one shaped like the u16 arm output
    // (offset-binary post-conversion values around 0).
    let i16_post: Vec<f32> = vec![1.0, 0.5, 0.0, -0.5, -1.0];
    let u16_post: Vec<f32> = vec![-1.0, -0.5, 0.0, 0.5, 0.999_969_5];
    tx.try_send(AudioFrame::new(48_000, 1, i16_post.clone()))
        .unwrap();
    tx.try_send(AudioFrame::new(44_100, 2, u16_post.clone()))
        .unwrap();

    let mut out = AudioFrame::default();
    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(out.sample_rate_hz(), 48_000);
    assert_eq!(out.samples(), i16_post.as_slice());

    assert_eq!(source.try_pull(&mut out), IngestStatus::Ready);
    assert_eq!(out.sample_rate_hz(), 44_100);
    assert_eq!(out.channels(), 2);
    assert_eq!(out.samples(), u16_post.as_slice());
}

#[test]
#[ignore = "smoke-only: requires a host audio device; tolerates either Ok or CpalDeviceCapture"]
fn try_default_input_smoke() {
    // Smoke test: confirm the production constructor runs end-to-end
    // on hosts where a default input device exists, and surfaces a
    // structured error otherwise. Either outcome is acceptable; the
    // pin is "the function compiles, links cpal symbols, and returns
    // a concrete result of the documented Result shape".
    match try_default_input() {
        Ok(mut src) => {
            let mut out = AudioFrame::default();
            // Single non-blocking pull. May be Ready / WouldBlock /
            // Closed depending on timing; any outcome is fine.
            let _ = src.try_pull(&mut out);
        }
        Err(AudioError::CpalDeviceCapture { reason }) => {
            assert!(
                !reason.is_empty(),
                "CpalDeviceCapture reason must be populated"
            );
        }
        Err(other) => panic!("unexpected non-CpalDeviceCapture error: {other:?}"),
    }
}
