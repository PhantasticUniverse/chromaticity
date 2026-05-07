// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity Syphon I/O adapter (macOS-only).
//!
//! Phase-4a Wave 1c-β Phase 3 PR 1 scaffold per ADR-021. The crate
//! is gated by `#![cfg(target_os = "macos")]`: on Windows + Linux it
//! compiles to an empty rlib (no items, no framework dependency).
//! The macOS-only feature gate lives at the crate root rather than
//! at workspace `[members]` per ADR-021 §1, so the workspace
//! topology stays single-list while non-macOS contributes zero items.
//!
//! Public surface (locked by ADR-021 §4):
//!
//! - [`SyphonInputAdapter`] — consumes frames published by other
//!   macOS apps on the Syphon bus. Polling-consume model per ADR-021
//!   §4 (block-callback path explicitly rejected for the binding
//!   simplicity + thread-affinity reasons cited in the ADR; cf.
//!   `docs/54-wave-1c-beta-research-lessons.md` Phase-3 lesson 1).
//! - [`SyphonOutputAdapter`] — publishes frames produced by the
//!   chromaticity render pipeline.
//! - [`SyphonServerDirectory`] — singleton discovery surface keyed
//!   off `SyphonServerDirectory`'s `+sharedDirectory` Objective-C
//!   class method, backed by `NSDistributedNotificationCenter` for
//!   cross-process server-add / server-update / server-remove
//!   notifications.
//! - [`SyphonServerId`] — stable server identifier with
//!   `dictionary_version: u32` forward-compat refusal field per
//!   ADR-021 §4 + the canonical `SyphonClientBase.m:87-93` defensive
//!   forward-compat refusal pattern.
//! - [`SyphonTexture`] — cross-seam texture handle. Pixel format is
//!   locked to BGRA8Unorm per docs/54 Phase-3 lesson 4 (universally-
//!   supported across Syphon consumers).
//!
//! ## Adapter shape
//!
//! Mirrors the OSC + MIDI adapter precedent per ADR-005: bind /
//! unbind / poll trait surface, defaults pinned via T1 drift pins
//! ([`defaults`]), route / discovery types in a sibling module
//! ([`directory`] + [`frames`]), no protocol-specific logic in core
//! rendering. The wgpu ↔ `IOSurface` bridge contract is locked in
//! ADR-021 §6 but lives engine-side at
//! `engine/render/src/syphon_adapter.rs` (Phase 3 PR 3) — this
//! crate exposes only the cross-seam texture handle.
//!
//! ## Unsafe posture
//!
//! Phase 3 PR 2 narrowly loosens the workspace
//! `unsafe_code = "forbid"` floor for this crate to `"deny"` (per
//! the crate-level `[lints.rust] unsafe_code = "deny"` override in
//! `Cargo.toml` authorized by ADR-021 §3 + `docs/43-development-standards.md`
//! §12.6). The `[crate::bindings]` module carries
//! `#![allow(unsafe_code)]` and is the **only** module in this
//! crate where unsafe is used; all other modules
//! ([`crate::adapter`], [`crate::defaults`], [`crate::directory`],
//! [`crate::frames`]) are safe Rust. Workspace floors for
//! `clippy::unwrap_used`, `clippy::expect_used`, and
//! `clippy::missing_safety_doc` stay in force unrelaxed.
//!
//! ## Texture binding namespace
//!
//! The engine-side wgpu interop bridge (Phase 3 PR 3) exposes input
//! textures under the `chroma_SyphonInput[N]` namespace; output
//! servers register under the [`defaults::DEFAULT_OUTPUT_NAME_TEMPLATE`]
//! template (operator-overridable).
//!
//! ## License
//!
//! Chromaticity links against the canonical Syphon-Framework
//! (BSD-3-Clause; copyright 2010-2023 bangnoise (Tom Butterworth) &
//! vade (Anton Marini); Metal subtree co-authored 2020-2023 by
//! Maxime Touroute & Philippe Chaurand of Millumin). License
//! compliance per ADR-021 §9: attribution lives at
//! `LICENSES/BSD-3-Clause.txt` and in the chromaticity About /
//! Acknowledgements UI surface; the no-endorsement clause restricts
//! marketing copy to factual interoperability ("Syphon-compatible")
//! rather than endorsement language.
//!
//! ## Evidence
//!
//! T1 evidence: [`EV-INTEGRATION-MATRIX`] (Syphon arm, Phase-4a
//! Wave 1c-β widening). The closing-docs PR 3 records the synthetic
//! narrowing the Phase 3 PR 2 T2 corpus uses (no live Syphon bus on
//! CI; mock framework trait) per the row 78 / row 80 / row 93
//! precedent.
//!
//! [`EV-INTEGRATION-MATRIX`]: https://github.com/PhantasticUniverse/chromaticity/blob/main/docs/26-evidence-artifact-registry.md

#![cfg(target_os = "macos")]

pub mod adapter;
pub mod bindings;
pub mod defaults;
pub mod directory;
pub mod frames;

pub use adapter::{AdapterHealth, SyphonAdapterError, SyphonInputAdapter, SyphonOutputAdapter};
pub use defaults::{DEFAULT_DISCOVERY_INTERVAL_MS, DEFAULT_OUTPUT_NAME_TEMPLATE};
pub use directory::SyphonServerDirectory;
pub use frames::{KNOWN_DICTIONARY_VERSION, SyphonPixelFormat, SyphonServerId, SyphonTexture};
