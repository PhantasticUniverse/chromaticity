# Synesthesia Contracts Dossier (Preliminary)

This document captures observed contracts from the packaged Synesthesia app resources and docs. It is treated as the baseline compatibility target for Chromaticity planning.

## Evidence Sources
- Bundled scene resources in `../Contents/Resources/**/*.synScene`.
- Embedded docs in `../Contents/Resources/views/embedded_docs`.
- Runtime keywords from `Synesthesia` executable and `SynesthesiaEngine.framework`.
- Meta control GLSL helpers in `../Contents/Resources/metacontrols.glsl`.

## 1) Scene Package Contract
- Reference-app scene package extension: `.synScene` (directory package). This is a reference-contract observation; Chromaticity's own native extension is `.chroma` with identical directory semantics, per `22-compatibility-versioning-policy.md` Sec.Package Extensions.
- Core expected files:
 - `scene.json` (required)
 - `main.glsl` (required)
 - `script.js` (optional, but heavily used)
 - optional `images/` assets

### Observed root keys in `scene.json`
- Common: `TITLE`, `CREDIT`, `DESCRIPTION`, `IMAGE_PATH`, `WIDTH`, `HEIGHT`, `CONTROLS`, `PASSES`, `HARD_TRANSITIONS`, `SMOOTH_TRANSITIONS`, `IMAGES`, `MEDIA`.
- Additional observed keys: `GPU`, `TAGS`.
- Legacy/outlier keys observed in shipped scenes: `IMAGE`, `CATEGORIES`.
- Chromaticity-reserved root keys (not in reference; parsed at MVP, populated post-MVP):
 - `DATA_SOURCES` - declares external data feeds the scene consumes, producing `syn_Data_*` uniforms per `36-reactive-symbol-catalog.md` Sec.1.6 and `41-data-ingestion-differentiator.md`. Schema locked in `28-implementation-contract-pack.md` CP-12.

### Observed control keys
- Standard: `TYPE`, `NAME`, `DEFAULT`, `MIN`, `MAX`, `UI_GROUP`, `DESCRIPTION`, `PARAMS`.
- Additional observed keys: `DEF_COLOR`, `LABELS`, `VALUES`, `IS_META`.
- Rare legacy key observed: `LABEL`.

### Observed control types (beyond simple docs examples)
- Primary: `slider`, `slider smooth`, `slider speed`, `knob`, `knob smooth`, `knob speed`, `toggle`, `toggle smooth`, `bang`, `bang smooth`, `bang counter`, `bang counter smooth`, `xy`, `xy smooth`, `xy speed`, `color`, `color smooth`, `dropdown`, `dropdown smooth`.
- Compatibility-significant outliers: `slider traveler`, `knob traveler`, `xy traveler`.

## 2) Rendering/Multi-pass Contract
- Scene `renderMain()` executes per pass with `PASSINDEX` incrementing.
- Multipass targets declared in `PASSES` become `sampler2D` uniforms.
- Previous final frame available as `syn_FinalPass` for feedback.
- Pass options include `TARGET`, `WIDTH`, `HEIGHT`, `FLOAT`, `WRAP`, `FILTER`.
- Runtime convention includes media/filter wrapping behavior and aspect correction helpers.

## 3) JavaScript Script Runtime Contract
- Script file `script.js` supports:
 - `setup()` once on scene load.
 - `update(dt)` each frame.
 - `transition()` supported by scene examples/runtime.
- Uniform APIs:
 - `setUniform(name, ...)` with scalar/vector/object overloads.
 - Access globals from scene controls + standard + audio uniforms.
- Control mutation APIs:
 - `setControl`, `setControlNormalized`, `randomizeControl`, `defaultControl`, `randomizeGroup`, `defaultGroup`.
- Event APIs:
 - `onChange`, `onOffToOn`, `onOnToOff`, `whileOn`, `whileOff`.
- Pixel readback helper observed in docs:
 - `textureFinalPass(x, y)`.
- Backward compatibility note:
 - Deprecated but still supported global `inputs` / `uniforms` objects.

## 4) Audio Uniform Contract
- Rich live uniform family with normalized and time-like streams:
 - Levels: `syn_Level`, `syn_BassLevel`, `syn_MidLevel`, `syn_MidHighLevel`, `syn_HighLevel`.
 - Hits: `syn_Hits`, `syn_BassHits`, `syn_MidHits`, `syn_MidHighHits`, `syn_HighHits`.
 - Time clocks: `syn_Time`, band times, `syn_CurvedTime`.
 - Presence: `syn_Presence`, band presence variants.
 - Beat/BPM: `syn_OnBeat`, `syn_ToggleOnBeat`, `syn_RandomOnBeat`, `syn_BPM`, `syn_BPMTwitcher`, BPM waveforms.
 - Large-scale dynamics: `syn_FadeInOut`, `syn_Intensity`.
- Audio data textures include spectrum and historical trails in docs/runtime names.

## 5) Media Contract
- Uniforms include:
 - `syn_Media` and `syn_MediaType`.
- `syn_MediaType` enum behavior:
 - `0` none, `1` image, `2` video, `3` webcam.
- Media sampling helper behavior is coupled to aspect correction and meta controls.

## 6) Meta Controls Contract
- Meta controls are applied via prepended/linked GLSL utility functions.
- Includes color correction, invert, mirror, alpha behavior, media transforms.
- Meta controls affect both scene output and media pipeline behavior.
- Strong implication: Chromaticity must model meta controls as first-class global state, not scene-local ad hoc parameters.

## 7) Integration Surface Contract
- Runtime strings indicate production use of:
 - OSC input/output (including control/state routes and audio variable output).
 - MIDI input/output and mapping workflows.
 - Camera ingestion via OpenCV/video IO pathing.
 - Syphon/Spout-style media input/output.
 - Presets, playlist orchestration, scene launch with preset targeting.
- OSC docs indicate stable route patterns for:
 - Scene-specific and global control addressing.
 - Bank/group actions.
 - Scene launch and preset actions.

## 8) Preliminary Quantitative Snapshot (Observed Bundled Corpus)
- 93 bundled scenes inspected.
- 66 scenes use explicit `PASSES`.
- 51 scenes include `script.js`.
- Average pass count ~1.58, maximum observed 10.
- Control distributions show high reliance on `toggle`, `slider smooth`, `xy smooth`, and scripted control automation.

## 9) Compatibility Implications for Chromaticity
- Loader must support strict canonical schema and lenient compatibility mode.
- Scene contract compatibility includes undocumented and legacy fields/types.
- Script runtime must preserve event semantics and control side effects.
- Audio uniform behavior parity is as important as naming parity.
- Integration layer must expose stable routing semantics, not protocol-only transport.

## 10) Contract Decisions for Implementation
- Traveler control semantics and transition timing are locked by `adr/ADR-008-transition-and-traveler-semantics.md`.
- Legacy/canonical key coexistence precedence and coercion classes are locked in `28-implementation-contract-pack.md`.
- Float pass target compatibility/performance handling follows `22-compatibility-versioning-policy.md` with deterministic coercion logging.
- Any net-new compatibility ambiguity requires ADR + evidence before implementation merge.

## 11) Known Malformed/Legacy Fixture Classes
These classes are observed in shipped materials and must be represented in the compatibility harness:

- **KM-01 Missing control type**
 - Control object is otherwise valid but omits `TYPE`.
 - Compatibility expectation: deterministic fallback type + explicit coercion warning.
- **KM-02 Inverted numeric ranges**
 - Control has `MIN > MAX`.
 - Compatibility expectation: deterministic range correction + explicit coercion warning.
- **KM-03 Mixed legacy + canonical keys**
 - Legacy and modern keys coexist in the same scene contract.
 - Compatibility expectation: documented precedence order + warning when rewrite occurs.
- **KM-04 Deprecated script globals**
 - Scene script uses deprecated `inputs` / `uniforms`.
 - Compatibility expectation: non-breaking shims and deprecation warnings.
- **KM-05 Transition hook heterogeneity**
 - Some scripts define `transition()`, others do not.
 - Compatibility expectation: lifecycle behavior remains stable in both cases.
- **KM-06 Malformed transition declaration**
 - `HARD_TRANSITIONS` or `SMOOTH_TRANSITIONS` entry missing required keys (`UNIFORM`, `VALUES`, `UNIFORMS`, or `DURATION`).
 - Compatibility expectation: skip the malformed entry and warn (per CR-05); strict mode rejects.
- **KM-07 Legacy media sampler alias**
 - Scene shader or script references legacy alias for `syn_Media` / `syn_UserImage`.
 - Compatibility expectation: alias resolved via CR-06; strict mode rejects.

## 12) Contract-Legal Handling Constraints
- Contract extraction from reference artifacts is behavior/spec only.
- No reference source code or documentation text is copied into shipped implementation artifacts.
- Any derived fixture must include provenance metadata and exclusion from release packaging by default.

## 13) `GPU` Field Handling Policy
- The `GPU` key appears in shipped scene contracts but is not formally specified by the reference SSF JSON documentation.
- Chromaticity MVP treatment:
 - preserve the value unchanged into `scene.extensions.GPU`,
 - do not gate load/compile behavior on the value,
 - do not interpret or coerce the value in strict or compatibility mode.
- If future evidence establishes semantic meaning, a new ADR must define behavior, add an evidence artifact, and update `22-compatibility-versioning-policy.md` with a migration entry.

## 14b) Canonical Symbol Catalog
Full enumeration of standard uniforms, audio uniforms, meta controls, and GLSL helper shims is locked in `36-reactive-symbol-catalog.md`. That document is normative; this file contains only the compatibility posture. Any conflict between this document's summary and the catalog resolves in favor of the catalog.

## 15) Transition Declaration Shapes (Normative)

### 15.1 `HARD_TRANSITIONS`
Array of entries:
```json
{
  "UNIFORM": "<uniform_name>",
  "VALUES": [ /* list of numeric or vector values; duplicates permitted for weighting */ ]
}
```
- On each musical transition detected by the runtime, the runtime picks one value uniformly at random from `VALUES` (duplicates weight the distribution) and writes it to `UNIFORM` atomically before the next frame.
- Strict mode requires `UNIFORM` to be a declared control `NAME` or a registered audio/standard/meta uniform. Compat mode warns and creates an auto-declared float uniform.

### 15.2 `SMOOTH_TRANSITIONS`
Array of entries:
```json
{
  "UNIFORMS": ["u_a", "u_b", "u_c"],
  "DURATION": 2.5
}
```
- In the **steady state between transitions**, exactly one uniform in `UNIFORMS` holds `1.0` and the others hold `0.0`.
- On a transition event, a new target is chosen uniformly at random from the remaining members. During the crossfade, **two uniforms are simultaneously non-zero** as their values interpolate linearly: the previous target animates from `1.0` to `0.0` and the new target animates from `0.0` to `1.0` over `DURATION` seconds. The sum of all `UNIFORMS` values remains `1.0` throughout the crossfade.
- After the crossfade completes, the steady-state invariant is restored.
- Minimum `DURATION` is `0.05` s; values below are clamped with a coercion warning.
- Scenes consume the values directly: `col = mix(colA*u_a + colB*u_b, colC*u_c, u_c)`-style patterns are idiomatic.

### 15.3 Precedence
- A uniform cannot appear in both `HARD_TRANSITIONS` and `SMOOTH_TRANSITIONS`. Strict mode rejects; compat mode keeps the first occurrence and warns.

## 16) Pass-Sizing Algebra (Normative)

### 16.1 Variables
- `SceneWidth`, `SceneHeight`: scene intrinsic size from root `WIDTH`/`HEIGHT`. Defaults to `1920x1080` if absent.
- `OutputWidth`, `OutputHeight`: runtime output surface size (fullscreen output, pop-out window, or Syphon/Spout/NDI sender size).
- Per-pass `PassWidth`, `PassHeight` from `PASSES[i].WIDTH` and `PASSES[i].HEIGHT`.

### 16.2 Size resolution rule
For each pass `i`:
1. If `PassWidth == 0` OR `PassHeight == 0` OR either is absent:
 - If pass `TARGET == "backbuff"` (final output target): pass size = `(OutputWidth, OutputHeight)` when "Scale Passes Relatively" preference is off, else scaled by scene-to-output ratio.
 - Otherwise (intermediate pass): pass size = `(SceneWidth, SceneHeight)`.
2. If both are positive: pass size = `(PassWidth, PassHeight)` (absolute).
3. "Scale Passes Relatively" preference scales all absolute sizes by `OutputWidth / SceneWidth` along x and `OutputHeight / SceneHeight` along y; final backbuffer is always the output surface.

### 16.3 Float-target policy
`FLOAT` defaults to `true` (RGBA32F); `false` selects RGBA8. Chromaticity may downgrade `FLOAT: true` to RGBA16F on MVP hardware classes when explicitly declared in `22-compatibility-versioning-policy.md` with deterministic coercion logging (see CP-02). Backend pixel-format availability and transpile posture are locked in `adr/ADR-011-rendering-backend.md` Sec.3.

## 17) Media Sampler Pair (Normative)

- `syn_Media` is the post-meta-media sampler: ingests the user-selected media, applies `_Media_Contrast`, `_Media_Gamma`, `_Media_Hue/Saturation`, `_Invert_Media`, and writes into a dedicated media pass.
- `syn_UserImage` is the pre-meta-media sampler: the raw media source before any color correction. Used by mask helpers (`_loadMediaAsMask`, `_textureMediaAsMask`) because masks must threshold the untouched source.
- `_Media_Scale` is applied in scene-sample space by helper functions (`_loadMedia`, `_textureMedia`), not in the ingest pass. This preserves letterbox/black-bar regions when scaling rather than clipping them to the media framebuffer bounds.
- Strict mode requires scripts and shaders referencing these samplers by the canonical names above. Compat mode accepts legacy names mapped by the alias table in `28-implementation-contract-pack.md` CP-07 and `36-reactive-symbol-catalog.md` Sec.4.9.

## 18) GLSL Include and Library Contract
- Scenes may reference shared GLSL via `#include "..."` directives.
- Resolution order for includes (deterministic):
  1. scene package `glsl/` folder (relative to `main.glsl`),
  2. user-level `glsl/` folder under the runtime user-data root,
  3. bundled library root provided by Chromaticity.
- Unresolved includes cause strict-mode compile failure; compatibility mode emits `coercion-applied` warning only when the include is empty and scene can still compile.
- Bundled libraries:
 - any library shipped with Chromaticity must have a provenance record and license entry per `29-cleanroom-legal-policy-baseline.md`.
 - shipping third-party libraries (e.g. public shader helper libraries) requires `EV-LICENSE-AUDIT` approval before release.
 - reference-adjacent helper names (e.g. specific file names observed in the reference product) are reconstructed clean-room or replaced with original implementations; no verbatim copying.
- Discovery:
 - scene preflight/lint must report unresolved includes and include-chain depth,
 - include graph is part of the scene's compatibility profile metadata.
