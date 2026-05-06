// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4a Wave 1a EV-HEALTH-METRICS-LOOPBACK-ONLY corpus (no-network-
//! egress half). Tier declaration and evidence tag live on the
//! `fixtures` inner module below, matching the pattern established by
//! `engine/runtime/tests/state_replay_corpus/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-HEALTH-METRICS-LOOPBACK-ONLY (T2) — Phase-1
    //! "external network sees zero egress" structural assertion.
    //! At Phase 1 the Chromaticity observability surface is
    //! in-process only with no transport whatsoever; the
    //! Phase-4d Prometheus exposition endpoint (loopback-only,
    //! `127.0.0.1:<port>`, opt-in via `telemetry.enabled` per
    //! `docs/adr/ADR-016-observability-and-telemetry-posture.md`
    //! §Health metrics) is deferred. Row 94 of
    //! `docs/26-evidence-artifact-registry.md` declares the
    //! invariant ("Validate telemetry endpoint binds only to
    //! loopback when enabled; external network sees zero egress")
    //! and this corpus pins the Phase-1 substrate: NOTHING in the
    //! `engine/observability` crate opens a socket, declares a
    //! network-bound dependency, or returns a future for async
    //! I/O.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! evidence-producing behaviour against a fixture corpus —
    //! here, the static text of `engine/observability/Cargo.toml`
    //! (dependency manifest) and `engine/observability/src/emit.rs`
    //! (the only emission-site implementation today) read via
    //! `include_str!`.
    //!
    //! Defense-in-depth design.
    //!
    //! - **Dependency-level (test #1).** No network-bound crate
    //!   may appear under `[dependencies]` /
    //!   `[dev-dependencies]` of the observability manifest. A
    //!   curated denylist plus an allowlist asserts the manifest
    //!   stays at `workspace-hack` only. Adding `reqwest`,
    //!   `hyper`, `tonic`, `tokio` (with `net` feature), or any
    //!   socket-management crate is the loud failure mode here.
    //! - **Source-level (test #2).** Even if a network dep slips
    //!   in via a transitive that workspace-hack pulls (a
    //!   workspace-hack semantic is to consolidate transitive
    //!   features, not introduce direct API surface), the
    //!   emission-site source itself must not call into
    //!   `std::net::*` or any socket constructor. A read-the-
    //!   source token check catches the manual-bind escape
    //!   route.
    //! - **API-shape (test #3).** [`EmitMetric::emit`] is sync
    //!   and object-safe per the trait declaration. Any future
    //!   widening to `async fn` (which would imply a transport
    //!   that needs scheduler interaction) is immediately
    //!   visible at the trait surface.
    //! - **Behavioural (test #4).** Driving the emitter against
    //!   the full catalog completes synchronously without
    //!   panicking. The Phase-1 stub has no I/O paths to fail;
    //!   driving it through the public catalog surface is the
    //!   "phase-1 emitter runs without network access"
    //!   demonstration the row asks for.
    //!
    //! Phase-4d (deferred). When the Prometheus loopback
    //! endpoint lands, this corpus widens to add (a) a bind-
    //! address assertion that the listening socket is on
    //! `127.0.0.1` (per ADR-016 §Health metrics line 36) and
    //! (b) an external-network observation harness. The Phase-1
    //! substrate this corpus pins is the "zero egress" half of
    //! row 94's invariant: at Phase 1 there is nothing to
    //! observe externally because there is no transport.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 94 declares `EV-HEALTH-METRICS-LOOPBACK-ONLY`
    //! Generation Source = "Binding + egress observation". This
    //! corpus is the egress-observation half (Phase-1 form: the
    //! observation that there is no transport to egress through);
    //! the binding half is partially in the sibling
    //! `catalog_conformance` corpus and lands fully under Phase-
    //! 4d when the loopback-port bind happens.

    use chromaticity_observability::catalog::{
        AudioIngestFrameStatus, CommandKind, EventKind, MetricEvent, ObservabilitySubsystem,
        OscPacketOutcome, SceneValidationOutcome,
    };
    use chromaticity_observability::current_schema_version;
    use chromaticity_observability::emit::{EmitMetric, InMemoryEmitter};

    /// Path-relative include of the observability manifest.
    /// `include_str!` resolves against this file's directory, so
    /// the path is `../../Cargo.toml` from
    /// `engine/observability/tests/no_network_egress/main.rs`.
    const OBSERVABILITY_CARGO_TOML: &str = include_str!("../../Cargo.toml");

    /// Path-relative include of the only Phase-1 emitter
    /// implementation source. The structural sanity check below
    /// reads this text and rejects any `std::net::*` socket-
    /// constructor token.
    const EMIT_SOURCE: &str = include_str!("../../src/emit.rs");

    /// Network-bound crate denylist. Any direct dependency on
    /// these crates is a transport surface that violates ADR-016
    /// §Health metrics line 37 ("No outbound HTTPS telemetry at
    /// any point") and the Phase-1 in-process-only posture.
    ///
    /// Curated to include the prevailing async-runtime + HTTP +
    /// gRPC + raw-socket-management names that a transport
    /// would draw from. Allowlisted exceptions (workspace-hack,
    /// std-only support crates) are matched explicitly via
    /// allowlist below.
    const NETWORK_BOUND_DEP_DENYLIST: &[&str] = &[
        "reqwest",
        "hyper",
        "tonic",
        "tokio",
        "tokio-net",
        "async-std",
        "smol",
        "socket2",
        "mio",
        "rustls",
        "native-tls",
        "rumqttc",
        "rdkafka",
        "lapin",
        "redis",
        "etcd-client",
    ];

    /// Allowlist of dependency names permitted on the
    /// observability crate. Anything else under `[dependencies]`
    /// or `[dev-dependencies]` is a regression vector even if
    /// not present on the network-bound denylist (the denylist
    /// is the loud signal; the allowlist is the conservative
    /// fence that catches a network-bound crate added under a
    /// new name).
    ///
    /// `serde_json` is allowlisted as of the Phase-4d Wave 4d.1
    /// Lane A `EV-CRASH-PAYLOAD-REDACTION` primitives PR: it backs
    /// the [`chromaticity_observability::crash_redaction`] module's
    /// [`serde_json::Value`]-based redactor (crash-report payload
    /// shapes are unknown until the future construction PR lands,
    /// so the redactor pivots through the JSON tree). `serde_json`
    /// is a pure data-format crate with no network surface; it does
    /// not appear on the denylist above and is allowlisted here so
    /// the dependency-set assertion still flags genuine regressions.
    const DEPENDENCY_ALLOWLIST: &[&str] = &["workspace-hack", "serde_json"];

    /// Socket-constructor tokens whose appearance in the emitter
    /// source signals the manual-bind escape route (the route
    /// that bypasses dependency-level checks by reaching for
    /// `std::net` directly).
    const SOCKET_TOKENS_FORBIDDEN_IN_EMIT: &[&str] = &[
        "UdpSocket",
        "TcpListener",
        "TcpStream",
        "SocketAddr",
        "::bind(",
        "::connect(",
        "::listen(",
    ];

    /// Strip `# ...` line comments and trim. Returns trimmed
    /// non-comment, non-blank lines.
    fn manifest_lines(toml: &str) -> impl Iterator<Item = &str> {
        toml.lines()
            .map(|line| match line.find('#') {
                Some(idx) => line[..idx].trim(),
                None => line.trim(),
            })
            .filter(|line| !line.is_empty())
    }

    /// Walk the manifest collecting every dependency-name token
    /// under any `[dependencies]`, `[dev-dependencies]`, or
    /// `[build-dependencies]` section. Targets the prevailing
    /// `name = "..."` and `name = { ... }` forms; renamed deps
    /// (`alias = { package = "real-name", ... }`) are recorded
    /// both by `alias` (the line key) AND by `real-name` (the
    /// `package = "..."` value) so the denylist catches a
    /// renamed network dep.
    fn collect_dependency_names(toml: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut in_deps = false;
        for line in manifest_lines(toml) {
            if let Some(stripped) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let section = stripped.trim();
                in_deps = section == "dependencies"
                    || section == "dev-dependencies"
                    || section == "build-dependencies"
                    || section.ends_with(".dependencies")
                    || section.ends_with(".dev-dependencies")
                    || section.ends_with(".build-dependencies");
                continue;
            }
            if !in_deps {
                continue;
            }
            // `name = "version"` or `name = { ... }`.
            if let Some(eq_idx) = line.find('=') {
                let key = line[..eq_idx].trim();
                if !key.is_empty() && !key.starts_with('[') {
                    names.push(key.to_string());
                }
                // If the value carries `package = "real-name"` the
                // real-name is the deny-target.
                let rhs = &line[eq_idx + 1..];
                if let Some(pkg_idx) = rhs.find("package") {
                    let after = &rhs[pkg_idx + "package".len()..];
                    if let Some(eq2) = after.find('=') {
                        let after2 = &after[eq2 + 1..];
                        if let Some(start) = after2.find('"') {
                            let rest = &after2[start + 1..];
                            if let Some(end) = rest.find('"') {
                                names.push(rest[..end].to_string());
                            }
                        }
                    }
                }
            }
        }
        names
    }

    // ---- Egress fixtures ----

    /// Test #1: dependency-level egress denial. The observability
    /// crate's `[dependencies]` / `[dev-dependencies]` /
    /// `[build-dependencies]` section MUST NOT cite any crate on
    /// the network-bound denylist, AND any name present must be
    /// on the allowlist. This is the loud failure mode for a
    /// transport surface slipping in.
    #[test]
    fn observability_crate_has_no_network_dependencies() {
        let names = collect_dependency_names(OBSERVABILITY_CARGO_TOML);
        assert!(
            !names.is_empty(),
            "manifest parser found no deps; either Cargo.toml format changed or parser is broken",
        );
        let denylist: std::collections::HashSet<&str> =
            NETWORK_BOUND_DEP_DENYLIST.iter().copied().collect();
        let allowlist: std::collections::HashSet<&str> =
            DEPENDENCY_ALLOWLIST.iter().copied().collect();
        for name in &names {
            assert!(
                !denylist.contains(name.as_str()),
                "observability crate carries network-bound dependency {name:?}; \
                 this violates ADR-016 §Health metrics and Phase-1 in-process-only posture. \
                 Adding a transport surface requires a `policy`-scoped PR amending docs/31 §Scope.",
            );
            assert!(
                allowlist.contains(name.as_str()),
                "observability crate carries dependency {name:?} not in the EV-HEALTH-METRICS-LOOPBACK-ONLY allowlist; \
                 widen the allowlist via a `policy`-scoped PR after AO sign-off, OR remove the dep.",
            );
        }
    }

    /// Test #2: source-level egress denial. Even if a network
    /// dep slipped in through a workspace-hack consolidation,
    /// the emission-site source itself must not call directly
    /// into `std::net::*` socket constructors. Defense-in-depth
    /// against the manual-bind escape route.
    #[test]
    fn in_memory_emitter_implementation_does_not_open_sockets() {
        // Strip line comments so the docstring's mention of
        // socket constructors (it documents what's forbidden,
        // it doesn't call them) does not produce false
        // positives.
        let mut stripped = String::new();
        for line in EMIT_SOURCE.lines() {
            let code = match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            };
            stripped.push_str(code);
            stripped.push('\n');
        }
        for token in SOCKET_TOKENS_FORBIDDEN_IN_EMIT {
            assert!(
                !stripped.contains(token),
                "engine/observability/src/emit.rs contains forbidden socket token {token:?}; \
                 the Phase-1 stub MUST be I/O-free per docs/31 §Scope and ADR-016 §Health metrics.",
            );
        }
        // Also forbid the std::net path explicitly — independent
        // of constructor names, any reference to the socket
        // namespace is a smell.
        assert!(
            !stripped.contains("std::net"),
            "engine/observability/src/emit.rs references std::net::; the Phase-1 stub MUST be I/O-free",
        );
    }

    /// Test #3: API-shape egress denial. [`EmitMetric::emit`]
    /// returns `()` synchronously; widening to `async fn` would
    /// signal scheduler interaction the Phase-1 in-process stub
    /// does not do. The compile surface enforces this, but the
    /// runtime-shape test below makes the invariant visible.
    #[test]
    fn metric_event_emission_completes_synchronously_no_async_io() {
        // If `emit` ever returns an `impl Future` the closure
        // type below changes and this test breaks at compile
        // time. The runtime body proves emission completes
        // before the next statement runs.
        let emitter = InMemoryEmitter::new();
        let _: () = emitter.emit(MetricEvent::ObservabilityEventsEmitted {
            subsystem: ObservabilitySubsystem::Observability,
            value: 1,
        });
        let _: () = emitter.emit(current_schema_version());
        // After both `emit` calls returned, the events are
        // already buffered — proof that `emit` is not deferring
        // the work onto an async runtime.
        assert_eq!(emitter.len(), 2);
    }

    /// Test #4: behavioural egress demonstration. Driving every
    /// catalog metric type through the in-process emitter
    /// completes successfully without panicking, without I/O
    /// errors, and without requiring any external network
    /// configuration. This is the Phase-1 "runs without network
    /// access" demonstration; the post-Phase-4d widening (drop
    /// network capability via cap_net_admin) is deferred per
    /// docs/31 §Scope Phase-4d row.
    #[test]
    fn phase_1_emitter_runs_without_network_access() {
        let emitter = InMemoryEmitter::new();
        // Drive a representative event from every subsystem
        // through the trait object — the slice-3 runtime-bus
        // wiring will use this exact `Box<dyn EmitMetric>`
        // shape.
        let sink: Box<dyn EmitMetric> = Box::new(emitter);
        sink.emit(MetricEvent::RuntimeCommandsDispatched {
            kind: CommandKind::ControlSet,
            value: 1,
        });
        sink.emit(MetricEvent::RuntimeEventsEmitted {
            kind: EventKind::ControlChanged,
            value: 1,
        });
        sink.emit(MetricEvent::SceneValidations {
            outcome: SceneValidationOutcome::Pass,
            value: 1,
        });
        sink.emit(MetricEvent::AudioIngestFrames {
            status: AudioIngestFrameStatus::Ready,
            value: 1,
        });
        sink.emit(MetricEvent::OscInboundPackets {
            outcome: OscPacketOutcome::Accepted,
            value: 1,
        });
        sink.emit(MetricEvent::ObservabilityEventsEmitted {
            subsystem: ObservabilitySubsystem::Observability,
            value: 1,
        });
        sink.emit(current_schema_version());
        // No panic, no I/O failure, no external network access
        // attempted. The test reaching this line IS the
        // demonstration row 94 asks for at Phase 1.
    }
}
