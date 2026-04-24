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

use chromaticity_observability::catalog::{OscAdapterState, OscBindErrorReason};
use chromaticity_osc_adapter::adapter::{OscAdapter, OscAdapterError, UdpOscAdapter};

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
        1,
        "occupied port should increment AddressInUse exactly once",
    );
    assert_eq!(
        adapter.bind_error_count(OscBindErrorReason::Other),
        0,
        "AddressInUse must not be folded into Other",
    );
}
