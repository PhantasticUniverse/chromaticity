# ADR-017: Real-Time External Data Ingestion

## Status
Proposed (reserved slot; normative content deferred post-MVP)

This ADR slot is named so that forward-compat references in the MVP corpus resolve. The normative decision content is not yet authored; design begins post-MVP per the reservation package in `41-data-ingestion-differentiator.md`. `EV-ADR-LINT` per `43-development-standards.md` Sec.14 accepts `Proposed` as a valid status for reserved slots; the `Status:` line uses only an allowed token and any nuance lives in the Context section below.

## Incoming requirements package

The full pre-authoring brief lives in `41-data-ingestion-differentiator.md`. That document enumerates:

- Product posture and differentiator stance.
- V2 (network-transported) vs V3 (hardware / SDR) scope.
- Data-class taxonomy.
- Forward-compatibility reservations already locked in MVP (uniform namespace, scene.json key, capability strings, OSC prefix, adapter-boundary extension points, data-pinned fixtures, outbound-inventory rules, frame-budget reservation, licensing policy).
- Design questions deferred to this ADR (cassette format, transform pipeline shape, thread-pool model, backpressure, cache policy, provider registry, operator UX, fixture pack, plugin architecture, V3 SDR scope, SELECT grammar, error-state rendering UX).
- Candidate V2 provider shortlist.
- Reserved evidence artifacts (`EV-DATA-INGEST-*` family).
- Open decisions requiring Accountable Owner signoff before authoring starts.

## Author entry criteria

Before this ADR may be authored:

1. MVP ships and stabilizes.
2. The four Accountable-Owner decisions in `41-data-ingestion-differentiator.md` Sec.Open decisions requiring Accountable Owner signoff before ADR-017 authoring starts are recorded.
3. At least one V2 provider has completed a discovery spike (provenance + license class + transport prototype) so the ADR can ground its cadence, payload-size, and backpressure decisions in at least one real data point.

## Traceability

- `41-data-ingestion-differentiator.md` - incoming requirements package (primary reference).
- `36-reactive-symbol-catalog.md` Sec.1.6 - reserved `syn_Data_*` namespace.
- `28-implementation-contract-pack.md` CP-12 - reserved `DATA_SOURCES` scene.json contract.
- `adr/ADR-005-integration-adapter-boundary.md` Sec.Post-MVP Extension Points - adapter-boundary axes this ADR must exercise.
- `adr/ADR-009-activation-and-entitlements.md` Sec.Capability namespace - reserved `data.ingest.*` capability strings.
- `adr/ADR-016-observability-and-telemetry-posture.md` Sec.Post-MVP data-ingestion interaction - outbound-traffic-inventory rules this ADR must honor.
- `22-compatibility-versioning-policy.md` Sec.Data-Pinned Fixtures - determinism contract this ADR must integrate with.
- `29-cleanroom-legal-policy-baseline.md` Sec.External Data Source Licensing Policy - per-provider licensing posture.
- `10-performance-latency-budgets.md` Sec.Frame Performance - reserved post-MVP data-upload budget this ADR must meet.
- `13-risk-register.md` `R-28` - architectural-debt risk this ADR closes on ratification.
