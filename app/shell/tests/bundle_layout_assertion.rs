// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Bundle-walk assertion corpus for the four-subdirectory bundled
//! scenes taxonomy per `docs/38-scene-library-and-import-spec.md` §2.1.
//!
//! Wave-4b.1 closes `EV-LIBRARY-BUNDLE-LAYOUT`
//! (`docs/26-evidence-artifact-registry.md` row 101): the bundled
//! scene root ships at
//! `<AppBundle>/Resources/scenes/{welcome,library,examples,shadertoy-fixtures}/`.
//! `bundle.resources` in `app/shell/tauri.conf.json` maps the source
//! tree under `engine/scene/tests/fixtures/scenes/` directly to that
//! AppBundle layout — every destination path under `scenes/<subdir>/`
//! pulls from the same `<subdir>/` in the source tree. The cross-seam
//! contract is therefore "the source tree honors the four-subdirectory
//! layout"; verifying that contract here pins the same invariant the
//! AppBundle ships at link time.
//!
//! ## Why we do not call `bundled_scene_root_impl` against `mock_app()`
//!
//! `tauri::test::mock_app()` returns an `App<MockRuntime>` whose
//! `path().resource_dir()` resolves to a test-runner scratch directory,
//! not to the post-link `<AppBundle>/Resources/`. The
//! `bundled_scene_root_impl` helper in
//! `app/shell/src/commands/library.rs` is a thin wrapper around
//! `resource_dir().join("scenes")`; calling it against the mock app
//! would assert presence of the taxonomy in a directory the mock
//! runtime does not populate, which is not what `EV-LIBRARY-BUNDLE-LAYOUT`
//! demands. The same stand-in pattern that
//! `list_scene_packages_bundled_root_yields_bundled_source_cards` uses
//! in `library.rs::tests` is the right shape: assert against the
//! source-of-truth fixtures directory that `bundle.resources` maps
//! verbatim into the AppBundle.
//!
//! Evidence: EV-LIBRARY-BUNDLE-LAYOUT (T2)

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::{Path, PathBuf};

/// Root of the source-of-truth bundled scenes tree. `bundle.resources`
/// in `app/shell/tauri.conf.json` maps every entry below this root
/// directly into `<AppBundle>/Resources/scenes/<...>/` at link time, so
/// asserting taxonomy at this path also pins the post-link bundle
/// layout.
fn bundled_scenes_source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/scenes")
}

/// Names of the canonical four subdirectories per `docs/38` §2.1.
const TAXONOMY_SUBDIRS: &[&str] = &["welcome", "library", "examples", "shadertoy-fixtures"];

#[test]
fn bundle_resources_root_contains_four_canonical_subdirectories() {
    // Evidence: EV-LIBRARY-BUNDLE-LAYOUT (T2) — every taxonomy
    // subdirectory listed in `docs/38` §2.1 must exist under the
    // source-of-truth bundled scenes root that `bundle.resources` maps
    // into `<AppBundle>/Resources/scenes/`.
    let root = bundled_scenes_source_root();
    assert!(
        root.is_dir(),
        "bundled scenes root must exist: {}",
        root.display()
    );
    for subdir in TAXONOMY_SUBDIRS {
        let path = root.join(subdir);
        assert!(
            path.is_dir(),
            "taxonomy subdirectory `{subdir}` must exist at {} per docs/38 §2.1",
            path.display(),
        );
    }
}

#[test]
fn bundled_library_subdir_contains_expected_demo_scenes() {
    // Evidence: EV-LIBRARY-BUNDLE-LAYOUT (T2) — the canonical
    // `library/` subdirectory carries the three Wave-10 demo `.chroma`
    // packages: kaleido (geometric), spectrum (audio-reactive), text.
    // These names are pinned by `bundle.resources` in
    // `app/shell/tauri.conf.json` and by the loader-level golden hashes
    // in `engine/scene/tests/import.rs`.
    let library = bundled_scenes_source_root().join("library");
    for demo in [
        "demo-kaleido.chroma",
        "demo-spectrum.chroma",
        "demo-text.chroma",
    ] {
        let pkg = library.join(demo);
        assert!(
            pkg.is_dir(),
            "demo package `{demo}` must exist under library/: {}",
            pkg.display(),
        );
        let scene_json = pkg.join("scene.json");
        assert!(
            scene_json.is_file(),
            "demo package `{demo}` must carry scene.json: {}",
            scene_json.display(),
        );
    }
}

#[test]
fn bundled_welcome_subdir_exists_for_future_population() {
    // Evidence: EV-LIBRARY-BUNDLE-LAYOUT (T2) — the welcome onboarding
    // scene is authored as a future Phase-4b work item; the taxonomy
    // directory is pinned now with a README placeholder so the
    // four-subdirectory layout exists at Wave-4b.1 closure.
    let welcome = bundled_scenes_source_root().join("welcome");
    assert!(
        welcome.is_dir(),
        "welcome/ subdirectory must exist: {}",
        welcome.display(),
    );
    let readme = welcome.join("README.md");
    assert!(
        readme.is_file(),
        "welcome/README.md must surface taxonomy intent: {}",
        readme.display(),
    );
}

#[test]
fn bundled_examples_subdir_exists_for_future_population() {
    // Evidence: EV-LIBRARY-BUNDLE-LAYOUT (T2) — the per-feature
    // examples corpus is authored across future Phase-4b waves and
    // linked from the embedded docs per ADR-010; the taxonomy
    // directory is pinned now with a README placeholder.
    let examples = bundled_scenes_source_root().join("examples");
    assert!(
        examples.is_dir(),
        "examples/ subdirectory must exist: {}",
        examples.display(),
    );
    let readme = examples.join("README.md");
    assert!(
        readme.is_file(),
        "examples/README.md must surface taxonomy intent: {}",
        readme.display(),
    );
}

#[test]
fn bundled_shadertoy_fixtures_subdir_exists_for_future_population() {
    // Evidence: EV-LIBRARY-BUNDLE-LAYOUT (T2) — the Shadertoy iChannel
    // fixture pack is authored under `EV-IMPORT-SHADERTOY` (`docs/26`
    // row 64); the taxonomy directory is pinned now with a README
    // placeholder. The bundled-list walk in
    // `app/shell/src/commands/library.rs::list_bundled_scene_packages`
    // intentionally skips this subdirectory because per `docs/38`
    // §5.2a it carries import-flow asset inputs, not playable scenes.
    let fixtures = bundled_scenes_source_root().join("shadertoy-fixtures");
    assert!(
        fixtures.is_dir(),
        "shadertoy-fixtures/ subdirectory must exist: {}",
        fixtures.display(),
    );
    let readme = fixtures.join("README.md");
    assert!(
        readme.is_file(),
        "shadertoy-fixtures/README.md must surface taxonomy intent: {}",
        readme.display(),
    );
}
