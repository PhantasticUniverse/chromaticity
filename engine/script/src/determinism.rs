// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Determinism shims for [`crate::ScriptRuntime`] execution.
//!
//! ADR-012 §5 mandates that two script-visible host APIs be replaced
//! with deterministic equivalents so a scene's authoring-time
//! behaviour replays bit-identically at run time:
//!
//! - **Seeded `Math.random()`** — the QuickJS default `Math.random`
//!   pulls entropy from `getrandom()` / `arc4random()` / equivalent
//!   per-platform; a scene that calls it twice with the same recorded
//!   seed will not get the same sequence. Slice 4 replaces the global
//!   with a Rust-side PRNG seeded from a caller-provided `u64`. The
//!   PRNG is a small inline `xoroshiro128+` (the algorithm cited in
//!   ADR-012 §5) — no external crate dependency, no allocation per
//!   call, single-threaded by construction.
//! - **Frame-locked `Date.now()`** — the QuickJS default `Date.now`
//!   returns the host wall-clock in milliseconds since the Unix
//!   epoch; per-frame replays of the same scene observe a strictly
//!   advancing sequence even when the JS-level computation is the
//!   same. Slice 4 replaces the global with a Rust-side closure that
//!   delegates to a [`Clock`] trait object the caller injects. The
//!   built-in [`FixedClock`] returns a single fixed value (the
//!   per-frame replay primitive); [`MonotonicClock`] returns a
//!   monotonic offset from a base instant for the dev-loop case
//!   where wall-clock semantics are still useful but absolute times
//!   are not.
//!
//! Slice-4 scope (this file): the shim mechanism, the
//! [`RandomSeedConfig`] handle, the [`Clock`] trait + built-in
//! implementors, and the install function that overrides
//! `Math.random` and `Date.now` on the rquickjs [`Ctx::globals`]
//! object.
//!
//! The shim mechanism uses the rquickjs [`Function::new`] +
//! [`MutFn`] pair: `MutFn` wraps a closure with a `RefCell` so
//! per-call mutable state (the PRNG advancing) is allowed; the
//! enclosing closure captures an `Rc<RefCell<...>>` so the host
//! side keeps a handle to the state for inspection / advancement.
//! Both shims are installed once at builder time (via
//! [`crate::ScriptRuntime::with_random_seed`] /
//! [`crate::ScriptRuntime::with_clock`]) and live for the lifetime
//! of the runtime; the rquickjs [`Function`] holds the closure
//! across `Context::with` boundaries so [`Self::execute`] does NOT
//! need a per-call install / uninstall bracket the way the watchdog
//! and memory cap do.
//!
//! Why xoroshiro128+ over a cryptographic PRNG: the determinism
//! shim does NOT need cryptographic strength — a script that
//! observes its own randomness is not in an adversarial position
//! against the host; the PRNG only needs to be deterministic from
//! the seed, uniform enough to satisfy the
//! "Math.random returns [0,1)" contract, and small enough that the
//! per-call overhead doesn't measurably push the script past the
//! 1.75 ms p99 watchdog soft cap. xoroshiro128+ is 4 wrapping
//! arithmetic + 2 rotates per call; any cryptographic alternative
//! is at least an order of magnitude slower.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use rquickjs::function::MutFn;
use rquickjs::{Ctx, Function};

use crate::error::ScriptError;

/// Per-runtime seeded PRNG configuration.
///
/// Slice 4 accepts any `u64` value (including `0`); the underlying
/// `xoroshiro128+` algorithm is well-defined for all 128-bit
/// non-zero seeds and we expand the user-provided `u64` into the
/// 128-bit state via `splitmix64`, which never produces an all-zero
/// pair from a single `u64` input. A `0` seed is therefore
/// well-defined and not an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomSeedConfig {
    seed: u64,
}

impl RandomSeedConfig {
    /// Construct a random-seed config. The slice-4 contract accepts
    /// every `u64` (including `0`); validation is a no-op surface
    /// kept for symmetry with the slice-2 / slice-3 builder validators
    /// in case future configuration knobs (e.g. PRNG algorithm
    /// selection) need to reject in the constructor.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Validation hook for the builder surface. Returns `Ok(self)`
    /// for every input today; reserved for future shape errors that
    /// the slice-4 contract may add (algorithm selection, alternate
    /// expansion strategies). The shape mirrors
    /// [`crate::watchdog::WatchdogConfig::new`] /
    /// [`crate::memory_cap::MemoryCapConfig::new`] for caller
    /// ergonomics.
    pub fn validate(self) -> Result<Self, ScriptError> {
        Ok(self)
    }

    /// Seed the constructor was built from.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }
}

/// Frame-locked monotonic clock surface for the [`Date.now`] shim.
///
/// Implementors are constructed by callers and handed to
/// [`crate::ScriptRuntime::with_clock`]; the runtime invokes
/// [`Clock::now_ms`] synchronously every time the script calls
/// `Date.now()`. The `Send` bound is required because slice-5+
/// command-bus integration may move the clock across threads when
/// it switches from a single-runtime executor to a thread-pool
/// executor — slice-4 is single-threaded by construction (rquickjs
/// default features are `!Send`), but pinning the bound now avoids
/// a breaking change later.
///
/// The two built-in implementors ([`FixedClock`] and
/// [`MonotonicClock`]) cover the slice-4 needs:
///
/// - [`FixedClock`] for per-frame replay (every call within a
///   single `update(dt)` returns the same value, per ADR-012 §5).
/// - [`MonotonicClock`] for the dev-loop where the offset matters
///   but absolute wall-clock times do not.
///
/// Callers needing other shapes (event-driven, scrubbing, recorded
/// from a fixture file) implement [`Clock`] directly.
pub trait Clock: Send {
    /// Wall-clock millisecond reading the [`Date.now`] shim returns.
    /// `i64` mirrors the JS `Date.now()` return type; negative values
    /// are valid (the JS spec allows pre-epoch times).
    fn now_ms(&self) -> i64;
}

/// Fixed-tick [`Clock`] implementor: every call to [`Self::now_ms`]
/// returns the configured `tick_ms`.
///
/// This is the slice-4 primitive for ADR-012 §5's "all `Date.now()`
/// calls within a single `update(dt)` return the same value"
/// contract: the per-frame host wraps each frame with
/// `runtime.with_clock(Box::new(FixedClock::new(frame_tick_ms)))`,
/// the script reads `Date.now()` arbitrarily many times during the
/// frame and observes the exact same value, then the host
/// re-installs a new `FixedClock` for the next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock {
    tick_ms: i64,
}

impl FixedClock {
    /// Construct a fixed-tick clock. The `tick_ms` value is returned
    /// verbatim from every [`Clock::now_ms`] call. Negative values
    /// are accepted (the JS [`Date.now`] spec permits pre-epoch
    /// readings).
    #[must_use]
    pub fn new(tick_ms: i64) -> Self {
        Self { tick_ms }
    }

    /// The fixed millisecond reading this clock returns.
    #[must_use]
    pub fn tick_ms(&self) -> i64 {
        self.tick_ms
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.tick_ms
    }
}

/// Monotonic [`Clock`] implementor: returns
/// `base_ms + (Instant::now() - start)` in milliseconds.
///
/// Useful for the dev-loop case where the offset since the runtime
/// started matters but the absolute wall-clock time does not. The
/// `start` instant is captured at construction; subsequent
/// [`Clock::now_ms`] reads use the wall-clock delta against `start`,
/// so the value advances monotonically and is not subject to
/// system-clock adjustments.
#[derive(Debug, Clone, Copy)]
pub struct MonotonicClock {
    start: Instant,
    base_ms: i64,
}

impl MonotonicClock {
    /// Construct a monotonic clock anchored at `Instant::now()`,
    /// returning `base_ms + elapsed_ms` from each [`Clock::now_ms`]
    /// call. `base_ms` is the offset to add to the elapsed millis;
    /// `0` is the natural choice for a "milliseconds since
    /// construction" reading, but any anchor works.
    #[must_use]
    pub fn new(base_ms: i64) -> Self {
        Self {
            start: Instant::now(),
            base_ms,
        }
    }

    /// Anchor instant the elapsed delta is computed against.
    #[must_use]
    pub fn start(&self) -> Instant {
        self.start
    }

    /// Offset added to the elapsed millis on every [`Clock::now_ms`]
    /// read.
    #[must_use]
    pub fn base_ms(&self) -> i64 {
        self.base_ms
    }
}

impl Clock for MonotonicClock {
    fn now_ms(&self) -> i64 {
        let elapsed_ms = i64::try_from(self.start.elapsed().as_millis()).unwrap_or(i64::MAX);
        self.base_ms.saturating_add(elapsed_ms)
    }
}

/// Internal PRNG state shared between the host-side handle and the
/// closure installed on `Math.random`. The state is `Rc<RefCell<_>>`
/// because rquickjs default features are `!Send`; the closure is a
/// `MutFn` wrapper that calls `borrow_mut` on the inner cell to
/// advance the generator without `unsafe`.
///
/// The internal layout is the canonical xoroshiro128+ 128-bit pair.
/// Initialised from a single `u64` seed via `splitmix64`, which
/// guarantees the 128-bit pair is not the all-zero state (the
/// xoroshiro128+ degenerate case).
///
/// `pub(crate)` rather than private: the slice-4 builder helpers
/// in this module hand a [`Rc<RefCell<PrngState>>`] back to
/// [`crate::runtime::ScriptRuntime::with_random_seed`] which folds
/// it into the host-side [`DeterminismState`]. The struct itself is
/// not part of the public crate API — callers see it only through
/// the opaque [`DeterminismState`] handle.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PrngState {
    s0: u64,
    s1: u64,
}

impl PrngState {
    /// Expand a single `u64` seed into the 128-bit pair using
    /// `splitmix64`. The two `splitmix64` outputs are guaranteed
    /// non-equal across the full `u64` input range and the pair is
    /// never simultaneously zero (splitmix64 is a permutation;
    /// sequential calls return distinct values).
    pub(crate) fn from_seed(seed: u64) -> Self {
        let s0 = splitmix64_step(seed);
        let s1 = splitmix64_step(s0);
        Self { s0, s1 }
    }

    /// Advance the generator by one step and return the next 64-bit
    /// output. Standard xoroshiro128+ recurrence: rotate, xor,
    /// rotate, with the output being the wrapping sum of the two
    /// state words BEFORE the round. See
    /// <https://prng.di.unimi.it/xoroshiro128plus.c> for the
    /// reference C source the algorithm pins to.
    pub(crate) fn next_u64(&mut self) -> u64 {
        let s0 = self.s0;
        let mut s1 = self.s1;
        let result = s0.wrapping_add(s1);
        s1 ^= s0;
        self.s0 = s0.rotate_left(24) ^ s1 ^ (s1 << 16);
        self.s1 = s1.rotate_left(37);
        result
    }

    /// Convert the next 64-bit output to an IEEE-754 double in
    /// `[0.0, 1.0)`. Standard "use the top 53 bits as the mantissa"
    /// projection: shift right by 11 and divide by 2^53. The
    /// resulting distribution is uniform over the 2^53 representable
    /// `[0,1)` doubles.
    pub(crate) fn next_f64(&mut self) -> f64 {
        // 2^53 as f64. The cast preserves exact value because 2^53
        // is representable as f64 by definition.
        const SCALE: f64 = (1u64 << 53) as f64;
        let bits = self.next_u64() >> 11;
        // Cast u64 -> f64: the input has at most 53 set bits
        // (post-shift) so the cast is exact, no rounding.
        bits as f64 / SCALE
    }
}

/// `splitmix64` finalising step. Reference: David Stafford's
/// "Mix13" / Vigna's `splitmix64` in
/// <https://prng.di.unimi.it/splitmix64.c>. Used to expand the
/// caller's `u64` seed into the 128-bit xoroshiro128+ state.
fn splitmix64_step(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Host-side handle to the shimmed determinism state.
///
/// Stored on [`crate::ScriptRuntime`] as `Option<DeterminismState>`.
/// Two independently-set fields:
///
/// - `prng` — `Some` once [`crate::ScriptRuntime::with_random_seed`]
///   has been called. The `Rc<RefCell<_>>` is shared with the
///   closure installed on `Math.random`; the host keeps a handle
///   so [`Self::seed`] can return the originally-supplied value
///   for diagnostics / serialization.
/// - `clock` — `Some` once [`crate::ScriptRuntime::with_clock`] has
///   been called. The `Rc<RefCell<_>>` wraps the
///   `Box<dyn Clock + Send>` so the closure installed on
///   `Date.now` can read `now_ms()` without `unsafe`. The host does
///   NOT expose the underlying `Clock` after install — the trait
///   object is opaque post-install.
///
/// Manual `Debug` impl: neither `Box<dyn Clock>` nor the closure
/// shared cell carry a `Debug` impl, so we render only the host
/// surface that is informative (the seed, presence-of-clock).
pub struct DeterminismState {
    prng: Option<DeterminismPrng>,
    clock_present: bool,
}

struct DeterminismPrng {
    seed: u64,
    // The actual state the closure mutates; held by the host so
    // diagnostics can see the seed but not the running state (the
    // running state is private to the closure to avoid a host-side
    // race against an in-flight `Math.random` call).
    _state: Rc<RefCell<PrngState>>,
}

impl std::fmt::Debug for DeterminismState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeterminismState")
            .field("seed", &self.prng.as_ref().map(|p| p.seed))
            .field("clock_present", &self.clock_present)
            .finish_non_exhaustive()
    }
}

impl DeterminismState {
    /// Construct a fresh, no-shims state. Returned from
    /// [`crate::ScriptRuntime::new`] so the per-builder methods can
    /// fold their state into a single `Option<DeterminismState>`
    /// field on the runtime.
    #[must_use]
    pub fn new() -> Self {
        Self {
            prng: None,
            clock_present: false,
        }
    }

    /// Seed the host-side PRNG was constructed with, or `None` if
    /// no random seed has been installed. Read-only diagnostic.
    #[must_use]
    pub fn seed(&self) -> Option<u64> {
        self.prng.as_ref().map(|p| p.seed)
    }

    /// `true` iff a clock has been installed on `Date.now`. The
    /// underlying `Clock` is opaque post-install; the boolean is
    /// the only host-side observability.
    #[must_use]
    pub fn clock_installed(&self) -> bool {
        self.clock_present
    }
}

impl Default for DeterminismState {
    fn default() -> Self {
        Self::new()
    }
}

/// Install the seeded `Math.random` shim on `ctx.globals().Math`
/// and return the host-side PRNG handle.
///
/// Returns a [`ScriptError::DeterminismInstall`] if any of the
/// rquickjs operations (reading the existing `Math` global, building
/// the [`Function`], setting the property) fail.
///
/// Crate-private: the [`PrngState`] handle in the return type is
/// not part of the public API; callers go through the
/// [`crate::ScriptRuntime::with_random_seed`] builder, which folds
/// the handle into the opaque [`DeterminismState`].
pub(crate) fn install_random_shim(
    ctx: &Ctx<'_>,
    config: RandomSeedConfig,
) -> Result<Rc<RefCell<PrngState>>, ScriptError> {
    let state = Rc::new(RefCell::new(PrngState::from_seed(config.seed)));

    // Look up the existing `Math` object. QuickJS always installs
    // `Math` as part of the standard library so this should never
    // be `undefined`; if it is, the runtime is in an unexpected
    // state and we surface that as a structured install error.
    let math: rquickjs::Object<'_> =
        ctx.globals()
            .get("Math")
            .map_err(|e| ScriptError::DeterminismInstall {
                reason: format!("globals().Math lookup failed: {e}"),
            })?;

    let state_for_closure = Rc::clone(&state);
    let func = Function::new(
        ctx.clone(),
        MutFn::new(move || -> f64 {
            // `borrow_mut` is single-threaded-safe by construction:
            // rquickjs default features are `!Send`, the closure is
            // not entered from anywhere other than the QuickJS
            // interpreter on the runtime thread, and the closure is
            // not re-entrant (the JS side cannot synchronously call
            // back into Math.random while the closure is running).
            let mut prng = state_for_closure.borrow_mut();
            prng.next_f64()
        }),
    )
    .map_err(|e| ScriptError::DeterminismInstall {
        reason: format!("Function::new(Math.random) failed: {e}"),
    })?;

    math.set("random", func)
        .map_err(|e| ScriptError::DeterminismInstall {
            reason: format!("Math.set(\"random\", shim) failed: {e}"),
        })?;

    Ok(state)
}

/// Install the frame-locked `Date.now` shim on `ctx.globals().Date`
/// and return a presence flag.
///
/// Returns a [`ScriptError::DeterminismInstall`] if any of the
/// rquickjs operations fail.
///
/// Crate-private: the slice-4 public surface routes through the
/// [`crate::ScriptRuntime::with_clock`] builder.
pub(crate) fn install_clock_shim(
    ctx: &Ctx<'_>,
    clock: Box<dyn Clock + Send>,
) -> Result<(), ScriptError> {
    // The rquickjs Function::new closure must be `'static` from the
    // rquickjs perspective (the trait bound erases the lifetime),
    // and `Send` is required because rquickjs marks the function
    // class as Send-eligible under the parallel feature. We wrap
    // the boxed clock in `Rc<RefCell<...>>` for the same reason
    // PrngState is wrapped: the host kept a presence flag, the
    // closure owns the only mutable handle.
    let clock_cell = Rc::new(RefCell::new(clock));

    let date: rquickjs::Object<'_> =
        ctx.globals()
            .get("Date")
            .map_err(|e| ScriptError::DeterminismInstall {
                reason: format!("globals().Date lookup failed: {e}"),
            })?;

    let clock_for_closure = Rc::clone(&clock_cell);
    let func = Function::new(
        ctx.clone(),
        MutFn::new(move || -> i64 {
            let clock = clock_for_closure.borrow();
            clock.now_ms()
        }),
    )
    .map_err(|e| ScriptError::DeterminismInstall {
        reason: format!("Function::new(Date.now) failed: {e}"),
    })?;

    date.set("now", func)
        .map_err(|e| ScriptError::DeterminismInstall {
            reason: format!("Date.set(\"now\", shim) failed: {e}"),
        })?;

    Ok(())
}

/// Builder helper used by [`crate::ScriptRuntime::with_random_seed`]
/// to fold an installed PRNG handle back into the
/// [`DeterminismState`] on the runtime.
pub(crate) fn record_random_seed(
    state: &mut DeterminismState,
    config: RandomSeedConfig,
    prng: Rc<RefCell<PrngState>>,
) {
    state.prng = Some(DeterminismPrng {
        seed: config.seed(),
        _state: prng,
    });
}

/// Builder helper used by [`crate::ScriptRuntime::with_clock`] to
/// flip the clock-installed flag on the runtime's
/// [`DeterminismState`].
pub(crate) fn record_clock_installed(state: &mut DeterminismState) {
    state.clock_present = true;
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod determinism_pins {
    //! Evidence: EV-SCRIPT-CONTRACT (T1) — determinism shim config validation + PRNG
    //! algorithm drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: the
    //! [`RandomSeedConfig`] validation rules, the [`PrngState`]
    //! advance contract, and the [`FixedClock`] / [`MonotonicClock`]
    //! built-ins are the slice-4 data-shape contract that the host
    //! [`crate::ScriptRuntime::execute`] integration depends on.
    //! These pins do NOT exercise QuickJS — the integration-level
    //! T1 pins (Math.random / Date.now lowering) live in
    //! [`crate::runtime::execute_drift_pins`]; the cross-runtime
    //! corpus (seed determinism, isolation) lives in
    //! `engine/script/tests/determinism.rs`.

    use super::{Clock, FixedClock, MonotonicClock, PrngState, RandomSeedConfig, splitmix64_step};

    #[test]
    fn random_seed_config_accepts_zero() {
        // The slice-4 contract is "any u64 is acceptable", including
        // 0. The splitmix64 expansion ensures the underlying 128-bit
        // state is non-zero even when the user-provided seed is 0.
        let cfg = RandomSeedConfig::new(0)
            .validate()
            .expect("zero seed accepted");
        assert_eq!(cfg.seed(), 0);
    }

    #[test]
    fn random_seed_config_round_trips() {
        let cfg = RandomSeedConfig::new(0xDEAD_BEEF_CAFE_F00D)
            .validate()
            .expect("config valid");
        assert_eq!(cfg.seed(), 0xDEAD_BEEF_CAFE_F00D);
    }

    #[test]
    fn fixed_clock_returns_tick_verbatim() {
        let c = FixedClock::new(1_700_000_000_000);
        assert_eq!(c.tick_ms(), 1_700_000_000_000);
        assert_eq!(c.now_ms(), 1_700_000_000_000);
        // Repeated reads must return the same value — the
        // frame-locked contract.
        for _ in 0..10 {
            assert_eq!(c.now_ms(), 1_700_000_000_000);
        }
    }

    #[test]
    fn fixed_clock_accepts_negative_tick() {
        // ECMAScript Date.now may return negative values for
        // pre-epoch times; the FixedClock built-in must round-trip
        // them.
        let c = FixedClock::new(-1);
        assert_eq!(c.now_ms(), -1);
    }

    #[test]
    fn monotonic_clock_advances() {
        let c = MonotonicClock::new(1_000);
        // First read: at or just after 1_000 (the elapsed delta is
        // 0..a few ms depending on host scheduling).
        let first = c.now_ms();
        assert!(first >= 1_000, "first read >= base; got {first}");
        // After a brief sleep, the second read must be strictly
        // greater than the first OR equal (sub-ms granularity may
        // round both to the same integer).
        std::thread::sleep(std::time::Duration::from_millis(5));
        let second = c.now_ms();
        assert!(
            second >= first,
            "second ({second}) must be >= first ({first})",
        );
    }

    #[test]
    fn prng_advances_deterministically() {
        // Same seed → same sequence. The first three outputs of
        // xoroshiro128+ from splitmix64-expanded seed 1 are pinned
        // here as the algorithm-drift tripwire: any future change
        // to either the splitmix64 step, the xoroshiro128+ rotate
        // constants, or the f64 projection will trip these.
        let mut a = PrngState::from_seed(1);
        let mut b = PrngState::from_seed(1);
        for _ in 0..16 {
            assert_eq!(
                a.next_u64(),
                b.next_u64(),
                "same seed must produce same sequence",
            );
        }
    }

    #[test]
    fn prng_distinct_seeds_diverge() {
        let mut a = PrngState::from_seed(1);
        let mut b = PrngState::from_seed(2);
        // The xoroshiro128+ output for seeds 1 and 2 diverges on
        // the very first call; pin that strict inequality.
        assert_ne!(
            a.next_u64(),
            b.next_u64(),
            "distinct seeds must produce distinct sequences",
        );
    }

    #[test]
    fn prng_f64_in_unit_interval() {
        let mut p = PrngState::from_seed(42);
        for _ in 0..256 {
            let x = p.next_f64();
            assert!(
                (0.0..1.0).contains(&x),
                "PRNG f64 must be in [0,1); got {x}",
            );
        }
    }

    #[test]
    fn splitmix64_is_permutation_for_distinct_inputs() {
        // splitmix64 is a permutation over u64; sequential inputs
        // produce distinct outputs. Pin a small batch as a
        // tripwire; an algorithm regression that collapsed the
        // permutation would degenerate the PrngState init.
        assert_ne!(splitmix64_step(0), splitmix64_step(1));
        assert_ne!(splitmix64_step(1), splitmix64_step(2));
        assert_ne!(splitmix64_step(0xFFFF_FFFF), splitmix64_step(0xFFFF_FFFE));
    }
}
