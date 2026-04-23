<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Security Policy

Chromaticity takes security seriously. This document describes how to
report a vulnerability, what to expect from us, and what is in scope.

## Reporting a vulnerability

**Do not open a public GitHub issue for security matters.** Public
disclosure before a fix is available puts users at risk.

Report vulnerabilities privately through either channel:

1. **GitHub private vulnerability advisory.** From the repository's
   Security tab, choose "Report a vulnerability." (Recommended — starts
   the coordination clock automatically.)
2. **Encrypted email** to **security@chromaticity.app**. Use the
   age/PGP public key published at
   `https://chromaticity.app/.well-known/security-public-key` (key
   material is hosted on a Chromaticity-owned domain, not a third
   party, per `docs/adr/ADR-016-observability-and-telemetry-posture.md`).

Please include:

- A clear description of the issue and the impact you expect.
- Reproduction steps or a proof-of-concept (attach files rather than
  pasting large binaries).
- Affected Chromaticity version(s), OS, and hardware profile.
- Your preferred attribution (name + handle, anonymous, or "do not
  credit"); we default to crediting reporters in the release that
  contains the fix.

## Coordinated disclosure window

Chromaticity operates on a **90-day coordinated-disclosure window** by
default, measured from the date we acknowledge receipt of your report.
We may request a reasonable extension if a fix requires coordinated
rollout across platforms, third-party vendors, or signing/notarization
authorities; extensions are negotiated with the reporter and recorded on
the advisory.

If you believe the issue is being actively exploited in the wild, tell
us in the initial report and we will accelerate the timeline
accordingly.

## What to expect

- **Acknowledgement**: within 3 business days.
- **Triage decision**: within 10 business days (valid / duplicate /
  out-of-scope).
- **Fix plan**: within 30 days of triage for valid reports.
- **Release cadence**: security-fix releases land on `main` under the
  normal PR policy but carry the `type:security` label and a
  `CHANGELOG.md` entry. Disclosure of the specific mitigation publishes
  with the release that contains the fix, not earlier.
- **Attribution**: reporters are credited in the release notes unless
  they request anonymity.
- **Safe harbor**: good-faith security research conducted within the
  scope below will not be pursued legally or otherwise, and Chromaticity
  will not seek prosecution or civil action against such research. We
  commit to working with researchers in good faith.

## Scope

**In scope:**

- Chromaticity desktop application binaries distributed by Chromaticity
  (macOS `.dmg` signed under the Chromaticity Developer ID; Windows
  installer signed under Chromaticity Authenticode).
- Source code in this repository.
- Release artifacts published under the `app/vX.Y.Z` tag stream
  (attested per `docs/43-development-standards.md` §11.2).
- Networking surface described in
  `docs/adr/ADR-013-networking-defaults-and-security.md` (OSC, NDI bind
  defaults, rate limits, allowlists).
- Auto-updater posture described in
  `docs/adr/ADR-014-signing-notarization-auto-update.md` (Sparkle 2 on
  macOS, EdDSA appcast signatures).
- Entitlement token handling per
  `docs/adr/ADR-009-activation-and-entitlements.md` (Ed25519 token
  verification and `kid` claim handling).
- Sandbox escape or determinism violation from a scene-author script
  that is supposed to be isolated per
  `docs/adr/ADR-012-script-sandbox-engine.md`.
- Support-bundle redaction failures (PII/secrets leaking into
  `EV-CRASH-PAYLOAD-REDACTION`-scoped artifacts).

**Out of scope:**

- Vulnerabilities in third-party scenes or user-authored JavaScript
  running under the documented sandbox; these are defended by the
  sandbox and are not the sandbox's own isolation failure.
- Denial-of-service caused by user-supplied content that exceeds
  documented budgets (e.g. intentional shader compile-time bombs
  below the compile-timeout cap).
- Vulnerabilities in dependencies that have a published advisory and a
  Dependabot or `cargo audit` PR already open; report to the upstream
  project.
- Self-XSS or social-engineering attacks that require the victim to
  paste untrusted content into their own terminal.
- Reports generated without evidence by automated scanners against
  Chromaticity's own marketing site; only findings against the shipped
  app and release artifacts are in scope.
- Non-MVP surfaces that are explicitly deferred in the planning corpus
  (e.g. real-time external data ingestion per
  `docs/41-data-ingestion-differentiator.md`) until the corresponding
  ADR lands.

## Key custody and rotation

Chromaticity uses three non-overlapping signing domains:

1. Source-provenance signing (Git commit / tag) —
   `docs/43-development-standards.md` §9.
2. Binary distribution signing (Apple Developer ID / Windows
   Authenticode / Sparkle EdDSA) —
   `docs/adr/ADR-014-signing-notarization-auto-update.md`.
3. Entitlement token signing (Ed25519, `kid` rotation) —
   `docs/adr/ADR-009-activation-and-entitlements.md`.

Compromise of any domain is treated as a security incident under this
policy. Key rotation procedures are documented in the corresponding ADR
and are exercised on a schedule recorded in the release runbook.

## Changes

Changes to this document are `policy`-scoped PRs authored by the
Accountable Owner. The `/LICENSES/`, `REUSE.toml`, and `SECURITY.md`
triad is a non-delegable AO subtree per
`docs/43-development-standards.md` §8.1.
