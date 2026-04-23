// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine scene.
//!
//! First-slice implementation: reserves the canonical-model module
//! topology (`schema` + `validator`) with std-only, internal-only
//! types. No public API surface is exported yet — both modules are
//! `pub mod` to make their items reachable from intra-crate tests
//! and future sibling modules, but every item inside stays
//! `pub(crate)` or private until the schema freeze begins.
//!
//! The validator stub reads no files yet; the real validator will
//! cross-check symbol references against
//! `docs/36-reactive-symbol-catalog.md` (the authoritative source
//! of truth for reactive symbol names).
//!
//! Evidence: EV-SCENE-IMPORT (baseline placeholder test in
//! `validator::tests`).
//!
//! Governing ADRs:
//!
//! - ADR-007 — scene warm-switch policy
//!   (`docs/adr/ADR-007-scene-warm-switch-policy.md`).
//! - ADR-008 — transition and traveler semantics
//!   (`docs/adr/ADR-008-transition-and-traveler-semantics.md`).
//!
//! Symbol source of truth: `docs/36-reactive-symbol-catalog.md`.

pub mod schema;
pub mod validator;
