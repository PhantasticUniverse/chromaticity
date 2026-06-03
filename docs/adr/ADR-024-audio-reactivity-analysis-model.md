<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-024: Audio-Reactivity Analysis Model (MilkDrop-class AGC + perceptual DSP)

## Status

Proposed

> The AO approved the design + phasing on 2026-06-03. Phase 0 (the per-band AGC +
> the mini-meter harness) has landed (`engine/audio/src/reactive_bands.rs`);
> remaining phases + the cross-seam contract change await implementation. Formal
> acceptance lands at PR merge per the AO-only ADR convention
> (`docs/29-cleanroom-legal-policy-baseline.md`).

## Date

2026-06-03

## Deciders

- Accountable Owner: Xavier Butler (@PhantasticUniverse)

## Context

Chromaticity's slice-1 audio analysis (`engine/audio/src/uniforms.rs`) emits
**raw, absolute** values: RMS / peak clamped to `[0,1]`, eight log-spaced FFT
band magnitudes, and an autocorrelation BPM. There is **no normalization, no
auto-gain, and no temporal smoothing**. That is *below* the industry quality bar
for a music visualizer and produced a concrete symptom: a known sine read 16 dB
low and jittery, and we found ourselves debating absolute levels (−9 vs −19 dBFS)
that a world-class analyzer renders irrelevant.

The AO set the bar: **match or exceed the reference (and the field) on both
features and quality**, with **reactivity prioritized** and **tempo as a
parity-only backup feature**.

A clean-room research sweep (2026-06-03, four-agent workflow) established the
state of the art from **public/open sources only** — MilkDrop 2, projectM, and
butterchurn source; the MIR literature; and published competitor docs. The
reference (`Contents/`) is treated strictly as an **observed black-box quality
target** + its published docs (`docs/comparison/`); its implementation/expression
is never parsed (`docs/29`).

Key findings:

- **MilkDrop's quality is auto-gain, not accuracy.** `bass`/`mid`/`treb` are
  *ratios*: instantaneous band energy ÷ a slow long-term average, centred on
  `~1.0` (`< 0.7` quiet, `> 1.3` loud), so reactivity is **volume-independent** —
  the same on a loud or quiet track. Verified verbatim across all three open
  codebases (dual EMAs: fast-attack `0.2` / slow-release `0.5` short average;
  `0.9` startup / `0.992` steady long average; FPS-compensated `rate^(30·dt)`).
- **Our documented feature catalog already exceeds the field** (a 33-symbol
  Synesthesia-parity vocabulary + raw RMS/peak/FFT primitives + a warming/ready/
  degraded readiness pill — the most complete surveyed). The gap is that the
  **live** surface is reduced (only 4 bands + BPM wired live; the FFT array is
  producer-only; spectrum rasterization is ADR-011-deferred) — wiring, not design.
- **The two GPL beat-tracking libraries (`aubio`, `BTrack`) are license-
  incompatible** with `LicenseRef-Chromaticity-Proprietary` + the clean-room SOP.
  Any beat tracker must be reimplemented first-party from the papers. `ebur128`
  (MIT/Apache), `realfft`, and `rustfft` are allowlist-safe.
- **Determinism is unaffected.** `canonical_state_hash` does not observe the
  audio snapshot; the analysis can be overhauled freely. Only the
  `EV-AUDIO-UNIFORM-IPC` round-trip must stay green (it does, via the `From` impl).

## Decision

Adopt a layered analysis model: **match MilkDrop's auto-gain, then exceed it with
perceptual DSP**, keep tempo at parity, and fix the ingest continuity bug.

### 1. Match MilkDrop — per-band AGC (the reactivity core)

A `ReactiveBands` analyzer computes, per analysis tick, `band = instantaneous /
long_avg` and `band_att = short_avg / long_avg` for bass/mid/treb, with the
verified dual-EMA constants, FPS-compensated, and a perceptual Hz band split
(`20–320 / 320–2800 / 2800–11025`). Output values centre on `~1.0` and are
volume-independent. **Auto-gain is the default** (matching MilkDrop + Synesthesia;
the operator noise-floor threshold — Shift+T / OSC `/audio/threshold` — stays the
escape hatch). *(Landed: `reactive_bands.rs`, 6 tests + the mini-meter rubric.)*

### 2. Exceed it — perceptual DSP

- **K-weighted LUFS** master level (ITU-R BS.1770 / `ebur128`) — perceptually
  correct "energy", and the key for the noise gate.
- **Mel / third-octave perceptual bands** (16–32) alongside the MilkDrop-
  compatible bass/mid/treb macro-bands — the #1 visualizer quality win.
- **Per-band attack/release one-pole envelopes** + **rolling-p95 adaptive
  normalization** (robust to transients).
- **Hysteresis noise gate** keyed on the K-weighted level — eliminates the
  "dances to room hiss" failure mode.
- **Dual raw + normalized** uniform surface (authors choose calibrated dBFS or
  relative `0..1`), and **deterministic / golden-tested** analysis — neither of
  which any live competitor can claim.

### 3. Tempo — parity only (AO directive + license constraint)

Keep the existing autocorrelation BPM (already ≥ MilkDrop, which has no real BPM
tracker) + add a cheap MilkDrop-style spike beat + the BPM-phase oscillators
already in the catalog. **Defer** the phase-locked beat *grid*; if ever pursued
it is reimplemented first-party from the Ellis / Davies-Plumbley papers (never the
GPL libs). The heavy BPM/onset analysis runs at a **lower cadence** than the bands.

### 4. Ingest continuity — drain-all + fixed-cadence

Replace the per-frame heavy analysis (FFT+BPM at ~1500 frames/s overrunning a
16-frame bounded channel → silent drops → gappy windows) with **drain-all-then-
analyze-at-a-fixed-cadence** (~60–100 Hz; heavy BPM lower). This keeps the channel
drained (no producer drops) and aligns CPU with priority.

## Quality rubric (the operational definition of "match or exceed")

Both our engine and the reference's *observed behavior* are scored against this;
ours must be ≥ on each. Reactivity rows are optimized hardest; tempo is parity.

| # | Metric | Measure | Priority |
| --- | --- | --- | --- |
| 1 | Volume independence (AGC) | response variance over a level sweep | **high** |
| 2 | Latency | onset → response (ms) | high |
| 3 | Jitter vs responsiveness | steady-tone variance vs transient attack | **high** |
| 4 | Beat/onset accuracy | F-measure vs annotated ground truth | parity |
| 5 | Tempo accuracy | BPM error vs ground truth | parity |
| 6 | Band/frequency accuracy | response over a frequency sweep | **high** |
| 7 | Perceptual loudness | correlation with LUFS | exceed |
| 8 | Silence handling | reactivity floor on silence/noise | high |
| 9 | Determinism | bit-exact reproducibility | exceed |
| 10 | CPU cost | analysis ms/frame | high |

The **mini meter** (`cargo run -p chromaticity-audio --example mini_meter`) scores
the synthetic rows in a tight CLI loop (no app/tap/consent); Phase-0 result:
volume independence Δ=0.000, clean band selectivity, silence centred, broadband
all ~1.0.

## Feature parity (match-or-exceed the reference surface)

Our documented vocabulary already meets/exceeds the reference; the work is making
it **live**: a float-array spectrum carrier (so `chroma_AudioFFT` is script-
readable/writable), per-band Level/Hits/Time families live, and the BPM-phase
oscillators. Determined from the reference's **published docs** only (`docs/36`,
`docs/comparison/`), never `Contents/` internals.

## Phasing

- **Phase 0** *(AGC + mini-meter landed; drop-fix pending)*: `ReactiveBands`
  auto-gain + the mini-meter benchmark harness + the drain-all/fixed-cadence
  drop-fix. Engine-only.
- **Phase 1**: K-weighted LUFS (`ebur128`) + Mel bands + adaptive-norm + hysteresis
  gate + **dual raw/normalized uniforms** — widens `AudioUniformSnapshot` /
  `AudioUniformSnapshotPayload` (+ ts-rs regen) and the `chroma_Audio*` surface.
- **Phase 2**: live wiring — float-array spectrum carrier + per-band families +
  BPM oscillators (some ADR-011-render-gated).
- **Phase 3** *(deferred)*: first-party phase-locked beat grid, only if reprioritized.

## Consequences

### Positive

- World-class, volume-independent reactivity that looks good on any material;
  perceptual loudness + bands; deterministic + benchmarked; no GPL dependency.

### Negative / risks

- Widening `AudioUniformSnapshot` ripples to the payload + ts-rs (mechanical, via
  the `From` impl). The band *semantics* change from raw→relative — additive (new
  fields; raw retained for back-compat / dual output).
- `ebur128` must be verified no-unsafe + added to the `deny.toml` allowlist before
  use (Phase 1).
- On-hardware reactivity quality remains AO-observed against the reference.

### Deferred

- Phase-locked beat grid (first-party from papers); configurable per-band ranges
  (post-MVP power-user); live GPU spectrum rasterization (ADR-011).

## Verification

- `engine/audio` unit tests + the mini-meter rubric scorecard (synthetic signals).
- A small annotated-beat corpus for the tempo/onset parity rows (Phase 1+).
- `cargo deny` clean (no GPL; `ebur128` verified before adding); determinism
  golden tests unchanged (snapshot not observed by `canonical_state_hash`);
  `EV-AUDIO-UNIFORM-IPC` round-trip stays green.

## Evidence

`EV-AUDIO-UNIFORM-IPC` (cross-seam round-trip — unchanged by the snapshot
widening, which rides the `From` impl) and `EV-AUDIO-TRACE` (the producer-side
analysis surface) are the tagged rows. Reactivity quality is evidenced by the
mini-meter rubric scorecard (`engine/audio/examples/mini_meter.rs`) + the
`reactive_bands` unit tests (Phase 0); the tempo/onset parity rows by a small
annotated-beat corpus (Phase 1+). On-hardware reactivity is AO-observed against
the reference (clean-room, behavioral — never implementation).

## Traceability

- `adr/ADR-019-audio-uniform-ipc-bridge.md` — the IPC bridge this analysis feeds;
  the snapshot widens, the bridge contract is unchanged.
- `adr/ADR-023-audio-metering-data-model.md` — the COMPLEMENTARY snapshot
  widening, and the one that must be coordinated with this. ADR-023 owns the
  operator-facing **metering** surface (true-peak, LUFS, stereo, clip flag, tempo
  confidence, discrete beat — the raw/calibrated half); this ADR owns the
  scene-facing **reactivity** surface (AGC relative bands + perceptual DSP — the
  normalized half). Both amend one `AudioUniformSnapshot` and MUST land
  coordinated: the K-weighted LUFS level is computed once and shared, and this
  ADR's "dual raw+normalized" surface IS ADR-023's calibrated surface + this
  model's normalized surface.
- `adr/ADR-022-system-audio-capture.md` — the capture source feeding the analysis.
- `adr/ADR-011-rendering-backend.md` — live spectrum rasterization (Phase 2 gate).
- `06-audio-engine.md` — the normative analysis spec (updated for this model).
- `26-evidence-artifact-registry.md` — `EV-AUDIO-UNIFORM-IPC` / `EV-AUDIO-TRACE`.
- `29-cleanroom-legal-policy-baseline.md` — public-source design; reference as
  observed black-box target only.
