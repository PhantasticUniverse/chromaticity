// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene schema — canonical-model types with the slice-3a frozen
//! `scene.json` wire surface.
//!
//! Slice-3a freezes the scene-schema wire format against
//! `docs/02-synesthesia-contracts.md` §Observed root keys (lines
//! 19–22), §Observed control keys (lines 26–29), and §2 Multipass,
//! together with `docs/05-scene-system.md` §Canonical Internal Scene
//! Model, §Normalization Rules, and §Loader Modes. Every
//! public-schema type now carries the `#[serde(rename = "…")]` wire
//! attributes named by those sections, every field is `pub` (the
//! slice-2 `pub(crate)` visibility was the slice-boundary signal this
//! slice realizes), and unknown top-level keys survive through a
//! flatten-absorbed `extensions` map per
//! `docs/28-implementation-contract-pack.md` §CP-12 parse-reserve rule.
//!
//! Once this slice lands, further non-additive changes to the shape
//! of any type in this module require a `compat!` profile bump per
//! `docs/22-compatibility-versioning-policy.md` §Version Axes
//! scene-schema row. Additive changes (new optional wire keys,
//! flatten-preserved extensions) remain compat-safe.
//!
//! Slice-3a does NOT land:
//!
//! - Structured parsing of `HARD_TRANSITIONS` / `SMOOTH_TRANSITIONS`
//!   (flatten-preserved in `Scene::extensions`).
//! - `DATA_SOURCES` interpretation (parse-reserved per CP-12).
//! - `.chromaPack` zip transport, `script.js` / `main.glsl` pairing,
//!   `images/` handling.
//! - The full `docs/36-reactive-symbol-catalog.md` loader.
//!
//! Governing ADRs:
//!
//! - ADR-007 — scene warm-switch policy
//!   (`docs/adr/ADR-007-scene-warm-switch-policy.md`).
//! - ADR-008 — transition and traveler semantics
//!   (`docs/adr/ADR-008-transition-and-traveler-semantics.md`).
//!
//! Symbol source of truth: `docs/36-reactive-symbol-catalog.md`.

use serde::{Deserialize, Deserializer, Serialize};

/// In-memory scene representation.
///
/// Slice-3a wire mapping:
///
/// - [`SceneHeader`] sub-fields (`SCHEMA_VERSION`) are promoted to
///   top-level wire keys via `#[serde(flatten)]`.
/// - `CONTROLS` → [`Self::controls`].
/// - `PASSES` → [`Self::passes`].
/// - Every other top-level wire key — `GPU`, `TAGS`, `TITLE`,
///   `CREDIT`, `DESCRIPTION`, `IMAGE_PATH`, `WIDTH`, `HEIGHT`,
///   `MEDIA`, `IMAGES`, `HARD_TRANSITIONS`, `SMOOTH_TRANSITIONS`,
///   legacy `IMAGE` / `CATEGORIES`, and reserved `DATA_SOURCES` —
///   flattens into [`Self::extensions`]. See `docs/05:37`
///   ("Preserve `GPU`, `TAGS`, and legacy/custom metadata in
///   `scene.extensions`") and `docs/28 §CP-12` (parse-reserve, don't
///   interpret `DATA_SOURCES` at slice-3a).
///
/// The normalized transition descriptors, media bindings, and
/// script-lifecycle metadata called out in
/// `docs/05-scene-system.md` §Canonical Internal Scene Model land in
/// subsequent slices; for slice-3a they ride along via `extensions`.
/// `dead_code` stays allowed because the normalizer that reads the
/// populated fields lands in a later slice.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    #[serde(flatten)]
    pub header: SceneHeader,
    #[serde(rename = "CONTROLS", default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<ControlDecl>,
    #[serde(rename = "PASSES", default, skip_serializing_if = "Vec::is_empty")]
    pub passes: Vec<PassSpec>,
    /// Unknown or post-MVP root keys flattened through for
    /// preservation per `docs/05:37` + `docs/28 §CP-12`. Includes the
    /// observed roots `GPU`, `TAGS`, `TITLE`, `CREDIT`,
    /// `DESCRIPTION`, `IMAGE_PATH`, `WIDTH`, `HEIGHT`, `MEDIA`,
    /// `IMAGES`, `HARD_TRANSITIONS`, `SMOOTH_TRANSITIONS`, legacy
    /// `IMAGE` / `CATEGORIES`, and the Chromaticity-reserved
    /// `DATA_SOURCES` stanza (parse-reserve only at slice-3a).
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Scene header carrying version and identity metadata.
///
/// `schema_version` wire-maps to the Chromaticity-reserved
/// top-level `SCHEMA_VERSION` key (absent from reference scenes, so
/// safe to claim). When missing, defaults to `1` — the
/// compat-syn-v1 baseline per
/// `docs/22-compatibility-versioning-policy.md` §Version Axes.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHeader {
    #[serde(rename = "SCHEMA_VERSION", default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

/// Scene-graph node reference.
///
/// Thin newtype around the author-declared node identifier. The
/// validator enforces uniqueness across a scene's `passes`; the
/// canonical-model normalizer in a later slice will additionally
/// resolve forward references against the full node graph.
///
/// The tuple payload is `pub` (already), so the `TARGET` wire string
/// round-trips through the derived `Deserialize` as a plain JSON
/// string — the newtype is transparent to serde.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeRef(pub String);

/// Uniform-binding declaration inside a [`PassSpec`].
///
/// `name` is the scene-author-declared binding label (free-form,
/// uniqueness not enforced at slice-3a). `symbol` is the reactive-
/// catalog symbol the binding publishes into the shader — validated
/// against the representative subset of
/// `docs/36-reactive-symbol-catalog.md` loaded by
/// [`crate::validator`]. A later slice widens to the full catalog.
///
/// # Wire spelling
///
/// Wire spelling is a canonical-model concern at slice-3a. The
/// observed reference-app wire format uses `UNIFORMS` only inside
/// `HARD_TRANSITIONS` / `SMOOTH_TRANSITIONS` entries as an array of
/// control-name strings (`docs/02` §15.1, §15.2) — it does NOT
/// appear per-pass. This struct therefore only round-trips through
/// a Chromaticity-native on-disk scene that chooses to emit
/// per-pass bindings; it is never populated from a reference-app
/// `.synScene`. Field names `NAME` / `SYMBOL` are the canonical-
/// model spelling.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniformBinding {
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "SYMBOL")]
    pub symbol: String,
}

/// Single render-pass specification.
///
/// Slice-3a wire mapping:
///
/// - `TARGET` → [`Self::node`] (round-trips through the [`NodeRef`]
///   newtype's derived `Deserialize`, which accepts a bare string).
/// - `UNIFORMS` → [`Self::uniform_bindings`] (canonical-model only;
///   reference-app wire does not carry this per-pass, so ingest
///   from a `.synScene` leaves it empty).
/// - Every other per-pass wire key — `WIDTH`, `HEIGHT`, `FLOAT`,
///   `WRAP`, `FILTER` per `docs/02` §2 — flattens into
///   [`Self::extensions`].
///
/// Pass-target format / precision / feedback-edge mode interpretation
/// lands in a later slice; slice-3a's job is to get the shape
/// flattened through without loss.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassSpec {
    #[serde(rename = "TARGET", default)]
    pub node: NodeRef,
    #[serde(rename = "UNIFORMS", default, skip_serializing_if = "Vec::is_empty")]
    pub uniform_bindings: Vec<UniformBinding>,
    /// Flatten-preserved per-pass keys (`WIDTH`, `HEIGHT`, `FLOAT`,
    /// `WRAP`, `FILTER`) per `docs/02` §2. Structured parsing lands
    /// in a later slice.
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

/// Author-declared control taxonomy per
/// `docs/28-implementation-contract-pack.md` §CP-01 CR-01.
///
/// Slice-3a covers the four parity-core kinds the CP-01 missing-TYPE
/// fallback rule lists (`slider`, `toggle`, `color`, `xy`). Future
/// kinds (e.g. native `enum` controls under `native-chroma-v1`) land
/// with the profile surface that introduces them.
///
/// The custom [`Deserialize`] impl applies the "first ASCII-
/// whitespace-split token" rule: reference-app scenes spell things
/// like `slider smooth`, `xy traveler`, `bang counter smooth` in
/// `TYPE` (`docs/02` §Observed control types lines 31–33), and the
/// slice-3a parser preserves only the canonical base kind. Unknown
/// first tokens produce a `serde::de::Error` carrying the raw
/// spelling.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
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

impl<'de> Deserialize<'de> for ControlKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let first = raw.split_ascii_whitespace().next().unwrap_or("");
        match first.to_ascii_lowercase().as_str() {
            "slider" => Ok(ControlKind::Slider),
            "toggle" => Ok(ControlKind::Toggle),
            "color" => Ok(ControlKind::Color),
            "xy" => Ok(ControlKind::Xy),
            _ => Err(serde::de::Error::custom(format!(
                "unknown-control-type: {raw}"
            ))),
        }
    }
}

/// Canonical control default-value shape.
///
/// Separate variants exist so the CP-01 coercion rule can inspect
/// default shape without a bare `f64` ambiguity. The
/// `#[serde(untagged)]` encoding discriminates variants by JSON
/// shape; ORDER matters and is pinned here:
///
/// 1. [`Self::Bool`] — matches JSON `true` / `false` so boolean
///    toggles don't fall through to the numeric path.
/// 2. [`Self::Float`] — matches JSON numbers.
/// 3. [`Self::Vec2`] / [`Self::Vec3`] / [`Self::Vec4`] — serde's
///    untagged array handling discriminates these by length.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(untagged)]
pub enum ControlValue {
    /// Boolean spelling of a toggle default.
    Bool(bool),
    /// Scalar float (slider default or toggle numeric spelling).
    Float(f64),
    /// 2-float tuple (xy default).
    Vec2([f64; 2]),
    /// 3-float tuple (color without alpha).
    Vec3([f64; 3]),
    /// 4-float tuple (color with alpha / `DEF_COLOR`).
    Vec4([f64; 4]),
}

impl Default for ControlValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Canonical control declaration.
///
/// Slice-3a wire mapping per `docs/02` §Observed control keys:
///
/// - `NAME` → [`Self::name`].
/// - `TYPE` → [`Self::kind`] (optional; [`None`] triggers the CP-01
///   missing-TYPE fallback in [`crate::validator`]).
/// - `DEFAULT` → [`Self::default`].
/// - `MIN` → [`Self::min`] (optional).
/// - `MAX` → [`Self::max`] (optional).
///
/// Additional observed keys (`UI_GROUP`, `DESCRIPTION`, `PARAMS`,
/// `DEF_COLOR`, `LABELS`, `VALUES`, `IS_META`, legacy `LABEL`) are
/// not yet mapped to canonical fields and are dropped by the
/// `Deserialize` derive at slice-3a. They become flattened
/// extensions in the slice that introduces them to the canonical
/// model.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlDecl {
    #[serde(rename = "NAME")]
    pub name: String,
    /// `None` means the incoming control object had no `TYPE` key;
    /// the validator either coerces to one of [`ControlKind`] or
    /// fails with [`crate::validator::ValidationError::MissingControlType`].
    #[serde(rename = "TYPE", default)]
    pub kind: Option<ControlKind>,
    #[serde(rename = "DEFAULT", default)]
    pub default: ControlValue,
    #[serde(rename = "MIN", default)]
    pub min: Option<f64>,
    #[serde(rename = "MAX", default)]
    pub max: Option<f64>,
}
