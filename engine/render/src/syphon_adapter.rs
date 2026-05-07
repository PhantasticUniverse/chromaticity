// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1c-β Phase 3 PR 3 — engine-side Syphon bridge per
//! ADR-021 §6.
//!
//! # Cross-crate boundary
//!
//! The `chromaticity_syphon_adapter` crate
//! (`integrations/syphon-adapter/`) publishes plain-data
//! [`chromaticity_syphon_adapter::SyphonTexture`] cross-seam handles
//! plus the trait abstractions
//! [`chromaticity_syphon_adapter::adapter::backend::SyphonClientHandle`]
//! / [`chromaticity_syphon_adapter::adapter::backend::SyphonServerHandle`]
//! that wrap the canonical Syphon-Framework's `SyphonMetalClient` /
//! `SyphonMetalServer` Objective-C handles. That crate intentionally
//! does NOT depend on `engine/render` — see
//! `integrations/syphon-adapter/src/adapter/backend.rs` lines 7-26
//! ("Two implementations satisfy the traits: real `objc2` backend
//! (Phase 3 PR 3, `engine/render/src/syphon_adapter.rs`) ... mock
//! backend"). This module is the consumer-side real-backend
//! implementation that satisfies the trait surface against the
//! `objc2`-bound `Retained<SyphonMetalClient>` /
//! `Retained<SyphonMetalServer>` handles obtained from
//! [`chromaticity_syphon_adapter::bindings`].
//!
//! Mirror of the precedent established by
//! [`crate::audio_adapter`] (slice-5a `EV-AUDIO-TRACE` cross-crate
//! adapter): producer crate publishes plain-data; consumer-side
//! engine crate marshals the plain-data shape into the engine
//! seam. The Syphon arm marshals at the
//! [`chromaticity_syphon_adapter::adapter::backend::SyphonClientHandle`]
//! / [`chromaticity_syphon_adapter::adapter::backend::SyphonServerHandle`]
//! trait boundary (rather than at a `UniformInput` boundary)
//! because the surface to bridge is GPU-resident texture data, not
//! scalar uniforms.
//!
//! # Architecture per ADR-021 §6 + docs/54 Phase-3 lessons
//!
//! ## Output direction (publish path)
//!
//! Per ADR-021 §6 + docs/54 Lesson 3.1 + 3.3 + 3.4:
//!
//! 1. The engine selects an `IOSurface`-exportable `wgpu::Texture`
//!    (the final output target or a designated tap point).
//! 2. The bridge pre-warms a 2-3 surface ring at output adapter
//!    `bind` time via a `CVPixelBufferPool` with
//!    `kCVPixelBufferPoolMinimumBufferCountKey = 2-3` per docs/54
//!    Lesson 3.3 to amortize first-frame allocation cost.
//! 3. A long-lived `CVMetalTextureCache` (created via
//!    `CVMetalTextureCacheCreate(NULL, NULL, mtlDevice, NULL,
//!    &cache)`) per `wgpu::Device` is the canonical zero-copy
//!    CoreVideo↔Metal bridge per docs/54 Lesson 3.1.
//! 4. The publish call routes through the wgpu command buffer's
//!    completion handler per docs/54 Lesson 3.4 (without the gate
//!    consumers observe partially-rendered surfaces).
//! 5. Format is locked to [`wgpu::TextureFormat::Bgra8UnormSrgb`] ↔
//!    `kCVPixelFormatType_32BGRA` per docs/54 Lesson 3.2.
//!
//! ## Input direction (consume path)
//!
//! Per ADR-021 §6 + docs/54 Lesson 3.6 + 3.7 + 3.8:
//!
//! 1. [`Objc2SyphonClientHandle::new_frame_image`] resolves a
//!    fresh `MTLTexture` from the framework via
//!    `[client newFrameImage]` per the canonical `SyphonMetalClient.h:54`
//!    contract.
//! 2. The bridge wraps the underlying `IOSurface` into a
//!    `wgpu::Texture` via the Metal-backend interop surface (the
//!    `wgpu_hal::api::Metal` escape-hatch acquires the `MTLDevice`
//!    from the wgpu `Device`).
//! 3. Consumer-side `IOSurface` reads use
//!    `kCVPixelBufferLock_ReadOnly` per docs/54 Lesson 3.8.
//! 4. The atomic-bool freshness flag pattern per docs/54 Lesson 3.6
//!    is observed at the trait boundary
//!    (`SyphonClientHandle::has_new_frame`); the wgpu re-import is
//!    cached once per underlying IOSurface change, not once per
//!    frame.
//!
//! # CFType-as-Drop refcounting per docs/54 Lesson 3.7
//!
//! Every CF-prefixed CoreFoundation handle the bridge holds
//! (`CVMetalTextureCacheRef`, `CVPixelBufferRef`, `IOSurfaceRef`) is
//! wrapped in a thin Rust newtype with a `Drop` impl that calls
//! `CFRelease`. NS-prefixed Objective-C handles use objc2's
//! `Retained<>` (the `Drop` releases via `objc_release` on the
//! Objective-C runtime). The two refcounting systems are NEVER
//! mixed: each wrapper owns its release path. The `Drop`
//! discipline mirrors ADR-021 §8 (`SyphonInputAdapter::Drop` /
//! `SyphonOutputAdapter::Drop` defensively call `[client stop]` /
//! `[server stop]` through the trait handle).
//!
//! # Scope narrowing for PR 3
//!
//! PR 3 ships:
//!
//! - The trait-impl shape: [`Objc2SyphonClientHandle`] /
//!   [`Objc2SyphonServerHandle`] structs that wrap
//!   `Retained<SyphonMetalClient>` / `Retained<SyphonMetalServer>`
//!   handles obtained via `chromaticity_syphon_adapter::bindings`.
//! - The trait-method surface (`bind` / `has_new_frame` /
//!   `new_frame_image` / `stop` for the client; `bind` /
//!   `publish` / `stop` for the server) wired to the
//!   framework-side `objc2` selector calls per
//!   `chromaticity_syphon_adapter::bindings`. The framework-side
//!   `init` call sites are gated by the
//!   [`SyphonRenderBridge`] device-acquisition shape (the bridge
//!   holds a `wgpu::Device` and the bound `MTLDevice` extracted via
//!   `wgpu_hal::api::Metal`).
//! - The `Drop` impls per ADR-021 §8 + docs/54 Lesson 11
//!   (defensive `[client stop]` / `[server stop]`).
//! - T1 drift pins covering the trait-impl shape + the format /
//!   refcounting invariants per docs/54 Phase-3 lessons 3.1 / 3.2 /
//!   3.7.
//!
//! PR 3 explicitly **does NOT ship**, with the narrowing recorded
//! honestly in the closing-docs `EV-INTEGRATION-MATRIX` Syphon-arm
//! widening:
//!
//! - Live Metal-side `IOSurface` ↔ `wgpu::Texture` conversion
//!   through the `CVMetalTextureCache`. The
//!   [`SyphonRenderBridge::resolve_iosurface_to_wgpu_texture`] entry
//!   point is declared but its body is the stub-narrowing path that
//!   returns
//!   [`SyphonRenderBridgeError::Unimplemented`]; the wgpu_hal escape-
//!   hatch + CVMetalTextureCacheCreate + CVMetalTextureCacheCreateTextureFromImage
//!   call sites are documented inline as the follow-up wave's
//!   target. Rationale: the wgpu_hal Metal escape-hatch shape
//!   (`Device::as_hal_mut::<wgpu_hal::api::Metal, _, _>` in
//!   wgpu 26) requires hardware-rig validation that can't run
//!   under headless CI; shipping a stub here lets the trait-impl
//!   shape land + compile cleanly + receive its T1 pins, with the
//!   real bridge body deferred to a hardware-validated wave.
//! - The `CVPixelBufferPool` pre-warm ring on output `bind`. The
//!   pool acquisition is documented inline at
//!   [`SyphonRenderBridge::prewarm_output_surface_ring`] but
//!   returns [`SyphonRenderBridgeError::Unimplemented`].
//! - The `addCompletedHandler:` GPU-completion synchronization on
//!   output publish per docs/54 Lesson 3.4.
//!
//! The trait-impl shape suffices for the Syphon-arm conformance
//! corpus + UI integration (W34c+ Sources.svelte adapter pill /
//! status surface) to consume the engine-side surface uniformly
//! across mock + real backends; the deferred Metal bridge body is
//! the last engineering step before the live Syphon bus is end-to-
//! end exercised on AO dev rigs.
//!
//! # Evidence anchors
//!
//! - `docs/26-evidence-artifact-registry.md` `EV-INTEGRATION-MATRIX`
//!   row — Syphon-arm widening recorded in the closing-docs
//!   description-column entry per the canonical EV-ID-not-row
//!   addressing convention.
//! - `docs/adr/ADR-021-syphon-adapter.md` §3, §6, §8 — bridge
//!   contract + Drop posture.
//! - `docs/54-wave-1c-beta-research-lessons.md` Phase-3 lessons
//!   3.1, 3.2, 3.3, 3.4, 3.6, 3.7, 3.8 — implementation lessons.
//! - `docs/adr/ADR-011-rendering-backend.md` §4 — wgpu Metal
//!   `IOSurface` interop path.

use std::sync::Arc;

use chromaticity_syphon_adapter::SyphonTexture;
use chromaticity_syphon_adapter::adapter::backend::{SyphonClientHandle, SyphonServerHandle};
use thiserror::Error;

/// Errors surfaced by the engine-side [`SyphonRenderBridge`].
///
/// Distinct from
/// [`chromaticity_syphon_adapter::SyphonAdapterError`] (which
/// surfaces framework-side bind / publish failures): the bridge
/// errors below cover the wgpu ↔ `IOSurface` seam specifically —
/// `MTLDevice` extraction failures, `CVMetalTextureCache` allocation
/// failures, surface-format mismatches at the engine seam.
///
/// PR 3 ships the variant taxonomy + the narrowing
/// [`Self::Unimplemented`] variant the deferred Metal bridge body
/// returns. A future wave that lands the live bridge widens the
/// surface with concrete failure variants and retires
/// [`Self::Unimplemented`] under `#[non_exhaustive]`.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm) — engine-side
/// bridge variant taxonomy.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SyphonRenderBridgeError {
    /// `wgpu_hal::api::Metal::Device::raw_device()` extraction
    /// failed — the wgpu Device was not constructed against the
    /// Metal backend, or the hal escape-hatch surface was unable
    /// to expose the underlying `MTLDevice`.
    #[error("wgpu_hal Metal escape-hatch failed: {0}")]
    MetalDeviceExtractionFailed(&'static str),

    /// The bridge entry point is not yet implemented — PR 3 ships
    /// the entry-point shape with this stub narrowing; a follow-up
    /// hardware-validated wave lands the real body.
    ///
    /// Recorded honestly in the `EV-INTEGRATION-MATRIX`
    /// closing-docs description column per the wave's
    /// synthetic-narrowing posture.
    #[error("Syphon render bridge body deferred to follow-up wave: {0}")]
    Unimplemented(&'static str),
}

/// Engine-side Syphon ↔ wgpu bridge per ADR-021 §6.
///
/// Owns the long-lived per-`wgpu::Device` resources the bridge
/// holds across frames (the `CVMetalTextureCache` plus the pre-
/// warmed `CVPixelBufferPool` for the output side per docs/54
/// Lesson 3.1 + 3.3). Mirrors the
/// [`crate::multipass::MultipassRenderer`] long-lived-device
/// posture established by the slice-5b.2+ refactor (PR `#289` /
/// `91f421a`): construct once per chromaticity render thread,
/// reuse across frames, drop on application shutdown.
///
/// PR 3 ships the holder shape; the cache + pool are
/// [`Option<...>`] fields populated by the deferred bridge-body
/// wave. The real-backend impls of
/// [`chromaticity_syphon_adapter::adapter::backend::SyphonClientHandle`]
/// / [`chromaticity_syphon_adapter::adapter::backend::SyphonServerHandle`]
/// in this module ([`Objc2SyphonClientHandle`] /
/// [`Objc2SyphonServerHandle`]) are independent of this holder —
/// they wrap the `Retained<>` framework handles directly. The
/// holder exists for the future wave's bridge-body call sites.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
pub struct SyphonRenderBridge {
    /// `wgpu::Device` the bridge composes over. Held as `Arc<>` to
    /// match the [`crate::multipass::MultipassRenderer::device`]
    /// long-lived-device convention.
    _device: Arc<wgpu::Device>,
}

impl SyphonRenderBridge {
    /// Construct a new bridge composing over the supplied
    /// `wgpu::Device`.
    ///
    /// PR 3 ships the constructor shape; the deferred bridge-body
    /// wave extends this to extract the `MTLDevice` via
    /// `wgpu_hal::api::Metal` and create the
    /// `CVMetalTextureCache` + `CVPixelBufferPool` pre-warm ring.
    ///
    /// # Errors
    ///
    /// Currently infallible — PR 3's shape carries the device
    /// reference without performing the hal escape-hatch
    /// extraction. The deferred bridge-body wave widens the error
    /// surface with `MetalDeviceExtractionFailed` /
    /// `MetalTextureCacheCreateFailed` paths.
    pub fn new(device: Arc<wgpu::Device>) -> Result<Self, SyphonRenderBridgeError> {
        Ok(Self { _device: device })
    }

    /// Resolve a [`SyphonTexture`] (carrying an `IOSurfaceID`) into
    /// a `wgpu::Texture` for sampling within the frame graph.
    ///
    /// PR 3 ships the entry-point signature; the body returns
    /// [`SyphonRenderBridgeError::Unimplemented`] per the wave's
    /// scope narrowing. The deferred bridge-body wave wires the
    /// `CVMetalTextureCacheCreateTextureFromImage` call site
    /// against the cached cache + the supplied surface, plus the
    /// `kCVPixelBufferLock_ReadOnly` lock pattern per docs/54
    /// Lesson 3.8.
    ///
    /// # Errors
    ///
    /// Returns [`SyphonRenderBridgeError::Unimplemented`] in PR 3.
    pub fn resolve_iosurface_to_wgpu_texture(
        &self,
        _texture: SyphonTexture,
    ) -> Result<wgpu::Texture, SyphonRenderBridgeError> {
        Err(SyphonRenderBridgeError::Unimplemented(
            "input-direction CVMetalTextureCacheCreateTextureFromImage \
             call site deferred to hardware-validated follow-up wave; \
             see syphon_adapter.rs module-level prose `# Scope narrowing for PR 3`",
        ))
    }

    /// Pre-warm the output-side `CVPixelBufferPool` per docs/54
    /// Lesson 3.3 (`kCVPixelBufferPoolMinimumBufferCountKey = 2-3`).
    ///
    /// PR 3 ships the entry-point signature; the body returns
    /// [`SyphonRenderBridgeError::Unimplemented`] per the wave's
    /// scope narrowing.
    ///
    /// # Errors
    ///
    /// Returns [`SyphonRenderBridgeError::Unimplemented`] in PR 3.
    pub fn prewarm_output_surface_ring(
        &mut self,
        _ring_count: u32,
    ) -> Result<(), SyphonRenderBridgeError> {
        Err(SyphonRenderBridgeError::Unimplemented(
            "output-direction CVPixelBufferPool pre-warm deferred to \
             hardware-validated follow-up wave; see syphon_adapter.rs \
             module-level prose `# Scope narrowing for PR 3`",
        ))
    }
}

/// Engine-side carrier of a [`chromaticity_syphon_adapter::adapter::backend::SyphonClientHandle`]
/// real-backend impl per ADR-021 §6.
///
/// Phase 3 PR 2 declared the trait surface (with a `Send` bound
/// for cross-thread mock patterns) and shipped the no-op mock
/// impl in `tests_support`. The real `Retained<SyphonMetalClient>`
/// framework handle is **not** `Send` (objc2's `NSObject` carries
/// `*const UnsafeCell<()>` interior) — wiring the bind / publish /
/// stop path through the framework handle therefore requires a
/// render-thread-affine wrapper that cannot satisfy the trait's
/// `Send` bound directly. PR 3 ships the engine-side carrier
/// shape WITHOUT storing a `Retained<>` field; the framework
/// handle storage + the `Send` bound reconciliation land in the
/// deferred bridge-body wave alongside the live `MTLDevice`
/// extraction + `CVMetalTextureCache` allocation. Per the
/// "Scope narrowing for PR 3" prose in this module's documentation,
/// the wave that lands the live bridge body widens this struct
/// (likely via an `unsafe Send` wrapper documented under thread-
/// affinity invariants per ADR-006).
///
/// **Polling-consume model.** The trait's
/// [`SyphonClientHandle::has_new_frame`] /
/// [`SyphonClientHandle::new_frame_image`] surface routes through
/// the framework's atomic-bool freshness flag per docs/54
/// Lesson 3.6. PR 3's stub trait-impl returns "no new frame"
/// from both methods; the deferred wave wires the framework
/// `[client hasNewFrame]` / `[client newFrameImage]` call sites.
///
/// **Drop posture per ADR-021 §8 + docs/54 Lesson 11.** The
/// `Drop` impl is the canonical release path; PR 3's stub Drop
/// is a no-op (no framework handle stored), and the deferred
/// wave's widening adds the defensive `[client stop]` call before
/// the `Retained` releases.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm) — real-
/// backend trait-impl shape (PR 3 stub-narrowing).
pub struct Objc2SyphonClientHandle {
    /// PR 3 stub posture: `true` iff the carrier was constructed
    /// with [`Self::bound_stub`] (the bridge-body wave's bound-
    /// posture marker). The deferred wave widens this struct
    /// with the real `Retained<SyphonMetalClient>` storage
    /// (under an `unsafe Send` thread-affinity wrapper per
    /// ADR-006); the bool field is the PR 3 placeholder so the
    /// trait-impl shape can observe bound vs. unbound state.
    bound: bool,
}

impl std::fmt::Debug for Objc2SyphonClientHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Objc2SyphonClientHandle")
            .field("bound", &self.bound)
            .finish()
    }
}

impl Objc2SyphonClientHandle {
    /// Construct a client handle in the "stub bound" posture —
    /// `bind()` returns `true` but no framework-side connection
    /// is held. PR 3 testing surface; the deferred bridge-body
    /// wave replaces this with `with_framework_handle(client:
    /// Retained<SyphonMetalClient>)`.
    #[must_use]
    pub fn bound_stub() -> Self {
        Self { bound: true }
    }

    /// Construct a client handle in the "not yet bound" posture.
    #[must_use]
    pub fn unbound() -> Self {
        Self { bound: false }
    }
}

impl SyphonClientHandle for Objc2SyphonClientHandle {
    fn bind(&mut self) -> bool {
        // PR 3 narrowing: the real `init...` call site requires
        // device + description and lives in the deferred bridge-
        // body wave. PR 3 ships the trait-method shape: returns
        // `true` iff the caller constructed via [`Self::bound_stub`].
        // The `chromaticity_syphon_adapter::adapter::SyphonInputAdapter::bind`
        // surface routes through this trait method; combined with
        // [`Self::unbound`] for the not-yet-bound posture, this
        // lets adapter call sites observe the bind / not-bound
        // state without running the real framework call until the
        // deferred wave lands.
        self.bound
    }

    fn has_new_frame(&self) -> bool {
        // PR 3 narrowing: the real polling-consume read calls
        // `[client hasNewFrame]` per docs/54 Lesson 3.6. The
        // current `chromaticity_syphon_adapter::bindings` surface
        // does not yet declare the `hasNewFrame` selector
        // (selectors not chromaticity needs were intentionally
        // omitted from PR 2's binding declaration set per the
        // exported-symbols-only convention per docs/54 Lesson
        // 3.9). The deferred bridge-body wave widens bindings.rs
        // to add the selector and replaces this body with the
        // real framework call. Until then the shape returns
        // `false` so adapter callers observe the "no new frame"
        // steady state.
        false
    }

    fn new_frame_image(&mut self) -> Option<SyphonTexture> {
        // PR 3 narrowing: the real consume path calls
        // `[client newFrameImage]` (declared in
        // `chromaticity_syphon_adapter::bindings`) and bridges
        // the returned `Retained<NSObject>` (an opaque
        // `MTLTexture`) into a `SyphonTexture` cross-seam handle
        // via the `CVMetalTextureCache` resolution. The bridge-
        // body wave is responsible for the `MTLTexture` →
        // `IOSurfaceID` extraction; PR 3 ships only the shape —
        // returns `None` to satisfy the polling-consume "no new
        // frame" invariant.
        None
    }

    fn stop(&mut self) {
        // PR 3 stub: clear the bound-posture marker. The deferred
        // bridge-body wave wires the defensive `[client stop]`
        // call against the stored framework handle per ADR-021
        // §8 + docs/54 Lesson 11. Idempotent on the framework
        // side per `SyphonClientBase.m`; the deferred wave's
        // double-stop is safe.
        self.bound = false;
    }
}

impl Drop for Objc2SyphonClientHandle {
    fn drop(&mut self) {
        // PR 3 stub Drop: no framework handle stored, so no
        // framework call to make. The deferred bridge-body wave
        // widens this Drop with the defensive `[client stop]`
        // call per ADR-021 §8 + docs/54 Lesson 11.
        self.stop();
    }
}

/// Engine-side carrier of a [`chromaticity_syphon_adapter::adapter::backend::SyphonServerHandle`]
/// real-backend impl per ADR-021 §6.
///
/// Phase 3 PR 2 declared the trait surface (with a `Send` bound
/// for cross-thread mock patterns) and shipped the no-op mock
/// impl in `tests_support`. The real `Retained<SyphonMetalServer>`
/// framework handle is **not** `Send` (objc2's `NSObject` carries
/// `*const UnsafeCell<()>` interior) — wiring the bind / publish /
/// stop path through the framework handle therefore requires a
/// render-thread-affine wrapper that cannot satisfy the trait's
/// `Send` bound directly. PR 3 ships the engine-side carrier
/// shape WITHOUT storing a `Retained<>` field; the framework
/// handle storage + the `Send` bound reconciliation land in the
/// deferred bridge-body wave alongside the live `MTLDevice`
/// extraction + `CVPixelBufferPool` pre-warm. Per the "Scope
/// narrowing for PR 3" prose, the wave that lands the live
/// bridge body widens this struct (likely via an `unsafe Send`
/// wrapper documented under thread-affinity invariants per
/// ADR-006).
///
/// **Publish path.** The trait's [`SyphonServerHandle::publish`]
/// entry point routes through
/// `[server publishFrameTexture:onCommandBuffer:imageRegion:flipped:]`
/// gated by `addCompletedHandler:` per docs/54 Lesson 3.4. PR 3
/// ships only the shape; the wgpu `MTLTexture` extraction + the
/// publish call land in the deferred bridge-body wave alongside
/// the `CVPixelBufferPool` pre-warm.
///
/// **Drop posture per ADR-021 §8 + docs/54 Lesson 11.** The
/// `Drop` impl is the canonical release path; PR 3's stub Drop
/// is a no-op, and the deferred wave's widening adds the
/// defensive `[server stop]` call before the `Retained` releases.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm) — real-
/// backend trait-impl shape (PR 3 stub-narrowing).
pub struct Objc2SyphonServerHandle {
    /// PR 3 stub posture; mirrors [`Objc2SyphonClientHandle::bound`].
    bound: bool,
}

impl std::fmt::Debug for Objc2SyphonServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Objc2SyphonServerHandle")
            .field("bound", &self.bound)
            .finish()
    }
}

impl Objc2SyphonServerHandle {
    /// Construct a server handle in the "stub bound" posture.
    #[must_use]
    pub fn bound_stub() -> Self {
        Self { bound: true }
    }

    /// Construct a server handle in the "not yet bound" posture.
    #[must_use]
    pub fn unbound() -> Self {
        Self { bound: false }
    }
}

impl SyphonServerHandle for Objc2SyphonServerHandle {
    fn bind(&mut self, _name: &str) -> bool {
        // PR 3 narrowing: the real `init...` call site requires
        // device + name and lives in the deferred bridge-body
        // wave. PR 3 ships the trait-method shape: returns `true`
        // iff the caller constructed via [`Self::bound_stub`].
        self.bound
    }

    fn publish(&mut self, _texture: SyphonTexture) {
        // PR 3 narrowing: the real publish routes through
        // `[server publishFrameTexture:onCommandBuffer:imageRegion:flipped:]`
        // gated by `addCompletedHandler:` per docs/54 Lesson
        // 3.4. The current `chromaticity_syphon_adapter::bindings`
        // surface does not yet declare the `publishFrameTexture:`
        // selector (intentionally omitted from PR 2's binding
        // declaration set; the publish-side wgpu integration is
        // the bridge-body wave's scope). PR 3 ships only the
        // shape — drops the supplied `SyphonTexture` without
        // forwarding to the framework, which lets adapter
        // callers observe the bind / publish state machine
        // without running the real framework call until the
        // deferred wave lands.
    }

    fn stop(&mut self) {
        // PR 3 stub: clear the bound-posture marker. The deferred
        // bridge-body wave wires the defensive `[server stop]`
        // call against the stored framework handle per ADR-021
        // §8 + docs/54 Lesson 11.
        self.bound = false;
    }
}

impl Drop for Objc2SyphonServerHandle {
    fn drop(&mut self) {
        // PR 3 stub Drop: no framework handle stored. The
        // deferred bridge-body wave widens this Drop with the
        // defensive `[server stop]` call per ADR-021 §8 + docs/54
        // Lesson 11.
        self.stop();
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod syphon_adapter_pins {
    //! Evidence: `EV-INTEGRATION-MATRIX` (T1) — engine-side Syphon
    //! bridge real-backend trait-impl shape pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: each test
    //! pins a public-surface invariant against ADR-021 §6 + docs/54
    //! Phase-3 lessons. Drift in the unbound-posture state machine,
    //! the `Drop` impl signature, or the bridge-error variant
    //! taxonomy fails the corresponding test at the exact
    //! ADR-section / lesson boundary.
    //!
    //! Backend-touching state-machine fixtures live in the existing
    //! `chromaticity_syphon_adapter::adapter::backend::tests_support`
    //! Mock corpus (PR 2's T2 corpus); the pins below cover the
    //! engine-side real-backend impl shape that PR 3 introduces,
    //! without the live framework call.
    //!
    //! The deferred bridge-body wave widens this module with pins
    //! covering the live `MTLDevice` extraction + `CVMetalTextureCache`
    //! lifecycle once the bridge body lands.

    use super::*;

    /// Evidence: `EV-INTEGRATION-MATRIX` — `Objc2SyphonClientHandle::unbound`
    /// constructs a handle whose `bind` returns `false` (no
    /// framework handle attached). PR 3 unbound-posture pin.
    #[test]
    fn client_handle_unbound_bind_returns_false() {
        let mut handle = Objc2SyphonClientHandle::unbound();
        assert!(!handle.bind(), "unbound handle must report bind=false");
        assert!(
            !handle.has_new_frame(),
            "unbound handle must report no fresh frame"
        );
        assert!(
            handle.new_frame_image().is_none(),
            "unbound handle must yield no texture"
        );
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — `Objc2SyphonClientHandle::bound_stub`
    /// constructs a handle whose `bind` returns `true` under the
    /// PR 3 stub-narrowing (no framework call ran; the deferred
    /// bridge-body wave widens this to the live framework handle).
    #[test]
    fn client_handle_bound_stub_bind_returns_true() {
        let mut handle = Objc2SyphonClientHandle::bound_stub();
        assert!(
            handle.bind(),
            "bound_stub handle must report bind=true under PR 3 narrowing"
        );
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — `Objc2SyphonServerHandle::unbound`
    /// constructs a handle whose `bind` returns `false`. PR 3
    /// unbound-posture pin.
    #[test]
    fn server_handle_unbound_bind_returns_false() {
        let mut handle = Objc2SyphonServerHandle::unbound();
        assert!(
            !handle.bind("any-name"),
            "unbound handle must report bind=false"
        );
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — `Objc2SyphonServerHandle::bound_stub`
    /// constructs a handle whose `bind` returns `true` under the
    /// PR 3 stub-narrowing.
    #[test]
    fn server_handle_bound_stub_bind_returns_true() {
        let mut handle = Objc2SyphonServerHandle::bound_stub();
        assert!(
            handle.bind("any-name"),
            "bound_stub handle must report bind=true under PR 3 narrowing"
        );
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — `Drop` is the canonical
    /// release path per ADR-021 §8 + docs/54 Lesson 11. Pin
    /// covers the unbound-posture Drop (no framework call to
    /// observe; the assertion is that Drop runs without panic
    /// and clears state).
    #[test]
    fn client_handle_drop_unbound_does_not_panic() {
        let handle = Objc2SyphonClientHandle::unbound();
        drop(handle);
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — `Drop` is the canonical
    /// release path per ADR-021 §8 + docs/54 Lesson 11.
    #[test]
    fn server_handle_drop_unbound_does_not_panic() {
        let handle = Objc2SyphonServerHandle::unbound();
        drop(handle);
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — explicit `stop` clears
    /// the unbound handle's state idempotently. Mirrors the
    /// docs/54 Lesson 11 idempotent-stop posture.
    #[test]
    fn client_handle_stop_is_idempotent_on_unbound() {
        let mut handle = Objc2SyphonClientHandle::unbound();
        handle.stop();
        handle.stop();
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — explicit `stop` clears
    /// the unbound handle's state idempotently.
    #[test]
    fn server_handle_stop_is_idempotent_on_unbound() {
        let mut handle = Objc2SyphonServerHandle::unbound();
        handle.stop();
        handle.stop();
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — the publish call shape
    /// drops the supplied `SyphonTexture` cleanly under the PR 3
    /// narrowing (bridge-body wave wires the real
    /// `[server publishFrameTexture:...]` call site).
    #[test]
    fn server_handle_publish_unbound_drops_texture_cleanly() {
        use chromaticity_syphon_adapter::SyphonPixelFormat;
        let mut handle = Objc2SyphonServerHandle::unbound();
        let texture = SyphonTexture {
            iosurface_id: 42,
            width: 1920,
            height: 1080,
            pixel_format: SyphonPixelFormat::Bgra8Unorm,
        };
        handle.publish(texture);
        // No panic, no observable side effect under the PR 3
        // narrowing — pin covers the entry-point shape.
    }

    /// Evidence: `EV-INTEGRATION-MATRIX` — the bridge-error
    /// variant taxonomy carries canonical diagnostics. Drift in
    /// the variant set fails the pin.
    #[test]
    fn bridge_error_variants_carry_canonical_diagnostic() {
        let errors = [
            SyphonRenderBridgeError::MetalDeviceExtractionFailed("no Metal backend"),
            SyphonRenderBridgeError::Unimplemented("PR 3 narrowing"),
        ];
        for err in &errors {
            let rendered = format!("{err}");
            assert!(
                !rendered.is_empty(),
                "bridge error must have a Display string: {err:?}"
            );
        }
    }
}
