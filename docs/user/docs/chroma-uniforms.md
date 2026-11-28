<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Uniform Reference

Chromaticity exposes two uniform namespaces to scene authors:

- `syn_*` — the Synesthesia compatibility namespace (`compat-syn-v1`).
  These names exist for parity with reference scenes and are the
  primary surface authors should reach for when writing scenes that
  need to load on both runtimes.
- `chroma_*` — the chromaticity-native namespace (`native-chroma-v1`).
  These names cover net-new analysis primitives and chromaticity-first
  surfaces that have no equivalent in the compatibility catalog.

Authors may mix the two namespaces freely inside a single scene. The
binding adapter publishes the uniforms in a stable per-pass order; GLSL
authors declare each uniform with its canonical name and type.

## Audio uniforms

The audio analysis pipeline produces a per-tick `AudioUniformSnapshot`
on the audio-analysis thread, which the audio-to-render adapter
marshals into the GPU uniform slots below. The slice-1 producer is
pure-data; the slice-2 BPM estimator carries a history-window
median-smoother for tempo-following. Uniforms that read zero are the
documented all-zero baseline for "no audio observed yet"; in
production, the BPM uniform clamps to the default tempo (120 BPM) on
the first tick before any analysis history accumulates.

### Scalar audio uniforms

| Uniform | Type | Range | Description |
| --- | --- | --- | --- |
| `syn_BPM` | `float` | `[50, 220]` | Estimated tempo, clamped to the musical range |
| `syn_BPMConfidence` | `float` | `[0, 1]` | Tempo-stability confidence; near 1 when the estimator agrees across the history window |
| `syn_BPMTwitcher` | `float` | `[0, inf)` | Exponential "jump" clock; `floor` is the beat count, `fract` is the per-beat twitch |
| `syn_BPMSin` | `float` | `[0, 1]` | Sine wave at BPM frequency, beat-phase-locked |
| `syn_BPMSin2` | `float` | `[0, 1]` | Sine wave at BPM divided by 2 |
| `syn_BPMSin4` | `float` | `[0, 1]` | Sine wave at BPM divided by 4 |
| `syn_BPMTri` | `float` | `[0, 1]` | Triangle wave at BPM frequency |
| `syn_BPMTri2` | `float` | `[0, 1]` | Triangle wave at BPM divided by 2 |
| `syn_BPMTri4` | `float` | `[0, 1]` | Triangle wave at BPM divided by 4 |
| `syn_InputLevel` | `float` | `[0, inf)` | Pre-AGC instantaneous input level, computed as `clamp(RMS / AGC_target, 0, inf)` |
| `syn_RawLevel` | `float` | `[0, inf)` | Alias of `syn_InputLevel` with identical semantics |
| `syn_Threshold` | `float` | `[0, 1]` | Operator-settable noise-floor gate; audio hops below this value publish zeroed `syn_Level*` and `syn_*Hits` |
| `chroma_AudioRMS` | `float` | `[0, inf)` | Analysis-window root-mean-square energy of the mono-mixed audio frame (computed over the same window as `chroma_AudioFFT` so smoothing is consistent across the audio uniform family) |
| `chroma_AudioPeak` | `float` | `[0, inf)` | Analysis-window peak magnitude of the mono-mixed audio frame |

### Array audio uniforms

| Uniform | Type | Description |
| --- | --- | --- |
| `chroma_AudioFFT[8]` | `float[8]` | Eight log-spaced FFT band magnitudes covering 20 Hz to 20 kHz; element 0 is the lowest band, element 7 is the highest |

The `chroma_AudioFFT` array binding was consolidated from a transitional
split-vec4 spelling (`chroma_AudioFFTLow` for bands 0..3,
`chroma_AudioFFTHigh` for bands 4..7) once the multipass scheduler grew
a `FloatArray` uniform variant. GLSL authors declare the array
directly:

```glsl
uniform float chroma_AudioFFT[8];

// later in main():
float bass = chroma_AudioFFT[0] + chroma_AudioFFT[1];
float treble = chroma_AudioFFT[6] + chroma_AudioFFT[7];
```

### Sampler audio uniforms

| Uniform | Type | Channels | Description |
| --- | --- | --- | --- |
| `syn_Spectrum` | `sampler1D` | `r=Raw FFT, g=Juiced, b=Smooth, a=Waveform` | Per-frame audio sampler with packed analysis variants |
| `syn_LevelTrail` | `sampler1D` | `r=Whole, g=Bass, b=Mid, a=High` | Per-band historical levels over time, newest sample at `u=0` |

The eight-band split that `chroma_AudioFFT[8]` exposes is a separate
surface from `syn_Spectrum`; the sampler retains parity with the
compatibility catalog while the fixed-size array gives shader authors
ergonomic indexing without a sampler lookup.

## Time uniforms

| Uniform | Type | Description |
| --- | --- | --- |
| `syn_Time` | `float` | Wallclock seconds since scene start |
| `syn_BassTime` | `float` | Time accumulator that accelerates on bass-band hits |
| `syn_MidTime` | `float` | Time accumulator that accelerates on mid-band hits |
| `syn_MidHighTime` | `float` | Time accumulator that accelerates on mid-high-band hits |
| `syn_HighTime` | `float` | Time accumulator that accelerates on high-band hits |
| `syn_CurvedTime` | `float` | Same base as `syn_Time` with exponential acceleration on volume rises |

## Authoring conventions

When a scene needs both raw analysis (the `chroma_*` audio primitives)
and the smoothed / normalized parity surface (`syn_Level*`,
`syn_Spectrum`), declare both uniforms; the binding adapter keeps the
raw and smoothed paths independent so authors can pick the surface that
matches each visual element. Avoid coining new `syn_*` names for
chromaticity-native features — the Synesthesia namespace is reserved
for compatibility coverage; net-new primitives belong under `chroma_*`.

The published GLSL spelling for every uniform on this page is exactly
the canonical name; the runtime does not perform per-binding casing
fixups. A typo at the GLSL declaration site silently leaves the
uniform unbound, which surfaces as a warning at scene load and as a
zero-valued sample at render time.

---

*Authored from `engine/render/src/audio_adapter.rs` (canonical
`UNIFORM_NAME_*` constants), `engine/audio/src/uniforms.rs` (producer
shape and band semantics), `docs/36-reactive-symbol-catalog.md`
(reactive symbol catalog), and the
`docs/adr/ADR-019-audio-uniform-ipc-bridge.md` IPC contract. Clean-room
per `docs/29-cleanroom-legal-policy-baseline.md`.*
