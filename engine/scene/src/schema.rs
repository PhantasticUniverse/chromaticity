// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene schema — slice-2 canonical-model types.
//!
//! Slice-2 grows the scene-schema surface from the slice-1 placeholder
//! stubs into the canonical-model topology that
//! `docs/05-scene-system.md` §Canonical Internal Scene Model and
//! `docs/28-implementation-contract-pack.md` §CP-01 name: typed
//! [`ControlDecl`] declarations, [`NodeRef`] scene-graph node
//! references, and [`PassSpec`] render-pass descriptors carrying
//! author-declared [`UniformBinding`] symbol references.
//!
//! `serde::Serialize` / `serde::Deserialize` are derived on every
//! public-schema type. The derive-only `serde` dependency is the
//! only new crate this slice adds; `serde_json` and any on-disk
//! `.chroma` wire format are **NOT** part of slice-2. The wire
//! format freezes with the slice-3 deserializer, which also widens
//! field visibility and opens the schema to external callers.
//!
//! Every field in every public-schema type remains `pub(crate)` by
//! design. The schema is not yet frozen — renames, field-set
//! changes, and coercion-taxonomy widening are all permitted
//! between slice-2 and slice-3 without a `compat!` commit. Once
//! slice-3 lands the first external reader, field visibility opens
//! and further non-additive changes require a compat-profile bump.
//!
//! Governing ADRs:
//!
//! - ADR-007 — scene warm-switch policy
//!   (`docs/adr/ADR-007-scene-warm-switch-policy.md`).
//! - ADR-008 — transition and traveler semantics
//!   (`docs/adr/ADR-008-transition-and-traveler-semantics.md`).
//!
//! Symbol source of truth: `docs/36-reactive-symbol-catalog.md`.

use serde::{Deserialize, Serialize};

/// In-memory scene representation.
///
/// Slice-2 surface: a header, an ordered `controls` list of typed
/// [`ControlDecl`] declarations, and an ordered `passes` list. The
/// normalized transition descriptors, media bindings, and
/// script-lifecycle metadata called out in
/// `docs/05-scene-system.md` §Canonical Internal Scene Model land
/// in subsequent slices. `dead_code` stays allowed because the
/// normalizer that reads the populated fields lands in slice-3.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub(crate) header: SceneHeader,
    pub(crate) controls: Vec<ControlDecl>,
    pub(crate) passes: Vec<PassSpec>,
}

/// Scene header carrying version and identity metadata.
///
/// `schema_version` is the only field populated in the first slice;
/// it exists so the validator has something non-trivial to branch
/// on once the schema freeze begins.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct SceneHeader {
    pub(crate) schema_version: u32,
}

/// Scene-graph node reference.
///
/// Thin newtype around the author-declared node identifier. The
/// validator enforces uniqueness across a scene's `passes`; the
/// canonical-model normalizer in slice-3 will additionally resolve
/// forward references against the full node graph.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeRef(pub String);

/// Uniform-binding declaration inside a [`PassSpec`].
///
/// `name` is the scene-author-declared binding label (free-form,
/// uniqueness not enforced at slice-2). `symbol` is the reactive-
/// catalog symbol the binding publishes into the shader — validated
/// against the representative subset of
/// `docs/36-reactive-symbol-catalog.md` loaded by
/// [`crate::validator`]. Slice-3 widens to the full catalog.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniformBinding {
    pub(crate) name: String,
    pub(crate) symbol: String,
}

/// Single render-pass specification.
///
/// Slice-2 surface: the pass's scene-graph [`NodeRef`] plus its
/// author-declared [`UniformBinding`] list. Pass-target format,
/// precision, feedback-edge mode, and transition hooks land in
/// slice-3 alongside the `.chroma` wire-format freeze.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct PassSpec {
    pub(crate) node: NodeRef,
    pub(crate) uniform_bindings: Vec<UniformBinding>,
}

/// Author-declared control taxonomy per
/// `docs/28-implementation-contract-pack.md` §CP-01 CR-01.
///
/// Slice-2 covers the four parity-core kinds the CP-01 missing-TYPE
/// fallback rule lists (`slider`, `toggle`, `color`, `xy`). Future
/// kinds (e.g. native `enum` controls under `native-chroma-v1`) land
/// with the profile surface that introduces them.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ControlKind {
    /// Numeric scalar control bounded by `min` / `max`.
    Slider,
    /// Boolean or `{0, 1}` toggle.
    Toggle,
    /// 3- or 4-component color tuple.
    Color,
    /// 2-component 2-D position.
    Xy,
}

/// Canonical control default-value shape.
///
/// Separate variants exist so the slice-2 CP-01 coercion rule can
/// inspect default shape without a bare `f64` ambiguity. The wire
/// encoding is not yet fixed; slice-3 deserializer decides how
/// JSON scalars / arrays map onto these variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ControlValue {
    /// Scalar float (slider default or toggle numeric spelling).
    Float(f64),
    /// 2-float tuple (xy default).
    Vec2([f64; 2]),
    /// 3-float tuple (color without alpha).
    Vec3([f64; 3]),
    /// 4-float tuple (color with alpha / `DEF_COLOR`).
    Vec4([f64; 4]),
    /// Boolean spelling of a toggle default.
    Bool(bool),
}

impl Default for ControlValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Canonical control declaration.
///
/// `kind` may be `None` on ingest; the validator runs the CP-01
/// missing-TYPE fallback rule (slider → toggle → color → xy →
/// strict-reject) and either coerces (compat mode) or rejects
/// (strict mode) per `docs/28` §CP-01.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ControlDecl {
    pub(crate) name: String,
    /// `None` means the incoming control object had no `TYPE` key;
    /// the validator either coerces to one of [`ControlKind`] or
    /// fails with [`crate::validator::ValidationError::MissingControlType`].
    pub(crate) kind: Option<ControlKind>,
    pub(crate) default: ControlValue,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
}
