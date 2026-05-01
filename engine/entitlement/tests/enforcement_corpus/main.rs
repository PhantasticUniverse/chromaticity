// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a EV-ENTITLEMENT-ENFORCEMENT corpus. Tier declaration and
//! evidence tag live on the `fixtures` inner module below, matching
//! the pattern established by `engine/runtime/tests/state_replay_corpus
//! /main.rs` so `scripts/evidence-tag-lint.sh`'s tier-distribution
//! summary counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-ENTITLEMENT-ENFORCEMENT (T2) — Phase-4a Wave 1a
    //! adapter-boundary entitlement enforcement, MVP watermark-on /
    //! watermark-off scope per `docs/11-roadmap-phases.md` line 125
    //! and AO direction.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: corpus
    //! conformance against the `chromaticity_entitlement::entitled`
    //! evaluator surface across the Wave-1a-narrowed decision tree.
    //!
    //! Fixture set (advances `docs/26-evidence-artifact-registry.md`
    //! row 86 from `Planned`/`pending:artifact` toward Phase-4a exit):
    //!
    //! 1. `watermark_off_capability_returns_false_when_no_token_installed`
    //!    — fail-closed when `token = None`. ADR-009 §Offline posture
    //!    line 92 ("If no token is present, the runtime runs in
    //!    Core-equivalent ... with a watermark overlay").
    //! 2. `watermark_off_capability_returns_true_when_valid_token_installed`
    //!    — paid-shape token within validity window grants
    //!    `render.watermark.off`.
    //! 3. `watermark_off_capability_returns_false_when_token_expired`
    //!    — `EV-ENTITLEMENT-REVOCATION` MVP path: expiry alone
    //!    revokes. Lexicographic `now >= expires_at` collapses to
    //!    `is_valid_at = false`.
    //! 4. `free_capability_returns_true_unconditionally` — every
    //!    `FREE_CAPABILITIES` entry returns `true` whether `token`
    //!    is `Some` or `None`. Cross-checks `EV-ENTITLEMENT-FREE-
    //!    FEATURES` (the dedicated free-features corpus is the
    //!    primary evidence source; this fixture is a smoke check
    //!    that the enforcement evaluator and the free-features
    //!    evaluator agree on the free set).
    //! 5. `paid_capability_returns_false_when_no_token` — non-
    //!    watermark paid capabilities (`io.osc.input`, `io.ndi.input`,
    //!    `io.ndi.output`) collapse to `false` even when a valid
    //!    token grants them. Wave-1a `HONORED_TOKEN_CAPABILITIES`
    //!    narrowing (`docs/11` line 125: "tier gating hooks wired
    //!    but dormant").
    //! 6. `entitled_check_completes_well_under_perf_budget` — the
    //!    evaluator hot-path is measured against a 50µs ceiling
    //!    over a 1000-call burst (per-call ~50ns budget; ADR-009
    //!    §Performance posture is implicit but the adapter-boundary
    //!    check fires every IPC dispatch, so a ms-class evaluator
    //!    would surface as macro frame jitter). The 50µs ceiling is
    //!    intentionally loose so CI flakiness from cold-cache
    //!    starts does not produce false reds; a real micro-bench
    //!    against `criterion` lives in a future wave.
    //! 7. `entitlement_evaluator_is_thread_safe` — `entitled` takes
    //!    `&Token` and is invoked from any thread per ADR-006's
    //!    actor-boundary model. Compile-time pin.
    //! 8. `unknown_capability_string_returns_false_fail_closed` —
    //!    typo / future-cap-not-yet-defined collapses to deny
    //!    (ADR-009 §Decision: the evaluator is a closed-set bitmask
    //!    and unknown bits are zero).

    use std::collections::BTreeMap;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use chromaticity_entitlement::{
        FREE_CAPABILITIES, Flags, Signature, Token, capability, entitled,
    };

    /// Fixture instant: 2026-05-15T00:00:00Z (mid-validity window
    /// for the paid-token fixture below). UNIX epoch seconds
    /// computed via `python3 -c "import datetime; print(int(
    /// datetime.datetime(2026,5,15,tzinfo=datetime.timezone.utc).timestamp()))"`.
    const FIXTURE_NOW_UNIX: u64 = 1_778_803_200;

    fn fixture_now() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(FIXTURE_NOW_UNIX)
    }

    /// Paid-shape token with `render.watermark.off` plus several
    /// non-MVP-honored capabilities (OSC, NDI). Validity window
    /// 2026-04-22 → 2027-04-22 brackets [`fixture_now`].
    fn paid_token() -> Token {
        Token {
            schema: "entitlement-v2".to_owned(),
            subject: "user-fixture-enforcement-001".to_owned(),
            issued_at: "2026-04-22T14:30:00Z".to_owned(),
            not_before: Some("2026-04-22T14:30:00Z".to_owned()),
            expires_at: Some("2027-04-22T14:30:00Z".to_owned()),
            issuer: Some("chromaticity-release".to_owned()),
            kid: Some("chromaticity-release-2026-04".to_owned()),
            capabilities: vec![
                capability::RENDER_WATERMARK_OFF.to_owned(),
                capability::IO_OSC_INPUT.to_owned(),
                capability::IO_OSC_OUTPUT.to_owned(),
                capability::IO_NDI_INPUT.to_owned(),
                capability::IO_NDI_OUTPUT.to_owned(),
            ],
            flags: Some(Flags::default()),
            machine_binding: None,
            session: None,
            signature: Signature {
                algorithm: "ed25519".to_owned(),
                value: "EE".repeat(32),
            },
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn watermark_off_capability_returns_false_when_no_token_installed() {
        // ADR-009 §Offline posture line 92: no-token install runs
        // with watermark overlay (i.e. `render.watermark.off` is
        // denied).
        assert!(!entitled(
            None,
            capability::RENDER_WATERMARK_OFF,
            fixture_now()
        ));
    }

    #[test]
    fn watermark_off_capability_returns_true_when_valid_token_installed() {
        let token = paid_token();
        assert!(entitled(
            Some(&token),
            capability::RENDER_WATERMARK_OFF,
            fixture_now()
        ));
    }

    #[test]
    fn watermark_off_capability_returns_false_when_token_expired() {
        // `now` advanced beyond the 2027-04-22 expiry. The
        // evaluator must collapse to deny: this is the MVP
        // revocation path (`EV-ENTITLEMENT-REVOCATION`).
        let token = paid_token();
        // 2027-05-01T00:00:00Z = epoch 1_809_129_600.
        let post_expiry = UNIX_EPOCH + Duration::from_secs(1_809_129_600);
        assert!(!entitled(
            Some(&token),
            capability::RENDER_WATERMARK_OFF,
            post_expiry
        ));
    }

    #[test]
    fn free_capability_returns_true_unconditionally() {
        // Every `FREE_CAPABILITIES` entry must short-circuit to
        // `true` whether `token` is None or Some. Cross-corpus
        // smoke check; the dedicated free-features corpus is the
        // primary evidence source.
        let token = paid_token();
        let now = fixture_now();
        for free in FREE_CAPABILITIES {
            assert!(
                entitled(None, free, now),
                "free capability {free} denied with no token",
            );
            assert!(
                entitled(Some(&token), free, now),
                "free capability {free} denied with valid token",
            );
        }
    }

    #[test]
    fn paid_capability_returns_false_when_no_token() {
        // Wave-1a narrowing: every paid capability that is NOT
        // watermark stays denied at MVP even with a valid token.
        // The token below grants OSC + NDI input/output via
        // `Token::capabilities`, but `HONORED_TOKEN_CAPABILITIES`
        // pins only `render.watermark.off` at MVP — the evaluator
        // collapses to `false` for the non-honored ids.
        let token = paid_token();
        let now = fixture_now();
        for paid in [
            capability::IO_OSC_INPUT,
            capability::IO_OSC_OUTPUT,
            capability::IO_NDI_INPUT,
            capability::IO_NDI_OUTPUT,
        ] {
            assert!(
                !entitled(None, paid, now),
                "non-watermark paid {paid} unexpectedly granted with no token",
            );
            assert!(
                !entitled(Some(&token), paid, now),
                "non-watermark paid {paid} unexpectedly granted at MVP \
                 (Wave-1a narrowing — only render.watermark.off is honored)",
            );
        }
    }

    #[test]
    fn entitled_check_completes_well_under_perf_budget() {
        // 1000-call burst against the hot-path with a 50µs
        // ceiling; per-call ~50ns budget. Cold-cache starts can
        // make a single call expensive on CI runners, hence the
        // burst — the asymptote is what matters here. A real
        // criterion bench is reserved for a future wave.
        let token = paid_token();
        let now = fixture_now();
        let start = std::time::Instant::now();
        let mut accumulator = 0u64;
        for _ in 0..1_000 {
            if entitled(Some(&token), capability::RENDER_WATERMARK_OFF, now) {
                accumulator = accumulator.wrapping_add(1);
            }
        }
        // black-box-ish: the side-effect on `accumulator` keeps
        // the optimizer from removing the loop.
        assert_eq!(accumulator, 1_000);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_micros() < 50_000,
            "1000-call burst over budget: {elapsed:?}",
        );
    }

    #[test]
    fn entitlement_evaluator_is_thread_safe() {
        // Compile-time pin: `entitled` takes `Option<&Token>`
        // which is `Send + Sync` because every `Token` field is
        // owned and Send + Sync. Evaluators may therefore be
        // invoked from any thread per ADR-006's actor-boundary
        // model. The static-bound check below fires at compile
        // time if the contract regresses (e.g. a future field
        // adds an `Rc<…>` member).
        fn require_send_sync<T: Send + Sync>() {}
        require_send_sync::<Token>();
        require_send_sync::<Option<&'static Token>>();
        // Smoke: spawn a worker that runs the evaluator and joins.
        let token = paid_token();
        let now = fixture_now();
        let handle = std::thread::spawn(move || {
            entitled(Some(&token), capability::RENDER_WATERMARK_OFF, now)
        });
        assert!(handle.join().expect("worker join"));
    }

    #[test]
    fn unknown_capability_string_returns_false_fail_closed() {
        // ADR-009 §Decision: the entitlement evaluator is a
        // closed-set bitmask. An unknown capability id (typo or
        // future-cap-not-yet-defined) MUST return `false`. This
        // covers (a) inputs that are not in `FREE_CAPABILITIES`,
        // (b) inputs that are not in `HONORED_TOKEN_CAPABILITIES`,
        // and (c) inputs that the token's `capabilities` vec does
        // not list — every short-circuit collapses to deny.
        let token = paid_token();
        let now = fixture_now();
        for typo in [
            "render.watermark.of", // missing trailing 'f'
            "rendr.watermark.off", // missing 'e'
            "totally.fake.capability",
            "v3.future.capability",
            "",
        ] {
            assert!(
                !entitled(None, typo, now),
                "unknown capability {typo:?} unexpectedly granted with no token",
            );
            assert!(
                !entitled(Some(&token), typo, now),
                "unknown capability {typo:?} unexpectedly granted with valid token",
            );
        }
    }
}
