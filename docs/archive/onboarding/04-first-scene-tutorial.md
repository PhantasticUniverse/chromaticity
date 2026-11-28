<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Your First Audio-Reactive Scene

This walkthrough builds the smallest meaningful audio-reactive scene from
scratch: a single-pass fragment shader that pulses a colour fill on
whole-band audio level. By the end you will know how a `.chroma` directory
fits together, how reactive symbols flow into a fragment program, and where
to look when something does not behave the way you expect.

**Prerequisites.**

- Chromaticity installed and runnable. (Phase-1 installs ship the app shell
  with Perform / Library / Author / Settings views.)
- A text editor.
- An audio source you can pipe into the system — line-in, virtual
  loopback, or any DAW / media player routed to a device Chromaticity can
  read.

**Time budget.** ~10 minutes if you have already read
`02-scene-format.md` and `03-standard-uniforms.md`. Read those first if you
have not — this tutorial assumes you know the manifest shape and the
slice-3a uniform set.

## Step 1 — Make the scene directory

Pick a working folder (anywhere on disk you can edit; the `Library` view
has a configurable root, defaulting to a user-scoped directory documented
in `38-scene-library-and-import-spec.md`). Create:

```
simple-pulse.chroma/
└── scene.json
```

The `.chroma` suffix is required — it is how the loader recognises the
package. The directory name is otherwise free-form.

## Step 2 — Write the manifest

Save the following as `simple-pulse.chroma/scene.json`. The full key
reference is in `02-scene-format.md`; this manifest uses only the slice-3a
loader-supported keys.

```json
{
  "SCHEMA_VERSION": 1,
  "TITLE": "Simple Pulse",
  "CREDIT": "you",
  "DESCRIPTION": "Single-pass scene that pulses colour on whole-band audio level.",
  "TAGS": ["audio", "reactive", "tutorial"],
  "CATEGORIES": ["AUDIO REACTIVE"],
  "CONTROLS": [
    {
      "NAME": "pulse_gain",
      "TYPE": "slider",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 4.0,
      "DESCRIPTION": "How strongly the level affects brightness."
    },
    {
      "NAME": "tint",
      "TYPE": "color",
      "DEFAULT": [0.27, 0.66, 0.93, 1.0],
      "DESCRIPTION": "Pulse colour."
    }
  ],
  "PASSES": [
    {
      "TARGET": "backbuff",
      "UNIFORMS": [
        { "NAME": "u_level",      "SYMBOL": "syn_Level"  },
        { "NAME": "u_time",       "SYMBOL": "TIME"       },
        { "NAME": "u_resolution", "SYMBOL": "RENDERSIZE" }
      ]
    }
  ]
}
```

What each field is doing:

- **`SCHEMA_VERSION: 1`** — pins the manifest to the slice-3a wire format.
- **`TITLE` / `CREDIT` / `DESCRIPTION` / `TAGS` / `CATEGORIES`** — Library
  metadata. `"AUDIO REACTIVE"` is one of the canonical category strings
  documented in the demo corpus README
  (`engine/scene/tests/fixtures/scenes/README.md`).
- **`CONTROLS`** — declares two author-facing parameters: a slider (`pulse_gain`)
  bounded `[0, 4]`, and an RGBA colour (`tint`) defaulting to a calm blue.
  Both appear in the inspector at runtime.
- **`PASSES`** — one render pass writing to the back-buffer. The `UNIFORMS`
  array binds three reactive symbols from the slice-3a representative subset
  (`03-standard-uniforms.md`): `syn_Level` for audio reactivity, `TIME` for
  an animating clock, `RENDERSIZE` so the shader knows its pixel size.

## Step 3 — Validate the manifest

Before you wire up GLSL, sanity-check the JSON. From the repository root,
in a developer build:

```sh
cargo test -p chromaticity-scene
```

The `engine/scene/tests/import.rs` harness exercises the loader and
validator against the demo corpus; if your manifest is malformed, the same
parser will reject it with a typed `ValidationError` you can read in the
test output. (For end-user installs, the `Library` and `Author` UI
surfaces report load errors inline — there is no separate CLI today.)

Common mistakes the validator catches:

- **Unknown reactive symbol.** A `"SYMBOL"` in `UNIFORMS` that is not in
  `KNOWN_SYMBOLS` (slice-3a representative subset) fails closed in both
  compat and strict modes.
- **Duplicate node ID.** Two passes with the same `TARGET` is rejected.
- **Missing `TYPE` on a control with no fallback signal.** Compat mode
  attempts the CP-01 fallback; if no branch matches, both modes fail.

## Step 4 — Author the fragment shader

The slice-3a loader does not yet pair a `main.glsl` with `scene.json`
automatically; that pairing lands in the slice-4 loader pass. The
following is the fragment program your `simple-pulse.chroma` will run
once GLSL pairing ships, and is also a useful reference for understanding
what the bound uniforms look like inside a shader.

`simple-pulse.chroma/main.glsl`:

```glsl
#version 450

uniform float u_level;        // bound to syn_Level — whole-band audio level [0,1]
uniform float u_time;         // bound to TIME — seconds since scene start
uniform vec2  u_resolution;   // bound to RENDERSIZE — current pass pixel size

// Author-declared controls reach the shader as uniforms with the control NAME.
uniform float pulse_gain;     // slider [0, 4], default 1.0
uniform vec4  tint;           // color, default (0.27, 0.66, 0.93, 1.0)

out vec4 fragColor;

void main() {
    // Normalised UV in [0,1] across the current pass.
    vec2 uv = gl_FragCoord.xy / u_resolution;

    // Centre the coordinate so the gradient is symmetric.
    vec2 c = uv - 0.5;

    // Soft radial vignette so the pulse reads cleanly.
    float radial = 1.0 - smoothstep(0.0, 0.7, length(c));

    // Drive brightness from audio level. pulse_gain scales how much the
    // level matters; u_time gives a slow background shimmer so the scene
    // is alive even during quiet passages.
    float brightness =
        clamp(u_level * pulse_gain, 0.0, 1.0)
        * radial
        + 0.05 * (0.5 + 0.5 * sin(u_time * 0.7));

    fragColor = vec4(tint.rgb * brightness, tint.a);
}
```

What each block is doing:

- **`uniform float u_level;`** matches the binding name `u_level` from
  `scene.json`. The runtime publishes `syn_Level` (whole-band, smoothed,
  normalised) into this slot once per frame.
- **`uniform float u_time;`** receives the `TIME` clock — seconds since the
  scene started rendering, refreshed per-pass.
- **`uniform vec2 u_resolution;`** receives `RENDERSIZE`, the current pass's
  pixel dimensions. We use it to compute normalised UVs.
- **`pulse_gain` and `tint`** are author-declared controls. They reach the
  shader as uniforms named exactly as their `NAME` field (per the script-
  host mirror rule in `36-reactive-symbol-catalog.md` §5; the GLSL surface
  follows the same convention).
- **`brightness`** scales the radial vignette by audio level (gated by the
  `pulse_gain` slider) and adds a small `TIME`-driven shimmer so the scene
  is not pitch black during silence.

## Step 5 — Load the scene and feed it audio

Once Chromaticity is running:

1. **Point the Library view at your working folder.** The Library scans
   `.chroma` packages and surfaces them in the grid. Your scene appears
   under `AUDIO REACTIVE` with the `TITLE` you set.
2. **Open the scene.** Selecting it brings up the Inspector with the two
   controls (`pulse_gain` and `tint`); the preview region shows the
   shader output.
3. **Route audio to the input device Chromaticity is configured to read.**
   Settings → Audio surfaces the input source and a sensitivity scalar
   `[0.5, 2.0]` documented in `06-audio-engine.md` "Configuration Surface".
4. **Watch it pulse.** Speech and quiet music produce gentle reactivity;
   loud sustained sound saturates the brightness. Drag `pulse_gain` to
   tune how strongly level drives the pulse.

## What you have learned

- A `.chroma` package is a directory with a `scene.json` manifest plus
  (eventually) GLSL programs and assets.
- `CONTROLS` declare author-facing parameters that surface in the
  Inspector and reach your shader as uniforms named after the control's
  `NAME`.
- `PASSES.UNIFORMS` binds reactive-symbol-catalog entries to per-pass
  uniform slots; `syn_Level`, `TIME`, and `RENDERSIZE` are the workhorses
  for a first scene.
- The audio analysis pipeline runs at a fixed `375 Hz` cadence, publishes
  a per-frame snapshot to the renderer, and provides graceful fallback
  values when audio is silent or unavailable.
- Compat mode tolerates legacy / outlier shapes (CP-01 missing-`TYPE`
  fallback); strict mode is the same shape with no coercion.

## Where to go next

- **More reactive symbols.** Read `03-standard-uniforms.md` to see what
  per-band levels, hits, BPM, and beat-synchronised waveforms look like —
  most are catalog-ready and become live as the slice-3 loader widens.
- **Multi-pass scenes.** Add a named intermediate `TARGET` (e.g.
  `"luma"`) and feed it into a second pass via `syn_FinalPass`-style
  feedback. The `engine/scene/tests/fixtures/scenes/multipass-chain.chroma`
  fixture is a clean-room example.
- **Larger reference patterns.** The demo corpus
  (`engine/scene/tests/fixtures/scenes/library/demo-*.chroma`) ships
  clean-room scenes covering geometric, audio-reactive, and text-driven
  shapes.
- **Library and import.** `38-scene-library-and-import-spec.md` documents
  the on-disk layout the Library expects and the import pipeline for
  external scene formats.
- **Authoring environment.** `37-authoring-environment-spec.md` is the
  contract for the in-app Author view.
- **Performance budgets.** `10-performance-latency-budgets.md` documents
  the per-frame budgets your scene should fit inside on supported hardware
  classes.
- **Risk register and known deferrals.** `13-risk-register.md` and
  `26-evidence-artifact-registry.md` are the canonical sources for "is X
  shipped yet?" questions.
