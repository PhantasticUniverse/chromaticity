<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Live Render Pipeline — Design Proposal (ADR-011 Phase-2)

> **STATUS: PROPOSAL — pending AO ratification. Implementation is blocked on
> sign-off.** This is a design input, not a decision. The ADR-011 / ADR-018
> amendments in §3 are AO-held and non-delegable; this document only frames
> them. Produced by a multi-agent design pass (understand → three independent
> architecture proposals → scored synthesis) grounded in the current code.

## Goal

Wire `engine/render`'s GPU pipeline (wgpu 26 + naga, GLSL 4.50 → SPIR-V,
multipass + feedback, already built) into **live** rendering in the app — both
the audience-facing OUTPUT and an in-UI PREVIEW — driven by the existing 60 Hz
scheduler with live scene + controls + audio. Today the engine renders only
offscreen via direct Rust calls (no Tauri command), and the visible surface is
the canvas-2d *procedural* adapter (`src/lib/widgets/scene-renderers.ts`).

## Recommended architecture — Hybrid (output window + readback preview, overlay later)

`LiveRenderDispatch` (implements the existing scheduler `RenderDispatch` trait)
→ bounded(1) channel → dedicated **render thread** → `MultipassRenderer` →
present to a **native wgpu OUTPUT window** (full-res / 60 Hz — the VJ stage
output), plus a **CPU-readback in-app preview** (320×180 / ~15 Hz JPEG → webview
`<img>`) as the documented R-33 fallback. The ADR-018 transparent-overlay
("zero-copy preview behind a transparent WebView hole") is a *later, isolated*
upgrade (Slice R-5), gated on the unexecuted R-33 spike (Windows WebView2 z-order
is the highest risk). The shell adapter mirrors `TauriIpcEmitter` exactly (a
`Send + Sync` struct holding a channel sender, in `app/shell`, implementing a
`chromaticity_runtime` trait — no engine crate sees Tauri).

Why this over "transparent overlay first": the VJ output window is the
higher-value first delivery and is needed regardless; the readback preview is the
*blessed R-33 fallback* materializing exactly when intended; the overlay can't
de-risk Windows until the spike runs; and the overlay→readback upgrade is a clean
one-PR swap (`<img>` → transparent CSS hole) that leaves engine/render, the
dispatch, the render thread, and the output window untouched.

**Determinism is unaffected in all variants.** The render thread reads
display-only data from `&State` (via a `RenderFrameSignal` snapshot), never
mutates, and `canonical_state_hash` never observes it.

### Critical correctness gap (independent of the display choice)

`MultipassRenderer::render_to_texture` rebuilds per-spec resources — including
feedback ping-pong textures — on **every** call. A live loop calling it once per
tick with `frame_count=1` means `syn_FinalPass` feedback does **not** survive
between ticks. Any live-render approach needs a `render_frame_live`-style entry
point that holds feedback textures across calls. This is an engine/render
prerequisite (Slice R-0), not a display concern.

## Phased slice plan (world-class, not rushed)

- **R-0 — engine prerequisites (`engine/render` only; additive, no ADR touch).**
  (a) `MultipassRenderer::new_with_surface_hint(&wgpu::Surface)` — passes
  `compatible_surface: Some(surface)` to adapter selection (current `new()` at
  `multipass.rs:477` selects with no surface hint → may pick a non-presenting
  adapter on multi-GPU). (b) `render_frame_live(&spec, frame_index, spec_key)` —
  persistent feedback textures keyed by `spec_key` (FNV of scene id + size),
  ping-pong on `frame_index % 2`, returns the final texture without readback;
  `render_to_texture`/`render` unchanged. (c) `SceneAdapterOptions.control_values:
  Vec<(String, f32)>` → injected into `PassDef::uniforms` (resolves the
  `_uniform_count` discard at `scene_adapter.rs:~160`). T1 tests. **Unblocks
  everything; architecture-agnostic (every proposal needs all three).**
- **R-1 — shell wiring + output window (`app/shell`; NO policy PR — see Amendment
  A, retired).** New `app/shell/src/render/` module (`dispatch`, `thread`,
  `signal`, `spec_builder`, `surface`, `blit`, `output_window`). The wgpu surface
  is created with the **safe** `Instance::create_surface(Arc<Window>)` —
  **no `unsafe`** — against a **plain, no-webview** `tauri::window::Window` (Tauri's
  `unstable` feature): a `WebviewWindow`'s `WKWebView` paints opaque over the Metal
  layer (`about:blank` renders solid white on top), so the GPU output window must be
  webview-free. **Confirmed end-to-end on macOS Metal (R-1a, 2026-06-05: animated
  gradient, correct RGBA channel order, ~60fps).** macOS main-thread constraint
  (load-bearing): the output window
  AND the surface AND adapter/device are created on the MAIN thread in Tauri
  `setup` (off-main-thread `create_surface`/window is a *panic*, not an error),
  then the `Send` `Surface<'static>` + renderer are moved into the dedicated render
  thread. Wire `LiveRenderDispatch` into the scheduler (`lib.rs:241-243`), add
  `render_thread` + `scene_update_tx` to `AppState` (drop order: render thread
  before scheduler). Bounded(1) channel = natural 60 Hz pacing; overflow →
  `Skipped`, no scheduler backpressure. Built incrementally: **R-1a** output window
  + surface + a test-pattern present (the riskiest seam, in isolation); **R-1b**
  scene loading + `spec_builder`; **R-1c** scheduler-driven live controls/audio.
  Smoke test: launch with the output window enabled, see live GPU GLSL reacting to
  controls + audio.
- **R-2 — in-app preview (`ui` only).** `SceneCanvas.svelte` listens for a
  `render_preview_frame` event → `<img>` JPEG; canvas-2d stays as fallback.
- **R-3 — ADR-024 reactive bands.** Extend `audio_adapter.rs` with
  `chroma_AudioBass/Mid/Treb` + `*_att` (additive, appended — no T2 reorder).
- **R-4 — shader pipeline cache (`engine/render`).** Correctness prereq: without
  it `render_frame_live` recompiles fragments every tick (50–800 ms cold per
  ADR-011 §5). Must land before/with R-1 going to production. `EV-SHADER-CACHE`.
- **R-5 — transparent overlay upgrade (AO-gated, post-R-33 spike; macOS-first).**
- **R-6 — output resize / fullscreen / Syphon wiring (post-R-4).**

## Decisions needing AO sign-off (§3 ADR amendments)

- **A — `unsafe` surface creation. RETIRED — no longer needed; nothing to ratify.**
  Confirmed against the pinned sources (tauri 2.10.3, wgpu 26.0.1, a single shared
  `raw-window-handle 0.6.2`): wgpu's **safe** `Instance::create_surface` accepts
  `Arc<WebviewWindow>` (tauri implements the rwh-0.6 traits directly; the `unsafe`
  lives inside tauri, not our tree), yielding `Surface<'static>` with **zero
  `unsafe`**. `unsafe_code = "forbid"` is preserved and no policy PR is needed.
  Guard: this rests on the single-`raw-window-handle` invariant — a `cargo deny`
  duplicate ban on `raw-window-handle` should pin it (a future split into two
  semver-incompatible rwh versions would break the safe path and force `unsafe`).
- **B — second native window as the canonical VJ-output path.** ADR-018 §Scene
  canvas embedding says "composited under a transparent section of the web view,"
  but also "the operator fullscreen path exits the web view entirely and places
  the wgpu surface full-screen natively." Proposal: ratify the second window as
  that native path. AO decides: in scope for ADR-011 Phase-2, or new ADR? **Note:**
  R-1a implements this as a no-webview `tauri::window::Window`, which requires
  Tauri's `unstable` cargo feature — a dependency to weigh in this ratification.
- **C — CPU readback preview as the initial in-app preview.** Proposal: ratify
  320×180 / ~15 Hz readback as the MVP preview, overlay (R-5) as the upgrade. AO
  decides: acceptable shipped state, or overlay required before any preview lands?
- **D — Windows/Vulkan for Phase-2 (DX12 deferred).** ADR-011 §1 names DX12 the
  Windows primary; it's disabled today (`engine/render/Cargo.toml:64-84`,
  `windows-core` conflict with Tauri 2). Proposal: Vulkan for Phase-2, DX12 tracked
  + deferred. AO decides: acceptable, or DX12 must precede live render?
- **E — `render_preview_frame` IPC event.** A new display-only event (JPEG
  payload), not a `State`-derived snapshot. AO decides: new `EV-` registry row, or
  covered under `EV-RENDER-CONFORMANCE`?

## Key considerations

- **Perf (docs/10: 16.67 ms budget, render graph ≤ 8.50 ms).** Scheduler-side
  `dispatch_frame` holds the state mutex < 50 µs (clone a 72-byte audio snapshot +
  a small control `Vec`, one `try_send`). Render thread is independent; single-pass
  1080p on Apple Silicon ~1–3 ms. Readback 320×180 RGBA8 = 230 KB; map + encode +
  emit < ~4 ms amortized every 4th frame; IPC < 1 MB/s (audio already does 12 KB/s
  at 60 Hz). The pipeline cache (R-4) is the gating perf prerequisite.
- **Platform.** macOS Metal primary for R-0..R-4 (`Bgra8UnormSrgb` swapchain gives
  hardware sRGB at present, ADR-011 §8). Windows Vulkan (query surface caps for the
  preferred sRGB format; blit pass handles format diff). DX12 backend-agnostic via
  wgpu when the version conflict resolves.
- **Render conformance.** The existing `EV-RENDER-CONFORMANCE` T2 offscreen corpus
  is the same rendering minus the final readback — it keeps passing unmodified.
  The 14 in-flight `shader-archetypes` fixtures (+ the 16 bundled scenes) are the
  first live content + the visual cross-check against the hash pins.

## Risks

- **R1** multi-GPU surface compat → `new_with_surface_hint` is the wgpu-idiomatic
  fix; fall back to offscreen + disable output window if no adapter presents.
- **R2** feedback reset on scene change → exactly one black frame (correct).
- **R3** `scene_adapter` requires a fragment-source map (`MissingFragmentSource`
  at `~line 138`); `spec_builder.rs` must populate it from each `PassSpec` GLSL —
  verify where the GLSL lives in `engine/scene/src/schema.rs` before R-0.
- **R4** `PassDef::uniforms` is positional std140 → `spec_builder.rs` must order
  deterministically (controls in scene order, then audio in adapter order);
  deterministic by construction each frame.
- **R5** ~~`unsafe_code` policy PR~~ — RETIRED (safe surface; Amendment A). New
  guard: keep `raw-window-handle` to a single version (a `cargo deny` duplicate
  ban) so the safe `create_surface(Arc<WebviewWindow>)` path holds.
- **R6** macOS main-thread affinity → window + `create_surface` + adapter/device
  must be created on the main thread (off-main is a *panic*); the `Send`
  `Surface<'static>` is then moved to the render thread. Build/verify on macOS;
  Windows has no such constraint.

## Deferred

R-33 transparent overlay (Windows WebView2 spike), DX12 re-enablement, Syphon real
backend (Wave 1c-β follow-up) + Spout, per-pass sampler WRAP/FILTER keys,
`PASSINDEX`/`RENDERSIZE` built-in injection, the frame-loop bench vs the 16.67 ms
budget (`EV-PERF-BUDGETS` Path B).

## Load-bearing files (for review)

`engine/render/src/{multipass.rs,scene_adapter.rs,audio_adapter.rs}` (R-0/R-3);
`app/shell/src/{lib.rs:241-243,state/mod.rs,Cargo.toml,capabilities/default.json}`
+ new `app/shell/src/render/` (R-1); `src/lib/widgets/SceneCanvas.svelte`,
`src/lib/views/Perform.svelte`, `app/ui-bridge/src/lib.rs` (R-2);
`docs/adr/ADR-011-rendering-backend.md`, `docs/adr/ADR-018-non-scene-ui-toolkit.md`
(amendments — AO-held).
