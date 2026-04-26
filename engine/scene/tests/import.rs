// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Evidence: EV-SCENE-IMPORT (T1) — scene-import golden harness;
//! Phase-1 forward pin against the bundled demo `.chroma` corpus.
//!
//! Loads each demo `.chroma` fixture under
//! `engine/scene/tests/fixtures/scenes/` and pins a canonical-form
//! FNV-1a 64-bit golden hash on its deserialized [`Scene`]
//! representation. This is the T1 drift-pin tier per
//! `docs/43-development-standards.md` §15.5; the T2 corpus coverage
//! that closes `EV-SCENE-IMPORT` (per `docs/26-evidence-artifact-registry.md`
//! row "Scene import harness + golden corpus fixtures") is W11 work
//! covering the CP-01 missing-`TYPE` fallback branches per
//! `docs/28-implementation-contract-pack.md` §CP-01.
//!
//! # Regenerating goldens
//!
//! When a fixture's content changes (e.g. a clean-room edit to the
//! authored corpus), run:
//!
//! ```text
//! cargo test -p chromaticity-scene --test import 2>&1 | grep "actual:"
//! ```
//!
//! Each failed `golden_*` test prints `actual: 0x...` in its assertion
//! message; copy the new hash into the matching `EXPECTED` constant
//! and re-run. The fourth test
//! (`loader_extracts_extensions_metadata`) does not need regeneration —
//! it asserts known string values from the fixtures and tracks the
//! same `extensions.get(...)` path the W9 library scanner uses in
//! `app/shell/src/commands/library.rs`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::PathBuf;

use chromaticity_scene::loader::load_chroma_dir;
use chromaticity_scene::schema::Scene;

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/scenes");

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(FIXTURE_DIR);
    p.push(name);
    p
}

// FNV-1a 64-bit. Mirrors `engine/runtime/tests/determinism/main.rs`.
const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Canonical form: serialize the `Scene` to a `serde_json::Value`,
/// recursively sort every object's keys, then re-serialize. Stable
/// across HashMap / serde_json::Map iteration-order quirks so the
/// golden hash is reproducible across compilers and platforms.
fn canonical_scene_bytes(scene: &Scene) -> Vec<u8> {
    let value = serde_json::to_value(scene).expect("scene must serialize to value");
    let canonical = sort_value_keys(value);
    serde_json::to_vec(&canonical).expect("canonical value must serialize to bytes")
}

fn sort_value_keys(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            let mut sorted: std::collections::BTreeMap<String, Value> =
                std::collections::BTreeMap::new();
            for (k, val) in map {
                sorted.insert(k, sort_value_keys(val));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sort_value_keys).collect()),
        other => other,
    }
}

fn load_and_hash(name: &str) -> u64 {
    let scene = load_chroma_dir(fixture_path(name)).expect("fixture must load");
    let bytes = canonical_scene_bytes(&scene);
    fnv1a_64(&bytes)
}

#[test]
fn golden_demo_kaleido() {
    let actual = load_and_hash("demo-kaleido.chroma");
    const EXPECTED: u64 = 0x0697_B269_BE16_8551;
    assert_eq!(
        actual, EXPECTED,
        "demo-kaleido golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_demo_spectrum() {
    let actual = load_and_hash("demo-spectrum.chroma");
    const EXPECTED: u64 = 0x4E43_612C_C703_0A58;
    assert_eq!(
        actual, EXPECTED,
        "demo-spectrum golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_demo_text() {
    let actual = load_and_hash("demo-text.chroma");
    const EXPECTED: u64 = 0x5AA7_78E5_7A91_01BE;
    assert_eq!(
        actual, EXPECTED,
        "demo-text golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn loader_extracts_extensions_metadata() {
    // Mirrors the `extensions.get(...)` path the W9 library scanner
    // uses in `app/shell/src/commands/library.rs` (per PR landing the
    // `list_scene_packages` IPC handler), pinning the canonical-model
    // metadata-field surfacing for each demo fixture.
    let kaleido = load_chroma_dir(fixture_path("demo-kaleido.chroma")).expect("kaleido must load");
    assert_eq!(
        kaleido.extensions.get("TITLE").and_then(|v| v.as_str()),
        Some("Demo Kaleido"),
    );
    assert_eq!(
        kaleido.extensions.get("CREDIT").and_then(|v| v.as_str()),
        Some("chromaticity demo"),
    );

    let spectrum =
        load_chroma_dir(fixture_path("demo-spectrum.chroma")).expect("spectrum must load");
    let tags: Vec<&str> = spectrum
        .extensions
        .get("TAGS")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    assert!(
        tags.contains(&"audio"),
        "demo-spectrum should carry 'audio' tag, got {tags:?}",
    );

    let text = load_chroma_dir(fixture_path("demo-text.chroma")).expect("text must load");
    let categories: Vec<&str> = text
        .extensions
        .get("CATEGORIES")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    assert!(
        categories.contains(&"TEXT"),
        "demo-text should carry 'TEXT' category, got {categories:?}",
    );
}
