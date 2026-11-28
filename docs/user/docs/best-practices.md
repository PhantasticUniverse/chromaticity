---
doc-type: explanation
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Best practices

A pattern catalogue for scene authors. Three top-level sections:

1. [Control design](#1-control-design) — how to lay out a scene's
   author-facing knobs so Perform, Snap, presets, and playlists stay
   composable.
2. [GLSL pitfalls](#2-glsl-pitfalls) — common foot-guns when writing
   shaders against the `chroma_*` / `syn_*` uniform surface.
3. [GLSL performance](#3-glsl-performance) — cost considerations,
   when to reach for Lygia, and the honest picture of what the V1
   preview adapter does and doesn't render.

Read top-down on the first pass; each subsection is a named pattern
or pitfall and is independently citable. If you only have five
minutes, the four most leverage-y items are
[Control surface composability](#13-snap-friendliness-and-the-control-surface-as-a-state-vector),
**Coerce booleans through `> 0.5`** (§2.2),
[Branchless over divergent branches](#32-prefer-branchless-arithmetic-over-uniform-branching),
and [The canvas-2d preview is not your scene](#36-the-canvas-2d-preview-adapter-is-not-your-final-render).

---

## 1 — Control design

A scene's `CONTROLS` array is the public API your scene exposes to
operators, snaps, presets, OSC, MIDI, and the in-app preview adapter
all at once. The fields are documented in [Scene Workflow](scene-workflow.md);
this section covers what to do with them.

### 1.1 Pick the smallest control surface that exposes intent

Every control you declare becomes:

- a row in Perform's Scene Controls panel,
- a slot in every preset captured against this scene,
- a value persisted into every Snap,
- a route under `/chroma/control/<name>` for OSC operators,
- a learnable target for MIDI bindings.

That's five surfaces multiplied by the count. Five controls is fine.
Twenty is operator-hostile. Thirty is unusable in Perform.

A useful sieve: **if two controls always move together in your own
testing, fold them into one.** A `tint` colour with a per-channel
`tint_r` / `tint_g` / `tint_b` triplet adjacent to it is one control,
not four. A `position` xy plus a separate `position_x` and
`position_y` is one xy control, not three.

### 1.2 Name controls for the operator, not the shader

The `NAME` field is what the operator reads at perform-time. Two
rules that age well:

- **Pick a short verb-or-noun phrase, lower-case, snake-cased.**
  `bass_pulse`, `palette_hue`, `tunnel_speed`. Avoid GLSL-internal
  spellings like `u_pulse_amp_norm`.
- **Use the `DESCRIPTION` field for the longer story.** Perform
  surfaces `NAME` as the row label; the description shows up in
  tooltips and in the operator's Help panel. There is no character
  budget on `DESCRIPTION`; treat it like a one-sentence
  operator-facing comment.

The `LABEL` legacy field is preserved for ingest of reference scenes
but does not need to be authored on new scenes; `NAME` carries.

### 1.3 Snap-friendliness and the control surface as a state vector

The Snap system captures the full vector of every control's current
value at the moment you save the slot. Recalling a slot writes every
captured value back. Presets behave the same way under Wave 2.4
PERF.C semantics: Cmd-click captures, click recalls. Playlists
transition between presets across scene boundaries.

The implication: **every author-facing control is part of the scene's
state vector**. Two anti-patterns follow:

- **Don't include "moment-of-author" controls** that you don't want
  operators to capture into Snaps. A debug `show_uv` toggle, a one-
  shot `randomize_seed` bang, a developer-only `wireframe` flag: drop
  these before publishing or move them behind your authoring branch.
- **Don't bake operator-facing logic into uniforms the scene
  hot-reloads under.** A control that takes effect only on scene
  reload is silently broken under Snap recall — the recalled value
  will not match the visible state. Make controls react every frame,
  not at boot.

### 1.4 Use `UI_GROUP` to keep Perform legible

Controls without a `UI_GROUP` land in the default panel and stack
top-to-bottom in declaration order. For scenes with more than ~6
controls, declare an explicit `UI_GROUP` per cluster. Perform
respects the groups as collapsible panels under Scene Controls; the
Library scene-detail pane shows the group taxonomy too.

A workable convention:

- `appearance` — colours, palettes, tint, alpha.
- `motion` — speed, rotation, warp, drift.
- `audio` — per-band gains, reactivity multipliers.
- `geometry` — scale, position, count, density.
- `meta-output` — the four canonical `IS_META` flags (mirror, invert,
  global tint, master gain).
- `meta-pipeline` — anything else that affects the global pipeline
  rather than the scene's intrinsic look.

The strings are operator-facing prose; pick what reads cleanly in
your scene's domain. Stay consistent across scenes if you ship a
library.

### 1.5 The `IS_META` flag and the `_Reactivity` meta-gate

`IS_META: true` on a control signals "this is a global pipeline
adjustment, not an intrinsic scene parameter." The fixture set in
`engine/scene/tests/fixtures/scenes/meta-controls.chroma` enumerates
the canonical four:

- `meta_mirror` (toggle)
- `meta_invert` (toggle)
- `meta_tint` (color)
- `meta_gain` (slider, `[0, 2]`)

Beyond those four, a separate first-class meta-control —
`_Reactivity` — multiplies every audio uniform globally before the
shader reads it. It's part of the Advanced edition meta-gate per the
catalog and is what gives operators a single knob to turn down
audio response without rewriting every shader's mix factor.

**Pattern.** Author your shader against the raw audio uniforms;
reach for an `IS_META` `audio_gain` slider local to your scene
**only** if you need scene-specific trim distinct from the global
`_Reactivity` knob. Otherwise leave the global meta-knob to do its
job.

### 1.6 Range, default, and the "interesting zone"

Three patterns that pay for themselves:

- **Set `MIN` and `MAX` to the range you actually want the operator
  to explore.** A slider `[0, 1000]` where every interesting value
  lives in `[0, 10]` wastes the operator's fine motor control.
- **Pick `DEFAULT` at the most-compelling-looking sample of the
  range, not the midpoint and not zero.** The operator's first
  encounter with your scene is the default. If `0.0` is dead-silent
  black, that's a bad first impression.
- **Avoid exact-zero defaults on multiplicative axes.** A `gain` or
  `intensity` slider that starts at `0.0` makes the scene look
  broken at load time. Start at `0.5` and let the operator dial
  down.

### 1.7 Curves: smooth variants, knobs vs sliders

The TYPE field accepts the base kind plus a smoothing variant after
the first whitespace token (`slider smooth`, `bang smooth`,
`xy smooth`). The base kind drives the schema; the variant drives
the runtime smoothing behaviour. Specifically:

- **`slider` vs `knob`** — the engine treats both as scalar inputs
  through the same pipeline. Pick `knob` when the visual affordance
  in Perform should be rotary (small parameter, fits the dense
  scene-controls grid), `slider` when linear travel reads more
  intuitive (gains, levels, anything 0-to-1).
- **`slider smooth` etc.** — apply the smoothing variant to any
  control the operator will sweep aggressively. The smoothing
  removes step-jaggies when the value is bound to MIDI faders or
  swept via OSC at high update rate.
- **`bang` vs `bang smooth`** — momentary triggers. Smoothing on
  `bang` decays the trigger toward zero with a configurable
  half-life. Without smoothing, the trigger fires for exactly one
  frame.

The `PARAMS` field on a control carries variant-specific smoothing
payload (target half-life, smoothing window, etc.) when the variant
uses one.

### 1.8 How the preview adapter consumes named controls

The Perform / Library / Author preview tiles render through the
canvas-2d procedural adapter (`SceneCanvas.svelte`), not through a
GPU GLSL pipeline. The preview is wireframe-faithful for procedural
shapes but is **not** rasterising your scene's actual shader. The
ADR-011 GPU pipeline lands in Phase-2+; until then, the preview
adapter reads three named pre-multiplier control names verbatim:

- **`speed`** (slider in `[0, 1]`, nominal `0.5`) pre-multiplies the
  renderer's time argument. `0.0` freezes the preview, `1.0` runs at
  2x.
- **`intensity`** (slider in `[0, 1]`, nominal `0.5`) pre-multiplies
  the renderer's audio-level argument with the same 2x scaling.
- **`tint`** (vec3 or vec4 in `[0, 1]` per channel) draws a
  translucent overlay over the procedural vignette at 20% of the
  supplied alpha.

Plus four control-coverage extras: `invert`, `mirror`, `hor_mirror` /
`horizontal_mirror`, `vert_mirror` / `vertical_mirror`. These render
as post-pass overlays at the canvas-2d adapter level.

**Pattern.** If you want your scene's preview tile to "do the right
thing" without operators having to imagine the GPU render, name your
top-level speed / intensity / tint controls exactly those three
spellings. The preview adapter will respect them. Other controls
still dispatch through the runtime command bus to the engine —
they're just not visualised in the canvas-2d preview.

---

## 2 — GLSL pitfalls

This section catalogues mistakes that compile cleanly but produce
silently-wrong visuals at runtime. Read [Chromaticity Uniforms](chroma-uniforms.md)
for the canonical uniform vocabulary before reaching here for
debugging.

### 2.1 A typo at the uniform declaration site silently binds zero

The runtime does not perform per-binding casing fixups. If your
shader declares `uniform float chroma_audiorms;` (lowercase `r`)
instead of `chroma_AudioRMS`, the binding adapter has no symbol
to publish into. The compile is clean. The uniform reads zero
forever. The scene renders dark.

**Pattern.** Copy uniform names from the
[Chromaticity Uniforms reference](chroma-uniforms.md) verbatim. The
scene loader surfaces a warning at load time when a declared
uniform name fails to resolve against the catalog; check the load
diagnostics on every fresh declaration. Don't rely on visual
inspection — a uniform binding zero often looks like the scene "just
needs more audio."

### 2.2 Coerce toggles through `> 0.5`, not `== 1.0`

GLSL toggles arrive as `float` uniforms (0.0 or 1.0 on disk). The
trip from operator → IPC → JSON → uniform → shader passes through
several float coercions; a value that started life as 1.0 may
arrive as 0.9999937 by the time the shader reads it. Comparing for
strict equality against 1.0 produces sporadic flickering when the
toggle is "on but reads slightly less than perfect."

```glsl
// FRAGILE — equality against a coerced float
if (u_invert == 1.0) {
    col = vec3(1.0) - col;
}

// ROBUST — half-range threshold
if (u_invert > 0.5) {
    col = vec3(1.0) - col;
}
```

The same rule applies to `if (audio_band == 0.0)` checks; use
`< 1e-6` or similar. CPU integer comparisons are not GPU float
comparisons; thinking in epsilons is the discipline.

### 2.3 Integer-vs-float coercion at uniform boundaries

GLSL implicitly promotes integer literals in expressions, but the
rules surprise people. A common pitfall:

```glsl
// WRONG — integer division, returns 0
float t = 1 / 2;

// RIGHT — float division, returns 0.5
float t = 1.0 / 2.0;
```

This bites authors who copy patterns from CPU languages where
literal integer division promotes. In GLSL, `1 / 2` is `0`, full
stop, and many downstream errors look like "the scene is dark" or
"the colours desaturated."

Similarly, when sampling `chroma_AudioFFT[i]`, declare `i` as `int`
explicitly:

```glsl
int idx = int(floor(t * 8.0));
idx = clamp(idx, 0, 7);
float band = chroma_AudioFFT[idx];
```

A `float`-indexed array access compiles on some GLSL versions and
fails on others; the `int` cast is portable.

### 2.4 Out-of-bounds array sampling

`chroma_AudioFFT[8]` has exactly eight bands, indices 0..7. Reading
`chroma_AudioFFT[8]` (the off-by-one) is undefined behaviour: some
drivers return 0, some return the next memory cell, some kernel-
panic the GPU on tiles where unused registers carry old fragment
state. Always `clamp(idx, 0, 7)` before indexing.

Sampler textures (`syn_Spectrum`, `syn_LevelTrail`) have the same
constraint: the sampler API clamps automatically at the GL level,
but the `u` coordinate you feed needs to be in `[0, 1]` for
documented behaviour. Wrapping mode is sampler-default
`GL_CLAMP_TO_EDGE` unless you opt into something else.

### 2.5 The all-zero baseline is real

Every `chroma_*` and `syn_*` audio uniform reads zero during the
no-audio-yet startup window. The first audio tick arrives some tens
of milliseconds after the scene loads. A shader that divides by
`chroma_AudioRMS` without an epsilon guard produces inf / NaN /
black-screen on the first frame.

```glsl
// FRAGILE — division by zero on first frame
vec3 col = base / chroma_AudioRMS;

// ROBUST — saturating fallback
float rms = max(chroma_AudioRMS, 1e-3);
vec3 col = base / rms;
```

The same pattern applies to `chroma_AudioPeak`, `syn_Level`,
`syn_BPMConfidence`, and any time-accelerator (`syn_BassTime` etc.)
that decays toward zero on silence.

### 2.6 Alpha pre-multiplication caveats

Chromaticity scenes write to a 32-bit RGBA target. The compositor
expects **non-pre-multiplied alpha** by default — i.e. your
`gl_FragColor` carries the colour you want at full opacity and the
alpha channel carries the blend factor. Authors who copy a pattern
that pre-multiplies (multiplying RGB by A inside the shader) will
see their scene appear dim when alpha is below 1.0.

```glsl
// NON-PREMULTIPLIED (default)
gl_FragColor = vec4(scene_rgb, scene_alpha);

// PRE-MULTIPLIED (rare; use only with a matching blend mode)
gl_FragColor = vec4(scene_rgb * scene_alpha, scene_alpha);
```

If you need pre-multiplied (compositing-over with HDR sources,
post-FX that operates in linear-light), document it in the scene's
`DESCRIPTION` field so the operator knows what they're consuming.

### 2.7 `gl_FragCoord` is in pixels, not normalised

A new author's first instinct is often:

```glsl
// WRONG — gl_FragCoord goes 0..width / 0..height, not 0..1
vec2 uv = gl_FragCoord.xy;
```

You want one of:

```glsl
// Normalised, anisotropic — uv ranges 0..1 in each axis
vec2 uv = gl_FragCoord.xy / iResolution.xy;

// Normalised, aspect-corrected — centre at (0,0), keep y in [-0.5, 0.5]
vec2 uv = (gl_FragCoord.xy - 0.5 * iResolution.xy) / iResolution.y;
```

The aspect-corrected form is what `audio-reactivity.md`'s worked
example uses; it is the canonical shape for scenes that draw
centred radial geometry.

### 2.8 Hot-reload preserves `chroma_time`, not your accumulators

Live-reload (`docs/37`) preserves the operator's control values
across the reload by re-applying the captured `PreservedState`. It
does **not** preserve in-shader-state like a feedback accumulator
in a multipass `FLOAT` target — that target gets cleared and your
post-reload first frame reads zero.

**Pattern.** Don't write logic that depends on the previous frame's
contents surviving a hot-reload. If your scene needs warm-up state,
seed it from a stable noise function of `chroma_time` or `syn_Time`
so the same pattern reproduces on every reload.

---

## 3 — GLSL performance

A 60 Hz render budget gives you ~16.67 ms per frame for everything:
geometry, multipass composition, post-FX, plus the scheduler tick
that drains audio and IPC. On a mid-range integrated GPU, that
budget is generous for most scenes and tight for the worst-case
ones. This section catalogues where cost actually accumulates.

### 3.1 Texture sampling is the largest single per-fragment cost

A modern GPU's texture sampler is fast but not free. A shader that
does eight `texture()` calls per fragment will spend most of its
budget in the sampler. The canonical mitigations:

- **Sample once, reuse.** If you need `chroma_AudioFFT[3]` twice in
  your shader, store the first read into a local `float` and reuse.
  The compiler may CSE for you on simple cases, but for sampler
  reads through `syn_Spectrum` it almost certainly will not.
- **Prefer the array uniform over the sampler.** Reading
  `chroma_AudioFFT[i]` is a uniform fetch, not a sampler read.
  Reading `texture(syn_Spectrum, u).r` is a sampler read. Use the
  array when you only need the eight-band shape; use the sampler
  when you need the full 256-or-512-bin spectrum.
- **Avoid texture sampling inside loops with variable bounds.**
  GPU drivers can hoist loop-invariant samples; they can't always
  hoist conditional ones.

### 3.2 Prefer branchless arithmetic over uniform branching

GPU fragments execute in lockstep across a warp / wavefront
(typically 32 or 64 lanes). When an `if` branches differently across
lanes, both sides of the branch execute and the results are masked
— the "diverged" cost is the sum, not the average.

A branchless pattern is often cheaper:

```glsl
// DIVERGENT (per-fragment branch on a control value)
vec3 col;
if (u_audio_gate > 0.5 && bass > 0.1) {
    col = pulse_color * bass;
} else {
    col = base_color;
}

// BRANCHLESS (mix on the predicate)
float gate = step(0.5, u_audio_gate) * step(0.1, bass);
vec3 col = mix(base_color, pulse_color * bass, gate);
```

`mix` / `step` / `smoothstep` are sampler-grade fast paths on
nearly every GPU. Use them in preference to per-lane control flow
whenever the predicate is cheap.

The exception: when the branch is genuinely uniform across the
warp (e.g. a toggle uniform that's the same for every fragment),
GLSL compilers can collapse it to a true branch. That case is free.
The expensive case is per-fragment divergence.

### 3.3 `iTime` / `chroma_time` cost vs `iFrame`

Both are scalar uniforms — the per-frame read cost is the same.
What matters is what you do with them:

- **`chroma_time` (float seconds)** is wallclock-aligned. Use it
  for animation phase, oscillators, wave functions. It is
  monotonically increasing within a session.
- **`iFrame` (integer frame counter)** is useful for stochastic
  patterns that need to sample a different pseudo-random value per
  frame. Modular arithmetic on `iFrame` is your friend for
  temporal dither, accumulation, or frame-blending tricks.

The performance impact comes from the shape of the math you build
on them. A `sin(chroma_time * 6.283 * freq)` is one transcendental
per fragment; if you need that pattern many times, compute it once
into a local and reuse. A nested-trig phase function like
`sin(cos(sin(t * a) * b) * c)` may look concise but executes three
transcendentals per fragment.

### 3.4 Lygia helpers vs hand-rolled math

Chromaticity ships [Lygia v1.1.4](https://github.com/patriciogonzalezvivo/lygia)
vendored under `third_party/lygia/` (per
[Shader Sourcing Research](#references)). Lygia is a utility
library — noise, SDF, blend, palette, raymarch helpers — that
composes against the `chroma_*` uniforms cleanly.

**When to reach for Lygia:**

- Noise functions: `lygia/generative/cnoise.glsl` (classic Perlin),
  `lygia/generative/snoise.glsl` (Simplex), `lygia/generative/fbm.glsl`
  (fractional Brownian motion), `lygia/generative/voronoi.glsl`
  (cellular). The vendored implementations are reference-quality
  ports of canonical algorithms.
- SDF primitives: `lygia/sdf/` carries `sphereSDF`, `boxSDF`, etc.
  Hand-rolling these is a foot-gun.
- Colour conversions: `lygia/color/` carries the standard sRGB /
  linear / HSL / OKLAB / palette helpers.

**When to skip Lygia and hand-roll:**

- One-shot math that's smaller than the include. A single
  `smoothstep` doesn't need a helper file.
- Performance-critical inner loops where the call boundary cost
  (or per-call uniform setup, if the helper expects extra
  arguments) is comparable to the helper's body.
- Anything where you need exact control over WGSL transpile
  semantics (Phase-2+ when the GPU pipeline lands).

**License-trial caveat.** Lygia ships under Prosperity-3.0.0 with a
30-day commercial trial. The trial does not bind Chromaticity's
shipped product — the license election lands at the Phase-5
`EV-LICENSE-AUDIT` gate. Treat Lygia as a foreground tool during
authoring; pre-stage the MIT-licensed alternative (webgl-noise +
glslify + three.js post-FX subsets per the
[Shader Sourcing Research](#references) shortlist) if you ship a
scene library you intend to redistribute.

### 3.5 Multipass composition costs scale with pass count

The scene schema's `PASSES` array declares N render passes that
compose into the final frame. Each pass:

- Pays a framebuffer-bind cost (~tens of microseconds on a modern
  GPU).
- Pays a texture-allocation cost on first use (one-time per pass).
- Reads the previous pass's output as a `sampler2D` texture.

Three passes is generally cheap. Eight is generous but manageable.
Twenty is asking the operator's integrated-GPU laptop to drop frames.

**Pattern.** Combine passes where the math allows it. If pass 2
does a horizontal blur and pass 3 does a vertical blur, that's a
classic separable filter and is a clean two-pass design — keep it
two. But if pass 4 does a colour grade that could fold into the
final composite of pass 5, fold it.

The `FLOAT` per-pass flag (preserved in pass `extensions` at
slice-3a) marks a pass as needing 16- or 32-bit per-channel
precision; that doubles or quadruples the framebuffer memory
footprint compared to a default 8-bit-per-channel target. Use
`FLOAT` only when you need HDR or feedback accumulation precision.

### 3.6 The canvas-2d preview adapter is not your final render

The preview tiles in Perform / Library / Author are rendered by
`src/lib/widgets/SceneCanvas.svelte` against the 16 procedural
renderers in `scene-renderers.ts`. This is an **operator preview
adapter**, not a GPU rasterisation of your shader. Specifically:

- It runs on the main UI thread via `requestAnimationFrame` and
  HTML `CanvasRenderingContext2D`.
- It resolves a `SceneKind` from your scene's metadata (title,
  tags, category) and picks one of 16 procedural patterns to
  display.
- It honours the three named pre-multipliers (`speed`,
  `intensity`, `tint`) plus the four mirror / invert post-pass
  flags described in §1.8 above.
- It reads the live audio snapshot when present so the preview
  pulse responds to real audio.

What it does **not** do:

- Execute your `main.glsl` source.
- Render your multipass chain.
- Resolve `lygia/` includes.
- Respect any uniform other than the named pre-multipliers above.

ADR-011 Phase-2+ work will replace the preview adapter with a real
wgpu-backed mini renderer that does run your shader. Until then,
the preview communicates "this is what the scene's about" rather
than "this is what your shader literally outputs at this moment."

**Pattern.** Don't tune your scene's look against the preview tile.
Tune it against the live render (Perform's main canvas, Author's
live-preview pane in non-preview-tile mode). Use the preview tile
as a visual table-of-contents.

### 3.7 Profile against the live runtime, not synthetic frames

The `EV-PERF-BUDGETS` work shipped a synthetic-workload bench
harness for the engine subsystems, but the only honest signal for
"does my scene hit 60 Hz" is the Perform top-bar FPS readout under
your actual scene workload. Two posture rules:

- **Measure with audio active.** A silent-mic test underestimates
  the analysis-thread cost; the scheduler tick that drains the
  audio snapshot is part of your per-frame budget.
- **Measure with the controls you intend to ship.** A control
  that defaults to a cheap value (`intensity = 0`) hides cost
  that the operator will pay later when they dial it up. Sweep
  the controls through their declared range before declaring the
  scene shippable.

### 3.8 Cheap defaults, expensive worst cases

A useful sanity check: the cheapest possible state of your scene
(all sliders at zero, all toggles off) should hit 60 Hz on an
integrated GPU. The most expensive state (all sliders at max, all
toggles on) should still degrade gracefully — drop to 30 Hz, not
freeze.

If a particular control flips the scene from cheap to crash-
expensive, that's a sign the control needs a clamp at the
shader-author boundary, not the operator boundary. A
`raymarch_steps` slider declared `MAX = 200` is the operator
choosing to render at 5 fps; declare `MAX = 64` and let your
shader's algorithmic budget do the right thing within that
range.

---

## References

- [Scene Workflow](scene-workflow.md) — `.chroma` package shape and
  scene-author lifecycle.
- [Chromaticity Uniforms](chroma-uniforms.md) — the canonical
  `chroma_*` / `syn_*` uniform vocabulary.
- [Audio Reactivity](audio-reactivity.md) — the audio path,
  snapshot fields, and the four-band reduction.
- [Importing Shadertoy Scenes](import-shadertoy.md) — the
  `iTime` / `iResolution` / `iChannel0` adapter for ShaderToy
  ports.
- [Settings Reference](settings-reference.md) — operator-side
  preferences that affect every scene.

External (links only; treat the body of these resources as
attribution material, not ship-time content):

- Inigo Quilez's articles at `iquilezles.org/articles/` — MIT-
  licensed primitives for SDF, noise, palette, raymarching.
- The Lygia shader library at `github.com/patriciogonzalezvivo/lygia`
  — currently trial-vendored, see the project's
  in-repo `docs/56-shader-sourcing-research.md` for the license
  posture.
- The Book of Shaders — excellent pedagogical reference. The
  example code in that repository ships under a proprietary
  license that bars vendoring; read for learning, don't copy
  into your scene.

---

*Authored from `engine/scene/src/schema.rs` (control schema
shape), `engine/scene/tests/fixtures/scenes/` (representative
scene fixtures), `third_party/lygia/` (vendored shader library),
`src/lib/widgets/SceneCanvas.svelte` (canvas-2d preview adapter),
`docs/56-shader-sourcing-research.md` (Lygia license posture),
and `docs/comparison/features/ssf-format.md` (best-practices
parity gap). Clean-room per
`docs/29-cleanroom-legal-policy-baseline.md`.*
