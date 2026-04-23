# State, Preset, and Playlist Migration Policy

## Purpose
Define compatibility and migration guarantees for control state, presets, playlists, and mapping state.

## Policy Alignment
- This policy mirrors the lifecycle guarantees in `22-compatibility-versioning-policy.md`.
- Scene schema compatibility and state/preset compatibility must be versioned independently but released together.

## Version Axes
- **State Schema Version**: canonical state object contract revision.
- **Preset Schema Version**: serialized preset payload revision.
- **Playlist Schema Version**: playlist transport and transition payload revision.
- **Mapping Schema Version**: OSC/MIDI mapping payload revision.

## Guarantees
- Minor version updates are backward compatible within the same major version.
- Any major version update requires:
 - migration tooling,
 - compatibility fallback behavior,
 - migration guide with rollback path.

## Migration Rules
- Deterministic migration transforms only; no nondeterministic defaults.
- Unknown fields are preserved in extension namespace where safe.
- Corrupted state payloads are rejected with no partial commit.
- Import path supports dry-run validation before commit.

## Rollback Policy
- Every migration writes pre-migration snapshot.
- Rollback must restore prior state/preset/playlist payloads atomically.
- Rollback metadata includes migration version, timestamp, and operator identifier.

## Validation and Evidence
- `EV-STATE-REPLAY`: deterministic state transition validation.
- `EV-PRESET-PLAYLIST`: preset/playlist semantics validation.
- `EV-SHOW-BUNDLE-RECOVERY`: end-to-end export/import/rollback validation.
- `EV-STATE-MIGRATION`: migration transform and rollback conformance tests.
