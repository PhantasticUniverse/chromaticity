# Requirements Traceability Matrix

This matrix links requirements to ownership, validation evidence, and risk coverage.

## Legend
- Evidence IDs refer to artifacts produced by CI/test harness, benchmark runs, or review checklists.
- Risk IDs refer to `13-risk-register.md`.

| Requirement | Profile | Owner | Validation Method | Evidence Artifact | Phase Gate | Risk Coverage |
|---|---|---|---|---|---|---|
| FR-1 Scene Package Support | Parity-Core | AI Scene Workcell | Integration tests + corpus import checks | `EV-SCENE-IMPORT` | Phase 1 exit | R-01, R-13 |
| FR-2 Schema and Validation | Parity-Core | AI Scene Workcell | Schema/unit tests + lint snapshots | `EV-SCHEMA-LINT` | Phase 1 exit | R-01, R-13 |
| FR-3 Rendering Runtime | Parity-Core | AI Runtime Workcell | Multipass conformance tests + perf bench | `EV-RENDER-CONFORMANCE` | Phase 2 exit | R-03 |
| FR-4 Script Runtime | Parity-Core | AI Runtime Workcell | Script API contract tests + fault injection | `EV-SCRIPT-CONTRACT` | Phase 2 exit | R-02 |
| FR-5 Audio Reactivity | Parity-Core | AI Audio Workcell | Audio fixture trace comparisons | `EV-AUDIO-TRACE` | Phase 2 exit (initial) + Phase 3 exit (full-corpus revalidation) | R-04 |
| FR-6 Controls and State | Parity-Core | AI Scene Workcell | Reducer replay tests + integration checks | `EV-STATE-REPLAY` | Phase 3 exit | R-06 |
| FR-7 Presets and Playlists | Parity-Core | AI Scene Workcell | Preset/playlist scenario tests + migration/rollback conformance | `EV-PRESET-PLAYLIST`, `EV-STATE-MIGRATION` | Phase 3 exit | R-10 |
| FR-8 External Integrations | Parity-Advanced | AI Integration Workcell | Adapter conformance + route matrix tests | `EV-INTEGRATION-MATRIX` | Phase 4a exit | R-05, R-11, R-12 |
| FR-9 Developer Experience | Differentiation | AI QA/Ops Workcell | Diagnostic message quality checks | `EV-DIAGNOSTIC-QUALITY` | Phase 3-5 quality gate (cumulative) | R-08 |
| FR-10 Clean-Room and Commercial Compliance | Parity-Core | Accountable Owner | Clean-room policy baseline audit + release legal checklist | `EV-CLEANROOM-POLICY-BASELINE` | Phase 0 exit | R-14, R-15, R-16 |
| FR-11 Audio Routing and System Audio | Parity-Core | AI Audio Workcell, AI Integration Workcell | Device/channel routing conformance + system-audio ingestion tests | `EV-AUDIO-ROUTING-MATRIX` | Phase 3 exit | R-04, R-05, R-12 |
| FR-12 Show-State Safety | Parity-Core | AI Scene Workcell | Signed show bundle import/export tests + rollback recovery drills | `EV-SHOW-BUNDLE-RECOVERY` | Phase 3 exit | R-06, R-10 |
| FR-13 Scene Authoring Environment | Parity-Core | AI Scene Workcell + AI QA/Ops Workcell | Editor/live-reload harness + diagnostic mapping checks | `EV-AUTHORING-LIVE-RELOAD`, `EV-AUTHORING-DIAGNOSTICS`, `EV-AUTHORING-SCAFFOLDER` | Phase 4b exit | R-02, R-13 |
| FR-14 Scene Import Adapters | Parity-Core | AI Scene Workcell | Shadertoy fixture conversion + ISF reject-path tests | `EV-IMPORT-SHADERTOY`, `EV-IMPORT-ISF-REJECT` | Phase 4b exit | R-01, R-13, R-15 |
| FR-15 Scene Library | Parity-Core | AI Scene Workcell | Library index/scan tests + thumbnail fallback determinism tests | `EV-LIBRARY-DISCOVERY`, `EV-LIBRARY-THUMBNAIL` | Phase 4b exit | R-13 |
| FR-16 Embedded Documentation | Parity-Core | Accountable Owner + AI QA/Ops Workcell | Docs provenance audit + version-sync and coverage CI gates | `EV-DOCS-CLEANROOM`, `EV-DOCS-VERSION-SYNC`, `EV-DOCS-COVERAGE` | Phase 4c exit | R-14, R-15, R-16 |
| FR-17 Operator Hotkey Map | Parity-Core | AI Integration Workcell | Hotkey binding conformance tests | `EV-HOTKEY-PARITY-DESIGN` (Phase 0 ratification), `EV-HOTKEY-PARITY-CONFORMANCE` / `EV-HOTKEY-COMPLETENESS` / `EV-HOTKEY-CONFLICT` (Phase 4a conformance) | Phase 0 exit (design) + Phase 4a exit (conformance) | R-11 |
| FR-18 Activation and Entitlement | Parity-Core | Accountable Owner + AI Integration Workcell | Signed `entitlement-v2` token verification tests + offline runtime drill + revocation drill + free-feature conformance drill (MVP: watermark scope) | `EV-ENTITLEMENT-ENFORCEMENT`, `EV-ENTITLEMENT-OFFLINE`, `EV-ENTITLEMENT-REVOCATION`, `EV-ENTITLEMENT-TOKEN-FORMAT`, `EV-ENTITLEMENT-FREE-FEATURES` | Phase 4a exit | R-05, R-15, R-17, R-18 |
| FR-19 Reference-App User Data Import | Parity-Core | AI Scene Workcell + AI Integration Workcell | Dry-run + commit + rollback drill across preset/playlist/mapping/hotkey artifact classes | `EV-IMPORT-REFERENCE-SHOW` | Phase 4b exit | R-01, R-13 |
| FR-20 Network Surface Hardening | Parity-Core | AI Integration Workcell | Bind-default, allowlist, rate-limit, payload-hardening conformance | `EV-NET-HARDENING` | Phase 4a exit | R-05, R-11, R-12 |
| FR-21 Crash Recovery and Show Resume | Parity-Core | AI QA/Ops Workcell + AI Scene Workcell | Crash-injection + resume drill with per-class state-restoration acceptance | `EV-CRASH-RESUME` | Phase 4d exit | R-07, R-08 |
| FR-22 Extended MIDI Surface | Parity-Core | AI Integration Workcell + AI Audio Workcell | Extended MIDI conformance (pitch bend, program change, clock sync, 14-bit CC, NRPN/RPN, MPE, MTC, learn-persistence) | `EV-MIDI-EXTENDED` | Phase 4a exit | R-04, R-05, R-11 |
| NFR-1 Reliability | Parity-Core | AI QA/Ops Workcell | Soak + chaos/fault injection | `EV-SOAK-CHAOS` | Phase 4d exit | R-07, R-12 |
| NFR-2 Determinism | Parity-Core | AI Runtime Workcell | Replay determinism suite | `EV-DETERMINISM` | Phase 1 exit (baseline) + release cumulative | R-02, R-06 |
| NFR-3 Performance | Parity-Core | AI QA/Ops Workcell | Budget benchmarks (p50/p95/p99) | `EV-PERF-BUDGETS` (Phase 3 exit), `EV-SHADER-CACHE` (Phase 4d exit) | Phase 3 exit + Phase 4d exit | R-03, R-04 |
| NFR-4 Observability | Parity-Core | AI QA/Ops Workcell | Telemetry schema and dashboard checks | `EV-OBSERVABILITY` | Phase 1 exit + release cumulative | R-08 |
| NFR-5 Portability | Parity-Advanced | AI Integration Workcell | Cross-platform conformance tests | `EV-PLATFORM-CONFORMANCE` | Release cumulative gate | R-05 |
| NFR-6 Security/Isolation | Parity-Core | AI Runtime Workcell | Sandbox escape + API allowlist tests | `EV-SANDBOX-SAFETY` | Phase 2 exit | R-02 |
| NFR-7 Legal and Licensing Hygiene | Parity-Core | Accountable Owner | License scan + legal review | `EV-LICENSE-AUDIT` | Release legal audit gate (Phase 5) | R-15, R-16 |
| NFR-8 Supportability and Privacy | Parity-Core | AI QA/Ops Workcell | Supportability policy audit + privacy/runbook checklist | `EV-SUPPORT-OPS-READINESS` | Phase 5 exit + release cumulative | R-08 |
| NFR-8a Observability Posture | Parity-Core | AI QA/Ops Workcell + Accountable Owner | Outbound-traffic audit, inventory check, crash-payload redaction drill, ATS-exception grep | `EV-OBSERVABILITY-INVENTORY`, `EV-ANALYTICS-ZERO-DEFAULT`, `EV-HEALTH-METRICS-LOOPBACK-ONLY`, `EV-CRASH-PAYLOAD-REDACTION`, `EV-ATS-NO-THIRD-PARTY-EXCEPTIONS` | Phase 0 exit (inventory) + Phase 4d exit (conformance) | R-25 |
| NFR-9 Network Surface | Parity-Core | AI Integration Workcell | Network-hardening conformance | `EV-NET-HARDENING` | Phase 4a exit | R-05, R-11, R-12 |
| NFR-10 Authoring Responsiveness | Parity-Core | AI Scene Workcell + AI Docs Workcell + AI QA/Ops Workcell | Latency harness for hover/search in editor and embedded docs | `EV-AUTHORING-UX-LATENCY`, `EV-DOCS-UX-LATENCY` | Phase 4b exit (authoring) + Phase 4c exit (docs) | R-13, R-20 |
| NFR-11 Accessibility Floor | Parity-Core | AI UX Workcell + AI Scene Workcell | axe-core automated SC 2.2 AA audit + VoiceOver/NVDA manual audit + reduced-motion conformance + 24x24 target-size check + 3:1 focus-indicator contrast audit on dark and light backgrounds. Scene canvas is explicitly out of scope. | `EV-ACCESSIBILITY-AA` | Phase 4b exit | R-29 |
| NFR-12 Color Management | Parity-Core | AI Runtime Workcell | V1 sRGB conformance: non-scene UI rendered in sRGB, OS display-profile respected, ColorSync (macOS) / ICC (Windows) round-trip of scene output under documented profiles. HDR is explicitly V2+ scope. | `EV-COLOR-MANAGEMENT-SRGB` | Phase 3 exit | R-30 |

## Governance and Gate Evidence Rows

| Governance Concern | Profile | Owner | Validation Method | Evidence Artifact | Phase Gate | Risk Coverage |
|---|---|---|---|---|---|---|
| Architecture baseline closure | All profiles | Accountable Owner | Contract baseline checklist + ADR reference audit (ADR-011..016 + ADR-018) | `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-09 |
| Gate ledger integrity and cumulative release evaluation | All profiles | AI QA/Ops Workcell + Accountable Owner | Gate audit checklist + release gate simulation | `EV-GATE-MODEL-AUDIT` | Phase 0 exit + Phase 5 release | R-08, R-09 |
| Reactive symbol catalog ratification | All profiles | AI Runtime Workcell + AI Scene Workcell | Catalog signoff + cross-doc reference audit | `EV-SYMBOL-CATALOG` | Phase 0 exit | R-01, R-02, R-04 |
| Entitlement substrate design approval | All profiles | Accountable Owner | ADR review + threat model sign-off | `EV-ENTITLEMENT-DESIGN` | Phase 0 exit | R-15, R-17, R-18 |
| Rendering backend decision | All profiles | Accountable Owner + AI Runtime Workcell | ADR-011 review + wgpu/naga coverage audit against Phase-0 scene set | Covered under `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-03, R-22 |
| Script sandbox engine decision | All profiles | Accountable Owner + AI Runtime Workcell | ADR-012 review + QuickJS determinism + budget audit | Covered under `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-02, R-17 |
| Network security defaults | All profiles | Accountable Owner + AI Integration Workcell | ADR-013 review | Covered under `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-05, R-11, R-12 |
| Signing/notarization/auto-update design | All profiles | Accountable Owner | ADR-014 review + Phase-0 key/cert readiness check | Covered under `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-14, R-15 |
| Reference-app user-data import strategy | All profiles | Accountable Owner + AI Scene Workcell | ADR-015 review + artifact-class scope confirmation | Covered under `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-01, R-13 |
| Non-scene UI toolkit decision | All profiles | Accountable Owner + AI UX Workcell | ADR-018 review + DPI/target-size/contrast conformance pre-check | Covered under `EV-ARCH-CONTRACT-BASELINE` | Phase 0 exit | R-29, R-30 |
| Embedded docs authoring lane | All profiles | Accountable Owner + AI Docs Workcell | WP-9 scope confirmation + Phase-0 seed authoring plan | `EV-DOCS-PHASE0-SEED` | Phase 0 exit | R-20 |

## Traceability Completion Criteria
- No requirement row with empty owner.
- No requirement row without a validation method.
- No requirement row without at least one evidence artifact.
- No critical risk left without requirement-level mitigation coverage.
- Every evidence artifact must exist in `26-evidence-artifact-registry.md`.
- Architecture baseline closure requires `EV-ARCH-CONTRACT-BASELINE` before Phase 0 exit.
- Gate model closure requires `EV-GATE-MODEL-AUDIT` before Phase 0 exit.
- Release is cumulative: every shipped requirement row must map to evidence status `Pass` in the registry.
