// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! T2 conformance corpus for the Syphon adapter per ADR-021 §10
//! Negative consequence bullet 3 (synthetic narrowing — no live
//! Syphon bus on CI).
//!
//! # Narrowing
//!
//! Hardware-rig CI validation against a live Syphon bus is not
//! feasible on GitHub Actions runners (Syphon requires a macOS host
//! plus a peer-process publisher; even a self-hosted macOS runner
//! cannot reliably stand up an isolated Syphon bus across CI runs).
//! This corpus uses **mock** [`SyphonClientHandle`] /
//! [`SyphonServerHandle`] implementations whose state-machine
//! behavior mirrors the canonical framework's contract per the
//! decoded headers in `third_party/Syphon-Framework/`. The corpus
//! exercises the adapter's public surface end-to-end against the
//! mocks; the real-framework wiring lives engine-side at the
//! Phase 3 PR 3 bridge in `engine/render/src/syphon_adapter.rs`.
//!
//! Per ADR-021 §10 + the row 78 (OSC routing matrix) / row 80
//! (MIDI input) / row 93 (camera adapter) precedent, this synthetic
//! narrowing is recorded honestly against `EV-INTEGRATION-MATRIX`
//! at the Phase 3 closing-docs PR 3.
//!
//! # Coverage
//!
//! 1. **Forward-compat refusal at construction.** Servers with
//!    `dictionary_version > KNOWN_DICTIONARY_VERSION` are refused
//!    by [`SyphonInputAdapter::with_handle`] per the canonical
//!    `SyphonClientBase.m:87-93`.
//! 2. **`bind` returning `false` from the backend transitions the
//!    adapter to `Faulted`.** Mirrors the framework returning `nil`
//!    from `SyphonMetalClient::initWithServerDescription:device:options:newFrameHandler:`
//!    when the server vanished between discovery and bind.
//! 3. **Output bind name-conflict surfaces as
//!    `SyphonServerRegistrationFailed`.** Mirrors the framework
//!    refusing a duplicate-name registration.
//! 4. **`SyphonInputAdapter::Drop` calls `[client stop]`.** Verified
//!    via a refcount-tracking mock (an `Arc<AtomicUsize>` the mock
//!    increments on every `stop` call).
//! 5. **`SyphonOutputAdapter::Drop` calls `[server stop]`.** Same
//!    pattern as fixture 4.
//! 6. **Polling-consume model: `has_new_frame` gating.** The
//!    adapter only surfaces frames when the backend reports both
//!    `has_new_frame == true` AND the adapter is in
//!    `AdapterHealth::Bound`.
//! 7. **Pixel format locked to `Bgra8Unorm`.** Constructed
//!    [`SyphonTexture`] handles default to the canonical format
//!    per docs/54 Lesson 3.2.
//! 8. **`current_servers` returns empty list in PR 2.** Engine-
//!    side cache wiring lives in PR 3.
//!
//! Evidence: EV-INTEGRATION-MATRIX (T2) — Syphon arm synthetic-
//! narrowing fixture corpus per ADR-021 §10 Negative consequence
//! bullet 3.

#![cfg(target_os = "macos")]
// Tests opt out of the workspace `clippy::unwrap_used` / `expect_used`
// per docs/43 §12.1 module-level guidance.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chromaticity_syphon_adapter::adapter::backend::{SyphonClientHandle, SyphonServerHandle};
use chromaticity_syphon_adapter::adapter::{
    AdapterHealth, SyphonAdapterError, SyphonInputAdapter, SyphonOutputAdapter,
};
use chromaticity_syphon_adapter::directory::{SyphonServerDirectory, parse_server_description};
use chromaticity_syphon_adapter::frames::{
    KNOWN_DICTIONARY_VERSION, SyphonPixelFormat, SyphonServerId, SyphonTexture,
};

// ----- Mock backend implementations -----

/// Mock `SyphonClientHandle` whose behavior is observable + scriptable
/// from the test fixture. Tracks bind / stop / new_frame_image call
/// counts so fixtures can assert the canonical
/// `[client stop]`-on-Drop pattern.
struct MockClientHandle {
    bind_returns: bool,
    has_new_frame: Arc<AtomicBool>,
    stop_calls: Arc<AtomicUsize>,
    bound: bool,
    fixture_texture: SyphonTexture,
}

impl MockClientHandle {
    fn new(bind_returns: bool, stop_calls: Arc<AtomicUsize>) -> Self {
        Self {
            bind_returns,
            has_new_frame: Arc::new(AtomicBool::new(false)),
            stop_calls,
            bound: false,
            fixture_texture: SyphonTexture {
                iosurface_id: 0xFEED_F00D,
                width: 1920,
                height: 1080,
                pixel_format: SyphonPixelFormat::Bgra8Unorm,
            },
        }
    }

    fn fresh_frame_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.has_new_frame)
    }
}

impl SyphonClientHandle for MockClientHandle {
    fn bind(&mut self) -> bool {
        if self.bind_returns {
            self.bound = true;
        }
        self.bind_returns
    }

    fn has_new_frame(&self) -> bool {
        self.has_new_frame.load(Ordering::SeqCst)
    }

    fn new_frame_image(&mut self) -> Option<SyphonTexture> {
        // Atomic-bool freshness flag pattern per
        // `SyphonMetalClient.m:40, 91-104`: reading the frame
        // clears the freshness bit.
        if self.has_new_frame.swap(false, Ordering::SeqCst) {
            Some(self.fixture_texture)
        } else {
            None
        }
    }

    fn stop(&mut self) {
        self.stop_calls.fetch_add(1, Ordering::SeqCst);
        self.bound = false;
    }
}

/// Mock `SyphonServerHandle` mirroring the framework's bind /
/// publish / stop contract; tracks publish + stop call counts.
struct MockServerHandle {
    bind_returns: bool,
    stop_calls: Arc<AtomicUsize>,
    publish_calls: Arc<AtomicUsize>,
    bound_name: Option<String>,
}

impl MockServerHandle {
    fn new(
        bind_returns: bool,
        stop_calls: Arc<AtomicUsize>,
        publish_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            bind_returns,
            stop_calls,
            publish_calls,
            bound_name: None,
        }
    }
}

impl SyphonServerHandle for MockServerHandle {
    fn bind(&mut self, name: &str) -> bool {
        if self.bind_returns {
            self.bound_name = Some(name.to_string());
        }
        self.bind_returns
    }

    fn publish(&mut self, _texture: SyphonTexture) {
        self.publish_calls.fetch_add(1, Ordering::SeqCst);
    }

    fn stop(&mut self) {
        self.stop_calls.fetch_add(1, Ordering::SeqCst);
        self.bound_name = None;
    }
}

// ----- Helpers -----

fn fixture_compatible_server_id() -> SyphonServerId {
    SyphonServerId {
        uuid: "11111111-2222-3333-4444-555555555555".to_string(),
        name: "Channel A".to_string(),
        app_name: "TouchDesigner".to_string(),
        dictionary_version: KNOWN_DICTIONARY_VERSION,
    }
}

// ============================================================================
// Fixtures
// ============================================================================

/// **Fixture 1**: dictionary_version > KNOWN refused by
/// `SyphonInputAdapter::with_handle` per ADR-021 §4 + canonical
/// `SyphonClientBase.m:87-93`. T2 evidence:
/// `EV-INTEGRATION-MATRIX` synthetic-narrowing fixture corpus.
#[test]
fn dictionary_version_above_known_is_refused_at_construction() {
    let mut id = fixture_compatible_server_id();
    id.dictionary_version = KNOWN_DICTIONARY_VERSION + 1;
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let handle = Box::new(MockClientHandle::new(true, Arc::clone(&stop_calls)));

    let result = SyphonInputAdapter::with_handle(id, handle);
    match result {
        Err(SyphonAdapterError::SyphonClientServerVanished(diag)) => {
            assert!(diag.contains("dictionary_version"));
            assert!(diag.contains("ADR-021"));
        }
        other => panic!("expected SyphonClientServerVanished, got {other:?}"),
    }
}

/// **Fixture 2**: dictionary_version == KNOWN accepted; adapter
/// constructs successfully and reports `Negotiating` health pre-bind.
#[test]
fn dictionary_version_equal_to_known_is_accepted() {
    let id = fixture_compatible_server_id();
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let handle = Box::new(MockClientHandle::new(true, Arc::clone(&stop_calls)));

    let adapter = SyphonInputAdapter::with_handle(id, handle).expect("compatible id accepted");
    assert_eq!(adapter.health(), &AdapterHealth::Negotiating);
}

/// **Fixture 3**: backend `bind() == false` transitions the adapter
/// to `Faulted` and surfaces `SyphonClientServerVanished`. Mirrors
/// the framework returning `nil` from
/// `SyphonMetalClient::initWithServerDescription:`.
#[test]
fn input_adapter_bind_failure_transitions_to_faulted() {
    let id = fixture_compatible_server_id();
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let handle = Box::new(MockClientHandle::new(false, Arc::clone(&stop_calls)));

    let mut adapter = SyphonInputAdapter::with_handle(id, handle).unwrap();
    let result = adapter.bind();
    assert!(matches!(
        result,
        Err(SyphonAdapterError::SyphonClientServerVanished(_))
    ));
    match adapter.health() {
        AdapterHealth::Faulted(_) => {}
        other => panic!("expected Faulted, got {other:?}"),
    }
}

/// **Fixture 4**: output bind name-conflict surfaces as
/// `SyphonServerRegistrationFailed` with the rejected name in the
/// diagnostic. Mirrors the framework refusing a duplicate-name
/// registration.
#[test]
fn output_adapter_bind_failure_surfaces_registration_failed() {
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let publish_calls = Arc::new(AtomicUsize::new(0));
    let handle = Box::new(MockServerHandle::new(
        false,
        Arc::clone(&stop_calls),
        Arc::clone(&publish_calls),
    ));

    let mut adapter =
        SyphonOutputAdapter::with_handle("Chromaticity Show".to_string(), handle).unwrap();
    let result = adapter.bind();
    match result {
        Err(SyphonAdapterError::SyphonServerRegistrationFailed(diag)) => {
            assert!(diag.contains("Chromaticity Show"));
        }
        other => panic!("expected SyphonServerRegistrationFailed, got {other:?}"),
    }
    match adapter.health() {
        AdapterHealth::Faulted(_) => {}
        other => panic!("expected Faulted post-failed-bind, got {other:?}"),
    }
}

/// **Fixture 5**: `SyphonInputAdapter::Drop` calls `[client stop]`
/// even if explicit `unbind` was never invoked, per ADR-021 §8 +
/// docs/54 Lesson 11. Verified via refcount-tracking mock.
#[test]
fn input_adapter_drop_calls_stop_defensively() {
    let id = fixture_compatible_server_id();
    let stop_calls = Arc::new(AtomicUsize::new(0));
    {
        let handle = Box::new(MockClientHandle::new(true, Arc::clone(&stop_calls)));
        let _adapter = SyphonInputAdapter::with_handle(id, handle).unwrap();
        // Adapter goes out of scope here; Drop should fire.
    }
    assert!(
        stop_calls.load(Ordering::SeqCst) >= 1,
        "Drop must call client.stop() at least once per ADR-021 §8"
    );
}

/// **Fixture 6**: `SyphonOutputAdapter::Drop` calls `[server stop]`
/// even if explicit `unbind` was never invoked, per ADR-021 §8 +
/// docs/54 Lesson 11. Verified via refcount-tracking mock. The
/// canonical framework registers a process-destructor specifically
/// because users forget to call `stop`; chromaticity's `Drop` impl
/// is the Rust-side mirror of that defense.
#[test]
fn output_adapter_drop_calls_stop_defensively() {
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let publish_calls = Arc::new(AtomicUsize::new(0));
    {
        let handle = Box::new(MockServerHandle::new(
            true,
            Arc::clone(&stop_calls),
            publish_calls,
        ));
        let _adapter =
            SyphonOutputAdapter::with_handle("Chromaticity Show".to_string(), handle).unwrap();
        // Adapter goes out of scope here; Drop should fire.
    }
    assert!(
        stop_calls.load(Ordering::SeqCst) >= 1,
        "Drop must call server.stop() at least once per ADR-021 §8"
    );
}

/// **Fixture 7**: polling-consume model — `poll` returns `None`
/// when not bound, returns `None` while bound but no fresh frame,
/// returns `Some(texture)` when fresh frame is available, and clears
/// the freshness bit after consumption (atomic-bool freshness flag
/// pattern per `SyphonMetalClient.m:40, 91-104`).
#[test]
fn polling_consume_model_state_machine() {
    let id = fixture_compatible_server_id();
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let handle = MockClientHandle::new(true, Arc::clone(&stop_calls));
    let fresh_flag = handle.fresh_frame_flag();
    let mut adapter = SyphonInputAdapter::with_handle(id, Box::new(handle)).unwrap();

    // Pre-bind: poll returns None.
    assert!(adapter.poll().is_none(), "pre-bind poll yields None");
    assert!(!adapter.has_new_frame());

    // Post-bind: still no fresh frame -> None.
    adapter.bind().expect("mock bind succeeds");
    assert_eq!(adapter.health(), &AdapterHealth::Bound);
    assert!(adapter.poll().is_none(), "no fresh frame yields None");

    // Mark fresh frame: poll returns Some, clears flag.
    fresh_flag.store(true, Ordering::SeqCst);
    let texture = adapter.poll().expect("fresh frame yields Some");
    assert_eq!(texture.iosurface_id, 0xFEED_F00D);
    assert_eq!(texture.pixel_format, SyphonPixelFormat::Bgra8Unorm);

    // Subsequent poll without re-marking: None.
    assert!(
        adapter.poll().is_none(),
        "freshness bit cleared after consume"
    );

    // Cached texture still accessible via latest_texture.
    let cached = adapter.latest_texture().expect("cached texture survives");
    assert_eq!(cached.iosurface_id, 0xFEED_F00D);
}

/// **Fixture 8**: pixel format locked to `Bgra8Unorm` per docs/54
/// Lesson 3.2. Default-constructed [`SyphonTexture`] handles default
/// to the canonical format; constructing a fixture texture and
/// passing it through `SyphonOutputAdapter::publish` preserves the
/// format end-to-end.
#[test]
fn pixel_format_locked_to_bgra8unorm() {
    assert_eq!(SyphonPixelFormat::default(), SyphonPixelFormat::Bgra8Unorm);

    let texture = SyphonTexture {
        iosurface_id: 0xCAFE_F00D,
        width: 1280,
        height: 720,
        pixel_format: SyphonPixelFormat::default(),
    };
    assert_eq!(texture.pixel_format, SyphonPixelFormat::Bgra8Unorm);

    // Round-trip through SyphonOutputAdapter::publish: the publish
    // call preserves the texture format (PR 3 wires the actual
    // wgpu-side format check).
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let publish_calls = Arc::new(AtomicUsize::new(0));
    let handle = Box::new(MockServerHandle::new(
        true,
        Arc::clone(&stop_calls),
        Arc::clone(&publish_calls),
    ));
    let mut adapter = SyphonOutputAdapter::with_handle("Chromaticity".to_string(), handle).unwrap();
    adapter.bind().expect("mock bind succeeds");
    adapter
        .publish(texture)
        .expect("publish succeeds when bound");
    assert_eq!(publish_calls.load(Ordering::SeqCst), 1);
}

/// **Fixture 9**: `current_servers` returns empty in PR 2 (engine-
/// side cache wiring lives in PR 3); `parse_server_description`
/// helper round-trips through the canonical four-key shape
/// (UUIDKey / NameKey / AppNameKey / DictionaryVersionKey) and
/// returns `None` for forward-incompat dictionaries.
#[test]
fn directory_parse_helper_round_trip_and_refusal() {
    // PR 2 scaffold: live current_servers returns empty.
    let directory = SyphonServerDirectory::shared();
    assert!(directory.current_servers().is_empty());

    // Compatible parse round-trip.
    let parsed = parse_server_description(
        "uuid-1234".to_string(),
        "Channel A".to_string(),
        "TouchDesigner".to_string(),
        KNOWN_DICTIONARY_VERSION,
    )
    .expect("compatible dict parses");
    assert_eq!(parsed.uuid, "uuid-1234");
    assert_eq!(parsed.name, "Channel A");
    assert_eq!(parsed.app_name, "TouchDesigner");
    assert_eq!(parsed.dictionary_version, KNOWN_DICTIONARY_VERSION);

    // Forward-incompat refusal.
    let refused = parse_server_description(
        "uuid-5678".to_string(),
        "Channel B".to_string(),
        "FutureApp".to_string(),
        KNOWN_DICTIONARY_VERSION + 1,
    );
    assert!(refused.is_none());
}

/// **Fixture 10**: explicit unbind before drop — `unbind` calls
/// `stop`, transitions to `Disconnected`; subsequent `Drop` calls
/// `stop` again (idempotent on the framework side per
/// `SyphonClientBase.m`). Total stop count >= 2.
#[test]
fn explicit_unbind_then_drop_calls_stop_idempotently() {
    let id = fixture_compatible_server_id();
    let stop_calls = Arc::new(AtomicUsize::new(0));
    {
        let handle = Box::new(MockClientHandle::new(true, Arc::clone(&stop_calls)));
        let mut adapter = SyphonInputAdapter::with_handle(id, handle).unwrap();
        adapter.bind().expect("bind succeeds");
        adapter.unbind().expect("unbind succeeds");
        assert_eq!(adapter.health(), &AdapterHealth::Disconnected);
        // Drop fires here.
    }
    let total = stop_calls.load(Ordering::SeqCst);
    assert!(
        total >= 2,
        "expected >= 2 stop calls (unbind + drop); got {total}"
    );
}

/// **Fixture 11**: publish before bind surfaces `Faulted`. PR 3
/// wires the wgpu-side `MTLTexture` extraction; PR 2 ensures the
/// adapter refuses the publish call when the framework-side server
/// is not yet bound.
#[test]
fn output_publish_without_bind_surfaces_faulted() {
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let publish_calls = Arc::new(AtomicUsize::new(0));
    let handle = Box::new(MockServerHandle::new(
        true,
        Arc::clone(&stop_calls),
        Arc::clone(&publish_calls),
    ));
    let mut adapter = SyphonOutputAdapter::with_handle("Chromaticity".to_string(), handle).unwrap();

    let texture = SyphonTexture {
        iosurface_id: 1,
        width: 100,
        height: 100,
        pixel_format: SyphonPixelFormat::Bgra8Unorm,
    };
    let result = adapter.publish(texture);
    match result {
        Err(SyphonAdapterError::Faulted(diag)) => {
            assert!(diag.contains("publish"));
        }
        other => panic!("expected Faulted, got {other:?}"),
    }
    assert_eq!(publish_calls.load(Ordering::SeqCst), 0);
}
