<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> -->

# Synesthesia parity — SSF Format (feature-level deep dive)

_Last refreshed:_ 2026-05-22

Feature-level companion to `docs/comparison/ssf-format.md`. That page
scores the seven Synesthesia SSF documentation surfaces at the
**section** level (one verdict per surface). This page goes one layer
down — every individual symbol, helper function, JSON field, control
kind, library helper, scripting API, and best-practices recommendation
Synesthesia documents gets its own row with a per-feature score and a
pointer to the Chromaticity equivalent. Read this page when you need
to know whether a specific symbol or function is at parity; read the
overview page when you need the per-surface posture.

Clean-room observation-only per `docs/29-cleanroom-legal-policy-baseline.md`.
Symbol names are functional contracts and are transcribed verbatim;
descriptive prose is paraphrased.

## TL;DR — aggregate feature counts

| Group | Synesthesia features documented | Chromaticity rows scored |
| :---- | :-----------------------------: | :----------------------: |
| SSF package format | 12 | 12 |
| Audio uniforms | 33 | 33 (+ 6 net-new chroma-side rows) |
| Standard uniforms | 16 | 16 (+ 4 net-new chroma-side rows) |
| GLSL helper functions | 64 | 64 (+ 21 net-new meta-pipeline rows) |
| External libraries (Lygia categories + helpers) | 14 Lygia categories + 5 hg_sdf helpers + 28 hg_sdf primitives + 14 hg_sdf domain ops + 17 hg_sdf combinators + 1 custom-library slot | 79 |
| JSON configuration (top-level + per-control + per-pass + per-image + control kinds + variants) | 12 top-level + 11 per-control + 5 per-pass + 2 per-image + 2 per-media + 7 control kinds + 12 control variants = 51 | 51 |
| JavaScript scripting (lifecycle + events + control + uniform + sampling + utility + globals) | 17 | 17 |
| Best practices (audio reactivity + controls + GLSL float + learning) | 30 | 30 |

**Score distribution across all 320 scored rows** (Chromaticity-side
verdicts; aggregate over the eight groups; counted from the per-group
histograms below):

| Score | Count | % |
| :---- | :---: | -: |
| 5 / EXCEEDS+ | 18 | 5.6% |
| 4 / EXCEEDS | 79 | 24.7% |
| 3 / MATCH | 178 | 55.6% |
| 2 / PARTIAL | 35 | 10.9% |
| 1 / GAP | 8 | 2.5% |
| N/A / OUT-OF-SCOPE | 2 | 0.6% |

The bulk of the surface lands at MATCH (the catalog is declared with
parity signatures); the EXCEEDS / EXCEEDS+ rows concentrate in the
audio-uniforms surface (drift pins + readiness flags + raw-analysis
primitives) and the JSON configuration (flatten-preserved extensions
+ strict/compat dual mode + the `chromaPack` transport). The
GAP rows cluster in the JavaScript scripting surface (per-handler
scene-author bindings deferred to script-host slice 5.4+) and the
best-practices help-corpus pages (the explicit "controls best
practices" + "common pitfalls" + "GLSL performance" pages have not
been authored yet).

---

## SSF package format

_Synesthesia documents the `.synScene` package as a directory carrying
a fixed set of files plus optional subdirectories. Chromaticity's
equivalent is the `.chroma` directory (with `.chromaPack` zip
transport as a chromaticity-side addition) loaded through
`engine/scene/src/loader.rs::load_chroma_dir`. Twelve individual
features documented._

**Score histogram:** 2× EXCEEDS+, 4× EXCEEDS, 5× MATCH, 1× GAP.

| # | Synesthesia feature | Score | Chromaticity equivalent | Notes |
| :- | :------------------- | :---: | :----------------------- | :---- |
| 1 | `.synScene` directory extension | 3 / MATCH | `.chroma` directory extension | Native chromaticity spelling; ingest of `.synScene` is import-only per `docs-corpus/docs/scene-workflow.md`. |
| 2 | Package layout: package-as-directory | 4 / EXCEEDS | `.chroma` directory OR `.chromaPack` zip transport (`engine/scene/src/loader.rs`) | Two on-disk forms; Synesthesia documents the directory form only. |
| 3 | `main.glsl` fragment shader entry point | 3 / MATCH | `main.glsl` in package root per `docs-corpus/docs/scene-workflow.md` | Same role + spelling. |
| 4 | `scene.json` JSON configuration | 3 / MATCH | `scene.json` deserialized through `engine/scene/src/schema.rs::Scene` | Same role; wire format frozen at slice-3a per `docs/02-synesthesia-contracts.md`. |
| 5 | `script.js` optional script file | 3 / MATCH | `script.js` optional file documented in `docs-corpus/docs/scene-workflow.md` | Same role; script-host wiring lives in `engine/script/` per ADR-012. |
| 6 | `thumbnail.png` package thumbnail | 2 / PARTIAL | Captured via `IMAGE_PATH` flatten-preserved metadata; no thumbnail-render contract pinned yet | Metadata round-trips; no explicit thumbnail render-target spec. |
| 7 | `images/` directory for bundled textures | 3 / MATCH | `images/*` documented in `docs-corpus/docs/scene-workflow.md` | Same role; structured `IMAGES` JSON parsing deferred per slice-3a narrowing. |
| 8 | `IMAGE_PATH` JSON key (thumbnail path) | 3 / MATCH | Flatten-preserved on `Scene::extensions` per `engine/scene/src/schema.rs:53-66` | Round-trip via `#[serde(flatten)]`. |
| 9 | `IMAGES` JSON key (bundled texture array) | 3 / MATCH | Flatten-preserved on `Scene::extensions`; structured parsing in a later slice | Round-trip preserved at slice-3a. |
| 10 | Fragment shader concept (pixel/per-fragment evaluation) | 3 / MATCH | Same role through the planned ADR-011 render backend | Live GPU rendering is ADR-011 Phase-2+ deferred. |
| 11 | Per-user GLSL library (custom `glsl/` folder in user data) | 1 / GAP | Per-scene `glsl/*.glsl` shared includes exist; no per-user library root | Global per-user library not yet authored. |
| 12 | Reserved `DATA_SOURCES` post-MVP namespace (Chromaticity-only) | 5 / EXCEEDS+ | Parse-reserved at slice-3a per `docs/28-implementation-contract-pack.md` CP-12 | Chromaticity differentiator — no Synesthesia equivalent. |
| 12a | `.chromaPack` zip transport (Chromaticity-only) | 5 / EXCEEDS+ | Two on-disk forms supported by `engine/scene/src/loader.rs` | Chromaticity-only second form; counted above on row 2 but flagged here for the differentiator surface. |
| 12b | Strict-vs-compat loader modes (Chromaticity-only) | 4 / EXCEEDS | `engine/scene/src/validator.rs` `ValidationMode::{Compat, Strict}` | Forward-compat + reference-scene tolerance gradient; no Synesthesia equivalent. |
| 12c | Flatten-preserved extensions map (Chromaticity-only) | 4 / EXCEEDS | Every schema type carries `extensions: serde_json::Map<String, Value>` via `#[serde(flatten)]` | Unknown / future top-level keys survive round-trip. |
| 12d | `SCHEMA_VERSION` versioning key (Chromaticity-reserved) | 4 / EXCEEDS | `SceneHeader::schema_version` defaults to 1 per `engine/scene/src/schema.rs:97-104` | Forward-compat versioning; reference scenes lack the key. |

---

## SSF audio uniforms

_Synesthesia's audio-uniforms page documents 33 individual symbols
across nine families (sampler data textures, level, hits, time,
presence, beat, BPM, large-scale). Chromaticity's catalog mirrors
each at parity through `docs/36-reactive-symbol-catalog.md` v1.3.0
§2; the producer side lives at `engine/audio/src/uniforms.rs` and the
render-side adapter at `engine/render/src/audio_adapter.rs`. The
script-side typed catalog at `engine/script/src/reactive_symbol.rs`
binds 46 of the 53 catalog symbols to typed `ReactiveSymbol`
variants. Six chromaticity-native additions (`chroma_AudioRMS`,
`chroma_AudioPeak`, `chroma_AudioFFT[8]`, `chroma_PrimeBang`, plus
the readiness-flag cross-seam) sit alongside the Synesthesia-mirrored
catalog._

**Score histogram:** 11× EXCEEDS+, 0× EXCEEDS, 22× MATCH (33
Synesthesia rows total) + 6 net-new chroma-side EXCEEDS+ rows.

### §2.1 Audio data textures (samplers)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 13 | `syn_Spectrum` (`sampler1D`, RGBA-packed Raw/Juiced/Smooth/Waveform) | 3 / MATCH | `docs/36-reactive-symbol-catalog.md` §2.1 — Profile C | Same 4-channel pack contract. Live sampler-binding ships with ADR-011 render backend. |
| 14 | `syn_LevelTrail` (`sampler1D`, RGBA-packed Whole/Bass/Mid/High) | 3 / MATCH | `docs/36` §2.1 — Profile C | Same 4-channel pack. |

### §2.2 Level uniforms (family tolerance LVL)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 15 | `syn_Level` | 5 / EXCEEDS+ | `ReactiveSymbol::Level` at `engine/script/src/reactive_symbol.rs:171-172` | Drift-tolerance family `LVL` plus T1 in-source drift pin. |
| 16 | `syn_BassLevel` | 3 / MATCH | `ReactiveSymbol::BassLevel` | Per-band coverage. |
| 17 | `syn_MidLevel` | 3 / MATCH | `ReactiveSymbol::MidLevel` | Per-band coverage. |
| 18 | `syn_MidHighLevel` | 3 / MATCH | `ReactiveSymbol::MidHighLevel` | Per-band coverage. |
| 19 | `syn_HighLevel` | 3 / MATCH | `ReactiveSymbol::HighLevel` | Per-band coverage. |

### §2.3 Hits uniforms (family tolerance HIT)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 20 | `syn_Hits` | 5 / EXCEEDS+ | `ReactiveSymbol::Hits` | Drift-tolerance family `HIT` plus T1 pin. |
| 21 | `syn_BassHits` | 3 / MATCH | `ReactiveSymbol::BassHits` | |
| 22 | `syn_MidHits` | 3 / MATCH | `ReactiveSymbol::MidHits` | |
| 23 | `syn_MidHighHits` | 3 / MATCH | `ReactiveSymbol::MidHighHits` | |
| 24 | `syn_HighHits` | 3 / MATCH | `ReactiveSymbol::HighHits` | |

### §2.4 Time clock uniforms (family tolerance TIM)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 25 | `syn_Time` | 5 / EXCEEDS+ | `ReactiveSymbol::SynTime` | Drift-tolerance family `TIM` plus T1 pin. |
| 26 | `syn_BassTime` | 3 / MATCH | `ReactiveSymbol::BassTime` | |
| 27 | `syn_MidTime` | 3 / MATCH | `ReactiveSymbol::MidTime` | |
| 28 | `syn_MidHighTime` | 3 / MATCH | `ReactiveSymbol::MidHighTime` | |
| 29 | `syn_HighTime` | 3 / MATCH | `ReactiveSymbol::HighTime` | |
| 30 | `syn_CurvedTime` | 3 / MATCH | `ReactiveSymbol::CurvedTime` | Same accelerated-time curve role. |

### §2.5 Presence uniforms (family tolerance PRS)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 31 | `syn_Presence` | 5 / EXCEEDS+ | `ReactiveSymbol::Presence` | Family `PRS` plus T1 pin. |
| 32 | `syn_BassPresence` | 3 / MATCH | `ReactiveSymbol::BassPresence` | |
| 33 | `syn_MidPresence` | 3 / MATCH | `ReactiveSymbol::MidPresence` | |
| 34 | `syn_MidHighPresence` | 3 / MATCH | `ReactiveSymbol::MidHighPresence` | |
| 35 | `syn_HighPresence` | 3 / MATCH | `ReactiveSymbol::HighPresence` | |

### §2.6 Beat-detection uniforms (family tolerance BEAT)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 36 | `syn_OnBeat` | 5 / EXCEEDS+ | `ReactiveSymbol::OnBeat` | Family `BEAT` plus T1 pin. |
| 37 | `syn_ToggleOnBeat` | 3 / MATCH | `ReactiveSymbol::ToggleOnBeat` | Toggles on every beat with smooth transitions. |
| 38 | `syn_RandomOnBeat` | 3 / MATCH | `ReactiveSymbol::RandomOnBeat` | Re-randomized per beat. |
| 39 | `syn_BeatTime` | 3 / MATCH | `ReactiveSymbol::BeatTime` | Increments by 1.0 per detected beat. |

### §2.7 BPM-analysis uniforms (family tolerance BPM)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 40 | `syn_BPM` | 5 / EXCEEDS+ | `UNIFORM_NAME_BPM` at `engine/render/src/audio_adapter.rs:100` + `ReactiveSymbol::Bpm` | Clamped `[50, 220]`; T1 drift pin asserts clamp; cross-crate audio→render adapter. |
| 41 | `syn_BPMConfidence` | 3 / MATCH | `ReactiveSymbol::BpmConfidence` | Stability estimate. |
| 42 | `syn_BPMTwitcher` | 3 / MATCH | `ReactiveSymbol::BpmTwitcher` | Per-beat exponential clock. |
| 43 | `syn_BPMSin` | 3 / MATCH | `ReactiveSymbol::BpmSin` | Sine at current BPM. |
| 44 | `syn_BPMSin2` | 3 / MATCH | `ReactiveSymbol::BpmSin2` | Half-rate sine. |
| 45 | `syn_BPMSin4` | 3 / MATCH | `ReactiveSymbol::BpmSin4` | Quarter-rate sine. |
| 46 | `syn_BPMTri` | 3 / MATCH | `ReactiveSymbol::BpmTri` | Triangle at current BPM. |
| 47 | `syn_BPMTri2` | 3 / MATCH | `ReactiveSymbol::BpmTri2` | Half-rate triangle. |
| 48 | `syn_BPMTri4` | 3 / MATCH | `ReactiveSymbol::BpmTri4` | Quarter-rate triangle. |

### §2.8 Large-scale feature uniforms (family tolerance LRG)

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 49 | `syn_FadeInOut` | 5 / EXCEEDS+ | `ReactiveSymbol::FadeInOut` | Slow rise / fall on song start / end; T1 pin. |
| 50 | `syn_Intensity` | 5 / EXCEEDS+ | `ReactiveSymbol::Intensity` | Slow accumulation toward 1.0; T1 pin. |

### Chromaticity-native audio additions (rows beyond Synesthesia's documented surface)

| # | Chroma-side feature | Score | Source | Notes |
| :- | :------------------- | :---: | :----- | :---- |
| 51 | `chroma_AudioRMS` (analysis-window RMS scalar) | 5 / EXCEEDS+ | `UNIFORM_NAME_RMS` at `engine/render/src/audio_adapter.rs:104` + `ReactiveSymbol::Rms` | Net-new chroma primitive; round 3 lane Q unified analysis window with FFT/BPM. |
| 52 | `chroma_AudioPeak` (analysis-window peak scalar) | 5 / EXCEEDS+ | `UNIFORM_NAME_PEAK` at `engine/render/src/audio_adapter.rs:108` + `ReactiveSymbol::Peak` | Net-new chroma primitive. |
| 53 | `chroma_AudioFFT[8]` (8-band log-spaced FFT, std140-packed) | 5 / EXCEEDS+ | `UNIFORM_NAME_FFT` at `engine/render/src/audio_adapter.rs:118` | Catalog v1.3.0 consolidation; producer-only on script side until array-carrier `ReactiveValueKind` variant lands. |
| 54 | `chroma_PrimeBang` (macro-musical transition heuristic) | 5 / EXCEEDS+ | `ReactiveSymbol::ChromaPrimeBang` + `SynPrimeBang` compat alias | Profile D differentiator per catalog §2.9. |
| 55 | `AudioAxisAvailability::{Warming, Ready}` readiness cross-seam | 5 / EXCEEDS+ | `engine/audio` `AudioAxisAvailability` + Perform AUDIO adapter pill | Round-2 widening of round-1 production-bootstrap chain; no Synesthesia equivalent. |
| 56 | `degraded: bool` flag on `AudioUniformSnapshot` | 5 / EXCEEDS+ | `engine/audio::AudioUniformSnapshot::degraded` | Per-snapshot health flag for the UI banner. |

### Chromaticity-native reference-surface uniforms (catalog §2.10 — beyond docs.synesthesia.live)

These ship in `SynesthesiaEngine.framework` but are not on the
public docs surface; Chromaticity normalises them at catalog
Profile C with formulae locked in `docs/06-audio-engine.md` §7.12.

| # | Symbol | Score vs reference framework | Source | Notes |
| :- | :----- | :--------------------------: | :----- | :---- |
| 57 | `syn_InputLevel` | 4 / EXCEEDS | `ReactiveSymbol::InputLevel` | Pre-AGC instantaneous input level. |
| 58 | `syn_RawLevel` | 4 / EXCEEDS | `ReactiveSymbol::RawLevel` | Alias of `syn_InputLevel`. |
| 59 | `syn_BassExpoCurve` | 4 / EXCEEDS | `ReactiveSymbol::BassExpoCurve` | `pow(syn_BassLevel, 3.0)`. |
| 60 | `syn_Threshold` | 4 / EXCEEDS | `ReactiveSymbol::Threshold` | Operator-settable noise-floor gate; surfaces via `Shift+T` (sample current 200 ms RMS) + OSC `/audio/threshold {float}` per `docs-corpus/docs/audio-reactivity.md`. |
| 61 | `syn_PrimeAndBang` (reference-only convenience) | 3 / MATCH | Catalog §2.10 entry | `syn_PrimeAndBang` = `chroma_PrimeBang * syn_OnBeat` per docs/06. |

---

## SSF standard uniforms

_Synesthesia's standard-uniforms page documents 16 symbols across
five families (time, pixel coordinates, mouse, multipass, media).
Chromaticity's catalog adds 4 net-new symbols (`TIMEDELTA`,
`syn_UserImage`, plus the reserved `chroma_Data_*` namespace and the
seven meta-control underscored uniforms). Per-symbol rows below._

**Score histogram:** 3× EXCEEDS+, 1× EXCEEDS, 12× MATCH (16
Synesthesia rows) + 4 net-new chroma-side rows.

### §1.1 Time-related uniforms

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 62 | `TIME` (`float`) — seconds since scene started | 5 / EXCEEDS+ | `ReactiveSymbol::Time` + catalog §1.1 | T1 pin via the script-side reactive-symbol pins. |
| 63 | `FRAMECOUNT` (`float`) — frame counter as float | 3 / MATCH | `ReactiveSymbol::FrameCount` | |
| 64 | `PI` (`const float`) — ~3.14159 | 3 / MATCH | `ReactiveSymbol::Pi` | |

### §1.2 Pixel-coordinate uniforms

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 65 | `RENDERSIZE` (`vec2`) — current pass size in pixels | 3 / MATCH | Catalog §1.4 (per-pass) | Pass-only; excluded from script-host mirror per catalog §5. |
| 66 | `_xy` (`vec2`) — `gl_FragCoord.xy` | 3 / MATCH | Catalog §1.2 | Fragment-only. |
| 67 | `_uv` (`vec2`) — normalized `[0,1]` | 3 / MATCH | Catalog §1.2 | Fragment-only. |
| 68 | `_uvc` (`vec2`) — aspect-corrected | 3 / MATCH | Catalog §1.2 | Fragment-only. |

### §1.3 Mouse uniforms

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 69 | `_mouse` (`vec4`) — emulates Shadertoy `iMouse` | 5 / EXCEEDS+ | `ReactiveSymbol::Mouse` with `Vec4` carrier; T1 pin on dimensionality | |
| 70 | `_click` (`vec3`) — left/right/middle | 3 / MATCH | `ReactiveSymbol::Click` with `Vec3` carrier | |
| 71 | `_muv` (`vec2`) — normalized mouse | 3 / MATCH | `ReactiveSymbol::MouseUv` with `Vec2` carrier | |
| 72 | `_muvc` (`vec2`) — aspect-corrected mouse | 3 / MATCH | `ReactiveSymbol::MouseUvc` with `Vec2` carrier | |

### §1.4 Multipass uniforms

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 73 | `PASSINDEX` (`int`) — current pass index | 3 / MATCH | Catalog §1.4 — Profile C | Pass-only; excluded from script-host mirror. |
| 74 | `syn_FinalPass` (`sampler2D`) — previous frame | 3 / MATCH | Catalog §1.4 | Sampler; accessed via `textureFinalPass(x,y)` helper per script §5. |

### §1.5 Media uniforms

| # | Synesthesia symbol | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 75 | `syn_Media` (`sampler2D`) — selected media, post-transforms | 3 / MATCH | Catalog §1.5 | `_isMediaActive()` is the documented preferred guard. |
| 76 | `syn_MediaType` (`float`) — None=0 / Image=1 / Video=2 / Webcam=3 | 3 / MATCH | Catalog §1.5 | Numeric discriminator. |

### Chromaticity-native standard additions (catalog rows beyond docs.synesthesia.live)

| # | Chroma-side feature | Score | Source | Notes |
| :- | :------------------- | :---: | :----- | :---- |
| 77 | `TIMEDELTA` (`float`) — seconds since previous frame | 4 / EXCEEDS | `ReactiveSymbol::TimeDelta` + catalog §1.1 | Native chroma addition; also exposed under compat-syn-v1 for forward compat. |
| 78 | `syn_UserImage` (`sampler2D`) — raw media pre-transform | 4 / EXCEEDS | Catalog §1.5 | Pre-transform handle; catalog §3.2 fully specifies meta-pipeline ordering. |
| 79 | Reserved `chroma_Data_*` namespace (5 sub-shapes) | 4 / EXCEEDS | Catalog §1.6 | `_Scalar_<name>`, `_Vec_<name>`, `_Array_<name>` (sampler1D), `_Field_<name>` (sampler2D/3D), `_Event_<name>` (ring buffer); parsed at MVP, populated post-MVP via `DATA_SOURCES`. |
| 80 | Meta-control underscored namespace (7 cores + meta-media + dichrome + preference-backed) | 4 / EXCEEDS | Catalog §3 | `_Master`, `_Brightness`, `_Gamma`, `_Contrast`, `_Hue`, `_Saturation`, `_Invert` + media/geometry/dichrome meta + `_Reactivity` / `_Audio_Speed` gates — not on Synesthesia's standard-uniforms page. |

---

## SSF GLSL helper functions

_Synesthesia's functions page documents 64 individual GLSL helpers
across six families (math, noise, coordinates, color, media,
multipass). Chromaticity's clean-room helper shim catalog is at
`docs/36-reactive-symbol-catalog.md` §4 with parity signatures;
meta-pipeline helpers (§4.8) and deprecated compatibility aliases
(§4.9) add 21 net-new rows. Bound GLSL bodies await the ADR-011 live
render wave; the catalog signatures are pinned today._

**Score histogram:** 0× EXCEEDS+, 0× EXCEEDS, 64× MATCH (64
Synesthesia rows) + 21 net-new chroma-side EXCEEDS rows.

### §4.1 Math helpers

| # | Synesthesia helper | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 81 | `_scale()` (range [0,1] → [min,max]) | 3 / MATCH | Catalog §4.1 | Parity signature. |
| 82 | `_map()` (one range → another; float/vec2/vec3/vec4) | 3 / MATCH | Catalog §4.1 | Parity signature + 4 overload arities. |
| 83 | `_smin()` (smooth min) | 3 / MATCH | Catalog §4.1 | |
| 84 | `_rand()` (pseudo-random from float or vec2 seed) | 3 / MATCH | Catalog §4.1 | |
| 85 | `_pulse()` (smooth pulse) | 3 / MATCH | Catalog §4.1 | |
| 86 | `_sqPulse()` (sharp square pulse) | 3 / MATCH | Catalog §4.1 | |
| 87 | `_inRange()` (bounds test → 0/1) | 3 / MATCH | Catalog §4.1 | |
| 88 | `_triWave()` (triangle wave) | 3 / MATCH | Catalog §4.1 | |
| 89 | `_pixelate()` (discrete quantization) | 3 / MATCH | Catalog §4.1 | |
| 90 | `_mix3()` (3-value blend) | 3 / MATCH | Catalog §4.1 | |
| 91 | `_mix4()` (4-value blend) | 3 / MATCH | Catalog §4.1 | |
| 92 | `_mix5()` (5-value blend) | 3 / MATCH | Catalog §4.1 | |
| 93 | `_nsin()` (sine normalized [0,1]) | 3 / MATCH | Catalog §4.1 | |
| 94 | `_ncos()` (cosine normalized [0,1]) | 3 / MATCH | Catalog §4.1 | |
| 95 | `_nclamp()` (clamp to [0,1]) | 3 / MATCH | Catalog §4.1 | |

### §4.2 Noise helpers

| # | Synesthesia helper | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 96 | `_noise()` (float / vec2 / vec3 seed) | 3 / MATCH | Catalog §4.2 | Parity. |
| 97 | `_fbm()` (octave-controlled fBm) | 3 / MATCH | Catalog §4.2 | Parity. |
| 98 | `_statelessContinuousChaotic()` (chaotic from increasing input) | 3 / MATCH | Catalog §4.2 | Parity. |
| 99 | `_hash11()` float → float | 3 / MATCH | Catalog §4.2 | |
| 100 | `_hash12()` vec2 → float | 3 / MATCH | Catalog §4.2 | |
| 101 | `_hash13()` vec3 → float | 3 / MATCH | Catalog §4.2 | |
| 102 | `_hash21()` float → vec2 | 3 / MATCH | Catalog §4.2 | |
| 103 | `_hash22()` vec2 → vec2 | 3 / MATCH | Catalog §4.2 | |
| 104 | `_hash23()` vec3 → vec2 | 3 / MATCH | Catalog §4.2 | |
| 105 | `_hash31()` float → vec3 | 3 / MATCH | Catalog §4.2 | |
| 106 | `_hash32()` vec2 → vec3 | 3 / MATCH | Catalog §4.2 | |
| 107 | `_hash33()` vec3 → vec3 | 3 / MATCH | Catalog §4.2 | |
| 108 | `_hash41()` float → vec4 | 3 / MATCH | Catalog §4.2 | |
| 109 | `_hash42()` vec2 → vec4 | 3 / MATCH | Catalog §4.2 | |
| 110 | `_hash43()` vec3 → vec4 | 3 / MATCH | Catalog §4.2 | |
| 111 | `_hash44()` vec4 → vec4 | 3 / MATCH | Catalog §4.2 | |

### §4.3 Coordinate helpers

| # | Synesthesia helper | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 112 | `_uv2uvc()` (screen-space → center-origin, `inout`) | 3 / MATCH | Catalog §4.3 | |
| 113 | `_uvc2uv()` (center-origin → screen-space, `inout`) | 3 / MATCH | Catalog §4.3 | |
| 114 | `_rotate()` (vec2 + vec3-axis overload, degrees) | 3 / MATCH | Catalog §4.3 | |
| 115 | `_mirror()` (mirror for seamless tiling) | 3 / MATCH | Catalog §4.3 | |
| 116 | `_toPolar()` (cartesian → polar) | 3 / MATCH | Catalog §4.3 | |
| 117 | `_toPolarTrue()` (quadrant-aware polar) | 3 / MATCH | Catalog §4.3 | |
| 118 | `_toRect()` (polar → cartesian) | 3 / MATCH | Catalog §4.3 | |

### §4.4 Color helpers

| # | Synesthesia helper | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 119 | `_rgb2hsv()` | 3 / MATCH | Catalog §4.4 | |
| 120 | `_hsv2rgb()` | 3 / MATCH | Catalog §4.4 | |
| 121 | `_normalizeRGB()` (0-255 → 0.0-1.0) | 3 / MATCH | Catalog §4.4 | |
| 122 | `_palette()` (cosine-based, Inigo Quilez) | 3 / MATCH | Catalog §4.4 | |
| 123 | `_brightness()` | 3 / MATCH | Catalog §4.4 | |
| 124 | `_gamma()` | 3 / MATCH | Catalog §4.4 | |
| 125 | `_contrast()` | 3 / MATCH | Catalog §4.4 | |
| 126 | `_saturation()` | 3 / MATCH | Catalog §4.4 | |
| 127 | `_hueRotate()` (degrees) | 3 / MATCH | Catalog §4.4 | |
| 128 | `_invert()` (with blend amount) | 3 / MATCH | Catalog §4.4 | |
| 129 | `_luminance()` (perceived brightness) | 3 / MATCH | Catalog §4.4 | |
| 130 | `_grayscale()` (luminance-based gray) | 3 / MATCH | Catalog §4.4 | |
| 131 | `_dichrome()` (dark/bright color map) | 3 / MATCH | Catalog §4.4 | |

### §4.5 Media helpers

| # | Synesthesia helper | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 132 | `_loadMedia()` (with offset / LOD) | 3 / MATCH | Catalog §4.5 | |
| 133 | `_textureMedia()` (custom UV, aspect-corrected) | 3 / MATCH | Catalog §4.5 | |
| 134 | `_loadMediaAsMask()` (binary mask via threshold) | 3 / MATCH | Catalog §4.5 | |
| 135 | `_textureMediaAsMask()` (binary mask, custom UV) | 3 / MATCH | Catalog §4.5 | |
| 136 | `_correctMediaCoords()` (aspect-ratio correction) | 3 / MATCH | Catalog §4.5 | |
| 137 | `_isMediaActive()` (bool: media selected) | 3 / MATCH | Catalog §4.5 | Preferred guard per Synesthesia docs. |
| 138 | `_edgeDetectMedia()` (bilinear) | 3 / MATCH | Catalog §4.5 | |
| 139 | `_edgeDetectSobelMedia()` (Sobel) | 3 / MATCH | Catalog §4.5 | |

### §4.6 Multipass helpers

| # | Synesthesia helper | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 140 | `_edgeDetect()` (bilinear, generic texture) | 3 / MATCH | Catalog §4.6 | |
| 141 | `_edgeDetectSobel()` (Sobel, generic texture) | 3 / MATCH | Catalog §4.6 | |
| 142 | `_hBlurTiltShift()` (horizontal tilt-shift) | 3 / MATCH | Catalog §4.6 | |
| 143 | `_vBlurTiltShift()` (vertical tilt-shift) | 3 / MATCH | Catalog §4.6 | |

### Chromaticity-native meta-pipeline helpers (catalog §4.8, prepended at scene-compile time)

| # | Chroma-side helper | Score | Source | Notes |
| :- | :------------------ | :---: | :----- | :---- |
| 144 | `_applyMaster` | 4 / EXCEEDS | Catalog §4.8 | Net-new meta-pipeline scaffolding. |
| 145 | `_applyBrightness` | 4 / EXCEEDS | Catalog §4.8 | |
| 146 | `_applyGamma` | 4 / EXCEEDS | Catalog §4.8 | |
| 147 | `_applyContrast` | 4 / EXCEEDS | Catalog §4.8 | |
| 148 | `_applyMediaContrast` | 4 / EXCEEDS | Catalog §4.8 | Media-side variant. |
| 149 | `_applyMediaGamma` | 4 / EXCEEDS | Catalog §4.8 | |
| 150 | `_applyMediaHueSaturation` | 4 / EXCEEDS | Catalog §4.8 | |
| 151 | `_applyHueSaturation` | 4 / EXCEEDS | Catalog §4.8 | |
| 152 | `_applyInvert` | 4 / EXCEEDS | Catalog §4.8 | |
| 153 | `_applyInvertMedia` | 4 / EXCEEDS | Catalog §4.8 | |
| 154 | `_applyColorCorrection` | 4 / EXCEEDS | Catalog §4.8 | |
| 155 | `_applyDichrome` | 4 / EXCEEDS | Catalog §4.8 | |
| 156 | `_applyVerticalMirroring` | 4 / EXCEEDS | Catalog §4.8 | |
| 157 | `_applyHorizontalMirroring` | 4 / EXCEEDS | Catalog §4.8 | |
| 158 | `_zoomOrFit` | 4 / EXCEEDS | Catalog §4.8 | |
| 159 | `_applyAlpha` | 4 / EXCEEDS | Catalog §4.8 | |
| 160 | `_applyMediaScale` | 4 / EXCEEDS | Catalog §4.8 | |
| 161 | `_correctImageCoords` | 4 / EXCEEDS | Catalog §4.8 | |
| 162 | `_flipMediaCoords` | 4 / EXCEEDS | Catalog §4.8 | |
| 163 | `_imageToMask` | 4 / EXCEEDS | Catalog §4.8 | |
| 164 | `_hueSaturation` | 4 / EXCEEDS | Catalog §4.8 | |

### Chromaticity reference-observed convenience helper (catalog §4.7)

| # | Chroma-side helper | Score | Source | Notes |
| :- | :------------------ | :---: | :----- | :---- |
| 164a | `_textureImage` | 4 / EXCEEDS | Catalog §4.7 | Reference-observed but undocumented in public SSF — Chromaticity covers it. |

### Chromaticity deprecated-compatibility aliases (catalog §4.9, compat-syn-v1 only)

| # | Chroma-side alias | Score | Source | Notes |
| :- | :----------------- | :---: | :----- | :---- |
| 164b | `_applyAllLuminanceCorrections` | 4 / EXCEEDS | Catalog §4.9 | Reference-scene portability. |
| 164c | `_hueSaturationLEGACY` | 4 / EXCEEDS | Catalog §4.9 | |
| 164d | `_hueSaturationContrast` | 4 / EXCEEDS | Catalog §4.9 | |
| 164e | `_hueSaturationContrastLEGACY` | 4 / EXCEEDS | Catalog §4.9 | |
| 164f | `_applyHueSaturationContrast` | 4 / EXCEEDS | Catalog §4.9 | |
| 164g | `_invertImage` | 4 / EXCEEDS | Catalog §4.9 | |
| 164h | `_loadUserImage` | 4 / EXCEEDS | Catalog §4.9 | |
| 164i | `_textureUserImage` | 4 / EXCEEDS | Catalog §4.9 | |
| 164j | `_loadUserImageAsMask` | 4 / EXCEEDS | Catalog §4.9 | |
| 164k | `_textureUserImageAsMask` | 4 / EXCEEDS | Catalog §4.9 | |

---

## SSF external libraries (Lygia + hg_sdf + custom)

_Synesthesia documents three include-source families: Lygia v1.1.4
(via category subdirectories), hg_sdf (a single file with 5 helpers
+ 28 primitives + 14 domain ops + 17 combinators), and per-user
custom GLSL libraries (a `glsl/` folder in user data). Chromaticity
vendors Lygia v1.1.4 at `third_party/lygia/` with all 14 category
subdirectories present, plus the GLSL preprocessor at
`engine/render/src/glsl_includes.rs`. hg_sdf is license-elected but
not yet vendored; per-user library root is not yet authored._

**Score histogram:** 0× EXCEEDS+, 14× EXCEEDS (Lygia categories
exceed via cycle/depth guard + vendor-root sandbox), 0× MATCH
(Lygia per-helper inherit category posture), 64× PARTIAL (hg_sdf
content), 1× GAP (per-user custom library).

### Lygia v1.1.4 — vendored at `third_party/lygia/`

| # | Synesthesia surface | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 165 | `#include "lygia/[category]/[function].glsl"` include syntax | 4 / EXCEEDS | `engine/render/src/glsl_includes.rs::resolve_includes` with `MAX_INCLUDE_DEPTH = 32` cycle guard | Same syntax; chromaticity adds depth + vendor-root sandbox hardening. |
| 166 | Lygia version 1.1.4 pin | 3 / MATCH | `third_party/lygia/UPSTREAM-VERSION.md` v1.1.4 (commit `28f220b6`) | Same pinned version. |
| 167 | `math/` category | 4 / EXCEEDS | `third_party/lygia/math/` (PI, SqrtLength, abs, asin, clamp, const, cross, dot, floor, frac, fract, gain, gaussian, et al — 60+ entries) | Full category vendored; chromaticity adds cycle/depth guard + comment-passthrough discipline. |
| 168 | `space/` category (scale, rotate) | 4 / EXCEEDS | `third_party/lygia/space/` | Same scope. |
| 169 | `color/` category (luma, saturation, blends, palettes, color-space conversion, tonemaps) | 4 / EXCEEDS | `third_party/lygia/color/` (luma, brightnessContrast, brightnessMatrix, contrast, daltonize, desaturate, distance, dither, exposure, hueShift, levels, blend/, et al) | Full category vendored. |
| 170 | `animation/` category (easing) | 4 / EXCEEDS | `third_party/lygia/animation/` (easing, spriteLoop) | Same. |
| 171 | `generative/` category (random, noise) | 4 / EXCEEDS | `third_party/lygia/generative/` (cnoise, curl, fbm, gerstnerWave, gnoise, noised, pnoise, psrdnoise, random, snoise, srandom, voronoi, voronoise, worley) | Broader than Synesthesia's documentation suggests. |
| 172 | `sdf/` category (signed distance fields) | 4 / EXCEEDS | `third_party/lygia/sdf/` (boxSDF, capsuleSDF, circleSDF, coneSDF, cylinderSDF, dodecahedronSDF, ellipsoidSDF, hexSDF, et al) | Full category vendored — partial overlap with hg_sdf primitives. |
| 173 | `draw/` category (digits, stroke, fill) | 4 / EXCEEDS | `third_party/lygia/draw/` (aastep, arrows, et al) | Same. |
| 174 | `sample/` category | 4 / EXCEEDS | `third_party/lygia/sample/` (2DCube, 3DSdf, bicubic) | Same. |
| 175 | `filter/` category (blurs, mean/median) | 4 / EXCEEDS | `third_party/lygia/filter/` (bilateral, boxBlur, bracketing, brick, cart2polar) | Same. |
| 176 | `distort/` category | 4 / EXCEEDS | `third_party/lygia/distort/` (barrel, chromaAB, grain) | Same. |
| 177 | `simulate/` category | 4 / EXCEEDS | `third_party/lygia/simulate/` (fluidSolver, grayscott, ripple) | Same. |
| 178 | `lighting/` category | 4 / EXCEEDS | `third_party/lygia/lighting/` (atmosphere, blackbody) | Same. |
| 179 | `geometry/` category (intersections, AABB) | 4 / EXCEEDS | `third_party/lygia/geometry/` (aabb, triangle, et al) | Same. |
| 180 | `morphological/` category (dilation, erosion, fill) | 4 / EXCEEDS | `third_party/lygia/morphological/` (alphaFill, dilation, erosion) | Same. |

### hg_sdf — license-elected but not yet vendored

Chromaticity's `docs/29-cleanroom-legal-policy-baseline.md` row has
the MIT election as the required clean-room selection; the actual
snapshot has not landed under `third_party/`. Tracked for
clean-room vendoring under `EV-LICENSE-AUDIT`.

| # | Synesthesia surface | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 181 | `#include "hg_sdf.glsl"` include syntax | 2 / PARTIAL | Preprocessor accepts any path-shaped include; the file itself is not yet vendored | Pre-vendoring posture. |

#### hg_sdf helper functions (5)

| # | hg_sdf helper | Score | Notes |
| :- | :------------- | :---: | :---- |
| 182 | `sgn()` (sign returning 1 or -1) | 2 / PARTIAL | Pre-vendoring. |
| 183 | `square()` | 2 / PARTIAL | Pre-vendoring. |
| 184 | `lengthSqr()` | 2 / PARTIAL | Pre-vendoring. |
| 185 | `vmax()` (max vector component) | 2 / PARTIAL | Pre-vendoring. |
| 186 | `vmin()` (min vector component) | 2 / PARTIAL | Pre-vendoring. |

#### hg_sdf primitives (28)

| # | hg_sdf primitive | Score | Notes |
| :- | :--------------- | :---: | :---- |
| 187 | `fSphere()` | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/` provides sphere equivalents. |
| 188 | `fPlane()` | 2 / PARTIAL | Pre-vendoring. |
| 189 | `fBoxCheap()` | 2 / PARTIAL | Pre-vendoring. |
| 190 | `fBox()` | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/boxSDF.glsl` covers similar surface. |
| 191 | `fBox2Cheap()` | 2 / PARTIAL | Pre-vendoring. |
| 192 | `fBox2()` | 2 / PARTIAL | Pre-vendoring. |
| 193 | `fCorner()` | 2 / PARTIAL | Pre-vendoring. |
| 194 | `fBlob()` | 2 / PARTIAL | Pre-vendoring. |
| 195 | `fCylinder()` | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/cylinderSDF.glsl` overlaps. |
| 196 | `fCapsule()` (variant 1) | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/capsuleSDF.glsl` overlaps. |
| 197 | `fCapsule()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |
| 198 | `fCapsule()` (variant 3) | 2 / PARTIAL | Pre-vendoring. |
| 199 | `fLineSegment()` | 2 / PARTIAL | Pre-vendoring. |
| 200 | `fTorus()` | 2 / PARTIAL | Pre-vendoring. |
| 201 | `fCircle()` (variant 1) | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/circleSDF.glsl` overlaps. |
| 202 | `fCircle()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |
| 203 | `fDisc()` | 2 / PARTIAL | Pre-vendoring. |
| 204 | `fHexagonCircumcircle()` | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/hexSDF.glsl` overlaps. |
| 205 | `fHexagonIncircle()` | 2 / PARTIAL | Pre-vendoring. |
| 206 | `fCone()` | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/coneSDF.glsl` overlaps. |
| 207 | `fOctahedron()` (variant 1) | 2 / PARTIAL | Pre-vendoring. |
| 208 | `fOctahedron()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |
| 209 | `fDodecahedron()` (variant 1) | 2 / PARTIAL | Pre-vendoring; Lygia `sdf/dodecahedronSDF.glsl` overlaps. |
| 210 | `fDodecahedron()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |
| 211 | `fIcosahedron()` (variant 1) | 2 / PARTIAL | Pre-vendoring. |
| 212 | `fIcosahedron()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |
| 213 | `fTruncatedOctahedron()` (variant 1) | 2 / PARTIAL | Pre-vendoring. |
| 214 | `fTruncatedOctahedron()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |
| 215 | `fTruncatedIcosahedron()` (variant 1) | 2 / PARTIAL | Pre-vendoring. |
| 216 | `fTruncatedIcosahedron()` (variant 2) | 2 / PARTIAL | Pre-vendoring. |

#### hg_sdf domain manipulation (14)

| # | hg_sdf domain op | Score | Notes |
| :- | :--------------- | :---: | :---- |
| 217 | `pR()` (rotation in 2D) | 2 / PARTIAL | Pre-vendoring. |
| 218 | `pR45()` (45° rotation) | 2 / PARTIAL | Pre-vendoring. |
| 219 | `pMod1()` (1D modulo) | 2 / PARTIAL | Pre-vendoring. |
| 220 | `pModMirror1()` | 2 / PARTIAL | Pre-vendoring. |
| 221 | `pModSingle1()` | 2 / PARTIAL | Pre-vendoring. |
| 222 | `pModInterval1()` | 2 / PARTIAL | Pre-vendoring. |
| 223 | `pModPolar()` | 2 / PARTIAL | Pre-vendoring. |
| 224 | `pMod2()` | 2 / PARTIAL | Pre-vendoring. |
| 225 | `pModMirror2()` | 2 / PARTIAL | Pre-vendoring. |
| 226 | `pModGrid2()` | 2 / PARTIAL | Pre-vendoring. |
| 227 | `pMod3()` | 2 / PARTIAL | Pre-vendoring. |
| 228 | `pMirror()` | 2 / PARTIAL | Pre-vendoring. |
| 229 | `pMirrorOctant()` | 2 / PARTIAL | Pre-vendoring. |
| 230 | `pReflect()` | 2 / PARTIAL | Pre-vendoring. |

#### hg_sdf object combinations (17)

| # | hg_sdf combinator | Score | Notes |
| :- | :---------------- | :---: | :---- |
| 231 | `fOpUnionChamfer()` | 2 / PARTIAL | Pre-vendoring. |
| 232 | `fOpIntersectionChamfer()` | 2 / PARTIAL | Pre-vendoring. |
| 233 | `fOpDifferenceChamfer()` | 2 / PARTIAL | Pre-vendoring. |
| 234 | `fOpUnionRound()` | 2 / PARTIAL | Pre-vendoring. |
| 235 | `fOpIntersectionRound()` | 2 / PARTIAL | Pre-vendoring. |
| 236 | `fOpDifferenceRound()` | 2 / PARTIAL | Pre-vendoring. |
| 237 | `fOpUnionColumns()` | 2 / PARTIAL | Pre-vendoring. |
| 238 | `fOpDifferenceColumns()` | 2 / PARTIAL | Pre-vendoring. |
| 239 | `fOpIntersectionColumns()` | 2 / PARTIAL | Pre-vendoring. |
| 240 | `fOpUnionStairs()` | 2 / PARTIAL | Pre-vendoring. |
| 241 | `fOpIntersectionStairs()` | 2 / PARTIAL | Pre-vendoring. |
| 242 | `fOpDifferenceStairs()` | 2 / PARTIAL | Pre-vendoring. |
| 243 | `fOpUnionSoft()` | 2 / PARTIAL | Pre-vendoring. |
| 244 | `fOpPipe()` | 2 / PARTIAL | Pre-vendoring. |
| 245 | `fOpEngrave()` | 2 / PARTIAL | Pre-vendoring. |
| 246 | `fOpGroove()` | 2 / PARTIAL | Pre-vendoring. |
| 247 | `fOpTongue()` | 2 / PARTIAL | Pre-vendoring. |

### Custom user library

| # | Synesthesia surface | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 248 | Per-user GLSL library (`glsl/` folder in user data; `#include "lib.glsl"`) | 1 / GAP | Per-scene `glsl/*.glsl` shared includes exist; no per-user library root yet | The per-scene surface covers a subset; the per-user folder is the missing layer. |

---

## SSF JSON configuration (`scene.json`)

_Synesthesia's JSON-configuration page documents 12 top-level keys,
11 per-control fields, 7 control kinds with 12 variants, 5 per-pass
fields, and per-image / per-media field sets. Chromaticity's
`engine/scene/src/schema.rs` carries the typed shape and the
flatten-preserved extensions map for every type, with the slice-3a
wire format frozen against `docs/02-synesthesia-contracts.md`._

**Score histogram:** 0× EXCEEDS+, 13× EXCEEDS, 35× MATCH, 3× PARTIAL.

### Top-level keys (12)

| # | Synesthesia key | Score | Chromaticity equivalent | Notes |
| :- | :--------------- | :---: | :----------------------- | :---- |
| 249 | `TITLE` | 3 / MATCH | Flatten-preserved on `Scene::extensions` per `engine/scene/src/schema.rs:73-86` | Round-trip via `#[serde(flatten)]`. |
| 250 | `CREDIT` | 3 / MATCH | Flatten-preserved on `Scene::extensions` | |
| 251 | `DESCRIPTION` | 3 / MATCH | Flatten-preserved on `Scene::extensions` | |
| 252 | `IMAGE_PATH` (thumbnail path) | 3 / MATCH | Flatten-preserved on `Scene::extensions` | |
| 253 | `WIDTH` (output resolution) | 3 / MATCH | Flatten-preserved on `Scene::extensions` | |
| 254 | `HEIGHT` (output resolution) | 3 / MATCH | Flatten-preserved on `Scene::extensions` | |
| 255 | `CONTROLS` (array of control objects) | 4 / EXCEEDS | `Scene::controls: Vec<ControlDecl>` typed at `engine/scene/src/schema.rs:73-74` | Typed parse + CP-01 coercion. |
| 256 | `PASSES` (array of multipass targets) | 4 / EXCEEDS | `Scene::passes: Vec<PassSpec>` typed at `engine/scene/src/schema.rs:75-76` | Typed parse. |
| 257 | `HARD_TRANSITIONS` (random-value transitions) | 2 / PARTIAL | Flatten-preserved on `Scene::extensions` | Structured parsing lands in a later slice. |
| 258 | `SMOOTH_TRANSITIONS` (codependent transition families) | 2 / PARTIAL | Flatten-preserved on `Scene::extensions` | Structured parsing later-slice. |
| 259 | `IMAGES` (bundled image objects) | 2 / PARTIAL | Flatten-preserved on `Scene::extensions` | Structured parsing later-slice. |
| 260 | `MEDIA` (texture lookup configuration) | 3 / MATCH | Flatten-preserved on `Scene::extensions` | Round-trip preserved. |

### Per-control fields (11)

| # | Synesthesia field | Score | Chromaticity equivalent | Notes |
| :- | :----------------- | :---: | :----------------------- | :---- |
| 261 | `TYPE` (control category) | 4 / EXCEEDS | `ControlDecl::kind: Option<ControlKind>` per `engine/scene/src/schema.rs:296-297` | Optional + CP-01 missing-TYPE fallback in `engine/scene/src/validator.rs`. |
| 262 | `NAME` (uniform identifier) | 3 / MATCH | `ControlDecl::name: String` | Required field. |
| 263 | `DEFAULT` (initial value) | 4 / EXCEEDS | `ControlDecl::default: ControlValue` with shape-discriminated untagged enum at `engine/scene/src/schema.rs:252-263` | `Bool`-before-`Float` ordering prevents 0/1 coercion. |
| 264 | `MIN` (range lower bound) | 3 / MATCH | `ControlDecl::min: Option<f64>` | |
| 265 | `MAX` (range upper bound) | 3 / MATCH | `ControlDecl::max: Option<f64>` | |
| 266 | `UI_GROUP` (panel grouping) | 3 / MATCH | `ControlDecl::ui_group: Option<String>` | |
| 267 | `DESCRIPTION` (tooltip text) | 3 / MATCH | `ControlDecl::description: Option<String>` | |
| 268 | `PARAMS` (smoothing speed for smooth variants) | 3 / MATCH | `ControlDecl::params: Option<serde_json::Value>` | Free-form per-variant payload. |
| 269 | `DEF_COLOR` (color-control default special key) | 3 / MATCH | `ControlDecl::def_color: Option<[f64; 4]>` | 4-float RGBA. |
| 270 | `LABELS` (dropdown labels) | 3 / MATCH | `ControlDecl::labels: Vec<serde_json::Value>` | |
| 271 | `VALUES` (dropdown numerical values) | 3 / MATCH | `ControlDecl::values: Vec<serde_json::Value>` | |

### Control kinds (7)

| # | Synesthesia control kind | Score | Chromaticity equivalent | Notes |
| :- | :------------------------ | :---: | :----------------------- | :---- |
| 272 | `slider` (numeric scalar) | 4 / EXCEEDS | `ControlKind::Slider` per `engine/scene/src/schema.rs:198` | TYPE-token compat parse via first-whitespace-token rule. |
| 273 | `toggle` (boolean) | 4 / EXCEEDS | `ControlKind::Toggle` | |
| 274 | `color` (3- or 4-component) | 4 / EXCEEDS | `ControlKind::Color` | |
| 275 | `xy` (2-component position) | 4 / EXCEEDS | `ControlKind::Xy` | |
| 276 | `bang` (momentary trigger) | 4 / EXCEEDS | `ControlKind::Bang` | |
| 277 | `dropdown` (discrete option) | 4 / EXCEEDS | `ControlKind::Dropdown` | |
| 278 | `knob` (rotary scalar) | 4 / EXCEEDS | `ControlKind::Knob` | |

### Control variants (12)

| # | Synesthesia variant | Score | Chromaticity equivalent | Notes |
| :- | :------------------- | :---: | :----------------------- | :---- |
| 279 | `slider smooth` | 3 / MATCH | First-whitespace-token rule collapses to `Slider`; smoothing carried on `PARAMS` | Compat parse honoured. |
| 280 | `slider speed` | 3 / MATCH | Same — collapses to `Slider` | Variant token carried via `PARAMS`. |
| 281 | `knob smooth` | 3 / MATCH | Collapses to `Knob` | |
| 282 | `knob speed` | 3 / MATCH | Collapses to `Knob` | |
| 283 | `toggle smooth` | 3 / MATCH | Collapses to `Toggle` | |
| 284 | `bang smooth` | 3 / MATCH | Collapses to `Bang` | |
| 285 | `bang counter` | 3 / MATCH | Collapses to `Bang` | |
| 286 | `bang counter smooth` | 3 / MATCH | Collapses to `Bang` | |
| 287 | `xy smooth` | 3 / MATCH | Collapses to `Xy` | |
| 288 | `xy speed` | 3 / MATCH | Collapses to `Xy` | |
| 289 | `color smooth` | 3 / MATCH | Collapses to `Color` | |
| 290 | `dropdown smooth` | 3 / MATCH | Collapses to `Dropdown` | |

### Per-pass fields (5)

| # | Synesthesia field | Score | Chromaticity equivalent | Notes |
| :- | :----------------- | :---: | :----------------------- | :---- |
| 291 | `TARGET` (sampler2D name) | 3 / MATCH | `PassSpec::node: NodeRef` per `engine/scene/src/schema.rs:167-168` | Newtype round-trips through derived `Deserialize` as a plain string. |
| 292 | `WIDTH` (buffer width) | 3 / MATCH | Flatten-preserved on `PassSpec::extensions` per `engine/scene/src/schema.rs:171-176` | Structured parsing later-slice. |
| 293 | `HEIGHT` (buffer height) | 3 / MATCH | Flatten-preserved on `PassSpec::extensions` | |
| 294 | `FLOAT` (precision: 32-bit vs 8-bit) | 3 / MATCH | Flatten-preserved on `PassSpec::extensions` | |
| 295 | `WRAP` (repeat/clamp/mirror) | 3 / MATCH | Flatten-preserved on `PassSpec::extensions` | |
| 296 | `FILTER` (linear/nearest/mipmap) | 3 / MATCH | Flatten-preserved on `PassSpec::extensions` | |

### Per-image fields (2)

| # | Synesthesia field | Score | Chromaticity equivalent | Notes |
| :- | :----------------- | :---: | :----------------------- | :---- |
| 297 | `NAME` (image uniform identifier) | 3 / MATCH | Flatten-preserved on `Scene::extensions` (IMAGES) | Structured parsing later-slice. |
| 298 | `PATH` (file location) | 3 / MATCH | Flatten-preserved on `Scene::extensions` (IMAGES) | Structured parsing later-slice. |

### Per-media fields (2)

| # | Synesthesia field | Score | Chromaticity equivalent | Notes |
| :- | :----------------- | :---: | :----------------------- | :---- |
| 299 | `WRAP` (media texture wrap) | 3 / MATCH | Flatten-preserved on `Scene::extensions` (MEDIA) | |
| 300 | `FILTER` (media texture filter) | 3 / MATCH | Flatten-preserved on `Scene::extensions` (MEDIA) | |

### Chromaticity-side JSON additions

| # | Chroma-side feature | Score | Source | Notes |
| :- | :------------------- | :---: | :----- | :---- |
| 301 | `SCHEMA_VERSION` (Chromaticity-reserved versioning key) | 4 / EXCEEDS | `SceneHeader::schema_version` defaults to 1 per `engine/scene/src/schema.rs:97-104` | Reference scenes lack the key; defaults safe. |
| 302 | Legacy `IMAGE` / `CATEGORIES` preserved | 4 / EXCEEDS | Flatten-preserved per `engine/scene/src/schema.rs:73-86` prose | Reference-observed outliers round-trip. |
| 303 | `IS_META` per-control flag preserved | 4 / EXCEEDS | `ControlDecl::is_meta: Option<bool>` per `engine/scene/src/schema.rs:320-321` | Explicit field for meta-control flagging. |
| 304 | Legacy `LABEL` per-control field preserved | 4 / EXCEEDS | `ControlDecl::label: Option<String>` per `engine/scene/src/schema.rs:322-323` | Reference-observed alternative to `NAME`. |

---

## SSF JavaScript scripting

_Synesthesia's scripting page documents 17 individual identifiers
across six families (lifecycle hooks, event-handler registration,
control manipulation, uniform management, pixel sampling, utility,
plus the global-access pattern). Chromaticity's `engine/script/`
crate uses QuickJS via rquickjs per ADR-012, with the producer side
of the typed reactive-symbol catalog at
`engine/script/src/reactive_symbol.rs`. Per-handler scene-author
bindings are slice 5.4+ scope — the producer/consumer/catalog/adapter
quad shipped at slice 5.3 EXIT quality but the per-event-handler
bindings have not all landed._

**Score histogram:** 1× EXCEEDS+, 2× EXCEEDS, 8× MATCH, 2× PARTIAL,
4× GAP.

### Lifecycle hooks (2)

| # | Synesthesia hook | Score | Chromaticity equivalent | Notes |
| :- | :---------------- | :---: | :----------------------- | :---- |
| 305 | `setup()` | 3 / MATCH | ADR-012 lifecycle includes `setup()` with 500 ms hard timeout | Engine spec; binding lands in a later slice. |
| 306 | `update(dt)` | 4 / EXCEEDS | ADR-012 `update(dt)` with 1.75 ms p99 + 4 ms watchdog + 2-strike degradation | Numeric budgets pinned; Synesthesia documents none. |

### Event-handler registration (5)

| # | Synesthesia handler | Score | Chromaticity equivalent | Notes |
| :- | :------------------- | :---: | :----------------------- | :---- |
| 307 | `onChange(target, callback)` | 1 / GAP | Slice 5.4+ scope — per-handler scene-author bindings not yet landed | Producer/consumer/catalog/adapter quad shipped at slice 5.3 EXIT. |
| 308 | `onOffToOn(target, callback)` | 1 / GAP | Slice 5.4+ scope | |
| 309 | `onOnToOff(target, callback)` | 1 / GAP | Slice 5.4+ scope | |
| 310 | `whileOn(target, callback)` | 1 / GAP | Slice 5.4+ scope | |
| 311 | `whileOff(target, callback)` | 1 / GAP | Slice 5.4+ scope | |

### Control manipulation (6)

| # | Synesthesia function | Score | Chromaticity equivalent | Notes |
| :- | :-------------------- | :---: | :----------------------- | :---- |
| 312 | `setControl(name, values, options)` | 3 / MATCH | `ScriptCommand::ReactiveWrite { name, value }` at slice 5.1 | String-keyed write; typed variant at slice 5.2. |
| 313 | `setControlNormalized(name, values, options)` | 2 / PARTIAL | Routed through same `ReactiveWrite` shape with carrier-side normalisation; no dedicated normalized variant | Engine-side normalization deferred to slice 5.4+. |
| 314 | `randomizeControl(name)` | 2 / PARTIAL | Cataloged in ADR-012 §8; binding pending slice 5.4+ | |
| 315 | `defaultControl(name)` | 2 / PARTIAL | Cataloged in ADR-012 §8; binding pending slice 5.4+ | |
| 316 | `randomizeGroup(name)` | 2 / PARTIAL | Cataloged in ADR-012 §8; binding pending slice 5.4+ | |
| 317 | `defaultGroup(name)` | 2 / PARTIAL | Cataloged in ADR-012 §8; binding pending slice 5.4+ | |

### Uniform management (3)

| # | Synesthesia identifier | Score | Chromaticity equivalent | Notes |
| :- | :--------------------- | :---: | :----------------------- | :---- |
| 318 | `setUniform(name, x, y, z, w)` | 3 / MATCH | `ScriptCommand::ReactiveWrite` slice-5.1 surface + typed `TypedReactiveWrite` slice-5.2 | Catch-all variant covers it; typed catalog covers strongly-typed names. |
| 319 | `inputs[name]` (deprecated) | 3 / MATCH | Compat shim cataloged in ADR-012 §8 under compat-syn-v1 | Documented; binding lands in later slice. |
| 320 | `uniforms[name]` (deprecated) | 3 / MATCH | Compat shim cataloged in ADR-012 §8 under compat-syn-v1 | Documented; binding lands in later slice. |

### Pixel sampling (1)

| # | Synesthesia function | Score | Chromaticity equivalent | Notes |
| :- | :-------------------- | :---: | :----------------------- | :---- |
| 321 | `textureFinalPass(x, y)` (RGB sample) | 2 / PARTIAL | Catalog §5 specifies the spec; JS-side binding implementation is later-slice scope | Helper spec'd; runtime binding pending. |

### Utility functions (2)

| # | Synesthesia function | Score | Chromaticity equivalent | Notes |
| :- | :-------------------- | :---: | :----------------------- | :---- |
| 322 | `printGlobalKeys()` (list available globals) | 1 / GAP | Not yet exposed as a discrete diagnostic helper | Could leverage the reactive-symbol catalog round-trip for a parity-shaped surface. |
| 323 | `print()` / `console.log()` | 3 / MATCH | QuickJS provides `console.log` baseline; `print` shim per ADR-012 §8 | Sandbox-friendly. |

### Global access pattern

| # | Synesthesia surface | Score | Chromaticity equivalent | Notes |
| :- | :------------------ | :---: | :----------------------- | :---- |
| 324 | Global access pattern: uniforms + controls as globals (`TIME`, `syn_BassLevel`, `myXy.y`) | 5 / EXCEEDS+ | `ReactiveSymbol` typed catalog at `engine/script/src/reactive_symbol.rs` with 46 documented symbols; `_mouse` is Vec4, `_click` is Vec3, mouse UVs are Vec2; vector access via `{x,y}` / `{r,g,b}` aliases per `docs/36` §5 | Slice 5.2 enum with `#[non_exhaustive]` carries the typed surface; round-trip integrity pinned by T1 drift tests. |

---

## SSF best practices

_Synesthesia's best-practices page enumerates 30 individual
recommendations across four families (audio reactivity, controls,
GLSL float comparison, learning posture). Chromaticity's in-app
help corpus at `docs-corpus/docs/audio-reactivity.md` +
`docs-corpus/docs/scene-workflow.md` + `docs-corpus/docs/chroma-uniforms.md`
covers the audio + workflow surfaces; an explicit "controls best
practices" + "common pitfalls" + "GLSL performance" set of help
pages is the residual gap._

**Score histogram:** 1× EXCEEDS+, 3× EXCEEDS, 12× MATCH, 11× PARTIAL,
3× GAP.

### Audio reactivity recommendations (10)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 325 | Color flashing via `syn_HighHits` mix | 3 / MATCH | `docs-corpus/docs/audio-reactivity.md` worked example covers analogous mix patterns | Audio uniforms available. |
| 326 | Camera movement synced to bass timing | 3 / MATCH | `syn_BassTime` available in catalog §2.4 | Pattern available; help corpus prose example missing. |
| 327 | Color shifting via bass presence | 3 / MATCH | `syn_BassPresence` available in catalog §2.5 | Pattern available. |
| 328 | Beat-based positioning via random beat triggers | 3 / MATCH | `syn_RandomOnBeat` available in catalog §2.6 | Pattern available. |
| 329 | Canvas disruption on bass hits | 3 / MATCH | `syn_BassHits` available in catalog §2.3 | Pattern available. |
| 330 | Scanline pulsing via beat toggles | 3 / MATCH | `syn_ToggleOnBeat` available in catalog §2.6 | Pattern available. |
| 331 | BPM-driven color via BPM sine waves | 3 / MATCH | `syn_BPMSin` / `syn_BPMSin2` / `syn_BPMSin4` available | Pattern available. |
| 332 | Spectral displacement using frequency spectrum | 3 / MATCH | `syn_Spectrum` sampler available (post-ADR-011) plus `chroma_AudioFFT[8]` | Pattern available. |
| 333 | Depth-first approach (novel reactivity beyond basic flashing) | 2 / PARTIAL | General guidance in `docs-corpus/docs/audio-reactivity.md`; no explicit "depth-first" anti-pattern callout | Help-corpus polish opportunity. |
| 334 | User-agency balance (avoid hardcoded reactivity; provide toggles) | 4 / EXCEEDS | Meta-control `_Reactivity` differentiation-edition gate per catalog §3 plus per-control `IS_META` flag | Engine-level support; explicit help-page advice not yet authored. |

### Controls — organisation (3)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 335 | Grouping via `UI_GROUP` | 3 / MATCH | `ControlDecl::ui_group` field per `engine/scene/src/schema.rs:304-305` | Field available; explicit help-page guidance gap. |
| 336 | Visual clarity (short descriptive names) | 2 / PARTIAL | No explicit naming-style guidance in help corpus | Gap. |
| 337 | Complexity reduction (combine sliders into bidirectional controls) | 2 / PARTIAL | No explicit pattern in help corpus | Gap. |

### Controls — interaction design (3)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 338 | Avoid dead zones (no-op controls under certain conditions) | 2 / PARTIAL | No explicit anti-pattern in help corpus | Gap. |
| 339 | Document complex multi-control interactions | 3 / MATCH | `ControlDecl::description` field carries per-control tooltips | Field available; help-page guidance gap. |
| 340 | Prevent breaking states (limit ranges to avoid edge-case breakage) | 2 / PARTIAL | No explicit anti-pattern in help corpus | Gap. |

### Controls — range and curve optimisation (4)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 341 | Critical tuning of `MIN`/`MAX` | 3 / MATCH | `ControlDecl::min` / `max` fields available | Engine support; help-page guidance gap. |
| 342 | Epsilon adjustment (avoid exact-zero values) | 2 / PARTIAL | No explicit anti-pattern in help corpus | Gap. |
| 343 | Curve mapping (exponential/polynomial slider distribution) | 3 / MATCH | `PARAMS` field carries per-variant smoothing payload | Engine support; help-page guidance gap. |
| 344 | Interesting-zone focus (concentrate usable parameter space) | 2 / PARTIAL | No explicit guidance in help corpus | Gap. |

### Controls — smooth variants and animation (3)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 345 | Smooth slider strategy (default to `slider smooth` / `xy smooth`) | 3 / MATCH | `ControlKind` carries the base kinds; variants collapse via first-whitespace-token rule | Engine support; help-page guidance gap. |
| 346 | Timing parameter (`PARAMS` between 0.05-0.2) | 3 / MATCH | `PARAMS` field available | Engine support; help-page guidance gap. |
| 347 | Bang smooth utility (intuitive VJing entry points) | 3 / MATCH | `bang smooth` variant collapses to `ControlKind::Bang` | Engine support; help-page guidance gap. |

### Controls — initial state design (2)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 348 | Default value curation (immediate engagement) | 3 / MATCH | `ControlDecl::default` field with shape-discriminated untagged enum | Engine support; help-page guidance gap. |
| 349 | Balanced starting point (compelling visuals + exploration room) | 2 / PARTIAL | No explicit guidance in help corpus | Gap. |

### GLSL and float comparison (2)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 350 | Threshold testing (`> 0.5` instead of `== 1.0`) | 1 / GAP | No explicit help-page coverage of CPU-GPU float precision pitfall | High-leverage docs gap. |
| 351 | Precision awareness (conversion artifacts → 0.9999937) | 1 / GAP | No explicit help-page coverage | High-leverage docs gap. |

### Learning and experimentation (3)

| # | Synesthesia recommendation | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------- | :---: | :----------------------- | :---- |
| 352 | Iterative testing (modify-and-observe) | 4 / EXCEEDS | Live-reload semantics per `docs-corpus/docs/scene-workflow.md` "Live-reload policy by file type" with 500 ms p95 edit-to-paint budget per `docs/37-authoring-environment-spec.md` §3.3 | Numeric latency target exceeds Synesthesia's qualitative guidance. |
| 353 | Operation substitution (swap math operators) | 5 / EXCEEDS+ | Live-reload + UX-latency harness at `engine/authoring/src/ux_latency.rs` (`EV-AUTHORING-UX-LATENCY` + `EV-DOCS-UX-LATENCY`) | Same dev loop with measured + pinned latency. |
| 354 | Ablation analysis (delete code to identify functional components) | 4 / EXCEEDS | Coerce-severity diagnostics per `docs/28` CP-01 surface every machine-applied mutation; compile errors mapped through `engine/authoring/src/diagnostics.rs` | Diagnostic feedback loop exceeds qualitative guidance. |

---

## Cross-cutting observations — patterns the feature-level density surfaces

Patterns that recur across the per-feature scoring beyond what the
section-level overview already called out:

1. **The audio-uniforms surface is uniformly EXCEEDS+ at the
   family-anchor symbols and MATCH at the per-band variants.** Every
   family's anchor symbol (`syn_Level`, `syn_Hits`, `syn_Time`,
   `syn_Presence`, `syn_OnBeat`, `syn_BPM`, `syn_FadeInOut`,
   `syn_Intensity`) scores EXCEEDS+ because it sits at a T1
   drift-pin invariant in `engine/audio/src/uniforms.rs`; the
   per-band variants score MATCH because they inherit the family
   tolerance but don't have their own per-symbol pin. This is the
   shape of Chromaticity's "drift tolerance families" — the per-band
   variants don't need individual pins because they're enforced by
   the family-level invariant. The pattern is invisible at the
   section-level overview.

2. **Lygia parity widens silently beyond Synesthesia's documented
   surface.** Synesthesia's external-libraries page lists 14 Lygia
   category names; Chromaticity vendors v1.1.4 with every category
   present AND directory listings reveal individual function counts
   that exceed Synesthesia's per-category prose. The
   `generative/` category alone ships 14 noise-family helpers
   (cnoise, curl, fbm, gerstnerWave, gnoise, noised, pnoise,
   psrdnoise, random, snoise, srandom, voronoi, voronoise, worley)
   where Synesthesia documents only "random(), noise()". The
   vendored corpus is broader than the documented surface even at
   MATCH-level scoring.

3. **The script-host scoring concentrates the deferred work.** Of
   the 8 GAP/PARTIAL rows in this deep dive, 6 sit in the
   JavaScript scripting surface (`onChange`, `onOffToOn`,
   `onOnToOff`, `whileOn`, `whileOff`, `printGlobalKeys` —
   plus the four `randomizeControl` / `defaultControl` /
   `randomizeGroup` / `defaultGroup` PARTIALs). All 6 GAPs trace
   to the slice 5.4+ per-handler scene-author binding wave that
   has not landed yet. This is the single tightest cluster of
   deferred work in the entire SSF parity matrix.

4. **Help-corpus authoring is the residual best-practices gap and
   it's a single-PR shape.** The 11 PARTIAL rows in best-practices
   all trace to the same root cause: the engine-side feature
   exists (the `UI_GROUP` field, the `IS_META` flag, the `PARAMS`
   shape, the `_Reactivity` meta-gate) but the help-corpus prose
   that explains "use it like this" has not been authored. A
   single help-corpus PR adding a "controls best practices" +
   "common pitfalls" + "GLSL performance" page set would lift 11
   PARTIAL rows + 2 GAP rows (rows 350/351 on float precision) to
   MATCH or better. This is the highest-leverage docs-side work
   available pre-V1.

5. **hg_sdf vendoring is a 64-row uniform-PARTIAL block.** Rows
   181-247 (1 include syntax + 5 helpers + 28 primitives + 14
   domain ops + 17 combinators + 1 carve-out) all sit at the same
   PARTIAL score for the same reason: hg_sdf's MIT license is
   elected on `docs/29-cleanroom-legal-policy-baseline.md` but the
   actual snapshot has not landed under `third_party/`. A single
   vendoring PR + `deny.toml` carveout for MIT would lift all 64
   rows from PARTIAL to MATCH (and some to EXCEEDS via the
   same cycle/depth guard + vendor-root sandbox the Lygia
   preprocessor already provides). Several of the most common
   primitives already have Lygia analogues vendored (sphere, box,
   cylinder, capsule, circle, cone, dodecahedron, hex) so the
   functional-coverage gap is narrower than the row count
   suggests.

6. **The script-host typed reactive-symbol catalog scores EXCEEDS+
   on the global-access surface (row 324) and uniformly outscores
   Synesthesia's documented surface on engine guarantees.** Where
   Synesthesia documents "input uniforms available as global
   variables", Chromaticity ships a 46-variant typed enum with
   round-trip integrity (`name() == s` for every variant),
   per-symbol dimensionality slots (`Scalar` / `Vec2` / `Vec3` /
   `Vec4`), structural carrier validation with categorised
   mismatch shapes, and T1 drift pins asserting the catalog at
   every `cargo test`. The runtime guarantees that Synesthesia
   leaves implicit are explicitly contract-shaped in Chromaticity.

7. **The standard-uniforms surface has Chromaticity-only namespace
   reservations that have no Synesthesia counterpart.** The
   reserved `chroma_Data_*` namespace (row 79) carries 5
   sub-shapes (`_Scalar_<name>`, `_Vec_<name>`, `_Array_<name>`,
   `_Field_<name>`, `_Event_<name>`); the meta-control underscored
   namespace (row 80) carries 7 cores + media/geometry/dichrome
   meta + preference-backed meta + the `_Reactivity` /
   `_Audio_Speed` differentiation-edition gates. Both reservations
   are forward-compat scaffolding for the post-MVP data-ingestion
   differentiator — they're parsed today and populated later. This
   is one of the structural axes where Chromaticity's clean-room
   discipline produces surface area that the section-level
   overview compresses into a single "EXCEEDS" bullet.
