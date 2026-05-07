// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Syphon adapter trait surface + scaffold impls per ADR-021 §4.
//!
//! Phase 3 PR 1 ships the type surface only:
//!
//! - [`AdapterHealth`] enum mirroring the camera-adapter shape per
//!   the parallel ADR-020 precedent.
//! - [`SyphonAdapterError`] enum with the 8-variant taxonomy per
//!   `docs/54-wave-1c-beta-research-lessons.md` Phase-3 lesson 10.
//! - [`SyphonInputAdapter`] / [`SyphonOutputAdapter`] structs with
//!   `bind` / `unbind` / `health` methods returning scaffold
//!   `Negotiating` health and `Ok(())`. The actual `objc2` binding
//!   declarations and lifecycle wiring land in Phase 3 PR 2.
//!
//! Polling-consume model per ADR-021 §4: the input adapter exposes
//! `has_new_frame` + `latest_texture` rather than a block-callback
//! handler. PR 2 wires the actual framework binding via
//! `SyphonMetalClient::initWithServerDescription:device:options:newFrameHandler:`
//! passing `nil` for the new-frame handler argument, opting into
//! polling explicitly per the canonical `SyphonMetalClient.m:40,
//! 91-104` atomic-bool freshness flag pattern.
//!
//! Drop posture per ADR-021 §8: the [`SyphonInputAdapter`] /
//! [`SyphonOutputAdapter`] wrappers MUST implement `Drop` calling
//! `[client stop]` / `[server stop]` defensively. The canonical
//! framework registers a process-destructor specifically because
//! users forget to call `stop`; mirroring that defense at the
//! Rust-side `Drop` impl avoids leaking framework-process state.
//! PR 1 ships an empty `Drop` impl so the type signature exists;
//! PR 2 wires the actual `[stop]` call.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

use std::borrow::Cow;

use thiserror::Error;

use crate::frames::{SyphonServerId, SyphonTexture};

/// Operational health of a Syphon adapter, mirroring the camera-
/// adapter shape per the parallel ADR-020 Phase-3 precedent.
///
/// State semantics: `Bound` — adapter has successfully bound to a
/// server (input) or registered itself on the bus (output).
/// `Disconnected` — adapter has no active framework-side connection
/// or registration. `Negotiating` — bind / register call has been
/// issued but the framework has not yet confirmed completion (PR 2's
/// bind state machine sits in this state during the
/// `+sharedDirectory` lookup + `SyphonMetalClient::init` call).
/// `Faulted` — adapter encountered an unrecoverable error; the
/// wrapped `Cow<'static, str>` carries an operator-facing diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterHealth {
    /// Adapter bound (input) or registered (output) and operational.
    Bound,
    /// Adapter has no active framework-side state.
    Disconnected,
    /// Adapter mid-bind / mid-register; PR 1 scaffold returns this
    /// state from `new()` until PR 2 wires the real bind path.
    Negotiating,
    /// Adapter encountered an unrecoverable error. The wrapped
    /// diagnostic identifies the failure for operator-facing surfaces
    /// (e.g. the `Sources.svelte` output-routing tile).
    Faulted(Cow<'static, str>),
}

/// Errors surfaced by [`SyphonInputAdapter`] / [`SyphonOutputAdapter`]
/// per `docs/54-wave-1c-beta-research-lessons.md` Phase-3 lesson 10.
///
/// The variant taxonomy below covers the canonical failure modes the
/// Syphon framework can surface across input + output paths. PR 1
/// ships the enum shape; PR 2 wires the variants to the actual
/// failure sites in the bind / unbind / poll / publish paths.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SyphonAdapterError {
    /// `IOSurfaceCreate` returned NULL when the output adapter
    /// attempted to allocate a backing surface for a frame to
    /// publish. The canonical Syphon-Metal subtree allocates
    /// surfaces via `IOSurfaceCreate` (NOT `CVPixelBufferPool`) per
    /// docs/54 Phase-3 lesson 1; allocation failure surfaces here.
    #[error("IOSurface allocation failed (Syphon output)")]
    IOSurfaceAllocationFailed,

    /// Pixel format advertised by the framework / requested by the
    /// caller is not supported. The canonical Syphon path locks to
    /// BGRA8Unorm (FourCC `BGRA`) per docs/54 Phase-3 lesson 4;
    /// any other format surfaces here.
    #[error("Syphon pixel format unsupported: 0x{0:08x}")]
    PixelFormatUnsupported(u32),

    /// Surface dimensions reported by the framework do not match
    /// the texture descriptor the engine-side bridge expects. The
    /// canonical mismatch source is a publisher whose surface
    /// resized between the discovery snapshot and the first
    /// `latest_texture` poll.
    #[error(
        "Syphon surface / texture dimension mismatch: surface={surface:?}, texture={texture:?}"
    )]
    DimensionMismatch {
        /// Dimensions reported by the framework's surface.
        surface: (u32, u32),
        /// Dimensions the engine-side texture descriptor expects.
        texture: (u32, u32),
    },

    /// `CVMetalTextureCacheCreate` returned non-zero status when the
    /// engine-side bridge attempted to construct the texture cache.
    /// The canonical Syphon-Metal subtree uses a Metal texture cache
    /// per `SyphonMetalServer.m`; cache creation failure surfaces
    /// here. Phase 3 PR 1 declares the variant; PR 2 / PR 3 wire it
    /// at the cache-creation site.
    #[error("CVMetalTextureCache creation failed (Syphon Metal interop)")]
    MetalTextureCacheCreateFailed,

    /// `SyphonMetalServer::initWithName:device:options:` returned
    /// `nil` (registration failed). Common cause: a peer process
    /// already registered a server under the same name and the
    /// framework refused the duplicate. Operator-facing surface
    /// resolves the conflict by re-binding under a different name.
    #[error("Syphon server registration failed: {0}")]
    SyphonServerRegistrationFailed(Cow<'static, str>),

    /// The bound server vanished from the bus mid-poll. The canonical
    /// failure mode: the publishing app exited / crashed between
    /// `bind` and `latest_texture`. The input adapter transitions to
    /// `AdapterHealth::Disconnected` and surfaces this variant.
    #[error("Syphon client: bound server vanished from the bus")]
    SyphonClientServerVanished,

    /// Operation requested by the caller is not supported by the
    /// current adapter posture. PR 1 ships the variant for forward-
    /// compat surfaces (e.g. multi-server output, per-pass tap
    /// publishing) that PR 2 / PR 3 explicitly reject per the
    /// ADR-021 §Out of Scope post-MVP deferrals.
    #[error("Syphon operation unsupported: {0}")]
    Unsupported(&'static str),

    /// Adapter is in `AdapterHealth::Faulted(_)` state and cannot
    /// service the request. Caller must `unbind` + re-`bind` to
    /// recover.
    #[error("Syphon adapter faulted: {0}")]
    Faulted(Cow<'static, str>),
}

/// Syphon input adapter — consumes frames published by other macOS
/// apps on the Syphon bus per ADR-021 §4.
///
/// **Polling-consume model**: the input adapter exposes
/// [`Self::has_new_frame`] + [`Self::latest_texture`] rather than a
/// block-callback handler. The canonical Syphon API supports both
/// models; chromaticity passes `nil` for the
/// `SyphonMetalClient::initWithServerDescription:device:options:newFrameHandler:`
/// new-frame handler argument and drives consumption from the
/// scheduler-tick. Rationale per ADR-021 §4: simpler binding (no
/// `objc2::rc::RcBlock` round-trip; no `Send`-bound closures), and
/// avoids cross-thread marshalling complexity (Syphon's new-frame
/// handler runs on its own dispatch queue, which would force
/// marshalling into the chromaticity render thread).
///
/// **PR 1 scaffold posture**: [`Self::new`] returns an adapter at
/// [`AdapterHealth::Negotiating`]; [`Self::bind`] / [`Self::unbind`]
/// / [`Self::has_new_frame`] / [`Self::latest_texture`] are
/// no-op-shaped scaffolds. PR 2 wires the real
/// `SyphonMetalClient` lifecycle.
///
/// **Drop posture**: per ADR-021 §8 the wrapper MUST implement
/// `Drop` calling `[client stop]` defensively. PR 1 ships the
/// `Drop` impl signature with an empty body; PR 2 wires the
/// actual call.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug)]
pub struct SyphonInputAdapter {
    // Phase 3 PR 1: scaffold fields. PR 2 adds:
    //   - `Retained<SyphonMetalClient>` framework handle.
    //   - `Retained<NSObject>` for the framework's
    //     `SyphonServerDescription` dictionary used at bind time.
    //   - Cached `SyphonTexture` from the most recent
    //     `latest_texture` resolution.
    server_id: SyphonServerId,
    health: AdapterHealth,
    last_texture: Option<SyphonTexture>,
}

impl SyphonInputAdapter {
    /// Construct a new input adapter targeting the supplied server.
    ///
    /// PR 1 scaffold posture: returns an adapter at
    /// [`AdapterHealth::Negotiating`] without invoking the framework.
    /// PR 2 wires the real `SyphonMetalClient` instantiation at
    /// bind time.
    ///
    /// # Errors
    ///
    /// PR 1: returns `Ok` unconditionally. PR 2: returns
    /// [`SyphonAdapterError::Faulted`] if the framework's
    /// `+sharedDirectory` lookup fails to resolve the supplied
    /// `server_id` to a current server description.
    pub fn new(server_id: SyphonServerId) -> Result<Self, SyphonAdapterError> {
        Ok(Self {
            server_id,
            health: AdapterHealth::Negotiating,
            last_texture: None,
        })
    }

    /// Current adapter health.
    #[must_use]
    pub fn health(&self) -> &AdapterHealth {
        &self.health
    }

    /// Server identity this adapter targets.
    #[must_use]
    pub fn server_id(&self) -> &SyphonServerId {
        &self.server_id
    }

    /// Bind the framework-side `SyphonMetalClient`.
    ///
    /// PR 1 scaffold posture: no-op returning `Ok(())`. PR 2 wires
    /// the actual `SyphonMetalClient::init` call + transitions
    /// health to [`AdapterHealth::Bound`] on success.
    ///
    /// # Errors
    ///
    /// PR 1: returns `Ok` unconditionally. PR 2: returns
    /// [`SyphonAdapterError::Faulted`] /
    /// [`SyphonAdapterError::SyphonClientServerVanished`] /
    /// [`SyphonAdapterError::MetalTextureCacheCreateFailed`] per
    /// the framework's failure mode at the call site.
    pub fn bind(&mut self) -> Result<(), SyphonAdapterError> {
        Ok(())
    }

    /// Release the framework-side `SyphonMetalClient`.
    ///
    /// PR 1 scaffold posture: no-op returning `Ok(())`. PR 2 wires
    /// the actual `[client stop]` call. Idempotent: calling
    /// `unbind` on an already-unbound adapter is not an error.
    ///
    /// # Errors
    ///
    /// PR 1: returns `Ok` unconditionally. PR 2: returns
    /// [`SyphonAdapterError::Faulted`] only if the framework
    /// surfaces an unexpected condition during teardown.
    pub fn unbind(&mut self) -> Result<(), SyphonAdapterError> {
        self.health = AdapterHealth::Disconnected;
        self.last_texture = None;
        Ok(())
    }

    /// Non-blocking check for whether the framework has published a
    /// new frame since the last [`Self::latest_texture`] call.
    ///
    /// Polling-consume model per ADR-021 §4. PR 2 wires this to the
    /// canonical `SyphonMetalClient::hasNewFrame` atomic-bool
    /// freshness flag (`SyphonMetalClient.m:40, 91-104`); PR 1
    /// returns `false` unconditionally.
    #[must_use]
    pub fn has_new_frame(&self) -> bool {
        false
    }

    /// Most-recent texture published by the framework, if any.
    ///
    /// Drop-frames-on-saturation per ADR-021 §4: the adapter
    /// exposes ONLY the most recent frame, not a queue. Callers
    /// that miss a frame between two consecutive scheduler-ticks
    /// observe the most-recent frame at the next tick rather than
    /// a backlog.
    ///
    /// PR 1 scaffold posture: returns `None`. PR 2 wires the actual
    /// `SyphonMetalClient::newFrameImage` resolution path that
    /// translates to `SyphonTexture` via the cached
    /// `IOSurfaceID` field.
    #[must_use]
    pub fn latest_texture(&self) -> Option<&SyphonTexture> {
        self.last_texture.as_ref()
    }
}

impl Drop for SyphonInputAdapter {
    fn drop(&mut self) {
        // PR 1 scaffold: empty Drop body holds the type signature
        // per ADR-021 §8 ("chromaticity's SyphonClient wrapper MUST
        // implement Drop calling [client stop]"). PR 2 wires the
        // actual `[client stop]` call to release the framework-side
        // connection deterministically rather than relying on
        // Objective-C runtime cleanup.
    }
}

/// Syphon output adapter — publishes frames produced by the
/// chromaticity render pipeline per ADR-021 §4.
///
/// **PR 1 scaffold posture**: [`Self::new`] returns an adapter at
/// [`AdapterHealth::Negotiating`]; [`Self::bind`] / [`Self::unbind`]
/// / [`Self::publish`] are no-op-shaped scaffolds. PR 2 wires the
/// real `SyphonMetalServer` lifecycle.
///
/// **Drop posture**: per ADR-021 §8 the wrapper MUST implement
/// `Drop` calling `[server stop]` defensively. The canonical
/// framework registers a process-destructor specifically because
/// users forget to call `stop`; leaked servers spam the system
/// with stale entries (visible in `SyphonServerDirectory`) even
/// after the chromaticity process exits, polluting the bus for
/// other VJ applications running on the same machine. PR 1 ships
/// the `Drop` impl signature with an empty body; PR 2 wires the
/// actual call.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug)]
pub struct SyphonOutputAdapter {
    // Phase 3 PR 1: scaffold fields. PR 2 adds:
    //   - `Retained<SyphonMetalServer>` framework handle.
    //   - Cached `MTLTexture` retained across publishes for the
    //     framework's `publishFrameTexture:` zero-copy path.
    name: String,
    health: AdapterHealth,
}

impl SyphonOutputAdapter {
    /// Construct a new output adapter advertising the supplied name.
    ///
    /// PR 1 scaffold posture: returns an adapter at
    /// [`AdapterHealth::Negotiating`] without invoking the framework.
    /// PR 2 wires the real `SyphonMetalServer::init` call at bind
    /// time.
    ///
    /// # Errors
    ///
    /// PR 1: returns `Ok` unconditionally. PR 2: returns
    /// [`SyphonAdapterError::SyphonServerRegistrationFailed`] if a
    /// peer process already registered a server under the same name
    /// and the framework refused the duplicate registration.
    pub fn new(name: String) -> Result<Self, SyphonAdapterError> {
        Ok(Self {
            name,
            health: AdapterHealth::Negotiating,
        })
    }

    /// Current adapter health.
    #[must_use]
    pub fn health(&self) -> &AdapterHealth {
        &self.health
    }

    /// Operator-facing server name advertised on the bus.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Bind the framework-side `SyphonMetalServer`.
    ///
    /// PR 1 scaffold posture: no-op returning `Ok(())`. PR 2 wires
    /// the actual `SyphonMetalServer::initWithName:device:options:`
    /// call + transitions health to [`AdapterHealth::Bound`] on
    /// success.
    ///
    /// # Errors
    ///
    /// PR 1: returns `Ok` unconditionally. PR 2: returns
    /// [`SyphonAdapterError::SyphonServerRegistrationFailed`] /
    /// [`SyphonAdapterError::IOSurfaceAllocationFailed`] /
    /// [`SyphonAdapterError::MetalTextureCacheCreateFailed`] per
    /// the framework's failure mode at the call site.
    pub fn bind(&mut self) -> Result<(), SyphonAdapterError> {
        Ok(())
    }

    /// Release the framework-side `SyphonMetalServer`.
    ///
    /// PR 1 scaffold posture: no-op returning `Ok(())`. PR 2 wires
    /// the actual `[server stop]` call. Idempotent: calling
    /// `unbind` on an already-unbound adapter is not an error.
    ///
    /// # Errors
    ///
    /// PR 1: returns `Ok` unconditionally. PR 2: returns
    /// [`SyphonAdapterError::Faulted`] only if the framework
    /// surfaces an unexpected condition during teardown.
    pub fn unbind(&mut self) -> Result<(), SyphonAdapterError> {
        self.health = AdapterHealth::Disconnected;
        Ok(())
    }
}

impl Drop for SyphonOutputAdapter {
    fn drop(&mut self) {
        // PR 1 scaffold: empty Drop body holds the type signature
        // per ADR-021 §8 ("chromaticity's SyphonServer Rust wrapper
        // MUST implement Drop calling [server stop] defensively").
        // PR 2 wires the actual `[server stop]` call to deregister
        // the server before the framework's process-destructor runs;
        // omitting the explicit stop leaks stale entries into the
        // bus-wide `SyphonServerDirectory` until the OS reclaims
        // the process.
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod adapter_pins {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — public adapter
    //! surface shape pins.
    //!
    //! T1 per docs/43 §15.5: each test pins a public-surface
    //! invariant against ADR-021 §4. Any drift in the
    //! `AdapterHealth` initial state, the `SyphonAdapterError`
    //! variant taxonomy, or the `Drop` impl signature fails the
    //! corresponding test at the exact ADR-section boundary.

    use super::*;
    use crate::frames::SyphonPixelFormat;

    fn fixture_server_id() -> SyphonServerId {
        SyphonServerId {
            uuid: "00000000-0000-0000-0000-000000000001".to_string(),
            name: "Fixture".to_string(),
            app_name: "FixtureApp".to_string(),
            dictionary_version: 0,
        }
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonInputAdapter::new`
    /// returns an adapter at `AdapterHealth::Negotiating` per the
    /// PR 1 scaffold posture documented at the type definition.
    /// PR 2 transitions to `AdapterHealth::Bound` after a successful
    /// `bind`.
    #[test]
    fn syphon_input_adapter_new_returns_negotiating() {
        let adapter =
            SyphonInputAdapter::new(fixture_server_id()).expect("scaffold new returns Ok");
        assert_eq!(adapter.health(), &AdapterHealth::Negotiating);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonOutputAdapter::new`
    /// returns an adapter at `AdapterHealth::Negotiating` per the
    /// PR 1 scaffold posture.
    #[test]
    fn syphon_output_adapter_new_returns_negotiating() {
        let adapter = SyphonOutputAdapter::new("Chromaticity Test".to_string())
            .expect("scaffold new returns Ok");
        assert_eq!(adapter.health(), &AdapterHealth::Negotiating);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonAdapterError`
    /// variants carry canonical diagnostics per docs/54 Phase-3
    /// lesson 10. The 8-variant taxonomy is the surface PR 2 wires
    /// against; drift would silently break the operator-facing
    /// error UI.
    #[test]
    fn adapter_error_variants_carry_canonical_diagnostic() {
        let errors = [
            SyphonAdapterError::IOSurfaceAllocationFailed,
            SyphonAdapterError::PixelFormatUnsupported(0xDEADBEEF),
            SyphonAdapterError::DimensionMismatch {
                surface: (1920, 1080),
                texture: (1280, 720),
            },
            SyphonAdapterError::MetalTextureCacheCreateFailed,
            SyphonAdapterError::SyphonServerRegistrationFailed(Cow::Borrowed("name conflict")),
            SyphonAdapterError::SyphonClientServerVanished,
            SyphonAdapterError::Unsupported("multi-server output"),
            SyphonAdapterError::Faulted(Cow::Borrowed("test fault")),
        ];
        for err in &errors {
            // Display impl from `thiserror` MUST surface a non-empty
            // diagnostic string. An empty string here would mean a
            // future variant was added without an `#[error(...)]`
            // attribute.
            let rendered = format!("{err}");
            assert!(
                !rendered.is_empty(),
                "error variant must have a Display string: {err:?}"
            );
        }
        assert_eq!(errors.len(), 8, "PR 1 fixes the variant count at 8");
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `unbind` transitions the
    /// input adapter to `AdapterHealth::Disconnected` and clears any
    /// cached frame handle. PR 2 builds on this state-machine pin
    /// when wiring the real `[client stop]` call.
    #[test]
    fn input_adapter_unbind_transitions_to_disconnected() {
        let mut adapter =
            SyphonInputAdapter::new(fixture_server_id()).expect("scaffold new returns Ok");
        adapter.unbind().expect("scaffold unbind returns Ok");
        assert_eq!(adapter.health(), &AdapterHealth::Disconnected);
        assert!(adapter.latest_texture().is_none());
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `unbind` transitions the
    /// output adapter to `AdapterHealth::Disconnected`. PR 2 builds
    /// on this state-machine pin when wiring the real `[server stop]`
    /// call.
    #[test]
    fn output_adapter_unbind_transitions_to_disconnected() {
        let mut adapter = SyphonOutputAdapter::new("Chromaticity Test".to_string())
            .expect("scaffold new returns Ok");
        adapter.unbind().expect("scaffold unbind returns Ok");
        assert_eq!(adapter.health(), &AdapterHealth::Disconnected);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `Drop` impl signatures
    /// compile and execute without panic. PR 2 wires the actual
    /// `[stop]` call inside the impl bodies; PR 1 ensures the
    /// signature is in place so PR 2's diff is the binding
    /// addition rather than an impl-block introduction.
    #[test]
    fn drop_impl_signature_compiles() {
        // Construct + drop both adapter types; PR 1 Drop bodies are
        // empty and must not panic. The test is a compile-time pin
        // primarily; the runtime assertion is "drop did not panic".
        {
            let _input =
                SyphonInputAdapter::new(fixture_server_id()).expect("scaffold new returns Ok");
        }
        {
            let _output = SyphonOutputAdapter::new("Chromaticity Drop Test".to_string())
                .expect("scaffold new returns Ok");
        }
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonInputAdapter::new`
    /// stores the supplied `server_id` for retrieval. PR 2 wires
    /// the actual `+sharedDirectory` lookup against this stored ID.
    #[test]
    fn input_adapter_stores_server_id() {
        let id = fixture_server_id();
        let adapter = SyphonInputAdapter::new(id.clone()).expect("scaffold new returns Ok");
        assert_eq!(adapter.server_id(), &id);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonOutputAdapter::new`
    /// stores the supplied name for retrieval. PR 2 wires the
    /// actual `SyphonMetalServer` registration against this stored
    /// name.
    #[test]
    fn output_adapter_stores_name() {
        let adapter = SyphonOutputAdapter::new("Chromaticity Tour 2026".to_string())
            .expect("scaffold new returns Ok");
        assert_eq!(adapter.name(), "Chromaticity Tour 2026");
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `has_new_frame` returns
    /// `false` and `latest_texture` returns `None` in the PR 1
    /// scaffold, with no panic. PR 2 wires the real
    /// `SyphonMetalClient::hasNewFrame` atomic-bool freshness flag.
    #[test]
    fn input_adapter_polling_scaffold_returns_no_frame() {
        let adapter =
            SyphonInputAdapter::new(fixture_server_id()).expect("scaffold new returns Ok");
        assert!(!adapter.has_new_frame());
        assert!(adapter.latest_texture().is_none());
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `bind` returns `Ok(())` in
    /// the PR 1 scaffold for both adapters. PR 2 transitions health
    /// to `AdapterHealth::Bound` on success and surfaces the
    /// canonical error variants on failure.
    #[test]
    fn bind_scaffold_returns_ok() {
        let mut input =
            SyphonInputAdapter::new(fixture_server_id()).expect("scaffold new returns Ok");
        input.bind().expect("scaffold bind returns Ok");

        let mut output = SyphonOutputAdapter::new("Chromaticity Bind Test".to_string())
            .expect("scaffold new returns Ok");
        output.bind().expect("scaffold bind returns Ok");
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `AdapterHealth::Faulted`
    /// carries an operator-facing diagnostic via `Cow<'static, str>`.
    /// PR 2 surfaces this state from the bind / poll paths when
    /// the framework surfaces an unrecoverable error.
    #[test]
    fn adapter_health_faulted_carries_diagnostic() {
        let faulted = AdapterHealth::Faulted(Cow::Borrowed("framework returned nil"));
        match faulted {
            AdapterHealth::Faulted(diag) => assert!(!diag.is_empty()),
            other => panic!("expected Faulted, got {other:?}"),
        }
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonTexture` carries
    /// `SyphonPixelFormat::Bgra8Unorm` as its pixel format default
    /// (cf. frame_pins::pixel_format_default_is_bgra8_unorm) and
    /// the default-shape constructor compiles with the canonical
    /// format. Pin guards against silent drift if `Default` ever
    /// changes the variant.
    #[test]
    fn syphon_texture_default_pixel_format_round_trip() {
        let texture = SyphonTexture {
            iosurface_id: 42,
            width: 1920,
            height: 1080,
            pixel_format: SyphonPixelFormat::default(),
        };
        assert_eq!(texture.pixel_format, SyphonPixelFormat::Bgra8Unorm);
    }
}
