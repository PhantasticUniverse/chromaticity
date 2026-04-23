# Control, State, and Preset System Blueprint

## Objective
Define deterministic semantics for control mutation, state persistence, presets, and playlist behaviors under local and external inputs.

## Canonical Stores
- **ControlStore**
 - Scene controls + meta controls.
 - Current value, default, lock state, group metadata.
- **PresetStore**
 - Preset collections keyed by scene.
 - Startup preset pointers and favorite-slot semantics.
- **PlaylistStore**
 - Active playlist, index, iteration mode, trigger policies.
- **MappingStore**
 - OSC/MIDI/global control mapping metadata.

## Command Model
- All changes pass through typed commands:
 - `SetControl`
 - `SetControlNormalized`
 - `RandomizeControl`
 - `DefaultControl`
 - `ApplyPreset`
 - `CreatePreset`
 - `RenamePreset`
 - `DeletePreset`
 - `StartSceneWithPreset`
 - `PlaylistNext`
 - `PlaylistSetActive`

## Ordering Rules
- Within a frame:
  1. External protocol input commands.
  2. Internal scene/script commands.
  3. Playlist transport commands.
- Conflict rule: last valid command in deterministic ordering wins.

## Preset Semantics
- Presets serialize only scene-relevant control state unless explicitly configured to include meta controls.
- Missing control fields on load use scene defaults.
- Unknown controls in older presets are ignored with warning.

## Playlist Semantics
- Playlist may specify scene-only transition or scene+preset transition.
- Auto-iteration policy configurable and externally triggerable.
- Playlist transitions should be atomic from state perspective.

## Compatibility Semantics
- Support scene-specific and global control addressing models.
- Support raw and normalized control update semantics.
- Preserve behavior of bank/group default and random actions.

## Fault Handling
- Invalid control address or preset reference returns structured non-fatal error.
- Store corruption protection via schema validation + backup snapshots.

## Implementation Contract Baseline
- State schema and command/event definitions are locked in `28-implementation-contract-pack.md`.
- Canonical store and command authority decisions are governed by `adr/ADR-004-state-model.md`.
- Persistence versioning and migration strategy is governed by `32-state-preset-playlist-migration-policy.md`.
