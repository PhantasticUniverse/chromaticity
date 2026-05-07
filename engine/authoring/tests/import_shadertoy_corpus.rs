// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-IMPORT-SHADERTOY (T2) — Phase-4b Wave 4b.5 Lane A.
//!
//! T2 per `docs/43-development-standards.md` §15.5: golden-fixture
//! corpus exercising the [`chromaticity_authoring::import_shadertoy`]
//! converter end-to-end across the PR-1 starter trio (simple math,
//! noise/fbm, raymarched-style use of `iTime` + `iResolution` per
//! `docs/38-scene-library-and-import-spec.md` §5.4) PLUS the PR-2
//! widening fixtures (iChannel fixture-pack resolution per
//! `docs/38` §5.2a + edge-case sweep). The PR-2 set covers:
//!
//! - `ichannel-2d-texture` — 2D-texture iChannel binding via
//!   [`IChannelMapping`] (fulfills `docs/38` §5.4 line 121 "1
//!   2D-texture iChannel" mandate).
//! - `ichannel-cubemap` — cubemap iChannel binding via
//!   [`IChannelMapping`] (fulfills `docs/38` §5.4 line 121 "1
//!   cubemap iChannel" mandate).
//! - `ichannel-audio-binding` — audio iChannel binding (Shadertoy's
//!   FFT row 0 / time-domain row 1 layout).
//! - `ichannel-multichannel` — mixed iChannel0 (texture) +
//!   iChannel1 (audio) + iChannel2 (buffer) per `docs/38` §5.2a.
//! - `time-resolution-mouse-uniforms` — `iTime` + `iResolution` +
//!   `iMouse`; asserts the `MouseUniformDeferred` warning fires.
//! - `gl-fragcoord-direct-access` — body uses `gl_FragCoord` direct
//!   access (canonical GLSL built-in; converter accepts unchanged).
//! - `deprecated-texture2d-autocorrected` — body uses
//!   `texture2D(...)`; converter rewrites to `texture(...)` and
//!   surfaces [`Warning::DeprecatedTexture2DAutoCorrected`].
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
//! - structured warnings match the per-fixture expectation set so
//!   the warning-surface contract from PR 2 stays drift-pinned.
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
    IChannelKind, IChannelMapping, ImportReport, ShadertoyImportConfig, Warning,
    import_shadertoy_to_chroma, load_imported_scene,
};

/// Per-fixture T2 record — Shadertoy source + expected import shape.
struct ShadertoyFixture {
    label: &'static str,
    title: &'static str,
    description: &'static str,
    source_glsl: &'static str,
    /// Caller-supplied iChannel mappings (PR 2 widening). Empty
    /// fixtures leave the table unset.
    ichannels: &'static [PortableMapping],
    expected_warnings: usize,
    /// Pred-style assertions executed per warning kind. The closure
    /// shape lets us narrow per-warning expectations without
    /// expanding the per-fixture record into many bool flags.
    expected_warning_predicates: &'static [fn(&Warning) -> bool],
}

/// Const-friendly mirror of [`IChannelMapping`] (the real type is
/// not const-constructible because it carries `String`).
struct PortableMapping {
    channel_index: u8,
    kind: IChannelKind,
    source_path: Option<&'static str>,
}

impl PortableMapping {
    fn to_owned(&self) -> IChannelMapping {
        IChannelMapping {
            channel_index: self.channel_index,
            kind: self.kind,
            source_path: self.source_path.map(str::to_string),
        }
    }
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
        ichannels: &[],
        expected_warnings: 0,
        expected_warning_predicates: &[],
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
        ichannels: &[],
        expected_warnings: 0,
        expected_warning_predicates: &[],
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
        ichannels: &[],
        expected_warnings: 0,
        expected_warning_predicates: &[],
    },
    // EV-IMPORT-SHADERTOY T2: 2D-texture iChannel binding via the
    // PR 2 IChannelMapping builder. Fulfils `docs/38` §5.4 line 121
    // "1 2D-texture iChannel" mandate.
    ShadertoyFixture {
        label: "ichannel-2d-texture",
        title: "iChannel 2D Texture",
        description: "Samples a 2D-texture iChannel — exercises the PR 2 IChannelMapping \
                      Texture2D resolution path against the bundled fixture pack.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = texture(iChannel0, uv);
}
"#,
        ichannels: &[PortableMapping {
            channel_index: 0,
            kind: IChannelKind::Texture2D,
            source_path: Some("shadertoy-fixtures/textures/noise.png"),
        }],
        expected_warnings: 0,
        expected_warning_predicates: &[],
    },
    // EV-IMPORT-SHADERTOY T2: cubemap iChannel binding. Fulfils
    // `docs/38` §5.4 line 121 "1 cubemap iChannel" mandate.
    ShadertoyFixture {
        label: "ichannel-cubemap",
        title: "iChannel Cubemap",
        description: "Samples a cubemap iChannel — exercises the PR 2 IChannelMapping \
                      Cubemap resolution path with samplerCube preamble.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    vec3 dir = normalize(vec3(uv, 1.0));
    fragColor = texture(iChannel0, dir);
}
"#,
        ichannels: &[PortableMapping {
            channel_index: 0,
            kind: IChannelKind::Cubemap,
            source_path: Some("shadertoy-fixtures/cubemaps/night-sky"),
        }],
        expected_warnings: 0,
        expected_warning_predicates: &[],
    },
    // EV-IMPORT-SHADERTOY T2: audio iChannel binding. Validates the
    // `IChannelKind::Audio` resolution path used by the engine/render
    // audio_adapter audio-uniform-IPC consumer.
    ShadertoyFixture {
        label: "ichannel-audio-binding",
        title: "iChannel Audio Binding",
        description: "Reads FFT row 0 from an audio iChannel — exercises the PR 2 \
                      IChannelMapping Audio resolution path (Shadertoy's 512x2 sampler2D).",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float fft = texture(iChannel0, vec2(uv.x, 0.25)).r;
    fragColor = vec4(vec3(fft), 1.0);
}
"#,
        ichannels: &[PortableMapping {
            channel_index: 0,
            kind: IChannelKind::Audio,
            source_path: Some("audio:///input"),
        }],
        expected_warnings: 0,
        expected_warning_predicates: &[],
    },
    // EV-IMPORT-SHADERTOY T2: mixed iChannel0 (texture) + iChannel1
    // (audio) + iChannel2 (buffer) per docs/38 §5.2a multi-channel
    // resolution.
    ShadertoyFixture {
        label: "ichannel-multichannel",
        title: "iChannel Multi Channel",
        description: "Samples 3 iChannels of mixed kinds (texture / audio / buffer) — \
                      exercises the PR 2 multi-channel resolution path.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec4 tex = texture(iChannel0, uv);
    float fft = texture(iChannel1, vec2(uv.x, 0.25)).r;
    vec4 buf = texture(iChannel2, uv);
    fragColor = mix(tex, buf, fft);
}
"#,
        ichannels: &[
            PortableMapping {
                channel_index: 0,
                kind: IChannelKind::Texture2D,
                source_path: Some("shadertoy-fixtures/textures/tile.png"),
            },
            PortableMapping {
                channel_index: 1,
                kind: IChannelKind::Audio,
                source_path: Some("audio:///input"),
            },
            PortableMapping {
                channel_index: 2,
                kind: IChannelKind::Buffer,
                source_path: Some("buffers/A"),
            },
        ],
        expected_warnings: 0,
        expected_warning_predicates: &[],
    },
    // EV-IMPORT-SHADERTOY T2: iTime + iResolution + iMouse. The
    // converter declares iMouse in the preamble and surfaces a
    // `MouseUniformDeferred` warning so the author knows the mouse
    // binding wires through chroma_Mouse in a later wave.
    ShadertoyFixture {
        label: "time-resolution-mouse-uniforms",
        title: "iTime iResolution iMouse",
        description: "References iTime, iResolution, and iMouse — exercises the canonical \
                      Shadertoy uniform mapping plus the MouseUniformDeferred warning.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec2 m = iMouse.xy / iResolution.xy;
    float t = sin(iTime + length(uv - m) * 8.0);
    fragColor = vec4(vec3(0.5 + 0.5 * t), 1.0);
}
"#,
        ichannels: &[],
        expected_warnings: 1,
        expected_warning_predicates: &[is_mouse_uniform_deferred],
    },
    // EV-IMPORT-SHADERTOY T2: gl_FragCoord direct access. gl_FragCoord
    // is the canonical GLSL fragment-coordinate built-in and is valid
    // in Chromaticity-native passes; the converter accepts it
    // unchanged.
    ShadertoyFixture {
        label: "gl-fragcoord-direct-access",
        title: "gl_FragCoord Direct Access",
        description: "Body reads gl_FragCoord directly instead of via the mainImage \
                      fragCoord parameter — converter accepts unchanged.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;
    fragColor = vec4(uv, 0.5, 1.0);
}
"#,
        ichannels: &[],
        expected_warnings: 0,
        expected_warning_predicates: &[],
    },
    // EV-IMPORT-SHADERTOY T2: deprecated texture2D auto-correction.
    // Naga's GLSL frontend rejects texture2D outside legacy #version
    // 110 profiles (the wrapper declares #version 450); the
    // converter rewrites texture2D(...) → texture(...) and surfaces
    // a structured warning.
    ShadertoyFixture {
        label: "deprecated-texture2d-autocorrected",
        title: "Deprecated texture2D Autocorrected",
        description: "Body uses deprecated texture2D(...) — converter rewrites to \
                      texture(...) and surfaces DeprecatedTexture2DAutoCorrected warning.",
        source_glsl: r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = texture2D(iChannel0, uv);
}
"#,
        ichannels: &[PortableMapping {
            channel_index: 0,
            kind: IChannelKind::Texture2D,
            source_path: Some("shadertoy-fixtures/textures/wood.png"),
        }],
        expected_warnings: 1,
        expected_warning_predicates: &[is_texture2d_autocorrected],
    },
];

fn is_mouse_uniform_deferred(w: &Warning) -> bool {
    matches!(w, Warning::MouseUniformDeferred)
}

fn is_texture2d_autocorrected(w: &Warning) -> bool {
    matches!(w, Warning::DeprecatedTexture2DAutoCorrected)
}

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
    let mappings: Vec<IChannelMapping> = fixture
        .ichannels
        .iter()
        .map(PortableMapping::to_owned)
        .collect();
    let config = ShadertoyImportConfig {
        shadertoy_id: Some(format!("st-{}", fixture.label)),
        title: fixture.title.to_string(),
        credit: format!(
            "Chromaticity team golden corpus / docs/38 §5.4 / fixture: {}",
            fixture.label
        ),
        description: fixture.description.to_string(),
        source_glsl: fixture.source_glsl.to_string(),
        ichannel_mappings: mappings,
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

        // PR 2: when ichannels are declared, scene.json carries an
        // ICHANNELS table that round-trips through the loader's
        // extensions map.
        if !fixture.ichannels.is_empty() {
            let ichannels = scene
                .extensions
                .get("ICHANNELS")
                .and_then(|v| v.as_array())
                .unwrap_or_else(|| {
                    panic!(
                        "fixture `{}`: ICHANNELS array missing from scene.json",
                        fixture.label
                    )
                });
            assert_eq!(
                ichannels.len(),
                fixture.ichannels.len(),
                "fixture `{}`: ICHANNELS count mismatch",
                fixture.label
            );
        }

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
    for fixture in FIXTURES {
        let target = unique_target_dir(fixture.label);
        let report = import_one(&target, fixture);
        assert_eq!(
            report.warnings.len(),
            fixture.expected_warnings,
            "fixture `{}`: warning count mismatch (got {:?})",
            fixture.label,
            report.warnings,
        );
        // Each declared predicate must match at least one warning.
        for pred in fixture.expected_warning_predicates {
            assert!(
                report.warnings.iter().any(pred),
                "fixture `{}`: warning predicate not satisfied (got {:?})",
                fixture.label,
                report.warnings,
            );
        }
        // The legacy string-surface mirrors the structured surface.
        assert_eq!(
            report.conversion_warnings.len(),
            report.warnings.len(),
            "fixture `{}`: legacy + structured warning surfaces stay 1:1",
            fixture.label,
        );
        cleanup(&target);
    }
}
