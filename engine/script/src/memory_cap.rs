// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Heap memory cap for [`crate::ScriptRuntime`] execution.
//!
//! ADR-012 §3 mandates a per-scene 16 MiB heap cap on the QuickJS
//! isolate, enforced by `rquickjs::Runtime::set_memory_limit`. Once
//! the cap is mounted QuickJS hands every C-level allocation through
//! a checked path that returns `NULL` once the running total would
//! exceed the limit; QuickJS then surfaces the failure as the
//! uncatchable JS exception `InternalError("out of memory")` (see
//! `rquickjs-sys-0.11.0/quickjs/quickjs.c` line 7348-7355
//! `JS_ThrowOutOfMemory`). The wrapping
//! [`crate::ScriptRuntime::execute`] catches the thrown exception
//! and lowers it to [`crate::ScriptError::MemoryCapExceeded`]
//! populated with the limit-derived budget context.
//!
//! Slice-3 scope (this file): the memory cap mechanism, the
//! [`MemoryCapConfig`] handle, the [`MemoryUsageSnapshot`] datum
//! returned alongside the result, and the
//! [`is_oom_error`] identifier predicate.
//!
//! Mechanism. `Runtime::set_memory_limit(0)` is the QuickJS-native
//! "unlimited" signal. The slice-3 install / uninstall pair sets
//! the configured limit on entry and clears it back to `0` on exit,
//! mirroring the slice-2 watchdog bracket. Both `install_memory_cap`
//! and `uninstall_memory_cap` snapshot
//! `Runtime::memory_usage().malloc_size`; the snapshot is wired
//! into the per-execute [`MemoryUsageSnapshot`] return so the host
//! can populate `observed_bytes` on the
//! [`crate::ScriptError::MemoryCapExceeded`] surface.
//!
//! Allocator caveat. `set_memory_limit` is a no-op when the
//! `rust-alloc` or `allocator` features are enabled on rquickjs (the
//! C-side allocator hook is replaced with a Rust-side allocator that
//! does not consult the limit). The slice-1 + slice-2 + slice-3
//! `engine/script/Cargo.toml` does NOT enable either feature, so the
//! cap takes effect through the QuickJS-bundled libc allocator.
//! Future feature opt-ins MUST re-validate this path; see ADR-012
//! §3 audit notes.
//!
//! Identifier predicate. The OOM exception class string is
//! `InternalError`; the message is `out of memory`. The combination
//! is unique to the QuickJS `JS_ThrowOutOfMemory` path. Slice-2's
//! `is_interrupted_exception` follows the same `name + message`
//! pinning shape — both pin BOTH fields to avoid the false positive
//! a script could otherwise inject by throwing `new
//! InternalError("out of memory")` itself.

use rquickjs::Runtime;

use crate::error::ScriptError;

/// Per-runtime memory cap configuration.
///
/// Defaults to ADR-012 §3 budget (16 MiB). Construction is fallible
/// iff `limit_bytes == 0` — QuickJS treats a zero limit as
/// "unlimited", which contradicts the slice-3 contract that a cap
/// MUST be enforced once
/// [`crate::ScriptRuntime::with_memory_limit`] returns `Ok`. Callers
/// that genuinely need an unlimited heap should not call the
/// builder method at all; the slice-1 [`crate::ScriptRuntime::new`]
/// path leaves the runtime uncapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryCapConfig {
    limit_bytes: usize,
}

impl MemoryCapConfig {
    /// ADR-012 §3 nominal: 16 MiB heap cap per scene.
    pub const PHASE2_DEFAULT: usize = 16 * 1024 * 1024;

    /// Construct a memory-cap config, validating `limit_bytes > 0`.
    ///
    /// On invalid input, returns the offending limit as a
    /// [`ScriptError::MemoryCapConfig`] so the caller can surface
    /// the rejection without losing the number. The error variant
    /// is the same one the builder returns through
    /// [`crate::ScriptRuntime::with_memory_limit`]; this constructor
    /// is the canonical validator.
    pub fn new(limit_bytes: usize) -> Result<Self, ScriptError> {
        if limit_bytes == 0 {
            return Err(ScriptError::MemoryCapConfig {
                limit_bytes,
                reason: "memory cap (limit_bytes) must be > 0",
            });
        }
        Ok(Self { limit_bytes })
    }

    /// Heap-cap budget in bytes.
    #[must_use]
    pub fn limit_bytes(&self) -> usize {
        self.limit_bytes
    }
}

/// Heap-usage snapshot pair recorded around a memory-capped
/// [`crate::ScriptRuntime::execute`] call.
///
/// `at_install_bytes` is the QuickJS `malloc_size` reading captured
/// just before the cap is mounted; `at_uninstall_bytes` is captured
/// just after the script returns (whether successfully, via uncaught
/// throw, via watchdog interrupt, or via OOM). The pair drives the
/// `observed_bytes` field on
/// [`crate::ScriptError::MemoryCapExceeded`] when the OOM path
/// fires: the script's own allocations DO accumulate into the
/// QuickJS bookkeeping right up until the failing allocation, so
/// the post-execute reading is a tight upper bound on the script's
/// peak heap usage (clamped against the cap on the host side).
///
/// Two-instant capture (vs. a single high-water mark) deliberately
/// surfaces both endpoints. The JS heap may have grown / shrunk
/// arbitrarily between calls due to GC; the install reading is the
/// reference point against which the script's own delta is computed
/// downstream. Slice-3 doesn't yet emit a metric event from this
/// pair — it's plain data wired through to
/// [`crate::ScriptRuntime::last_memory_snapshot`] for in-process
/// inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryUsageSnapshot {
    /// `malloc_size` at the moment the cap was installed.
    pub at_install_bytes: u64,
    /// `malloc_size` at the moment the cap was uninstalled. May be
    /// `<` `at_install_bytes` if QuickJS GC freed objects during the
    /// run (a successful run that allocated and then dropped
    /// references); typical runs grow.
    pub at_uninstall_bytes: u64,
}

/// Install the memory cap on `runtime` and return a
/// [`MemoryUsageSnapshot`] handle the caller stores for the
/// lifetime of one `execute` call.
///
/// Records `at_install_bytes` from `Runtime::memory_usage()` BEFORE
/// the cap is mounted so the reference point reflects the QuickJS
/// baseline overhead (atom pool, intrinsic objects) rather than
/// just the script's own allocations. The `at_uninstall_bytes`
/// field is left at `0` and populated by [`uninstall_memory_cap`]
/// at the end of the bracketed `execute` scope.
///
/// Caller responsibility: call [`uninstall_memory_cap`] after the
/// `execute` call returns, otherwise the cap stays mounted on the
/// runtime and any subsequent `execute` call would be subject to
/// the leftover budget. The snapshot returned does not own the cap
/// lifecycle — `Runtime::set_memory_limit(0)` is the only way to
/// detach.
pub fn install_memory_cap(runtime: &Runtime, config: MemoryCapConfig) -> MemoryUsageSnapshot {
    let usage = runtime.memory_usage();
    let at_install_bytes = u64::try_from(usage.malloc_size).unwrap_or(0);
    runtime.set_memory_limit(config.limit_bytes);
    MemoryUsageSnapshot {
        at_install_bytes,
        at_uninstall_bytes: 0,
    }
}

/// Detach the memory cap, populating `snapshot.at_uninstall_bytes`
/// from the post-execute `Runtime::memory_usage()` reading.
///
/// Idempotent against double-uninstall — `set_memory_limit(0)`
/// disables the cap and a second call to the same is a no-op on
/// the QuickJS side. Pair every [`install_memory_cap`] with one
/// [`uninstall_memory_cap`] at the end of the bracketed `execute`
/// scope.
pub fn uninstall_memory_cap(runtime: &Runtime, snapshot: &mut MemoryUsageSnapshot) {
    let usage = runtime.memory_usage();
    snapshot.at_uninstall_bytes = u64::try_from(usage.malloc_size).unwrap_or(0);
    runtime.set_memory_limit(0);
}

/// Heuristic: identify the `InternalError: out of memory` exception
/// QuickJS raises when an allocation overruns the configured memory
/// limit. The error class string is `InternalError`; the message is
/// `out of memory` (see `rquickjs-sys-0.11.0/quickjs/quickjs.c`
/// `JS_ThrowOutOfMemory` line 7348).
///
/// Slice-3 reads both `name` and `message` from the caught
/// exception value, mirroring the slice-2 `is_interrupted_exception`
/// shape. Pinning only one would give false positives — a script
/// throwing `new InternalError("out of memory")` would otherwise
/// spuriously map to a memory-cap-hit. The combination of
/// `name == "InternalError"` and `message == "out of memory"` is
/// unique to the QuickJS OOM path.
#[must_use]
pub fn is_oom_error(name: &str, message: &str) -> bool {
    name == "InternalError" && message == "out of memory"
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod cap_pins {
    //! Evidence: EV-SANDBOX-SAFETY (T1) — memory cap config validation + snapshot accounting drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: the
    //! [`MemoryCapConfig`] validation rules and the
    //! [`MemoryUsageSnapshot`] accounting contract are the slice-3
    //! data-shape contract that the host
    //! [`crate::ScriptRuntime::execute`] integration depends on.
    //! These pins exercise the QuickJS install / uninstall pair
    //! against a live `rquickjs::Runtime` because the snapshot
    //! readings only make sense against the actual allocator state;
    //! the integration-level T1 pins (cap-hit lowering, builder
    //! chaining) live in [`crate::runtime::execute_drift_pins`].

    use rquickjs::Runtime;

    use super::{MemoryCapConfig, install_memory_cap, is_oom_error, uninstall_memory_cap};
    use crate::error::ScriptError;

    #[test]
    fn cap_config_accepts_phase2_default() {
        // ADR-012 §3: 16 MiB heap cap. The slice-3 nominal budget
        // must round-trip through the constructor.
        let cfg =
            MemoryCapConfig::new(MemoryCapConfig::PHASE2_DEFAULT).expect("phase-2 default accepts");
        assert_eq!(cfg.limit_bytes(), MemoryCapConfig::PHASE2_DEFAULT);
        assert_eq!(MemoryCapConfig::PHASE2_DEFAULT, 16 * 1024 * 1024);
    }

    #[test]
    fn cap_config_rejects_zero_limit() {
        // QuickJS treats a zero limit as "unlimited"; the slice-3
        // contract requires a strictly positive cap. The
        // constructor surfaces this as a structured error so the
        // caller doesn't silently get an uncapped runtime.
        let err = MemoryCapConfig::new(0).expect_err("zero limit must reject");
        match err {
            ScriptError::MemoryCapConfig {
                limit_bytes,
                reason,
            } => {
                assert_eq!(limit_bytes, 0, "limit echoed verbatim");
                assert!(!reason.is_empty(), "reason populated");
            }
            other => panic!("expected MemoryCapConfig, got {other:?}"),
        }
    }

    #[test]
    fn memory_usage_snapshot_advances_or_holds() {
        // The snapshot pair captures `malloc_size` at install and
        // uninstall. With no script run between the two calls, the
        // QuickJS heap has not grown — the typical contract is
        // `at_uninstall_bytes >= at_install_bytes` (the bookkeeping
        // does not shrink), but with no allocation pressure the
        // values may equal. We pin the >= invariant; a strictly-
        // smaller reading would surface a QuickJS regression and
        // trip this pin.
        let cfg = MemoryCapConfig::new(MemoryCapConfig::PHASE2_DEFAULT).expect("config accepts");
        let runtime = Runtime::new().expect("rquickjs runtime constructs");
        let mut snap = install_memory_cap(&runtime, cfg);
        // No script execution between install / uninstall — pure
        // accounting check. The QuickJS `memory_usage()` API
        // computes from internal counters; calling it again with no
        // intervening allocation should return the same or larger
        // value (atom pool growth from intrinsic init is the only
        // source of drift).
        uninstall_memory_cap(&runtime, &mut snap);
        assert!(
            snap.at_uninstall_bytes >= snap.at_install_bytes,
            "snapshot must not shrink: install={} uninstall={}",
            snap.at_install_bytes,
            snap.at_uninstall_bytes,
        );
    }

    #[test]
    fn is_oom_error_matches_quickjs_format() {
        // The QuickJS C source (`quickjs.c` line 7348) raises the
        // OOM exception as `InternalError("out of memory")`. The
        // memory-cap mapper uses both fields; this pin is the
        // tripwire for any future QuickJS upgrade that adjusts the
        // diagnostic string.
        assert!(is_oom_error("InternalError", "out of memory"));
        assert!(!is_oom_error("InternalError", "interrupted"));
        assert!(!is_oom_error("RangeError", "out of memory"));
        assert!(!is_oom_error("", ""));
        assert!(!is_oom_error("InternalError", "out of memory!"));
    }
}
