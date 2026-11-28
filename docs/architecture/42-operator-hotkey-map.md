# Operator Hotkey Map (Normative)

This document locks the canonical hotkey table that the UX team implements in the Settings -> Hotkeys panel. V1 ships the table below as defaults; V2 permits operator rebinding. Bindings here are orthogonal to the authoring-environment hotkeys in `37-authoring-environment-spec.md` Sec.4 - authoring hotkeys apply only when focus is in the authoring panel; operator hotkeys apply everywhere else.

This document is the normative source. Any hotkey that appears in the embedded docs, tutorials, marketing copy, or in-app help must match this table.

## Scope distinction

- **Operator hotkeys** (this doc): used during live performance with focus on the main operator window.
- **Authoring hotkeys** (`37-authoring-environment-spec.md` Sec.4): used inside the scene authoring environment.
- **Control-focused hotkeys**: the reference-parity set `D / R / P / L` per `03-product-requirements.md` FR-17; these apply only when a specific control is focused.

## Conventions

- macOS modifier notation: `Cmd` = Cmd, `Alt` = Option, `Ctrl` = Control, `Shift` = Shift, `Space` = Space, `Enter` = Return, `Esc` = Escape.
- Windows modifier notation: `Ctrl`, `Alt`, `Shift`, `Win`.
- Cross-platform rows list macOS first, then Windows after `/`.
- All modifiers use the command-key convention of the host OS: `Cmd` on macOS maps to `Ctrl` on Windows unless a row calls out a distinction.

## 1) Control-focused hotkeys (reference parity; inherited from FR-17)

When a single control is focused in the control panel, these apply:

| Key | Action | Notes |
|---|---|---|
| `D` | Set control to its default value | Reference-parity. |
| `R` | Set control to a random value in its range | Reference-parity. |
| `P` | Set control to its value in the current preset | Reference-parity. |
| `L` | Toggle the control's lock state (ignore randomization) | Reference-parity. |

Control focus is established by mouse click or by tabbing. `Esc` exits control-focused mode.

## 2) Scene and playlist transport

| Key | Action | OSC route | Capability |
|---|---|---|---|
| `Space` | Toggle playlist play/pause | `/playlist/toggleplay` | free |
| `->` or `K` | Next scene in playlist | `/playlist/next` | free |
| `Left` or `J` | Previous scene in playlist | `/playlist/previous` | free |
| `Alt+Cmd->` / `Ctrl+Alt+->` | Jump to last in playlist | `/playlists/position -1` | free |
| `Alt+Cmd+Left` / `Ctrl+Alt+Left` | Jump to first in playlist | `/playlists/position 0` | free |
| `Enter` | Launch selected scene from the library | `/scenes/{name}` | free |
| `Shift+Enter` | Launch selected scene with preset prompt | `/scenes/{name} {preset}` | free |
| `Cmd+Enter` / `Ctrl+Enter` | Launch selected scene into a playlist slot | (internal) | free |

## 3) Snap slots

Chromaticity's per-scene quick-recall preset slots are called "Snap slots" (see `03-product-requirements.md` FR-18 and `29-cleanroom-legal-policy-baseline.md` Sec.Brand/Trademark Review Checklist). The `compat-syn-v1` OSC route surface retains the `favslot` literal.

| Key | Action | OSC route | Capability |
|---|---|---|---|
| `1` - `9`, `0` | Recall Snap slot 1-10 for the current scene | `/favslots/{n}` (apply) | free |
| `Shift+1` - `Shift+0` | Save current state to Snap slot 1-10 (overwrite without prompt) | `/favslots/{n}/save` | free |
| `Alt+Shift+1` - `Alt+Shift+0` / `Ctrl+Alt+Shift+1` - `Ctrl+Alt+Shift+0` | Clear Snap slot 1-10 (confirmation prompt) | `/favslots/{n}/clear` | free |
| `Cmd+S` / `Ctrl+S` | Save current state to the next empty Snap slot | `/favslots/{auto}/save` | free |
| `Cmd+Opt+S` / `Ctrl+Alt+S` | Save as a new named preset (prompts) | `/presets/new {name}` | free |
| `Cmd+0` / `Ctrl+0` | Recall the scene's "gold" preset (Snap 0) | `/favslots/0` (apply) | free |

## 4) Display / output

| Key | Action | OSC route | Capability |
|---|---|---|---|
| `F` | Toggle fullscreen on the primary output | (internal) | free |
| `Shift+F` | Toggle fullscreen on the secondary output (multi-display) | (internal) | free |
| `Cmd+Shift+F` / `Ctrl+Shift+F` | Cycle output window placement across displays | (internal) | free |
| `Cmd\` / `Ctrl+\` | Swap primary and secondary output | (internal) | free |

## 5) Rendering

| Key | Action | OSC route | Capability |
|---|---|---|---|
| `.` | Toggle render loop on/off | `/render/enabled {0\|1}` | free |
| `,` | Toggle render-preview window | (internal) | free |
| `Cmd+=` / `Ctrl+=` | Step render resolution up | `/render/resolution {w} {h}` | `render.custom-resolution` |
| `Cmd+-` / `Ctrl+-` | Step render resolution down | `/render/resolution {w} {h}` | `render.custom-resolution` |
| `Cmd+9` / `Ctrl+9` | Reset render resolution to scene default | `/render/resolution {sceneW} {sceneH}` | `render.custom-resolution` |
| `Cmd+Shift+=` / `Ctrl+Shift+=` | Open FPS target dialog | `/render/fps {float}` | free |

`Cmd+0` is globally bound to Snap 0 recall (see §3). The former "Cmd+0 in Render menu = reset resolution" binding has been moved to `Cmd+9` to remove the context-sensitive collision; Render menu items also work as Settings -> Render buttons for mouse-driven workflows.

## 6) Meta controls

Each of these enters a modal slider-adjust mode where arrow keys or the scroll wheel modifies the value. `Esc` exits the mode.

| Key | Meta control | GLSL symbol |
|---|---|---|
| `M` | Master | `_Master` |
| `B` | Brightness | `_Brightness` |
| `G` | Gamma | `_Gamma` |
| `C` | Contrast | `_Contrast` |
| `H` | Hue | `_Hue` |
| `Y` | Saturation | `_Saturation` |
| `I` | Invert toggle (no modal; binary flip) | `_Invert` |
| `V` | Vertical mirror toggle | `_Vert_Mirror` |
| `Shift+V` | Horizontal mirror toggle | `_Hor_Mirror` |
| `E` (from "Edition") | Dichrome toggle | `_Limit_Colors` |

## 7) Audio

| Key | Action | OSC route | Capability |
|---|---|---|---|
| `A` | Toggle audio-input mute | (internal) | free |
| `Cmd+A` / `Ctrl+A` | Open audio settings panel | (internal) | free |
| `T` | Toggle audio-threshold visualizer overlay | (internal) | free |
| `Shift+T` | Set audio threshold to current RMS level | `/audio/threshold/sample` | free |

## 8) Navigation panels

| Key | Action |
|---|---|
| `Cmd+K` / `Ctrl+K` | Open command palette (searches scenes, presets, settings, help) |
| `Cmd/` / `Ctrl+/` | Open help / docs search |
| `Cmd+L` / `Ctrl+L` | Open scene library |
| `Cmd,` / `Ctrl+,` | Open settings |
| `Cmd+O` / `Ctrl+O` | Open OSC / MIDI mapping panel |

## 9) Diagnostics and debug

| Key | Action |
|---|---|
| `Alt+Cmd+D` / `Ctrl+Alt+D` | Toggle diagnostics console |
| `Alt+Cmd+P` / `Ctrl+Alt+P` | Toggle performance HUD |
| `Alt+Cmd+L` / `Ctrl+Alt+L` | Toggle OSC/MIDI learn indicator |
| `Cmd+R` / `Ctrl+R` | Reload current scene |
| `Alt+Cmd+R` / `Ctrl+Alt+R` | Reset meta controls to defaults |

## 10) Show state

| Key | Action |
|---|---|
| `Cmd+Shift+N` / `Ctrl+Shift+N` | New show (with confirmation if unsaved changes) |
| `Cmd+Shift+O` / `Ctrl+Shift+O` | Open `.chromaShow` bundle |
| `Cmd+Shift+S` / `Ctrl+Shift+S` | Save show as `.chromaShow` bundle |
| `Cmd+Z` / `Ctrl+Z` | Undo (operator-level; one level for preset recall, meta tweak, scene switch) |
| `Cmd+Shift+Z` / `Ctrl+Shift+Z` | Redo |

## Conflict and priority rules

1. **Control-focused D/R/P/L** (Sec.1) take priority over all other bindings when a control is focused. `Esc` is the always-available exit.
2. **Modal meta-control slider mode** (Sec.6) intercepts arrow keys and scroll events; `Esc` exits, which returns those keys to their default bindings.
3. **Authoring-env hotkeys** (`37-authoring-environment-spec.md` §4) take priority when focus is inside the authoring panel. Notable shared-key cases:
   - `Cmd+R` / `Ctrl+R`: operator context reloads the current scene (§9 Diagnostics); authoring context forces a full scene-package rebuild. Both actions are semantically reload; the focus context selects the appropriate scope.
   - `Cmd+S` / `Ctrl+S`: operator context saves to the next empty Snap slot (§3); authoring context saves all dirty tabs. These are different operations; focus context selects.
   - `Cmd+P` / `Ctrl+P`: operator context is unbound; authoring context invokes Quick-Open-File.
   - `Cmd+F` / `Ctrl+F`: operator context is unbound (plain `F` toggles fullscreen per §4 Display); authoring context invokes Find-in-File.
   - `Cmd+/` / `Ctrl+/`: operator context opens help/docs search (§8); authoring context toggles line comment.
4. **Command palette** (`Cmd+K`) can be invoked from anywhere, including inside control focus; it takes over input until dismissed.
5. **Text-input fields** (preset name, mapping label, search) always pass through text characters and disable application-level bindings while focused.
6. **No binding is silent-overridden by the reference app's presence**; if both Chromaticity and the reference are installed and running, neither captures the other's hotkeys.

## V1 rebinding posture

- V1 ships all bindings above as read-only defaults in Settings -> Hotkeys.
- V1 does not ship custom rebinding UI; every binding is shown for reference and a short "rebinding coming in V2" affordance.
- V2 ships a rebinding UI; conflicts are resolved with a warning pattern (not silent overwrite).

## Evidence

- `EV-HOTKEY-PARITY-DESIGN` (Phase 0 ratification) - validates that this document locks the D/R/P/L reference-parity bindings per FR-17 in `03-product-requirements.md` and that the table has been reviewed by the AI UX Workcell.
- `EV-HOTKEY-PARITY-CONFORMANCE` (Phase 4a exit) - runtime conformance: the D/R/P/L bindings fire the documented actions under control focus.
- `EV-HOTKEY-COMPLETENESS` (Phase 4a exit) - validates every row in Sec.1-10 is bound at launch to the action named; the in-app Settings -> Hotkeys view matches this document.
- `EV-HOTKEY-CONFLICT` (Phase 4a exit) - validates the conflict-and-priority rules above; no two bindings silently fire the same key in the same focus context.

## Traceability

- `03-product-requirements.md` FR-17 Operator Hotkey Map - normative source for the D/R/P/L subset and for the requirement that this table exists.
- `37-authoring-environment-spec.md` Sec.4 - authoring-env hotkeys, orthogonal to this table.
- `08-io-integrations.md` - OSC route surface that some hotkeys emit to.
- `adr/ADR-009-activation-and-entitlements.md` - capability-gating for resolution hotkeys.
- `22-compatibility-versioning-policy.md` Sec.Package Extensions - `.chromaShow` open/save hotkeys.
- `29-cleanroom-legal-policy-baseline.md` Sec.Brand/Trademark Review Checklist - Snap vs favslot naming.
- `adr/ADR-010-embedded-docs-generation.md` - the `/manual/hotkeys` docs page renders this table; the build-time docs generator reads this document as its single source of truth.
