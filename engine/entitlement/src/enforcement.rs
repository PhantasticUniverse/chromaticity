// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a entitlement-enforcement evaluator (MVP watermark scope).
//!
//! Realizes the runtime `entitled(capability) -> bool` surface that
//! [`super::token::Token`] declared as Phase-0-out-of-scope (per the
//! [`crate`] crate prose and `engine/entitlement/Cargo.toml` line 25).
//! Phase-4a Wave 1a narrows the surface to **watermark-on / watermark-
//! off** only — no tier gating of NDI / OSC / Syphon / custom-resolution
//! at MVP per `docs/11-roadmap-phases.md` line 125 and the AO Wave-1a
//! direction. Tier gating reuses the same evaluator surface in a future
//! wave; the only change required is widening the
//! [`HONORED_TOKEN_CAPABILITIES`] set.
//!
//! # Surface
//!
//! - [`FREE_CAPABILITIES`] — capability strings that are
//!   unconditionally available regardless of token presence
//!   (ADR-009 §Explicitly free, lines 71-79). These IDs do NOT live
//!   in [`crate::capability`] because ADR-009 line 81 mandates that
//!   no `free.*` placeholders ship in the v2 namespace; the strings
//!   are instead pinned here as the evaluator's free-set.
//! - [`HONORED_TOKEN_CAPABILITIES`] — paid capabilities the MVP
//!   evaluator actually checks against the token. At Wave-1a this is
//!   `{render.watermark.off}` per the AO narrowing. Other paid
//!   capabilities are neither free nor MVP-honored — the evaluator
//!   returns `false` for them even when present in `Token::capabilities`.
//! - [`Clock`] — clock injection trait, exists for CI determinism
//!   under hosts where `std::time` cannot be moved. [`SystemClock`]
//!   wraps `SystemTime::now()`; [`FixedClock`] returns a constant
//!   instant (the corpus uses this so the offline-window assertions
//!   stay reproducible).
//! - [`is_valid_at`] — boundary check: does the token's
//!   `expires_at` claim place `now` inside the validity window?
//!   The check is exclusive at the upper bound (`now < expires_at`)
//!   per the boundary semantics ADR-009 §Token lifetimes line 84
//!   pins (`expires_at` is a hard ceiling).
//! - [`entitled`] — the evaluator. Composes the above: free
//!   capabilities short-circuit to `true`; otherwise the token must
//!   be present, valid at `now`, AND the capability must be in
//!   [`HONORED_TOKEN_CAPABILITIES`] AND in `Token::capabilities`.
//!
//! # Fail-closed semantics (ADR-009 §Decision)
//!
//! Every code path that can short-circuit returns `false` rather than
//! `true` when input is unrecognised, malformed, or expired. Unknown
//! capability strings, unparseable `expires_at` claims, and absent
//! tokens all collapse to deny. This is the normative ADR-009
//! posture: a token whose schema fails to parse or whose expiry
//! cannot be evaluated MUST be treated as no-token-installed.
//!
//! # No-new-deps note
//!
//! The expiry check operates on the RFC 3339 `String` representation
//! that [`super::token::Token::expires_at`] carries. RFC 3339 strings
//! sort lexicographically when normalized (UTC `Z` suffix, fixed-
//! width fields), so this module compares strings rather than parsed
//! `SystemTime` values. The Phase-0 fixture corpus already pins
//! `expires_at` values to UTC `Z`-suffixed forms; widening to
//! arbitrary timezones would require a date crate (`chrono` /
//! `time`), which is intentionally not pulled in here. The
//! [`Clock::now_rfc3339`] helper formats `SystemTime` values as
//! UTC `Z`-suffixed RFC 3339 with second precision so injected
//! clocks compare cleanly against fixture token strings.
//!
//! # Phase-4a evidence rows
//!
//! - `EV-ENTITLEMENT-ENFORCEMENT` — adapter-boundary enforcement,
//!   MVP watermark scope. `engine/entitlement/tests/
//!   enforcement_corpus/main.rs`.
//! - `EV-ENTITLEMENT-OFFLINE` — 14-day offline window with valid
//!   token. `engine/entitlement/tests/offline_revocation_corpus/
//!   main.rs`.
//! - `EV-ENTITLEMENT-REVOCATION` — expiry revocation behavior at
//!   MVP (deny-list deferred). Same corpus as
//!   `EV-ENTITLEMENT-OFFLINE` (dual-tagged inner module).
//! - `EV-ENTITLEMENT-FREE-FEATURES` — explicitly-free capabilities
//!   pass with no token installed. `engine/entitlement/tests/
//!   free_features_corpus/main.rs`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::capability::RENDER_WATERMARK_OFF;
use crate::token::Token;

/// Capabilities that are unconditionally available regardless of
/// token presence, per ADR-009 §Explicitly free (lines 71-79).
///
/// These IDs deliberately do NOT live in [`crate::capability`] —
/// ADR-009 line 81 mandates that no `free.*` placeholders ship in
/// the v2 capability namespace. The evaluator's free-set is pinned
/// here so the runtime answer for any of these IDs is `true` even
/// when no token is installed and even when [`HONORED_TOKEN_CAPABILITIES`]
/// stays narrow at MVP. The strings line up with the bundled
/// `feature-dev` UX vocabulary (W30 `LICENSE_FIXTURE` declared
/// `render.pause`, `transitions.off`, `scene.import`, `scene.edit`
/// alongside the paid set; the MVP evaluator pins the free names
/// alongside playlist navigation, preset recall, snap recall/save,
/// and scene export per ADR-009 §Explicitly free).
pub const FREE_CAPABILITIES: &[&str] = &[
    "runtime.pause",
    "runtime.transitions.off",
    "playlist.next",
    "playlist.previous",
    "playlist.select",
    "preset.recall",
    "snap.recall",
    "snap.save",
    "scene.export",
];

/// Capabilities the MVP evaluator honors from a valid token. At
/// Wave-1a this is the watermark-off bit only — every other paid
/// capability stays denied at MVP per the
/// `docs/11-roadmap-phases.md` line 125 narrowing
/// ("Entitlement substrate integrated at adapter boundaries; MVP
/// watermark-on/off only; tier gating hooks wired but dormant").
/// Post-MVP wave widens this set to include OSC / Syphon / Spout /
/// NDI / camera / authoring / custom-resolution / meta-* — no
/// evaluator change is required at that point.
pub const HONORED_TOKEN_CAPABILITIES: &[&str] = &[RENDER_WATERMARK_OFF];

/// Clock injection surface. The evaluator reads `now` through this
/// trait so the offline / revocation corpus can drive a fixed
/// instant without mutating `std::time::SystemTime`.
pub trait Clock: Send + Sync {
    /// Current instant.
    fn now(&self) -> SystemTime;

    /// Current instant formatted as RFC 3339 with `Z` suffix and
    /// second precision. The default impl works against any
    /// well-formed `SystemTime`; the Phase-0 fixture corpus pins
    /// every `expires_at` to a UTC `Z`-suffixed string so the
    /// lexicographic comparison in [`is_valid_at`] is exact.
    fn now_rfc3339(&self) -> String {
        format_rfc3339_utc_z(self.now())
    }
}

/// `Clock` impl backed by `SystemTime::now()`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// `Clock` impl returning a fixed instant. Used by the offline /
/// revocation corpus to drive the evaluator across the 14-day
/// window deterministically.
#[derive(Clone, Copy, Debug)]
pub struct FixedClock {
    /// The instant this clock reports for every `now()` call.
    instant: SystemTime,
}

impl FixedClock {
    /// Build a `FixedClock` reporting the given instant.
    pub fn new(instant: SystemTime) -> Self {
        Self { instant }
    }

    /// Build a `FixedClock` from a UNIX-epoch second offset.
    pub fn from_unix_seconds(secs: u64) -> Self {
        Self {
            instant: UNIX_EPOCH + Duration::from_secs(secs),
        }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        self.instant
    }
}

/// Compile-time assertion that the clock impls (and the evaluator
/// surface that takes `&dyn Clock`) are `Send + Sync` so the
/// evaluator can be invoked from any thread per ADR-006's actor
/// boundary expectations.
const _ASSERT_CLOCKS_SEND_SYNC: fn() = || {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<SystemClock>();
    require_send_sync::<FixedClock>();
};

/// Is the token's expiry window open at `now`?
///
/// Returns `false` when:
///
/// - The token has no `expires_at` claim. The MVP evaluator does
///   not honor expiry-less tokens (ADR-009 §Token lifetimes line 84
///   sets a default `expires_at = issued_at + 1 year` for paid; an
///   absent claim is therefore treated as malformed).
/// - The `expires_at` string fails the simple shape check (not a
///   UTC `Z`-suffixed RFC 3339 form). Fail-closed.
/// - `now` is at or after `expires_at` (exclusive upper-bound check
///   per ADR-009 §Token lifetimes line 84: `expires_at` is a hard
///   ceiling).
pub fn is_valid_at(token: &Token, now: SystemTime) -> bool {
    let Some(expires_at) = token.expires_at.as_deref() else {
        return false;
    };
    if !looks_like_utc_z_rfc3339(expires_at) {
        return false;
    }
    let now_str = format_rfc3339_utc_z(now);
    // Lexicographic comparison: well-formed UTC-Z RFC 3339 strings
    // sort in chronological order. The fixture corpus pins every
    // `expires_at` value to this canonical form.
    now_str.as_str() < expires_at
}

/// MVP-watermark-scoped entitlement evaluator. Returns `true` iff
/// `capability` is held under the supplied token at `now`.
///
/// Decision tree:
///
/// 1. If `capability` is in [`FREE_CAPABILITIES`], return `true`
///    (regardless of `token` — explicit free per ADR-009 §Explicitly
///    free).
/// 2. If `token` is `None`, return `false` (no-token-installed
///    fail-closed).
/// 3. If [`is_valid_at`] returns `false` for `(token, now)`, return
///    `false` (expired or malformed-expiry fail-closed; this is the
///    `EV-ENTITLEMENT-REVOCATION` MVP path).
/// 4. If `capability` is NOT in [`HONORED_TOKEN_CAPABILITIES`],
///    return `false` (Wave-1a narrowing — every non-watermark paid
///    capability stays denied at MVP).
/// 5. If the token's `capabilities` vec contains `capability`,
///    return `true`; otherwise return `false`.
///
/// Unknown / typo capability strings collapse through step 4 and
/// return `false` (fail-closed).
pub fn entitled(token: Option<&Token>, capability: &str, now: SystemTime) -> bool {
    if FREE_CAPABILITIES.iter().any(|free| *free == capability) {
        return true;
    }
    let Some(token) = token else {
        return false;
    };
    if !is_valid_at(token, now) {
        return false;
    }
    if !HONORED_TOKEN_CAPABILITIES
        .iter()
        .any(|honored| *honored == capability)
    {
        return false;
    }
    token.capabilities.iter().any(|held| held == capability)
}

/// Format a `SystemTime` instant as RFC 3339 with `Z` suffix and
/// second precision. Pure-Rust no-deps implementation; the algorithm
/// matches the civil-from-days conversion enumerated in
/// <https://howardhinnant.github.io/date_algorithms.html> §
/// "civil_from_days" (a public-domain integer-arithmetic formulation
/// of the proleptic Gregorian calendar). Sub-second components are
/// truncated; the fixture corpus pins every `expires_at` value to
/// second precision.
fn format_rfc3339_utc_z(instant: SystemTime) -> String {
    let total_secs = match instant.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        // Pre-epoch instants are not used by the corpus; the
        // fail-closed posture treats them as the earliest
        // representable form.
        Err(err) => -(err.duration().as_secs() as i64),
    };
    let secs_per_day: i64 = 86_400;
    // Divmod that always yields a non-negative remainder.
    let mut day = total_secs.div_euclid(secs_per_day);
    let secs_in_day = total_secs.rem_euclid(secs_per_day);

    // Howard Hinnant's civil_from_days, shifted so day 0 = 1970-01-01.
    // Algorithm operates on a "shifted era" calendar starting at
    // 0000-03-01; we offset by 719468 days to match epoch 1970-01-01.
    day += 719_468;
    let era = day.div_euclid(146_097);
    let doe = (day - era * 146_097) as u64; // day-of-era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp.wrapping_sub(9) };
    let year = if m <= 2 { y + 1 } else { y };

    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let second = secs_in_day % 60;

    format!(
        "{year:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}Z",
        year = year,
        m = m,
        d = d,
        hour = hour,
        minute = minute,
        second = second,
    )
}

/// Structural shape check on an RFC 3339 / UTC-Z string.
///
/// Accepts the exact form `YYYY-MM-DDTHH:MM:SSZ` (20 ASCII bytes,
/// fixed-width fields, `Z` suffix). The fixture corpus pins every
/// `expires_at` to this form; widening to fractional-second or
/// timezone-offset variants would require a date crate, which is
/// out of scope for Wave-1a per the no-new-deps constraint.
fn looks_like_utc_z_rfc3339(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    if bytes[19] != b'Z' || bytes[10] != b'T' {
        return false;
    }
    let dashes = [4, 7];
    let colons = [13, 16];
    for &i in &dashes {
        if bytes[i] != b'-' {
            return false;
        }
    }
    for &i in &colons {
        if bytes[i] != b':' {
            return false;
        }
    }
    let digit_positions: [usize; 14] = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    for &i in &digit_positions {
        if !bytes[i].is_ascii_digit() {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod helper_pins {
    //! Evidence: EV-ENTITLEMENT-ENFORCEMENT (T1) — drift pins on the
    //! pure-helper surface ([`format_rfc3339_utc_z`],
    //! [`looks_like_utc_z_rfc3339`]) the fail-closed expiry check
    //! depends on.

    use super::*;

    #[test]
    fn rfc3339_format_round_trips_known_epoch_offsets() {
        // 2026-04-22T14:30:00Z = 1_776_868_200 seconds since epoch.
        // Calculated against `date -u -d "2026-04-22 14:30:00" +%s`.
        let formatted = format_rfc3339_utc_z(UNIX_EPOCH + Duration::from_secs(1_776_868_200));
        assert_eq!(formatted, "2026-04-22T14:30:00Z");
    }

    #[test]
    fn rfc3339_shape_check_accepts_canonical_form() {
        assert!(looks_like_utc_z_rfc3339("2026-04-22T14:30:00Z"));
    }

    #[test]
    fn rfc3339_shape_check_rejects_non_z_suffix() {
        assert!(!looks_like_utc_z_rfc3339("2026-04-22T14:30:00+00:00"));
        assert!(!looks_like_utc_z_rfc3339("2026-04-22T14:30:00.000Z"));
    }

    #[test]
    fn rfc3339_shape_check_rejects_short_strings() {
        assert!(!looks_like_utc_z_rfc3339(""));
        assert!(!looks_like_utc_z_rfc3339("2026"));
    }
}
