// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Backend-trait abstraction for the Syphon adapter per ADR-021 §4.
//!
//! The trait abstraction below is the seam between the adapter's
//! public surface (`SyphonInputAdapter` / `SyphonOutputAdapter`) and
//! the framework-side `SyphonMetalClient` / `SyphonMetalServer`
//! handles. Two implementations satisfy the traits:
//!
//! 1. **Real `objc2` backend** (Phase 3 PR 3, `engine/render/src/syphon_adapter.rs`):
//!    wraps `Retained<SyphonMetalClient>` / `Retained<SyphonMetalServer>`
//!    handles obtained via [`crate::bindings`] against the wgpu-owned
//!    `MTLDevice`.
//!
//! 2. **Mock backend** (PR 2 T2 corpus,
//!    `tests/syphon_adapter_corpus.rs`): in-memory fixture-driven
//!    impl that exercises the adapter's bind / unbind / poll /
//!    publish state machine without a live Syphon bus on CI.
//!
//! ADR-021 §10 Negative consequence bullet 3 records this synthetic
//! narrowing honestly: hardware-rig CI validation against a live
//! Syphon bus is not feasible on GitHub Actions runners; the trait
//! abstraction is the conformance-corpus seam that lets the T2
//! corpus assert state-machine behavior end-to-end against a mock
//! framework.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

use crate::frames::SyphonTexture;

/// Abstraction over a `SyphonMetalClient` framework handle.
///
/// The trait surface mirrors the four canonical client methods
/// chromaticity uses: `init` (modeled as [`Self::bind`]),
/// `hasNewFrame`, `newFrameImage`, and `stop`. The
/// `initWithServerDescription:device:options:newFrameHandler:`
/// arguments are captured at trait-impl construction time rather
/// than passed through `bind` because the server description +
/// `MTLDevice` are properties of the impl, not of the bind call.
pub trait SyphonClientHandle: Send {
    /// Resolve the framework-side `SyphonMetalClient` connection
    /// against the captured server description + device. Returns
    /// `true` on successful framework-side `init`, `false` if the
    /// framework returned `nil` (server vanished or version
    /// incompatible).
    ///
    /// Idempotent: calling `bind` on an already-bound handle is a
    /// no-op that returns `true`.
    fn bind(&mut self) -> bool;

    /// Atomic-bool freshness flag check per the canonical
    /// `SyphonMetalClient.m:40, 91-104` polling pattern. Returns
    /// `true` iff the framework has published a new frame since the
    /// last [`Self::new_frame_image`] call.
    fn has_new_frame(&self) -> bool;

    /// Resolve the most recent frame published by the bound server
    /// into a [`SyphonTexture`] cross-seam descriptor.
    ///
    /// Returns `None` when the framework has no frame to surface
    /// (server published before bind, or `newFrameImage` returned
    /// `nil` for an internal framework reason). Caller is responsible
    /// for releasing the underlying `MTLTexture` per the canonical
    /// `SyphonMetalClient.h:54` doc comment; the real-backend impl
    /// in PR 3 holds the `Retained<NSObject>` until the next call
    /// to `new_frame_image` (drop-frames-on-saturation per ADR-021
    /// §4).
    fn new_frame_image(&mut self) -> Option<SyphonTexture>;

    /// Stop the framework-side client. Idempotent on the framework
    /// side per `SyphonClientBase.m`; safe to call multiple times.
    /// Called both from explicit `unbind` AND from the
    /// `SyphonInputAdapter::Drop` impl per ADR-021 §8 + docs/54
    /// Lesson 11.
    fn stop(&mut self);
}

/// Abstraction over a `SyphonMetalServer` framework handle.
///
/// The trait surface mirrors the canonical server methods
/// chromaticity uses: `init` (modeled as [`Self::bind`] taking the
/// name argument), `publishFrameTexture:onCommandBuffer:imageRegion:flipped:`
/// (modeled as [`Self::publish`] taking the cross-seam handle), and
/// `stop`. The `MTLDevice` is captured at trait-impl construction
/// time per the same rationale as `SyphonClientHandle`.
pub trait SyphonServerHandle: Send {
    /// Register the framework-side `SyphonMetalServer` advertising
    /// the supplied name. Returns `true` on successful framework-
    /// side `init`, `false` if the framework refused the
    /// registration (typically: a peer process already registered a
    /// server under the same name).
    ///
    /// Idempotent: calling `bind` on an already-bound handle with
    /// the same name is a no-op that returns `true`.
    fn bind(&mut self, name: &str) -> bool;

    /// Publish a frame to the bus. Wraps
    /// `publishFrameTexture:onCommandBuffer:imageRegion:flipped:`.
    /// PR 2 ships the type-shape contract; PR 3 wires the wgpu
    /// `MTLTexture` extraction + the actual `publishFrameTexture:`
    /// call site through the `engine/render` bridge.
    fn publish(&mut self, texture: SyphonTexture);

    /// Stop the framework-side server. Idempotent on the framework
    /// side per `SyphonServerBase.m`; safe to call multiple times.
    /// Called both from explicit `unbind` AND from the
    /// `SyphonOutputAdapter::Drop` impl per ADR-021 §8 + docs/54
    /// Lesson 11.
    fn stop(&mut self);
}

#[doc(hidden)]
pub mod tests_support {
    //! In-crate test-support fixtures. Used by the in-crate T1 pins
    //! (e.g. `adapter::adapter_pins::input_adapter_refuses_forward_incompat_dictionary_version`)
    //! and re-exported via `pub` so the workspace-level T2 corpus at
    //! `tests/syphon_adapter_corpus.rs` can build a fuller mock-
    //! backend taxonomy on top of these primitives.
    //!
    //! Marked `#[doc(hidden)]` so the public crate documentation
    //! does not surface the test-support module; the module is
    //! reachable from the integration-test crate (which sees the
    //! crate's full public surface) but is not promoted in the
    //! generated rustdoc.

    use super::*;

    /// No-op client handle that always reports `bind() == true` and
    /// `has_new_frame() == false`. Used for shape-only T1 pins where
    /// the bind-state-machine behavior is not the subject under test.
    #[derive(Debug)]
    pub struct NoopClientHandle;

    impl SyphonClientHandle for NoopClientHandle {
        fn bind(&mut self) -> bool {
            true
        }
        fn has_new_frame(&self) -> bool {
            false
        }
        fn new_frame_image(&mut self) -> Option<SyphonTexture> {
            None
        }
        fn stop(&mut self) {}
    }

    /// No-op server handle that always reports `bind() == true`.
    /// Used for shape-only T1 pins.
    #[derive(Debug)]
    pub struct NoopServerHandle;

    impl SyphonServerHandle for NoopServerHandle {
        fn bind(&mut self, _name: &str) -> bool {
            true
        }
        fn publish(&mut self, _texture: SyphonTexture) {}
        fn stop(&mut self) {}
    }
}
