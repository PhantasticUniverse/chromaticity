# ADR-015: Reference-App User Data Import

## Status
Accepted

## Context
A reference-app user's accumulated work (preset collections, MIDI-learn mappings, favslot bindings, playlists, show state) represents real creative investment. Chromaticity can honor scene schema compatibility (`02-synesthesia-contracts.md`, `22-compatibility-versioning-policy.md`) yet still force users to rebuild every mapping and preset from scratch, which is a credible switching-cost wall with no engineering justification. Closing that gap is FR-19.

The reference-app user-data root is observable (not reference-source code, not copyrightable - it is a file-system location):
- macOS: `~/Library/Application Support/com.gravitycurrent.synesthesia/Data`
- Windows: `C:\Users\<USER>\AppData\Local\Synesthesia\Data`

File formats inside those roots are JSON (per the reference's own FAQ guidance about "one long line of text" that parses as JSON). Chromaticity inspects these files as data, not as code; all transform logic is original.

## Decision

### Scope (MVP)
The Chromaticity importer supports the following artifact classes from a reference-app user-data folder. Each class is individually selectable in the dry-run report; every class has its own `import-diagnostic` reason codes and its own row in `EV-IMPORT-REFERENCE-SHOW`.

- **Settings snapshot**: audio device selection, output/display preferences, OSC input/output ports, and any other reference-app settings that have a Chromaticity equivalent. Unknown keys preserved in `settings.extensions.reference-app-legacy` namespace (neutral wording; the reference-app product name is not embedded in shipped Chromaticity JSON keys per `29-cleanroom-legal-policy-baseline.md` Brand/Trademark Review Checklist).
- **Scene-specific presets**: per-scene named preset collections and startup preset pointers.
- **Snap slots** (reference literal: `favslots`): per-scene slots 0-15 including gold-slot-0 pointer and per-slot user name, mapped into Chromaticity's Snap store. Slot indexing and the gold-slot semantics match CP-05.
- **Meta/Media preset channels**: shared preset payloads covering meta-controls and media settings per CP-05 channel model.
- **Playlists**: scene lists with iteration mode and transition settings.
- **MIDI mappings**: per-scene and global CC/note->control bindings including soft-takeover, invert, and MIN/MAX per-mapping options (MIDI-CONF-01..10 surface).
- **OSC mapping aliases**: route alias tables and per-mapping source-IP allowlist entries, where present.
- **Hotkey remappings**: per-user operator-hotkey rebindings where the reference app persists them. Chromaticity V1 does not ship a rebinding UI (see `42-operator-hotkey-map.md` V1 rebinding posture), so imported rebinds that conflict with the V1 locked bindings are captured as diagnostics with reason `hotkey-v1-locked` and stored dormant; they activate automatically when the V2 rebinding UI ships. Control-focused D/R/P/L bindings (FR-17) are never rebindable and any import attempt emits `hotkey-control-focus-locked` without side effect.

All six importable classes have independent operator toggles in the dry-run UI; operators may import any subset atomically.

Out of scope:
- Show bundles authored by third-party tools.
- User-authored reference-app scene packages (`.synScene` / `.synScenePack`) and Chromaticity-native scenes (`.chroma` / `.chromaPack`): those flow through the scene-import path in FR-13/FR-14 and are converted per `38-scene-library-and-import-spec.md` Sec.5 and `37-authoring-environment-spec.md` Sec.3.7, not through this ADR.
- Shadertoy or ISF imports (those are FR-14).

### Mechanism

1. **Discovery**: operator chooses "Import from reference app" in Chromaticity (neutral menu label; the specific product name does not appear in the Chromaticity shipped UI per `29-cleanroom-legal-policy-baseline.md` Brand/Trademark Review Checklist). The tool auto-detects the canonical user-data path for the current OS and allows operator override if the reference install lives elsewhere. The dry-run summary heading and import-diagnostic copy also use "reference-app"/"reference user data" phrasing rather than the product name.
2. **Dry run**: importer parses all artifact JSON into an internal diff model. Produces a report: counts per class, symbol-mismatch warnings (unknown scene names, unknown control names, outlier control types), and compatibility-profile tags.
3. **Operator confirmation**: operator reviews the dry-run report and selects which artifact classes to import. No commit occurs without confirmation.
4. **Migration**: deterministic transforms per artifact class:
 - Settings -> Chromaticity settings with field mapping table; unknown keys preserved in `settings.extensions.reference-app-legacy` namespace.
 - Presets -> Chromaticity preset store keyed by scene name; per-control values mapped through the normalization rules in CP-01.
 - Snap slots -> Chromaticity Snap store keyed by scene name with slot index preserved; gold-slot-0 pointer preserved; per-slot user names preserved.
 - Playlists -> Chromaticity playlist store; iteration mode and transition settings mapped explicitly.
 - MIDI mappings -> `midi-parity-v1` bindings in the `MappingStore`; soft-takeover and invert flags preserved.
 - OSC aliases -> route alias tables in the IntegrationHub, canonicalized against the `compat-syn-v1` alias table in `08-io-integrations.md`.
 - Hotkey remappings -> dormant entries in the Chromaticity hotkey store tagged `source=imported` and `status=v1-locked-pending-v2` (see Hotkey remappings scope row).
5. **Compatibility profile**: all imported artifacts are tagged `compat-syn-v1`. They remain readable by the `native-chroma-v1` profile but round-trip behavior is only guaranteed under the imported profile.
6. **Rollback snapshot**: imports write a pre-migration snapshot per `32-state-preset-playlist-migration-policy.md`. Rollback restores the Chromaticity user-data root to its pre-import state atomically.
7. **Non-destructive**: the importer never mutates the source reference-app user-data folder. Only reads.

### Diagnostics
Every artifact that cannot be imported cleanly emits an `import-diagnostic` entry with:
- source path,
- artifact class (`settings` / `preset` / `snap` / `playlist` / `midi` / `osc` / `hotkey`),
- reason code: one of
  - `unknown-scene` - preset or Snap references a scene name not present in the Chromaticity library,
  - `unknown-control` - a control `NAME` does not exist on the target scene,
  - `outlier-control-type` - a control type is not in the CP-01 canonical set,
  - `coercion-applied` - a CR-01..04 coercion was used,
  - `unsupported-schema-version` - a reference artifact shape does not match any supported schema heuristic,
  - `hotkey-conflict` - imported hotkey collides with another imported binding,
  - `hotkey-v1-locked` - imported hotkey differs from the V1 locked binding and is stored dormant pending V2,
  - `hotkey-control-focus-locked` - import attempted to rebind a control-focused D/R/P/L hotkey (never rebindable),
  - `unknown-action` - imported hotkey references an action that has no Chromaticity equivalent,
  - `osc-alias-collision` - imported OSC alias would shadow a canonical Chromaticity route,
  - `midi-mapping-device-missing` - imported MIDI mapping references a device ID not present on the importing machine (binding restored; surfaces as `Disconnected` per `08-io-integrations.md` health state machine until the device appears).
- recommended operator action,
- dry-run / committed flag.

The full diagnostic log is exported alongside the pre-migration snapshot for retrospective review.

### Versioning
- Reference artifact JSON schemas can change across reference-app releases. Chromaticity's importer pins a supported set of reference schema versions; out-of-range versions emit a reject diagnostic ("unsupported-schema-version") without partial-commit.
- Schema version detection is best-effort: reference artifacts do not consistently embed a version field, so the importer uses shape heuristics (presence/absence of known keys) plus a fallback inspection mode that lets the operator force-import at their own risk.

### Clean-room posture
- Importer reads reference JSON as data; no reference source code is consulted for transform logic.
- Transform table entries (e.g. which reference key maps to which Chromaticity field) are produced from behavior observation + the Chromaticity schema; they are authored clean-room.
- No reference documentation text is reproduced in the importer UI; all operator-facing copy is Chromaticity-original.

## Consequences

### Positive
- Users switching from the reference app keep their creative investment.
- Single dry-run/confirm/rollback flow is consistent with `32-state-preset-playlist-migration-policy.md` rollback guarantees.
- Clean-room posture is preserved because transform logic is derived from data shapes, not reference source.

### Negative
- Reference artifact schemas evolve without contract guarantees; the importer must be maintained as reference releases ship new JSON shapes.
- Diagnostic quality directly affects perceived switching cost; a noisy or confusing importer is worse than no importer.
- Attack surface: importer parses third-party JSON. Must defend against malformed/over-deep/over-large payloads (parser hardening, per-file size caps).

## Alternatives Considered

- **No import at MVP**: rejected. Switching cost is the biggest commercial risk for a parity product.
- **Manual operator-authored migration guide only**: rejected. Unsigned, error-prone, does not scale to non-trivial preset libraries.
- **Live bidirectional sync**: rejected. Breaks clean-room posture and creates a permanent dependency on reference-app schemas.
- **Commercial third-party migration tool**: rejected. Splits accountability and introduces supply-chain risk for customer data.

## Evidence
- `EV-IMPORT-REFERENCE-SHOW`: validates dry-run, commit, rollback across every artifact class in Sec.Scope (settings / presets / Snap slots / meta-media channels / playlists / MIDI / OSC / hotkey). Each class has its own rows in the fixture set:
 - **Baseline fixtures**: empty root; populated root with one scene preset, one Snap slot save, one playlist, one MIDI mapping, one OSC alias, one imported hotkey rebind.
 - **Malformed fixtures**: corrupt preset JSON; MIN>MAX in a preset control value; unknown control name; outlier control type; hotkey rebind that collides with a V1-locked binding; OSC alias that would shadow a canonical route.
 - **Round-trip fixtures**: import -> operator save as new `.chromaShow` -> quit -> relaunch -> open `.chromaShow` -> observe all six classes behave as they did pre-quit (covers MIDI-CONF-20 learn persistence).
 - **Rollback drill**: import all classes, then invoke rollback; verify `32-state-preset-playlist-migration-policy.md` pre-migration snapshot restores byte-identical state.

## Traceability
- `03-product-requirements.md` FR-19
- `28-implementation-contract-pack.md` CP-01 (normalization precedence), CP-05 (preset channel scope)
- `32-state-preset-playlist-migration-policy.md` (rollback policy)
- `22-compatibility-versioning-policy.md` (`compat-syn-v1` profile posture)
- `26-evidence-artifact-registry.md` `EV-IMPORT-REFERENCE-SHOW`
- `20-requirements-traceability-matrix.md` FR-19 row
