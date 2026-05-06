// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4b Wave 4b.4 Lane A `EV-AUTHORING-SCAFFOLDER` corpus. Tier
//! declaration and evidence tag live on the `fixtures` inner module
//! below, matching the pattern established by
//! `engine/runtime/tests/crash_resume_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-AUTHORING-SCAFFOLDER (T2) — Phase-4b Wave 4b.4
    //! Lane A scaffolder golden-output corpus.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises the
    //! evidence-producing behaviour against a fixture corpus. Each
    //! fixture (a) constructs a [`ScaffolderConfig`] under varying
    //! shapes, (b) drives [`scaffold_to_path`] to emit the
    //! `<scene_id>.chroma` package skeleton, (c) loads the produced
    //! `scene.json` through the canonical
    //! `chromaticity_scene::loader::load_chroma_dir` parser, and (d)
    //! invokes the same naga GLSL frontend + IR validator pair the
    //! engine/render single-pass entry point uses internally to
    //! confirm the embedded `main.glsl` compiles + validates.
    //!
    //! Fixtures cover the EV-AUTHORING-SCAFFOLDER contract per
    //! `docs/26-evidence-artifact-registry.md` row 63:
    //!
    //! 1. `scaffold_then_load_full_round_trip` — the scaffolder's
    //!    `scene.json` round-trips through the scene loader,
    //!    preserving `TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`,
    //!    `CATEGORIES`, and `SCHEMA_VERSION`.
    //! 2. `scaffold_then_render_compile_passes` — the scaffolder's
    //!    `main.glsl` parses + validates through naga, exercising
    //!    the same surface engine/render uses on author-supplied
    //!    fragments.
    //! 3. `scaffold_with_unicode_metadata_preserves_utf8` —
    //!    non-ASCII title / credit / tags survive the on-disk round
    //!    trip without mojibake or lossy normalisation.
    //! 4. `scaffold_to_path_inside_existing_library_does_not_clash`
    //!    — scaffolding a new scene next to scaffolded sibling
    //!    packages does not collide on the package-name path.
    //! 5. `scaffold_with_minimal_metadata_still_loads` — empty
    //!    `tags` / `categories` and short single-character `title`
    //!    survive the round trip.
    //! 6. `scaffold_emits_pretty_printed_scene_json` — the
    //!    on-disk `scene.json` is human-editable (multi-line, not
    //!    a single packed line) so authors can hand-edit the
    //!    scaffolded package without tooling.

    use chromaticity_authoring::scaffolder::{
        ScaffolderConfig, ScaffolderReport, load_scaffolded_scene, scaffold_to_path,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

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
            "chromaticity-scaffolder-corpus-{label}-{pid}-{nanos}-{n}"
        ));
        fs::create_dir_all(&path).expect("create unique target dir");
        path
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn baseline_config(scene_id: &str) -> ScaffolderConfig {
        ScaffolderConfig {
            scene_id: scene_id.to_string(),
            title: "Corpus Fixture".to_string(),
            credit: "chromaticity test corpus".to_string(),
            description: "Scaffolder T2 corpus fixture scene.".to_string(),
            tags: vec!["test".to_string(), "corpus".to_string()],
            categories: vec!["FIXTURE".to_string(), "AUTHORING".to_string()],
        }
    }

    fn validate_scaffolded_glsl(report: &ScaffolderReport) {
        let main_glsl_path = report.package_root.join("main.glsl");
        let source = fs::read_to_string(&main_glsl_path).expect("read main.glsl");

        let mut frontend = naga::front::glsl::Frontend::default();
        let module = frontend
            .parse(
                &naga::front::glsl::Options {
                    stage: naga::ShaderStage::Fragment,
                    defines: Default::default(),
                },
                &source,
            )
            .expect("scaffolded fragment parses through naga");

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator
            .validate(&module)
            .expect("scaffolded fragment validates through naga");
        assert!(
            !module.entry_points.is_empty(),
            "scaffolded fragment should expose a fragment entry point",
        );
    }

    #[test]
    fn scaffold_then_load_full_round_trip() {
        let target = unique_target_dir("round-trip");
        let config = baseline_config("corpus-round-trip");

        let report = scaffold_to_path(&target, &config).expect("scaffold succeeds");
        let scene = load_scaffolded_scene(&report).expect("loader accepts scaffolded scene");

        assert_eq!(scene.header.schema_version, 1);
        assert_eq!(
            scene.extensions.get("TITLE").and_then(|v| v.as_str()),
            Some(config.title.as_str()),
        );
        assert_eq!(
            scene.extensions.get("CREDIT").and_then(|v| v.as_str()),
            Some(config.credit.as_str()),
        );
        assert_eq!(
            scene.extensions.get("DESCRIPTION").and_then(|v| v.as_str()),
            Some(config.description.as_str()),
        );

        let tags: Vec<String> = scene
            .extensions
            .get("TAGS")
            .and_then(|v| v.as_array())
            .expect("TAGS array present")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        assert_eq!(tags, config.tags);

        let categories: Vec<String> = scene
            .extensions
            .get("CATEGORIES")
            .and_then(|v| v.as_array())
            .expect("CATEGORIES array present")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        assert_eq!(categories, config.categories);

        assert!(
            scene.controls.is_empty(),
            "scaffolded scene starts with empty CONTROLS",
        );
        assert!(
            scene.passes.is_empty(),
            "scaffolded scene starts with empty PASSES",
        );

        cleanup(&target);
    }

    #[test]
    fn scaffold_then_render_compile_passes() {
        let target = unique_target_dir("render-compile");
        let config = baseline_config("corpus-render-compile");

        let report = scaffold_to_path(&target, &config).expect("scaffold succeeds");
        validate_scaffolded_glsl(&report);

        cleanup(&target);
    }

    #[test]
    fn scaffold_with_unicode_metadata_preserves_utf8() {
        let target = unique_target_dir("unicode");
        let config = ScaffolderConfig {
            scene_id: "corpus-unicode".to_string(),
            title: "色彩 — Iridescence ✨".to_string(),
            credit: "Café Société".to_string(),
            description: "Multi-bytê descripção with combining: e\u{0301}".to_string(),
            tags: vec!["日本語".to_string(), "français".to_string()],
            categories: vec!["UNICODE".to_string()],
        };

        let report = scaffold_to_path(&target, &config).expect("scaffold succeeds");

        // On-disk file must be valid UTF-8.
        let scene_json_path = report.package_root.join("scene.json");
        let raw = fs::read(&scene_json_path).expect("read raw bytes");
        let body = std::str::from_utf8(&raw).expect("scene.json is valid UTF-8");
        // Confirm at least one non-ASCII codepoint round-tripped.
        assert!(
            body.contains("色彩") || body.contains("\\u8272\\u5f69"),
            "scene.json should preserve unicode title (raw or escaped JSON form)",
        );

        let scene = load_scaffolded_scene(&report).expect("loader accepts unicode scene");
        assert_eq!(
            scene.extensions.get("TITLE").and_then(|v| v.as_str()),
            Some(config.title.as_str()),
        );
        assert_eq!(
            scene.extensions.get("CREDIT").and_then(|v| v.as_str()),
            Some(config.credit.as_str()),
        );
        assert_eq!(
            scene.extensions.get("DESCRIPTION").and_then(|v| v.as_str()),
            Some(config.description.as_str()),
        );

        cleanup(&target);
    }

    #[test]
    fn scaffold_to_path_inside_existing_library_does_not_clash() {
        // Scaffold three sibling scenes under the same parent dir
        // and assert each emits its own package directory without
        // colliding.
        let target = unique_target_dir("library");
        let configs = [
            baseline_config("library-scene-a"),
            baseline_config("library-scene-b"),
            baseline_config("library-scene-c"),
        ];

        let reports: Vec<ScaffolderReport> = configs
            .iter()
            .map(|c| scaffold_to_path(&target, c).expect("scaffold sibling succeeds"))
            .collect();

        for (config, report) in configs.iter().zip(reports.iter()) {
            assert_eq!(
                report.package_root,
                target.join(format!("{}.chroma", config.scene_id)),
            );
            // Each one independently round-trips.
            let scene = load_scaffolded_scene(report).expect("sibling loads");
            assert_eq!(
                scene.extensions.get("TITLE").and_then(|v| v.as_str()),
                Some(config.title.as_str()),
            );
        }

        // No two reports point at the same directory.
        for (i, lhs) in reports.iter().enumerate() {
            for (j, rhs) in reports.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        lhs.package_root, rhs.package_root,
                        "sibling packages must not collide",
                    );
                }
            }
        }

        cleanup(&target);
    }

    #[test]
    fn scaffold_with_minimal_metadata_still_loads() {
        let target = unique_target_dir("minimal");
        let config = ScaffolderConfig {
            scene_id: "min".to_string(),
            title: "X".to_string(),
            credit: String::new(),
            description: String::new(),
            tags: Vec::new(),
            categories: Vec::new(),
        };

        let report = scaffold_to_path(&target, &config).expect("minimal scaffold succeeds");
        let scene = load_scaffolded_scene(&report).expect("minimal scene loads");
        assert_eq!(
            scene.extensions.get("TITLE").and_then(|v| v.as_str()),
            Some("X"),
        );
        let tags = scene
            .extensions
            .get("TAGS")
            .and_then(|v| v.as_array())
            .expect("TAGS still emitted as array");
        assert!(tags.is_empty(), "empty TAGS array round-trips");

        validate_scaffolded_glsl(&report);

        cleanup(&target);
    }

    #[test]
    fn scaffold_emits_pretty_printed_scene_json() {
        let target = unique_target_dir("pretty");
        let config = baseline_config("corpus-pretty");

        let report = scaffold_to_path(&target, &config).expect("scaffold succeeds");
        let body =
            fs::read_to_string(report.package_root.join("scene.json")).expect("read scene.json");

        // Pretty-printed JSON: more than one line, has the canonical
        // 2-space indent the serde_json::to_string_pretty default
        // produces.
        let lines: Vec<&str> = body.lines().collect();
        assert!(
            lines.len() > 5,
            "pretty-printed scene.json should be multi-line, got {} line(s)",
            lines.len(),
        );
        assert!(
            body.contains("  \"TITLE\":"),
            "pretty-printed scene.json should carry the 2-space indent",
        );

        cleanup(&target);
    }
}
