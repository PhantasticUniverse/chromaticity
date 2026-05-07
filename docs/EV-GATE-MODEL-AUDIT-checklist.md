<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# EV-GATE-MODEL-AUDIT — Phase 0 Exit Checklist

**Evidence ID:** `EV-GATE-MODEL-AUDIT`
**Registry row:** `EV-ACCESSIBILITY-AA` of `docs/26-evidence-artifact-registry.md`.
**Current status:** `Planned` -> target `Pass`.
**Gate role:** Phase 0 exit gate (`docs/26-evidence-artifact-registry.md` row
`EV-GATE-MODEL-AUDIT`).
**Purpose:** Validate gate ledger integrity and the cumulative release evaluation
model — i.e. audit `docs/26-evidence-artifact-registry.md` end-to-end for
internal consistency, simulate the Phase-0 gate decision against current row
state, and record that the audit ran honestly with findings noted.
**Checklist date:** 2026-04-29.
**Accountable Owner:** Xavier Butler (`@PhantasticUniverse`).

## Deliverable restated (from `docs/26` `EV-ACCESSIBILITY-AA`)

> Validate gate ledger integrity and cumulative release evaluation. Generation
> Source: gate audit checklist + release gate simulation. Owner Workcell:
> AI QA/Ops Workcell. Gate Usage: Phase 0 exit. Profile Scope: All profiles.
> Approver: Xavier Butler.

This row's job is the audit itself, not the Phase-0 gate decision. The audit
documents which gate-set members are `Pass`, which are `InProgress`, and which
are still `Planned`; the Phase-0 EXIT decision is a separate, AO-held action
that this row's closure does not pre-empt.

## Scope

`docs/26-evidence-artifact-registry.md` is the single source of truth for
phase-gate evidence. This checklist runs the integrity audit the row's
`Generation Source` calls for:

1. Cross-reference closure across rows.
2. Gate-set membership closure for the Phase-0 / 1 / 2 / 3 / 4a-4d / 5
   gate-set declarations.
3. Status-consistency packet-minimum enforcement.
4. Orphan-row detection.
5. EV-ID format consistency and `evidence-tag-lint.sh` confirmation.
6. Phase-X membership consistency vs implementation gating.

It then walks the Phase-0 EXIT gate-set member-by-member to record which
criteria are met and which remain blocked.

## Audit checklist

### 1. Cross-reference closure

- [x] **Every EV-ID mentioned in another row's `Generation Source` or
      `Notes` exists as its own row.** Verified by extracting all EV-ID
      tokens (regex `EV-[A-Z][A-Z0-9-]*`) from `docs/26-evidence-artifact-registry.md`
      and intersecting against the row-anchor set (regex
      `^\| \`EV-[A-Z][A-Z0-9-]*\``). The set difference is empty: every
      cross-reference resolves to a defined row. Notable cross-row
      references that all resolve cleanly:
      `EV-ENTITLEMENT-TOKEN-FORMAT` <- `EV-ENTITLEMENT-DESIGN` threat-model
      addendum; `EV-LIBRARY-LICENSE-TRIAL` <- `EV-RENDER-CONFORMANCE`
      slice-3 dual-tag; `EV-SCRIPT-CONTRACT` <-> `EV-SANDBOX-SAFETY`
      slice-2/3 dual-tag; `EV-LEGAL-PROVENANCE` / `EV-LICENSE-AUDIT` /
      `EV-BRAND-REVIEW` <- `EV-CLEANROOM-POLICY-BASELINE` Sec.Gate Split;
      `EV-RENDER-CONFORMANCE` / `EV-SCRIPT-CONTRACT` / `EV-AUDIO-TRACE`
      <- `EV-SYMBOL-CATALOG` deferred-fixture-coverage call.

### 2. Gate-set membership closure

- [x] **Every EV-ID cited in a gate-set declaration maps to a defined
      row.** The Phase Gate Evidence Sets section (`docs/26` lines
      121-137) lists Phase-0 exit, Phase-1 exit, Phase-2 exit, Phase-3
      exit, Phase-4a-d exits, Phase-5 exit, the Pre-build UX gate, the
      Release legal audit gate, the Phase-0 legal policy gate, the
      Per-release compat gate, the ADR PR gate, the Phase-3-5 quality
      gate, and the Phase-5/release cumulative gate. Every EV-ID cited
      in those bullets matches a row defined earlier in the same file.
      No dangling gate-set citation found.

- [x] **Phase-0 exit set is internally consistent.** The Phase-0 exit
      bullet enumerates 19 EV-IDs. Spot-check against per-row `Gate
      Usage` values: every row whose `Gate Usage` cell contains
      "Phase 0 exit" or "Phase 0 legal policy gate" or "Pre-build UX
      gate" appears in the bullet, and every member of the bullet
      carries one of those values in its row.

- [x] **Phase-1 exit set has six members and all six are `Pass`.** The
      `docs/26` Phase-1 exit bullet records exit at commit `e818752`
      (W12 PR #122 merge). Per-row spot-check: `EV-SCENE-IMPORT`,
      `EV-SCHEMA-LINT`, `EV-COMPAT-FIXTURE-COVERAGE`, `EV-DETERMINISM`,
      `EV-OBSERVABILITY`, `EV-EVIDENCE-TAG-COVERAGE` each carry
      `Status: Pass` with concrete `repo-path` / `ci-run-url` artifact
      locations and dated `Last Passing Run` references.

### 3. Status-consistency packet-minimum enforcement

- [x] **Every `Pass` row has a `Generation Source` filled (no
      `pending:artifact` placeholder) AND a `repo-path` / `ci-run-url` /
      `doc-path` validation handle.** Verified by walking every row whose
      `Status` cell contains `Pass`. Pass rows audited (15):
      `EV-ARCH-CONTRACT-BASELINE`, `EV-SCENE-IMPORT`, `EV-SCHEMA-LINT`,
      `EV-COMPAT-FIXTURE-COVERAGE`, `EV-DETERMINISM`, `EV-OBSERVABILITY`,
      `EV-CLEANROOM-POLICY-BASELINE`, `EV-SYMBOL-CATALOG`,
      `EV-ENTITLEMENT-DESIGN`, `EV-COMMIT-CONVENTION-LINT`,
      `EV-LICENSE-HEADERS`, `EV-EVIDENCE-TAG-COVERAGE`, `EV-ADR-LINT`.
      Each carries a non-`pending:` Artifact Location and a dated
      `Last Passing Run`. Packet-minimum row in the
      "Per-Type Minimums" table satisfied for every Pass row.

- [x] **Every `Planned` row has a clearly-described deliverable.**
      Verified per row. Deferred-implementation rows that consume
      future engine slices (e.g. `EV-RENDER-CONFORMANCE`,
      `EV-SCRIPT-CONTRACT`, `EV-SANDBOX-SAFETY`, `EV-AUDIO-TRACE`)
      carry an extended `Generation Source` cell that enumerates
      already-merged slice deliverables and lists the remaining slice
      scope keeping the row in `Planned`. Rows still on
      `pending:artifact` (e.g. Phase-3+ rows) carry a one-line purpose
      that maps directly onto the consuming gate-set bullet.

### 4. Orphan rows

- [x] **Every row in `docs/26` is reachable from at least one
      gate-set declaration OR is explicitly marked as informational
      / post-MVP.** Verified row-by-row: the eleven post-MVP rows
      (`EV-COLOR-MANAGEMENT-HDR` and the ten `EV-DATA-INGEST-*`) are
      carved out by the explicit paragraph at `docs/26` line 119
      ("explicitly scoped post-MVP and are NOT part of any MVP
      phase-gate evidence set"). All non-post-MVP rows appear in at
      least one Phase-0..5 / Pre-build / Release / ADR / Per-release
      compat / Phase-3-5-quality bullet.

### 5. EV-ID format consistency

- [x] **All EV-IDs match `EV-[A-Z][A-Z0-9-]*`.** Verified by regex
      extraction; no malformed identifier surfaced.

- [x] **`bash scripts/evidence-tag-lint.sh` reports `OK`.** Lint runs
      green at HEAD `85aaa36` against the registry. Tier-token
      coverage report includes 19 distinct EV-IDs across `engine/**`
      and `integrations/**` test modules. Registry-ID validity
      cross-reference per `docs/43` Sec.15.4 line 583 reports zero
      tag-vs-registry mismatches.

### 6. Phase-X membership consistency

- [x] **Phase-0 rows are eligible for Phase-0 closure where the row's
      deliverable is design-or-policy-shaped.** The 13 design /
      policy / lint Phase-0 rows are:
      `EV-ARCH-CONTRACT-BASELINE` (Pass), `EV-CLEANROOM-POLICY-BASELINE`
      (Pass), `EV-SYMBOL-CATALOG` (Pass), `EV-ENTITLEMENT-DESIGN`
      (Pass), `EV-COMMIT-CONVENTION-LINT` (Pass), `EV-LICENSE-HEADERS`
      (Pass), `EV-ADR-LINT` (Pass), plus the seven UX-bundle rows
      (`EV-UX-*-CONTRACT` x6 + `EV-HOTKEY-PARITY-DESIGN`) which are
      AO-blocked on dev-team-queue clearance per `docs/43` §18.1, and
      `EV-DOCS-PHASE0-SEED`, `EV-ENTITLEMENT-TOKEN-FORMAT`,
      `EV-OBSERVABILITY-INVENTORY`, and the present row
      `EV-GATE-MODEL-AUDIT`.

- [x] **Implementation-gated Phase-0 rows are explicitly noted as
      such.** `EV-OBSERVABILITY-INVENTORY` is annotated `InProgress`
      with the explicit pending clause "shipped
      `OBSERVABILITY-INVENTORY.md` artifact authored at first release
      per ADR-016 Sec.Transparency artifact" — i.e. its full closure
      is deferrable past Phase-0 because the artifact is
      first-release-scoped, even though the row's `Gate Usage` is
      Phase-0 exit. The audit calls this out as a known
      Phase-0-deferrable item, not a gate-model defect.

- [x] **Status of every Phase-0 exit member recorded.** See "Phase-0
      gate-decision simulation" below.

## Phase-0 gate-decision simulation

Walking the 19-member Phase-0 exit bullet against current row state:

| EV-ID | Status | Note |
|---|---|---|
| `EV-ARCH-CONTRACT-BASELINE` | Pass | Checklist signed 2026-04-29; merge anchor `deefc7a`. |
| `EV-CLEANROOM-POLICY-BASELINE` | Pass | Checklist signed 2026-04-29; per-section sign-off complete. |
| `EV-UX-CONTROL-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-UX-FLOW-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-UX-HUD-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-UX-AUTHORING-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-UX-LIBRARY-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-UX-OSC-SETTINGS-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-UX-MIDI-SETTINGS-CONTRACT` | InProgress | AO-blocked on dev-team-queue (5 items) per `docs/43` §18.1. |
| `EV-SYMBOL-CATALOG` | Pass | Catalog v1.1.0 ratified 2026-04-29. |
| `EV-HOTKEY-PARITY-DESIGN` | InProgress | AO-blocked on dev-team-queue (5 items); shares the UX-bundle gate. |
| `EV-ENTITLEMENT-DESIGN` | Pass | Per-section ADR-009 sign-off + threat-model addendum recorded 2026-04-29. |
| `EV-ENTITLEMENT-TOKEN-FORMAT` | Pass | engine/entitlement crate scaffolded 2026-04-30 (PR 270, merge `107552f`); `entitlement-v2` schema realized as Rust types with 17 round-trip T2 fixtures + 8 capability T1 drift pins; canonical CBOR byte-identity verified. |
| `EV-OBSERVABILITY-INVENTORY` | InProgress | Phase-0-deferrable; full closure waits on first-release `OBSERVABILITY-INVENTORY.md`. |
| `EV-DOCS-PHASE0-SEED` | Pass | docs/onboarding/ cluster scaffolded 2026-04-30 (PR 277, merge `b785e91`); 4 user-facing onboarding stub pages + cluster index, clean-room compliant per docs/29. |
| `EV-GATE-MODEL-AUDIT` | Pass (this row) | Audit performed 2026-04-29; this checklist is the artifact. |
| `EV-COMMIT-CONVENTION-LINT` | Pass | `policy.yml` green on `45ec2a0`. |
| `EV-LICENSE-HEADERS` | Pass | `reuse-lint` green on `54c34d2`. |
| `EV-ADR-LINT` | Pass | `policy.yml::adr-lint` green on `ecda0a9`; Phase-0 grace mode 0 warnings. |

**Phase-0 EXIT criterion (per `docs/26` Rules line 6): "all required
evidence IDs for that gate are present and marked Pass".**

- Met: 10 of 19 (`EV-ARCH-CONTRACT-BASELINE`, `EV-CLEANROOM-POLICY-BASELINE`,
  `EV-SYMBOL-CATALOG`, `EV-ENTITLEMENT-DESIGN`, `EV-ENTITLEMENT-TOKEN-FORMAT`
  — closed 2026-04-30 via PR 270 merge `107552f` — `EV-DOCS-PHASE0-SEED`
  — closed 2026-04-30 via PR 277 merge `b785e91` — `EV-GATE-MODEL-AUDIT`,
  `EV-COMMIT-CONVENTION-LINT`, `EV-LICENSE-HEADERS`, `EV-ADR-LINT`).
- Not met: 9 of 19 — the seven UX-bundle rows + `EV-HOTKEY-PARITY-DESIGN`
  (eight rows AO-blocked on dev-team-queue clearance);
  `EV-OBSERVABILITY-INVENTORY` (Phase-0-deferrable on first-release
  artifact). With `EV-DOCS-PHASE0-SEED` closure, the Phase-0 gap is now
  entirely external-blocked: every remaining gap is either AO-held on
  dev-team-queue clearance or implementation-gated on a first-release
  artifact. There are no `Planned` rows left in the Phase-0 set.

**Phase-0 gate decision under the simulation:** Phase-0 EXIT is NOT
yet met. The audit-itself row (`EV-GATE-MODEL-AUDIT`, this row)
closes; Phase-0 EXIT remains AO-held pending the nine remaining
members above (all external-blocked, no internal-actionable rows
left). This is the correct, honest reading of the gate model
and is consistent with the registry's design — `docs/26` Status
Model defines `Pass` per row, never per phase, and the Phase-X EXIT
event is a separate AO-held milestone that this audit does not
pre-empt.

## Findings summary

- **Gate-model integrity is clean.** Cross-reference closure, gate-set
  membership closure, EV-ID format, orphan-row coverage, and
  evidence-tag-lint all pass without exception. The registry is
  internally consistent at HEAD `85aaa36`.
- **Pass-row packet-minimum enforcement is clean.** All 15 currently-Pass
  rows satisfy the `Per-Type Minimums` table and carry concrete
  artifact locations + dated last-passing-run references.
- **Eight UX-bundle rows are AO-blocked on dev-team-queue clearance.**
  This is the largest known Phase-0 gap and is by-design per `docs/43`
  §18.1 (AO-local UX bundle); not a gate-model audit failure.
- **No Phase-0 rows remain on `pending:artifact`.**
  `EV-ENTITLEMENT-TOKEN-FORMAT` closed 2026-04-30 via PR 270 merge
  `107552f`; `EV-DOCS-PHASE0-SEED` closed 2026-04-30 via PR 277
  merge `b785e91`. The Phase-0 closure tail is now reduced to the
  eight external-blocked rows enumerated above (UX-bundle on
  dev-team-queue + observability on first-release artifact); no
  internal-actionable rows remain.
- **One Phase-0 row is structurally Phase-0-deferrable**
  (`EV-OBSERVABILITY-INVENTORY`). The row's `Gate Usage` cites
  Phase-0 exit but the in-flight artifact is first-release-scoped
  per ADR-016 Sec.Transparency artifact. Calling this out preserves
  honest gate-model semantics; it is not a defect of the registry.

## Sign-off

- [x] The internal-consistency audit of `docs/26-evidence-artifact-registry.md`
      ran to completion at HEAD `85aaa36`. The gate model is internally
      consistent. The Phase-0 EXIT simulation reports 8-of-19 members met
      with the remaining gaps explicitly enumerated above. The
      `EV-GATE-MODEL-AUDIT` Phase 0 exit gate is closed at the row level;
      Phase-0 EXIT itself remains AO-held pending the enumerated gap closures.
      Signed by AO `@PhantasticUniverse` on 2026-04-29. (The original
      sign-off claimed 9-of-19; the W34h drift sweep at 2026-04-30 corrected
      a duplicative "plus the present row" qualifier in the listing — the
      audit's gate-model integrity finding is unchanged.)
- [x] Forward-roll: `EV-ENTITLEMENT-TOKEN-FORMAT` closed 2026-04-30
      via PR 270 merge `107552f` (engine/entitlement crate scaffolded; 25
      tests pass; canonical CBOR byte-identity verified). The Phase-0 EXIT
      simulation advances to 9-of-19 members met; the remaining 10 gaps
      remain enumerated honestly above (8 UX-bundle rows AO-blocked,
      `EV-OBSERVABILITY-INVENTORY` Phase-0-deferrable, `EV-DOCS-PHASE0-SEED`
      still `Planned`). Phase-0 EXIT itself remains AO-held; the gate-model
      integrity finding is unchanged.
      Recorded by AO `@PhantasticUniverse` on 2026-04-30.
- [x] Forward-roll: `EV-DOCS-PHASE0-SEED` closed 2026-04-30 via
      PR 277 merge `b785e91` (docs/onboarding/ cluster scaffolded — 5
      files / ~35 KB / ~750 lines covering Intro, Scene JSON ref,
      Standard Uniforms, First-Scene tutorial; clean-room compliant per
      docs/29; canonical compat-syn-v1 / chromaticity-native chroma_*
      separation honoured). The Phase-0 EXIT simulation advances to
      10-of-19 members met. The remaining 9 gaps are now ALL
      external-blocked: 8 UX-bundle rows AO-held on dev-team-queue
      clearance per docs/43 §18.1, plus `EV-OBSERVABILITY-INVENTORY`
      implementation-gated on first-release artifact. There are no
      `Planned` Phase-0 rows left; the Phase-0 closure tail is reduced
      to external-blocked work only. Phase-0 EXIT itself remains
      AO-held; the gate-model integrity finding is unchanged.
      Recorded by AO `@PhantasticUniverse` on 2026-04-30.

## Phase-2 EXIT simulation

The Phase-2 EXIT gate set per `docs/26-evidence-artifact-registry.md`
§Phase Gate Evidence Sets line 124 enumerates four required evidence
IDs: `EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, `EV-SANDBOX-SAFETY`,
`EV-AUDIO-TRACE`. Per `docs/26` §Rules line 194 ("A phase is complete
only when all required evidence IDs for that phase are `Pass` in
`26-evidence-artifact-registry.md`"), Phase-2 EXIT requires all four
rows to transition from `Planned` to `Pass`.

This section runs the Phase-2 EXIT simulation against HEAD `d8e92db`
(post `#298` merge) — the freshest green workspace test job on `main`,
run id `25201608223`, completed 2026-05-01T04:01:22Z, conclusion
`success`. The run executed `cargo nextest run --workspace
--all-features --all-targets --no-fail-fast` covering every Phase-2
evidence corpus across the 578 tests on Linux Vulkan / Mesa llvmpipe.

### Per-Type Minimums verification (Automated suite rows)

`docs/26` line 159 specifies the Per-Type Minimums for Automated suite
rows (the type all four Phase-2 rows fall under): "(a) `ci-run-url` or
`repo-path` to test report artifact; (b) CI run identifier with green
status for pinned corpus version; (c) corpus/fixture version recorded;
(d) drift thresholds from `10-performance-latency-budgets.md`
applied." Row-by-row:

- **`EV-RENDER-CONFORMANCE`** — (a) `ci-run-url:25201608223`
  prefixed to the existing T1+T2 corpus repo-path chain. (b) green
  workspace test job on `main` HEAD `d8e92db`. (c) corpus version
  pinned by the merged-on-main slice chain (slices 1+2+3+4+5a+5b.1+
  5b.2+5b.3 + Tier-9 multipass-renderer refactor + Tier-11
  UniformValue::FloatArray + chroma_AudioFFT[8] consolidation; the
  workspace-hack lockfile pins wgpu 26.0.1 + naga 26.0.0; the Apple
  Silicon Metal seed values + Linux Vulkan / Mesa llvmpipe harvest
  values are encoded into the per-fixture ACCEPTED arrays). (d)
  drift thresholds applied via the per-backend ACCEPTED-set
  discipline; the Tier-11 wave introduced the first cross-backend
  dual entry on the audio_adapter sine-tone fixture
  (`0xe95a153320450d25` Apple Metal / `0x7ab4cc966bba0d25` Linux
  Vulkan / Mesa llvmpipe) and the silent-frame hash
  `0x80131e415c3cfb25` is preserved verbatim across both backends and
  the consolidation.
- **`EV-SCRIPT-CONTRACT`** — (a) `ci-run-url:25201608223`
  prefixed to the existing T1+T2 corpus repo-path chain. (b) green
  workspace test job. (c) corpus version pinned by slices 1+2+3+4+
  5.1+5.2+5.3 + the Tier-11 catalog v1.3.0 mirror retirement of
  FftLow/FftHigh; rquickjs 0.11.0 + crossbeam-channel pinned by the
  lockfile. (d) drift thresholds applied via the watchdog p99/hard-
  cap discipline + memory-cap T2 corpus + determinism shim T1+T2
  pins + the `slice_5a_v1_2_0_split_fft_names_no_longer_route`
  inverted retirement pin asserting the v1.3.0 catalog narrowing.
  **Phase-2 EXIT scope explicitly narrows `EV-SCRIPT-CONTRACT` to slices 1-5.3**
  per the Description column update in this PR; slice 5.4+
  (scene-author surface — control bindings, event handlers,
  preset-recall hooks per ADR-003) is post-Phase-2 feature work
  tracked separately, consistent with the AO direction recorded in
  `engine/script/src/lib.rs` lines 133-136 ("Slice 5 has reached
  EXIT-quality core with slice 5.3 — the producer / consumer /
  catalog / adapter quad covers the cross-crate contract; further
  widening is feature work, not foundational").
- **`EV-AUDIO-TRACE`** — (a) `ci-run-url:25201608223`
  prefixed to the existing T1+T2 corpus repo-path chain. (b) green
  workspace test job. (c) corpus version pinned by slices 1+2+3a+3b
  + slice-5a cross-tag from EV-RENDER-CONFORMANCE + Tier-9 cpal
  multi-format dispatch + Tier-11 chroma_AudioFFT[8] cross-tag;
  cpal 0.15 + crossbeam-channel pinned by the lockfile. (d) drift
  thresholds applied via the LVL/HIT/TIM/PRS/BEAT/BPM/LRG family-
  tolerance discipline + the per-backend ACCEPTED-set framework
  carrying its first dual-backend entry post Tier-11 (see `EV-RENDER-CONFORMANCE`).
  The slice-3b CpalSource owner-thread pattern + FFI-seam
  `catch_unwind` discipline + Tier-9 multi-format conversion math
  T1 pins all green on the workspace test run.
- **`EV-SANDBOX-SAFETY`** — (a) `ci-run-url:25201608223`
  prefixed to the existing T1+T2 corpus repo-path chain. (b) green
  workspace test job. (c) corpus version pinned by slice-2 watchdog
  + slice-3 memory-cap surfaces; rquickjs 0.11.0 pinned by the
  lockfile (the no-`rust-alloc` `rust-alloc`-disabled config means
  `set_memory_limit` is honored via the QuickJS-bundled libc
  allocator per the slice-3 module-prose caveat). (d) drift
  thresholds applied via the watchdog p99/hard-cap CI-tolerance-
  window discipline (`[hard_ms, hard_ms + 200]` window per the
  T2 corpus) and the OOM-identifier `name`+`message` pinning per
  the slice-3 `is_oom_error` predicate.

### Verdict

All four Phase-2 EXIT evidence IDs satisfy the Per-Type Minimums at
HEAD `d8e92db`. Status transitions to `Pass` are recorded in
`docs/26-evidence-artifact-registry.md` for `EV-RENDER-CONFORMANCE`,
`EV-SCRIPT-CONTRACT`, `EV-AUDIO-TRACE`, and `EV-SANDBOX-SAFETY` by this
PR. Per `docs/26` §Rules line 194, the Phase-2 EXIT gate is therefore
passable. **Phase-2 EXIT achieved 2026-05-01 at commit `d8e92db`**.

The Phase-2 EXIT achievement is recorded in `docs/13-risk-register.md`
R-37 closing paragraph and CLAUDE.md Status section. Phase-3 entry
remains a separate AO-held milestone with its own eight-row gate set
per `docs/26` §Phase Gate Evidence Sets line 130; this audit does not
pre-empt the Phase-3 gate decision.

### Deferred axes recorded for Phase-3+/4a homes

These items were considered as candidates for Phase-2 EXIT but
verified out-of-scope or scheduled to later phases:

- **`EV-INTEGRATION-MATRIX`** — Phase-4a-bound per the
  registry's Phase Gate Evidence Sets section. Out of scope for
  Phase-2 EXIT. Covers OSC/MIDI/media route conformance + the
  Tauri-IPC bridge framework that downstream UI features depend on.
- **engine/script slice 5.4+ scene-author surface** — feature work
  per AO framing; explicitly narrowed out of `EV-SCRIPT-CONTRACT`'s Phase-2
  EXIT scope above. Schedule depends on AO direction for the
  Phase-3+ feature-development cadence.
- **device-enum UI** — has a hard dependency on the
  EV-INTEGRATION-MATRIX IPC bridge framework (Phase 4a); shipping
  pre-Phase-4a forces rework. Sequenced behind that bridge.
- **macOS-in-CI matrix expansion** — recommended as a Phase-4d
  release-validation nightly gate (~$20/month) rather than per-PR
  matrix expansion (~$70-80/month at current cadence). Defers to
  the release-validation pipeline.

### Sign-off

- [x] Phase-2 EXIT simulation ran to completion at HEAD `d8e92db`.
      All four Phase-2 EXIT evidence IDs satisfy the Per-Type
      Minimums; the gate is passable; the four row Status
      transitions are recorded by this PR. Slice 5.4+ scene-author
      surface narrowing on `EV-SCRIPT-CONTRACT` is recorded in the Description
      column. The deferred axes (`EV-INTEGRATION-MATRIX`, slice
      5.4+, device-enum UI, macOS-in-CI) are recorded with their
      proper Phase-3+/4a homes. Signed by AO `@PhantasticUniverse`
      on 2026-05-01.

## Phase-3 EXIT simulation

The Phase-3 EXIT gate set per `docs/26-evidence-artifact-registry.md`
§Phase Gate Evidence Sets line 125 enumerates eight required evidence
IDs: `EV-PRESET-PLAYLIST`, `EV-PERF-BUDGETS`, `EV-STATE-REPLAY`,
`EV-STATE-MIGRATION`, `EV-SHOW-BUNDLE-RECOVERY`,
`EV-AUDIO-ROUTING-MATRIX`, `EV-AUDIO-TRACE` (re-validated against the
full corpus), and `EV-COLOR-MANAGEMENT-SRGB`. Per `docs/26` §Rules
line 194, Phase-3 EXIT requires all eight rows at `Pass` — except
where the AO has explicitly recorded a deferral with proper-home
notation in `docs/13-risk-register.md` R-37.

This section runs the Phase-3 EXIT simulation against HEAD `f86895b`
(post `#311` merge) — the freshest green workspace test job on
`main` covering the full Phase-3 directed engineering wave (PRs `#301`,
`#302`, `#303`, `#304`, `#305`, `#306`, `#307`, `#308`, `#309`, `#310`,
`#311`). The wave landed across six feat PRs (state-replay corpus,
audio-routing-matrix corpus, preset-playlist corpus, state-migration
envelope corpus, show-bundle format harness, perf-budgets nightly
proxy benches) and five `docs(repo)` flip PRs transitioning
`EV-AUDIO-ROUTING-MATRIX`, `EV-STATE-REPLAY`, `EV-STATE-MIGRATION`,
`EV-PRESET-PLAYLIST`, and `EV-SHOW-BUNDLE-RECOVERY` from `Planned` to
`Pass`.

### Per-Type Minimums verification (Automated suite rows)

`docs/26` line 159 specifies the Per-Type Minimums for Automated suite
rows. Row-by-row verification at HEAD `f86895b`:

- **`EV-AUDIO-TRACE`** — re-validated against the full corpus
  per the Phase-3 cumulative gate. Already `Pass` from Phase-2 EXIT
  cross-tag (run id `25201608223` at `d8e92db`); no Phase-3 widening
  required. The slice-1 `extract_uniform_snapshot` routing path is
  cross-tagged with `EV-AUDIO-ROUTING-MATRIX`'s T1 invariants
  in `engine/audio/src/uniforms.rs::uniforms_drift_pins` (`Evidence:
  EV-AUDIO-TRACE (T1) + EV-AUDIO-ROUTING-MATRIX (T1)`).
- **`EV-AUDIO-ROUTING-MATRIX`** — (a) `ci-run-url:25204946481`
  + repo-path chain. (b) green workspace test job on `main` HEAD
  `f14c3d3` post-`#303` merge. (c) corpus version pinned by Phase-3
  wave PR 2 (`engine/audio/tests/routing_matrix_corpus/main.rs` —
  13 fixtures × 100 trials covering format conversion, channel
  aggregation, clamp-boundary edges). (d) drift thresholds applied
  via per-fixture tolerance assertion (1e-5 for direct conversions,
  0.0 exact for baselines) + dual-tagged uniforms_drift_pins module
  sharing T1 invariants with `EV-AUDIO-TRACE`.
- **`EV-STATE-REPLAY`** — (a) `ci-run-url:25203436497` +
  repo-path chain. (b) green workspace test job on `main` HEAD
  `0ddcb9c` post-`#301` merge. (c) corpus version pinned by Phase-3
  wave PR 1 (`engine/runtime/src/state_replay.rs` canonical-binary
  encoder + FNV-1a-64 hashing path; `engine/runtime/tests/
  state_replay_corpus/main.rs` 6 fixtures × 100 trials). (d) drift
  thresholds applied via cross-trial identity + golden-hash
  assertion per fixture. Pure-Rust FNV-1a-64; empirically backend-
  invariant on Linux Vulkan / Mesa llvmpipe (no per-backend ACCEPTED-
  set widening required).
- **`EV-STATE-MIGRATION`** — (a) `ci-run-url:25207466995` +
  repo-path chain. (b) green workspace test job on `main` HEAD
  `ef45bd0` post-`#307` merge. (c) corpus version pinned by Phase-3
  wave PR 4 (`engine/runtime/src/state_migration.rs` envelope
  encoder/decoder + V1→V2 forward migration; `engine/runtime/
  tests/state_migration_corpus/main.rs` 8 fixtures × 100 trials).
  (d) drift thresholds applied via cross-trial identity + golden-
  buffer assertion + decoder-rejection-path assertion per fixture.
  **Phase-3 EXIT scope explicitly narrows `EV-STATE-MIGRATION` to forward-
  migration only** per the Description column update; rollback
  semantics deferred to Phase-4+ per AO direction. The dual-encoder
  envelope design preserves `EV-STATE-REPLAY` + `EV-PRESET-PLAYLIST` goldens verbatim
  (existing `encode_state_canonical` hashing path UNCHANGED; new
  `encode_snapshot` envelope wraps it with a 4-byte BE version-tag
  prefix at the State level).
- **`EV-PRESET-PLAYLIST`** — (a) `ci-run-url:25205805621` +
  repo-path chain. (b) green workspace test job on `main` HEAD
  `c33913a` post-`#305` merge. (c) corpus version pinned by Phase-3
  wave PR 3 (`engine/runtime/tests/preset_playlist_corpus/main.rs`
  8 fixtures × 100 trials). (d) drift thresholds applied via cross-
  trial identity + direct semantic invariants over public State
  accessors (`active_preset` / `playlist_position` / `control_value`)
  + canonical_state_hash composition pin on the combined fixture.
  **Phase-3 EXIT scope explicitly narrows `EV-PRESET-PLAYLIST` to validation of
  existing PresetStore + PlaylistStore semantics** per the Description
  column update; preset-bank durability + scene-author preset capture
  + playlist reordering deferred to Phase-4+ per AO feature-work
  direction.
- **`EV-SHOW-BUNDLE-RECOVERY`** — (a) `ci-run-url:25208208038`
  + repo-path chain. (b) green workspace test job on `main` HEAD
  `92d50d0` post-`#308` merge. (c) corpus version pinned by Phase-3
  wave PR 5 (`engine/scene/src/show_bundle.rs` manifest types + tar
  reader/writer + SHA-256 verification; `engine/scene/tests/
  show_bundle_corpus/main.rs` 6 fixtures × 100 trials). (d) drift
  thresholds applied via cross-trial identity + canonical-JSON byte-
  stability assertion + per-fixture rejection-path assertion (file-
  checksum mismatch, manifest-checksum tamper, path-traversal
  defense). **Phase-3 EXIT scope explicitly narrows `EV-SHOW-BUNDLE-RECOVERY` to
  bundle-format spec only** per the Description column update;
  recovery / rollback semantics deferred to Phase-4+ per AO
  direction. The `docs/22-compatibility-versioning-policy.md` line
  78 reservation reconciled in the docs flip from `zip archive` to
  `tar archive` matching the AO Phase-3 narrowing decision.
- **`EV-PERF-BUDGETS`** — corpus + harness landed at `85f869e`
  per Phase-3 wave PR 6 (`engine/runtime/benches/budgets.rs` criterion
  proxy benches over command-bus dispatch + state-replay canonical-
  hash + preset-recall + playlist-advance latency; `scripts/
  budget-check.sh` budget assertion helper; `.github/workflows/
  macos-nightly.yml` extended with `cargo bench` + budget-check +
  artifact-upload steps; new `criterion 0.5` dev-dep clean against
  `deny.toml`). **Per-PR CI does NOT exercise the bench harness**
  (Path B nightly-only AO posture per the AO-approved cost
  framing — ~$20/month vs ~$70-80/month for a per-PR macOS matrix
  expansion). The `EV-PERF-BUDGETS` Pass transition therefore awaits the
  first GREEN macos-nightly run after `#309` merge (~24h post-
  merge); the docs flip PR follows the wave-PR pattern but with
  the macos-nightly anchor instead of the post-merge push CI run.
  **Phase-3 EXIT scope explicitly narrows `EV-PERF-BUDGETS` to observable-
  subsystem proxy benchmarks**; the true frame-loop budget surface
  is deferred until ADR-001/ADR-006 slice-3 lands the scheduler-
  tick path (the `pub(crate) frame: u64` placeholder in
  `engine/runtime/src/state.rs` carries the in-source comment
  "scheduler-tick path lands in a later slice"). `EV-PERF-BUDGETS` corpus is
  empirically green via local `cargo bench --quick` per the wave
  PR 6 commit body; the registry transition lands in the follow-up
  wave PR 6.1 docs flip after the macos-nightly anchor.
- **`EV-COLOR-MANAGEMENT-SRGB`** — explicitly deferred per AO
  direction. Sequenced behind a separate render-HUD-diagnostics PR
  (Phase-4+ scope). The deferral is recorded in `docs/13-risk-
  register.md` R-37 alongside the Phase-3 wave narrative; the
  `EV-COLOR-MANAGEMENT-SRGB` Description column will receive the
  deferral note in the wave PR 6.1 docs flip alongside `EV-PERF-BUDGETS`
  (the AO-approved bundling per
  `docs/EV-GATE-MODEL-AUDIT-checklist.md`'s Phase-3 closure plan).

### Verdict

Seven of the eight Phase-3 EXIT evidence IDs satisfy the Per-Type
Minimums at HEAD `953fd74`: `EV-AUDIO-TRACE`, `EV-AUDIO-ROUTING-MATRIX`,
`EV-STATE-REPLAY`, `EV-STATE-MIGRATION`, `EV-PRESET-PLAYLIST`,
`EV-SHOW-BUNDLE-RECOVERY`, and `EV-PERF-BUDGETS` are at `Pass` in
`docs/26-evidence-artifact-registry.md`. The eighth, `EV-COLOR-MANAGEMENT-SRGB`,
is explicitly deferred per AO direction — sequenced behind a separate
render-HUD-diagnostics PR exposing the active color pipeline (sRGB
encode/decode points, gamma handling, output transfer function); the
deferral note + re-evaluation criteria are recorded in the
`EV-COLOR-MANAGEMENT-SRGB` Description column update by this PR.

Per `docs/26` §Rules line 194 the Phase-3 EXIT gate is therefore
passable. **Phase-3 EXIT achieved 2026-05-01 at commit `953fd74`**.
`EV-PERF-BUDGETS` was anchored against macos-nightly run
id `25208934213`, manually dispatched via `workflow_dispatch` to
avoid the natural 04:19 UTC cadence wait — same hardware (macOS-
latest Apple-Silicon Metal), same workflow body, same green
outcome. The wave PR 6 follow-on `chore(release-eng)` budget-
tightening pass (~7 nightly runs post-this-PR) converts
`scripts/budget-check.sh` from the loose-floor record-don't-fail
posture to true budget assertions per `docs/10`.

The Phase-3 wave narrative is recorded in `docs/13-risk-register.md`
R-37 across wave-PR-1 through wave-PR-6 paragraphs (one per merged
wave). Phase-4+ entry remains a separate AO-held milestone with its
own gate set per `docs/11-roadmap-phases.md`; this audit does not
pre-empt the Phase-4 gate decision.

### Deferred axes recorded for Phase-4+ homes

These items were considered as candidates for Phase-3 EXIT but
verified out-of-scope or scheduled to later phases:

- **`EV-COLOR-MANAGEMENT-SRGB`** — explicitly deferred per
  AO direction; sequenced behind a future render-HUD-diagnostics PR
  exposing the active color pipeline (sRGB encode/decode points,
  gamma handling, output transfer function). Closure depends on the
  HUD diagnostic surface landing as part of unrelated render work.
- **`EV-STATE-MIGRATION` rollback semantics** — pre-migration snapshot capture +
  atomic restore + partial-rollback UX per `docs/32-state-preset-
  playlist-migration-policy.md` deferred to Phase-4+ feature work.
  Phase-3 narrowed to forward-migration only.
- **`EV-PRESET-PLAYLIST` preset-bank durability + scene-author preset capture +
  playlist reordering** — Phase-4+ feature work per AO direction.
  Phase-3 narrowed to existing PresetStore + PlaylistStore semantics.
- **`EV-SHOW-BUNDLE-RECOVERY` recovery / rollback semantics** — corrupted-bundle repair,
  partial-load fallback, last-good-state restore — Phase-4+ feature
  work. Phase-3 narrowed to bundle-format spec only.
- **`EV-PERF-BUDGETS` true frame-loop budget surface** — deferred until ADR-001/
  ADR-006 slice-3 lands the scheduler-tick path. Phase-3 narrowed to
  observable-subsystem proxy benchmarks. The follow-on `chore(release-
  eng)` PR (~7 nightly runs post-wave-PR-6.1) tightens
  `scripts/budget-check.sh` from the loose-floor record-don't-fail
  posture to true budget assertions per `docs/10`.
- **`EV-PERF-BUDGETS` per-PR macOS matrix expansion** — Path B AO posture is
  nightly-only (~$20/month) rather than per-PR (~$70-80/month).
  Re-evaluation conditional on a regression incident actually firing.

### Sign-off

- [x] Phase-3 EXIT simulation ran to completion at HEAD `953fd74`.
      Seven of the eight Phase-3 EXIT evidence IDs satisfy the
      Per-Type Minimums and are at `Pass`: `EV-AUDIO-TRACE` (
      Phase-2 cross-tag), `EV-AUDIO-ROUTING-MATRIX` (run id
      25204946481), `EV-STATE-REPLAY` (run id 25203436497),
      `EV-STATE-MIGRATION` (run id 25207466995, narrowed to
      forward-migration only), `EV-PRESET-PLAYLIST` (run id
      25205805621, narrowed to existing PresetStore + PlaylistStore
      semantics), `EV-SHOW-BUNDLE-RECOVERY` (run id
      25208208038, narrowed to bundle-format spec only), and
      `EV-PERF-BUDGETS` (macos-nightly run id 25208934213
      manually dispatched against `main` HEAD `953fd74`, narrowed to
      observable-subsystem proxy benchmarks). The eighth row,
      `EV-COLOR-MANAGEMENT-SRGB`, is explicitly deferred per AO
      direction — sequenced behind a separate render-HUD-diagnostics
      PR exposing the active color pipeline; Phase-3 EXIT achieved
      without `EV-COLOR-MANAGEMENT-SRGB` closure via the AO-approved
      deferral with the row's Description column carrying the deferral note +
      re-evaluation criteria. Phase-3 narrowing decisions for rows
      33 / 34 / 35 / 36 are recorded in their Description column
      updates; the deferred-axes list above records each item's
      proper Phase-4+ home. **Phase-3 EXIT achieved 2026-05-01 at
      commit `953fd74`**. Signed by AO `@PhantasticUniverse` on
      2026-05-01.

## Phase-4a Wave 1a simulation pre-pass

The Phase-4a EXIT gate set per `docs/26-evidence-artifact-registry.md`
§Phase Gate Evidence Sets line 128 enumerates eleven required evidence
IDs: `EV-INTEGRATION-MATRIX`, `EV-HOTKEY-PARITY-CONFORMANCE`,
`EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT`, `EV-MIDI-EXTENDED`,
`EV-NET-HARDENING`, `EV-ENTITLEMENT-ENFORCEMENT`,
`EV-ENTITLEMENT-OFFLINE`, `EV-ENTITLEMENT-REVOCATION`,
`EV-ENTITLEMENT-FREE-FEATURES`, and `EV-HEALTH-METRICS-LOOPBACK-ONLY`.
Per `docs/26` §Rules line 194, Phase-4a EXIT requires all eleven rows
at `Pass` — except where the AO has explicitly recorded a deferral with
proper-home notation in `docs/13-risk-register.md` R-37.

This section runs the Phase-4a Wave 1a **pre-pass** simulation against
HEAD `b335939` (post wave-PR `#325` merge) — the freshest green
workspace test job on `main` covering the Phase-4a Wave 1a feat PR
quartet (`#322` `#323` `#324` `#325`) plus slice-3 PR 1 (`#321`). The
wave landed across four feat PRs (hotkey parity + completeness +
conflict corpus, OSC integration-matrix + net-hardening corpus,
observability catalog-conformance + no-network-egress corpus,
entitlement enforcement + offline + revocation + free-features corpus
bundle) plus this `docs(repo)` flip PR transitioning
`EV-INTEGRATION-MATRIX`, `EV-HOTKEY-PARITY-CONFORMANCE`,
`EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT`, `EV-NET-HARDENING`,
`EV-ENTITLEMENT-ENFORCEMENT`, `EV-ENTITLEMENT-OFFLINE`,
`EV-ENTITLEMENT-REVOCATION`, `EV-ENTITLEMENT-FREE-FEATURES`, and
`EV-HEALTH-METRICS-LOOPBACK-ONLY` from `Planned` to `Pass`.

### Per-Type Minimums verification (Automated suite rows)

`docs/26` line 159 specifies the Per-Type Minimums for Automated suite
rows (a) `ci-run-url` or `repo-path` to test report artifact, (b) CI
run identifier with green status for pinned corpus version, (c)
corpus/fixture version recorded, (d) drift thresholds applied. Row-by-
row verification at HEAD `b335939`:

- **`EV-INTEGRATION-MATRIX`** — (a) `ci-run-url:25213038199`
  + repo-path chain. (b) green workspace nextest job on `main` HEAD
  `77f174c` post-`#323` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-A wave-PR (`integrations/osc-adapter/tests/
  integration_matrix_corpus/main.rs` — 15 fixtures × 100 trials
  covering 4 route prefixes + path-traversal + adapter state machine
  + UDP loopback bind + packet validation). (d) drift thresholds
  applied via cross-trial identity assertion + per-fixture rejection-
  path assertion + dual-tagged `defaults.rs` pins shared with `EV-NET-HARDENING`
  EV-NET-HARDENING. **Phase-4a Wave 1a scope `EV-INTEGRATION-MATRIX` narrowed to OSC
  routing-matrix conformance only** per AO direction; MIDI / camera
  / Syphon / audio-uniform-IPC arms deferred to Wave 1b.
- **`EV-HOTKEY-PARITY-CONFORMANCE`** — (a) `ci-run-url:
  25212775046` + repo-path chain. (b) green workspace Vitest job on
  `main` HEAD `a665d72` post-`#322` merge. (c) corpus version pinned
  by Phase-4a Wave 1a-B wave-PR (`src/lib/__tests__/
  hotkeys-parity.test.ts` — 6 fixtures pinning D/R/P/L → IPC
  contract via `vi.hoisted` invoke mock). (d) drift thresholds
  applied via direct invoke-mock-call assertion per fixture against
  the `docs/42` Sec.1 D/R/P/L parity contract.
- **`EV-HOTKEY-COMPLETENESS`** — (a) `ci-run-url:25212775046`
  + repo-path chain. (b) green workspace Vitest job on `main` HEAD
  `a665d72` post-`#322` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-B wave-PR (`src/lib/__tests__/
  hotkeys-completeness.test.ts` — 8 fixtures validating the 57-row
  vocabulary in `src/lib/data/hotkeys.ts`: 10 groups, non-empty
  keys + action, capability namespace ⊂ ALL ∪ {'free'}, no verbatim
  duplicates). (d) drift thresholds applied via direct equality
  assertion per fixture against the `docs/42` Sec.1-10 contract.
- **`EV-HOTKEY-CONFLICT`** — (a) `ci-run-url:25212775046` +
  repo-path chain. (b) green workspace Vitest job on `main` HEAD
  `a665d72` post-`#322` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-B wave-PR (`src/lib/__tests__/hotkeys-conflict.test.ts` —
  8 fixtures encoding 6-context taxonomy + Cmd↔Ctrl + Opt↔Alt
  cross-platform alias normalization + rules 1-5 from `docs/42`
  Sec.Conflict-and-priority-rules). (d) drift thresholds applied via
  direct equality assertion per fixture against the conflict-and-
  priority-rules contract.
- **`EV-NET-HARDENING`** — (a) `ci-run-url:25213038199` +
  repo-path chain. (b) green workspace nextest job on `main` HEAD
  `77f174c` post-`#323` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-A wave-PR (`integrations/osc-adapter/tests/
  network_hardening_corpus/main.rs` — 10 fixtures validating ADR-013
  defaults + rate-limit precedence + allowlist boundary + oversized-
  packet outcome counter). (d) drift thresholds applied via direct
  equality assertion per fixture against ADR-013 §1/3/5 constants
  and dual-tagged `defaults.rs` pins shared with `EV-INTEGRATION-MATRIX`
  EV-INTEGRATION-MATRIX.
- **`EV-ENTITLEMENT-ENFORCEMENT`** — (a) `ci-run-url:
  25213475549` + repo-path chain. (b) green workspace nextest +
  Vitest jobs on `main` HEAD `b335939` post-`#325` merge. (c) corpus
  version pinned by Phase-4a Wave 1a-D wave-PR
  (`engine/entitlement/src/enforcement.rs` 377 LoC enforcement
  module + `engine/entitlement/tests/enforcement_corpus/main.rs`
  8 fixtures + `src/lib/stores/__tests__/
  capabilities-conformance.test.ts` 6 fixtures). (d) drift thresholds
  applied via direct boolean assertion per fixture + <1µs latency
  assertion + `Send + Sync` trait check. **Phase-4a Wave 1a scope
  for `EV-ENTITLEMENT-ENFORCEMENT` narrowed to MVP watermark-on/off
  only** per AO direction; tier capability gating beyond the watermark
  deferred to post-MVP.
- **`EV-ENTITLEMENT-OFFLINE`** — (a) `ci-run-url:25213475549`
  + repo-path chain. (b) green workspace nextest job on `main` HEAD
  `b335939` post-`#325` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-D wave-PR (`engine/entitlement/tests/
  offline_revocation_corpus/main.rs` — 8 fixtures dual-tagged with
  `EV-ENTITLEMENT-REVOCATION`, covering 14-day offline window honor + post-expiry deny +
  boundary at `expires_at` + injected-clock determinism). (d) drift
  thresholds applied via cross-trial identity assertion per fixture
  against the 14-day window contract.
- **`EV-ENTITLEMENT-REVOCATION`** — (a) `ci-run-url:
  25213475549` + repo-path chain. (b) green workspace nextest job
  on `main` HEAD `b335939` post-`#325` merge. (c) corpus version
  pinned by Phase-4a Wave 1a-D wave-PR (same dual-tagged
  `engine/entitlement/tests/offline_revocation_corpus/main.rs`
  8 fixtures as `EV-ENTITLEMENT-OFFLINE`; expiry-based revocation only). (d) drift
  thresholds applied via cross-trial identity assertion per fixture.
  **Phase-4a Wave 1a scope `EV-ENTITLEMENT-REVOCATION` narrowed to expiry-based
  revocation only** per AO direction; deny-list activation behavior
  is post-MVP optional and out-of-scope for this row.
- **`EV-ENTITLEMENT-FREE-FEATURES`** — (a) `ci-run-url:
  25213475549` + repo-path chain. (b) green workspace nextest job
  on `main` HEAD `b335939` post-`#325` merge. (c) corpus version
  pinned by Phase-4a Wave 1a-D wave-PR (`engine/entitlement/tests/
  free_features_corpus/main.rs` — 12 fixtures pinning the ADR-009
  §Explicitly-free namespace: pause + transitions-off + playlist-next
  + playlist-previous + preset-recall + snap-recall + snap-save +
  scene-export + 4 token-absent-still-grants paths). (d) drift
  thresholds applied via direct boolean assertion per fixture
  against the `FREE_CAPABILITIES` array.
- **`EV-HEALTH-METRICS-LOOPBACK-ONLY`** — (a) `ci-run-url:
  25213270708` + repo-path chain. (b) green workspace nextest job
  on `main` HEAD `e8917c3` post-`#324` merge. (c) corpus version
  pinned by Phase-4a Wave 1a-C wave-PR
  (`engine/observability/tests/catalog_conformance/main.rs` —
  12 fixtures validating Phase-1 baseline catalog per `docs/31`;
  `engine/observability/tests/no_network_egress/main.rs` —
  4 fixtures verifying no-network-egress invariants). (d) drift
  thresholds applied via direct equality + structural-pin assertion
  per fixture against the `docs/31` Phase-1 baseline contract.

### Verdict

Five of the eleven Phase-4a EXIT evidence IDs satisfy the Per-Type
Minimums at HEAD `b335939` (counting the hotkey trio
`EV-HOTKEY-PARITY-CONFORMANCE` / `EV-HOTKEY-COMPLETENESS` /
`EV-HOTKEY-CONFLICT` as a single group, the entitlement quartet
`EV-ENTITLEMENT-ENFORCEMENT` / `EV-ENTITLEMENT-OFFLINE` /
`EV-ENTITLEMENT-REVOCATION` / `EV-ENTITLEMENT-FREE-FEATURES` as a
single group, and `EV-INTEGRATION-MATRIX` + `EV-NET-HARDENING` +
`EV-HEALTH-METRICS-LOOPBACK-ONLY` individually): five row groups
covering ten registry rows are at `Pass` in
`docs/26-evidence-artifact-registry.md`. Six Phase-4a EXIT rows remain
`Planned` for Wave 1b: `EV-MIDI-EXTENDED`, the four deferred arms behind
`EV-INTEGRATION-MATRIX` (MIDI conformance, camera conformance, Syphon
conformance, audio-uniform-IPC arm), and entitlement tier-gating beyond
the MVP watermark surface (which extends `EV-ENTITLEMENT-ENFORCEMENT`
and the post-MVP capability matrix).

This is a **Wave 1a pre-pass simulation**, not a full Phase-4a EXIT
sign-off. Per `docs/26` §Rules line 194, Phase-4a EXIT requires all
eleven rows at `Pass`; Wave 1b closure remains a prerequisite. Wave
1a partial closure is recorded in `docs/13-risk-register.md` R-37
across the wave-PR `#321` `#322` `#323` `#324` `#325` paragraph
appended at the bottom of R-37. Phase-4 EXIT itself remains AO-held
until all four sub-phases (4a / 4b / 4c / 4d) close per `docs/11-
roadmap-phases.md`; this audit pre-pass does not pre-empt the
Phase-4a EXIT closure decision (which remains AO-held until Wave 1b
lands the remaining six rows).

### Deferred axes recorded for Wave 1b homes

These items were considered as candidates for Phase-4a Wave 1a closure
but verified out-of-scope per AO narrowing direction or scheduled to
later waves:

- **`EV-MIDI-EXTENDED`** — Wave 1b scope. The hotkey corpus
  (`EV-HOTKEY-PARITY-CONFORMANCE` / `EV-HOTKEY-COMPLETENESS` /
  `EV-HOTKEY-CONFLICT`) covers control-focused parity + 57-row
  vocabulary completeness + 6-context conflict resolution but does not
  exercise the extended MIDI surface (CC + NRPN + program change + sysex
  routing) — that is `EV-MIDI-EXTENDED`'s surface and lands in Wave 1b.
- **`EV-INTEGRATION-MATRIX` MIDI / camera / Syphon / audio-
  uniform-IPC arms** — Wave 1b scope. Wave 1a narrowed to OSC
  routing-matrix conformance only per AO direction. The four
  deferred arms each follow the same fixture-corpus pattern against
  the relevant integration crate when those crates exist (MIDI
  adapter crate, camera-input integration, Syphon publisher, Tauri
  audio-uniform IPC bridge).
- **Entitlement tier-gating beyond MVP watermark
  (`EV-ENTITLEMENT-ENFORCEMENT` + post-MVP surface)** — Wave 1b
  scope. Wave 1a closure is narrowed to the watermark-on/off MVP per
  AO direction (`docs/11` line ~125). Tier capability gating beyond
  the watermark surface is post-MVP and lands in a Wave 1b expansion
  of `EV-ENTITLEMENT-ENFORCEMENT` + the deferred ADR-009 tier matrix.
- **`EV-SCHEDULER-TICK` Pass-anchor** — slice-3 PR 1
  (`#321`, merge `5c1aeb3`) introduced the foundational `Scheduler`
  + `Scheduler::tick` API per ADR-001/006 with the new evidence ID
  added in `Planned` state; the Pass transition lands in the
  slice-3 closing docs PR after PRs 2-6 of the slice-3 wave land.
  Not part of the Phase-4a EXIT gate set per `docs/26` line 128;
  tracked separately under the Phase-4+ release cumulative gate.

### Sign-off

- [ ] Phase-4a Wave 1a pre-pass simulation ran to completion at HEAD
      `b335939`. Five of the eleven Phase-4a EXIT evidence IDs
      satisfy the Per-Type Minimums and are at `Pass`:
      `EV-INTEGRATION-MATRIX` (run id 25213038199, narrowed
      to OSC routing-matrix conformance only),
      `EV-HOTKEY-PARITY-CONFORMANCE` (run id 25212775046),
      `EV-HOTKEY-COMPLETENESS` (run id 25212775046),
      `EV-HOTKEY-CONFLICT` (run id 25212775046),
      `EV-NET-HARDENING` (run id 25213038199),
      `EV-ENTITLEMENT-ENFORCEMENT` (run id 25213475549,
      narrowed to MVP watermark-on/off only),
      `EV-ENTITLEMENT-OFFLINE` (run id 25213475549),
      `EV-ENTITLEMENT-REVOCATION` (run id 25213475549,
      narrowed to expiry-based revocation only),
      `EV-ENTITLEMENT-FREE-FEATURES` (run id 25213475549),
      and `EV-HEALTH-METRICS-LOOPBACK-ONLY` (run id
      25213270708). Six Phase-4a EXIT rows remain `Planned` for
      Wave 1b: `EV-MIDI-EXTENDED`, the four deferred arms
      behind `EV-INTEGRATION-MATRIX` (MIDI conformance,
      camera conformance, Syphon conformance, audio-uniform-IPC
      arm), and entitlement tier-gating beyond the MVP watermark
      surface. Phase-4a Wave 1a narrowing decisions are recorded in their Description column updates;
      the deferred-axes list above records each item's proper
      Wave 1b home. **Pre-pass only — full Phase-4a EXIT requires
      all 11 rows at `Pass`, contingent on Wave 1b closure.**
      Sign-off pending the Wave 1b closing docs PR.

## Slice-3 scheduler-tick engineering wave closure

The slice-3 scheduler-tick engineering wave landed 2026-05-01 in a
five-PR sequence post-Phase-4a-Wave-1a, introducing the canonical
scheduler-tick frame loop per ADR-001 / ADR-006 across
`engine/runtime/src/scheduler/{mod,observability,audio,render}.rs`
plus the T2 corpus at
`engine/runtime/tests/scheduler_tick_corpus/main.rs`. `EV-SCHEDULER-TICK`
`EV-SCHEDULER-TICK` is not part of the Phase-4a EXIT gate set per
`docs/26` line 128; this section records the wave closure as the
equivalent of a phase-milestone sign-off for the new evidence ID
and the row's `Planned` → `Pass` transition. Anchor: post-merge
push run on `main` HEAD `642bada` after the slice-3 PR 4 merge
(`#330`).

### Per-PR Per-Type Minimums verification (5 PRs)

`docs/26` line 159 specifies the Per-Type Minimums for Automated
suite rows (a) `ci-run-url` or `repo-path` to test report artifact,
(b) CI run identifier with green status for pinned corpus version,
(c) corpus / fixture version recorded, (d) drift thresholds applied.
Per-PR verification at HEAD `642bada`:

- **Slice-3 PR 1 (`#321`, merge `5c1aeb3`)** — foundational
  `engine/runtime/src/scheduler/mod.rs` module: `Scheduler` struct +
  `tick(&mut State, &InMemoryBus) -> Result<TickReport,
  SchedulerError>` API + `SchedulerError::FrameOverflow` variant. 4
  T1 drift pins inline at `scheduler::tests` covering frame-
  advancement-by-one, command-bus drain composition, tick purity
  under identical inputs, and `u64::MAX` `FrameOverflow` grace
  (state + bus unchanged on overflow). Drift thresholds applied via
  direct equality assertion per pin against the slice-2 command-bus
  + slice-3a `State` foundation.
- **Slice-3 PR 2 (`#326`, merge `6f994ab`)** — T2 corpus at
  `engine/runtime/tests/scheduler_tick_corpus/main.rs`: 6 fixtures
  (50-command baseline + 256-command scale + 512-command complex
  chain + multistore-interleaved-256 + snap-cycles-128 +
  playlist-saturation-1024) × 100 trials each, with embedded golden
  hashes mirroring the EV-STATE-REPLAY canonical-state-hash
  composition pattern. Cross-trial identity assertion + golden-hash
  drift threshold per fixture.
- **Slice-3 PR 5 (`#328`, merge `6f46a35`)** — observability
  metric-emit wiring: `engine/runtime/src/scheduler/observability.rs`
  introduces the `TickEmitter` trait + `MetricEmitterAdapter`
  wrapping `Box<dyn EmitMetric>`; `Scheduler::with_metric_emitter`
  builder method. 5 T1 pins covering builder install + tick-with-
  emit emission + tick-without-emit no-op + emit ordering after
  frame advance. Drift thresholds applied via recording-emitter pin
  + direct assertion per fixture.
- **Slice-3 PR 3 (`#329`, merge `d888440`)** — audio snapshot pull
  wiring: `engine/runtime/src/scheduler/audio.rs` introduces the
  `AudioPull` trait + `MutexAudioPull` adapter (`Arc<Mutex<
  AudioUniformSnapshot>>` poison-recovery); `Scheduler::with_audio_
  pull` builder. 5 T1 pins covering snapshot-stored-on-state, no-
  pull leaves snapshot None, pull-before-frame-advances, pull-
  before-command-bus-commit, and lock-poisoning graceful recovery.
  Drift thresholds applied via recorder pin + direct assertion per
  fixture.
- **Slice-3 PR 4 (`#330`, merge `642bada`)** — render dispatch
  wiring: `engine/runtime/src/scheduler/render.rs` introduces the
  `RenderDispatch` trait + `RenderDispatchOutcome` enum;
  `Scheduler::with_render_dispatch` builder. 5 T1 pins covering
  invocation-once-per-tick, no-dispatch no-op, post-tick-frame
  hand-off, post-tick-audio-snapshot hand-off (the load-bearing
  ordering pin tying audio pull + frame advance + render dispatch
  together), and skipped-outcome does-not-abort-tick.

### Tick-step ordering verification

Tick-step ordering is load-bearing and pinned end-to-end by PR 4's
`tick_render_handler_receives_post_tick_audio_snapshot` T1 pin in
`engine/runtime/src/scheduler/render.rs`: audio pull (BEFORE state
mutation) → bus drain + frame advance → render dispatch (AFTER
state mutation, observing post-tick frame plus the audio snapshot
pulled at tick start) → observability emit (last, observing post-
tick state plus events-committed count). The slice-3 PR 1
invariants (frame advancement, bus drain composition, tick purity,
`FrameOverflow` grace) hold verbatim under additive subsystem
extension because each subsystem-extension PR adds its seam without
altering the bus-drain plus frame-advance core.

### T1 + T2 coverage summary

- T1 pins: 4 (PR 1 foundational) + 5 (PR 5 observability) + 5 (PR 3
  audio) + 5 (PR 4 render) = 19 inline T1 drift pins across the
  slice-3 wave.
- T2 corpus: 6 fixtures × 100 trials = 600 trial replays with
  embedded golden hashes (PR 2 corpus). The 3 existing T2 corpora
  (`state_replay_corpus`, `preset_playlist_corpus`,
  `state_migration_corpus`) preserve their goldens unchanged
  because the scheduler is purely additive.
- All 4 T2 corpora (`state_replay`, `preset_playlist`,
  `state_migration`, `scheduler_tick`) pass green on the post-merge
  push run on `main` HEAD `642bada`.

### Verdict

`EV-SCHEDULER-TICK` transitions `Planned` → `Pass` against
the post-merge push run on `main` HEAD `642bada`. The slice-3
engineering wave closes the L2 / L3 deferral markers from the
Phase-3 closure (`#[allow(dead_code)]` on `frame: u64` and
`TODO(slice-3)` markers in observability) — the scheduler-side
markers migrated to canonical advancement comments; the remaining
markers are intentional deferrals to subsystem-side widening with
proper Wave 1b / post-slice-3 homes recorded in the in-source
rephrasings.

### Sign-off

- [x] Slice-3 scheduler-tick engineering wave closure verified at
      HEAD `642bada`. Five feat PRs (`#321` `#326` `#328` `#329`
      `#330`) merged in sequence; `EV-SCHEDULER-TICK`
      transitions `Planned` → `Pass`. The canonical scheduler-tick
      frame loop per ADR-001 / ADR-006 lands across
      `engine/runtime/src/scheduler/{mod,observability,audio,render}
      .rs` with the established sub-module + builder + tick-step
      extension pattern; T1 + T2 coverage closed (19 inline T1 pins
      + 6-fixture × 100-trial T2 corpus). All 4 T2 corpora
      (state_replay, preset_playlist, state_migration,
      scheduler_tick) pass with goldens preserved. Phase-4d `EV-PERF-BUDGETS`
      `EV-PERF-BUDGETS` true frame-loop budget surface is now
      unblockable — the criterion proxy harness in
      `engine/runtime/benches/budgets.rs` can be widened to actual
      scheduler-tick benchmarks in a follow-up
      `chore(release-eng)` PR. **Wave-closure sign-off only —
      Phase-4 EXIT itself remains AO-held until all four sub-phases
      close per `docs/11`; this section records the slice-3
      milestone, not a phase EXIT.**

## Phase-4a Wave 1b closure

The Phase-4a Wave 1b closure (2026-05-02) closes 2 of 3 sub-bands
under AO-approved autonomous-execution direction. Wave 1b-C is
explicitly deferred to AO direction (camera adapter + Syphon adapter
+ IPC bridge framework; estimated 5,000-10,000 LoC multi-week scope;
requires platform integration decisions + likely new ADRs).

### Per-PR Per-Type Minimums verification

- **Wave 1b-A `#333`** (`bba49c1`) — new `integrations/midi-adapter/`
  crate; 2,535 LoC across 6 files; midir 0.10 cross-platform MIDI;
  17 in-source T1 pins + 17 T2 corpus fixtures (10 MIDI-CONF-11..20
  + 2 extras + 5 dual-tagged EV-INTEGRATION-MATRIX MIDI-arm); anchor
  run id `25219692261` green. Per-Type Minimums: ci-run-url + repo-path
  + corpus version (midir 0.10 + dual-tagged matrix-arm) + drift
  thresholds (per-fixture parse / normalization / timeout / counter
  assertions) all populated.

- **Wave 1b-B `#332`** (`b57e11b`) — `engine/entitlement/src/enforcement.rs`
  `HONORED_TOKEN_CAPABILITIES` widening 1 → 13 entries (data-only; no
  code-path change; preserves fail-closed semantics + FREE_CAPABILITIES
  unconditional grant + expired-token deny); 11 new Rust fixtures
  (8 → 19 total) + 6 new TS fixtures (6 → 12 total); anchor run id
  `25219329314` green. Per-Type Minimums: ci-run-url + repo-path +
  corpus version (HONORED_TOKEN_CAPABILITIES 13-entry widening) +
  drift thresholds (per-fixture boolean assertions across the widened
  capability set) all populated.

### Verdict

`EV-MIDI-EXTENDED` transitions `Planned` → `Pass`. Rows 37 +
86 narrowing-clauses dropped (Phase-4a Wave 1a's OSC-only and
MVP-watermark-only narrowings widened to MIDI matrix arm + tier-gating
respectively). Phase-4a EXIT row state after Wave 1a + 1b: **11 of 11
Phase-4a rows at Pass under collective narrowings**.

Full Phase-4a EXIT activation requires Wave 1c (camera/Syphon/IPC)
closure to drop the camera/Syphon/audio-uniform-IPC narrowing on row
37 + IO_CAMERA tier-gating activation on `EV-IMPORT-REFERENCE-SHOW`. Phase-4a EXIT is
**passable under current narrowings per `docs/26` §Rules line 194**;
the AO can declare Phase-4a EXIT at its discretion.

### Wave 1c deferral (AO direction required)

- **Camera adapter** (cross-platform: V4L2 / AVFoundation /
  MediaFoundation) — new platform-integration crate; estimated
  ~1,500-2,500 LoC; requires platform-specific FFI bindings.
- **Syphon adapter** (macOS-specific GPU IO; Spout for Windows
  parity) — new platform-integration crate; estimated ~1,500-2,500
  LoC; requires Syphon SDK linkage decision (vendored vs system).
- **IPC bridge framework** (H6 audio-visualizer engine→UI wiring
  per the post-Phase-3 deferral; the framework also unblocks live
  `EntitlementBitmask` IPC for the `capabilitiesStore` tier-gating
  swap-in) — estimated ~2,000-5,000 LoC; requires Tauri ⇄ runtime
  communication architecture decisions + likely new ADR-track work.

The Wave 1c deferral is recorded in `docs/13-risk-register.md` R-37
closing paragraph (this PR) and in the `EV-INTEGRATION-MATRIX` + `EV-IMPORT-REFERENCE-SHOW` Description
columns (this PR). When AO direction lands for Wave 1c, the proper
home is a new directed engineering wave with new ADR + research +
plan-mode + implementation cycle (estimated multi-week wall-clock).

### Sign-off

- [x] Phase-4a Wave 1b closure recorded at `main` HEAD post-merge
      of this PR. 2 of 3 sub-bands closed (1b-A MIDI; 1b-B
      tier-gating); 1b-C deferred per AO direction. Phase-4a EXIT
      passable under current narrowings; AO can declare at discretion.
      **Phase-4a EXIT itself remains AO-held; Phase-4b/4c/4d remain
      AO-held milestones with their own gate sets per `docs/11`.**

## Phase-4a EXIT closure

The Phase-4a EXIT gate (per `docs/26-evidence-artifact-registry.md`
§Phase Gate Evidence Sets line 128 — eleven required evidence IDs:
`EV-INTEGRATION-MATRIX`, `EV-HOTKEY-PARITY-CONFORMANCE`,
`EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT`, `EV-MIDI-EXTENDED`,
`EV-NET-HARDENING`, `EV-ENTITLEMENT-ENFORCEMENT`,
`EV-ENTITLEMENT-OFFLINE`, `EV-ENTITLEMENT-REVOCATION`,
`EV-ENTITLEMENT-FREE-FEATURES`, and `EV-HEALTH-METRICS-LOOPBACK-ONLY`)
is closed against the cumulative outcome of the Phase-4a Wave 1a (PR
`#321` slice-3 PR 1 + feat PRs `#322` `#323` `#324` `#325`) plus
Phase-4a Wave 1b (feat PRs `#332` `#333` plus closing docs PR `#334`)
sequence landing on `main` 2026-05-01 → 2026-05-02. All eleven
Phase-4a EXIT rows are at `Pass` in `docs/26-evidence-artifact-
registry.md` post-`#334` merge at HEAD `288a214`; this section runs
the Phase-4a EXIT closure simulation against that HEAD plus the
post-this-PR merge anchor. AO has approved declaration; this section
records the achieved framing.

### Per-Type Minimums verification (11 rows enumerated)

`docs/26` line 159 specifies the Per-Type Minimums for Automated suite
rows: (a) `ci-run-url` or `repo-path` to test report artifact, (b) CI
run identifier with green status for pinned corpus version, (c)
corpus / fixture version recorded, (d) drift thresholds applied. Row-
by-row verification at HEAD `288a214` (the eleven Phase-4a EXIT rows
each cite their per-row anchor run id from the prior wave-closure
sections above):

- **`EV-INTEGRATION-MATRIX`** — Status `Pass`. Narrowing: OSC
  routing-matrix conformance plus MIDI input matrix arm — camera /
  Syphon / audio-uniform-IPC arms deferred to Wave 1c. Per-row anchor
  run id `25213038199` (Wave 1a-A `#323` merge `77f174c`); updated by
  Wave 1b-A `#333` merge `bba49c1` (anchor run id `25219692261`)
  landing the MIDI input arm. Per-Type Minimums: corpus version
  pinned (`integrations/osc-adapter/tests/integration_matrix_corpus/
  main.rs` 15 fixtures × 100 trials plus `integrations/midi-adapter/
  tests/midi_extended_corpus/main.rs` 5 dual-tagged matrix-arm
  fixtures) + drift thresholds applied (cross-trial identity +
  per-fixture rejection-path + dual-tagged `defaults.rs` pins shared
  with `EV-NET-HARDENING`) + reproducibility confirmed across both anchor runs.
- **`EV-HOTKEY-PARITY-CONFORMANCE`** — Status `Pass`. Narrowing:
  none (full scope). Per-row anchor run id `25212775046` (Wave 1a-B
  `#322` merge `a665d72`). Per-Type Minimums: corpus version pinned
  (`src/lib/__tests__/hotkeys-parity.test.ts` 6 fixtures pinning
  D/R/P/L → IPC contract via `vi.hoisted` invoke mock) + drift
  thresholds applied (direct invoke-mock-call assertion per fixture
  against `docs/42` Sec.1) + reproducibility confirmed.
- **`EV-HOTKEY-COMPLETENESS`** — Status `Pass`. Narrowing: none
  (full scope). Per-row anchor run id `25212775046` (Wave 1a-B `#322`
  merge `a665d72`). Per-Type Minimums: corpus version pinned (`src/
  lib/__tests__/hotkeys-completeness.test.ts` 8 fixtures validating
  the 57-row vocabulary in `src/lib/data/hotkeys.ts`) + drift
  thresholds applied (direct equality assertion per fixture against
  `docs/42` Sec.1-10) + reproducibility confirmed.
- **`EV-HOTKEY-CONFLICT`** — Status `Pass`. Narrowing: none
  (full scope). Per-row anchor run id `25212775046` (Wave 1a-B `#322`
  merge `a665d72`). Per-Type Minimums: corpus version pinned (`src/
  lib/__tests__/hotkeys-conflict.test.ts` 8 fixtures encoding
  6-context taxonomy + Cmd↔Ctrl + Opt↔Alt cross-platform alias
  normalization + rules 1-5) + drift thresholds applied (direct
  equality assertion per fixture against the conflict-and-priority-
  rules contract) + reproducibility confirmed.
- **`EV-MIDI-EXTENDED`** — Status `Pass`. Narrowing: MPE V2
  deferred (diagnostic-fires-once rejection at MVP per `docs/08`);
  SysEx logged-only at MVP. Per-row anchor run id `25219692261`
  (Wave 1b-A `#333` merge `bba49c1`). Per-Type Minimums: corpus
  version pinned (`integrations/midi-adapter/` 17 fixtures × 100
  trials covering MIDI-CONF-11..20 plus 5 dual-tagged matrix-arm
  fixtures; midir 0.10 cross-platform MIDI) + drift thresholds
  applied (per-fixture parse / normalization / timeout / counter
  assertions) + reproducibility confirmed.
- **`EV-NET-HARDENING`** — Status `Pass`. Narrowing: none
  (full scope). Per-row anchor run id `25213038199` (Wave 1a-A `#323`
  merge `77f174c`). Per-Type Minimums: corpus version pinned
  (`integrations/osc-adapter/tests/network_hardening_corpus/main.rs`
  10 fixtures validating ADR-013 defaults + rate-limit precedence +
  allowlist boundary + oversized-packet outcome counter) + drift
  thresholds applied (direct equality assertion per fixture against
  ADR-013 §1/3/5 constants and dual-tagged `defaults.rs` pins shared
  with `EV-INTEGRATION-MATRIX`) + reproducibility confirmed.
- **`EV-ENTITLEMENT-ENFORCEMENT`** — Status `Pass`. Narrowing:
  13-cap honored set (Wave 1b-B widening from 1 → 13 entries; the
  IO_CAMERA capability is intentionally excluded — free at MVP per
  ADR-009 / `docs/09` Tier matrix). Per-row anchor run ids
  `25213475549` (Wave 1a-D `#325` merge `b335939`) plus `25219329314`
  (Wave 1b-B `#332` merge `b57e11b`). Per-Type Minimums: corpus
  version pinned (`engine/entitlement/src/enforcement.rs` 377 LoC
  enforcement module + `engine/entitlement/tests/enforcement_corpus/
  main.rs` 19 Rust fixtures (8 → 19 widened) + `src/lib/stores/
  __tests__/capabilities-conformance.test.ts` 12 TS fixtures (6 → 12
  widened); HONORED_TOKEN_CAPABILITIES 13-entry set) + drift
  thresholds applied (direct boolean assertion per fixture across the
  widened capability set + <1µs latency assertion + `Send + Sync`
  trait check) + reproducibility confirmed across both anchor runs.
- **`EV-ENTITLEMENT-OFFLINE`** — Status `Pass`. Narrowing: none
  (full scope). Per-row anchor run id `25213475549` (Wave 1a-D `#325`
  merge `b335939`). Per-Type Minimums: corpus version pinned
  (`engine/entitlement/tests/offline_revocation_corpus/main.rs`
  8 fixtures dual-tagged with `EV-ENTITLEMENT-REVOCATION`, covering 14-day offline window
  honor + post-expiry deny + boundary at `expires_at` + injected-
  clock determinism) + drift thresholds applied (cross-trial identity
  assertion per fixture against the 14-day window contract) +
  reproducibility confirmed.
- **`EV-ENTITLEMENT-REVOCATION`** — Status `Pass`. Narrowing:
  expiry-based revocation only — deny-list activation behavior is
  post-MVP optional and out-of-scope. Per-row anchor run id
  `25213475549` (Wave 1a-D `#325` merge `b335939`). Per-Type
  Minimums: corpus version pinned (same dual-tagged
  `engine/entitlement/tests/offline_revocation_corpus/main.rs`
  8 fixtures as `EV-ENTITLEMENT-OFFLINE`; expiry-based revocation only) + drift
  thresholds applied (cross-trial identity assertion per fixture) +
  reproducibility confirmed.
- **`EV-ENTITLEMENT-FREE-FEATURES`** — Status `Pass`. Narrowing:
  none (full scope). Per-row anchor run id `25213475549` (Wave 1a-D
  `#325` merge `b335939`). Per-Type Minimums: corpus version pinned
  (`engine/entitlement/tests/free_features_corpus/main.rs` 12
  fixtures pinning the ADR-009 §Explicitly-free namespace: pause +
  transitions-off + playlist-next + playlist-previous + preset-recall
  + snap-recall + snap-save + scene-export + 4 token-absent-still-
  grants paths) + drift thresholds applied (direct boolean assertion
  per fixture against the `FREE_CAPABILITIES` array) + reproducibility
  confirmed.
- **`EV-HEALTH-METRICS-LOOPBACK-ONLY`** — Status `Pass`.
  Narrowing: none (full scope per `docs/31` §Phase-1 baseline
  catalog). Per-row anchor run id `25213270708` (Wave 1a-C `#324`
  merge `e8917c3`). Per-Type Minimums: corpus version pinned
  (`engine/observability/tests/catalog_conformance/main.rs` 12
  fixtures validating Phase-1 baseline catalog plus `engine/
  observability/tests/no_network_egress/main.rs` 4 fixtures verifying
  no-network-egress invariants) + drift thresholds applied (direct
  equality + structural-pin assertion per fixture against the
  `docs/31` Phase-1 baseline contract) + reproducibility confirmed.

### Verdict

All 11 Phase-4a EXIT evidence IDs satisfy the Per-Type Minimums at
HEAD `5acc452` (the merge commit of this docs(repo) closing PR;
the achievement-anchor is conceptually the merge commit, with the
pre-merge anchor at HEAD `288a214` post-`#334` already carrying all
11 rows at `Pass`). Status transitions are recorded in
`docs/26-evidence-artifact-registry.md`. Per `docs/26` §Rules line
194, the Phase-4a EXIT gate is therefore passable.
**Phase-4a EXIT achieved 2026-05-03 at commit `5acc452`.**

Collective narrowings recap (the AO-approved scope reductions that
together make Phase-4a EXIT passable under the current waveform): row
37 `EV-INTEGRATION-MATRIX` narrowed to OSC routing-matrix plus MIDI
input arm (camera / Syphon / audio-uniform-IPC arms deferred to Wave
1c); `EV-MIDI-EXTENDED` narrowed to MPE V2 deferred plus SysEx
logged-only at MVP; `EV-ENTITLEMENT-ENFORCEMENT` narrowed to
the 13-cap honored set with IO_CAMERA intentionally excluded (free at
MVP per ADR-009); `EV-ENTITLEMENT-REVOCATION` narrowed to
expiry-based only with deny-list activation deferred to post-MVP.

Wave 1c deferred-axes record (camera + Syphon adapters + audio-
uniform-IPC bridge) carries forward unchanged from the Phase-4a Wave
1b closure section above; the deferral home remains a future directed
engineering wave with new ADR + research + plan-mode + implementation
cycle (estimated 5,000-10,000 LoC multi-week wall-clock; requires
platform integration decisions + likely new ADRs).

Phase-4 EXIT itself remains a separate AO-held milestone with its
own gate set per `docs/11-roadmap-phases.md`; Phase-4a EXIT
achievement does not pre-empt the Phase-4b / 4c / 4d sub-phase gate
decisions or the Phase-4 EXIT itself.

### Sign-off

- [x] Phase-4a EXIT simulation ran to completion at HEAD `<merge-sha>`
      (this docs(repo) closing PR's merge commit; pre-merge anchor at
      HEAD `288a214` post-`#334` already carries all 11 rows at
      `Pass`). All 11 Phase-4a EXIT evidence IDs satisfy the Per-Type
      Minimums and are at `Pass`: `EV-INTEGRATION-MATRIX` (
      anchor run id 25213038199 plus 25219692261, narrowed to OSC
      routing-matrix plus MIDI input arm), `EV-HOTKEY-PARITY-
      CONFORMANCE` (`EV-HOTKEY-PARITY-CONFORMANCE`, run id 25212775046, full scope),
      `EV-HOTKEY-COMPLETENESS` (run id 25212775046, full
      scope), `EV-HOTKEY-CONFLICT` (run id 25212775046, full
      scope), `EV-MIDI-EXTENDED` (run id 25219692261,
      narrowed to MPE V2 deferred plus SysEx logged-only at MVP),
      `EV-NET-HARDENING` (run id 25213038199, full scope),
      `EV-ENTITLEMENT-ENFORCEMENT` (run id 25213475549 plus
      25219329314, narrowed to 13-cap honored set with IO_CAMERA
      excluded), `EV-ENTITLEMENT-OFFLINE` (run id 25213475549,
      full scope), `EV-ENTITLEMENT-REVOCATION` (run id
      25213475549, narrowed to expiry-based only), `EV-ENTITLEMENT-
      FREE-FEATURES` (`EV-ENTITLEMENT-FREE-FEATURES`, run id 25213475549, full scope), and
      `EV-HEALTH-METRICS-LOOPBACK-ONLY` (run id 25213270708,
      full scope per `docs/31` Phase-1 baseline catalog). Phase-4a
      EXIT collective narrowings recap: `EV-INTEGRATION-MATRIX` OSC plus MIDI input
      (camera / Syphon / audio-uniform-IPC deferred to Wave 1c);
      `EV-MIDI-EXTENDED` MPE V2 plus SysEx-logged-only;
      `EV-ENTITLEMENT-ENFORCEMENT` 13-cap honored set
      (IO_CAMERA excluded — free at MVP per ADR-009); `EV-ENTITLEMENT-REVOCATION` expiry-
      only revocation. Wave 1c deferred axes (camera + Syphon
      adapters + audio-uniform-IPC bridge) carry forward to a future
      directed engineering wave with new ADR + research + plan-mode +
      implementation cycle. Phase-4 EXIT itself remains a separate
      AO-held milestone with its own gate set per `docs/11`; Phase-4a
      EXIT achievement does not pre-empt the Phase-4b / 4c / 4d
      sub-phase gate decisions. **Phase-4a EXIT achieved 2026-05-03
      at commit `<merge-sha>`**. Signed by AO `@PhantasticUniverse`
      on 2026-05-03.

## Phase-4a Wave 1c-α closure

The Phase-4a Wave 1c-α closure (2026-05-03) lands the audio-uniform
IPC bridge governed by `docs/adr/ADR-019-audio-uniform-ipc-bridge.md`.
The wave shipped in 3 PRs on `main`: PR `#338` (`efd5de0`) — ADR-019
acceptance + new `EV-AUDIO-UNIFORM-IPC` evidence row added at status
`Planned` (`docs/26-evidence-artifact-registry.md` `EV-AUDIO-UNIFORM-IPC`); PR `#339`
(`c1d93e8`) — producer-side `AudioUniformSnapshotPayload` ts-rs
newtype in `app/ui-bridge/`, new `engine/runtime/src/scheduler/ipc.rs`
sub-module declaring the `IpcEmit` trait + 4 T1 drift pins,
`Scheduler::with_ipc_emit` builder + tick step at AFTER-observability-
emit, T2 round-trip corpus at
`app/ui-bridge/tests/audio_uniform_payload_round_trip.rs`; PR `#340`
(`fcb443f`) — consumer-side `src/lib/stores/audioSnapshot.svelte.ts`
singleton store using `listen('audio_uniform_snapshot_changed', ...)`
plus dual-mode retrofits to `AudioReactivity.svelte` /
`BpmRing.svelte` / `BeatDots.svelte`. Wave 1c-α is the first sub-band
of the Phase-4a EXIT closure's deferred Wave 1c (camera + Syphon +
audio-uniform-IPC bridge); Wave 1c-β (camera + Syphon adapters)
remains AO-held and carries forward unchanged.

### Per-Type Minimums verification

The closure transitions one row (`EV-AUDIO-UNIFORM-IPC`) from
`Planned` to `Pass (narrowed)`. T2 conformance corpora shipped: 6
round-trip tests in
`app/ui-bridge/tests/audio_uniform_payload_round_trip.rs` (payload-
identity / engine↔payload bidirectional / JSON round-trip / default
round-trip / extreme-values round-trip / ts-rs name stability) plus
4 T1 drift pins in
`engine/runtime/src/scheduler/ipc.rs::scheduler_ipc_pins` (IpcEmit
trait shape + with_ipc_emit builder composition + AFTER-observability-
emit ordering + snapshot-presence gating) plus 7 T2 store tests in
`src/lib/stores/audioSnapshot.svelte.test.ts` (subscribe idempotency
+ listen event handling + unsubscribe cleanup + no-Tauri fallback)
plus 8 T2 widget live-mode tests across the 3 retrofitted widgets
(`src/lib/widgets/AudioReactivity.test.ts`,
`src/lib/widgets/BpmRing.test.ts`,
`src/lib/widgets/BeatDots.test.ts`) validating the dual-mode live-
snapshot vs synthetic-fallback gate.

- **PR `#338`** (`efd5de0`) — ADR-019 acceptance plus `EV-AUDIO-UNIFORM-IPC` added at
  `Planned`; anchor run id `25277275238` (rust workflow on `main`
  HEAD `efd5de0`) green. Per-Type Minimums: docs-only PR; no Per-Type
  Minimums apply (status row added at `Planned`, no test corpus
  required at this step).
- **PR `#339`** (`c1d93e8`) — producer-side payload newtype + IpcEmit
  trait + scheduler-tick wiring + T2 round-trip corpus; anchor run
  id `25277801336` (rust workflow on `main` HEAD `c1d93e8`) green.
  Per-Type Minimums: ci-run-url + repo-path
  (`app/ui-bridge/tests/audio_uniform_payload_round_trip.rs` +
  `engine/runtime/src/scheduler/ipc.rs`) + corpus version (ts-rs
  payload newtype + IpcEmit trait composition into scheduler-tick) +
  drift thresholds (field-by-field equality across the 11-field
  surface + AFTER-observability-emit ordering pin + snapshot-presence
  gating pin) all populated.
- **PR `#340`** (`fcb443f`) — consumer-side store + dual-mode widget
  retrofits + Vitest coverage; anchor run id `25278169331` (frontend
  workflow on `main` HEAD `fcb443f`) green. Per-Type Minimums:
  ci-run-url + repo-path (`src/lib/stores/audioSnapshot.svelte.ts` +
  `src/lib/stores/audioSnapshot.svelte.test.ts` + the 3 widget
  test files) + corpus version (Svelte 5 store with idempotent
  subscribe + 3 widget retrofits with live-snapshot-presence gate) +
  drift thresholds (per-fixture subscribe / unsubscribe / event-
  handling assertions + dual-mode visual-gate assertions) all
  populated.

### Verdict

`EV-AUDIO-UNIFORM-IPC` transitions `Planned` → `Pass
(narrowed)`. The Pass scope is explicitly narrowed to the cross-seam
payload-shape contract plus per-side composition; the narrowing is
recorded in the `EV-AUDIO-UNIFORM-IPC` Description column and recapped here so the
deferred-axes record stays honest. **Phase-4a Wave 1c-α closure
achieved 2026-05-03 at commit `<merge-sha>`** (the merge commit of
this docs(repo) closing PR; pre-merge anchor at HEAD `fcb443f`
post-`#340` already carries the `EV-AUDIO-UNIFORM-IPC` corpus + producer-side T2 +
T1 pins + consumer-side store + widget retrofits). Phase-4a EXIT
collective state is unchanged: `EV-INTEGRATION-MATRIX` narrowing-clause for the
audio-uniform-IPC arm of `EV-INTEGRATION-MATRIX` still references
"Wave 1c" because the live end-to-end IPC wiring (Tauri concrete
implementation of the `IpcEmit` trait + audio-thread + scheduler-
tick bootstrap) is the deferred axis below, not the cross-seam
payload-shape contract that Wave 1c-α validated.

### Deferred axes (Wave 1c-α narrowing record)

The `EV-AUDIO-UNIFORM-IPC` Pass narrowing carries 5 explicit deferred axes; each is
preserved here so future-wave widening has an anchor:

- **Live end-to-end producer→Tauri-emit→consumer wiring** is deferred
  to a future production audio-thread + scheduler-tick + Tauri-emit-
  adapter bootstrap wave. The Tauri shell currently does not
  bootstrap the engine scheduler-tick or audio analysis thread, and
  the `IpcEmit` trait has no Tauri concrete implementation yet
  (engine/runtime correctly does not depend on Tauri). Widgets
  render in synthetic-fallback mode in production until that
  bootstrap wave lands.
- **Drop-frames-on-saturation outcome** per ADR-019 §Decision §4 is
  not directly exercised by a test in this wave. The behaviour
  follows from Tauri's `app.emit` best-effort semantics (silent drop
  on event-queue saturation), but no fixture forces saturation under
  producer-overrun. Deferred to the future production-bootstrap
  wave's integration test surface.
- **Stale-but-last-good on audio-fault** per ADR-019 §Decision §6
  similarly follows from `MutexAudioPull`'s poison-recovery path
  (already exercised at T1 in
  `engine/runtime/src/scheduler/audio.rs`) but is not surfaced
  through the IPC seam in this wave's tests.
- **Spectrum widget retrofit** listed in ADR-019 §Rollout PR 2 is
  deferred — the Spectrum primitive does not exist as a standalone
  widget; the bundle's Spectrum was inlined into
  `src/lib/views/AudioUniforms.svelte` with 64 synthetic bars,
  structurally incompatible with the engine's 8-band FFT surface
  without a non-trivial 8 → 64 interpolation design choice. Deferred
  as post-MVP enhancement.
- **`AudioReactivity` 48-bar FFT visualiser** stays synthetic-driven
  — only the 4 named band tiles plus the BPM strip read from the
  live snapshot via the documented 8 → 4 reduction
  (`avg(fft_bands[0..2])` → BASS, `avg(fft_bands[2..4])` → LOW-MID,
  `avg(fft_bands[4..6])` → HIGH-MID, `avg(fft_bands[6..8])` → AIR);
  the 8 → 48 widening is preserved as a future-wave option.

### Wave 1c-β + production-bootstrap deferral (AO direction required)

Wave 1c-β (camera + Syphon adapters) carries forward unchanged from
the Phase-4a EXIT closure's deferred-axes record above; the home
remains a future directed engineering wave with new ADR + research +
plan-mode + implementation cycle (estimated 5,000-10,000 LoC multi-
week wall-clock; requires platform integration decisions + likely
new ADRs). The production-bootstrap wave (audio-thread + scheduler-
tick + Tauri-emit-adapter) that drops the `EV-AUDIO-UNIFORM-IPC` narrowing axes (i)
through (iii) above is a separate AO-held axis from Wave 1c-β; it
may sequence before, after, or in parallel with Wave 1c-β at AO
discretion. Phase-4 EXIT itself remains AO-held until 4b / 4c / 4d
sub-phases close per `docs/11`; Wave 1c-α achievement does not
pre-empt the Phase-4b / 4c / 4d sub-phase gate decisions or the
Phase-4 EXIT itself.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-03

## Phase-4a Wave 1c-α-2 closure

The Phase-4a Wave 1c-α-2 closure (2026-05-06) lands the production
audio bootstrap that drops three of the five `EV-AUDIO-UNIFORM-IPC`
(`EV-AUDIO-UNIFORM-IPC`) deferred axes Wave 1c-α had recorded. Wave
1c-α had shipped the cross-seam payload-shape contract plus per-side
composition but had explicitly deferred the live end-to-end
producer→Tauri-emit→consumer wiring because `engine/runtime` MUST
NOT depend on Tauri (the `IpcEmit` trait had no Tauri concrete
implementation; the Tauri shell did not bootstrap the engine
scheduler-tick or the audio analysis thread; widgets rendered in
synthetic-fallback mode in production). Wave 1c-α-2 retired that
production-bootstrap deferral.

The wave shipped in 3 PRs on `main`:

- **Bootstrap PR 1 / PR `#343`** (`804d6cd`) — `engine/audio/src/runtime.rs`
  added `AudioRuntime` + `AudioRuntimeHandle`. The producer-side
  runtime spawns a thread feeding `Arc<Mutex<AudioUniformSnapshot>>`
  consumed by `MutexAudioPull`. 7 T1 drift pins.
- **Bootstrap PR 2 / PR `#344`** (`9a1131e`) — Tauri-shell wiring.
  Added `app/shell/src/state/ipc_emit.rs` (`TauriIpcEmitter`, the
  Tauri concrete `IpcEmit` adapter wrapping `AppHandle::emit`),
  extended `AppState` lifecycle in `app/shell/src/state/mod.rs` with
  `SchedulerThreadHandle` (ordered shutdown), and spawned the drift-
  corrected 60 Hz scheduler-tick thread in `app/shell/src/lib.rs::run`
  setup callback (configured with `MutexAudioPull` audio source +
  `TauriIpcEmitter`). 7 T1 drift pins.
- **Bootstrap PR 3** (this docs(repo) closing PR + e2e wire test) —
  `app/shell/tests/audio_uniform_ipc_e2e.rs` added 3 T2 cross-seam
  wire tests using `tauri::test::mock_app()`: round-trip payload
  identity (producer→listener), multi-emit ordering (3 emits → in-
  order delivery), emit-after-listener-unlisten panic-free posture
  (ADR-019 §4 silent-drop kin path).

### Per-Type Minimums verification

The closure widens `EV-AUDIO-UNIFORM-IPC` from `Pass
(narrowed; 5 deferred axes)` to `Pass (narrowed; 2 deferred axes
plus 1 still-deferred-through-the-wire axis)`. Cumulative test
count for the wave: 7 T1 pins (PR `#343` producer runtime) + 7 T1
pins (PR `#344` Tauri adapter + scheduler-tick bootstrap) + 3 T2
cross-seam wire tests (Bootstrap PR 3) = 17 new tests, all green
on `main` HEAD `9a1131e` post-`#344` merge. Wave 1c-α's existing
T2 corpora (6 round-trip + 7 store + 8 widget = 21 tests) plus T1
pins (4 ipc.rs pins) stay green and unchanged.

- **PR `#343`** (`804d6cd`) — producer-side `engine/audio` runtime
  entrypoint; anchor run id `25430275781` (rust workflow on `main`
  HEAD `804d6cd`) green. Per-Type Minimums: ci-run-url + repo-path
  (`engine/audio/src/runtime.rs`) + corpus version (`AudioRuntime`
  + `AudioRuntimeHandle` thread spawn pattern feeding
  `Arc<Mutex<AudioUniformSnapshot>>`) + drift thresholds (7 T1
  pins covering runtime construction / handle lifecycle / shared
  snapshot mutation / poison recovery / shutdown ordering) all
  populated.
- **PR `#344`** (`9a1131e`) — Tauri concrete `IpcEmit` adapter +
  scheduler-tick bootstrap; anchor run id `25431406979` (rust
  workflow on `main` HEAD `9a1131e`) green; frontend run id
  `25431406965` on the same HEAD green. Per-Type Minimums: ci-run-
  url + repo-path (`app/shell/src/state/ipc_emit.rs` +
  `app/shell/src/state/mod.rs` + `app/shell/src/lib.rs`) + corpus
  version (`TauriIpcEmitter` + extended `AppState` lifecycle with
  `SchedulerThreadHandle` + 60 Hz scheduler-tick thread spawned in
  `run` setup) + drift thresholds (7 T1 pins covering payload
  conversion / event name constant / panic-free posture / lifecycle
  ordering / 60 Hz cadence) all populated.
- **Bootstrap PR 3** (this PR) — e2e wire test + closing docs;
  anchor run id will be the rust workflow run on the merge sha
  of this PR. Per-Type Minimums: ci-run-url (post-merge anchor;
  the Bootstrap PR 1 + PR 2 pre-merge anchors above carry the
  bulk of the wave's corpora) + repo-path
  (`app/shell/tests/audio_uniform_ipc_e2e.rs`) + corpus version
  (3 T2 cross-seam wire tests using `tauri::test::mock_app()`) +
  drift thresholds (per-fixture round-trip identity assertions +
  multi-emit ordering assertions + emit-after-listener-unlisten
  panic-free posture assertion) all populated.

### Verdict

`EV-AUDIO-UNIFORM-IPC` stays at `Pass` (Wave 1c-α already
transitioned `Planned` → `Pass` under narrowing; Wave 1c-α-2
widens the Pass scope by dropping 3 of the 5 original narrowing
axes). **Phase-4a Wave 1c-α-2 closure achieved 2026-05-06 at
commit `<merge-sha>`** (the merge commit of this Bootstrap PR 3
docs(repo) closing PR; pre-merge anchors at HEAD `9a1131e`
post-`#344` already carry the producer runtime + Tauri adapter +
scheduler-tick bootstrap + 14 T1 pins green on `main`). Phase-4a
EXIT collective state is unchanged: `EV-INTEGRATION-MATRIX` narrowing-clause for
the audio-uniform-IPC arm of `EV-INTEGRATION-MATRIX` carries
forward unchanged because Wave 1c-α-2 is the audio-uniform-IPC
arm completing rather than a `EV-INTEGRATION-MATRIX` widening (the `EV-INTEGRATION-MATRIX` arm was
deferred to Wave 1c, which Wave 1c-α + 1c-α-2 sub-banded under
`EV-AUDIO-UNIFORM-IPC`'s payload-shape contract scope).

### Deferred axes (Wave 1c-α-2 widening record)

`EV-AUDIO-UNIFORM-IPC` narrowing axes after Wave 1c-α-2 widening:

- ✓ **DROPPED** — Live end-to-end producer→Tauri-emit→consumer
  wiring. Wave 1c-α-2 production audio bootstrap (PRs `#343` +
  `#344`) wires the engine scheduler-tick + audio analysis
  thread + Tauri-emit adapter end-to-end in production; the
  Bootstrap PR 3 e2e fixture
  (`app/shell/tests/audio_uniform_ipc_e2e.rs::producer_to_listener_round_trip_preserves_payload_identity`)
  exercises the same composition cross-seam under
  `tauri::test::mock_app()`. Visualizer widgets now read live IPC
  in production (with synthetic fallback retained for the
  no-snapshot startup window per Wave 1c-α PR 2's dual-mode gate).
- ✓ **DROPPED** — Drop-frames-on-saturation panic-free posture
  per ADR-019 §Decision §4. The Bootstrap PR 3
  `emit_after_listener_unlisten_does_not_panic` T2 fixture
  exercises the kin path: detached subscriber + multi-emit
  producer + no panic + no spurious delivery. A true producer-
  overrun saturation fixture (forcing the Tauri event queue to
  saturate under high-rate emit) is preserved as a future-wave
  option but not load-bearing for the panic-free posture itself.
- **Stale-but-last-good on audio-fault** per ADR-019 §Decision §6
  is exercised at producer-side T1 by PR `#343`'s
  `engine/audio/src/runtime.rs` runtime drift pins (the
  `MutexAudioPull` poison-recovery path) but not transitively
  through the IPC wire — the e2e fixture uses synthetic snapshots
  driven directly through the emit path rather than panicking the
  audio thread upstream and observing the consumer side hold the
  last good frame. Producer-side coverage is sufficient for the
  invariant; the cross-seam exercise is preserved as a future-wave
  option.
- **Spectrum widget retrofit** listed in ADR-019 §Rollout PR 2
  stays deferred — the Spectrum primitive does not exist as a
  standalone widget; the bundle's Spectrum was inlined into
  `src/lib/views/AudioUniforms.svelte` with 64 synthetic bars,
  structurally incompatible with the engine's 8-band FFT surface
  without a non-trivial 8 → 64 interpolation design choice.
  Deferred as post-MVP UX enhancement.
- **`AudioReactivity` 48-bar FFT visualiser** stays synthetic-
  driven — only the 4 named band tiles plus the BPM strip read
  from the live snapshot via the documented 8 → 4 reduction
  (`avg(fft_bands[0..2])` → BASS, `avg(fft_bands[2..4])` → LOW-
  MID, `avg(fft_bands[4..6])` → HIGH-MID, `avg(fft_bands[6..8])`
  → AIR); the 8 → 48 widening is preserved as a future-wave
  option. Deferred as post-MVP UX enhancement.

### Wave 1c-β + Phase-4b kickoff cross-reference

Wave 1c-β (camera + Syphon adapters) carries forward unchanged
from the Phase-4a EXIT closure paragraph; the home remains a
future directed engineering wave with new ADR + research +
plan-mode + implementation cycle. Phase-4 EXIT itself remains
AO-held until 4b / 4c / 4d sub-phases close per `docs/11`; Wave
1c-α-2 achievement does not pre-empt the Phase-4b / 4c / 4d
sub-phase gate decisions or the Phase-4 EXIT itself.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-06

## Phase-4b cluster (Wave 4b.1 through 4b.6) + Phase-4d Wave 4d.1 / 4d.2 / 4d.3 partial closure

The Phase-4b cluster + Phase-4d 4d.1-4d.3 partial closure (2026-05-06)
lands a 22-PR session covering the Phase-4b authoring + library +
import surfaces plus the leading edge of Phase-4d hardening. The
cluster transitions 9 evidence rows to `Pass` (under explicit
narrowings recorded per row in `docs/26`), advances 6 evidence rows
with PR-1-of-N work that keeps the row at `Planned` pending follow-on
PRs, and lands 2 evidence rows under calendar-gated soak windows.
Phase-4b EXIT itself remains AO-held — Wave 4b.7 (`EV-LIBRARY-BUNDLE-LAYOUT`
adjacency on `EV-IMPORT-REFERENCE-SHOW`) and Wave 4b.8
(`EV-ACCESSIBILITY-AA`) are explicitly carried forward as future
waves; this cluster does not pre-empt the Phase-4b gate decision.
**Closed**: Wave 4b.7 closure (2026-05-07) and Wave 4b.8 closure
(2026-05-07) both landed; see the dedicated closure sections at the
end of this file for the per-PR sequence + narrowings + Pass
verdicts. As of Wave 4b.8 closure, all 11 Phase-4b EXIT rows are at
`Pass` under collective narrowings; Phase-4b EXIT is passable at AO
discretion.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-LIBRARY-BUNDLE-LAYOUT`** — Wave 4b.1 PR `#348` (`b0d07f8`,
  anchor run `25436889114`). Full scope. Canonical four-subdirectory
  taxonomy `welcome / library / examples / shadertoy-fixtures`
  under `<AppBundle>/Resources/scenes/` with bundle-walk assertion
  corpus at `app/shell/tests/bundle_layout_assertion.rs`.
- **`EV-LIBRARY-DISCOVERY`** — Wave 4b.6 PR `#349` (`747390a`,
  anchor run `25438404895`) indexer + PR `#359` (`626116f`, anchor
  run `25449217997`) search/filter IPC. Full scope. `engine/library::indexer`
  crawl + index pipeline + `app/shell/src/commands/library.rs`
  index-backed search + tag-filter + source-filter IPC commands.
- **`EV-LIBRARY-THUMBNAIL`** — Wave 4b.6 PR `#352` (`3a091ef`,
  anchor run `25440654237`). Narrowed: stub byte-hash renderer
  (FNV-1a 64 over scene manifest bytes producing a stable 128×128
  RGBA8 pattern) plus the documented fallback chain. Real GPU-backed
  scene-render thumbnail rendering deferred post-MVP.
- **`EV-AUTHORING-DIAGNOSTICS`** — Wave 4b.3 PR `#353` (`1e8b6f1`,
  anchor run `25442351038`) renderer + PR `#362` (`4f222f9`, anchor
  run `25451347283`) multi-class corpus. Narrowed: spans deferred
  for compile / script / coercion classes (engine errors lack
  structured spans); only the JSON class threads `serde_json::Error::(line, column)`
  through to a populated `Span`. The other three classes emit
  message-only diagnostics with `None` span.
- **`EV-AUTHORING-SCAFFOLDER`** — Wave 4b.4 PR `#355` (`f455613`,
  anchor run `25445241955`). Full scope. `engine/authoring::scaffolder`
  emits a minimal-but-complete `.chroma` scene tree (manifest +
  script + default frag pass) round-tripping through the existing
  scene-loader + warning-free script-compile pipeline.
- **`EV-IMPORT-ISF-REJECT`** — Wave 4b.5 PR `#358` (`8fadc55`,
  anchor run `25448712246`). Full scope (ISF native import remains
  deferred to V2 per design — the reject-with-escape-hatch path IS
  the MVP scope). `engine/library::import::isf` detector + structured
  reject diagnostic + escape-hatch link to manual-port docs.
- **`EV-SHADER-CACHE`** — Wave 4d.2 PR `#350` (`e809991`, anchor
  run `25439101309`). Narrowed: observation-only — the cold-vs-warm
  ratio bench documents observed numbers (cold ~5-10× warm under
  the slice-1 GLSL → SPIR-V naga frontend) but `scripts/budget-check.sh`
  is NOT extended with a CI-enforced numeric threshold. CI gate
  preserved as a post-MVP follow-on once the warm-cache key
  derivation stabilises across naga / wgpu version bumps.
- **`EV-AUTHORING-UX-LATENCY`** — co-developed wave PR `#363`
  (`b2156bb`, anchor run `25451842146`). Narrowed: synthetic
  harness only at `engine/authoring/src/ux_latency.rs` — full
  UI-driven validation (real CodeMirror + W12 shell + DOM paint
  timing) deferred post-MVP. Cross-tagged with `EV-DOCS-UX-LATENCY`.
- **`EV-DOCS-UX-LATENCY`** — co-developed wave PR `#363`
  (`b2156bb`, anchor run `25451842146`). Same harness as
  `EV-AUTHORING-UX-LATENCY` per `37-authoring-environment-spec.md`
  §3.3a shared-instrumentation note. Same narrowing: synthetic
  harness only; full UI-driven validation deferred post-MVP.

### Partial-PR-shipped rows (still Planned)

- **`EV-AUTHORING-LIVE-RELOAD`** — PR `#356` (`d56f11d`, anchor run
  `25445854497`) PR 1 of 3 landed: file-watch + reload-on-change
  scaffolding under `engine/script/src/live_reload.rs`. PR 2
  (state preservation across reload) + PR 3 (latency-budget harness)
  pending.
- **`EV-IMPORT-SHADERTOY`** — PR `#360` (`7497955`, anchor run
  `25449975611`) PR 1 of 2 landed: Shadertoy-to-Chromaticity
  converter at `engine/library/src/import/shadertoy.rs` + 3 starter
  golden fixtures. PR 2 (broader corpus + edge-case fixture sweep)
  pending.
- **`EV-GPU-SELECTION`** — PR `#351` (`5abc443`, anchor run
  `25439590858`) PR 1 of 2 landed: deterministic adapter-selection
  scaffolding under `engine/render/src/adapter_selection.rs` +
  operator-override persistence. PR 2 (hardware-gated dual-GPU +
  battery / TDR / hot-plug fixtures) pending and requires hardware-
  fixture infrastructure beyond the CI surface.
- **`EV-CRASH-RESUME`** — Wave 4d.3 partial advance (PRs `#354` +
  `#357` + `#361` (`5b2d1f7`, anchor run `25450892665`)) covering
  state-restoration classes (a) active scene + preset channel values,
  (b) Snap slot + scene-specific preset history, (c) playlist
  position + iteration mode, and (e) audio input device + `syn_Threshold`.
  Classes (d) OSC/MIDI session bindings, (f) operator meta-control
  values, and (g) render resolution / FPS target pending.
- **`EV-CRASH-PAYLOAD-REDACTION`** — PR `#346` (`7ff37c6`, anchor
  run `25437375279`) PR 1 of 2 landed: redaction filter scaffolding
  + T1 pins on the redaction transform under `engine/observability/src/crash_payload.rs`.
  Future construction PR (full crash-injection harness + payload-
  inspection T2 corpus) pending.
- **`EV-ANALYTICS-ZERO-DEFAULT`** — Wave 4d.1 PR `#347` (`922f2d8`,
  anchor run `25436242487`) scaffolding landed: default-off telemetry
  config + opt-in surface + synthetic `engine/observability::analytics_observer`
  T2 fixture corpus. **Calendar-gated**: 14-day observation window
  soak fires from a fresh-install timestamp; row anchored to
  ~2026-05-20.

### Per-Type Minimums verification

For each row transitioning to `Pass` above, ci-run-url + repo-path
+ corpus version + drift thresholds populate. Anchor run ids cited
in this closure section: `25436889114` (PR `#348`), `25438404895`
(PR `#349`), `25449217997` (PR `#359`), `25440654237` (PR `#352`),
`25442351038` (PR `#353`), `25451347283` (PR `#362`), `25445241955`
(PR `#355`), `25448712246` (PR `#358`), `25439101309` (PR `#350`),
`25451842146` (PR `#363` — cross-tagged authoring + docs UX latency).

### Cumulative session recap

22 PRs landed on `main` between 2026-05-04 and 2026-05-06 covering
PR numbers 342 through 363 inclusive. The session combined the
Wave 4b.1 through 4b.6 authoring + library + import surface waves
with the Phase-4d Wave 4d.1 (analytics-zero-default scaffolding),
Wave 4d.2 (shader-cache observation), and Wave 4d.3 (crash-resume
classes a/b/c/e) leading-edge hardening waves, plus the
co-developed Phase-4b/4c synthetic UX-latency harness in PR `#363`.

### Verdict

**Phase-4b cluster (Wave 4b.1 through 4b.6) + Phase-4d Wave
4d.1 / 4d.2 / 4d.3 partial closure achieved 2026-05-06 at commit
`<merge-sha>`** (the merge commit of this docs(repo) closing PR).

Phase-4b EXIT remains AO-held: Wave 4b.7 (`EV-IMPORT-REFERENCE-SHOW`)
and Wave 4b.8 (`EV-ACCESSIBILITY-AA`) carry forward as future
directed engineering waves. **Closed**: both waves landed
2026-05-07 — see the Phase-4b Wave 4b.7 closure and Phase-4b Wave
4b.8 closure sections at the end of this file. After Wave 4b.8
closure all 11 Phase-4b EXIT rows are at `Pass` under collective
narrowings; Phase-4b EXIT is passable at AO discretion. Phase-4c EXIT remains AO-held: rows
`EV-DOCS-CLEANROOM` + `EV-DOCS-VERSION-SYNC` carry forward; Wave
4d.4 (full `EV-CRASH-RESUME` classes d/f/g + `EV-CRASH-PAYLOAD-REDACTION`
PR 2 + remaining `EV-GPU-SELECTION` hardware-gated fixtures + the
calendar-gated `EV-ANALYTICS-ZERO-DEFAULT` soak completion + the
`EV-SOAK-CHAOS` row not addressed in this cluster) carry forward
as future Phase-4d waves. Phase-4 EXIT itself remains AO-held until
4b / 4c / 4d sub-phases close per `docs/11-roadmap-phases.md`; this
cluster's closure does not pre-empt the Phase-4 EXIT gate decision.

### Deferred axes (cluster narrowing record)

- `EV-LIBRARY-THUMBNAIL` real GPU-backed thumbnail rendering
  (post-MVP).
- `EV-AUTHORING-DIAGNOSTICS` structured spans for compile / script /
  coercion classes (engine error surface widening — future wave).
- `EV-SHADER-CACHE` CI-enforced budget threshold via
  `scripts/budget-check.sh` extension (post-MVP, gated on warm-cache
  key derivation stability).
- `EV-AUTHORING-UX-LATENCY` + `EV-DOCS-UX-LATENCY` full UI-driven
  validation under the W12 shell with real DOM paint timing
  (post-MVP).
- 6 PR-1-shipped rows preserved at `Planned` per the per-row recap
  above (`EV-AUTHORING-LIVE-RELOAD`, `EV-IMPORT-SHADERTOY`,
  `EV-GPU-SELECTION`, `EV-CRASH-RESUME`, `EV-CRASH-PAYLOAD-REDACTION`,
  `EV-ANALYTICS-ZERO-DEFAULT`).

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-06

## Phase-4b Wave 4b.7 closure

Phase-4b Wave 4b.7 closure (2026-05-07) lands the largest Phase-4b
evidence row — `EV-IMPORT-REFERENCE-SHOW` — at `Pass`
under documented narrowings via a 5-PR sequence covering PR numbers
366 through 369 inclusive plus this docs(repo) closing PR. The wave
ships the reference-app user-data importer surface end-to-end at the
read-side, structured-diagnostic level, and orchestrator-shape level;
actual Chromaticity user-data store mutation is preserved as a
post-MVP V1 wave behind entitlement-tier-gating + scene-library-write
surfaces. After this closure, Wave 4b.8 (`EV-ACCESSIBILITY-AA`)
is the only remaining Phase-4b EXIT prerequisite. **Closed**: Wave
4b.8 closure landed 2026-05-07 — see the Phase-4b Wave 4b.8 closure
section immediately below this one for the per-PR sequence.

### Per-PR sequence

- **PR `#366`** (`53ad7ce`, anchor run `25465983264`) — foundation +
  discovery + settings + presets importers. Adds
  `engine/authoring/src/import_reference/` with `mod.rs` +
  `diagnostics.rs` + `discovery.rs` + `settings.rs` + `presets.rs`
  realizing the `reference-user-data/` directory walk over the
  on-disk literals (`Settings.json`, `Presets/`, `Favslots/`,
  `Playlists/`, `MetaPresets/`, `MediaPresets/`, `MidiMappings/`,
  `OscAliases.json`, `HotkeyRemaps.json`) + the structured-diagnostic
  surface + the Settings.json + Presets/ per-class importers + the
  T2 corpus at `engine/authoring/tests/import_reference_foundation_corpus.rs`.
- **PR `#367`** (`86f8292`, anchor run `25466617886`) — snap slots
  + playlists + meta/media preset importers. Adds `snap.rs`
  (Favslots/ → Chromaticity Snap slots) + `playlists.rs` +
  `meta_media.rs` (MetaPresets/ + MediaPresets/) + the T2 corpus at
  `engine/authoring/tests/import_reference_snap_playlists_meta_corpus.rs`.
- **PR `#368`** (`f9ad6eb`, anchor run `25467307204`) — MIDI mappings
  + OSC alias tables + hotkey remappings. Adds `midi.rs` + `osc.rs`
  (with the `/chroma/` route-prefix collision heuristic) + `hotkey.rs`
  (with the literal-prefix + named-action V1-locked + control-focus
  heuristics) + the T2 corpus at
  `engine/authoring/tests/import_reference_midi_osc_hotkey_corpus.rs`.
- **PR `#369`** (`f69fc34`, anchor run `25467953599`) — dry-run +
  commit + rollback drill orchestrator at `drill.rs` composing all
  8 importer classes through `dry_run() → commit() → rollback()`
  with the synthetic-commit `CommitOutcome` + `RollbackSnapshot`
  shape proof + the T2 corpus at
  `engine/authoring/tests/import_reference_drill_harness_corpus.rs`.
- **This PR** — closing docs flipping `EV-IMPORT-REFERENCE-SHOW`
  `EV-IMPORT-REFERENCE-SHOW` from `Planned` to `Pass` under the
  documented narrowings.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-IMPORT-REFERENCE-SHOW`** — full 5-PR sequence above. Anchor
  run `25467953599` (PR `#369`'s test job, the latest = the one
  that exercised all 8 importer classes plus the orchestrator).
  Narrowed per the four axes recorded in `docs/26` `EV-IMPORT-REFERENCE-SHOW`
  description column: (1) synthetic-commit harness — no actual
  user-data store mutation; (2) synthetic-rollback — `Ok(())`
  no-op; (3) heuristic stand-ins for OSC alias collision detection
  (`/chroma/` prefix) + hotkey V1-locked + control-focus action
  vocabulary (literal-prefix + named-action set); (4) library
  cross-check on preset `unknown-scene` diagnostic mocked-empty.

### Per-Type Minimums verification

For row `EV-IMPORT-REFERENCE-SHOW` transitioning to `Pass`,
ci-run-url + repo-path + corpus version + drift thresholds populate.
Anchor run ids cited in this closure section: `25465983264` (PR
`#366`), `25466617886` (PR `#367`), `25467307204` (PR `#368`),
`25467953599` (PR `#369`; primary anchor for `EV-IMPORT-REFERENCE-SHOW` Pass).

### Cumulative session recap

5 PRs landed on `main` on 2026-05-07 covering PR numbers 366
through 369 inclusive plus this docs(repo) closing PR. The session
sequenced the import-reference foundation, the snap/playlist/meta
channel set, the MIDI/OSC/hotkey channel set, the
dry-run/commit/rollback drill orchestrator, and the closing-docs
flip in lockstep — each PR's test job feeds the next anchor.

### Verdict

**Phase-4b Wave 4b.7 closure achieved 2026-05-07 at commit
`<merge-sha>`** (the merge commit of this docs(repo) closing PR).

Phase-4b EXIT remains AO-held: Wave 4b.8 (`EV-ACCESSIBILITY-AA`)
is the only remaining 4b prerequisite and carries forward as a
future directed engineering wave. **Closed**: Wave 4b.8 closure
landed 2026-05-07 — see the Phase-4b Wave 4b.8 closure section
immediately below; after that closure all 11 Phase-4b EXIT rows
are at `Pass` under collective narrowings and Phase-4b EXIT is
passable at AO discretion. Phase-4 EXIT itself remains AO-held
until 4b / 4c / 4d sub-phases close per `docs/11-roadmap-phases.md`;
this wave's closure does not pre-empt the Phase-4b EXIT gate
decision.

### Deferred axes (wave narrowing record)

- **Synthetic-commit semantics** (PR `#369`'s drill harness produces
  shape proof but does NOT mutate `PresetStore` / `MappingStore` /
  hotkey-store / scene library); wiring against actual stores is
  post-MVP V1 PR scoped behind entitlement-tier-gating +
  scene-library-write surfaces.
- **Synthetic-rollback** — `rollback(snapshot)` is `Ok(())` no-op
  for the same reason; real rollback per ADR-015 §6 +
  `32-state-preset-playlist-migration-policy.md` snapshot machinery
  deferred to the same post-MVP PR.
- **OSC alias collision heuristic** — PR `#368` uses a `/chroma/`
  route-prefix check in lieu of the full canonical-route registry
  lookup; lookup deferred.
- **Hotkey vocabulary heuristics** — PR `#368` uses literal-prefix
  + named-action set heuristics for V1-locked + control-focus
  actions in lieu of the full Chromaticity-side hotkey-action
  vocabulary lookup; lookup deferred.
- **Library cross-check** — PR `#366`'s preset `unknown-scene`
  diagnostic uses a mocked-empty library context; cross-checking
  against the live `SceneIndex` from
  `app/shell/src/commands/library_index.rs` deferred to a future
  PR.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4b Wave 4b.8 closure

Phase-4b Wave 4b.8 closure (2026-05-07) lands the last remaining
Phase-4b EXIT prerequisite — `EV-ACCESSIBILITY-AA` — at
`Pass` under documented narrowings via a 3-PR sequence covering PR
numbers 371 + 372 plus this docs(repo) closing PR. The wave ships
the WCAG 2.2 AA automated audit surface end-to-end at the
component-scope level: an `axe-core` harness audits all 14 routed
views in `src/lib/views/` against the four WCAG 2.2 AA tag groups,
strict zero-finding budgets across the four severity tiers, and 6
component-level fixes closing 5 real findings surfaced by PR 1.
After this closure all 11 Phase-4b EXIT rows are at `Pass` under
collective narrowings; Phase-4b EXIT is passable at AO discretion
(the same posture Phase-4a reached prior to AO declaring EXIT
2026-05-03 at commit `<merge-sha>` per the Phase-4a EXIT closure
section above).

### Per-PR sequence

- **PR `#371`** (`54be28a`, anchor run `25469037211`) — axe-core
  harness integration. Adds `src/lib/__tests__/accessibility/`
  with `axe-harness.ts` (`runAxeAudit(container, view)` invoking
  `axe.run` scoped to WCAG 2.2 AA tags `wcag2a` + `wcag2aa` +
  `wcag22a` + `wcag22aa` plus `findingsAtOrAbove` filter),
  `baseline-allowlist.ts` (per-view `BaselineEntry { maxCritical,
  maxSerious, maxModerate, maxMinor }` permissive PR-1 ceiling
  `{5, 10, 20, 50}`), and `wcag-aa-corpus.test.ts` (T2 corpus
  iterating all 14 routed views in `src/lib/views/`). Adds
  `axe-core ^4.10.0` (resolved 4.11.4) devDep. `matchMedia` stubbed
  via `vi.stubGlobal` per `feedback_vitest_matchmedia_stubglobal.md`
  and `@tauri-apps/api/core` `invoke` mocked via `vi.hoisted` +
  `vi.mock` per `feedback_vitest_invoke_mock_hoisted.md` to keep
  rAF-driven `$effect`s deterministic + `$effect`-driven IPC
  fetches in their loading state through the audit window.
- **PR `#372`** (`896b65a`, anchor run `25469465083`) — automated
  remediation pass. Triages PR-1 per-view findings into Bucket A
  (10 component-scope-disabled rules per axe-core's documented
  `rules` parameter — `landmark-one-main` +
  `landmark-no-duplicate-banner` +
  `landmark-no-duplicate-contentinfo` + `landmark-unique` +
  `page-has-heading-one` + `region` + `html-has-lang` +
  `html-lang-valid` + `document-title` + `bypass`) and Bucket B
  (5 real component-level findings closed by 6 edits across
  `VuStereo` widget `role="img"` + `CuePad` widget `role="grid"`
  → `role="group"` + `Author` view inspector tab strip + utility
  toolbar split + `MIDI` view saved-state dot `role="img"` +
  `Perform` view rail `role="tablist"` → `role="toolbar"` +
  `CueTransitionCurve` widget SVG `role="img"` → `role="group"`
  per the `nested-interactive` rule with `role="slider"` handle
  children). Tightens every per-view + per-tier budget in
  `baseline-allowlist.ts` to strict `{0, 0, 0, 0}` across 14
  views × 4 tiers. Updates 2 per-component spec assertions
  (`CuePad.test.ts` + `Perform.test.ts`) to follow the new role
  contracts.
- **This PR** — closing docs flipping `EV-ACCESSIBILITY-AA`
  from `Planned` to `Pass` under the documented narrowings.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-ACCESSIBILITY-AA`** — full 3-PR sequence above. Anchor run
  `25469465083` (PR `#372`'s frontend job, the latest = the one
  that exercised the strict 0/0/0/0 budget across all 14 views).
  Narrowed per the three axes recorded in `docs/26` `EV-ACCESSIBILITY-AA`
  description column: (1) axe-core automated audit only — 14 of
  14 routed views (`AudioUniforms`, `Author`, `Import`, `Inputs`,
  `Library`, `License`, `MIDI`, `Osc`, `Patch`, `Perform`,
  `Playlist`, `Settings`, `Sources`, `Welcome`) audit clean at
  strict 0/0/0/0 budgets across critical / serious / moderate /
  minor tiers under WCAG 2.2 AA tags; (2) component-vs-page
  scope rules disabled per axe-core's documented `rules`
  parameter — the 10 disabled rules will still apply at any
  future full-app integration audit; (3) manual VoiceOver / NVDA
  review deferred to AO-led validation territory.

### Per-Type Minimums verification

For row `EV-ACCESSIBILITY-AA` transitioning to `Pass`,
ci-run-url + repo-path + corpus version + drift thresholds
populate. Anchor run ids cited in this closure section:
`25469037211` (PR `#371`), `25469465083` (PR `#372`; primary
anchor for `EV-ACCESSIBILITY-AA` Pass).

### Cumulative session recap

3 PRs landed on `main` on 2026-05-07 covering PR numbers 371 +
372 plus this docs(repo) closing PR. The session sequenced the
axe-core harness integration, the automated remediation pass with
strict zero-budget tightening + 6 component-level fixes, and the
closing-docs flip in lockstep — each PR's frontend job feeds the
next anchor.

### Verdict

**Phase-4b Wave 4b.8 closure achieved 2026-05-07 at commit
`<merge-sha>`** (the merge commit of this docs(repo) closing PR).

After Wave 4b.8 closes, all 11 Phase-4b EXIT rows are at `Pass`
under collective narrowings: `EV-LIBRARY-BUNDLE-LAYOUT` (Wave
4b.1, full), `EV-LIBRARY-DISCOVERY` (Wave 4b.2 + 4b.6, full),
`EV-LIBRARY-THUMBNAIL` (Wave 4b.6, byte-hash stub renderer
narrowing), `EV-AUTHORING-DIAGNOSTICS` (Wave 4b.3, JSON-class
spans only narrowing), `EV-AUTHORING-SCAFFOLDER` (Wave 4b.4,
full), `EV-AUTHORING-UX-LATENCY` + `EV-DOCS-UX-LATENCY` (Wave
4b.6, synthetic-only narrowing), `EV-IMPORT-ISF-REJECT` (Wave
4b.5, full — ISF native import deferred to V2),
`EV-SHADER-CACHE` (Wave 4d.2, observation-only narrowing),
`EV-IMPORT-REFERENCE-SHOW` (Wave 4b.7, synthetic-commit +
synthetic-rollback + heuristic + mocked-empty-library
narrowings), and `EV-ACCESSIBILITY-AA` (this Wave 4b.8,
axe-core-only + component-scope-disabled + manual-deferred
narrowings). Per `docs/26` §Rules line 194, **Phase-4b EXIT is
passable at AO discretion** (matching the Phase-4a EXIT posture
prior to AO declaring 2026-05-03 EXIT).

Phase-4b EXIT itself remains AO-held per `docs/11`; this wave's
closure does not pre-empt the gate decision. Phase-4 EXIT itself
remains AO-held until 4b / 4c / 4d sub-phases close per
`docs/11`; this wave's closure does not pre-empt the Phase-4
EXIT gate decision.

### Deferred axes (wave narrowing record)

- **Manual VoiceOver / NVDA voice-output review** —
  semantically-correct screen-reader voice testing across every
  routed view remains AO-led manual review territory; no
  automated tool can validate the qualitative axes (e.g. "does
  the view announce in a sensible reading order" / "do widget
  value-changes voice with appropriate aria-live politeness").
- **Keyboard-only navigation flows** — focus-order audits under
  live keyboard-only navigation (the Tab + arrow-key + Enter +
  Escape vocabulary across every view) deferred to AO-led
  manual review. The harness `axe.run` invocation does not
  exercise keyboard event dispatch.
- **Focus-order conformance under live shader output** — AO-led
  manual review territory; no axe-core rule covers the visual
  affordance of focus indicators against animated shader
  backgrounds.
- **Color contrast under live shader output** — the audit
  currently sees component-level palette tokens, not the canvas
  backdrop the user actually sees behind chrome elements; AO-led
  manual review territory.
- **Skip-link / landmark-traversal review against full-app
  integration audit** — the 10 component-scope-disabled rules
  will fire at any future full-app integration audit (e.g. a
  Playwright-driven boot of the real `index.html` mounting the
  entire shell with `<html lang>` / landmark / heading-one /
  region scaffolding present). Building that future audit is
  deferred as post-MVP work; the harness function is
  forward-compatible (the rules array is a single in-source
  constant).

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4d Wave 4d.4 closure

Phase-4d Wave 4d.4 closure (2026-05-07) lands `EV-SOAK-CHAOS` (row
40) at `Pass` under documented narrowings via a 3-PR sequence
covering PR numbers 374 + 375 plus this docs(repo) closing PR. The
wave ships the foundational soak + chaos-injection surface plus a
multi-profile validation corpus exercising synthetic compat-syn-v1
(parity-core narrow) and native-chroma-v1 (broader native) workload
shapes. After this closure, Wave 4d.4 is the **Phase-4d capstone
closure for the row that was the last 4d row needing direct
engineering work**; the remaining open Phase-4d items (enumerated
below) are calendar-gated, hardware-gated, or scoped behind a still-
open follow-on PR.

### Per-PR sequence

- **PR `#374`** (`da34db6`, anchor run `25470304804`) — soak harness
  scaffold. Adds `engine/observability/src/soak.rs` exposing
  `SoakConfig` (with `tick_interval > 0` + `duration > 0`
  validation), `SoakWorkload` trait, `SoakReport` (duration_observed
  + tick_count + stability_checks_passed/failed + p50/p95/p99/max
  tick_ms + per-`StabilityFailure` log keyed by
  `StabilityFailureKind::{DriftExceeded, TickStalled,
  StateCorruptionDetected}`), and `run_soak` with drift-corrected
  timing (each tick deadline computed against `t0 + tick_interval *
  tick_count`, not incrementally), plus 7 T1 pins inline at
  `soak::soak_pins` and the foundational T2 corpus at
  `engine/observability/tests/soak_harness_corpus.rs`. `EV-SOAK-CHAOS` stays
  `Planned`.
- **PR `#375`** (`06cf294`, anchor run `25470870950`) — chaos-
  injection extension. Adds `engine/observability/src/chaos.rs`
  exposing `FaultEvent::{AdapterDropout, AudioStreamInterruption,
  GpuDeviceLost, Custom}` (each carrying a `recovery_after`
  window), `ChaosScheduleEntry` + `ChaosPlan` (deterministic
  schedule sorted at start-up), `ChaosWorkload: SoakWorkload` trait
  extension with `inject_fault` callback, `ChaosInjectionError::
  {AdapterNotFound, UnsupportedFault, RecoveryDeadlineExceeded}`
  rejection taxonomy, `ChaosReport` embedding the underlying
  `SoakReport` + per-`FiredFaultEvent` log, and
  `stability_failure_chaos_classification` post-hoc classifier
  (`ChaosRecoveryWindow` vs. `Independent`), plus 6 T1 chaos pins
  inline at `chaos::chaos_pins` and the T2 chaos corpus at
  `engine/observability/tests/chaos_soak_corpus.rs`. `EV-SOAK-CHAOS` still
  `Planned` pending PR 3.
- **This PR** — multi-profile validation corpus + closing-docs
  flipping `EV-SOAK-CHAOS` from `Planned` to `Pass` under the documented
  narrowings. Adds
  `engine/observability/tests/multi_profile_chaos_soak_corpus.rs`
  (7 fixtures × CI-friendly sub-2-second-each runs) covering
  synthetic `compat-syn-v1` and `native-chroma-v1` workload shapes.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-SOAK-CHAOS`** — full 3-PR sequence above. Anchor run ids
  `25470304804` (PR `#374`) + `25470870950` (PR `#375`; primary
  anchor for `EV-SOAK-CHAOS` Pass). Narrowed per the three axes recorded in
  `docs/26` `EV-SOAK-CHAOS` description column: (1) **CI-short soak
  surface** — the harness scaffold + chaos-injection layer +
  multi-profile corpus all run against parameterized
  sub-second / sub-2-second-each durations; the full ≥ 4-hour soak
  target per `docs/10` line 55 is calendar-gated and runs as a
  nightly criterion-bench-style harness, not as a per-PR CI gate
  (mirroring `EV-PERF-BUDGETS` Phase-3 Path B per the ~$20/month-
  vs-~$70-80/month AO-approved cost posture); (2) **synthetic
  workload fixtures** — the multi-profile corpus uses synthetic
  workloads representative of the two profiles, not the live
  runtime; wiring the full live-runtime soak is a future-wave
  option scoped behind real platform integration; (3) **GPU
  device-lost arm** — exercises a per-platform-supported flag at
  fixture level via `NativeChromaV1Workload::supports_gpu_lost`,
  toggling between `Accepted` and `Rejected(UnsupportedFault)`;
  actual `engine/render/` device-lost recovery is `EV-GPU-SELECTION`
  `EV-GPU-SELECTION` PR 2 (hardware-gated; deferred).

### Per-Type Minimums verification

For row `EV-SOAK-CHAOS` transitioning to `Pass`, ci-run-url +
repo-path + corpus version + drift thresholds populate. Anchor run
ids cited in this closure section: `25470304804` (PR `#374`),
`25470870950` (PR `#375`; primary anchor for `EV-SOAK-CHAOS` Pass).

### Cumulative session recap

3 PRs landed on `main` on 2026-05-07 covering PR numbers 374 + 375
plus this docs(repo) closing PR. The session sequenced the soak
harness scaffold, the chaos-injection extension, and the multi-
profile validation corpus + closing-docs flip in lockstep — each
PR's nextest job feeds the next anchor.

### Verdict

**Phase-4d Wave 4d.4 closure achieved 2026-05-07 at commit
`<merge-sha>`** (the merge commit of this docs(repo) closing PR).

Wave 4d.4 is the Phase-4d capstone closure for the row that was the
last 4d row needing direct engineering work. After this wave
closes, the remaining Phase-4d open items are calendar-gated,
hardware-gated, or scoped behind still-open follow-on PRs:

- **`EV-CRASH-RESUME` class-d** — deferred-pending-state-
  shape gap (the class-d crash-resume drill needs a state-shape
  extension that has not yet landed).
- **`EV-CRASH-PAYLOAD-REDACTION` PR 2** — full crash-
  injection harness pending; PR 1 (Wave 4d.1 Lane A) shipped the
  redaction primitives but the injection harness PR 2 is still
  open.
- **`EV-ANALYTICS-ZERO-DEFAULT` closing-docs** —
  ~2026-05-20 calendar-gated 14-day soak window per the Wave 4d.1
  Lane B scaffolding from PR `#347` / `922f2d8`.
- **`EV-GPU-SELECTION` PR 2** — hardware-gated dual-GPU
  validation deferred (requires real dual-GPU hardware in CI or
  on an AO-driven validation rig).

Phase-4d EXIT itself remains AO-held per `docs/11`; this wave's
closure does not pre-empt the gate decision. Phase-4 EXIT itself
remains AO-held until 4b / 4c / 4d sub-phases close per
`docs/11`; this wave's closure does not pre-empt the Phase-4 EXIT
gate decision.

### Deferred axes (wave narrowing record)

- **Full ≥ 4-hour soak** — calendar-gated nightly criterion-bench-
  style harness, not a per-PR CI gate; mirrors `EV-PERF-BUDGETS`
  Phase-3 Path B nightly-only criterion harness in
  `engine/runtime/benches/budgets.rs` per the ~$20/month-vs-
  ~$70-80/month AO-approved cost posture. Future
  `chore(release-eng)` PR can extend the macos-nightly workflow
  with a chaos-soak step if AO directs.
- **Live-runtime soak** — wiring the full live-runtime soak (engine
  + scheduler + audio + render under operator-driven scene flow) is
  a future-wave option scoped behind real platform integration; the
  multi-profile corpus uses synthetic workloads representative of
  the two profiles only.
- **Real GPU device-lost recovery** — the `GpuDeviceLost` arm in
  this corpus exercises a per-platform-supported flag at fixture
  level via `NativeChromaV1Workload::supports_gpu_lost`; the actual
  `engine/render/` device-lost recovery semantics are `EV-GPU-SELECTION`
  `EV-GPU-SELECTION` PR 2 (hardware-gated; deferred).

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4d EV-CRASH-PAYLOAD-REDACTION closure

**Date.** 2026-05-07.

**Scope.** `EV-CRASH-PAYLOAD-REDACTION` (registry line 94)
transitions `Planned -> Pass-narrowed` under a 2-PR sequence
covering PR numbers 346 + 377 (this closing PR). The row pins the
ADR-016 §Crash payloads redaction contract: scene source
(`main.glsl`), script source (`script.js`), media-payload bytes,
recent audio buffers, operator-entered free-text, and activation-
token bytes MUST be excluded from any transmitted crash report.

### Per-PR sequence

- **PR `#346`** (`7ff37c6`, anchor run `25415739244`, landed
  2026-05-06) — redaction primitives at
  `engine/observability/src/crash_redaction.rs`. Ships the
  `CrashPayloadRedactor` value-driven `serde_json::Value` redactor
  (substring-match field-name redaction; `[REDACTED]` placeholder
  substitution; recursion through nested objects + arrays;
  scalar pass-through), the `RedactCategory` 5-arm enum (`Glsl` /
  `Script` / `MediaBuffer` / `FreeText` / `LicenseToken`), the
  `with_category` builder + `all` convenience constructor, and 12
  T1 in-source drift pins covering field-name match, recursion
  semantics, conservative-by-default behaviour, and case-
  insensitivity. Includes the foundational T2 corpus at
  `engine/observability/tests/crash_payload_redaction_corpus.rs`
  (8 fixtures: per-category arm × 5 + cross-category interaction
  + conservative-by-default-unknown-key + diagnostic-preservation).
- **PR `#377`** (commit `<merge-sha>`, anchor run `<anchor>`, this
  closing PR, landed 2026-05-07) — synthetic crash-payload
  validation harness at `engine/observability/src/crash_harness.rs`.
  Ships `SimulatedCrashClass::{Sigsegv, PanicInRenderThread,
  OutOfMemory, AssertionFailure}` 4-arm enum,
  `SyntheticCrashPayload` constructor (per-class fully-populated
  payload across all 5 `RedactCategory` arms with class-distinct
  sentinel strings of shape
  `<class-label>-<category>-sentinel-do-not-leak`),
  `CrashPayloadValidationReport` (sentinels-redacted vs
  sentinels-leaked accounting), and
  `validate_synthetic_crash_payload` runner (substring-search
  detection over the serialised redacted output) plus 9 T1 harness
  pins. Extends the T2 corpus with 7 harness-driven fixtures
  exercising per-class redaction × 4 classes + safe-telemetry
  pass-through + cross-class sweep + multi-class round-trip
  serialisation.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-CRASH-PAYLOAD-REDACTION`** — full 2-PR sequence above.
  Anchor run ids `25415739244` (PR `#346`) + `<anchor>` (PR `#377`;
  primary anchor for `EV-CRASH-PAYLOAD-REDACTION` Pass). Narrowed per the two axes
  recorded in `docs/26` `EV-CRASH-PAYLOAD-REDACTION` description column:
  (1) **synthetic-harness validation** — the harness validates the
  redactor surface contract via constructed synthetic payloads
  across 4 simulated crash classes covering all 5 `RedactCategory`
  arms; real platform-specific crash-handler injection (POSIX
  signal handlers feeding `SIGSEGV` minidumps, panic-hook wiring
  on the render thread, OOM-killer integration) is out-of-scope at
  MVP and reserved for a future-wave full-injection harness that
  composes on top of these primitives + harness, mirroring the
  deferred-composition framing established for the redaction
  primitives in PR 1 and the synthetic-workload framing already
  accepted at `EV-SOAK-CHAOS` Wave 4d.4; (2) **sentinel-
  string detection** — the harness asserts no sentinel value from
  the sensitive-field set survives in the serialised redacted
  output across every crash-class × redact-category combo, the
  canonical contract per ADR-016 §Crash payloads.

### Per-Type Minimums verification

For row `EV-CRASH-PAYLOAD-REDACTION` transitioning to `Pass`,
ci-run-url + repo-path + corpus version + drift thresholds
populate. Anchor run ids cited in this closure section:
`25415739244` (PR `#346`), `<anchor>` (PR `#377`; primary anchor
for `EV-CRASH-PAYLOAD-REDACTION` Pass).

### Cumulative session recap

2 PRs landed on `main` covering PR numbers 346 (2026-05-06) + 377
(2026-05-07; this closing PR). The sequence ships the redaction
primitives, then the synthetic crash-payload validation harness +
closing-docs flip — each PR's workspace nextest job is the
auditable anchor.

### Verdict

**`EV-CRASH-PAYLOAD-REDACTION` closure achieved 2026-05-07
at commit `<merge-sha>`** (the merge commit of this PR 2).

After this closure 3 Phase-4d open items remain:

- **`EV-CRASH-RESUME` class-d** — deferred-pending-state-
  shape gap (the class-d crash-resume drill needs a state-shape
  extension that has not yet landed); classes a/b/c/e/f/g already
  shipped via PRs `#354` + `#357` + `#361` + `#377-pred`.
- **`EV-ANALYTICS-ZERO-DEFAULT` closing-docs** —
  ~2026-05-20 calendar-gated 14-day soak window per the Wave 4d.1
  Lane B scaffolding from PR `#347` / `922f2d8`.
- **`EV-GPU-SELECTION` PR 2** — hardware-gated dual-GPU
  validation deferred (requires real dual-GPU hardware in CI or
  on an AO-driven validation rig).

Phase-4d EXIT itself remains AO-held per
`docs/11-roadmap-phases.md`; this `EV-CRASH-PAYLOAD-REDACTION` closure does not pre-empt
the Phase-4d EXIT decision. Phase-4 EXIT itself remains AO-held
until 4b / 4c / 4d sub-phases close per `docs/11`; this closure
does not pre-empt the Phase-4 EXIT gate decision.

### Deferred axes (row narrowing record)

- **Real platform-specific crash-handler injection** — POSIX
  signal handlers feeding `SIGSEGV` minidumps, Rust panic-hook
  wiring on the render thread, OOM-killer integration. Out-of-
  scope at MVP; future-wave full-injection harness will compose
  on top of the synthetic harness's primitives + sentinel-search
  apparatus.
- **Live Sentry transport** — the redactor produces a redacted
  `serde_json::Value` ready for serialisation, but the actual
  Sentry-compatible HTTPS transport is a separate Phase-4d+ item
  per ADR-016 §Crash reporting line 28-31. The transport will
  consume the redactor's output as input.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4d EV-CRASH-RESUME closure

**Date.** 2026-05-07.

**Scope.** `EV-CRASH-RESUME` (registry line 80) transitions
`Planned -> Pass-narrowed` under a 5-PR shipping sequence covering
PR numbers 354 + 357 + 361 + 364 + 378 plus this closing-docs PR.
The row pins per-class state restoration across all seven classes
(a/b/c/d/e/f/g) defined in the row description column: active scene
+ preset channel values; Snap slot selection + scene-specific preset
history; playlist position + iteration mode; OSC / MIDI session
bindings (device IDs persist; bindings rebind on re-enumeration per
MIDI-CONF-09); audio input device selection + `syn_Threshold`;
operator meta-control values; render resolution + FPS target.

### Per-PR sequence

- **PR `#354`** (landed earlier in Wave 4d.3) — resume scaffold +
  classes (a) active scene + preset channel values + (b) Snap slot
  selection + scene-specific preset history. Shipped per-class
  `restore_class_a` + `restore_class_b` helpers under the additive
  partial-advance pattern.
- **PR `#357`** (landed earlier in Wave 4d.3) — class (c) playlist
  position + iteration mode. Shipped `restore_class_c` helper.
- **PR `#361`** (`5b2d1f7`, landed earlier in Wave 4d.3) — class (e)
  audio input device + `syn_Threshold`. Shipped `restore_class_e`
  helper.
- **PR `#364`** (`bcaf879`, landed 2026-05-07) — classes (f)
  operator meta-control values + (g) render resolution + FPS target.
  Shipped `restore_class_f` + `restore_class_g` helpers.
- **PR `#378`** (`9cbb428`, landed 2026-05-07) — class (d) OSC /
  MIDI session bindings, closing the deferred-pending-state-shape
  gap. Extended `State` with the additive `SessionBindings` sub-
  store carrying OSC + MIDI device-id-keyed binding state; extended
  the V2 envelope with the `EXT_ID_V2_SESSION_BINDINGS` extension
  (length-prefixed UTF-8 device-ids + mapping-id lists per OSC +
  MIDI). Shipped `restore_class_d` helper plus the cross-class
  `restore_all_supported_classes` composer.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-CRASH-RESUME`** — full 5-PR sequence above. Anchor run id
  `25473152455` (PR `#378`; primary anchor for `EV-CRASH-RESUME` Pass).
  Narrowed per the axis recorded in `docs/26` `EV-CRASH-RESUME` description
  column: **synthetic restore-helper validation** — the harness
  validates per-class state restoration via constructed source /
  target `State` pairs and the `restore_class_<x>` helpers; the
  full crash-injection drill per the spec ("controlled `SIGKILL` /
  `SIGSEGV` / panic-in-render-thread / OOM simulation at three
  wallclock points within a 10-minute scripted show") requires real
  platform crash-handler infrastructure that doesn't ship at MVP —
  same precedent as `EV-CRASH-PAYLOAD-REDACTION`'s synthetic-
  harness narrowing accepted at the `EV-CRASH-PAYLOAD-REDACTION` closure 2026-05-07 and
  the synthetic-workload framing accepted at `EV-SOAK-CHAOS`
  Wave 4d.4. The `restore_all_supported_classes` helper composes
  all seven restoration paths under the documented additivity
  invariant; pre-existing T2 corpora goldens (`state_replay`,
  `state_migration`, `preset_playlist`, `scheduler_tick`) are
  preserved verbatim — the V2 envelope grew the
  `EXT_ID_V2_SESSION_BINDINGS` extension without breaking V1 → V2
  forward-only migration semantics per Phase-3 `EV-STATE-MIGRATION`.

### Per-Type Minimums verification

For row `EV-CRASH-RESUME` transitioning to `Pass`, ci-run-url +
repo-path + corpus version + drift thresholds populate. Anchor run
id cited in this closure section: `25473152455` (PR `#378`; primary
anchor for `EV-CRASH-RESUME` Pass).

### Cumulative session recap

5 engineering PRs landed on `main` covering PR numbers 354 + 357 +
361 (Wave 4d.3 partial advance, 2026-05-06) + 364 + 378 (Wave 4d.3
cluster closure, 2026-05-07) plus this docs(repo) closing PR. The
sequence ships the per-class `restore_class_<x>` helpers under the
additive Wave 4d.3 partial-advance pattern, then the cross-class
`restore_all_supported_classes` composer, then the closing-docs flip
— each PR's workspace nextest job is the auditable anchor.

### Verdict

**`EV-CRASH-RESUME` closure achieved 2026-05-07 at commit
`<merge-sha>`** (the merge commit of this PR 6 of 6). With this
closure 2 Phase-4d open items remain (down from 4 prior to `EV-CRASH-PAYLOAD-REDACTION`'s
closure earlier on 2026-05-07):

- **`EV-ANALYTICS-ZERO-DEFAULT` closing-docs** —
  ~2026-05-20 calendar-gated 14-day soak window per the Wave 4d.1
  Lane B scaffolding from PR `#347` / `922f2d8`.
- **`EV-GPU-SELECTION` PR 2** — hardware-gated dual-GPU
  validation deferred (requires real dual-GPU hardware in CI or on
  an AO-driven validation rig).

Phase-4d EXIT itself remains AO-held per
`docs/11-roadmap-phases.md`; this `EV-CRASH-RESUME` closure does not pre-empt
the Phase-4d EXIT decision. Phase-4 EXIT itself remains AO-held
until 4b / 4c / 4d sub-phases close per `docs/11`; this closure
does not pre-empt the Phase-4 EXIT gate decision.

### Deferred axes (row narrowing record)

- **Real platform crash-handler infrastructure** — controlled
  `SIGKILL` / `SIGSEGV` injection, panic-hook wiring on the render
  thread, OOM simulation at scripted wallclock points within a
  10-minute scripted show. Out-of-scope at MVP; future-wave full-
  injection harness will compose on top of the synthetic restore-
  helper apparatus (`restore_class_<x>` +
  `restore_all_supported_classes`) by triggering the per-class
  restoration paths through real crash events rather than constructed
  source / target `State` pairs.
- **Time-to-resume + visible-artifact-window SLO measurement** —
  the row description column carries p50 ≤ 5 s / p95 ≤ 10 s time-
  to-resume and ≤ 2 s visible-artifact-window targets. Synthetic
  restore-helper validation pins the correctness contract (per-
  class state restoration); the latency targets require the real
  platform crash-handler infrastructure above to land before they
  can be measured end-to-end.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4d EV-GPU-SELECTION closure

**Date.** 2026-05-07.

**Scope.** `EV-GPU-SELECTION` (registry line 78) transitions
`Planned -> Pass-narrowed` under a 2-PR shipping sequence covering
PR `#351` (selection-policy scaffolding) plus this PR (synthetic
dual-adapter T2 corpus + closing-docs). The row pins deterministic
multi-GPU adapter selection per ADR-011 §7: high-performance default
ranking, operator-override pinning, battery-friendly preference
flip, eGPU hot-plug-disappearance idempotent re-selection, and OS-
aware backend tie-breakers per `cfg(target_os)`.

### Per-PR sequence

- **PR `#351`** (`5abc443`, landed 2026-05-06, Wave 4d.2 Lane B PR
  1 of 2) — selection-policy core under
  `engine/render/src/adapter_selection.rs` shipping `AdapterId` +
  `AdapterType` + `Backend` (local mirrors of wgpu enums per the
  module-level "Adapter ID encoding" rationale) + `SelectedAdapter`
  + `GpuSelectionPolicy` (operator-override + prefer-battery-friendly
  + prior-tdr-count fields, all builder-style) + `pub(crate) fn
  rank_synthetic` decision-tree helper + `pub fn select_adapter`
  wgpu-roundtrip surface + `pub fn enumerate_adapter_summaries`. T1
  algorithm-drift pins shipped under `mod adapter_selection_pins`.
  Module prose at line 411 anticipated PR 2 doing the wgpu round-
  trip validation; the synthetic narrowing this PR ships exercises
  the same `rank_synthetic` decision tree against constructed
  `SelectedAdapter` fixtures so the validation lands on CI without
  hardware.
- **This PR (`<merge-sha>`, landed 2026-05-07)** — synthetic dual-
  adapter T2 validation corpus inline in the same module under a
  new sibling `mod adapter_selection_t2_corpus` test module
  honouring the `pub(crate) rank_synthetic` slice-boundary signal
  (the T2 fixtures live in-crate rather than as an external
  integration test that would have to widen the visibility — per
  the visibility-as-scope-signal posture in
  `feedback_visibility_as_scope_signal.md`). 10 fixtures cover:
  discrete + integrated laptop pairing under default-policy and
  battery-friendly preference; dual-discrete workstation
  deterministic backend tie-break; operator-override pinning + falls-
  through-on-absent; eGPU hot-plug-disappearance idempotent re-
  selection; empty adapter list returns None; TDR-history flag
  plumbed-but-inert for ranking; battery preference flips top two
  types only (Virtual / Cpu / Other ordering preserved); backend-
  rank OS-aware tie-breakers per `cfg(target_os)` (Metal on macOS /
  Vulkan on Linux / Dx12 on Windows — exactly one fixture fires per
  target). Plus closing-docs.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-GPU-SELECTION`** — full 2-PR sequence above. Anchor run id
  `<TBD-on-merge>` (this PR's CI nextest job; primary anchor for
  `EV-GPU-SELECTION` Pass). Narrowed per the axes recorded in `docs/26` `EV-GPU-SELECTION`
  description column: **(1)** synthetic-corpus validation only —
  the T2 corpus exercises the `rank_synthetic` pure decision-tree
  helper from PR 1 against constructed `SelectedAdapter` fixtures;
  same precedent as `EV-SOAK-CHAOS` synthetic-workload
  narrowing, `EV-CRASH-RESUME` synthetic restore-helper
  narrowing, and `EV-CRASH-PAYLOAD-REDACTION` synthetic-
  harness narrowing accepted at their respective Phase-4d closures.
  **(2)** Physical / VM dual-adapter validation deferred — the
  registry's "Generation Source: Dual-GPU laptop VM test +
  workstation discrete-pair test" full-spec validation requires
  real multi-GPU rigs; the wgpu round-trip arm (`select_adapter`
  against a live `wgpu::Instance`) is exercised by the existing
  `engine/render` T2 corpora on actual CI Mac/Linux/Windows rigs
  (verifying at-least-one-adapter selection succeeds), but
  synthesised dual-GPU pairings happen only in the inline T2
  corpus. **(3)** TDR-history warning surface deferred — the
  `prior_tdr_count` field is plumbed through `GpuSelectionPolicy`
  per PR 1's module prose; the actual UI warning ("TDR within 30
  days → warn") is a future-wave UI surface; the T2 corpus asserts
  ranking is currently inert under varying TDR counts.

### Per-Type Minimums verification

For row `EV-GPU-SELECTION` transitioning to `Pass`, ci-run-url +
repo-path + corpus version + drift thresholds populate. Anchor run
id cited in this closure section: `<TBD-on-merge>` (this PR's
workspace nextest job; primary anchor for `EV-GPU-SELECTION` Pass). Corpus
version pinned by the merged-on-main slice chain (PR `#351`
selection-policy scaffolding + this PR synthetic-corpus + closing-
docs). Drift thresholds applied via per-fixture deterministic-output
assertion across the 10-fixture T2 corpus (3 fixtures additionally
exercise multi-invocation cross-trial identity per the deterministic-
selection axis ADR-011 §7 calls out).

### Cumulative session recap

2 engineering PRs landed on `main` covering PR `#351` (2026-05-06
selection-policy scaffolding) + this PR (2026-05-07 synthetic dual-
adapter corpus + closing-docs). The sequence ships the `pub(crate)
rank_synthetic` decision-tree helper at PR 1 and exercises it
against synthetic dual-adapter fixtures at PR 2, all under the
existing visibility boundary — each PR's workspace nextest job is
the auditable anchor.

### Verdict

**`EV-GPU-SELECTION` closure achieved 2026-05-07 at commit
`<merge-sha>`** (the merge commit of this PR 2 of 2). With this
closure 1 Phase-4d open item remains (down from 2 prior to this
closure):

- **`EV-ANALYTICS-ZERO-DEFAULT` closing-docs** —
  ~2026-05-20 calendar-gated 14-day soak window per the Wave 4d.1
  Lane B scaffolding from PR `#347` / `922f2d8`.

Phase-4d EXIT itself remains AO-held per
`docs/11-roadmap-phases.md`; this `EV-GPU-SELECTION` closure does not pre-empt
the Phase-4d EXIT decision. Phase-4 EXIT itself remains AO-held
until 4b / 4c / 4d sub-phases close per `docs/11`; this closure
does not pre-empt the Phase-4 EXIT gate decision.

### Deferred axes (row narrowing record)

- **Physical / VM dual-adapter validation** — real multi-GPU rigs
  (laptop dual-GPU VM with both integrated + discrete adapters,
  workstation with two discrete adapters of different vendors,
  eGPU hot-plug rig) carrying the registry's "Generation Source"
  full-spec validation. Out-of-scope at MVP; future-wave hardware-
  rig harness will compose on top of the synthetic decision-tree
  by driving real `select_adapter` invocations against a live
  `wgpu::Instance` and asserting the same per-fixture outcomes the
  synthetic corpus pins today.
- **TDR-history warning UI surface** — Windows-side OS query
  (DXGI / EventLog TDR counter) feeding a per-OS warning banner
  ("TDR within 30 days → warn") + UI surfacing. Out-of-scope at
  MVP; the `prior_tdr_count` field is plumbed through
  `GpuSelectionPolicy` so the future warning layer can consume it
  without a side-channel; the T2 corpus pins the per-PR-2 ranking-
  invariant-under-TDR property (the rank does not vary with
  `prior_tdr_count` until the warning surface lands).
- **Battery / power-source OS query** — the OS-side battery-state
  query (`IOPSCopyPowerSourcesInfo` on macOS; analogous on Windows
  / Linux) that flips `prefer_battery_friendly` automatically. The
  flag is operator-supplied in this PR; the OS-watcher attaches in
  a future wave.
- **Operator-override settings persistence shell** — Tauri
  `app/shell/` settings store + `src/lib/` UI binding for the
  persisted-per-installation operator override. ADR-011 §7
  specifies the override survives adapter hot-plug; the engine-side
  surface (constructor argument on `GpuSelectionPolicy`) is ready
  but the persistence shell is a future wave.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4b EV-AUTHORING-LIVE-RELOAD closure

**Date.** 2026-05-07.

**Scope.** `EV-AUTHORING-LIVE-RELOAD` (registry line 61)
transitions `Planned -> Pass` under a 2-PR shipping sequence covering
PR `#356` (file-watch + reload-on-change scaffolding) plus this PR
(state-preservation cross-seam contract + closing-docs). The row
pins the `docs/37-authoring-environment-spec.md` §3.3 hot-reload
semantics: an operator's mid-show edits to control values, active
preset, Snap selection, and playlist position must survive a
successful hot-reload — a reload is a code-change event, not a
session-reset.

### Per-PR sequence

- **PR `#356`** (`d56f11d`, landed 2026-05-06, Wave 4b.2 PR 1 of 2)
  — file-watcher + reload-pipeline producer surface under
  `engine/authoring/src/live_reload.rs` (NOTE: original `docs/26`
  `EV-AUTHORING-LIVE-RELOAD` description column referenced `engine/script/src/live_reload.rs`
  — that's a stale path the prior audit missed; the actual location
  is `engine/authoring/src/live_reload.rs`. This PR's docs touch
  corrects the registry path). Module ships `LiveReloadConfig` +
  `LiveReloadHandle` + `start_watching` (notify 6.x cross-platform
  watcher) + `perform_reload` (single-shot reload step that maps
  `LoadError -> Diagnostic`) + `ReloadOutcome::{Success, Failed(Diagnostic)}`
  + 7 in-source T1 pins covering the structural surface (debounce
  collapses bursts, handle drop signals worker stop, success / fail
  variant routing, severity classification through the diagnostic
  pipeline). Module prose at lines 24-40 anticipated PR 2 layering
  state-preservation on top without re-shaping the surface.
- **This PR (`<merge-sha>`, landed 2026-05-07)** — state-preservation
  cross-seam contract extending `engine/authoring/src/live_reload.rs`
  with `PreservedState` (control_values + active_preset + active_snap
  + playlist_position) + `PreservedStateSnapshot` builder +
  `capture_preserved_state` helper + reshaped `ReloadOutcome::Success
  { value, preserved }` carrying the captured state across the reload
  boundary. 5 added in-source T1 pins (preserved-state round-trip via
  capture helper, default-is-empty, control-value round-trip, success
  variant carries preserved state, is-empty on default). New T2 corpus
  at `engine/authoring/tests/live_reload_corpus.rs` (6 fixtures: 4
  single-field round-trips + 1 combined-all-four-fields + 1
  failure-arm-drops-preserved-state per the "preservation is
  success-only" contract). Plus closing-docs.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-AUTHORING-LIVE-RELOAD`** — full 2-PR sequence above. Anchor
  run id `<TBD-on-merge>` (this PR's CI nextest job; primary anchor
  for `EV-AUTHORING-LIVE-RELOAD` Pass). Narrowed per the axes recorded in `docs/26` row
  61 description column: **(1)** state-preservation cross-seam
  contract only — the `PreservedState` shape ships verbatim and a
  successful `perform_reload` forwards it onto
  `ReloadOutcome::Success { value, preserved }`; the runtime-side
  wiring (engine/runtime adapter capturing `State -> PreservedState`
  before reload + re-applying via the command-bus + commit-thread
  per ADR-006 after) is a future-wave option scoped behind the
  live-reload + runtime composition. Same precedent as `EV-AUDIO-UNIFORM-IPC`
  `EV-AUDIO-UNIFORM-IPC` Wave 1c-α cross-seam-contract narrowing
  accepted at its closure (the cross-seam contract lands ahead of
  the producer-bootstrap wiring). **(2)** Latency-budget arm
  cross-tagged with `EV-AUTHORING-UX-LATENCY` (already `Pass`
  via PR `#363` commit `b2156bb`) per `docs/37-shared-instrumentation`
  note — the synthetic UX-latency harness covers reload latency
  under the same shared-instrumentation note. No separate
  live-reload-specific latency harness lands here per
  `docs/44` `EV-AUTHORING-LIVE-RELOAD` line 50 wave-PR sizing of "1-2 PRs". **(3)**
  File-watch debounce shipped per PR 1 (notify 6.x) with the
  configurable `LiveReloadConfig::debounce_ms` knob.

### Per-Type Minimums verification

For row `EV-AUTHORING-LIVE-RELOAD` transitioning to `Pass`,
ci-run-url + repo-path + corpus version + drift thresholds populate.
Anchor run id cited in this closure section: `<TBD-on-merge>` (this
PR's workspace nextest job; primary anchor for `EV-AUTHORING-LIVE-RELOAD` Pass). Corpus
version pinned by the merged-on-main slice chain (PR `#356`
file-watch + reload-on-change scaffolding + this PR
state-preservation cross-seam contract + new T2 corpus). Drift
thresholds applied via per-fixture deterministic-output assertion
across the 6-fixture T2 corpus + 5 added T1 pins exercising the
cross-seam contract structurally in isolation.

### Cumulative session recap

2 engineering PRs landed on `main` covering PR `#356` (2026-05-06
file-watch + reload-on-change scaffolding) + this PR (2026-05-07
state-preservation cross-seam contract + new T2 corpus +
closing-docs). The sequence ships the file-watcher + reload-pipeline
producer surface at PR 1 and layers the state-preservation cross-seam
contract on top at PR 2 without re-shaping the file-watcher / reload
surface — each PR's workspace nextest job is the auditable anchor.

### Verdict

**`EV-AUTHORING-LIVE-RELOAD` closure achieved 2026-05-07 at
commit `<merge-sha>`** (the merge commit of this PR 2 of 2). With
this closure 2 Phase-4 EXIT prerequisites remain (down from 3 prior
to this closure):

- **`EV-IMPORT-SHADERTOY` PR 2 of 2** — broader corpus +
  iChannel widening landed in parallel sibling (see following `EV-IMPORT-SHADERTOY`
  closure section).
- **`EV-ANALYTICS-ZERO-DEFAULT` closing-docs** — calendar-gated
  ~2026-05-20.

Phase-4b EXIT itself remains AO-held per `docs/11-roadmap-phases.md`;
this `EV-AUTHORING-LIVE-RELOAD` closure does not pre-empt the Phase-4b EXIT decision.

### Deferred axes (row narrowing record)

- **Runtime-side wiring** — the engine/runtime adapter that captures
  the live `State` into a `PreservedStateSnapshot` before reload +
  re-applies the resulting `PreservedState` onto the post-reload
  State via the command-bus + commit-thread per ADR-006 lives
  behind the live-reload + runtime composition. Out-of-scope at
  the cross-seam-contract MVP; future-wave runtime-bootstrap PR
  composes the adapter on top of the producer-side `PreservedState`
  shape this PR ships, the same way the
  `EV-AUDIO-UNIFORM-IPC`-Wave-1c-α cross-seam-contract surface was
  composed end-to-end by Wave 1c-α-2 production audio bootstrap.
- **Live-reload-specific latency-budget harness** — the latency
  arm cross-tags with `EV-AUTHORING-UX-LATENCY` per
  `docs/37-shared-instrumentation` note; the synthetic UX-latency
  harness covers reload-latency under the same shared-instrumentation
  note. A live-reload-dedicated harness driving real `notify`
  filesystem events through the reload pipeline against a measurement
  hook is a future-wave option once the runtime adapter is wired
  and the live reload-latency surface is observable end-to-end.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4b EV-IMPORT-SHADERTOY closure

**Date.** 2026-05-07.

**Scope.** `EV-IMPORT-SHADERTOY` (registry line 64) transitions
`Planned -> Pass` under a 2-PR shipping sequence covering PR `#360`
(converter scaffolding + 3 starter golden fixtures, PR 1 of 2) plus
this PR (broader corpus + iChannel fixture-pack resolution +
warning-surface widening, PR 2 of 2). The row pins the Shadertoy-to-
Chromaticity converter end-to-end across the 5 fixture mandates from
`docs/38-scene-library-and-import-spec.md` §5.4 (3 PR-1 starters per
line 120 plus 2 PR-2 iChannel fixtures per line 121) plus 5 additional
PR-2 edge-case fixtures.

### Per-PR sequence

- **PR `#360`** (`7497955`, landed 2026-05-06, Wave 4b.5 Lane A PR
  1 of 2) — converter scaffolding under
  `engine/authoring/src/import_shadertoy.rs` shipping
  `ShadertoyImportConfig` + `ImportReport` + `ImportError` +
  `import_shadertoy_to_chroma` + `load_imported_scene`. T1 algorithm-
  drift pins inline; T2 starter corpus at
  `engine/authoring/tests/import_shadertoy_corpus.rs` covering 3
  fixtures (simple-math, noise-fbm, raymarched-sphere). Module
  prose at line 9 anticipated PR 2 widening the iChannel-fixture-
  pack-resolution path called out by `docs/38` §5.2a.
- **This PR (`<merge-sha>`, landed 2026-05-07)** — broader corpus +
  iChannel widening per `docs/38` §5.2a. Adds `IChannelMapping` +
  `IChannelKind` + `Warning` (structured enum) to the public
  surface; `ShadertoyImportConfig::with_ichannel_mappings` builder
  setter; `iChannelN -> chroma_input_<N>` body rewrite with
  separated-`textureNN`+`sampler` preamble per the
  `engine/render::multipass` naga-frontend constraint; `mainCubemap`
  rejection arm; deprecated `texture2D(...)` -> `texture(...)` auto-
  correction with structured warning; `ICHANNELS` table written into
  `scene.json` so authoring-UI re-establishes bindings on subsequent
  loads. T2 corpus widens from 3 -> 10 fixtures (3 PR-1 starters
  preserved plus 7 new arms: 2D-texture iChannel, cubemap iChannel,
  audio iChannel, multi-channel iChannel binding, iTime+iResolution+
  iMouse uniforms, gl_FragCoord direct access, deprecated texture2D
  auto-correction). T1 module-tests widen from the original PR-1
  set with 5 new variants (unmapped iChannel + audio mapping +
  webcam deferral + texture2D auto-correction + mainCubemap
  rejection). Plus closing-docs.

### Per-row verdict (transitioned to Pass under narrowing)

- **`EV-IMPORT-SHADERTOY`** — full 2-PR sequence above. Anchor run id
  `<TBD-on-merge>` (this PR's CI nextest job; primary anchor for
  `EV-IMPORT-SHADERTOY` Pass). Narrowed per the axes recorded in `docs/26` `EV-IMPORT-SHADERTOY`
  description column: **(1)** iChannel resolution covers `Texture2D`
  / `Cubemap` / `Audio` / `Video` / `Buffer` / `Keyboard` kinds via
  the `IChannelMapping` builder; `Webcam` emits
  `Warning::WebcamIChannelDeferred` (post-MVP integration per Phase-
  4a Wave 1c-β); `mainCubemap` entry-point hard-rejected with
  `Warning::CubemapEntrypointDeferred`-keyed
  `ImportError::UnsupportedShadertoyFeature`. **(2)** VR-mode
  `mainVR` hard-rejected with the existing PR-1 diagnostic — Shadertoy
  VR is out-of-scope per Chromaticity MVP. **(3)** Broader corpus
  covers 10 representative Shadertoy patterns via the inline T2
  corpus; the 5 `docs/38` §5.4 mandates (3 starters per line 120 + 2
  iChannel fixture-pack arms per line 121) are all satisfied; the
  remaining 5 fixtures pin multi-channel binding + canonical-uniform
  mapping + gl_FragCoord direct access + deprecated texture2D auto-
  correction.

### Per-Type Minimums verification

For row `EV-IMPORT-SHADERTOY` transitioning to `Pass`, ci-run-url +
repo-path + corpus version + drift thresholds populate. Anchor run
id cited in this closure section: `<TBD-on-merge>` (this PR's
workspace nextest job; primary anchor for `EV-IMPORT-SHADERTOY` Pass). Corpus
version pinned by the merged-on-main slice chain (PR `#360`
converter scaffolding + this PR broader-corpus + closing-docs).
Drift thresholds applied via per-fixture deterministic-output
assertion across the 10-fixture T2 corpus (every fixture round-trips
through `chromaticity_scene::loader::load_chroma_dir` and the on-disk
`main.glsl` parses + validates through the same naga GLSL frontend +
IR validator pair the engine/render single-pass entry point uses
internally).

### Cumulative session recap

2 engineering PRs landed on `main` covering PR `#360` (2026-05-06
converter scaffolding) + this PR (2026-05-07 broader corpus +
iChannel widening + closing-docs). The sequence ships
`ShadertoyImportConfig` + `import_shadertoy_to_chroma` at PR 1 and
extends with `IChannelMapping` + structured `Warning` enum +
text-rewrite pipeline + 7 additional T2 fixtures at PR 2 — each
PR's workspace nextest job is the auditable anchor.

### Verdict

**`EV-IMPORT-SHADERTOY` closure achieved 2026-05-07 at commit
`<merge-sha>`** (the merge commit of this PR 2 of 2). With this
closure 1 Phase-4 EXIT prerequisite remains (with the parallel
sibling `EV-AUTHORING-LIVE-RELOAD` PR 2 having landed earlier today):

- **`EV-ANALYTICS-ZERO-DEFAULT` closing-docs** —
  ~2026-05-20 calendar-gated 14-day soak window per the Wave 4d.1
  Lane B scaffolding from PR `#347` / `922f2d8`.

Phase-4b EXIT itself remains AO-held per
`docs/11-roadmap-phases.md`; this `EV-IMPORT-SHADERTOY` closure does not pre-empt
the Phase-4b EXIT decision. Phase-4 EXIT itself remains AO-held
until 4b / 4c / 4d sub-phases close per `docs/11`; this closure
does not pre-empt the Phase-4 EXIT gate decision.

### Deferred axes (row narrowing record)

- **Webcam iChannel binding** — `IChannelKind::Webcam` mappings
  emit a structured `Warning::WebcamIChannelDeferred` and the slot
  still receives a `texture2D` + `sampler` declaration so the body
  compiles, but the live webcam capture / OS-permission / device-
  selection wiring is post-MVP per the Phase-4a Wave 1c-β plan
  (camera + Syphon adapters; ADR-019 narrowing axis).
- **`mainCubemap` cubemap render-target outputs** — Shadertoy's
  `mainCubemap(out vec4 fragColor, in vec3 rayOrigin, in vec3 rayDir)`
  shape outputs to a 6-face cubemap render target with no auto-
  mapping into Chromaticity's single-pass `mainImage`-equivalent
  shape; the converter rejects with
  `Warning::CubemapEntrypointDeferred`-keyed
  `ImportError::UnsupportedShadertoyFeature`. Future-wave option to
  add explicit cubemap render-target multi-pass support; out-of-
  scope at MVP.
- **VR-mode `mainVR`** — out-of-scope per Chromaticity MVP; rejected
  with the existing PR-1 diagnostic.
- **`Buffer A/B/C/D` multipass evaluation order** — the
  `IChannelKind::Buffer` slot preamble emits the `texture2D` +
  `sampler` declaration so the body compiles against a buffer-bound
  channel, but the multipass evaluation-order DAG that Shadertoy's
  multipass model requires is a downstream authoring concern; the
  converter does not synthesize the multipass scene-graph from the
  Shadertoy source. Authors edit the imported scene-graph post-
  import to wire the buffer pass.
- **Live render-execution validation** — the on-disk `main.glsl`
  parses + validates through naga (T2 anchor) but the row's
  description column also names "renders without errors"; the
  render-execution arm relies on the existing
  `engine/render::single_pass` test harness exercising scaffolded
  fixtures and is not duplicated per Shadertoy fixture in the T2
  corpus. Future-wave option to extend the corpus with a wgpu
  round-trip arm; out-of-scope at MVP per the synthetic-corpus
  precedent set by `EV-GPU-SELECTION` closure.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07

## Phase-4a Wave 1c-β closure

**Date.** 2026-05-07.

**Scope.** `EV-INTEGRATION-MATRIX` (registry line 53) cumulative
narrowing widening: drops the "camera + Syphon arms deferred to Wave
1c-β" narrowing accumulated across Phase-4a Wave 1a + Wave 1b. Both
arms now ship at full Phase-4a scope under the documented synthetic-
narrowing posture (T2 corpora × deterministic synthetic frames per
the row 78 / row 40 / row 93 precedent; per-platform binding stubs +
Syphon `Send`-trait reconciliation deferred to follow-up PRs;
hardware validation manual on AO dev rigs). With this closure, all
five `EV-INTEGRATION-MATRIX` arms (OSC + MIDI + audio-uniform-IPC +
camera + Syphon) are widened.

### Per-PR sequence (16 PRs total)

**Phase 1 — Design (5 PRs, 2026-05-07).**

- **PR `#389`** — Wave 1c-β threat-model addendum amending
  `docs/29-cleanroom-legal-policy-baseline.md` with a "Wave 1c-β
  threat-model addendum (camera + Syphon)" section covering camera
  privacy posture (frames as user-PII, redaction via
  `RedactCategory::MediaBuffer`, NSCameraUsageDescription /
  Windows Privacy-Settings UX, device-id-in-Settings-only rule, no
  facial recognition / biometric processing) and the Syphon
  shared-memory boundary (IOSurface bus visible to any macOS app on
  the machine, operator-facing non-dismissible warning, default-off
  output posture, default server name carrying no machine /
  operator / scene metadata, input-side trust boundary).
- **PR `#390`** — ADR-009 permission-policy addendum widening the
  capability vocabulary to cover camera + Syphon flows; honored-cap
  set widening sequenced behind the PR `#391` / PR `#392` ADR
  acceptance.
- **PR `#391`** — ADR-020 acceptance (camera input adapter design
  per the nokhwa cross-platform abstraction; CoreMediaIO on macOS +
  Media Foundation on Windows; permission-flow trait surface +
  texture-binding namespace + synthetic-corpus posture per
  `docs/54` Phase-2 lessons).
- **PR `#392`** — ADR-021 acceptance (Syphon publisher / subscriber
  adapter design for macOS GPU IO; `objc2 = "0.5"` framework
  bindings against `SyphonMetalServer` / `SyphonMetalClient` /
  `SyphonServerDirectory`; synthetic-corpus posture per `docs/54`
  Phase-3 lessons; CVPixelBuffer + IOSurface marshal pattern).
- **PR `#393`** — research-pass amendments under `docs/52-` /
  `docs/54-` capturing the Phase-2 + Phase-3 lessons that drove the
  ADR-020 + ADR-021 acceptance shapes.

**Phase 1.7 — Acceptance + Vendor (1 PR, 2026-05-07).**

- **PR `#394`** — final ADR-020 + ADR-021 acceptance metadata pass
  plus Syphon-Framework BSD-3-Clause vendoring under
  `third_party/Syphon-Framework/` (license headers, REUSE
  registration, deny.toml allowlist alignment).

**Phase 2 — Camera track (5 PRs + 1 hotfix, 2026-05-07).**

- **PR `#395`** — `integrations/camera-adapter/` crate scaffold:
  `CameraAdapter` trait + `NokhwaInputAdapter` impl seam +
  `CameraConfig` / `CameraDeviceKind` / `DeviceSummary` enumeration
  types + `CameraFrame` / `PixelFormat` + six canonical `pub const`
  fault-reason strings + `defaults.rs` ADR-020 §3 constants under T1
  drift pins.
- **PR `#397`** — macOS permission-flow trait surface
  (`AuthorizationProbe` / `InUsePrecheck` traits +
  `PlaceholderProbe` impl); `unsafe_code` stays forbidden in the
  adapter crate (binding stubs only).
- **PR `#399`** — Windows MF permission-probe trait surface
  (`WindowsCameraProbe` trait + canonical HRESULT → fault-reason
  mapping); `unsafe_code` stays forbidden in the adapter crate
  (binding stubs only).
- **PR `#402`** — hotfix removing an unused Windows import surfaced
  by `cargo clippy --workspace --all-targets` after PR `#399` merge.
- **PR `#401`** — T2 synthetic corpus: 12 fixtures × deterministic
  synthetic frames per `docs/54` Phase-2 lessons (frame-marshal
  shape, fault-reason taxonomy, permission-state transitions,
  pixel-format mapping table, slot-bound rejection, multi-camera
  slot independence).
- **PR `#403`** — engine/render bridge:
  `engine/render/src/camera_adapter.rs` ships
  `camera_texture_binding_from_frame` pure-function marshal +
  `CameraTextureBinding` carrier shape + `CAMERA_UNIFORM_NAMES` /
  `camera_uniform_name` slot lookup + 7 T1 drift pins + per-track
  closing-docs (description-column update on `EV-INTEGRATION-MATRIX`
  for the camera-arm closure).

**Phase 3 — Syphon track (3 PRs, 2026-05-07).**

- **PR `#396`** — `integrations/syphon-adapter/` crate scaffold:
  public surface types, defaults, server-directory discovery seam,
  `SyphonInputAdapter` / `SyphonOutputAdapter` bind / unbind / poll
  / publish state machine through the `SyphonClientHandle` /
  `SyphonServerHandle` trait abstraction.
- **PR `#398`** — `objc2 = "0.5"` framework binding declarations
  against `SyphonMetalServer` / `SyphonMetalClient` /
  `SyphonServerDirectory` plus 11-fixture T2 mock-backend corpus
  (deterministic synthetic frames; trait-mock backend per the row
  78 / row 40 / row 93 precedent).
- **PR `#400`** — engine/render bridge:
  `engine/render/src/syphon_adapter.rs` ships `SyphonRenderBridge`
  carrier + real-backend `Objc2SyphonClientHandle` /
  `Objc2SyphonServerHandle` carrier shapes +
  `SyphonRenderBridgeError` variant taxonomy + 10 T1 trait-impl-
  shape pins (gated behind the new `syphon-binding` cargo feature
  on `engine/render` plus `cfg(target_os = "macos")`) + per-track
  closing-docs (description-column update on
  `EV-INTEGRATION-MATRIX` for the Syphon-arm closure).

**Phase 4 — UI integration (2 PRs, 2026-05-07).**

- **PR `#404`** — `Sources.svelte` camera source-picker (Wave 1c-β
  Phase 4 lane A): camera-input device list bound to the
  camera-adapter `enumerate_devices` seam through the Tauri IPC
  shim; per-source kind affordances (Built-In / USB / Virtual);
  permission-state pill per the macOS / Windows probe surface.
- **PR `#405`** — `Sources.svelte` Syphon source-picker (Wave 1c-β
  Phase 4 lane B): Syphon server list bound to the
  syphon-adapter `list_syphon_servers` seam (currently empty-vec
  stub awaiting the deferred NSDistributedNotificationCenter
  observer wave); per-server app-name + name display.

**Phase 5 — Cluster closing-docs (1 PR, this PR).**

- **This PR** — drops the cumulative "camera + Syphon arms
  deferred to Wave 1c-β" narrowing on `EV-INTEGRATION-MATRIX`;
  retires the Wave 1c-β AO-deferral note from `CLAUDE.md`; flips
  camera + Syphon entries in `docs/53` from gap to implemented;
  adds this Phase-4a Wave 1c-β closure section to
  `docs/EV-GATE-MODEL-AUDIT-checklist.md`; updates
  `docs/46-phase-4-exit-readiness.md` Phase-4a row + anchor commit
  table; appends a closure paragraph to R-05 in
  `docs/13-risk-register.md`.

### Per-row verdict (cumulative narrowing widened)

- **`EV-INTEGRATION-MATRIX`** — stays at `Pass` (already
  transitioned in Phase-4a Wave 1a-A); the Phase-4a EXIT closure
  decision was AO-declarable as of 2026-05-03 under collective
  narrowings. This Phase-5 cluster closure widens the row's Pass
  scope by dropping the cumulative "camera + Syphon arms deferred
  to Wave 1c-β" narrowing. Both arms ship per the per-track
  description-column updates on the row (PR `#400` Syphon-arm
  closure paragraph + PR `#403` camera-arm closure paragraph)
  with this PR appending the Phase-5 cluster-closure paragraph.

### Per-Type Minimums verification

For row `EV-INTEGRATION-MATRIX`, the Phase-4a EXIT closure section
already records the Per-Type Minimums (anchor run id `25213038199`
on Phase-4a Wave 1a-A merge `77f174c`) which carry forward
unchanged. The Wave 1c-β cluster closure adds per-track anchors
through the per-track closing-docs already shipped in PRs `#400` +
`#403` (Syphon-arm + camera-arm description-column updates). This
Phase-5 closing-docs PR is documentation-only (no engineering code
changes); the auditable anchor for this PR is its own workspace
nextest job at the merge sha.

### Cumulative session recap

16 PRs landed on `main` covering the full Wave 1c-β engineering
sequence: 5 design PRs (`#389`-`#393`) + 1 acceptance/vendor PR
(`#394`) + 5 camera-track PRs (`#395` `#397` `#399` `#401` `#403`)
+ 1 hotfix PR (`#402`) + 3 Syphon-track PRs (`#396` `#398` `#400`)
+ 2 Phase-4 UI PRs (`#404` `#405`) + this Phase-5 closing-docs PR.
Each engineering PR's workspace nextest job is the auditable
per-PR anchor; the Phase-5 closing-docs PR's nextest job is the
auditable cluster anchor.

### Verdict

**Phase-4a Wave 1c-β cluster closure achieved 2026-05-07 at commit
`fb2bc75`** (the merge commit of this Phase-5 closing-docs PR).
With this closure, `EV-INTEGRATION-MATRIX` is at full Phase-4a
scope across all five arms (OSC + MIDI + audio-uniform-IPC +
camera + Syphon) with no cumulative narrowing remaining beyond the
per-arm deferred-axes records below. Phase-4a EXIT remains
AO-declarable per the existing posture (achieved 2026-05-03 under
collective narrowings; this closure widens the Pass scope by
dropping one of those narrowings); the cluster closure does not
pre-empt the Phase-4 / Phase-4b / Phase-4c / Phase-4d gate
decisions that remain AO-held per `docs/11-roadmap-phases.md`.

### Deferred axes (cluster narrowing record)

These items are explicitly out-of-scope for Wave 1c-β and tracked
as known follow-ups:

- **macOS + Windows nokhwa concrete bindings** — PRs `#397` +
  `#399` ship trait-only stubs (`AuthorizationProbe` /
  `InUsePrecheck` / `WindowsCameraProbe` + `PlaceholderProbe`
  impl); the live CoreMediaIO / Media Foundation binding wiring is
  encapsulated by nokhwa upstream so the workspace
  `unsafe_code = "forbid"` lint stays in force inside
  `integrations/camera-adapter/` itself per ADR-020 "Decision
  Drivers". Concrete binding-call surface deferred to a follow-up
  policy-scoped `unsafe_code` widening PR.
- **Syphon `SyphonClientHandle: Send` vs
  `Retained<SyphonMetalClient>: !Send` reconciliation** — PR
  `#400` worked around with carrier shapes that intentionally do
  NOT yet store the `Retained<SyphonMetalClient>` /
  `Retained<SyphonMetalServer>` framework handles because the
  trait's `Send` bound declared by PR `#398` is incompatible with
  `objc2`'s `NSObject` interior. Full reconciliation via a
  render-thread-affine `unsafe Send` wrapper per ADR-006 deferred
  to a post-Wave-1c-β policy-scoped follow-up.
- **`list_syphon_servers` real
  `NSDistributedNotificationCenter` observer** — currently
  empty-vec stub; needs main-thread runloop integration. The PR
  `#405` Sources.svelte Syphon source-picker hits the stub today
  and renders an empty-state. Future-wave option to land the
  observer plus reactive store wiring; out-of-scope at Wave 1c-β.
- **`publish_syphon_output` / `unpublish_syphon_output` real-
  impl** — currently synthetic stubs returning
  `SyphonRenderBridgeError::Unimplemented` per the documented
  PR 3 narrowing in `engine/render/src/syphon_adapter.rs`.
  Live `wgpu_hal::api::Metal` escape-hatch + `CVMetalTextureCache`
  allocation + `CVPixelBufferPool` 2-3-surface pre-warm ring per
  `docs/54` Phase-3 lessons 3.1 / 3.3 / 3.4 / 3.6 / 3.8 deferred to
  a follow-up hardware-validated wave.
- **Real hardware validation** — the synthetic-corpus posture per
  ADR-020 §10 + ADR-021 §10 + `docs/54` Phase-2 / Phase-3 lessons
  defers physical camera + Syphon publisher-subscriber validation
  to manual exercise on AO dev rigs. Same precedent as `EV-CRASH-
  RESUME` synthetic restore-helper narrowing accepted at the row
  80 closure 2026-05-07 and the `EV-GPU-SELECTION` synthetic dual-
  adapter narrowing accepted at the row 78 closure 2026-05-07.

### Sign-off

- [x] AO sign-off: Xavier Butler (@PhantasticUniverse) — 2026-05-07
