// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene loader — slice-3a JSON deserializer surface.
//!
//! Parses the `scene.json` wire format defined by
//! `docs/02-synesthesia-contracts.md` §Observed root keys into the
//! canonical-model types in [`crate::schema`]. The wire format is
//! now frozen against this doc section; non-additive changes require
//! a compat-profile bump per
//! `docs/22-compatibility-versioning-policy.md` §Version Axes.
//!
//! Slice-3a scope: in-memory deserialization only
//! ([`load_scene_json`] over `impl Read`, [`parse_scene_json`] over
//! `&str`). On-disk path helpers, `.chromaPack` zip transport,
//! `script.js` / `main.glsl` pairing, and the `DATA_SOURCES` stanza
//! interpretation all remain deferred to later slices. The first
//! external reader for [`crate::schema`] field visibility is the
//! T2 on-disk corpus harness that C-α2 will land at
//! `engine/scene/tests/validator/main.rs`; see
//! `docs/26-evidence-artifact-registry.md` row 23
//! (`EV-SCENE-IMPORT`).

use crate::schema::Scene;

/// Errors returned by the slice-3a JSON loader surface.
///
/// [`Self::UnknownControlType`] is reserved for a later slice's
/// control-type emission path. At slice-3a, unknown control types
/// surface through [`Self::Json`] because the custom
/// [`crate::schema::ControlKind`] `Deserialize` impl produces a
/// `serde::de::Error::custom("unknown-control-type: <raw>")` — this
/// variant exists in the signature so the C-α2 harness can route
/// the raised error into a classified diagnostic without a
/// compat-surface change.
#[non_exhaustive]
#[derive(Debug)]
pub enum LoadError {
    /// I/O failure reading the scene source. Only [`load_scene_json`]
    /// can produce this variant.
    Io(std::io::Error),
    /// JSON shape-level failure surfaced by [`serde_json`], including
    /// the slice-3a spelling of unknown control types (see the type
    /// doc).
    Json(serde_json::Error),
    /// Reserved for future slices. Slice-3a does not emit this
    /// variant; it exists on the non-exhaustive signature so the
    /// C-α2 harness can introduce the classified spelling without a
    /// compat-surface change.
    UnknownControlType {
        /// Author-declared control name.
        control_name: String,
        /// Raw `TYPE` wire value as spelled in the source.
        raw: String,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "scene loader I/O error: {err}"),
            LoadError::Json(err) => write!(f, "scene loader JSON error: {err}"),
            LoadError::UnknownControlType { control_name, raw } => {
                write!(f, "unknown control type for {control_name:?}: {raw:?}")
            }
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(err) => Some(err),
            LoadError::Json(err) => Some(err),
            LoadError::UnknownControlType { .. } => None,
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(err: std::io::Error) -> Self {
        LoadError::Io(err)
    }
}

impl From<serde_json::Error> for LoadError {
    fn from(err: serde_json::Error) -> Self {
        LoadError::Json(err)
    }
}

/// Parse a `scene.json` stream from any [`std::io::Read`] source.
pub fn load_scene_json(reader: impl std::io::Read) -> Result<Scene, LoadError> {
    Ok(serde_json::from_reader(reader)?)
}

/// Parse a `scene.json` string into the canonical [`Scene`] model.
pub fn parse_scene_json(s: &str) -> Result<Scene, LoadError> {
    Ok(serde_json::from_str(s)?)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod loader_drift_pins {
    //! Evidence: EV-SCENE-IMPORT (T1) — loader-surface drift pins
    //! against `docs/02-synesthesia-contracts.md` §Observed root keys,
    //! §Observed control keys, §2 Multipass, and
    //! `docs/05-scene-system.md` §Normalization Rules.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! loader surface (error variant shape, root-key flatten
    //! behaviour, CONTROLS round-trip, PASSES round-trip, CP-01
    //! missing-TYPE ingestion) against a minimal in-memory
    //! deserializer exercise. Drift in any pinned wire-key spelling,
    //! extensions-absorption behaviour, or `ControlKind` first-token
    //! split rule fails at build.
    //!
    //! Phase-1 exit still requires T2 (conformance over the baseline
    //! scene corpus); the T2 on-disk corpus harness lands with C-α2
    //! at `engine/scene/tests/validator/main.rs`.

    use super::{LoadError, parse_scene_json};
    use crate::schema::{ControlKind, ControlValue};

    #[test]
    fn empty_string_is_json_error() {
        let err = parse_scene_json("").expect_err("empty input must fail");
        assert!(
            matches!(err, LoadError::Json(_)),
            "expected LoadError::Json, got {err:?}",
        );
    }

    #[test]
    fn empty_object_parses_to_default_like_scene() {
        let scene = parse_scene_json("{}").expect("empty object must parse");
        assert!(scene.controls.is_empty());
        assert!(scene.passes.is_empty());
        assert!(scene.extensions.is_empty());
        // `SCHEMA_VERSION` defaults to 1 per compat-syn-v1 baseline.
        assert_eq!(scene.header.schema_version, 1);
    }

    #[test]
    fn schema_version_round_trips_when_present() {
        let scene =
            parse_scene_json(r#"{"SCHEMA_VERSION": 7}"#).expect("valid SCHEMA_VERSION must parse");
        assert_eq!(scene.header.schema_version, 7);
    }

    #[test]
    fn unknown_root_keys_land_in_extensions() {
        // Author-observed GPU + TAGS survive into Scene.extensions
        // per docs/05:37 + docs/28 §CP-12.
        let scene = parse_scene_json(
            r#"{
                "GPU": 2,
                "TAGS": ["glitch", "colorful"],
                "TITLE": "demo",
                "DATA_SOURCES": [{"NAME": "foo"}]
            }"#,
        )
        .expect("unknown root keys must flatten");
        assert!(scene.extensions.contains_key("GPU"));
        assert!(scene.extensions.contains_key("TAGS"));
        assert!(scene.extensions.contains_key("TITLE"));
        // CP-12: parse-reserve, do not interpret.
        assert!(scene.extensions.contains_key("DATA_SOURCES"));
    }

    #[test]
    fn controls_stanza_maps_wire_keys_to_canonical_fields() {
        let scene = parse_scene_json(
            r#"{
                "CONTROLS": [
                    {
                        "NAME": "brightness",
                        "TYPE": "slider",
                        "DEFAULT": 0.5,
                        "MIN": 0.0,
                        "MAX": 1.0
                    }
                ]
            }"#,
        )
        .expect("canonical CONTROLS entry must parse");
        assert_eq!(scene.controls.len(), 1);
        let c = &scene.controls[0];
        assert_eq!(c.name, "brightness");
        assert_eq!(c.kind, Some(ControlKind::Slider));
        assert_eq!(c.default, ControlValue::Float(0.5));
        assert_eq!(c.min, Some(0.0));
        assert_eq!(c.max, Some(1.0));
    }

    #[test]
    fn control_type_first_token_split_accepts_compat_variants() {
        // docs/02 §Observed control types lines 31-33: reference-app
        // scenes spell `slider smooth`, `xy traveler`, etc. The
        // slice-3a parser splits on ASCII whitespace and keeps the
        // canonical base kind.
        let scene = parse_scene_json(
            r#"{
                "CONTROLS": [
                    {"NAME": "a", "TYPE": "slider smooth", "DEFAULT": 0.0},
                    {"NAME": "b", "TYPE": "xy traveler", "DEFAULT": [0.0, 0.0]},
                    {"NAME": "c", "TYPE": "toggle", "DEFAULT": true}
                ]
            }"#,
        )
        .expect("compat TYPE spellings must parse");
        assert_eq!(scene.controls[0].kind, Some(ControlKind::Slider));
        assert_eq!(scene.controls[1].kind, Some(ControlKind::Xy));
        assert_eq!(scene.controls[2].kind, Some(ControlKind::Toggle));
    }

    #[test]
    fn unknown_control_type_raises_json_error() {
        // Slice-3a emits the "unknown-control-type: <raw>" spelling
        // through `serde::de::Error::custom`; a later slice will
        // route this through `LoadError::UnknownControlType`.
        let err = parse_scene_json(
            r#"{
                "CONTROLS": [{"NAME": "a", "TYPE": "nonsense", "DEFAULT": 0.0}]
            }"#,
        )
        .expect_err("unknown TYPE must fail");
        match &err {
            LoadError::Json(je) => assert!(
                je.to_string().contains("unknown-control-type"),
                "expected unknown-control-type message, got {je}",
            ),
            other => panic!("expected LoadError::Json, got {other:?}"),
        }
    }

    #[test]
    fn control_default_untagged_discriminates_by_shape() {
        // ControlValue variant order: Bool before Float before
        // VecN. Pins the untagged-serde ordering invariant.
        let scene = parse_scene_json(
            r#"{
                "CONTROLS": [
                    {"NAME": "t", "TYPE": "toggle", "DEFAULT": true},
                    {"NAME": "s", "TYPE": "slider", "DEFAULT": 0.5},
                    {"NAME": "p", "TYPE": "xy",     "DEFAULT": [0.1, 0.2]},
                    {"NAME": "c", "TYPE": "color",  "DEFAULT": [0.1, 0.2, 0.3]},
                    {"NAME": "a", "TYPE": "color",  "DEFAULT": [0.1, 0.2, 0.3, 1.0]}
                ]
            }"#,
        )
        .expect("mixed-shape defaults must parse");
        assert_eq!(scene.controls[0].default, ControlValue::Bool(true));
        assert_eq!(scene.controls[1].default, ControlValue::Float(0.5));
        assert_eq!(scene.controls[2].default, ControlValue::Vec2([0.1, 0.2]));
        assert_eq!(
            scene.controls[3].default,
            ControlValue::Vec3([0.1, 0.2, 0.3]),
        );
        assert_eq!(
            scene.controls[4].default,
            ControlValue::Vec4([0.1, 0.2, 0.3, 1.0]),
        );
    }

    #[test]
    fn passes_stanza_maps_target_and_flattens_pass_options() {
        // docs/02 §2: PASSES entries carry TARGET plus WIDTH / HEIGHT
        // / FLOAT / WRAP / FILTER. Slice-3a maps TARGET to
        // PassSpec.node and flattens the rest into
        // PassSpec.extensions.
        let scene = parse_scene_json(
            r#"{
                "PASSES": [
                    {"TARGET": "buffA", "WIDTH": 1920, "HEIGHT": 1080, "FLOAT": true},
                    {"TARGET": "buffB", "WRAP": "clamp"}
                ]
            }"#,
        )
        .expect("canonical PASSES must parse");
        assert_eq!(scene.passes.len(), 2);
        assert_eq!(scene.passes[0].node.0, "buffA");
        assert!(scene.passes[0].extensions.contains_key("WIDTH"));
        assert!(scene.passes[0].extensions.contains_key("HEIGHT"));
        assert!(scene.passes[0].extensions.contains_key("FLOAT"));
        assert_eq!(scene.passes[1].node.0, "buffB");
        assert!(scene.passes[1].extensions.contains_key("WRAP"));
        // UNIFORMS is a canonical-model field; wire does not carry
        // it per-pass on reference scenes.
        assert!(scene.passes[0].uniform_bindings.is_empty());
    }

    #[test]
    fn optional_control_fields_default_when_absent() {
        // `docs/02` §Observed control keys: MIN / MAX / TYPE are
        // optional in the wire format.
        let scene = parse_scene_json(
            r#"{
                "CONTROLS": [{"NAME": "x", "DEFAULT": 0.0}]
            }"#,
        )
        .expect("minimal CONTROLS entry must parse");
        let c = &scene.controls[0];
        assert_eq!(c.name, "x");
        assert!(c.kind.is_none());
        assert_eq!(c.default, ControlValue::Float(0.0));
        assert!(c.min.is_none());
        assert!(c.max.is_none());
    }
}
