// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Default constants per ADR-021 §5.
//!
//! ADR-021 §5 fixes the values in this module. Any change here is a
//! conformance break and must be reviewed against the ADR-021
//! Decision section before merge. Operator-facing overrides do not
//! live here: per ADR-021 §5 each default is overridable via the
//! Settings panel at runtime; this module pins the out-of-the-box
//! posture the runtime starts from.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm) per the drift
//! pins in the nested `tests` module.

/// Default polling interval for [`crate::SyphonServerDirectory`]
/// updates, in milliseconds.
///
/// 500 ms balances UI responsiveness against
/// `NSDistributedNotificationCenter` load. Operator-facing UI
/// surfaces a fresh server list within ~500 ms of bus changes,
/// matching the reference Syphon Recorder cadence per ADR-021 §5
/// bullet 1.
pub const DEFAULT_DISCOVERY_INTERVAL_MS: u64 = 500;

/// Default output server name template per ADR-021 §5 bullet 2.
///
/// Operator-overridable via the Settings panel. The
/// `${profile_name}` placeholder interpolates the active
/// chromaticity profile (e.g. "Tour 2026") for multi-instance
/// disambiguation when an operator runs two chromaticity processes
/// on the same machine. Phase 3 PR 2 wires the actual interpolation
/// at `bind` time; Phase 3 PR 1 pins the template literal as the
/// out-of-the-box default.
pub const DEFAULT_OUTPUT_NAME_TEMPLATE: &str = "Chromaticity ${profile_name}";

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod defaults_pins {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — ADR-021 §5 default-
    //! value pins.
    //!
    //! T1 per docs/43 §15.5: each test pins an external-spec
    //! constant against the governing ADR. Any drift in the
    //! discovery interval or output-name template defaults fails
    //! the corresponding test at the exact ADR-section boundary, so
    //! review can route the change to the `EV-INTEGRATION-MATRIX`
    //! gate before merge. T1 counts toward
    //! `EV-EVIDENCE-TAG-COVERAGE`; Phase-4a EXIT requires T2 over
    //! the integration-matrix fixture corpus (Syphon arm narrowed
    //! to synthetic / mock per ADR-021 §10 Negative consequence
    //! bullet 3).

    use super::*;

    /// Evidence: EV-INTEGRATION-MATRIX — ADR-021 §5 bullet 1
    /// discovery interval default holds at 500 ms.
    #[test]
    fn discovery_interval_default_matches_adr_021() {
        assert_eq!(DEFAULT_DISCOVERY_INTERVAL_MS, 500);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — ADR-021 §5 bullet 2 output
    /// name template default holds.
    #[test]
    fn output_name_template_default_matches_adr_021() {
        assert_eq!(DEFAULT_OUTPUT_NAME_TEMPLATE, "Chromaticity ${profile_name}");
    }

    /// Evidence: EV-INTEGRATION-MATRIX — the template carries the
    /// `${profile_name}` placeholder so PR 2's interpolation step
    /// has a substitution target. Drift would silently break
    /// multi-instance disambiguation.
    #[test]
    fn output_name_template_carries_profile_name_placeholder() {
        assert!(
            DEFAULT_OUTPUT_NAME_TEMPLATE.contains("${profile_name}"),
            "default output name template must carry ${{profile_name}} placeholder per ADR-021 §5"
        );
    }
}
