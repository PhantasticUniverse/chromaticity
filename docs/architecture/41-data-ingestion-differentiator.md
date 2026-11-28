# Real-Time External Data Ingestion (Differentiator Reservation)

## Purpose

This document is a **design reservation**, not a build spec. It exists so that the MVP does not foreclose post-MVP design space for what is intended to be one of Chromaticity's strongest differentiators: real-time external data streams as first-class scene inputs.

No MVP feature is gated by this document. Nothing in here adds implementation work to Phase 1-5. Its job is to ensure every architectural decision that *could* impact data ingestion has either been made with post-MVP flexibility in mind (and is cross-referenced from this document), or has been flagged as needing a reversal later (risk `R-28`).

The normative spec for the feature itself is deferred to `adr/ADR-017-real-time-external-data-ingestion.md` (slot reserved; authoring begins after MVP ships).

## Product posture

> **Chromaticity is the live-visuals runtime where external data streams are first-class scene inputs.**

Other live-visuals tools (TouchDesigner, Notch, VDMX via OSC) can ingest external data through bespoke operator-built plumbing. Chromaticity's differentiation is that it's **scene-declarative**: the scene author writes a `DATA_SOURCES` stanza in `scene.json` and the runtime publishes `chroma_Data_*` uniforms. No operator plumbing per source. No per-operator IT work. The author's scene runs the same on any operator's machine.

Four supporting qualities make this viable where competitors can't (or won't):
1. **Scene-declarative schema** - the author describes the data, not the transport.
2. **Live-performance safe** - `adr/ADR-016-observability-and-telemetry-posture.md` guarantees no undeclared outbound traffic and no third-party ATS exceptions; data sources are per-source opt-in with operator awareness.
3. **Replayable** - data-pinned fixtures (`22-compatibility-versioning-policy.md` Sec.Data-Pinned Fixtures) preserve `NFR-2 Determinism` for data-driven scenes.
4. **Capability-gated commerce** - the capability-bitmask substrate (`adr/ADR-009-activation-and-entitlements.md`) lets paid data SKUs ship as a data change, not a runtime rebuild.

## Scope

### V2 (primary landing): network-transported external data
The V2 scope covers any data source reachable over TCP/UDP/HTTP/WebSocket/MQTT from a Chromaticity install.

- **HTTP-polling**: weather grids (NOAA GRIB2, MRMS radar), orbital elements (Celestrak TLE), geospatial tiles (OSM, satellite imagery), elevation rasters.
- **WebSocket / MQTT subscription**: live event streams (ADS-B aircraft positions, market ticks, IoT sensor telemetry).
- **REST APIs**: any provider with operator-supplied API keys (commercial weather, mapping, data feeds).
- **Local file-watch**: JSON/CSV files the operator refreshes via other tooling (reserved; decision in ADR-017).

Transformations in scope: query/filter, coordinate transformation, time-alignment, field resampling.

### V3 (deferred): hardware-backed data
The V3 scope adds local-hardware data sources.

- **Software-defined radio (SDR)**: RTL-SDR, HackRF, LimeSDR. Real-time I/Q and band-power ingestion.
- **Serial / USB instruments**: MIDI-like protocols over USB-serial, lab instruments.
- **Other local hardware**: platform-specific sensor access (Mac's CoreLocation, Windows sensor stack).

V3 is materially harder than V2: USB device enumeration, platform driver stacks, and real-time I/Q processing introduce architectural weight that V2 does not. V3 gets its own companion ADR alongside ADR-017 when the time comes.

### Explicitly out of scope (any version)

- Cloud-synced collaborative data sources (different product surface; belongs in a future "remote control" story, not data ingestion).
- Authentication-heavy enterprise data (OAuth 2.0 interactive, SAML, enterprise-SSO). Operators can supply pre-issued bearer tokens; the runtime does not perform interactive auth flows.
- Per-viewer or per-session data (data sources are global to the Chromaticity install, not per-viewer of a rendered output).

## Data class taxonomy

| Data class | Representative examples | Cadence (wall-clock) | Payload shape (declared `TYPE`) | Transport |
|---|---|---|---|---|
| Orbital / positional | ISS, Starlink, GNSS constellations from TLEs | 0.1-1 Hz observer-pos derivation; TLEs refresh daily | `vec3` or `array` of `vec3` | HTTP polling |
| Geospatial static | Country boundaries, OSM features, elevation rasters | event-driven, typically on scene load | `field-2d` (raster), `array` (feature list) | HTTP polling, tile fetch |
| Weather / environmental | NOAA radar, GRIB2 grids, lightning strikes, AQI | 1 min - 5 min refresh | `field-2d` / `field-3d` | HTTP polling |
| Aviation / transport | ADS-B, ship AIS, GTFS-RT | 1 Hz - 10 Hz event stream | `event` ring buffer | WebSocket / MQTT |
| Market / exchange | Price ticks, order-book depth, index values | 1 ms - 1 s | `scalar` / `array` / `event` | WebSocket / FIX-lite |
| Social / event firehose | RSS, Mastodon streaming, sports scores | event-driven | `event` ring buffer | WebSocket / HTTP long-poll |
| Scientific / physical | Seismograph feeds, tide gauges, solar weather | seconds to minutes | `scalar` / `array` | HTTP polling |
| RF spectrum *(V3)* | SDR I/Q -> band power, ADS-B demodulation | 1 MHz+ sample rate -> 60+ Hz frame rate | `array` (sampler1D) | USB hardware |

## Forward-compatibility reservations locked in MVP

Every item below is an MVP-scope reservation that prevents a V2/V3 breaking change. Each one is a small cost paid now for a large cost avoided later.

| # | Reservation | Location |
|---|---|---|
| 1 | `chroma_Data_*` uniform namespace (shape primitives for scalar/vec/array/field/event) | `36-reactive-symbol-catalog.md` Sec.1.6 |
| 2 | `DATA_SOURCES` scene.json root key (schema parsed at MVP, zero-valued publication) | `02-synesthesia-contracts.md` Sec.1 and `28-implementation-contract-pack.md` CP-12 |
| 3 | `CR-07` coercion class for data-source-unavailable handling | `28-implementation-contract-pack.md` CP-01 |
| 4 | Capability strings `data.ingest.network`, `data.ingest.geospatial`, `data.ingest.sdr` | `adr/ADR-009-activation-and-entitlements.md` Sec.Capability namespace |
| 5 | OSC prefix `/data/*` and `/data/subscribe/*` reserved with `prefix-reserved` reject at MVP; `OSC-CONF-16` asserts it | `08-io-integrations.md` |
| 6 | Adapter-boundary post-MVP extension points (cadence, payload, thread-pool, backpressure) acknowledged | `adr/ADR-005-integration-adapter-boundary.md` Sec.Post-MVP Extension Points |
| 7 | Data-pinned fixture concept and profile tags (`data-live`, `data-cassette`, `data-unavailable`) | `22-compatibility-versioning-policy.md` Sec.Data-Pinned Fixtures |
| 8 | Observability-inventory rules for data-source outbound destinations | `adr/ADR-016-observability-and-telemetry-posture.md` Sec.Post-MVP data-ingestion interaction |
| 9 | Data-source licensing policy extending `EV-LICENSE-AUDIT` | `29-cleanroom-legal-policy-baseline.md` Sec.External Data Source Licensing Policy |
| 10 | Reserved frame-budget row (data-upload <= 0.5 ms p95 on render thread; fetch wall-clock off-thread) | `10-performance-latency-budgets.md` Sec.Frame Performance |
| 11 | Risk row `R-28` for architectural-debt-if-reservations-are-skipped | `13-risk-register.md` |

## Design questions deferred to ADR-017

These are the questions ADR-017 must answer. Listing them here so the ADR author (post-MVP) has a complete agenda.

1. **Cassette format.** Serialization, time-indexing, signing, size budgets, compression. Must be forward-compatible (a V2 cassette must still replay under V3). Candidate formats: JSON-lines, MessagePack streams, a custom binary.
2. **Data-transform pipeline shape.** Scene GLSL is too slow and too small for per-source transforms (coordinate conversion, query filter, temporal interpolation). Script host (`adr/ADR-012-script-sandbox-engine.md`) is per-frame and sandboxed. Options:
 - Transforms expressed as declarative JSON in the `DATA_SOURCES` stanza (bounded expressiveness, easy to lint).
 - Transforms authored as a separate Chromaticity DSL.
 - Transforms implemented in per-provider native code (only adapter authors can write them).
3. **Thread-pool model.** Data adapters need their own thread pool, distinct from the single IO thread in `adr/ADR-006-thread-and-process-model.md`. Pool sizing, adapter quotas, shutdown semantics.
4. **Backpressure and fetch-scheduling.** What happens when a provider is slow, absent, rate-limited, or misbehaving. Jitter budget for fetch scheduling so adapters don't line up at frame boundaries.
5. **Cache policy.** Disk cache shape, size limits, TTL, invalidation, per-provider opt-out, cache sharing across scenes that reference the same source.
6. **Provider registry.** Manifest format enumerating each shipped adapter, with fields per `29-cleanroom-legal-policy-baseline.md` Sec.External Data Source Licensing Policy: provider, license class, attribution surface, redistribution posture, cost model, capability requirement.
7. **Operator UX.** Per-source configuration panel, API-key storage (OS secrets store), cadence override, viewport picker (geospatial), live fetch-health indicators, cost-aware warnings ("this source charges per query; you are polling at 1 Hz").
8. **Fixture pack.** Bundled cassette fixtures for common "just works offline" demos - analogous to the Shadertoy iChannel fixture pack in `38-scene-library-and-import-spec.md` Sec.5.2a.
9. **Plugin architecture.** Can third parties author data adapters? If yes: trust boundary (wasm sandbox? native plugin DLLs with signed manifests?), capability exposure, license-audit implications. Probably reuses `adr/ADR-012-script-sandbox-engine.md` primitives, but is a separate policy decision.
10. **V3 SDR scope.** Device enumeration model, cross-platform driver posture (libusb on Linux/Mac vs WinUSB), hardware permission UX, real-time I/Q processing pipeline, interaction with audio-engine (both are real-time streams with latency budgets).
11. **`DATA_SOURCES` SELECT grammar.** How provider-specific query parameters are expressed without every provider inventing its own config schema.
12. **Error-state rendering UX.** A scene whose data source is unavailable must look reasonable (not black, not flashing-red). Default-fallback shape and authoring affordance.

## Candidate V2 provider shortlist (illustrative, not committing)

First-party Chromaticity V2 ship list, subject to revision during ADR-017 authoring and per-provider licensing review:

| Provider | License class (per `29-cleanroom-legal-policy-baseline.md`) | Representative data |
|---|---|---|
| NOAA Weather (GRIB2 / MRMS / NWS API) | `public-domain` | Radar reflectivity, hourly forecast grids, watches/warnings |
| Celestrak | `permissive-attribution` | TLEs for named satellites and satellite groups |
| USGS Earthquake Hazards | `public-domain` | Real-time earthquake event stream |
| OpenStreetMap (tiles + Overpass API) | `permissive-attribution` | Basemap tiles, feature queries |
| FAA / ADS-B Exchange | provider-dependent | Aircraft position / heading / altitude |
| NASA GIBS | `public-domain` | Earth-imagery tiles |
| NOAA Space Weather | `public-domain` | Solar wind, Kp index, auroral activity |
| Operator-BYO commercial (e.g. premium weather, financial) | `commercial-per-query` | Whatever the operator pays for |

The V2 ship list doubles as the `EV-DATA-INGEST-PROVIDER-REGISTRY` coverage target.

## Evidence artifacts (authored when ADR-017 ships)

Reserved evidence IDs (registered as post-MVP placeholders in `26-evidence-artifact-registry.md`; activated when ADR-017 ships):

- `EV-DATA-INGEST-SCHEMA` - `DATA_SOURCES` schema conformance against the full taxonomy.
- `EV-DATA-INGEST-PROVIDER-REGISTRY` - every shipped provider has provenance, license class, capability mapping, and outbound-inventory entry.
- `EV-DATA-INGEST-FRAME-BUDGET` - upload <= 0.5 ms p95, fetch <= provider bound, no frame-thread blocking.
- `EV-DATA-INGEST-OFFLINE` - every provider has a defined fallback state and renders sanely with the source unavailable.
- `EV-DATA-INGEST-CASSETTE` - cassette write/replay is bit-exact over the full taxonomy.
- `EV-DATA-INGEST-OBSERVABILITY` - runtime outbound destinations are a strict subset of `OBSERVABILITY-INVENTORY.md Sec.Data Sources`.
- `EV-DATA-INGEST-LICENSE` - extends `EV-LICENSE-AUDIT` to each shipped provider.
- `EV-DATA-INGEST-CAPABILITY-GATING` - disabled-capability tokens block the adapter with `data-entitlement-denied`.
- `EV-DATA-INGEST-KEY-STORAGE` - operator-supplied API keys are stored in OS secrets store only and never surface in support bundles, crash reports, or telemetry.

## Open decisions requiring Accountable Owner signoff before ADR-017 authoring starts

These are the product decisions that must be locked before ADR-017 can be authored, not implementation details the ADR itself will resolve:

1. **V2 transport scope**: network-only (HTTP, WebSocket, MQTT, REST) vs network + local-file-watch. Recommendation: include local-file-watch since it's cheap and addresses the "operator has their own ETL pipeline" use case.
2. **Operator BYO-key UX**: OS secrets store only (Keychain / Credential Manager / Secret Service) vs Chromaticity-encrypted vault. Recommendation: OS secrets store only; avoids inventing our own key-management.
3. **Fixture pack for V2**: ship a curated set of CC0 / public-domain data cassettes so V2 launch demos work offline without any API key. Recommendation: yes, minimum five cassettes (NOAA radar sample, ISS TLE, a USGS earthquake burst, an OSM area, a NOAA space-weather sample).
4. **Plugin architecture**: in-scope V2 or deferred V3+? Recommendation: deferred. Ship a curated provider set in V2. Open the plugin surface when demand proves it out.

## Traceability

- `01-charter.md` Sec.Scope - differentiation features are listed as "post-MVP tracked follow-ons"; this document is the canonical post-MVP follow-on registry for data ingestion.
- `03-product-requirements.md` FR-8 External Integrations, FR-20 Network Surface Hardening.
- `10-performance-latency-budgets.md` Sec.Frame Performance reserved-post-MVP-budget row.
- `13-risk-register.md` `R-28`.
- `22-compatibility-versioning-policy.md` Sec.Data-Pinned Fixtures.
- `26-evidence-artifact-registry.md` (no new rows at MVP; post-ADR-017 the `EV-DATA-INGEST-*` family is added).
- `28-implementation-contract-pack.md` CP-12.
- `29-cleanroom-legal-policy-baseline.md` Sec.External Data Source Licensing Policy.
- `36-reactive-symbol-catalog.md` Sec.1.6.
- `adr/ADR-005-integration-adapter-boundary.md` Sec.Post-MVP Extension Points.
- `adr/ADR-009-activation-and-entitlements.md` Sec.Capability namespace.
- `adr/ADR-016-observability-and-telemetry-posture.md` Sec.Post-MVP data-ingestion interaction.
- `adr/ADR-017-real-time-external-data-ingestion.md` - slot reserved; this document is its incoming-requirements package.
