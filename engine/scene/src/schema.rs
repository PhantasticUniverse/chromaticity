// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene schema — first-slice Rust types.
//!
//! Reserves the canonical-model topology for the scene import
//! pipeline. Field coverage is intentionally minimal: just enough
//! for the `crate::validator` stub to typecheck and for the
//! `EV-SCENE-IMPORT` baseline test to exercise a no-op validation.
//!
//! Serde derives and the full field set land in a later slice once
//! the governing docs freeze the JSON surface. Until then, nothing
//! in this module is part of a user-visible API — the field names
//! here do NOT commit the `.chroma` JSON surface.
//!
//! Governing ADRs:
//!
//! - ADR-007 — scene warm-switch policy
//!   (`docs/adr/ADR-007-scene-warm-switch-policy.md`).
//! - ADR-008 — transition and traveler semantics
//!   (`docs/adr/ADR-008-transition-and-traveler-semantics.md`).
//!
//! Symbol source of truth: `docs/36-reactive-symbol-catalog.md`.

/// In-memory scene representation.
///
/// First-slice placeholder: the field set will grow as the
/// canonical-model normalizer lands. Fields stay crate-private for
/// now so that name churn cannot leak into downstream crates before
/// the schema freezes. `dead_code` is allowed because the fields
/// exist to reserve the canonical-model topology; the normalizer
/// that reads them lands in a follow-up slice.
#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct Scene {
    pub(crate) header: SceneHeader,
    pub(crate) passes: Vec<PassSpec>,
}

/// Scene header carrying version and identity metadata.
///
/// `schema_version` is the only field populated in the first slice;
/// it exists so the validator has something non-trivial to branch
/// on once the schema freeze begins.
#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub(crate) struct SceneHeader {
    pub(crate) schema_version: u32,
}

/// Single render-pass specification.
///
/// First-slice placeholder: intentionally empty. Pass bindings,
/// uniform declarations, and symbol references land with the
/// canonical-model normalizer.
#[derive(Debug, Default, Clone)]
pub(crate) struct PassSpec {}
