---
doc-type: howto
---

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
selectors for enumerations. Each control row carries a per-control
hotkey-hint badge; while a control has keyboard focus, the
control-focused hotkeys take effect:

| Key | Action |
| --- | --- |
| `D` | Set the focused control to its default value |
| `R` | Set the focused control to a random value in its range |
| `P` | Set the focused control to its value in the active preset |
| `L` | Toggle the lock state (the locked control ignores randomisation) |

The full operator hotkey vocabulary lives at
[Hotkey Reference](hotkeys-reference.md).

## Meta Controls

The Meta Controls panel exposes the scene-agnostic colour pipeline:
master, brightness, gamma, contrast, hue, saturation, invert, vertical
mirror, horizontal mirror, dichrome. Each meta control binds onto a
canonical `_Meta*` uniform that is published to every scene; authors
do not opt in or out of the pipeline. The hotkeys `M` / `B` / `G` /
`C` / `H` / `Y` enter modal slider-adjust mode against the matching
meta control; `Esc` exits.

## The Preview hero

The Preview hero shows the live render output. It mirrors the actual
output surface so the operator sees exactly what the audience sees.
Beneath the preview, the [Audio Reactivity](audio-reactivity.md)
panel surfaces the live audio uniforms — four named band tiles, the
BPM strip, and the FFT visualiser.

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
