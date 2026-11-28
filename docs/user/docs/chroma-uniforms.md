---
doc-type: reference
---

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

## Live GPU UBO Layout (`std140`, binding 0)

In Chromaticity's live `wgpu`/Metal preview renderer (`app/shell/src/render.rs`),
the runtime packs a single `std140` uniform block at `layout(set = 0, binding = 0)`.
Native `.chroma` shaders declare the packed `vec4` block directly:

```glsl
layout(set = 0, binding = 0, std140) uniform ChromaUniforms {
    vec4 chroma_Time;       // x = elapsed_s, y = dt, z = frame, w = aspect
    vec4 chroma_Resolution; // x = width, y = height, z = 1/width, w = 1/height
    vec4 syn_BPM;           // x = bpm, y = beat_time, z = on_beat, w = bpm_sin
    vec4 chroma_AudioRMS;   // x = rms
    vec4 chroma_AudioPeak;  // x = peak
    float chroma_AudioFFT[8]; // 8 log-spaced FFT bands (std140-padded array)
    vec4 chroma_Ctrl0;      // user controls 0..3
    vec4 chroma_Ctrl1;      // user controls 4..7
    vec4 chroma_Ctrl2;      // user controls 8..11
    vec4 chroma_Ctrl3;      // user controls 12..15 (including XY.y & script.js)
    // Optional extended reactive envelopes (appended when shader references chroma_AudioEnv0):
    vec4 chroma_AudioEnv0;  // x = level, y = hits, z = presence, w = time
    vec4 chroma_AudioEnv1;  // x = bass_level, y = bass_hits, z = bass_presence, w = bass_time
    vec4 chroma_AudioEnv2;  // x = mid_level, y = mid_hits, z = mid_presence, w = mid_time
    vec4 chroma_AudioEnv3;  // x = high_level, y = high_hits, z = high_presence, w = high_time
    vec4 chroma_AudioMeta;  // x = fade_in_out, y = intensity, z = bpm_confidence, w = random_on_beat
};
```

> **Note on extended envelopes:** When a shader references `chroma_AudioEnv0`,
> the renderer automatically appends `chroma_AudioEnv0..3` and
> `chroma_AudioMeta` to the binding-0 std140 buffer.

## Audio uniforms

The audio analysis pipeline produces a per-tick `AudioUniformSnapshot`
on the audio-analysis thread, which the audio-to-render adapter
marshals into the GPU uniform slots above. Uniforms that read zero are the
documented all-zero baseline for "no audio observed yet"; in `Auto` BPM mode,
`syn_BPM.x` reports `0.0` until audio tempo locks (or locks to the operator's
manual tempo in `Manual` mode). Imported `.synScene` shaders automatically
receive compatibility macros for `syn_BPMTwitcher`, `syn_BPMSin2/4/8`,
`syn_BPMTri/2/4/8`, and `syn_Spectrum` backed by `chroma_AudioFFT[8]` via
`SYN_LIVE_GPU_ABI_BRIDGE`.

### Scalar audio uniforms (runtime & planned catalog)

| Uniform | Type | Range | Live GPU Status | Description |
| --- | --- | --- | --- | --- |
| `syn_BPM` | `vec4` (`syn_BPM.x`) | `[50, 220]` | Live | Estimated tempo (`x=bpm, y=beat_time, z=on_beat, w=bpm_sin`) |
| `syn_BPMConfidence` | `float` (`chroma_AudioMeta.z`) | `[0, 1]` | Live | Tempo-stability confidence; near 1 when the estimator agrees across the history window |
| `syn_BPMTwitcher` | `float` | `[0, inf)` | Planned | Exponential "jump" clock; `floor` is the beat count, `fract` is the per-beat twitch |
| `syn_BPMSin` | `float` (`syn_BPM.w`) | `[0, 1]` | Live | Sine wave at BPM frequency, beat-phase-locked |
| `syn_BPMSin2` | `float` | `[0, 1]` | Planned | Sine wave at BPM divided by 2 |
| `syn_BPMSin4` | `float` | `[0, 1]` | Planned | Sine wave at BPM divided by 4 |
| `syn_BPMTri` | `float` | `[0, 1]` | Planned | Triangle wave at BPM frequency |
| `syn_BPMTri2` | `float` | `[0, 1]` | Planned | Triangle wave at BPM divided by 2 |
| `syn_BPMTri4` | `float` | `[0, 1]` | Planned | Triangle wave at BPM divided by 4 |
| `syn_InputLevel` | `float` (`chroma_AudioRMS.x`) | `[0, inf)` | Live | Pre-AGC instantaneous input level |
| `syn_RawLevel` | `float` (`chroma_AudioRMS.x`) | `[0, inf)` | Live | Alias of `syn_InputLevel` with identical semantics |
| `syn_Threshold` | `float` | `[0, 1]` | Runtime / OSC | Operator-settable noise-floor gate; audio hops below this value publish zeroed `syn_Level*` and `syn_*Hits` |
| `chroma_AudioRMS` | `vec4` (`chroma_AudioRMS.x`) | `[0, inf)` | Live | Analysis-window root-mean-square energy of the mono-mixed audio frame |
| `chroma_AudioPeak` | `vec4` (`chroma_AudioPeak.x`) | `[0, inf)` | Live | Analysis-window peak magnitude of the mono-mixed audio frame |

### Array & sampler audio uniforms

| Uniform | Type | Live GPU Status | Description |
| --- | --- | --- | --- |
| `chroma_AudioFFT[8]` | `float[8]` | Live (std140 binding 0) | Eight log-spaced FFT band magnitudes covering 20 Hz to 20 kHz; element 0 is the lowest band, element 7 is the highest |
| `syn_Spectrum` | `sampler1D` | Planned | Per-frame audio sampler (`r=Raw FFT, g=Juiced, b=Smooth, a=Waveform`) |
| `syn_LevelTrail` | `sampler1D` | Planned | Per-band historical levels over time, newest sample at `u=0` |

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
