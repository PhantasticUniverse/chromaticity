# Risk Register

## Scale
- Probability: Low / Medium / High
- Impact: Medium / High / Critical

## Residual-Risk Convention
Every risk row carries a **Residual** field expressing post-mitigation probability/impact. Residual is assessed on the assumption that every listed mitigation is fully deployed and its monitoring signal is green. The residual is what the release gate operates against; raw probability/impact describes the uncontrolled risk and motivates mitigation selection. If residual is `High/Critical` the risk remains release-blocking even after mitigations; if residual is `Low/Medium` or lower the risk is acceptable contingent on its mitigations holding.

## Document Structure (read this before cross-referencing)

This register has a single flat risk-ID space **R-01..R-37**. Each ID appears exactly once as a full entry:

- **R-01..R-10** are foundational risks with full entries immediately below under **Risks**.
- **R-11..R-30** are deep-dive, cleanroom-legal, supportability, accessibility, and color-management risks with full entries under **Deep-Dive Additions** at the bottom of this document. R-18 is out of numerical order (it lives near R-28 in that section due to how it was added; content is authoritative).
- **R-31..R-37** are non-scene UI implementation risks surfaced by `47-ui-port-plan.md` §12 + UX bundle intake throughput, with full entries under **UI Port + UX Bundle Additions**.
- The **Detection and Escalation Signals** table sits between the two section sets and indexes **all** R-01..R-30 in one place as one-line detection-signal pointers. The table's short phrasing is a summary, not an alternate definition; the section heading below the table is the authoritative title.

When another document cites `R-NN`, it refers to the full entry (the `###` section), not the table row. The table exists to make every risk's detection signal scannable in one view.

## Risks

### R-01 Compatibility Gaps in Scene Schema Variants
- Probability: High
- Impact: Critical
- Trigger: Imported scenes fail to load or behave differently.
- Mitigation:
 - compatibility normalizer with warning taxonomy,
 - corpus-based contract tests,
 - explicit migration guidance.
- Residual: Low / Medium. With `EV-SCENE-IMPORT` covering the full reference corpus and KM-01..07 fixtures exercising every known malformed class, the residual is limited to undiscovered legacy variants surfaced by field bugs (those flow to R-23).
- Owner: Scene system lead.

### R-02 Script Runtime Behavioral Drift
- Probability: Medium
- Impact: High
- Trigger: Event hooks/control mutations diverge from expected behavior.
- Mitigation:
 - script API contract tests,
 - deterministic event ordering docs,
 - compatibility shims for deprecated APIs.
- Residual: Low / Medium. QuickJS pinning + `EV-SCRIPT-CONTRACT` + `EV-DETERMINISM` close known cases; residual covers script patterns we haven't yet seen in the reference corpus.
- Owner: Script runtime lead.

### R-03 Performance Budget Overruns in Multipass Scenes
- Probability: High
- Impact: Critical
- Trigger: Frame time exceeds 16.67 ms budget in common scenes.
- Mitigation:
 - pass-level instrumentation,
 - quality degradation policy,
 - targeted optimization sprints.
- Residual: Medium / High. Baseline-profile-fixture-subset (SLO-03) is achievable but the long tail of 10-pass scenes on GPU-A is the known stress point; `stress-profile` has relaxed targets specifically because residual cannot be driven lower without hardware uplift.
- Owner: Rendering lead.

### R-04 Audio Latency and Jitter Instability
- Probability: Medium
- Impact: High
- Trigger: Input-to-uniform latency exceeds p95 budget.
- Mitigation:
 - bounded buffering strategy,
 - cadence synchronization audits,
 - benchmark fixtures and profiling.
- Residual: Low / Medium. `06-audio-engine.md` Sec.7 pins the pipeline at 375 Hz hop with a ~19.3 ms staleness bound; CP-02a snapshot handoff is lock-free. Residual is dominated by OS-level audio driver jitter on cold devices.
- Owner: Audio lead.

### R-05 Integration Adapter Fragility (OSC/MIDI/Media)
- Probability: Medium
- Impact: High
- Trigger: route mismatches, dropped events, reconnect failures.
- Mitigation:
 - adapter conformance tests,
 - health state machine,
 - bounded retry and clear diagnostics.
- Residual: Low. `EV-INTEGRATION-MATRIX` + 100/250/500/1000/2000 reconnect cadence + per-adapter isolation hold SLO-04 comfortably; the OSC adapter additionally realises bounded retry (5 attempts, exponential 100/200/400/800/1600 ms — landed across W4 OSC adapter slice work) and an adapter-internal health state machine (`OscHealthState::{Healthy, Recovering, Degraded}` in PR 100, with `simulate_recv_failure_for_test` seam added W7 PR2 to cover the `Degraded` transition). Route-mismatch diagnostics remain a slice-3 TODO. Residual is dominated by vendor-specific driver behaviour.
- Owner: Integration lead.

### R-06 State Race Conditions Across Input Sources
- Probability: Medium
- Impact: High
- Trigger: inconsistent control state under concurrent script/protocol updates.
- Mitigation:
 - command bus ordering rules,
 - single canonical commit path,
 - replay-based state validation tests.
- Residual: Low. Single canonical commit path plus `EV-STATE-REPLAY` deterministic replay effectively eliminate races in bounded tests; residual covers pathological burst patterns beyond fuzz coverage.
- Owner: State model lead.

### R-07 Memory Growth During Long Sessions
- Probability: Medium
- Impact: High
- Trigger: sustained memory growth beyond drift budget.
- Mitigation:
 - soak tests,
 - resource lifetime audits,
 - allocator telemetry.
- Residual: Low / Medium. `EV-SOAK-CHAOS` 24 h soak is sufficient for typical tour lengths; residual covers multi-day residency-club installations above soak coverage.
- Owner: Runtime/platform lead.

### R-08 Under-scoped Test Harness
- Probability: Medium
- Impact: High
- Trigger: parity claims without reproducible evidence.
- Mitigation:
 - prioritize golden harness as first-class deliverable,
 - require gate evidence before phase exits,
 - enforce supportability/privacy readiness audit before release,
 - W7 audit identified pre-W11 frontend/scene gaps; W11 PR #119 (commit `a62ef90`) introduced the first frontend test framework — Vitest + jsdom + `@testing-library/svelte` + `@testing-library/jest-dom` — closing the UI-side test-harness gap; W12 PR #121 (commit `2b4bffe`) added 7 reactive-symbol scene-import fixtures + 7 T2 golden hashes; W12 PR #122 (commit `e818752`) added the scene-lint pipeline + 14 T1 unit tests + 15 T2 corpus tests. Phase-1 EXIT achieved with 6/6 evidence gates Pass per `26-evidence-artifact-registry.md`.
- Residual: Low. Gate-ledger discipline + `EV-GATE-MODEL-AUDIT` make missing evidence a visible release-blocker, not a silent gap; the Phase-1 closeout demonstrates the harness scales to gate close on schedule.
- Owner: Quality lead.

### R-09 Architecture Drift Without Decision Logging
- Probability: Medium
- Impact: Medium
- Trigger: implementation choices diverge from requirements silently.
- Mitigation:
 - ADR requirement for architecture-impacting changes,
 - recurring design review checkpoint tied to phase-sequence gates.
- Residual: Low. Independent Architecture Reviewer sign-off (`23-decision-ownership-matrix.md`) closes the loop for major changes; minor drift is caught in PR review.
- Owner: Tech lead.

### R-10 Scope Creep Before MVP
- Probability: High
- Impact: High
- Trigger: adding non-critical features before parity foundation.
- Mitigation:
 - strict phase gate enforcement,
 - MVP vs post-MVP backlog separation.
- Residual: Medium. Round 6 trimmed scope to Core-only + watermark mechanism; residual covers ongoing pressure to pull post-MVP bets forward during implementation, which is a governance-discipline concern rather than a technical one.
- Owner: Product/tech lead.

## Review Cadence
- Weekly risk review during planning and implementation.
- Re-score probability and impact after each roadmap phase.
- Close risks only with objective evidence (tests, metrics, or production-like runs).

## Detection and Escalation Signals

| Risk ID | Detection Signal | Escalation Condition |
|---|---|---|
| R-01 | Scene import failures in `EV-SCENE-IMPORT` | >2% corpus failure in latest validation cycle |
| R-02 | Script contract test regressions (`EV-SCRIPT-CONTRACT`) | Any Severity 1 regression or repeated nondeterminism |
| R-03 | Performance benchmark misses (`EV-PERF-BUDGETS`) | p95 frame-time miss by >10% relative margin |
| R-04 | Audio latency benchmark misses | p95 audio latency above target in two consecutive windows |
| R-05 | Adapter conformance failures (`EV-INTEGRATION-MATRIX`) | Any required route/adapter fail without workaround |
| R-06 | State replay mismatches (`EV-STATE-REPLAY`) | Determinism failure on canonical replay suite |
| R-07 | Soak telemetry drift (`EV-SOAK-CHAOS`) | Memory drift exceeds threshold or crash in soak window |
| R-08 | Missing/weak evidence artifacts | Phase gate reached without required evidence |
| R-09 | ADR/implementation drift audits | Architecture-impacting changes merged without ADR update |
| R-10 | Backlog growth beyond phase scope | V1 scope includes non-bet features without waiver |
| R-11 | OSC route contract churn (`EV-INTEGRATION-MATRIX` alias coverage) | Breaking route behavior across profile version update |
| R-12 | Manual reconnection burden (`EV-INTEGRATION-MATRIX` + SLO-04) | Reconnect success below SLO or recovery time above target |
| R-13 | Authoring contract ambiguity (docs vs runtime behavior) | Increasing unresolved high-severity compatibility warnings |
| R-14 | IP contamination from reference materials (`EV-CLEANROOM-POLICY-BASELINE`, `EV-LEGAL-PROVENANCE`) | Missing kickoff policy artifacts or unresolved contamination finding in release candidate |
| R-15 | Third-party license inheritance risk (`EV-LICENSE-AUDIT`) | Any unresolved incompatible or unknown license in shipped artifacts |
| R-16 | Trademark or brand confusion risk (`EV-BRAND-REVIEW`) | Any unresolved trademark confusion issue before launch freeze |
| R-17 | Entitlement enforcement test regressions | Watermark-off achieved with absent/invalid token in any test run |
| R-18 | Post-MVP tier rollout readiness | Commerce backend or token-issuance not on track 1 quarter before post-MVP target date |
| R-19 | Accountable Owner unavailability | Owner unreachable beyond the succession threshold during implementation or review sequence |
| R-20 | Embedded docs authoring throughput below release velocity | `EV-DOCS-COVERAGE` below 60% of target pages by Phase 4c start |
| R-21 | Audio spec ambiguity | `EV-AUDIO-TRACE` fixtures fail drift tolerance on Phase 2 audio parity run |
| R-22 | Rendering-backend capability gap | wgpu/naga unable to compile a shipped scene's GLSL subset without author-visible refactor |
| R-23 | Reference-app behavior excavation incomplete | Field bug reports a reference-observed behavior not captured in any Chromaticity contract doc |
| R-24 | Undocumented reference uniforms break parity | `syn_InputLevel` / `syn_RawLevel` / `syn_BassExpoCurve` formula divergence surfaces as `EV-AUDIO-TRACE` drift or broken shipped scene load |
| R-25 | Default outbound traffic leak | Any release build produces network traffic to a destination not declared in `OBSERVABILITY-INVENTORY.md` with all observability toggles off |
| R-26 | Bundle hygiene regression | Release candidate contains a disallowed artifact class from `adr/ADR-014-signing-notarization-auto-update.md` Sec.7 (C/C++ headers, legacy DSA pem, etc.) |
| R-27 | Reference-app extension handling regression | Chromaticity accidentally registers as a handler for `.synScene` (reference-owned), or `.chroma` / `.chromaPack` / `.chromaShow` associations fail to install/uninstall cleanly |
| R-28 | Data-ingestion architectural debt | A scene ships that depends on `syn_Data_*` / `DATA_SOURCES` without the forward-compat reservations honored, forcing a breaking change when ADR-017 lands |
| R-29 | Accessibility regression (`EV-ACCESSIBILITY-AA`) | Any new axe-core violation on non-scene UI, or WCAG 2.2 AA manual-audit failure not fixed before release candidate |
| R-30 | Color management regression (`EV-COLOR-MANAGEMENT-SRGB`) | Any non-scene UI pixel rendered outside sRGB on a color-managed display in release candidate; ColorSync/ICC round-trip failure on documented profiles |
| R-31 | `ts-rs` type-binding drift between Rust core and TypeScript UI | CI fails the `npm run check` step on stale bindings; any UI build against a bindings file whose hash differs from a fresh `build.rs` export |
| R-32 | CodeMirror 6 plugin-ecosystem fragmentation | Any pinned CM6 plugin goes >12 months without upstream activity AND ships a known regression in the authoring flow |
| R-33 | wgpu / WebView2 composition spike fails | wgpu-composition spike per `47-ui-port-plan.md` §6 (rolled forward to `EV-RENDER-CONFORMANCE` Phase-2 coverage after Phase-1 closeout) cannot achieve transparent-region compositing on Windows WebView2, or DPI scaling at >=1.75x regresses pixel-rounding below `target.min` |
| R-34 | Svelte 5 runes idiom drift | Two or more PRs in the same review sequence diverge on state placement (`$state` vs store) for equivalent cross-component concerns; or `eslint-plugin-chromaticity-ui-tokens` lint fails on hard-coded tokens after a one-release grace sequence |
| R-35 | NFR-10 keystroke-to-paint latency miss on WebView2 | p95 keystroke-to-paint budget (`37-authoring-environment-spec.md` §3.3a) exceeded on the GPU-A baseline Windows class; macOS WKWebView may pass while Windows WebView2 fails |
| R-36 | Tauri 2 signing / notarization surface upstream churn | Apple notarization path or Authenticode signing workflow breaks across a minor Tauri 2 bump; `EV-RELEASE-ATTESTATION` regression on release candidate build |
| R-37 | UX bundle revision throughput below Phase 0 velocity | `EV-UX-*-CONTRACT` rows remain `InProgress` with no bundle `v0.2-beta` landing in the declared Phase-0 sequence; P0 gap-audit rows in `44-ux-bundle-gap-audit.md` accumulate without remediation |

## Deep-Dive Additions

### R-11 Route Contract Churn in OSC Ecosystem
- Probability: High
- Impact: High
- Trigger: external show-control patches break after updates or environment changes.
- Mitigation:
 - versioned OSC contract,
 - route alias compatibility layer,
 - route conformance regression tests.
- Residual: Low / Medium. `osc-parity-v1` version pinning + enumerated `compat-syn-v1` alias table mean existing TouchOSC/Chataigne templates survive upgrades; residual covers new controllers post-V1.
- Owner: Integration lead.

### R-12 Manual Reconnection Burden During Live Performance
- Probability: Medium
- Impact: Critical
- Trigger: controller or network adapter drop requires manual intervention mid-show.
- Mitigation:
 - automatic reconnect orchestration,
 - persistent binding state,
 - operator health alerts with fallback recommendations.
- Residual: Low. SLO-04 with p50 <= 3 s / p95 <= 10 s / p99 <= 20 s plus binding-persistence cover the live-show case; residual is bounded to catastrophic transport failure (operator action required anyway).
- Owner: Integration/runtime lead.

### R-13 Authoring Contract Ambiguity (Docs vs Runtime Behavior)
- Probability: High
- Impact: High
- Trigger: scenes rely on undocumented or legacy behavior and fail under strict interpretation.
- Mitigation:
 - compatibility-aware loader + warning taxonomy,
 - preflight lint suite,
 - lifecycle contract documentation and tests.
- Residual: Medium. Docs-vs-runtime divergence is an ongoing excavation problem; every release cycle may surface a new ambiguity. Paired with R-23 (behavior excavation) as the escalation path.
- Owner: Scene/authoring lead.

### R-14 IP Contamination from Reference Materials
- Probability: Medium
- Impact: Critical
- Trigger: shipped implementation artifacts include copied reference code/assets/doc text.
- Mitigation:
 - kickoff policy baseline (`EV-CLEANROOM-POLICY-BASELINE`) and mandatory provenance manifests,
 - release artifact scans for prohibited content,
 - legal/compliance gate before MVP freeze.
- Residual: Low. Internal-vs-customer corpus partitioning plus per-page `EV-DOCS-CLEANROOM` provenance stamping plus `EV-LEGAL-PROVENANCE` audit at Phase 5 collectively make accidental contamination a gate failure, not a shipped defect.
- Owner: Accountable owner.

### R-15 Third-Party License Inheritance Risk
- Probability: Medium
- Impact: Critical
- Trigger: bundled references or dependencies introduce incompatible license obligations.
- Mitigation:
 - dependency and asset license inventory with approval workflow,
 - release-blocking license audit evidence (`EV-LICENSE-AUDIT`),
 - legal sign-off required for exceptions.
- Residual: Low / Medium. `EV-LICENSE-AUDIT` is release-blocking; lygia/hg_sdf handled as Conditional with license election pre-inclusion. Residual covers transitive dependencies relicensed mid-cycle.
- Owner: Accountable owner.

### R-16 Trademark or Brand Confusion Risk
- Probability: Low
- Impact: High
- Trigger: naming, UI copy, or marketing surfaces imply affiliation with reference product.
- Mitigation:
 - product naming and copy review checklist,
 - legal review of launch-facing assets,
 - release brand evidence gate (`EV-BRAND-REVIEW`),
 - explicit brand differentiation guidelines.
- Residual: Low. Brand/Trademark Review Checklist is itemized (name/domain/framework names/docs taxonomy/outbound endpoints); Snap rename + `.chroma*` extension ownership + `/reference/*` docs path already distance Chromaticity from reference-owned surface.
- Owner: Accountable owner.

### R-17 Entitlement Bypass Risk
- Probability: Low at MVP (watermark-only gating has low revenue impact); Medium post-MVP once tiered features are activated.
- Impact: Low-Medium at MVP (cosmetic watermark removal is the only forgery payoff); Medium-High post-MVP (Advanced/NDI/Developer capabilities).
- Trigger: signed entitlement token forged, scripted sandbox escape mutates entitlement state, or adapter-boundary feature check skipped.
- Mitigation:
 - signed token verification using embedded public key per `adr/ADR-009-activation-and-entitlements.md`,
 - adapter-boundary enforcement (feature checks at adapter startup, not post-hoc),
 - script sandbox prevents entitlement-state mutation (NFR-6 + `adr/ADR-003-script-runtime-sandbox.md` + `adr/ADR-012-script-sandbox-engine.md`),
 - short-expiry tokens and deny-list revocation,
 - integration evidence `EV-ENTITLEMENT-ENFORCEMENT` / `EV-ENTITLEMENT-OFFLINE` / `EV-ENTITLEMENT-REVOCATION` cover positive and negative cases.
- Residual: Low at MVP (watermark-only payoff); Low / Medium post-MVP with commerce backend (short-expiry tokens plus adapter-boundary enforcement make runtime bypass uneconomic vs license-fee).
- Owner: Accountable owner.

### R-19 Accountable Owner Unavailability (SPOF)
- Probability: Medium over a multi-phase timeline
- Impact: High (blocks every gate that requires Accountable Owner sign-off)
- Trigger: illness, burnout, travel beyond reach, personal emergency exceeding 72 hours.
- Mitigation:
 - Deputy Accountable Owner (Fletcher Hammond) named in `23-decision-ownership-matrix.md` with standing interim-authority assignment,
 - three-tier succession ladder (T1 >72h interim / T2 >14 days acting / T3 permanent) that cleanly transfers non-legal authority,
 - interim decisions re-ratified within 5 business days of AO return,
 - Reviewer Roster document with up-to-date contacts + canonical return-of-availability signals,
 - non-waivable legal gates remain blocked (no interim legal decisions by Deputy; contracted counsel engaged separately).
- Residual: Low. With Deputy slot filled, the SPOF is structurally removed for non-legal gates; residual covers only legal gate timing during combined AO-plus-counsel unavailability windows.
- Owner: Accountable owner (pre-designated succession).

### R-20 Embedded Docs Authoring Throughput Below Release Velocity
- Probability: Medium
- Impact: High (blocks `EV-DOCS-COVERAGE` release cumulative gate)
- Trigger: docs authored per sprint falls behind the symbol catalog and OSC/MIDI route additions; clean-room review queue accumulates.
- Mitigation:
 - WP-9 is a dedicated work package with named owner (AI Docs Workcell),
 - Phase-0 seed pages gate (`EV-DOCS-PHASE0-SEED`) forces early authoring cadence,
 - docs-version-sync CI (`EV-DOCS-VERSION-SYNC`) blocks drift each release,
 - per-page provenance workflow makes throughput visible in status reviews.
- Residual: Medium. Version-sync CI prevents silent drift but does not create authoring capacity; scope overrun remains the realistic failure mode and is the one R-20 ultimately escalates on.
- Owner: Accountable owner.

### R-21 Audio Algorithm Spec Ambiguity
- Probability: Low (now that `06-audio-engine.md` Sec.7 is normative), Medium-High if Sec.7 drifts out of sync with implementation.
- Impact: High (blocks `EV-AUDIO-TRACE` parity claims)
- Trigger: implementation deviates from Sec.7 constants; fixtures fail drift tolerance; `syn_BPMConfidence` or hit algorithm produces qualitatively different output than declared.
- Mitigation:
 - Sec.7 constants pinned per compatibility profile; changes require profile bump,
 - audio fixtures include algorithm-parameter checksum in `EV-AUDIO-TRACE` run metadata,
 - Phase 2 audio parity gate exercises every family before Phase 3.
- Residual: Low. Sec.7 is mathematically complete; the parameter-checksum gating mechanism makes spec-vs-implementation drift an immediate CI failure rather than a field surprise.
- Owner: AI Audio Workcell + Accountable Owner.

### R-22 Rendering-Backend Capability Gap
- Probability: Medium (naga's GLSL frontend has known gaps)
- Impact: High (a shipped scene that won't compile is a trust-erosion event)
- Trigger: wgpu/naga unable to compile a reference-corpus GLSL subset without requiring the author to refactor; backend crashes under multipass pressure on a target GPU class.
- Mitigation:
 - Phase 1 `EV-RENDER-CONFORMANCE` exercises the full Phase-0 scene set on both Metal and DX12,
 - wgpu version pinned per release, upgrade gated on regression run,
 - naga gap list maintained in the embedded docs so scene authors know what to avoid,
 - fallback: for unsupported scenes, authoring environment surfaces explicit diagnostic rather than silent miscompile.
- Residual: Medium. wgpu/naga maturity is improving but not fully mature; the published gap list plus authoring diagnostic make the failure mode "scene refuses to compile with clear guidance" rather than silent miscompile, which keeps impact bounded.
- Owner: AI Runtime Workcell + Accountable Owner.

### R-23 Reference-App Behavior Excavation Incomplete
- Probability: High (behavior excavation is inherently open-ended)
- Impact: Medium (field bug fatigue, not a show-ending failure)
- Trigger: field bug report reveals a reference-observed behavior not captured in Chromaticity contracts, symbol catalog, or `06-audio-engine.md` Sec.7.
- Mitigation:
 - contract documents explicitly list known-unknowns as Open Questions (retained in each document),
 - field bug reports trigger targeted behavior excavation and, where required, an ADR + profile bump,
 - `R-13 Authoring Contract Ambiguity` already covers docs-vs-runtime divergence; this risk covers the broader "we didn't know this was a thing" case.
- Triage tie-breaker vs R-13: R-13 applies when the diverging behavior is **documented in the reference's embedded docs** (docs-vs-implementation gap); R-23 applies when the behavior is **only observed in a shipped scene or runtime** with no reference-doc coverage (excavation gap).
- Residual: Medium. Excavation is inherently open-ended and residual stays Medium until the reference corpus is fully regression-tested. Profile-bump workflow bounds the impact of each new discovery.
- Owner: AI Scene Workcell + Accountable Owner.

### R-18 Post-MVP Tiered Edition Rollout Slip
- Probability: Medium
- Impact: Medium (revenue deferral; does not block product function)
- Trigger: commerce backend and token-issuance service not ready when Advanced/NDI/Developer features are otherwise ship-ready post-MVP.
- Mitigation:
 - MVP ships Core-only with watermark-on/off; no tier dependency at freeze,
 - capability-bitmask substrate means activating tokens post-MVP is a **data** change, not a runtime rearchitecture (`adr/ADR-009-activation-and-entitlements.md`),
 - `entitlement-v2` token schema already carries trial/expiry/session/machine-binding claims so commerce backend addition requires no token-format migration,
 - adapter-boundary hooks wired in Phase 4a so activating tokens post-MVP is a data change,
 - early vendor/identity-provider scoping for token issuance (no lock-in),
 - tracked as scheduled post-MVP work package with named owner.
- Residual: Low / Medium. MVP ships without dependency so slip is revenue-only, not functional; residual tracks the business timing risk rather than a technical one.
- Owner: Accountable owner.

### R-24 Undocumented Reference Uniform Formula Drift
- Probability: Medium
- Impact: High (one broken shipped scene is a visible parity failure)
- Trigger: shipped reference scenes that use `syn_InputLevel`, `syn_RawLevel`, or `syn_BassExpoCurve` render differently on Chromaticity than on the reference, or `EV-AUDIO-TRACE` fixtures for family `LVL` fall outside tolerance on those uniforms.
- Mitigation:
 - `36-reactive-symbol-catalog.md` Sec.2.10 and `06-audio-engine.md` Sec.7.12 lock normative formulas derived from observed reference behaviour,
 - `EV-AUDIO-TRACE` fixture set includes `audio_indicators.synScene` (the one shipped scene observed to use `syn_InputLevel`) and synthetic fixtures exercising each formula branch,
 - compat-profile bump required if a later observation invalidates the Sec.7.12 formulas, per `22-compatibility-versioning-policy.md`.
- Residual: Low / Medium. Formulas are locked and fixtures exist; residual covers the small number of scenes that may use these uniforms in unobserved patterns.
- Owner: AI Audio Workcell + Accountable Owner.

### R-25 Default-Off Outbound Traffic Leak
- Probability: Low
- Impact: Critical (a single show-night outbound leak undoes the entire operator-trust posture of `adr/ADR-016-observability-and-telemetry-posture.md`)
- Trigger: release build produces network traffic to any destination not declared in `OBSERVABILITY-INVENTORY.md` when all observability toggles are off.
- Mitigation:
 - `EV-OBSERVABILITY-INVENTORY` and `EV-ANALYTICS-ZERO-DEFAULT` assert this in Phase 0 and Phase 4d respectively,
 - `EV-ATS-NO-THIRD-PARTY-EXCEPTIONS` fails the release if `Info.plist` grows a new `NSExceptionDomains` entry,
 - dependency intake checklist requires any third-party library to declare its outbound calls; unexpected calls fail intake review.
- Residual: Low. Three independent gates (inventory, analytics-zero-default, ATS-exception grep) make a leak a release-blocker; residual is bounded to new-dependency oversight between release cycles.
- Owner: AI QA/Ops Workcell + Accountable Owner.

### R-26 Bundle Hygiene Regression
- Probability: Medium (build-tool output drift is a known failure mode)
- Impact: Medium (downloadable size inflation + supply-chain-scanner noise, not functional failure)
- Trigger: a release candidate bundle contains a C/C++ header, `.pdb`/`.dSYM`, legacy DSA `.pem`, or a build-config file (`package.json`, `webpack.config.js`, `.asmdef`) per `adr/ADR-014-signing-notarization-auto-update.md` Sec.7.
- Mitigation:
 - `EV-BUNDLE-HYGIENE` deny-list pipeline enforced pre-notarization,
 - release checklist explicitly asks "has any new dependency been added that ships source trees?",
 - notarization diff against previous release flags unexpected file-count growth.
- Residual: Low. Deny-list is deterministic and runs pre-notarization; residual is bounded to file-class types not yet on the deny-list.
- Owner: AI QA/Ops Workcell.

### R-27 Reference-App Extension Handling Regression
- Probability: Low
- Impact: High (legal/branding damage if Chromaticity claims the reference's `.synScene` extension; operator confusion if `.chroma` associations fail to install or uninstall cleanly)
- Trigger: dual-install test reveals Chromaticity declaring a UTI or ProgID for `.synScene`; or fresh-install test shows `.chroma`/`.chromaPack`/`.chromaShow` not routing to Chromaticity; or uninstall leaves orphan file associations.
- Mitigation:
 - `22-compatibility-versioning-policy.md` Sec.Package Extensions bans any `.synScene` UTI/ProgID declaration and enumerates the three Chromaticity-owned extensions,
 - `EV-EXTENSION-OWNERSHIP` runs at Phase 5 and grep-asserts `Info.plist` and the Windows registry footprint against the ban list,
 - uninstall script is reviewed against the policy each release.
- Residual: Low. Automated grep gate + dual-install drill + per-release uninstall review collectively make violation a release failure, not a shipped defect.
- Owner: AI Integration Workcell + Accountable Owner.

### R-28 Data-Ingestion Architectural Debt
- Probability: Medium (the differentiator is compelling; pressure will exist to ship a "quick" data-ingesting scene before ADR-017 lands).
- Impact: High (a scene shipped against a non-reserved uniform namespace, scene-json key, or OSC prefix becomes a breaking-change debt when ADR-017 activates its intended semantics; retrofitting is invariably more expensive than reservation).
- Trigger: any of (a) a shipped scene references a `syn_Data_*` uniform without a matching `DATA_SOURCES` stanza, (b) a shipped scene uses an undeclared scene.json root key that conflicts with the reserved `DATA_SOURCES` schema, (c) a third-party tool or integration squats on the `/data/*` OSC prefix expecting a different semantic, (d) a `data.ingest.*` capability token is issued to a client runtime that has not implemented the capability evaluator.
- Mitigation:
 - Every reservation in `41-data-ingestion-differentiator.md` Sec.Forward-compatibility reservations locked in MVP is in place and asserted by existing evidence (`EV-SYMBOL-CATALOG`, `EV-ARCH-CONTRACT-BASELINE`, `EV-ENTITLEMENT-ENFORCEMENT`, `OSC-CONF-16`).
 - MVP runtime **fails closed** on the reservations: typos in `syn_Data_*` strict-reject (see `36-reactive-symbol-catalog.md` Sec.1.6 rules), missing `DATA_SOURCES` validation drops to strict-reject or compat-drop-with-warn (see CP-12), `/data/*` OSC prefix rejects inbound at MVP with `prefix-reserved`. These behaviors mean pre-ADR-017 misuse cannot silently succeed.
 - ADR-017 must be authored before any first-party scene or provider ships that uses the `syn_Data_*` surface; `EV-ARCH-CONTRACT-BASELINE` release gate grows an `ADR-017 required` item at the point the first `DATA_SOURCES`-declaring scene is proposed for inclusion in the bundled corpus.
- Residual: Low. Fail-closed posture plus `prefix-reserved` diagnostic make architectural debt a visible runtime rejection rather than silent shipping; the only remaining risk is third-party scope-creep pressure, which is governance-level.
- Owner: Accountable Owner (for scope discipline) + AI Runtime Workcell (for enforcement of the fail-closed posture).

### R-29 Accessibility Regression on Non-Scene UI
- Probability: Medium
- Impact: High (inaccessibility blocks release per NFR-11 and is reputational/legal risk for accessible-workplace operators)
- Trigger: `EV-ACCESSIBILITY-AA` fails on any WCAG 2.2 AA success criterion applicable to non-scene UI; axe-core CI reports new violations; VoiceOver/NVDA manual audit finds unlabeled controls; 3:1 focus-indicator contrast fails on either dark or light backgrounds; 24x24 target-size violation shipped; OS reduce-motion signal not honored.
- Mitigation:
 - non-scene UI toolkit decision locked in ADR-018 with DPI-scaling + target-size + contrast behavior documented before UX build-out,
 - axe-core gate runs on every PR that touches non-scene UI code,
 - per-release VoiceOver (macOS) and NVDA (Windows) manual audit checklist,
 - design-token table with min-contrast pairs maintained alongside `37-authoring-environment-spec.md`,
 - reduced-motion behavior validated as an explicit sub-assertion inside `EV-ACCESSIBILITY-AA`.
- Residual: Low / Medium. CI gate catches automatable regressions; residual covers subjective assistive-tech behavior and is bounded by the per-release manual audit.
- Owner: AI UX Workcell + Accountable Owner.

### R-30 Color Management Regression
- Probability: Low
- Impact: Medium-High (wrong-color output on calibrated displays is an immediate operator-trust failure in touring environments)
- Trigger: shipped build renders non-scene UI outside sRGB on a color-managed display; scene output ignores OS display profile; ColorSync (macOS) or ICC (Windows) round-trip test fails on documented profiles; HDR scope creeps into V1 without ADR-011 Sec.8 update.
- Mitigation:
 - ADR-011 Sec.7 + Sec.8 lock V1 as sRGB with OS display-profile respect; HDR is V2+ and is scope-guarded in `01-charter.md`,
 - `EV-COLOR-MANAGEMENT-SRGB` runs on the Phase 3 exit and on every release candidate across the declared display profiles,
 - multi-GPU selection policy (ADR-011 Sec.7) pins the measurement GPU for reproducibility,
 - non-scene UI toolkit decision (ADR-018) must declare its color-management defaults.
- Residual: Low. V1 scope (sRGB only) is narrow and testable; HDR scope creep is a governance concern already covered by R-10.
- Owner: AI Runtime Workcell + Accountable Owner.

## UI Port + UX Bundle Additions

### R-31 `ts-rs` Type-Binding Drift Between Rust Core and TypeScript UI
- Probability: Medium (realistic without CI enforcement; low with enforcement)
- Impact: High (subtly broken IPC contracts compile but mis-route values at runtime, surfacing as hard-to-diagnose UI bugs)
- Trigger: `npm run check` passes while `app/shell/` types have drifted from the committed bindings; or CI produces fresh `ts-rs` export output that differs from the repo under `src/lib/ipc/bindings/`.
- Mitigation:
 - `build.rs` in `app/shell/` runs `ts_rs::export_to(...)` on every Rust build,
 - CI gate fails if a fresh `ts-rs` export produces a diff vs the committed bindings,
 - `47-ui-port-plan.md` §7 locks the shared-type inventory and `EV-COMPAT-PROFILE-DIFF` consumes the drift check,
 - all cross-boundary types live in a single crate (`chroma-ui-bridge`) to concentrate review attention.
- Residual: Low. The CI gate deterministically catches drift; residual risk is confined to a brief window during a multi-commit migration that gets bisected.
- Owner: AI Runtime Workcell + AI QA/Ops Workcell.

### R-32 CodeMirror 6 Plugin-Ecosystem Fragmentation
- Probability: Medium (CM6 is the newer major and some plugins lag)
- Impact: Medium (authoring-env authoring friction; does not affect operator path)
- Trigger: A pinned CM6 plugin goes dormant (>12 months without upstream activity) and ships a known regression; OR a required extension (GLSL mode, hover-docs bridge) has no actively-maintained community implementation.
- Mitigation:
 - pin exact versions per `22-compatibility-versioning-policy.md`,
 - vendor any off-the-shelf plugin with license-clear code into the repo if upstream goes dormant (Chromaticity-authored fork),
 - Chromaticity-specific extensions (`chromaSymbolCompletion` / `chromaSymbolHoverDoc` / `chromaDiagnostics` / `chromaLiveReloadIndicator` per `47-ui-port-plan.md` §5.2) are first-party so the project owns their lifecycle,
 - GLSL language mode choice recorded during Phase 1 (lezer-glsl community grammar OR hand-rolled StreamLanguage); `EV-LICENSE-AUDIT` validates either path.
- Residual: Low / Medium. Maintenance cost is absorbed by the first-party-extension posture; community-plugin choice is fallback-able.
- Owner: AI Scene Workcell (editor surface) + AI QA/Ops Workcell (dependency health audit).

### R-33 wgpu / WebView2 Composition Spike Fails
- Probability: Medium (named as a Phase 1 spike in ADR-018 §Consequences; Phase-1 closeout at `4cbb0a4` exited without `engine/runtime/spikes/wgpu-composition/` landing, so the spike is now carried under `EV-RENDER-CONFORMANCE` Phase-2 coverage instead of a standalone Phase-1 deliverable)
- Impact: High (fallback paths compromise scene-canvas colour fidelity or introduce perceived latency on output)
- Trigger: wgpu-composition spike per `47-ui-port-plan.md` §6.1 fails any of: native-surface attach/detach across view changes, colour-space preservation on display-profile switch, fullscreen-transition timing within one frame, DPI scaling at 1.0x-3.0x without pixel rounding regressions, preview-tile latency within NFR-10.
- Mitigation:
 - de-risking framing locked in ADR-018 §Consequences,
 - fallback: web-rendered canvas via `<canvas>` + WebGPU (or WebGL) for non-fullscreen preview, native wgpu only in fullscreen path,
 - preview-tile fallback in `47-ui-port-plan.md` §6.2 uses CPU-decoded periodic readbacks for off-stage views regardless of spike outcome,
 - `EV-RENDER-CONFORMANCE` (Planned per `26-evidence-artifact-registry.md`; Phase 2 + release cumulative gate) is the rolled-forward coverage for the spike outcome.
- Residual: Medium. Even with fallback, composition-path complexity is a long-tail source of platform-specific bugs; residual is bounded by scope of the fallback.
- Owner: AI Runtime Workcell + Accountable Owner.

### R-34 Svelte 5 Runes Idiom Drift
- Probability: Low (port waves W6-W22 settled the rune-vs-store split across primitives, app shell, and routed views; Phase-1 EXIT was W12 PR #122 commit `e818752` and the `**/*.svelte.ts` parser override landed in the same wave at W12 PR #123 commit `0d83fb5` once the first reactive store needed in-module runes — post-Phase-1 ports W13-W22 carried the convention forward without re-litigation)
- Impact: Medium (inconsistent state placement produces bugs that are easy to introduce and expensive to debug)
- Trigger: Two or more PRs in the same review sequence diverge on state placement for equivalent cross-component concerns (e.g. one puts `$state` in a child, another lifts it to a parent context); OR `eslint-plugin-chromaticity-ui-tokens` lint fails on hard-coded tokens after a one-release grace sequence.
- Mitigation:
 - Svelte 5 runes-in-module idiom is documented in `43-development-standards.md` §12.5 (file-naming requirement + ESLint parser-override rationale, anchored on the W12 first-occurrence at `src/lib/stores/route.svelte.ts`),
 - single code-owner for `ui/` reviews held through Phase 1 to let idioms settle,
 - `eslint-plugin-chromaticity-ui-tokens` fails CI on hard-coded tokens,
 - state-placement decision tree (rune vs store vs context) is rolled into the same `43-development-standards.md` §12.5 subsection rather than a separate §13 block as originally scoped.
- Residual: Low. Ownership + standards + lint close the loop; Phase-1 closeout demonstrated the harness held without divergence-driven rework.
- Owner: AI UX Workcell + AI QA/Ops Workcell.

### R-35 NFR-10 Keystroke-to-Paint Latency Miss on WebView2
- Probability: Medium (macOS WKWebView and Windows WebView2 have historically differed on layout / paint characteristics)
- Impact: High (NFR-10 authoring budget miss is a release blocker)
- Trigger: Measured p95 keystroke-to-paint exceeds `37-authoring-environment-spec.md` §3.3a budget on the GPU-A baseline Windows class during M5 latency measurement; macOS path may be green while Windows fails.
- Mitigation:
 - measurement harness runs on both platforms during S5 per `47-ui-port-plan.md` implementation sequence,
 - if Windows misses, defer hover-doc / completion to an async render tier and fall back to pre-computed symbol-catalog tries,
 - worst case: reduce `chromaSymbolHoverDoc` latency via Rust-side precomputed index rather than linear scan.
- Residual: Low / Medium. Mitigations are layered; the worst-case hover-doc degrades from instant to "~50ms" which is still acceptable for an authoring surface.
- Owner: AI Runtime Workcell + AI UX Workcell.

### R-36 Tauri 2 Signing / Notarization Surface Upstream Churn
- Probability: Low (Tauri 2 is post-1.0, API surface for signing is stable)
- Impact: High (release blocker if Apple notarization or Authenticode workflow breaks)
- Trigger: Apple notarization path or Authenticode signing workflow breaks across a minor Tauri 2 bump; `EV-RELEASE-ATTESTATION` regression on release candidate build.
- Mitigation:
 - pin Tauri to a specific 2.0.x minor per `22-compatibility-versioning-policy.md`,
 - bumps run the full regression suite including signing + notarization round-trip on both platforms,
 - Sparkle 2 (macOS) and Authenticode (Windows) paths per ADR-014 are isolated from Tauri 2 internals to the extent possible,
 - contingency: direct `codesign` / `signtool` invocation outside Tauri's wrapper if needed.
- Residual: Low. Tauri 2 signing surface has been stable through the pinned release window; contingency paths exist.
- Owner: AI QA/Ops Workcell + Accountable Owner.

### R-37 UX Bundle Revision Throughput Below Phase 0 Velocity
- Probability: Medium (Phase-0 bundle delivery risk closed by v0.2-beta canonical 2026-04-23f; row retained as a post-Phase-1 port-velocity monitor against the post-bundle dev queue)
- Impact: High (PARTIAL view — Settings — gates Phase 2 authoring and live-reactivity work; remaining chrome element (CrashResumeBanner) gates the Phase 3+ entitlement / crash-recovery surface; Vitest adoption depth gates regression catch on the ported views)
- Trigger: Settings PARTIAL state regresses or stalls through Phase 2 kickoff with no completion signal; OR CrashResumeBanner misses the Phase 3+ runtime CrashResumeState IPC wiring window; OR Vitest view-integration coverage remains absent past Phase 2 kickoff.
- Mitigation:
 - bundle v0.2-beta canonical 2026-04-23f closed the original Phase-0 throughput risk; Phase-0 mitigations (single-rev scope brief, pre-ratified strawmans in `48-ux-bundle-open-questions-routing.md`, ADR-018 structural amendment) ran their course and are retained for historical record,
 - `47-ui-port-plan.md` wave-log discipline (catch-up landed for W22-W25 + W26-W29 PR #180) preserves per-wave port tracking and surfaces stalls before they accumulate,
 - `52-parity-coverage-matrix.md` tracks port-state per view (DONE / PARTIAL / STUB / NOT STARTED) so chrome-element gaps and PARTIAL-view regressions stay visible,
 - capabilities-store milestone (ChromaCtx → Svelte) landed at W33b PR #189 (`src/lib/stores/capabilities.svelte.ts`); chrome cohort consumers (WatermarkOverlay, AdapterPill, LockedRow) read through the store directly without prop-drilling.
- Residual: Low. Bundle v0.2-beta canonical (stamped 2026-04-23f); 13 of 14 routed views DONE (W30 PR #182 License STUB → DONE; W31 PR #184 Patch PARTIAL → DONE; W32 PR #186 Inputs PARTIAL → DONE — xy / bang / dropdown widget primitives; W33c PR #190 added Inputs expansion-A search + group-tab toolbar); 1 PARTIAL (Settings — L-scope per W31 architect-agent scout); 0 STUB. Chrome-element cohort largely closed: W33a PR #188 shipped WatermarkOverlay (live-wired in W33b PR #189 to `capabilities.has('render.watermark.off')`); W34a PR #192 shipped AdapterPill replacing the W15 PR1 inline placeholder in TopBar with the bundle-faithful 6-state widget; W34b PR #193 shipped LockedRow as the canonical feature-locked-affordance treatment ready for post-MVP SKU rollouts. Only CrashResumeBanner remains in the cohort, pending Phase-3+/4a runtime CrashResumeState IPC. Residual covers Settings PARTIAL slip, CrashResumeBanner port slip, and Vitest adoption depth.
- Owner: AI UX Workcell + Accountable Owner.
