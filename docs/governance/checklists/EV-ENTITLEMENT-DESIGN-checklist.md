<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# EV-ENTITLEMENT-DESIGN — Phase 0 Exit Checklist

**Evidence ID:** `EV-ENTITLEMENT-DESIGN`
**Gate role:** Phase 0 exit gate (`docs/26-evidence-artifact-registry.md` row
`EV-ENTITLEMENT-DESIGN`).
**Purpose:** Validate the formal design-review packet for ADR-009 (Activation and
Entitlements) — both per-section ADR sign-off and a threat-model addendum that
goes beyond the ADR itself.
**Checklist date:** 2026-04-29.
**Accountable Owner:** Xavier Butler (`@PhantasticUniverse`).

## Scope

This checklist instantiates the formal design-review packet the registry row
called for as the remaining `InProgress` work item. ADR-009
(`docs/adr/ADR-009-activation-and-entitlements.md`) is `Accepted` and merged on
`main`; the registry row's pending work was a packet "with threat-model sign-off
artifact beyond the ADR itself."

This checklist is structured in two parts: (1) per-section ADR sign-off covering
each design area in ADR-009, and (2) a threat-model addendum enumerating the
named attack surfaces with mitigation citations into ADR-009 and adjacent ADRs.

ADR-009 was imported into `docs/adr/` as part of the single normative-corpus
promotion in commit `deefc7a` (2026-04-23, `docs(policy): promote normative
planning corpus into docs/ + docs/adr/`). That commit is the merge-evidence
anchor for every per-section row below.

## Part 1 — Per-section ADR-009 sign-off

- [x] **Capability namespace (`docs/adr/ADR-009` Sec.Decision —
      Capability namespace).**
      Scope: dotted-namespace capability strings; MVP-evaluated capabilities
      (watermark and distribution: `render.watermark.off`; I/O adapters:
      `io.osc.input` / `io.osc.output` / `io.syphon.input` / `io.syphon.output` /
      `io.spout.input` / `io.spout.output` / `io.ndi.input` / `io.ndi.output` /
      `io.camera`; authoring: `authoring.ide` / `authoring.import` /
      `authoring.edit` / `authoring.builtin-scenes`; rendering:
      `render.custom-resolution`; meta and audio: `meta.reactivity` /
      `meta.audio-speed`); reserved post-MVP namespace (`data.ingest.network` /
      `data.ingest.geospatial` / `data.ingest.sdr`).
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Token schema (`docs/adr/ADR-009` Sec.Decision —
      Entitlement representation, schema `entitlement-v2`).**
      Scope: forward-compat superset of `entitlement-v1`; required claims
      (`schema`, `subject`, `issued_at`, `capabilities`, `signature`); optional
      claims (`not_before`, `expires_at`, `issuer`, `kid`, `flags`,
      `machine_binding`, `session`); Ed25519 signature over canonical CBOR
      encoding excluding the `signature` field; unknown claim keys
      forward-compat-ignored.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Post-MVP edition model (`docs/adr/ADR-009` Sec.Decision —
      Entitlement representation; Sec.Capability namespace; Sec.Alternatives
      Considered).**
      Scope: capability-bitmask substrate is the runtime data model; there is
      no runtime concept of "edition" — editions are marketing-only labels
      that curate named presets over the bitmask; pricing and SKU packaging
      can change without touching the runtime; the explicit-free list (pause,
      transitions-off, playlist navigation, preset recall and Snap slots,
      scene export, OSC/MIDI route discovery) is unconditionally available in
      every edition, including with no token present; rejection of the
      earlier edition-tiered draft is recorded under Alternatives Considered.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Signing and rotation (`docs/adr/ADR-009` Sec.Decision —
      Key rotation; cross-reference `docs/adr/ADR-014`).**
      Scope: multiple public keys may be embedded per release; `kid` field
      selects the verifier; rotation cadence annual + on-incident; rotation
      procedure (release N adds new key, release N+1 uses it for signing,
      both keys accepted in N and N+1, old key removed in N+2); embedded-key
      provenance tied to the signing/notarization pipeline in ADR-014.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Offline window (`docs/adr/ADR-009` Sec.Decision —
      Offline posture; cross-reference `EV-ENTITLEMENT-OFFLINE`).**
      Scope: token read from local storage at startup, no network required;
      refresh over HTTPS when available; failure to refresh does not revoke a
      currently-valid token until `expires_at`; hard-offline 14-day operation
      is an evidence row (`EV-ENTITLEMENT-OFFLINE`); if no token present, runtime
      runs in Core-equivalent (no `render.watermark.off`) with watermark
      overlay; all free capabilities remain unconditionally available.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Watermark policy (`docs/adr/ADR-009` Sec.Decision —
      Enforcement points; Sec.Decision — Capability namespace —
      `render.watermark.off`).**
      Scope: watermark overlay is unconditional when `render.watermark.off`
      is not entitled; removed when it is; any non-empty valid entitlement
      token implies `render.watermark.off` at MVP (post-MVP paid SKUs require
      it explicitly); UI affordances for gated capabilities render in a
      discoverable-but-disabled state, not hidden — a deliberate
      differentiation from the reference's mid-feature license errors.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

## Part 2 — Threat-model addendum

This addendum enumerates the named attack surfaces against the entitlement
substrate, with mitigation summary and the ADR section that addresses each.
It is the "threat-model sign-off artifact beyond the ADR itself" the registry
row called for.

- [x] **Token forgery.**
      *Surface*: an attacker fabricates a valid-looking `entitlement-v2`
      token to obtain capabilities (e.g. `render.watermark.off`).
      *Mitigation*: signature verification is Ed25519 over canonical CBOR of
      the payload excluding `signature`; only tokens whose `signature.value`
      verifies against an embedded public key (selected by `kid`) are
      accepted. Forgery requires possession of the private signing key,
      which is held only by the build pipeline and rotated per the key-
      rotation procedure.
      *ADR cite*: `docs/adr/ADR-009` Sec.Decision — Entitlement representation
      (token schema and Ed25519-over-CBOR signature); Sec.Decision — Key
      rotation. Cross-references signing-pipeline integrity in
      `docs/adr/ADR-014` Sec.Signing/notarization. Adversarial-fabrication
      coverage flows into `EV-ENTITLEMENT-TOKEN-FORMAT` at Phase-0 (already
      `Planned`) and `EV-SIGNING-INTEGRITY` at Phase-5.

- [x] **Machine-binding bypass.**
      *Surface*: a single token issued for one machine is shared and used on
      multiple machines, defeating per-seat licensing.
      *Mitigation*: `machine_binding` field carries an algorithm identifier
      (`blake3-hmac` baseline) and a digest derived from machine identity;
      runtime computes the local digest at startup and rejects tokens whose
      `machine_binding.digest` does not match; trial tokens require
      `machine_binding` (no shareable trial path); enterprise tokens may
      waive it as an explicit site-license posture. Sandbox isolation in
      `docs/adr/ADR-012` prevents script code from reading or rewriting the
      machine-identity inputs.
      *ADR cite*: `docs/adr/ADR-009` Sec.Decision — Entitlement representation
      (`machine_binding.algorithm` / `.digest`); Sec.Decision — Token
      lifetimes and trial mode (machine_binding required for trial; waivable
      for enterprise); cross-reference `docs/adr/ADR-012`
      Sec.Sandbox isolation. Coverage flows into
      `EV-ENTITLEMENT-TOKEN-FORMAT` at Phase-0 and
      `EV-ENTITLEMENT-ENFORCEMENT` at Phase-4a.

- [x] **Key rotation hijack.**
      *Surface*: an attacker forces the runtime to verify a token against a
      key the attacker controls (e.g. by manipulating `kid` selection or
      injecting a key into the embedded set).
      *Mitigation*: embedded keys are part of the signed application bundle;
      tampering with the embedded key set invalidates the macOS Developer
      ID / hardened-runtime / notarization signature and the Windows
      Authenticode EV signature, both of which are runtime-checked at launch
      per `docs/adr/ADR-014`. `kid` selects only among keys already in the
      signed bundle; an attacker-controlled `kid` value resolves to no key
      and the token is rejected. Rotation procedure (release N adds, N+1
      signs-with, N+2 removes) bounds the validity window for any compromised
      key.
      *ADR cite*: `docs/adr/ADR-009` Sec.Decision — Key rotation; cross-
      reference `docs/adr/ADR-014` Sec.Signing/notarization (bundle integrity)
      and Sec.7 (bundle hygiene). Coverage flows into `EV-SIGNING-INTEGRITY`
      and `EV-BUNDLE-HYGIENE` at Phase-5.

- [x] **Offline fallback abuse.**
      *Surface*: an attacker keeps a revoked or expired token alive by
      keeping the runtime offline indefinitely, defeating revocation and
      expiry.
      *Mitigation*: offline operation is bounded by `expires_at`; even
      offline, the runtime treats `expires_at` as the hard ceiling. Hard-
      offline operation past the 14-day evidence-row window
      (`EV-ENTITLEMENT-OFFLINE`) is supported only for tokens whose
      `expires_at` extends past that window. Trial tokens cap
      `expires_at - issued_at <= 14 days` so the offline-abuse window for a
      trial is the same as the trial duration. Paid lifetime tokens default
      to one-year expiry with monthly online refresh when available.
      *ADR cite*: `docs/adr/ADR-009` Sec.Decision — Offline posture (the
      14-day window the row name calls out); Sec.Decision — Token lifetimes
      and trial mode; Sec.Decision — Revocation. Coverage flows into
      `EV-ENTITLEMENT-OFFLINE` and `EV-ENTITLEMENT-REVOCATION` at Phase-4a.

- [x] **Watermark removal attempts.**
      *Surface*: an attacker patches the rendered output, the watermark
      overlay code, or the entitlement-query path to remove the watermark
      without a valid token.
      *Mitigation*: capability query (`chromaticity.entitled(capability_string)
      -> bool`) is the single entry point; adapter boundary, UI, and script
      paths all consult it; scripts run sandboxed per `docs/adr/ADR-012` and
      cannot mutate runtime state (including the `EntitlementSet`); watermark
      overlay is unconditional when `render.watermark.off` is not entitled.
      Binary-patching the watermark overlay code or the entitlement-query
      path tampers with the signed bundle and invalidates the macOS / Windows
      release signature per `docs/adr/ADR-014`. The bundle-hygiene row
      (`EV-BUNDLE-HYGIENE`) and signing-integrity row
      (`EV-SIGNING-INTEGRITY`) catch shipped artifacts that ship without the
      enforced overlay; runtime self-tamper detection is out of scope for
      MVP (DRM was rejected in Sec.Alternatives Considered).
      *ADR cite*: `docs/adr/ADR-009` Sec.Decision — Enforcement points (#1
      capability query, #3 watermark overlay); cross-reference
      `docs/adr/ADR-012` Sec.Sandbox isolation; cross-reference
      `docs/adr/ADR-014` Sec.7 (bundle hygiene), `EV-BUNDLE-HYGIENE`,
      `EV-SIGNING-INTEGRITY`. Coverage flows into `EV-ENTITLEMENT-ENFORCEMENT`
      at Phase-4a.

- [x] **MITM during entitlement fetch.**
      *Surface*: an attacker intercepts the entitlement-refresh HTTPS
      request and substitutes a token (e.g. a long-expiry token from
      another subject) or substitutes a deny-list payload that grants
      capabilities to a deny-listed subject.
      *Mitigation*: refresh occurs over HTTPS; the substituted token must
      still verify against an embedded public key, which the attacker does
      not control (token forgery mitigation applies); the substituted deny
      list is itself signed and verified before being applied; neither
      surface accepts unsigned payloads. The runtime never hard-fails on
      revocation latency: if the deny list cannot be fetched (or fails
      verification), the current valid token continues to honor its local
      `expires_at`. Endpoint is on a Chromaticity-controlled domain only,
      pinned per `docs/adr/ADR-016` Sec.Outbound posture, and is the only
      outbound network endpoint at MVP that touches the entitlement
      subsystem.
      *ADR cite*: `docs/adr/ADR-009` Sec.Decision — Offline posture
      (HTTPS refresh; failure-tolerant); Sec.Decision — Revocation (signed
      deny list; revocation-latency tolerance); cross-reference
      `docs/adr/ADR-013` Sec.Networking defaults (HTTPS-only outbound);
      cross-reference `docs/adr/ADR-016` Sec.Outbound posture
      (Chromaticity-domain-only). Coverage flows into
      `EV-NET-HARDENING` at Phase-4a and `EV-OBSERVABILITY-INVENTORY` at
      Phase-0.

## Overall sign-off

- [x] All six ADR-009 design areas are reviewed and adopted; the threat-
      model addendum enumerates the six named attack surfaces with mitigation
      citations into ADR-009 and the adjacent ADRs (ADR-012, ADR-013,
      ADR-014, ADR-016). The `EV-ENTITLEMENT-DESIGN` Phase 0 exit gate is
      closed. The downstream entitlement-runtime evidence rows
      (`EV-ENTITLEMENT-ENFORCEMENT`, `EV-ENTITLEMENT-OFFLINE`,
      `EV-ENTITLEMENT-REVOCATION`, `EV-ENTITLEMENT-TOKEN-FORMAT`,
      `EV-ENTITLEMENT-FREE-FEATURES`) remain on their planned phase
      schedules and are not gated by this row's closure.
      Signed by AO `@PhantasticUniverse` on 2026-04-29.
