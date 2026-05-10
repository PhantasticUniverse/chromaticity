<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Audio Reactivity

Chromaticity is an audio-reactive runtime: every scene can sample tempo,
energy, and per-band spectrum data from the live audio input and bind
them as GLSL uniforms. This page describes how audio enters the engine,
how the analysis pipeline produces uniform snapshots, and how the
operator HUD widgets and the GLSL shader binding layer consume that
data.

## Audio path overview

The audio pipeline runs across three thread boundaries, established by
the runtime thread-and-process model:

1. **Audio input** — Operating-system audio capture is owned by the
   audio analysis thread. A frame-source implementation pulls
   interleaved sample data, mixes multi-channel input down to mono,
   and produces a fixed-shape `AudioFrame` value type.
2. **Analysis** — The same thread calls `extract_uniform_snapshot` on
   each frame. The function is pure: same input frame yields the same
   output snapshot. Its responsibilities are RMS, peak, an
   eight-band log-spaced FFT, and a single-frame autocorrelation BPM
   estimate.
3. **Hand-off** — The producer-side runtime publishes the snapshot
   into a shared `Arc<Mutex<AudioUniformSnapshot>>` cell. The
   scheduler tick reads the cell, marshals it to GPU uniform inputs
   for the render path, and emits it to the UI over the engine-to-UI
   IPC bridge.

The audio crate intentionally does not depend on the render or IPC
crates; consumers marshal the plain-data snapshot into whatever
surface they own. This is what lets the analysis thread stay
render-agnostic and lets the IPC bridge attach without any audio-side
churn.

## What the snapshot carries

Each tick publishes the following fields:

| Field | Range | Notes |
| --- | --- | --- |
| `bpm` | `[50, 220]` | Estimated tempo, clamped to the musical range; defaults to 120 on too-short frames |
| `rms` | `[0, 1]` | Root-mean-square energy of the mono-mixed frame, clamped at 1 for out-of-range PCM input |
| `peak` | `[0, 1]` | Peak absolute magnitude of the mono-mixed frame |
| `fft_bands[8]` | `[0, 1]` per band | Eight log-spaced FFT magnitudes covering 20 Hz to 20 kHz |

The snapshot is a `#[derive(Debug, Clone, Copy, PartialEq)]` value
type; consumers can copy it freely without coordination. The default
shape is the all-zero baseline that consumers see during the
no-audio-yet startup window.

## The eight-to-four band reduction

The slice-1 producer publishes eight log-spaced FFT bands. Several
operator-facing widgets expose four named bands ("bass", "low-mid",
"mid", "high") to keep the UX legible. The reduction combines
adjacent bands pairwise:

- bass = `(fft[0] + fft[1]) / 2`
- low-mid = `(fft[2] + fft[3]) / 2`
- mid = `(fft[4] + fft[5]) / 2`
- high = `(fft[6] + fft[7]) / 2`

This four-band view drives the `AudioReactivity` widget's named tile
strip and the BPM strip below it. Authors who want raw eight-band
resolution read `chroma_AudioFFT` directly in their GLSL.

## How the operator HUD uses live audio

Three visualizer widgets in the operator HUD read live audio over the
IPC bridge:

- **`AudioReactivity`** — Four named band tiles plus a tempo strip
  derived from `bpm`. The widget falls back to a synthetic
  time-domain function during the no-snapshot startup window so the
  UI is never blank.
- **`BpmRing`** — Circular tempo indicator phase-locked to `bpm`. The
  ring's animation cadence reads the current snapshot directly; the
  synthetic fallback uses a fixed 120 BPM.
- **`BeatDots`** — Discrete beat-pulse indicator that lights up on
  each beat boundary derived from `bpm`. The synthetic fallback fires
  at the same fixed tempo.

All three widgets implement a "live snapshot present" gate: when the
last received snapshot is fresh, visuals run from real data; when the
snapshot is stale or absent, the synthetic fallback paints. This dual
mode is what lets the widgets ship as wireframe-faithful chrome before
the live engine wiring lands and continue to work after the audio
input source is silent.

The 48-bar FFT visualizer inside `AudioReactivity` and the inline
64-bar Spectrum surface in the Audio Uniforms view both stay
synthetic-driven in the V1 release; live-driven retrofits are post-MVP
UX work.

## Worked example: a radial pulse shader

A minimal scene that pulses on the bass band:

```glsl
uniform float chroma_AudioFFT[8];
uniform float chroma_AudioRMS;
uniform vec2  iResolution;

void main() {
    vec2 uv  = (gl_FragCoord.xy - 0.5 * iResolution) / iResolution.y;
    float r  = length(uv);

    // Bass = bands 0 and 1 averaged.
    float bass  = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]);

    // Pulse: an inverse-distance ring that thickens with bass energy
    // and brightens with overall RMS.
    float pulse = smoothstep(0.40 - 0.20 * bass, 0.42, r)
                * (1.0 - smoothstep(0.42, 0.44 + 0.10 * bass, r));

    gl_FragColor = vec4(vec3(pulse * (0.5 + chroma_AudioRMS)), 1.0);
}
```

The same scene becomes a tempo-locked breathing pulse by mixing in
`syn_BPMSin` against the radius. Authors can compose any of the
audio uniforms; the binding adapter publishes them all in a stable
per-pass order so per-pass uniform-buffer slot indices stay stable
across reloads.

## Setting the audio threshold

The audio noise-floor gate is exposed as the `syn_Threshold` uniform
and is operator-controllable through the audio settings panel and the
OSC routes:

- `Shift+T` samples the current 200 ms rolling RMS of `syn_Level` and
  sets the threshold to that value (also reachable as
  `/audio/threshold/sample`).
- `/audio/threshold {float}` accepts an absolute threshold in `[0, 1]`.

Audio hops with RMS below the threshold publish zeroed `syn_Level*`
and `syn_*Hits` family uniforms. The threshold is the operator's
primary tool for rejecting room noise, HVAC bleed, and crowd murmur in
quiet sections of a show.

---

*Authored from `engine/audio/src/runtime.rs` (producer thread shape),
`engine/audio/src/uniforms.rs` (snapshot field semantics),
`engine/render/src/audio_adapter.rs` (uniform binding contract), and
the `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` IPC contract
combined with the `docs/adr/ADR-006-thread-and-process-model.md`
thread-affinity invariants. Clean-room per
`docs/29-cleanroom-legal-policy-baseline.md`.*
