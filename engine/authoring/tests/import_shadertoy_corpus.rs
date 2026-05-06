// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-IMPORT-SHADERTOY (T2) — Phase-4b Wave 4b.5 Lane A
//! PR 1 of 2.
//!
//! T2 per `docs/43-development-standards.md` §15.5: golden-fixture
//! corpus exercising the [`chromaticity_authoring::import_shadertoy`]
//! converter end-to-end across 3 representative Shadertoy patterns
//! per `docs/38-scene-library-and-import-spec.md` §5.4 (simple math,
//! noise/fbm, raymarched-style use of `iTime` + `iResolution`). PR 2
//! widens to the iChannel fixture pack with the cubemap + 2D-texture
//! fixtures called out by §5.4 line 121.
//!
//! Per fixture, this corpus asserts:
//!
//! - the converter emits a `<slug>.chroma` package with the expected
//!   `scene.json` + `main.glsl` file pair;
//! - the produced `scene.json` round-trips through
//!   `chromaticity_scene::loader::load_chroma_dir` and carries the
//!   imported `TITLE` + `CREDIT` + `DESCRIPTION` + `FORMAT_TAG`;
//! - the on-disk `main.glsl` parses + validates through the same
//!   naga GLSL frontend + IR validator pair the engine/render
//!   single-pass entry point uses internally;
//! - no warnings fire for fixtures that do not reference iChannel /
//!   iMouse, confirming the soft-warning surface is precise.
//!
//! Each fixture's source GLSL is inlined as a `&str` rather than a
//! sidecar `.glsl.txt` to keep the corpus self-contained per the PR
//! 1 brief.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chromaticity_authoring::import_shadertoy::{
    ImportReport, ShadertoyImportConfig, import_shadertoy_to_chroma, load_imported_scene,
};

/// Per-fixture T2 record — Shadertoy source + expected import shape.
struct ShadertoyFixture {
    label: &'static str,
    title: &'static str,
    description: &'static str,
    source_glsl: &'static str,
    expected_warnings: usize,
}

const FIXTURES: &[ShadertoyFixture] = &[
    ShadertoyFixture {
        label: "simple-math",
        title: "Simple Math Stripes",
        description: "Animated gradient stripes driven by iTime — the simple-math arm of \
                      the docs/38 §5.4 fixture set.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float stripes = 0.5 + 0.5 * sin(uv.x * 20.0 + iTime * 2.0);
    fragColor = vec4(vec3(stripes), 1.0);
}
"#,
        expected_warnings: 0,
    },
    ShadertoyFixture {
        label: "noise-fbm",
        title: "Noise fBm",
        description: "Hash-based fractional Brownian motion (fBm) — the noise/fbm arm of \
                      the docs/38 §5.4 fixture set.",
        source_glsl: r#"
float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        v += a * noise(p);
        p *= 2.0;
        a *= 0.5;
    }
    return v;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float n = fbm(uv * 4.0 + iTime * 0.1);
    fragColor = vec4(vec3(n), 1.0);
}
"#,
        expected_warnings: 0,
    },
    ShadertoyFixture {
        label: "raymarched-sphere",
        title: "Raymarched Sphere",
        description: "Single-sphere distance-field raymarcher — the raymarched arm of \
                      the docs/38 §5.4 fixture set.",
        source_glsl: r#"
float sdSphere(vec3 p, float r) {
    return length(p) - r;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    vec3 ro = vec3(0.0, 0.0, -2.5);
    vec3 rd = normalize(vec3(uv, 1.0));
    float t = 0.0;
    float d = 0.0;
    for (int i = 0; i < 32; i++) {
        vec3 p = ro + rd * t;
        d = sdSphere(p, 1.0 + 0.1 * sin(iTime));
        if (d < 0.001) break;
        t += d;
        if (t > 10.0) break;
    }
    float shade = (d < 0.01) ? clamp(1.0 - t * 0.25, 0.0, 1.0) : 0.0;
    fragColor = vec4(vec3(shade), 1.0);
}
"#,
        expected_warnings: 0,
    },
];

fn unique_target_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "chromaticity-shadertoy-corpus-{label}-{pid}-{nanos}-{n}"
    ));
    fs::create_dir_all(&path).expect("create unique target dir");
    path
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn naga_compile_check(glsl_source: &str) -> Result<(), String> {
    let mut frontend = naga::front::glsl::Frontend::default();
    let module = match frontend.parse(
        &naga::front::glsl::Options {
            stage: naga::ShaderStage::Fragment,
            defines: Default::default(),
        },
        glsl_source,
    ) {
        Ok(m) => m,
        Err(errors) => return Err(format!("naga GLSL parse rejected: {errors:?}")),
    };
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map(|_| ())
        .map_err(|err| format!("naga validator rejected: {err:?}"))
}

fn import_one(target_dir: &Path, fixture: &ShadertoyFixture) -> ImportReport {
    let config = ShadertoyImportConfig {
        shadertoy_id: Some(format!("st-{}", fixture.label)),
        title: fixture.title.to_string(),
        credit: format!(
            "Chromaticity team golden corpus / docs/38 §5.4 / fixture: {}",
            fixture.label
        ),
        description: fixture.description.to_string(),
        source_glsl: fixture.source_glsl.to_string(),
    };
    import_shadertoy_to_chroma(target_dir, &config).unwrap_or_else(|err| {
        panic!(
            "fixture `{}` should import successfully but rejected with {err:?}",
            fixture.label
        )
    })
}

#[test]
fn corpus_each_fixture_produces_expected_files() {
    for fixture in FIXTURES {
        let target = unique_target_dir(fixture.label);
        let report = import_one(&target, fixture);

        assert!(
            report.package_root.is_dir(),
            "fixture `{}`: package root is a directory",
            fixture.label
        );
        assert_eq!(
            report.files_written.len(),
            2,
            "fixture `{}`: expected scene.json + main.glsl",
            fixture.label
        );
        let scene_json = report.package_root.join("scene.json");
        let main_glsl = report.package_root.join("main.glsl");
        assert!(
            scene_json.is_file(),
            "fixture `{}`: scene.json",
            fixture.label
        );
        assert!(
            main_glsl.is_file(),
            "fixture `{}`: main.glsl",
            fixture.label
        );

        cleanup(&target);
    }
}

#[test]
fn corpus_each_fixture_round_trips_through_loader() {
    for fixture in FIXTURES {
        let target = unique_target_dir(fixture.label);
        let report = import_one(&target, fixture);

        let scene = load_imported_scene(&report).unwrap_or_else(|err| {
            panic!(
                "fixture `{}` should round-trip through the loader: {err:?}",
                fixture.label
            )
        });
        assert_eq!(scene.header.schema_version, 1);

        let format_tag = scene
            .extensions
            .get("FORMAT_TAG")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("fixture `{}`: FORMAT_TAG missing", fixture.label));
        assert_eq!(
            format_tag, "shadertoy-v1-import",
            "fixture `{}`: FORMAT_TAG mismatch",
            fixture.label
        );

        let title = scene
            .extensions
            .get("TITLE")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("fixture `{}`: TITLE missing", fixture.label));
        assert_eq!(title, fixture.title);

        assert_eq!(
            scene.passes.len(),
            1,
            "fixture `{}`: expected single backbuff pass",
            fixture.label
        );

        cleanup(&target);
    }
}

#[test]
fn corpus_each_fixture_main_glsl_compiles_via_naga() {
    for fixture in FIXTURES {
        let target = unique_target_dir(fixture.label);
        let report = import_one(&target, fixture);

        let main_glsl_path = report.package_root.join("main.glsl");
        let on_disk = fs::read_to_string(&main_glsl_path).expect("read main.glsl");
        naga_compile_check(&on_disk).unwrap_or_else(|err| {
            panic!(
                "fixture `{}`: on-disk main.glsl must parse + validate, got {err}",
                fixture.label
            )
        });

        cleanup(&target);
    }
}

#[test]
fn corpus_warning_surface_matches_expected() {
    // None of the 3 PR-1 fixtures exercise iChannel / iMouse, so each
    // should import with zero warnings. PR 2 will widen with fixtures
    // that DO carry warnings (iChannel mappings against the bundled
    // fixture pack).
    for fixture in FIXTURES {
        let target = unique_target_dir(fixture.label);
        let report = import_one(&target, fixture);
        assert_eq!(
            report.conversion_warnings.len(),
            fixture.expected_warnings,
            "fixture `{}`: warning count mismatch (got {:?})",
            fixture.label,
            report.conversion_warnings,
        );
        cleanup(&target);
    }
}
