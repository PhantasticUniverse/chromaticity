<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Scene Format Reference (`.chroma`)

A Chromaticity scene is a **directory** with a `.chroma` suffix containing a
`scene.json` manifest. This page documents the manifest shape that the Phase-1
loader (`engine/scene/src/loader.rs`) and validator
(`engine/scene/src/validator.rs`) accept today.

The canonical schema source-of-truth is `engine/scene/src/schema.rs`. When
this page and the schema file disagree, the schema file wins; please open an
issue or a `docs(repo)` follow-up to reconcile.

## Layout on disk

```
my-scene.chroma/
└── scene.json     # required: manifest
```

Future loader slices will introduce `main.glsl`, `script.js`, `images/`, and
`.chromaPack` zip transport; today only `scene.json` is read end-to-end.
Other paths inside the directory are tolerated (they are simply not loaded
yet).

## Top-level structure

The wire format mirrors the keys observed in reference scenes for
compatibility, with a small set of Chromaticity-reserved additions. All
top-level keys are **uppercase**.

| Key | Type | Required | Notes |
|---|---|---|---|
| `SCHEMA_VERSION` | integer | optional | Defaults to `1`. Reserved for future schema bumps; today only `1` is meaningful. |
| `TITLE` | string | recommended | Human-readable scene title. Surfaced in Library views. |
| `CREDIT` | string | recommended | Author credit. |
| `DESCRIPTION` | string | recommended | One- or two-sentence summary. |
| `TAGS` | string array | optional | Free-form labels for search and filtering (`"audio"`, `"reactive"`, `"geometric"`). |
| `CATEGORIES` | string array | optional | Library taxonomy buckets (e.g. `"BUILT-IN"`, `"AUDIO REACTIVE"`, `"TEXT"`). |
| `CONTROLS` | object array | optional | Author-declared controls (sliders, toggles, etc.). See [Controls](#controls). |
| `PASSES` | object array | optional | Render-pass list. See [Passes](#passes). Empty / missing means "no rendering yet"; useful for parameter-only seed scenes. |
| `HARD_TRANSITIONS` | object | reserved | Parsed but not yet executed; flattens through `Scene::extensions`. Lifecycle lands in slice 5b. |
| `SMOOTH_TRANSITIONS` | object | reserved | As above. |
| `DATA_SOURCES` | object | reserved | Post-MVP external data ingestion per `41-data-ingestion-differentiator.md`. Parse-reserve only at MVP per `28-implementation-contract-pack.md` §CP-12. |
| (other) | any | tolerated | Unknown top-level keys round-trip through `Scene::extensions` per the §CP-12 parse-reserve rule. |

## Controls

`CONTROLS` is an array of author-declared control objects. Each control
appears in the UI inspector and the script-host global namespace; per-pass
`UNIFORMS` bindings reference reactive symbols from the catalog rather than
controls, but controls drive the symbols indirectly through the runtime
state.

| Control field | Type | Notes |
|---|---|---|
| `NAME` | string | Required. Free-form author-declared label. Becomes the script-global identifier; uniqueness is recommended (the slice-3a validator does not enforce it yet). |
| `TYPE` | string | Optional. One of `slider`, `toggle`, `color`, `xy`, `bang`, `dropdown`, `knob`. Compound spellings (`"slider smooth"`, `"xy traveler"`) are accepted; only the first whitespace-delimited token is canonical at slice-3a. If absent, the CP-01 missing-`TYPE` fallback runs. |
| `DEFAULT` | scalar / array / bool | Default value. Shape varies by type: `slider`/`knob`/`bang` = scalar; `toggle` = bool or `0`/`1`; `color` = 3- or 4-tuple; `xy` = 2-tuple; `dropdown` = scalar index. |
| `MIN` | float | Optional. Lower bound for `slider` / `knob`. |
| `MAX` | float | Optional. Upper bound for `slider` / `knob`. |
| `UI_GROUP` | string | Optional. Inspector grouping label. |
| `DESCRIPTION` | string | Optional. Tooltip-class help text. |
| `LABELS`, `VALUES` | arrays | `dropdown` option labels and underlying values. |
| `IS_META` | bool | Optional. Marks the control as state-owned (CP-08); runtime-state semantics defer past slice-3a. |
| `DEF_COLOR` | 4-tuple | Optional. Color default (also serves as the CP-01 fallback signal for missing-`TYPE` colour controls). |
| `LABEL` | string | Optional legacy single-label spelling; preserved for ingest. |
| `PARAMS` | object | Optional. Type-specific parameter blob (e.g. smoothing constants); preserved opaquely. |
| (other) | any | Unknown control keys flatten through the per-control `extensions` map. |

### Missing-`TYPE` fallback (CP-01)

If a control omits `TYPE`, the validator runs the contract-pack §CP-01
fallback predicate. **Compat mode** coerces to a best-fit kind and emits a
diagnostic; **strict mode** rejects the scene. The disambiguation rules and
the five branches (slider, toggle, color, xy, reject) are documented in
`docs/28-implementation-contract-pack.md` §CP-01 and pinned by the
`cp01-*.chroma` fixture corpus under `engine/scene/tests/fixtures/scenes/`.

## Passes

`PASSES` is an array of render-pass objects describing the scene's render
graph. Pass order is the array order; the final visible output is the last
pass whose `TARGET` resolves to the back-buffer (`backbuff`).

| Pass field | Type | Notes |
|---|---|---|
| `TARGET` | string | Render-target node name. Use `"backbuff"` for the visible output; named intermediates (`"luma"`, `"blurH"`, etc.) become inputs to subsequent passes. Node-ID uniqueness across `PASSES` is enforced by the validator. |
| `UNIFORMS` | object array | Optional. Per-pass uniform bindings; each entry is `{ "NAME": "<binding>", "SYMBOL": "<catalog symbol>" }`. The symbol must be in the slice-3a representative subset (see `03-standard-uniforms.md`); unknown symbols are rejected in both compat and strict modes. |
| `WIDTH`, `HEIGHT` | integer | Optional. Pass dimensions; `0` (or absence) means "scene size" per `28-implementation-contract-pack.md` §CP-10. |
| `FLOAT` | bool | Optional. Float colour buffer hint. |
| `WRAP`, `FILTER` | string | Optional. Sampler hints. |
| (other) | any | Unknown per-pass keys flatten through the per-pass `extensions` map. |

## Minimal example

The smallest meaningful audio-reactive scene: one pass, one binding, one
slider control. Save the JSON below as `simple-pulse.chroma/scene.json`.

```json
{
  "SCHEMA_VERSION": 1,
  "TITLE": "Simple Pulse",
  "CREDIT": "you",
  "DESCRIPTION": "Single-pass scene that pulses on whole-band audio level.",
  "TAGS": ["audio", "reactive", "tutorial"],
  "CATEGORIES": ["AUDIO REACTIVE"],
  "CONTROLS": [
    {
      "NAME": "pulse_gain",
      "TYPE": "slider",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 4.0
    }
  ],
  "PASSES": [
    {
      "TARGET": "backbuff",
      "UNIFORMS": [
        { "NAME": "u_level", "SYMBOL": "syn_Level" },
        { "NAME": "u_time",  "SYMBOL": "TIME" },
        { "NAME": "u_resolution", "SYMBOL": "RENDERSIZE" }
      ]
    }
  ]
}
```

`04-first-scene-tutorial.md` walks through the matching fragment shader and
how to load the scene end-to-end.

## Validation

Two modes apply at scene-load time:

- **Compat (default).** Missing `TYPE` is coerced per CP-01; recoverable
  shape mismatches emit warnings rather than errors. Imported reference
  scenes with legacy spellings continue to load.
- **Strict.** Same shape, no coercion. Missing `TYPE`, unknown reactive
  symbols, duplicate node IDs, and CP-01-rejectable controls fail closed.

The validator's representative-subset of reactive symbols is documented in
`03-standard-uniforms.md`; the full slice-3a list lives in
`engine/scene/src/validator.rs` (`KNOWN_SYMBOLS`).

## Where to look next

- **Canonical schema:** `engine/scene/src/schema.rs` (every `serde(rename)`
  attribute names a wire key).
- **Validator and CP-01 fallback:** `engine/scene/src/validator.rs`.
- **Demo corpus:** `engine/scene/tests/fixtures/scenes/` and its
  `README.md` — both the `demo-*.chroma` set and the `cp01-*.chroma`
  fixtures are clean-room originals you can copy and modify.
- **Evidence rows:** `26-evidence-artifact-registry.md` rows
  `EV-SCENE-IMPORT` (loader conformance) and `EV-COMPAT-FIXTURE-COVERAGE`
  (CP-01 fixture corpus). When the schema widens past slice-3a, the row
  bodies and fixture set update first.
- **Compatibility versioning rules:** `22-compatibility-versioning-policy.md`
  documents how schema changes interact with the `compat-syn-v1` and
  `native-chroma-v1` profile axes.
