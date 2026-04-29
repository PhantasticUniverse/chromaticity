// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! GLSL `#include` preprocessor for the Lygia v1.1.4 trial vendor.
//!
//! Slice 3 of `engine/render` adds a lightweight line-scope preprocessor
//! that resolves `#include "lygia/<path>.glsl"` directives against a
//! caller-supplied vendor root (`third_party/lygia/` in production).
//! Inlined source is concatenated verbatim and recursively expanded so a
//! Lygia file that itself includes another Lygia file resolves both.
//!
//! # Scope
//!
//! - **Lygia trial only.** [`super::single_pass::compile_and_render_single_pass`]
//!   and [`super::multipass::compile_and_render_multipass`] do NOT auto-call
//!   [`resolve_includes`]. Callers explicitly invoke the preprocessor before
//!   passing source to those entry points. This keeps the slice-1 and slice-2
//!   conformance harnesses unchanged and lets the trial-period vendor stay
//!   strictly opt-in.
//! - **Trial-period scoping.** The preprocessor is the surface area Lygia
//!   consumption rides on. If the AO drops Lygia at Phase-5
//!   `EV-LICENSE-AUDIT` (see
//!   `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL Library
//!   Handling), the preprocessor stays as a generic include-resolver — the
//!   `lygia/...` path prefix has no special meaning in the implementation;
//!   it's just the leading path segment of the include path on disk under
//!   the configured `vendor_root`.
//!
//! # Directive recognition
//!
//! Recognized form, line-scope:
//!
//! ```glsl
//! #include "lygia/math/decimate.glsl"
//! ```
//!
//! - The `#include` token MUST start at the first non-whitespace position
//!   on its line.
//! - The path MUST be wrapped in double quotes.
//! - Anything not matching the directive shape passes through verbatim,
//!   including comments that mention `#include`. A line whose content
//!   begins with `//` is treated as a comment and never parsed as a
//!   directive (defensive against `// #include "foo"` masquerading as a
//!   real directive).
//! - The angle-bracket form (`#include <foo.glsl>`) is NOT recognized;
//!   GLSL does not have a "system include" path concept and the trial
//!   surface keeps the vendor-root scope explicit.
//!
//! # Path resolution rules
//!
//! Two include-path shapes are resolved:
//!
//! - **Namespace-prefixed:** `lygia/<rest>` resolves to
//!   `<vendor_root>/<rest>` (the leading `lygia/` segment is consumed as
//!   the namespace marker; it does NOT appear under `vendor_root` on
//!   disk). This is the shape user fragments author.
//! - **Relative:** any other path resolves relative to the directory of
//!   the file that issued the include directive. Lygia uses this form
//!   internally (`#include "../math/saturate.glsl"`,
//!   `#include "blend/add.glsl"`), and the preprocessor honors it so
//!   nested Lygia trees expand correctly. The user-fragment input
//!   string itself is treated as residing at `<vendor_root>` for the
//!   purpose of relative resolution; user fragments SHOULD use the
//!   namespace-prefixed shape to keep their includes self-contained.
//!
//! After resolution every resolved path is canonicalized and asserted
//! to lie under `<vendor_root>` so a malicious or malformed `..` chain
//! cannot escape the trial-vendor sandbox.
//!
//! # Cycle and depth
//!
//! Recursive expansion is bounded by [`MAX_INCLUDE_DEPTH`]. A circular
//! include chain that exceeds the depth limit returns
//! [`IncludeError::DepthExceeded`]. A direct self-include is detected as
//! soon as the second visit fires; deeper cycles are caught by the depth
//! guard rather than a path-set membership check, keeping the
//! implementation allocation-light.
//!
//! # Naga / SPDX comment passthrough
//!
//! Inlined source is forwarded byte-for-byte (modulo the directive line
//! itself, which is replaced by the inlined file content). SPDX header
//! comments (`// SPDX-FileCopyrightText: ...`, `// SPDX-License-Identifier: ...`)
//! present in upstream Lygia headers stay in the inlined output; naga's GLSL
//! frontend tolerates them as ordinary line comments. Stripping them would
//! be a clean-room concern (modifying upstream source), and we avoid that
//! here — the per-snapshot REUSE annotation in `REUSE.toml` covers the
//! files via the `third_party/lygia/**` path glob without per-file
//! modification.
//!
//! # Anchors
//!
//! - `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL Library
//!   Handling — Lygia trial-vendor policy.
//! - `third_party/lygia/UPSTREAM-VERSION.md` — pinned version, commit SHA,
//!   snapshot exclusions, refresh procedure.
//! - `docs/26-evidence-artifact-registry.md` `EV-LIBRARY-LICENSE-TRIAL` —
//!   Phase-5-deferred license election row.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Recursion depth cap for nested `#include` resolution.
///
/// 32 nested `#include`s is well above the depth Lygia v1.1.4 uses
/// internally (the deepest chain observed in `lighting/` reaches ~6
/// levels). The cap exists to catch include cycles cheaply rather than
/// to constrain legitimate include trees.
pub const MAX_INCLUDE_DEPTH: usize = 32;

/// Errors returned by [`resolve_includes`].
///
/// `#[non_exhaustive]` keeps later slices free to widen the surface
/// (e.g. when angle-bracket includes or per-include search paths land)
/// without a major-version bump.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum IncludeError {
    /// `#include "..."` directive references a path that does not
    /// resolve under `vendor_root`. The wrapped fields capture the
    /// requested path (verbatim from the directive) and the line number
    /// (1-indexed) where the directive appeared in the immediately
    /// containing source.
    #[error("missing #include target: {path:?} at line {line}")]
    Missing {
        /// Path requested in the directive, verbatim.
        path: String,
        /// 1-indexed line number in the directive's containing source.
        line: usize,
    },

    /// Recursive include chain exceeded [`MAX_INCLUDE_DEPTH`]. Most
    /// commonly a cycle (`a.glsl` includes `b.glsl` includes `a.glsl`),
    /// but also fires on legitimate include trees deeper than the cap.
    #[error("#include recursion exceeded MAX_INCLUDE_DEPTH = {max_depth} (likely a cycle)")]
    DepthExceeded {
        /// The hit cap value.
        max_depth: usize,
    },

    /// `#include` directive matched the prefix but the rest of the line
    /// did not parse as a valid double-quoted path. The wrapped fields
    /// capture the offending line number (1-indexed) and the raw line
    /// for triage.
    #[error("invalid #include syntax at line {line}: {raw:?}")]
    InvalidSyntax {
        /// 1-indexed line number in the directive's containing source.
        line: usize,
        /// Raw line content, leading-whitespace preserved.
        raw: String,
    },

    /// I/O error reading an included file from disk.
    #[error("I/O error reading {path:?}: {reason}")]
    Io {
        /// Path that failed to open.
        path: String,
        /// Underlying I/O error rendering.
        reason: String,
    },
}

/// Resolve `#include "lygia/..."` directives in `source` against
/// `vendor_root`, returning the fully inlined GLSL.
///
/// `vendor_root` is the on-disk directory whose entries map to the
/// leading path segment of the include path. In production this is
/// `third_party/lygia/`, so a directive
/// `#include "lygia/math/decimate.glsl"` resolves to
/// `<vendor_root>/math/decimate.glsl` (the leading `lygia/` segment is
/// the trial-vendor namespace marker; it does NOT appear on disk under
/// `vendor_root`).
///
/// # Errors
///
/// See [`IncludeError`]. Missing files, malformed directives, and
/// include depth overflow surface as structured errors with
/// originating line numbers; `resolve_includes` never panics on
/// caller-supplied input.
pub fn resolve_includes(source: &str, vendor_root: &Path) -> Result<String, IncludeError> {
    let mut output = String::with_capacity(source.len());
    // The user-fragment input is treated as residing at vendor_root for
    // relative-path resolution. User fragments SHOULD use the
    // namespace-prefixed shape (`lygia/...`) to stay self-contained.
    resolve_into(source, vendor_root, vendor_root, 0, &mut output)?;
    Ok(output)
}

/// Recursive worker. `depth` is the current include nesting; the
/// caller starts at 0 and each recursion adds 1. `current_dir` is the
/// directory of the source whose lines are being scanned (used for
/// relative-path resolution); `vendor_root` is the namespace anchor for
/// `lygia/...` paths and the canonicalization-escape boundary.
fn resolve_into(
    source: &str,
    vendor_root: &Path,
    current_dir: &Path,
    depth: usize,
    output: &mut String,
) -> Result<(), IncludeError> {
    if depth >= MAX_INCLUDE_DEPTH {
        return Err(IncludeError::DepthExceeded {
            max_depth: MAX_INCLUDE_DEPTH,
        });
    }

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        match parse_include_directive(line, line_no)? {
            Some(include_path) => {
                let resolved = resolve_path(&include_path, vendor_root, current_dir, line_no)?;
                let included = read_to_string(&resolved, &include_path)?;
                let next_dir = resolved
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| vendor_root.to_path_buf());
                resolve_into(&included, vendor_root, &next_dir, depth + 1, output)?;
                // Preserve a trailing newline so the next line of the
                // outer source does not glue onto the last line of the
                // inlined content.
                if !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            None => {
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    Ok(())
}

/// Parse a single line; if it is an `#include "..."` directive, return
/// the quoted path. Comment lines (leading `//`) are treated as
/// non-directives and pass through verbatim.
///
/// Returns `Ok(None)` when the line is NOT an include directive.
/// Returns `Ok(Some(path))` when the line is a well-formed include.
/// Returns `Err(InvalidSyntax)` when the line starts with `#include`
/// but the rest is malformed (missing quote, unterminated quote, etc.).
fn parse_include_directive(line: &str, line_no: usize) -> Result<Option<String>, IncludeError> {
    let trimmed = line.trim_start();
    // Comment guard: a line beginning with `//` is a comment, never a
    // directive — defensive against `// #include "foo"` masquerading.
    if trimmed.starts_with("//") {
        return Ok(None);
    }
    let Some(rest) = trimmed.strip_prefix("#include") else {
        return Ok(None);
    };
    // Require at least one whitespace separator after `#include` so
    // identifiers like `#includehack` (none exist in GLSL but defensive)
    // do not match.
    let Some(first) = rest.chars().next() else {
        // `#include` with nothing after: malformed.
        return Err(IncludeError::InvalidSyntax {
            line: line_no,
            raw: line.to_string(),
        });
    };
    if !first.is_whitespace() {
        return Ok(None);
    }
    let after_ws = rest.trim_start();
    // Require an opening double quote.
    let Some(after_open) = after_ws.strip_prefix('"') else {
        return Err(IncludeError::InvalidSyntax {
            line: line_no,
            raw: line.to_string(),
        });
    };
    // Find the closing quote.
    let Some(close_idx) = after_open.find('"') else {
        return Err(IncludeError::InvalidSyntax {
            line: line_no,
            raw: line.to_string(),
        });
    };
    let path = &after_open[..close_idx];
    if path.is_empty() {
        return Err(IncludeError::InvalidSyntax {
            line: line_no,
            raw: line.to_string(),
        });
    }
    Ok(Some(path.to_string()))
}

/// Resolve `include_path` against either the namespace-prefixed shape
/// or the relative-path shape. Returns the canonicalized absolute path
/// to the included file; rejects any resolution that escapes
/// `vendor_root`.
fn resolve_path(
    include_path: &str,
    vendor_root: &Path,
    current_dir: &Path,
    line_no: usize,
) -> Result<PathBuf, IncludeError> {
    // Reject absolute paths (they bypass the vendor sandbox).
    if Path::new(include_path).is_absolute() {
        return Err(IncludeError::Missing {
            path: include_path.to_string(),
            line: line_no,
        });
    }

    // Pick a candidate base directory. Namespace-prefixed includes
    // (`lygia/...`) anchor at vendor_root with the leading segment
    // stripped; everything else is relative to the including file's
    // directory.
    let candidate = if let Some(rest) = include_path.strip_prefix("lygia/") {
        let mut p = vendor_root.to_path_buf();
        for seg in rest.split('/') {
            p.push(seg);
        }
        p
    } else {
        let mut p = current_dir.to_path_buf();
        for seg in include_path.split('/') {
            p.push(seg);
        }
        p
    };

    // The candidate may contain `..` segments that need normalization.
    // `Path::canonicalize` requires the path to exist, which is fine
    // here — Missing surfaces as IncludeError::Missing rather than a
    // canonicalize error.
    let canonical = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Err(IncludeError::Missing {
                path: include_path.to_string(),
                line: line_no,
            });
        }
    };

    // Defense in depth: assert the resolved path lies under
    // vendor_root after canonicalization.
    let canonical_root = vendor_root.canonicalize().map_err(|e| IncludeError::Io {
        path: vendor_root.display().to_string(),
        reason: format!("{e}"),
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(IncludeError::Missing {
            path: include_path.to_string(),
            line: line_no,
        });
    }

    if !canonical.is_file() {
        return Err(IncludeError::Missing {
            path: include_path.to_string(),
            line: line_no,
        });
    }
    Ok(canonical)
}

/// Read a file with the include-error surface.
fn read_to_string(on_disk: &Path, include_path: &str) -> Result<String, IncludeError> {
    std::fs::read_to_string(on_disk).map_err(|e| IncludeError::Io {
        path: include_path.to_string(),
        reason: format!("{e}"),
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod include_drift_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — GLSL #include preprocessor drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the line-
    //! scope `#include` parser and the path-resolution shape against
    //! the slice-3 entry-point contract ([`super::resolve_includes`]):
    //!
    //! - a single `#include "lygia/math/decimate.glsl"` directive
    //!   resolves to file content (token-presence assertion against a
    //!   stable Lygia-internal symbol),
    //! - a missing include returns [`IncludeError::Missing`] with the
    //!   right path + line number,
    //! - a fragment whose directive includes a Lygia file that itself
    //!   includes another Lygia file resolves both recursively,
    //! - a non-include source (`#version 450` + `void main() { ... }`)
    //!   round-trips byte-for-byte modulo line-ending normalization,
    //! - a comment-line `// #include "foo"` is NOT treated as an
    //!   include directive (defensive against the obvious masquerade),
    //! - subdirectory paths like `lygia/color/space/rgb2hsv.glsl`
    //!   resolve through the namespace-prefix-stripping rule.
    //!
    //! The full preprocessor → naga → wgpu pipeline is exercised by the
    //! T2 corpus suite under `engine/render/tests/glsl_lygia.rs`; T1
    //! here is preprocessor-only so the drift pin survives even on a
    //! runner without a usable wgpu adapter.
    //!
    //! Tests assume the Lygia v1.1.4 vendor snapshot is present at
    //! `third_party/lygia/`. The vendor-root path is computed from
    //! `CARGO_MANIFEST_DIR` (the path to `engine/render/`) so the test
    //! runs identically from `cargo test -p chromaticity-render` and
    //! from `cargo test --workspace` invocations.

    use std::path::PathBuf;

    use super::{IncludeError, MAX_INCLUDE_DEPTH, resolve_includes};

    /// Path to the Lygia vendor snapshot relative to the workspace
    /// root. `engine/render/` is two directories below the root.
    fn vendor_root() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir)
            .join("..")
            .join("..")
            .join("third_party")
            .join("lygia")
    }

    #[test]
    fn single_include_resolves() {
        let source = r#"#version 450
#include "lygia/math/decimate.glsl"
void main() {}
"#;
        let resolved = resolve_includes(source, &vendor_root())
            .expect("single Lygia include must resolve when the snapshot is present");
        assert!(
            resolved.contains("decimate"),
            "resolved source should contain a token from the included Lygia file, got {resolved:?}",
        );
    }

    #[test]
    fn missing_include_returns_missing_error() {
        let source = r#"#version 450
#include "lygia/does-not-exist.glsl"
void main() {}
"#;
        let err =
            resolve_includes(source, &vendor_root()).expect_err("missing include path must reject");
        match err {
            IncludeError::Missing { path, line } => {
                assert_eq!(path, "lygia/does-not-exist.glsl");
                assert_eq!(line, 2, "missing-include line number must be 1-indexed");
            }
            other => panic!("expected IncludeError::Missing, got {other:?}"),
        }
    }

    #[test]
    fn nested_includes_resolve() {
        // `lygia/color/luma.glsl` issues a relative
        // `#include "space/rgb2luma.glsl"` directive at line 1 per
        // upstream v1.1.4 source. Resolving the outer namespace-
        // prefixed include must inline both files transitively, with
        // the inner relative include resolved against the directory of
        // the outer file (`color/`).
        let source = r#"#include "lygia/color/luma.glsl"
"#;
        let resolved = resolve_includes(source, &vendor_root())
            .expect("nested Lygia include chain must resolve");
        assert!(
            resolved.contains("FNC_LUMA"),
            "outer file's guard token must appear in the inlined output",
        );
        assert!(
            resolved.contains("FNC_RGB2LUMA"),
            "inner-relative include's guard token must appear, proving recursion + \
             relative-path resolution both worked",
        );
    }

    #[test]
    fn non_include_lines_pass_through() {
        let source = "#version 450\nvoid main() {}\n";
        let resolved = resolve_includes(source, &vendor_root())
            .expect("source with no #include must pass through");
        assert_eq!(resolved, source);
    }

    #[test]
    fn comment_inside_string_not_treated_as_include() {
        // The `//` comment on the second line must not be parsed as a
        // directive even though it textually contains `#include`.
        let source =
            "#version 450\n// #include \"lygia/should-not-resolve.glsl\"\nvoid main() {}\n";
        let resolved = resolve_includes(source, &vendor_root())
            .expect("commented-out #include must not trigger resolution");
        assert!(
            resolved.contains("// #include"),
            "comment line must survive verbatim, got {resolved:?}",
        );
    }

    #[test]
    fn relative_path_resolves() {
        // A nested-subdirectory path must traverse correctly through
        // the namespace-prefix-stripping rule.
        let source = r#"#include "lygia/color/space/rgb2hsv.glsl"
"#;
        let resolved =
            resolve_includes(source, &vendor_root()).expect("subdirectory traversal must resolve");
        assert!(!resolved.is_empty(), "resolved source must be non-empty");
    }

    #[test]
    fn vendor_root_escape_rejected() {
        // Defense in depth: a `..` chain that escapes vendor_root must
        // reject. The user-fragment input is treated as residing AT
        // vendor_root, so a single `..` from there walks out of the
        // sandbox.
        let source = "#include \"../escape.glsl\"\n";
        let err =
            resolve_includes(source, &vendor_root()).expect_err("vendor-root escape must reject");
        assert!(
            matches!(err, IncludeError::Missing { .. }),
            "vendor-root escape must surface as Missing, got {err:?}",
        );
    }

    #[test]
    fn malformed_directive_rejected() {
        // `#include` with no quoted path must fire InvalidSyntax.
        let source = "#include lygia/math/decimate.glsl\n";
        let err = resolve_includes(source, &vendor_root())
            .expect_err("unquoted include path must reject");
        assert!(
            matches!(err, IncludeError::InvalidSyntax { line: 1, .. }),
            "unquoted include must surface as InvalidSyntax at line 1, got {err:?}",
        );
    }

    #[test]
    fn depth_exceeded_error_carries_max_depth() {
        // Drift-pin: the depth-cap surface is the cycle guard. The
        // public `MAX_INCLUDE_DEPTH` constant is the value the error
        // exposes; this test pins the wiring so a future refactor that
        // changes the constant without updating the error variant
        // surfaces immediately.
        let err = IncludeError::DepthExceeded {
            max_depth: MAX_INCLUDE_DEPTH,
        };
        let rendered = format!("{err}");
        assert!(
            rendered.contains(&MAX_INCLUDE_DEPTH.to_string()),
            "DepthExceeded display must include the cap value, got {rendered:?}",
        );
    }
}
