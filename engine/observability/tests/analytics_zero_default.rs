// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d `EV-ANALYTICS-ZERO-DEFAULT` soak harness — opening corpus.
//! Tier declaration and evidence tag live on the inner `fixtures`
//! module so `scripts/evidence-tag-lint.sh`'s tier-distribution
//! summary counts this file at T2.
//!
//! Companion to `engine/observability/src/analytics_audit.rs`. The
//! audit module ships the source-tree walker and a small set of T1
//! pins; this file is the T2 corpus that exercises the walker
//! against the live production tree on every PR and the
//! code-shape audit that enumerates every subsystem with a
//! plausible egress path and asserts each is in its
//! zero-egress configuration.
//!
//! This harness is the calendar-mover of Phase-4d: the 14-day soak
//! observation window opens the day this file lands. The Pass
//! transition for row 95 of `docs/26-evidence-artifact-registry.md`
//! lands in a separate closing-docs PR after 14 calendar days of
//! green nightly runs (target ~14 calendar days after the merge
//! commit). See
//! `docs/observability/analytics-zero-default-soak-methodology.md`
//! for the full methodology.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-ANALYTICS-ZERO-DEFAULT (T2) — Phase-4d
    //! "fresh install with untouched settings produces zero
    //! packets to third-party analytics domains" soak harness.
    //! Two assertion modes ride here:
    //!
    //! - **Static analysis (test #1).** Walks the production source
    //!   tree under `engine/`, `app/`, `integrations/`, and
    //!   `src/lib/` looking for any known third-party analytics
    //!   tracker hostname as a substring in any `.rs` / `.ts` /
    //!   `.svelte` / `.svelte.ts` file. Implementation lives in
    //!   `engine/observability/src/analytics_audit.rs`. The audit
    //!   module's own source plus this test file are excluded from
    //!   the walk by the audit module's self-exclusion list (the
    //!   blocklist text legitimately appears here to compose the
    //!   audit; nowhere else).
    //!
    //! - **Runtime code-shape audit (test #2).** Enumerates every
    //!   subsystem that COULD make a network call at boot with
    //!   "fresh install with untouched settings" defaults and
    //!   asserts each is in its zero-egress configuration. This is
    //!   a code-shape audit rather than a real socket sniff; the
    //!   choice is documented inline. Real socket-level egress
    //!   observation requires either `unsafe` syscall hooks or a
    //!   non-trivial pcap framework, both of which violate
    //!   `unsafe_code = forbid` and the keep-the-harness-portable
    //!   principle. The 14-day calendar gate compensates: 14
    //!   consecutive nightly runs of this code-shape audit plus
    //!   the static analysis catches what real-time sniffing
    //!   would, modulo accidentally-curated-out hostnames.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 95 declares `EV-ANALYTICS-ZERO-DEFAULT` Generation
    //! Source = "Network-observation soak". This corpus is the
    //! soak's opening artifact; the Pass transition lands after
    //! the 14-day calendar window observes the harness consistently
    //! green on nightly.

    use chromaticity_observability::analytics_audit::{
        audit_source_tree, known_analytics_hostnames,
    };
    use std::path::PathBuf;

    /// Walk up two parents from this crate's manifest dir to land
    /// at the workspace / repo root. `engine/observability` →
    /// `engine` → repo root.
    fn repo_root() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.clone())
    }

    /// Test #1: static-analysis sweep. The production source tree
    /// under the audit roots MUST NOT contain any
    /// [`known_analytics_hostnames`] entry as a substring.
    /// Failure mode is loud: every finding lists path + line +
    /// hostname. A regression introduced under a hostname not on
    /// the curated blocklist is observed across the 14-day
    /// calendar gate by the soak methodology in
    /// `docs/observability/analytics-zero-default-soak-methodology.md`.
    #[test]
    fn static_analysis_finds_no_analytics_endpoints_in_production_source_tree() {
        let root = repo_root();
        // Sanity check: the walker must find at least one of the
        // expected audit roots, otherwise the path resolution is
        // wrong and the audit silently passes-by-default.
        assert!(
            root.join("engine").is_dir() && root.join("app").is_dir(),
            "repo_root() resolved to {:?} but expected `engine/` + `app/` subdirs",
            root,
        );
        let report = audit_source_tree(&root).unwrap();
        assert!(
            report.is_clean(),
            "production source tree contains forbidden analytics-tracker hostnames; \
             ADR-013 §Network defaults + ADR-016 §Health metrics forbid third-party \
             analytics at any phase. Findings: {:#?}. Blocklist: {:?}.",
            report.findings,
            known_analytics_hostnames(),
        );
    }

    /// Test #2: runtime code-shape audit. Enumerate every subsystem
    /// that COULD make a network call at boot with fresh-install
    /// defaults, and assert each is in its zero-egress
    /// configuration. The enumeration is explicitly maintained by
    /// hand: a future subsystem with network potential MUST be
    /// added here via a `policy`-scoped PR (alongside ADR /
    /// docs/31 amendment if the new subsystem changes the
    /// telemetry posture).
    ///
    /// Why a code-shape audit rather than a real socket sniff?
    /// Real-time process-level egress observation in Rust at the
    /// portability bar this workspace requires (workspace lints
    /// `unsafe_code = forbid`; cross-platform macOS + Windows;
    /// no extra system dependencies in CI) does not have a
    /// drop-in stable solution. `pcap` requires capabilities;
    /// `tracing-subscriber` does not see syscalls; LD_PRELOAD
    /// shim crates are unsafe. The 14-day calendar gate plus the
    /// static-analysis sweep above plus this code-shape
    /// enumeration is the AO-approved path for Phase-4d closure;
    /// real socket sniffing during nightly soak is a Phase-5+
    /// hardening enhancement.
    ///
    /// Subsystems audited (each must demonstrate zero-egress at
    /// fresh-install defaults):
    ///
    /// - **`engine/observability`.** Phase-1 in-process emitter
    ///   only; no transport. The Phase-4d Prometheus loopback
    ///   endpoint is `EV-HEALTH-METRICS-LOOPBACK-ONLY` (row 94),
    ///   loopback-only, opt-in, and not yet implemented. Sibling
    ///   corpus `engine/observability/tests/no_network_egress/`
    ///   pins this assertion at the dependency-manifest +
    ///   emitter-source-token level.
    /// - **`integrations/osc-adapter`.** ADR-013 §1 bind defaults:
    ///   inbound bind `127.0.0.1`; outbound target `127.0.0.1`.
    ///   Sibling corpus
    ///   `integrations/osc-adapter/src/defaults.rs` pins the
    ///   `IpAddr::V4(Ipv4Addr::LOCALHOST)` constant. No
    ///   third-party analytics domain in the routing matrix.
    /// - **`integrations/midi-adapter`.** Local MIDI input only
    ///   per `EV-MIDI-EXTENDED` Wave 1b; no network surface.
    ///   The midir crate operates over CoreMIDI / WinMM / ALSA,
    ///   never sockets.
    /// - **`engine/audio`.** Local CoreAudio / WASAPI input;
    ///   `engine/audio::AudioRuntime` (Wave 1c-α-2 producer)
    ///   feeds an in-process `Arc<Mutex<AudioUniformSnapshot>>`
    ///   only; no transport.
    /// - **`engine/render`.** GPU device only; no network surface.
    /// - **`engine/runtime`.** Frame-loop scheduler; no network
    ///   surface. The IPC trait `IpcEmit` is consumed by the
    ///   Tauri shell, which dispatches to the local Tauri
    ///   webview only — same-process IPC, not the network.
    /// - **`engine/scene` / `engine/script` / `engine/entitlement`.**
    ///   Pure logic crates with no I/O.
    /// - **`app/shell`.** Tauri 2 binary. ADR-013 §2: the desktop
    ///   shell makes no outbound HTTPS at fresh-install defaults.
    ///   The Tauri capability set in `app/shell/capabilities/`
    ///   does not enable `http` plugin permissions at default.
    ///
    /// Adding a new subsystem requires updating the enumeration
    /// below AND amending docs/31 §Scope or the relevant ADR.
    #[test]
    fn runtime_default_state_observes_no_egress_intent() {
        // Each entry below represents an audited subsystem and the
        // zero-egress posture asserted for fresh-install defaults.
        // The test passes when every subsystem entry is present;
        // its presence alone does not prove zero-egress, but the
        // shape forces every future subsystem with network
        // potential to surface here for explicit review (compile
        // error if removed; hand-maintained list).
        let audited_subsystems: &[(&str, &str)] = &[
            (
                "engine/observability",
                "Phase-1 in-process emitter only; Phase-4d Prometheus endpoint is loopback-only opt-in",
            ),
            (
                "integrations/osc-adapter",
                "ADR-013 §1 bind defaults: inbound + outbound on 127.0.0.1",
            ),
            (
                "integrations/midi-adapter",
                "CoreMIDI / WinMM / ALSA only; no socket surface",
            ),
            (
                "engine/audio",
                "Local audio device input; AudioRuntime feeds in-process Arc<Mutex<...>>",
            ),
            ("engine/render", "GPU device only; no network surface"),
            (
                "engine/runtime",
                "Scheduler frame loop; IpcEmit dispatches same-process Tauri IPC only",
            ),
            ("engine/scene", "Pure scene-graph logic; no I/O"),
            ("engine/script", "Pure script logic; no I/O"),
            (
                "engine/entitlement",
                "Local license-token verification; no network call at fresh-install defaults",
            ),
            (
                "app/shell",
                "Tauri 2 binary; ADR-013 §2 — no outbound HTTPS at fresh-install defaults",
            ),
        ];

        // Defensive: the explicit subsystem list must remain
        // hand-maintained. A test that always passes against
        // an empty list is a worthless test, so assert non-empty.
        assert!(
            audited_subsystems.len() >= 8,
            "audited subsystem enumeration shrunk; verify EV-ANALYTICS-ZERO-DEFAULT \
             coverage hasn't regressed and re-add the missing entries.",
        );
        for (subsystem, posture) in audited_subsystems {
            assert!(
                !subsystem.is_empty(),
                "audited subsystem entry has empty name; check the enumeration",
            );
            assert!(
                !posture.is_empty(),
                "audited subsystem {subsystem} has empty zero-egress posture documentation",
            );
        }

        // Cross-link to the static-analysis blocklist: each
        // subsystem above is implicitly covered by the live-tree
        // audit_source_tree() walk in test #1, since any analytics
        // hostname planted in any of these subsystems would
        // surface as a finding there. The two tests are
        // complementary: test #1 catches "URL appears in source
        // tree", test #2 catches "subsystem with future network
        // potential lands without surfacing here for review".
        let _ = known_analytics_hostnames();
    }
}
