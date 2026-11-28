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
Per `adr/ADR-024-native-chroma-vocabulary.md`:
- `chroma_*` is the **only native reactive namespace**, alongside the non-prefixed standard names already locked in this catalog (`TIME`, `RENDERSIZE`, `PASSINDEX`, `_mouse`, `_muv`, the `_Meta` controls and the `_helper` functions).
- Every canonical name in this catalog is native. There are **no compatibility aliases** in the native catalog: native parsers (`ReactiveSymbol::from_name` in `engine/script/src/reactive_symbol.rs`, the scene validator) accept `chroma_*` spellings only.
- Reference-format names exist only at the `.synScene` import boundary. They are translated to native names by the single frozen table `SYN_TO_NATIVE` in `engine/authoring/src/syn_import.rs`, applied by the import adapters (`engine/authoring/src/syn_import.rs`, `app/shell/src/syn_compat.rs`). The full table is reproduced in Sec.9.
- New feature work must not introduce reference-format names anywhere outside import code. `scripts/syn-prefix-lint.sh` enforces this boundary over engine, app, integrations, frontend, bundled scenes, user docs and built help.

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
| `chroma_FinalPass` | `sampler2D` | per-frame | C | Previous frame's final output; used for feedback. |

### 1.5 Media
| Symbol | GLSL type | Cadence | Profile | Semantics |
|---|---|---|---|---|
| `chroma_Media` | `sampler2D` | per-frame | C | Selected media after meta-media transforms (contrast/gamma/hue/saturation/invert/scale). Use `_loadMedia` / `_textureMedia`. Imported scenes that read either of the reference format's two media samplers (post-transform or raw) are bound to `chroma_Media` (see Sec.9); there is no separate native raw-media sampler name. |
| `chroma_MediaType` | `float` | per-frame | C | Enum: `0` none, `1` image, `2` video, `3` webcam. Use `_isMediaActive()` preferred. |

### 1.6 Reserved namespace: external data ingestion (`chroma_Data_*`) - activated post-MVP
Chromaticity reserves the `chroma_Data_*` namespace for V2+ external-data uniforms. The reference format has no equivalent surface, so there is no import mapping for it. At MVP the namespace is **parsed but not populated**: scenes that declare data sources via the `DATA_SOURCES` scene.json key (see `02-synesthesia-contracts.md` Sec.1 and `28-implementation-contract-pack.md` CP-01) compile and run with the declared symbols exposed as zero-valued uniforms of the declared shape. This forward-compatibility reservation is what allows `41-data-ingestion-differentiator.md` to land as a data change, not a runtime rearchitecture.

Reserved shape primitives:

| Pattern | GLSL type | Backing resource | Post-MVP semantics |
|---|---|---|---|
| `chroma_Data_Scalar_<name>` | `float` | per-frame uniform | Single scalar value from the named source, sampled at the source's declared cadence; held-last-value between samples. |
| `chroma_Data_Vec_<name>` | `vec3` / `vec4` | per-frame uniform | Vector value (e.g. observer position, `vec3` XYZ in user-chosen coordinate frame). |
| `chroma_Data_Array_<name>` | `sampler1D` | per-frame texture upload | Ordered sequence, newest at `u=0` (convention matches `chroma_LevelTrail`). |
| `chroma_Data_Field_<name>` | `sampler2D` / `sampler3D` | per-frame texture upload | Scalar or vector field (weather grid, elevation, density volume). |
| `chroma_Data_Event_<name>` | ring buffer + `float chroma_Data_Event_<name>_count` | per-frame texture + scalar | Recent events, `count` holds valid-entry count, `sampler1D` holds per-event tuple. |

Rules (normative at MVP even though unpopulated):
- Names under `chroma_Data_*` are **strict-reject** unless a matching `DATA_SOURCES` entry declares the symbol. "Coerce to zero on typo" is explicitly **not** the policy - typos must fail loudly so authors can't quietly ship a scene that silently reads zeros instead of data.
- All authoring must emit `chroma_Data_*`; no alias spelling is accepted.
- Profile tag `D` (Differentiation) applies to every symbol in this namespace.
- Per-frame cadence is the runtime-visible cadence; sources with slower wall-clock cadence hold-last-value. Sources with faster wall-clock cadence are coalesced to one value per frame by the ingest subsystem (policy authored in ADR-017 post-MVP).
- Forward-compat `#include` behavior: scenes may `#include "data-helpers.glsl"` (a future Chromaticity-shipped helper library under `adr/ADR-010-embedded-docs-generation.md`); at MVP the include resolves to an empty file with a documented comment, so scenes already written against it compile.
- Fixture determinism: any golden-corpus scene that reads from `chroma_Data_*` must pin a recorded data cassette per `22-compatibility-versioning-policy.md` Sec.Data-pinned fixtures.

## 2) Audio Uniforms

All audio uniforms are refreshed into a versioned snapshot at the audio-thread cadence (`375 Hz` per `06-audio-engine.md` Sec.7.1) and handed off to the render loop once per frame before pass 0 via the CP-02a snapshot contract (`28-implementation-contract-pack.md`). Every pass in a frame observes the same snapshot. Band edges, smoothing constants, hit/presence/time/BPM algorithms, and `chroma_PrimeBang` heuristics are normatively specified in `06-audio-engine.md` Sec.7 and tested via `EV-AUDIO-TRACE` per the family tolerance rows in `10-performance-latency-budgets.md`. The staleness bound between an audio hop and its visible frame is `~19.3 ms` at 60 FPS (`06-audio-engine.md` Sec.7.11).

### 2.1 Data textures
| Symbol | GLSL type | Channels | Profile | Semantics |
|---|---|---|---|---|
| `chroma_Spectrum` | `sampler1D` (or emulated 2D with height 1) | `r`=Raw FFT, `g`=Juiced FFT, `b`=Smooth FFT, `a`=Waveform | C | `Juiced FFT` is the default authoring channel: bin smoothing + temporal smoothing + logarithmic frequency scaling + high-end boost. `Smooth FFT` is a lower-resolution variant with the same smoothing/scaling. Waveform refreshes every frame. |
| `chroma_LevelTrail` | `sampler1D` | `r`=Whole, `g`=Bass, `b`=Mid, `a`=High | C | Historical per-band levels over time, newest on one edge (convention: newest at `u=0`). Used for trail effects. |

**Reference-engine per-band spellings (import-side only, non-parity surface).** The reference `SynesthesiaEngine.framework` binary also exposes per-band time-series samplers as separate names (`syn_TimeSeries`, `syn_BassTimeSeries`, `syn_MidTimeSeries`, `syn_HighTimeSeries`) and per-channel spectrum samplers (`syn_SpectrumSmooth`, `syn_SpectrumJuiced`, `syn_Waveform`). These reference spellings have no entry in `SYN_TO_NATIVE` and are **not** published under `compat-syn-v1` or `native-chroma-v1`; authors must read the packed RGBA channels of `chroma_LevelTrail` and `chroma_Spectrum`. Imported scenes referencing the per-band or per-channel spellings are strict-rejected and compat-coerced to the corresponding channel read of the canonical sampler with a `coercion-applied` diagnostic (see `28-implementation-contract-pack.md` CP-01 CR-06 media-sampler alias rule, extended to audio samplers).

### 2.2 Level (normalized `[0,1]`, per-frame, family tolerance `LVL`)
`chroma_Level`, `chroma_BassLevel`, `chroma_MidLevel`, `chroma_MidHighLevel`, `chroma_HighLevel`.

### 2.3 Hits (normalized `[0,1]`, per-frame, family tolerance `HIT`)
`chroma_Hits`, `chroma_BassHits`, `chroma_MidHits`, `chroma_MidHighHits`, `chroma_HighHits`.

### 2.4 Time clocks (non-normalized, per-frame, family tolerance `TIM`)
`chroma_AudioTime`, `chroma_BassTime`, `chroma_MidTime`, `chroma_MidHighTime`, `chroma_HighTime`, `chroma_CurvedTime` (same base as `chroma_AudioTime`, with exponential acceleration on volume rises).

`chroma_AudioTime` is the energy-integrated audio clock. It is distinct from the wall-clock `TIME` (Sec.1.1) and from the wall-clock `chroma_Time` shader-block member (ADR-024); this is why the reference format's whole-band audio clock maps to `chroma_AudioTime` rather than by prefix swap (Sec.9).

### 2.5 Presence (normalized `[0,1]`, per-frame, family tolerance `PRS`)
`chroma_Presence`, `chroma_BassPresence`, `chroma_MidPresence`, `chroma_MidHighPresence`, `chroma_HighPresence`.

### 2.6 Beat (per-frame, family tolerance `BEAT`)
| Symbol | Type | Semantics |
|---|---|---|
| `chroma_OnBeat` | `float` | 1.0 on beat detection, decays rapidly to 0.0. |
| `chroma_ToggleOnBeat` | `float` | Alternates 0.0/1.0 on each detected beat, logistic-smoothed transitions. |
| `chroma_RandomOnBeat` | `float` | New random value in `[0,1]` on each beat, logistic-smoothed between values. |
| `chroma_BeatTime` | `float` | Integer clock incremented on each detected beat. |

### 2.7 BPM (per-frame, family tolerance `BPM`)
| Symbol | Type | Semantics |
|---|---|---|
| `chroma_BPM` | `float` | Estimated BPM, clamped `[50, 220]`. In the live native shader uniform block the member `chroma_BPM` is a packed `vec4`: `x` = BPM, `y` = beat time, `z` = on-beat, `w` = BPM sine. |
| `chroma_BPMConfidence` | `float` | `[0,1]`; near 1 when BPM is stable. |
| `chroma_BPMTwitcher` | `float` | Exponential "jump" clock; `floor` = beat count; `fract` = per-beat twitch. |
| `chroma_BPMSin` | `float` | Sine `[0,1]` at BPM frequency, beat-phase-locked. |
| `chroma_BPMSin2` | `float` | Sine `[0,1]` at BPM/2. |
| `chroma_BPMSin4` | `float` | Sine `[0,1]` at BPM/4. |
| `chroma_BPMTri` | `float` | Triangle `[0,1]` at BPM. |
| `chroma_BPMTri2` | `float` | Triangle `[0,1]` at BPM/2. |
| `chroma_BPMTri4` | `float` | Triangle `[0,1]` at BPM/4. |

### 2.8 Large features (per-frame, family tolerance `LRG`)
| Symbol | Type | Semantics |
|---|---|---|
| `chroma_FadeInOut` | `float` | Slowly rises to 1.0 on music start, falls to 0.0 on silence. |
| `chroma_Intensity` | `float` | Slow accumulator to 1.0 tracking overall song intensity. |

### 2.9 Heuristic differentiator: `chroma_PrimeBang` (Profile D)
The reference contains bundled scenes using a prime-bang-style name (observed in `boreal.synScene/script.js`). Chromaticity's only native symbol is `chroma_PrimeBang`; imported scenes using the reference spelling are translated to it by `SYN_TO_NATIVE` (Sec.9). This signal is a **Profile D (Differentiation) heuristic**, not a behavior-cloned parity symbol. Scenes relying on it will receive a Chromaticity-original pulse on large-scale musical transitions; scenes that want bit-exact reference parity should avoid it.
- Formula and acceptance criterion are locked in `06-audio-engine.md` Sec.7.14.
- Fixtures live in `34-golden-corpus-definition.md` under the Profile-D heuristic category; they assert trigger behavior under seeded input, not numeric parity.

### 2.10 Reference-surface uniforms not in the documented catalog
These symbols appear in `SynesthesiaEngine.framework` and/or bundled reference scenes but are absent from `ssf/audio_uniforms.html`. They are shipped-behaviour and must have a defined Chromaticity disposition so import and parity fixtures are deterministic. The Symbol column gives the native name; the reference spelling each one is imported from is listed in Sec.9.

| Symbol | Type | Profile | Evidence | Chromaticity behaviour |
|---|---|---|---|---|
| `chroma_InputLevel` | `float` | C | `audio_indicators.synScene` (GLSL + JS) | Pre-AGC instantaneous input-level, range `[0, inf)`. Published as `clamp(RMS(currentAudioHop) / AGC_targetRMS, 0, inf)`. Authors who want clamped/normalized behavior use `chroma_Level`. Formula locked in `06-audio-engine.md` Sec.7.12. |
| `chroma_RawLevel` | `float` | C | `ssf/audio_uniforms.html` (documented, not observed in release scenes) | Same value as `chroma_InputLevel` with identical semantics. Both native names exist because the reference docs and the reference scenes use different spellings for this value, and each is imported onto its own native name (Sec.9). |
| `chroma_BassExpoCurve` | `float` | C | 1 release scene (GLSL) | `pow(chroma_BassLevel, 3.0)`, i.e. exponential curve on bass level for non-linear reactivity. Formula locked in `06-audio-engine.md` Sec.7.12. Forward-compat: any further members of the family are native `chroma_*ExpoCurve` names. |
| `chroma_Threshold` | `float` | C | `SynesthesiaEngine.framework` symbol, no release-scene usage | Operator-settable noise-floor gate in `[0,1]`; audio hops with RMS below this value publish zeroed `chroma_Level*` and `chroma_*Hits` families. Exposed via settings panel and OSC route `/audio/threshold {float}`. Formula locked in `06-audio-engine.md` Sec.7.12. |
| `syn_PrimeAndBang` (reference spelling; no native name) | `float` | C | `boreal.synScene/script.js` (commented line) | Import-side only. A deprecated reference-format variant of the prime-bang name; it has no entry in `SYN_TO_NATIVE` and no native catalog name, and is never accepted by native parsers. Native scenes use `chroma_PrimeBang`. Import-time handling is governed by `28-implementation-contract-pack.md` CP-07. |

Rationale for promoting these to Profile C: `chroma_InputLevel` backs a value read by a shipped reference scene, so an import that cannot map it breaks out-of-the-box. `chroma_RawLevel` backs the reference's documented name for the same value; mapping only one of the two reference spellings leaves authors who followed the reference docs stranded. `chroma_BassExpoCurve` likewise breaks exactly one shipped scene if omitted but the formula is trivial.

### 2.11 Native audio analysis primitives (`chroma_*`; slice-5a-introduced)

These three uniforms are net-new chromaticity-native primitives introduced by the slice-5a `engine/render` ↔ `engine/audio` cross-crate adapter (`engine/render/src/audio_adapter.rs`). They expose raw analysis values from the `AudioUniformSnapshot` producer, complementing the normalised / smoothed `chroma_Level*` (Sec.2.2) and `chroma_Spectrum` (Sec.2.1) parity surfaces with lower-level inputs that scenes can post-process to taste. The production runtime sources RMS, peak, FFT, and BPM readiness from the same rolling analysis window so callbacks with bursty hop-level energy do not publish mismatched scalar-vs-spectrum snapshots. They are a chromaticity-first surface — the catalog has no native equivalents at this abstraction level — so the namespace governance rule (Sec.Namespace governance) requires the `chroma_*` prefix for the new symbols.

| Symbol | GLSL type | Profile | Cadence | Semantics |
|---|---|---|---|---|
| `chroma_AudioRMS` | `float` | D | per-frame | Root-mean-square energy over the producer's mono analysis window, sourced from `chromaticity_audio::extract_uniform_snapshot`. One-shot callers may pass a single callback buffer, but the production runtime passes its rolling FFT/BPM analysis window. Non-normalised (range `[0, inf)`); for normalised band-level reactivity use the `chroma_Level*` family. |
| `chroma_AudioPeak` | `float` | D | per-frame | Peak absolute magnitude over the same mono analysis window as `chroma_AudioRMS`. Non-normalised; pairs with `chroma_AudioRMS` for envelope-shaped reactivity. |
| `chroma_AudioFFT[8]` | `float[8]` | D | per-frame | Log-spaced FFT bands `[0..=7]` from `AudioUniformSnapshot::fft_bands`. Element `[0]` is the lowest band (low-edge `chromaticity_audio::FFT_BAND_MIN_HZ`); element `[7]` is the highest band. Authors index the array directly (`chroma_AudioFFT[K]`). Band-edge contract is the producer-side FFT analyser (`06-audio-engine.md` Sec.7.4). |

Notes:

- `chroma_BPM` (Sec.2.7) is the fourth uniform the slice-5a adapter binds (`engine/render/src/audio_adapter.rs`, as a scalar). The adapter marshals it from `AudioUniformSnapshot::bpm` (the slice-2 history-window estimator).
- RMS and peak are analysis-window values in the production runtime. The UI's existing EMA / hysteresis layer remains downstream and unchanged.
- **`chroma_AudioFFT[8]` is the consolidated v1.3.0 shape.** The slice-5a initial release (catalog v1.2.0) used a transitional split-vec4 spelling (`chroma_AudioFFTLow` for bands `[0..=3]` + `chroma_AudioFFTHigh` for bands `[4..=7]`) because `engine/render/src/multipass.rs::UniformValue` had no array variant. The slice-5 follow-up landed `UniformValue::FloatArray` and consolidated the FFT into a single 8-element array binding. The two scalar primitives (`chroma_AudioRMS`, `chroma_AudioPeak`) and the BPM uniform (now `chroma_BPM`) are unchanged across the consolidation; only the FFT shape moved. The std140 packing of `float arr[N]` puts each element on a 16-byte (vec4) boundary; the per-pass uniform buffer grows by 8 slots for each `FloatArray(8)` binding. Authors who previously wrote `chroma_AudioFFTLow.x` migrate to `chroma_AudioFFT[0]`; `chroma_AudioFFTHigh.w` migrates to `chroma_AudioFFT[7]`.
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

### 3.2 Media meta (applied during media ingest pass, before scene reads `chroma_Media`)
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
| `syn_Prefs_ZoomOrFit` (reference spelling; no native name assigned) | `float` | 1.0 | When `>0.5`, `_zoomOrFit(wr, hr)` returns `max(wr, hr)` (zoom-to-fill); else `min(wr, hr)` (letterbox fit). Governs media/output fit behavior. Import-side only: this reference preference has no entry in `SYN_TO_NATIVE` and no native catalog name yet; native parsers do not accept it. |

### 3.6 Differentiation (edition-gated; `Reactivity`, `Audio Speed`)
| Symbol | Type | Default | Profile | Semantics |
|---|---|---|---|---|
| `_Reactivity` | `float` | 1.0 | A (Advanced edition) | Global multiplier on all audio uniforms before shader read. Matches reference "Reactivity" Pro meta. |
| `_Audio_Speed` | `float` | 1.0 | A (Advanced edition) | Multiplier on the time clocks (`chroma_AudioTime`, `chroma_*Time`, `chroma_CurvedTime`). |

### 3.7 Meta-pipeline ordering (normative)
1. Media ingest -> `_Media_Contrast` -> `_Media_Gamma` -> `_Media_Hue/Saturation` -> `_Invert_Media` -> write `chroma_Media`.
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
- Samplers: `chroma_FinalPass`, `chroma_Media`, `chroma_Spectrum`, `chroma_LevelTrail` are not exposed as direct values; use `textureFinalPass(x, y)` and a future `textureMedia(x, y)` helper (tracked in CP-07 addendum).

Vector values are JS objects: `vec2 = {x, y}`, `vec3 = {x, y, z}` or `{r, g, b}`, `vec4 = {x, y, z, w}` or `{r, g, b, a}`.

**Array-shaped uniforms are producer-only on the script-write side.** Symbols whose underlying GLSL type is `float[N]` (or any future `vec2[N]` / `vec4[N]` batched array) — currently `chroma_AudioFFT[8]` from §2.11, and reserved future vec2-batched array uniforms — are exposed to the script host as JS arrays for read access via the producer pipeline, but cannot be the target of `__chromaticityEmit(name, value)` reactive writes from script. The `engine/script` carrier vocabulary (`ReactiveValueKind::{Scalar, Vec2, Vec3, Vec4}`) does not yet support 8-element float-array writes; see the inline retirement note in `engine/script/src/reactive_symbol.rs` for the canonical reasoning. Catalog v1.3.0 retired the v1.2.0 transitional split-vec4 spelling (`chroma_AudioFFTLow` / `chroma_AudioFFTHigh`) on the script-write side alongside the engine-side consolidation; an array-carrier `ReactiveValueKind` variant is future-slice scope and will reopen the script-write surface for `chroma_AudioFFT[8]` when it lands.

Controls appear as globals named exactly as their `NAME`. For a scene with `NAME: "rate"`, the script global `rate` is the current value. The deprecated `inputs[name]` and `uniforms[name]` shims mirror these under `compat-syn-v1` only.

### 5.1 Known limitations (script-write surface)

- **`chroma_AudioFFT[8]` is producer-only on the script-write side.** The audio→render path is wired end-to-end (`engine/audio::AudioUniformSnapshot::fft_bands` → `engine/render::audio_adapter::uniform_input_from_audio_snapshot` → `UniformValue::FloatArray(8)` GPU binding under catalog v1.3.0), but the script→runtime path does not yet support 8-element float-array reactive writes: `engine/script::ReactiveValueKind::{Scalar, Vec2, Vec3, Vec4}` carries no array variant, so `"chroma_AudioFFT"` resolves to `None` from `ReactiveSymbol::from_name` and surfaces to script as `ScriptError::ReactiveSymbolUnknown` if a scene attempts a `__chromaticityEmit("chroma_AudioFFT", …)` write. Workaround for scripted scenes that need to synthesize / mock the FFT surface today: write the producer-side `AudioUniformSnapshot` directly (test-harness path), or fall back to the per-band scalar primitives `chroma_AudioRMS` / `chroma_AudioPeak` for coarse level-driven reactivity. Closing the asymmetry requires a `ReactiveValueKind::FloatArray<N>` variant plus matching carrier-shape extension on the typed-reactive-write pathway. Tracked as Phase-3+ feature work alongside the deferred axes in `docs/00-phase-0-placeholder-inventory.md` Phase-3+ known limitations and `docs/13-risk-register.md` R-37 Deferred axes; will reopen the script-write surface for `chroma_AudioFFT[8]` when it lands.

## 6) Coercion Classes (Script and GLSL)
Canonical names live in this catalog. Coercion classes `CR-01..CR-04` (see `28-implementation-contract-pack.md`) apply to:
- Case mismatch on control `NAME` -> strict reject, compat warn-and-coerce to canonical case.
- Reading an undeclared audio uniform -> strict compile error, compat compile to `0.0`.
- Writing to a read-only uniform -> strict runtime error, compat warn-and-ignore.
- Calling a helper with wrong arity -> strict compile error; compat attempts the reference-equivalent overload when unambiguous.
- Namespace: there is no native namespace coercion. Reference-format names are translated only by the import adapters via `SYN_TO_NATIVE` (Sec.9); a native scene that names a non-`chroma_*` reactive symbol fails validation.

## 7) Drift Tolerance Cross-Reference
Families in Sec.2 map to the tolerance rows in `10-performance-latency-budgets.md`:
- `LVL` - level family
- `HIT` - hit family
- `TIM` - time family (including `chroma_AudioTime` and `chroma_CurvedTime`)
- `PRS` - presence family
- `BEAT` - `chroma_OnBeat`, `chroma_ToggleOnBeat`, `chroma_RandomOnBeat`, `chroma_BeatTime`
- `BPM` - BPM family (all `chroma_BPM*`)
- `LRG` - `chroma_FadeInOut`, `chroma_Intensity`

Meta-control visual drift is tested by the meta-pipeline fixture set (see `34-golden-corpus-definition.md`).

## 8) Change Control
- Any addition to this catalog requires: (a) a bumped catalog version; (b) a new `EV-*` fixture; (c) an entry in `22-compatibility-versioning-policy.md` release notes.
- Any removal or rename of a canonical name is a breaking change and requires a major catalog version bump and a major profile bump.
- New symbols are `chroma_*` only (ADR-024). A newly supported reference-format name is added as a row in `SYN_TO_NATIVE` (and in Sec.9) mapping it to a native `chroma_*` name; it is never added to the native catalog.
- Catalog version at ratification: `1.0.0` (targets Chromaticity `compat-syn-v1` and `native-chroma-v1` baseline).
- Catalog version `1.1.0`: added Sec.2.10 reference-surface uniforms (`syn_InputLevel`, `syn_RawLevel`, `syn_BassExpoCurve`, `syn_Threshold`, `syn_PrimeAndBang`) and Sec.2.1 non-parity per-band time-series/spectrum spelling disposition. Formulas in `06-audio-engine.md` Sec.7.12.
- Catalog version `1.2.0`: added Sec.2.11 native audio analysis primitives (`chroma_AudioRMS`, `chroma_AudioPeak`, `chroma_AudioFFTLow`, `chroma_AudioFFTHigh`) introduced by the slice-5a `engine/render` ↔ `engine/audio` cross-crate adapter. Sec.8 (b) `EV-*` fixture coverage is satisfied by slice-5a's pre-existing T1 drift pins (`engine/render/src/audio_adapter.rs::audio_adapter_drift_pins`) and T2 GPU-roundtrip corpus (`engine/render/tests/audio_adapter.rs`) under `EV-RENDER-CONFORMANCE` and `EV-AUDIO-TRACE`; release-notes entry appended to `docs/22-compatibility-versioning-policy.md`. The split-`vec4` FFT spelling (`chroma_AudioFFTLow` + `chroma_AudioFFTHigh`) was documented as transitional pending an array variant on `engine/render/src/multipass.rs::UniformValue`; that consolidation landed in v1.3.0.
- Catalog version `1.3.0`: consolidated the v1.2.0 split-vec4 FFT spelling (`chroma_AudioFFTLow` + `chroma_AudioFFTHigh`) into a single `chroma_AudioFFT[8]` array binding. Foundational widening: added `UniformValue::FloatArray` to `engine/render/src/multipass.rs` with std140-friendly packing (each element vec4-aligned to 16 bytes, matching `uniform float arr[N]`), and rebuilt `engine/render/src/audio_adapter.rs::uniform_input_from_audio_snapshot` to bind `chroma_AudioFFT` once via `UniformValue::FloatArray(snapshot.fft_bands.to_vec())`. The two scalar primitives (`chroma_AudioRMS`, `chroma_AudioPeak`) and `syn_BPM` are unchanged across the consolidation. Sec.8 (b) `EV-*` fixture coverage: existing slice-5a T1 drift pins are updated in place to assert the new shape (`adapter_returns_four_uniforms_in_documented_order`, `adapter_packs_fft_bands_into_float_array_in_band_order`); a new `multipass::multipass_drift_pins::uniform_marshalling_float_array_layout` pin covers the std140 packing invariant; the T2 GPU-roundtrip corpus's sine-tone ACCEPTED hash is rolled forward (the silent-frame hash is preserved verbatim because zero FFT bands sum identically under any summation order). Release-notes entry appended to `docs/22-compatibility-versioning-policy.md`. Authoring-surface migration: `chroma_AudioFFTLow.x` → `chroma_AudioFFT[0]`, ..., `chroma_AudioFFTHigh.w` → `chroma_AudioFFT[7]`. No scenes shipped against the v1.2.0 split-vec4 surface, so no compat aliases are required.
- Catalog version `2.0.0` (2026-11-28, `adr/ADR-024-native-chroma-vocabulary.md`): **breaking rename.** Every canonical reactive name formerly spelled with the reference-format prefix is renamed to its native `chroma_*` name (prefix swap, except the whole-band audio clock, now `chroma_AudioTime`, and the two reference media samplers, both now `chroma_Media`). The native compatibility-alias tier is removed: native parsers accept `chroma_*` only, the former reference-spelled `PrimeBang` alias variant is removed (only `chroma_PrimeBang` remains), and the `chroma_Data_*` alias spellings in Sec.1.6 are withdrawn. Reference-format names now appear in this catalog only in Sec.9 and in explicitly import-side notes. The native shader uniform-block BPM member is `chroma_BPM`. Namespace governance and Sec.6 and Sec.8 rules are rewritten to match ADR-024. Sec.8 (b) `EV-*` fixture coverage: `EV-SYMBOL-CATALOG` is pinned by `engine/script/tests/reactive_symbol.rs` (`reference_format_names_are_not_native_symbols`); the import translation is pinned under `EV-SCENE-IMPORT`. Release-notes entry appended to `22-compatibility-versioning-policy.md`. `Profile-Impact: native-chroma-v1-major`.

## 9) Import name mapping (`.synScene`)

The table below reproduces `SYN_TO_NATIVE` in `engine/authoring/src/syn_import.rs`, the single frozen translation table from reference-format reactive names to native names (`adr/ADR-024-native-chroma-vocabulary.md`). The reference names in the left column are accepted **only** by the import adapters (`engine/authoring/src/syn_import.rs` for package conversion, `app/shell/src/syn_compat.rs` for the live shader bridge and load-time `script.js` adaptation). They are never accepted by native parsers. If this table and the code disagree, the code wins.

The mapping is a prefix swap except `syn_Time -> chroma_AudioTime` (energy-integrated audio clock; see Sec.2.4) and `syn_UserImage` / `syn_Media -> chroma_Media`. A few native targets (`chroma_BPMSin8`, `chroma_BPMTri8`, `chroma_MediaActive`) are import targets without a catalog row in Sec.1-3 at this version.

| Reference name (import only) | Native name |
|---|---|
| `syn_Level` | `chroma_Level` |
| `syn_BassLevel` | `chroma_BassLevel` |
| `syn_MidLevel` | `chroma_MidLevel` |
| `syn_MidHighLevel` | `chroma_MidHighLevel` |
| `syn_HighLevel` | `chroma_HighLevel` |
| `syn_Hits` | `chroma_Hits` |
| `syn_BassHits` | `chroma_BassHits` |
| `syn_MidHits` | `chroma_MidHits` |
| `syn_MidHighHits` | `chroma_MidHighHits` |
| `syn_HighHits` | `chroma_HighHits` |
| `syn_Time` | `chroma_AudioTime` |
| `syn_BassTime` | `chroma_BassTime` |
| `syn_MidTime` | `chroma_MidTime` |
| `syn_MidHighTime` | `chroma_MidHighTime` |
| `syn_HighTime` | `chroma_HighTime` |
| `syn_CurvedTime` | `chroma_CurvedTime` |
| `syn_Presence` | `chroma_Presence` |
| `syn_BassPresence` | `chroma_BassPresence` |
| `syn_MidPresence` | `chroma_MidPresence` |
| `syn_MidHighPresence` | `chroma_MidHighPresence` |
| `syn_HighPresence` | `chroma_HighPresence` |
| `syn_OnBeat` | `chroma_OnBeat` |
| `syn_ToggleOnBeat` | `chroma_ToggleOnBeat` |
| `syn_RandomOnBeat` | `chroma_RandomOnBeat` |
| `syn_BeatTime` | `chroma_BeatTime` |
| `syn_BPM` | `chroma_BPM` |
| `syn_BPMConfidence` | `chroma_BPMConfidence` |
| `syn_BPMTwitcher` | `chroma_BPMTwitcher` |
| `syn_BPMSin` | `chroma_BPMSin` |
| `syn_BPMSin2` | `chroma_BPMSin2` |
| `syn_BPMSin4` | `chroma_BPMSin4` |
| `syn_BPMSin8` | `chroma_BPMSin8` |
| `syn_BPMTri` | `chroma_BPMTri` |
| `syn_BPMTri2` | `chroma_BPMTri2` |
| `syn_BPMTri4` | `chroma_BPMTri4` |
| `syn_BPMTri8` | `chroma_BPMTri8` |
| `syn_FadeInOut` | `chroma_FadeInOut` |
| `syn_Intensity` | `chroma_Intensity` |
| `syn_PrimeBang` | `chroma_PrimeBang` |
| `syn_InputLevel` | `chroma_InputLevel` |
| `syn_RawLevel` | `chroma_RawLevel` |
| `syn_BassExpoCurve` | `chroma_BassExpoCurve` |
| `syn_Threshold` | `chroma_Threshold` |
| `syn_FinalPass` | `chroma_FinalPass` |
| `syn_UserImage` | `chroma_Media` |
| `syn_Media` | `chroma_Media` |
| `syn_MediaActive` | `chroma_MediaActive` |
| `syn_MediaType` | `chroma_MediaType` |
| `syn_Spectrum` | `chroma_Spectrum` |
| `syn_LevelTrail` | `chroma_LevelTrail` |
