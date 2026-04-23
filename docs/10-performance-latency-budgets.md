# Performance and Latency Budgets

## Objective
Define measurable operational budgets for live-use viability and planning acceptance.

## Core Targets (MVP Planning Baseline)

### Frame Performance
- Target frame rates:
 - primary target: 60 FPS
 - minimum acceptable under medium scene complexity: 30 FPS
- Frame-time hard budget at 60 FPS: 16.67 ms.
- Subsystem allocation totals **14.00 ms** with **2.67 ms reserved for jitter/OS-scheduling/preemption**:
 - command + state updates: <= 1.25 ms
 - script runtime: <= 1.75 ms
 - audio snapshot read (from CP-02a): <= 0.50 ms
 - render graph (all passes): <= 8.50 ms
 - present and overhead: <= 2.00 ms
 - jitter/scheduling reserve: 2.67 ms (unallocated)
- Off-budget operations (do not count against the 14.00 ms allocation):
 - shader compile on hot-reload or first-use: swapped in at the next frame boundary after compile completes; compile itself runs on a worker thread.
 - shader cache warm-up at startup: runs before the first rendered frame; measured by `EV-SHADER-CACHE`.
 - scene warm-switch `PrepareNext`/`PrimeNext` phases per CP-03: run in parallel with the current scene's frame loop.
 - thumbnail regeneration, docs-index build, auto-generated previews: background priority only.
- **Reserved post-MVP budget (activated with ADR-017 per `41-data-ingestion-differentiator.md`):**
 - external-data upload on the render thread (texture-uploading `syn_Data_Array_*` / `syn_Data_Field_*`, writing `syn_Data_Scalar_*`/`Vec_*` uniforms): <= 0.50 ms p95 per frame. Carved from the `render graph` allocation once data ingest ships; scenes must respect this budget or the frame breaks the hard limit.
 - external-data fetch wall-clock (off-thread, does not count against the 14.00 ms allocation): provider-specific bounds locked in ADR-017. Adapter framework is responsible for guaranteeing no fetch ever blocks the render thread.
 - cassette replay cost for golden-corpus fixtures (per `22-compatibility-versioning-policy.md` Sec.Data-pinned fixtures): same <= 0.50 ms upload budget applies; replay itself runs off-thread.

### Input Responsiveness
- OSC/MIDI input to visible output delta:
 - target p50 <= 25 ms
 - target p95 <= 45 ms

### Scene Operations
- Scene hot-load to first valid frame:
 - target <= 1.5 s for standard scenes (warm shader cache per `adr/ADR-011-rendering-backend.md` Sec.5).
 - target <= 4.0 s for standard scenes on cold cache.
 - "standard scene" = single-pass or multipass <= 3 passes, no more than 2 media bindings.
- Scene switch continuity:
 - no hard freeze > 100 ms on warm cache.
 - warm-switch uses `PrepareNext` overlap per CP-03; cold-cache switch may exceed `PrepareNext` timeout (see CP-03 parametrization).

### Shader Cache
- Cold-start shader compile budget for the Phase-0 seed scene set: median <= 800 ms per shader program, p95 <= 1800 ms. Measured by `EV-SHADER-CACHE`.
- Warm-hit shader load budget: p95 <= 40 ms per program.
- Persistent cache key per `adr/ADR-011-rendering-backend.md` Sec.5; cache invalidation on Chromaticity version change or backend variant change.

### Audio Pipeline Latency
- Audio sample ingestion to published frame uniform:
 - target p50 <= 20 ms
 - target p95 <= 35 ms

## Stability Budgets
- Continuous run soak target: >= 4 hours without crash.
- Memory growth target during steady playback:
 - no unbounded growth; drift <= 5% over 30-minute windows.

## Compatibility Budgets
- Golden-scene visual drift threshold:
 - configurable tolerance per scene class.
- Uniform trace drift:
 - bounded epsilon by family (level/hits/time/presence/BPM).

## Determinism and Drift Acceptance Table (Normative)

This table is the single source of truth for drift epsilons and ties `EV-DETERMINISM`, `EV-AUDIO-TRACE`, `EV-RENDER-CONFORMANCE`, and SLO-05 together.

| Drift Dimension | Epsilon / Threshold | Measurement | Binding Evidence |
|---|---|---|---|
| Deterministic replay pass ratio | >= 99.0% per profile | golden corpus replay suite | `EV-DETERMINISM` + SLO-05 |
| Audio level family (`syn_Level*`) | per-sample epsilon <= 1e-3 | audio fixture trace diff | `EV-AUDIO-TRACE` |
| Audio hits family (`syn_*Hits`) | event-count delta <= 0 over window; timing delta <= 1 frame | audio fixture trace diff | `EV-AUDIO-TRACE` |
| Audio time family (`syn_*Time`, `syn_CurvedTime`) | monotonic delta epsilon <= 1 frame period | audio fixture trace diff | `EV-AUDIO-TRACE` |
| Audio presence family (`syn_*Presence`) | per-sample epsilon <= 5e-3 | audio fixture trace diff | `EV-AUDIO-TRACE` |
| Audio BPM family (`syn_BPM*`, `syn_OnBeat`, `syn_ToggleOnBeat`, `syn_RandomOnBeat`) | BPM epsilon <= 0.5 bpm; beat timing delta <= 1 frame | audio fixture trace diff | `EV-AUDIO-TRACE` |
| Visual frame diff (standard scenes) | per-pixel Euclidean RGB <= 2/255; <= 0.5% pixels above per-channel 1/255 | frame snapshot diff | `EV-RENDER-CONFORMANCE` |
| Visual frame diff (feedback/multipass) | per-pixel Euclidean RGB <= 4/255; <= 2.0% pixels above per-channel 2/255 | frame snapshot diff | `EV-RENDER-CONFORMANCE` |
| Script state replay | bit-identical state hash across runs | state replay suite | `EV-STATE-REPLAY` |
| Cross-profile drift (macOS vs Windows same scene) | visual drift within 2x of same-profile threshold; reported separately | cross-profile diff report | `EV-PLATFORM-CONFORMANCE` |

Notes:
- Thresholds above are normative for MVP; profile-specific tightening requires ADR and registry update.
- Any test asserting drift must reference this table's dimension name and threshold explicitly.
- Golden corpus provenance and coverage rules live in `34-golden-corpus-definition.md`.
- "1 frame" in audio timing rows resolves to the staleness bound in `06-audio-engine.md` Sec.7.11 (`~19.3 ms` at 60 FPS = 1 hop + 1 frame period). Fixtures assert this bound, not a 16.67 ms single-frame window.
- Audio analysis algorithm constants, band edges, and smoothing parameters are pinned in `06-audio-engine.md` Sec.7. Changes to those constants require a compatibility profile bump per `22-compatibility-versioning-policy.md`.

## Measurement Strategy
- Instrumented timing points:
 - adapter input timestamp,
 - command commit timestamp,
 - script completion timestamp,
 - per-pass render start/end,
 - present timestamp.
- Persist benchmark runs with scene profile metadata.
- Run profiles are selected from `30-mvp-support-matrix.md`.
- Track p50/p95/p99 and max for all key latencies.

## Degradation Policy
- If frame budget is exceeded:
  1. optional reduce non-critical pass resolutions,
  2. adjust quality settings according to policy,
  3. preserve interaction responsiveness over visual fidelity.

## Planning Exit Criteria
- Every budget maps to at least one automated benchmark.
- Budget ownership assigned in work breakdown.
- Risk register includes mitigation for likely budget violations.
- Budget conformance is reconciled with SLO targets in `21-operational-slos-slis.md`.
- Budget evidence must include `baseline-profile` and `performance-profile` runs from `30-mvp-support-matrix.md`.
