// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! `chromaticity_scene::Scene` → [`crate::MultipassSpec`] adapter
//! (slice 4, gated behind the `scene-adapter` cargo feature).
//!
//! The render scheduler ([`crate::compile_and_render_multipass`])
//! deliberately stays scene-agnostic — it consumes a flat plain-data
//! [`crate::MultipassSpec`] with no awareness of the scene-schema
//! wire format. This module is the bridge: it reads a
//! [`chromaticity_scene::Scene`], walks its
//! [`chromaticity_scene::PassSpec`] list, and produces a
//! [`crate::MultipassSpec`] whose semantics match the slice-2
//! scheduler contract.
//!
//! # Schema → runtime mapping (slice-4 scope)
//!
//! Slice 4's adapter handles the structural skeleton; per-pass
//! GLSL fragment authoring, audio-uniform binding, and
//! transition-edge metadata land in subsequent slices.
//!
//! - `Scene::passes[i].node` (`TARGET` wire key) — surfaced as the
//!   adapted [`crate::PassDef::label`]. The slice-2 scheduler does
//!   not yet route fragment shaders by node ID; the
//!   `node`-keyed lookup is a future-slice concern.
//! - `Scene::passes[i].uniform_bindings` (`UNIFORMS` wire key) —
//!   ADR-011 Phase-2 R-0 bridges these via
//!   [`SceneAdapterOptions::control_values`]: every declared binding
//!   whose NAME appears in the caller's control-value map yields a
//!   scalar [`crate::PassDef::uniforms`] entry (declaration order).
//!   With an empty `control_values` the adapter emits the prior
//!   empty-uniforms list, preserving slice-2 corpus continuity. The
//!   audio-uniform binding slice still owns the symbol-routed audio
//!   uniforms; R-0 only marshals operator/runtime control scalars.
//! - `Scene::passes[i].extensions["FLOAT"]` — when present and
//!   parseable as `true` / `"RGBA32F"` / `"RGBA16F"`, sets
//!   [`crate::PassDef::target_format`] to the corresponding
//!   [`crate::TargetFormat`] variant. Default is
//!   [`crate::TargetFormat::Rgba8Unorm`] preserving slice-2 corpus
//!   continuity. Unrecognized values surface as
//!   [`AdapterError::UnsupportedFloatExtension`].
//! - `Scene::passes[i].extensions["FEEDBACK"]` — when present and
//!   parseable as `true`, sets
//!   [`crate::PassDef::feedback_self`]. Default is `false`.
//! - `Scene::passes[i].extensions["WIDTH"]` /
//!   `extensions["HEIGHT"]` — slice 4 reads them, but folds into
//!   a single shared [`crate::MultipassSpec::target_size`]
//!   (max-of-all-passes) since the scheduler does not yet support
//!   per-pass target sizes. The mismatched-size case surfaces as
//!   [`AdapterError::MixedTargetSizes`] when the per-pass values
//!   diverge AND the caller passes `strict_size = true`. The
//!   non-strict mode preserves the legacy
//!   reference-app-style "every pass renders at the canvas size"
//!   semantic.
//!
//! # Caller surface
//!
//! Two entry points:
//!
//! - [`MultipassSpec::from_scene`] — convenience wrapper with
//!   default options.
//! - [`MultipassSpec::from_scene_with_options`] — explicit
//!   [`SceneAdapterOptions`] for callers that need to set the
//!   target size, frame count, or strict-size mode.
//!
//! Adapter errors convert into [`crate::RenderError::SceneAdapterError`]
//! at the [`crate::compile_and_render_multipass`] boundary so
//! downstream callers see one structured error surface; this module
//! exposes [`AdapterError`] directly for adapter-only consumers
//! (e.g. UI preflight checks).
//!
//! Anchors:
//! - `engine/scene/src/schema.rs` — the source `Scene` /
//!   `PassSpec` / `UniformBinding` types.
//! - `docs/05-scene-system.md` §Canonical Internal Scene Model.
//! - `docs/02-synesthesia-contracts.md` §2 Multipass — `WIDTH`,
//!   `HEIGHT`, `FLOAT`, `WRAP`, `FILTER` per-pass extension keys.
//! - `crate::multipass` module-level doc — slice-2 scheduler
//!   contract this adapter targets.

use chromaticity_scene::schema::{PassSpec, Scene};
use thiserror::Error;

use crate::multipass::{MultipassSpec, PassDef, TargetFormat, UniformInput, UniformValue};

/// Errors returned by the scene → multipass adapter.
///
/// Variants stay narrow in slice 4 — the adapter is intentionally
/// strict in what it accepts so a malformed scene fails fast rather
/// than silently degrading. `#[non_exhaustive]` keeps subsequent
/// slices free to add variants without a major bump on this module's
/// public surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AdapterError {
    /// `Scene::passes` is empty. Slice 4 does not synthesize a
    /// fallback pass; an empty scene rejects.
    #[error("scene has no passes")]
    EmptyScene,

    /// A pass's `extensions["FLOAT"]` carried a value the adapter
    /// could not interpret. Accepted values are `true` (≡
    /// `Rgba16Float`), `"RGBA16F"` / `"RGBA32F"` strings, and
    /// `false` (≡ `Rgba8Unorm`).
    #[error("scene pass {pass_index} extension FLOAT carries unsupported value {value}: {reason}")]
    UnsupportedFloatExtension {
        /// Index into `Scene::passes`.
        pass_index: usize,
        /// Stringified JSON value for diagnostic.
        value: String,
        /// Free-form reason.
        reason: String,
    },

    /// A pass's `extensions["WIDTH"]` / `extensions["HEIGHT"]`
    /// disagreed with another pass's value AND the caller asked
    /// for strict mode. Non-strict mode silently picks the maximum.
    #[error(
        "scene passes carry mixed target sizes (pass {pass_a_index}={size_a:?} vs \
         pass {pass_b_index}={size_b:?}); slice-4 scheduler uses one shared target_size"
    )]
    MixedTargetSizes {
        pass_a_index: usize,
        size_a: (u32, u32),
        pass_b_index: usize,
        size_b: (u32, u32),
    },

    /// A pass referenced a per-pass extension key the adapter does
    /// not yet support. Slice 4's adapter recognizes `FLOAT`,
    /// `FEEDBACK`, `WIDTH`, `HEIGHT`. Slice 5+ widens for `WRAP` /
    /// `FILTER` / sampler controls.
    #[error("scene pass {pass_index} carries unsupported extension key {key}")]
    UnsupportedExtensionKey { pass_index: usize, key: String },

    /// Slice 4 does not yet author per-pass GLSL fragment sources
    /// from the scene graph (the audio-uniform binding slice owns
    /// that). The caller MUST supply a fragment-source map keyed by
    /// pass index. This variant fires when the map is missing the
    /// expected key.
    #[error("scene pass {pass_index} (node={node}) has no fragment source in the supplied map")]
    MissingFragmentSource { pass_index: usize, node: String },
}

/// Caller-supplied options for [`MultipassSpec::from_scene_with_options`].
///
/// Slice-4's adapter does NOT synthesize fragment GLSL from the
/// scene graph — that's the `engine/audio` integration slice. The
/// caller MUST therefore supply [`Self::fragment_sources`] keyed by
/// pass index. This is the slice-4-shape way to cleanly separate
/// "bridge the scene types" from "compile fragment sources" until
/// the next slice unifies them.
#[derive(Debug, Clone)]
pub struct SceneAdapterOptions {
    /// Per-pass fragment GLSL keyed by pass index. Missing keys
    /// surface as [`AdapterError::MissingFragmentSource`].
    pub fragment_sources: Vec<Option<String>>,
    /// Default target size when no pass carries `WIDTH` / `HEIGHT`
    /// extensions. Slice-4's scheduler requires a non-zero size.
    pub default_target_size: (u32, u32),
    /// Number of frames to render. Threaded into
    /// [`crate::MultipassSpec::frame_count`]. Default `1` matches
    /// the slice-2 single-frame semantics.
    pub frame_count: u32,
    /// When `true`, mismatched per-pass `WIDTH` / `HEIGHT` extension
    /// values surface as [`AdapterError::MixedTargetSizes`]. When
    /// `false` (the default), the adapter uses the max-of-all
    /// observed sizes, mirroring the reference-app-style semantic
    /// that all passes render at one canvas size.
    pub strict_size: bool,
    /// Live control values keyed by uniform NAME, injected into every
    /// pass whose `UNIFORMS` block declares a binding of that name.
    ///
    /// ADR-011 Phase-2 R-0 prerequisite: prior slices emitted an
    /// empty [`crate::PassDef::uniforms`] list (the audio-uniform
    /// binding slice owned `UNIFORMS`). R-0 resolves the previously
    /// discarded `_uniform_count` by marshalling the caller's runtime
    /// control values onto each matching pass uniform binding: for
    /// every [`chromaticity_scene::schema::UniformBinding`] in a pass
    /// whose `name` appears here, the adapter pushes a
    /// [`UniformInput`] carrying [`UniformValue::Scalar`].
    ///
    /// Empty (the default) is a no-op — existing callers that relied
    /// on the empty-uniforms shape are unaffected, preserving the
    /// slice-2 corpus continuity.
    pub control_values: Vec<(String, f32)>,
}

impl Default for SceneAdapterOptions {
    fn default() -> Self {
        Self {
            fragment_sources: Vec::new(),
            default_target_size: (16, 16),
            frame_count: 1,
            strict_size: false,
            control_values: Vec::new(),
        }
    }
}

impl MultipassSpec {
    /// Convenience wrapper around
    /// [`Self::from_scene_with_options`] that uses the default
    /// `SceneAdapterOptions` (`default_target_size = (16, 16)`,
    /// `frame_count = 1`, non-strict size mode).
    ///
    /// `fragment_sources` is keyed by pass index; missing entries
    /// must be `None` placeholders so the slice index aligns with
    /// `scene.passes`.
    pub fn from_scene(
        scene: &Scene,
        fragment_sources: Vec<Option<String>>,
    ) -> Result<Self, AdapterError> {
        let options = SceneAdapterOptions {
            fragment_sources,
            ..SceneAdapterOptions::default()
        };
        Self::from_scene_with_options(scene, &options)
    }

    /// Adapt a [`Scene`] to a [`MultipassSpec`] using the supplied
    /// [`SceneAdapterOptions`]. See module-level doc for the
    /// schema → runtime mapping rules.
    pub fn from_scene_with_options(
        scene: &Scene,
        options: &SceneAdapterOptions,
    ) -> Result<Self, AdapterError> {
        if scene.passes.is_empty() {
            return Err(AdapterError::EmptyScene);
        }

        let mut passes = Vec::with_capacity(scene.passes.len());
        let mut observed_sizes: Vec<ObservedSize> = Vec::new();
        let mut size_with_default = options.default_target_size;
        let mut size_seen_explicit = false;

        for (pass_index, pass_spec) in scene.passes.iter().enumerate() {
            // Resolve fragment source from the caller-supplied map.
            let fragment_glsl = options
                .fragment_sources
                .get(pass_index)
                .and_then(|opt| opt.clone())
                .ok_or_else(|| AdapterError::MissingFragmentSource {
                    pass_index,
                    node: pass_spec.node.0.clone(),
                })?;

            let target_format = parse_float_extension(pass_index, pass_spec)?;
            let feedback_self = parse_feedback_extension(pass_index, pass_spec)?;
            let pass_size = parse_size_extension(pass_index, pass_spec)?;
            if let Some(size) = pass_size {
                observed_sizes.push((pass_index, size));
                size_with_default = if !size_seen_explicit {
                    size
                } else {
                    (
                        size_with_default.0.max(size.0),
                        size_with_default.1.max(size.1),
                    )
                };
                size_seen_explicit = true;
            }

            // Slice-4 strict-mode check: any per-pass keys we do
            // NOT yet recognize surface as an explicit error so
            // the caller cannot silently lose semantics.
            for (key, _value) in pass_spec.extensions.iter() {
                let recognized = matches!(key.as_str(), "WIDTH" | "HEIGHT" | "FLOAT" | "FEEDBACK");
                if !recognized {
                    return Err(AdapterError::UnsupportedExtensionKey {
                        pass_index,
                        key: key.clone(),
                    });
                }
            }

            // ADR-011 Phase-2 R-0: bridge the pass `UNIFORMS` block by
            // injecting the caller's live control values. For each
            // declared binding whose NAME appears in
            // `options.control_values`, push a scalar `UniformInput`
            // in binding-declaration order. With an empty
            // `control_values` (the default) this resolves to the
            // prior empty-uniforms shape, so existing callers and the
            // slice-2 corpus are unaffected.
            let mut uniforms: Vec<UniformInput> = Vec::new();
            for binding in &pass_spec.uniform_bindings {
                if let Some((_, value)) = options
                    .control_values
                    .iter()
                    .find(|(name, _)| name == &binding.name)
                {
                    uniforms.push(UniformInput {
                        name: binding.name.clone(),
                        value: UniformValue::Scalar(*value),
                    });
                }
            }

            let label = if pass_spec.node.0.is_empty() {
                Some(format!("scene-adapter/pass-{pass_index}"))
            } else {
                Some(format!(
                    "scene-adapter/pass-{pass_index}/node:{}",
                    pass_spec.node.0
                ))
            };

            passes.push(PassDef {
                label,
                fragment_glsl,
                uniforms,
                input_textures: Vec::new(),
                target_format,
                feedback_self,
            });
        }

        // Strict-size cross-check: when enabled, divergent
        // per-pass sizes are an error. Non-strict mode picks the
        // max (already accumulated in `size_with_default`).
        if options.strict_size {
            if let Some(((idx_a, size_a), (idx_b, size_b))) = first_size_mismatch(&observed_sizes) {
                return Err(AdapterError::MixedTargetSizes {
                    pass_a_index: idx_a,
                    size_a,
                    pass_b_index: idx_b,
                    size_b,
                });
            }
        }

        Ok(MultipassSpec {
            passes,
            target_size: size_with_default,
            frame_count: options.frame_count,
        })
    }
}

/// Read `pass.extensions["FLOAT"]` and map to a [`TargetFormat`].
///
/// Recognized values: bool `true` ⇒ `Rgba16Float`, bool `false` ⇒
/// `Rgba8Unorm`; string `"RGBA16F"` ⇒ `Rgba16Float`, string
/// `"RGBA32F"` ⇒ `Rgba32Float`. Absent key ⇒ `Rgba8Unorm`.
fn parse_float_extension(pass_index: usize, pass: &PassSpec) -> Result<TargetFormat, AdapterError> {
    let Some(value) = pass.extensions.get("FLOAT") else {
        return Ok(TargetFormat::Rgba8Unorm);
    };
    if let Some(b) = value.as_bool() {
        return Ok(if b {
            TargetFormat::Rgba16Float
        } else {
            TargetFormat::Rgba8Unorm
        });
    }
    if let Some(s) = value.as_str() {
        return match s {
            "RGBA8" | "RGBA8UNORM" | "rgba8" => Ok(TargetFormat::Rgba8Unorm),
            "RGBA16F" | "rgba16f" => Ok(TargetFormat::Rgba16Float),
            "RGBA32F" | "rgba32f" => Ok(TargetFormat::Rgba32Float),
            other => Err(AdapterError::UnsupportedFloatExtension {
                pass_index,
                value: format!("\"{other}\""),
                reason: "expected one of RGBA8 / RGBA16F / RGBA32F".to_string(),
            }),
        };
    }
    Err(AdapterError::UnsupportedFloatExtension {
        pass_index,
        value: value.to_string(),
        reason: "FLOAT must be a bool or one of the recognized format strings".to_string(),
    })
}

/// Read `pass.extensions["FEEDBACK"]` and map to a bool.
fn parse_feedback_extension(pass_index: usize, pass: &PassSpec) -> Result<bool, AdapterError> {
    let Some(value) = pass.extensions.get("FEEDBACK") else {
        return Ok(false);
    };
    if let Some(b) = value.as_bool() {
        return Ok(b);
    }
    Err(AdapterError::UnsupportedExtensionKey {
        pass_index,
        // Recycling the variant — slice-4's adapter is strict on
        // FEEDBACK shape, and we don't have a dedicated variant.
        key: format!("FEEDBACK={value}"),
    })
}

/// Read `pass.extensions["WIDTH"]` / `["HEIGHT"]` and return
/// `Some((w, h))` if BOTH are present. A single-key half-spec
/// surfaces as an error.
fn parse_size_extension(
    pass_index: usize,
    pass: &PassSpec,
) -> Result<Option<(u32, u32)>, AdapterError> {
    let w = pass.extensions.get("WIDTH").and_then(|v| v.as_u64());
    let h = pass.extensions.get("HEIGHT").and_then(|v| v.as_u64());
    match (w, h) {
        (Some(w), Some(h)) => {
            // u32 cast: per-pass target sizes are bounded by the
            // adapter's max texture dimension; values exceeding
            // u32::MAX are themselves a structural error.
            let w_u32 = u32::try_from(w).map_err(|_| AdapterError::UnsupportedExtensionKey {
                pass_index,
                key: format!("WIDTH={w} (exceeds u32::MAX)"),
            })?;
            let h_u32 = u32::try_from(h).map_err(|_| AdapterError::UnsupportedExtensionKey {
                pass_index,
                key: format!("HEIGHT={h} (exceeds u32::MAX)"),
            })?;
            Ok(Some((w_u32, h_u32)))
        }
        (None, None) => Ok(None),
        (Some(_), None) => Err(AdapterError::UnsupportedExtensionKey {
            pass_index,
            key: "WIDTH (without matching HEIGHT)".to_string(),
        }),
        (None, Some(_)) => Err(AdapterError::UnsupportedExtensionKey {
            pass_index,
            key: "HEIGHT (without matching WIDTH)".to_string(),
        }),
    }
}

/// `(pass_index, (width, height))` pair as observed during the
/// per-pass extension scan. Aliased to keep
/// [`first_size_mismatch`] under the workspace clippy
/// `type-complexity` ceiling.
type ObservedSize = (usize, (u32, u32));

/// Find the first divergent size pair in the observed sizes list.
fn first_size_mismatch(observed: &[ObservedSize]) -> Option<(ObservedSize, ObservedSize)> {
    let mut iter = observed.iter();
    let first = iter.next()?;
    for entry in iter {
        if entry.1 != first.1 {
            return Some((*first, *entry));
        }
    }
    None
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod adapter_drift_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — scene adapter drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: adapter-
    //! input validation pins that fire WITHOUT a usable wgpu
    //! adapter. The corpus T2 fixture in
    //! `engine/render/tests/scene_adapter.rs` covers the round-trip
    //! through the GPU.
    //!
    //! Pinned invariants:
    //!
    //! - Empty scene rejects with [`AdapterError::EmptyScene`].
    //! - Missing fragment source rejects with
    //!   [`AdapterError::MissingFragmentSource`].
    //! - `FLOAT = true` ⇒ `Rgba16Float`; `FLOAT = "RGBA32F"` ⇒
    //!   `Rgba32Float`; absent ⇒ `Rgba8Unorm` (corpus continuity).
    //! - `FEEDBACK = true` ⇒ `feedback_self = true`.
    //! - Non-strict mode: max-of-all per-pass sizes folds into
    //!   the shared `target_size`.
    //! - Strict mode: divergent sizes reject with
    //!   [`AdapterError::MixedTargetSizes`].
    //! - Unrecognized per-pass extension key rejects with
    //!   [`AdapterError::UnsupportedExtensionKey`].

    use super::{AdapterError, SceneAdapterOptions};
    use chromaticity_scene::schema::{NodeRef, PassSpec, Scene, UniformBinding};
    use serde_json::json;

    use crate::multipass::{MultipassSpec, TargetFormat, UniformValue};

    fn pass_with_extensions(node: &str, extensions: serde_json::Value) -> PassSpec {
        let extensions_map = match extensions {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        PassSpec {
            node: NodeRef(node.to_string()),
            uniform_bindings: Vec::new(),
            extensions: extensions_map,
        }
    }

    /// Build a pass declaring `UNIFORMS` bindings by name (the
    /// `symbol` is irrelevant to the R-0 control-value bridge, which
    /// matches on `name`).
    fn pass_with_uniform_bindings(node: &str, names: &[&str]) -> PassSpec {
        PassSpec {
            node: NodeRef(node.to_string()),
            uniform_bindings: names
                .iter()
                .map(|n| UniformBinding {
                    // Native `chroma_*` namespace: `syn_*` is the Synesthesia
                    // compat surface and must not be invented for new fixtures
                    // (clean-room). The R-0 bridge keys on `name`, not `symbol`.
                    name: (*n).to_string(),
                    symbol: format!("chroma_{}", n.to_uppercase()),
                })
                .collect(),
            extensions: serde_json::Map::new(),
        }
    }

    const TRIVIAL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() { fragColor = vec4(uv, 0.0, 1.0); }
"#;

    #[test]
    fn empty_scene_rejects() {
        let scene = Scene::default();
        let err = MultipassSpec::from_scene(&scene, vec![]).expect_err("empty scene must reject");
        assert!(matches!(err, AdapterError::EmptyScene), "got {err:?}");
    }

    #[test]
    fn missing_fragment_source_rejects() {
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_extensions("node-zero", json!({})));
        let err = MultipassSpec::from_scene(&scene, vec![None])
            .expect_err("missing fragment source must reject");
        match err {
            AdapterError::MissingFragmentSource { pass_index, node } => {
                assert_eq!(pass_index, 0);
                assert_eq!(node, "node-zero");
            }
            other => panic!("expected MissingFragmentSource, got {other:?}"),
        }
    }

    #[test]
    fn float_extension_true_maps_to_rgba16f() {
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_extensions("node", json!({ "FLOAT": true })));
        let spec = MultipassSpec::from_scene(&scene, vec![Some(TRIVIAL_FRAGMENT.to_string())])
            .expect("FLOAT=true must adapt");
        assert_eq!(spec.passes.len(), 1);
        assert_eq!(spec.passes[0].target_format, TargetFormat::Rgba16Float);
    }

    #[test]
    fn float_extension_string_maps_to_rgba32f() {
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_extensions("node", json!({ "FLOAT": "RGBA32F" })));
        let spec = MultipassSpec::from_scene(&scene, vec![Some(TRIVIAL_FRAGMENT.to_string())])
            .expect("FLOAT=RGBA32F must adapt");
        assert_eq!(spec.passes[0].target_format, TargetFormat::Rgba32Float);
    }

    #[test]
    fn float_extension_absent_defaults_rgba8unorm() {
        let mut scene = Scene::default();
        scene.passes.push(pass_with_extensions("node", json!({})));
        let spec = MultipassSpec::from_scene(&scene, vec![Some(TRIVIAL_FRAGMENT.to_string())])
            .expect("absent FLOAT must default");
        assert_eq!(spec.passes[0].target_format, TargetFormat::Rgba8Unorm);
    }

    #[test]
    fn float_extension_unsupported_string_rejects() {
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_extensions("node", json!({ "FLOAT": "RGBA64F" })));
        let err = MultipassSpec::from_scene(&scene, vec![Some(TRIVIAL_FRAGMENT.to_string())])
            .expect_err("unrecognized FLOAT must reject");
        assert!(
            matches!(err, AdapterError::UnsupportedFloatExtension { .. }),
            "got {err:?}",
        );
    }

    #[test]
    fn feedback_extension_true_sets_self_feedback() {
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_extensions("node", json!({ "FEEDBACK": true })));
        let spec = MultipassSpec::from_scene(&scene, vec![Some(TRIVIAL_FRAGMENT.to_string())])
            .expect("FEEDBACK=true must adapt");
        assert!(spec.passes[0].feedback_self);
    }

    #[test]
    fn unsupported_extension_key_rejects() {
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_extensions("node", json!({ "WRAP": "REPEAT" })));
        // WRAP is recognized in scene-schema but slice-4's adapter
        // does not yet route it; should reject.
        let err = MultipassSpec::from_scene(&scene, vec![Some(TRIVIAL_FRAGMENT.to_string())])
            .expect_err("unrecognized adapter key must reject");
        match err {
            AdapterError::UnsupportedExtensionKey { pass_index, key } => {
                assert_eq!(pass_index, 0);
                assert!(key.contains("WRAP"), "got key {key}");
            }
            other => panic!("expected UnsupportedExtensionKey, got {other:?}"),
        }
    }

    #[test]
    fn strict_size_rejects_divergence() {
        let mut scene = Scene::default();
        scene.passes.push(pass_with_extensions(
            "a",
            json!({ "WIDTH": 16, "HEIGHT": 16 }),
        ));
        scene.passes.push(pass_with_extensions(
            "b",
            json!({ "WIDTH": 32, "HEIGHT": 16 }),
        ));
        let options = SceneAdapterOptions {
            fragment_sources: vec![
                Some(TRIVIAL_FRAGMENT.to_string()),
                Some(TRIVIAL_FRAGMENT.to_string()),
            ],
            default_target_size: (16, 16),
            frame_count: 1,
            strict_size: true,
            control_values: Vec::new(),
        };
        let err = MultipassSpec::from_scene_with_options(&scene, &options)
            .expect_err("strict mode must reject mixed sizes");
        assert!(
            matches!(err, AdapterError::MixedTargetSizes { .. }),
            "got {err:?}",
        );
    }

    #[test]
    fn non_strict_size_picks_max() {
        let mut scene = Scene::default();
        scene.passes.push(pass_with_extensions(
            "a",
            json!({ "WIDTH": 16, "HEIGHT": 16 }),
        ));
        scene.passes.push(pass_with_extensions(
            "b",
            json!({ "WIDTH": 32, "HEIGHT": 24 }),
        ));
        let options = SceneAdapterOptions {
            fragment_sources: vec![
                Some(TRIVIAL_FRAGMENT.to_string()),
                Some(TRIVIAL_FRAGMENT.to_string()),
            ],
            default_target_size: (16, 16),
            frame_count: 1,
            strict_size: false,
            control_values: Vec::new(),
        };
        let spec = MultipassSpec::from_scene_with_options(&scene, &options)
            .expect("non-strict mode must accept");
        assert_eq!(spec.target_size, (32, 24));
    }

    #[test]
    fn frame_count_threads_through() {
        let mut scene = Scene::default();
        scene.passes.push(pass_with_extensions("node", json!({})));
        let options = SceneAdapterOptions {
            fragment_sources: vec![Some(TRIVIAL_FRAGMENT.to_string())],
            default_target_size: (16, 16),
            frame_count: 7,
            strict_size: false,
            control_values: Vec::new(),
        };
        let spec = MultipassSpec::from_scene_with_options(&scene, &options)
            .expect("frame_count must thread through");
        assert_eq!(spec.frame_count, 7);
    }

    #[test]
    fn control_values_inject_matching_uniforms() {
        // R-0 drift pin: a control value whose key matches a declared
        // `UNIFORMS` binding name lands as a scalar `UniformInput` on
        // that pass, in binding-declaration order. The non-matching
        // control value (`u_unused`) is silently dropped.
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_uniform_bindings("node", &["u_decay", "u_warp"]));
        let options = SceneAdapterOptions {
            fragment_sources: vec![Some(TRIVIAL_FRAGMENT.to_string())],
            control_values: vec![
                ("u_warp".to_string(), 0.75_f32),
                ("u_decay".to_string(), 0.5_f32),
                ("u_unused".to_string(), 9.0_f32),
            ],
            ..SceneAdapterOptions::default()
        };
        let spec = MultipassSpec::from_scene_with_options(&scene, &options)
            .expect("control-value injection must adapt");
        assert_eq!(spec.passes.len(), 1);
        let uniforms = &spec.passes[0].uniforms;
        // Two of the three control values match a declared binding;
        // emitted in binding-declaration order (`u_decay` then
        // `u_warp`), NOT control-value order.
        assert_eq!(uniforms.len(), 2, "only matching bindings inject");
        assert_eq!(uniforms[0].name, "u_decay");
        assert!(
            matches!(uniforms[0].value, UniformValue::Scalar(v) if (v - 0.5).abs() < f32::EPSILON),
            "got {:?}",
            uniforms[0].value,
        );
        assert_eq!(uniforms[1].name, "u_warp");
        assert!(
            matches!(uniforms[1].value, UniformValue::Scalar(v) if (v - 0.75).abs() < f32::EPSILON),
            "got {:?}",
            uniforms[1].value,
        );
    }

    #[test]
    fn empty_control_values_yield_no_uniforms() {
        // R-0 drift pin: with `control_values` empty (the default),
        // the adapter emits the prior empty-uniforms shape even when
        // the pass DECLARES `UNIFORMS` bindings — preserving slice-2
        // corpus continuity and the existing-caller contract.
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with_uniform_bindings("node", &["u_decay", "u_warp"]));
        let options = SceneAdapterOptions {
            fragment_sources: vec![Some(TRIVIAL_FRAGMENT.to_string())],
            ..SceneAdapterOptions::default()
        };
        assert!(
            options.control_values.is_empty(),
            "default control_values must be empty",
        );
        let spec = MultipassSpec::from_scene_with_options(&scene, &options)
            .expect("empty control values must adapt");
        assert!(
            spec.passes[0].uniforms.is_empty(),
            "empty control_values must be a no-op (no injected uniforms)",
        );
    }
}
