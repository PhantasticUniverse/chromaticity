# ADR-014: Signing, Notarization, and Auto-Update Pipeline

## Status
Accepted

Design is accepted at Phase 0 (this ADR); pipeline implementation lands at Phase 5 per `11-roadmap-phases.md`. The Phase 0 / Phase 5 split is a scheduling artifact, not a status qualifier.

## Context
Prior review rounds deferred signing, notarization, and auto-update implementation to Phase 5 / post-MVP and noted that "pre-GA they become P0". That posture is fine for implementation but leaves the **design** undefined. On macOS, Developer ID signing and notarization via notarytool requires an Apple Developer account, a hardware-stored private key, and a notary-ticket-staple step per build; this has weeks of operational setup lead time. On Windows, Authenticode with an EV certificate similarly requires a trusted CA enrollment. Deferring the design to Phase 5 means MVP freeze will slip waiting on certificate issuance. The minimal cure is to approve the design at Phase 0 (this ADR) and keep pipeline implementation at Phase 5.

The reference app ships with Sparkle (`Contents/Frameworks/Sparkle.framework`) for auto-updates on macOS and BugsplatMac for crash reporting. Sparkle is a well-established choice for Cocoa apps. Chromaticity reuses Sparkle on macOS for the same reason.

## Decision

### 1. Code signing

#### macOS
- Apple Developer ID Application certificate, hardware-stored on a YubiKey or equivalent FIPS-140-2 key.
- Hardened runtime enabled (`--options runtime`).
- Timestamped signature (`--timestamp`).
- Entitlements file declares required capabilities only (microphone, camera if enabled by operator, network client, ports for OSC).
- Notarized via `notarytool submit --wait` in CI release pipeline; stapled with `stapler staple` before DMG packaging.

#### Windows
- Authenticode signature with an EV code-signing certificate from a trusted CA.
- Signed installer (MSI or NSIS) and signed main executable.
- Timestamp server configured to avoid post-expiry signature invalidation.
- SmartScreen reputation is built over time; no Microsoft-backed fast path at MVP.

### 2. Auto-update channel

#### macOS
- Sparkle 2 (Swift) - parity footprint with the reference app's `Sparkle.framework`.
- **EdDSA-only signature verification.** `SUPublicEDKey` is the sole appcast-verification key embedded in `Info.plist`. The legacy Sparkle 1 DSA path (`SUPublicDSAKeyFile`, `synesthesia_full_dsa_pub.pem`-style PEM files) is **not shipped**, ever. Updates that fail EdDSA verification fail closed; no DSA fallback chain exists. The reference app ships both keys to support rollback to pre-Sparkle-2 releases; Chromaticity ships only Sparkle 2, so the DSA surface is unnecessary attack surface we decline to carry.
- Appcast served over HTTPS from a Chromaticity-controlled domain.

#### Windows
- WinSparkle (Sparkle port for Win32) or a custom signed-manifest flow. Decision deferred to Phase 5 implementation; ADR records both as acceptable.

### 3. Live-show update discipline
- Auto-update checks are **disabled by default while a show is live** (indicated by an active playlist or any adapter in `Connected` state with recent traffic).
- Update notifications queue silently and surface only after the show ends (all adapters in `Disconnected` / `Idle` for > 10 minutes).
- Explicit operator trigger via "Check for Updates..." bypasses the live-show guard.
- An update never restarts the app without explicit operator consent. Silent restart during a show is a banned behavior per `NFR-1`.

### 4. Key management
- Code-signing keys are stored offline in hardware-backed enclaves (YubiKey / HSM).
- Signing is gated behind a single-human approval step in the release pipeline; the Accountable Owner holds the key PIN.
- Sparkle EdDSA signing key rotates on a defined cadence (annual); rotation is documented in the security runbook.
- Revocation plan documented: if a signing key is compromised, builds are re-issued under a new key and published via appcast with a "mandatory update" flag.

### 5. Release pipeline (target Phase 5)
1. CI builds universal macOS binary (arm64 + x86_64 via Rosetta-safe Rust toolchain) and Windows x86_64 binary.
2. Sign, notarize (macOS), timestamp.
3. Bundle into DMG (macOS) / MSI (Windows).
4. Sign the installer.
5. Compute EdDSA Sparkle signature of the installer.
6. Upload to CDN, update appcast with new version entry and signature.
7. Internal smoke test on a reference install; only then publish.

### 6. Pre-GA blockers
- Apple Developer ID certificate issued (Phase 1).
- Windows EV code-signing certificate issued (Phase 1 / Phase 2).
- Sparkle EdDSA keypair generated and public key embedded in app (Phase 0 since public key is a build input).
- Appcast host provisioned (Phase 4d or 5).

### 7. Bundle hygiene (normative)
The reference bundle ships several artifacts that have no runtime purpose and either leak implementation detail or inflate download size. Chromaticity's build pipeline rejects any release candidate that contains any of the following:

- **C/C++ header files.** No `.h`, `.hpp`, or `.hxx` anywhere in `Contents/Resources/` or `Contents/Frameworks/*/Headers/` (the reference ships `Contents/Resources/opencv2/*.hpp` from its build tree - that is the exact class of artifact we exclude).
- **Debug symbols.** No `.pdb` (Windows) or `.dSYM` bundles inside the app bundle. dSYMs are uploaded to the crash-reporter back-end (per `adr/ADR-016-observability-and-telemetry-posture.md`) and retained separately for release-candidate symbolication, not shipped in the installer.
- **Source files** from build-tool caches (e.g. `.rs`, `.swift`, `.m` files). Regex check covers `*/.cargo/*`, `*/src/*`.
- **Test fixtures and golden-image corpora** that are not runtime-required. These belong in `chromaticity-planning/34-golden-corpus-definition.md` artifact storage, not in the shipping bundle.
- **Legacy signing keys.** Specifically, no Sparkle 1 DSA `.pem` files (`synesthesia_full_dsa_pub.pem` or similar). See Sec.2 EdDSA-only posture.
- **Third-party license-compliance tarballs** or `LICENSE.txt` duplicates in deep subfolders - licenses aggregate into one `LICENSES.md` at bundle root plus per-framework embedded metadata, not scattered.
- **Build-system config** (`package.json`, `webpack.config.js`, `.asmdef`, etc.) that ships as a side-effect of copying a third-party library tree in full. If a library directory contains such files, the library is pruned before bundling.

These rules are enforced by a pipeline step that walks the built bundle and matches against a deny-list glob spec; a match fails the release. Evidence row: `EV-BUNDLE-HYGIENE`.

Rationale: the reference bundle has been observed shipping ~20 MB of OpenCV C++ headers, the full `lygia/` source tree including its `webpack.config.js`, and a legacy `.pem` that serves only to enable DSA verification for a signing path Chromaticity does not use. Eliminating these shrinks the installer, reduces notarization review surface, and removes mis-signals to supply-chain scanners.

## Consequences

### Positive
- MVP freeze is not blocked by certificate lead time because design decisions and key generation happen in Phase 0; certificate issuance can run in parallel with implementation.
- Sparkle 2 reuses an established pattern (reference app ships it).
- Live-show guard preserves NFR-1 output continuity under update pressure.

### Negative
- Annual cert renewal adds operational overhead.
- Windows EV cert costs $300-800/year; Apple Developer account costs $99/year.
- Hardware key requirement means signing cannot run unattended; single-human approval step adds release latency.

## Alternatives Considered

- **No code signing at MVP**: rejected. macOS Gatekeeper blocks unsigned binaries; Windows SmartScreen flags unsigned installers aggressively.
- **Ad-hoc signing**: rejected on macOS (Gatekeeper prompts on every launch); not applicable on Windows.
- **Self-hosted CA**: rejected. Users cannot validate.
- **Custom update protocol instead of Sparkle**: rejected on macOS (Sparkle is battle-tested; rewriting it is effort with no upside). Remains open on Windows.
- **Auto-update disabled by default**: rejected. Operators expect auto-updates; the live-show guard resolves the safety concern without sacrificing the feature.

## Evidence
- Phase 0 exit covers this ADR under `EV-ARCH-CONTRACT-BASELINE`.
- Release pipeline evidence (Phase 5) tracked as `EV-RELEASE-PIPELINE` (new row in registry).
- `EV-BUNDLE-HYGIENE` - built app bundle contains no disallowed artifact per Sec.7.
- `EV-SIGNING-INTEGRITY` - no DSA verification code path or legacy DSA key resides in the shipped bundle (grep-assertion + runtime assertion that `SUPublicDSAKeyFile` is unset).

## Traceability
- `01-charter.md` Out-of-Scope list (auto-updater implementation) amended to note this ADR covers design.
- `11-roadmap-phases.md` Phase 0 gates reference this ADR.
- `13-risk-register.md` R-14/R-15/R-16 (legal/brand/license) remain applicable to signed-distribution content.
- `31-supportability-privacy-ops-policy.md` support-bundle hygiene aligns with Sec.7 bundle hygiene rules.
- `adr/ADR-016-observability-and-telemetry-posture.md` defines crash-reporter vendor (Sentry self-hosted) and dSYM retention path; the reference's BugsplatMac is not shipped.
- `adr/ADR-009-activation-and-entitlements.md` defines the embedded public-key set used for `entitlement-v2` tokens, signed by the same release-pipeline discipline.
