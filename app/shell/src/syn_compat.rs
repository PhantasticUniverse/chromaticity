// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! `.synScene` import compatibility for the live shell.
//!
//! Chromaticity's native vocabulary is `chroma_*`. Reference-format
//! `syn_*` names are allowed only in import code: this module (live
//! shader ABI bridge + imported `script.js` adaptation) and
//! `engine/authoring/src/syn_import.rs` (package conversion and the
//! `SYN_TO_NATIVE` translation table). `scripts/syn-prefix-lint.sh`
//! enforces the boundary.

/// Adapt an imported `script.js` source to the native scripting API
/// (no-op for native scripts). See
/// [`chromaticity_authoring::adapt_syn_script`].
#[must_use]
pub(crate) fn adapt_syn_script(source: &str) -> String {
    chromaticity_authoring::adapt_syn_script(source)
}

/// Live GPU std140 uniform block and Synesthesia builtin aliases substituted for
/// `_ChromaSynBuiltins` when executing `.synScene`-imported shaders on `GpuPreviewEngine`.
const SYN_LIVE_GPU_ABI_BRIDGE: &str = r#"
layout(set = 0, binding = 0, std140) uniform ChromaUniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 _chroma_bpm_vec;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    float chroma_AudioFFT[8];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
    vec4 chroma_AudioEnv0;
    vec4 chroma_AudioEnv1;
    vec4 chroma_AudioEnv2;
    vec4 chroma_AudioEnv3;
    vec4 chroma_AudioMeta;
};

#define RENDERSIZE (chroma_Resolution.xy)
#define TIME (chroma_Time.x)
#define PASSINDEX (0.0)
#define FRAMECOUNT (chroma_Time.z)
#define syn_Time (chroma_AudioEnv0.w)
#define syn_BeatTime (_chroma_bpm_vec.y)
#define syn_BassTime (chroma_AudioEnv1.w)
#define syn_MidTime (chroma_AudioEnv2.w)
#define syn_MidHighTime (0.5 * (chroma_AudioEnv2.w + chroma_AudioEnv3.w))
#define syn_HighTime (chroma_AudioEnv3.w)
#define syn_CurvedTime (chroma_AudioEnv0.w)
#define syn_Level (chroma_AudioEnv0.x)
#define syn_BassLevel (chroma_AudioEnv1.x)
#define syn_MidLevel (chroma_AudioEnv2.x)
#define syn_MidHighLevel (0.5 * (chroma_AudioEnv2.x + chroma_AudioEnv3.x))
#define syn_HighLevel (chroma_AudioEnv3.x)
#define syn_Hits (chroma_AudioEnv0.y)
#define syn_BassHits (chroma_AudioEnv1.y)
#define syn_MidHits (chroma_AudioEnv2.y)
#define syn_MidHighHits (0.5 * (chroma_AudioEnv2.y + chroma_AudioEnv3.y))
#define syn_HighHits (chroma_AudioEnv3.y)
#define syn_Presence (chroma_AudioEnv0.z)
#define syn_BassPresence (chroma_AudioEnv1.z)
#define syn_MidPresence (chroma_AudioEnv2.z)
#define syn_MidHighPresence (0.5 * (chroma_AudioEnv2.z + chroma_AudioEnv3.z))
#define syn_HighPresence (chroma_AudioEnv3.z)
#define syn_Intensity (chroma_AudioMeta.y)
#define syn_OnBeat (_chroma_bpm_vec.z)
#define syn_ToggleOnBeat (step(0.5, fract(_chroma_bpm_vec.y * 0.5)))
#define syn_RandomOnBeat (chroma_AudioMeta.w)
#define syn_FadeInOut (chroma_AudioMeta.x)
#define syn_BPM (_chroma_bpm_vec.x)
#define syn_BPMConfidence (chroma_AudioMeta.z)
#define syn_BPMTwitcher (floor(_chroma_bpm_vec.y) + pow(fract(_chroma_bpm_vec.y), 2.0))
#define syn_BPMSin (_chroma_bpm_vec.w)
#define syn_BPMSin2 (0.5 + 0.5 * sin(_chroma_bpm_vec.y * 3.14159265))
#define syn_BPMSin4 (0.5 + 0.5 * sin(_chroma_bpm_vec.y * 1.57079632))
#define syn_BPMSin8 (0.5 + 0.5 * sin(_chroma_bpm_vec.y * 0.78539816))
#define syn_BPMTri (abs(fract(_chroma_bpm_vec.y) * 2.0 - 1.0))
#define syn_BPMTri2 (abs(fract(_chroma_bpm_vec.y * 0.5) * 2.0 - 1.0))
#define syn_BPMTri4 (abs(fract(_chroma_bpm_vec.y * 0.25) * 2.0 - 1.0))
#define syn_BPMTri8 (abs(fract(_chroma_bpm_vec.y * 0.125) * 2.0 - 1.0))
layout(set = 1, binding = 0) uniform texture2D _chroma_media_tex;
layout(set = 1, binding = 1) uniform sampler _chroma_media_samp;
layout(set = 1, binding = 2, std140) uniform _ChromaMediaInfo {
    vec4 _chroma_media_info;
    vec4 _chroma_media_target;
};
#define syn_MediaActive (_chroma_media_info.x)

vec4 _chroma_sample_spectrum(vec2 uv) {
    int idx = clamp(int(uv.x * 8.0), 0, 7);
    float v = chroma_AudioFFT[idx];
    return vec4(v, v, v, v);
}

vec4 _chroma_sample_spectrum(vec2 uv, float lod) {
    return _chroma_sample_spectrum(uv) + 0.0 * vec4(lod);
}

vec4 _chroma_sample_media(vec2 uv) {
    return texture(sampler2D(_chroma_media_tex, _chroma_media_samp), uv);
}

vec4 _chroma_sample_media(vec2 uv, float lod) {
    return textureLod(sampler2D(_chroma_media_tex, _chroma_media_samp), uv, lod);
}
"#;

fn ctrl_channel_expr(slot: usize) -> Option<&'static str> {
    const SLOTS: [&str; 16] = [
        "chroma_Ctrl0.x",
        "chroma_Ctrl0.y",
        "chroma_Ctrl0.z",
        "chroma_Ctrl0.w",
        "chroma_Ctrl1.x",
        "chroma_Ctrl1.y",
        "chroma_Ctrl1.z",
        "chroma_Ctrl1.w",
        "chroma_Ctrl2.x",
        "chroma_Ctrl2.y",
        "chroma_Ctrl2.z",
        "chroma_Ctrl2.w",
        "chroma_Ctrl3.x",
        "chroma_Ctrl3.y",
        "chroma_Ctrl3.z",
        "chroma_Ctrl3.w",
    ];
    SLOTS.get(slot).copied()
}

/// Rewrite `texture(`, `texture2D(`, and `textureLod(` calls targeting a
/// Synesthesia virtual sampler (`syn_UserImage` or `syn_Spectrum`) into the
/// live std140 helper function, tolerating arbitrary whitespace around the
/// sampler argument.
fn rewrite_syn_sampler_calls(source: &str, sampler_name: &str, replacement_fn: &str) -> String {
    const CALL_NAMES: &[&str] = &["textureLod", "texture2D", "texture"];
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;

    while cursor < source.len() {
        let mut matched: Option<usize> = None;
        for &call in CALL_NAMES {
            if source[cursor..].starts_with(call) {
                let prev_is_ident = cursor > 0
                    && (bytes[cursor - 1].is_ascii_alphanumeric() || bytes[cursor - 1] == b'_');
                if prev_is_ident {
                    continue;
                }
                let after_name = cursor + call.len();
                let ws_before_paren = source[after_name..]
                    .bytes()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count();
                let paren_pos = after_name + ws_before_paren;
                if paren_pos >= source.len() || bytes[paren_pos] != b'(' {
                    continue;
                }
                let after_paren = paren_pos + 1;
                let ws_before_arg = source[after_paren..]
                    .bytes()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count();
                let arg_pos = after_paren + ws_before_arg;
                if !source[arg_pos..].starts_with(sampler_name) {
                    continue;
                }
                let after_arg = arg_pos + sampler_name.len();
                let ws_before_comma = source[after_arg..]
                    .bytes()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count();
                let comma_pos = after_arg + ws_before_comma;
                if comma_pos < source.len() && bytes[comma_pos] == b',' {
                    matched = Some(comma_pos + 1);
                    break;
                }
            }
        }
        if let Some(next_pos) = matched {
            out.push_str(replacement_fn);
            out.push('(');
            cursor = next_pos;
        } else if let Some(ch) = source[cursor..].chars().next() {
            out.push(ch);
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Adapt a `.synScene`-imported fragment shader (emitting `_ChromaSynBuiltins`
/// and `_ChromaSynControls`) to Chromaticity's live binding-0 std140 UBO ABI
/// so imported scenes compile and render live on `GpuPreviewEngine`.
///
/// `pass_index` parameterizes `#define PASSINDEX (<pass_index>.0)` for
/// multi-pass `.synScene` packages declaring `PASSES`.
pub(crate) fn adapt_syn_imported_shader(source: &str, pass_index: u32) -> String {
    const BUILTINS_START: &str =
        "layout(set = 0, binding = 0, std140) uniform _ChromaSynBuiltins {";
    const SPECTRUM_END: &str =
        "#define syn_Spectrum sampler2D(_chroma_spectrum_tex, _chroma_spectrum_samp)";

    if !source.contains(BUILTINS_START) {
        return source.to_string();
    }

    let mut out = source.to_string();
    if let (Some(start), Some(end_rel)) = (out.find(BUILTINS_START), out.find(SPECTRUM_END)) {
        let end = end_rel + SPECTRUM_END.len();
        if end >= start {
            let bridge = if pass_index == 0 {
                SYN_LIVE_GPU_ABI_BRIDGE.to_string()
            } else {
                SYN_LIVE_GPU_ABI_BRIDGE.replace(
                    "#define PASSINDEX (0.0)",
                    &format!("#define PASSINDEX ({pass_index}.0)"),
                )
            };
            out.replace_range(start..end, &bridge);
        }
    }

    // Rewrite `_ChromaSynControls` UBO at binding 5 into `#define` macros backed by `chroma_Ctrl0..3`.
    const CONTROLS_START: &str =
        "layout(set = 0, binding = 5, std140) uniform _ChromaSynControls {";
    if let Some(c_start) = out.find(CONTROLS_START) {
        if let Some(c_close_rel) = out[c_start..].find("};") {
            let c_end = c_start + c_close_rel + 2;
            let body = &out[c_start + CONTROLS_START.len()..c_start + c_close_rel];
            let mut macros = String::new();
            let mut next_slot = 0usize;
            for line in body.lines() {
                let trimmed = line.trim().trim_end_matches(';');
                if trimmed.is_empty() {
                    continue;
                }
                let mut parts = trimmed.split_whitespace();
                let (Some(ty), Some(name)) = (parts.next(), parts.next()) else {
                    continue;
                };
                match ty {
                    "float" => {
                        let expr = ctrl_channel_expr(next_slot).unwrap_or("0.5");
                        next_slot += 1;
                        macros.push_str(&format!("#define {name} ({expr})\n"));
                    }
                    "vec2" => {
                        let x = ctrl_channel_expr(next_slot).unwrap_or("0.5");
                        let y = ctrl_channel_expr(next_slot + 1).unwrap_or("0.5");
                        next_slot += 2;
                        macros.push_str(&format!("#define {name} vec2({x}, {y})\n"));
                    }
                    "vec3" => {
                        let x = ctrl_channel_expr(next_slot).unwrap_or("0.5");
                        let y = ctrl_channel_expr(next_slot + 1).unwrap_or("0.5");
                        let z = ctrl_channel_expr(next_slot + 2).unwrap_or("0.5");
                        next_slot += 3;
                        macros.push_str(&format!("#define {name} vec3({x}, {y}, {z})\n"));
                    }
                    "vec4" => {
                        let x = ctrl_channel_expr(next_slot).unwrap_or("0.5");
                        let y = ctrl_channel_expr(next_slot + 1).unwrap_or("0.5");
                        let z = ctrl_channel_expr(next_slot + 2).unwrap_or("0.5");
                        let w = ctrl_channel_expr(next_slot + 3).unwrap_or("1.0");
                        next_slot += 4;
                        macros.push_str(&format!("#define {name} vec4({x}, {y}, {z}, {w})\n"));
                    }
                    _ => {}
                }
            }
            out.replace_range(c_start..c_end, &macros);
        }
    }

    // Route media lookups to the live group-1 media input and spectrum
    // lookups to the std140 FFT fallback.
    out = rewrite_syn_sampler_calls(&out, "syn_Spectrum", "_chroma_sample_spectrum");
    out = rewrite_syn_sampler_calls(&out, "syn_UserImage", "_chroma_sample_media");
    out = out.replace(
        "textureSize(syn_UserImage, 0)",
        "ivec2(max(_chroma_media_info.zw, vec2(1.0)))",
    );
    out = out.replace("textureSize(syn_Spectrum, 0)", "ivec2(8, 1)");

    out
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod syn_compat_pins {
    //! Evidence: EV-SCENE-IMPORT (T1) — imported `.synScene` shaders and scripts run on the native ABI.

    use super::*;
    use crate::render::GpuPreviewEngine;
    use crate::script_host::ScriptHost;
    use chromaticity_audio::AudioUniformSnapshot;

    #[test]
    fn imported_syn_scene_renders_live_on_gpu() {
        use chromaticity_authoring::syn_import::import_syn_scene_to_chroma;

        let base =
            std::env::temp_dir().join(format!("chromaticity-syn-gpu-test-{}", std::process::id()));
        let src_dir = base.join("prism_pulse.synScene");
        let out_parent = base.join("out");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&out_parent).unwrap();

        let syn_json = r#"{
            "TITLE": "Prism Pulse",
            "CREDIT": "Test Author",
            "DESCRIPTION": "Imported synScene live GPU ABI test",
            "CONTROLS": [
                { "NAME": "gain", "TYPE": "slider", "MIN": 0.0, "MAX": 2.0, "DEFAULT": 1.0 },
                { "NAME": "offset_xy", "TYPE": "xy", "MIN": -1.0, "MAX": 1.0, "DEFAULT": [0.0, 0.0] }
            ]
        }"#;
        let syn_glsl = r#"
vec4 renderMain() {
    vec2 p = _rotate(_uv - 0.5 + offset_xy * 0.1, syn_Time * 0.25);
    float spec = texture(syn_Spectrum, vec2(_uv.x, 0.5)).r;
    float glow = (0.35 + syn_BassLevel * 0.4 + spec * 0.25) * max(gain, 0.2);
    vec3 col = vec3(glow, 0.3 + 0.5 * abs(p.x), 0.5 + 0.4 * syn_BPMSin);
    return vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;
        std::fs::write(src_dir.join("scene.json"), syn_json).unwrap();
        std::fs::write(src_dir.join("main.glsl"), syn_glsl).unwrap();

        let report = import_syn_scene_to_chroma(&src_dir, &out_parent)
            .expect("synScene import must succeed");
        let mut engine = GpuPreviewEngine::new();
        engine.set_scene(report.slug.clone(), &report.output_dir);
        let audio = AudioUniformSnapshot {
            bpm: 124.0,
            rms: 0.6,
            peak: 0.85,
            fft_bands: [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1],
            ..AudioUniformSnapshot::default()
        };
        let bytes = engine
            .render_frame(160, 90, 1.5, 90, &audio, [0.9; 16])
            .expect("imported .synScene must compile and render live on Metal");
        assert_eq!(bytes.len(), 160 * 90 * 4);
        assert!(
            bytes
                .chunks_exact(4)
                .any(|px| px[0] > 20 && px[1] > 20 && px[2] > 20),
            "imported .synScene rendered all-black"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn adapt_syn_imported_shader_parameterizes_passindex_and_media_calls() {
        let sample = r#"#version 450
layout(set = 0, binding = 0, std140) uniform _ChromaSynBuiltins {
    vec2 RENDERSIZE;
    float PASSINDEX;
};
#define syn_UserImage sampler2D(_chroma_media_tex, _chroma_media_samp)
#define syn_Spectrum sampler2D(_chroma_spectrum_tex, _chroma_spectrum_samp)
vec4 renderMain() {
    vec4 a = texture(syn_UserImage, uv);
    vec4 b = texture( syn_UserImage , uv );
    vec4 c = texture2D(syn_UserImage, uv);
    vec4 d = textureLod(syn_UserImage, uv, 1.0);
    vec4 e = texture(syn_Spectrum, vec2(uv.x, 0.5));
    vec4 f = textureLod( syn_Spectrum , vec2(uv.x, 0.5), 0.0);
    return a + b + c + d + e + f + vec4(PASSINDEX);
}
"#;
        let pass0 = adapt_syn_imported_shader(sample, 0);
        assert!(pass0.contains("#define PASSINDEX (0.0)"));
        assert!(!pass0.contains("syn_UserImage"));
        assert!(!pass0.contains("syn_Spectrum"));
        let body = pass0.split("vec4 renderMain()").nth(1).unwrap();
        assert_eq!(body.matches("_chroma_sample_media(").count(), 4);
        assert_eq!(body.matches("_chroma_sample_spectrum(").count(), 2);

        let pass2 = adapt_syn_imported_shader(sample, 2);
        assert!(pass2.contains("#define PASSINDEX (2.0)"));
        assert!(!pass2.contains("#define PASSINDEX (0.0)"));
    }

    #[test]
    fn imported_script_using_syn_names_is_adapted_to_native_globals() {
        let mut host = ScriptHost::new();
        host.set_inline_script(
            "syn-legacy".into(),
            r#"
            var accum = 0.0;
            function update(dt) {
                var unknownFallback = inputs.nonexistent_control_xyz;
                accum += (inputs.syn_BassLevel * 2.0 + unknownFallback) * dt;
                uniforms.my_pulse = inputs.syn_BassLevel + inputs.syn_Time * 0.1;
                uniforms.accum_time = accum;
                uniforms.bpm_echo = inputs.syn_BPM;
                uniforms.global_bpm = syn_BPM;
            }
            "#
            .into(),
        );
        assert_eq!(host.state_name(), "active");
        let audio = AudioUniformSnapshot {
            bpm: 128.0,
            rms: 0.4,
            peak: 0.7,
            fft_bands: [0.8, 0.6, 0.5, 0.3, 0.25, 0.15, 0.1, 0.05],
            ..AudioUniformSnapshot::default()
        };
        let out = host.tick(2.0, 0.5, 10, &audio);
        assert!(host.status().error.is_none());
        let pulse = out.get("my_pulse").copied().expect("my_pulse emitted");
        assert!(pulse > 0.7 && pulse < 1.0, "pulse={pulse}");
        let accum = out.get("accum_time").copied().expect("accum_time emitted");
        assert!((accum - 0.7).abs() < 1e-4, "accum={accum}");
        for key in ["bpm_echo", "global_bpm"] {
            let v = out.get(key).copied().expect("bpm emitted");
            assert!((v - 128.0).abs() < 1e-4, "{key}={v}");
        }
    }

    #[test]
    fn imported_scene_media_helpers_read_the_live_media_input() {
        use chromaticity_authoring::syn_import::import_syn_scene_to_chroma;
        use chromaticity_render::MediaKind;

        let base = std::env::temp_dir().join(format!(
            "chromaticity-syn-media-test-{}",
            std::process::id()
        ));
        let src_dir = base.join("media_probe.synScene");
        let out_parent = base.join("out");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&out_parent).unwrap();
        std::fs::write(
            src_dir.join("scene.json"),
            r#"{ "TITLE": "Media Probe", "CREDIT": "Test", "CONTROLS": [] }"#,
        )
        .unwrap();
        // Left half: _loadMedia(); right half: _loadMediaAsMask();
        // alpha carries syn_MediaActive.
        std::fs::write(
            src_dir.join("main.glsl"),
            r#"
vec4 renderMain() {
    vec4 c = _uv.x < 0.5 ? _loadMedia() : _loadMediaAsMask();
    return vec4(c.rgb, syn_MediaActive);
}
"#,
        )
        .unwrap();
        let report = import_syn_scene_to_chroma(&src_dir, &out_parent).expect("import");

        let mut engine = GpuPreviewEngine::new();
        engine.set_scene(report.slug.clone(), &report.output_dir);
        let audio = AudioUniformSnapshot::default();
        let inactive = engine
            .render_frame(16, 16, 0.0, 1, &audio, [0.5; 16])
            .expect("imported media scene renders without media");
        assert_eq!(
            &inactive[0..4],
            &[0, 0, 0, 0],
            "no media: black, syn_MediaActive 0"
        );

        // 2x2 quadrants (TL red, TR green, BL blue, BR white): detects a
        // vertical flip through the importer's `_uv` (gl_FragCoord-based).
        let frame = crate::render::SceneMediaFrame {
            width: 2,
            height: 2,
            kind: MediaKind::Image,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        };
        engine.sync_media(1, Some(&frame)).expect("sync");
        let bytes = engine
            .render_frame(16, 16, 0.016, 2, &audio, [0.5; 16])
            .expect("imported media scene renders with media");
        let px = |x: usize, y: usize| bytes[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4].to_vec();
        // Left half: _loadMedia() upright, alpha = syn_MediaActive.
        assert_eq!(px(3, 3), vec![255, 0, 0, 255], "top-left red");
        assert_eq!(px(3, 12), vec![0, 0, 255, 255], "bottom-left blue");
        // Right half: _loadMediaAsMask() luminance of green (0.587 * 255) and white.
        let green_mask = px(12, 3);
        assert!(
            green_mask[0] == green_mask[1] && green_mask[1] == green_mask[2],
            "mask is grey: {green_mask:?}"
        );
        assert!(
            (149..=150).contains(&green_mask[0]),
            "green luminance {green_mask:?}"
        );
        assert_eq!(px(12, 12), vec![255, 255, 255, 255], "white mask");
        let _ = std::fs::remove_dir_all(&base);
    }
}
