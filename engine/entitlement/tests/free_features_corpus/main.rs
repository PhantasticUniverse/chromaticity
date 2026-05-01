// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a EV-ENTITLEMENT-FREE-FEATURES corpus. Tier declaration
//! and evidence tag live on the `fixtures` inner module below.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-ENTITLEMENT-FREE-FEATURES (T2) — Phase-4a
    //! Wave 1a free-features conformance per ADR-009 §Explicitly
    //! free (lines 71-79). Every capability the ADR enumerates as
    //! "unconditionally available in every edition, including when
    //! no token is present" must answer `true` from
    //! [`chromaticity_entitlement::entitled`] regardless of token
    //! state.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: corpus
    //! conformance against the explicitly-free set. Advances
    //! `docs/26-evidence-artifact-registry.md` row 90 from
    //! `Planned`/`pending:artifact` toward Phase-4a exit.
    //!
    //! ADR-009 line 81 mandates that no `free.*` capability strings
    //! ship in the v2 capability namespace
    //! (`chromaticity_entitlement::capability::ALL` — see the
    //! `engine/entitlement/src/capability.rs` drift pin
    //! `all_includes_every_declared_capability`). The free-features
    //! set therefore lives on the runtime evaluator's
    //! [`chromaticity_entitlement::FREE_CAPABILITIES`] surface as a
    //! parallel array; this corpus pins both that the strings
    //! enumerate the ADR-009 §Explicitly free items and that each
    //! ID resolves to `true` under both no-token and token-present
    //! states.
    //!
    //! Fixture set:
    //!
    //! 1. `pause_capability_is_unconditionally_free` — ADR-009
    //!    line 74 ("Pause. Basic transport control; gating it is
    //!    hostile-UX").
    //! 2. `transitions_off_capability_is_unconditionally_free` —
    //!    ADR-009 line 75 ("Transitions-off (hard-cut scene
    //!    switches)").
    //! 3. `playlist_next_capability_is_unconditionally_free` —
    //!    ADR-009 line 76 ("Playlist navigation (next/previous/select)").
    //! 4. `playlist_previous_capability_is_unconditionally_free` —
    //!    same line.
    //! 5. `playlist_select_capability_is_unconditionally_free` —
    //!    same line (third entry in the navigation tuple).
    //! 6. `preset_recall_capability_is_unconditionally_free` —
    //!    ADR-009 line 77 ("Preset recall and Snap slots").
    //! 7. `snap_recall_capability_is_unconditionally_free` —
    //!    ADR-009 line 77 ("Snap slots" — recall surface).
    //! 8. `snap_save_capability_is_unconditionally_free` —
    //!    ADR-009 line 77 ("Snap slots" — save-to-slot surface).
    //! 9. `scene_export_capability_is_unconditionally_free` —
    //!    ADR-009 line 78 ("Scene export (for backup or hand-off)").
    //! 10. `free_capabilities_namespace_size_matches_adr_count` —
    //!     drift pin: the count is what ADR-009 §Explicitly free
    //!     enumerates (nine entries: pause + transitions-off +
    //!     three playlist-nav + preset-recall + snap-recall +
    //!     snap-save + scene-export).
    //! 11. `free_capabilities_resolve_true_with_no_token` — full
    //!     corpus pass with `token = None`.
    //! 12. `free_capabilities_resolve_true_with_expired_token` —
    //!     full corpus pass with `token = Some(expired)`. Free
    //!     means free even when the token is no longer valid for
    //!     paid features (ADR-009 line 92: "all free capabilities
    //!     remain unconditionally available").

    use std::collections::BTreeMap;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use chromaticity_entitlement::{
        FREE_CAPABILITIES, Flags, Signature, Token, capability, entitled,
    };

    /// Test instant (2026-05-15T00:00:00Z). Same fixture instant
    /// the `enforcement_corpus` uses; correlated so a future scope
    /// widening to deterministic-evaluator fuzz cross-cuts both
    /// rows from the same anchor.
    const FIXTURE_NOW_UNIX: u64 = 1_778_803_200;

    fn fixture_now() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(FIXTURE_NOW_UNIX)
    }

    /// Token whose validity window ended in 2024 — used by the
    /// expired-token fixture below.
    fn expired_token() -> Token {
        Token {
            schema: "entitlement-v2".to_owned(),
            subject: "user-fixture-free-001".to_owned(),
            issued_at: "2024-01-01T00:00:00Z".to_owned(),
            not_before: Some("2024-01-01T00:00:00Z".to_owned()),
            expires_at: Some("2025-01-01T00:00:00Z".to_owned()),
            issuer: Some("chromaticity-release".to_owned()),
            kid: Some("chromaticity-release-2024-01".to_owned()),
            capabilities: vec![capability::RENDER_WATERMARK_OFF.to_owned()],
            flags: Some(Flags::default()),
            machine_binding: None,
            session: None,
            signature: Signature {
                algorithm: "ed25519".to_owned(),
                value: "AB".repeat(32),
            },
            extra: BTreeMap::new(),
        }
    }

    #[test]
    fn pause_capability_is_unconditionally_free() {
        assert!(entitled(None, "runtime.pause", fixture_now()));
    }

    #[test]
    fn transitions_off_capability_is_unconditionally_free() {
        assert!(entitled(None, "runtime.transitions.off", fixture_now()));
    }

    #[test]
    fn playlist_next_capability_is_unconditionally_free() {
        assert!(entitled(None, "playlist.next", fixture_now()));
    }

    #[test]
    fn playlist_previous_capability_is_unconditionally_free() {
        assert!(entitled(None, "playlist.previous", fixture_now()));
    }

    #[test]
    fn playlist_select_capability_is_unconditionally_free() {
        assert!(entitled(None, "playlist.select", fixture_now()));
    }

    #[test]
    fn preset_recall_capability_is_unconditionally_free() {
        assert!(entitled(None, "preset.recall", fixture_now()));
    }

    #[test]
    fn snap_recall_capability_is_unconditionally_free() {
        assert!(entitled(None, "snap.recall", fixture_now()));
    }

    #[test]
    fn snap_save_capability_is_unconditionally_free() {
        assert!(entitled(None, "snap.save", fixture_now()));
    }

    #[test]
    fn scene_export_capability_is_unconditionally_free() {
        assert!(entitled(None, "scene.export", fixture_now()));
    }

    #[test]
    fn free_capabilities_namespace_size_matches_adr_count() {
        // Drift pin: ADR-009 §Explicitly free enumerates nine
        // entries — pause + transitions-off + three navigation
        // entries (next / previous / select) + preset-recall +
        // snap-recall + snap-save + scene-export. A silent
        // insertion or deletion fires this assertion.
        assert_eq!(FREE_CAPABILITIES.len(), 9);
    }

    #[test]
    fn free_capabilities_resolve_true_with_no_token() {
        for free in FREE_CAPABILITIES {
            assert!(
                entitled(None, free, fixture_now()),
                "free capability {free} denied with no token",
            );
        }
    }

    #[test]
    fn free_capabilities_resolve_true_with_expired_token() {
        // ADR-009 line 92: "all free capabilities remain
        // unconditionally available" — even when paid capabilities
        // have collapsed (no-token / expired-token). The free
        // short-circuit in `entitled` runs BEFORE the token
        // validity check, so the answer is `true` for the free
        // set even with an expired token in hand.
        let token = expired_token();
        for free in FREE_CAPABILITIES {
            assert!(
                entitled(Some(&token), free, fixture_now()),
                "free capability {free} denied with expired token",
            );
        }
        // Sanity: the same token evaluated against
        // render.watermark.off (paid) DOES collapse — confirming
        // the test isn't a false-positive from a broken evaluator.
        assert!(!entitled(
            Some(&token),
            capability::RENDER_WATERMARK_OFF,
            fixture_now()
        ));
    }
}
