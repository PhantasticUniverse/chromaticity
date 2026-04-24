# ADR-005: Integration Adapter Boundary (OSC/MIDI/Media IO)

## Status
Accepted (planning baseline)

## Context
Protocol and platform integrations are volatile and failure-prone. Chromaticity needs stable product semantics independent of transport details.

## Decision
Adopt adapter boundaries:
- ProtocolAdapter interfaces for OSC and MIDI.
- MediaInputAdapter interfaces for camera and external texture feeds.
- IntegrationHub translates adapter events into canonical commands/events.
- No protocol-specific logic in core rendering or state stores.

Define compatibility route semantics at the IntegrationHub contract layer.

## Consequences
### Positive
- Isolation of platform/protocol complexity.
- Easier testing with adapter fakes.
- Lower blast radius for integration failures.

### Negative
- Requires careful translation layer design.

## Alternatives Considered
- Embed protocol handling directly in control/state components (rejected: brittle, hard to test).

## Post-MVP Extension Points (non-normative; track through ADR-017)

The MVP boundary is shaped for push-style, message-sized transports (OSC, MIDI, NDI discovery, camera frames). The post-MVP real-time external-data differentiator (`41-data-ingestion-differentiator.md`, reserved ADR-017) will exercise three axes the current boundary does not fully cover. Acknowledging them here prevents locking an interface that excludes them.

1. **Cadence model**: the current interface assumes adapters push events; `DataAdapter` implementations will be a mix of push (WebSocket, MQTT) and pull (HTTP polling, tile fetches). The IntegrationHub contract will need a pull-cadence descriptor per adapter (declared requested-Hz, bounded by capability and rate limit) distinct from the push-style event model.
2. **Payload size and transfer**: OSC/MIDI payloads are tens of bytes; data adapters will transfer KB-to-MB payloads (GeoJSON features, GRIB2 grids, image tiles). The boundary interface must accommodate zero-copy or arena-backed delivery to the render thread without per-frame allocation churn.
3. **Thread and backpressure model**: `adr/ADR-006-thread-and-process-model.md` allocates a single IO thread. Data adapters need a thread pool (independent of size-bounded IO thread work) and explicit backpressure policy so a slow remote tile server cannot delay the frame scheduler or audio snapshot publication. ADR-006 will require a companion section when ADR-017 is authored.

No MVP code paths are changed by this note; it is a reminder to the author of ADR-017 that the boundary interface must grow along these three axes, not be retrofitted with special cases per data adapter.

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The adapter-boundary contract this ADR establishes is the architectural precondition for the integration-conformance evidence set; without the `ProtocolAdapter` / `MediaInputAdapter` / `IntegrationHub` separation the rows below could not be validated independently of any one transport.

- `EV-INTEGRATION-MATRIX` — adapter conformance matrix is the direct gate-runner for this boundary at Phase 4a; the matrix's per-adapter rows correspond one-for-one to the adapters declared here.
- `EV-NET-HARDENING` — OSC bind defaults, allowlist enforcement, and rate limits are properties of the `ProtocolAdapter` boundary; this ADR's "no protocol-specific logic in core rendering or state stores" decision forces the hardening surface to live at the adapter, where the conformance suite measures it.
- `EV-MIDI-EXTENDED` — extended MIDI surface (pitch-bend, program change, MIDI Clock) is implemented inside the MIDI `ProtocolAdapter` per this boundary; the conformance suite asserts the extended surface is exposed via the adapter contract, not via direct integration into core control state.
- `EV-PLATFORM-CONFORMANCE` — cross-platform adapter portability is the sole property the Phase-5 cumulative gate measures against this ADR; per-platform adapter implementations all satisfy the same `IntegrationHub` contract.
- `EV-HOTKEY-PARITY-CONFORMANCE`, `EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT` — hotkey routing flows through the `ProtocolAdapter` -> `IntegrationHub` event translation layer this ADR defines; the conformance rows assert the canonical-event semantics the boundary produces.
- `EV-DATA-INGEST-SCHEMA`, `EV-DATA-INGEST-PROVIDER-REGISTRY`, `EV-DATA-INGEST-CASSETTE`, `EV-DATA-INGEST-FRAME-BUDGET`, `EV-DATA-INGEST-BACKPRESSURE`, `EV-DATA-INGEST-OFFLINE`, `EV-DATA-INGEST-OBSERVABILITY`, `EV-DATA-INGEST-LICENSE`, `EV-DATA-INGEST-CAPABILITY-GATING`, `EV-DATA-INGEST-KEY-STORAGE` — post-MVP data-ingestion family. These rows are blocked on `ADR-017-real-time-external-data-ingestion.md` activation, but the Sec.Post-MVP Extension Points note in this ADR is what keeps the boundary interface extensible along the three axes (cadence, payload size, thread + backpressure) those rows will measure. A regression here that locked the boundary to push-style message-sized transports would cascade into all ten rows.

## Traceability

- `adr/ADR-006-thread-and-process-model.md` - owns thread allocation; post-MVP data-adapter thread pool is a co-change.
- `28-implementation-contract-pack.md` CP-06 (integration route matrix baseline) and CP-12 (reserved data sources contract).
- `41-data-ingestion-differentiator.md` - reservations that depend on this boundary remaining extensible.
- `adr/ADR-017-real-time-external-data-ingestion.md` - design slot reserved.
