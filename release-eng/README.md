<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# release-eng

Operator runbook for the Chromaticity release pipeline scaffold per
[`docs/adr/ADR-014-signing-notarization-auto-update.md`][adr-014]. Wave 1e-alpha
lands the directory + stub scripts only; production cert procurement
(Apple Developer ID, Windows EV cert, Sparkle EdDSA keypair) is AO-gated
work tracked outside this scaffold.

## Scaffold posture

Each script reads its inputs from environment variables. When the
required variables for a script are unset the script prints

```
release-eng: stub mode (set <vars> for real signing)
```

and exits 0. This lets CI matrices, local dry-runs, and downstream
pipeline composers exercise the scaffold without real signing material.

When the variables are set the script invokes the real tool
(`codesign`, `signtool`, `xcrun notarytool` + `xcrun stapler`) with the
posture mandated by ADR-014. The exception is `stage-appcast.sh`: with
all three variables set it validates the artifact and prints a deferral
notice — the real-mode Sparkle `sign_update` invocation + appcast XML
emission are deferred to AO-gated production wiring (ADR-014 Sec.4).

**Scaffold lands stubs only; production cert + Apple Developer ID +
Windows EV cert procurement is AO-gated work happening outside this
scaffold.**

## Scripts

| Script             | Step (ADR-014)            | Required env                                                             | Real tool                                         |
| ------------------ | ------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------- |
| `sign-macos.sh`    | Sec.1 macOS, Sec.5 step 2 | `MACOS_CERT_NAME`, `APPLE_TEAM_ID`, `BUILD_ARTIFACT_PATH`                | `codesign`                                        |
| `sign-windows.sh`  | Sec.1 Windows, Sec.5      | `WINDOWS_CERT_PATH`, `BUILD_ARTIFACT_PATH`                               | `signtool`                                        |
| `notarize.sh`      | Sec.1 macOS, Sec.5 step 2 | `APPLE_ID`, `APPLE_APP_PASSWORD`, `APPLE_TEAM_ID`, `BUILD_ARTIFACT_PATH` | `xcrun notarytool` + `xcrun stapler`              |
| `stage-appcast.sh` | Sec.2 macOS, Sec.5 step 6 | `APPCAST_FEED_URL`, `RELEASE_VERSION`, `BUILD_ARTIFACT_PATH`             | Sparkle `sign_update` (real-mode wiring deferred) |

Optional env:

- `WINDOWS_TIMESTAMP_URL` (default `http://timestamp.digicert.com`).
- `SPARKLE_TOOLS_PATH` (where `sign_update` lives if not on `PATH`;
  consumed once the deferred real-mode appcast wiring lands).

## Entitlements

`entitlements.plist` declares the macOS hardened-runtime entitlements
consumed by `sign-macos.sh`:

- `com.apple.security.device.audio-input` (microphone).
- `com.apple.security.device.camera`.
- `com.apple.security.network.client` (outbound HTTPS for appcast + crash
  egress).

Loopback OSC ports do not need a hardened-runtime entitlement; the
sandbox `com.apple.security.network.server` key is a sandbox surface and
Chromaticity ships hardened-runtime non-sandboxed at MVP.

## Smoke test (stub mode)

```
bash release-eng/sign-macos.sh
bash release-eng/sign-windows.sh
bash release-eng/notarize.sh
bash release-eng/stage-appcast.sh
```

Each prints its stub-mode notice and exits 0.

## Cert procurement (AO-gated)

The scaffold deliberately does not include any of:

- Apple Developer ID Application certificate (hardware-stored on YubiKey
  / FIPS-140-2 key per ADR-014 Sec.4).
- Windows EV code-signing certificate from a trusted CA (Sec.1 Windows).
- Sparkle 2 EdDSA keypair (public half embedded as `SUPublicEDKey` in
  `Info.plist` per Sec.2; private half stays offline per Sec.4).

Procurement, key rotation, and the single-human approval gate around the
key PIN are tracked in the security runbook and remain Accountable
Owner work.

[adr-014]: ../docs/adr/ADR-014-signing-notarization-auto-update.md
