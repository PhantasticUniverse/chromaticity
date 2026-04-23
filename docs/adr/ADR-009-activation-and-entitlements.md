# ADR-009: Activation and Entitlements

## Status
Accepted

Revision history: this ADR supersedes an earlier edition-tiered draft that lived in the same numeric slot during planning. The accepted version locks a capability-bitmask substrate, the forward-compat `entitlement-v2` token format, and an explicit list of unconditionally-free features. The earlier draft is not retained elsewhere and is referenced only here for audit history.

## Context
MVP ships a single edition (Core) per `01-charter.md` and `30-mvp-support-matrix.md`. Post-MVP editions are designed here for forward compatibility but do not drive MVP evidence. Even at MVP, Chromaticity needs a minimal enforcement substrate: watermark-on when no valid token is present, watermark-off when any valid token is present. This is the floor the runtime must ship on day one.

The reference Synesthesia app encodes entitlements as a **per-feature capability set** (observed feature strings in `SynesthesiaEngine.framework`: `*Built-In Scene* access`, `*IDE* access`, `*NDI* access`, `*OSC input* access`, `*OSC output* access`, `*Syphon/Spout input* access`, `*Syphon/Spout output* access`, `*custom resolution* access`, `*no transitions* access`, `*pause* access`, `*scene editing* access`, `*scene importing* access`, plus a `TRIAL_VERSION_CODE` path and `SERVER_*_SYNC_LICENSE`/`SESSION_INVALIDATE` sync protocol strings). An edition-tiered data model cannot express this without implicit feature bundling and leaks into product decisions (pricing, SKU packaging) that belong in commercial layers, not runtime. Chromaticity therefore locks a **capability-bitmask substrate**: runtime evaluates per-capability flags; editions are curated named presets over the bitmask, owned by product, not by this ADR.

The project charter scopes out "commercial/licensing model design" (pricing, SKUs, commerce backend). MVP token distribution does not require a commerce backend: tokens for closed beta are issued statically, and internal/developer tokens are owner-signed against the embedded public key. Post-MVP paid SKUs require a commerce backend; this is a deliberately deferred work package (risk `R-18`).

## Decision

### Entitlement representation (capability-bitmask substrate)
- Runtime consumes a signed JSON **entitlement token** at startup and on refresh events.
- The runtime maintains a single in-memory `EntitlementSet`: a monotonic bitmask keyed by the capability namespace below. Every entitlement query resolves to a `bool` lookup against this set. There is **no runtime concept of "edition"**; editions are marketing-only labels.
- Token schema (`entitlement-v2`, forward-compat superset of the abandoned `entitlement-v1` draft):
  ```json
  {
    "schema": "entitlement-v2",
    "subject": "<opaque-user-or-site-id>",
    "issued_at": "2026-04-22T14:30:00Z",
    "not_before": "2026-04-22T14:30:00Z",
    "expires_at": "2027-04-22T14:30:00Z",
    "issuer": "chromaticity-release",
    "kid": "chromaticity-release-2026-04",
    "capabilities": ["render.watermark.off", "io.ndi.input", "io.ndi.output"],
    "flags": { "trial": false, "beta": false, "enterprise": false },
    "machine_binding": { "algorithm": "blake3-hmac", "digest": "..." },
    "session": { "id": "opaque-session-id", "revocable": true },
    "signature": { "algorithm": "ed25519", "value": "..." }
  }
  ```
- Signature is Ed25519 over the canonical CBOR encoding of the payload excluding `signature`. `kid` identifies which embedded public key signed it; key rotation is supported without runtime rebuilds (see Sec.Key Rotation).
- All claims other than `schema`, `subject`, `issued_at`, `capabilities`, and `signature` are optional. Unknown claim keys are forward-compat-ignored (not rejected), so post-MVP additions (`cloud.sync`, `device.count`, metered quotas) ship as data.

### Capability namespace (initial floor)
Capabilities are strings in a dotted namespace. The MVP runtime evaluates only those below; additions require a bumped capability catalog version and an entry in `22-compatibility-versioning-policy.md`.

**Watermark and distribution**
- `render.watermark.off` - removes the watermark overlay. Any non-empty, valid entitlement token implies this in MVP; post-MVP paid SKUs require it explicitly.

**I/O adapters (reference-parity capability split)**
- `io.osc.input`, `io.osc.output` - independently licensable, matching the reference's split.
- `io.syphon.input`, `io.syphon.output` (macOS) and `io.spout.input`, `io.spout.output` (Windows) - independently licensable; platform-gated.
- `io.ndi.input`, `io.ndi.output` - independently licensable.
- `io.camera` - camera ingestion capability (free; listed so post-MVP gating remains a data change, not a code change).

**Authoring**
- `authoring.ide` - in-app authoring environment (reference gates this as `*IDE* access`).
- `authoring.import` - Shadertoy/ISF import (reference gates this as `*scene importing* access`).
- `authoring.edit` - scene editing (reference gates as `*scene editing* access`).
- `authoring.builtin-scenes` - access to bundled built-in scenes (reference gates as `*Built-In Scene* access`).

**Rendering**
- `render.custom-resolution` - operator-settable render resolution (reference gates this).

**Meta and audio**
- `meta.reactivity`, `meta.audio-speed` - Differentiation-profile meta controls (`36-reactive-symbol-catalog.md` Sec.3.6).

**External data ingestion (reserved; activated post-MVP)**
- `data.ingest.network` - network-transported external data sources (weather grids, orbital elements, geospatial tiles, event streams). Reserved for V2 per `41-data-ingestion-differentiator.md`; MVP does not activate this capability.
- `data.ingest.geospatial` - geospatial basemap/tile providers specifically, split from generic network because per-tile commercial licensing differs materially. Reserved for V2.
- `data.ingest.sdr` - software-defined-radio / USB-device-backed RF ingestion. Reserved for V3 per the scope decision in `41-data-ingestion-differentiator.md` Sec.Scope.

At MVP the runtime parses these capability strings when present in a token but evaluates every capability query against them as `false` (the data-ingest subsystem is not yet wired). This preserves forward token-format compatibility so V2/V3 tokens issued to pre-existing installs do not break the bitmask evaluator.

### Explicitly free (not gated, ever)
The reference gates the following as paid capabilities. Chromaticity makes a different product decision and ships all of them **unconditionally available in every edition, including when no token is present**:

- **Pause.** Basic transport control; gating it is hostile-UX.
- **Transitions-off (hard-cut scene switches).** Experienced operators require the ability to disable transitions for tight cue-stack workflows; putting this behind a paywall is a known reference pain point.
- **Playlist navigation** (next/previous/select).
- **Preset recall and Snap slots** (the Chromaticity UX term for per-scene quick-recall preset slots; retained as `favslot` only in the `compat-syn-v1` OSC route surface).
- **Scene export** (for backup or hand-off); scene authoring creation is `authoring.edit`, but exporting one's own scene is always free.
- **OSC/MIDI route discovery** messages (not send/receive of control values, which remain capability-gated).

These appear as capabilities in the namespace **only** as explicit `free.*` placeholders if a future SKU decision requires them; no such placeholders exist at MVP.

### Token lifetimes and trial mode
- **Paid lifetime tokens**: default `expires_at = issued_at + 1 year`; runtime refreshes monthly when online; `expires_at` is the hard ceiling.
- **Trial tokens**: carry `flags.trial = true`; `expires_at - issued_at <= 14 days`; `machine_binding` is **required**; `not_before` must equal `issued_at` (no deferred-start trials). Post-MVP only; MVP does not issue trial tokens but the runtime accepts the schema.
- **Beta tokens**: `flags.beta = true`; may carry longer expiries; surfaced in the support bundle.
- **Enterprise tokens**: `flags.enterprise = true`; may waive `machine_binding`; site-licensed.

### Offline posture
- Default: entitlement token is read from local storage at startup. No network required.
- Refresh happens over HTTPS when available; failure to refresh does not revoke a currently-valid token until `expires_at`. Hard-offline 14-day operation is an `EV-ENTITLEMENT-OFFLINE` evidence row.
- If no token is present, the runtime runs in Core-equivalent (no `render.watermark.off`) with a watermark overlay; all free capabilities remain unconditionally available.

### Revocation
- Short-expiry tokens (paid <= 1 year, trial <= 14 days) are the primary revocation vector.
- A signed **deny list** (`SERVER_OUT_FLUSH_LICENSE` equivalent) can be fetched on refresh; deny entries remove specific `subject` or `session.id` entries regardless of token expiry. Deny-list items carry their own expiry to bound local storage growth.
- **Session invalidation** (`SESSION_INVALIDATE` equivalent): when a token carries a `session.id`, the server can invalidate that session on refresh without revoking the subject.
- Runtime never hard-fails on revocation latency: if the deny list can't be fetched, the current valid token continues to honor its local expiry.

### Key rotation
- Multiple public keys may be embedded per release; the `kid` field selects which one verifies.
- Key rotation cadence: annual + on-incident. Rotation is documented in the security runbook.
- Rotation procedure: (1) new key embedded in release N, used-for-signing in release N+1; (2) both keys accepted during N and N+1; (3) old key removed in release N+2.

### Enforcement points
1. **Capability query** is the single entry point: `chromaticity.entitled(capability_string) -> bool`. All adapter code, UI code, and scripts consult this. Scripts run in a sandbox (`adr/ADR-012-script-sandbox-engine.md`) that prevents direct state mutation.
2. **Adapter boundary**: every I/O adapter refuses to start unless its capability is entitled (`adr/ADR-005-integration-adapter-boundary.md`). At MVP this only evaluates `render.watermark.off`; the remaining adapter capabilities are wired post-MVP as a data change (no architecture change).
3. **Watermark overlay** is unconditional when `render.watermark.off` is not entitled; removed when it is.
4. **UI affordances** for gated capabilities render in a discoverable-but-disabled state (not hidden) so operators can see what an upgrade would unlock. This is a deliberate differentiation from the reference, which throws mid-feature license errors.

### Developer and CI entitlements
- Developer tokens are issued by the build system, bound to CI machine hashes, and include `expires_at` <= 30 days.
- CI uses a **test entitlement** profile that grants every capability; builds signed with the test-entitlement keypair are clearly marked in release metadata and fail `EV-SIGNING-INTEGRITY` if they leak into a release channel.

### User flows
- **Activate** - enter a key or sign in; runtime exchanges for a token and stores it.
- **Deactivate (local)** - removes the token from storage; runtime drops to Core-equivalent.
- **Deactivate (remote)** - invalidates a subject's tokens on the next refresh.
- **Transfer** - deactivate on machine A, activate on machine B. Requires network at both endpoints for sync; offline transfer requires a signed handoff token (post-MVP).

## Consequences

### Positive
- **Capability-bitmask substrate removes product decisions from the runtime.** Editions become marketing-only presets; pricing and SKU packaging can change without touching the runtime.
- Edition tiering becomes enforceable as a data change, unblocking NDI / OSC-in / OSC-out / authoring SKUs without re-architecture.
- `entitlement-v2` already carries the claim surface post-MVP backends will need (trial, machine binding, session revocation, key rotation). Day-one paid SKU launch is a **server-side and packaging** change, not a runtime-format migration.
- Explicit free-feature list (`pause`, `transitions-off`, playlist, preset recall, scene export) is a differentiation posture vs the reference and an operator-trust signal.
- Offline-first posture protects live performers (no festival-night network dependency).

### Negative
- Adds a signed-token distribution requirement to the build pipeline even at MVP (must produce and ship at least the embedded public key set + static closed-beta token set).
- Introduces a small ongoing security obligation (key rotation; deny-list hosting is optional at MVP and becomes required when paid SKUs ship).
- Post-MVP paid SKUs require a commerce backend and the license-sync wire protocol (`R-18`).

## Alternatives Considered
- **Edition-tiered data model** (earlier draft of this ADR): rejected. Couples product SKU decisions to runtime; unable to express the reference's per-capability gates (e.g. OSC-in-only) without ad-hoc special cases.
- **Online-only activation**: rejected. Live performers need offline operation.
- **DRM / hardware dongle**: rejected. Out of proportion to product value; bad operator experience.
- **Feature-unlock flag file** (unsigned): rejected. Trivial to forge; undermines Advanced-edition revenue.
- **No enforcement** (honor system): rejected. Matches neither the reference's activation posture nor the product's commercial intent.
- **Replicate reference license-sync protocol** (`SERVER_IN/OUT_SYNC_LICENSE`) at MVP: rejected for MVP scope; the `entitlement-v2` token format is designed to be compatible with such a protocol when it ships post-MVP.

## Evidence
- `EV-ENTITLEMENT-ENFORCEMENT` - validates every capability string gates the correct adapter / UI affordance / script query.
- `EV-ENTITLEMENT-OFFLINE` - runtime survives 14-day network outage with valid token.
- `EV-ENTITLEMENT-REVOCATION` - deny list, subject invalidation, and session invalidation behaviors match this ADR.
- `EV-ENTITLEMENT-TOKEN-FORMAT` - `entitlement-v2` fixture pack covers: paid/trial/beta/enterprise flags; `machine_binding` present/absent; `kid` rotation round-trip; canonical CBOR determinism.
- `EV-ENTITLEMENT-FREE-FEATURES` - asserts `pause`, `transitions-off`, playlist navigation, preset recall, Snap slots, and scene export all function with **no token** installed.

## Traceability
- `01-charter.md` Sec.In Scope (entitlement substrate is in scope; commerce backend is out).
- `30-mvp-support-matrix.md` edition rows (now expressed as capability presets).
- `03-product-requirements.md` FR-8 (capability-gated features) and NFR-6 (sandbox isolation).
- `36-reactive-symbol-catalog.md` Sec.3.6 (capability-gated metas).
- `13-risk-register.md` `R-17` (Entitlement bypass) and `R-18` (commerce-backend slippage).
- `adr/ADR-005-integration-adapter-boundary.md` (adapter gating enforcement).
- `adr/ADR-012-script-sandbox-engine.md` (script cannot bypass capability query).
- `adr/ADR-014-signing-notarization-auto-update.md` (embedded-key provenance and rotation).
