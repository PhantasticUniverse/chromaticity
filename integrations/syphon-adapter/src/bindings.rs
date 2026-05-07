// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! `objc2 = "0.5"` framework binding declarations for the canonical
//! Syphon-Framework classes per ADR-021 §2 + §3.
//!
//! This module is the **only** place in the crate where the workspace
//! `unsafe_code = "forbid"` floor is loosened. The crate-level
//! `[lints.rust] unsafe_code = "deny"` override in `Cargo.toml`
//! permits the per-module `#![allow(unsafe_code)]` annotation below;
//! all other modules of this crate are safe Rust. ADR-021 §3
//! authorizes this narrow loosening exclusively for the
//! Objective-C framework binding declarations.
//!
//! The bindings target three classes from the canonical
//! `third_party/Syphon-Framework/`:
//!
//! - `SyphonMetalServer` (declared in `SyphonMetalServer.h`) — the
//!   publish surface. Methods bound:
//!   - `-initWithName:device:options:`
//!   - `-publishFrameTexture:onCommandBuffer:imageRegion:flipped:`
//!   - `-stop`
//!   - `-name` (read property)
//!
//! - `SyphonMetalClient` (declared in `SyphonMetalClient.h`) — the
//!   consume surface. Methods bound:
//!   - `-initWithServerDescription:device:options:newFrameHandler:`
//!     (chromaticity passes `nil` for the new-frame handler per
//!     ADR-021 §4 polling-consume preference; cf.
//!     `docs/54-wave-1c-beta-research-lessons.md` Lesson 3.6
//!     atomic-bool freshness flag).
//!   - `-newFrameImage` — caller is responsible for releasing the
//!     returned `MTLTexture` (per the canonical
//!     `SyphonMetalClient.h:54-56` doc comment).
//!   - `-stop`
//!
//! - `SyphonServerDirectory` (declared in
//!   `SyphonServerDirectory.h`) — the discovery singleton. Methods
//!   bound:
//!   - `+sharedDirectory` (class method, returns the process-global
//!     singleton).
//!   - `-servers` (read property; returns
//!     `NSArray<NSDictionary<NSString *, id<NSCoding>> *>`).
//!
//! The canonical `kSyphonDictionaryVersion` constant
//! (`SyphonPrivate.h:30`, `#define kSyphonDictionaryVersion 0U`) is
//! mirrored at [`crate::frames::KNOWN_DICTIONARY_VERSION`] rather
//! than re-exposed here; the per-server filter that reads
//! `SyphonServerDescriptionDictionaryVersionKey` and refuses
//! versions exceeding the known value lives in
//! [`crate::directory`] per the canonical `SyphonClientBase.m:87-93`
//! defensive forward-compat refusal pattern.
//!
//! ## Linkage posture
//!
//! These declarations name the framework-side classes by their
//! Objective-C selector strings; the runtime resolves them via
//! `objc_getClass` on first call. Linkage to the framework binary is
//! orchestrated by `build.rs` (the `cargo:rustc-link-lib=framework=Syphon`
//! directive when the framework is present, or a `cargo:warning` skip
//! when absent). `cargo check` does not link, so the declarations
//! below compile cleanly even on a host that lacks the framework
//! binary; link-time resolution surfaces only at `cargo build` /
//! `cargo test`.
//!
//! ## Wire-up to higher-level wrappers
//!
//! The trait abstraction in [`crate::adapter::backend`] keeps the
//! polling-consume and publish paths testable without a live Syphon
//! bus: a real `Objc2Backend` (this module) wraps `Retained<...>`
//! handles into the trait, and a `MockBackend` in the T2 corpus
//! satisfies the same trait against in-memory fixtures.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

#![allow(unsafe_code)]
// Binding method names mirror the Objective-C selector shape
// (e.g. `initWithName:device:options:` -> `initWithName_device_options`)
// so the diff target against a future framework upgrade is the
// selector string verbatim. Standard Rust snake-case would obscure
// the selector mapping at no clarity gain. This allow is scoped to
// the `extern_methods!` blocks below; non-binding code in this
// module follows the workspace snake-case convention.
#![allow(non_snake_case)]

use objc2::ClassType;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{extern_class, extern_methods, msg_send_id, mutability};
use objc2_foundation::{NSArray, NSDictionary, NSString};

extern_class!(
    /// Bind for `SyphonMetalServer` (declared
    /// `third_party/Syphon-Framework/SyphonMetalServer.h:50`). The
    /// canonical class hierarchy is
    /// `SyphonMetalServer -> SyphonServerBase -> NSObject`; the
    /// chromaticity binding flattens the hierarchy to `NSObject` per
    /// the standard objc2 0.5 cookbook because the intermediate
    /// `SyphonServerBase` does not expose any methods chromaticity
    /// calls (the `init*` / `publish*` / `stop` methods chromaticity
    /// uses are declared on `SyphonMetalServer` directly).
    #[derive(PartialEq, Eq, Hash)]
    pub struct SyphonMetalServer;

    unsafe impl ClassType for SyphonMetalServer {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "SyphonMetalServer";
    }
);

extern_methods!(
    /// `SyphonMetalServer` framework methods chromaticity binds.
    ///
    /// # Safety
    ///
    /// The `device: &NSObject` parameter on `initWithName:device:options:`
    /// must point to a valid `id<MTLDevice>` for the lifetime of the
    /// returned server; passing a non-Metal-device pointer is
    /// undefined behavior per the framework contract. The
    /// chromaticity engine-side bridge in Phase 3 PR 3 supplies the
    /// wgpu-owned `MTLDevice` from the `wgpu::hal` Metal backend per
    /// ADR-011 §4.
    unsafe impl SyphonMetalServer {
        /// `-[SyphonMetalServer initWithName:device:options:]`. Returns
        /// `nil` (mapped to `None`) if the framework refuses the
        /// registration (most commonly: a peer process already
        /// registered a server under the same name).
        ///
        /// # Safety
        ///
        /// `device` must be a valid `id<MTLDevice>` per the framework
        /// contract. `options` may be `nil`; chromaticity passes
        /// `None` for the MVP path (no `SyphonServerOptionIsPrivate`
        /// usage).
        #[method_id(initWithName:device:options:)]
        pub unsafe fn initWithName_device_options(
            this: objc2::rc::Allocated<Self>,
            name: Option<&NSString>,
            device: &NSObject,
            options: Option<&NSDictionary<NSString, NSObject>>,
        ) -> Option<Retained<Self>>;

        /// `-[SyphonMetalServer name]`. Read property: returns the
        /// human-readable server name set at init time.
        #[method_id(name)]
        pub fn name(&self) -> Option<Retained<NSString>>;

        /// `-[SyphonMetalServer stop]`. Stops the server instance.
        /// Idempotent on the framework side per `SyphonServerBase.m`.
        ///
        /// chromaticity's [`crate::adapter::SyphonOutputAdapter::Drop`]
        /// impl calls this defensively per ADR-021 §8 + docs/54
        /// Lesson 11.
        #[method(stop)]
        pub fn stop(&self);
    }
);

extern_class!(
    /// Bind for `SyphonMetalClient` (declared
    /// `third_party/Syphon-Framework/SyphonMetalClient.h:36`). The
    /// canonical class hierarchy is
    /// `SyphonMetalClient -> SyphonClientBase -> NSObject`; the
    /// chromaticity binding flattens to `NSObject` for the same
    /// reason as `SyphonMetalServer` above (the
    /// `SyphonClientBase` intermediate does not expose any methods
    /// chromaticity calls directly through this binding — the
    /// `init*` / `newFrameImage` / `stop` methods chromaticity uses
    /// are declared on `SyphonMetalClient`).
    #[derive(PartialEq, Eq, Hash)]
    pub struct SyphonMetalClient;

    unsafe impl ClassType for SyphonMetalClient {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "SyphonMetalClient";
    }
);

extern_methods!(
    /// `SyphonMetalClient` framework methods chromaticity binds.
    ///
    /// # Safety
    ///
    /// The `description` parameter on
    /// `initWithServerDescription:device:options:newFrameHandler:`
    /// must be a server-description `NSDictionary` obtained from
    /// `[SyphonServerDirectory sharedDirectory].servers` (or one of
    /// the announce / update notifications). Passing an arbitrary
    /// dictionary is undefined behavior per the framework contract.
    /// Chromaticity's [`crate::directory`] surface enforces this by
    /// only ever yielding descriptions sourced from the canonical
    /// directory.
    ///
    /// The `device: &NSObject` must be a valid `id<MTLDevice>`. The
    /// `handler` argument is **always `nil`** in chromaticity's
    /// binding per ADR-021 §4 (polling-consume model preference);
    /// the type signature exposes `Option<&NSObject>` only to keep
    /// the binding declaration faithful to the framework contract.
    unsafe impl SyphonMetalClient {
        /// `-[SyphonMetalClient initWithServerDescription:device:options:newFrameHandler:]`.
        /// Returns `nil` (mapped to `None`) if the framework refuses
        /// the connection (most commonly: the server vanished from
        /// the bus between discovery and bind, or the
        /// `dictionaryVersion` exceeded `kSyphonDictionaryVersion`
        /// per `SyphonClientBase.m:87-93`).
        ///
        /// # Safety
        ///
        /// `description` must be a server-description `NSDictionary`
        /// from the canonical directory; `device` must be a valid
        /// `id<MTLDevice>`; `handler` is always `None` in
        /// chromaticity's binding per ADR-021 §4.
        #[method_id(initWithServerDescription:device:options:newFrameHandler:)]
        pub unsafe fn initWithServerDescription_device_options_newFrameHandler(
            this: objc2::rc::Allocated<Self>,
            description: &NSDictionary<NSString, NSObject>,
            device: &NSObject,
            options: Option<&NSDictionary<NSString, NSObject>>,
            handler: Option<&NSObject>,
        ) -> Option<Retained<Self>>;

        /// `-[SyphonMetalClient newFrameImage]`. Returns the most
        /// recent `MTLTexture` published by the server, or `nil`
        /// when no frame has been received since the last call.
        ///
        /// **YOU ARE RESPONSIBLE FOR RELEASING THIS OBJECT** per the
        /// canonical `SyphonMetalClient.h:54` doc comment. The
        /// `Retained<NSObject>` return type carries the +1 ownership;
        /// dropping the `Retained` releases the texture.
        #[method_id(newFrameImage)]
        pub fn newFrameImage(&self) -> Option<Retained<NSObject>>;

        /// `-[SyphonMetalClient stop]`. Stops the client from
        /// receiving any further frames. Idempotent on the framework
        /// side per `SyphonClientBase.m`.
        ///
        /// chromaticity's [`crate::adapter::SyphonInputAdapter::Drop`]
        /// impl calls this defensively per ADR-021 §8 + docs/54
        /// Lesson 11.
        #[method(stop)]
        pub fn stop(&self);
    }
);

extern_class!(
    /// Bind for `SyphonServerDirectory` (declared
    /// `third_party/Syphon-Framework/SyphonServerDirectory.h:80`).
    /// The class hierarchy is `SyphonServerDirectory -> NSObject`.
    /// The class is a process-global singleton accessed via the
    /// `+sharedDirectory` class method.
    #[derive(PartialEq, Eq, Hash)]
    pub struct SyphonServerDirectoryClass;

    unsafe impl ClassType for SyphonServerDirectoryClass {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "SyphonServerDirectory";
    }
);

extern_methods!(
    /// `SyphonServerDirectory` framework methods chromaticity binds.
    ///
    /// # Safety
    ///
    /// `+sharedDirectory` is safe per the canonical Apple-runtime
    /// guarantee that class methods returning singletons may be
    /// called from any thread without synchronization. The returned
    /// dictionary array (`servers` property) is KVO-compliant and
    /// may change between successive reads; callers must snapshot
    /// the array before iterating.
    unsafe impl SyphonServerDirectoryClass {
        /// `+[SyphonServerDirectory sharedDirectory]`. Returns the
        /// process-global singleton instance.
        ///
        /// chromaticity's [`crate::directory::SyphonServerDirectory`]
        /// caches a `Retained<SyphonServerDirectoryClass>` from the
        /// first call and reuses it for subsequent `current_servers`
        /// queries.
        pub fn shared_directory() -> Retained<Self> {
            // SAFETY: `+sharedDirectory` is a class method per the
            // canonical `SyphonServerDirectory.h:86`; `msg_send_id!`
            // with the class identifier returns a `Retained<Self>`.
            unsafe { msg_send_id![Self::class(), sharedDirectory] }
        }

        /// `-[SyphonServerDirectory servers]`. Read property:
        /// snapshot of the current set of advertised servers as an
        /// `NSArray<NSDictionary<NSString *, id>>`.
        #[method_id(servers)]
        pub fn servers(&self) -> Retained<NSArray<NSDictionary<NSString, NSObject>>>;
    }
);

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod bindings_pins {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — `objc2` framework
    //! binding-declaration shape pins.
    //!
    //! These pins compile-only — they assert that the binding types
    //! satisfy the `ClassType` trait so a future objc2 version bump
    //! that breaks the macro expansion fails the corresponding test
    //! at the type-system boundary rather than at first runtime call.
    //! No live framework call is made here; the pins are static-shape
    //! verifications.

    use super::*;

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonMetalServer` binding
    /// satisfies `ClassType`. A future `objc2` version that breaks
    /// the `extern_class!` macro expansion fails this pin first.
    #[test]
    fn server_binding_implements_class_type() {
        fn assert_class_type<T: ClassType>() {}
        assert_class_type::<SyphonMetalServer>();
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonMetalClient` binding
    /// satisfies `ClassType`. PR 2 lock pin.
    #[test]
    fn client_binding_implements_class_type() {
        fn assert_class_type<T: ClassType>() {}
        assert_class_type::<SyphonMetalClient>();
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `SyphonServerDirectoryClass`
    /// binding satisfies `ClassType`. PR 2 lock pin.
    #[test]
    fn directory_binding_implements_class_type() {
        fn assert_class_type<T: ClassType>() {}
        assert_class_type::<SyphonServerDirectoryClass>();
    }
}
