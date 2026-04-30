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
- Probability: Medium (named as a Phase 1 spike in ADR-018 §Consequences; Phase-1 closeout at `4cbb0a4` exited without `engine/runtime/spikes/wgpu-composition/` landing, so the spike is now carried under `EV-RENDER-CONFORMANCE` Phase-2 coverage instead of a standalone Phase-1 deliverable; first Phase-2 evidence-producing slice landed 2026-04-29 — `engine/render/` crate + `compile_and_render_single_pass` entry point + T1 naga-frontend drift pins + T2 deterministic UV-gradient byte-hash corpus, advancing the deferred spike's foundational pipeline plumbing)
- Impact: High (fallback paths compromise scene-canvas colour fidelity or introduce perceived latency on output)
- Trigger: wgpu-composition spike per `47-ui-port-plan.md` §6.1 fails any of: native-surface attach/detach across view changes, colour-space preservation on display-profile switch, fullscreen-transition timing within one frame, DPI scaling at 1.0x-3.0x without pixel rounding regressions, preview-tile latency within NFR-10.
- Mitigation:
 - de-risking framing locked in ADR-018 §Consequences,
 - fallback: web-rendered canvas via `<canvas>` + WebGPU (or WebGL) for non-fullscreen preview, native wgpu only in fullscreen path,
 - preview-tile fallback in `47-ui-port-plan.md` §6.2 uses CPU-decoded periodic readbacks for off-stage views regardless of spike outcome,
 - `EV-RENDER-CONFORMANCE` (Planned per `26-evidence-artifact-registry.md`; Phase 2 + release cumulative gate) is the rolled-forward coverage for the spike outcome; slice-1 toolchain plumbing now landed (`engine/render` crate, wgpu 26 + naga 26 GLSL→SPIR-V via `ShaderSource::Naga`, RGBA8Unorm offscreen target, FNV-1a 64 byte-hash corpus) so successive slices can build multi-pass / feedback / transition / backend-matrix conformance against the same harness rather than starting from zero.
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
- Residual: Low. Bundle v0.2-beta canonical (stamped 2026-04-23f); **14 of 14 routed views DONE** (W30 PR #182 License STUB → DONE; W31 PR #184 Patch PARTIAL → DONE; W32 PR #186 Inputs PARTIAL → DONE — xy / bang / dropdown widget primitives; W33c PR #190 added Inputs expansion-A search + group-tab toolbar; W35 PR #195 added Playlist inspector edit-mode — Label / Scene / Duration / Crossfade / Control Preset / Trigger inputs + Save / Delete actions, mutations local-state-only with persistence deferred to Phase-3 EV-PRESET-PLAYLIST per docs/26; W38-W42 PRs #201/#203/#206/#208/#210 closed the Settings PARTIAL slip end-to-end across right-rail Preview+Diagnostics, Audio Input Gain+Monitoring meter, Tempo & Sync extensions, Graphics extensions, and Live Sources extensions); **0 PARTIAL; 0 STUB**. Chrome-element cohort largely closed: W33a PR #188 shipped WatermarkOverlay (live-wired in W33b PR #189 to `capabilities.has('render.watermark.off')`); W34a PR #192 shipped AdapterPill replacing the W15 PR1 inline placeholder in TopBar with the bundle-faithful 6-state widget; W34b PR #193 shipped LockedRow as the canonical feature-locked-affordance treatment ready for post-MVP SKU rollouts. W44 PR #214 ported the bundle's `CrashResumeBanner` (`src/lib/widgets/CrashResumeBanner.svelte`) as a frozen-frame chrome-cohort surface with `role="status"` + `aria-live="polite"`, mounted in `AppShell` between the DegradedBanner and the routed view; default-off (`crashResumeVisible = $state(false)`) mirrors the deferred-IPC reality, with a fixture payload (`Warehouse · Saturday`, p50=480ms, status chips `restored / restored / restored / partial / restored`) ready for the live `CrashResumeState` stream to flip on in Phase-3+/4a. The chrome-element cohort is now closed in terms of UI surface; the only outstanding work is the IPC wiring that flips the visibility flag from runtime state. Shared-primitive widget gaps closing: W36 PR #197 ported `SceneStill` (canvas-thumbnail widget with 16 procedural SceneKind renderers per the bundle's `shaders.jsx`), replacing the W18 PR1 hue-rotating gradient surrogate in SnapBar; **the remaining bundle primitives (RailIcon, SceneRow, MediaCard, Timecode, and the live-animated SceneCanvas sibling) landed in PR Tier-2 — `src/lib/widgets/{RailIcon,SceneRow,MediaCard,Timecode,SceneCanvas}.svelte` plus paired Vitest files. SceneRow + MediaCard replace the inline placeholder markup in `views/Perform.svelte` (the W18 PR1 hue-rotating surrogate is now retired across the scene rail and media strip alike, with both surfaces consuming the W36 `SceneStill` canvas via the new widgets). Timecode mounts in `TopBar.svelte` gated on the bundle's `PERF_VIEWS` set (perform / author / audio-uniforms) per the canonical `Chromaticity.html` adoption site (line 204), supplying the session-HUD timecode chip alongside the existing fps / cpu / REC indicators. SceneCanvas ships as the live-animated sibling to SceneStill and shares the `RENDERERS` map via `scene-renderers.ts` — wiring it into Perform / Cue preview heroes is a follow-up wave once the rendering backend (ADR-011) is decided so the synthetic-audio variant doesn't accidentally read as production output. RailIcon ships ready for adoption at the bundle's other rail-affordance sites (inspector / toolbar surfaces); the W12 `SidebarRail` keeps its route-aware button shape for now (its complexity exceeds the RailIcon primitive's contract).** The `mediaCardBlock` and `scene-row` snippets / inline rules in `Perform.svelte` are retired (CSS owned by the widget files now); the Perform view tests update their class-name + aria-current expectations to match the new widget surface (`media-card.active` instead of `media-card--active`, aria-current elided when inactive per the bundle's `aria-current={active || undefined}` idiom). Spectrum is already realised via SVG in `AudioUniforms.svelte` lines 235-255 and `AudioReactivity.svelte`, per the deliberate canvas→SVG divergence documented in those views' header comments. Vitest adoption depth widening: W37 PR #199 covered the W8 core visual primitives (Fader / Knob / Toggle / Swatch / ControlGroup, 35 new tests); W43 PR #213 added 29 tests for TestCard, DegradedBanner, OutputRoutingTile, and SnapBar, plus W39 / W40 / W44 added Meter (10) / TabSegment (7) / CrashResumeBanner (8) test coverage as their respective primitives landed. The Vitest backfill queue narrows from 11 widgets to **6** (AudioReactivity, BeatDots, BpmRing, CommandPalette, CrossFader, CuePad, VuStereo — the rAF-driven and pointer-drag widgets that need timer-mock plumbing); W45 PR #218 closed the rAF-driven half of the queue (AudioReactivity, BeatDots, BpmRing, VuStereo — 25 new tests; surfaced the jsdom matchMedia gotcha now memorialized in `feedback_vitest_matchmedia_stubglobal.md` — `vi.spyOn` doesn't work because jsdom doesn't pre-expose `matchMedia`, so the rAF widgets need `vi.stubGlobal('matchMedia', ...)`); W46 PR #217 closed the pointer/keyboard half of the queue (CommandPalette, CrossFader, CuePad — 24 new tests; caught a brief variance — the Cmd+K keydown listener actually lives in `AppShell.svelte` not `CommandPalette.svelte` — and discovered the jsdom needs `getBoundingClientRect` + `setPointerCapture` shims for the CrossFader pointer-drag harness). All 11 widgets now have Vitest coverage, closing the Vitest "widgets" track entirely. W47 PR #216 narrowed the shell-component queue with 24 tests for Icon (8) / EmptyState (9) / StatusBar (7); W48 PR #222 / W49 PR #221 / W50 PR #220 closed six more shells in a parallel-agent triple (HotkeyOverlay 10 + ThemeSwitcher 6 in W48, SidebarRail 9 + TopBar 9 in W49, NewSceneModal 9 + SceneDiff 9 in W50 — 52 new tests across 6 files), surfacing two non-obvious jsdom test patterns now memorialized: focus-trap assertions need `event.defaultPrevented` from a capture-phase listener (jsdom resets `document.activeElement` after `fireEvent.keyDown`, so in-handler `el.focus()` calls are not observable post-dispatch — `feedback_vitest_focus_trap_default_prevented.md`), and modal autofocus via `queueMicrotask` inside `$effect` requires `await Promise.resolve()` before the activeElement assertion (`feedback_vitest_microtask_autofocus.md`). W51 PR #224 closed AppShell with 14 tests covering always-on chrome landmarks, route dispatch (limited to non-IPC views), perform-gated SnapBar mount, DegradedBanner / CrashResumeBanner state, Cmd+K open / toggle / Ctrl+K / preventDefault / Shift-modifier-guard, WatermarkOverlay capability inversion, and keydown-listener cleanup on unmount — surfacing two harness refinements: (a) `capabilitiesStore` reset uses a captured seed snapshot at module load (mirrors routeStore's pattern), and (b) Cmd+K `preventDefault` assertion needs raw `KeyboardEvent` + `window.dispatchEvent` rather than `fireEvent.keyDown` because the Testing Library helper does not expose the synthesized event for `defaultPrevented` inspection. W52 PR #225 closed ConsoleDrawer with 15 tests covering open/close gating, fixture-corpus row count, autofocus, Pause/Live swap, document-level Escape, backdrop / Close-button dismissal, severity-chip filter, source-dropdown filter, search-substring narrowing, empty-state copy, Clear-filters reset, Shift+Tab focus-trap wrap, and Copy-button clipboard write — refining `feedback_vitest_microtask_autofocus.md` with the **focus-trap-after-autofocus** pattern (drain the autofocus microtask via `await Promise.resolve()` BEFORE manually moving focus, otherwise the queued autofocus resolves during the keydown await and steals focus back from the test's intended target). With AppShell + ConsoleDrawer landed, **the Vitest shell-component test track is fully closed** — every component file in `src/lib/*.svelte` now has a paired test file (Editor is a top-level CodeMirror 6 wrapper consumed by Author, covered directly by `src/lib/Editor.test.ts` per W63 PR). The W53/W54/W55/W56 four-agent parallel wave then opened the routed-view track in earnest: W53 PR #227 covered Welcome (8 tests, the simplest of the cohort — no microtask, no focus-trap, no `matchMedia`, no fixture imports); W54 PR #228 covered Import (10 tests, surfacing that exact-name role queries — `getByRole('textbox', { name: '<title> source' })` — are needed to disambiguate input-vs-Browse-button when sibling `aria-label` strings substring-match each other); W55 PR #230 covered Osc (12 tests, the first repo test to mock `@tauri-apps/api/core`'s `invoke` directly and surfacing that `vi.mock` factories must `vi.hoisted` any externally-named mock fns or fail at module-load with `ReferenceError: Cannot access 'invokeMock' before initialization` — pattern memorialised in `feedback_vitest_invoke_mock_hoisted.md` for the upcoming Library / Author / AudioUniforms / MIDI IPC-effect waves); W56 PR #229 covered Sources (13 tests; surfaced two real source/fixture defects flagged for AO follow-up: (a) `src/lib/stores/platform.ts`'s `currentPlatform` detector used `ua.toLowerCase().includes('win')` which matched `darwin` substring under jsdom and any non-Windows UA carrying a `darwin` token, misclassifying the platform as Windows and leaving Spout-on-mac rows incorrectly enabled — **closed by the post-W56 follow-up `fix(ui)` PR #235 which switched to a word-boundary regex `/\bwin/.test(ua)` and added `src/lib/stores/platform.test.ts` (5 unit tests covering jsdom darwin, Windows NT, macOS Safari, undefined navigator, and a synthetic darwin kernel-info UA) plus a Spout-disabled-on-mac integration test in `Sources.test.ts`**, and (b) `DEFAULT_ASSET_ID = jellyfish_loop_04.mp4` was fixture-orphaned because the asset-rotation generator emitted only `_00`/`_12`/`_24`, so the inspector rendered the empty state at first paint — **closed by a follow-up `fix(ui)` PR which special-cases `i=0` in `buildAssets()` to emit `_04` (instead of `_00`), so the bundle-canonical default selection resolves against `MEDIA_ASSETS` and the inspector mounts on first paint with the default-asset header + static `INSPECTOR_META` body table; the existing Sources view and `INSPECTOR_META` hardcoded six-row table are unchanged, and `Sources.test.ts` was updated to assert the positive default-paint state and to exercise selection change via a non-default click**). The routed-view track narrows from 10 of 14 remaining to **6 of 14 remaining** (Library, AudioUniforms, Perform, Author, Settings, MIDI — the IPC-heavy and largest views, deferred for solo-or-pair waves with the new `vi.hoisted` invoke-mock pattern as floor harness). The W57/W58/W59 three-agent parallel wave then cleared three of the six: W57 PR #234 covered Library (13 tests; first multi-`invoke` view test in the repo, fanning three Tauri commands — `default_scene_root`, `list_bundled_scene_packages`, `list_scene_packages` — through one mocked `invoke` via a per-test `dispatchInvoke({...})` helper, with `clearCache()` in both `beforeEach` and `afterEach` to reset the `src/lib/stores/library.ts` module-singleton cache between tests, and `waitFor(...)` rather than chained microtask awaits for the post-`finally` `aria-busy` flip because Svelte 5's scheduler defers rune updates past two ticks unreliably); W58 PR #232 covered AudioUniforms (15 tests; surfaced three setup gotchas — `<aside aria-label="Audio uniforms reference">` causes case-insensitive `getByText('Audio Uniforms')` to substring-match, requiring `.au__title` H2 scoping; `syn_BPMSin` appears as both a BPM-card name and a docs-token, requiring `within(grid)` scoping; and the view AND its embedded `BpmRing` / `BeatDots` widgets all call `matchMedia`, so the global `vi.stubGlobal` must be in place before the very first render); W59 PR #233 covered Perform (15 tests; Perform writes only local `$state` with no IPC and no store mutations, so no route/capability store reset is needed beyond the standard `cleanup()` plus `vi.unstubAllGlobals()` for the embedded `AudioReactivity` / `VuStereo` widgets' `prefers-reduced-motion` consumption — two name collisions required scoped queries, both DOM-shape and not bugs: panel rail `Edit` icon-button vs `.scene-panel__mode` mode-toggle, and `Glitch` appearing in both Scene Controls title and Preview hero label). The routed-view track narrows further from 6 to **3 of 14 remaining** (Author 1810 LoC, Settings 1925 LoC, MIDI 2014 LoC — the three largest views, each requiring solo agents in the next batch). The W60/W61/W62 three-agent solo wave then closed the remaining three: W60 PR #239 covered Author (20 tests; surfaced that CodeMirror 6 mounts cleanly under jsdom in this Vitest 4 + @testing-library/svelte stack — empirically the simple path works without any `vi.mock('@codemirror/...')` shim, contradicting the brief's pre-flight concern; the `matchMedia` stub is required by the embedded `VuStereo` preview-tools widget not Author itself, since `VuStereo` queries `prefers-reduced-motion` in a mount-time `$effect`); W61 PR #240 covered Settings (23 tests; surfaced five setup gotchas — `TabSegment` uses `role="tab"`/`aria-selected` not `role="button"`/`aria-pressed` so tests must disambiguate by role from the sibling pill toggles; `MIDI Clock Input` is both a `<label for=...>` and an `aria-label` so `getByLabelText` finds both elements, forcing `getByRole('button', { name: '...' })` for scoping; whitespace around the readout `{inputGainDbLabel}` breaks `getByText` exact-match so the slider's bundle-canonical `+3.2 dB` value is asserted via `aria-valuetext` + `.textContent?.trim()`; `AUDIO_INPUT_GAIN_DEFAULT = 0.59` produces the bundle's "+3.2 dB" readout under the `(gain - 0.5) * 36` mapping; and `ThemeSwitcher` writes `document.documentElement.dataset.theme` via a `$effect` so jsdom's `documentElement` persists across tests, requiring an explicit `delete document.documentElement.dataset.theme` BEFORE `cleanup()` in `afterEach`); W62 PR #238 covered MIDI (17 tests; the brief's IPC-heavy framing did not match reality — MIDI is a frozen-fixture port that imports purely from `src/lib/data/midi-fixtures.ts` with no `invoke` calls, so no `dispatchInvoke` helper or library-cache reset was needed; the `matchMedia` stub seeded with `matches: true` was used as a **deterministic kill-switch** for the learn-mode `$effect`'s rAF + setInterval pair, since the view explicitly short-circuits to the static fixture when reduced-motion is set, keeping every test jsdom-clean without `vi.useFakeTimers`). The W56 finding (b) — `DEFAULT_ASSET_ID = jellyfish_loop_04.mp4` fixture-orphan — was also closed by `fix(ui)` PR #237 which special-cases `i === 0` in `buildAssets()` to emit padded `_04` (instead of `_00`), so `MEDIA_ASSETS` keeps three `jellyfish_loop` entries (count unchanged for the existing `'jellyfish'` search test) but one of them now matches `DEFAULT_ASSET_ID`; the inspector mounts on first paint with the default-asset header + static `INSPECTOR_META` body table, and `Sources.test.ts` was updated to replace the empty-state-on-first-paint test with a positive default-paint assertion plus a tightened click-asset test that clicks `cells[1]` (a non-default asset) so it actually exercises selection change. With Author + Settings + MIDI landed and the W56 fixture-orphan closed, **the Vitest routed-view test track is fully closed** — every routed view file in `src/lib/views/*.svelte` now has a paired test file (Welcome, Import, Osc, Sources, Patch, License, Playlist, Inputs, Library, AudioUniforms, Perform, Author, Settings, MIDI = 14 of 14 covered). All three Vitest test-coverage tracks are now closed: widgets at W45/W46 (`src/lib/widgets/` — 22 primitives), shell components at W51/W52 (`src/lib/*.svelte`), and routed views at W60/W61/W62 (`src/lib/views/`). Both W56 findings — (a) `platform.ts` substring-match by PR #235 and (b) `DEFAULT_ASSET_ID` fixture-orphan by PR #237 — are now closed. Settings PARTIAL narrowing: W38 PR #201 added the bundle's right-rail Preview tile (consuming the W36 SceneStill widget at `kind=kaleido seed=7`) and Diagnostics panel (6 hardware-data rows alongside the existing Support Bundle / View Logs buttons), closing the most-visible Settings parity gap and converting the layout from single-column to the bundle-faithful two-column grid. W39 PR #203 narrowed the gap further by porting the **Audio Input Gain + Monitoring meter** sub-section — three new bundle-faithful rows in the Audio Device section: Channel Routing (paired L=1/3/5 + R=2/4/6 selects), Input Gain (range slider with `(gain - 0.5) * 36` ±18 dB symmetric mapping yielding "+3.2 dB" at the bundle's ~62% mark), and Monitoring (the bundle's `<Meter level={0.68} peak={0.82} bars={28} />` LED-segment-ladder snapshot). The W39 wave also landed the bundle's `Meter` widget primitive (`src/lib/widgets/Meter.svelte`) — distinct from the existing W23 PR2 `VuStereo` continuous-stripe — alongside ten Vitest cases exercising segment-count math, peak-marker placement, clip-threshold boundary (peak precedence over clip), default and overridden ARIA labels, and out-of-range level clamping. `DESIGN.md` token contract gained `components.meter.clip-color: '{colors.danger}'` so the rendered third segment state (clip = danger) is documented in the design contract. W40 PR #206 narrowed the gap further by porting the **Tempo & Sync extensions** sub-section — five new bundle-faithful rows after the W22 PR1 BPM-source-priority panel: Ableton Link toggle + cyan peer pill ("4 peers · 128.00 BPM"), Start/Stop Sync toggle, MIDI Clock Input toggle + device select, MIDI Clock Output toggle + device select, and Quantize Transitions tab-seg ("off / beat / bar / phrase", default "bar"). The wave also landed the bundle's `TabSegment` widget primitive (`src/lib/widgets/TabSegment.svelte`) — display-only segmented control with optional ArrowLeft/Right keyboard nav, scoped class name `tab-segment` to leave room for a future global stylesheet port — alongside seven Vitest cases. The TabSegment primitive is reusable for the upcoming W41 Graphics waves (Color Space + Multi-Display Mode tab-segs). W41 PR #208 narrowed the gap further by porting the **Graphics extensions** sub-section — six new bundle-faithful rows in Settings → Graphics/Output: Color Space tab-seg (sRGB / Rec.709 / Display P3 / Rec.2020, default sRGB), Multi-Display Mode tab-seg (single / mirror / span 2 / span 3, default single), Render Scale slider (0.5×–2.0× at 0.05 step, default 1.0×), Screen Fitting select (Fit Within Window / Fill Window crop / Stretch / 1:1 native), Visualizer Window action buttons (Show resizable / Toggle fullscreen, disabled pending Phase-3+/4a IPC), and FPS Slider toggle. Two of the six rows reuse the W40 `TabSegment` widget primitive, validating the W40 forward-looking note that TabSegment would unlock these ports without further widget-primitive work. Only one Settings sub-section gap remains: Live Sources INPUT toggles + Sender Name + Constant Output Resolution. **W42 PR #210 closed the final sub-section** by adding a new Live Sources article (between Graphics/Output and Storage) carrying five pill toggles (Spout/Syphon Input + Output, NDI Input + Output, Constant Output Resolution) plus the Sender Name text input (default "chrm" per the bundle wireframe). All five toggles carry `data-cap` attributes (`io.syphon.input`, `io.syphon.output`, `io.ndi.input`, `io.ndi.output`, `render.custom-resolution`) for the W15+ post-MVP entitlement infrastructure to discover without further markup changes. The Settings view is now fully bundle-parity; the W22 architect-agent scout "L-scope" Settings PARTIAL slip is closed end-to-end, and the routed-view tally moves to **14 of 14 DONE / 0 PARTIAL / 0 STUB**. Layout density: PR #212 fixed the Perform view's 1440×900 page-scroll regression — `.perform` gained `height: 100%`, the three column containers became grids with `grid-template-rows: auto auto minmax(0, 1fr)` row tracks (so the panel stack honours its constraints), `.panel` gained `min-height: 0` + `flex-shrink: 1`, and `AudioReactivity.svelte` swapped its `height: 100%` for `flex: 1; min-height: 0`. Per `feedback_layout_fits_viewport.md` the Perform-class views fit ~1440×900 without page-level scroll; only specific inner panels (META CONTROLS, MEDIA card grid, scenes list) scroll within themselves. Residual now covers CrashResumeBanner IPC wiring (the visual surface is in-tree per W44; the live `CrashResumeState` stream that flips visibility lands in Phase-3+/4a) and a tightened Vitest adoption-depth ledger: **all widget primitives covered** post W45/W46 + Tier-2 (the test-coverage track for `src/lib/widgets/` stays closed; the Tier-2 backfill adds RailIcon / SceneRow / MediaCard / Timecode / SceneCanvas with paired Vitest files, widening the widget total from 22 to 27); **all top-level shell components covered** post W51/W52 (the test-coverage track for `src/lib/*.svelte` is closed); **0 of 14 routed views** remain on the backfill queue post W60/W61/W62 (the test-coverage track for `src/lib/views/` is closed). The shared-primitive port slip clause is closed by the Tier-2 wave — RailIcon, SceneRow, MediaCard, Timecode, and the live-animated SceneCanvas sibling are all in-tree; the only forward-looking residual on the canvas/animated cohort is wiring SceneCanvas into Perform / Cue preview heroes once ADR-011 lands. All three Vitest test-coverage tracks — widgets (`src/lib/widgets/`), shell components (`src/lib/*.svelte`), and routed views (`src/lib/views/`) — are now closed end-to-end; the Vitest store-test-pair parity gap (`src/lib/stores/`) was closed by the post-Tier-2 follow-up wave with paired Vitest files for the seven stores that previously lacked them (`console`, `control`, `library`, `playlist`, `preset`, `route`, `snap`; +80 tests across seven new files using the `vi.hoisted` invoke-mock convention from `feedback_vitest_invoke_mock_hoisted.md` for the IPC-touching cases). A subsequent depth-deepening wave brought the MIDI view's coverage ratio from 0.84 to 2.58 tests/100 LoC (+35 tests covering the 7 tabs, 4 binding modes, 5 encoder modes, 16 snap slots, learn-mode panel internals, and BPM chain; no source/fixture defects surfaced). Further test additions widen depth (more cases per file) rather than breadth (more files). Suite total now 68 files / 657 tests. Phase-2 evidence-producing work has begun: slice 1 landed 2026-04-29 as the new `engine/render` crate (wgpu 26 + naga 26 + GLSL 4.50 → SPIR-V toolchain per ADR-011) with `compile_and_render_single_pass` + 4 T1 naga-frontend drift pins + 2 T2 deterministic UV-gradient corpus tests, advancing both `EV-RENDER-CONFORMANCE` (`docs/26` row 28) and the deferred R-33 wgpu/WebView2 composition spike from a Planned-with-no-artifact posture to a Planned-with-foundational-harness posture. Slice 2 (2026-04-29) extended this with multi-pass scheduling: a scene-agnostic `MultipassSpec` plain-data scheduler input (`engine/render/src/multipass.rs`) + `compile_and_render_multipass` entry point + 6 new T1 drift pins (empty pass list, forward-ref + self-ref input cycles, zero target size, scalar/vec3/int/uint uniform marshalling, empty uniform name) + 4 new T2 corpus fixtures (red→blue 2-pass, UV-attenuation 3-pass, single-pass uniform binding, single-element multipass consistency invariant) at `engine/render/tests/multipass.rs`; Apple Silicon Metal seeds pinned in three `ACCEPTED: &[u64]` slices with CI Linux/Vulkan + Windows/Vulkan variants reserved for follow-up commits if observed values differ. The `engine/scene::PassSpec` → `MultipassSpec` adapter, RGBA32F/RGBA16F float-target conformance, feedback-texture sampling, transition orchestration (ADR-007 warm-switch), audio-uniform binding, and DX12 backend re-enablement remain later-slice scope. The Lygia GLSL library trial vendor landed under `third_party/lygia/` (v1.1.4 snapshot at upstream `28f220b6`) under Prosperity-3.0.0 (SPDX `LicenseRef-Prosperity-3.0.0` because Prosperity-3.0.0 is not in the SPDX license list as of the 2026-04-29 snapshot date) per `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL Library Handling — final commercial-use license election deferred to Phase-5 `EV-LICENSE-AUDIT`; the trial-narrow `deny.toml` allow slot is documented in-comment, and a new `engine/render/src/glsl_includes.rs` line-scope `#include` preprocessor (8 T1 drift pins covering single/nested/relative-path resolution, missing-include errors, depth-cycle protection, and comment-not-treated-as-include defensives, plus 1 T2 end-to-end smoke through the full preprocessor → naga → wgpu pipeline) is the surface area on which Lygia consumption rides. The trial is tracked under `EV-LIBRARY-LICENSE-TRIAL` (`docs/26` row 49). Phase-2 also opened `EV-SCRIPT-CONTRACT`: slice 1 landed the foundational `engine/script` crate (rquickjs 0.11.0 binding to QuickJS, MIT) with sandbox-default module configuration (no `globalThis.fs`, no `globalThis.process`, no `globalThis.window`, synchronous `import` rejected), `ScriptRuntime::execute(source)` sync entry point, structured `ScriptError` enum under `#[non_exhaustive]`, `ScriptValue` round-trip for Null/Bool/Number/String/Array/Object, 9 T1 drift pins + 4 T2 contract fixtures (`fib(10)` determinism, array round-trip, object round-trip, multi-runtime isolation). Watchdog timeouts (slice 2 via `Runtime::set_interrupt_handler`), memory cap (slice 3 via `Runtime::set_memory_limit`), determinism shims (slice 4 — seeded `Math.random()`, frame-locked `Date.now()`), and command-bus integration (slice 5 — `engine/runtime` mount + `docs/36` reactive-symbol-catalog binding) remain later-slice scope; `EV-SANDBOX-SAFETY` opens with slice 2's watchdog. The Tier-1 follow-on wave (2026-04-29) landed engine/render slice 4 (feedback-texture sampling via per-pass ping-pong aliasing on the new `feedback_self: bool` `PassDef` field, RGBA32F/RGBA16F float-target conformance via the new `TargetFormat` enum, `engine/scene::PassSpec` → `MultipassSpec` adapter via the feature-gated `scene-adapter` Cargo feature with `MultipassSpec::from_scene[_with_options]` + 5-variant `AdapterError`, plus `MultipassSpec::frame_count` for multi-frame execution; 3 new `RenderError` variants `FeedbackAliasError` / `FloatTargetUnsupported` / `SceneAdapterError`; T2 corpus across `engine/render/tests/{feedback,float_targets,scene_adapter}.rs` with cross-backend ACCEPTED hash entries for both Apple Silicon Metal and Linux x86_64 Vulkan/Mesa-llvmpipe on the Rgba16Float fixture) and engine/script slice 2 (`with_watchdog(p99_ms, hard_ms)` builder via `rquickjs::Runtime::set_interrupt_handler`, new `engine/script/src/watchdog.rs` with `WatchdogConfig` + `Deadline` + `WatchdogObservation`, T2 sandbox corpus in `engine/script/tests/sandbox.rs` tolerant of CI clock variance via `[hard_ms, hard_ms + 200ms]` tolerance window, dual-tagged under `EV-SANDBOX-SAFETY` and `EV-SCRIPT-CONTRACT`); `EV-SANDBOX-SAFETY` (`docs/26` row 42) transitions from `pending:artifact` to slice-2 watchdog scaffold, bringing all four Phase-2 evidence rows (`EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, `EV-SANDBOX-SAFETY`, `EV-AUDIO-TRACE`) to a state where three carry foundational evidence. The Tier-2 follow-on wave (2026-04-29) closes the last `pending:artifact` Phase-2 row and widens sandbox-safety: engine/audio slice 1 (foundational uniforms pipeline at `engine/audio/src/uniforms.rs` per `docs/36-reactive-symbol-catalog.md` v1.1.0 — `AudioUniformSnapshot` plain-data shape carrying `bpm` + `rms` + `peak` + `fft_bands[8]`, `extract_uniform_snapshot(&AudioFrame)` sync entry point, `rustfft` 6 with default-features SIMD log-spaced FFT bands across `[20 Hz, 20 kHz]`, single-frame autocorrelation BPM with `BPM_DEFAULT = 120.0` fallback for sub-window frames; T1 drift pins covering RMS / peak / FFT band-edge math / default-zeroing; T2 corpus over silence + 440 Hz sine + seeded white noise + 1 kHz tone) opens `EV-AUDIO-TRACE` (`docs/26` row 30) from `pending:artifact` to slice-1 scaffold, with the cross-crate `engine/render::UniformInput` adapter and BPM history-window estimator explicitly deferred to slice 2; and engine/script slice 3 widens `EV-SANDBOX-SAFETY` with the memory-cap surface (`with_memory_limit(bytes)` builder via `rquickjs::Runtime::set_memory_limit`, 16 MiB heap default per ADR-012; new `engine/script/src/memory_cap.rs` with `MemoryCapConfig` + `MemoryUsageSnapshot` + `is_oom_error` predicate matching QuickJS's `InternalError: out of memory` shape; 4 T1 cap pins + 6 additional T1 pins in `runtime::execute_drift_pins`; 3 T2 cap-targeted fixtures appended to `engine/script/tests/sandbox.rs` dual-tagged with `EV-SCRIPT-CONTRACT`). With the Tier-2 wave landed, **all four Phase-2 evidence rows now carry foundational evidence**; the deferred-to-later-slice list at that point narrowed to the determinism shims (slice 4 — seeded `Math.random()`, frame-locked `Date.now()`), command-bus integration (slice 5), the cross-crate `AudioUniformSnapshot` → `UniformInput` adapter, transition orchestration (ADR-007 warm-switch), audio observability widening (slice 2 audio), DX12 backend re-enablement, and the cross-backend Windows Vulkan accept-set seed expansion. The Tier-3 follow-on wave (2026-04-29) landed engine/script slice 4 determinism shims and engine/audio slice 2 history-window BPM + ingest observability widening: engine/script slice 4 introduces the `with_random_seed(seed: u64)` and `with_clock(Box<dyn Clock + Send>)` builders chaining after slice-2 watchdog and slice-3 memory-cap, the new `engine/script/src/determinism.rs` module exposing the `Clock` trait + two built-in implementors `FixedClock` (per-frame replay primitive — every call within a single frame returns the same value) and `MonotonicClock` + the `RandomSeedConfig` newtype + the opaque host-side `DeterminismState` handle + a xoroshiro128+ PRNG seeded via `splitmix64` expansion, two new `ScriptError` variants `DeterminismConfig` / `DeterminismInstall` under `#[non_exhaustive]`, 9 T1 algorithm-drift pins in `determinism::determinism_pins` plus 6 T1 integration pins in `runtime::execute_drift_pins`, and a T2 corpus at `engine/script/tests/determinism.rs` evidence-tagged `EV-SCRIPT-CONTRACT` (8 tests covering seeded-random advances per call + replays across runtimes + diverges across seeds + state isolation per runtime, frame-locked clock returns tick exactly across 100 calls + accepts pre-epoch ticks); the shim install pattern uses rquickjs `Function::new` + `MutFn` once at builder time (no per-call install / uninstall bracket) with the closures capturing `Rc<RefCell<...>>` shares of host-side state surviving across `Context::with` boundaries, and slice 4 deliberately does NOT widen `EV-SANDBOX-SAFETY` because the determinism shims are not part of the sandbox isolation contract. engine/audio slice 2 introduces the state-carrying `BpmEstimator` median-smoothed history-window estimator (default 8-frame window via `BPM_HISTORY_DEFAULT_WINDOW`, configurable `1..=128`; ring-buffered per-frame autocorrelation reusing the slice-1 `compute_bpm_autocorr_for_frame` `pub(crate)` helper; cold-start partial-buffer median; the slice-1 single-frame `extract_uniform_snapshot` entry point STAYS UNCHANGED), the audio crate's first structured `AudioError` enum under `#[non_exhaustive]` with `WindowConfig { reason }` for `BpmEstimator::with_window(0)` rejection (and rejection above the 128-frame ceiling), and `engine/audio/src/observability.rs` widening with `IngestModeTracker` mirroring the §7.11 `IngestMode` (Normal/Degraded) transition counter via `ingest_mode_transition_count` plus an in-current-mode dwell counter via `ingest_mode_frames_in_current_count` (both routed through the existing `record_pull_outcome` lock-hold so every `try_pull` outcome accumulates mode-tracker state alongside the slice-1 frame / sample counters); slice-2 T1 drift pins in `bpm::bpm_drift_pins` cover window-validation surface (default constant, `with_window(0)` reject, `with_window(>128)` reject, ceiling acceptance, single-frame median, even/odd-length median math, reset preserves window), and the slice-2 T2 corpus at `engine/audio/tests/bpm_history.rs` exercises 60 / 80 / 90 BPM synthesised click-tracks at the canonical 48 kHz with ±5 BPM convergence tolerance over a 4-frame window plus cold-start partial-fill, multi-instance independence, mixed real/silent observation, buffered-saturates-at-window, and a slice-1 regression pin (the slice-1 corpus also gains a `slice_2_compatibility_extract_uniform_snapshot_unchanged_on_440_hz` regression pin against the slice-2 estimator addition silently drifting the slice-1 surface); the slice-2 corpus tempos are deliberately chosen in the lower half of the `[50, 220]` clamp range because slice-1 single-frame autocorrelation locks onto the half-tempo lag for BPMs ≥100 — an octave-error class slice-2 median smoothing by design cannot fix and which a Phase-3 fundamental-tracking estimator is later-slice scope to address. With the Tier-3 wave landed, three Phase-2 EV rows extend to deepened coverage: `EV-SCRIPT-CONTRACT` (`docs/26` row 29) cites slices 1+2+3+4; `EV-AUDIO-TRACE` (`docs/26` row 30) cites slices 1+2 with the cross-crate adapter still deferred; `EV-SANDBOX-SAFETY` (`docs/26` row 42) remains dual-tagged at slices 2+3 (slice 4 by design does not widen it). The deferred-to-later-slice list narrows further to engine/script slice 5 (command-bus integration to `engine/runtime` + `docs/36` reactive-symbol-catalog binding), the cross-crate `AudioUniformSnapshot` → `UniformInput` adapter (engine/render slice 5a), transition orchestration (ADR-007 warm-switch — engine/render slice 5b), engine/audio slice 3 (ingest-mode capability negotiation paired with the cpal device-capture integration), DX12 backend re-enablement, and the cross-backend Windows Vulkan accept-set seed expansion. Phase-0 closure tail: `EV-GATE-MODEL-AUDIT` (`docs/26` row 52) was promoted from `Planned` to `Pass` via the per-row checklist file `docs/EV-GATE-MODEL-AUDIT-checklist.md` (mirrors the EV-ARCH-CONTRACT-BASELINE / CLEANROOM / SYMBOL-CATALOG / ENTITLEMENT-DESIGN templates) — the audit-itself row closes; Phase-0 EXIT itself remains AO-held (the Phase-0 EXIT simulation in the new checklist reports 9-of-19 gate members met, with 8 UX-bundle rows AO-blocked on dev-team-queue clearance per `docs/43` §18.1, `EV-OBSERVABILITY-INVENTORY` Phase-0-deferrable on first-release artifact, and `EV-DOCS-PHASE0-SEED` + `EV-ENTITLEMENT-TOKEN-FORMAT` still `Planned`). The Tier-4 follow-on wave (2026-04-30) closes the cross-crate audio binding deferred since the Tier-3 wave and flips the second Phase-0 closure tail row: engine/render slice 5a introduces the feature-gated `audio-binding` Cargo feature with optional `chromaticity-audio` dep + new `engine/render/src/audio_adapter.rs` exposing `uniform_input_from_audio_snapshot(&AudioUniformSnapshot) -> UniformInput` per `docs/36-reactive-symbol-catalog.md` v1.1.0 — `bpm` / `rms` / `peak` emit four scalar `syn_BPM` / `chroma_AudioRMS` / `chroma_AudioPeak` `UniformValue::Scalar` bindings, and the 8-band `fft_bands` array splits into `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` `Vec4`s under the slice-2 multipass `UniformValue` enum constraint (no array variant yet — forward-compatible with the future `audio[]` widening called out at `engine/render/src/multipass.rs` lines 50-53; the four primary scalar names are stable across that widening), with the slice-2 multipass packing also forcing a `vec4 syn_BPM; ... syn_BPM.x` shader idiom that's documented at the test shader source + the adapter module-level prose + the slice 5b+ residue; 7 T1 algorithm-drift pins in `audio_adapter::audio_adapter_pins` + 3 T2 GPU-roundtrip fixtures in `engine/render/tests/audio_adapter.rs` dual-tagged under both `EV-RENDER-CONFORMANCE` (`docs/26` row 28) and `EV-AUDIO-TRACE` (`docs/26` row 30), backend-aware ACCEPTED set seeded with Apple Silicon Metal hashes only (Linux Vulkan / Windows seeds queued via separate `docs(repo)` commits per the `engine/render/tests/float_targets.rs` precedent); and `EV-ENTITLEMENT-TOKEN-FORMAT` (`docs/26` row 88) is promoted from `Planned` to `Pass` via the new `engine/entitlement` crate (`chromaticity-entitlement v0.0.0`, added to the workspace members) realizing the `entitlement-v2` schema from `adr/ADR-009-activation-and-entitlements.md` as `Token` / `Flags` / `MachineBinding` / `Session` / `Signature` serde-derive Rust types in `engine/entitlement/src/token.rs` with `#[serde(flatten)]`-flattened `BTreeMap<String, serde_json::Value>` extra-claims forward-compat (BTreeMap not HashMap so canonical CBOR re-encode is byte-identical regardless of hasher seed — the gate property the Phase-4a verifier needs), the MVP capability-namespace constants in `engine/entitlement/src/capability.rs` per ADR-009, and `engine/entitlement/tests/token_fixtures.rs` carrying 17 round-trip T2 fixtures (paid + trial + beta + enterprise flag combinations × machine_binding present/absent × `kid` rotation × canonical CBOR determinism gate test × forward-compat unknown-claim preservation across both serialization legs) plus 8 capability T1 drift pins evidence-tagged `EV-ENTITLEMENT-TOKEN-FORMAT`; signature verification, offline-expiry, revocation, and free-features validation remain Phase-4a scope (`EV-ENTITLEMENT-ENFORCEMENT` / `EV-ENTITLEMENT-OFFLINE` / `EV-ENTITLEMENT-REVOCATION` / `EV-ENTITLEMENT-FREE-FEATURES`) and are deliberately out of slice. With both Tier-4 closures landed the Phase-0 EXIT simulation advances to 10-of-19 gate members met (the remaining 9 gaps still enumerated honestly: 8 UX-bundle rows AO-blocked on dev-team-queue clearance per `docs/43` §18.1, `EV-OBSERVABILITY-INVENTORY` Phase-0-deferrable on first-release artifact, and `EV-DOCS-PHASE0-SEED` still `Planned`); the deferred-to-later-slice list narrows again to engine/script slice 5 (command-bus integration to `engine/runtime` + `docs/36` reactive-symbol-catalog binding), transition orchestration (ADR-007 warm-switch — engine/render slice 5b), engine/audio slice 3 (ingest-mode capability negotiation paired with the cpal device-capture integration), DX12 backend re-enablement, and the cross-backend Windows Vulkan accept-set seed expansion. The Tier-5 follow-on wave (2026-04-30) lands engine/script slice 5.1 (foundational command-bus mount) and engine/audio slice 3a (ingest-mode capability negotiation logic without cpal): engine/script slice 5.1 introduces the `ScriptCommandSender` trait as the cross-crate seam (engine/script does NOT take a `chromaticity-runtime` dep — the consumer-side adapter and dep direction are slice-5.2 scope), the `ScriptCommand::ReactiveWrite { name, value }` plain-data shape, the `ScriptRuntime::with_command_sender(sender)` builder chaining after slice-4's `with_clock`, 3 new `ScriptError` variants under `#[non_exhaustive]` for command-bus failure modes, 8 T1 algorithm-drift pins in `command_bus.rs` + 8 additional T1 integration pins in `runtime::execute_drift_pins`, and a T2 corpus at `engine/script/tests/command_bus.rs` (9 fixtures: end-to-end mutation → emit, multi-write ordering, error-path propagation, builder composition across all prior slices); the runtime crate is UNCHANGED — slice 5.1 establishes the producer-side seam without committing the dep direction. engine/audio slice 3a introduces the `IngestSource` enum (Synthetic / SilentFallback with a `#[non_exhaustive]` cpal placeholder for slice 3b), the `IngestNegotiator` builder claiming a primary source with fallback chain to silent on degraded, `IngestModeCriteria` exposing configurable demotion/promotion thresholds via `with_degraded_after` (default 2) / `with_normal_after` (default 1) — defaults match the slice-2 `IngestState` baseline driven by the `docs/06-audio-engine.md` Reliability/Fallback Policy two-consecutive-non-`Ready` rule, the audio crate's `AudioError::IngestNegotiation { reason }` variant under `#[non_exhaustive]`, 10 T1 algorithm-drift pins inline in `engine/audio/src/ingest_negotiation.rs::negotiation_pins`, and a T2 corpus at `engine/audio/tests/ingest_negotiation.rs` (transition behavior + demotion/promotion thresholds + fallback frame service). With slice 3a landed, the slice-2 `IngestModeTracker` counter is now driven by real transition criteria rather than staying always-Normal — the slice-2 partial-impl gap is closed. The deferred-to-later-slice list narrows further to engine/script slice 5.2 (reactive-symbol catalog binding from `syn_*` names to typed runtime command variants per `docs/36` v1.1.0), engine/render slice 5b (transition orchestration ADR-007 warm-switch), engine/audio slice 3b (cpal device-capture integration riding on the slice-3a negotiation surface), DX12 backend re-enablement, and the cross-backend Windows Vulkan accept-set seed expansion. A separate `docs(audio)` follow-up will reconcile a slice-2-author-introduced §7.11 reference drift surfaced during slice-3a implementation (the slice-2 `IngestModeTracker` prose cites `docs/06` §7.11 as the IngestMode normative source, but `docs/06` §7.11 is actually "Cadence, snapshot handoff, and staleness bound" — the actual normative source for the Normal/Degraded fall-back rule is the unnumbered "Reliability/Fallback Policy" section above §7; surgical correction across `engine/audio/src/{lib.rs,observability.rs,ingest_negotiation.rs,silent.rs}` and the `docs/26` row 30 slice-2 prose deferred to that follow-up to keep this docs delta focused on the wave-cadence cycle). The Tier-6 follow-on wave (2026-04-30) lands engine/script slice 5.2 (typed reactive-symbol-catalog binding), engine/render slice 5b.1 (transition lifecycle state machine per ADR-007 four-phase warm-switch first half), and `EV-DOCS-PHASE0-SEED` (the last actionable Phase-0 row — 4 user-facing onboarding stub pages closing the Phase-0 closure tail). Engine/script slice 5.2 introduces a new `engine/script/src/reactive_symbol.rs` module with `ReactiveSymbol` `#[non_exhaustive]` enum (53 variants spanning the script-host-visible subset of `docs/36` v1.1.0 — §1.1 frame/timing, §1.3 mouse, full §2 audio-uniform families including the canonical `syn_BPM` compat-syn-v1 retention plus the four chromaticity-native slice-5a primitives `chroma_AudioRMS` / `chroma_AudioPeak` / `chroma_AudioFFTLow` / `chroma_AudioFFTHigh`), `ReactiveValueKind` (Scalar / Vec2 / Vec3 / Vec4) dimensionality slots, `ReactiveSymbol::from_name` / `name` / `expected_kind` accessors, plus a `ScriptCommand::TypedReactiveWrite { symbol, value }` variant alongside the slice-5.1 catch-all `ReactiveWrite { name, value }`, and a producer-side `ScriptCommand::route` conversion. 2 new `ScriptError` variants under `#[non_exhaustive]` (`ReactiveSymbolUnknown`, `ReactiveValueKindMismatch`). The catalog stays engine/script-side (engine/script does NOT take a `chromaticity-runtime` dep — slice-5.3+ scope). Engine/render slice 5b.1 introduces the transition lifecycle state machine per ADR-007 four-phase warm-switch (`PrepareNext` / `PrimeNext` / `CrossfadeOrHardCut` / `CommitNext`) plus an `Idle` between-transitions state for accessor ergonomics; new `engine/render/src/transitions.rs` exposes `TransitionState`, `TransitionOrchestrator` (pure-data state machine — does NOT own a `wgpu::Device`; slice 5b.2 will plug in the GPU composition entry point), `TransitionError` with `InvalidStateTransition` / `SpecMismatch` / `ResourcePreallocFailed` (the latter declared in 5b.1 so 5b.2's GPU wiring is a non-breaking widening), and explicit phase-advance methods (`cue_next` / `mark_primed` / `begin_crossfade` / `commit_next` / `abort`); 12 T1 algorithm-drift pins inline in `transitions::transitions_pins` + 8 T2 lifecycle fixtures in `engine/render/tests/transitions.rs`. ADR-007's 500ms timeout + hard-cut fallback is deferred to slice 5b.2 since it requires the wall-clock-driving `CrossfadeOrHardCut` frame loop. `EV-DOCS-PHASE0-SEED` (`docs/26` row 66) is promoted from `Planned` to `Pass` via a new `docs/onboarding/` cluster (5 files, ~35 KB / ~750 lines): `01-intro.md` (chromaticity overview + what-you-can-do-today / what-you-cannot-yet lists), `02-scene-format.md` (`.chroma` JSON ref with minimal example), `03-standard-uniforms.md` (`docs/36` v1.1.0 summary for first-time authors), `04-first-scene-tutorial.md` (Simple Pulse hands-on walkthrough — single-pass fragment shader pulsing on `syn_Level` with 1 slider + 1 colour control), and `README.md` cluster index with explicit clean-room provenance note per `docs/29`. Hierarchical `docs/onboarding/` subdirectory layout (rather than claiming new flat numbers in the planning spine, reserved for normative planning rows) — `docs/adr/` is the established cluster precedent. Cross-cutting in this wave, a `refactor(runtime)` correction sweep renamed slice-5a's invented `syn_RMS` / `syn_PEAK` / `syn_FFT_LO` / `syn_FFT_HI` to chromaticity-native `chroma_AudioRMS` / `chroma_AudioPeak` / `chroma_AudioFFTLow` / `chroma_AudioFFTHigh` per `docs/36` Sec.Namespace governance (line 19: new feature work must NOT add fresh `syn_*` symbols as canonical names) — surfaced by Lane C's onboarding-page audit during this wave; the `syn_BPM` reference is preserved as a documented compat-syn-v1 retention (`docs/36` line 117); naga GLSL → SPIR-V is positional, not name-keyed, so the rename preserves the slice-5a Apple Silicon Metal ACCEPTED hashes without regeneration; Profile-Impact `native-chroma-v1-minor`. With both Tier-6 evidence-row closures landed plus the namespace correction, the Phase-0 EXIT simulation advances to **11-of-19 gate members met** (the remaining 8 gaps are all external-blocked: 8 UX-bundle rows AO-blocked on dev-team-queue clearance per `docs/43` §18.1, and `EV-OBSERVABILITY-INVENTORY` Phase-0-deferrable on first-release artifact). The deferred-to-later-slice list narrows further to engine/script slice 5.3+ (scene-author surface + engine/runtime adapter on the typed catalog contract), engine/render slice 5b.2 (crossfade composition + 500ms timeout + GPU resource preallocation wiring), engine/audio slice 3b (cpal device-capture integration on the slice-3a negotiation surface), DX12 backend re-enablement, and the cross-backend Windows Vulkan accept-set seed expansion. Phase-2 EXIT now requires deepening these foundational scaffolds toward EXIT-quality coverage rather than opening any new evidence row.
- Owner: AI UX Workcell + Accountable Owner.
