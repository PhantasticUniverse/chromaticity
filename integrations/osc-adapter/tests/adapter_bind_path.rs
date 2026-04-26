// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-OBSERVABILITY (T1) — OSC adapter state-transition and bind-error emission sites against docs/31 §Phase-1 baseline catalog.
//!
//! T1 per `docs/43-development-standards.md` §15.5: each test pins
//! the slice-2 adapter-side emission contract for the OSC adapter
//! lifecycle and bind-error counters against live `std::net::UdpSocket`
//! behavior. Slice-3 will rebase these adapter-local counters onto
//! the runtime-bus
//! [`chromaticity_observability::catalog::MetricEvent::OscAdapterStateTransition`]
//! and [`chromaticity_observability::catalog::MetricEvent::OscBindErrors`]
//! emissions.
//!
//! `PermissionDenied` is intentionally not live-pinned here: privileged
//! port behavior varies across macOS, Linux distributions, containers,
//! and rootless CI settings. The `Other` branch is also not forced via a
//! live socket because the symmetric `set_nonblocking` failure site has
//! no portable trigger through `std::net::UdpSocket`; the deterministic
//! bind failure below pins the shared `BindFailed` emission path with
//! `AddressInUse`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::Duration;

use chromaticity_observability::catalog::{OscAdapterState, OscBindErrorReason};
use chromaticity_osc_adapter::adapter::{
    MAX_BIND_ATTEMPTS, OscAdapter, OscAdapterError, OscHealthState, UdpOscAdapter,
};

fn loopback_ephemeral() -> SocketAddr {
    "127.0.0.1:0"
        .parse()
        .expect("loopback:0 parses as SocketAddr")
}

#[test]
fn bind_success_records_unbound_binding_bound_transitions() {
    let mut adapter = UdpOscAdapter::new();

    adapter
        .bind(loopback_ephemeral())
        .expect("bind ephemeral loopback");

    assert!(
        adapter.local_addr().is_some(),
        "successful bind should expose a local socket address",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Unbound, OscAdapterState::Binding),
        1,
        "bind success should record Unbound -> Binding",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Binding, OscAdapterState::Bound),
        1,
        "bind success should record Binding -> Bound",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Binding, OscAdapterState::Unbound),
        0,
        "successful bind must not record the failure transition",
    );
    assert_eq!(
        adapter.bind_error_count(OscBindErrorReason::AddressInUse),
        0,
        "successful bind must not record bind errors",
    );
}

#[test]
fn unbind_from_bound_records_bound_unbinding_unbound_transitions() {
    let mut adapter = UdpOscAdapter::new();
    adapter
        .bind(loopback_ephemeral())
        .expect("bind ephemeral loopback");

    adapter.unbind().expect("unbind bound adapter");

    assert!(
        adapter.local_addr().is_none(),
        "unbind should release the local socket address",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Bound, OscAdapterState::Unbinding),
        1,
        "unbind from Bound should record Bound -> Unbinding",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Unbinding, OscAdapterState::Unbound),
        1,
        "unbind from Bound should record Unbinding -> Unbound",
    );

    adapter.unbind().expect("second unbind is idempotent");

    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Bound, OscAdapterState::Unbinding),
        1,
        "idempotent unbind from Unbound must not record another transition",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Unbinding, OscAdapterState::Unbound),
        1,
        "idempotent unbind from Unbound must not record another transition",
    );
}

#[test]
fn bind_failure_records_address_in_use_and_returns_to_unbound() {
    let occupied_socket = UdpSocket::bind(loopback_ephemeral()).expect("bind occupied socket");
    let occupied_addr = occupied_socket
        .local_addr()
        .expect("occupied socket exposes local_addr");
    let mut adapter = UdpOscAdapter::new();

    let err = adapter
        .bind(occupied_addr)
        .expect_err("binding to an occupied port should fail");

    match err {
        OscAdapterError::BindFailed(io_error) => {
            assert_eq!(
                io_error.kind(),
                ErrorKind::AddrInUse,
                "occupied port should map to std::io::ErrorKind::AddrInUse",
            );
        }
        other => panic!("expected BindFailed, got {other:?}"),
    }

    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Unbound, OscAdapterState::Binding),
        1,
        "bind failure should record Unbound -> Binding",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Binding, OscAdapterState::Unbound),
        1,
        "bind failure should record Binding -> Unbound",
    );
    assert_eq!(
        adapter.state_transition_count(OscAdapterState::Binding, OscAdapterState::Bound),
        0,
        "failed bind must not record Binding -> Bound",
    );
    assert_eq!(
        adapter.bind_error_count(OscBindErrorReason::AddressInUse),
        u64::from(MAX_BIND_ATTEMPTS),
        "permanently occupied port should record one AddressInUse per retry attempt",
    );
    assert_eq!(
        adapter.attempted_binds(),
        MAX_BIND_ATTEMPTS,
        "retry loop must exhaust MAX_BIND_ATTEMPTS before surfacing BindFailed",
    );
    assert_eq!(
        adapter.bind_error_count(OscBindErrorReason::Other),
        0,
        "AddressInUse must not be folded into Other",
    );
}

// EV-NET-HARDENING (T1) — R-05 fragility-mitigation pins for the
// bounded-retry bind loop and the operational-health state machine.
// See `docs/13-risk-register.md` R-05 "bounded retry and clear
// diagnostics" mitigation bullet; the registry row stays
// Planned/`pending:artifact` until the Phase 4a conformance harness
// produces the full artifact.

#[test]
fn bind_with_transient_address_in_use_succeeds_after_retry() {
    let occupier = UdpSocket::bind(loopback_ephemeral()).expect("bind transient occupier");
    let occupied_addr = occupier.local_addr().expect("occupier exposes local_addr");
    let mut adapter = UdpOscAdapter::new();

    // Release the port mid-retry so a later attempt in the bounded
    // retry loop succeeds. The first BIND_BACKOFF_MS step is 100 ms;
    // releasing after ~150 ms forces at least one failed attempt to
    // exercise the retry path before the success.
    let release_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        drop(occupier);
    });

    let result = adapter.bind(occupied_addr);
    release_handle
        .join()
        .expect("occupier-release thread joined");

    assert!(
        result.is_ok(),
        "bind retry loop should recover from transient AddressInUse: {result:?}",
    );
    let attempts = adapter.attempted_binds();
    assert!(
        (2..=MAX_BIND_ATTEMPTS).contains(&attempts),
        "successful retry should record between 2 and MAX_BIND_ATTEMPTS attempts; saw {attempts}",
    );
    assert_eq!(
        adapter.health_state(),
        OscHealthState::Recovering,
        "post-retry health state should be Recovering until the first successful poll",
    );
    assert!(
        adapter.bind_error_count(OscBindErrorReason::AddressInUse) >= 1,
        "transient AddressInUse should bump the bind-error counter at least once",
    );
}

#[test]
fn state_machine_tracks_recovery_after_bind_failure() {
    // A first-shot success transitions Healthy directly without
    // touching the Recovering posture.
    let mut healthy = UdpOscAdapter::new();
    assert_eq!(healthy.health_state(), OscHealthState::Healthy);
    healthy
        .bind(loopback_ephemeral())
        .expect("first-shot bind succeeds on ephemeral loopback");
    assert_eq!(healthy.attempted_binds(), 1);
    assert_eq!(
        healthy.health_state(),
        OscHealthState::Healthy,
        "first-shot bind must not flip health to Recovering",
    );

    // A retry-driven success records the Recovering posture and the
    // catalog-side Unbound -> Binding -> Bound transition path.
    let occupier = UdpSocket::bind(loopback_ephemeral()).expect("bind occupier");
    let occupied_addr = occupier.local_addr().expect("occupier exposes local_addr");
    let mut recovering = UdpOscAdapter::new();
    let release_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        drop(occupier);
    });

    recovering
        .bind(occupied_addr)
        .expect("retry path eventually succeeds");
    release_handle
        .join()
        .expect("occupier-release thread joined");

    assert_eq!(
        recovering.health_state(),
        OscHealthState::Recovering,
        "retry-driven bind must surface Recovering health",
    );
    assert!(recovering.attempted_binds() >= 2);
    assert_eq!(
        recovering.state_transition_count(OscAdapterState::Unbound, OscAdapterState::Binding),
        1,
        "lifecycle path stays Unbound -> Binding regardless of retry count",
    );
    assert_eq!(
        recovering.state_transition_count(OscAdapterState::Binding, OscAdapterState::Bound),
        1,
        "successful retry records exactly one Binding -> Bound transition",
    );

    // unbind() clears the Recovering sticky-state and the attempt
    // counter so the next bind starts clean.
    recovering.unbind().expect("unbind succeeds");
    assert_eq!(recovering.health_state(), OscHealthState::Healthy);
    assert_eq!(recovering.attempted_binds(), 0);
}
