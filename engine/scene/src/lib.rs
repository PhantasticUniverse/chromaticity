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
//! - **Slice 2** — real validator, CP-01 coercion, typed control
//!   declarations, serde derives (derive-only). Schema fields
//!   remained `pub(crate)`; no wire-format freeze.
//! - **Slice 3a (this PR)** — `scene.json` wire-format freeze against
//!   `docs/02-synesthesia-contracts.md` §Observed root keys + §Observed
//!   control keys + §2 Multipass, schema field visibility widened from
//!   `pub(crate)` to `pub`, and a minimal `serde_json`-backed loader
//!   ([`loader::parse_scene_json`] / [`loader::load_scene_json`]). Per
//!   `docs/22-compatibility-versioning-policy.md` §Version Axes
//!   scene-schema row, further non-additive wire changes now require a
//!   `compat!` profile bump.
//! - **Slice 4 (next)** — structured parsing of `HARD_TRANSITIONS` /
//!   `SMOOTH_TRANSITIONS` per `docs/02` §15, `.chromaPack` zip transport,
//!   `script.js` / `main.glsl` / `images/` pairing, the full
//!   `docs/36-reactive-symbol-catalog.md` loader, and the scene-lint
//!   harness emitting the [`lint::Warning`] taxonomy outside the
//!   validate path.
//!
//! Produces evidence for `EV-SCENE-IMPORT` (T0 placeholder retained
//! in `validator::ctor_placeholder`; T1 drift pins in
//! `validator::drift_pins` and `loader::loader_drift_pins`, the
//! latter added in slice-3a ahead of the C-α2 T2 on-disk corpus
//! harness that advances `docs/26` row 23 past `pending:artifact`)
//! and `EV-SCHEMA-LINT` (T0 variant constructibility in
//! `lint::drift_pins`).
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
pub mod loader;
pub mod observability;
pub mod schema;
pub mod validator;
