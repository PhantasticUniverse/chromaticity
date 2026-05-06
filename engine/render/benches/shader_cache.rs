// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d-exit `EV-SHADER-CACHE` cold-vs-warm shader-compile criterion
//! harness (Path B, nightly-only).
//!
//! Produces evidence for `EV-SHADER-CACHE` row 79 of
//! `docs/26-evidence-artifact-registry.md`. Wired into the existing
//! `.github/workflows/macos-nightly.yml` workflow's
//! `cargo bench --workspace --benches` step so the harness runs once
//! per day on Apple Silicon Metal alongside the `engine/runtime`
//! `budgets` bench — no per-PR matrix expansion (the AO posture
//! rejects the per-PR macOS surcharge in favour of nightly-only
//! cadence at ~$20/month with at most a 24h discovery delay; same
//! framing the macos-nightly workflow records for the Metal-backend
//! coverage rationale).
//!
//! # docs/10 budget anchors
//!
//! `docs/10-performance-latency-budgets.md` §Shader Cache pins:
//!
//! - cold-start shader compile: median <= 800 ms / p95 <= 1800 ms
//!   per shader program;
//! - warm-hit shader load: p95 <= 40 ms per program.
//!
//! This harness produces the underlying timing series. Hard-budget
//! assertion against those numbers is OUT OF SCOPE for this PR and
//! lands in a follow-up `chore(release-eng)` PR that extends
//! `scripts/budget-check.sh` with shader-cache thresholds. The
//! existing `budget-check.sh` hard-codes the 9 `EV-PERF-BUDGETS`
//! proxy keys against the docs/10 "command + state updates: <= 1.25
//! ms" budget; shader-cache cold/warm budgets sit on a different
//! scale and need their own threshold table.
//!
//! # Bench groups
//!
//! - `shader_cold_compile/{trivial,uv_gradient,inline_helpers}` —
//!   each iteration constructs a fresh
//!   [`chromaticity_render::ShaderCache`] and compiles a
//!   representative GLSL fragment from scratch. Measures the cold
//!   path (naga GLSL→IR→validate cost paid every iteration).
//! - `shader_warm_compile/{trivial,uv_gradient,inline_helpers}` —
//!   the bench prewarms the cache once OUTSIDE the measurement
//!   closure, then each iteration calls
//!   [`ShaderCache::compile_or_get_module`] against the same id and
//!   gets a cached module back. Measures the warm path (cache
//!   lookup only — no naga frontend re-run).
//!
//! Three fixtures stress the GLSL frontend across a range of work-
//! load sizes:
//!
//! - `trivial` — bare UV passthrough, smallest meaningful naga
//!   workload.
//! - `uv_gradient` — multi-statement fragment with arithmetic +
//!   smoothstep; representative of typical scene fragments.
//! - `inline_helpers` — fragment with two inline helper functions
//!   exercising the parser's function-table + validator's call-graph
//!   path; representative of a Lygia-style include after
//!   preprocessing.
//!
//! All three fixtures live inline so the bench has no fixture-
//! filesystem dependency (the bench runner under criterion does not
//! set CWD to the crate root the way `cargo test` does for
//! integration tests).
//!
//! # Implementation notes
//!
//! - `default harness = false` is set in `Cargo.toml`; criterion ships
//!   its own [`criterion_main!`] entry point.
//! - `cache.compile_or_get_module(...).expect(...)` bench-side is
//!   acceptable: the bench file is opted out of the workspace
//!   `clippy::unwrap_used` / `clippy::expect_used` denials at file
//!   scope (criterion benches aren't `#[test]` functions, so the
//!   standard `cfg_attr(test, allow(...))` form doesn't apply; a
//!   plain file-scope `#![allow(...)]` is the equivalent for
//!   benches, mirroring `engine/runtime/benches/budgets.rs`).
//! - The warm-compile bench groups intentionally re-create the
//!   prewarm closure inside `iter_batched` semantics by holding the
//!   prewarmed cache in a `let mut` outside `b.iter` — criterion's
//!   `b.iter` runs the closure body many times; the cache stays
//!   warm across all iterations, which is exactly the warm-hit
//!   shape we want to measure.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromaticity_render::{ShaderCache, ShaderId};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Bare UV passthrough; smallest meaningful naga workload. Mirrors
/// the `TRIVIAL_FRAGMENT` shape used in
/// `engine/render/src/single_pass.rs::compile_drift_pins`.
const TRIVIAL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv, 0.0, 1.0);
}
"#;

/// Multi-statement fragment with arithmetic + smoothstep;
/// representative of a typical scene fragment. Larger than
/// `TRIVIAL_FRAGMENT` so the cold-vs-warm ratio is more legible
/// in the criterion output.
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

/// Fragment with two inline helper functions exercising the parser's
/// function-table + validator's call-graph path. Representative of a
/// Lygia-style include AFTER preprocessing (helpers inlined into the
/// caller's source rather than declared in a header file).
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

const FIXTURES: &[(&str, &str)] = &[
    ("trivial", TRIVIAL_FRAGMENT),
    ("uv_gradient", UV_GRADIENT_FRAGMENT),
    ("inline_helpers", INLINE_HELPERS_FRAGMENT),
];

fn bench_shader_cold_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("shader_cold_compile");
    for &(label, source) in FIXTURES {
        let id = ShaderId::from(label);
        group.bench_function(label, |b| {
            b.iter(|| {
                // Fresh cache every iteration ⇒ first call is always
                // cold. `black_box` on the source defeats LLVM
                // attempts to constant-fold the literal across
                // iterations.
                let mut cache = ShaderCache::new();
                let module = cache
                    .compile_or_get_module(&id, black_box(source), naga::ShaderStage::Fragment)
                    .expect("fixture fragment must compile");
                black_box(module.entry_points.len())
            });
        });
    }
    group.finish();
}

fn bench_shader_warm_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("shader_warm_compile");
    for &(label, source) in FIXTURES {
        let id = ShaderId::from(label);
        // Prewarm OUTSIDE the measurement closure: this single call
        // pays the cold compile cost; every `b.iter` iteration below
        // is a warm hit. Mirrors the
        // `cache_lookup_is_idempotent_for_same_shader_id` T1 pin's
        // post-prewarm shape.
        let mut cache = ShaderCache::new();
        cache
            .compile_or_get_module(&id, source, naga::ShaderStage::Fragment)
            .expect("prewarm compile must succeed");
        assert!(cache.contains(&id), "warm bench setup must populate cache");
        group.bench_function(label, |b| {
            b.iter(|| {
                let module = cache
                    .compile_or_get_module(&id, black_box(source), naga::ShaderStage::Fragment)
                    .expect("warm hit must succeed");
                black_box(module.entry_points.len())
            });
        });
    }
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets =
        bench_shader_cold_compile,
        bench_shader_warm_compile,
);
criterion_main!(benches);
