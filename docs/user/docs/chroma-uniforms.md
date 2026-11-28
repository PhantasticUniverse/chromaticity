---
doc-type: reference
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Uniform Reference

Every reactive uniform, script global and symbol Chromaticity exposes to
scene authors uses the `chroma_*` namespace. Standard non-prefixed names
(`TIME`, `RENDERSIZE`, the `_Meta` controls and the `_*` helper functions)
complete the authoring surface.

Imported `.synScene` packages keep working: the importer and the live
renderer translate their reference-format names onto the `chroma_*` names
below automatically, so you never need to write them yourself.

## Live GPU UBO Layout (`std140`, binding 0)

In Chromaticity's live `wgpu`/Metal preview renderer (`app/shell/src/render.rs`),
the runtime packs a single `std140` uniform block at `layout(set = 0, binding = 0)`.
Native `.chroma` shaders declare the packed `vec4` block directly:

```glsl
layout(set = 0, binding = 0, std140) uniform ChromaUniforms {
    vec4 chroma_Time;       // x = elapsed_s, y = dt, z = frame, w = aspect
    vec4 chroma_Resolution; // x = width, y = height, z = 1/width, w = 1/height
    vec4 chroma_BPM;           // x = bpm, y = beat_time, z = on_beat, w = bpm_sin
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
`chroma_BPM.x` reports `0.0` until audio tempo locks (or locks to the operator's
manual tempo in `Manual` mode). The beat shapes
(`chroma_BPMTwitcher`, `chroma_BPMSin2/4/8`, `chroma_BPMTri/2/4/8`) and the
`chroma_Spectrum` sampler (backed by `chroma_AudioFFT[8]`) are currently
computed live only for imported `.synScene` shaders; for native `.chroma`
shaders they are planned (marked below).

### Scalar audio uniforms (runtime & planned catalog)

| Uniform | Type | Range | Live GPU Status | Description |
| --- | --- | --- | --- | --- |
| `chroma_BPM` | `vec4` (`chroma_BPM.x`) | `[50, 220]` | Live | Estimated tempo (`x=bpm, y=beat_time, z=on_beat, w=bpm_sin`) |
| `chroma_BPMConfidence` | `float` (`chroma_AudioMeta.z`) | `[0, 1]` | Live | Tempo-stability confidence; near 1 when the estimator agrees across the history window |
| `chroma_BPMTwitcher` | `float` | `[0, inf)` | Live (imported `.synScene`) / Planned (native `.chroma`) | Exponential "jump" clock; `floor` is the beat count, `fract` is the per-beat twitch |
| `chroma_BPMSin` | `float` (`chroma_BPM.w`) | `[0, 1]` | Live | Sine wave at BPM frequency, beat-phase-locked |
| `chroma_BPMSin2` | `float` | `[0, 1]` | Live (imported `.synScene`) / Planned (native `.chroma`) | Sine wave at BPM divided by 2 |
| `chroma_BPMSin4` | `float` | `[0, 1]` | Live (imported `.synScene`) / Planned (native `.chroma`) | Sine wave at BPM divided by 4 |
| `chroma_BPMTri` | `float` | `[0, 1]` | Live (imported `.synScene`) / Planned (native `.chroma`) | Triangle wave at BPM frequency |
| `chroma_BPMTri2` | `float` | `[0, 1]` | Live (imported `.synScene`) / Planned (native `.chroma`) | Triangle wave at BPM divided by 2 |
| `chroma_BPMTri4` | `float` | `[0, 1]` | Live (imported `.synScene`) / Planned (native `.chroma`) | Triangle wave at BPM divided by 4 |
| `chroma_InputLevel` | `float` (`chroma_AudioRMS.x`) | `[0, inf)` | Live | Pre-AGC instantaneous input level |
| `chroma_RawLevel` | `float` (`chroma_AudioRMS.x`) | `[0, inf)` | Live | Alias of `chroma_InputLevel` with identical semantics |
| `chroma_Threshold` | `float` | `[0, 1]` | Runtime / OSC | Operator-settable noise-floor gate; audio hops below this value publish zeroed `chroma_Level*` and `chroma_*Hits` |
| `chroma_AudioRMS` | `vec4` (`chroma_AudioRMS.x`) | `[0, inf)` | Live | Analysis-window root-mean-square energy of the mono-mixed audio frame |
| `chroma_AudioPeak` | `vec4` (`chroma_AudioPeak.x`) | `[0, inf)` | Live | Analysis-window peak magnitude of the mono-mixed audio frame |

### Array & sampler audio uniforms

| Uniform | Type | Live GPU Status | Description |
| --- | --- | --- | --- |
| `chroma_AudioFFT[8]` | `float[8]` | Live (std140 binding 0) | Eight log-spaced FFT band magnitudes covering 20 Hz to 20 kHz; element 0 is the lowest band, element 7 is the highest |
| `chroma_Spectrum` | `sampler1D` | Live (imported `.synScene`) / Planned (native `.chroma`) | Per-frame audio sampler (`r=Raw FFT, g=Juiced, b=Smooth, a=Waveform`); bridged from `chroma_AudioFFT[8]` on imported `.synScene` shaders |
| `chroma_LevelTrail` | `sampler1D` | Planned | Per-band historical levels over time, newest sample at `u=0` |

## Time uniforms

| Uniform | Type | Description |
| --- | --- | --- |
| `chroma_AudioTime` | `float` | Audio-energy-integrated clock: advances faster when the music is louder (wall-clock time is `chroma_Time.x`) |
| `chroma_BassTime` | `float` | Time accumulator that accelerates on bass-band hits |
| `chroma_MidTime` | `float` | Time accumulator that accelerates on mid-band hits |
| `chroma_MidHighTime` | `float` | Time accumulator that accelerates on mid-high-band hits |
| `chroma_HighTime` | `float` | Time accumulator that accelerates on high-band hits |
| `chroma_CurvedTime` | `float` | Same base as `chroma_AudioTime` with exponential acceleration on volume rises |

## Authoring conventions

When a scene needs both raw analysis (the `chroma_*` audio primitives)
and the smoothed / normalized parity surface (`chroma_Level*`,
`chroma_Spectrum`), declare both uniforms; the binding adapter keeps the
raw and smoothed paths independent so authors can pick the surface that
matches each visual element. New uniforms and script outputs you
define yourself can use any name; built-in reactive values always use
`chroma_*`.

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
