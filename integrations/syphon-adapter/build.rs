// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Build script linking against the Syphon framework on macOS per
//! ADR-021 §3 ("Framework vendor strategy").
//!
//! On macOS targets the script:
//!
//! 1. Reads the `SYPHON_FRAMEWORK_PATH` environment variable,
//!    defaulting to `/Library/Frameworks/Syphon.framework` (the
//!    canonical macOS install location per ADR-021 §3) and falling
//!    back to a vendored copy under
//!    `third_party/Syphon-Framework/build/Syphon.framework` if the
//!    primary path is absent and the vendored Xcode-built framework
//!    is present.
//! 2. Emits `cargo:rustc-link-search=framework=<resolved parent dir>`
//!    so the link step finds the framework bundle.
//! 3. Emits `cargo:rustc-link-lib=framework=Syphon` to actually
//!    link.
//!
//! On non-macOS targets the script is a no-op: no link directives
//! are emitted because the entire crate body is gated by
//! `#![cfg(target_os = "macos")]` in `src/lib.rs`. Linux + Windows
//! workspace builds therefore contribute zero items from this crate
//! and require zero framework presence.
//!
//! The framework binary itself is **NOT vendored** in the
//! chromaticity repo per ADR-021 §3 (vendoring a Mach-O binary
//! would substantially inflate the repo and bind the workspace to
//! a specific framework build). Production app builds resolve the
//! framework via the Tauri bundling step that pulls
//! `Contents/Frameworks/Syphon.framework` into the `.app` bundle.
//! The vendored source under `third_party/Syphon-Framework/`
//! exists for inspection / build-from-source purposes only; the
//! framework binary is built once via Xcode in a follow-up wave.
//!
//! CI runners that lack the framework cannot link the crate and
//! will fail at link-time. PR 1 ships the scaffold types only;
//! `cargo check -p chromaticity-syphon-adapter` does not invoke the
//! linker, so PR 1 CI exercises the type surface without requiring
//! framework presence. PR 2 introduces the first real binding-call
//! site that triggers the link step.

fn main() {
    println!("cargo:rerun-if-env-changed=SYPHON_FRAMEWORK_PATH");
    println!("cargo:rerun-if-changed=build.rs");

    // Non-macOS: the entire crate body is `#![cfg(target_os = "macos")]`
    // gated. No link directives are required and emitting them on
    // Linux / Windows would produce a confusing "framework=Syphon"
    // request the linker has no concept of. Early-return.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let framework_path = resolve_framework_path();

    // ADR-021 §10 Negative consequence bullet 2: CI runners that
    // lack the framework will fail to link the macOS Syphon-adapter
    // crate; mitigated by gating the link step on framework
    // presence (an environment-variable check in the Cargo build
    // script that emits a `cargo:warning` and skips the link step
    // when the framework is absent, allowing `cargo check` and
    // `cargo test` to run on CI runners that lack the framework).
    //
    // Phase 3 PR 1 ships the scaffold types only; no binding-call
    // sites exist yet, so the missing framework binary does not
    // trigger an unresolved-symbol error. Phase 3 PR 2 introduces
    // the first real binding-call site; from PR 2 onward, link
    // failure on a runner that lacks the framework is the surfaced
    // diagnostic for "framework path is wrong" — same posture as
    // the canonical Syphon-Framework Xcode build.
    let framework_bundle = std::path::Path::new(&framework_path);
    if !framework_bundle.exists() {
        println!(
            "cargo:warning=Syphon.framework not found at {}; skipping link directives. Set SYPHON_FRAMEWORK_PATH or build the framework via Xcode (Phase 3 PR 2 prerequisite).",
            framework_path
        );
        return;
    }

    // Emit the framework search path: cargo's rustc-link-search
    // expects the directory CONTAINING the .framework bundle, not
    // the bundle itself. Strip the bundle name to get the parent
    // search root.
    if let Some(parent) = framework_bundle.parent() {
        println!("cargo:rustc-link-search=framework={}", parent.display());
    }
    println!("cargo:rustc-link-lib=framework=Syphon");
}

/// Resolve the framework path with the precedence ADR-021 §3 names:
///
/// 1. `SYPHON_FRAMEWORK_PATH` env var if set.
/// 2. Canonical `/Library/Frameworks/Syphon.framework` install path.
/// 3. Vendored `third_party/Syphon-Framework/build/Syphon.framework`
///    fallback (built once via Xcode in a follow-up wave).
///
/// The function does not assert the path exists on disk; rustc /
/// the linker will surface the error if the resolved path lacks the
/// framework binary at link time. This separates path resolution
/// (deterministic across environments) from path validation (which
/// requires filesystem access this build script cannot universally
/// rely on under sandboxed CI).
fn resolve_framework_path() -> String {
    if let Ok(env_path) = std::env::var("SYPHON_FRAMEWORK_PATH") {
        if !env_path.is_empty() {
            return env_path;
        }
    }
    let canonical = "/Library/Frameworks/Syphon.framework";
    if std::path::Path::new(canonical).exists() {
        return canonical.to_string();
    }
    // Vendored fallback: built via Xcode against the vendored source.
    // Path is relative to the crate directory at build time. The
    // fallback is honest about being a fallback — if neither the env
    // nor the canonical install resolved, the vendored path is the
    // best-effort guess and link-time failure is the surfaced
    // diagnostic per the docstring above.
    "../../third_party/Syphon-Framework/build/Syphon.framework".to_string()
}
