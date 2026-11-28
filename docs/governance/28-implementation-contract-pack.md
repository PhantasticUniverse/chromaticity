# Implementation Contract Pack (Baseline)

## Purpose
Provide the minimum contract baseline required to begin implementation without unresolved behavior gaps.

## Scope
- Scene schema normalization precedence and coercion behavior.
- Runtime threading/process model and frame-loop ownership.
- Warm-switch semantics for scene transitions.
- Traveler and transition timing semantics.
- State command/event schema and ordering.
- Integration route compatibility matrix and normalization policy.

## Contract Decisions

### CP-01 Scene Normalization Precedence
- Canonical keys always win when canonical and legacy keys coexist.
- Legacy keys are mapped deterministically and logged with rewrite class IDs.
- Coercion order:
  1. required structure checks,
  2. key precedence rewrite,
  3. type/range coercion,
  4. deprecated API shim mapping,
  5. warning emission.
- Coercion classes:
 - `CR-01` missing control `TYPE` -> see fallback rule below.
 - `CR-02` inverted `MIN/MAX` -> deterministic swap.
 - `CR-03` legacy key coexistence -> canonical precedence.
 - `CR-04` deprecated script globals (`inputs`/`uniforms`) -> compatibility shim.
 - `CR-05` malformed transition declaration -> skip entry and warn (see CP-09).
 - `CR-06` legacy media sampler alias -> alias map to canonical sampler (see CP-11 and `36-reactive-symbol-catalog.md` Sec.4.9). Extended per `36-reactive-symbol-catalog.md` Sec.2.1 to cover audio-sampler per-band spellings (`syn_BassTimeSeries`, `syn_MidTimeSeries`, `syn_HighTimeSeries`, `syn_TimeSeries`, `syn_SpectrumSmooth`, `syn_SpectrumJuiced`, `syn_Waveform`) -> alias to the corresponding RGBA channel of `syn_LevelTrail` or `syn_Spectrum`.
 - `CR-07` external data source unavailable -> zero-valued publish of declared `syn_Data_*` symbols with `data-ingest-deferred` (MVP) or `data-source-unavailable` (post-MVP) diagnostic (see CP-12).

#### CR-01 fallback rule (normative)
Observed in shipped reference corpus: 15 control objects across 8+ release scenes omit `TYPE`. The fallback decision must be deterministic per `02-synesthesia-contracts.md` Sec.11 KM-01.

Given a control object with no `TYPE` key, runtime selects in this order:
1. **`slider`** - if `MIN`, `MAX`, and `DEFAULT` are all present and numeric (`float`), and `DEFAULT in [MIN, MAX]`.
2. **`toggle`** - otherwise, if `DEFAULT` is present and `DEFAULT in {0, 1}` (integer or float-equal) and neither `MIN` nor `MAX` is present.
3. **`color`** - otherwise, if `DEF_COLOR` is present (4-float tuple) or `DEFAULT` is a 3- or 4-float array.
4. **`xy`** - otherwise, if `DEFAULT` is a 2-float array.
5. **strict-reject** - otherwise, with coercion diagnostic `missing-type`.

In compat mode (`compat-syn-v1`), the selected fallback is applied silently (with a `coercion-applied` warning logged, not shown mid-performance). In strict mode (`native-chroma-v1`), step 5 is the only outcome: the scene fails to load with `missing-type` and a pointer to the first offending control `NAME`.

The fallback must use `MIN`/`MAX` when present even if they are not the canonical `slider` defaults (`[0, 1]`); do not overwrite author-provided range hints.

`EV-COMPAT-FIXTURE-COVERAGE` includes at least one fixture per branch of this rule (slider, toggle, color, xy, strict-reject) using either the actual shipped reference scenes that trigger KM-01 or clean-room reconstructions.

### CP-02 Runtime Thread/Process Model
- Runtime uses a single process with dedicated subsystem threads:
 - render thread (frame scheduler + render graph),
 - audio analysis thread,
 - integration IO thread(s),
 - state/command thread.
- Cross-thread mutation of durable state is prohibited; all durable writes flow through `CommandBus` -> `StateCore`.
- Frame execution snapshots state at frame start; no mid-frame durable commit is visible until next frame tick.

### CP-02a Audio Snapshot Handoff
Locks the audio-thread -> render-thread contract so all cadence-dependent drift tolerances are deterministic. Source references: `06-audio-engine.md` Sec.7.11, `36-reactive-symbol-catalog.md` Sec.2, `10-performance-latency-budgets.md` drift table.

- **Audio snapshot structure**: a POD struct containing every symbol in `36-reactive-symbol-catalog.md` Sec.2 (scalars, level/hit/time/presence/beat/BPM/large-features families) plus the spectrum and level-trail texture pointers versioned by buffer index.
- **Publish cadence**: audio analysis thread writes a new snapshot at the hop rate `H_snap = 375 Hz` (`1/H_snap = 2.667 ms`) per `06-audio-engine.md` Sec.7.1. Snapshots are versioned with a monotonic `snapshot_seq` counter.
- **Concurrency**: lock-free triple-buffered ring. Audio thread rotates `write -> commit -> reader-owned -> reclaim`. Render thread atomically swaps the "current" snapshot pointer at frame start without blocking the audio thread.
- **Frame-start read**: frame scheduler reads `current_snapshot` at the beginning of each frame tick before any subsystem (StateCore, Scripts, RenderGraph) observes audio. This snapshot is immutable for the duration of the frame.
- **Single-snapshot-per-frame invariant**: every pass in a frame and the Script Host `update(dt)` invocation see the same `snapshot_seq`. This preserves the "once per frame before pass 0" contract in `36-reactive-symbol-catalog.md` Sec.2.
- **Texture samplers** (`syn_Spectrum`, `syn_LevelTrail`, `syn_FinalPass`): backed by double-buffered GPU textures owned by the render thread; audio thread writes spectrum/level-trail into staging buffers queued for next-frame upload.
- **Staleness bound**: `max_audio_staleness = 1/H_snap + frame_period = 2.67 ms + 16.67 ms ~= 19.3 ms` at 60 FPS. This is the canonical "1 frame" definition used by all audio rows of the Determinism and Drift Acceptance Table in `10-performance-latency-budgets.md`.
- **Degraded modes**:
 - audio thread fails to publish within 2 consecutive hop windows -> render loop reuses last snapshot and emits `audio-stall` diagnostic; uniforms fall back to silent/default per `06-audio-engine.md` reliability policy.
 - audio device recovery after stall -> snapshot sequence resumes; no warm-up fade is enforced (authoring scenes should rely on `syn_FadeInOut` for silence semantics).
- **Evidence binding**: `EV-AUDIO-TRACE` run metadata MUST record `snapshot_seq` per frame; drift comparisons are made at matching `snapshot_seq` values, not wall-clock timestamps, to ensure deterministic replay.

### CP-03 Warm-Switch Semantics
- Scene switch has four phases:
  1. `PrepareNext` (compile and validate target scene),
  2. `PrimeNext` (allocate pass targets and script setup),
  3. `CrossfadeOrHardCut` (per transition contract),
  4. `CommitNext` (state activation and cleanup old scene resources).
- If target scene preparation fails, active scene remains running and emits diagnostics.
- Warm-switch timeout is parametrized by shader-cache state:
 - warm cache (target scene's shaders already compiled and cached per `adr/ADR-011-rendering-backend.md` Sec.5): default 500 ms.
 - cold cache (target scene's shaders not yet compiled on this install): default 1500 ms.
- Timeout triggers fallback to hard cut + warning; the compile worker continues in the background so a subsequent launch of the same target scene may transition warm.

### CP-04 Traveler and Transition Timing Semantics
- Traveler controls are interpreted as normalized path-time drivers (`0.0..1.0`) with speed modifier support.
- Traveler out-of-range values are clamped in compatibility mode and rejected in strict mode.
- `transition()` script hook runs once per transition boundary after transition variables are bound and before first target-scene render pass.
- If `transition()` is absent, runtime proceeds with non-hook transition flow without warning.

### CP-05 State Command/Event Contract
- Command schema families:
 - control commands (`SetControl`, `SetControlNormalized`, `DefaultControl`, `RandomizeControl`),
 - preset commands (`CreatePreset`, `ApplyPreset`, `RenamePreset`, `DeletePreset`, `ApplyPresetChannel`),
 - Snap commands (`ApplySnap`, `SaveSnap`, `ClearSnap`) — UX and command-schema surface per the v1.2 `favslot` → `Snap` rename; `/favslots/{n}`, `/favslots/{n}/save`, `/favslots/{n}/clear` OSC wire routes are retained unchanged under `compat-syn-v1` per `08-io-integrations.md` Sec.OSC routes and the `compat-syn-v1` Alias Table,
 - playlist commands (`PlaylistSetActive`, `PlaylistNext`, `PlaylistPrevious`, `PlaylistSelect`, `PlaylistSetPosition`, `PlaylistPlay`, `PlaylistTogglePlay`, `StartSceneWithPreset`),
 - integration commands (`ApplyMappedInput`, `SetAdapterState`).
- Preset channel scope:
 - `ApplyPresetChannel` accepts one of `scene`, `meta`, or `media` to apply only that portion of a preset.
 - a plain `ApplyPreset` applies all declared channels.
- Scene launch defaulting:
 - `StartSceneWithPreset` with no preset resolves to the scene's Snap slot 0 ("gold") preset if present.
 - if no gold preset exists, scene loads with declared control defaults and no preset applied.
- Playlist position semantics:
 - negative position values count from the end of the playlist,
 - out-of-range positions wrap per playlist iteration mode.
- Every accepted command emits:
 - `CommandAccepted` event,
 - `StateDeltaCommitted` event with deterministic ordering index.
- Every rejected command emits `CommandRejected` with structured reason code and no side effects.

### CP-06 Integration Route Matrix Baseline
- OSC contract baseline: `osc-parity-v1`.
- MIDI contract baseline: `midi-parity-v1`.
- Required matrix dimensions:
 - route family,
 - payload type,
 - normalization behavior,
 - side-effect scope,
 - compatibility alias mapping,
 - conformance test ID.
- Reconnect contract:
 - preserve route bindings on recoverable disconnect,
 - reconnect success and time must satisfy SLO targets in `21-operational-slos-slis.md`.

### CP-07 Script Host API Parity
Locks the host-facing script surface so strict and compat modes are distinguishable and testable. Source contract reference: `02-synesthesia-contracts.md` section 3.

- Lifecycle hooks:
 - `setup()` runs once on scene load before first frame.
 - `update(dt)` runs each frame before render graph execution. `dt` is seconds since previous `update`.
 - `transition()` runs once per transition boundary per `adr/ADR-008-transition-and-traveler-semantics.md`. Absence is not a compatibility error.
- Uniform publishing API (`setUniform`):
 - accepted overloads: scalar number, `vec2`/`vec3`/`vec4` tuples, boolean (mapped to `0.0`/`1.0`), color triplet/quadruplet.
 - strict-mode rejects: unknown uniform names, type mismatch vs declared GLSL uniform, dimension mismatch.
 - compat-mode: clamps out-of-range color/position payloads and logs `coercion-applied`; accepts legacy numeric-string inputs with warning.
- Control mutation API:
 - `setControl(name, value)` and `setControlNormalized(name, value01)`.
 - `randomizeControl(name)`, `defaultControl(name)`.
 - `randomizeGroup(groupName)`, `defaultGroup(groupName)`.
 - strict-mode rejects unknown names/groups; compat-mode emits non-fatal warning and ignores unknown reference.
- Event registration API:
 - `onChange(name, handler)`, `onOffToOn(name, handler)`, `onOnToOff(name, handler)`, `whileOn(name, handler)`, `whileOff(name, handler)`.
 - handlers are invoked with deterministic ordering index relative to frame commit.
 - handler exceptions are contained (do not propagate to render loop) and are reported via diagnostics channel.
- Pixel readback API:
 - `textureFinalPass(x, y)` returns sampled `vec4` from previous-frame final pass buffer.
 - out-of-range coordinates clamp in compat-mode and return zero with diagnostic in strict-mode.
- Deprecated global shims (compat-only):
 - `inputs` and `uniforms` globals remain accessible as read/write proxies to canonical APIs.
 - each access site emits `deprecated-script-api` warning once per scene load.
 - shims are not available under `native-chroma-v1` strict profile.
- Execution budgets:
 - per-frame script budget follows `10-performance-latency-budgets.md` and `adr/ADR-012-script-sandbox-engine.md` Sec.2 (target 1.75 ms p99, hard 4.00 ms watchdog).
 - memory and stack caps per `adr/ADR-012-script-sandbox-engine.md` Sec.3 (16 MiB heap, 256 KiB stack per scene).
 - watchdog action: preempt -> degraded marker (2 consecutive) -> scene reset (3 consecutive); output pipeline is preserved per NFR-1.

#### Parity Matrix (strict vs compat)

| Surface | Strict (`native-chroma-v1`) | Compat (`compat-syn-v1`) |
|---|---|---|
| `setup()` / `update(dt)` / `transition()` | required signatures | same; missing `transition()` allowed silently |
| `setUniform` type/dimension mismatch | reject + diagnostic | coerce where possible + `coercion-applied` warning |
| Unknown control/group in mutation APIs | reject + diagnostic | ignore + deprecation warning |
| `textureFinalPass` out-of-range coords | return zero + diagnostic | clamp + non-fatal warning |
| Global `inputs` / `uniforms` access | unavailable | shimmed with `deprecated-script-api` warning |
| Event handler exceptions | contained; diagnostic emitted | contained; diagnostic emitted |

### CP-08 Meta Controls Parity
Meta controls are first-class global state layered over every scene and the media pipeline. Source contract reference: `02-synesthesia-contracts.md` section 6 and `../Contents/Resources/metacontrols.glsl`.

- State ownership:
 - meta control state is owned by `StateCore` (not scene-local),
 - visible to every scene uniformly,
 - persists across scene switches unless explicitly reset.
- Apply order (meta pipeline stages):
  1. media ingest and aspect correction,
  2. meta media transforms (mirror, invert, color correction, alpha),
  3. scene render graph execution (scene sees post-meta media sampling),
  4. meta output transforms applied to final frame before present.
- GLSL injection point:
 - meta helpers are prepended to compiled scene program so scene code references canonical meta symbols,
 - helpers are versioned and locked per compatibility profile.
- Scope guarantees:
 - meta controls affect both scene output and media pipeline.
 - scene-local controls cannot override meta state outside their own uniform namespace.
- Compatibility:
 - `compat-syn-v1` preserves observed apply order and symbol names,
 - `native-chroma-v1` retains the same semantic order with canonical symbol names; any rename requires migration entry in `22-compatibility-versioning-policy.md`.
- Validation binding:
 - meta parity is validated under `EV-RENDER-CONFORMANCE` and `EV-SCRIPT-CONTRACT` (for script-visible meta reads) against the golden corpus in `34-golden-corpus-definition.md`.

### CP-09 Transition Declaration Shapes and Semantics
Source: `02-synesthesia-contracts.md` Sec.15.

- **HARD_TRANSITIONS entry shape:** `{ "UNIFORM": <string>, "VALUES": [<scalar|vec>] }`. Duplicates in `VALUES` weight the selection distribution. On transition event, uniform picks `VALUES[floor(rand() * len(VALUES))]` atomically before the next frame.
- **SMOOTH_TRANSITIONS entry shape:** `{ "UNIFORMS": [<string>...], "DURATION": <float seconds> }`. Exactly one uniform holds `1.0`; others hold `0.0`. On transition, crossfade linearly over `DURATION` with minimum `0.05 s` (below clamped + warn).
- **Precedence:** a uniform appearing in both arrays is strict-reject / compat-keep-first-and-warn.
- **Validation:** `EV-RENDER-CONFORMANCE` and the new transition fixture set in `34-golden-corpus-definition.md`.
- **Compatibility:** coercion class `CR-05 transition-shape-malformed`: missing `UNIFORM`, `VALUES`, `UNIFORMS`, or `DURATION` is strict-reject / compat-skip-entry-and-warn.

### CP-10 Pass-Sizing Algebra
Source: `02-synesthesia-contracts.md` Sec.16.

- Variables `SceneWidth/Height`, `OutputWidth/Height`, `PassWidth/Height` (see source).
- Sentinel `0` or absent `PassWidth`/`PassHeight` resolves to `Output*` for `TARGET == "backbuff"` and to `Scene*` otherwise.
- "Scale Passes Relatively" preference scales absolute pass sizes by the scene-to-output ratio; final backbuffer always matches the output surface.
- `FLOAT` defaults to true (RGBA32F); `22-compatibility-versioning-policy.md` may register deterministic RGBA16F downgrade for specific MVP hardware classes with `coercion-applied` logging. Backend pixel-format guarantees are locked in `adr/ADR-011-rendering-backend.md` Sec.3.
- Validation: new pass-sizing fixture set in `34-golden-corpus-definition.md` covering: all-`0` sentinel, mixed sentinel, absolute-only, relative-scale-on, relative-scale-off, MVP-class downgrade.

### CP-11 Media Sampler Pair
Source: `02-synesthesia-contracts.md` Sec.17 and `36-reactive-symbol-catalog.md` Sec.1.5, Sec.3.2.

- Canonical samplers: `syn_Media` (post-meta-media) and `syn_UserImage` (pre-meta-media).
- Mask helpers (`_loadMediaAsMask`, `_textureMediaAsMask`) MUST sample `syn_UserImage`, not `syn_Media`.
- `_Media_Scale` is applied in sample-space by media helper functions, not in the ingest pass.
- Script Host may read media metadata (dimensions, type) via future host API (tracked in CP-07 addendum) but does not see samplers as direct values.
- Legacy compat aliases resolved per `36-reactive-symbol-catalog.md` Sec.4.9; strict mode does not expose aliases.
- Coercion class `CR-06 media-sampler-alias`: legacy name used under strict = reject; compat = alias + warn.

### CP-12 External Data Sources (`DATA_SOURCES`) - reserved; activated post-MVP
Source: `36-reactive-symbol-catalog.md` Sec.1.6 and `41-data-ingestion-differentiator.md`. This contract is a forward-compatibility reservation: MVP parses the key, exposes the declared symbols as zero-valued uniforms, and logs an informational `data-ingest-deferred` diagnostic per source. No network fetches, no transformation pipeline, and no scheduler impact at MVP. Full behavior is specified in ADR-017 (reserved slot).

- **Key name**: `DATA_SOURCES` (root-level in `scene.json`). Array of source objects. If absent or empty, no behavior change.
- **Source object required fields**: `NAME` (unique within scene, matches the suffix in `syn_Data_<Shape>_<NAME>`), `TYPE` (one of `scalar`, `vec3`, `vec4`, `array`, `field-2d`, `field-3d`, `event`), `UNIFORMS` (array of declared `syn_Data_*` symbols the source produces).
- **Source object optional fields**: `SOURCE` (string identifier for the provider, e.g. `noaa-mrms`, `celestrak-tle`, opaque to MVP runtime), `SELECT` (object; provider-specific query), `CADENCE_HZ` (float; runtime may coalesce if exceeded), `VIEWPORT` (object for geospatial sources), `FALLBACK` (what to publish when unavailable; default zero-valued of declared shape), `CACHE_POLICY` (object; reserved).
- **Schema validation** at MVP: required fields present; `TYPE` in the enum above; `UNIFORMS` names match `syn_Data_<Shape>_<NAME>` pattern consistent with declared `TYPE`. Violations are strict-reject with diagnostic `data-source-schema`; compat mode drops the offending source and warns.
- **Uniform shape inference**: `TYPE` determines the GLSL type of each declared uniform per `36-reactive-symbol-catalog.md` Sec.1.6 table. Mismatched pairings (e.g. `TYPE: scalar` with `syn_Data_Vec_foo`) are strict-reject.
- **Capability gating** (post-MVP): each `SOURCE` is tagged by the provider registry with which `data.ingest.*` capability it requires (`data.ingest.network`, `data.ingest.geospatial`, `data.ingest.sdr`). Without entitlement, the source publishes its `FALLBACK` and logs `data-entitlement-denied`. At MVP every source behaves as if unentitled.
- **Fixture determinism**: per `22-compatibility-versioning-policy.md` Sec.Data-pinned fixtures, any golden-corpus scene referencing `DATA_SOURCES` must ship with a recorded cassette or be excluded from `EV-RENDER-CONFORMANCE` until cassette support ships with ADR-017.
- **Coercion class** `CR-07 data-source-unavailable`: at MVP universal (all sources are unavailable); at V2+ on a transient fetch failure. Strict mode: reject on schema error, succeed-with-zero on unavailable; compat mode: warn-and-zero on both. This keeps scenes renderable (black-or-configured-fallback) rather than unloaded.

## Governing ADRs
- `adr/ADR-001-runtime-model.md`
- `adr/ADR-002-rendering-backend.md` (superseded on backend-selection by ADR-011)
- `adr/ADR-003-script-runtime-sandbox.md`
- `adr/ADR-004-state-model.md`
- `adr/ADR-005-integration-adapter-boundary.md`
- `adr/ADR-006-thread-and-process-model.md`
- `adr/ADR-007-scene-warm-switch-policy.md`
- `adr/ADR-008-transition-and-traveler-semantics.md`
- `adr/ADR-009-activation-and-entitlements.md`
- `adr/ADR-010-embedded-docs-generation.md`
- `adr/ADR-011-rendering-backend.md`
- `adr/ADR-012-script-sandbox-engine.md`
- `adr/ADR-013-networking-defaults-and-security.md`
- `adr/ADR-014-signing-notarization-auto-update.md`
- `adr/ADR-015-reference-app-user-data-import.md`
- `adr/ADR-016-observability-and-telemetry-posture.md`
- `adr/ADR-017-real-time-external-data-ingestion.md` (slot reserved; design post-MVP per `41-data-ingestion-differentiator.md`)

## Closure Evidence
- `EV-ARCH-CONTRACT-BASELINE`: contract baseline checklist, ADR references, and contract conformance smoke results.
- `EV-SYMBOL-CATALOG`: ratification of `36-reactive-symbol-catalog.md` as the canonical symbol source of truth.
