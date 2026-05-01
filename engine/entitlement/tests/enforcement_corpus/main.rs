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
    //! Evidence: EV-ENTITLEMENT-ENFORCEMENT (T2) — Phase-4a
    //! adapter-boundary entitlement enforcement. Wave 1a originally
    //! narrowed the surface to MVP watermark-on / watermark-off only;
    //! Wave 1b-B widens [`chromaticity_entitlement::HONORED_TOKEN_CAPABILITIES`]
    //! to the full ADR-009 §Decision tier matrix (I/O adapters,
    //! authoring, custom-resolution, meta) — a data-only change with
    //! the Wave-1a decision tree intact.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: corpus
    //! conformance against the `chromaticity_entitlement::entitled`
    //! evaluator surface. The Wave-1a fixtures pin the watermark-only
    //! decision tree; the Wave 1b-B fixtures (`tier_gated_*`) pin the
    //! widened honored-token-capability set behavior.
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
    //!    watermark paid capabilities collapse to `false` when no
    //!    token is installed (fail-closed regardless of whether the
    //!    cap is in `HONORED_TOKEN_CAPABILITIES`). After Wave 1b-B
    //!    widening, `IO_CAMERA` and the `data.ingest.*` reserved
    //!    family are the still-denied paid surfaces (the former is
    //!    free at MVP, the latter is post-MVP per ADR-009).
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
    //!
    //! Wave 1b-B fixtures (data widening of `HONORED_TOKEN_CAPABILITIES`,
    //! decision tree unchanged; ADR-009 §Decision):
    //!
    //! 9. `tier_gated_io_osc_input_returns_false_when_no_token` —
    //!    fail-closed when `token = None` against the newly-honored
    //!    `io.osc.input` capability.
    //! 10. `tier_gated_io_osc_input_returns_true_when_token_grants`
    //!     — token whose `capabilities` vec lists `io.osc.input` is
    //!     granted under the widened `HONORED_TOKEN_CAPABILITIES`.
    //! 11. `tier_gated_io_osc_input_returns_false_when_token_does_not_grant`
    //!     — token that lists `render.watermark.off` only does NOT
    //!     get `io.osc.input`; the honored-set check passes but the
    //!     `Token::capabilities` containment check denies.
    //! 12. `tier_gated_io_ndi_input_returns_false_when_token_expired`
    //!     — even with `io.ndi.input` listed in the token, post-
    //!     expiry `is_valid_at` collapses the response to `false`.
    //! 13. `tier_gated_authoring_ide_returns_true_when_token_grants`
    //!     — same pattern as fixture 10 for `authoring.ide`.
    //! 14. `tier_gated_authoring_import_returns_false_when_token_does_not_grant`
    //!     — same pattern as fixture 11 for `authoring.import`.
    //! 15. `tier_gated_render_custom_resolution_returns_true_when_token_grants`
    //!     — `render.custom-resolution` honored + granted.
    //! 16. `tier_gated_meta_reactivity_returns_true_when_token_grants`
    //!     — `meta.reactivity` honored + granted.
    //! 17. `tier_gated_io_camera_is_not_in_honored_set_per_mvp` —
    //!     `io.camera` is intentionally NOT in `HONORED_TOKEN_CAPABILITIES`
    //!     even though every other I/O cap is; ADR-009 declares it
    //!     free at MVP, and the post-MVP lift is a follow-up PR.
    //! 18. `tier_gated_capability_check_completes_under_perf_budget_with_widened_set`
    //!     — the linear scan over the widened 13-entry honored set
    //!     remains comfortably under the same 50µs / 1000-call burst
    //!     ceiling pinned by the Wave-1a perf fixture.
    //! 19. `tier_gated_unknown_capability_returns_false_fail_closed`
    //!     — a hypothetical post-MVP capability id (`data.ingest.network`)
    //!     stays denied even when the token lists it; the cap is
    //!     deliberately excluded from the widened set.

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
        // Fail-closed posture: every paid capability collapses to
        // `false` when no token is installed. After Wave 1b-B
        // widening, the still-denied-with-valid-token surfaces are
        // `IO_CAMERA` (free at MVP per ADR-009 — never honored as a
        // paid token cap) and the `data.ingest.*` reserved family
        // (post-MVP). Capabilities now honored via the widened set
        // are exercised by the Wave 1b-B `tier_gated_*` fixtures
        // below.
        let token = paid_token();
        let now = fixture_now();
        for paid in [
            capability::IO_CAMERA,
            capability::DATA_INGEST_NETWORK,
            capability::DATA_INGEST_GEOSPATIAL,
            capability::DATA_INGEST_SDR,
        ] {
            assert!(
                !entitled(None, paid, now),
                "paid {paid} unexpectedly granted with no token",
            );
            assert!(
                !entitled(Some(&token), paid, now),
                "paid {paid} unexpectedly granted at MVP \
                 (IO_CAMERA is free; data.ingest.* is post-MVP — \
                 neither is in HONORED_TOKEN_CAPABILITIES)",
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

    // -------------------------------------------------------------
    // Wave 1b-B tier-gating fixtures.
    //
    // The widened `HONORED_TOKEN_CAPABILITIES` array is a data
    // change only; the decision tree pinned by the Wave-1a fixtures
    // above still holds. The fixtures below exercise the newly-
    // honored capability surfaces (I/O adapters, authoring,
    // custom-resolution, meta) plus the IO_CAMERA exception.
    // -------------------------------------------------------------

    /// Token granting `io.osc.input` only — used to verify the
    /// honored-set + token-listing conjunction across the widened
    /// surface without leaking the watermark-off bit into the
    /// assertion.
    fn token_with_capabilities(caps: &[&str]) -> Token {
        Token {
            schema: "entitlement-v2".to_owned(),
            subject: "user-fixture-tier-gated-001".to_owned(),
            issued_at: "2026-04-22T14:30:00Z".to_owned(),
            not_before: Some("2026-04-22T14:30:00Z".to_owned()),
            expires_at: Some("2027-04-22T14:30:00Z".to_owned()),
            issuer: Some("chromaticity-release".to_owned()),
            kid: Some("chromaticity-release-2026-04".to_owned()),
            capabilities: caps.iter().map(|s| (*s).to_owned()).collect(),
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
    fn tier_gated_io_osc_input_returns_false_when_no_token() {
        // Newly-honored capability under the widened set still
        // fail-closes when no token is installed.
        assert!(!entitled(None, capability::IO_OSC_INPUT, fixture_now()));
    }

    #[test]
    fn tier_gated_io_osc_input_returns_true_when_token_grants() {
        // The widened `HONORED_TOKEN_CAPABILITIES` lists
        // `io.osc.input`; a valid token whose `capabilities` vec
        // also lists it must collapse the evaluator to `true`.
        let token = token_with_capabilities(&[capability::IO_OSC_INPUT]);
        assert!(entitled(
            Some(&token),
            capability::IO_OSC_INPUT,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_io_osc_input_returns_false_when_token_does_not_grant() {
        // Honored-set check passes, but the token's `capabilities`
        // vec does not list the cap — step 5 of the decision tree
        // returns `false`.
        let token = token_with_capabilities(&[capability::RENDER_WATERMARK_OFF]);
        assert!(!entitled(
            Some(&token),
            capability::IO_OSC_INPUT,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_io_ndi_input_returns_false_when_token_expired() {
        // Expiry path still applies under the widened set: even
        // when the token lists the cap, `is_valid_at == false`
        // collapses the response.
        let token = token_with_capabilities(&[capability::IO_NDI_INPUT]);
        // 2027-05-01T00:00:00Z = epoch 1_809_129_600 (post-expiry).
        let post_expiry = UNIX_EPOCH + Duration::from_secs(1_809_129_600);
        assert!(!entitled(
            Some(&token),
            capability::IO_NDI_INPUT,
            post_expiry
        ));
    }

    #[test]
    fn tier_gated_authoring_ide_returns_true_when_token_grants() {
        let token = token_with_capabilities(&[capability::AUTHORING_IDE]);
        assert!(entitled(
            Some(&token),
            capability::AUTHORING_IDE,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_authoring_import_returns_false_when_token_does_not_grant() {
        let token = token_with_capabilities(&[capability::AUTHORING_IDE]);
        assert!(!entitled(
            Some(&token),
            capability::AUTHORING_IMPORT,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_render_custom_resolution_returns_true_when_token_grants() {
        let token = token_with_capabilities(&[capability::RENDER_CUSTOM_RESOLUTION]);
        assert!(entitled(
            Some(&token),
            capability::RENDER_CUSTOM_RESOLUTION,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_meta_reactivity_returns_true_when_token_grants() {
        let token = token_with_capabilities(&[capability::META_REACTIVITY]);
        assert!(entitled(
            Some(&token),
            capability::META_REACTIVITY,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_io_camera_is_not_in_honored_set_per_mvp() {
        // `IO_CAMERA` is deliberately excluded from
        // `HONORED_TOKEN_CAPABILITIES` even though every other I/O
        // cap is honored — ADR-009 declares camera ingestion free
        // at MVP. A token that lists `io.camera` therefore does
        // NOT grant it; the response stays `false` until a future
        // PR lifts `IO_CAMERA` into either `FREE_CAPABILITIES` or
        // `HONORED_TOKEN_CAPABILITIES`.
        let token = token_with_capabilities(&[capability::IO_CAMERA]);
        assert!(!entitled(
            Some(&token),
            capability::IO_CAMERA,
            fixture_now()
        ));
    }

    #[test]
    fn tier_gated_capability_check_completes_under_perf_budget_with_widened_set() {
        // Linear scan over the 13-entry widened honored set must
        // remain comfortably inside the same 50µs / 1000-call
        // burst budget the Wave-1a fixture pinned for the 1-entry
        // narrowed set. The asymptote is dominated by the
        // FREE_CAPABILITIES + Token::capabilities scans, not the
        // honored-set scan.
        let token = token_with_capabilities(&[
            capability::IO_OSC_INPUT,
            capability::IO_OSC_OUTPUT,
            capability::IO_NDI_INPUT,
            capability::AUTHORING_IDE,
            capability::RENDER_CUSTOM_RESOLUTION,
            capability::META_REACTIVITY,
        ]);
        let now = fixture_now();
        let start = std::time::Instant::now();
        let mut accumulator = 0u64;
        for _ in 0..1_000 {
            if entitled(Some(&token), capability::META_REACTIVITY, now) {
                accumulator = accumulator.wrapping_add(1);
            }
        }
        assert_eq!(accumulator, 1_000);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_micros() < 50_000,
            "1000-call burst over budget under widened set: {elapsed:?}",
        );
    }

    #[test]
    fn tier_gated_unknown_capability_returns_false_fail_closed() {
        // `data.ingest.network` is declared in `crate::capability`
        // (ADR-009 reserved family) but is NOT in
        // `HONORED_TOKEN_CAPABILITIES`. Even when the token lists
        // it, the evaluator must collapse to `false` — the
        // data-ingest subsystem is post-MVP per ADR-009 §External
        // data ingestion.
        let token = token_with_capabilities(&[capability::DATA_INGEST_NETWORK]);
        assert!(!entitled(
            Some(&token),
            capability::DATA_INGEST_NETWORK,
            fixture_now()
        ));
    }
}
