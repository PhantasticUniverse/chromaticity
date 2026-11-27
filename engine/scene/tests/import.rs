// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SCENE-IMPORT (T1) — scene-import golden harness;
//! Phase-1 forward pin against the bundled demo `.chroma` corpus.
//! Evidence: EV-SCENE-IMPORT (T2) — reactive-symbol-family corpus
//! golden hashes pinned against the Wave-12 PR1 `.chroma` fixtures.
//! Evidence: EV-COMPAT-FIXTURE-COVERAGE (T2) — CP-01 fallback corpus
//! golden hashes pinned against the shipped `cp01-*.chroma` fixtures.
//!
//! Loads each `.chroma` fixture under
//! `engine/scene/tests/fixtures/scenes/` and pins a canonical-form
//! FNV-1a 64-bit golden hash on its deserialized [`Scene`]
//! representation.
//!
//! The four `golden_demo_*` / extensions-metadata tests stay at the T1
//! drift-pin tier per `docs/43-development-standards.md` §15.5.
//!
//! The seven `golden_t2_*` tests added in Wave-12 PR1 are the T2
//! corpus-conformance wing of `EV-SCENE-IMPORT` per `docs/26` row 25
//! and `docs/43-development-standards.md` §15.5: they pin the
//! deserialized canonical-form hash of seven reactive-symbol-family
//! fixtures (`multipass-feedback`, `multipass-chain`, `reactive-audio`,
//! `meta-controls`, `traveler-controls`, `transition-hard`,
//! `pass-sizing-sentinel`) selected against the docs/34 Coverage
//! Dimensions the slice-3a schema actually parses. Closing T2 over
//! this corpus advances `EV-SCENE-IMPORT` past `pending:artifact`
//! to Pass.
//!
//! The five `golden_cp01_*` tests added in Wave-11 PR1 are the T2
//! file-fixture wing of `EV-COMPAT-FIXTURE-COVERAGE` per `docs/26`
//! row 27 — they pin the deserialized canonical-form hash of each
//! shipped CP-01 fallback fixture (`cp01-slider`, `cp01-toggle`,
//! `cp01-color`, `cp01-xy`, `cp01-reject`) so silent drift in either
//! the schema serialization or the fixture content fails at build.
//! The companion compat-vs-strict assertions live in
//! `engine/scene/tests/cp01_compat.rs`.
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
//! and re-run. The `loader_extracts_extensions_metadata` test does not
//! need regeneration — it asserts known string values from the fixtures
//! and tracks the same `extensions.get(...)` path the W9 library
//! scanner uses in `app/shell/src/commands/library.rs`.

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
    let actual = load_and_hash("library/demo-kaleido.chroma");
    const EXPECTED: u64 = 0x0697_B269_BE16_8551;
    assert_eq!(
        actual, EXPECTED,
        "demo-kaleido golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_demo_spectrum() {
    let actual = load_and_hash("library/demo-spectrum.chroma");
    const EXPECTED: u64 = 0x4E43_612C_C703_0A58;
    assert_eq!(
        actual, EXPECTED,
        "demo-spectrum golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_demo_text() {
    let actual = load_and_hash("library/demo-text.chroma");
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
    let kaleido =
        load_chroma_dir(fixture_path("library/demo-kaleido.chroma")).expect("kaleido must load");
    assert_eq!(
        kaleido.extensions.get("TITLE").and_then(|v| v.as_str()),
        Some("Demo Kaleido"),
    );
    assert_eq!(
        kaleido.extensions.get("CREDIT").and_then(|v| v.as_str()),
        Some("chromaticity demo"),
    );

    let spectrum =
        load_chroma_dir(fixture_path("library/demo-spectrum.chroma")).expect("spectrum must load");
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

    let text = load_chroma_dir(fixture_path("library/demo-text.chroma")).expect("text must load");
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

// ---------------------------------------------------------------------------
// EV-COMPAT-FIXTURE-COVERAGE (T2) — CP-01 fallback corpus golden hashes.
//
// Each `golden_cp01_*` test pins the canonical-form FNV-1a 64-bit hash of a
// shipped `cp01-<branch>.chroma` fixture under
// `engine/scene/tests/fixtures/scenes/`. Drift in the schema serializer or
// the fixture content fails the matching test. The companion
// compat-vs-strict assertions live in
// `engine/scene/tests/cp01_compat.rs`. See `docs/26` row 27 + `docs/28`
// §CP-01 for the rule definitions, and `docs/43` §15.5 for the T2 tier.
// ---------------------------------------------------------------------------

#[test]
fn golden_cp01_slider() {
    // EV-COMPAT-FIXTURE-COVERAGE (T2)
    let actual = load_and_hash("cp01-slider.chroma");
    const EXPECTED: u64 = 0xE1E8_7D83_B56F_0D23;
    assert_eq!(
        actual, EXPECTED,
        "cp01-slider golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_cp01_toggle() {
    // EV-COMPAT-FIXTURE-COVERAGE (T2)
    let actual = load_and_hash("cp01-toggle.chroma");
    const EXPECTED: u64 = 0x0A64_B0F5_2503_93EF;
    assert_eq!(
        actual, EXPECTED,
        "cp01-toggle golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_cp01_color() {
    // EV-COMPAT-FIXTURE-COVERAGE (T2)
    let actual = load_and_hash("cp01-color.chroma");
    const EXPECTED: u64 = 0x1772_E3B6_672C_17A9;
    assert_eq!(
        actual, EXPECTED,
        "cp01-color golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_cp01_xy() {
    // EV-COMPAT-FIXTURE-COVERAGE (T2)
    let actual = load_and_hash("cp01-xy.chroma");
    const EXPECTED: u64 = 0x574D_6B1F_7584_AE3A;
    assert_eq!(
        actual, EXPECTED,
        "cp01-xy golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_cp01_reject() {
    // EV-COMPAT-FIXTURE-COVERAGE (T2)
    let actual = load_and_hash("cp01-reject.chroma");
    const EXPECTED: u64 = 0x2F3A_9DD8_5A52_670C;
    assert_eq!(
        actual, EXPECTED,
        "cp01-reject golden mismatch (actual: {actual:#018x})",
    );
}

// ---------------------------------------------------------------------------
// EV-SCENE-IMPORT (T2) — reactive-symbol-family corpus golden hashes.
//
// Each `golden_t2_*` test pins the canonical-form FNV-1a 64-bit hash of one
// Wave-12 PR1 fixture under `engine/scene/tests/fixtures/scenes/`. Together
// they exercise the docs/34 Coverage Dimensions the slice-3a schema parses:
// multipass shape (2- and 4-pass chains, per-pass UNIFORMS canonical
// bindings), the reactive-symbol family over the slice-3a representative
// subset (`syn_Spectrum`, `syn_Level`, `syn_Time`, `TIME`, `RENDERSIZE`,
// `syn_FinalPass`), CP-08 meta-control flagging via `IS_META`, ADR-008
// traveler shape ingestion via compound TYPE tokens, CP-09 transition
// declarations (parse-reserve only at slice-3a; structured execution
// deferred), and CP-10 pass-sizing sentinels (`WIDTH`/`HEIGHT` of `0` and
// absent). Closing T2 over this corpus advances `EV-SCENE-IMPORT` past
// `pending:artifact` per `docs/26` row 25 + `docs/43` §15.5.
// ---------------------------------------------------------------------------

#[test]
fn golden_t2_multipass_feedback() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("multipass-feedback.chroma");
    const EXPECTED: u64 = 0x4E69_6B09_F0B3_FEF2;
    assert_eq!(
        actual, EXPECTED,
        "multipass-feedback golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_t2_multipass_chain() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("multipass-chain.chroma");
    const EXPECTED: u64 = 0x4682_4B11_BA4B_BE97;
    assert_eq!(
        actual, EXPECTED,
        "multipass-chain golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_t2_reactive_audio() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("reactive-audio.chroma");
    const EXPECTED: u64 = 0x42F7_41DA_9256_EE7E;
    assert_eq!(
        actual, EXPECTED,
        "reactive-audio golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_t2_meta_controls() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("meta-controls.chroma");
    const EXPECTED: u64 = 0x9716_E621_FEB7_7762;
    assert_eq!(
        actual, EXPECTED,
        "meta-controls golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_t2_traveler_controls() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("traveler-controls.chroma");
    const EXPECTED: u64 = 0xAC6A_7DB9_46C2_40C3;
    assert_eq!(
        actual, EXPECTED,
        "traveler-controls golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_t2_transition_hard() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("transition-hard.chroma");
    const EXPECTED: u64 = 0x3B25_C96A_FBBD_2C94;
    assert_eq!(
        actual, EXPECTED,
        "transition-hard golden mismatch (actual: {actual:#018x})",
    );
}

#[test]
fn golden_t2_pass_sizing_sentinel() {
    // EV-SCENE-IMPORT (T2)
    let actual = load_and_hash("pass-sizing-sentinel.chroma");
    const EXPECTED: u64 = 0x1C2D_3C1F_6528_9D82;
    assert_eq!(
        actual, EXPECTED,
        "pass-sizing-sentinel golden mismatch (actual: {actual:#018x})",
    );
}
