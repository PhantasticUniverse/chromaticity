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
    //! unconditionally.)
    //!
    //! Beyond parse + validate, every fixture is checked against the engine's
    //! **std140 array convention**: `encode_uniform_slots` packs each
    //! `FloatArray` element into its own 16-byte slot, but naga reads a GLSL
    //! `float arr[N]` with a tight 4-byte stride — so a float-array uniform
    //! silently reads only every 4th band (the bug that made 6 of 8 FFT bars
    //! vanish on the diagnostic card). The shipped convention is
    //! `vec4 arr[N]` read via `.x`; [`float_array_uniform_decls`] rejects the
    //! broken pattern at test time since naga alone cannot catch it.

    use std::fs;
    use std::path::Path;

    /// Per-fixture outcome: slug, scene-load result, GLSL result (naga
    /// compile chained with the std140 convention check).
    type FixtureResult = (String, Result<(), String>, Result<(), String>);

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

    /// Find `float <name>[N]` uniform-style declarations — the std140 stride
    /// trap (see the module doc). Returns the offending declarations; empty
    /// means the source follows the `vec4 arr[N]` convention.
    fn float_array_uniform_decls(src: &str) -> Vec<String> {
        src.lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//"))
            .filter(|line| {
                // Scope the check to the declaration itself (everything from
                // `float ` on) so a `layout(binding = N)` prefix can't mask
                // it, while initialised locals (`float w[3] = ...`) stay
                // excluded via their `=`.
                line.find("float ").is_some_and(|idx| {
                    let decl = &line[idx..];
                    decl.contains('[') && decl.contains("];") && !decl.contains('=')
                })
            })
            .map(ToString::to_string)
            .collect()
    }

    /// naga compile + the engine std140 array-convention check, as one
    /// per-fixture GLSL verdict.
    fn glsl_verdict(src: &str) -> Result<(), String> {
        naga_compile(src)?;
        let bad = float_array_uniform_decls(src);
        if bad.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "std140 convention: float-array uniform(s) {bad:?} read with a \
                 4-byte stride under naga but the engine packs FloatArray as one \
                 16-byte slot per element — declare `vec4 arr[N]` and read `.x`"
            ))
        }
    }

    /// Load + GLSL-verdict every `.chroma` directly inside `dir`.
    fn check_fixture_dir(dir: &Path) -> Vec<FixtureResult> {
        let mut results: Vec<FixtureResult> = Vec::new();
        for entry in fs::read_dir(dir).expect("read fixture dir") {
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
                Ok(src) => glsl_verdict(&src),
                Err(e) => Err(format!("read main.glsl: {e}")),
            };

            results.push((slug, load, compile));
        }
        results
    }

    #[test]
    fn all_shader_archetypes_load_and_compile() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../scene/tests/fixtures/scenes/shader-archetypes");
        assert!(dir.is_dir(), "shader-archetypes dir missing: {dir:?}");

        let mut results = check_fixture_dir(&dir);

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

    #[test]
    fn diagnostic_test_card_loads_and_compiles() {
        // The audio-reactivity diagnostic card is the REFERENCE fixture for
        // the vec4-per-band std140 convention — hold it to the same bar.
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../scene/tests/fixtures/scenes/diagnostic");
        assert!(dir.is_dir(), "diagnostic fixture dir missing: {dir:?}");

        let results = check_fixture_dir(&dir);
        assert!(!results.is_empty(), "no .chroma fixtures under {dir:?}");
        for (slug, load, compile) in &results {
            assert!(load.is_ok(), "{slug}: LOAD FAIL [{load:?}]");
            assert!(compile.is_ok(), "{slug}: GLSL FAIL [{compile:?}]");
        }
    }
}
