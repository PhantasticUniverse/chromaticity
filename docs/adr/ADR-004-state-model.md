# ADR-004: Canonical State Model for Controls, Presets, and Playlists

## Status
Accepted (planning baseline)

## Context
Chromaticity needs predictable behavior across live control input, scripts, presets, playlists, and external protocol updates. Fragmented state ownership creates race conditions and non-determinism.

## Decision
Define a canonical state model with event-sourced updates:
- ControlStore for scene + meta controls.
- PresetStore for per-scene named presets and startup preset pointers.
- PlaylistStore for sequence state and transport actions.
- MappingStore for protocol mappings (OSC/MIDI).

All external and internal updates become typed commands validated before commit.

## Consequences
### Positive
- Deterministic and auditable state transitions.
- Easier sync with integrations and UI.
- Reduced race condition surface.

### Negative
- Requires command/event schema governance.

## Alternatives Considered
- Direct mutable state from each subsystem (rejected: high coupling and drift).

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The four-store decomposition (`ControlStore` / `PresetStore` / `PlaylistStore` / `MappingStore`) plus the typed-command-validated-before-commit posture is the architectural precondition for the state-class evidence set; direct subsystem mutation would have made every row below either untestable or reducible to a non-deterministic regression-suite.

- `EV-STATE-REPLAY` — deterministic state transitions are the row's direct measurement; the canonical state + typed-commands posture is what makes the transitions deterministic and replayable. Refines `adr/ADR-001-runtime-model.md`'s "single canonical state model" decision into the per-store split this row tests.
- `EV-STATE-MIGRATION` — state / preset / playlist migration and rollback transforms; the canonical model declared here is the schema that `32-state-preset-playlist-migration-policy.md` migrates between versions of.
- `EV-PRESET-PLAYLIST` — `PresetStore` and `PlaylistStore` are the row's direct subjects; the per-scene named-preset + startup-preset-pointer + playlist-transport contract are properties of the stores defined here.
- `EV-DETERMINISM` — typed commands validated before commit is the runtime-determinism precondition `engine/runtime/src/command_bus.rs` realizes (per `docs/26` `EV-INTEGRATION-MATRIX` Pass at slice-2). The canonical state's commit-thread posture (`adr/ADR-006-thread-and-process-model.md`) is the threading half; this ADR is the data-model half.
- `EV-INTEGRATION-MATRIX` — `MappingStore` is the canonical sink for OSC/MIDI mapping bindings; the conformance matrix asserts integrations write through `MappingStore` rather than directly into per-protocol state.
- `EV-SHOW-BUNDLE-RECOVERY` — show-state recovery serializes the canonical state; without the per-store decomposition declared here the bundle would be an opaque blob with no per-class restoration semantics.
- `EV-CRASH-RESUME` — per-class state-restoration acceptance criteria in `docs/26` `EV-HOTKEY-CONFLICT` (active scene + preset values, Snap slot selection, playlist position, OSC/MIDI bindings, audio device, meta-control values, render resolution + FPS) map one-for-one onto the four stores; the row's restoration scope was authored against this ADR's decomposition.

## Traceability

- `07-control-state-presets.md` — primary planning doc for the canonical state model.
- `32-state-preset-playlist-migration-policy.md` — schema migration and rollback transforms over the model declared here.
- `08-io-integrations.md` — `MappingStore` contract for OSC/MIDI bindings.
- `28-implementation-contract-pack.md` CP-04 (typed command schema), CP-09 (preset/playlist contract), CP-12 (parse-reserve rule for forward-compat keys in serialized state).
- `adr/ADR-001-runtime-model.md` — "single canonical state model" decision this ADR refines into the four-store decomposition.
- `adr/ADR-006-thread-and-process-model.md` — `StateCore` on the state commit thread; the threading half of this ADR's typed-command-validated-before-commit posture.
- `adr/ADR-005-integration-adapter-boundary.md` — `MappingStore` is the canonical sink the adapter boundary writes through.
- `adr/ADR-007-scene-warm-switch-policy.md` — preserves preset/control state across scene transitions per the model declared here.
- `13-risk-register.md` R-06 (state race conditions across input sources) — typed-commands-validated-before-commit is the closure path; closes once `EV-STATE-REPLAY` reaches Pass on canonical replay corpus.
- `03-product-requirements.md` FR-6 (Controls and State), FR-7 (Presets and Playlists), FR-12 (Show-State Safety); NFR-2 (Determinism).
