<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Phase-0 Placeholder Inventory

This document tracks every identifier in the repository that is
currently a **placeholder** pending Phase-0 activation: domain
registration, trademark clearance, certificate enrollment, key
generation, or account provisioning. The placeholder posture is normal
for a Phase-0 governance bootstrap; nothing in this inventory ships to
end users.

Every placeholder MUST be resolved or explicitly waived before:

- the first `app/v0.N.M` preview build is cut, **or**
- the repository flips visibility from private to public per
  `docs/43-development-standards.md` §18.2,

whichever comes first. The Accountable Owner owns each resolution.

## Domain / email placeholders

All `*@chromaticity.app` addresses and all `chromaticity.app` URLs in
the repository are **placeholders** pending domain-clearance per
`docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark Review
Checklist (`EV-BRAND-REVIEW`).

| Placeholder | Appears in | Activation dependency |
|---|---|---|
| `legal@chromaticity.app` | `LICENSE`, `.github/ISSUE_TEMPLATE/config.yml` | Domain + mailbox provisioning |
| `security@chromaticity.app` | `SECURITY.md` | Domain + mailbox + PGP/age key publication |
| `conduct@chromaticity.app` | `CODE_OF_CONDUCT.md` | Domain + mailbox provisioning |
| `https://chromaticity.app/.well-known/security-public-key` | `SECURITY.md` | Domain + static-hosting + key publication |
| `https://github.com/chromaticity-app/chromaticity` | `SECURITY.md`, `.github/ISSUE_TEMPLATE/*` | GitHub org + repo naming confirmation |
| `chromaticity.app` trademark use | all governance files | Trademark clearance in live-visuals, audio, and color-science classes per `docs/29-cleanroom-legal-policy-baseline.md` |

Until resolution:

- Any inbound email to these addresses bounces. Reporters will not be
  able to use encrypted email until the `security@` mailbox and the
  `.well-known/security-public-key` endpoint both resolve.
- The GitHub private vulnerability advisory channel referenced in
  `SECURITY.md` remains the primary reporting path once the repo is
  created on GitHub.
- The repo is private through Phase 0 and Phase 1
  (`docs/43-development-standards.md` §18.2), which bounds the window
  during which a public-facing broken link or bounce could harm
  reporters.

## Identity placeholders

| Placeholder | Appears in | Activation dependency |
|---|---|---|
| `@xjbutler` (GitHub handle) | `CODEOWNERS`, `.github/CODEOWNERS` | GitHub org + handle confirmation |
| `@fletcher-hammond` (GitHub handle) | `CODEOWNERS`, `.github/CODEOWNERS` | Deputy onboarding + handle confirmation |

Both handles are reserved in the governance documents even though
Fletcher Hammond's Deputy onboarding is still in progress per
`docs/23-decision-ownership-matrix.md`. If a handle changes during
onboarding, a single `policy`-scoped PR updates `CODEOWNERS`,
`.github/CODEOWNERS`, and this inventory in one commit.

## Signing key / certificate placeholders

These are enumerated here for tracking; the detailed custody and
rotation posture lives in the referenced ADR.

| Placeholder | Scope | Governing ADR |
|---|---|---|
| Source-provenance signing key (gitsign OIDC, GPG, or SSH) | Commit + release-tag signatures | `docs/43-development-standards.md` §9.1 |
| Apple Developer ID certificate | macOS notarized builds | `docs/adr/ADR-014-signing-notarization-auto-update.md` |
| Windows Authenticode certificate | Windows signed installer | `docs/adr/ADR-014-signing-notarization-auto-update.md` |
| Sparkle 2 EdDSA appcast keypair | Auto-update signature | `docs/adr/ADR-014-signing-notarization-auto-update.md` |
| `entitlement-v2` Ed25519 signing key | Entitlement tokens | `docs/adr/ADR-009-activation-and-entitlements.md` |

None of these keys exist yet. Enrollment is a Phase-0 work-package
item; the first `app/v0.N.M` preview build cannot be cut without at
least Source-provenance signing and platform Developer ID enrolled.

## Hosted-service placeholders

| Placeholder | Appears in | Activation dependency |
|---|---|---|
| Self-hosted Sentry host | `docs/adr/ADR-016-observability-and-telemetry-posture.md` | Infra provisioning + hostname on a Chromaticity-owned domain |
| CI runner selection | `docs/43-development-standards.md` §2 (out of scope of that doc) | Release-eng Phase-0 infra item |
| Artifact registry / release host | `docs/43-development-standards.md` §11.1 | Release-eng Phase-0 infra item |

## Application-shell placeholders

Tauri configuration artifacts introduced by the Phase-1 scaffold PR
`build(ui)`. Each item is deferred rather than undecided; the
activation dependency names the work package that will land the
resolved value.

| Placeholder | Appears in | Activation dependency |
|---|---|---|
| `app.chromaticity.desktop` bundle identifier | `src-tauri/tauri.conf.json` | Trademark clearance in live-visuals, audio, and color-science classes per `docs/29-cleanroom-legal-policy-baseline.md`. Changing the bundle identifier after first signed release is a breaking change for auto-update (ADR-014 Sparkle appcast) and macOS notarization; this row MUST be resolved before the first `app/v0.N.M` preview build. |
| `app.security.csp: null` | `src-tauri/tauri.conf.json` | Tauri 2 scaffold ships `csp: null` to unblock initial Svelte bundle load. The concrete CSP (script-src, style-src, connect-src consistent with ADR-013 networking defaults + ADR-016 Sentry opt-in) lands with a dedicated `build(ui)` PR owned by the security-review work package. Until then, the `csp: null` posture is documented here as a known-deferred item. |

## Resolution workflow

1. A Phase-0 work package claims a row.
2. The row's activation produces an evidence-registry update (see
   `docs/26-evidence-artifact-registry.md`) in the same PR.
3. The PR edits this inventory to strike the row (do not delete:
   keep as `~~RESOLVED: YYYY-MM-DD, commit <sha>~~` so the audit
   trail survives).
4. When every row is struck, this document is retired with a final
   `docs(policy): retire phase-0 placeholder inventory` commit.

## Change control

This document is a `docs`-scoped artifact, not a policy document.
Additions and strike-throughs are normal `docs` commits. Removing a
row (rather than striking it) requires a `policy`-scoped PR.
