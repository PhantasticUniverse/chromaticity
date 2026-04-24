# Parity Coverage Matrix

This matrix is a read-only claim-hygiene companion to `26-evidence-artifact-registry.md`. It maps observed reference-behavior families to the current Chromaticity coverage posture so reviews can see what is implemented, what is planned under an evidence row, what is post-MVP, and what is intentionally rejected. Status vocabulary is `implemented`, `planned:<EV-ID>`, `post-MVP`, `intentionally-rejected`, and `n/a`; if no valid evidence row exists yet, use `planned:gap:<description>` so the gap can be triaged without inventing an EV-ID. Every `planned:<EV-ID>` row must cite an EV-ID that exists in `26-evidence-artifact-registry.md`; run `bash scripts/evidence-tag-lint.sh` after edits to keep the EV namespace valid. This matrix is not a gate. It is a coverage index; gate authority stays with `26-evidence-artifact-registry.md` for the release cumulative gate and `11-roadmap-phases.md` for phase-exit gates.

## Scene schema and package

| Feature | Status | Reference | Notes |
|---|---|---|---|
| `scene.json` deserialization | implemented | `02-synesthesia-contracts.md` §Observed root keys; `05-scene-system.md` §Loader Modes | `engine/scene/src/loader.rs` parses in-memory JSON into the canonical schema; this does not imply package-level loading. |
| `.chroma` directory entry point | planned:EV-SCENE-IMPORT | `05-scene-system.md` §Scene Package Model; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | Native package semantics are specified, but the loader still documents on-disk helpers as deferred in the current tree. |
| `.chromaPack` archive | planned:EV-SCENE-IMPORT | `22-compatibility-versioning-policy.md` §Package Extensions and File-Type Ownership Policy; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | Zip transport remains planned; extension ownership is separately validated by `EV-EXTENSION-OWNERSHIP`. |
| `script.js` interpretation | planned:EV-SCRIPT-CONTRACT | `02-synesthesia-contracts.md` §3 Script Lifecycle; `28-implementation-contract-pack.md` §CP-07 | Script lifecycle, mutation APIs, and sandbox behavior are not implemented by the current scene loader. |
| `main.glsl` pairing | planned:EV-SCENE-IMPORT | `02-synesthesia-contracts.md` §Scene Package Contract; `05-scene-system.md` §Scene Package Model | Package-to-shader entry pairing is still deferred; current schema parses `scene.json` only. |
| Multipass lifecycle | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §2 Multipass; `34-golden-corpus-definition.md` §Corpus Size Targets | `PASSES` shape preservation exists, but pass ordering, feedback, sizing, and execution lifecycle remain render-conformance work. |
| Media/image loading | planned:EV-INTEGRATION-MATRIX | `02-synesthesia-contracts.md` §Observed root keys; `08-io-integrations.md` §Camera/Media Contract | `MEDIA` / `IMAGES` keys are preserved as extensions only; runtime loading and adapter behavior are future integration work. |
| Transition lifecycle | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §15 Transitions; `28-implementation-contract-pack.md` §CP-09 | Transition descriptors are preserved, but HARD / SMOOTH transition execution and diagnostics remain planned. |
| Pass execution | planned:EV-RENDER-CONFORMANCE | `05-scene-system.md` §Canonical Internal Scene Model; `adr/ADR-011-rendering-backend.md` | No renderer pass execution path is present in the current engine tree. |
| `.synScene` conversion input | planned:EV-SCENE-IMPORT | `22-compatibility-versioning-policy.md` §Reference-app extension handling; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `.synScene` is accepted only as non-destructive import input and converted to `.chroma`; no conversion implementation is present yet. |
| `.synScene` OS association / extension claiming | intentionally-rejected | `22-compatibility-versioning-policy.md` §Reference Extension Handling | Chromaticity must not claim, register, or re-register the reference-owned extension. |
| Control metadata preservation (`UI_GROUP`, `DESCRIPTION`, `PARAMS`, `DEF_COLOR`, `LABELS`, `VALUES`, `IS_META`, `LABEL`) | planned:EV-SCENE-IMPORT | `02-synesthesia-contracts.md` §Observed control keys; `28-implementation-contract-pack.md` §CP-01 | The current `ControlDecl` still drops these fields rather than preserving them structurally. |

## Control taxonomy

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Base kinds: `slider`, `toggle`, `color`, `xy` | implemented | `28-implementation-contract-pack.md` §CP-01; `02-synesthesia-contracts.md` §Observed control types | The parser accepts these base kinds and the validator exercises CP-01 fallback paths for their current shapes. |
| Base kinds: `bang`, `dropdown`, `knob` | planned:EV-SCENE-IMPORT | `02-synesthesia-contracts.md` §Observed control types; `08-io-integrations.md` §OSC Discovery and Announcement Routes | The current `ControlKind` rejects these first tokens; runtime semantics and UI / OSC exposure are not present. |
| Slider variants: `smooth`, `traveler`, `counter`, `speed` | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §Traveler Controls; `adr/ADR-008-transition-and-traveler-semantics.md` | First-token parsing accepts some compound spellings, but variant semantics are not preserved or executed. |
| Missing-`TYPE` compat coercion | implemented | `28-implementation-contract-pack.md` §CP-01; `34-golden-corpus-definition.md` §Coverage Dimensions | T1 validator tests cover the deterministic fallback order for existing schema shapes; T2 corpus coverage remains tracked by `EV-COMPAT-FIXTURE-COVERAGE`. |
| CP-01 `DEF_COLOR` fallback | planned:EV-COMPAT-FIXTURE-COVERAGE | `28-implementation-contract-pack.md` §CP-01 | The current schema does not preserve `DEF_COLOR`, so the documented color fallback branch is not executable yet. |
| Dropdown labels / values exposure | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Discovery and Announcement Routes; `02-synesthesia-contracts.md` §Observed control keys | `LABELS` / `VALUES` are not yet preserved, so dropdown discovery and MIDI mapping cannot be fully conformed. |

## Library and import

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Bundled scenes under `Resources/scenes/` | planned:EV-LIBRARY-BUNDLE-LAYOUT | `38-scene-library-and-import-spec.md` §2.1 | Bundle layout is specified; no bundled scene corpus or bundle-walk assertion is implemented in this tree. |
| User-folder scanning | planned:EV-LIBRARY-DISCOVERY | `38-scene-library-and-import-spec.md` §2 | Operator-owned scene folder indexing remains planned. |
| Thumbnails + fallback | planned:EV-LIBRARY-THUMBNAIL | `38-scene-library-and-import-spec.md` §3 | Thumbnail source chain and deterministic regeneration are specified but not implemented. |
| Search / filter / tag | planned:EV-LIBRARY-DISCOVERY | `38-scene-library-and-import-spec.md` §2 | Full-text search, tags, filters, sort, and index behavior remain planned. |
| Drag-to-playlist | planned:EV-UX-LIBRARY-CONTRACT | `38-scene-library-and-import-spec.md` §2; `03-product-requirements.md` FR-14 | UX contract coverage exists as a planned gate; runtime playlist mutation is covered separately by `EV-PRESET-PLAYLIST`. |
| Preset launch from library | planned:EV-UX-LIBRARY-CONTRACT | `38-scene-library-and-import-spec.md` §2; `03-product-requirements.md` FR-6 / FR-14 | Launch-with-preset is a library UX and runtime state handshake; neither is complete. |
| Shadertoy conversion + diagnostics | planned:EV-IMPORT-SHADERTOY | `38-scene-library-and-import-spec.md` §5.1 | Conversion scaffolding, asset mapping, and actionable diagnostics remain planned. |
| V1 ISF rejection | planned:EV-IMPORT-ISF-REJECT | `38-scene-library-and-import-spec.md` §6 ISF Import | ISF is V1 reject-with-diagnostic, not native import. |
| Author workspace surface | planned:EV-UX-AUTHORING-CONTRACT | `37-authoring-environment-spec.md` §3 Authoring Environment Components | Authoring workspace UX and live authoring contracts remain pre-build UX gates / later implementation work. |
| Reference show / user-data import | planned:EV-IMPORT-REFERENCE-SHOW | `03-product-requirements.md` FR-24; `adr/ADR-015-reference-app-user-data-import.md` | Presets, mappings, playlists, favslots, and settings import are not implemented yet. |

## Runtime state

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Controls (per-node control values) | planned:EV-STATE-REPLAY | `04-runtime-architecture.md` §Core Components; `28-implementation-contract-pack.md` §CP-01 | Command/event envelopes exist, but the current `State` control store is still a placeholder. |
| Presets (snapshot + recall) | planned:EV-PRESET-PLAYLIST | `03-product-requirements.md` FR-6; `07-control-state-presets.md` | `PresetRecall` exists as an event shape; durable preset state and snapshot semantics are not implemented. |
| Playlists (advance / rewind / bounds) | planned:EV-PRESET-PLAYLIST | `03-product-requirements.md` FR-6; `08-io-integrations.md` §OSC Conformance Test IDs | `PlaylistAdvance` exists as an event shape, but cursor bounds, wrap, rewind, and persistence are not implemented. |
| Snap / favslots (16-slot surface) | planned:EV-PRESET-PLAYLIST | `38-scene-library-and-import-spec.md` §8 Evidence; `08-io-integrations.md` §OSC Discovery and Announcement Routes | Snap slot count and OSC surface are specified; runtime storage / save / clear / recall are not implemented. |
| MIDI mappings | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §MIDI Planning Contract | Command shapes can name MIDI as a mapping source, but learn, persistence, reconnect, and adapter behavior are planned. |
| OSC mappings | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Planning Contract; `adr/ADR-013-networking-defaults-and-security.md` | OSC defaults/routes have scaffolding; durable mapping state and full route side effects remain planned. |
| Show bundles | planned:EV-SHOW-BUNDLE-RECOVERY | `22-compatibility-versioning-policy.md` §Package Extensions; `03-product-requirements.md` FR-12 | `.chromaShow` is reserved and not implemented. |
| Rollback | planned:EV-STATE-MIGRATION | `03-product-requirements.md` FR-12 / FR-24; `32-state-preset-playlist-migration-policy.md` | Rollback snapshots and migration transforms remain planned. |
| Crash-resume | planned:EV-CRASH-RESUME | `03-product-requirements.md` FR-21; `31-supportability-privacy-ops-policy.md` §Support Bundle Policy | Resume previous show after crash is specified but not implemented. |
| Command/event deterministic replay baseline | implemented | `04-runtime-architecture.md` §Command Flow; `26-evidence-artifact-registry.md` `EV-DETERMINISM` | The current runtime has deterministic command-bus replay coverage; this is not full state reconciliation. |

## Integrations

| Feature | Status | Reference | Notes |
|---|---|---|---|
| OSC adapter (bind, poll, validate, rate-limit, allowlist) | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Planning Contract; `adr/ADR-013-networking-defaults-and-security.md` | Route parsing and hardening defaults exist, but full socket bind / poll / command side effects are not complete. |
| MIDI adapter | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §MIDI Planning Contract | MIDI learn, device reconnect, outbound mirror, and persistence remain planned. |
| Media / camera | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §Camera/Media Contract; `30-mvp-support-matrix.md` §Media Path Classes | Media files and camera adapters are planned integration surfaces. |
| System audio ingest | planned:EV-AUDIO-ROUTING-MATRIX | `06-audio-engine.md` §Pipeline; `30-mvp-support-matrix.md` §Audio Device Classes | Audio frame scaffolding exists, but real system-audio / loopback routing is not complete. |
| Audio uniform trace parity | planned:EV-AUDIO-TRACE | `06-audio-engine.md` §7 Numeric Audio Contract; `10-performance-latency-budgets.md` | Golden audio fixtures and trace conformance remain planned. |
| Syphon (macOS) | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §External Texture I/O Contract; `adr/ADR-011-rendering-backend.md` §Syphon / Spout interop | Platform adapter remains planned. |
| Spout (Windows) | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §External Texture I/O Contract; `adr/ADR-011-rendering-backend.md` §Syphon / Spout interop | Platform adapter remains planned. |
| Output routing | planned:EV-PLATFORM-CONFORMANCE | `08-io-integrations.md` §Display/Output Surface Workflow; `30-mvp-support-matrix.md` §Output Surface Classes | Display / external texture / adapter class conformance remains planned. |
| NDI | post-MVP | `30-mvp-support-matrix.md` §Media Path Classes; `adr/ADR-011-rendering-backend.md` §Texture interop | NDI is design-only at MVP freeze and not an MVP gate. |
| Open network OSC bind by default | intentionally-rejected | `adr/ADR-013-networking-defaults-and-security.md` §Rejected Alternatives | Loopback-only is the default; `0.0.0.0` requires explicit operator opt-in. |

## Observability

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Metric schema conformance | implemented | `31-supportability-privacy-ops-policy.md` §Scheme and Metric Catalog; `26-evidence-artifact-registry.md` `EV-OBSERVABILITY` | `engine/observability` mirrors the 14-metric catalog with conformance tests. |
| Metric emission sites (per-metric) | planned:EV-OBSERVABILITY | `31-supportability-privacy-ops-policy.md` §Evidence Mapping | The current crate provides an in-process emitter stub; runtime subsystem wiring remains planned. |
| Observability inventory (ADR-016 transparency artifact) | planned:EV-OBSERVABILITY-INVENTORY | `adr/ADR-016-observability-and-telemetry-posture.md` §Transparency artifact | The shipped `OBSERVABILITY-INVENTORY.md` artifact is not present yet. |
| Support bundle | planned:EV-SUPPORT-OPS-READINESS | `31-supportability-privacy-ops-policy.md` §Support Bundle Policy | Support bundle export and inclusion controls remain planned. |
| Loopback health metrics endpoint | planned:EV-HEALTH-METRICS-LOOPBACK-ONLY | `adr/ADR-016-observability-and-telemetry-posture.md` §Health metrics / telemetry | Transport is Phase 4d; Phase 1 has no network I/O. |
| Crash payload redaction | planned:EV-CRASH-PAYLOAD-REDACTION | `adr/ADR-016-observability-and-telemetry-posture.md` §Evidence | Crash-report payload inspection remains planned. |
| Analytics zero-default network posture | planned:EV-ANALYTICS-ZERO-DEFAULT | `adr/ADR-016-observability-and-telemetry-posture.md` §Decision | Fresh-install network observation remains planned. |
| Outbound HTTPS telemetry by default | intentionally-rejected | `adr/ADR-016-observability-and-telemetry-posture.md` §Decision | Health metrics must be opt-in loopback-only; no default outbound telemetry. |

## Release-eng

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Signing (macOS Developer ID, Windows Authenticode) | planned:EV-RELEASE-PIPELINE | `adr/ADR-014-signing-notarization-auto-update.md` §Code signing | Design is accepted; release pipeline implementation remains Phase 5. |
| Notarization | planned:EV-RELEASE-PIPELINE | `adr/ADR-014-signing-notarization-auto-update.md` §Code signing | macOS notarization / stapling remains planned. |
| Updater (Sparkle-EdDSA) | planned:EV-RELEASE-PIPELINE | `adr/ADR-014-signing-notarization-auto-update.md` §Auto-update | Sparkle 2 / EdDSA appcast signing is specified but not implemented. |
| SBOM (CycloneDX 1.5) | planned:EV-SBOM-COVERAGE | `43-development-standards.md` §10.4; `.github/workflows/sbom.yml` | SBOM workflow scaffolding exists; release artifact coverage remains planned. |
| Release attestation | planned:EV-RELEASE-ATTESTATION | `43-development-standards.md` §11.2 | Attested release artifacts remain planned. |
| Bundle hygiene / signing integrity | planned:EV-SIGNING-INTEGRITY | `adr/ADR-014-signing-notarization-auto-update.md` §Bundle hygiene deny-list | No-legacy-key and shipped-bundle denial checks remain planned. |
| Sparkle DSA fallback / legacy DSA key shipping | intentionally-rejected | `adr/ADR-014-signing-notarization-auto-update.md` §Auto-update; §Rejected Alternatives | Chromaticity ships EdDSA-only updater verification and never ships a legacy DSA key path. |
| ATS no third-party exceptions | planned:EV-ATS-NO-THIRD-PARTY-EXCEPTIONS | `adr/ADR-016-observability-and-telemetry-posture.md` §Evidence | Bundle-level ATS inspection remains planned. |
| Platform conformance for release support classes | planned:EV-PLATFORM-CONFORMANCE | `30-mvp-support-matrix.md` §Evidence Binding | Cross-platform release support matrix validation remains planned. |

## Summary tally

These counts are informational only; this matrix is not a gate.

| implemented | planned:* | post-MVP | intentionally-rejected | n/a |
|---:|---:|---:|---:|---:|
| 5 | 55 | 1 | 4 | 0 |
