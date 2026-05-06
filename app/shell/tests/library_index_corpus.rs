// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! End-to-end corpus assertions for the Wave 4b.2 PR 1 scene-library
//! indexer surface against the repo-resident demo corpus.
//!
//! `bundle.resources` in `app/shell/tauri.conf.json` maps every entry
//! under `engine/scene/tests/fixtures/scenes/` directly into the
//! AppBundle's `<AppBundle>/Resources/scenes/<...>/` layout at link
//! time per the `bundle_layout_assertion.rs` cross-seam contract. This
//! corpus therefore stands in for the post-link bundled-scenes tree
//! the indexer will read in production, without depending on a
//! `tauri::test::mock_app()` runtime that does not populate
//! `resource_dir().join("scenes")`.
//!
//! Evidence: EV-LIBRARY-DISCOVERY (T2) — fixture-backed corpus
//! validation per `docs/26-evidence-artifact-registry.md` row 66
//! (`Generation Source` = "Library index suite") read against
//! `docs/43` §15.5: cross-crate integration tests against real
//! fixtures qualify as T2.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_app_lib::commands::library_index::SceneIndex;
use chromaticity_ui_bridge::SceneSource;
use std::path::{Path, PathBuf};

/// Repo-resident library fixture path (the source-of-truth directory
/// `bundle.resources` maps into `<AppBundle>/Resources/scenes/library/`).
fn library_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine/scene/tests/fixtures/scenes/library")
}

#[test]
fn build_index_against_real_bundled_scenes_finds_three_demos() {
    // Evidence: EV-LIBRARY-DISCOVERY (T2) — pin the index's discovery
    // contract against the canonical Wave-4b.1 library corpus
    // (`demo-kaleido`, `demo-spectrum`, `demo-text` per the loader
    // golden in `engine/scene/tests/import.rs`).
    let index = SceneIndex::build(&[(library_fixture_root(), SceneSource::Bundled)])
        .expect("library corpus must build");
    let ids: Vec<&str> = index.cards().iter().map(|c| c.id.as_str()).collect();
    for expected in ["demo-kaleido", "demo-spectrum", "demo-text"] {
        assert!(
            ids.contains(&expected),
            "{expected} must appear in the indexed corpus, got {ids:?}"
        );
    }
}

#[test]
fn search_query_matches_against_real_corpus() {
    // Evidence: EV-LIBRARY-DISCOVERY (T2) — the search primitive must
    // surface a known title substring against the production corpus.
    let index = SceneIndex::build(&[(library_fixture_root(), SceneSource::Bundled)])
        .expect("library corpus must build");
    let hits = index.search("kaleido");
    let ids: Vec<&str> = hits.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["demo-kaleido"],
        "kaleido search must surface exactly demo-kaleido, got {ids:?}"
    );
}

#[test]
fn filter_by_tag_against_real_corpus_filters_correctly() {
    // Evidence: EV-LIBRARY-DISCOVERY (T2) — the tag filter must
    // surface only cards carrying the requested tag. demo-spectrum
    // alone carries TAGS ["audio", "reactive"] in the canonical
    // corpus.
    let index = SceneIndex::build(&[(library_fixture_root(), SceneSource::Bundled)])
        .expect("library corpus must build");
    let hits = index.filter_by_tag("audio");
    let ids: Vec<&str> = hits.iter().map(|c| c.id.as_str()).collect();
    assert!(
        ids.contains(&"demo-spectrum"),
        "demo-spectrum must surface for tag=audio, got {ids:?}"
    );
    assert!(
        !ids.contains(&"demo-text"),
        "demo-text does not carry tag=audio, got {ids:?}"
    );
    assert!(
        !ids.contains(&"demo-kaleido"),
        "demo-kaleido does not carry tag=audio, got {ids:?}"
    );
}

#[test]
fn index_size_matches_walk_count() {
    // Evidence: EV-LIBRARY-DISCOVERY (T2) — the index card count
    // must equal the directory's `.chroma` package count (the indexer
    // composes the same walker the IPC handler uses, so the per-root
    // count is the same shape). Counts the `.chroma` packages in the
    // library fixture directory directly to avoid coupling to a
    // hard-coded 3.
    let root = library_fixture_root();
    let walk_count = std::fs::read_dir(&root)
        .expect("fixture root must read")
        .flatten()
        .filter(|e| {
            let p = e.path();
            p.is_dir() && p.extension().and_then(|s| s.to_str()) == Some("chroma")
        })
        .count();
    let index =
        SceneIndex::build(&[(root, SceneSource::Bundled)]).expect("library corpus must build");
    assert_eq!(
        index.cards().len(),
        walk_count,
        "index card count must match raw .chroma walk count"
    );
}

#[test]
fn filter_by_source_segregates_bundled_versus_user() {
    // Evidence: EV-LIBRARY-DISCOVERY (T2) — the source filter must
    // bucket cards by provenance. Pointing the same fixture at two
    // sources exercises the dedup-by-id rule: the second walk's
    // cards drop because their ids collide with the first walk.
    let index = SceneIndex::build(&[
        (library_fixture_root(), SceneSource::Bundled),
        (library_fixture_root(), SceneSource::User),
    ])
    .expect("dual-source corpus must build");
    let bundled = index.filter_by_source(SceneSource::Bundled);
    let user = index.filter_by_source(SceneSource::User);
    assert_eq!(
        bundled.len(),
        index.cards().len(),
        "every surviving card carries the first-seen Bundled tag"
    );
    assert!(
        user.is_empty(),
        "User filter empty after dedup against Bundled-first walk"
    );
}
