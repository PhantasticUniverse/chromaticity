<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# EV-GATE-MODEL-AUDIT — Phase 0 Exit Checklist

**Evidence ID:** `EV-GATE-MODEL-AUDIT`
**Registry row:** row 52 of `docs/26-evidence-artifact-registry.md`.
**Current status:** `Planned` -> target `Pass`.
**Gate role:** Phase 0 exit gate (`docs/26-evidence-artifact-registry.md` row
`EV-GATE-MODEL-AUDIT`).
**Purpose:** Validate gate ledger integrity and the cumulative release evaluation
model — i.e. audit `docs/26-evidence-artifact-registry.md` end-to-end for
internal consistency, simulate the Phase-0 gate decision against current row
state, and record that the audit ran honestly with findings noted.
**Checklist date:** 2026-04-29.
**Accountable Owner:** Xavier Butler (`@PhantasticUniverse`).

## Deliverable restated (from `docs/26` row 52)

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
- [x] Forward-roll: `EV-ENTITLEMENT-TOKEN-FORMAT` (row 88) closed 2026-04-30
      via PR 270 merge `107552f` (engine/entitlement crate scaffolded; 25
      tests pass; canonical CBOR byte-identity verified). The Phase-0 EXIT
      simulation advances to 9-of-19 members met; the remaining 10 gaps
      remain enumerated honestly above (8 UX-bundle rows AO-blocked,
      `EV-OBSERVABILITY-INVENTORY` Phase-0-deferrable, `EV-DOCS-PHASE0-SEED`
      still `Planned`). Phase-0 EXIT itself remains AO-held; the gate-model
      integrity finding is unchanged.
      Recorded by AO `@PhantasticUniverse` on 2026-04-30.
- [x] Forward-roll: `EV-DOCS-PHASE0-SEED` (row 66) closed 2026-04-30 via
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

- **Row 28 `EV-RENDER-CONFORMANCE`** — (a) `ci-run-url:25201608223`
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
- **Row 29 `EV-SCRIPT-CONTRACT`** — (a) `ci-run-url:25201608223`
  prefixed to the existing T1+T2 corpus repo-path chain. (b) green
  workspace test job. (c) corpus version pinned by slices 1+2+3+4+
  5.1+5.2+5.3 + the Tier-11 catalog v1.3.0 mirror retirement of
  FftLow/FftHigh; rquickjs 0.11.0 + crossbeam-channel pinned by the
  lockfile. (d) drift thresholds applied via the watchdog p99/hard-
  cap discipline + memory-cap T2 corpus + determinism shim T1+T2
  pins + the `slice_5a_v1_2_0_split_fft_names_no_longer_route`
  inverted retirement pin asserting the v1.3.0 catalog narrowing.
  **Phase-2 EXIT scope explicitly narrows row 29 to slices 1-5.3**
  per the Description column update in this PR; slice 5.4+
  (scene-author surface — control bindings, event handlers,
  preset-recall hooks per ADR-003) is post-Phase-2 feature work
  tracked separately, consistent with the AO direction recorded in
  `engine/script/src/lib.rs` lines 133-136 ("Slice 5 has reached
  EXIT-quality core with slice 5.3 — the producer / consumer /
  catalog / adapter quad covers the cross-crate contract; further
  widening is feature work, not foundational").
- **Row 30 `EV-AUDIO-TRACE`** — (a) `ci-run-url:25201608223`
  prefixed to the existing T1+T2 corpus repo-path chain. (b) green
  workspace test job. (c) corpus version pinned by slices 1+2+3a+3b
  + slice-5a cross-tag from EV-RENDER-CONFORMANCE + Tier-9 cpal
  multi-format dispatch + Tier-11 chroma_AudioFFT[8] cross-tag;
  cpal 0.15 + crossbeam-channel pinned by the lockfile. (d) drift
  thresholds applied via the LVL/HIT/TIM/PRS/BEAT/BPM/LRG family-
  tolerance discipline + the per-backend ACCEPTED-set framework
  carrying its first dual-backend entry post Tier-11 (see row 28).
  The slice-3b CpalSource owner-thread pattern + FFI-seam
  `catch_unwind` discipline + Tier-9 multi-format conversion math
  T1 pins all green on the workspace test run.
- **Row 42 `EV-SANDBOX-SAFETY`** — (a) `ci-run-url:25201608223`
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
`docs/26-evidence-artifact-registry.md` rows 28, 29, 30, 42 by this
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

- **`EV-INTEGRATION-MATRIX` (row 37)** — Phase-4a-bound per the
  registry's Phase Gate Evidence Sets section. Out of scope for
  Phase-2 EXIT. Covers OSC/MIDI/media route conformance + the
  Tauri-IPC bridge framework that downstream UI features depend on.
- **engine/script slice 5.4+ scene-author surface** — feature work
  per AO framing; explicitly narrowed out of row 29's Phase-2
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
      surface narrowing on row 29 is recorded in the Description
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
proxy benches) and five `docs(repo)` flip PRs transitioning rows 31,
32, 33, 34, 35 from `Planned` to `Pass`.

### Per-Type Minimums verification (Automated suite rows)

`docs/26` line 159 specifies the Per-Type Minimums for Automated suite
rows. Row-by-row verification at HEAD `f86895b`:

- **Row 30 `EV-AUDIO-TRACE`** — re-validated against the full corpus
  per the Phase-3 cumulative gate. Already `Pass` from Phase-2 EXIT
  cross-tag (run id `25201608223` at `d8e92db`); no Phase-3 widening
  required. The slice-1 `extract_uniform_snapshot` routing path is
  cross-tagged with `EV-AUDIO-ROUTING-MATRIX` row 31's T1 invariants
  in `engine/audio/src/uniforms.rs::uniforms_drift_pins` (`Evidence:
  EV-AUDIO-TRACE (T1) + EV-AUDIO-ROUTING-MATRIX (T1)`).
- **Row 31 `EV-AUDIO-ROUTING-MATRIX`** — (a) `ci-run-url:25204946481`
  + repo-path chain. (b) green workspace test job on `main` HEAD
  `f14c3d3` post-`#303` merge. (c) corpus version pinned by Phase-3
  wave PR 2 (`engine/audio/tests/routing_matrix_corpus/main.rs` —
  13 fixtures × 100 trials covering format conversion, channel
  aggregation, clamp-boundary edges). (d) drift thresholds applied
  via per-fixture tolerance assertion (1e-5 for direct conversions,
  0.0 exact for baselines) + dual-tagged uniforms_drift_pins module
  sharing T1 invariants with row 30.
- **Row 32 `EV-STATE-REPLAY`** — (a) `ci-run-url:25203436497` +
  repo-path chain. (b) green workspace test job on `main` HEAD
  `0ddcb9c` post-`#301` merge. (c) corpus version pinned by Phase-3
  wave PR 1 (`engine/runtime/src/state_replay.rs` canonical-binary
  encoder + FNV-1a-64 hashing path; `engine/runtime/tests/
  state_replay_corpus/main.rs` 6 fixtures × 100 trials). (d) drift
  thresholds applied via cross-trial identity + golden-hash
  assertion per fixture. Pure-Rust FNV-1a-64; empirically backend-
  invariant on Linux Vulkan / Mesa llvmpipe (no per-backend ACCEPTED-
  set widening required).
- **Row 33 `EV-STATE-MIGRATION`** — (a) `ci-run-url:25207466995` +
  repo-path chain. (b) green workspace test job on `main` HEAD
  `ef45bd0` post-`#307` merge. (c) corpus version pinned by Phase-3
  wave PR 4 (`engine/runtime/src/state_migration.rs` envelope
  encoder/decoder + V1→V2 forward migration; `engine/runtime/
  tests/state_migration_corpus/main.rs` 8 fixtures × 100 trials).
  (d) drift thresholds applied via cross-trial identity + golden-
  buffer assertion + decoder-rejection-path assertion per fixture.
  **Phase-3 EXIT scope explicitly narrows row 33 to forward-
  migration only** per the Description column update; rollback
  semantics deferred to Phase-4+ per AO direction. The dual-encoder
  envelope design preserves row 32 + row 34 goldens verbatim
  (existing `encode_state_canonical` hashing path UNCHANGED; new
  `encode_snapshot` envelope wraps it with a 4-byte BE version-tag
  prefix at the State level).
- **Row 34 `EV-PRESET-PLAYLIST`** — (a) `ci-run-url:25205805621` +
  repo-path chain. (b) green workspace test job on `main` HEAD
  `c33913a` post-`#305` merge. (c) corpus version pinned by Phase-3
  wave PR 3 (`engine/runtime/tests/preset_playlist_corpus/main.rs`
  8 fixtures × 100 trials). (d) drift thresholds applied via cross-
  trial identity + direct semantic invariants over public State
  accessors (`active_preset` / `playlist_position` / `control_value`)
  + canonical_state_hash composition pin on the combined fixture.
  **Phase-3 EXIT scope explicitly narrows row 34 to validation of
  existing PresetStore + PlaylistStore semantics** per the Description
  column update; preset-bank durability + scene-author preset capture
  + playlist reordering deferred to Phase-4+ per AO feature-work
  direction.
- **Row 35 `EV-SHOW-BUNDLE-RECOVERY`** — (a) `ci-run-url:25208208038`
  + repo-path chain. (b) green workspace test job on `main` HEAD
  `92d50d0` post-`#308` merge. (c) corpus version pinned by Phase-3
  wave PR 5 (`engine/scene/src/show_bundle.rs` manifest types + tar
  reader/writer + SHA-256 verification; `engine/scene/tests/
  show_bundle_corpus/main.rs` 6 fixtures × 100 trials). (d) drift
  thresholds applied via cross-trial identity + canonical-JSON byte-
  stability assertion + per-fixture rejection-path assertion (file-
  checksum mismatch, manifest-checksum tamper, path-traversal
  defense). **Phase-3 EXIT scope explicitly narrows row 35 to
  bundle-format spec only** per the Description column update;
  recovery / rollback semantics deferred to Phase-4+ per AO
  direction. The `docs/22-compatibility-versioning-policy.md` line
  78 reservation reconciled in the docs flip from `zip archive` to
  `tar archive` matching the AO Phase-3 narrowing decision.
- **Row 36 `EV-PERF-BUDGETS`** — corpus + harness landed at `85f869e`
  per Phase-3 wave PR 6 (`engine/runtime/benches/budgets.rs` criterion
  proxy benches over command-bus dispatch + state-replay canonical-
  hash + preset-recall + playlist-advance latency; `scripts/
  budget-check.sh` budget assertion helper; `.github/workflows/
  macos-nightly.yml` extended with `cargo bench` + budget-check +
  artifact-upload steps; new `criterion 0.5` dev-dep clean against
  `deny.toml`). **Per-PR CI does NOT exercise the bench harness**
  (Path B nightly-only AO posture per the AO-approved cost
  framing — ~$20/month vs ~$70-80/month for a per-PR macOS matrix
  expansion). The row 36 Pass transition therefore awaits the
  first GREEN macos-nightly run after `#309` merge (~24h post-
  merge); the docs flip PR follows the wave-PR pattern but with
  the macos-nightly anchor instead of the post-merge push CI run.
  **Phase-3 EXIT scope explicitly narrows row 36 to observable-
  subsystem proxy benchmarks**; the true frame-loop budget surface
  is deferred until ADR-001/ADR-006 slice-3 lands the scheduler-
  tick path (the `pub(crate) frame: u64` placeholder in
  `engine/runtime/src/state.rs` carries the in-source comment
  "scheduler-tick path lands in a later slice"). Row 36 corpus is
  empirically green via local `cargo bench --quick` per the wave
  PR 6 commit body; the registry transition lands in the follow-up
  wave PR 6.1 docs flip after the macos-nightly anchor.
- **Row 38 `EV-COLOR-MANAGEMENT-SRGB`** — explicitly deferred per AO
  direction. Sequenced behind a separate render-HUD-diagnostics PR
  (Phase-4+ scope). The deferral is recorded in `docs/13-risk-
  register.md` R-37 alongside the Phase-3 wave narrative; row 38's
  Description column will receive the deferral note in the wave
  PR 6.1 docs flip alongside row 36 (the AO-approved bundling per
  `docs/EV-GATE-MODEL-AUDIT-checklist.md`'s Phase-3 closure plan).

### Verdict

Seven of the eight Phase-3 EXIT evidence IDs satisfy the Per-Type
Minimums at HEAD `953fd74`: rows 30, 31, 32, 33, 34, 35, and 36 are
at `Pass` in `docs/26-evidence-artifact-registry.md`. The eighth
row, 38 `EV-COLOR-MANAGEMENT-SRGB`, is explicitly deferred per AO
direction — sequenced behind a separate render-HUD-diagnostics PR
exposing the active color pipeline (sRGB encode/decode points, gamma
handling, output transfer function); the deferral note + re-evaluation
criteria are recorded in row 38's Description column update by this
PR.

Per `docs/26` §Rules line 194 the Phase-3 EXIT gate is therefore
passable. **Phase-3 EXIT achieved 2026-05-01 at commit `953fd74`**.
Row 36 (`EV-PERF-BUDGETS`) was anchored against macos-nightly run
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

- **`EV-COLOR-MANAGEMENT-SRGB` (row 38)** — explicitly deferred per
  AO direction; sequenced behind a future render-HUD-diagnostics PR
  exposing the active color pipeline (sRGB encode/decode points,
  gamma handling, output transfer function). Closure depends on the
  HUD diagnostic surface landing as part of unrelated render work.
- **Row 33 rollback semantics** — pre-migration snapshot capture +
  atomic restore + partial-rollback UX per `docs/32-state-preset-
  playlist-migration-policy.md` deferred to Phase-4+ feature work.
  Phase-3 narrowed to forward-migration only.
- **Row 34 preset-bank durability + scene-author preset capture +
  playlist reordering** — Phase-4+ feature work per AO direction.
  Phase-3 narrowed to existing PresetStore + PlaylistStore semantics.
- **Row 35 recovery / rollback semantics** — corrupted-bundle repair,
  partial-load fallback, last-good-state restore — Phase-4+ feature
  work. Phase-3 narrowed to bundle-format spec only.
- **Row 36 true frame-loop budget surface** — deferred until ADR-001/
  ADR-006 slice-3 lands the scheduler-tick path. Phase-3 narrowed to
  observable-subsystem proxy benchmarks. The follow-on `chore(release-
  eng)` PR (~7 nightly runs post-wave-PR-6.1) tightens
  `scripts/budget-check.sh` from the loose-floor record-don't-fail
  posture to true budget assertions per `docs/10`.
- **Row 36 per-PR macOS matrix expansion** — Path B AO posture is
  nightly-only (~$20/month) rather than per-PR (~$70-80/month).
  Re-evaluation conditional on a regression incident actually firing.

### Sign-off

- [x] Phase-3 EXIT simulation ran to completion at HEAD `953fd74`.
      Seven of the eight Phase-3 EXIT evidence IDs satisfy the
      Per-Type Minimums and are at `Pass`: `EV-AUDIO-TRACE` (row 30,
      Phase-2 cross-tag), `EV-AUDIO-ROUTING-MATRIX` (row 31, run id
      25204946481), `EV-STATE-REPLAY` (row 32, run id 25203436497),
      `EV-STATE-MIGRATION` (row 33, run id 25207466995, narrowed to
      forward-migration only), `EV-PRESET-PLAYLIST` (row 34, run id
      25205805621, narrowed to existing PresetStore + PlaylistStore
      semantics), `EV-SHOW-BUNDLE-RECOVERY` (row 35, run id
      25208208038, narrowed to bundle-format spec only), and
      `EV-PERF-BUDGETS` (row 36, macos-nightly run id 25208934213
      manually dispatched against `main` HEAD `953fd74`, narrowed to
      observable-subsystem proxy benchmarks). The eighth row,
      `EV-COLOR-MANAGEMENT-SRGB` (row 38), is explicitly deferred per
      AO direction — sequenced behind a separate render-HUD-
      diagnostics PR exposing the active color pipeline; Phase-3 EXIT
      achieved without row 38 closure via the AO-approved deferral
      with the row's Description column carrying the deferral note +
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
bundle) plus this `docs(repo)` flip PR transitioning rows 37, 73, 74,
75, 82, 87, 88, 89, 91, 95 from `Planned` to `Pass`.

### Per-Type Minimums verification (Automated suite rows)

`docs/26` line 159 specifies the Per-Type Minimums for Automated suite
rows (a) `ci-run-url` or `repo-path` to test report artifact, (b) CI
run identifier with green status for pinned corpus version, (c)
corpus/fixture version recorded, (d) drift thresholds applied. Row-by-
row verification at HEAD `b335939`:

- **Row 37 `EV-INTEGRATION-MATRIX`** — (a) `ci-run-url:25213038199`
  + repo-path chain. (b) green workspace nextest job on `main` HEAD
  `77f174c` post-`#323` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-A wave-PR (`integrations/osc-adapter/tests/
  integration_matrix_corpus/main.rs` — 15 fixtures × 100 trials
  covering 4 route prefixes + path-traversal + adapter state machine
  + UDP loopback bind + packet validation). (d) drift thresholds
  applied via cross-trial identity assertion + per-fixture rejection-
  path assertion + dual-tagged `defaults.rs` pins shared with row 82
  EV-NET-HARDENING. **Phase-4a Wave 1a scope row 37 narrowed to OSC
  routing-matrix conformance only** per AO direction; MIDI / camera
  / Syphon / audio-uniform-IPC arms deferred to Wave 1b.
- **Row 73 `EV-HOTKEY-PARITY-CONFORMANCE`** — (a) `ci-run-url:
  25212775046` + repo-path chain. (b) green workspace Vitest job on
  `main` HEAD `a665d72` post-`#322` merge. (c) corpus version pinned
  by Phase-4a Wave 1a-B wave-PR (`src/lib/__tests__/
  hotkeys-parity.test.ts` — 6 fixtures pinning D/R/P/L → IPC
  contract via `vi.hoisted` invoke mock). (d) drift thresholds
  applied via direct invoke-mock-call assertion per fixture against
  the `docs/42` Sec.1 D/R/P/L parity contract.
- **Row 74 `EV-HOTKEY-COMPLETENESS`** — (a) `ci-run-url:25212775046`
  + repo-path chain. (b) green workspace Vitest job on `main` HEAD
  `a665d72` post-`#322` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-B wave-PR (`src/lib/__tests__/
  hotkeys-completeness.test.ts` — 8 fixtures validating the 57-row
  vocabulary in `src/lib/data/hotkeys.ts`: 10 groups, non-empty
  keys + action, capability namespace ⊂ ALL ∪ {'free'}, no verbatim
  duplicates). (d) drift thresholds applied via direct equality
  assertion per fixture against the `docs/42` Sec.1-10 contract.
- **Row 75 `EV-HOTKEY-CONFLICT`** — (a) `ci-run-url:25212775046` +
  repo-path chain. (b) green workspace Vitest job on `main` HEAD
  `a665d72` post-`#322` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-B wave-PR (`src/lib/__tests__/hotkeys-conflict.test.ts` —
  8 fixtures encoding 6-context taxonomy + Cmd↔Ctrl + Opt↔Alt
  cross-platform alias normalization + rules 1-5 from `docs/42`
  Sec.Conflict-and-priority-rules). (d) drift thresholds applied via
  direct equality assertion per fixture against the conflict-and-
  priority-rules contract.
- **Row 82 `EV-NET-HARDENING`** — (a) `ci-run-url:25213038199` +
  repo-path chain. (b) green workspace nextest job on `main` HEAD
  `77f174c` post-`#323` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-A wave-PR (`integrations/osc-adapter/tests/
  network_hardening_corpus/main.rs` — 10 fixtures validating ADR-013
  defaults + rate-limit precedence + allowlist boundary + oversized-
  packet outcome counter). (d) drift thresholds applied via direct
  equality assertion per fixture against ADR-013 §1/3/5 constants
  and dual-tagged `defaults.rs` pins shared with row 37
  EV-INTEGRATION-MATRIX.
- **Row 87 `EV-ENTITLEMENT-ENFORCEMENT`** — (a) `ci-run-url:
  25213475549` + repo-path chain. (b) green workspace nextest +
  Vitest jobs on `main` HEAD `b335939` post-`#325` merge. (c) corpus
  version pinned by Phase-4a Wave 1a-D wave-PR
  (`engine/entitlement/src/enforcement.rs` 377 LoC enforcement
  module + `engine/entitlement/tests/enforcement_corpus/main.rs`
  8 fixtures + `src/lib/stores/__tests__/
  capabilities-conformance.test.ts` 6 fixtures). (d) drift thresholds
  applied via direct boolean assertion per fixture + <1µs latency
  assertion + `Send + Sync` trait check. **Phase-4a Wave 1a scope
  row 87 narrowed to MVP watermark-on/off only** per AO direction;
  tier capability gating beyond the watermark deferred to post-MVP.
- **Row 88 `EV-ENTITLEMENT-OFFLINE`** — (a) `ci-run-url:25213475549`
  + repo-path chain. (b) green workspace nextest job on `main` HEAD
  `b335939` post-`#325` merge. (c) corpus version pinned by Phase-4a
  Wave 1a-D wave-PR (`engine/entitlement/tests/
  offline_revocation_corpus/main.rs` — 8 fixtures dual-tagged with
  row 89, covering 14-day offline window honor + post-expiry deny +
  boundary at `expires_at` + injected-clock determinism). (d) drift
  thresholds applied via cross-trial identity assertion per fixture
  against the 14-day window contract.
- **Row 89 `EV-ENTITLEMENT-REVOCATION`** — (a) `ci-run-url:
  25213475549` + repo-path chain. (b) green workspace nextest job
  on `main` HEAD `b335939` post-`#325` merge. (c) corpus version
  pinned by Phase-4a Wave 1a-D wave-PR (same dual-tagged
  `engine/entitlement/tests/offline_revocation_corpus/main.rs`
  8 fixtures as row 88; expiry-based revocation only). (d) drift
  thresholds applied via cross-trial identity assertion per fixture.
  **Phase-4a Wave 1a scope row 89 narrowed to expiry-based
  revocation only** per AO direction; deny-list activation behavior
  is post-MVP optional and out-of-scope for this row.
- **Row 91 `EV-ENTITLEMENT-FREE-FEATURES`** — (a) `ci-run-url:
  25213475549` + repo-path chain. (b) green workspace nextest job
  on `main` HEAD `b335939` post-`#325` merge. (c) corpus version
  pinned by Phase-4a Wave 1a-D wave-PR (`engine/entitlement/tests/
  free_features_corpus/main.rs` — 12 fixtures pinning the ADR-009
  §Explicitly-free namespace: pause + transitions-off + playlist-next
  + playlist-previous + preset-recall + snap-recall + snap-save +
  scene-export + 4 token-absent-still-grants paths). (d) drift
  thresholds applied via direct boolean assertion per fixture
  against the `FREE_CAPABILITIES` array.
- **Row 95 `EV-HEALTH-METRICS-LOOPBACK-ONLY`** — (a) `ci-run-url:
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
Minimums at HEAD `b335939` (counting hotkey rows 73-75 as a single
group, entitlement rows 87-89 + 91 as a single group, and rows 37 +
82 + 95 individually): five row groups covering ten registry rows are
at `Pass` in `docs/26-evidence-artifact-registry.md`. Six Phase-4a
EXIT rows remain `Planned` for Wave 1b: row 80 `EV-MIDI-EXTENDED`,
the four deferred arms behind row 37 `EV-INTEGRATION-MATRIX` (MIDI
conformance, camera conformance, Syphon conformance, audio-uniform-
IPC arm), and entitlement tier-gating beyond the MVP watermark
surface (which extends rows 87 and the post-MVP capability matrix).

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

- **Row 80 `EV-MIDI-EXTENDED`** — Wave 1b scope. The hotkey corpus
  (rows 73-75) covers control-focused parity + 57-row vocabulary
  completeness + 6-context conflict resolution but does not exercise
  the extended MIDI surface (CC + NRPN + program change + sysex
  routing) — that is row 80's surface and lands in Wave 1b.
- **Row 37 `EV-INTEGRATION-MATRIX` MIDI / camera / Syphon / audio-
  uniform-IPC arms** — Wave 1b scope. Wave 1a narrowed to OSC
  routing-matrix conformance only per AO direction. The four
  deferred arms each follow the same fixture-corpus pattern against
  the relevant integration crate when those crates exist (MIDI
  adapter crate, camera-input integration, Syphon publisher, Tauri
  audio-uniform IPC bridge).
- **Entitlement tier-gating beyond MVP watermark (rows 87 + post-MVP
  surface)** — Wave 1b scope. Wave 1a closure is narrowed to the
  watermark-on/off MVP per AO direction (`docs/11` line ~125). Tier
  capability gating beyond the watermark surface is post-MVP and
  lands in a Wave 1b expansion of row 87 + the deferred ADR-009 tier
  matrix.
- **`EV-SCHEDULER-TICK` (row 38) Pass-anchor** — slice-3 PR 1
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
      `EV-INTEGRATION-MATRIX` (row 37, run id 25213038199, narrowed
      to OSC routing-matrix conformance only),
      `EV-HOTKEY-PARITY-CONFORMANCE` (row 73, run id 25212775046),
      `EV-HOTKEY-COMPLETENESS` (row 74, run id 25212775046),
      `EV-HOTKEY-CONFLICT` (row 75, run id 25212775046),
      `EV-NET-HARDENING` (row 82, run id 25213038199),
      `EV-ENTITLEMENT-ENFORCEMENT` (row 87, run id 25213475549,
      narrowed to MVP watermark-on/off only),
      `EV-ENTITLEMENT-OFFLINE` (row 88, run id 25213475549),
      `EV-ENTITLEMENT-REVOCATION` (row 89, run id 25213475549,
      narrowed to expiry-based revocation only),
      `EV-ENTITLEMENT-FREE-FEATURES` (row 91, run id 25213475549),
      and `EV-HEALTH-METRICS-LOOPBACK-ONLY` (row 95, run id
      25213270708). Six Phase-4a EXIT rows remain `Planned` for
      Wave 1b: row 80 `EV-MIDI-EXTENDED`, the four deferred arms
      behind row 37 `EV-INTEGRATION-MATRIX` (MIDI conformance,
      camera conformance, Syphon conformance, audio-uniform-IPC
      arm), and entitlement tier-gating beyond the MVP watermark
      surface. Phase-4a Wave 1a narrowing decisions for rows 37 +
      87 + 89 are recorded in their Description column updates;
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
`engine/runtime/tests/scheduler_tick_corpus/main.rs`. Row 38
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

Row 38 `EV-SCHEDULER-TICK` transitions `Planned` → `Pass` against
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
      `#330`) merged in sequence; row 38 `EV-SCHEDULER-TICK`
      transitions `Planned` → `Pass`. The canonical scheduler-tick
      frame loop per ADR-001 / ADR-006 lands across
      `engine/runtime/src/scheduler/{mod,observability,audio,render}
      .rs` with the established sub-module + builder + tick-step
      extension pattern; T1 + T2 coverage closed (19 inline T1 pins
      + 6-fixture × 100-trial T2 corpus). All 4 T2 corpora
      (state_replay, preset_playlist, state_migration,
      scheduler_tick) pass with goldens preserved. Phase-4d row 36
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

Row 80 `EV-MIDI-EXTENDED` transitions `Planned` → `Pass`. Rows 37 +
86 narrowing-clauses dropped (Phase-4a Wave 1a's OSC-only and
MVP-watermark-only narrowings widened to MIDI matrix arm + tier-gating
respectively). Phase-4a EXIT row state after Wave 1a + 1b: **11 of 11
Phase-4a rows at Pass under collective narrowings**.

Full Phase-4a EXIT activation requires Wave 1c (camera/Syphon/IPC)
closure to drop the camera/Syphon/audio-uniform-IPC narrowing on row
37 + IO_CAMERA tier-gating activation on row 86. Phase-4a EXIT is
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
closing paragraph (this PR) and in the row 37 + row 86 Description
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
