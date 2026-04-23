# Scene System Blueprint

## Objective
Define how Chromaticity ingests, validates, compiles, executes, and migrates scene packages with compatibility guarantees.

## Scene Package Model
- Native package: `.chroma` directory package (see `22-compatibility-versioning-policy.md` Sec.Package Extensions). Transport form is `.chromaPack` (zip archive of the directory).
- Reference-app `.synScene` directory packages are accepted as **import input only** via one-way conversion to `.chroma`; see `38-scene-library-and-import-spec.md` Sec.5.2 and `37-authoring-environment-spec.md` Sec.3.7.
- Required files:
 - `scene.json`
 - `main.glsl`
- Optional files:
 - `script.js`
 - `images/*`
 - `glsl/*` (includes, per `02-synesthesia-contracts.md` Sec.18)
 - `data/*` (reserved; post-MVP with ADR-017)

## Loader Modes
- **Strict mode**
 - Enforce canonical schema only.
 - Hard-fail unknown required contract violations.
- **Compatibility mode**
 - Accept legacy keys and observed outliers.
 - Normalize to canonical internal model and emit warnings.
 - Apply profile behavior from `22-compatibility-versioning-policy.md`.

## Canonical Internal Scene Model
- Metadata
- Controls (typed, validated, grouped)
- Render pass descriptors
- Transition descriptors
- Image/media bindings
- Script lifecycle metadata

## Normalization Rules (Initial)
- `IMAGE` -> `IMAGE_PATH` when present.
- Preserve `GPU`, `TAGS`, and legacy/custom metadata in `scene.extensions`.
- Accept traveler control variants in compatibility mode.
- Normalize control naming/case behavior for script/event API lookup.
- If control `TYPE` is missing but shape is otherwise valid, coerce to `slider` and emit `coercion-applied`.
- If numeric control ranges are inverted (`MIN > MAX`), swap deterministically and emit `coercion-applied`.
- If control metadata is malformed but recoverable, keep control in compatibility mode with warning rather than hard failure.
- Preserve deprecated script globals (`inputs`, `uniforms`) through compatibility shims.
- Support `transition()` hook when defined, while remaining compatible with scenes that only provide `setup()`/`update(dt)`.

## Compile Pipeline
1. Parse and validate `scene.json`.
2. Build canonical model.
3. Preprocess GLSL (meta includes/helpers).
4. Compile shader stages/pass graph.
5. Initialize script sandbox and register events.
6. Bind resource descriptors (passes/images/media).

## Scene Lifecycle States
- `Unloaded`
- `Loaded`
- `Ready`
- `Running`
- `Paused`
- `Error`
- `Stopped`

## Transition Contract
- Support hard and smooth transition descriptors.
- Transition triggers are tied to runtime transition events (manual + audio-driven policy).
- Transition variables are projected into scene uniforms before pass execution.

## Compatibility Risk Areas
- Traveler control semantics.
- Legacy metadata field behavior.
- Script/runtime expectations around deprecated globals.
- Pass precision defaults and feedback edge behavior.
- Missing control type fallback behavior.
- Numeric range inversion and coercion determinism.
- Coexistence rules when legacy and canonical keys are both present.

## Validation and Tooling Requirements
- Human-readable diagnostics with file and key context.
- Compatibility warning classes:
 - `legacy-key`
 - `unknown-control-type`
 - `deprecated-script-api`
 - `coercion-applied`
- Scene lint mode for authors before runtime load.

## Implementation Contract Baseline
- Scene schema and normalization precedence are locked in `28-implementation-contract-pack.md`.
- Canonical scene model and loader API behavior are governed by `adr/ADR-001-runtime-model.md` and `adr/ADR-004-state-model.md`.
- Transition and traveler behavior is governed by `adr/ADR-008-transition-and-traveler-semantics.md`.
- Compatibility profile/version metadata and coercion logging are governed by `22-compatibility-versioning-policy.md`.
