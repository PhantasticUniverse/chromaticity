<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Standard Uniforms — A First-Author's Guide

This page is a first-time-author rendering of
`36-reactive-symbol-catalog.md` v1.2.0. It introduces the small set of
uniforms you need to make your first audio-reactive scene work and points
at the catalog for the full picture.

The catalog is **normative**; this page is a tutorial summary. When this
page and the catalog disagree, the catalog wins.

## How uniforms reach your shader

1. **Audio enters the system** through the configured input source (line-in,
   loopback, file). The audio analysis thread runs at a fixed `375 Hz` hop
   cadence — see `06-audio-engine.md` §7.1.
2. **Analysis publishes a snapshot** of every reactive value into a versioned
   structure once per hop. The contract is `06-audio-engine.md` §7.11.
3. **The render scheduler reads the most recent snapshot** at frame start
   (before pass 0). Every pass in the same frame observes the same snapshot
   — values do not shift mid-frame.
4. **Your shader sees the values as `uniform` declarations**, named by the
   per-pass `UNIFORMS` bindings in your `scene.json` (see
   `02-scene-format.md`).

The maximum staleness between an audio sample and the visible frame that
reflects it is `~19.3 ms` at 60 FPS (one hop period plus one frame period).

## Slice-3a representative subset

The Phase-1 validator
(`engine/scene/src/validator.rs::KNOWN_SYMBOLS`) accepts the seven symbols
below. These are enough for a meaningful audio-reactive first scene; the
full `36-reactive-symbol-catalog.md` set lands with the slice-3 loader.

| Symbol | GLSL type | Cadence | What it is | Where to use it |
|---|---|---|---|---|
| `TIME` | `float` | per-pass | Seconds since the scene started rendering. Monotonic; resets on scene load. | Anywhere you want an animating clock. |
| `RENDERSIZE` | `vec2` | per-pass | Pixel size of the **current** pass (not the scene). | `gl_FragCoord.xy / RENDERSIZE` for normalised UVs. |
| `_mouse` | `vec4` | per-frame | `(x, y, x_prev_click, y_prev_click)` in pixel units. Shadertoy-`iMouse` compatible. | Cursor-driven affordances in preview. |
| `syn_FinalPass` | `sampler2D` | per-frame | Previous frame's final output. | Feedback effects, trails, recursion. |
| `syn_Spectrum` | `sampler1D` | per-frame | RGBA-packed spectrum: r=raw FFT, g=juiced FFT, b=smooth FFT, a=waveform. | Frequency-band visualisations; spectrum bars. |
| `syn_Level` | `float` | per-frame | Smoothed whole-band audio level, normalised to `[0, 1]`. | Pulse / brightness / scale on overall energy. |
| `syn_Time` | `float` | per-frame | Whole-band audio time clock — accumulates faster when audio is loud. | Audio-driven animation phase distinct from `TIME`. |

**Reading the table.** *Cadence* is when the snapshot updates the value
(per-frame is once at frame start; per-pass is before each pass within a
frame). *GLSL type* is what your shader's `uniform` declaration must use —
mismatches are compile errors.

## How `syn_Spectrum` packs four channels

`syn_Spectrum` is one sampler that carries four spectrum representations in
its colour channels. Sample it once and read the channel you want:

```glsl
uniform sampler1D u_spectrum;   // bound to syn_Spectrum

float low_band  = texture(u_spectrum, 0.05).g;   // juiced FFT, low end
float mid_band  = texture(u_spectrum, 0.30).g;   // juiced FFT, mids
float high_band = texture(u_spectrum, 0.80).g;   // juiced FFT, highs
float wave      = texture(u_spectrum, 0.50).a;   // waveform sample
```

Channel meanings (per `06-audio-engine.md` §7.2):

- **`r` (Raw FFT)** — magnitude of the FFT bin, no post-processing. Useful
  if you want to do your own smoothing.
- **`g` (Juiced FFT)** — log-frequency-mapped, temporally smoothed,
  high-end-boosted, normalised to `[0, 1]`. **This is the default authoring
  channel.**
- **`b` (Smooth FFT)** — lower-resolution variant of Juiced; smoother bands
  for less twitchy visuals.
- **`a` (Waveform)** — raw time-domain samples mapped to `[0, 1]` (centred
  at `0.5`). Use for oscilloscope-style displays.

## Beyond the slice-3a subset

The full `36-reactive-symbol-catalog.md` catalog has many more symbols that
will become available as the slice-3 loader widens. Some highlights you
will likely reach for:

### Per-band level, hits, time, presence (catalog §2.2 – §2.5)

For each frequency band — `Bass`, `Mid`, `MidHigh`, `High`, plus the
`Whole`-band canonicals already in the subset — five family clocks publish:

- **`syn_*Level`** (e.g. `syn_BassLevel`) — normalised level `[0, 1]`,
  smoothed.
- **`syn_*Hits`** (e.g. `syn_BassHits`) — impulse on detected onsets in
  the band, decays per-frame.
- **`syn_*Time`** (e.g. `syn_BassTime`) — accumulating clock; faster when
  the band is loud.
- **`syn_*Presence`** (e.g. `syn_BassPresence`) — slow `[0, 1]` measure of
  whether the band is "active" (5-second time constant).

Band edges (Hz) are pinned in `06-audio-engine.md` §7.3; band-aggregation
formulas are in §7.4 – §7.7.

### Beat and BPM (catalog §2.6, §2.7)

- **`syn_OnBeat`** — `1.0` impulse on beat detection, decays in ~90 ms.
- **`syn_BPM`** — estimated BPM, clamped to `[50, 220]`.
- **`syn_BPMConfidence`** — `[0, 1]`, near 1 when BPM is stable.
- **`syn_BPMSin`**, **`syn_BPMTri`**, etc. — phase-locked sine and triangle
  waveforms at BPM and BPM/2, BPM/4.

The two-stage estimator (inter-onset-interval plus autocorrelation refinement)
is documented in `06-audio-engine.md` §7.8 – §7.9.

### Large features (catalog §2.8)

- **`syn_FadeInOut`** — slow rise on music start, slow fall on silence.
  Useful for "song is happening" gating.
- **`syn_Intensity`** — slow accumulator following overall song intensity.

### Native audio analysis primitives (catalog §2.11)

Slice-5a introduced four chromaticity-native uniforms exposing raw analysis
values from the audio hop, complementing the smoothed/normalised `syn_Level*`
and `syn_Spectrum` parity surfaces:

- **`chroma_AudioRMS`** (`float`) — instantaneous RMS energy of the current
  hop. Non-normalised.
- **`chroma_AudioPeak`** (`float`) — instantaneous peak magnitude of the
  current hop. Non-normalised.
- **`chroma_AudioFFTLow`** (`vec4`) — log-spaced FFT bands `[0..=3]` (lowest
  band at `.x`, band 3 at `.w`).
- **`chroma_AudioFFTHigh`** (`vec4`) — log-spaced FFT bands `[4..=7]` (band 4
  at `.x`, highest band at `.w`).

Use these when you want raw inputs for your own envelope or smoothing logic;
reach for the `syn_Level*` family when you want behaviour matched to reference
scenes. The split-`vec4` FFT spelling is transitional — the catalog notes the
planned consolidation to a single `chroma_AudioFFT[8]` array uniform once
the render-side `UniformValue` enum grows an array variant.

## What the values do when audio is silent or unavailable

- **Silence detection** triggers when whole-band level stays below `0.05`
  for more than `0.8 s`. `syn_FadeInOut` decays toward `0`; `syn_Level`
  family values track instantaneously.
- **Audio device failure** preserves the runtime, surfaces a warning in the
  UI, and falls back to silent / default reactive outputs (per
  `06-audio-engine.md` "Reliability/Fallback Policy"). Your shader still
  runs; reactive uniforms just publish their default-quiet values.
- **Unstable BPM** degrades BPM-derived outputs gracefully —
  `syn_BPMConfidence` falls and BPM-locked waveforms continue with the
  last stable phase rather than producing erratic spikes.

## Naming notes for new authors

- Symbols starting with `syn_` are the **compatibility namespace**; they
  exist to keep ported / imported scenes working under `compat-syn-v1`.
- Symbols starting with `chroma_` are the **native** namespace; new
  Chromaticity-first symbol families will use this prefix. The
  `chroma_Data_*` external-data namespace is reserved (catalog §1.6) and
  parsed but not populated at MVP.
- Symbols starting with `_` (single underscore) are pixel- or
  fragment-scope helpers and aspect-corrected coordinate utilities (`_uv`,
  `_uvc`, `_xy`, `_mouse`). Some are slice-3a-available (`_mouse`); most
  others land with the helper-shim shipping pass.
- `TIME`, `RENDERSIZE`, `FRAMECOUNT`, `PASSINDEX`, and `PI` are unprefixed
  per the reference legacy convention (catalog §1.1 – §1.4).

## Read next

- **Build something with these symbols:** `04-first-scene-tutorial.md`.
- **Scene manifest reference:** `02-scene-format.md`.
- **The full normative catalog:** `36-reactive-symbol-catalog.md` v1.2.0.
- **Audio analysis spec (numeric contract):** `06-audio-engine.md` §7.
- **Drift tolerance families:** `10-performance-latency-budgets.md`
  (`LVL`, `HIT`, `TIM`, `PRS`, `BEAT`, `BPM`, `LRG`).
