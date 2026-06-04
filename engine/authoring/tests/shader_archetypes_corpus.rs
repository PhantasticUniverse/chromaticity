// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Empirical verification of the shader-archetypes/ fixtures: each
//! `.chroma` loads through the canonical scene loader AND its `main.glsl`
//! parses + validates through the same naga GLSL 4.50 frontend +
//! IR validator pair engine/render uses on author-supplied fragments.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod shader_archetypes {
    //! Evidence: EV-RENDER-CONFORMANCE (T2) — the `shader-archetypes/` fixture
    //! corpus. Each `.chroma` loads through `chromaticity_scene::loader` and its
    //! `main.glsl` parses + validates through the same naga GLSL 4.50 frontend +
    //! IR validator pair `engine/render` uses on author-supplied fragments.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises the
    //! evidence-producing behaviour (scene-load + GLSL compile) against a fixture
    //! corpus. (Lives in `engine/authoring/tests/` because that crate carries both
    //! the `chromaticity-scene` loader dep and the naga `glsl-in` dev-dep
    //! unconditionally; folds into the engine/render render-conformance home when
    //! the live-render Slice R-0 lands.)

    use std::fs;
    use std::path::Path;

    fn naga_compile(src: &str) -> Result<(), String> {
        let mut frontend = naga::front::glsl::Frontend::default();
        let module = frontend
            .parse(
                &naga::front::glsl::Options {
                    stage: naga::ShaderStage::Fragment,
                    defines: Default::default(),
                },
                src,
            )
            .map_err(|e| format!("parse: {e:?}"))?;
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator
            .validate(&module)
            .map(|_| ())
            .map_err(|e| format!("validate: {e:?}"))
    }

    #[test]
    fn all_shader_archetypes_load_and_compile() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../scene/tests/fixtures/scenes/shader-archetypes");
        assert!(dir.is_dir(), "shader-archetypes dir missing: {dir:?}");

        let mut results: Vec<(String, Result<(), String>, Result<(), String>)> = Vec::new();
        for entry in fs::read_dir(&dir).expect("read shader-archetypes dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("chroma") {
                continue;
            }
            let slug = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let load = chromaticity_scene::loader::load_chroma_dir(&path)
                .map(|_| ())
                .map_err(|e| format!("{e:?}"));

            let compile = match fs::read_to_string(path.join("main.glsl")) {
                Ok(src) => naga_compile(&src),
                Err(e) => Err(format!("read main.glsl: {e}")),
            };

            results.push((slug, load, compile));
        }

        results.sort_by(|a, b| a.0.cmp(&b.0));
        let mut failures = 0usize;
        for (slug, load, compile) in &results {
            let l = match load {
                Ok(()) => "load OK".to_string(),
                Err(e) => format!("LOAD FAIL [{e}]"),
            };
            let c = match compile {
                Ok(()) => "compile OK".to_string(),
                Err(e) => format!("COMPILE FAIL [{e}]"),
            };
            println!("{slug}: {l} | {c}");
            if load.is_err() || compile.is_err() {
                failures += 1;
            }
        }
        assert_eq!(
            results.len(),
            14,
            "expected 14 archetypes, found {}",
            results.len()
        );
        assert_eq!(
            failures, 0,
            "{failures} shader archetype(s) failed load/compile (see stdout)"
        );
    }
}
