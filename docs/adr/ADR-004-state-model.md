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
