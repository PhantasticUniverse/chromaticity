<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# External GLSL shader source research

**Generated:** 2026-05-11 (Wave 2.2 Lane R.2). Anchor: `main` HEAD post-Wave-2.1
closure. **Scope:** evaluate clean-room-compatible upstream GLSL shader
corpora so the AO can set the V1 GPU-shader baseline before Phase-2+
ADR-011 render-pipeline work lands.

This is a **research artifact**, not a gate or a binding decision. It does
not vendor any source, does not edit `deny.toml`, does not negotiate a
license. It catalogues what is actually usable under Chromaticity's
proprietary distribution posture, so when ADR-011 ships and Lane S
(speculative naming) needs a curated corpus, the work can start from a
vetted shortlist rather than a fresh discovery pass.

## 1 — Licensing baseline

Chromaticity ships under `LicenseRef-Chromaticity-Proprietary`
(`SPDX-FileCopyrightText: 2026 Chromaticity`). Per `docs/29-cleanroom-legal-policy-baseline.md`
Sec.Bundled GLSL Library Handling and the `deny.toml` allowlist, the
following upstream licenses are compatible with shipped binaries:

**Permissive (safe to bundle, attribution may be required):**

- `MIT`, `MIT-0`
- `BSD-2-Clause`, `BSD-3-Clause`
- `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`
- `ISC`
- `Zlib`
- `Unicode-3.0`, `Unicode-DFS-2016`
- `CC0-1.0` (public domain dedication)
- `Unlicense` (effectively CC0; not yet in `deny.toml` allowlist —
  trivial `policy`-scoped extension if a sourced corpus needs it)
- Blue Oak Model License 1.0.0 (not in `deny.toml` allowlist; would need
  same trivial extension)

**Weak copyleft (case-by-case):**

- `MPL-2.0` — per-file copyleft; bundling permitted, modifications to
  MPL-licensed files must remain MPL.

**Trial-narrow (Lygia ONLY):**

- `LicenseRef-Prosperity-3.0.0` — non-commercial default + 30-day
  commercial trial. Permitted ONLY for `third_party/lygia/**` per the
  comment in `deny.toml` lines 85-95. Final commercial-use election
  deferred to Phase-5 `EV-LICENSE-AUDIT`.

**INCOMPATIBLE — DO NOT INCLUDE under any form:**

- `GPL-2.0`, `GPL-3.0`, `AGPL-3.0` (entire GPL family — viral copyleft).
  Linking with proprietary code is a license violation regardless of
  whether the GPL code is shipped verbatim or modified.
- `LGPL-2.1`, `LGPL-3.0` — compatible only for dynamic linking with
  unmodified library binaries; Chromaticity's static-binary distribution
  posture (Tauri 2 single-binary per platform per `ADR-018`) precludes
  LGPL inclusion for shader-side libraries. Pattern extraction OK
  (per `docs/54` posture for ffmpeg / VLC); source copy NOT OK.
- `CC-BY-NC-*` family (any NonCommercial Creative Commons variant) — bars
  commercial distribution. Chromaticity is a commercial product per
  `docs/29` brand/trademark review.
- `CC-BY-NC-SA-3.0` — Shadertoy default, see §2.4.
- `CC-BY-3.0` / `CC-BY-4.0` for code (not docs) — the Creative Commons
  organisation itself recommends against using CC-BY for code due to
  patent / sub-licensing ambiguities (`docs/47-ui-port-plan.md` cites
  this for visual-asset attribution; the same posture applies to
  shader source).
- `Prosperity-3.0.0` for any vendor OTHER than Lygia trial.
- Unspecified / "all rights reserved" — default-blocked per `docs/29`
  Sec.Allowed-License Policy Baseline ("Any unknown license class is
  treated as blocked pending legal review").

**Operational implication.** For every individual shader file proposed
for inclusion, a clean-room reviewer must verify (a) the source
repository's top-level LICENSE, (b) any per-file SPDX header, and
(c) for ShaderToy-sourced material, the in-shader license comment
header. The default of "no header on a ShaderToy shader" is
CC-BY-NC-SA-3.0 — the per-shader audit is mandatory, not optional.

## 2 — Source candidates

### 2.1 Lygia (currently trial-vendored)

- **Identity:** Patricio Gonzalez Vivo + community.
  `github.com/patriciogonzalezvivo/lygia`. Trial-vendored at
  `third_party/lygia/v1.1.4` (commit `28f220b6045cbbd38ad6e23a69d21c5e67070cdd`),
  896 shader files spanning `animation/`, `color/`, `distort/`, `draw/`,
  `filter/`, `generative/`, `geometry/`, `lighting/`, `math/`,
  `morphological/`, `sample/`, `sdf/`, `simulate/`, `space/`.
- **License:** `LicenseRef-Prosperity-3.0.0` (non-commercial + 30-day
  commercial trial). Dual-licensed with a Patron License for sponsors
  (separate commercial tier requires direct contact with maintainer).
- **Compatibility verdict:** TRIAL-ONLY. The 30-day commercial trial
  window does not bind a proprietary shipped product — final election
  is deferred to Phase-5 `EV-LICENSE-AUDIT` per `docs/29` row.
- **Corpus quality:** ~896 files. Actively maintained. High code
  quality, cross-platform (GLSL, HLSL, Metal, WGSL, GLES, CUDA). Cited
  in the reference application's `lib/lygia/` as well, which the AO
  treated as a parity signal at trial-vendor time.
- **Vendoring shape:** already vendored flat-copy from release tag.
- **Integration cost:** Lygia is a **utility library** (noise, SDF,
  color, raymarch helpers, blend modes) — not a corpus of complete
  scenes. It composes with first-party scenes via the Chromaticity
  helper shims (`_pulse`, `_fbm`, `_noise`, `_rotate` per `docs/29`
  table) but is not a turnkey "play with shaders" source.

**Recommendation.** Continue the trial through Phase-2/3 dev. Open
license dialogue with Patricio (the Patron License is the obvious
commercial path; alternatively the upstream's MIT contribution-back
clause per `LICENSE.md` line 25 means a Chromaticity-internal
re-implementation of needed helpers could be contributed back to
upstream under MIT). Have the alternative — first-party clean-room
helper shim corpus per `docs/29` line 104 — drafted by Phase-3
EXIT so the decision at Phase-5 `EV-LICENSE-AUDIT` is binary
(elect-trial-commercial vs ship-clean-room-shims), not blocking.

### 2.2 Inigo Quilez (iquilezles.org + his Shadertoy account)

- **Identity:** Inigo Quilez (co-founder of Shadertoy). Personal site
  `iquilezles.org` + Shadertoy profile `shadertoy.com/user/iq`.
  ~400+ catalogued articles + ~250+ public Shadertoy shaders.
- **License:** Per his personal site, **technical code snippets are
  released under MIT**. The "mathematical/shader art" (i.e. the
  finished compositional pieces) requires separate licensing. For
  individual Shadertoy shaders the in-shader header is authoritative
  — many of his Shadertoy entries carry explicit MIT comments (e.g.
  the basic Voronoi `MslGD8` from 2013-05-22). Per-shader audit is
  mandatory.
- **Compatibility verdict:** USABLE for code-snippet helpers + any
  individual Shadertoy entry carrying an explicit MIT/CC0 header.
  TRIAL-ONLY / case-by-case for compositional pieces lacking an
  explicit header (default falls back to Shadertoy CC-BY-NC-SA — see
  §2.4).
- **Corpus quality:** Reference-quality. IQ is the canonical author
  for raymarching primitives, SDF operations, noise variants,
  procedural geometry, palette generation. Code style is concise but
  not always commented; suitable for vendoring with attribution
  preserved.
- **Vendoring shape:** cherry-pick subset (~30-50 high-value
  snippets / articles) into `third_party/iq/<topic>/<name>.glsl` with
  per-file SPDX-Identifier-MIT headers + a top-level
  `third_party/iq/AUTHORS.md` + `third_party/iq/PROVENANCE.md`.
- **Integration cost:** Most IQ snippets are vanilla GLSL ES 3.0
  compatible and translate cleanly to WGSL via Naga (see §4.7).
  Compositional Shadertoy pieces use the Shadertoy uniform vocabulary
  (`iTime`, `iResolution`, `iMouse`, `iChannel0..3`) — needs a thin
  adapter to `chroma_*` (see §3.4).

**Recommendation.** Highest-priority single-author source. Start with
~15 raymarching primitive snippets + ~10 noise/FBM variants + ~5
palette/color articles, all from his `iquilezles.org/articles/` series
that carry the explicit MIT statement. Defer the compositional
Shadertoy ports until per-shader audits clear them.

### 2.3 Three.js examples/jsm/shaders (post-processing pack)

- **Identity:** mrdoob + the three.js community.
  `github.com/mrdoob/three.js/tree/dev/examples/jsm/shaders`.
- **License:** `MIT` at the repository root, applying to all
  subdirectories by default (per `docs/29` clean-room SOP, the
  per-file check still applies). 52 named shader files at
  `examples/jsm/shaders/` covering tonemapping, blur, bloom, FXAA,
  SMAA, SSAO, bokeh, color correction, halftone, kaleidoscope, RGB
  shift, sobel-edge, sepia, vignette, water refraction, etc.
- **Compatibility verdict:** USABLE (with one historical caveat).
  Note that `FilmShader.js` was historically flagged as CC-BY-3.0 in
  a three.js issue (the upstream issue is `mrdoob/three.js#26571`)
  — current dev branch appears to have removed that header but a
  per-file SPDX audit remains mandatory before vendoring any
  individual shader from this set.
- **Corpus quality:** Excellent. Battle-tested by the entire
  three.js / React-Three-Fiber ecosystem; well-commented; consistent
  uniform-naming convention; covers ~all classical post-FX needs.
- **Vendoring shape:** cherry-pick to `third_party/threejs/shaders/`.
  Each file is small (40-200 lines) and self-contained; no transitive
  three.js JS dependency.
- **Integration cost:** Vanilla GLSL — these are written for
  three.js's `THREE.ShaderMaterial` convention (`uniform sampler2D
  tDiffuse; uniform vec2 resolution; varying vec2 vUv;`). Adapting to
  the Chromaticity `chroma_*` convention is mechanical: ~1 hour per
  shader to remap inputs. Naga WGSL transpile is clean for the
  ~95th-percentile of these (the few that use `texelFetch` with
  integer offsets or non-uniform branching may need WGSL
  manual-touch).

**Recommendation.** Second-highest priority. Vendor 15-20 of the most
useful post-processing shaders (FXAA, SMAA, Bloom, Bokeh,
ColorCorrection, RGBShift, Halftone, Sobel, Sepia, Vignette,
Kaleido, HueSaturation, Tonemapping) as `third_party/threejs/`
with a per-file SPDX-Identifier-MIT review pass.

### 2.4 ShaderToy individual MIT-relicensed shaders

- **Identity:** various authors. `shadertoy.com`.
- **License:** **Default is CC-BY-NC-SA-3.0** per the Shadertoy
  ToS — INCOMPATIBLE. Individual authors may relicense by placing
  an explicit license comment as the first lines of the shader
  source. The format the community has converged on is one of:

  ```
  // License: MIT
  // Copyright (C) <YEAR> <AUTHOR>
  ```
  or
  ```
  // CC0 - https://creativecommons.org/publicdomain/zero/1.0/
  ```

  In the absence of such a header, the shader defaults to
  CC-BY-NC-SA-3.0 and is NOT usable.

- **Compatibility verdict:** USABLE only for individual shaders that
  carry an explicit MIT/CC0/permissive header in the source comment
  block. Per-shader audit MANDATORY; no bulk vendor permitted.
- **Corpus quality:** Highly variable. Top-tier (IQ, Dave Hoskins,
  Fabrice Neyret, Shane, mrange) authors produce reference-quality
  work; long-tail shaders may not be portable.
- **Vendoring shape:** N/A directly — would need to be downloaded
  individually, audited, and dropped into a curated
  `third_party/shadertoy-curated/` directory with per-file SPDX
  headers + per-shader provenance (URL, capture date, author handle,
  in-shader license header verbatim quote).
- **Integration cost:** ShaderToy uses its own uniform vocabulary
  (`iTime`, `iResolution`, `iMouse`, `iChannel0..3`,
  `iChannelResolution[N]`, `iChannelTime[N]`, `iFrame`, `iDate`,
  `iSampleRate`). The Chromaticity scene system already has the
  shadertoy-import path via `EV-IMPORT-SHADERTOY` (`docs/38`
  §5.2a) — the `chroma_*` ↔ `i*` mapping is established. iChannel
  fixture-pack resolution closed Phase-4b on 2026-05-07 per
  `CLAUDE.md`. Per-shader audit is the cost, not the GLSL
  translation.

**Recommendation.** Treat ShaderToy as a sourcing pool for
case-by-case curation, NOT a corpus. Maintain a single
`third_party/shadertoy-curated/` directory; every entry has a
per-file `LICENSE-HEADER.txt` snippet quoting the author's
in-shader license verbatim + a `PROVENANCE.md` row recording the
shadertoy.com URL and capture timestamp.

### 2.5 The Library of Shaders

- **Identity:** Community-maintained, `the-library-of-shaders.github.io`.
- **License:** **`CC0-1.0`** — public domain dedication.
- **Compatibility verdict:** USABLE.
- **Corpus quality:** Modest size (the page lists vertex, fragment,
  and post-processing shaders with categories like Chromatic
  Aberration, Bloom, Vignette, Gaussian Blur — total count not
  cleanly stated). Education-oriented; less compositional/artistic
  variety than three.js or IQ.
- **Vendoring shape:** flat-copy to `third_party/library-of-shaders/`
  with the CC0 attestation file preserved.
- **Integration cost:** vanilla GLSL; clean translation to WGSL
  expected for the simple effects.

**Recommendation.** Vendor as a CC0 baseline of "named effect"
implementations. ~5-10 entries for tutorial/onboarding value.

### 2.6 The Book of Shaders (Patricio Gonzalez Vivo)

- **Identity:** Patricio Gonzalez Vivo (same author as Lygia) +
  Jen Lowe. `github.com/patriciogonzalezvivo/thebookofshaders`.
- **License:** **Custom proprietary, all rights reserved.** Per the
  repository LICENSE file: *"You cannot host, display, distribute or
  share this Work in any form, including physical and digital. You
  cannot use this Work in any commercial or non-commercial product,
  website or project."*
- **Compatibility verdict:** **INCOMPATIBLE.** Despite the educational
  framing and the same author as Lygia, the example code under this
  repository is NOT usable.
- **Notes for AO:** This is a meaningful surprise for the AO since
  "The Book of Shaders" is widely cited as the go-to tutorial corpus
  for GLSL. The pedagogical content is excellent but the licensing
  bars vendoring. The Lygia trial-vendor decision was correctly
  scoped to the upstream Lygia repo, NOT this one.

**Recommendation.** Cite as educational reference in `docs-corpus/`
help pages (a URL link is not vendoring) but DO NOT include any
source from this repository. If a specific algorithm or pattern in
the book is needed, a clean-room rewrite per `docs/29` SOP is the
only path.

### 2.7 Bonzomatic + the Shader Showdown demoscene archives

- **Identity:** Gargaj / Conspiracy + the demoscene live-coding
  community. `github.com/Gargaj/Bonzomatic`.
- **License:** **`Unlicense`** (public domain dedication, equivalent
  to CC0 for usage purposes).
- **Compatibility verdict:** USABLE for the **tool** if we ever
  wanted to embed live-coding in Chromaticity. For shader **corpus**,
  Bonzomatic itself ships NO bundled shaders — the tool is just the
  editor + renderer. The Shader Showdown competition archives at
  `livecode.demozoo.org` host per-event shaders but those are
  individually-authored without consistent licensing; per-author
  permission required to vendor any of them.
- **Corpus quality:** Showdown shaders are typically 25-minute
  improv pieces — visually striking, code quality variable, often
  uncommented.
- **Integration cost:** Bonzomatic-format shaders use a different
  uniform convention than ShaderToy (`fGlobalTime`, `v2Resolution`)
  — would need additional adapter work.

**Recommendation.** Defer. The signal-to-noise for vendoring is
poor at the corpus level (each shader needs an author email +
license grant), and the artistic value mostly duplicates what is
already accessible via the Shadertoy curated route.

### 2.8 Hydra (livecoding-visuals)

- **Identity:** Olivia Jack + community. `github.com/hydra-synth/hydra`.
- **License:** **`AGPL-3.0`** at the runtime level (`hydra-synth`
  uses AGPL).
- **Compatibility verdict:** **INCOMPATIBLE.** The AGPL viral copyleft
  reaches network-served code and is broadly incompatible with
  Chromaticity's proprietary distribution posture even if only the
  Hydra-shader-string corpus were vendored.
- **Notes for AO:** Hydra's procedural style (`osc().rotate().out()`)
  is a fragment-shader DSL transpiled to GLSL — even the DSL output
  carries the AGPL. Pattern-only research is fine (`docs/54` posture);
  no source copy.

**Recommendation.** Do NOT vendor any Hydra-derived shader. If the
declarative-DSL pattern is interesting for V2+ authoring UX, that's a
clean-room reimplementation, not a vendor.

### 2.9 Milkdrop preset libraries / projectM

- **Identity:** projectM is a Milkdrop-compatible visualizer
  (`github.com/projectM-visualizer/projectm`). The two main preset
  packs are `presets-cream-of-the-crop` and `presets-milkdrop-original`.
- **License:** projectM **runtime is `LGPL-2.1`** (INCOMPATIBLE for
  static-link distribution per §1). The **preset packs** carry a
  bespoke notice: *"Milkdrop presets were, in almost all cases, not
  released under any specific license. Theoretically, each preset
  author holds the full copyright on any released presets. Since the
  presets were freely released and have been used in so many packages
  and applications in the past two decades, it is safe to assume them
  to be in the public domain."* The repo also includes a
  removal-on-request clause.
- **Compatibility verdict:** TRIAL-ONLY / case-by-case. The "assume
  public domain" framing is a community convention, NOT a binding
  legal grant. A risk-averse legal review may default-block
  ("any unknown license class is treated as blocked" per `docs/29`
  Sec.Allowed-License Policy Baseline).
- **Corpus size:** Thousands of presets (the original Milkdrop
  legacy library is decades-deep).
- **Format problem:** Milkdrop presets are NOT GLSL. They use a
  custom DSL of "per-frame equations" (PEL — preset expression
  language) + a `.milk` file format. Rendering them requires either
  (a) linking projectM (LGPL-INCOMPATIBLE), or (b) writing a
  Chromaticity-side DSL → GLSL transpiler — a multi-month
  engineering effort comparable in scope to the entire
  `EV-IMPORT-SHADERTOY` import path.

**Recommendation.** Defer to V2+. The combined "legal soft-status +
custom-DSL transpiler effort" makes this a poor V1 fit. Document
in `docs-corpus/` as a potential import path. If demand surfaces
in tester feedback, the path of least resistance is to vendor only
the *visual concepts* (e.g. "audio-reactive flame", "tunnel zoom")
as clean-room original `.chroma` scenes, not the Milkdrop preset
files themselves.

### 2.10 Interactive Shader Format (ISF) — Vidvox

- **Identity:** Vidvox (creators of VDMX). `github.com/Vidvox/ISF-Files`.
- **License:** **`MIT`** at the repository level.
- **Compatibility verdict:** USABLE. The ISF-Files repo holds **~200+
  generators and filters** all under MIT.
- **Corpus quality:** Professional-grade. ISF is used in production
  VJ software (VDMX, MadMapper, Magic Music Visuals); shaders are
  battle-tested at performance time. Audio-reactivity, time control,
  parameter-binding metadata all baked in via the ISF JSON-header
  convention (essentially a strict superset of ShaderToy).
- **Vendoring shape:** flat-copy or selective vendor to
  `third_party/vidvox-isf/`. Per-file MIT header audit confirms each
  shader's licensing.
- **Integration cost:** ISF format = `{JSON header} + {GLSL body}`.
  The JSON header carries `INPUTS` (uniform metadata),
  `PASSES` (multipass shape), `IMPORTED` (resource bindings) — this
  is largely compatible with how Chromaticity's `.chroma` package
  carries control schema in `manifest.json`. Building an ISF →
  `.chroma` adapter is a moderate-effort task (estimated ~1-2 weeks
  engineering) but yields immediate access to 200+ professional
  shaders. ISF JSON metadata maps cleanly to `chroma_*` symbol
  catalog per `docs/36`.

**Recommendation.** Highest-value compositional corpus available.
Vendor ~30-50 shaders selectively (avoid duplicates of three.js
post-processing). The ISF → `.chroma` adapter belongs in the
authoring/import roadmap as a Phase-2 PR.

### 2.11 stackgl / glslify modular GLSL ecosystem

- **Identity:** Hugh Kennedy, Matt DesLauriers, Mikola Lysenko et al.
  `github.com/glslify/glslify`, `github.com/stackgl/*`,
  community npm modules prefixed `glsl-*`.
- **License:** **`MIT`** at the glslify root and across the major
  npm `glsl-*` modules (per-module check still required).
  Specific high-value modules: `glsl-noise` (port of
  Ashima/Stegu webgl-noise), `glsl-fbm`, `glsl-blend`,
  `glsl-easings`, `glsl-curvature`, `glsl-camera-ray`,
  `glsl-raytrace`, `glsl-sdf-primitives`.
- **Compatibility verdict:** USABLE.
- **Corpus quality:** Modular utility library, well-tested in the
  WebGL ecosystem. Less direct overlap with Lygia than expected:
  glslify modules are individually published, individually MIT-headered,
  and explicitly designed for composition.
- **Vendoring shape:** cherry-pick the modules whose functionality
  isn't already in Chromaticity helper shims, into
  `third_party/glslify-modules/<module>/`.
- **Integration cost:** Each module is a standalone `.glsl` file
  with a `#pragma glslify: export(fn)` directive. Strip the pragma
  and the file is plain GLSL. Vanilla GLSL ES 3.0 compatible.

**Recommendation.** Use as a fallback / supplement to Lygia. If the
Phase-5 `EV-LICENSE-AUDIT` resolves Lygia to a commercial-trial-end
position, glslify modules are the natural MIT-licensed replacement
substrate. Vendor 10-15 high-utility modules (the noise/blend/sdf
subset) preemptively in Phase-2/3 as a hedge.

### 2.12 Ashima / Stegu webgl-noise

- **Identity:** Ashima Arts (defunct) / Stefan Gustavson.
  `github.com/ashima/webgl-noise` (frozen),
  `github.com/stegu/webgl-noise` (continued).
- **License:** **`MIT`** per per-file headers (`// License : Copyright
  (C) 2011 Ashima Arts. All rights reserved. Distributed under the
  MIT License.`).
- **Compatibility verdict:** USABLE.
- **Corpus:** ~20 noise variants (Perlin, Simplex, Worley/Cellular)
  in 2D/3D/4D, "classic" + "periodic" + modern simplex flavors.
- **Vendoring shape:** flat-copy of `src/` to
  `third_party/webgl-noise/` (small total surface).
- **Integration cost:** vanilla GLSL ES 3.0 compatible; the canonical
  reference implementation. Probably the single most-referenced
  noise corpus in WebGL — already transitively present inside many
  ShaderToy shaders.

**Recommendation.** Vendor unconditionally. This is the canonical
noise substrate; small file count, MIT, no surprises.

### 2.13 TouchDesigner / Notch / Resolume preset libraries

- **Identity:** commercial proprietary VJ tools.
- **License:** Their bundled assets ship under per-product EULAs
  forbidding redistribution outside the host product.
- **Compatibility verdict:** **INCOMPATIBLE.** Even the visual
  concepts are proprietary.
- **Recommendation.** Do not vendor. Per `docs/29` clean-room SOP,
  reference materials in this class are for "behavior/spec
  extraction only" — and even that is suspect for non-open assets.

### 2.14 Mercury Demogroup hg_sdf (vendored 2026-05-22)

- **Identity:** Mercury Demogroup's "HG_SDF" library — a single-file
  GLSL library of signed-distance-function primitives (boxes, spheres,
  torus, prism, polyhedra), domain operations (repeat, fold, mirror,
  twist), and combination operators (smooth-union, intersection,
  subtraction). The de facto standard for SDF-based rendering in the
  demoscene. Maintained since 2011.
- **License:** **Dual MIT OR CC-BY-NC-4.0** at the upstream's election
  (per the inline SPDX header in `hg_sdf.glsl`, "SPDX-License-Identifier:
  MIT OR CC-BY-NC-4.0"). Chromaticity **elects the MIT track**. The
  CC-BY-NC-4.0 track fails the commercial-distribution guard per
  `docs/29`; the MIT track is permissive and unambiguous.
- **Compatibility verdict:** **CLEAN under MIT election.** Copyright
  (c) 2011-2021 Mercury Demogroup. Single 830-line file, no external
  dependencies. `deny.toml` `licenses.allow` already covers MIT.
- **Recommendation.** Vendored under `third_party/hg_sdf/` 2026-05-22.
  Production-use election state from the day of vendor (unlike Lygia,
  no Phase-5 `EV-LICENSE-AUDIT` re-election needed — MIT is
  permanent). Closes 64 PARTIAL rows in
  `docs/comparison/features/ssf-format.md` SDF primitive coverage.
- **Snapshot metadata.** See
  `third_party/hg_sdf/UPSTREAM-VERSION.md` for direct source URL,
  version date, election rationale, and provenance verification.

## 3 — Recommended seed corpus (V1 GPU-shader baseline)

Below is a curated ~30-shader shortlist for the V1 GPU-shader corpus
when ADR-011 lands. Each row carries: source, author, license,
description, GLSL/WGSL compatibility estimate, and required
`chroma_*` uniform binding cost.

**Tier 1 — utility/substrate (no further author audit needed):**

| Source | File | Author | License | Description | GLSL→WGSL | Chroma uniforms |
|---|---|---|---|---|---|---|
| webgl-noise | `noise2D.glsl` | Ashima/Gustavson | MIT | Simplex 2D | Clean | None |
| webgl-noise | `noise3D.glsl` | Ashima/Gustavson | MIT | Simplex 3D | Clean | None |
| webgl-noise | `noise4D.glsl` | Ashima/Gustavson | MIT | Simplex 4D | Clean | None |
| webgl-noise | `classicnoise2D.glsl` | Ashima/Gustavson | MIT | Classic Perlin 2D | Clean | None |
| webgl-noise | `cellular2D.glsl` | Ashima/Gustavson | MIT | Worley/cellular | Clean | None |
| glslify | `glsl-fbm` | Yi-Wen Lin | MIT | Fractional Brownian motion | Clean | None |
| glslify | `glsl-blend` | Jam3 | MIT | Photoshop-style blend modes | Clean | None |
| glslify | `glsl-easings` | glslify community | MIT | Penner easings | Clean | None |

**Tier 2 — post-processing pack (three.js):**

| Source | File | Author | License | Description | GLSL→WGSL | Chroma uniforms |
|---|---|---|---|---|---|---|
| three.js | `FXAAShader.js` | three.js authors | MIT | Anti-aliasing | Manual touch (texelFetch) | `chroma_inputTexture`, `chroma_resolution` |
| three.js | `BloomShader` (LuminosityHighPass+blur+composite) | three.js | MIT | HDR bloom 3-pass | Manual touch | bloom-threshold, bloom-strength |
| three.js | `BokehShader.js` | three.js | MIT | DOF bokeh blur | Clean | focus-distance, aperture |
| three.js | `RGBShiftShader.js` | three.js | MIT | Chromatic-aberration RGB split | Clean | amount, angle |
| three.js | `HalftoneShader.js` | three.js | MIT | Halftone print effect | Clean | radius, rotateR/G/B |
| three.js | `KaleidoShader.js` | three.js | MIT | Kaleidoscope | Clean | sides, angle |
| three.js | `SobelOperatorShader.js` | three.js | MIT | Edge detect | Clean | None |
| three.js | `VignetteShader.js` | three.js | MIT | Vignette | Clean | offset, darkness |
| three.js | `ColorCorrectionShader.js` | three.js | MIT | RGB curves | Clean | powRGB, mulRGB |
| three.js | `ACESFilmicToneMappingShader.js` | three.js | MIT | ACES tonemap | Clean | exposure |

**Tier 3 — compositional / scene-level (ISF):**

| Source | File | Author | License | Description | GLSL→WGSL | Chroma uniforms |
|---|---|---|---|---|---|---|
| ISF-Files | `Bad TV.fs` | Vidvox | MIT | Analog TV scan distort | Clean | `chroma_time`, intensity |
| ISF-Files | `Plasma.fs` | Vidvox | MIT | Classic plasma | Clean | `chroma_time`, palette |
| ISF-Files | `Audio Reactive Bars.fs` | Vidvox | MIT | FFT bar visualizer | Manual (audio uniform map) | `chroma_FFT_LO..HI` (the to-be-renamed audio bands) |
| ISF-Files | `Audio Reactive Circle.fs` | Vidvox | MIT | Radial RMS | Clean | `chroma_RMS` |
| ISF-Files | `Tunnel.fs` | Vidvox | MIT | Texture tunnel | Clean | `chroma_time`, speed |
| ISF-Files | `Kaleidoscope.fs` | Vidvox | MIT | Kaleidoscope (compositional) | Clean | sides, rotation |
| ISF-Files | `Pixellate.fs` | Vidvox | MIT | Pixel quantizer | Clean | `chroma_pixelSize` |
| ISF-Files | `Mirror.fs` | Vidvox | MIT | Mirror split | Clean | axis, position |
| ISF-Files | `Color Cube LUT.fs` | Vidvox | MIT | 3D LUT color grade | Clean | `chroma_LUT` |
| ISF-Files | `Slit Scan.fs` | Vidvox | MIT | Time-displacement | Clean | history-depth |

**Tier 4 — Inigo Quilez selected (CC0/MIT articles, NOT Shadertoy
compositional pieces):**

| Source | Topic | Author | License | Description | GLSL→WGSL | Chroma uniforms |
|---|---|---|---|---|---|---|
| iquilezles.org/articles | `distfunctions` | IQ | MIT | SDF primitive library | Clean | None |
| iquilezles.org/articles | `distgradfunctions2d` | IQ | MIT | 2D SDF + gradient | Clean | None |
| iquilezles.org/articles | `palettes` | IQ | MIT | Palette synthesis | Clean | palette params |
| iquilezles.org/articles | `smin` | IQ | MIT | Smooth-min for SDF blend | Clean | None |
| iquilezles.org/articles | `intersectors` | IQ | MIT | Ray/primitive intersect | Clean | None |

**Total seed corpus: 33 shaders / utility files.** All MIT or CC0
permissive — zero TRIAL-ONLY, zero per-shader audit residual. Provides
substrate (noise + utilities) + post-processing pack + compositional
scenes + reference SDF/raymarch fundamentals.

## 4 — Vendoring posture

### 4.1 Directory layout

Vendor under `third_party/<source>/` following the precedent set by
`third_party/lygia/`:

```
third_party/
├── lygia/                  # existing — Prosperity-3.0.0 trial
├── webgl-noise/            # new — MIT (Ashima/Stegu)
├── glslify-modules/        # new — MIT (per-module subdirs)
├── threejs-shaders/        # new — MIT (cherry-pick from examples/jsm)
├── vidvox-isf/             # new — MIT (cherry-pick from ISF-Files)
├── iquilezles/             # new — MIT (cherry-pick from iquilezles.org)
└── shadertoy-curated/      # new — per-file MIT/CC0 audited subset
```

Each subdirectory carries:
- `LICENSE` (verbatim upstream).
- `UPSTREAM-VERSION.md` (per-source pinned commit / capture date,
  mirrored from the existing `third_party/lygia/UPSTREAM-VERSION.md`
  format).
- `PROVENANCE.md` (per the `docs/29` Provenance Manifest Template).
- `README.md` (one-paragraph what-and-why, link to upstream).
- Per-file `// SPDX-License-Identifier: MIT` headers (or CC0-1.0 etc.)
  preserved where present; **added** where the upstream did not carry
  the explicit SPDX form (REUSE 3.2 compliance — `docs/43`).

### 4.2 Submodule vs flatten

Follow the Lygia precedent: **flatten-copy from a release tag**.
Submodules add CI complexity and don't survive the squash-merge
workflow cleanly. Snapshot at a known-good commit, record provenance,
and refresh via a `chore(repo): refresh third_party/<source> to <tag>`
PR when needed.

### 4.3 Attribution surfaces

Per `docs/29` Sec.Brand/Trademark Review Checklist and
`adr/ADR-010-embedded-docs-generation.md`, attribution flows through
three surfaces:

1. **In-bundle license aggregation.** Each subdir's `LICENSE`
   file ships via `bundle.resources` (precedent: the
   `dist/help/.gitkeep` + `tauri.conf.json` `bundle.resources`
   pattern from Wave 1d-α).
2. **Settings → About → Open-source acknowledgements.** A
   programmatic enumeration of `third_party/*/LICENSE` rendered as
   a scrollable Help-corpus page (Wave 1d-α extension).
3. **Release-notes attribution line.** Each `release/RELEASE-NOTES.md`
   entry on a third-party vendor change includes a one-line credit.

### 4.4 deny.toml extensions needed

Current `deny.toml` allowlist (`Apache-2.0`, `BSD-2-Clause`,
`BSD-3-Clause`, `CC0-1.0`, `ISC`, `MIT`, `MIT-0`, `MPL-2.0`,
`LicenseRef-Prosperity-3.0.0` trial, `Unicode-3.0`, `Unicode-DFS-2016`,
`Zlib`, `LicenseRef-Chromaticity-Proprietary`) covers all the §3
seed-corpus sources. **No `deny.toml` change is needed for the V1
seed corpus.** If Bonzomatic or another Unlicense-licensed source is
sourced later, add `Unlicense` via a `policy`-scoped PR per
`CLAUDE.md` "Serial only" rule.

### 4.5 REUSE.toml override entries

Per `docs/43` and the existing `REUSE.toml` precedent for
`docs/` → `CC-BY-4.0`:

```toml
[[annotations]]
path = "third_party/webgl-noise/**"
SPDX-FileCopyrightText = "2011 Ashima Arts, 2016+ Stefan Gustavson"
SPDX-License-Identifier = "MIT"

[[annotations]]
path = "third_party/threejs-shaders/**"
SPDX-FileCopyrightText = "2010-2026 three.js authors"
SPDX-License-Identifier = "MIT"

[[annotations]]
path = "third_party/vidvox-isf/**"
SPDX-FileCopyrightText = "2013-2026 VIDVOX"
SPDX-License-Identifier = "MIT"

[[annotations]]
path = "third_party/iquilezles/**"
SPDX-FileCopyrightText = "<year> Inigo Quilez"
SPDX-License-Identifier = "MIT"

[[annotations]]
path = "third_party/glslify-modules/**"
SPDX-FileCopyrightText = "various authors per per-module LICENSE"
SPDX-License-Identifier = "MIT"

[[annotations]]
path = "third_party/shadertoy-curated/**"
SPDX-FileCopyrightText = "various authors per in-shader header"
SPDX-License-Identifier = "MIT OR CC0-1.0"
```

### 4.6 Per-shader audit checklist (operationalised at vendor-PR time)

For every individual shader entering `third_party/`:

1. URL captured + timestamp recorded.
2. Upstream LICENSE file verified at the vendor-PR commit.
3. Per-file SPDX header present (added if missing — verbatim upstream
   text in `PROVENANCE.md`).
4. SHA256 of the source captured.
5. `scripts/evidence-tag-lint.sh` passes (engine-side tier-token
   coverage isn't affected; the script's coverage of
   `third_party/**` is enforced by the path-rule in
   `eslint.config.js` + `REUSE.toml`).
6. `cargo deny check licenses` green.
7. The new evidence row `EV-SHADER-CORPUS-V1` (proposed) lands in
   `docs/26-evidence-artifact-registry.md` to track per-PR additions
   to the corpus.

### 4.7 GLSL → WGSL / Naga compatibility posture

The render backend (ADR-011, Phase-2+) is wgpu over Naga; the canonical
shipped shader language is **WGSL**, with **GLSL ES 3.0** supported as
an alternative frontend through Naga's `glsl-in` translator. Per the
Naga maintainer notes (`naga` crate, 2024-2026): the GLSL → SPIR-V →
WGSL path is stable; ~95th-percentile vanilla GLSL ES 3.0 translates
cleanly. The 5%-tail failure modes are:

- `texelFetch` with integer offsets (some Naga versions reject).
- Non-uniform control flow with derivatives (works in GLSL, blocked
  by WGSL's uniformity analysis).
- Implicit type promotions (`1.0 / 2` vs `1.0 / 2.0`).
- `#extension` directives (must be removed in WGSL).
- ShaderToy entry points (`mainImage(out vec4 fragColor, in vec2 fragCoord)`)
  need a thin GLSL→GLSL adapter to plain `main() { gl_FragColor = ...; }`
  form before Naga eats it. This adapter is part of the
  `EV-IMPORT-SHADERTOY` import path scope.

**Operational implication.** A vendor-PR's CI must run `naga --validate`
against every new `.glsl` file in `third_party/**`. Add a `naga`
toolchain dev-dep + a `scripts/naga-validate.sh` walker as a
prerequisite PR before any large-batch vendor.

## 5 — Milkdrop, Hydra, ISF, live-coding posture

### 5.1 Milkdrop / projectM presets

Defer to **V2+**. Rationale (consolidating §2.9):

- LGPL runtime is INCOMPATIBLE for static linking.
- Preset format is a custom DSL (PEL), not GLSL — transpiler effort is
  multi-month.
- Preset corpus has soft-public-domain status that may not survive a
  conservative legal review.
- Visual concepts duplicate what Chromaticity can ship as clean-room
  `.chroma` originals at lower legal risk.

Document Milkdrop in `docs-corpus/` (in-app help) as a known
import-path-not-supported source with a "for V2+" disposition.

### 5.2 Hydra

**Excluded** under any form. AGPL-3.0 viral-copyleft contamination is
the binary blocker. The declarative DSL pattern (`osc().rotate().out()`)
is interesting as a UX direction for V2+ authoring — a clean-room
reimplementation per `docs/29` SOP would be the path, not vendor.

### 5.3 ISF — first-class V1 path

ISF is the highest-leverage compositional source (§2.10, §3 Tier 3).
The recommended posture is:

- **V1**: vendor ~30 ISF shaders directly to `third_party/vidvox-isf/`
  with per-file MIT preservation.
- **V1**: ship an **ISF → `.chroma` adapter** in
  `engine/scene/src/import/isf.rs` (Phase-2 PR scope; estimated 1-2 weeks).
  This compounds the value of vendored ISFs because every new ISF in
  the corpus is also a `.chroma` package, accessible through Library +
  the existing UI workflows.
- **Note re Phase-4b `EV-IMPORT-ISF-REJECT` narrowing**: that row
  shipped ISF-import-rejection (full ISF native import deferred to V2
  per design). The Phase-2+ revisit to ISF will need to widen that
  evidence row's scope.

### 5.4 Single canonical native format

The **`.chroma`** package format (manifest.json + main.glsl/main.wgsl
+ thumbnails + assets) is and should remain the canonical
Chromaticity-native scene format. Third-party authors who want their
work in Chromaticity should target `.chroma` directly. The
shadertoy-import + (eventual) ISF-import paths exist for migration
convenience, not as runtime alternatives.

## 6 — Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| **R-SHADER-01:** Lygia license election (Phase-5 EV-LICENSE-AUDIT) resolves to commercial-paid or clean-room replacement; corpus carry-cost. | Medium. AO-controllable. | Pre-stage the glslify/webgl-noise/threejs alternatives (§2.11, §2.12, §2.3) so a Lygia removal is a refactor, not a rewrite. |
| **R-SHADER-02:** ShaderToy-curated subset captures a shader whose author later removes the explicit MIT/CC0 header upstream. | Medium. Per `docs/29` excuse clause for unknowing breaches but reputational risk. | Capture-time snapshot of in-shader license header preserved in `PROVENANCE.md`; SHA256-pinned source body; never live-fetch at runtime. |
| **R-SHADER-03:** Three.js `examples/jsm/shaders/` historical CC-BY-3.0 contamination (`FilmShader.js` per `mrdoob/three.js#26571`). | Low at vendor-PR time IF per-file SPDX audit runs. | Per-shader audit mandatory at vendor-PR time per §4.6. Per-file SPDX header preserved/added. |
| **R-SHADER-04:** Bit-rot of GLSL ES 3.0 compatibility as wgpu/Naga evolves. | Medium-low. | `scripts/naga-validate.sh` runs on every vendor PR + nightly CI. Pin Naga major version in `Cargo.toml`; bump via `policy`-scoped PR (per CLAUDE.md "serial only" list — Cargo.toml workspace members are choke-point). |
| **R-SHADER-05:** Attribution drift — UI cradle for "Open-source acknowledgements" not actually wired by Phase-5. | Low if Wave 1d-α extension lands. | Add `EV-LIBRARY-LICENSE-DISCLOSURE` evidence row (proposed); cross-tag to `EV-DOCS-CORPUS`. |
| **R-SHADER-06:** Per-shader testing burden — every new shader needs visual smoke + WGSL transpile validation. | Medium. | Adopt the deterministic-byte-hash thumbnail-render approach already in use for `EV-LIBRARY-THUMBNAIL` (Wave 4b — deterministic stub renderer, no GPU dependency). Visual review is human; transpile validation is `naga --validate`. |
| **R-SHADER-07:** Author NC-clause overlooked on ShaderToy port; ships in customer build; AO legal exposure. | High consequence, controllable probability. | Mandatory per-shader header audit at vendor-PR time (§4.6); `scripts/evidence-tag-lint.sh` extension to verify per-file SPDX header non-empty (proposed). |
| **R-SHADER-08:** Patricio license dialogue (Lygia + Book of Shaders) goes adversarial. | Low. Patricio's posture has been collaborative (open MIT-contribute-back clause in Prosperity license; active engagement on issues). | Open dialogue early (Phase-2/3, not Phase-5). Pre-stage the clean-room shim alternative per `docs/29` line 104. |

## 7 — Recommended next actions

### 7.1 AO decisions needed (in priority order)

1. **Lygia license election timing.** Continue the trial through
   Phase-3 EXIT (Lygia is functioning today, no urgency). Open the
   commercial dialogue with Patricio at Phase-4 EXIT confirmation
   (deadline before Phase-5 EV-LICENSE-AUDIT gate). Recommended
   default: **continue trial, plan for both outcomes** by pre-staging
   the glslify alternative per §7.2 task 4.
2. **ShaderToy curated corpus posture.** Either (a) open the
   per-shader audit pipeline (curation cost: ~30 min per shader by a
   reviewer with legal-checklist familiarity) or (b) skip ShaderToy
   entirely in V1 and rely on ISF + three.js + IQ articles. Lighter
   path is (b); higher-yield is (a). Recommended: defer to V1.1.
3. **ISF → `.chroma` adapter scope.** Confirm whether the Phase-2 PR
   to widen `EV-IMPORT-ISF-REJECT` into a real import path is in
   scope for V1 or V1.1. Recommended: V1.1 — V1 ships ISF shaders as
   pre-converted `.chroma` packages (manual one-time conversion at
   vendor-PR time), V1.1 ships the live adapter.
4. **`deny.toml` extension to allow `Unlicense`.** No-op until a
   real Unlicense source is needed. Probably never required for V1.

### 7.2 Engineering tasks (Wave 2.3 candidate scope)

These tasks are pre-Phase-2-ADR-011 — they prepare the corpus so
when ADR-011 lands and the render pipeline is real, content is ready
to consume:

1. **Wave 2.3 Lane S.1: vendor 10 webgl-noise + glslify shader-utility
   modules.** Smallest first; lowest-risk vendor PR; validates the
   `third_party/` per-vendor pattern at scale. Estimated 1-2 days.
2. **Wave 2.3 Lane S.2: vendor 10 three.js post-processing shaders.**
   Add `scripts/naga-validate.sh` + the per-shader SPDX header
   audit script. Estimated 2-3 days (includes the validate script).
3. **Wave 2.3 Lane S.3: vendor 10 ISF shaders (manual ISF→GLSL strip
   pre-conversion).** Yields the compositional baseline. Estimated
   3-4 days.
4. **Wave 2.4 Lane S.4: vendor 10 IQ MIT articles' code (raymarching
   primitives + SDF library + smin/palettes).** Adapter from IQ
   article style to Chromaticity uniform convention. Estimated 2-3
   days.
5. **Wave 2.4 Lane S.5: clean-room helper-shim fallback corpus**
   (`docs/29` line 104). Independent of all upstream vendors; insurance
   policy for any Phase-5 license-audit outcome. Estimated 1 week.
6. **Wave 2.5 Lane S.6: add `EV-SHADER-CORPUS-V1` evidence row** to
   `docs/26` covering the cumulative seed-corpus state. Cross-tag
   to `EV-LICENSE-AUDIT` at the Phase-5 gate.

### 7.3 Open questions for AO direction

1. **Audio uniform binding for vendored shaders.** ShaderToy convention
   is `iChannel0` as a 512-sample audio waveform texture; ISF convention
   is `audioFFT` + `audioRMS` + `audioPeak` as named uniforms. Should
   the Chromaticity convention for vendored shaders converge on the
   ShaderToy iChannel0 pattern (already in `EV-IMPORT-SHADERTOY`
   import path) or on the ISF named-uniform pattern (cleaner mapping
   to `chroma_*`)? Recommended default: **named-uniform pattern**
   (cleaner, matches `chroma_FFT_*` + `chroma_RMS` audio contract per
   `docs/06-audio-engine.md`). The Shadertoy-import adapter
   transparently bridges the convention gap for that specific
   import path.
2. **Vendoring vs runtime-fetch posture.** Should vendored shaders
   ship in the binary (current `bundle.resources` + scene-library
   precedent per Wave 1d-β) or be a downloadable "shader pack" from
   a Chromaticity-controlled CDN (post-MVP)? Recommended default:
   **ship in binary at V1**; CDN posture is a V2 distribution
   optimisation once the corpus grows past ~100 shaders.
3. **Per-shader thumbnail rendering at vendor-PR time vs runtime.**
   Wave 4b `EV-LIBRARY-THUMBNAIL` ships a deterministic-byte-hash
   stub renderer (real GPU rendering deferred post-MVP). Should
   vendored shaders ship with pre-rendered thumbnail images
   (curator effort + per-platform GPU snapshot) or use the
   byte-hash stub? Recommended default: **byte-hash stub at V1**;
   real-render thumbnails are a Wave-5 surface that coincides with
   ADR-011 render-pipeline landing.
4. **License text aggregation surface naming.** The Settings → About
   → Open-source acknowledgements page (Wave 1d-α extension) hasn't
   been formally named. The bundle precedent calls it "Credits";
   the Lygia ecosystem calls it "Attribution"; SPDX calls it
   "License notices". Recommended: **"Open-source notices"**, mirrors
   industry standard (e.g. Apple "Open Source Licenses", Atom
   "Acknowledgements"), surfaced as a help-corpus page.

## 8 — Cross-references

- `docs/29-cleanroom-legal-policy-baseline.md` — Sec.Bundled GLSL
  Library Handling, Sec.Allowed-License Policy Baseline.
- `deny.toml` — cargo-deny SPDX allowlist (lines 73-100).
- `third_party/lygia/LICENSE.md` — Prosperity-3.0.0 verbatim text.
- `third_party/lygia/UPSTREAM-VERSION.md` — per-snapshot provenance
  format (template for new vendors).
- `docs/26-evidence-artifact-registry.md` — `EV-LIBRARY-LICENSE-TRIAL`
  + `EV-LICENSE-AUDIT` + `EV-IMPORT-SHADERTOY` rows.
- `docs/36-reactive-symbol-catalog.md` — `chroma_*` uniform vocabulary
  reference.
- `docs/38-scene-library-and-import-spec.md` — Shadertoy import path,
  ISF rejection narrowing.
- `docs/43-development-standards.md` — REUSE 3.2 + SPDX header
  requirements.
- `docs/47-ui-port-plan.md` — UX bundle's port path; informs
  attribution-surface design.
- `docs/54-wave-1c-beta-research-lessons.md` — pattern-only research
  posture for GPL/LGPL upstream (ffmpeg/VLC).
- `adr/ADR-010-embedded-docs-generation.md` — embedded-docs URL
  taxonomy + provenance tagging requirements.
- `adr/ADR-011-rendering-backend.md` — wgpu/Naga render-pipeline
  contract (Phase-2+).
- `adr/ADR-018-non-scene-ui-toolkit.md` — Tauri 2 single-binary
  distribution posture.

## 9 — Document status

- **Authored:** 2026-05-11 (Wave 2.2 Lane R.2).
- **Status:** Planning artifact. No code changes shipped with this
  PR. No vendoring performed. No `deny.toml` edits.
- **Next revision trigger:** AO decision on Lygia license election
  timing (§7.1 item 1) or first Wave 2.3 Lane S.* vendor PR landing
  (whichever happens first).
- **Owner:** Xavier Butler (AO) for §7.1 decisions; Wave 2.3+ lane
  agents for §7.2 task closure.
