// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Structured error surface for [`crate::ScriptRuntime`].
//!
//! Variants are ordered roughly by pipeline stage:
//!
//! 1. Runtime / context construction
//!    ([`ScriptError::RuntimeInit`], [`ScriptError::ContextInit`]).
//! 2. Source compilation ([`ScriptError::Compile`]).
//! 3. Source execution ([`ScriptError::Runtime`]).
//! 4. Result conversion back to a [`crate::ScriptValue`]
//!    ([`ScriptError::ValueConversion`]).
//!
//! `#[non_exhaustive]` keeps slice-2+ free to add variants without
//! a major-version bump on the crate's error surface. Reserved
//! future variants:
//!
//! - **Slice 2**: `TimeoutExceeded { phase, budget_ms }` — the
//!   watchdog fired during compilation or execution.
//! - **Slice 3**: `MemoryCapExceeded { used, cap }` — the QuickJS
//!   allocator hit the per-runtime heap cap.

use thiserror::Error;

/// Errors returned by [`crate::ScriptRuntime::execute`].
///
/// The string payloads carry the upstream rquickjs / QuickJS
/// diagnostic verbatim. Slice-1 does NOT attempt to parse line /
/// column out of the diagnostic string — QuickJS surfaces the
/// position inside its formatted message and the slice-2 corpus
/// does not yet need structured offsets. A later slice may widen
/// `Compile` to include `line: u32, col: u32` once the surface
/// stabilises around author-tooling diagnostics.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ScriptError {
    /// `rquickjs::Runtime::new` failed. Practically this only fires
    /// on heap-allocation failure during slice 1; slice 3's
    /// memory-cap mount may surface this on cap-too-small mis-config.
    #[error("rquickjs runtime init failed: {0}")]
    RuntimeInit(String),

    /// `rquickjs::Context::full` (or whichever context factory the
    /// slice mounts) failed. Same heap-allocation triage as
    /// [`ScriptError::RuntimeInit`].
    #[error("rquickjs context init failed: {0}")]
    ContextInit(String),

    /// QuickJS rejected the source at compile / parse time.
    /// Examples: `let x =` (incomplete declaration), `import('x')`
    /// when the runtime is not configured for ES-module mode (slice
    /// 1 evaluates as `Eval` mode, where `import` is a syntax
    /// error).
    #[error("script compile failed: {0}")]
    Compile(String),

    /// The script executed to completion but threw an uncaught
    /// value. The wrapped string is the QuickJS `Error.toString()`
    /// rendering — slice 4+ may extend this with a structured
    /// `name` / `message` / `stack` triple once the author-tooling
    /// surface stabilises.
    #[error("script threw uncaught value: {0}")]
    Runtime(String),

    /// The execution result could not be converted into a
    /// [`crate::ScriptValue`]. Slice 1's value surface is
    /// deliberately narrow (Null / Bool / Number / String / Array /
    /// Object); a returned `Symbol`, `BigInt`, `Function`, or
    /// `Promise` surfaces here. Later slices will widen the value
    /// vocabulary as the script-API surface lands.
    #[error("script result not representable as ScriptValue: {0}")]
    ValueConversion(String),
}
