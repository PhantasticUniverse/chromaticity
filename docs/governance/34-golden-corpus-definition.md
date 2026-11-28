# Golden Corpus Definition (Internal Regression Corpus)

## Purpose
Define the canonical **internal regression corpus** used by `EV-SCENE-IMPORT`, `EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, `EV-AUDIO-TRACE`, `EV-STATE-REPLAY`, and `EV-DETERMINISM`. This corpus lives in the repository under `test/corpus/` and never ships in a release bundle (see `29-cleanroom-legal-policy-baseline.md` Sec. Corpus Partitioning). The customer-facing scene library is a separate, independently-authored corpus governed by `38-scene-library-and-import-spec.md`.

## Ownership
- Primary: AI Scene Workcell.
- Accountable: Accountable Owner.
- Every corpus revision requires ADR or an approved registry entry per `23-decision-ownership-matrix.md`.

## Selection Criteria
- Fixtures must exercise contract surfaces defined in `02-synesthesia-contracts.md` and `28-implementation-contract-pack.md`.
- Each fixture is tagged with:
 - compatibility profile (`compat-syn-v1` or `native-chroma-v1`),
 - coverage dimensions (multipass, media, script, transitions, traveler controls, meta controls, KM class),
 - provenance class (`original`, `derived-fixture`, `reference-adjacent`).
- Fixtures that rely on reference materials (`reference-adjacent`) must be minimal, documented clean-room reconstructions and are excluded from shipped packages by default.

## Minimum Coverage Requirements (MVP)

| Dimension | Minimum Count | Notes |
|---|---:|---|
| Single-pass scenes | 10 | baseline render parity |
| Multipass scenes (2 to 5 passes) | 10 | pass ordering and feedback |
| Multipass scenes (> 5 passes) | 3 | stress path |
| Script-heavy scenes | 8 | exercises CP-07 surface |
| Scenes using deprecated `inputs`/`uniforms` globals | 3 | covers KM-04 |
| Scenes defining `transition()` hook | 3 | covers KM-05 |
| Scenes without `transition()` hook | 3 | covers KM-05 negative |
| Media-dependent scenes (image) | 4 | covers MED-1 |
| Media-dependent scenes (video) | 3 | covers MED-1 |
| Camera-driven scenes | 2 | covers MED-2 |
| External texture feed scenes | 2 | covers MED-3 |
| Traveler control fixtures | 3 | covers ADR-008 |
| Meta control fixtures | 4 | covers CP-08 |
| Malformed fixtures (KM-01 missing `TYPE`) | 2 | coercion path CR-01 |
| Malformed fixtures (KM-02 inverted `MIN/MAX`) | 2 | coercion path CR-02 |
| Legacy/canonical key coexistence fixtures | 2 | coercion path CR-03 |
| Meta-pipeline fixtures (per meta symbol) | 20 | one fixture per meta symbol in `36-reactive-symbol-catalog.md` Sec.3.1-3.6; asserts formula correctness and apply-order |
| GLSL helper fixtures (per `_` helper edge case) | 30 | one fixture per helper listed in `36-reactive-symbol-catalog.md` Sec.4.1-4.7; asserts signature, defaults, and documented edge cases |
| Transition-shape fixtures (HARD weight distribution) | 2 | covers CP-09 `HARD_TRANSITIONS` value selection and duplicate weighting |
| Transition-shape fixtures (SMOOTH crossfade + DURATION clamp) | 2 | covers CP-09 `SMOOTH_TRANSITIONS` crossfade math and `DURATION` clamp |
| Transition-shape malformed fixtures (CR-05) | 2 | missing `UNIFORM` / `VALUES` / `UNIFORMS` / `DURATION` |
| Pass-sizing sentinel fixtures (`0` and absent) | 3 | covers CP-10: backbuff `0,0`, intermediate `0,0`, mixed sentinel |
| Pass-sizing absolute vs relative fixtures | 2 | covers CP-10: "Scale Passes Relatively" on and off |
| Media sampler pair fixtures | 3 | covers CP-11: `syn_Media` vs `syn_UserImage` mask behavior; `_Media_Scale` applied in sample-space |
| Script host mirror fixtures | 5 | covers Sec.5 mirror rules: excluded fragment-only symbols produce strict errors; vector-object shapes accepted; control globals by `NAME` |
| Shadertoy import fixtures | 3 | covers FR-14 Shadertoy path (`EV-IMPORT-SHADERTOY`) |
| ISF reject fixtures | 2 | covers FR-14 ISF reject path (`EV-IMPORT-ISF-REJECT`) |

Minimum total: >= 148 fixtures (adds one Profile-D `chroma_PrimeBang` heuristic fixture). Additions are expected; the minimum only guards coverage floor.

### Baseline Fixture Subset (for SLO-03)
`baseline-profile-fixture-subset` is a declared sub-corpus used by SLO-03 targets in `21-operational-slos-slis.md`. It contains:
- all 10 single-pass fixtures
- the 5 lowest-complexity multipass fixtures (<= 3 passes each)
- 3 script-heavy fixtures with scripts that stay within the p99 budget
- 2 media-image fixtures (no video)

Total: ~20 fixtures, authored to stay within `p95 frame-time <= 16.67 ms` on `baseline-profile` hardware (GPU-A). Anything else (feedback/multipass > 3, video media, camera ingest, stress fixtures) is outside SLO-03 scope and is measured by `EV-PERF-BUDGETS` against the `performance-profile`/`stress-profile` targets instead.

### Profile-D Heuristic Fixtures
| Dimension | Minimum Count | Notes |
|---|---:|---|
| `chroma_PrimeBang` trigger fixtures | 1 | Covers `06-audio-engine.md` Sec.7.14 heuristic gate. Acceptance criterion: symbol fires on the seeded large-transition input and stays silent on the seeded no-transition input. Not epsilon-matched to any reference output. |

## Provenance and Licensing Rules
- Every fixture directory contains a `PROVENANCE.md` with origin class, author/workcell, design reference, and `release_inclusion` flag.
- This document enumerates fixtures for the **Internal Regression Corpus** defined in `29-cleanroom-legal-policy-baseline.md`. All fixtures here are CI-only by default (`release_inclusion=false`) and never ship in a release bundle.
- The **Customer Sample Corpus** (scenes that ship in `<AppBundle>/Resources/scenes/`) is authored separately as `origin=original` scenes and is not governed by this document's coverage floor; see `38-scene-library-and-import-spec.md` Sec.2.1 and `29-cleanroom-legal-policy-baseline.md` for that corpus's rules.
- Build-pipeline gate: `EV-LICENSE-AUDIT` scanner blocks any release artifact containing a file with `release_inclusion=false`. Promotion of a regression fixture to the shipping corpus requires authoring a new original scene from first principles (not relabeling).

## Refresh and Versioning Cadence
- Corpus version is pinned per release; version is recorded in `EV-DETERMINISM` and `EV-RENDER-CONFORMANCE` run metadata.
- Corpus may grow between versions but must not shrink without ADR.
- Breaking changes to compatibility behavior require a new corpus version alongside a compatibility profile bump per `22-compatibility-versioning-policy.md`.

## Evidence Bindings
- `EV-SCENE-IMPORT` must import every fixture without strict-mode failure (compat fixtures pass under compatibility mode).
- `EV-RENDER-CONFORMANCE` and `EV-AUDIO-TRACE` apply the drift thresholds in the Determinism and Drift Acceptance Table in `10-performance-latency-budgets.md`.
- `EV-SCRIPT-CONTRACT` must cover every script-API surface in CP-07 at least once and every script-host mirror rule in `36-reactive-symbol-catalog.md` Sec.5.
- `EV-STATE-REPLAY` and `EV-DETERMINISM` run against the pinned corpus version per release.
- `EV-RENDER-CONFORMANCE` must include the meta-pipeline and GLSL helper fixtures (every row tied to `36-reactive-symbol-catalog.md` Sec.3-Sec.4) and the transition-shape + pass-sizing + media-sampler fixtures tied to CP-09/CP-10/CP-11.
- `EV-IMPORT-SHADERTOY` and `EV-IMPORT-ISF-REJECT` consume their respective fixture categories above.

## Change Control
- Corpus changes require:
 - updated minimum-coverage table entry if a dimension shifts,
 - provenance records for new fixtures,
 - evidence re-run for affected EV IDs before phase or release gate advancement.
