# In-App Scene Authoring Environment Spec

This document locks the product contract for the in-app scene authoring environment. It is a first-class product feature, not UI polish.

## 1) Purpose
Authors shipped 93 bundled reference scenes using an in-app editor with hot-reload, console output, and inline diagnostics. Chromaticity must match this authoring velocity or the product regresses for its most-valuable user class. This spec is normative for `FR-13 Scene Authoring Environment`.

## 2) Audience
- Scene authors writing `scene.json`, `main.glsl`, `script.js`, and shared `glsl/` includes inside the app.
- Shader porters importing from Shadertoy/ISF.
- Support engineers reproducing a reported bug via a customer's scene bundle.

## 3) Authoring Environment Components
### 3.1 Scene file panel
- Tabs per file in the current scene package: `scene.json`, `main.glsl`, `script.js`, plus any `glsl/*.glsl` includes and `images/*` asset listing.
- File tree surfaces the package contents 1:1 with disk.
- New file action for `script.js`, `glsl/`, `images/`.
- Delete requires confirmation.
- Unsaved markers per tab.

### 3.2 Editor surface
- Monospaced editor with syntax highlighting for GLSL, JavaScript, JSON.
- Minimum editor contract:
 - Line numbers, current-line highlight, matching brace, basic find/replace, multi-cursor on a hotkey, undo/redo per tab.
 - Tab size = 2, soft tabs by default (matches reference convention in shipped scenes).
 - Autocomplete for every symbol in `36-reactive-symbol-catalog.md` (standard uniforms, audio uniforms, meta controls, helper signatures) with hover docs.
 - Control `NAME` autocomplete inside `script.js` from the current `scene.json`.
- Editor library choice is an implementation detail, but the feature set must match or exceed the list above. Candidates: Monaco, CodeMirror 6. Ace is the reference's editor and is acceptable.

### 3.3 Live reload
Hot reload rules, per file type:

| File | Hot-reload behavior |
|---|---|
| `scene.json` | Full scene rebuild: rebuild control state, recompile shader with new prepend, recreate passes, preserve in-flight playlist/OSC bindings where control `NAME` unchanged. |
| `main.glsl` | Recompile shader only; preserve all control values, script state, pass targets if pass declarations unchanged. |
| `glsl/*.glsl` (include) | Same as `main.glsl` for every scene currently using the include. |
| `script.js` | Rebind script host, re-run `setup()`, keep control values, reset script-local state (per ADR-003). |
| `images/*` | Reload sampler, keep everything else. |

Latency budget: edit -> visible result `p95 < 500 ms` for `main.glsl` changes on the MVP hardware class (tied to `10-performance-latency-budgets.md`). Hot reloads are not allowed to cause a frame drop on the output pipeline; if recompile exceeds the frame budget, the new shader is swapped in on the next frame boundary.

### 3.3a NFR-10 Measurement Harness (normative)

This section is the single source of truth for how `EV-AUTHORING-UX-LATENCY` and `EV-DOCS-UX-LATENCY` are measured. UX designs against these numbers; implementation instruments against these sources.

- **Event timestamp source** (input):
  - macOS: `NSEvent.timestamp` (mach-absolute-time converted to Mach clock seconds).
  - Windows: `RAWINPUT` message timestamp converted from `QueryPerformanceCounter`.
  - Web-view docs/hover fallback (if a surface uses an embedded web view): the OS-level event timestamp is the source of truth; the web-view's internal timestamp is not used.
- **Paint-completion timestamp source** (output):
  - macOS: `CAMetalLayer nextDrawable` present-ack callback timestamp for the first frame whose pixels include the rendered response.
  - Windows: `IDXGISwapChain::Present` completion timestamp for the same frame.
- **Metrics (paired with the SLO-02 tails discipline)**:
  - Hover-docs latency: hover-target-intersect event -> tooltip first-paint present-ack. Target: p95 <= 80 ms, p99 <= 120 ms.
  - Symbol-search latency: first keystroke in the search field -> first result row present-ack. Target: p95 <= 150 ms, p99 <= 220 ms.
  - Doc-page open latency: click on a sidebar entry -> page content first-paint present-ack. Target: p95 <= 300 ms, p99 <= 450 ms.
  - Shader-edit-to-visible-result latency: last keystroke in `main.glsl` -> first frame containing the new shader present-ack. Target: p95 <= 500 ms (existing budget above).
- **Measurement population**: 500 events per metric per platform per release-candidate build; events emitted against the baseline-profile fixture subset under warm shader cache conditions; no concurrent GPU-heavy background work.
- **Instrumentation overhead budget**: the harness must not inflate the measured p95 by more than 1 ms; overhead is audited by running the harness with and without instrumentation on a synthetic canned-input fixture and diff-asserting p95 deltas.
- **Exit**: `EV-AUTHORING-UX-LATENCY` (Phase 4b) and `EV-DOCS-UX-LATENCY` (Phase 4c) each Pass if every target is met for their relevant metric; one-metric regressions trigger a waiver per `23-decision-ownership-matrix.md`, not a blanket fail.

### 3.4 Console panel
- Streams diagnostics from every subsystem: GLSL compiler, JS host, OSC/MIDI adapters, audio adapter, media adapter.
- Each line is tagged with: timestamp, subsystem, scene name (if scene-origin), severity (`info`|`warn`|`error`|`coerce`).
- Script `print()` and `console.log()` output lands here, tagged with scene name.
- `coerce` severity covers all `CR-01..CR-04` coercions (see `28-implementation-contract-pack.md`).
- Console supports filter by subsystem and by severity, and a "clear on scene load" toggle.

### 3.5 Inline diagnostics
- GLSL compile errors are mapped back to the author's source line (post-prepend mapping table is maintained so the prepended helper header doesn't skew line numbers).
- JSON schema violations link to the offending path and propose a fix when the violation is in `CR-01..CR-04`.
- Script runtime exceptions surface the stack with `scene:line:col` and do not unload the scene (NFR-1).

### 3.6 New Scene scaffolder
The scaffolder creates a new `<name>.chroma/` under the active authoring folder. "New Scene" is the first page every author sees; the default template must be rich enough to **actually demonstrate** audio reactivity and the meta pipeline, not a 4-line flat-color gradient (which is what the reference's `new_scene/` template produces).

#### Default template (Chromaticity's improvement over the reference)
The default template delivers an audio-reactive colour-shifting gradient with one interactive control, one meta-media interaction, and a comment block pointing to every doc page the author will need next.

- **`scene.json`**: `TITLE` set to the requested name, `CREDIT` prompted at creation (remembered for subsequent new scenes), `DESCRIPTION` placeholder, `IMAGE_PATH: "thumbnail.png"`, `PASSES: [{ "TARGET": "backbuff" }]`, and one `slider smooth` control named `intensity` with `PARAMS: 0.04`, range `[0, 1]`, default `0.5`, `UI_GROUP: "reactivity"`, descriptive `DESCRIPTION`.
- **`main.glsl`** (authored as a compact, commented starting point that demonstrates reactivity):
  ```glsl
  // Welcome to your new Chromaticity scene!
  // Docs: /reference/scene-json  /reference/standard-uniforms  /reference/audio-uniforms
  //
  // Available audio uniforms: chroma_Level, chroma_BassLevel, chroma_MidLevel, chroma_HighLevel,
  //                            chroma_OnBeat, chroma_BPMSin, chroma_AudioTime, chroma_CurvedTime.
  // Available standard uniforms: _uv, _uvc, TIME, RENDERSIZE, chroma_Media.

  vec4 renderMain() {
      vec2 uv = _uvc;
      float r = length(uv);
      float a = atan(uv.y, uv.x);

      // audio-reactive color ramp
      vec3 colA = _palette(r + chroma_AudioTime * 0.1,
                           vec3(0.5), vec3(0.5),
                           vec3(1.0), vec3(0.00, 0.33, 0.67));
      vec3 colB = colA * (0.8 + 0.4 * chroma_BassLevel);

      // on-beat accent ring
      float ring = smoothstep(0.01, 0.0, abs(r - 0.5 - 0.1 * chroma_OnBeat));
      vec3 col = mix(colB, vec3(1.0), ring * intensity);

      // media blend (fades in if operator has media selected)
      if (_isMediaActive()) {
          col = mix(col, _textureMedia(uv).rgb, 0.3 * intensity);
      }

      return vec4(col, 1.0);
  }
  ```
- **`thumbnail.png`**: a 512x512 auto-captured frame at `TIME=2.0` with seeded audio silence, consistent with the thumbnail pipeline in `38-scene-library-and-import-spec.md` Sec.3. Regenerable from the context menu.
- Total line count (including comments): ~= 25 GLSL lines + ~= 15 JSON lines. Compact enough to read in one screen; complete enough to actually **look like a Chromaticity scene** on first render.

#### Scaffolder UX
- Scaffolder dialog accepts a `NAME` (default `untitled-<n>`), and offers checkboxes:
 - **Include Script** - adds `script.js` with `setup()`, `update(dt)`, and `transition()` skeletons, each containing a one-line example.
 - **Multi-pass** - extends `PASSES` to two entries (`main` intermediate + `backbuff` composite) with an example feedback-read from `chroma_FinalPass`.
 - **Audio-reactive template** (on by default) - produces the template above. When off, produces the minimal placeholder template retained for backward compatibility.
- Creation places the new scene package under the active authoring folder and opens it in a new tab set.

#### Minimal-placeholder template (retained for compatibility)
For authors who explicitly prefer a blank slate, a one-control, one-line `main.glsl` template is available via the "Minimal template" toggle. This is the template the reference produces by default; Chromaticity demotes it to an opt-in so the first-launch experience is the richer template.

### 3.7 Scene package export/import
- Export current scene as a `.chroma` directory package (native Chromaticity format) or a `.chromaPack` zip archive (single-file transport form). See `22-compatibility-versioning-policy.md` Sec.Package Extensions and `38-scene-library-and-import-spec.md` Sec.7.
- Import accepts Chromaticity-native `.chroma` / `.chromaPack` directly and accepts reference-app `.synScene` / `.synScenePack` inputs via one-way conversion to `.chroma`. The original reference file is never modified, renamed, or moved.
- Import validates against the canonical schema, runs the compat migrator (`22-compatibility-versioning-policy.md`), emits coercion diagnostics, writes a `source-provenance.json` sidecar for converted scenes, and stages the result in the authoring folder.
- Import must accept scenes built by the reference app unchanged in behavior (rendering parity under `compat-syn-v1`).

### 3.8 Performance feedback
- Per-pass GPU cost estimate in the HUD (from timer queries where available; estimate otherwise).
- Frame-time sparkline with target-budget overlay.
- Memory footprint (scene textures, shader module, script heap).
- These numbers are also written to the diagnostics console every N seconds at `info` severity.

## 4) Hotkeys (authoring scope)

| Hotkey | Action |
|---|---|
| Cmd/Ctrl+S | Save all dirty tabs in the current scene package. |
| Cmd/Ctrl+Shift+S | Save-as a new scene package name. |
| Cmd/Ctrl+R | Force full scene rebuild. |
| Cmd/Ctrl+/ | Toggle line comment (GLSL/JS). |
| Cmd/Ctrl+F | Find in current file. |
| Cmd/Ctrl+Shift+F | Find across scene package. |
| Cmd/Ctrl+P | Quick-open file in current package. |
| Cmd/Ctrl+E | Toggle console panel. |
| Cmd/Ctrl+Shift+E | Toggle performance HUD. |
| F5 | Reload scene (parity shortcut). |
| Esc | Exit fullscreen output (parity with reference Windows convention). |

This is distinct from the operator hotkey map (see `03-product-requirements.md` FR-17 and the control-hotkey table).

## 5) Determinism Constraints
- Live-reload rebuilds must re-seed the script host deterministically from the same scene seed. Random state is reset at rebuild boundaries.
- Frame clocks (`TIME`, `FRAMECOUNT`) reset on scene package rebuild but NOT on shader-only hot reload; this matches reference behavior and keeps visuals stable while iterating.
- Audio uniforms are not paused during hot reload.

## 6) Error and Failure Semantics
- A shader compile failure surfaces the error inline and keeps the previous successful shader running (no black frame). Output pipeline continuity is NFR-1.
- A JS exception during `setup` or `update` does not crash the runtime. The offending scene is marked degraded; operator can retry.
- A JSON parse failure keeps the previous compiled `scene.json` and surfaces a diagnostic. Rebuild retried on next save.

## 7) Evidence
- `EV-AUTHORING-LIVE-RELOAD` - scripted harness measures hot-reload p50/p95 on each file class; checks state preservation per the table in Sec.3.3.
- `EV-AUTHORING-DIAGNOSTICS` - validates that every `CR-01..CR-04` coercion and every `KM-01..KM-05` malformed fixture produces an actionable, correctly-mapped diagnostic in the console.
- `EV-AUTHORING-SCAFFOLDER` - validates New Scene output compiles and runs without warnings.

## 8) Out of Scope (for V1)
- Cloud-backed scene sync.
- Collaborative multi-cursor editing.
- AI-assisted authoring (deferred to a future differentiation bet, not part of Parity-Core).
- Visual node-based editor.

## 9) Traceability
- Requirement: `03-product-requirements.md` FR-13.
- Phase gates: `11-roadmap-phases.md` Phase 0 UX gate "Authoring Env DoD" and Phase 1 "Authoring Env Alpha".
- Symbol autocomplete depends on `36-reactive-symbol-catalog.md`.
- Hot-reload latency rolls up to `10-performance-latency-budgets.md`.
