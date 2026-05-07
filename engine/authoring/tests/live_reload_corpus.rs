// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4b Wave 4b.2 PR 2 of 2 live-reload state-preservation T2
//! corpus. Tier declaration and evidence tag live on the `fixtures`
//! inner module below, matching the pattern established by
//! `engine/authoring/tests/scaffolder_corpus.rs` and
//! `engine/authoring/tests/ux_latency_corpus.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary
//! counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-AUTHORING-LIVE-RELOAD (T2) — Phase-4b Wave 4b.2
    //! PR 2 state-preservation cross-seam contract corpus.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) builds a [`PreservedStateSnapshot`] with a
    //! deliberately chosen field combination, (b) runs it through
    //! [`capture_preserved_state`] and a successful (or failing)
    //! [`perform_reload`] invocation, and (c) asserts the
    //! [`ReloadOutcome::Success`] forwards the captured state
    //! verbatim — or, in the failure-case fixture, asserts the
    //! [`ReloadOutcome::Failed`] does NOT carry preserved state per
    //! the "preservation is success-only" contract documented on
    //! [`ReloadOutcome::Failed`].
    //!
    //! The harness is synthetic per the PR-2 module-level
    //! contract: this corpus does not drive a runtime adapter that
    //! captures `State -> PreservedStateSnapshot` from a live engine
    //! state; the runtime-side wiring is a future-wave option per
    //! the row narrowing recorded in `docs/26` row 61.
    //!
    //! Latency-budget arm: cross-tagged with row 83
    //! `EV-AUTHORING-UX-LATENCY` (already `Pass` via PR `#363`
    //! commit `b2156bb` shared synthetic UX-latency harness) per
    //! `docs/37-shared-instrumentation` note. No separate
    //! live-reload-specific latency harness lands here.
    //!
    //! Fixture matrix (6 fixtures):
    //!
    //! 1. `preserve_three_control_values_round_trip` — control-value
    //!    captured shape preserved verbatim across reload boundary.
    //! 2. `preserve_with_active_preset_round_trip` — `active_preset`
    //!    Some-arm preserved.
    //! 3. `preserve_with_active_snap_round_trip` — `active_snap`
    //!    Some-arm preserved.
    //! 4. `preserve_with_playlist_position_round_trip` —
    //!    `playlist_position` Some-arm preserved.
    //! 5. `combined_preserve_all_four_fields_round_trip` — all four
    //!    fields populated, all preserved on success.
    //! 6. `failed_reload_diagnostic_does_not_carry_preserved_state` —
    //!    failure path drops the `PreservedState`; the producer
    //!    retains the pre-reload state in its own buffer per the
    //!    "preservation is success-only" contract.

    use chromaticity_authoring::{
        DiagnosticCode, PreservedStateSnapshot, ReloadOutcome, capture_preserved_state,
        perform_reload,
    };
    use chromaticity_scene::loader::LoadError;
    use std::path::{Path, PathBuf};

    fn ok_loader(_p: &Path) -> Result<u32, LoadError> {
        Ok(7)
    }

    fn err_loader(_p: &Path) -> Result<u32, LoadError> {
        Err(LoadError::MissingSceneJson(PathBuf::from(
            "/dummy/scene.json",
        )))
    }

    #[test]
    fn preserve_three_control_values_round_trip() {
        let snapshot = PreservedStateSnapshot {
            control_values: vec![
                ("gain".to_string(), 0.25),
                ("tint".to_string(), 0.75),
                ("pulse".to_string(), 1.5),
            ],
            ..Default::default()
        };
        let preserved = capture_preserved_state(snapshot);
        let outcome = perform_reload(Path::new("/dummy"), preserved, ok_loader);
        let ReloadOutcome::Success { value, preserved } = outcome else {
            panic!("expected Success");
        };
        assert_eq!(value, 7);
        assert_eq!(preserved.control_values.len(), 3);
        assert_eq!(preserved.control_values[0].0, "gain");
        assert!((preserved.control_values[0].1 - 0.25).abs() < f32::EPSILON);
        assert_eq!(preserved.control_values[1].0, "tint");
        assert!((preserved.control_values[1].1 - 0.75).abs() < f32::EPSILON);
        assert_eq!(preserved.control_values[2].0, "pulse");
        assert!((preserved.control_values[2].1 - 1.5).abs() < f32::EPSILON);
        assert!(preserved.active_preset.is_none());
        assert!(preserved.active_snap.is_none());
        assert!(preserved.playlist_position.is_none());
    }

    #[test]
    fn preserve_with_active_preset_round_trip() {
        let snapshot = PreservedStateSnapshot {
            active_preset: Some("warm-up".to_string()),
            ..Default::default()
        };
        let preserved = capture_preserved_state(snapshot);
        assert!(!preserved.is_empty());
        let outcome = perform_reload(Path::new("/dummy"), preserved, ok_loader);
        let ReloadOutcome::Success { preserved, .. } = outcome else {
            panic!("expected Success");
        };
        assert_eq!(preserved.active_preset.as_deref(), Some("warm-up"));
        assert!(preserved.control_values.is_empty());
        assert!(preserved.active_snap.is_none());
        assert!(preserved.playlist_position.is_none());
    }

    #[test]
    fn preserve_with_active_snap_round_trip() {
        let snapshot = PreservedStateSnapshot {
            active_snap: Some(5),
            ..Default::default()
        };
        let preserved = capture_preserved_state(snapshot);
        assert!(!preserved.is_empty());
        let outcome = perform_reload(Path::new("/dummy"), preserved, ok_loader);
        let ReloadOutcome::Success { preserved, .. } = outcome else {
            panic!("expected Success");
        };
        assert_eq!(preserved.active_snap, Some(5));
        assert!(preserved.control_values.is_empty());
        assert!(preserved.active_preset.is_none());
        assert!(preserved.playlist_position.is_none());
    }

    #[test]
    fn preserve_with_playlist_position_round_trip() {
        let snapshot = PreservedStateSnapshot {
            playlist_position: Some(11),
            ..Default::default()
        };
        let preserved = capture_preserved_state(snapshot);
        assert!(!preserved.is_empty());
        let outcome = perform_reload(Path::new("/dummy"), preserved, ok_loader);
        let ReloadOutcome::Success { preserved, .. } = outcome else {
            panic!("expected Success");
        };
        assert_eq!(preserved.playlist_position, Some(11));
        assert!(preserved.control_values.is_empty());
        assert!(preserved.active_preset.is_none());
        assert!(preserved.active_snap.is_none());
    }

    #[test]
    fn combined_preserve_all_four_fields_round_trip() {
        let snapshot = PreservedStateSnapshot {
            control_values: vec![("gain".to_string(), 0.5), ("tint".to_string(), 1.0)],
            active_preset: Some("scene-mid".to_string()),
            active_snap: Some(2),
            playlist_position: Some(8),
        };
        let preserved = capture_preserved_state(snapshot);
        assert!(!preserved.is_empty());

        let expected = preserved.clone();
        let outcome = perform_reload(Path::new("/dummy"), preserved, ok_loader);
        let ReloadOutcome::Success { value, preserved } = outcome else {
            panic!("expected Success");
        };
        assert_eq!(value, 7);
        // Full equality across the cross-seam round-trip — the
        // contract is verbatim forwarding, so structural equality
        // holds.
        assert_eq!(preserved, expected);
    }

    #[test]
    fn failed_reload_diagnostic_does_not_carry_preserved_state() {
        // Even though the caller passes a populated PreservedState,
        // a failed reload yields a `ReloadOutcome::Failed` carrying
        // only the structured Diagnostic — preservation is
        // success-only by contract per `ReloadOutcome::Failed`'s
        // docstring. The producer retains the pre-reload state in
        // its own buffer so the next-attempt reload can re-capture
        // or re-use it.
        let snapshot = PreservedStateSnapshot {
            control_values: vec![("gain".to_string(), 0.5)],
            active_preset: Some("scene-a".to_string()),
            active_snap: Some(1),
            playlist_position: Some(3),
        };
        let preserved = capture_preserved_state(snapshot);
        assert!(!preserved.is_empty());
        let outcome = perform_reload(Path::new("/dummy"), preserved, err_loader);
        let ReloadOutcome::Failed(d) = outcome else {
            panic!("expected Failed");
        };
        assert_eq!(d.code, DiagnosticCode::Json);
        // No PreservedState forwarded on the failure arm; this
        // assertion is a structural pin (the variant only has the
        // Diagnostic field) — the test exists to surface a
        // regression if a future wave attempts to add a preserved
        // payload to the Failed arm without an explicit
        // contract-change.
    }
}
