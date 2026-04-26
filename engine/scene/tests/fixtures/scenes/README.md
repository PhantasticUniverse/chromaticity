<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

# Demo `.chroma` corpus

This directory holds the bundled clean-room demo scene corpus used by Wave-10
critical-path work. Each `<name>.chroma/` directory carries a `scene.json`
that satisfies the slice-3a `Scene` deserializer in
`engine/scene/src/schema.rs` and the `docs/38-scene-library-and-import-spec.md`
§2.2 metadata fields (`TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`, `CATEGORIES`).

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
- `engine/scene/tests/import.rs` — Wave-10 PR3 conformance harness; closes
  `EV-SCENE-IMPORT` (T2) over the demo corpus, and Wave-11 PR1 extends the
  same harness with golden hashes for the `cp01-*.chroma` fallback fixtures
  under `EV-COMPAT-FIXTURE-COVERAGE` (T2).
- `engine/scene/tests/cp01_compat.rs` — Wave-11 PR1 compat-vs-strict
  assertions over the `cp01-*.chroma` fixtures; this is the file-fixture
  counterpart to the `compat_fixture_coverage` T1 module in
  `engine/scene/src/validator.rs`, advancing `EV-COMPAT-FIXTURE-COVERAGE`
  past `pending:artifact` per `docs/26` row 27.
- `scripts/stage-demo-scenes.sh` — developer-facing idempotent copy of the
  three demo packages to the platform-default scene root.

## Demo corpus

- `demo-kaleido.chroma` — geometric reference scene exercising slider, knob,
  toggle, color, and xy controls.
- `demo-spectrum.chroma` — audio-reactive reference scene carrying the
  `audio` tag for library-scanner coverage.
- `demo-text.chroma` — text-category reference scene carrying the `TEXT`
  category for library-scanner coverage.

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
