---
doc-type: reference
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Hotkey Reference

Chromaticity ships a fixed operator hotkey vocabulary covering scene and
playlist transport, Snap-slot recall and capture, render and display
control, the meta colour pipeline, audio threshold management,
diagnostics, and show-state actions. Press `?` at any time to toggle the
in-app overlay that lists the same vocabulary against the active focus
context.

Shortcuts marked *(planned)* are part of the documented vocabulary but
are not wired yet; pressing them does nothing. The in-app overlay shows the
same rows dimmed.

The V1 hotkey map is read-only. Rebinding ships in V2 per the operator
hotkey-map spec. The `cap` column on each row indicates whether the
action is a free MVP capability or whether it requires an entitlement to
fire; entitlement-gated rows fail closed when the matching capability is
absent.

## Control-focused mode

Active only while a single control has keyboard focus. `Esc` exits
control-focused mode.

| Key | Action |
| --- | --- |
| `D` | Set control to its default value |
| `R` | Set control to a random value in its range |
| `P` | Set control to its value in the current preset *(planned)* |
| `L` | Toggle the control's lock state (ignore randomization) |

## Scene and playlist transport

| Key | Action | OSC | cap |
| --- | --- | --- | --- |
| `Space` | Toggle playlist play/pause | `/playlist/toggleplay` | free |
| `->`, `K` | Next scene in playlist | `/playlist/next` | free |
| `<-`, `J` | Previous scene in playlist | `/playlist/previous` | free |
| `Alt+Cmd+->`, `Ctrl+Alt+->` | Jump to last in playlist *(planned)* | `/playlists/position -1` | free |
| `Alt+Cmd+<-`, `Ctrl+Alt+<-` | Jump to first in playlist *(planned)* | `/playlists/position 0` | free |
| `Enter` | Launch selected scene from the library | `/scenes/{name}` | free |
| `Shift+Enter` | Launch selected scene with preset prompt *(planned)* | `/scenes/{name} {preset}` | free |
| `Cmd+Enter`, `Ctrl+Enter` | Launch selected scene into a playlist slot *(planned)* | (internal) | free |

## Snap slots

Per-scene quick-recall preset slots. The wire literal `favslot` is
retained under `compat-syn-v1` for OSC compatibility; the operator
surface uses the name "Snap".

| Key | Action | OSC | cap |
| --- | --- | --- | --- |
| `1` to `9`, `0` | Recall Snap slot 1 to 10 for the current scene (`1`–`8` wired in Perform; `9`, `0` planned) | `/favslots/{n}` (apply) | free |
| `Shift+1` to `Shift+0` | Save current state to Snap slot 1 to 10 (overwrite without prompt) *(planned)* | `/favslots/{n}/save` | free |
| `Alt+Shift+1` to `Alt+Shift+0` | Clear Snap slot 1 to 10 (confirmation prompt) *(planned)* | `/favslots/{n}/clear` | free |
| `Cmd+S`, `Ctrl+S` | Save current state to the next empty Snap slot *(planned)* | `/favslots/{auto}/save` | free |
| `Cmd+Opt+S`, `Ctrl+Alt+S` | Save as a new named preset (prompts for name) *(planned)* | `/presets/new {name}` | free |
| `Cmd+0`, `Ctrl+0` | Recall the scene's "gold" preset (Snap 0) *(planned)* | `/favslots/0` (apply) | free |

## Display and output

| Key | Action | cap |
| --- | --- | --- |
| `F` | Toggle fullscreen on the primary output | free |
| `Shift+F` | Toggle fullscreen on the secondary output (multi-display) *(planned)* | free |
| `Cmd+Shift+F`, `Ctrl+Shift+F` | Cycle output window placement across displays *(planned)* | free |
| `Cmd+\`, `Ctrl+\` | Swap primary and secondary output *(planned)* | free |

## Rendering

`Cmd+0` is globally bound to Snap 0; the former "reset resolution"
binding has moved to `Cmd+9`.

| Key | Action | OSC | cap |
| --- | --- | --- | --- |
| `.` | Toggle render loop on/off *(planned)* | `/render/enabled {0|1}` | free |
| `,` | Toggle render-preview window *(planned)* | (internal) | free |
| `Cmd+=`, `Ctrl+=` | Step render resolution up *(planned)* | `/render/resolution {w} {h}` | `render.custom-resolution` |
| `Cmd+-`, `Ctrl+-` | Step render resolution down *(planned)* | `/render/resolution {w} {h}` | `render.custom-resolution` |
| `Cmd+9`, `Ctrl+9` | Reset render resolution to scene default *(planned)* | `/render/resolution {sceneW} {sceneH}` | `render.custom-resolution` |
| `Cmd+Shift+=`, `Ctrl+Shift+=` | Open FPS target dialog *(planned)* | `/render/fps {float}` | free |

## Meta controls

Each row enters a modal slider-adjust mode (arrow keys or scroll wheel).
`Esc` exits the modal. The `glsl` column names the meta-colour uniform
the binding maps onto.

| Key | Action | glsl |
| --- | --- | --- |
| `M` | Master *(planned)* | `_Master` |
| `B` | Brightness | `_Brightness` |
| `G` | Gamma *(planned)* | `_Gamma` |
| `C` | Contrast *(planned)* | `_Contrast` |
| `H` | Hue *(planned)* | `_Hue` |
| `Y` | Saturation *(planned)* | `_Saturation` |
| `I` | Invert toggle (no modal; binary flip) *(planned)* | `_Invert` |
| `V` | Vertical mirror toggle *(planned)* | `_Vert_Mirror` |
| `Shift+V` | Horizontal mirror toggle *(planned)* | `_Hor_Mirror` |
| `E` | Dichrome toggle (from "Edition") *(planned)* | `_Limit_Colors` |

## Audio

| Key | Action | OSC | cap |
| --- | --- | --- | --- |
| `A` | Toggle audio-input mute *(planned)* | (internal) | free |
| `Cmd+A`, `Ctrl+A` | Open audio settings panel *(planned)* | (internal) | free |
| `T` | Toggle audio-threshold visualizer overlay *(planned)* | (internal) | free |
| `Shift+T` | Set audio threshold to current RMS level | `/audio/threshold {f}` (absolute) | free |

## Navigation panels

| Key | Action |
| --- | --- |
| `Cmd+K`, `Ctrl+K` | Open command palette (searches scenes, presets, settings, help) |
| `Cmd+/`, `Ctrl+/` | Open help and docs search |
| `Cmd+L`, `Ctrl+L` | Open scene library |
| `Cmd+,`, `Ctrl+,` | Open settings |
| `Cmd+O`, `Ctrl+O` | Open OSC and MIDI mapping panel |

## Diagnostics and debug

| Key | Action |
| --- | --- |
| `Alt+Cmd+D`, `Ctrl+Alt+D` | Toggle diagnostics console *(planned)* |
| `Alt+Cmd+P`, `Ctrl+Alt+P` | Toggle performance HUD *(planned)* |
| `Alt+Cmd+L`, `Ctrl+Alt+L` | Toggle OSC and MIDI learn indicator *(planned)* |
| `Cmd+R`, `Ctrl+R` | Reload current scene *(planned)* |
| `Alt+Cmd+R`, `Ctrl+Alt+R` | Reset meta controls to defaults *(planned)* |

## Show state

| Key | Action |
| --- | --- |
| `Cmd+Shift+N`, `Ctrl+Shift+N` | New show (confirmation if unsaved changes) *(planned)* |
| `Cmd+Shift+O`, `Ctrl+Shift+O` | Open `.chromaShow` bundle *(planned)* |
| `Cmd+Shift+S`, `Ctrl+Shift+S` | Save show as `.chromaShow` bundle *(planned)* |
| `Cmd+Z`, `Ctrl+Z` | Undo (operator-level; one level for preset recall, meta tweak, scene switch) *(planned)* |
| `Cmd+Shift+Z`, `Ctrl+Shift+Z` | Redo *(planned)* |

---

*Authored from `src/lib/data/hotkeys.ts` (canonical 57-row chromaticity
hotkey vocabulary, mirrored from the upstream
`docs/42-operator-hotkey-map.md` spec). Clean-room per
`docs/29-cleanroom-legal-policy-baseline.md`.*
