// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a EV-ENTITLEMENT-OFFLINE + EV-ENTITLEMENT-REVOCATION
//! corpus. Tier declaration and dual evidence tag live on the
//! `fixtures` inner module below. The offline window check and the
//! expiry-revocation check share a clock-injection harness, so this
//! file dual-tags both rows per the multi-row dual-tagging pattern
//! the Phase-3 wave PRs surfaced.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-ENTITLEMENT-OFFLINE (T2) + EV-ENTITLEMENT-REVOCATION (T2)
    //! — Phase-4a Wave 1a 14-day offline window with valid token AND
    //! expiry-based revocation behavior at MVP (deny-list deferred per
    //! AO direction). Both rows share the
    //! [`chromaticity_entitlement::is_valid_at`] +
    //! [`chromaticity_entitlement::FixedClock`] surface; this single
    //! corpus closes both `docs/26-evidence-artifact-registry.md`
    //! row 87 and row 88.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: corpus
    //! conformance against the boundary semantics ADR-009 §Token
    //! lifetimes line 84 pins.
    //!
    //! Fixture set:
    //!
    //! 1. `token_within_14_day_window_is_honored` — issued + 7 days
    //!    falls cleanly inside the trial-token validity window.
    //! 2. `token_at_exactly_expires_at_is_no_longer_honored` —
    //!    boundary check: `now == expires_at` collapses to `false`.
    //!    ADR-009 §Token lifetimes line 84: `expires_at` is a hard
    //!    ceiling.
    //! 3. `token_one_second_after_expires_at_is_no_longer_honored`
    //!    — strict-monotone advancement past expiry collapses
    //!    immediately. Cross-cuts both EV-ENTITLEMENT-OFFLINE and
    //!    EV-ENTITLEMENT-REVOCATION (the offline window is the
    //!    interval up to expiry; revocation is what happens at
    //!    expiry).
    //! 4. `token_one_second_before_expires_at_is_still_honored` —
    //!    boundary symmetry: the last second of the validity
    //!    window must still grant.
    //! 5. `expiry_check_uses_injected_clock_for_determinism` —
    //!    exercises the [`FixedClock`] injection path explicitly:
    //!    advancing the clock through the validity window and past
    //!    expiry produces the same answers `is_valid_at` returns
    //!    when called with the same `SystemTime` directly.
    //! 6. `system_time_advancement_simulates_offline_window` —
    //!    walks `now` across the full 14-day trial window in
    //!    one-day strides and asserts the validity flips exactly
    //!    once at the expiry boundary. This is the
    //!    EV-ENTITLEMENT-OFFLINE 14-day soak in deterministic form
    //!    (no real-clock time-skew required).
    //! 7. `token_with_no_expires_at_claim_is_treated_as_invalid` —
    //!    fail-closed when the optional `expires_at` claim is
    //!    absent. ADR-009 §Token lifetimes sets a default
    //!    `expires_at = issued_at + 1 year` for paid tokens; an
    //!    omitted claim is therefore malformed.
    //! 8. `token_with_malformed_expires_at_is_treated_as_invalid`
    //!    — fail-closed when `expires_at` is not the canonical
    //!    UTC-Z RFC 3339 form (e.g. trailing `+00:00`, fractional
    //!    seconds, or empty string).

    use std::collections::BTreeMap;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use chromaticity_entitlement::{
        Clock, FixedClock, Flags, MachineBinding, Signature, Token, capability, is_valid_at,
    };

    /// Trial-shape token issued 2026-04-22T14:30:00Z, expiring 14
    /// days later at 2026-05-06T14:30:00Z. Mirrors the
    /// `trial_token_fixture` in `engine/entitlement/tests/
    /// token_fixtures.rs` so the offline-window assertions exercise
    /// the same shape the Phase-0 round-trip corpus already pins.
    fn trial_token() -> Token {
        Token {
            schema: "entitlement-v2".to_owned(),
            subject: "user-fixture-offline-001".to_owned(),
            issued_at: "2026-04-22T14:30:00Z".to_owned(),
            not_before: Some("2026-04-22T14:30:00Z".to_owned()),
            // ADR-009 line 85: trial tokens have a 14-day ceiling.
            expires_at: Some("2026-05-06T14:30:00Z".to_owned()),
            issuer: Some("chromaticity-release".to_owned()),
            kid: Some("chromaticity-release-2026-04".to_owned()),
            capabilities: vec![capability::RENDER_WATERMARK_OFF.to_owned()],
            flags: Some(Flags {
                trial: true,
                ..Flags::default()
            }),
            machine_binding: Some(MachineBinding {
                algorithm: "blake3-hmac".to_owned(),
                digest: "abcdef0123456789".repeat(4),
            }),
            session: None,
            signature: Signature {
                algorithm: "ed25519".to_owned(),
                value: "FF".repeat(32),
            },
            extra: BTreeMap::new(),
        }
    }

    // Fixture epoch constants — computed against
    // `python3 -c "import datetime; ..."` to stay reproducible
    // across CI runners.
    const ISSUED_AT_UNIX: u64 = 1_776_868_200; // 2026-04-22T14:30:00Z
    const EXPIRES_AT_UNIX: u64 = 1_778_077_800; // 2026-05-06T14:30:00Z
    const DAY_7_UNIX: u64 = 1_777_473_000; // issued + 7 days
    const ONE_SEC_BEFORE_EXPIRY_UNIX: u64 = 1_778_077_799; // expires - 1s

    fn at_unix(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn token_within_14_day_window_is_honored() {
        let token = trial_token();
        // Day 7: middle of the 14-day window.
        assert!(is_valid_at(&token, at_unix(DAY_7_UNIX)));
    }

    #[test]
    fn token_at_exactly_expires_at_is_no_longer_honored() {
        let token = trial_token();
        // Boundary check: `now == expires_at` denies. ADR-009
        // §Token lifetimes line 84: `expires_at` is a hard ceiling
        // (exclusive upper bound).
        assert!(!is_valid_at(&token, at_unix(EXPIRES_AT_UNIX)));
    }

    #[test]
    fn token_one_second_after_expires_at_is_no_longer_honored() {
        let token = trial_token();
        assert!(!is_valid_at(&token, at_unix(EXPIRES_AT_UNIX + 1)));
    }

    #[test]
    fn token_one_second_before_expires_at_is_still_honored() {
        let token = trial_token();
        // Symmetry with the boundary test: the last second of the
        // window grants. If this and `token_at_exactly_expires_at_*`
        // both hold then the boundary semantics is exact-not-fuzzy.
        assert!(is_valid_at(&token, at_unix(ONE_SEC_BEFORE_EXPIRY_UNIX)));
    }

    #[test]
    fn expiry_check_uses_injected_clock_for_determinism() {
        // Drive `is_valid_at` through `FixedClock::now()` rather
        // than feeding the `SystemTime` directly; the answers must
        // match. This is the CI-environment-system-time-mocking
        // workaround the brief calls out — `FixedClock` is the
        // injection path Phase-4a runtime evaluators consume.
        let token = trial_token();
        for unix in [DAY_7_UNIX, EXPIRES_AT_UNIX, ONE_SEC_BEFORE_EXPIRY_UNIX] {
            let clock = FixedClock::from_unix_seconds(unix);
            let direct = is_valid_at(&token, at_unix(unix));
            let via_clock = is_valid_at(&token, clock.now());
            assert_eq!(
                direct, via_clock,
                "FixedClock-injected answer diverges from direct SystemTime call \
                 at unix={unix}: direct={direct} via_clock={via_clock}",
            );
        }
    }

    #[test]
    fn system_time_advancement_simulates_offline_window() {
        // EV-ENTITLEMENT-OFFLINE 14-day soak in deterministic form.
        // Walk `now` from `issued_at` to `expires_at + 1 day` in
        // one-day strides and assert the validity flips exactly
        // once at the boundary.
        let token = trial_token();
        let mut prior_validity: Option<bool> = None;
        let mut transition_count = 0usize;
        let mut transition_at: Option<u64> = None;
        for day in 0..=15 {
            let unix = ISSUED_AT_UNIX + (day * 86_400);
            let valid = is_valid_at(&token, at_unix(unix));
            if let Some(prev) = prior_validity {
                if prev != valid {
                    transition_count += 1;
                    transition_at = Some(unix);
                }
            }
            prior_validity = Some(valid);
        }
        assert_eq!(
            transition_count, 1,
            "validity transitioned {transition_count} times across the 14-day window; \
             expected exactly one (true → false at expiry)",
        );
        // The transition must land at the day-14 boundary
        // (`issued_at + 14 days`).
        assert_eq!(
            transition_at,
            Some(ISSUED_AT_UNIX + 14 * 86_400),
            "validity transition at unexpected unix instant",
        );
    }

    #[test]
    fn token_with_no_expires_at_claim_is_treated_as_invalid() {
        // ADR-009 §Token lifetimes mandates an `expires_at` claim
        // for both paid and trial tokens; an absent claim is
        // malformed and the evaluator fails closed.
        let mut token = trial_token();
        token.expires_at = None;
        assert!(!is_valid_at(&token, at_unix(DAY_7_UNIX)));
    }

    #[test]
    fn token_with_malformed_expires_at_is_treated_as_invalid() {
        // Wave-1a evaluator pins canonical UTC-Z RFC 3339 form
        // (`YYYY-MM-DDTHH:MM:SSZ`). Anything else fails closed —
        // widening to fractional seconds or non-UTC offsets would
        // require a date crate (chrono / time), out of scope for
        // Wave-1a per the no-new-deps constraint.
        let mut token = trial_token();
        for malformed in [
            "",
            "2026-05-06T14:30:00+00:00",
            "2026-05-06T14:30:00.000Z",
            "2026-05-06 14:30:00Z",
            "not-a-date",
        ] {
            token.expires_at = Some(malformed.to_owned());
            assert!(
                !is_valid_at(&token, at_unix(DAY_7_UNIX)),
                "malformed expires_at {malformed:?} unexpectedly accepted",
            );
        }
    }
}
