// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shadertoy & ISF import IPC handlers backed by `chromaticity-authoring`.

use chromaticity_authoring::{
    ShadertoyImportConfig, import_shadertoy_to_chroma, try_reject_isf_input,
};
use serde::Serialize;
use std::path::PathBuf;

/// Structured response returned to `Import.svelte` after Shadertoy conversion.
#[derive(Debug, Clone, Serialize)]
pub struct ShadertoyImportResponse {
    pub scene_id: String,
    pub output_dir: String,
    pub main_glsl: String,
    pub scene_json: String,
    pub warnings: Vec<String>,
}

/// Structured response returned by `check_isf_import`.
#[derive(Debug, Clone, Serialize)]
pub struct IsfCheckResponse {
    pub is_isf: bool,
    pub message: Option<String>,
    pub hint: Option<String>,
}

/// Convert a Shadertoy GLSL snippet (`mainImage`) into a validated `.chroma`
/// package using `chromaticity-authoring`.
#[tauri::command]
pub fn import_shadertoy_source(
    glsl: String,
    title: Option<String>,
    credit: Option<String>,
    target_dir: Option<String>,
) -> Result<ShadertoyImportResponse, String> {
    import_shadertoy_source_impl(glsl, title, credit, target_dir)
}

pub(crate) fn import_shadertoy_source_impl(
    glsl: String,
    title: Option<String>,
    credit: Option<String>,
    target_dir: Option<String>,
) -> Result<ShadertoyImportResponse, String> {
    let clean_title = title
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported Shadertoy".to_string());
    let scene_id = slugify_scene_id(&clean_title);

    let root = target_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("chromaticity-imports"));
    std::fs::create_dir_all(&root).map_err(|err| {
        format!(
            "failed to prepare import directory {}: {err}",
            root.display()
        )
    })?;

    let pkg_dir = root.join(format!("{scene_id}.chroma"));
    if pkg_dir.exists() {
        let _ = std::fs::remove_dir_all(&pkg_dir);
    }

    let config = ShadertoyImportConfig {
        shadertoy_id: None,
        title: clean_title,
        credit: credit
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Imported via Chromaticity Shadertoy converter".to_string()),
        description: "Converted from Shadertoy GLSL via chromaticity-authoring.".to_string(),
        source_glsl: glsl,
        ichannel_mappings: Vec::new(),
    };

    let report = import_shadertoy_to_chroma(&root, &config).map_err(|err| err.to_string())?;
    let main_glsl = std::fs::read_to_string(pkg_dir.join("main.glsl"))
        .map_err(|err| format!("failed to read converted main.glsl: {err}"))?;
    let scene_json = std::fs::read_to_string(pkg_dir.join("scene.json"))
        .map_err(|err| format!("failed to read converted scene.json: {err}"))?;

    Ok(ShadertoyImportResponse {
        scene_id,
        output_dir: pkg_dir.to_string_lossy().into_owned(),
        main_glsl,
        scene_json,
        warnings: report.warnings.iter().map(ToString::to_string).collect(),
    })
}

/// Inspect a shader source string for the V1 ISF fingerprint and return the
/// canonical actionable diagnostic if detected.
#[tauri::command]
pub fn check_isf_import(source: String) -> Result<IsfCheckResponse, String> {
    Ok(check_isf_import_impl(&source))
}

pub(crate) fn check_isf_import_impl(source: &str) -> IsfCheckResponse {
    match try_reject_isf_input(std::path::Path::new("<inline.fs>"), source) {
        None => IsfCheckResponse {
            is_isf: false,
            message: None,
            hint: None,
        },
        Some(diag) => IsfCheckResponse {
            is_isf: true,
            message: Some(diag.message),
            hint: diag.hint,
        },
    }
}

fn slugify_scene_id(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "imported-shader".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod import_command_pins {
    use super::*;

    #[test]
    fn shadertoy_import_converts_valid_main_image() {
        let tmp = std::env::temp_dir().join("chromaticity-import-test");
        let glsl = r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}
"#;
        let res = import_shadertoy_source_impl(
            glsl.to_string(),
            Some("Test Gradient".to_string()),
            None,
            Some(tmp.to_string_lossy().into_owned()),
        )
        .expect("valid Shadertoy mainImage should convert");
        assert_eq!(res.scene_id, "test-gradient");
        assert!(res.main_glsl.contains("mainImage"));
        assert!(res.scene_json.contains("Test Gradient"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn isf_check_detects_isf_and_passes_shadertoy() {
        let isf = r#"/*{"ISFVSN":"2","INPUTS":[]}*/ void main() { gl_FragColor = vec4(1.0); }"#;
        let check = check_isf_import_impl(isf);
        assert!(check.is_isf);
        assert!(check.message.is_some());
        assert!(check.hint.is_some());

        let clean = check_isf_import_impl("void mainImage(out vec4 c, in vec2 f) {}");
        assert!(!clean.is_isf);
    }
}
