<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Demo `.chroma` corpus

This directory holds the bundled clean-room demo scene corpus used by Wave-10
critical-path work. Each `<name>.chroma/` directory carries a `scene.json`
that satisfies the slice-3a `Scene` deserializer in
`engine/scene/src/schema.rs` and the `docs/38-scene-library-and-import-spec.md`
§2.2 metadata fields (`TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`, `CATEGORIES`).

## Layout — four-subdirectory taxonomy (Wave-4b.1)

Per `docs/38-scene-library-and-import-spec.md` §2.1 the bundled scene root
ships at `<AppBundle>/Resources/scenes/{welcome,library,examples,shadertoy-fixtures}/`.
This fixtures root mirrors that layout because `bundle.resources` in
`app/shell/tauri.conf.json` maps directly from this tree to the AppBundle:

- `library/` — canonical bundled demos (the three `demo-*.chroma` packages).
  See `library/README.md`.
- `welcome/` — first-launch onboarding scene placeholder.
  See `welcome/README.md`.
- `examples/` — per-feature demonstration scenes placeholder.
  See `examples/README.md`.
- `shadertoy-fixtures/` — Shadertoy iChannel asset-pack placeholder.
  See `shadertoy-fixtures/README.md`.

The CP-01 compat-fallback fixtures (`cp01-*.chroma`) and the reactive-symbol-
family fixtures (`multipass-*`, `reactive-audio`, `meta-controls`,
`traveler-controls`, `transition-hard`, `pass-sizing-sentinel`) remain at
the top level of this directory: they are test corpora for
`engine/scene/tests/{import,cp01_compat,lint_corpus}.rs` and are not
shipped into the AppBundle.

## Provenance

The fixtures and this README were authored clean-room by Xavier Butler on
2026-04-26 per `docs/29-cleanroom-legal-policy-baseline.md`. No prose, control
names, default values, palette tokens, or category labels were copied from the
reference app, ISF, Shadertoy, or any third-party scene corpus. Category
spellings (`BUILT-IN`, `ABSTRACT`, `GEOMETRIC`, `AUDIO REACTIVE`, `TEXT`) match
the wireframe-01 chip set, which is itself clean-room-coined. Each `scene.json`
falls under the repo-default REUSE annotation (`LicenseRef-Chromaticity-Proprietary`,
`2026 Chromaticity`); JSON cannot carry an inline SPDX header.

## Consumers

- `app/shell/src/commands/library.rs` tests — un-ignored in Wave-10 PR1; the
  fixture-backed coverage exercises `chromaticity_scene::loader::load_chroma_dir`
  on `demo-kaleido.chroma` directly without staging under `$HOME`.
- `engine/scene/tests/import.rs` — Wave-10 PR3 conformance harness; pins
  the demo corpus golden hashes (T1 drift-pin tier), Wave-11 PR1 extends
  the same harness with golden hashes for the `cp01-*.chroma` fallback
  fixtures under `EV-COMPAT-FIXTURE-COVERAGE` (T2), and Wave-12 PR1 adds
  T2 golden hashes for the reactive-symbol-family fixtures listed below
  to close `EV-SCENE-IMPORT` per `docs/26` row 25 and `docs/43` §15.5.
- `engine/scene/tests/cp01_compat.rs` — Wave-11 PR1 compat-vs-strict
  assertions over the `cp01-*.chroma` fixtures; this is the file-fixture
  counterpart to the `compat_fixture_coverage` T1 module in
  `engine/scene/src/validator.rs`, advancing `EV-COMPAT-FIXTURE-COVERAGE`
  past `pending:artifact` per `docs/26` row 27.
- `scripts/stage-demo-scenes.sh` — developer-facing idempotent copy of the
  three demo packages to the platform-default scene root.

## Demo corpus (under `library/`)

- `library/demo-kaleido.chroma` — geometric reference scene exercising
  slider, knob, toggle, color, and xy controls.
- `library/demo-spectrum.chroma` — audio-reactive reference scene
  carrying the `audio` tag for library-scanner coverage.
- `library/demo-text.chroma` — text-category reference scene carrying
  the `TEXT` category for library-scanner coverage.

## CP-01 fallback corpus (Wave-11 PR1)

Five additional `cp01-<branch>.chroma` packages exist solely to satisfy the
`EV-COMPAT-FIXTURE-COVERAGE` T2 row of `docs/26-evidence-artifact-registry.md`.
Each fixture carries exactly one `CONTROLS` entry whose `TYPE` field is
deliberately omitted, exercising one branch of the CR-01 missing-`TYPE`
fallback rule per `docs/28-implementation-contract-pack.md` §CP-01:

- `cp01-slider.chroma` — step 1: numeric DEFAULT inside `[MIN, MAX]`. Compat
  coerces to `slider`; strict rejects with `MissingControlType`.
- `cp01-toggle.chroma` — step 2: numeric DEFAULT in `{0, 1}` with neither
  MIN nor MAX. Compat coerces to `toggle`; strict rejects.
- `cp01-color.chroma` — step 3: `DEF_COLOR` 4-float tuple present. Compat
  coerces to `color`; strict rejects.
- `cp01-xy.chroma` — step 4: 2-float DEFAULT array, no MIN/MAX/DEF_COLOR.
  Compat coerces to `xy`; strict rejects.
- `cp01-reject.chroma` — step 5: numeric DEFAULT outside MIN/MAX (slider
  range guard fails, MIN/MAX presence disqualifies toggle, scalar default
  disqualifies color/xy). Compat AND strict both reject — CP-01 step 5
  applies in compat mode too when no fallback matches.

These fixtures are the file-shipped form of the CR-01 branch coverage that
the in-validator `compat_fixture_coverage` T1 module pins against
hand-built `Scene` structs; `engine/scene/tests/cp01_compat.rs` re-runs the
same compat-vs-strict assertions through `load_chroma_dir`, which is what
"shipped reference scenes" requires per `docs/26` row 27.

## Reactive-symbol-family corpus (Wave-12 PR1)

Seven additional `.chroma` packages exercise broader Coverage Dimensions named
in `docs/34-golden-corpus-definition.md` §Minimum Coverage Requirements within
the bounds of what the slice-3a schema in `engine/scene/src/schema.rs` plus
the `engine/scene/src/loader.rs` deserializer actually parses. Closing
`EV-SCENE-IMPORT` per `docs/26` row 25 requires conformance over a fixture
corpus per `docs/43` §15.5; these fixtures are the file-shipped corpus and
`engine/scene/tests/import.rs` pins their canonical-form FNV-1a 64-bit
golden hashes:

- `multipass-feedback.chroma` — two-pass scene with per-pass `UNIFORMS`
  canonical bindings and a `backbuff` final target. Covers docs/34 multipass
  plus per-pass uniform-binding shape.
- `multipass-chain.chroma` — four-pass linear chain producing distinct
  intermediate render targets (`luma`, `blurH`, `blurV`, `backbuff`). Covers
  docs/34 multipass-chain shape.
- `reactive-audio.chroma` — six audio-reactive controls and per-pass
  `UNIFORMS` bindings to `chroma_Spectrum`, `chroma_Level`, `chroma_AudioTime`, `TIME`, and
  `RENDERSIZE`. Covers the reactive-symbol-family branch of docs/34 over the
  slice-3a representative subset in
  `engine/scene/src/validator.rs::KNOWN_SYMBOLS`.
- `meta-controls.chroma` — four `IS_META: true` controls covering CP-08
  meta ingestion shape (state-ownership runtime semantics deferred).
- `traveler-controls.chroma` — three controls with compound `TYPE` tokens
  (`slider traveler`, `xy traveler`, `bang counter`). Covers the ADR-008
  traveler shape ingestion path; the slice-3a parser keeps only the
  canonical base kind, with the original spelling round-tripping through the
  JSON.
- `transition-hard.chroma` — declares `HARD_TRANSITIONS` and
  `SMOOTH_TRANSITIONS` per CP-09. Slice-3a flattens these into
  `Scene::extensions` (parse-reserve); structured execution lands in a later
  slice.
- `pass-sizing-sentinel.chroma` — three passes exercising CP-10 sentinels:
  a `WIDTH`/`HEIGHT` of `0` (scene-size resolution), an absolute
  half-resolution pass, and a `backbuff` with absent dimensions (output-size
  resolution).

These fixtures intentionally do NOT cover dimensions the slice-3a loader cannot
parse: `script.js` interpretation, `.chromaPack` zip transport, `main.glsl`
pairing, structured transition execution, media / camera / external-texture
runtime, Shadertoy import, ISF reject. Those defer to slice-4+ work.

## Adding new fixtures

1. Create `<name>.chroma/scene.json` in this directory.
2. Make sure the JSON deserializes via `chromaticity_scene::loader::parse_scene_json`
   (every wire key in `engine/scene/src/schema.rs` is honored; unknown root keys
   flatten into `Scene::extensions`).
3. Carry the §2.2 metadata block (`TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`,
   `CATEGORIES`) so library tooling has something to surface.
4. Re-run `cargo test -p chromaticity-scene` to refresh the loader-level
   goldens, then `cargo test -p chromaticity-app` for the library-handler
   coverage.
