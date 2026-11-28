---
doc-type: howto
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Importing Shadertoy Scenes

Chromaticity ships a one-way Shadertoy-to-`.chroma` converter that lets
authors port single-pass Shadertoy fragments directly into the Chromaticity
authoring environment. The converter wraps the Shadertoy body in a
chromaticity-native shell, declares the canonical Shadertoy uniforms so
the body code compiles unchanged, and emits a `.chroma` package that the
runtime then loads through the standard scene pipeline.

## What the converter accepts

The input is a Shadertoy GLSL body containing:

```glsl
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    // body
}
```

The converter wraps the body in a `void main()` entry that targets the
naga GLSL frontend used by the engine's render path. The wrapper preamble
declares the canonical Shadertoy-spelled uniforms so the imported body
compiles without edits:

| Uniform | Purpose |
| --- | --- |
| `iTime` | Seconds since scene start |
| `iResolution` | Pixel size of the render target |
| `iTimeDelta` | Seconds since the previous frame |
| `iFrame` | Frame counter |
| `iMouse` | Mouse position and click state, Shadertoy-compatible |

The runtime binding from these Shadertoy spellings to the canonical
chromaticity-native reactive symbols (`TIME`, `RENDERSIZE`, …) is a
binding-layer concern — the imported scene works in place without
edits, and authors can migrate to the chromaticity-native symbols
later if they want.

## What the converter rejects or warns on

The converter is strict about features it cannot represent in a
single-pass `mainImage` shape. The rejection and warning surface is
tagged with author-readable diagnostics:

- **VR mode (`mainVR`)** — rejected; no plausible auto-mapping into a
  single-pass `mainImage`.
- **Cubemap entrypoint (`mainCubemap`)** — surfaced as
  `Warning::CubemapEntrypointDeferred`; deferred to a future
  multipass-import pathway.
- **Webcam iChannel mapping** — surfaced as
  `Warning::WebcamIChannelDeferred`; Chromaticity defers webcam binding
  to post-MVP. The body still compiles; the author can edit the
  `.chroma` package post-import to add a real binding.
- **Deprecated `texture2D(...)` calls** — auto-corrected to
  `texture(...)` and surfaced as
  `Warning::DeprecatedTexture2DAutoCorrected` so the author has a
  trail of every machine-applied edit.

The converter is **atomic**: the caller sees either a fully-populated
`<scene_id>.chroma` package or no package at all. Cleanup errors from
a partially-written package are swallowed so the originating I/O
cause stays surfaced in the diagnostics console.

## iChannel asset binding

Shadertoy scenes reference up to four `iChannel0`..`iChannel3` slots
backed by textures, cubemaps, video, audio, or the camera. The
converter accepts a per-channel mapping at import time; each declared
mapping causes the wrapper preamble to add a chromaticity-native
binding-slot uniform (`chroma_input_<channel>`) and to rewrite the
body's `iChannelN` references to that slot symbol before naga sees
the source. Channels referenced by the body but not mapped at import
emit `Warning::MissingIChannelMapping` so the author can either add
the mapping in the converter and re-run, or edit the `.chroma`
package post-import to bind the slot.

The `shadertoy-fixtures/` subdirectory under
`<AppBundle>/Resources/scenes/` ships the iChannel asset corpus the
import tests draw from — see [Library Workflow](library-workflow.md)
for how that subdirectory is excluded from the launchable scene set.

## After import

The output is a `.chroma` package matching the same on-disk shape that
the in-app scaffolder produces. Once written, the package surfaces in
the library through the standard discovery path and is loadable
through the standard scene pipeline described in
[Scene Workflow](scene-workflow.md). Imported scenes carry the format
tag `shadertoy-v1-import` so the operator can filter them in the
[library browser](library-workflow.md).

## What is not imported

The Shadertoy import path is intentionally narrow:

- **ISF scenes** — imported through a separate ISF-import path that
  is currently a reject-with-diagnostic surface; native ISF execution
  is deferred to V2.
- **Multi-buffer Shadertoy compositions** — out of scope for the V1
  import; the multipass-import pathway is a future-wave ADR.
- **Sound shaders** — Chromaticity is a visual runtime; Shadertoy
  sound-shader bodies are rejected.

## Capability gating

Shadertoy import is gated by the `authoring.import` capability. With
no entitlement token present, the import action is disabled in the
operator HUD with the standard locked-row affordance; see
[Settings Reference](settings-reference.md) for the entitlement model.

---

<!-- Clean-room authored from engine/authoring/src/import_shadertoy.rs module-level doc-prose, docs/38-scene-library-and-import-spec.md sections 5.2 and 5.2a, and the EV-IMPORT-SHADERTOY row in docs/26-evidence-artifact-registry.md; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
