// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! License action IPC stubs — Wave-30 License view port.
//!
//! Defines the three Tauri command contract surfaces that the W30
//! License view's Install / Remove / Request-trial buttons invoke. The
//! wireframe-aligned UI surface is the deliverable here; the real
//! entitlement-token activation wiring remains deferred. The
//! `engine/entitlement` crate is on the workspace graph (Token +
//! canonical CBOR + the `enforcement` surface;
//! `EV-ENTITLEMENT-ENFORCEMENT` closed `Pass` in Phase-4a Wave 1a/1b
//! at the 16-cap honored set, data-only), but the Ed25519
//! signature-verification path is not yet wired (see
//! `engine/entitlement/src/lib.rs` "Still-future scope"), `app/shell`
//! does not yet depend on the crate, and the activation HTTP endpoint
//! is not yet contracted per
//! `docs/adr/ADR-009-activation-and-entitlements.md`.
//!
//! Contract:
//! - `install_license() -> Result<(), String>`
//! - `remove_license() -> Result<(), String>`
//! - `request_trial() -> Result<(), String>`
//! - All three return `Err("not implemented")` at W30. The frontend
//!   `<ErrorMessage>` surfaces the message verbatim so the IPC plumbing
//!   round-trips end-to-end during dev. The contract shapes are
//!   intentionally final so the deferred activation wiring is a pure
//!   backend change with no frontend churn — the caller's
//!   `Promise<void>` resolves on `Ok(())` once real handlers land.
//!
//! Refresh is intentionally not exposed here — the bundle JSX
//! (`v02_supplement.jsx` line 261) renders the Refresh button as
//! `disabled` with an "Available in V2" tooltip, so no IPC contract
//! is owed at MVP scope.

const NOT_IMPLEMENTED: &str = "not implemented";

#[tauri::command]
pub fn install_license() -> Result<(), String> {
    // TODO(deferred activation wiring; see module doc): verify the
    // supplied JWT against the embedded public key per
    // `adr/ADR-009-activation-and-entitlements.md` §Verifier, persist
    // the token to the per-user state store, and emit the
    // `entitlement.installed` observability event.
    Err(NOT_IMPLEMENTED.to_string())
}

#[tauri::command]
pub fn remove_license() -> Result<(), String> {
    // TODO(deferred activation wiring; see module doc): delete the
    // stored token, flip the runtime to watermark-on, and emit the
    // `entitlement.removed` observability event.
    Err(NOT_IMPLEMENTED.to_string())
}

#[tauri::command]
pub fn request_trial() -> Result<(), String> {
    // TODO(deferred activation wiring; see module doc): ping the
    // activation HTTP endpoint with a fresh machine-binding payload
    // per `adr/ADR-009-activation-and-entitlements.md` §Trial
    // Issuance and persist the issued token via `install_license`.
    Err(NOT_IMPLEMENTED.to_string())
}
