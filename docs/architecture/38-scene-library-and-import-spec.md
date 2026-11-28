# Scene Library and Import Spec

This document locks the product contract for (a) the scene library browser, (b) thumbnail and metadata handling, and (c) Shadertoy / ISF scene import.

## 1) Purpose
The reference app ships 93 scenes that authors browse, filter, and queue into playlists. This surface is a product feature, not UI polish. Chromaticity must match this breadth of navigation and import without regression.

## 2) Scene Library Browser (FR-15)

### 2.1 Sources
- **Bundled scenes** - read-only scene packages shipped with Chromaticity under a single consolidated root: `<AppBundle>/Resources/scenes/`. Subfolders:
 - `<AppBundle>/Resources/scenes/welcome/` - the one-scene welcome package shown on first launch.
 - `<AppBundle>/Resources/scenes/library/` - the full bundled scene corpus (Chromaticity-original clean-room scenes).
 - `<AppBundle>/Resources/scenes/examples/` - minimal demonstration scenes for each authoring feature (multi-pass, script, transitions, media, audio-reactive). Referenced from the embedded docs (`adr/ADR-010-embedded-docs-generation.md`).

  This consolidates the reference's three separate scene locations (`release/`, `synesthesia_reference_files/demo/`, `default/welcome_scene/`) into a single predictable root. No other bundled-scene paths are allowed.
- **User scenes** - scenes under the active user scene folder. Path: `<UserDataRoot>/Scenes/`. User may add additional folders (parity with reference "Scene Folders" concept); results are merged into one logical library.
- **Author workspace** - scenes currently open in the authoring environment (`37-authoring-environment-spec.md`).

Library discovery is filesystem-backed; rescan on user request and on filesystem change events.

### 2.2 Metadata surfaced per scene
From `scene.json`:
- `TITLE` (required for library display; falls back to package directory name)
- `CREDIT`
- `DESCRIPTION`
- `IMAGE_PATH` (thumbnail; see Sec.3)
- `TAGS` (searchable/filterable)
- `CATEGORIES` (legacy; accepted and indexed alongside `TAGS`)

From filesystem:
- Package path, scene format (`.chroma` Chromaticity-native / `.synScene` reference-app original / imported-shadertoy / imported-isf). Chromaticity-native and reference-original are distinguished by directory extension and `source-provenance.json` presence.
- Last modified time
- Format tag (catalog metadata): one of `native-chroma-scene`, `compat-syn-scene`, `shadertoy-v1-import`, `isf-import`. Format tags are distinct from runtime compatibility profiles to prevent identifier collision with profile names `native-chroma-v1` / `compat-syn-v1`; see `22-compatibility-versioning-policy.md` for the profile vs tag distinction. Every imported scene also carries a runtime profile (`compat-syn-v1` or `native-chroma-v1`).

### 2.3 Browser capabilities
- Grid view with thumbnails and list view with metadata columns.
- Full-text search across `TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`, `CATEGORIES`.
- Filter by compat profile, by tag, by presence of scripts, by presence of media.
- Sort by title, credit, last modified, tag relevance, author-defined order (bundled "demo playlist" in reference).
- Drag-to-playlist from any view.
- Keyboard nav: arrow keys + Enter to launch, Shift+Enter to launch with preset prompt.
- "Start with Preset" action invokes `StartSceneWithPreset` per `CP-05`.

### 2.4 Launch semantics
- Single-click selects; double-click / Enter launches.
- Launch target obeys current playlist vs instant-scene mode.
- Optional preset launch uses the Snap-slot semantics from `CP-05` (default = Snap 0 "gold preset"; the underlying state key and OSC route retain the `favslot` literal for `compat-syn-v1`).

## 3) Thumbnail Pipeline

### 3.1 Canonical thumbnail
- Each scene declares `IMAGE_PATH` relative to the scene package. It is loaded at library-display time.
- If `IMAGE_PATH` is absent or unresolvable, fall back in this order:
  1. `<package>/<package_stem>.png`
  2. Auto-generated preview (see Sec.3.3)
  3. Format-family fallback icon (native-chroma-scene / compat-syn-scene / shadertoy-v1-import / isf-import / camera-dependent / media-dependent)

Format-family fallback icons are shipped as Chromaticity-original art in `<AppBundle>/Resources/thumbnails/`:
- `thumbnail-default.png`
- `thumbnail-fallback.png`
- `thumbnail-shadertoy.png`
- `thumbnail-isf.png`

These replace the reference's `default_thumbnail.png`, `fallback_thumbnail.png`, `shadertoy_thumbnail.png`, `isf_thumbnail.png` with clean-room originals per `29-cleanroom-legal-policy-baseline.md`.

### 3.2 Thumbnail refresh
- "Regenerate Thumbnail" action in the scene context menu.
- Thumbnail is captured at the scene's declared `WIDTH`x`HEIGHT` or scene's natural pass-0 target size, then downscaled to 512x512 max for storage.
- Regeneration requires audio and default meta state to be deterministic (seeded RNG, fixed `TIME=0` capture frame, or a configurable capture frame per scene).

### 3.3 Auto-preview on import
- On user scene import, a thumbnail is auto-generated at frame 60 with default audio-silent state if no thumbnail is declared.
- Failed generation writes the fallback and records the failure in the console.

## 4) Tag and Category Model

- `TAGS` and `CATEGORIES` are both accepted. Tokens are case-insensitive for search; preserved exactly in storage.
- Chromaticity reserves a curated base tag vocabulary (`geometric`, `audio-reactive`, `media-driven`, `raymarch`, `fluid`, `particle`, `video-fx`, `test`, `utility`). Authors may add any tag.
- Tag search supports substring and exact-match modes.
- The library UI exposes the "top N used tags" as filter chips.

## 5) Shadertoy Import

### 5.1 Stance
**In scope for V1 as a converter**, not a live Shadertoy-compatibility runtime.

### 5.2 Mechanism
- Import flow: user pastes a Shadertoy URL OR paste/upload the Shadertoy code body.
- Chromaticity fetches `mainImage(out vec4 fragColor, in vec2 fragCoord)` and produces a Chromaticity scene:
 - `scene.json` with a single `backbuff` pass at the user-chosen resolution.
 - `main.glsl` with a `renderMain()` wrapper that maps Shadertoy inputs:
 - `iResolution` -> `vec3(RENDERSIZE, 1.0)`
 - `iTime` -> `TIME`
 - `iTimeDelta` -> `TIMEDELTA`
 - `iFrame` -> `int(FRAMECOUNT)`
 - `iMouse` -> `_mouse`
 - `iDate` -> `vec4(2000.0, 0.0, 0.0, mod(TIME, 86400.0))` (synthetic; documented as approximate)
 - `iChannelTime[i]` -> `TIME` (documented as approximate)
 - `iChannel0..3` -> user-configurable during import. Choices per slot: `none`, `webcam`, `media`, `cubemap-from-fixture-pack`, `texture-from-fixture-pack`, or `custom-asset` (user-supplied file at import time).
 - A `_mainImage_body()` helper inlines the Shadertoy `mainImage` body and `renderMain()` invokes it.
- Importer emits a coercion diagnostic only for patterns that cannot be auto-mapped even with the fixture pack: `Buffer A/B/C/D` multipass (Shadertoy's own multipass model), keyboard input, soundcloud-audio iChannels, and VR patterns. Each diagnostic leaves a TODO comment block for the author.

### 5.2a Shadertoy iChannel fixture pack (FR-14 supporting artifact)
To avoid the "paste a Shadertoy that uses a cubemap -> import fails" failure mode that the reference corpus exhibits, Chromaticity ships a curated **iChannel fixture pack** at `<AppBundle>/Resources/scenes/shadertoy-fixtures/`:

- **Cubemaps** (6 cubemaps x 6 faces = 36 images): a clean-room set covering the typical cubemap shape authors import from Shadertoy (night-sky, interior, art-gallery-atrium, studio, street, test-grid). All Chromaticity-original or CC0-licensed with provenance recorded per `29-cleanroom-legal-policy-baseline.md`; names are generic descriptors and do not reference specific known third-party cubemap assets. Files: `cubemap-<name>-{px,nx,py,ny,pz,nz}.{jpg|png}`.
- **2D textures** (>= 12 fixtures): a clean-room set covering Shadertoy's common 2D asset library (noise, tile, rock, wood, abstract, gradient-grid). CC0 or Chromaticity-original.
- **Name-to-asset mapping table** (`shadertoy-fixtures/manifest.json`): maps the 20 most-common Shadertoy channel names and hashes observed in the Shadertoy Top-100 to the local fixture files. Import UI offers these as "best-match" suggestions when the Shadertoy source references a known channel.
- **Licensing**: every fixture has a LICENSE sidecar recording origin, author, license text, and hash. `EV-LICENSE-AUDIT` asserts no fixture has unresolved provenance.

The fixture pack is in scope for V1; cubemap iChannels are **not** a coercion-diagnostic case at import time (they were, per the earlier draft of this spec). Patterns that reference asset names not in the fixture pack still open the file-picker fallback for `custom-asset`.

### 5.3 Legal posture
- Users are responsible for respecting the original author's license (Shadertoy defaults to CC-BY-NC-SA 3.0).
- Import UI surfaces a consent dialog with that exact text once per session, and the imported scene's `CREDIT` field is pre-populated with the source URL and an attribution stub.
- Bundled scenes shipped by Chromaticity are original works per `29-cleanroom-legal-policy-baseline.md`; the importer does not bundle any Shadertoy-origin scenes.

### 5.4 Fixtures
- At least 3 golden Shadertoy-import fixtures in `34-golden-corpus-definition.md` (simple math, noise/fbm, raymarched) authored by Chromaticity team with permissive licensing and full provenance.
- At least 2 additional fixtures exercising the iChannel fixture pack (one 2D-texture iChannel, one cubemap iChannel) so the fixture-pack resolution path is covered by `EV-IMPORT-SHADERTOY`.

## 6) ISF Import

### 6.1 Stance
**Reject with actionable diagnostic in V1**. Deferred to V2 as a differentiation bet.

### 6.2 Rationale
- ISF's `PASSES`, `INPUTS`, and `DESCRIPTION` JSON keys overlap with SSF syntactically but have different semantics (`INPUTS` != `CONTROLS`; ISF supports persistent buffers differently).
- A half-working ISF import would produce broken scenes and erode trust in the importer surface.
- Adding a correct ISF import in V2 leaves room to build on the Shadertoy-import machinery.

### 6.3 V1 behavior
- When the importer detects an ISF header (`*/` after a JSON block in a `.fs` file) or `.isf` package, it:
 - Rejects the import.
 - Emits a clear diagnostic: "ISF scenes are not yet supported. Chromaticity V2 will support ISF scenes. Tracking issue: CHR-ISF-V2."
 - Offers a "copy as text into a blank scene" escape hatch that writes the raw `.fs` body as a comment in a new Chromaticity scene, with no attempt to interpret it.

## 7) Packaging and UTI
Chromaticity ships three native extensions, locked in `22-compatibility-versioning-policy.md` Sec.Package Extensions:
- **`.chroma`** - directory package, native scene format. Semantics match the reference's `.synScene` directory model; extension is distinct because `.synScene` is reference-owned.
- **`.chromaPack`** - zip archive transport form of a `.chroma` package.
- **`.chromaShow`** - zip archive show bundle containing one or more scenes plus playlists, preset banks, Snap slots, bindings, and settings.

Reference-app extensions (`.synScene`, `.synScenePack`) are accepted as **import input only**; Chromaticity does not claim, register, or declare a UTI / ProgID for them. Double-click handling for reference extensions on a Chromaticity-only install is intentionally absent - an operator who wants it sets "Always Open With Chromaticity" via OS affordances.

Icons (`chroma.icns`, `chromaPack.icns`, `chromaShow.icns`, Windows `.ico` equivalents) are clean-room-original per `29-cleanroom-legal-policy-baseline.md`; no icon reuses or visually mimics the reference's `synScene.icns`.

## 8) Evidence
- `EV-LIBRARY-DISCOVERY` - library rescan under N scenes discovers all, indexes metadata, supports filter/search queries.
- `EV-LIBRARY-THUMBNAIL` - fallback chain works for a scene with missing thumbnail; regeneration produces a deterministic image given seeded state.
- `EV-IMPORT-SHADERTOY` - 3 code-pattern fixtures + 2 iChannel-fixture-pack fixtures (1 cubemap, 1 2D texture) convert, compile, and render without errors.
- `EV-IMPORT-ISF-REJECT` - ISF input produces the exact V2-deferral diagnostic; escape hatch writes a valid blank scene.
- `EV-LIBRARY-BUNDLE-LAYOUT` - asserts the bundled scene root is exactly `<AppBundle>/Resources/scenes/{welcome,library,examples,shadertoy-fixtures}/`; no legacy `release/` or `synesthesia_reference_files/` paths are present.

## 9) Open Decisions
- Snap-slot count: locked at 16 per scene (reference parity for `favslot` count; see `07-control-state-presets.md` update).
- "Scene Folders" UX (multiple user roots vs a single root): decision deferred to wireframes; this doc permits either.

## 10) Traceability
- Requirements: `03-product-requirements.md` FR-14 (Imports), FR-15 (Library), FR-16 (Embedded Docs cross-link to library), FR-17 (Hotkeys).
- Evidence registry: `26-evidence-artifact-registry.md` (`EV-LIBRARY-*`, `EV-IMPORT-*`).
- Cleanroom: `29-cleanroom-legal-policy-baseline.md` (fallback icons, bundled fixtures).
