# Reactive Symbol Catalog (Normative)

This is the canonical inventory of every scene-facing symbol Chromaticity publishes to GLSL and the Script Host. It is the single source of truth for:
- the `compat-syn-v1` surface (must match reference behavior for parity)
- the `native-chroma-v1` surface (may add, may not remove)
- `EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, and `EV-AUDIO-TRACE` fixtures
- the drift/tolerance table in `10-performance-latency-budgets.md`
- the embedded docs generation (see `adr/ADR-010-embedded-docs-generation.md`)

## Conventions
- Cadence `per-frame` means the value is refreshed once per render frame before scene pass 0 runs.
- Cadence `per-pass` means the value is refreshed before every pass inside a frame.
- `Profile` values: `C` = Parity-Core (required for parity), `A` = Parity-Advanced, `D` = Differentiation.
- Tolerance IDs reference families in `10-performance-latency-budgets.md` Sec.Determinism and Drift.

### Namespace governance (normative)
- `syn_*` is a **compatibility namespace**. It exists to preserve reference-scene continuity under `compat-syn-v1`.
- `native-chroma-v1` is **native-first**: new symbols must use `chroma_*` (or the existing `_Meta` / legacy standard names already locked in this catalog).
- New feature work must not add fresh `syn_*` symbols as canonical names. If compat continuity is required, add a `syn_*` alias under `compat-syn-v1` only and document it in Sec.4.9 + Sec.8.
- Existing `syn_*` entries in this catalog are retained for parity continuity. They are valid input spellings under `compat-syn-v1`; native-authoring docs and templates should prefer `chroma_*` where a native equivalent exists.

## 1) Standard Uniforms

### 1.1 Frame / timing
| Symbol | GLSL type | Cadence | Profile | Semantics |
|---|---|---|---|---|
| `TIME` | `float` | per-pass | C | Seconds since scene started rendering. Monotonic. Resets on scene load. |
| `FRAMECOUNT` | `float` | per-pass | C | Frames rendered since scene load (float to match reference). |
| `TIMEDELTA` | `float` | per-pass | C | Seconds since previous frame (`native-chroma-v1` addition; in `compat-syn-v1` also exposed for forward-compat; not documented in reference but trivially non-breaking). |
| `PI` | `const float` | compile-time | C | `3.14159265358979323846`. |

### 1.2 Pixel coordinates
| Symbol | GLSL type | Cadence | Profile | Semantics |
|---|---|---|---|---|
| `_xy` | `vec2` | per-fragment | C | Integer pixel position. Equivalent to `gl_FragCoord.xy`. |
| `_uv` | `vec2` | per-fragment | C | Normalized `[0,1]` = `_xy / RENDERSIZE`. |
| `_uvc` | `vec2` | per-fragment | C | Aspect-corrected `_uv` centered at origin. Rule: `x` is scaled by `RENDERSIZE.x/RENDERSIZE.y` when landscape, else `y` is scaled by `RENDERSIZE.y/RENDERSIZE.x`. Matches reference behavior that circles drawn with `_uvc` are not stretched. |

### 1.3 Mouse
| Symbol | GLSL type | Cadence | Profile | Semantics |
|---|---|---|---|---|
| `_mouse` | `vec4` | per-frame | C | x, y, x_prev_click, y_prev_click (pixel units). `prev` components are negated while mouse is not down. Shadertoy `iMouse`-compatible. |
| `_click` | `vec3` | per-frame | C | `{left, right, middle}`: 1.0 while held, 0.0 otherwise. |
| `_muv` | `vec2` | per-frame | C | Mouse normalized `[0,1]` in preview. |
| `_muvc` | `vec2` | per-frame | C | Aspect-corrected mouse UV. |

### 1.4 Multipass
| Symbol | GLSL type | Cadence | Profile | Semantics |
|---|---|---|---|---|
| `RENDERSIZE` | `vec2` | per-pass | C | Pixel size of the CURRENT pass, not the scene. |
| `PASSINDEX` | `int` | per-pass | C | Pass index; starts 0, increments per pass. |
| `syn_FinalPass` | `sampler2D` | per-frame | C | Previous frame's final output; used for feedback. |

### 1.5 Media
| Symbol | GLSL type | Cadence | Profile | Semantics |
|---|---|---|---|---|
| `syn_Media` | `sampler2D` | per-frame | C | Selected media after meta-media transforms (contrast/gamma/hue/saturation/invert/scale). Use `_loadMedia` / `_textureMedia`. |
| `syn_UserImage` | `sampler2D` | per-frame | C | Raw media sampler before any meta-media transforms. Used by mask helpers and when scenes need the untouched source. |
| `syn_MediaType` | `float` | per-frame | C | Enum: `0` none, `1` image, `2` video, `3` webcam. Use `_isMediaActive()` preferred. |

### 1.6 Reserved namespace: external data ingestion (`chroma_Data_*`; `syn_Data_*` compat aliases) - activated post-MVP
Chromaticity reserves the `chroma_Data_*` namespace for V2+ external-data uniforms. For compatibility continuity, `syn_Data_*` remains accepted as an alias surface under `compat-syn-v1`. At MVP the namespace is **parsed but not populated**: scenes that declare data sources via the `DATA_SOURCES` scene.json key (see `02-synesthesia-contracts.md` Sec.1 and `28-implementation-contract-pack.md` CP-01) compile and run with the declared symbols exposed as zero-valued uniforms of the declared shape. This forward-compatibility reservation is what allows `41-data-ingestion-differentiator.md` to land as a data change, not a runtime rearchitecture.

Reserved shape primitives:

| Pattern | GLSL type | Backing resource | Post-MVP semantics |
|---|---|---|---|
| `chroma_Data_Scalar_<name>` (`syn_Data_Scalar_<name>` alias) | `float` | per-frame uniform | Single scalar value from the named source, sampled at the source's declared cadence; held-last-value between samples. |
| `chroma_Data_Vec_<name>` (`syn_Data_Vec_<name>` alias) | `vec3` / `vec4` | per-frame uniform | Vector value (e.g. observer position, `vec3` XYZ in user-chosen coordinate frame). |
| `chroma_Data_Array_<name>` (`syn_Data_Array_<name>` alias) | `sampler1D` | per-frame texture upload | Ordered sequence, newest at `u=0` (convention matches `syn_LevelTrail`). |
| `chroma_Data_Field_<name>` (`syn_Data_Field_<name>` alias) | `sampler2D` / `sampler3D` | per-frame texture upload | Scalar or vector field (weather grid, elevation, density volume). |
| `chroma_Data_Event_<name>` (`syn_Data_Event_<name>` alias) | ring buffer + `float chroma_Data_Event_<name>_count` (`syn_Data_Event_<name>_count` alias) | per-frame texture + scalar | Recent events, `count` holds valid-entry count, `sampler1D` holds per-event tuple. |

Rules (normative at MVP even though unpopulated):
- Names under `chroma_Data_*` and `syn_Data_*` are **strict-reject** unless a matching `DATA_SOURCES` entry declares the symbol. "Coerce to zero on typo" is explicitly **not** the policy - typos must fail loudly so authors can't quietly ship a scene that silently reads zeros instead of data.
- Native-first authoring (`native-chroma-v1`) must emit `chroma_Data_*`. `syn_Data_*` is accepted as a compat alias surface for imported/reference-parity scenes.
- Profile tag `D` (Differentiation) applies to every symbol in this namespace.
- Per-frame cadence is the runtime-visible cadence; sources with slower wall-clock cadence hold-last-value. Sources with faster wall-clock cadence are coalesced to one value per frame by the ingest subsystem (policy authored in ADR-017 post-MVP).
- Forward-compat `#include` behavior: scenes may `#include "data-helpers.glsl"` (a future Chromaticity-shipped helper library under `adr/ADR-010-embedded-docs-generation.md`); at MVP the include resolves to an empty file with a documented comment, so scenes already written against it compile.
- Fixture determinism: any golden-corpus scene that reads from `chroma_Data_*` or `syn_Data_*` must pin a recorded data cassette per `22-compatibility-versioning-policy.md` Sec.Data-pinned fixtures.

## 2) Audio Uniforms

All audio uniforms are refreshed into a versioned snapshot at the audio-thread cadence (`375 Hz` per `06-audio-engine.md` Sec.7.1) and handed off to the render loop once per frame before pass 0 via the CP-02a snapshot contract (`28-implementation-contract-pack.md`). Every pass in a frame observes the same snapshot. Band edges, smoothing constants, hit/presence/time/BPM algorithms, and `chroma_PrimeBang` (`syn_PrimeBang` compat alias) heuristics are normatively specified in `06-audio-engine.md` Sec.7 and tested via `EV-AUDIO-TRACE` per the family tolerance rows in `10-performance-latency-budgets.md`. The staleness bound between an audio hop and its visible frame is `~19.3 ms` at 60 FPS (`06-audio-engine.md` Sec.7.11).

### 2.1 Data textures
| Symbol | GLSL type | Channels | Profile | Semantics |
|---|---|---|---|---|
| `syn_Spectrum` | `sampler1D` (or emulated 2D with height 1) | `r`=Raw FFT, `g`=Juiced FFT, `b`=Smooth FFT, `a`=Waveform | C | `Juiced FFT` is the default authoring channel: bin smoothing + temporal smoothing + logarithmic frequency scaling + high-end boost. `Smooth FFT` is a lower-resolution variant with the same smoothing/scaling. Waveform refreshes every frame. |
| `syn_LevelTrail` | `sampler1D` | `r`=Whole, `g`=Bass, `b`=Mid, `a`=High | C | Historical per-band levels over time, newest on one edge (convention: newest at `u=0`). Used for trail effects. |

**Reference-engine per-band spellings (non-parity surface).** The reference `SynesthesiaEngine.framework` binary also exposes per-band time-series samplers as separate names (`syn_TimeSeries`, `syn_BassTimeSeries`, `syn_MidTimeSeries`, `syn_HighTimeSeries`) and per-channel spectrum samplers (`syn_SpectrumSmooth`, `syn_SpectrumJuiced`, `syn_Waveform`). These are **not** published under `compat-syn-v1` or `native-chroma-v1`; authors must read the packed RGBA channels of `syn_LevelTrail` and `syn_Spectrum`. Scenes referencing the per-band or per-channel spellings are strict-rejected and compat-coerced to the corresponding channel read of the canonical sampler with a `coercion-applied` diagnostic (see `28-implementation-contract-pack.md` CP-01 CR-06 media-sampler alias rule, extended to audio samplers).

### 2.2 Level (normalized `[0,1]`, per-frame, family tolerance `LVL`)
`syn_Level`, `syn_BassLevel`, `syn_MidLevel`, `syn_MidHighLevel`, `syn_HighLevel`.

### 2.3 Hits (normalized `[0,1]`, per-frame, family tolerance `HIT`)
`syn_Hits`, `syn_BassHits`, `syn_MidHits`, `syn_MidHighHits`, `syn_HighHits`.

### 2.4 Time clocks (non-normalized, per-frame, family tolerance `TIM`)
`syn_Time`, `syn_BassTime`, `syn_MidTime`, `syn_MidHighTime`, `syn_HighTime`, `syn_CurvedTime` (same base as `syn_Time`, with exponential acceleration on volume rises).

### 2.5 Presence (normalized `[0,1]`, per-frame, family tolerance `PRS`)
`syn_Presence`, `syn_BassPresence`, `syn_MidPresence`, `syn_MidHighPresence`, `syn_HighPresence`.

### 2.6 Beat (per-frame, family tolerance `BEAT`)
| Symbol | Type | Semantics |
|---|---|---|
| `syn_OnBeat` | `float` | 1.0 on beat detection, decays rapidly to 0.0. |
| `syn_ToggleOnBeat` | `float` | Alternates 0.0/1.0 on each detected beat, logistic-smoothed transitions. |
| `syn_RandomOnBeat` | `float` | New random value in `[0,1]` on each beat, logistic-smoothed between values. |
| `syn_BeatTime` | `float` | Integer clock incremented on each detected beat. |

### 2.7 BPM (per-frame, family tolerance `BPM`)
| Symbol | Type | Semantics |
|---|---|---|
| `syn_BPM` | `float` | Estimated BPM, clamped `[50, 220]`. |
| `syn_BPMConfidence` | `float` | `[0,1]`; near 1 when BPM is stable. |
| `syn_BPMTwitcher` | `float` | Exponential "jump" clock; `floor` = beat count; `fract` = per-beat twitch. |
| `syn_BPMSin` | `float` | Sine `[0,1]` at BPM frequency, beat-phase-locked. |
| `syn_BPMSin2` | `float` | Sine `[0,1]` at BPM/2. |
| `syn_BPMSin4` | `float` | Sine `[0,1]` at BPM/4. |
| `syn_BPMTri` | `float` | Triangle `[0,1]` at BPM. |
| `syn_BPMTri2` | `float` | Triangle `[0,1]` at BPM/2. |
| `syn_BPMTri4` | `float` | Triangle `[0,1]` at BPM/4. |

### 2.8 Large features (per-frame, family tolerance `LRG`)
| Symbol | Type | Semantics |
|---|---|---|
| `syn_FadeInOut` | `float` | Slowly rises to 1.0 on music start, falls to 0.0 on silence. |
| `syn_Intensity` | `float` | Slow accumulator to 1.0 tracking overall song intensity. |

### 2.9 Heuristic differentiator: `chroma_PrimeBang` (`syn_PrimeBang` compat alias) (Profile D)
The reference contains bundled scenes using `syn_PrimeBang`-style names (observed in `boreal.synScene/script.js`). Chromaticity's native canonical symbol is `chroma_PrimeBang`; `syn_PrimeBang` is retained as a compatibility alias for scene continuity under `compat-syn-v1`. This signal is a **Profile D (Differentiation) heuristic**, not a behavior-cloned parity symbol. Scenes relying on it will receive a Chromaticity-original pulse on large-scale musical transitions; scenes that want bit-exact reference parity should avoid it.
- Formula and acceptance criterion are locked in `06-audio-engine.md` Sec.7.14.
- Fixtures live in `34-golden-corpus-definition.md` under the Profile-D heuristic category; they assert trigger behavior under seeded input, not numeric parity.

### 2.10 Reference-surface uniforms not in the documented catalog
These symbols appear in `SynesthesiaEngine.framework` and/or bundled reference scenes but are absent from `ssf/audio_uniforms.html`. They are shipped-behaviour and must have a defined Chromaticity disposition so import and parity fixtures are deterministic.

| Symbol | Type | Profile | Evidence | Chromaticity behaviour |
|---|---|---|---|---|
| `syn_InputLevel` | `float` | C | `audio_indicators.synScene` (GLSL + JS) | Pre-AGC instantaneous input-level, range `[0, inf)`. Published as `clamp(RMS(currentAudioHop) / AGC_targetRMS, 0, inf)`. Authors who want clamped/normalized behavior use `syn_Level`. Formula locked in `06-audio-engine.md` Sec.7.12. |
| `syn_RawLevel` | `float` | C | `ssf/audio_uniforms.html` (documented, not observed in release scenes) | Alias of `syn_InputLevel` with identical semantics. Both spellings are authoring-surface because the reference docs use `syn_RawLevel` and reference scenes use `syn_InputLevel`. |
| `syn_BassExpoCurve` | `float` | C | 1 release scene (GLSL) | `pow(syn_BassLevel, 3.0)`, i.e. exponential curve on bass level for non-linear reactivity. Formula locked in `06-audio-engine.md` Sec.7.12. Forward-compat: native extension family uses `chroma_*ExpoCurve`; any `syn_*ExpoCurve` additions are compat aliases only. |
| `syn_Threshold` | `float` | C | `SynesthesiaEngine.framework` symbol, no release-scene usage | Operator-settable noise-floor gate in `[0,1]`; audio hops with RMS below this value publish zeroed `syn_Level*` and `syn_*Hits` families. Exposed via settings panel and OSC route `/audio/threshold {float}`. Formula locked in `06-audio-engine.md` Sec.7.12. |
| `syn_PrimeAndBang` | `float` | C | `boreal.synScene/script.js` (commented line) | Deprecated alias for `chroma_PrimeBang` (legacy `syn_PrimeBang` spelling retained as compat alias). Resolved via the alias table in `28-implementation-contract-pack.md` CP-07. Strict-rejected, compat-warn-and-coerce. |

Rationale for promoting these to Profile C: `syn_InputLevel` appears in a shipped reference scene, so any parity scene loader that rejects it breaks out-of-the-box. `syn_RawLevel` is the reference's documented name for the same value; shipping only one of the two spellings leaves authors who followed the docs stranded. `syn_BassExpoCurve` likewise breaks exactly one shipped scene if omitted but the formula is trivial.

### 2.11 Native audio analysis primitives (`chroma_*`; slice-5a-introduced)

These three uniforms are net-new chromaticity-native primitives introduced by the slice-5a `engine/render` ↔ `engine/audio` cross-crate adapter (`engine/render/src/audio_adapter.rs`). They expose raw analysis values from the slice-1 `AudioUniformSnapshot` producer, complementing the normalised / smoothed `syn_Level*` (Sec.2.2) and `syn_Spectrum` (Sec.2.1) parity surfaces with lower-level inputs that scenes can post-process to taste. They are a chromaticity-first surface — the catalog has no native equivalents at this abstraction level — so the namespace governance rule (Sec.Namespace governance) requires the `chroma_*` prefix for the new symbols.

| Symbol | GLSL type | Profile | Cadence | Semantics |
|---|---|---|---|---|
| `chroma_AudioRMS` | `float` | D | per-frame | Instantaneous root-mean-square energy of the current audio hop, sourced from `chromaticity_audio::extract_uniform_snapshot`. Non-normalised (range `[0, inf)`); for normalised band-level reactivity use the `syn_Level*` family. |
| `chroma_AudioPeak` | `float` | D | per-frame | Instantaneous peak absolute magnitude of the current audio hop. Non-normalised; pairs with `chroma_AudioRMS` for envelope-shaped reactivity. |
| `chroma_AudioFFT[8]` | `float[8]` | D | per-frame | Log-spaced FFT bands `[0..=7]` from `AudioUniformSnapshot::fft_bands`. Element `[0]` is the lowest band (low-edge `chromaticity_audio::FFT_BAND_MIN_HZ`); element `[7]` is the highest band. Authors index the array directly (`chroma_AudioFFT[K]`). Band-edge contract is the producer-side FFT analyser (`06-audio-engine.md` Sec.7.4). |

Notes:

- `syn_BPM` (Sec.2.7) is the fourth uniform the slice-5a adapter binds and is unchanged here. It stays under `compat-syn-v1` canonical retention per Sec.Namespace governance line 18; the adapter marshals it from `AudioUniformSnapshot::bpm` (the slice-2 history-window estimator).
- **`chroma_AudioFFT[8]` is the consolidated v1.3.0 shape.** The slice-5a initial release (catalog v1.2.0) used a transitional split-vec4 spelling (`chroma_AudioFFTLow` for bands `[0..=3]` + `chroma_AudioFFTHigh` for bands `[4..=7]`) because `engine/render/src/multipass.rs::UniformValue` had no array variant. The slice-5 follow-up landed `UniformValue::FloatArray` and consolidated the FFT into a single 8-element array binding. The two scalar primitives (`chroma_AudioRMS`, `chroma_AudioPeak`) and `syn_BPM` are unchanged across the consolidation; only the FFT shape moved. The std140 packing of `float arr[N]` puts each element on a 16-byte (vec4) boundary; the per-pass uniform buffer grows by 8 slots for each `FloatArray(8)` binding. Authors who previously wrote `chroma_AudioFFTLow.x` migrate to `chroma_AudioFFT[0]`; `chroma_AudioFFTHigh.w` migrates to `chroma_AudioFFT[7]`.
- Evidence anchors: `docs/26-evidence-artifact-registry.md` `EV-RENDER-CONFORMANCE` (`EV-RENDER-CONFORMANCE`) and `EV-AUDIO-TRACE` (`EV-AUDIO-TRACE`) both cite the slice-5a adapter, its T1 drift pins (`engine/render/src/audio_adapter.rs::audio_adapter_drift_pins`, including `adapter_packs_fft_bands_into_float_array_in_band_order`), the new `multipass::multipass_drift_pins::uniform_marshalling_float_array_layout` pin, and its T2 GPU-roundtrip corpus (`engine/render/tests/audio_adapter.rs`, with the v1.3.0 ACCEPTED hash for the sine-tone fixture reflecting the GLSL `arr[K]` summation order; the silent-frame hash is preserved across the consolidation because zero FFT bands sum identically under any summation order).

## 3) Meta Controls

Meta controls are global runtime state, published as GLSL uniforms with a leading underscore naming convention (`_Foo`) and helper functions that apply them (`_applyFoo`). They are owned by `StateCore` (see `28-implementation-contract-pack.md` CP-04). They persist across scene switches unless the operator resets them.

### 3.1 Color meta (scene output)
| Symbol | Type | Default | Range | Applied-by | Formula |
|---|---|---|---|---|---|
| `_Master` | `float` | 1.0 | `[0,1]` | `_applyMaster` | `clamp(col,0,1); col *= _Master`. Linear fade to black. |
| `_Brightness` | `float` | 1.0 | `[0,2]` | `_applyBrightness` | `_brightness(col, _Brightness)`. |
| `_Gamma` | `float` | 1.0 | `[0,2]` | `_applyGamma` | `gamma = _Gamma; if (gamma>1) gamma = 1 + (gamma-1)*2; _gamma(col, gamma)`. Extended range `[0,3]`. |
| `_Contrast` | `float` | 1.0 | `[0,2]` | `_applyContrast` | `c = _Contrast; if (c>1) c *= c; _contrast(col, c)`. |
| `_Hue` | `float` | 0.0 | `[-1,1]` | `_applyHueSaturation` | `_hueSaturation(col, _Hue, _Saturation)`. |
| `_Saturation` | `float` | 0.0 | `[-1,1]` | `_applyHueSaturation` | HSV saturation: `col.s = clamp(col.s*(s*abs(s)*2.5+1), 0, 1)`. |
| `_Invert` | `float` | 0.0 | `[0,1]` | `_applyInvert` | `_invert(col, _Invert)`. |

### 3.2 Media meta (applied during media ingest pass, before scene reads `syn_Media`)
| Symbol | Type | Default | Range | Applied-by | Formula |
|---|---|---|---|---|---|
| `_Media_Contrast` | `float` | 1.0 | `[0,2]` | `_applyMediaContrast` | Same squaring rule as `_Contrast`. |
| `_Media_Gamma` | `float` | 1.0 | `[0,2]` | `_applyMediaGamma` | `g = _Media_Gamma; if (g>1) g *= g; _gamma(col, g)`. |
| `_Media_Hue` | `float` | 0.0 | `[-1,1]` | `_applyMediaHueSaturation` | `_hueSaturation(col, _Media_Hue, _Media_Saturation)`. |
| `_Media_Saturation` | `float` | 0.0 | `[-1,1]` | `_applyMediaHueSaturation` | (see `_Saturation`). |
| `_Invert_Media` | `float` | 0.0 | `[0,1]` | `_applyInvertMedia` | `_invert(col, _Invert_Media)`. |
| `_Media_Scale` | `float` | 1.0 | `[0.1, 10.0]` | `_applyMediaScale` | Scales sample UVs in scene-space so letterbox/black-bar regions are preserved, not clipped. |

### 3.3 Geometry meta
| Symbol | Type | Default | Semantics |
|---|---|---|---|
| `_Vert_Mirror` | `float` | 0.0 | When `>0.5`, pixels with `uv.x>0.5` are mirrored: `uv.x = 1 - uv.x`. |
| `_Hor_Mirror` | `float` | 0.0 | When `>0.5`, pixels with `uv.y<0.5` are mirrored: `uv.y = 1 - uv.y`. |
| `_Alpha_Channel` | `float` | 0.0 | When `<0.5`, scene alpha is forced to 1.0 at output. |

### 3.4 Dichrome meta
| Symbol | Type | Default | Semantics |
|---|---|---|---|
| `_Limit_Colors` | `float` | 0.0 | When `>0.5`, dichrome pass runs. |
| `_Low_Color` | `vec3` | `vec3(0.0)` | Dark end of dichrome ramp. |
| `_High_Color` | `vec3` | `vec3(1.0)` | Light end of dichrome ramp. |

### 3.5 Preference-backed meta
| Symbol | Type | Default | Semantics |
|---|---|---|---|
| `syn_Prefs_ZoomOrFit` | `float` | 1.0 | When `>0.5`, `_zoomOrFit(wr, hr)` returns `max(wr, hr)` (zoom-to-fill); else `min(wr, hr)` (letterbox fit). Governs media/output fit behavior. |

### 3.6 Differentiation (edition-gated; `Reactivity`, `Audio Speed`)
| Symbol | Type | Default | Profile | Semantics |
|---|---|---|---|---|
| `_Reactivity` | `float` | 1.0 | A (Advanced edition) | Global multiplier on all audio uniforms before shader read. Matches reference "Reactivity" Pro meta. |
| `_Audio_Speed` | `float` | 1.0 | A (Advanced edition) | Multiplier on the time clocks (`syn_*Time`, `syn_CurvedTime`). |

### 3.7 Meta-pipeline ordering (normative)
1. Media ingest -> `_Media_Contrast` -> `_Media_Gamma` -> `_Media_Hue/Saturation` -> `_Invert_Media` -> write `syn_Media`. `syn_UserImage` is the pre-transform snapshot.
2. `_Media_Scale` is applied in scene-sample-space by media helper functions, not in the ingest pass.
3. Scene renders -> apply `_Vert_Mirror` / `_Hor_Mirror` on UV sampling per helper.
4. Apply `_applyDichrome` if `_Limit_Colors > 0.5`.
5. Apply `_applyColorCorrection`: `Invert -> HueSaturation -> Contrast -> Gamma -> Brightness`.
6. Apply `_applyMaster` (final clamp and linear fade).
7. Apply `_applyAlpha`.

## 4) GLSL Helper Shim Catalog

All helpers are Chromaticity originals (clean-room). Signatures match reference so existing scenes compile unchanged. Every helper has a fixture in `34-golden-corpus-definition.md` Sec.Helper fixtures.

### 4.1 Math
`_scale(float v, float min, float max) -> float`
`_map(T v, T min, T max, T new_min, T new_max) -> T` (T in `float|vec2|vec3|vec4`)
`_smin(float a, float b, float k) -> float` (Inigo-Quilez style smooth min)
`_rand(float seed) -> float`, `_rand(vec2 seed) -> float`
`_pulse(float position, float center, float size) -> float` (smooth)
`_sqPulse(float position, float center, float size) -> float` (square)
`_inRange(T v[, float min=0, float max=1]) -> float` (T in `float|vec2|vec3|vec4`)
`_triWave(float position, float period) -> float` (range `[-0.5, 0.0]`, per reference)
`_pixelate(T v, float amount) -> T` (T in `float|vec2|vec3`)
`_mix3(T,T,T, float) -> T`, `_mix4(T,T,T,T, float) -> T`, `_mix5(T,T,T,T,T, float) -> T`
`_nsin(float) -> float`, `_ncos(float) -> float` (sin/cos normalized to `[0,1]`)
`_nclamp(T) -> T` (clamp to `[0,1]`)

### 4.2 Noise
`_noise(vec2|vec3) -> float` (value noise)
`_fbm(vec2|vec3) -> float` (fractal Brownian motion)
`_statelessContinuousChaotic(vec2|vec3, float) -> float`
`_hash11`, `_hash12`, `_hash13`, `_hash21`, `_hash22`, `_hash23`, `_hash31`, `_hash32`, `_hash33` (pattern: `_hash{out}{in}()`; `in`/`out` encode vec dimensions)

### 4.3 Coordinates
`_uv2uvc(vec2) -> vec2`
`_uvc2uv(vec2) -> vec2`
`_rotate(vec2, float rad) -> vec2`, `_rotate(vec3, vec3 axis, float rad) -> vec3`
`_mirror(vec2) -> vec2` (one-axis default; overloads per axis)
`_toPolar(vec2) -> vec2` (angle/radius; reference-compat curve)
`_toPolarTrue(vec2) -> vec2` (true mathematical polar)
`_toRect(vec2) -> vec2`

### 4.4 Color
`_rgb2hsv(vec3) -> vec3`, `_hsv2rgb(vec3) -> vec3`
`_normalizeRGB(vec3|vec4) -> vec3|vec4`
`_palette(float t, vec3 a, vec3 b, vec3 c, vec3 d) -> vec3` (Inigo-Quilez palette)
`_brightness(vec3|vec4 col, float b) -> vec3|vec4`
`_gamma(vec3|vec4 col, float g) -> vec3|vec4`
`_contrast(vec3|vec4 col, float c) -> vec3|vec4`
`_saturation(vec3|vec4 col, float s) -> vec3|vec4`
`_hueRotate(vec3|vec4 col, float degrees) -> vec3|vec4`
`_invert(vec3|vec4 col, float amount) -> vec3|vec4`
`_luminance(vec3|vec4) -> float`
`_grayscale(vec3|vec4[, float intensity=1]) -> vec3|vec4`
`_dichrome(vec3|vec4 col, vec3 low, vec3 high) -> vec3|vec4`

### 4.5 Media
`_loadMedia([vec2 offset,] [float lod]) -> vec4`
`_textureMedia(vec2 uv[, float lod]) -> vec4`
`_loadMediaAsMask([vec2 offset,] [float lod]) -> vec4`
`_textureMediaAsMask(vec2 uv[, float lod]) -> vec4`
`_correctMediaCoords([vec2 uv]) -> vec2`
`_isMediaActive() -> bool`
`_edgeDetectMedia() -> vec4`
`_edgeDetectSobelMedia() -> vec4`

### 4.6 Multipass
`_edgeDetect(sampler2D tex[, vec2 uv]) -> vec4`
`_edgeDetectSobel(sampler2D tex[, vec2 uv]) -> vec4`
`_hBlurTiltShift(sampler2D tex[, vec2 uv][, float blurAmt=5.0][, float blurLoc=0.5]) -> vec4`
`_vBlurTiltShift(sampler2D tex[, vec2 uv][, float blurAmt=5.0][, float blurLoc=0.5]) -> vec4`

### 4.7 Convenience alias `_textureImage`
The reference ships scenes that call `_textureImage(sampler2D, vec2)`. It is not documented but is used (e.g. `[boreal.synScene/main.glsl](../Contents/Resources/release/boreal.synScene/main.glsl)`). Chromaticity must supply it under `compat-syn-v1` as:
`_textureImage(sampler2D tex, vec2 uv) -> vec4 = texture(tex, _correctMediaCoords(uv))` (default) with a spec fixture in the corpus.

### 4.8 Meta helpers (prepended to every compiled scene)
`_applyMaster`, `_applyBrightness`, `_applyGamma`, `_applyContrast`, `_applyMediaContrast`, `_applyMediaGamma`, `_applyMediaHueSaturation`, `_applyHueSaturation`, `_applyInvert`, `_applyInvertMedia`, `_applyColorCorrection`, `_applyDichrome`, `_applyVerticalMirroring`, `_applyHorizontalMirroring`, `_zoomOrFit`, `_applyAlpha`, `_applyMediaScale`, `_correctImageCoords`, `_flipMediaCoords`, `_imageToMask`, `_hueSaturation`.

### 4.9 Deprecated compatibility aliases
Shipped for `compat-syn-v1` only; not available under `native-chroma-v1`:
`_applyAllLuminanceCorrections`, `_hueSaturationLEGACY`, `_hueSaturationContrast`, `_hueSaturationContrastLEGACY`, `_applyHueSaturationContrast`, `_invertImage`, `_loadUserImage`, `_textureUserImage`, `_loadUserImageAsMask`, `_textureUserImageAsMask`.

## 5) Script Host Mirror Rules

The Script Host (`script.js`) sees every symbol in Sec.1-3 as a JS global with matching name, EXCEPT:
- Fragment-only: `_uv`, `_uvc`, `_xy` are not available (no per-pixel semantics in script scope).
- Pass-only: `PASSINDEX`, `RENDERSIZE` are not available (no pass scope in script).
- Samplers: `syn_FinalPass`, `syn_Media`, `syn_UserImage`, `syn_Spectrum`, `syn_LevelTrail` are not exposed as direct values; use `textureFinalPass(x, y)` and a future `textureMedia(x, y)` helper (tracked in CP-07 addendum).

Vector values are JS objects: `vec2 = {x, y}`, `vec3 = {x, y, z}` or `{r, g, b}`, `vec4 = {x, y, z, w}` or `{r, g, b, a}`.

**Array-shaped uniforms are producer-only on the script-write side.** Symbols whose underlying GLSL type is `float[N]` (or any future `vec2[N]` / `vec4[N]` batched array) — currently `chroma_AudioFFT[8]` from §2.11, and reserved future `syn_RES`-style vec2-batched uniforms cited in `engine/render/src/multipass.rs` module prose — are exposed to the script host as JS arrays for read access via the producer pipeline, but cannot be the target of `__chromaticityEmit(name, value)` reactive writes from script. The `engine/script` carrier vocabulary (`ReactiveValueKind::{Scalar, Vec2, Vec3, Vec4}`) does not yet support 8-element float-array writes; see the inline retirement note in `engine/script/src/reactive_symbol.rs` for the canonical reasoning. Catalog v1.3.0 retired the v1.2.0 transitional split-vec4 spelling (`chroma_AudioFFTLow` / `chroma_AudioFFTHigh`) on the script-write side alongside the engine-side consolidation; an array-carrier `ReactiveValueKind` variant is future-slice scope and will reopen the script-write surface for `chroma_AudioFFT[8]` when it lands.

Controls appear as globals named exactly as their `NAME`. For a scene with `NAME: "rate"`, the script global `rate` is the current value. The deprecated `inputs[name]` and `uniforms[name]` shims mirror these under `compat-syn-v1` only.

### 5.1 Known limitations (script-write surface)

- **`chroma_AudioFFT[8]` is producer-only on the script-write side.** The audio→render path is wired end-to-end (`engine/audio::AudioUniformSnapshot::fft_bands` → `engine/render::audio_adapter::uniform_input_from_audio_snapshot` → `UniformValue::FloatArray(8)` GPU binding under catalog v1.3.0), but the script→runtime path does not yet support 8-element float-array reactive writes: `engine/script::ReactiveValueKind::{Scalar, Vec2, Vec3, Vec4}` carries no array variant, so `"chroma_AudioFFT"` resolves to `None` from `ReactiveSymbol::from_name` and surfaces to script as `ScriptError::ReactiveSymbolUnknown` if a scene attempts a `__chromaticityEmit("chroma_AudioFFT", …)` write. Workaround for scripted scenes that need to synthesize / mock the FFT surface today: write the producer-side `AudioUniformSnapshot` directly (test-harness path), or fall back to the per-band scalar primitives `chroma_AudioRMS` / `chroma_AudioPeak` for coarse level-driven reactivity. Closing the asymmetry requires a `ReactiveValueKind::FloatArray<N>` variant plus matching carrier-shape extension on the typed-reactive-write pathway. Tracked as Phase-3+ feature work alongside the deferred axes in `docs/00-phase-0-placeholder-inventory.md` Phase-3+ known limitations and `docs/13-risk-register.md` R-37 Deferred axes; will reopen the script-write surface for `chroma_AudioFFT[8]` when it lands.

## 6) Coercion Classes (Script and GLSL)
Canonical names live in this catalog. Coercion classes `CR-01..CR-04` (see `28-implementation-contract-pack.md`) apply to:
- Case mismatch on control `NAME` -> strict reject, compat warn-and-coerce to canonical case.
- Reading an undeclared audio uniform -> strict compile error, compat compile to `0.0`.
- Writing to a read-only uniform -> strict runtime error, compat warn-and-ignore.
- Calling a helper with wrong arity -> strict compile error; compat attempts the reference-equivalent overload when unambiguous.
- Namespace coercion: `syn_*` spellings that have a documented `chroma_*` canonical twin remain accepted as compat aliases under `compat-syn-v1`; native templates/docs must emit `chroma_*`.

## 7) Drift Tolerance Cross-Reference
Families in Sec.2 map to the tolerance rows in `10-performance-latency-budgets.md`:
- `LVL` - level family
- `HIT` - hit family
- `TIM` - time family (including `syn_CurvedTime`)
- `PRS` - presence family
- `BEAT` - `syn_OnBeat`, `syn_ToggleOnBeat`, `syn_RandomOnBeat`, `syn_BeatTime`
- `BPM` - BPM family (all `syn_BPM*`)
- `LRG` - `syn_FadeInOut`, `syn_Intensity`

Meta-control visual drift is tested by the meta-pipeline fixture set (see `34-golden-corpus-definition.md`).

## 8) Change Control
- Any addition to this catalog requires: (a) a bumped catalog version; (b) a new `EV-*` fixture; (c) an entry in `22-compatibility-versioning-policy.md` release notes.
- Any removal is a breaking change and requires a major profile bump.
- Any new `syn_*` symbol proposal requires explicit compatibility-only justification and an accompanying `chroma_*` canonical proposal (or a written exception citing unavoidable parity lock-in).
- Catalog version at ratification: `1.0.0` (targets Chromaticity `compat-syn-v1` and `native-chroma-v1` baseline).
- Catalog version `1.1.0`: added Sec.2.10 reference-surface uniforms (`syn_InputLevel`, `syn_RawLevel`, `syn_BassExpoCurve`, `syn_Threshold`, `syn_PrimeAndBang`) and Sec.2.1 non-parity per-band time-series/spectrum spelling disposition. Formulas in `06-audio-engine.md` Sec.7.12.
- Catalog version `1.2.0`: added Sec.2.11 native audio analysis primitives (`chroma_AudioRMS`, `chroma_AudioPeak`, `chroma_AudioFFTLow`, `chroma_AudioFFTHigh`) introduced by the slice-5a `engine/render` ↔ `engine/audio` cross-crate adapter. Sec.8 (b) `EV-*` fixture coverage is satisfied by slice-5a's pre-existing T1 drift pins (`engine/render/src/audio_adapter.rs::audio_adapter_drift_pins`) and T2 GPU-roundtrip corpus (`engine/render/tests/audio_adapter.rs`) under `EV-RENDER-CONFORMANCE` and `EV-AUDIO-TRACE`; release-notes entry appended to `docs/22-compatibility-versioning-policy.md`. The split-`vec4` FFT spelling (`chroma_AudioFFTLow` + `chroma_AudioFFTHigh`) was documented as transitional pending an array variant on `engine/render/src/multipass.rs::UniformValue`; that consolidation landed in v1.3.0.
- Catalog version `1.3.0`: consolidated the v1.2.0 split-vec4 FFT spelling (`chroma_AudioFFTLow` + `chroma_AudioFFTHigh`) into a single `chroma_AudioFFT[8]` array binding. Foundational widening: added `UniformValue::FloatArray` to `engine/render/src/multipass.rs` with std140-friendly packing (each element vec4-aligned to 16 bytes, matching `uniform float arr[N]`), and rebuilt `engine/render/src/audio_adapter.rs::uniform_input_from_audio_snapshot` to bind `chroma_AudioFFT` once via `UniformValue::FloatArray(snapshot.fft_bands.to_vec())`. The two scalar primitives (`chroma_AudioRMS`, `chroma_AudioPeak`) and `syn_BPM` are unchanged across the consolidation. Sec.8 (b) `EV-*` fixture coverage: existing slice-5a T1 drift pins are updated in place to assert the new shape (`adapter_returns_four_uniforms_in_documented_order`, `adapter_packs_fft_bands_into_float_array_in_band_order`); a new `multipass::multipass_drift_pins::uniform_marshalling_float_array_layout` pin covers the std140 packing invariant; the T2 GPU-roundtrip corpus's sine-tone ACCEPTED hash is rolled forward (the silent-frame hash is preserved verbatim because zero FFT bands sum identically under any summation order). Release-notes entry appended to `docs/22-compatibility-versioning-policy.md`. Authoring-surface migration: `chroma_AudioFFTLow.x` → `chroma_AudioFFT[0]`, ..., `chroma_AudioFFTHigh.w` → `chroma_AudioFFT[7]`. No scenes shipped against the v1.2.0 split-vec4 surface, so no compat aliases are required.
