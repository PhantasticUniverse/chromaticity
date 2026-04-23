# ADR-016: Observability and Telemetry Posture

## Status
Accepted

## Context
The reference Synesthesia app ships with outbound network traffic to two third-party services: **Mixpanel** (analytics; observed as `NSExceptionDomains` entry for `mixpanel.com` in `Contents/Info.plist`) and **Bugsplat** (crash reporting; observed as `BugsplatServerURL` key pointing to `https://synesthesia.bugsplat.com/` and `Contents/Frameworks/BugsplatMac.framework` linkage). The reference does not clearly surface these to operators, does not provide an opt-out, and creates outbound traffic on every launch.

Chromaticity's target customer is a **live-performance operator**. In that context, unannounced outbound traffic is a trust violation: shows run on venue Wi-Fi, guest networks, and air-gapped backline; a surprise HTTPS call mid-show is an incident waiting to happen. `31-supportability-privacy-ops-policy.md` sets general telemetry policy but leaves the vendor choice and outbound-traffic posture unbounded, and `adr/ADR-014-signing-notarization-auto-update.md` Sec.Out-of-Scope explicitly punts crash-reporting vendor selection to this ADR.

This ADR locks the three outbound-traffic decisions together (analytics, crash reporting, health metrics) because they share the same trust surface and the same ATS-exception concerns in `Info.plist`.

## Decision

### Default posture: zero outbound traffic except Sparkle update checks
On a fresh install, Chromaticity emits outbound traffic to exactly one endpoint: the Sparkle appcast URL governed by `adr/ADR-014-signing-notarization-auto-update.md`. No analytics, no crash reports, no health pings, no feature-flag fetches. Auto-update itself is opt-out-able via a settings toggle.

### Analytics (`analytics.enabled`)
- **Default: off.** No analytics events are collected, buffered, or transmitted.
- **Vendor: none.** Chromaticity does **not** integrate Mixpanel, Amplitude, PostHog-cloud, Segment, or any analytics SDK at MVP. No third-party analytics domain appears in `Info.plist` `NSAppTransportSecurity` exceptions.
- **Post-MVP addition path**: if and when product analytics becomes necessary, the only acceptable vendor profile is a **self-hosted** collector (self-hosted PostHog or equivalent) running on a Chromaticity-owned domain. The ADR will be re-opened before any such integration ships, with an explicit data-inventory section describing exactly what events and fields are transmitted.
- Operator UI affordance: even in the default-off state, Settings surfaces an "Analytics" row reading **"Not collected."** so the posture is auditable without reading docs.

### Crash reporting (`crash.reporting.enabled`)
- **Default: off.** Crashes produce a local minidump/backtrace artifact saved to the support-bundle directory. The operator chooses whether to submit it.
- **Vendor: Sentry, self-hosted.** When the operator opts in, crash payloads are transmitted to a Chromaticity-owned Sentry instance (`crash.<chromaticity-domain>`). Public Sentry SaaS is rejected because payload residency cannot be controlled.
- **Bugsplat rejected** (reference's choice): vendor lock-in to a smaller ecosystem with no self-host option.
- **Payload redaction (required before any transmission)**:
 - Excluded: scene source code (`main.glsl`, `script.js`), media payload content, recent audio buffers, operator-entered free-text (preset names, playlist names).
 - Included: build version, profile class, subsystem label, stack trace with source-file / line-number only (no surrounding source), GPU vendor/driver, OS version, minimum redacted performance counters (frame time percentile bucket).
- **Submission model**: opt-in per submission. No "upload all future crashes" toggle. Each crash shows a preview of the payload before submission.
- **Offline and local-only support**: the local minidump is always available via the support-bundle export; submission is a convenience, not a requirement.

### Health metrics / telemetry (`telemetry.enabled`)
- **Default: off.** No runtime health metrics are transmitted.
- **Vendor: Prometheus exposition over loopback only.** When operators opt in, the runtime exposes a local Prometheus text endpoint on `127.0.0.1:<port>` (port chosen per `adr/ADR-013-networking-defaults-and-security.md` bind defaults). Operators who want to aggregate across machines run their own scraper.
- **No outbound HTTPS telemetry at any point.** Health metrics never leave the local machine without an explicit operator-configured scraper pointed at the loopback endpoint.
- Scheme and metric catalog: documented in `31-supportability-privacy-ops-policy.md`.

### Opt-in discovery and consent UX
- On first launch, the app shows a single **Observability** settings card with three toggles (Analytics / Crash Reporting / Health Metrics) - all off - and a short paragraph describing what each toggle does when enabled.
- No "Skip for now; we'll collect by default and ask later" dark-pattern path exists. There is no "ask later."
- The consent state is persisted per-installation, not per-session; changing it requires returning to Settings.

### Info.plist and ATS posture
- `NSAppTransportSecurity` contains **no third-party exception domains**. Specifically, neither `mixpanel.com` nor `bugsplat.com` appears anywhere in the bundle.
- `NSMicrophoneUsageDescription` and `NSCameraUsageDescription` match reference semantics but the copy is Chromaticity-original per `29-cleanroom-legal-policy-baseline.md`.
- The self-hosted Sentry domain, if opt-in crash reporting is enabled at build time, is declared under `NSAppTransportSecurity` with forward-secrecy required; no insecure-HTTP exception.

### Transparency artifact
- Every release ships with `OBSERVABILITY-INVENTORY.md` in the user-data root describing exactly what outbound destinations the binary can contact, keyed by which settings toggle enables each one. `EV-OBSERVABILITY-INVENTORY` asserts this document matches the observed outbound-destination list at runtime.
- Release notes for any change to outbound behavior must include a diff against the previous inventory.

### Post-MVP data-ingestion interaction (reserved)
The real-time external-data differentiator (`41-data-ingestion-differentiator.md`, reserved ADR-017) will introduce a new class of outbound destinations: per-data-source providers (weather APIs, orbital-element hosts, geospatial tile servers, event streams). When that feature ships, this ADR's posture is preserved by these rules:

- **Default-off.** No data source is enabled by default. Each source is an explicit per-source opt-in in Settings, parallel to the Analytics / Crash Reporting / Health Metrics toggles.
- **Capability-gated.** Each source declares which `data.ingest.*` capability it requires (`adr/ADR-009-activation-and-entitlements.md`). Without the capability, the source is inert regardless of the per-source toggle state.
- **Inventory expansion.** `OBSERVABILITY-INVENTORY.md` grows a Sec.Data Sources section enumerating each shipped provider (domain, capability, purpose). `EV-OBSERVABILITY-INVENTORY` continues to assert the runtime egress set equals the declared set.
- **No data-source payload logging at infrastructure level.** Data-source fetch metadata (timing, success/failure, bytes transferred) may be surfaced in the opt-in telemetry endpoint per `telemetry.enabled`, but fetch payload contents are never logged, telemetry-transmitted, or crash-reported. `EV-CRASH-PAYLOAD-REDACTION` extends to exclude any bytes returned from a `DATA_SOURCES` fetch.
- **ATS exceptions remain forbidden.** Every post-MVP data source must be reachable under the default `NSAppTransportSecurity` posture (TLS 1.2+, forward secrecy, no insecure-HTTP exceptions). Providers that cannot meet this floor are not shipped.

This reservation keeps the operator-trust differentiation intact through V2+ even as the outbound-destination set grows.

## Consequences

### Positive
- **Operator trust differentiation.** The reference phones home by default; Chromaticity does not. This is a marketable security-posture advantage in venues with hostile or metered network environments.
- **Zero third-party ATS exceptions** makes notarization simpler and reduces supply-chain surface (`adr/ADR-014-signing-notarization-auto-update.md`).
- **Self-hosted crash reporting** (Sentry) keeps payload residency controllable and removes vendor pricing risk.
- **Loopback-only health metrics** integrate with existing operator observability stacks (Grafana / Prometheus) without forcing a cloud dependency.

### Negative
- **Slower product learning loop.** Without analytics, Chromaticity relies on support channels, community feedback, and explicit beta programs instead of passive telemetry. This is a deliberate trade.
- **More operational cost** to host the opt-in Sentry instance than to pay a SaaS vendor. Accepted as a non-trivial but bounded cost.
- **No "silent" field crash data.** Field crashes only surface when operators opt in, which means lower bug-report volume than a default-on vendor would yield. Mitigated by the support-bundle policy in `31-supportability-privacy-ops-policy.md`, which produces the same payload locally.

## Alternatives Considered
- **Ship Bugsplat (reference parity)**: rejected. Hostile default; no self-host; vendor lock-in.
- **Ship Sentry SaaS (default-off)**: rejected. Payload residency cannot be controlled; incompatible with enterprise operator expectations.
- **Default-on analytics with prominent opt-out**: rejected. Violates the consent posture above; creates a mid-show outbound-traffic risk for operators who miss the opt-out.
- **No crash reporting at all**: rejected. Local minidumps without a submission path force operators through manual upload friction when they do want to report.
- **Crash reporter as a separate ADR**: rejected. Analytics, crash reporting, and health metrics share one trust surface and one `Info.plist` configuration; splitting them obscures the consent contract.

## Evidence
- `EV-OBSERVABILITY-INVENTORY` - the shipped `OBSERVABILITY-INVENTORY.md` enumerates every outbound endpoint and matches runtime behavior (e.g. network-trace test with all toggles off sees traffic only to the Sparkle appcast URL).
- `EV-CRASH-PAYLOAD-REDACTION` - fixture crash triggers scene-source-loaded and script-source-loaded conditions; payload inspection confirms no scene source, script source, or media payload leaves the process.
- `EV-ANALYTICS-ZERO-DEFAULT` - a fresh install with no settings touched produces zero packets to any `mixpanel.com`, `amplitude.com`, `segment.com`, `posthog.com`, or similar domain over a 14-day observation window.
- `EV-HEALTH-METRICS-LOOPBACK-ONLY` - with `telemetry.enabled = true`, the only listening socket is loopback; external network observation sees zero egress.
- `EV-ATS-NO-THIRD-PARTY-EXCEPTIONS` - `Info.plist` contains no third-party `NSExceptionDomains` entries.

## Traceability
- `31-supportability-privacy-ops-policy.md` (telemetry redaction, crash-report payload policy, retention).
- `adr/ADR-013-networking-defaults-and-security.md` (loopback-default bind policy for telemetry endpoint).
- `adr/ADR-014-signing-notarization-auto-update.md` (Sparkle update posture remains the sole default outbound channel).
- `29-cleanroom-legal-policy-baseline.md` (privacy-policy copy originates from Chromaticity counsel, not reference).
- `01-charter.md` Sec.Out of Scope (commerce / analytics product decisions are out of scope at MVP; this ADR locks the runtime posture only).
- `26-evidence-artifact-registry.md` gains `EV-OBSERVABILITY-INVENTORY`, `EV-CRASH-PAYLOAD-REDACTION`, `EV-ANALYTICS-ZERO-DEFAULT`, `EV-HEALTH-METRICS-LOOPBACK-ONLY`, `EV-ATS-NO-THIRD-PARTY-EXCEPTIONS`.
