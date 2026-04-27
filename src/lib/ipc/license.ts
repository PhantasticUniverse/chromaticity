/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Typed Tauri IPC wrappers for the W30 License view action stubs.
//
// W30 ships the contract shapes — `install_license() / remove_license()
// / request_trial() -> Promise<void>` — and the License view binds the
// Install / Remove / Request-trial buttons to these wrappers. The
// backend stubs return `Err("not implemented")` so the View surfaces
// the error via `<ErrorMessage>` during dev; Phase-3+/4a swaps the
// stubs for real entitlement-token operations against the runtime
// license verifier per `adr/ADR-009-activation-and-entitlements.md`.
//
// Refresh is intentionally NOT shipped here — its button stays
// disabled with the bundle's "Available in V2" tooltip per the
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
// `LicenseView` JSX (line 261).

import { invoke } from '@tauri-apps/api/core';

/// Install a license token. The W30 backend stub returns
/// `Err("not implemented")`; Phase-3+/4a wires the runtime token
/// verifier against a user-supplied or paste-buffer-supplied JWT.
export async function installLicense(): Promise<void> {
  await invoke<void>('install_license');
}

/// Remove the currently installed license token. The W30 backend
/// stub returns `Err("not implemented")`; Phase-3+/4a deletes the
/// stored token, flips the runtime to watermark-on, and re-emits the
/// observability event.
export async function removeLicense(): Promise<void> {
  await invoke<void>('remove_license');
}

/// Request a fresh trial token for the current machine binding. The
/// W30 backend stub returns `Err("not implemented")`; Phase-3+/4a
/// pings the activation endpoint per
/// `adr/ADR-009-activation-and-entitlements.md` §Trial Issuance and
/// returns the issued token to the caller.
export async function requestTrial(): Promise<void> {
  await invoke<void>('request_trial');
}
