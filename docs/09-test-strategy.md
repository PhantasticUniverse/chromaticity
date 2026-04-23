# Test Strategy

## Objective
Guarantee compatibility, reliability, and performance readiness before and during implementation.

## Test Layers
- **Unit Tests**
 - Schema parsing and normalization.
 - Command validation and state reducers.
 - Audio feature extraction utilities.
- **Contract Tests**
 - Script host API behavior.
 - Integration route translation.
 - Scene loader strict vs compatibility mode expectations.
- **Integration Tests**
 - End-to-end scene load -> render -> state transitions.
 - OSC/MIDI adapter command flow into runtime.
 - Preset and playlist execution semantics.
- **Regression Tests** (golden corpus)
 - Golden scene corpus frame-diff and trace-diff per `34-golden-corpus-definition.md`.
 - Audio fixture to uniform trace comparisons.
- **Fuzz Tests**
 - Driver: structure-aware fuzzing over documented grammar/schema plus protocol framing; runs on every PR touching an affected surface and in nightly extended sessions.
 - Targets:
   - `scene.json` schema fuzz (malformed keys, MIN>MAX inversion, type coercion classes KM-01..07),
   - `main.glsl` shader-source fuzz (malformed preprocessor directives, include-depth overflow, illegal uniform decls),
   - `script.js` sandbox-input fuzz (API misuse, deterministic-RNG surface, event-hook re-entry patterns),
   - OSC packet fuzz (malformed typetags, oversized payloads, bundle-depth overflow per ADR-013 cap, invalid symbolic payloads),
   - MIDI packet fuzz (malformed SysEx framing, 14-bit CC pair re-ordering, NRPN null-parameter sequences, MPE rejection path, MTC quarter-frame interleave),
   - `.synScene` importer fuzz (malformed directory packages, missing required files, mixed legacy+canonical keys, CATEGORIES vs TAGS collisions),
   - `.chromaPack` extractor fuzz (malformed archives, zip-slip, oversized members, duplicate entries),
   - `.chromaShow` reader fuzz (tampered signatures, truncated bundles, version-skew fields).
 - Exit: no Severity 1 or Severity 2 issue open in any fuzz target; coverage floor per surface recorded as a phase-local metric.
- **Chaos Tests**
 - Driver: scheduled-event chaos harness that injects faults at randomized timestamps during sustained runs, with deterministic replay of each seed.
 - Targets:
   - network partition on OSC/NDI adapters,
   - USB device hot-unplug on MIDI, USB-audio, USB-camera surfaces mid-show,
   - Syphon/Spout producer disappearance mid-frame,
   - disk-full during state write (preset save, show export, Snap save, rollback snapshot),
   - CPU/GPU thermal throttling (macOS `pmset` bounded throttle + Windows power-plan forcing),
   - GPU device-reset / TDR simulation,
   - audio driver sample-rate change mid-show,
   - low-memory pressure (bounded-working-set injection),
   - OS sleep/wake mid-show.
 - Exit: every target has a deterministic reproducible seed; recovery path is exercised end-to-end; no runtime crash and no scene-state corruption. Backs `R-07`, `R-12`, `R-25`, and `EV-SOAK-CHAOS`.
- **Performance Tests**
 - Driver: `EV-PERF-BUDGETS` harness plus `EV-SHADER-CACHE` cold/warm split.
 - Populations: `baseline-profile-fixture-subset`, `performance-profile`, `stress-profile` on each declared GPU class.
 - Measures per-subsystem budgets in `10-performance-latency-budgets.md` (audio, script, render, composition, readback, present, shader compile off-budget) against SLO-02 input-to-output and SLO-03 frame-delivery targets.
 - Exit: p50/p95/p99 frame-time within allocation across the three populations; no subsystem exceeds its carve-out without a waiver.
- **Compatibility Tests**
 - Driver: cross-profile conformance against `22-compatibility-versioning-policy.md` for `compat-syn-v1` and `native-chroma-v1`, plus `compat-syn-v1` alias table exhaustion.
 - Targets: every scene in the reference corpus loads and renders under `compat-syn-v1`; every legacy OSC alias in `08-io-integrations.md` Sec.OSC Compatibility Alias Table resolves to the canonical form deterministically; every KM-01..07 malformed fixture reaches the declared coercion outcome with the expected diagnostic severity.
 - Exit: `EV-SCENE-IMPORT` Pass for the reference-corpus subset; `EV-INTEGRATION-MATRIX` includes alias-form differential assertions.
- **Resilience Tests**
 - Fault injection (script exception, shader compile failure, adapter disconnect) beyond what Fuzz/Chaos layers cover.
 - Covers authoring-path failures (`37-authoring-environment-spec.md` §6) that keep the previous successful shader running and surface diagnostics without tearing down the output pipeline.

## Golden Scene Regression Program
- The canonical corpus definition, coverage floor, and provenance rules live in `34-golden-corpus-definition.md`.
- For each scene:
 - capture deterministic snapshots at fixed ticks,
 - capture control and uniform traces,
 - compare against the Determinism and Drift Acceptance Table in `10-performance-latency-budgets.md`.

## Acceptance Gates
- **Gate A** Scene compatibility
 - No critical schema incompatibilities in corpus.
- **Gate B** Runtime stability
 - No process crashes under scripted fault suite.
- **Gate C** Integration correctness
 - Protocol route test matrix passes for canonical command translation.
- **Gate D** Performance compliance
 - Meets budgets in `10-performance-latency-budgets.md`.

## SLO Validation Coupling
- Operational acceptance must also satisfy SLO targets in `21-operational-slos-slis.md`.
- Evidence artifacts must include:
 - `EV-SOAK-CHAOS`
 - `EV-PERF-BUDGETS`
 - `EV-INTEGRATION-MATRIX`
 - `EV-DETERMINISM`

## Tooling Plan
- Test harness executable for headless scene runs.
- Fixture framework:
 - scene fixtures,
 - audio fixtures,
 - protocol message fixtures.
- Automated diff reports for frames and traces.

## Triage Policy
- Severity 1: crash, frame-deadlock, data corruption.
- Severity 2: contract mismatch that breaks authored scenes.
- Severity 3: quality drift within acceptable fallback behavior.

## Exit Criteria for Planning Phase
- Test taxonomy finalized.
- Golden corpus definition approved per `34-golden-corpus-definition.md`.
- Required harness capabilities scoped into WBS and roadmap.

## Experiment Standardization
- All validation experiments shall use `24-experiment-protocol-template.md`.
- No test-derived decision should be accepted without a completed experiment record and linked evidence artifact ID.
