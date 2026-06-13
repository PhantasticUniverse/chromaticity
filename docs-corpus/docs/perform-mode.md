<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Perform Mode

Perform mode is the operator's main runtime surface during a show. It
arranges live scene controls, the meta colour pipeline, the media
strip, the preview hero, the audio-reactivity panel, and the scene
rail into a single dense layout that is reachable without opening
nested panels.

## The five panels

The Perform view is laid out across three columns:

| Column | Panels |
| --- | --- |
| Left | Scene Controls, Meta Controls, Media Strip |
| Middle | Preview hero, Audio Reactivity panel |
| Right | Scene rail with Perform / Edit toggle, Setlist selector, Prev / Next / Shuffle row, scrollable scene list |

Every interactive element on the surface is reachable through both
mouse and keyboard.

## Scene Controls

The Scene Controls panel surfaces every author-declared control for
the currently active scene as a typed widget — sliders for numeric
parameters, toggles for booleans, swatches for colours, and combo
selectors for enumerations. Each focused widget answers the standard
keyboard vocabulary: arrow keys nudge the value, `PageUp` /
`PageDown` step coarsely, `Home` / `End` jump to the range ends, and
`Space` / `Enter` flip toggles. Above the panel, three quick-action
buttons operate on every control at once — **Default** resets to
declared defaults, **Random** randomises within declared ranges
(locked rows are skipped), and **Lock-all** toggles the per-row
locks; per-row Lock / Reset / Random affordances cover single
controls.

The control-focused single-key vocabulary (`D` default, `R` random,
`P` preset, `L` lock) is reserved in the operator hotkey map and is
not active in this build. The full operator hotkey vocabulary lives
at [Hotkey Reference](hotkeys-reference.md).

## Meta Controls

The Meta Controls panel exposes the scene-agnostic colour pipeline:
master, brightness, gamma, contrast, hue, saturation, invert, vertical
mirror, horizontal mirror, dichrome. Each meta control binds onto a
canonical `_Meta*` uniform that is published to every scene; authors
do not opt in or out of the pipeline. Meta controls are adjusted
directly through their widgets; the `M` / `B` / `G` / `C` / `H` / `Y`
modal slider-adjust vocabulary is reserved in the operator hotkey map
and is not active in this build.

## The Preview hero

The Preview hero renders a canvas-2d procedural visualisation of the
active scene, keyed off the scene's metadata. It is wireframe-faithful
structural feedback — rotation, warp, and audio reactivity respond
visibly — not the rendered GLSL output, so it is not pixel-identical
to the audience-facing output surface (see the preview warning in
[Quickstart](quickstart.md)).
Beneath the preview, the [Audio Reactivity](audio-reactivity.md)
panel surfaces the live audio uniforms — four named band tiles and
the BPM strip — plus the 48-bar FFT visualiser, which stays
synthetic-driven in the V1 release.

## The Scene rail

The right-column scene rail surfaces:

- A four-button rail to switch between Scenes / Presets / Edit /
  Add modes.
- A Perform / Edit toggle that gates the Author surface (Edit mode is
  capability-gated by `authoring.edit`).
- A Setlist selector pinning the active playlist.
- A Prev / Next / Shuffle row mirroring the playlist transport
  hotkeys (`<-` / `->` / `Space`).
- A scrollable list of every scene in the active setlist, with the
  current scene highlighted.

## The Snap bar

A Snap bar persists across the bottom of Perform mode: ten quick-recall
slots, addressed by `1` through `0` in operator focus. Saving uses
`Shift+1` through `Shift+0`; clearing uses `Alt+Shift+1` through
`Alt+Shift+0`. Slot zero is the "gold preset" and is recallable through
`Cmd+0` / `Ctrl+0` as a guaranteed safe state.

The Snap surface is the chromaticity-native naming for what the
`compat-syn-v1` OSC routes still call `favslots` — see
[OSC Routes](osc-routes.md) for the wire-level mapping.

## Status-bar adapter pills

A status bar across the bottom of Perform mode renders one pill per
runtime adapter — audio, OSC, MIDI, camera, Syphon (macOS only).
Each pill surfaces:

- A connectivity dot (online / degraded / offline).
- The bound interface or device name.
- A click target opening the per-adapter inspector.

When an adapter is degraded, a one-line banner appears at the top of
the view explaining the cause and pointing at Settings → Sources for
the recovery action.

## Why Perform mode is the default

Perform mode is the surface every operator hits first because it
balances density with safety: every common live action is reachable
without nesting, and the destructive actions (Snap clear, scene
delete) require either a confirmation prompt or a modifier.
The Author and Library views are routed under their own sidebar
entries for the iteration loops where density beats safety the other
way around.

---

<!-- Clean-room authored from src/lib/views/Perform.svelte (W18 PR1 + W26 PR1 layout commentary), src/lib/data/hotkeys.ts (operator hotkey vocabulary), docs/42-operator-hotkey-map.md, and docs/29-cleanroom-legal-policy-baseline.md sec.Wave-1c-β threat-model addendum (adapter-pill semantics); never derived from Contents/ or chromaticity-planning/ per docs/29. -->
