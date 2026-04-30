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
//! a major-version bump on the crate's error surface. Slice 2
//! lands [`ScriptError::TimeoutExceeded`] (watchdog fired during
//! execution) and [`ScriptError::WatchdogConfig`] (caller passed an
//! invalid p99 / hard pair). Slice 3 lands
//! [`ScriptError::MemoryCapExceeded`] (QuickJS allocator hit the
//! per-runtime heap cap) and [`ScriptError::MemoryCapConfig`]
//! (caller passed an invalid heap-cap budget). Slice 4 lands
//! [`ScriptError::DeterminismConfig`] (caller passed an invalid
//! seeded-PRNG / clock config) and [`ScriptError::DeterminismInstall`]
//! (rquickjs failed wiring the `Math.random` / `Date.now` global
//! shim during builder execution). Slice 5.1 lands
//! [`ScriptError::CommandBusInstall`] (rquickjs failed wiring the
//! script-side `__chromaticityEmit` global onto
//! [`rquickjs::Ctx::globals`]), [`ScriptError::CommandBusDisconnected`]
//! (the host-side [`crate::command_bus::ScriptCommandSender`] reported
//! the receive end has been dropped), and
//! [`ScriptError::CommandBusBackpressure`] (the sender shed the
//! command without enqueuing it). Slice 5.2 lands
//! [`ScriptError::ReactiveSymbolUnknown`] (the
//! [`crate::ScriptCommand::route`] caller passed a name that did
//! not resolve against the
//! `docs/36-reactive-symbol-catalog.md` v1.1.0 catalog) and
//! [`ScriptError::ReactiveValueKindMismatch`] (the carrier value's
//! [`crate::ScriptValue`] shape did not conform to the resolved
//! symbol's [`crate::ReactiveValueKind`] dimensionality slot).

use thiserror::Error;

use crate::reactive_symbol::{ReactiveValueKind, ReactiveValueKindMismatchDetail};

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

    /// The watchdog hard cap fired during execution and QuickJS
    /// raised the uncatchable `InternalError: interrupted`
    /// exception. The runtime caught the exception and lowered it
    /// to this variant. `observed_ms` is the wall-clock time the
    /// script ran for before the interrupt handler returned `true`;
    /// it is necessarily `>= hard_ms` modulo the runner clock
    /// resolution. ADR-012 §2 mandates the 1.75 ms p99 / 4 ms hard
    /// pair; slice-2 honours integer-millisecond budgets. The soft
    /// cap (`p99_ms`) is instrumentation-only and does NOT trip
    /// this variant.
    #[error(
        "script execution exceeded watchdog hard cap: observed {observed_ms} ms (p99 {p99_ms} ms / hard {hard_ms} ms)"
    )]
    TimeoutExceeded {
        /// Soft cap (p99) configured for the call, in milliseconds.
        p99_ms: u64,
        /// Hard cap configured for the call, in milliseconds.
        hard_ms: u64,
        /// Observed wall-clock execution time at interrupt, in
        /// milliseconds. Saturating-converted from
        /// [`std::time::Duration::as_millis`].
        observed_ms: u64,
    },

    /// The caller passed an invalid `(p99_ms, hard_ms)` pair to
    /// [`crate::ScriptRuntime::with_watchdog`]. The constructor
    /// validates `0 < p99_ms < hard_ms`; values outside that
    /// envelope surface here with the offending pair attached.
    /// `reason` carries a static rationale string from
    /// [`crate::watchdog::WatchdogConfigError`] for ergonomic
    /// rendering.
    #[error("watchdog config invalid: p99 {p99_ms} ms / hard {hard_ms} ms ({reason})")]
    WatchdogConfig {
        /// Offending soft cap value supplied by the caller.
        p99_ms: u64,
        /// Offending hard cap value supplied by the caller.
        hard_ms: u64,
        /// Static rationale for the rejection.
        reason: &'static str,
    },

    /// The QuickJS allocator hit the configured per-runtime heap
    /// cap during script execution. ADR-012 §3 mandates a 16 MiB
    /// cap per scene; this variant fires once an allocation would
    /// have crossed the limit and QuickJS raised the uncatchable
    /// `InternalError: out of memory` exception. The runtime
    /// caught the exception and lowered it to this variant.
    /// `observed_bytes` is the QuickJS `memory_usage().malloc_size`
    /// reading captured at host-return; it is necessarily a tight
    /// upper bound on the script's peak allocation (the failing
    /// allocation is not recorded in the bookkeeping). On the
    /// happy path of "cap was exactly hit", `observed_bytes` is
    /// usually within the cap by a small margin — QuickJS aborts
    /// the allocation BEFORE updating the running total.
    #[error(
        "script execution exceeded memory cap: observed {observed_bytes} bytes (limit {limit_bytes} bytes)"
    )]
    MemoryCapExceeded {
        /// Heap-cap budget configured for the call, in bytes.
        limit_bytes: usize,
        /// Observed QuickJS `malloc_size` at host-return, in bytes.
        /// Saturating-converted from `JSMemoryUsage::malloc_size`
        /// (`i64` upstream).
        observed_bytes: u64,
    },

    /// The caller passed an invalid `limit_bytes` to
    /// [`crate::ScriptRuntime::with_memory_limit`]. The constructor
    /// validates `limit_bytes > 0` (a zero limit is QuickJS-native
    /// for "unlimited", which contradicts the slice-3 contract that
    /// a cap MUST be enforced once the builder returns `Ok`).
    /// `reason` carries a static rationale string for ergonomic
    /// rendering.
    #[error("memory cap config invalid: limit {limit_bytes} bytes ({reason})")]
    MemoryCapConfig {
        /// Offending limit value supplied by the caller.
        limit_bytes: usize,
        /// Static rationale for the rejection.
        reason: &'static str,
    },

    /// The caller passed an invalid configuration to one of the
    /// slice-4 determinism builders
    /// ([`crate::ScriptRuntime::with_random_seed`] /
    /// [`crate::ScriptRuntime::with_clock`]). The slice-4 contract
    /// today accepts every `u64` seed (including `0`) and every
    /// [`crate::determinism::Clock`] implementor, so this variant is
    /// reserved for future configuration knobs (alternate PRNG
    /// algorithms, alternate clock contracts) that need to reject
    /// in the constructor.
    #[error("determinism config invalid: {reason}")]
    DeterminismConfig {
        /// Free-form rationale for the rejection. Owned `String`
        /// rather than `&'static str` because future config knobs
        /// may want to embed user-supplied diagnostic context.
        reason: String,
    },

    /// The slice-4 determinism shim install path failed during
    /// [`crate::ScriptRuntime::with_random_seed`] /
    /// [`crate::ScriptRuntime::with_clock`]. Surfaces an rquickjs
    /// error wiring the `Math.random` / `Date.now` global override
    /// onto the runtime's [`rquickjs::Ctx::globals`] object — e.g.
    /// the [`Function::new`] call failed, the `Math` / `Date`
    /// global was missing, or the property `set` raised.
    #[error("determinism shim install failed: {reason}")]
    DeterminismInstall {
        /// Free-form diagnostic from the rquickjs install path.
        reason: String,
    },

    /// The slice-5.1 command-bus shim install path failed during
    /// [`crate::ScriptRuntime::with_command_sender`]. Surfaces an
    /// rquickjs error wiring the `__chromaticityEmit` global function
    /// onto the runtime's [`rquickjs::Ctx::globals`] object — e.g.
    /// the [`rquickjs::Function::new`] call failed or the property
    /// `set` raised. Distinct from
    /// [`ScriptError::DeterminismInstall`] so callers can distinguish
    /// shim categories without parsing the `reason` string.
    #[error("command-bus shim install failed: {reason}")]
    CommandBusInstall {
        /// Free-form diagnostic from the rquickjs install path.
        reason: String,
    },

    /// The host-side [`crate::command_bus::ScriptCommandSender`]
    /// reported that the receive end has been dropped. The script's
    /// emit call surfaces this synchronously on the JS side as a
    /// caught Rust-throw; in slice-5.1 the QuickJS-side handler
    /// renders this category through the slice-1 `Error` path so a
    /// `try { __chromaticityEmit(...) } catch (e) { ... }` block
    /// observes the disconnection.
    #[error("script command bus disconnected")]
    CommandBusDisconnected,

    /// The host-side [`crate::command_bus::ScriptCommandSender`] is
    /// at capacity and shed the command without enqueuing it. Same
    /// JS-side surfacing rationale as
    /// [`ScriptError::CommandBusDisconnected`].
    #[error("script command bus backpressure")]
    CommandBusBackpressure,

    /// [`crate::ScriptCommand::route`] received a string-keyed
    /// [`crate::ScriptCommand::ReactiveWrite`] whose `name` did not
    /// resolve against the `docs/36-reactive-symbol-catalog.md`
    /// v1.1.0 catalog (slice 5.2). The caller decides the policy:
    /// drop the write, escalate as an error to the script author,
    /// or fall back to the catch-all
    /// [`crate::ScriptCommand::ReactiveWrite`] variant. The
    /// catch-all variant is preserved on the
    /// [`crate::ScriptCommand`] enum precisely so a host can route
    /// unresolved writes through it without needing to mutate the
    /// JS-side surface.
    #[error("reactive symbol unknown: {name}")]
    ReactiveSymbolUnknown {
        /// Verbatim name string the script supplied.
        name: String,
    },

    /// [`crate::ScriptCommand::route`] resolved the name against
    /// the `docs/36` catalog but the carrier
    /// [`crate::ScriptValue`]'s shape did not conform to the
    /// resolved symbol's [`ReactiveValueKind`] dimensionality slot
    /// (slice 5.2). The diagnostic carries the symbol's canonical
    /// name (allocation-free `&'static str` from
    /// [`crate::ReactiveSymbol::name`]), the expected
    /// [`ReactiveValueKind`], and a structured
    /// [`ReactiveValueKindMismatchDetail`] describing the actual
    /// observed shape.
    #[error(
        "reactive value kind mismatch: symbol {symbol_name} expected {expected:?} ({detail:?})"
    )]
    ReactiveValueKindMismatch {
        /// Canonical catalog name of the resolved symbol.
        symbol_name: &'static str,
        /// Expected dimensionality slot.
        expected: ReactiveValueKind,
        /// Structured observed-shape diagnostic.
        detail: ReactiveValueKindMismatchDetail,
    },
}
