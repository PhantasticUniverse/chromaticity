// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Cold-vs-warm shader-compile cache + instrumentation surface for the
//! Phase-4d-exit `EV-SHADER-CACHE` evidence row (row 79 of
//! `docs/26-evidence-artifact-registry.md`).
//!
//! This module is the minimal cache wrapper sufficient to MEASURE
//! cold-vs-warm shader-compile latency through a queryable surface.
//! The companion criterion bench harness lives at
//! `engine/render/benches/shader_cache.rs` and consumes this module
//! across two iteration shapes:
//!
//! - **Cold compile** — each iteration constructs a fresh
//!   [`ShaderCache`] and calls
//!   [`ShaderCache::compile_or_get_module`]. The first call for any
//!   given [`ShaderId`] records a metric with `was_cold = true` and
//!   pays the full naga GLSL→IR→validate cost.
//! - **Warm compile** — the bench prewarms the cache once outside
//!   the measurement closure, then each iteration calls
//!   [`ShaderCache::compile_or_get_module`] against the same
//!   [`ShaderId`]. The metric records `was_cold = false` and the
//!   call returns the cached module without re-running the naga
//!   frontend.
//!
//! # docs/10 budget anchors
//!
//! `docs/10-performance-latency-budgets.md` §Shader Cache pins:
//!
//! - cold-start shader compile: median <= 800 ms / p95 <= 1800 ms
//!   per shader program;
//! - warm-hit shader load: p95 <= 40 ms per program.
//!
//! Those are the hard budgets `EV-SHADER-CACHE` validates. This
//! module's harness produces the underlying timing series; assertion
//! against the budgets is OUT OF SCOPE for the foundational PR and
//! lands in a follow-up `chore(release-eng)` PR that extends
//! `scripts/budget-check.sh` (the existing helper hard-codes the 9
//! `EV-PERF-BUDGETS` proxy keys against the docs/10 "command + state
//! updates: <= 1.25 ms" budget; shader-cache cold/warm budgets sit on
//! a different scale and need their own threshold table).
//!
//! # Why "shader id" is a caller-supplied string, not a content hash
//!
//! Mirroring wgpu's own `ShaderModuleDescriptor::label` posture:
//! identity is what the caller declares it to be. A scene author who
//! re-imports the same `.glsl` after a no-op edit has the same
//! conceptual shader; a scene author who edits the source MUST re-
//! key the cache (the cold-vs-warm benchmark below treats
//! distinct-source-same-id as a programmer error and is not in
//! scope for this row's harness). The downstream renderer can
//! choose to hash the source itself when constructing
//! [`ShaderId`]s; this module is intentionally agnostic.
//!
//! # Why not push instrumentation into wgpu?
//!
//! wgpu's adapter-internal pipeline cache is opaque to callers and
//! varies per backend (Metal, Vulkan, DX12, GLES). Measuring at the
//! chromaticity-render seam — the naga GLSL frontend +
//! [`single_pass::compile_glsl_module`] surface — gives a portable,
//! deterministic cold-vs-warm split that does not depend on the
//! backend's pipeline cache state. The bench therefore measures the
//! crate's OWN compile cost; backend-side pipeline caching is a
//! separate axis (covered later by `EV-GPU-SELECTION`).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::single_pass::{RenderError, compile_glsl_module};

/// Caller-supplied identity for a shader program. Wraps a `String`
/// so the cache key is explicit at every call site — a bare
/// `&str` would let two distinct shaders accidentally collide on
/// the same name without the type system complaining.
///
/// `Arc<str>` storage keeps cloning cheap when a caller looks up
/// the same id repeatedly across frames.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ShaderId(Arc<str>);

impl ShaderId {
    /// Construct a [`ShaderId`] from any string-like value.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying string for diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ShaderId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ShaderId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Per-compile measurement record produced by
/// [`ShaderCache::compile_or_get_module`]. Stored append-only inside
/// the cache so the bench harness (or a future runtime-side observer)
/// can read the cold-vs-warm trace without instrumenting individual
/// call sites.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderCompileMetric {
    /// Identity of the shader the metric refers to.
    pub shader_id: ShaderId,
    /// `true` when the call missed the cache and ran the naga
    /// frontend; `false` when the call hit the cache.
    pub was_cold: bool,
    /// Wall-clock duration the call observed, in nanoseconds.
    /// Recorded via [`Instant::elapsed`] inside
    /// [`ShaderCache::compile_or_get_module`]; covers the cache
    /// lookup AND (on a cold compile) the naga frontend.
    pub elapsed_ns: u64,
}

/// In-memory cache of compiled `naga::Module` IR keyed by
/// [`ShaderId`]. Append-only metrics surface let the bench harness
/// classify each call as cold-vs-warm without re-walking the
/// `HashMap`.
///
/// `Default` constructs an empty cache; that's the cold-start
/// shape the bench's `shader_cold_compile` group exercises.
#[derive(Debug, Default)]
pub struct ShaderCache {
    modules: HashMap<ShaderId, naga::Module>,
    metrics: Vec<ShaderCompileMetric>,
}

impl ShaderCache {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up `id`; on a miss, run the naga GLSL frontend over
    /// `source` at `stage` and store the result. Returns a borrow of
    /// the cached `naga::Module` either way. Records a
    /// [`ShaderCompileMetric`] with `was_cold` set to `true` on the
    /// first call for `id` and `false` on every subsequent call.
    ///
    /// # Errors
    ///
    /// On a cache miss, propagates [`RenderError::FragmentParse`] or
    /// [`RenderError::FragmentValidate`] from the naga frontend.
    /// Cache hits never error.
    pub fn compile_or_get_module(
        &mut self,
        id: &ShaderId,
        source: &str,
        stage: naga::ShaderStage,
    ) -> Result<&naga::Module, RenderError> {
        let start = Instant::now();
        if self.modules.contains_key(id) {
            let elapsed_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            self.metrics.push(ShaderCompileMetric {
                shader_id: id.clone(),
                was_cold: false,
                elapsed_ns,
            });
            // The contains_key gate above guarantees the get returns
            // Some; expressing the lookup as a separate borrow avoids
            // the entry-API pattern (which would require constructing
            // the value eagerly even on the hit path).
            return self
                .modules
                .get(id)
                .ok_or_else(|| RenderError::FragmentValidate("cache lookup race".to_string()));
        }
        let module = compile_glsl_module(source, stage)?;
        let elapsed_ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.modules.insert(id.clone(), module);
        self.metrics.push(ShaderCompileMetric {
            shader_id: id.clone(),
            was_cold: true,
            elapsed_ns,
        });
        // Same get-after-insert shape used elsewhere in this crate.
        self.modules
            .get(id)
            .ok_or_else(|| RenderError::FragmentValidate("cache insert race".to_string()))
    }

    /// Read the append-only metrics trace.
    pub fn metrics(&self) -> &[ShaderCompileMetric] {
        &self.metrics
    }

    /// Number of distinct shader ids currently cached.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// `true` when no ids have been cached yet.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// `true` when `id` is already in the cache. Useful from a bench
    /// harness that needs to assert the warm-path setup landed
    /// before entering the timed loop.
    pub fn contains(&self, id: &ShaderId) -> bool {
        self.modules.contains_key(id)
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod shader_cache_pins {
    //! Evidence: EV-SHADER-CACHE (T1) — cold-vs-warm cache wrapper drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: drift pins
    //! against the [`super::ShaderCache`] surface — cold compile
    //! records `was_cold = true`, warm hit records `was_cold = false`,
    //! lookup is idempotent for a given id, and the cache isolates
    //! per-shader-id. The full cold-vs-warm timing corpus lives in
    //! `engine/render/tests/shader_cache_corpus.rs` (T2); the bench
    //! harness lives at `engine/render/benches/shader_cache.rs`.
    //!
    //! These pins do NOT assert wall-clock budgets — that is the
    //! bench harness's job (and the future
    //! `chore(release-eng)`-PR-extended `scripts/budget-check.sh`
    //! threshold tables). T1 here is purely structural.

    use super::{ShaderCache, ShaderId};

    const TRIVIAL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv, 0.0, 1.0);
}
"#;

    const ALTERNATE_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(1.0 - uv.x, uv.y, 0.5, 1.0);
}
"#;

    #[test]
    fn cold_compile_records_metric_with_was_cold_true() {
        let mut cache = ShaderCache::new();
        let id = ShaderId::from("trivial");
        let _ = cache
            .compile_or_get_module(&id, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("trivial fragment must compile");
        let metrics = cache.metrics();
        assert_eq!(metrics.len(), 1, "exactly one metric after one compile");
        assert!(metrics[0].was_cold, "first compile must be cold");
        assert_eq!(metrics[0].shader_id, id, "metric carries shader id");
    }

    #[test]
    fn warm_compile_records_metric_with_was_cold_false() {
        let mut cache = ShaderCache::new();
        let id = ShaderId::from("trivial");
        let _ = cache
            .compile_or_get_module(&id, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("first compile must succeed");
        let _ = cache
            .compile_or_get_module(&id, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("second compile must hit cache");
        let metrics = cache.metrics();
        assert_eq!(metrics.len(), 2, "one cold + one warm metric");
        assert!(metrics[0].was_cold, "first call cold");
        assert!(!metrics[1].was_cold, "second call warm");
    }

    #[test]
    fn cache_lookup_is_idempotent_for_same_shader_id() {
        let mut cache = ShaderCache::new();
        let id = ShaderId::from("trivial");
        let _ = cache
            .compile_or_get_module(&id, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("first compile must succeed");
        for _ in 0..4 {
            let _ = cache
                .compile_or_get_module(&id, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
                .expect("warm compile must succeed");
        }
        assert_eq!(cache.len(), 1, "id count unchanged across warm hits");
        let cold_count = cache.metrics().iter().filter(|m| m.was_cold).count();
        assert_eq!(cold_count, 1, "exactly one cold compile recorded");
    }

    #[test]
    fn cache_isolates_per_shader_id() {
        let mut cache = ShaderCache::new();
        let trivial = ShaderId::from("trivial");
        let alternate = ShaderId::from("alternate");
        let _ = cache
            .compile_or_get_module(&trivial, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("trivial must compile");
        let _ = cache
            .compile_or_get_module(&alternate, ALTERNATE_FRAGMENT, naga::ShaderStage::Fragment)
            .expect("alternate must compile");
        assert_eq!(cache.len(), 2, "two distinct ids cached");
        assert!(cache.contains(&trivial));
        assert!(cache.contains(&alternate));
        let cold_count = cache.metrics().iter().filter(|m| m.was_cold).count();
        assert_eq!(cold_count, 2, "each distinct id pays cold compile once");
    }
}
