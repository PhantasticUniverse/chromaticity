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

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. This ADR is a reserved slot; the eleven evidence rows below are post-MVP placeholders enumerated in `26-evidence-artifact-registry.md` so that forward-references resolve. None can transition out of `Planned` until this ADR is authored, ratified, and the corresponding implementation slices ship.

- `EV-DATA-INGEST-SCHEMA` — `DATA_SOURCES` scene.json schema conformance. Activated when this ADR locks the schema shape per `41-data-ingestion-differentiator.md` Sec.Forward-compat reservations and the slice that lands `engine/scene` parser support ships.
- `EV-DATA-INGEST-PROVIDER-REGISTRY` — provider registry structure, licensing posture, capability-token binding. Activated on this ADR's provider-registry decision plus `adr/ADR-009-activation-and-entitlements.md` capability-token surface.
- `EV-DATA-INGEST-CASSETTE` — cassette record/replay reproducibility per `22-compatibility-versioning-policy.md` Sec.Data-Pinned Fixtures. Activated when this ADR ships the cassette format decision.
- `EV-DATA-INGEST-FRAME-BUDGET` — data-ingestion frame-budget reservation under live fetch per `10-performance-latency-budgets.md` Sec.Frame Performance reserved post-MVP data-upload budget. Activated on this ADR's thread-pool + scheduler-interaction decision.
- `EV-DATA-INGEST-BACKPRESSURE` — backpressure and degrade-on-stall posture. Activated on this ADR's backpressure policy decision.
- `EV-DATA-INGEST-OFFLINE` — every provider has a defined fallback state and renders sanely with the source unavailable. Activated on this ADR's error-state rendering UX decision.
- `EV-DATA-INGEST-OBSERVABILITY` — runtime outbound destinations are a strict subset of `OBSERVABILITY-INVENTORY.md` Sec.Data Sources per `adr/ADR-016-observability-and-telemetry-posture.md` Sec.Post-MVP data-ingestion interaction. Activated on this ADR plus the first inventory entry per provider.
- `EV-DATA-INGEST-LICENSE` — extends `EV-LICENSE-AUDIT` scope to cover each shipped data provider (provenance, license class, ToS posture). Activated per-provider when this ADR's provider-registry decision admits the provider per `29-cleanroom-legal-policy-baseline.md` Sec.External Data Source Licensing Policy.
- `EV-DATA-INGEST-CAPABILITY-GATING` — disabled-capability tokens block the data adapter with `data-entitlement-denied` diagnostic per `adr/ADR-009-activation-and-entitlements.md` Sec.Capability namespace `data.ingest.*`. Activated on this ADR + ADR-009 capability-token surface.
- `EV-DATA-INGEST-KEY-STORAGE` — operator-supplied API keys stored in OS secrets store only and excluded from support bundles, crash reports, telemetry. Activated on this ADR's key-handling decision plus `adr/ADR-016-observability-and-telemetry-posture.md` payload-redaction posture.

The reserved-slot status `Proposed (reserved slot; normative content deferred post-MVP)` is the documented acceptable state during Phase 0 through MVP per `43-development-standards.md` Sec.14; `EV-ARCH-CONTRACT-BASELINE` (`26-evidence-artifact-registry.md` row 22) does not enumerate this ADR among its gating-ADR artifact set precisely because the reserved-slot posture is intentional and contract-pack closure is not blocked on this ADR's authoring.

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
