<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-023: Audio Metering Data Model (AudioUniformSnapshot Widening)

## Status

Proposed

> Amends the `AudioUniformSnapshot` contract defined by
> `adr/ADR-019-audio-uniform-ipc-bridge.md`. Drafted for AO review; each
> engine analysis-path change below is profile-impacting (golden-corpus
> diff) and lands only on AO sign-off. ADR-019 itself is not edited (it
> is non-delegable); this ADR is the additive amendment record.

## Date

2026-05-29

## Deciders

- Accountable Owner: Xavier Butler (@PhantasticUniverse)

## Context

ADR-019 defined the audio-uniform IPC bridge around **four mono analysis
primitives** — `bpm`, `rms`, `peak`, `fft_bands[8]` — plus per-axis
availability flags, with `rms`/`peak` clamped to `[0,1]`
(`engine/audio/src/uniforms.rs`). A clean-room world-class-metering
parity investigation (2026-05-29, benchmarked against the reference's
published behaviour per `docs/29`) found that the operator-facing meters
are **pre-professional precisely because of this data model**, not the
UI: a single clamped mono level destroys over-0-dBFS information, offers
no true-peak, no loudness, no stereo, no clip flag, no tempo confidence,
and no discrete beat.

The **frontend foundation has already shipped** on the data available
today (commit `80ebd7a`): a shared dt-aware metering primitive
(`src/lib/audio/metering.ts`) + a logarithmic dBFS `Meter.svelte` with
calibrated ballistics, dB color zones, peak-hold, and a no-signal state.
Below 0 dBFS the existing `rms`/`peak` are immediately usable on the dB
scale; **what the UI cannot synthesise** is anything the engine clamps
away or never computes. This ADR locks the engine-side widening that
closes the rest of the gap, and corrects the related over-claim in
`docs/comparison/features/faq.md:293` ("Yellow=clipping EXCEEDS" — today
the engine **clamps**, it does not **detect** clipping).

`docs/06-audio-engine.md` already specifies several of these axes
normatively (§7.4 / §7.12 AGC + threshold, §7.9 the two-stage fused BPM
estimator + confidence); they simply never reached the payload. The
onset detector and AGC must be **independently derived from public
methods** (e.g. spectral-flux onset detection), never transcribed from
the reference, per `docs/29`.

## Decision Drivers

- ADR-019 framed the payload as exactly four primitives; widening the
  analysis path **and** the wire shape is an architectural change pinned
  by the `From` impl at `app/shell/src/state/ipc_emit.rs` and two T2
  round-trip corpora — every field needs regenerated ts-rs bindings
  (`cargo test -p chromaticity-ui-bridge`).
- `docs/06 §7.13`: any change to the §7.4/§7.12/§7.9 analysis constants
  is a **profile bump** requiring a golden-corpus regression diff and an
  AO-reviewed report. Each field below is gated on that, individually.
- Determinism is unaffected at the state-hash level — `canonical_state_hash`
  does not observe the audio snapshot — but the analysis path is
  golden-pinned against synthetic audio corpora, so behaviour changes are
  still profile-visible.
- `unsafe_code = "forbid"` is unaffected (pure-Rust DSP; the true-peak
  oversampler and onset detector are safe Rust).
- Honour the reference's low-friction "set-and-forget" ethos: surface the
  AGC-normalised level as the default-friendly view, keep the dBFS/
  true-peak/LUFS as an opt-in pro detail layer — do not reintroduce a
  calibration tax.

## Considered Options

1. **Additively widen `AudioUniformSnapshot` + the analysis path,
   field-by-field, each behind its own profile-bump + golden diff.**
   (Chosen.) Keeps ADR-019's additive posture; the UI foundation already
   consumes the dB scale and lights up each field as it lands.
2. **Ship a second, parallel "pro metering" snapshot/event.** Rejected —
   forks the seam, duplicates the producer thread, and splits the wire
   contract the T2 corpora pin.
3. **Compute true-peak / LUFS in the UI from the existing samples.**
   Impossible — the engine clamps the level path to `[0,1]` and the UI
   never sees raw inter-sample data; the dishonest clip light is exactly
   the bug being fixed.

## Decision

Adopt **Option 1**. Extend `AudioUniformSnapshot` (and its ts-rs payload)
with the fields below. Each is an independent, AO-gated, profile-bumping
landing; the order is the recommended build sequence.

### 1. Clip safety — `true_peak_dbtp: f32` + `clip: bool`

Add a ≥4× oversampled true-peak stage (ITU-R BS.1770) in
`compute_peak`'s neighbourhood and stop clamping the level path at `1.0`,
so a clean 0 dBFS signal is distinguishable from a clipped one. `clip`
latches when `true_peak ≥ -1 dBTP` (or sample ≥ 0 dBFS); the UI owns the
hold time + reset. Honest clip light; retires the `faq.md:293` over-claim.

### 2. Real stereo — `rms_l/rms_r` + `peak_l/peak_r: f32`

Preserve the channel split **before** `mono_mix` so genuine per-channel
energy is computed, replacing `VuStereo`'s `rms * 0.94` mono-mirror. When
the device exposes one channel, both lanes collapse to the mono value.

### 3. AGC / normalised level — `input_level: f32` (+ raw)

Implement `docs/06 §7.4/§7.12`: `L_peak30` (30 s p95), `L_norm` asymmetric
EMA, and surface **both** the raw level and the AGC-normalised level so
the meter can offer the "you don't need to tweak this" default while
still showing honest dBFS.

### 4. Threshold / gate — `threshold` + `gated: bool`

Surface `syn_Threshold` (operator gate, default 0.0, already on OSC
`/audio/threshold` per §7.12) and whether the current hop is gated, so the
meter can render the noise floor and the gate.

### 5. Tempo confidence — `bpm_confidence: f32`

Replace the single-frame autocorrelation with the §7.9 two-stage fused
IOI + autocorrelation estimator and surface `bpm_confidence`
(`0.4·c_ac + 0.6·exp(-σ_ioi/8)`), so the UI can dim/annotate an unstable
tempo instead of presenting a guess as a lock.

### 6. Discrete beat — `on_beat`

A monotonic beat counter or 1.0-then-decay flag from an **independently
derived** spectral-flux onset detector, so `BeatDots` flash phase-locked
to real transients rather than a free-running `now·bpm/60` metronome.

### 7. Wire `input_gain` (correctness)

`settings.input_gain` (±18 dB, default unity) is currently a **persisted
no-op** — nothing in `engine/audio` reads it. Apply it to the analysis
buffer so the meter and every reactive uniform reflect the monitored
post-gain signal, and correct the `faq.md:293` "input gain EXCEEDS"
over-claim. Profile-impacting (affects every reactive uniform).

### Optional (exceed; AO call)

- `momentary_lufs` / `short_term_lufs` (K-weighted 400 ms / 3 s, EBU R128
  / BS.1770, -70 LUFS gated) — a genuine differentiator no VJ tool ships.
  K-system / SPL calibration is **out** (no calibrated monitoring chain in
  a live runtime).
- A wider spectrum array (e.g. `fft_bands_64`) or a raw-bin / waveform
  channel to draw a presentation-grade ("Juiced") spectrum + oscilloscope;
  `fft_bands[8]` is the binding constraint there. Any band-count change is
  pinned by the two T2 round-trip corpora.

## Consequences

### Positive

- Closes the metering parity gap to world-class + exceeds the reference
  (honest true-peak clip light, real stereo, LUFS) on a data model the
  shipped UI foundation already consumes.
- Each field is additive + independently shippable; the four-primitive
  ADR-019 wire contract is extended, not broken.

### Negative / risks

- Each field is a profile bump: ts-rs regen + the two T2 round-trip
  corpora + a golden-corpus regression diff + AO sign-off. Non-trivial,
  deliberately serialised.
- The §7.9 estimator + true-peak oversampler are real DSP with their own
  test corpora; the onset detector must be clean-room-derived.

### Neutral

- `unsafe_code = "forbid"` and `canonical_state_hash` are unaffected.

## Build sequence

1. `true_peak_dbtp` + `clip` (stop clamping) — wires the real clip-hold LED.
2. Stereo `rms_l/r` + `peak_l/r` — retires the `VuStereo` mono-mirror.
3. `input_gain` wiring (rides the same analysis-path profile bump).
4. AGC / threshold (`input_level`, `threshold`, `gated`).
5. Two-stage BPM + `bpm_confidence` + `on_beat`.
6. (Optional) LUFS; presentation-grade spectrum + wider band array.

## Verification

Per landing: `cargo test -p chromaticity-audio` + `-p chromaticity-ui-bridge`
(regenerated ts-rs round-trip), a golden-corpus regression diff for the
touched §7.x constants, and the frontend meter consuming the new field.

## Evidence

`EV-AUDIO-UNIFORM-IPC` + `EV-AUDIO-TRACE`. Related: `ADR-019-audio-uniform-ipc-bridge.md`,
`docs/06-audio-engine.md` §7.4/§7.9/§7.12/§7.13, the world-class metering
design (frontend foundation landed in `80ebd7a`).

## Traceability

- `adr/ADR-019-audio-uniform-ipc-bridge.md` — the `AudioUniformSnapshot` four-primitive
  wire contract this ADR additively amends; the `From` impl at
  `app/shell/src/state/ipc_emit.rs` and the two T2 round-trip corpora pin the shape, so
  every new field needs regenerated ts-rs bindings.
- `06-audio-engine.md` §7.4 / §7.12 (AGC `L_peak30` / `L_norm`, `syn_Threshold`, OSC
  `/audio/threshold`) and §7.9 (two-stage fused BPM estimator + `syn_BPMConfidence`) —
  already-normative specs these fields surface to the payload.
- `06-audio-engine.md` §7.13 — the profile-bump + golden-corpus regression-diff rule
  that gates each analysis-path constant change here.
- `26-evidence-artifact-registry.md` — `EV-AUDIO-UNIFORM-IPC` (wire shape) +
  `EV-AUDIO-TRACE` (analysis path) are the tagged surfaces each field lands against.
- `29-cleanroom-legal-policy-baseline.md` — the onset detector + AGC are independently
  derived from public DSP methods (spectral-flux onset, BS.1770 weighting), never
  transcribed from the reference.
- `43-development-standards.md` §15.5 — T-tier definitions for the per-field unit + T2
  round-trip corpora.
- `adr/ADR-014-signing-notarization-auto-update.md` — only relevant for the optional
  LUFS exceed insofar as no calibrated monitoring chain ships; K-system/SPL stays out.
