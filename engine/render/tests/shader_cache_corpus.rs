// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SHADER-CACHE (T2) — cold-vs-warm cache wrapper conformance corpus.
//!
//! T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set. Advances row 79 of
//! `docs/26-evidence-artifact-registry.md` (`EV-SHADER-CACHE`) past
//! the foundational T1 drift pins in
//! `engine/render/src/shader_cache.rs::shader_cache_pins`.
//!
//! The companion criterion bench harness lives at
//! `engine/render/benches/shader_cache.rs`; this corpus is the
//! correctness side of the same surface — the bench measures the
//! cold-vs-warm timing series, the corpus pins the cold-vs-warm
//! semantics. Both consume the same three fixtures so a regression
//! that breaks the corpus also breaks the bench (and vice-versa).
//!
//! # Anchors
//!
//! - `docs/10-performance-latency-budgets.md` §Shader Cache —
//!   cold-vs-warm budget targets. The corpus does NOT assert wall-
//!   clock budgets; that's the bench harness's job.
//! - `docs/26-evidence-artifact-registry.md` row 79 — `EV-SHADER-CACHE`.
//! - `docs/adr/ADR-011-rendering-backend.md` — wgpu + naga + GLSL→SPIR-V pinning.
//!
//! # Why GPU-free
//!
//! [`ShaderCache::compile_or_get_module`] runs the naga GLSL frontend
//! only — no wgpu adapter, no device, no GPU work. The corpus
//! therefore runs unchanged on adapter-less CI runners (the same
//! property the slice-1 `single_pass::compile_drift_pins` T1 module
//! relies on). The wgpu-side end-to-end pipeline cache is a
//! separate axis and is not part of `EV-SHADER-CACHE`'s scope.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_render::{ShaderCache, ShaderId};

const TRIVIAL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv, 0.0, 1.0);
}
"#;

const UV_GRADIENT_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    vec2 centered = uv - vec2(0.5);
    float radius = length(centered);
    float falloff = smoothstep(0.5, 0.0, radius);
    vec3 color = mix(vec3(0.05, 0.10, 0.20), vec3(1.0, 0.85, 0.6), falloff);
    fragColor = vec4(color, 1.0);
}
"#;

const INLINE_HELPERS_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

float hash21(vec2 p) {
    p = fract(p * vec2(123.45, 678.90));
    p += dot(p, p + 34.567);
    return fract(p.x * p.y);
}

vec3 palette(float t) {
    vec3 a = vec3(0.5, 0.5, 0.5);
    vec3 b = vec3(0.5, 0.5, 0.5);
    vec3 c = vec3(1.0, 1.0, 1.0);
    vec3 d = vec3(0.00, 0.10, 0.20);
    return a + b * cos(6.28318 * (c * t + d));
}

void main() {
    float n = hash21(uv);
    vec3 col = palette(n);
    fragColor = vec4(col, 1.0);
}
"#;

/// Three named fixtures stress the GLSL frontend across a range of
/// workload sizes. Mirrors the bench harness at
/// `engine/render/benches/shader_cache.rs` so a regression on either
/// side surfaces on both.
const FIXTURES: &[(&str, &str)] = &[
    ("trivial", TRIVIAL_FRAGMENT),
    ("uv_gradient", UV_GRADIENT_FRAGMENT),
    ("inline_helpers", INLINE_HELPERS_FRAGMENT),
];

#[test]
fn corpus_cold_then_warm_per_fixture() {
    let mut cache = ShaderCache::new();
    for &(label, source) in FIXTURES {
        let id = ShaderId::from(label);
        // Cold compile.
        let _ = cache
            .compile_or_get_module(&id, source, naga::ShaderStage::Fragment)
            .unwrap_or_else(|e| panic!("cold compile of {label} must succeed: {e:?}"));
        // Warm hit (3x for stability).
        for _ in 0..3 {
            let _ = cache
                .compile_or_get_module(&id, source, naga::ShaderStage::Fragment)
                .unwrap_or_else(|e| panic!("warm hit of {label} must succeed: {e:?}"));
        }
    }
    assert_eq!(
        cache.len(),
        FIXTURES.len(),
        "every fixture id is cached exactly once"
    );
    let cold_count = cache.metrics().iter().filter(|m| m.was_cold).count();
    let warm_count = cache.metrics().iter().filter(|m| !m.was_cold).count();
    assert_eq!(cold_count, FIXTURES.len(), "one cold compile per fixture");
    assert_eq!(
        warm_count,
        FIXTURES.len() * 3,
        "three warm hits per fixture"
    );
}

#[test]
fn corpus_metrics_preserve_call_order() {
    let mut cache = ShaderCache::new();
    let trivial = ShaderId::from("trivial");
    let gradient = ShaderId::from("uv_gradient");
    // Interleaved call sequence: cold(trivial), cold(gradient),
    // warm(trivial), warm(gradient), warm(trivial). Append-only
    // metrics preserve the sequence.
    let _ = cache
        .compile_or_get_module(&trivial, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
        .expect("first trivial compile must succeed");
    let _ = cache
        .compile_or_get_module(&gradient, UV_GRADIENT_FRAGMENT, naga::ShaderStage::Fragment)
        .expect("first gradient compile must succeed");
    let _ = cache
        .compile_or_get_module(&trivial, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
        .expect("second trivial compile must succeed");
    let _ = cache
        .compile_or_get_module(&gradient, UV_GRADIENT_FRAGMENT, naga::ShaderStage::Fragment)
        .expect("second gradient compile must succeed");
    let _ = cache
        .compile_or_get_module(&trivial, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
        .expect("third trivial compile must succeed");
    let metrics = cache.metrics();
    assert_eq!(metrics.len(), 5);
    let expected: [(&str, bool); 5] = [
        ("trivial", true),
        ("uv_gradient", true),
        ("trivial", false),
        ("uv_gradient", false),
        ("trivial", false),
    ];
    for (i, (id, was_cold)) in expected.iter().enumerate() {
        assert_eq!(metrics[i].shader_id.as_str(), *id, "metric {i} id");
        assert_eq!(metrics[i].was_cold, *was_cold, "metric {i} was_cold");
    }
}

#[test]
fn corpus_distinct_sources_under_distinct_ids_isolated() {
    let mut cache = ShaderCache::new();
    for &(label, source) in FIXTURES {
        let id = ShaderId::from(label);
        let _ = cache
            .compile_or_get_module(&id, source, naga::ShaderStage::Fragment)
            .unwrap_or_else(|e| panic!("cold compile of {label} must succeed: {e:?}"));
    }
    // Every fixture stays cached after the loop completes; later
    // lookups for any of them must register as warm hits.
    for &(label, source) in FIXTURES {
        let id = ShaderId::from(label);
        assert!(
            cache.contains(&id),
            "id {label} stays in cache after sibling compiles"
        );
        let _ = cache
            .compile_or_get_module(&id, source, naga::ShaderStage::Fragment)
            .unwrap_or_else(|e| panic!("warm hit of {label} must succeed: {e:?}"));
    }
    let cold_count = cache.metrics().iter().filter(|m| m.was_cold).count();
    assert_eq!(
        cold_count,
        FIXTURES.len(),
        "exactly one cold compile per distinct id, no cross-id false-warm"
    );
}

#[test]
fn corpus_invalid_source_rejects_without_caching() {
    let mut cache = ShaderCache::new();
    let id = ShaderId::from("broken");
    let broken = r#"
#version 450
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = NOT_A_VALID_TOKEN;
}
"#;
    let err = cache
        .compile_or_get_module(&id, broken, naga::ShaderStage::Fragment)
        .expect_err("broken GLSL must reject");
    // Error variant is a naga frontend error; the exact variant is
    // pinned by the slice-1 `single_pass::compile_drift_pins` T1.
    let _ = err;
    assert!(
        !cache.contains(&id),
        "rejected source must NOT populate the cache",
    );
    assert_eq!(cache.len(), 0, "rejected source leaves cache empty",);
    // Subsequent valid compile under a fresh id still works (the
    // failed call did not poison the cache).
    let good_id = ShaderId::from("recovered");
    let _ = cache
        .compile_or_get_module(&good_id, TRIVIAL_FRAGMENT, naga::ShaderStage::Fragment)
        .expect("post-rejection compile must succeed");
    assert!(cache.contains(&good_id));
}
