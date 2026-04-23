// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine scene.
//!
//! Slice-2 ships the scene-schema canonical-model types (typed
//! [`schema::ControlDecl`], [`schema::NodeRef`], grown
//! [`schema::PassSpec`] carrying [`schema::UniformBinding`] lists)
//! and the real [`validator::validate`] that replaces the slice-1
//! always-`Ok(())` stub. The validator cross-checks every uniform
//! binding against a representative subset of
//! `docs/36-reactive-symbol-catalog.md`, applies the
//! `docs/28-implementation-contract-pack.md` §CP-01 missing-TYPE
//! coercion fallback in [`validator::ValidationMode::Compat`], and
//! enforces node-ID uniqueness across scene passes. The
//! compatibility warning taxonomy from `docs/05-scene-system.md` §5
//! lives in [`lint::Warning`].
//!
//! Slice progression:
//!
//! - **Slice 1** — canonical-model module topology reserved;
//!   always-`Ok(())` validator stub; schema fields `pub(crate)` and
//!   no serde derives.
//! - **Slice 2 (this PR)** — real validator, CP-01 coercion, typed
//!   control declarations, serde derives (derive-only). Schema
//!   fields remain `pub(crate)`; the `.chroma` wire format is NOT
//!   frozen.
//! - **Slice 3 (next)** — `.chroma` wire-format deserializer, full
//!   symbol-catalog loader over `docs/36`, scene-lint harness
//!   emitting the [`lint::Warning`] taxonomy outside the validate
//!   path, and the first external reader that opens schema field
//!   visibility.
//!
//! Produces evidence for `EV-SCENE-IMPORT` (T0 placeholder retained
//! in `validator::ctor_placeholder`; T1 drift pins in
//! `validator::drift_pins`) and `EV-SCHEMA-LINT` (T0 variant
//! constructibility in `lint::drift_pins` — first claim against the
//! row anywhere in the tree).
//!
//! Governing ADRs:
//!
//! - ADR-007 — scene warm-switch policy
//!   (`docs/adr/ADR-007-scene-warm-switch-policy.md`).
//! - ADR-008 — transition and traveler semantics
//!   (`docs/adr/ADR-008-transition-and-traveler-semantics.md`).
//!
//! Symbol source of truth: `docs/36-reactive-symbol-catalog.md`.

pub mod lint;
pub mod schema;
pub mod validator;
