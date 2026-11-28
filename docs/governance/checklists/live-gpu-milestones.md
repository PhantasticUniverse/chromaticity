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
| 2a | Live `syn_UserImage` GPU texture upload | Next | `_chroma_sample_media` is still a black stub in `app/shell/src/render.rs`; `syn_MediaActive` is hard-wired `0.0`. Needs a media texture binding slot after pass inputs in both `CachedSinglePass` and `CachedMultipass`. |
| 1 | Native `wgpu` stage swapchain + GPU meta post-pass | Open | Replaces readback + CPU `apply_meta_color_pass` + IPC Canvas2D blit. `MultipassRenderer::instance()` already exposed for surfaces. |
| 3 | Real audio in local REC/replay + 4-bus `BroadcastMixer` | Open | Replace `anullsrc` on non-macOS. |
| 4 | GPU crossfades, Syphon/Spout/NDI, native compositor in Mix | Open | `TransitionOrchestrator` still uses `commit_hard_cut()`. |
| 5 | Mix production features + fixture cleanup | Open | Scene collections, per-layer capture, scene-scoped presets, >16 controls. |

## Known gaps carried by 2b

- Two consumers at the SAME resolution share one pipeline, so each advances
  feedback once per request (history moves at the combined request rate).
- `CachedSinglePass` still reallocates its target when consumers at
  different sizes alternate (stateless, so only a perf cost).
- Feedback is verified with a synthetic trail scene; no bundled scene declares
  `PERSISTENT` passes yet.
