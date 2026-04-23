# ADR-013: Networking Defaults and Security Model

## Status
Accepted

## Context
`08-io-integrations.md` and prior review rounds set up OSC/MIDI/NDI integration contracts and conformance test IDs but did not lock the networking posture: which interface each adapter binds to by default, whether senders must be authenticated, rate limits under attacker load, and discovery scope. OSC is plaintext UDP/TCP with no built-in authentication. NDI broadcasts over local subnet by default. A commercial live-performance product running on venue Wi-Fi is trivially DoS-able and trivially triggerable by any host on the same subnet if this posture is left open. `16-deep-dive-integrations-findings.md` ITF-06 already flagged firewall/port troubleshooting as operator burden; making the default posture conservative is the minimal-change cure.

This ADR locks the defaults and the operator opt-in model.

## Decision

### 1. Bind defaults per adapter
| Adapter | Default bind | Opt-in to expand |
|---|---|---|
| OSC inbound | `127.0.0.1` (loopback only) | operator selects specific interface or `0.0.0.0` in settings |
| OSC outbound | `127.0.0.1` target unless operator sets target IP | operator-configurable per subscriber |
| NDI input | local subnet discovery allowed | operator can restrict discovery to allowlist |
| NDI output | local subnet advertise | operator can set "local-host only" or denylist |
| Syphon / Spout | local-host only (protocol constraint) | n/a |

Rationale: most live operators run Chromaticity locally and only need OSC over the loopback (e.g. TouchOSC bridge, QLab on same machine). Exposing OSC on the network is an opt-in, not the default.

### 2. Source-IP allowlist
- Per-OSC-mapping allowlist field: empty means "accept from bound interface only"; otherwise only senders whose IP is on the allowlist pass.
- Allowlist honors CIDR notation (`192.168.1.0/24`) for subnet trust.
- Violations emit `osc-source-rejected` diagnostic with source IP + route; no side effects on state.

### 3. Rate limits
- Inbound per-adapter rate ceiling: 1000 messages/second hard cap; operator-configurable floor is 100 msg/s default.
- Per-route rate ceiling: 100 msg/s per unique `(adapter, route)` pair default; operator-configurable.
- Over-rate messages are dropped with `osc-rate-limited` diagnostic emitted at a throttled cadence (no more than 1/s per route).
- NDI and Syphon/Spout frame rates follow the output pipeline; no throttle needed because the cap is upstream (the pipeline) not adapter-level.

### 4. Discovery scope
- OSC listeners advertise only on the interface they are explicitly bound to. No broadcast ping or Bonjour/mDNS advertising at MVP.
- NDI discovery defaults to local subnet; operator may restrict to a specific interface or allowlist.
- No service-discovery broadcast on unbound interfaces under any circumstance.

### 5. Payload hardening
- OSC packet max size: 64 KiB; oversize packets rejected with diagnostic.
- OSC nested bundle depth cap: 4 levels; deeper payloads rejected.
- Malformed OSC (invalid type tag, truncated) -> rejected with `osc-malformed` diagnostic, no state mutation (already required by `08-io-integrations.md`; restated here for completeness).
- NDI/Syphon frames are not author-controlled; no payload hardening needed beyond crash-tolerant decode.

### 6. Operator override surface
- Every default above is operator-overridable in a Settings panel that carries an explicit "changing this setting may expose Chromaticity to the network" note.
- Overrides are persisted per show; exporting a show bundle (`FR-12`) includes the override set so venue setups are reproducible.
- A "Network Diagnostics" panel per `16-deep-dive-integrations-findings.md` ITF-06 reflects current bind status, rate-limit counters, recent rejections.

## Consequences

### Positive
- Out-of-the-box Chromaticity does not expose any network attack surface without operator action.
- Rate limits protect the runtime from accidental OSC broadcasts (e.g. a patchbay sending thousands of messages per second).
- Allowlist model scales from "solo laptop" to "multi-machine venue rig" with the same contract.

### Negative
- First-time operators may need to flip OSC from loopback to a network interface when setting up a remote controller; documented as a Phase-0 FAQ item.
- Per-route rate limits can drop messages in rare high-density mappings; operator diagnostic surfaces this.
- Overrides multiply the test matrix for `EV-NET-HARDENING`.

## Alternatives Considered

- **Open-by-default OSC on all interfaces**: rejected. Security and DoS risk outweigh the convenience.
- **TLS/auth for OSC**: rejected. OSC ecosystem does not support it; would break interop with every standard OSC tool. (Considered for post-MVP as a Chromaticity-specific extension.)
- **VPN-only networking mode**: rejected. Out of proportion; operators who need this can use an OS-level VPN.

## Evidence
- `EV-NET-HARDENING`:
 - Bind-default matrix passes per Sec.1.
 - Source-IP allowlist enforces per Sec.2 (positive and negative fixtures).
 - Rate-limit behavior under synthetic attacker load per Sec.3.
 - Malformed/oversize payload rejection per Sec.5.
 - Operator-override round-trip (export/import show bundle preserves settings).

## Traceability
- `03-product-requirements.md` FR-20
- `08-io-integrations.md` (security and route sections reference this ADR)
- `16-deep-dive-integrations-findings.md` ITF-06
- `13-risk-register.md` (R-05, R-11, R-12 already cover adapter risks; network-surface risk is now mitigated by the defaults locked here)
