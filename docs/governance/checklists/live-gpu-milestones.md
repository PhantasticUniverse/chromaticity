<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Live GPU / Output Milestones — Progress Tracker

Resumable plan for the post-`3471e5b` milestone sequence (agent handoff of
2026-11-28). Update the status column as each slice lands so any session can
pick up the next item.

| # | Milestone | Status | Notes |
| - | --------- | ------ | ----- |
| 2b | Persistent multipass pipeline across live frames | Done | `CachedMultipass` in `engine/render/src/multipass/cached.rs`; shell keys up to 3 pipelines by resolution in `GpuPreviewEngine`. Byte-equivalence pins vs the one-shot scheduler in `multipass.rs` `cached_multipass_equivalence`. |
| 2a | Live `chroma_Media` GPU texture upload | Done | Engine `MediaInput` at bind group 1 (`engine/render/src/media_input.rs`); shell ingest `push_scene_media_frame` / `clear_scene_media` (`app/shell/src/commands/media.rs`) synced on the render path; native media prelude in `app/shell/src/render.rs`; import bridge samples it (`syn_compat.rs`); webview producer `src/lib/stores/sceneMedia.svelte.ts` driven by the Perform MediaStrip; example `examples/media-mirror.chroma`. |
| 1 | Native `wgpu` stage swapchain + GPU meta post-pass | Next | Replaces readback + CPU `apply_meta_color_pass` + IPC Canvas2D blit. `MultipassRenderer::instance()` already exposed for surfaces. |
| 3 | Real audio in local REC/replay + 4-bus `BroadcastMixer` | Open | Replace `anullsrc` on non-macOS. Scope copied from the handoff; not re-checked. |
| 4 | GPU crossfades, Syphon/Spout/NDI, native compositor in Mix | Open | `TransitionOrchestrator` still uses `commit_hard_cut()`. |
| 5 | Mix production features + fixture cleanup | Open | Scene collections, per-layer capture, scene-scoped presets, >16 controls. Scope copied from the handoff; not re-checked. |

## Known gaps carried by 2b

- Two consumers at the SAME resolution share one pipeline, so each advances
  feedback once per request (history moves at the combined request rate).
- `CachedSinglePass` still reallocates its target when consumers at
  different sizes alternate (stateless, so only a perf cost).
- Any new resolution (window resize, DPR change) compiles a fresh pipeline
  and cold-starts its feedback; a fourth concurrent size evicts the least
  recently used consumer's history.
- `CachedMultipass` allocates a readback buffer per call and exposes no
  final-texture accessor yet; milestone 1 (native swapchain) needs one.
- Feedback is verified with a synthetic trail scene; all 25 bundled scenes
  declare `"PASSES": []`. Whether `import_syn_scene` carries `PERSISTENT`
  through to the imported `scene.json` has not been checked.

## Known gaps carried by 2a

- One untransformed `chroma_Media` (decision 2026-11-28): the catalog's
  media-transform Meta controls (`_Media_Contrast` etc.) are not applied, and
  there is no separate raw-media binding.
- Webview canvas → IPC → texture path is not exercised by any automated
  test; only verified in-process (push → snapshot → sync → render). Needs one
  manual run: Perform → test card with `examples/media-mirror.chroma` loaded.
- Producer caps media at 640×360 / 30 fps; frames are raw RGBA over IPC
  (about 0.9 MB each). Release-build cost of a fresh upload every frame:
  +0.12 ms median at 1280×720.
- YouTube (cross-origin iframe) cannot feed scene media.
- The five `.local/scenes/imported-syn` packages come from an older importer
  whose preamble predates the duplicate-helper guard and fail to compile
  (pre-existing); re-importing them with the current importer is the fix.
- Media selection is a single global source; per-layer capture is milestone 5.
