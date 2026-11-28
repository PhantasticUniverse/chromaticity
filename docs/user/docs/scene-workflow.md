---
doc-type: howto
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Scene Workflow

A Chromaticity scene is an on-disk package that the runtime loads, compiles,
and executes. This page covers the package layout, the load lifecycle, and
the live-reload behaviour authors rely on while iterating.

## Scene package layout

The native package extension is `.chroma`. A package is a plain directory;
the runtime accepts both the on-disk directory form and the `.chromaPack`
zip transport form. The directory contains a small fixed set of files:

| File | Required | Purpose |
| --- | --- | --- |
| `scene.json` | yes | Schema metadata, control declarations, pass declarations |
| `main.glsl` | yes | Fragment-shader body for the single-pass entry point |
| `script.js` | no | Optional script-host module: `setup()`, `update(dt)`, `transition()` |
| `glsl/*.glsl` | no | Shared GLSL include files |
| `images/*` | no | Bundled media samplers used by the scene |
| `data/*` | reserved | Reserved for the post-MVP external-data ingestion surface |

The reference application's `.synScene` packages are accepted as **import
input only** through the one-way Shadertoy / ISF / reference-show
converters — they are never executed in place. See
[Library Workflow](library-workflow.md) for how imported packages surface
in the library after conversion.

## The load lifecycle

When a scene loads, it transitions through a fixed lifecycle: `Unloaded`
→ `Loaded` → `Ready` → `Running`. The `Paused`, `Error`, and `Stopped`
terminal states are reachable from `Running`. Each transition is observable
through the runtime's diagnostics console.

The compile pipeline runs in this order:

1. Parse and validate `scene.json` against the canonical schema.
2. Build the canonical internal scene model (controls, passes,
   transitions, media bindings, script lifecycle metadata).
3. Preprocess GLSL — apply meta-pipeline includes and helpers.
4. Compile shader stages and resolve the pass graph.
5. Initialise the script sandbox, register lifecycle events.
6. Bind resource descriptors (passes, images, media slots).

The loader runs in two modes. **Strict** mode enforces the canonical
schema and hard-fails on contract violations; this is the default for
new authoring. **Compatibility** mode accepts legacy keys and observed
outliers from the `compat-syn-v1` corpus, normalises them to the
canonical model, and emits warnings — this is the mode the importers
target.

## Controls and passes

Controls are typed parameters declared in `scene.json` under the
`CONTROLS` array. Every control carries a `NAME`, `TYPE`, default, and
optional range. The runtime exposes each control as a uniform of the
matching GLSL type and as a script-host accessor; see the
[Hotkey Reference](hotkeys-reference.md) for the keyboard bindings that
operate on the focused control.

Passes are declared under the `PASSES` array. Single-pass scenes leave
`PASSES` empty and use `main.glsl` directly; multipass scenes name each
pass and reference earlier passes via the multipass uniforms documented
under [Chromaticity Uniforms](chroma-uniforms.md).

## Live reload while authoring

The authoring environment watches every file in the active package and
applies a per-file reload policy:

!!! note "Live-reload policy by file type"

    - `scene.json` — full scene rebuild; control state preserved where
      `NAME` is unchanged across the edit.
    - `main.glsl` — recompile only; control values, script state, and
      pass targets are preserved as long as pass declarations are
      unchanged.
    - `glsl/*.glsl` — same as `main.glsl` for every scene currently
      using the include.
    - `script.js` — rebind the script host, re-run `setup()`, keep
      control values, reset script-local state.
    - `images/*` — reload the affected sampler; everything else is
      preserved.

The shader-edit-to-visible-result latency budget is `p95 ≤ 500 ms` on
the MVP hardware class. Hot reloads are not allowed to drop output
frames; if a recompile exceeds the frame budget, the new shader is
swapped in on the next frame boundary.

## Transitions

Each scene declares its accepted transition shapes through the
`HARD_TRANSITIONS` and `SMOOTH_TRANSITIONS` keys. Transition triggers
are tied to runtime transition events (manual cuts, audio-driven
cues, playlist advancement). Transition variables are projected into
scene uniforms before pass execution so authors can sample them in
GLSL or JavaScript.

## Diagnostics

The authoring console streams diagnostics from every subsystem (GLSL
compiler, JS host, OSC and MIDI adapters, audio adapter, media adapter)
tagged with timestamp, subsystem, scene name, and severity. The
`coerce` severity covers every compatibility-mode coercion the loader
applies — useful when porting reference scenes to the canonical model.

GLSL compile errors map back to the author's source line through a
post-prepend mapping table, so the meta-pipeline header injected at
compile time does not skew line numbers. JSON schema violations link
directly to the offending path.

---

<!-- Clean-room authored from docs/05-scene-system.md, docs/37-authoring-environment-spec.md, engine/scene/src/schema.rs (slice-3a wire-format module-level prose), and engine/authoring/src/live_reload.rs; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
