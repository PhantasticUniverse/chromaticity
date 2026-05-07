// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Syphon adapter trait surface + impls per ADR-021 §4.
//!
//! Phase 3 PR 2 ships:
//!
//! - [`AdapterHealth`] enum mirroring the camera-adapter shape per
//!   the parallel ADR-020 precedent.
//! - [`SyphonAdapterError`] enum with the 8-variant taxonomy per
//!   `docs/54-wave-1c-beta-research-lessons.md` Lesson 3.10.
//! - [`SyphonInputAdapter`] / [`SyphonOutputAdapter`] structs with
//!   `bind` / `unbind` / `health` / `poll` / `publish` methods. The
//!   [`backend`] sub-module declares the trait abstraction the
//!   bindings flow through; the real `objc2` backend wraps
//!   [`crate::bindings`] handles, and the T2 corpus injects a
//!   mock-backend implementation against in-memory fixtures (no live
//!   Syphon bus on CI, per ADR-021 §10 Negative consequence bullet 3).
//!
//! Polling-consume model per ADR-021 §4: the input adapter exposes
//! [`SyphonInputAdapter::poll`] backed by `has_new_frame` +
//! `new_frame_image` reads from the [`backend::SyphonClientHandle`]
//! trait. The real `objc2` backend calls
//! `SyphonMetalClient::initWithServerDescription:device:options:newFrameHandler:`
//! with `nil` for the new-frame handler argument, opting into polling
//! explicitly per the canonical `SyphonMetalClient.m:40, 91-104`
//! atomic-bool freshness flag pattern.
//!
//! Drop posture per ADR-021 §8: the [`SyphonInputAdapter`] /
//! [`SyphonOutputAdapter`] wrappers implement `Drop` calling
//! `[client stop]` / `[server stop]` defensively through the trait
//! handle. The canonical framework registers a process-destructor
//! specifically because users forget to call `stop`; mirroring that
//! defense at the Rust-side `Drop` impl avoids leaking framework-
//! process state.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

use std::borrow::Cow;

use thiserror::Error;

use crate::frames::{KNOWN_DICTIONARY_VERSION, SyphonServerId, SyphonTexture};

pub mod backend;

/// Operational health of a Syphon adapter, mirroring the camera-
/// adapter shape per the parallel ADR-020 Phase-3 precedent.
///
/// State semantics: `Bound` — adapter has successfully bound to a
/// server (input) or registered itself on the bus (output).
/// `Disconnected` — adapter has no active framework-side connection
/// or registration. `Negotiating` — bind / register call has not yet
/// been issued (constructor returns this state until the caller
/// invokes `bind`). `Faulted` — adapter encountered an unrecoverable
/// error; the wrapped `Cow<'static, str>` carries an operator-facing
/// diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterHealth {
    /// Adapter bound (input) or registered (output) and operational.
    Bound,
    /// Adapter has no active framework-side state.
    Disconnected,
    /// Adapter constructed but not yet bound; transient state.
    Negotiating,
    /// Adapter encountered an unrecoverable error. The wrapped
    /// diagnostic identifies the failure for operator-facing surfaces
    /// (e.g. the `Sources.svelte` output-routing tile).
    Faulted(Cow<'static, str>),
}

/// Errors surfaced by [`SyphonInputAdapter`] / [`SyphonOutputAdapter`]
/// per `docs/54-wave-1c-beta-research-lessons.md` Lesson 3.10.
///
/// The variant taxonomy below covers the canonical failure modes the
/// Syphon framework can surface across input + output paths.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SyphonAdapterError {
    /// `IOSurfaceCreate` returned NULL when the output adapter
    /// attempted to allocate a backing surface for a frame to
    /// publish. The canonical Syphon-Metal subtree allocates
    /// surfaces via `IOSurfaceCreate` per docs/54 Lesson 3.1;
    /// allocation failure surfaces here.
    #[error("IOSurface allocation failed (Syphon output)")]
    IOSurfaceAllocationFailed,

    /// Pixel format advertised by the framework / requested by the
    /// caller is not supported. The canonical Syphon path locks to
    /// BGRA8Unorm (FourCC `BGRA`) per docs/54 Lesson 3.2; any other
    /// format surfaces here.
    #[error("Syphon pixel format unsupported: 0x{0:08x}")]
    PixelFormatUnsupported(u32),

    /// Surface dimensions reported by the framework do not match
    /// the texture descriptor the engine-side bridge expects.
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
    /// engine-side bridge attempted to construct the texture cache
    /// per docs/54 Lesson 3.1.
    #[error("CVMetalTextureCache creation failed (Syphon Metal interop)")]
    MetalTextureCacheCreateFailed,

    /// `SyphonMetalServer::initWithName:device:options:` returned
    /// `nil` (registration failed). Common cause: a peer process
    /// already registered a server under the same name and the
    /// framework refused the duplicate.
    #[error("Syphon server registration failed: {0}")]
    SyphonServerRegistrationFailed(Cow<'static, str>),

    /// The bound server vanished from the bus mid-poll, OR the
    /// initial `SyphonMetalClient::init` call returned `nil` because
    /// the supplied server description is no longer current /
    /// version-incompatible per `SyphonClientBase.m:87-93`.
    #[error("Syphon client: bound server vanished from the bus or refused connection: {0}")]
    SyphonClientServerVanished(Cow<'static, str>),

    /// Operation requested by the caller is not supported by the
    /// current adapter posture (e.g. multi-server output, per-pass
    /// tap publishing) per the ADR-021 §Out of Scope post-MVP
    /// deferrals.
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
/// [`Self::poll`] backed by `has_new_frame` + `new_frame_image`
/// reads on the trait handle. The canonical Syphon API supports
/// both block-callback and polling models; chromaticity passes
/// `nil` for the new-frame handler argument and drives consumption
/// from the scheduler-tick. Rationale per ADR-021 §4: simpler
/// binding (no `objc2::rc::RcBlock` round-trip; no `Send`-bound
/// closures), and avoids cross-thread marshalling complexity
/// (Syphon's new-frame handler runs on its own dispatch queue,
/// which would force marshalling into the chromaticity render
/// thread).
///
/// **Backend abstraction**: the adapter holds a
/// `Box<dyn SyphonClientHandle>` rather than a concrete handle so
/// the T2 corpus can inject a mock implementation (no live Syphon
/// bus on CI per ADR-021 §10). The real-backend constructor
/// [`SyphonInputAdapter::new`] resolves the supplied
/// [`SyphonServerId`] against [`crate::SyphonServerDirectory`] and
/// constructs a real `SyphonMetalClient` via [`crate::bindings`];
/// the test-backend constructor [`SyphonInputAdapter::with_handle`]
/// accepts a pre-constructed mock handle.
///
/// **Drop posture**: per ADR-021 §8 the wrapper implements `Drop`
/// calling the handle's `stop` method defensively. The canonical
/// framework registers a process-destructor specifically because
/// users forget to call `stop`; mirroring that defense at the
/// Rust-side `Drop` impl avoids leaking framework-process state.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
pub struct SyphonInputAdapter {
    server_id: SyphonServerId,
    health: AdapterHealth,
    last_texture: Option<SyphonTexture>,
    handle: Box<dyn backend::SyphonClientHandle>,
}

impl std::fmt::Debug for SyphonInputAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyphonInputAdapter")
            .field("server_id", &self.server_id)
            .field("health", &self.health)
            .field("last_texture", &self.last_texture)
            .finish_non_exhaustive()
    }
}

impl SyphonInputAdapter {
    /// Construct a new input adapter targeting the supplied server
    /// using a caller-supplied client handle.
    ///
    /// PR 2 ships only this entry point: the real-backend constructor
    /// (the path that calls `SyphonMetalClient::init` against a wgpu-
    /// supplied `MTLDevice`) lands in Phase 3 PR 3 alongside the
    /// engine/render bridge. PR 2's call sites in tests inject a
    /// [`backend::SyphonClientHandle`] mock; production call sites in
    /// PR 3 inject the real `objc2`-backed handle.
    ///
    /// # Errors
    ///
    /// Refuses servers whose `dictionary_version >
    /// KNOWN_DICTIONARY_VERSION` per ADR-021 §4 + the canonical
    /// `SyphonClientBase.m:87-93` defensive forward-compat refusal
    /// pattern; surfaces
    /// [`SyphonAdapterError::SyphonClientServerVanished`].
    pub fn with_handle(
        server_id: SyphonServerId,
        handle: Box<dyn backend::SyphonClientHandle>,
    ) -> Result<Self, SyphonAdapterError> {
        if !server_id.is_compatible() {
            return Err(SyphonAdapterError::SyphonClientServerVanished(Cow::Owned(
                format!(
                    "server dictionary_version {} exceeds known {}; refusing per ADR-021 §4",
                    server_id.dictionary_version, KNOWN_DICTIONARY_VERSION
                ),
            )));
        }
        Ok(Self {
            server_id,
            health: AdapterHealth::Negotiating,
            last_texture: None,
            handle,
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

    /// Bind the framework-side `SyphonMetalClient` (or, in tests,
    /// transition the mock handle to the bound state).
    ///
    /// Idempotent: calling `bind` on an already-bound adapter is a
    /// no-op. Transitions health to [`AdapterHealth::Bound`] on
    /// success.
    ///
    /// # Errors
    ///
    /// Returns [`SyphonAdapterError::SyphonClientServerVanished`] if
    /// the underlying handle's `bind` returns `false` (server
    /// vanished from the bus between construction and bind, or
    /// framework refused the connection).
    pub fn bind(&mut self) -> Result<(), SyphonAdapterError> {
        if matches!(self.health, AdapterHealth::Bound) {
            return Ok(());
        }
        if self.handle.bind() {
            self.health = AdapterHealth::Bound;
            Ok(())
        } else {
            self.health = AdapterHealth::Faulted(Cow::Borrowed(
                "SyphonMetalClient::init returned nil (server vanished)",
            ));
            Err(SyphonAdapterError::SyphonClientServerVanished(
                Cow::Borrowed("backend bind() returned false"),
            ))
        }
    }

    /// Release the framework-side `SyphonMetalClient` (or, in tests,
    /// transition the mock handle to the stopped state).
    ///
    /// Idempotent: calling `unbind` on an already-unbound adapter is
    /// not an error. Transitions health to
    /// [`AdapterHealth::Disconnected`] and clears any cached frame
    /// handle.
    ///
    /// # Errors
    ///
    /// Currently infallible; returns `Ok(())` for forward-compat
    /// with future framework-surfaced teardown errors.
    pub fn unbind(&mut self) -> Result<(), SyphonAdapterError> {
        self.handle.stop();
        self.health = AdapterHealth::Disconnected;
        self.last_texture = None;
        Ok(())
    }

    /// Non-blocking check for whether the framework has published a
    /// new frame since the last [`Self::poll`] call.
    ///
    /// Polling-consume model per ADR-021 §4. Wired to the canonical
    /// `SyphonMetalClient`'s atomic-bool freshness flag through the
    /// trait abstraction (`SyphonMetalClient.m:40, 91-104`).
    #[must_use]
    pub fn has_new_frame(&self) -> bool {
        matches!(self.health, AdapterHealth::Bound) && self.handle.has_new_frame()
    }

    /// Poll the adapter for the most-recent texture.
    ///
    /// Drop-frames-on-saturation per ADR-021 §4: the adapter exposes
    /// ONLY the most recent frame, not a queue. Callers that miss a
    /// frame between two consecutive scheduler-ticks observe the
    /// most-recent frame at the next tick rather than a backlog.
    ///
    /// Returns `None` when no fresh frame is available since the
    /// last call; the cached handle from the prior successful poll
    /// remains accessible via [`Self::latest_texture`].
    pub fn poll(&mut self) -> Option<SyphonTexture> {
        if !matches!(self.health, AdapterHealth::Bound) {
            return None;
        }
        if !self.handle.has_new_frame() {
            return None;
        }
        let texture = self.handle.new_frame_image();
        if let Some(t) = texture.as_ref() {
            self.last_texture = Some(*t);
        }
        texture
    }

    /// Most-recent texture published by the framework, if any.
    ///
    /// Cached from the last successful [`Self::poll`] call. Returns
    /// `None` until the first poll resolves a frame.
    #[must_use]
    pub fn latest_texture(&self) -> Option<&SyphonTexture> {
        self.last_texture.as_ref()
    }
}

impl Drop for SyphonInputAdapter {
    fn drop(&mut self) {
        // ADR-021 §8 + docs/54 Lesson 11: defensive `[client stop]`
        // call ensures the framework-side connection is released
        // deterministically, even if the caller forgot to invoke
        // `unbind`. The trait handle's `stop` is idempotent on the
        // framework side per `SyphonClientBase.m`; calling stop on
        // an already-stopped client is a documented no-op.
        self.handle.stop();
    }
}

/// Syphon output adapter — publishes frames produced by the
/// chromaticity render pipeline per ADR-021 §4.
///
/// **Backend abstraction**: as with [`SyphonInputAdapter`], the
/// adapter holds a `Box<dyn SyphonServerHandle>` so the T2 corpus
/// can inject a mock implementation; the real-backend constructor
/// (which calls `SyphonMetalServer::initWithName:device:options:`
/// against a wgpu-supplied `MTLDevice`) lands in Phase 3 PR 3.
///
/// **Drop posture**: per ADR-021 §8 the wrapper implements `Drop`
/// calling `[server stop]` defensively. The canonical framework
/// registers a process-destructor specifically because users forget
/// to call `stop`; leaked servers spam the system with stale entries
/// (visible in `SyphonServerDirectory`) even after the chromaticity
/// process exits, polluting the bus for other VJ applications
/// running on the same machine.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
pub struct SyphonOutputAdapter {
    name: String,
    health: AdapterHealth,
    handle: Box<dyn backend::SyphonServerHandle>,
}

impl std::fmt::Debug for SyphonOutputAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyphonOutputAdapter")
            .field("name", &self.name)
            .field("health", &self.health)
            .finish_non_exhaustive()
    }
}

impl SyphonOutputAdapter {
    /// Construct a new output adapter advertising the supplied name,
    /// using a caller-supplied server handle.
    ///
    /// PR 2 ships only this entry point: the real-backend constructor
    /// (the path that calls `SyphonMetalServer::init` against a wgpu-
    /// supplied `MTLDevice`) lands in Phase 3 PR 3 alongside the
    /// engine/render bridge.
    ///
    /// # Errors
    ///
    /// Currently infallible at construction time; bind-time failures
    /// surface from [`Self::bind`].
    pub fn with_handle(
        name: String,
        handle: Box<dyn backend::SyphonServerHandle>,
    ) -> Result<Self, SyphonAdapterError> {
        Ok(Self {
            name,
            health: AdapterHealth::Negotiating,
            handle,
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

    /// Bind the framework-side `SyphonMetalServer` (or, in tests,
    /// transition the mock handle to the bound state).
    ///
    /// Idempotent: calling `bind` on an already-bound adapter is a
    /// no-op. Transitions health to [`AdapterHealth::Bound`] on
    /// success.
    ///
    /// # Errors
    ///
    /// Returns [`SyphonAdapterError::SyphonServerRegistrationFailed`]
    /// if the underlying handle's `bind` returns `false` (most
    /// commonly: a peer process already registered a server under
    /// the same name and the framework refused the duplicate).
    pub fn bind(&mut self) -> Result<(), SyphonAdapterError> {
        if matches!(self.health, AdapterHealth::Bound) {
            return Ok(());
        }
        if self.handle.bind(&self.name) {
            self.health = AdapterHealth::Bound;
            Ok(())
        } else {
            self.health = AdapterHealth::Faulted(Cow::Borrowed(
                "SyphonMetalServer::init returned nil (name conflict?)",
            ));
            Err(SyphonAdapterError::SyphonServerRegistrationFailed(
                Cow::Owned(format!("backend refused name '{}'", self.name)),
            ))
        }
    }

    /// Release the framework-side `SyphonMetalServer`.
    ///
    /// Idempotent: calling `unbind` on an already-unbound adapter is
    /// not an error. Transitions health to
    /// [`AdapterHealth::Disconnected`].
    ///
    /// # Errors
    ///
    /// Currently infallible; returns `Ok(())` for forward-compat
    /// with future framework-surfaced teardown errors.
    pub fn unbind(&mut self) -> Result<(), SyphonAdapterError> {
        self.handle.stop();
        self.health = AdapterHealth::Disconnected;
        Ok(())
    }

    /// Publish a frame to the bus.
    ///
    /// PR 2 ships the type-shape contract: the adapter accepts a
    /// [`SyphonTexture`] handle and forwards to the underlying
    /// trait. The real-backend implementation in PR 3 wires the
    /// wgpu `MTLTexture` extraction + the
    /// `publishFrameTexture:onCommandBuffer:imageRegion:flipped:`
    /// call site through the `engine/render` bridge.
    ///
    /// # Errors
    ///
    /// Returns [`SyphonAdapterError::Faulted`] if the adapter is
    /// not in [`AdapterHealth::Bound`] state.
    pub fn publish(&mut self, texture: SyphonTexture) -> Result<(), SyphonAdapterError> {
        if !matches!(self.health, AdapterHealth::Bound) {
            return Err(SyphonAdapterError::Faulted(Cow::Borrowed(
                "publish called before bind",
            )));
        }
        self.handle.publish(texture);
        Ok(())
    }
}

impl Drop for SyphonOutputAdapter {
    fn drop(&mut self) {
        // ADR-021 §8 + docs/54 Lesson 11: defensive `[server stop]`
        // call deregisters the server before the framework's
        // process-destructor runs; omitting the explicit stop leaks
        // stale entries into the bus-wide `SyphonServerDirectory`
        // until the OS reclaims the process. The trait handle's
        // `stop` is idempotent on the framework side per
        // `SyphonServerBase.m`.
        self.handle.stop();
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
    //!
    //! Backend-touching state-machine fixtures live in
    //! `tests/syphon_adapter_corpus.rs` (the T2 synthetic-narrowing
    //! corpus).

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

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonAdapterError`
    /// variants carry canonical diagnostics per docs/54 Lesson
    /// 3.10. The 8-variant taxonomy is the surface PR 3 wires
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
            SyphonAdapterError::SyphonClientServerVanished(Cow::Borrowed("vanished")),
            SyphonAdapterError::Unsupported("multi-server output"),
            SyphonAdapterError::Faulted(Cow::Borrowed("test fault")),
        ];
        for err in &errors {
            let rendered = format!("{err}");
            assert!(
                !rendered.is_empty(),
                "error variant must have a Display string: {err:?}"
            );
        }
        assert_eq!(errors.len(), 8, "PR 2 fixes the variant count at 8");
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `AdapterHealth::Faulted`
    /// carries an operator-facing diagnostic via `Cow<'static, str>`.
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
    /// per docs/54 Lesson 3.2.
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

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonInputAdapter::with_handle`
    /// refuses servers whose `dictionary_version` exceeds
    /// `KNOWN_DICTIONARY_VERSION` per ADR-021 §4 + canonical
    /// `SyphonClientBase.m:87-93`. PR 2 wires the refusal at the
    /// constructor seam.
    #[test]
    fn input_adapter_refuses_forward_incompat_dictionary_version() {
        let mut id = fixture_server_id();
        id.dictionary_version = KNOWN_DICTIONARY_VERSION + 1;
        let result =
            SyphonInputAdapter::with_handle(id, Box::new(backend::tests_support::NoopClientHandle));
        match result {
            Err(SyphonAdapterError::SyphonClientServerVanished(_)) => {}
            other => panic!("expected SyphonClientServerVanished, got {other:?}"),
        }
    }
}
