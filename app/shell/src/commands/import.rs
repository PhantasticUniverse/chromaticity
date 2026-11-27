// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shadertoy, Synesthesia (`.synScene`), and ISF import IPC handlers backed by
//! `chromaticity-authoring`.

use chromaticity_authoring::{
    ShadertoyImportConfig, import_shadertoy_to_chroma, shadertoy_license::parse_shadertoy_license,
    syn_import::import_syn_scene_to_chroma, try_reject_isf_input,
};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

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

/// Structured response returned by `import_syn_scene`.
#[derive(Debug, Clone, Serialize)]
pub struct SynImportResultDto {
    pub output_dir: String,
    pub scene_title: String,
    pub control_count: usize,
    pub has_script: bool,
    pub warnings: Vec<String>,
}

/// Structured response returned by `inspect_shadertoy_license`.
#[derive(Debug, Clone, Serialize)]
pub struct ShadertoyLicenseDto {
    pub license: String,
    pub is_commercial_safe: bool,
    pub requires_attribution: bool,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub attribution_notice: String,
    pub warning: Option<String>,
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

/// Import a Synesthesia `.synScene` directory into a validated `.chroma`
/// package under `target_parent_dir` (or the app's default scene root when `None`).
#[tauri::command]
pub fn import_syn_scene(
    source_dir: String,
    target_parent_dir: Option<String>,
) -> Result<SynImportResultDto, String> {
    import_syn_scene_impl(source_dir, target_parent_dir)
}

pub(crate) fn import_syn_scene_impl(
    source_dir: String,
    target_parent_dir: Option<String>,
) -> Result<SynImportResultDto, String> {
    let source_path = validate_path_input(&source_dir, "source_dir")?;
    let target_root = match target_parent_dir {
        Some(dir) if !dir.trim().is_empty() => validate_path_input(&dir, "target_parent_dir")?,
        _ => PathBuf::from(crate::commands::library::default_scene_root()?),
    };

    let report =
        import_syn_scene_to_chroma(&source_path, &target_root).map_err(|err| err.to_string())?;
    let scene_title = extract_scene_title(&report.scene_json_path, &report.slug);

    Ok(SynImportResultDto {
        output_dir: report.output_dir.to_string_lossy().into_owned(),
        scene_title,
        control_count: report.declared_control_uniforms.len(),
        has_script: report.script_js_copied,
        warnings: report.warnings,
    })
}

/// Inspect a Shadertoy GLSL header for license classification, attribution,
/// and non-commercial warnings.
#[tauri::command]
pub fn inspect_shadertoy_license(
    glsl_source: String,
    fallback_author: Option<String>,
    source_url: Option<String>,
) -> ShadertoyLicenseDto {
    inspect_shadertoy_license_impl(
        &glsl_source,
        fallback_author.as_deref(),
        source_url.as_deref(),
    )
}

pub(crate) fn inspect_shadertoy_license_impl(
    glsl_source: &str,
    fallback_author: Option<&str>,
    source_url: Option<&str>,
) -> ShadertoyLicenseDto {
    let report = parse_shadertoy_license(glsl_source, fallback_author, source_url);
    ShadertoyLicenseDto {
        license: report.license.spdx_or_label().to_string(),
        is_commercial_safe: report.is_commercial_safe,
        requires_attribution: report.requires_attribution,
        author: report.author,
        source_url: report.source_url,
        attribution_notice: report.attribution_notice,
        warning: report.warning,
    }
}

fn validate_path_input(raw: &str, label: &str) -> Result<PathBuf, String> {
    if raw.contains('\0') {
        return Err(format!("{label} must not contain NUL bytes"));
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    let path = PathBuf::from(trimmed);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "{label} must not contain parent-directory ('..') traversal components"
        ));
    }
    Ok(path)
}

fn extract_scene_title(scene_json_path: &Path, fallback_slug: &str) -> String {
    std::fs::read_to_string(scene_json_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|val| {
            val.get("TITLE")
                .and_then(|title| title.as_str())
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| fallback_slug.to_string())
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

    #[test]
    fn syn_scene_import_round_trips_valid_package() {
        let base = std::env::temp_dir().join(format!(
            "chromaticity-syn-import-cmd-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let source_dir = base.join("NeonPulse.synScene");
        let target_parent = base.join("out");
        std::fs::create_dir_all(&source_dir).expect("source .synScene dir must be created");
        std::fs::create_dir_all(&target_parent).expect("target parent dir must be created");

        let scene_json = r#"{
  "TITLE": "Neon Pulse",
  "CREDIT": "Test Author",
  "DESCRIPTION": "Round-trip synScene import test",
  "CONTROLS": [
    {
      "NAME": "speed",
      "TYPE": "slider",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 5.0
    },
    {
      "NAME": "tint_color",
      "TYPE": "color",
      "DEFAULT": [0.2, 0.6, 1.0]
    }
  ]
}"#;
        let main_glsl = r#"
vec4 renderMain() {
    float wave = pow(sin(TIME * speed), 2.0);
    return vec4(tint_color * wave, 1.0);
}
"#;
        std::fs::write(source_dir.join("scene.json"), scene_json)
            .expect("scene.json must be written");
        std::fs::write(source_dir.join("main.glsl"), main_glsl).expect("main.glsl must be written");
        std::fs::write(source_dir.join("script.js"), "function update(dt) {}\n")
            .expect("script.js must be written");

        let dto = import_syn_scene_impl(
            source_dir.to_string_lossy().into_owned(),
            Some(target_parent.to_string_lossy().into_owned()),
        )
        .expect("valid .synScene package must import cleanly");

        assert_eq!(dto.scene_title, "Neon Pulse");
        assert_eq!(dto.control_count, 2);
        assert!(dto.has_script);
        let out_path = PathBuf::from(&dto.output_dir);
        assert!(out_path.join("scene.json").is_file());
        assert!(out_path.join("main.glsl").is_file());
        assert!(out_path.join("script.js").is_file());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn syn_scene_import_rejects_nul_and_parent_traversal() {
        let nul_err = import_syn_scene_impl("/tmp/bad\0scene.synScene".to_string(), None)
            .expect_err("NUL byte in source_dir must be rejected");
        assert!(nul_err.contains("NUL"));

        let trav_err = import_syn_scene_impl("/tmp/a/../b.synScene".to_string(), None)
            .expect_err("parent traversal in source_dir must be rejected");
        assert!(trav_err.contains(".."));

        let target_trav_err = import_syn_scene_impl(
            "/tmp/clean.synScene".to_string(),
            Some("/tmp/out/../escape".to_string()),
        )
        .expect_err("parent traversal in target_parent_dir must be rejected");
        assert!(target_trav_err.contains(".."));
    }

    #[test]
    fn inspect_shadertoy_license_preserves_cc_by_nc_warning_and_detects_mit() {
        let cc_by_nc = r#"
// SPDX-License-Identifier: CC-BY-NC-4.0
// Author: Alex Rivera
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(0.5);
}
"#;
        let nc_dto = inspect_shadertoy_license_impl(
            cc_by_nc,
            None,
            Some("https://www.shadertoy.com/view/testNC"),
        );
        assert_eq!(nc_dto.license, "CC-BY-NC");
        assert!(!nc_dto.is_commercial_safe);
        assert!(nc_dto.requires_attribution);
        assert_eq!(nc_dto.author.as_deref(), Some("Alex Rivera"));
        assert_eq!(
            nc_dto.source_url.as_deref(),
            Some("https://www.shadertoy.com/view/testNC")
        );
        let warning = nc_dto
            .warning
            .as_deref()
            .expect("CC BY-NC must emit a non-commercial warning");
        assert!(warning.contains("Non-Commercial"));
        assert!(nc_dto.attribution_notice.contains("CC-BY-NC"));

        let unlabeled = "void mainImage(out vec4 c, in vec2 f) { c = vec4(1.0); }";
        let default_dto = inspect_shadertoy_license_impl(unlabeled, Some("CommunityAuthor"), None);
        assert_eq!(default_dto.license, "CC-BY-NC-SA-3.0 (Shadertoy Default)");
        assert!(!default_dto.is_commercial_safe);
        assert!(default_dto.requires_attribution);
        assert_eq!(default_dto.author.as_deref(), Some("CommunityAuthor"));
        assert!(
            default_dto
                .warning
                .as_deref()
                .is_some_and(|w| w.contains("Non-Commercial"))
        );

        let mit = r#"
// The MIT License
// Copyright (c) 2024 Inigo Quilez
void mainImage(out vec4 c, in vec2 f) { c = vec4(0.0); }
"#;
        let mit_dto = inspect_shadertoy_license_impl(mit, None, None);
        assert_eq!(mit_dto.license, "MIT");
        assert!(mit_dto.is_commercial_safe);
        assert!(mit_dto.requires_attribution);
        assert!(mit_dto.warning.is_none());
    }
}
