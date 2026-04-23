# Decision Ownership Matrix

This matrix defines decision authority for a solo accountable owner operating with AI workcells.

## Operating Roles
- **Accountable Owner (Human)**: Xavier Butler (single decision authority for scope, architecture, risk acceptance, and release gates).
- **Deputy Accountable Owner / Succession Fallback (Human)**: Fletcher Hammond. Holds standing authority for interim gate-unblocking decisions and non-legal waiver ratification when the Accountable Owner is unreachable beyond the succession threshold defined below. Does not hold scope-change authority unless the Accountable Owner is permanently unreachable (>14 consecutive calendar days with no return-of-availability signal), at which point the Deputy becomes Acting Accountable Owner pending a formal successor designation. Legal and brand gates remain non-delegable and do not pass to the Deputy.
- **AI Runtime Workcell**: runtime scheduler, render graph, sandbox, determinism.
- **AI Scene Workcell**: scene schema, normalization, compatibility fixtures, migration behavior.
- **AI Audio Workcell**: analysis pipeline, audio uniform contracts, cadence quality.
- **AI Integration Workcell**: OSC/MIDI/media adapters, reconnect behavior, route conformance.
- **AI QA/Ops Workcell**: evidence generation, SLO verification, regression gates, diagnostics quality.
- **AI UX Workcell**: wireframes, interaction contracts, keyboard+OSC/MIDI parity notes, operator flow diagrams, UX gate DoD authoring per `11-roadmap-phases.md`. Produces the seven UX contract evidence packets (`EV-UX-CONTROL-CONTRACT`, `EV-UX-FLOW-CONTRACT`, `EV-UX-HUD-CONTRACT`, `EV-UX-AUTHORING-CONTRACT`, `EV-UX-LIBRARY-CONTRACT`, `EV-UX-OSC-SETTINGS-CONTRACT`, `EV-UX-MIDI-SETTINGS-CONTRACT`) under WP-0. Final visual design language (color, type, iconography) remains out of scope per `01-charter.md`.
- **AI Docs Workcell**: embedded docs authoring, provenance tag maintenance, version-sync gate compliance. Produces the docs evidence packets (`EV-DOCS-CLEANROOM`, `EV-DOCS-VERSION-SYNC`, `EV-DOCS-COVERAGE`, `EV-DOCS-UX-LATENCY`) under WP-9 per `adr/ADR-010-embedded-docs-generation.md`.

## Decision Matrix

| Decision Area | Accountable | Responsible Workcell(s) | Required Review Evidence |
|---|---|---|---|
| Runtime model and scheduler | Accountable Owner | AI Runtime Workcell | ADR + `EV-DETERMINISM` |
| Render graph semantics | Accountable Owner | AI Runtime Workcell | ADR + `EV-RENDER-CONFORMANCE` |
| Scene schema and normalization | Accountable Owner | AI Scene Workcell | ADR + `EV-SCHEMA-LINT`, `EV-SCENE-IMPORT` |
| Script sandbox and host API policy | Accountable Owner | AI Runtime Workcell | ADR + `EV-SANDBOX-SAFETY`, `EV-SCRIPT-CONTRACT` |
| Audio contract and analysis model | Accountable Owner | AI Audio Workcell | ADR + `EV-AUDIO-TRACE`, `EV-PERF-BUDGETS` |
| OSC/MIDI/media contract policy | Accountable Owner | AI Integration Workcell | ADR + `EV-INTEGRATION-MATRIX` |
| Preset/playlist state semantics | Accountable Owner | AI Scene Workcell, AI Runtime Workcell | ADR + `EV-PRESET-PLAYLIST`, `EV-STATE-REPLAY` |
| Performance budgets and SLO targets | Accountable Owner | AI Runtime Workcell, AI Audio Workcell, AI QA/Ops Workcell | ADR + `EV-PERF-BUDGETS`, `EV-SOAK-CHAOS` |
| Compatibility profile/version changes | Accountable Owner | AI Scene Workcell, AI Integration Workcell | ADR + profile diff report + migration note |
| UX interaction contract authoring | Accountable Owner | AI UX Workcell | ADR/contract packet + `EV-UX-*` sign-off per UX gate DoD in `11-roadmap-phases.md` |
| Embedded documentation authoring | Accountable Owner | AI Docs Workcell | Provenance manifest entries + `EV-DOCS-CLEANROOM`/`EV-DOCS-VERSION-SYNC`/`EV-DOCS-COVERAGE` |
| Release gate sign-off | Accountable Owner | AI QA/Ops Workcell | Full evidence set from `26-evidence-artifact-registry.md` |
| Development standards change | Accountable Owner | AI QA/Ops Workcell (lint/CI wiring), AI Docs Workcell (authoring) | `43-development-standards.md` revision packet + `EV-COMMIT-CONVENTION-LINT`, `EV-LICENSE-HEADERS`, `EV-ADR-LINT`, `EV-EVIDENCE-TAG-COVERAGE`, `EV-SBOM-COVERAGE`, `EV-RELEASE-ATTESTATION`, `EV-COMPAT-PROFILE-DIFF` (as impacted). Material changes (versioning axes, commit scope vocabulary, signing requirements, license allowlist) require Independent Architecture Reviewer sign-off per `43-development-standards.md` Sec.22. |

## Mandatory Decision Rules
- Any architecture-impacting change requires an ADR before merge.
- Any architecture-impacting change requires at least one linked evidence artifact from `26-evidence-artifact-registry.md`.
- Any compatibility-breaking proposal requires explicit profile/version impact statement and migration path.
- Any missed critical SLO blocks release sign-off until remediation is verified or a waiver is approved.

## Waiver Process (Exception Path)
- Waivers are allowed only for time-bound release-risk decisions.
- Required waiver fields:
 - decision scope and impacted requirements,
 - risk acceptance statement,
 - temporary controls/rollback plan,
 - expiry date and re-validation trigger.
- Only the Accountable Owner can approve a waiver.
- No waiver can bypass legal/compliance gates (`EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`).

## Delegation and Independent Review Model
- Accountable ownership remains non-delegable for scope acceptance, gate approval, and waiver approval.
- Delegable review execution:
 - workcell-level technical validation may be delegated to designated reviewers.
 - evidence generation and pre-check approval may be delegated to AI QA/Ops Workcell.
- Independent checklist sign-off is required before Accountable Owner gate decision for:
 - architecture changes (`EV-ARCH-CONTRACT-BASELINE`),
 - cumulative release gate (`EV-GATE-MODEL-AUDIT`),
 - legal and brand gates (`EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`, `EV-BRAND-REVIEW`).

### Named Independent Reviewer Roles
Independent review cannot be performed by the same AI workcell that produced the artifact under review, and cannot be the Accountable Owner. The following external reviewer roles are designated; each role must be filled by a named human before Phase 0 exit.

| Reviewer role | Primary assignee | Deputy / backup | Scope | Required before |
|---|---|---|---|---|
| Independent Architecture Reviewer | Xavier Butler | Fletcher Hammond | `EV-ARCH-CONTRACT-BASELINE`, all ADRs | Phase 0 exit |
| Independent Release QA Reviewer | Xavier Butler | Fletcher Hammond | `EV-GATE-MODEL-AUDIT`, cumulative release gate | Phase 5 / release candidate |
| Independent Legal Reviewer | Xavier Butler | External contracted counsel (engaged per-gate; roster entry required before first legal-gate closure) | `EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`, `EV-BRAND-REVIEW`, `EV-CLEANROOM-POLICY-BASELINE` | Phase 0 (policy baseline) and Phase 5 (audit) |
| Independent Docs Clean-Room Reviewer | Xavier Butler | Fletcher Hammond | per-page `EV-DOCS-CLEANROOM` sign-off | Each release cycle |

Primary/Deputy assignment rationale: the Accountable Owner (Xavier Butler) is named Primary on three of four non-legal reviewer roles because the AO supervises the AI workcells that produce most gate artifacts and has the highest-resolution view of each gate's acceptance criteria. The independence mechanism that satisfies the "cannot be performed by the same AI workcell that produced the artifact under review, and cannot be the Accountable Owner" rule above in this configuration is the Deputy-performs-review clause in the paragraph immediately below: when the Primary produced (or supervised production of) the artifact under review, the Deputy performs the review. In practice this means Fletcher Hammond conducts independent review for AO-produced or AO-supervised architecture, release-QA, and docs-clean-room artifacts, with Xavier acting as Primary reviewer for all other review traffic. Legal-review independence is tracked separately via the contracted-counsel engagement row and does not fall to the Deputy.

Reviewer assignments, contact details, and conflict-of-interest declarations live in an internal "Reviewer Roster" document referenced by `EV-ARCH-CONTRACT-BASELINE`. A vacant reviewer role blocks the gate it covers. When the primary assignee produced the artifact under review, the Deputy performs the review to preserve independence. Legal gates require either the Primary or contracted counsel; they do not fall to the Deputy.

Given current team composition (two humans), independent-review role separation is partial: the primary and deputy assignments above satisfy role separation for all non-legal gates. Legal gate separation requires the contracted counsel engagement and is tracked as a residual condition on `EV-LEGAL-PROVENANCE`. Any gate closed with Primary-only sign-off during a Deputy-unavailable window must carry an explicit waiver record in the gate packet.


### Accountable Owner Succession
Succession is a three-tier ladder. Each threshold is measured from the last confirmed return-of-availability signal (commit, review comment, acknowledged message, or scheduled check-in).

| Tier | Threshold | Authority transferred | Scope boundary |
|---|---|---|---|
| T0 | Accountable Owner available | Xavier Butler holds full authority | All decisions |
| T1 | AO unreachable >72 hours during an active phase/review window | Deputy Accountable Owner (Fletcher Hammond) assumes **interim** authority | Gate-level unblocking, SLA-breach escalations, non-legal waiver ratification with 5-business-day re-ratification requirement. **No** scope or architecture changes. **No** legal, brand, or license-audit sign-off. **No** edits to `chromaticity-planning/` or `adr/` under CODEOWNERS (per `43-development-standards.md` Sec.8.1 + Sec.7.6). **UX contract evidence rows** (`EV-UX-*`, `EV-HOTKEY-PARITY-DESIGN`) may be held by the Deputy at `InProgress` but transitions to `Pass` require AO return (or T2 escalation); this follows from the "interim, unblocking" scope rule and from the non-delegable gate-approval rule below. |
| T2 | AO unreachable >14 consecutive calendar days with no return signal | Deputy becomes **Acting** Accountable Owner | Full gate authority including scope and architecture, except legal/brand gates which still require contracted counsel. Acting status is logged and ratified by either the AO on return or by a documented successor-designation process. |
| T3 | AO permanently unreachable (fatality, incapacity, or written resignation) | Deputy becomes Accountable Owner of record | Full AO authority; Deputy names a new Deputy within 30 days. |

- Full scope/waiver authority returns to the primary Accountable Owner on availability at T1 and T2; interim decisions are logged and re-ratified within 5 business days of return. T3 transitions are irreversible.
- The Deputy operates under the same ADR + evidence requirements as the primary AO; interim/acting decisions still require ADRs and evidence artifacts, not verbal approvals.
- Succession escalation path, contact details, and the canonical return-of-availability signal definitions are maintained in the Reviewer Roster document referenced above. Risk `R-19` tracks availability gaps; with the Deputy slot filled, the residual probability of a fully-unmitigated AO SPOF drops from Medium to Low.

## Approval SLA Targets
- Architecture-impacting ADR review: 2 business days maximum.
- Phase gate decision after evidence packet ready: 1 business day maximum.
- Release gate decision after full evidence set ready: 2 business days maximum.
- SLA breaches require explicit escalation note and revised target timestamp.
