<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> -->

# Synesthesia parity — SSF Format

_Last refreshed:_ 2026-05-22

Match-or-exceed comparison of Chromaticity against the Synesthesia public
"SSF Documentation" surface (the seven pages under
`https://app.synesthesia.live/docs/ssf/`: `ssf.html`,
`audio_uniforms.html`, `standard_uniforms.html`, `functions.html`,
`libraries.html`, `json.html`, `script.html`, `best_practices.html`).
Clean-room observation-only per
`docs/29-cleanroom-legal-policy-baseline.md`. Rubric and per-item
template from `docs/comparison/README.md`.

This sweep scores Chromaticity's `chroma_*` native surface, the `syn_*`
compatibility surface (where one exists), and the supporting scene
package / preprocessor / scripting / authoring crates against the
Synesthesia public documentation. The aim is not API cloning — it is
to ensure every capability Synesthesia documents has a Chromaticity
equivalent at parity or better.

## TL;DR scoreboard

_Scores grade **spec parity** (format / feature coverage against the
documented surface, observation-only) — not live-runtime parity (what is
wired into the shipped render path)._

| Surface | Score | One-line verdict |
| :------ | :---: | :--------------- |
| SSF package format (extension, layout, components) | 4 / EXCEEDS | Native `.chroma` + `.chromaPack` zip + frozen `scene.json` shape + parse-reserved `DATA_SOURCES` differentiator |
| Audio uniforms (level / hits / time / presence / beat / BPM / large / spectrum / level-trail) | 5 / EXCEEDS+ | Full 33-symbol parity catalog with normative tolerance families + 3 net-new `chroma_*` raw-analysis primitives + readiness-flag cross-seam |
| Standard uniforms (timing / pixel coords / mouse / multipass / media) | 4 / EXCEEDS | Every documented symbol carried at parity; `TIMEDELTA` net-new; `syn_UserImage` pre-transform sampler net-new; reserved `chroma_Data_*` post-MVP namespace |
| GLSL helper functions (math / noise / coordinates / color / media / multipass) | 3 / MATCH | Full clean-room shim catalog with parity signatures; meta-pipeline helpers exceed |
| External libraries (Lygia, hg_sdf) | 3 / MATCH | Lygia v1.1.4 vendored with namespace-prefixed `#include` preprocessor + cycle/depth guard; hg_sdf license-elected but not yet vendored |
| JSON configuration (controls, passes, transitions, media) | 4 / EXCEEDS | Seven control kinds + slice-3a frozen wire format + flatten-preserved extension keys + canonical-model normalisation |
| JavaScript scripting (runtime, hooks, host API) | 3 / MATCH | QuickJS deterministic sandbox + watchdog + memory cap + seeded PRNG + frame-locked clocks + typed reactive-symbol catalog; lifecycle hooks parity but full host-API parity matrix still in progress |
| Best practices (audio reactivity, controls, misc) | 2 / PARTIAL | In-app help corpus covers audio reactivity + scene workflow; explicit "controls best practices" + "misc tips" pages not yet authored |

## Per-item comparison

### SSF package format and extension

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `.chroma` directory + `.chromaPack` zip
  transport. Loader entry point `chromaticity_scene::loader::load_chroma_dir`
  at `/Users/xaviermac/Documents/2_Areas/Coding-Projects/chromaticity-dev/engine/scene/src/loader.rs`. Wire-format frozen at slice-3a per
  `docs/02-synesthesia-contracts.md` §Observed root keys and
  `docs/05-scene-system.md` §Canonical Internal Scene Model. Help corpus
  page `docs-corpus/docs/scene-workflow.md` documents the package layout.
- **Match notes:** Equivalent directory-package shape (a directory with
  `scene.json` + `main.glsl` + optional `script.js` + optional `images/`
  + optional `glsl/` shared includes). Same conceptual lifecycle (parse
  → validate → preprocess → compile → init script → bind resources).
  Optional thumbnail role covered by `IMAGE_PATH` preservation in the
  flatten-extensions map.
- **Exceed notes:**
  - Two on-disk forms: directory `.chroma` AND `.chromaPack` zip transport
    (Synesthesia documents the directory form only).
  - **Strict-vs-compat loader modes** (`engine/scene/src/loader.rs`):
    strict mode hard-fails on contract violations for new authoring;
    compat mode accepts legacy / observed-outlier shapes and emits
    `coerce`-severity diagnostics. Synesthesia does not document a
    contract-strictness gradient.
  - Reserved post-MVP `DATA_SOURCES` stanza (parse-reserved at slice-3a
    per `docs/28-implementation-contract-pack.md` CP-12) for external
    real-time data ingestion per `docs/41-data-ingestion-differentiator.md`.
    No equivalent in Synesthesia.
  - Flatten-absorbed `extensions: serde_json::Map<...>` on every schema
    type — unknown / future top-level keys survive round-trip without
    loss. Forward-compat by construction.
- **Gap notes:** `script.js` + `main.glsl` runtime wiring is staged
  across slices (slice-3a deserialises the package shape; script-host
  binding lives in the `engine/script` slice 5 corpus). The
  Synesthesia-documented bundled "thumbnail.png" is captured only via
  metadata `IMAGE_PATH` today; no thumbnail-render contract pinned at
  the scene loader.

### Audio uniforms

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** Reactive symbol catalog v1.3.0 at
  `docs/36-reactive-symbol-catalog.md` §2 (audio uniforms). Producer
  at `engine/audio/src/uniforms.rs` (`extract_uniform_snapshot`,
  `AudioUniformSnapshot`). Render-side adapter at
  `engine/render/src/audio_adapter.rs` (canonical `UNIFORM_NAME_BPM` /
  `UNIFORM_NAME_RMS` / `UNIFORM_NAME_PEAK` / `UNIFORM_NAME_FFT`
  constants). Help page `docs-corpus/docs/chroma-uniforms.md` +
  `docs-corpus/docs/audio-reactivity.md`. ADR-019 governs the IPC
  bridge.
- **Match notes:** Every Synesthesia-documented audio symbol is present
  in the catalog at parity semantics:

  | Synesthesia symbol | Catalog disposition | Notes |
  | :----------------- | :------------------ | :---- |
  | `syn_Spectrum` (sampler1D, RGBA-packed) | §2.1 — Profile C | Same 4-channel pack (Raw / Juiced / Smooth / Waveform) |
  | `syn_LevelTrail` (sampler1D, RGBA-packed) | §2.1 — Profile C | Same 4-channel pack (Whole / Bass / Mid / High) |
  | `syn_Level`, `syn_BassLevel`, `syn_MidLevel`, `syn_MidHighLevel`, `syn_HighLevel` | §2.2 — Profile C | Family tolerance `LVL` |
  | `syn_Hits`, `syn_BassHits`, `syn_MidHits`, `syn_MidHighHits`, `syn_HighHits` | §2.3 — Profile C | Family tolerance `HIT` |
  | `syn_Time`, `syn_BassTime`, `syn_MidTime`, `syn_MidHighTime`, `syn_HighTime`, `syn_CurvedTime` | §2.4 — Profile C | Family tolerance `TIM` |
  | `syn_Presence`, `syn_BassPresence`, `syn_MidPresence`, `syn_MidHighPresence`, `syn_HighPresence` | §2.5 — Profile C | Family tolerance `PRS` |
  | `syn_OnBeat`, `syn_ToggleOnBeat`, `syn_RandomOnBeat`, `syn_BeatTime` | §2.6 — Profile C | Family tolerance `BEAT` |
  | `syn_BPM`, `syn_BPMConfidence`, `syn_BPMTwitcher`, `syn_BPMSin{,2,4}`, `syn_BPMTri{,2,4}` | §2.7 — Profile C | Family tolerance `BPM`; `syn_BPM` clamped `[50, 220]` with T1 drift pin |
  | `syn_FadeInOut`, `syn_Intensity` | §2.8 — Profile C | Family tolerance `LRG` |

- **Exceed notes:**
  - **Three net-new `chroma_*` raw-analysis primitives** (§2.11 of the
    catalog, slice-5a in `engine/render/src/audio_adapter.rs`):
    `chroma_AudioRMS` (analysis-window RMS), `chroma_AudioPeak`
    (analysis-window peak), and `chroma_AudioFFT[8]` (eight log-spaced
    FFT bands, 20 Hz – 20 kHz, std140-packed `float[8]` binding).
    Synesthesia's documented surface gives smoothed / normalised
    band-level family symbols but does not document raw analysis-window
    primitives at this abstraction level.
  - **Readiness cross-seam.** `AudioAxisAvailability::{Warming, Ready}`
    plus a `degraded: bool` flag on `AudioUniformSnapshot` — the
    producer signals when each axis (BPM, FFT) has filled enough
    rolling window to read as a real estimate vs warmup-fallback. This
    surfaces cross-seam to the UI as the AUDIO adapter pill in the
    Perform top bar. No equivalent contract in the public Synesthesia
    docs.
  - **Heuristic differentiator.** `chroma_PrimeBang` (with
    `syn_PrimeBang` compat alias) — Profile D differentiator covering
    macro-musical-transition pulse heuristic per catalog §2.9.
  - **Reference-surface uniforms beyond the documented catalog** —
    `syn_InputLevel`, `syn_RawLevel`, `syn_BassExpoCurve`, `syn_Threshold`,
    `syn_PrimeAndBang` (§2.10) shipped at Profile C with formulae
    locked in `docs/06-audio-engine.md` §7.12. Synesthesia ships these
    in `SynesthesiaEngine.framework` but does not document them on the
    public surface; Chromaticity normalises them.
  - **Normative drift tolerance families** mapped 1:1 to performance
    budgets in `docs/10-performance-latency-budgets.md` (`LVL`, `HIT`,
    `TIM`, `PRS`, `BEAT`, `BPM`, `LRG`). T1 drift pins in
    `engine/audio/src/uniforms.rs::uniforms_drift_pins` assert RMS,
    peak, band-edge, BPM-clamp invariants on every `cargo test`.
  - **Producer-determinism.** `extract_uniform_snapshot` is pure: same
    input frame, same output snapshot. Rolling history is a separate
    stateful helper (`AudioAnalysisWindow`) so single-frame callers
    cannot accidentally inherit residual state. Documented for clean
    EV-AUDIO-TRACE evidence anchoring.
  - **Operator-controllable noise-floor gate.** `syn_Threshold`
    exposed through the Audio settings panel + `Shift+T` (sample
    current 200 ms RMS) + OSC `/audio/threshold {float}` per
    `docs-corpus/docs/audio-reactivity.md`.
- **Gap notes:** `chroma_AudioFFT[8]` is producer-only on the
  script-write side — the `ReactiveValueKind` enum has no
  `FloatArray<N>` variant yet, so a script attempting
  `__chromaticityEmit("chroma_AudioFFT", …)` rejects with
  `ScriptError::ReactiveSymbolUnknown`. Tracked alongside Phase-3+
  deferred axes; closing requires a typed-carrier extension. Read
  access from script is unaffected.

### Standard uniforms

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** Reactive symbol catalog
  `docs/36-reactive-symbol-catalog.md` §1.1 (timing) / §1.2 (pixel
  coords) / §1.3 (mouse) / §1.4 (multipass) / §1.5 (media). Catalog §3
  covers the meta-control underscored namespace not documented in
  Synesthesia's standard-uniforms page (`_Master`, `_Brightness`,
  `_Gamma`, `_Contrast`, `_Hue`, `_Saturation`, `_Invert`, plus media
  meta, geometry meta, dichrome meta, preference-backed meta, and the
  `_Reactivity` / `_Audio_Speed` differentiation-edition gates).
- **Match notes:** Every Synesthesia-documented standard uniform is
  carried at parity semantics:

  | Synesthesia symbol | Catalog disposition | Notes |
  | :----------------- | :------------------ | :---- |
  | `RENDERSIZE` (`vec2`) | §1.4 — Profile C, per-pass | Pixel size of current pass, not scene |
  | `TIME` (`float`) | §1.1 — Profile C | Seconds since scene load, monotonic |
  | `FRAMECOUNT` (`float`) | §1.1 — Profile C | Frame counter as float (matches reference) |
  | `_xy` (`vec2`) | §1.2 — Profile C | `gl_FragCoord.xy` |
  | `_uv` (`vec2`) | §1.2 — Profile C | Normalised `[0,1]` |
  | `_uvc` (`vec2`) | §1.2 — Profile C | Aspect-corrected |
  | `PI` (`const float`) | §1.1 — Profile C | `3.14159265358979323846` |
  | `_mouse` (`vec4`) | §1.3 — Profile C | Shadertoy-compatible |
  | `_click` (`vec3`) | §1.3 — Profile C | Left / right / middle |
  | `_muv` (`vec2`) | §1.3 — Profile C | Normalised mouse |
  | `_muvc` (`vec2`) | §1.3 — Profile C | Aspect-corrected mouse |
  | `PASSINDEX` (`int`) | §1.4 — Profile C | Pass index, starts 0 |
  | `syn_FinalPass` (`sampler2D`) | §1.4 — Profile C | Previous frame for feedback |
  | `syn_Media` (`sampler2D`) | §1.5 — Profile C | After meta-media transforms |
  | `syn_MediaType` (`float`) | §1.5 — Profile C | `_isMediaActive()` preferred |

- **Exceed notes:**
  - **`TIMEDELTA` (`float`)** added under §1.1 — seconds since previous
    frame; native-chroma-v1 addition, also exposed under compat-syn-v1
    for forward compat. Not in the Synesthesia public docs.
  - **`syn_UserImage` (`sampler2D`)** added under §1.5 — raw media
    sampler before any meta-media transforms (Chromaticity's meta-media
    pipeline is documented; `syn_Media` is the post-transform output
    and `syn_UserImage` is the pre-transform handle). Catalog §3.2
    fully specifies the meta-pipeline ordering.
  - **Reserved `chroma_Data_*` namespace** (catalog §1.6) for post-MVP
    external-data ingestion: `chroma_Data_Scalar_<name>`,
    `chroma_Data_Vec_<name>`, `chroma_Data_Array_<name>` (`sampler1D`),
    `chroma_Data_Field_<name>` (`sampler2D` / `sampler3D`),
    `chroma_Data_Event_<name>` (ring buffer + count scalar). Parsed at
    MVP, populated post-MVP via the `DATA_SOURCES` stanza. No
    equivalent surface in Synesthesia.
  - **Strict-reject on typos.** Reading an undeclared audio /
    data-source uniform is a strict compile error (canonical) or
    compile-to-`0.0` warning (compat) per catalog §6 coercion classes.
    Synesthesia's documented behaviour is "the uniform is provided"
    with no documented rejection path for typo'd names.
- **Gap notes:** None at the symbol surface. Live GPU rendering of
  these uniforms is gated on ADR-011 Phase-2+ — at MVP the canvas-2d
  procedural preview pipeline reads control schema and a subset of
  named axes (`speed`, `intensity`, `tint`, `rotation_speed`,
  `warp_amount`) per Wave 2.3+ widening but the loaded GLSL is not
  rasterised. This is a backend gap, not a symbol-surface gap.

### GLSL helper functions

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Catalog §4 "GLSL Helper Shim Catalog" in
  `docs/36-reactive-symbol-catalog.md`. Clean-room helper shim corpus
  with parity signatures. Phase-5 `EV-LICENSE-AUDIT` plus
  `docs/56-shader-sourcing-research.md` covers vendoring vs
  hand-authored sourcing.
- **Match notes:** Every documented Synesthesia helper has a catalog
  entry with the same signature:

  | Family | Helpers covered | Catalog section |
  | :----- | :-------------- | :-------------- |
  | Math | `_scale`, `_map`, `_smin`, `_rand`, `_pulse`, `_sqPulse`, `_inRange`, `_triWave`, `_pixelate`, `_mix3` / `_mix4` / `_mix5`, `_nsin`, `_ncos`, `_nclamp` | §4.1 |
  | Noise | `_noise` (vec2 / vec3), `_fbm` (vec2 / vec3), `_statelessContinuousChaotic`, `_hash11` / `_hash12` / `_hash13` / `_hash21` / `_hash22` / `_hash23` / `_hash31` / `_hash32` / `_hash33` | §4.2 |
  | Coordinates | `_uv2uvc`, `_uvc2uv`, `_rotate` (vec2 + vec3-axis overload), `_mirror`, `_toPolar`, `_toPolarTrue`, `_toRect` | §4.3 |
  | Color | `_rgb2hsv`, `_hsv2rgb`, `_normalizeRGB`, `_palette` (Inigo-Quilez), `_brightness`, `_gamma`, `_contrast`, `_saturation`, `_hueRotate`, `_invert`, `_luminance`, `_grayscale`, `_dichrome` | §4.4 |
  | Media | `_loadMedia`, `_textureMedia`, `_loadMediaAsMask`, `_textureMediaAsMask`, `_correctMediaCoords`, `_isMediaActive`, `_edgeDetectMedia`, `_edgeDetectSobelMedia` | §4.5 |
  | Multipass | `_edgeDetect`, `_edgeDetectSobel`, `_hBlurTiltShift`, `_vBlurTiltShift` | §4.6 |

- **Exceed notes:**
  - **Meta-pipeline helpers** (§4.8) prepended to every compiled scene:
    `_applyMaster`, `_applyBrightness`, `_applyGamma`, `_applyContrast`,
    `_applyMediaContrast`, `_applyMediaGamma`, `_applyMediaHueSaturation`,
    `_applyHueSaturation`, `_applyInvert`, `_applyInvertMedia`,
    `_applyColorCorrection`, `_applyDichrome`, `_applyVerticalMirroring`,
    `_applyHorizontalMirroring`, `_zoomOrFit`, `_applyAlpha`,
    `_applyMediaScale`, `_correctImageCoords`, `_flipMediaCoords`,
    `_imageToMask`, `_hueSaturation`. The Synesthesia docs page does not
    enumerate a meta-pipeline helper layer of this scope.
  - **Convenience alias `_textureImage`** (§4.7) covers a
    reference-observed undocumented helper.
  - **Deprecated compatibility aliases** (§4.9) shipped under
    `compat-syn-v1` only: `_applyAllLuminanceCorrections`,
    `_hueSaturationLEGACY`, `_hueSaturationContrast`,
    `_hueSaturationContrastLEGACY`, `_applyHueSaturationContrast`,
    `_invertImage`, `_loadUserImage`, `_textureUserImage`,
    `_loadUserImageAsMask`, `_textureUserImageAsMask`. Reference-scene
    portability beyond the public docs surface.
  - **Coercion-class taxonomy** for helper arity mismatches: strict
    compile error; compat-mode attempts the reference-equivalent
    overload when unambiguous (catalog §6).
- **Gap notes:** The helper implementations themselves are catalog
  declarations; the bound GLSL source must ship as part of the
  meta-pipeline preamble at scene-compile time. The render pipeline
  for live GPU execution is ADR-011 Phase-2+ deferred. Helper
  signatures are pinned; the shipped GLSL bodies await the live render
  wave. The canvas-2d procedural preview adapter does not call into
  these helpers (it's a separate visual-stub layer).

### External libraries (Lygia, hg_sdf)

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Lygia v1.1.4 vendored at
  `third_party/lygia/` (commit `28f220b6045cbbd38ad6e23a69d21c5e67070cdd`).
  GLSL preprocessor at `engine/render/src/glsl_includes.rs`
  (`resolve_includes`, `MAX_INCLUDE_DEPTH = 32`,
  `IncludeError::DepthExceeded` cycle guard). Research catalog at
  `docs/56-shader-sourcing-research.md` covers 13 candidate upstream
  sources against the `deny.toml` allowlist. Clean-room policy
  baseline at `docs/29-cleanroom-legal-policy-baseline.md` §Bundled
  GLSL Library Handling.
- **Match notes:** Same `#include "lygia/<path>.glsl"` syntax. Same
  pinned version (v1.1.4). Recursive expansion supported so a Lygia
  file that itself includes another Lygia file resolves both. Each
  resolved path is canonicalised and asserted to lie under
  `<vendor_root>` so a `..`-chain cannot escape the trial-vendor
  sandbox.
- **Exceed notes:**
  - **Depth + cycle bound.** `MAX_INCLUDE_DEPTH = 32` cycle guard plus
    direct self-include detection. Synesthesia's docs do not specify a
    cycle/depth policy.
  - **Vendor-root sandbox.** Path-traversal hardening on the resolved
    canonical path (sandbox-escape rejection). Defence-in-depth beyond
    "include and hope".
  - **Comment-passthrough discipline.** Inlined source forwarded
    byte-for-byte; SPDX header comments stay in the inlined output so
    naga's GLSL frontend tolerates them as ordinary line comments;
    stripping would be a clean-room concern.
  - **Trial-vs-elect license posture** documented end-to-end:
    `LicenseRef-Prosperity-3.0.0` is permitted ONLY for
    `third_party/lygia/**` per the `deny.toml` carveout; final
    commercial-use election is gated by Phase-5 `EV-LICENSE-AUDIT` per
    `docs/29` row. If election goes against trial vendoring, the
    preprocessor stays as a generic include-resolver and the helper
    catalog covers parity through the §4 clean-room shims.
  - **Research catalog over 13 candidate corpora** with per-source
    licence, vendoring shape, integration cost, and recommendation
    (Lygia, glslify, ShaderToy, gl-noise, hg_sdf, ARMTOO, Inigo
    Quilez articles, Patrick Cozzi’s WebGL Workshop, Three.js
    examples, GLM, ImGui-shaders, Smoothlife, OpenGL Mathematics
    book examples). The Synesthesia docs document Lygia + hg_sdf only.
- **Gap notes:** **hg_sdf is not yet vendored** in `third_party/`.
  Synesthesia ships both Lygia and hg_sdf; Chromaticity has the
  vendoring shape ready (`docs/29` row carries the MIT election as the
  required clean-room selection) but the actual snapshot has not
  landed. Tracked for clean-room vendoring under `EV-LICENSE-AUDIT`.
  User-custom GLSL libraries (Synesthesia's "personal `glsl/` folder
  in user data") map onto our per-scene `glsl/*.glsl` shared-include
  files but a global per-user library root is not yet authored.

### JSON configuration (`scene.json`)

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `engine/scene/src/schema.rs`
  (`Scene`, `SceneHeader`, `ControlDecl`, `PassSpec`, `UniformBinding`,
  `ControlKind`, `ControlValue`). Loader at
  `engine/scene/src/loader.rs`. Slice-3a wire format frozen against
  `docs/02-synesthesia-contracts.md` §Observed root keys + §Observed
  control keys + §2 Multipass. `docs/28-implementation-contract-pack.md`
  §CP-12 covers the parse-reserve flatten-extensions rule. Coercion
  classes in CP-01 (CR-01 through CR-04).
- **Match notes:** Equivalent top-level wire keys round-trip through
  serde:

  | Synesthesia key | Chromaticity disposition |
  | :-------------- | :----------------------- |
  | `TITLE`, `CREDIT`, `DESCRIPTION`, `IMAGE_PATH`, `WIDTH`, `HEIGHT` | Flatten-preserved on `Scene::extensions` |
  | `CONTROLS` | Typed `Vec<ControlDecl>` on `Scene::controls` |
  | `PASSES` | Typed `Vec<PassSpec>` on `Scene::passes` |
  | `HARD_TRANSITIONS`, `SMOOTH_TRANSITIONS` | Flatten-preserved (structured parsing later-slice) |
  | `IMAGES`, `MEDIA` | Flatten-preserved with structured parsing post-MVP |
  | `TAGS`, `GPU` | Flatten-preserved; observed-reference compat |

  All seven Synesthesia control kinds map to `ControlKind` variants
  (`slider`, `toggle`, `color`, `xy`, `bang`, `dropdown`, `knob`).
  Per-control fields `NAME`, `TYPE`, `DEFAULT`, `MIN`, `MAX`,
  `UI_GROUP`, `DESCRIPTION`, `PARAMS`, `DEF_COLOR`, `LABELS`, `VALUES`
  all carried.
- **Exceed notes:**
  - **Flatten-preserved unknown-key handling.** Every schema type
    carries an `extensions: serde_json::Map<String, Value>` map via
    `#[serde(flatten)]` so unknown / future / observed-outlier keys
    round-trip through deserialise → re-serialise without loss. The
    `SCHEMA_VERSION` field (chromaticity-reserved, absent from
    reference scenes) defaults to `1` (compat-syn-v1 baseline) for
    forward-compat versioning.
  - **`TYPE`-token compat parsing.** The custom `ControlKind`
    `Deserialize` impl applies the "first ASCII-whitespace-split
    token" rule so reference-observed multi-word spellings like
    `"slider smooth"`, `"xy traveler"`, `"bang counter smooth"`
    deserialise to the canonical base kind. Unknown first tokens
    produce a typed `serde::de::Error::custom("unknown-control-type:
    <raw>")` rather than a silent fallthrough.
  - **`ControlValue` shape-discriminated untagged enum** —
    `Bool`-before-`Float` ordering pinned so `true` / `false`
    deserialises as `Bool` (not a numeric 0/1 coercion), and
    `Vec2` / `Vec3` / `Vec4` discriminate by JSON array length.
  - **CP-01 coercion classes** (CR-01 through CR-04) per
    `docs/28-implementation-contract-pack.md`: case-mismatch on
    control `NAME` → strict reject, compat warn-and-coerce; missing
    `TYPE` → coerce-by-`DEFAULT`-shape fallback (`slider` / `toggle`
    / `color` / `xy`); arity errors classified at validation time.
  - **Multipass pass-graph** with per-pass `WIDTH`, `HEIGHT`, `FLOAT`,
    `WRAP`, `FILTER` flatten-preserved at slice-3a; structured pass-
    target / precision / feedback-edge interpretation lands in a
    later slice.
  - **Compat-syn-v1 vs native-chroma-v1 axis pair.** Native authoring
    paths default to strict mode and emit `chroma_*` symbols;
    `syn_*` is accepted under the compat surface for reference-scene
    continuity. This bifurcation is normatively spec'd in
    `docs/22-compatibility-versioning-policy.md`.
- **Gap notes:** Structured `HARD_TRANSITIONS` / `SMOOTH_TRANSITIONS`
  parsing is flatten-preserved at slice-3a; the typed shape lands in
  a later slice (per the schema module prose). Structured `MEDIA` /
  `IMAGES` parsing similarly deferred. `DATA_SOURCES` is parse-
  reserved per CP-12. None of these are wire-format gaps — the keys
  round-trip without loss — but full structured ergonomic surfaces
  haven't all landed.

### JavaScript scripting

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `engine/script/` crate with QuickJS
  via `rquickjs` per ADR-012. Public entry point `ScriptRuntime` at
  `engine/script/src/lib.rs` + `runtime.rs`. Sandbox surface across
  five composing modules: `watchdog` (Slice 2 — interrupt-handler
  timeouts), `memory_cap` (Slice 3 — 16 MiB heap cap),
  `determinism` (Slice 4 — seeded `Math.random` + frame-locked
  `Date.now`), `command_bus` (Slice 5.1 — JS-side
  `__chromaticityEmit(name, value)` reactive-write bridge),
  `reactive_symbol` (Slice 5.2 — typed catalog of 53
  catalog-binding variants). Governed by ADR-003 (architectural
  sandbox) + ADR-012 (engine choice + budgets).
- **Match notes:** QuickJS-based deterministic interpreter parity
  with Synesthesia's documented lifecycle hooks `setup()` and
  `update(dt)`. Reactive-symbol reads exposed to JS as globals
  matching the catalog name (the documented Synesthesia behaviour).
  Vector values exposed as JS objects with `{x, y}` /
  `{x, y, z}` / `{x, y, z, w}` (and `r/g/b` / `r/g/b/a` aliases).
  Control globals named exactly per the scene control `NAME`.
  Deprecated `inputs[name]` / `uniforms[name]` proxies provided
  under `compat-syn-v1` per catalog §5.
- **Exceed notes:**
  - **Engine pinned and justified.** ADR-012 pins QuickJS
    (interpreter-only, no JIT, ~600 KB, BSD-2-Clause) precisely so
    `EV-DETERMINISM` and `EV-SCRIPT-CONTRACT` can assert bit-
    identical state hashes across runs and platforms. Synesthesia's
    public docs do not specify the engine.
  - **Per-frame budgets** pinned numerically: 1.75 ms p99 CPU target
    for `update(dt)`, 4 ms hard watchdog, 500 ms hard timeout for
    `setup()`, 250 ms for `transition()`, 0.5 ms per event-handler
    call. Synesthesia documents no budgets.
  - **Memory cap** at 16 MiB heap per scene via
    `JS_SetMemoryLimit`; 256 KiB stack; heap-snapshot diagnostic at
    ≥80% used.
  - **Watchdog escalation policy** documented in ADR-012 §4:
    timeout on `update(dt)` preempts the script and continues the
    frame with last-known uniform state; 2 consecutive timeouts
    mark the scene `Degraded`; 3 consecutive timeouts trigger a
    CP-03 `PrepareNext` / `PrimeNext` rebuild. Synesthesia does
    not document the timeout / recovery contract.
  - **Determinism shims.** Seeded `Math.random()` via xoroshiro128+
    (seed expanded from caller-supplied `u64` via splitmix64) and
    frame-locked `Date.now()` via a caller-injected `Clock` trait.
    Builder methods `with_random_seed` / `with_clock` compose with
    the watchdog and memory cap on the same runtime. `EV-DETERMINISM`
    opens for the script lane with the recorded-seed replay corpus.
  - **Typed reactive-symbol catalog.** 53-variant `ReactiveSymbol`
    enum at `engine/script/src/reactive_symbol.rs` covers the
    script-host-visible subset of `docs/36` v1.3.0 §1 + §2.
    `ScriptCommand::TypedReactiveWrite { symbol, value }` is the
    typed producer-side variant; carrier-shape validation rejects
    mismatches (`ScriptError::ReactiveValueKindMismatch` /
    `ScriptError::ReactiveSymbolUnknown`).
  - **Sandbox isolation discipline.** Per-scene QuickJS isolate;
    no cross-scene memory; no `fetch` / `XMLHttpRequest` /
    `require` / `import` globals; entitlement mirror is read-only
    behind a frozen proxy. Synesthesia's docs say nothing about
    cross-scene isolation or host-API sandbox guarantees.
- **Gap notes:**
  - **Scene-author hooks (event handlers).** Slice 5.4+ scope —
    `onChange()` / `onOffToOn()` / `onOnToOff()` / `whileOn()` /
    `whileOff()` are documented by Synesthesia but the
    chromaticity scene-author surface bindings haven't all landed
    yet (the producer/consumer/catalog/adapter quad covers the
    cross-crate seam at slice 5.3 EXIT quality; the per-handler
    bindings land in later slices).
  - **`textureFinalPass(x, y)`** helper is in the catalog §5
    spec but the JS-side binding implementation is later-slice
    scope.
  - **`setUniform(name, values)`** producer-side helper is the
    catch-all `ScriptCommand::ReactiveWrite` shape at slice 5.1;
    the typed catalog covers strongly-typed writes from slice 5.2
    onward. Full host-API parity matrix per ADR-012 § 8 (compat
    shims for `inputs[name]` / `uniforms[name]` / `print` /
    `console.log`) is pinned but the slice progression is in
    flight.
  - **Per-scene script `.js` file** binding to the loader is
    later-slice scope (slice-3a deserialises the scene-package
    shape; the script-host wiring lives in `engine/script` slice 5).

### Best practices

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** In-app help corpus pages
  `docs-corpus/docs/audio-reactivity.md` (audio integration
  techniques with a worked GLSL example),
  `docs-corpus/docs/scene-workflow.md` (control / pass / live-reload
  / transition concepts), `docs-corpus/docs/chroma-uniforms.md`
  (uniform surface reference). Authoring environment spec
  `docs/37-authoring-environment-spec.md` covers latency budgets
  + diagnostics + hover-docs.
- **Match notes:** Audio reactivity guidance is covered with a
  worked example (`docs-corpus/docs/audio-reactivity.md` "Worked
  example: a radial pulse shader" section) and rationale for the
  eight-band → four-band reduction. Control workflow + live-reload
  posture covered in `docs-corpus/docs/scene-workflow.md`. The
  `chroma_*` vs `syn_*` namespace governance rule (catalog
  §Namespace governance) is normative: "Avoid coining new `syn_*`
  names for chromaticity-native features."
- **Exceed notes:**
  - **Latency budgets explicit.** §3.3 of `docs/37`: `main.glsl`
    edit-to-paint p95 ≤ 500 ms. UX-latency harness at
    `engine/authoring/src/ux_latency.rs` (`EV-AUTHORING-UX-LATENCY`
    + `EV-DOCS-UX-LATENCY` co-developed). Synesthesia gives no
    measured latency targets.
  - **Hot-reload semantics** per `docs-corpus/docs/scene-workflow.md`
    "Live-reload policy by file type": preserves control values
    across shader edits; rebinds script host on `.js` edits;
    everything else preserved on `images/*` reloads. Synesthesia
    does not document per-file reload semantics.
  - **Coerce-severity diagnostics** for compat-mode loads so
    porters see every machine-applied mutation. Catalog §6
    coercion classes + `docs/28` CP-01 coercion rules.
- **Gap notes:**
  - **Explicit "controls best practices" page** — Synesthesia
    covers organising controls into groups, avoiding non-functional
    sliders, tuning ranges, picking variants, setting thoughtful
    defaults. Chromaticity's catalog §3 + UI port plan §47 cover
    the engine side, but a single help-corpus page synthesising
    operator-facing control-design guidance is not authored.
  - **Explicit "misc tips" / "common pitfalls" page** — Synesthesia
    documents `> 0.5` comparisons instead of equality checks for
    float values, and pointer toward learning from existing scene
    code. Chromaticity could surface analogous guidance in the
    help corpus.
  - **GLSL performance guidance** is not on Synesthesia's best-
    practices page either; both runtimes carry this gap.

## Cross-cutting observations

Patterns that recur across the per-item scoring:

1. **Chromaticity scores higher on determinism / typed schemas /
   contract surface.** Every major surface (audio, standard
   uniforms, scene format, script sandbox) has a normatively spec'd
   contract, drift-tolerance families, T1 in-source drift pins, and
   T2 evidence corpora anchored to the `EV-*` registry in
   `docs/26`. The Synesthesia public docs operate at a lighter
   contract weight — capability descriptions but rarely numeric
   budgets or determinism guarantees. This is structural: a
   live-show runtime that must reproduce a recorded show requires
   the heavier contract.

2. **Chromaticity scores higher on clean-room safety / namespace
   discipline.** The `chroma_*` / `syn_*` bifurcation, the
   reserved-but-unimplemented `chroma_Data_*` namespace, the
   `DATA_SOURCES` parse-reserve, the Lygia vendor-root sandbox, the
   flatten-preserved extensions map — all of these are forward-
   compat scaffolding for a clean-room reimplementation of a
   reference application. Synesthesia, being the reference,
   doesn't need any of this discipline and doesn't document it.

3. **Synesthesia scores higher on scripting-API maturity.** The
   Synesthesia docs enumerate `onChange` / `onOffToOn` /
   `onOnToOff` / `whileOn` / `whileOff` / `randomizeControl` /
   `defaultControl` / `randomizeGroup` / `defaultGroup` /
   `textureFinalPass` / `printGlobalKeys`. Chromaticity's slice
   5.3 ships the producer/consumer/catalog/adapter quad cross-crate
   seam at EXIT quality, but the per-handler scene-author bindings
   are slice 5.4+ scope. Closing this gap is a known phase-3+
   trajectory.

4. **Synesthesia scores higher on corpus depth.** Synesthesia ships
   bundled scenes covering every documented surface, including
   reference scenes for `chroma_PrimeBang` / `syn_PrimeBang`-style
   heuristics. Chromaticity ships 12 clean-room scenes
   (`engine/scene/tests/fixtures/scenes/` + Wave 1d-β `welcome/` +
   `examples/`) plus the operator's local-scene folder
   (`scenes-local/`); the V1 corpus is intentionally narrower than
   reference parity. `EV-LIBRARY-CORPUS-MIN` passed at the V1
   minimum; future widening rides operator demand.

5. **Both runtimes ship a live GPU render pipeline gap at
   Chromaticity's current phase.** Synesthesia's docs presume a
   working GPU render path (their whole product is a GLSL VJ
   instrument). Chromaticity's ADR-011 Phase-2+ live-render
   pipeline is deferred — the canvas-2d procedural preview
   adapter at `src/lib/widgets/scene-renderers.ts` covers the
   operator surface today, with the loaded GLSL parsed and the
   control schema honored but not rasterised. This is the single
   biggest "gap that needs to close to ship V1 at parity," and the
   work is queued behind ADR-011 backend selection. It does not
   reflect on the per-symbol catalog or the per-feature scoring
   above; it is a separate axis.

6. **Help-corpus coverage is the residual MATCH-vs-EXCEEDS gap.**
   The seven engine surfaces are at parity-or-better; the
   in-app help corpus (`docs-corpus/`) covers the engine surfaces
   thoroughly but does not yet ship explicit "best practices /
   controls / common pitfalls / GLSL performance" pages matching
   Synesthesia's best-practices section. This is the single
   highest-leverage docs-side improvement available pre-V1.
