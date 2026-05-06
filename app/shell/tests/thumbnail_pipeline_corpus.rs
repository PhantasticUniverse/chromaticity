// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! End-to-end corpus assertions for the Wave 4b.3 Lane A thumbnail
//! pipeline against the repo-resident demo scenes.
//!
//! The cache + fallback chain landed in
//! `app/shell/src/commands/thumbnail.rs` (T1 drift pins) and reads
//! `(scene_id, format_version)` keys derived from the
//! [`SceneIndex`]-emitted [`SceneCard`] entries that the
//! Wave 4b.2 indexer surfaces against
//! `engine/scene/tests/fixtures/scenes/library/`. This corpus pins
//! the deterministic-regen contract end-to-end against those real
//! cards.
//!
//! Evidence: EV-LIBRARY-THUMBNAIL (T2) — fixture-backed corpus
//! validation per `docs/26-evidence-artifact-registry.md` row 67
//! (`Generation Source` = "Thumbnail pipeline + deterministic-regen
//! corpus") read against `docs/43` §15.5: cross-crate integration
//! tests against real fixtures qualify as T2.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_app_lib::commands::library_index::SceneIndex;
use chromaticity_app_lib::commands::thumbnail::{
    DeterministicStubRenderer, PLACEHOLDER_BYTES, ThumbnailCache, ThumbnailError, ThumbnailKey,
    ThumbnailRenderer, ThumbnailSource,
};
use chromaticity_ui_bridge::{SceneCard, SceneSource};
use std::path::{Path, PathBuf};

fn library_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine/scene/tests/fixtures/scenes/library")
}

fn fresh_cache_root(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "chromaticity-thumb-corpus-{}-{}",
        std::process::id(),
        tag
    ));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn key_for(card: &SceneCard) -> ThumbnailKey {
    // `format_version` composes the format tag — slice-1 only emits
    // `native-chroma-scene` so the version is constant for the
    // bundled corpus, matching the EV-LIBRARY-THUMBNAIL contract that
    // a schema bump would invalidate the cache by changing this
    // string. Once additional format tags land (compat-syn-scene,
    // shadertoy-v1-import, etc.) the format_version string can compose
    // SCHEMA_VERSION + format_tag.
    ThumbnailKey {
        scene_id: card.id.clone(),
        format_version: card.format_tag.clone(),
    }
}

fn first_demo_card() -> SceneCard {
    let index = SceneIndex::build(&[(library_fixture_root(), SceneSource::Bundled)])
        .expect("library corpus must build");
    index
        .cards()
        .iter()
        .find(|c| c.id == "demo-kaleido")
        .cloned()
        .expect("demo-kaleido must surface in the bundled corpus")
}

#[test]
fn cache_round_trip_against_real_demo_scene() {
    // Evidence: EV-LIBRARY-THUMBNAIL (T2) — pin the cache lookup +
    // write-through path against a real bundled card. First call
    // renders + caches; second call surfaces as Cached with byte-for-
    // byte equality.
    let card = first_demo_card();
    let key = key_for(&card);
    let cache = ThumbnailCache::new(
        fresh_cache_root("round-trip"),
        Box::new(DeterministicStubRenderer),
    );
    let first = cache.get_or_render(&key, &card).unwrap();
    assert_eq!(first.source, ThumbnailSource::Rendered);
    assert!(!first.bytes.is_empty(), "renderer must produce bytes");

    let second = cache.get_or_render(&key, &card).unwrap();
    assert_eq!(second.source, ThumbnailSource::Cached);
    assert_eq!(
        first.bytes, second.bytes,
        "cached read-back must match rendered bytes"
    );
}

#[test]
fn deterministic_regeneration_produces_byte_identical_thumbnails() {
    // Evidence: EV-LIBRARY-THUMBNAIL (T2) — clear the cache, render
    // twice via the render-on-demand path, and assert byte equality.
    // This is the deterministic-regen contract from `docs/26` row 67.
    let card = first_demo_card();
    let key = key_for(&card);

    let cache_a = ThumbnailCache::new(
        fresh_cache_root("regen-a"),
        Box::new(DeterministicStubRenderer),
    );
    let entry_a = cache_a.get_or_render(&key, &card).unwrap();
    assert_eq!(entry_a.source, ThumbnailSource::Rendered);

    // Independent cache root, fresh renderer instance, fresh disk.
    let cache_b = ThumbnailCache::new(
        fresh_cache_root("regen-b"),
        Box::new(DeterministicStubRenderer),
    );
    let entry_b = cache_b.get_or_render(&key, &card).unwrap();
    assert_eq!(entry_b.source, ThumbnailSource::Rendered);

    assert_eq!(
        entry_a.bytes, entry_b.bytes,
        "deterministic regeneration must produce byte-identical thumbnails"
    );
}

/// Renderer that always errors — exercises the placeholder fallback
/// arm of the cache's contract.
struct PanicRenderer;
impl ThumbnailRenderer for PanicRenderer {
    fn render(&self, _key: &ThumbnailKey, _scene: &SceneCard) -> Result<Vec<u8>, ThumbnailError> {
        Err(ThumbnailError::RenderError(
            "intentional corpus failure".into(),
        ))
    }
}

#[test]
fn placeholder_fallback_when_renderer_panics() {
    // Evidence: EV-LIBRARY-THUMBNAIL (T2) — when the renderer
    // surfaces an error against a real card, the cache must surface
    // the placeholder source + bytes per the documented fallback
    // chain.
    let card = first_demo_card();
    let key = key_for(&card);
    let cache = ThumbnailCache::new(fresh_cache_root("placeholder"), Box::new(PanicRenderer));
    let entry = cache.get_or_render(&key, &card).unwrap();
    assert_eq!(entry.source, ThumbnailSource::Placeholder);
    assert_eq!(
        entry.bytes, PLACEHOLDER_BYTES,
        "placeholder bytes must equal PLACEHOLDER_BYTES"
    );
}

#[test]
fn cache_eviction_followed_by_lookup_re_renders() {
    // Evidence: EV-LIBRARY-THUMBNAIL (T2) — evict invalidates the
    // on-disk entry such that the next lookup must re-render rather
    // than surface a stale cached read.
    let card = first_demo_card();
    let key = key_for(&card);
    let cache = ThumbnailCache::new(
        fresh_cache_root("evict-rerender"),
        Box::new(DeterministicStubRenderer),
    );

    let first = cache.get_or_render(&key, &card).unwrap();
    assert_eq!(first.source, ThumbnailSource::Rendered);
    let cached = cache.get_or_render(&key, &card).unwrap();
    assert_eq!(cached.source, ThumbnailSource::Cached);

    cache.evict(&key).unwrap();
    let after_evict = cache.get_or_render(&key, &card).unwrap();
    assert_eq!(
        after_evict.source,
        ThumbnailSource::Rendered,
        "post-evict lookup must re-render"
    );
    // Determinism is preserved across the evict → re-render cycle.
    assert_eq!(first.bytes, after_evict.bytes);
}
