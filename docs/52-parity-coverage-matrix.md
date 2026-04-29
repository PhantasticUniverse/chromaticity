# Parity Coverage Matrix

This matrix is a read-only claim-hygiene companion to `26-evidence-artifact-registry.md`. It maps observed reference-behavior families to the current Chromaticity coverage posture so reviews can see what is implemented, what is planned under an evidence row, what is post-MVP, and what is intentionally rejected. Status vocabulary is `implemented`, `planned:<EV-ID>`, `post-MVP`, `intentionally-rejected`, and `n/a`; if no valid evidence row exists yet, use `planned:gap:<description>` so the gap can be triaged without inventing an EV-ID. Every `planned:<EV-ID>` row must cite an EV-ID that exists in `26-evidence-artifact-registry.md`; run `bash scripts/evidence-tag-lint.sh` after edits to keep the EV namespace valid. This matrix is not a gate. It is a coverage index; gate authority stays with `26-evidence-artifact-registry.md` for the release cumulative gate and `11-roadmap-phases.md` for phase-exit gates.

## Scene schema and package

| Feature | Status | Reference | Notes |
|---|---|---|---|
| `scene.json` deserialization | implemented | `02-synesthesia-contracts.md` §Observed root keys; `05-scene-system.md` §Loader Modes | `engine/scene/src/loader.rs` parses in-memory JSON into the canonical schema; this does not imply package-level loading. |
| `.chroma` directory entry point | implemented | `05-scene-system.md` §Scene Package Model; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `engine/scene/src/loader.rs::load_chroma_dir` validates the `.chroma` directory contract and deserializes `scene.json`; shader, script, asset, and zip-package interpretation remain deferred per the module's slice-boundary comment. |
| `.chromaPack` archive | planned:EV-SCENE-IMPORT | `22-compatibility-versioning-policy.md` §Package Extensions and File-Type Ownership Policy; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `EV-SCENE-IMPORT` closed at Phase-1 EXIT (W12 PR #121, commit `2b4bffe`) for the canonical-model conversion contract; zip-transport (`.chromaPack`) remains release-cumulative scope and gates this row independently. Extension ownership is separately validated by `EV-EXTENSION-OWNERSHIP`. |
| `script.js` interpretation | planned:EV-SCRIPT-CONTRACT | `02-synesthesia-contracts.md` §3 Script Lifecycle; `28-implementation-contract-pack.md` §CP-07 | Script lifecycle, mutation APIs, and sandbox behavior are not implemented by the current scene loader. |
| `main.glsl` pairing | planned:EV-SCENE-IMPORT | `02-synesthesia-contracts.md` §Scene Package Contract; `05-scene-system.md` §Scene Package Model | `EV-SCENE-IMPORT` closed at Phase-1 EXIT (W12 PR #121, commit `2b4bffe`) for canonical-model coverage of the seven reactive-symbol fixtures; package-to-shader entry pairing is Phase-2 render-conformance scope (gated by `EV-RENDER-CONFORMANCE`) and remains deferred under this row. |
| Multipass lifecycle | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §2 Multipass; `34-golden-corpus-definition.md` §Corpus Size Targets | `PASSES` shape preservation exists, but pass ordering, feedback, sizing, and execution lifecycle remain render-conformance work. |
| Media/image loading | planned:EV-INTEGRATION-MATRIX | `02-synesthesia-contracts.md` §Observed root keys; `08-io-integrations.md` §Camera/Media Contract | `MEDIA` / `IMAGES` keys are preserved as extensions only; runtime loading and adapter behavior are future integration work. |
| Transition lifecycle | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §15 Transitions; `28-implementation-contract-pack.md` §CP-09 | Transition descriptors are preserved, but HARD / SMOOTH transition execution and diagnostics remain planned. |
| Pass execution | planned:EV-RENDER-CONFORMANCE | `05-scene-system.md` §Canonical Internal Scene Model; `adr/ADR-011-rendering-backend.md` | No renderer pass execution path is present in the current engine tree. |
| `.synScene` conversion input | planned:EV-SCENE-IMPORT | `22-compatibility-versioning-policy.md` §Reference-app extension handling; `38-scene-library-and-import-spec.md` §7 Packaging and UTI | `EV-SCENE-IMPORT` closed at Phase-1 EXIT (W12 PR #121, commit `2b4bffe`) for reactive-symbol canonical-model coverage; `.synScene -> .chroma` non-destructive conversion remains Phase-4b scope (gated by `EV-IMPORT-REFERENCE-SHOW`) and is unimplemented under this row. |
| `.synScene` OS association / extension claiming | intentionally-rejected | `22-compatibility-versioning-policy.md` §Reference Extension Handling | Chromaticity must not claim, register, or re-register the reference-owned extension. |
| Control metadata preservation (`UI_GROUP`, `DESCRIPTION`, `PARAMS`, `DEF_COLOR`, `LABELS`, `VALUES`, `IS_META`, `LABEL`) | implemented | `02-synesthesia-contracts.md` §Observed control keys; `28-implementation-contract-pack.md` §CP-01 | `engine/scene/src/schema.rs::ControlDecl` preserves all eight fields as explicit `serde(rename = ...)` bindings; unknown keys flatten non-lossily through the `extensions` bag. |

## Control taxonomy

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Base kinds: `slider`, `toggle`, `color`, `xy` | implemented | `28-implementation-contract-pack.md` §CP-01; `02-synesthesia-contracts.md` §Observed control types | The parser accepts these base kinds and the validator exercises CP-01 fallback paths for their current shapes. |
| Base kinds: `bang`, `dropdown`, `knob` | implemented | `02-synesthesia-contracts.md` §Observed control types; `08-io-integrations.md` §OSC Discovery and Announcement Routes | `engine/scene/src/schema.rs::ControlKind` accepts all three first tokens; these kinds require explicit `TYPE` and do not trigger CP-01 coercion. Runtime semantics (bang edge, dropdown discrete-option exposure, knob scalar normalization) and UI / OSC wiring are tracked under their own rows. |
| Slider variants: `smooth`, `traveler`, `counter`, `speed` | planned:EV-RENDER-CONFORMANCE | `02-synesthesia-contracts.md` §Traveler Controls; `adr/ADR-008-transition-and-traveler-semantics.md` | First-token parsing accepts some compound spellings, but variant semantics are not preserved or executed. |
| Missing-`TYPE` compat coercion | implemented | `28-implementation-contract-pack.md` §CP-01; `34-golden-corpus-definition.md` §Coverage Dimensions | `EV-COMPAT-FIXTURE-COVERAGE` closed at W11 PR #118 (commit `3a3b09b`) — T1 validator unit tests + T2 corpus coverage via `engine/scene/tests/cp01_compat.rs` against the 5 CP-01 fallback fixtures (slider/toggle/color/xy/reject) under `engine/scene/tests/fixtures/scenes/cp01-*.chroma/` plus 10 compat-vs-strict tests. |
| CP-01 `DEF_COLOR` fallback | implemented | `28-implementation-contract-pack.md` §CP-01 | `ControlDecl.def_color` is preserved at the schema and `engine/scene/src/validator.rs::coerce_missing_type` consumes it under CP-01 step 3; the `compat_fixture_coverage` T1 `cp01_step3_def_color_coerces_in_compat_rejects_in_strict` test pins the compat-vs-strict branch. |
| Dropdown labels / values exposure | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Discovery and Announcement Routes; `02-synesthesia-contracts.md` §Observed control keys | `LABELS` / `VALUES` are preserved structurally on `ControlDecl`; OSC discovery routes and MIDI mapping surface remain planned under this row. |

## Library and import

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Bundled scenes under `Resources/scenes/` | planned:EV-LIBRARY-BUNDLE-LAYOUT | `38-scene-library-and-import-spec.md` §2.1 | W11 PR #120 (commit `a18ba20`) implements the bundled-resources path with merge-on-fetch semantics: three demo `.chroma` packages are bundled at link time via `bundle.resources` in `app/shell/tauri.conf.json` and surfaced via the `bundled_scene_root` / `list_bundled_scene_packages` IPC commands; cards carry `SceneSource::Bundled` so the Library view renders a BUILT-IN chip. Full Phase-4b sub-directory layout (`welcome / library / examples / shadertoy-fixtures`) per `38-scene-library-and-import-spec.md` §2.1 and the bundle-walk assertion remain Phase-4b scope. |
| User-folder scanning | implemented | `38-scene-library-and-import-spec.md` §2 | `app/shell/src/commands/library.rs` (PR 112 — `list_scene_packages` + `default_scene_root`); `src/lib/views/Library.svelte` (this PR); `src/lib/stores/library.ts` (PR 112 store cache). |
| Thumbnails + fallback | planned:EV-LIBRARY-THUMBNAIL | `38-scene-library-and-import-spec.md` §3 | Thumbnail source chain and deterministic regeneration are specified but not implemented. |
| Search / filter / tag | implemented | `38-scene-library-and-import-spec.md` §2; §4 | `src/lib/views/Library.svelte` (this PR — substring search across title/credit/description/tags, 8 hardcoded category chips, and a small/medium/large size selector); §4 catalog promotion will replace the hardcoded chip list with a dynamic curated vocabulary loaded from the registry (TODO comment in source). |
| Drag-to-playlist | planned:EV-UX-LIBRARY-CONTRACT | `38-scene-library-and-import-spec.md` §2; `03-product-requirements.md` FR-14 | UX contract coverage exists as a planned gate; runtime playlist mutation is covered separately by `EV-PRESET-PLAYLIST`. |
| Preset launch from library | planned:EV-UX-LIBRARY-CONTRACT | `38-scene-library-and-import-spec.md` §2; `03-product-requirements.md` FR-6 / FR-14 | Launch-with-preset is a library UX and runtime state handshake; neither is complete. |
| Shadertoy conversion + diagnostics | planned:EV-IMPORT-SHADERTOY | `38-scene-library-and-import-spec.md` §5.1 | Conversion scaffolding, asset mapping, and actionable diagnostics remain planned. |
| V1 ISF rejection | planned:EV-IMPORT-ISF-REJECT | `38-scene-library-and-import-spec.md` §6 ISF Import | ISF is V1 reject-with-diagnostic, not native import. |
| Author workspace surface | planned:EV-UX-AUTHORING-CONTRACT | `37-authoring-environment-spec.md` §3 Authoring Environment Components | Authoring workspace UX and live authoring contracts remain pre-build UX gates / later implementation work. |
| Reference show / user-data import | planned:EV-IMPORT-REFERENCE-SHOW | `03-product-requirements.md` FR-24; `adr/ADR-015-reference-app-user-data-import.md` | Presets, mappings, playlists, favslots, and settings import are not implemented yet. |

## Runtime state

| Feature | Status | Reference | Notes |
|---|---|---|---|
| Controls (per-node control values) | implemented | `04-runtime-architecture.md` §Core Components; `28-implementation-contract-pack.md` §CP-01 | `engine/runtime/src/state.rs::State::control_value` reads the post-commit value; `app/shell/src/commands/control.rs` exposes the `set_control` / `get_control` Tauri round-trip (PR #94). |
| Presets (snapshot + recall) | planned:EV-PRESET-PLAYLIST | `03-product-requirements.md` FR-6; `07-control-state-presets.md` | `PresetRecall` exists as an event shape; durable preset state and snapshot semantics are not implemented. |
| Playlists (advance / rewind / bounds) | planned:EV-PRESET-PLAYLIST | `03-product-requirements.md` FR-6; `08-io-integrations.md` §OSC Conformance Test IDs | `PlaylistAdvance` exists as an event shape, but cursor bounds, wrap, rewind, and persistence are not implemented. |
| Snap / favslots (16-slot surface) | implemented | `04-runtime-architecture.md` §Core Components; `38-scene-library-and-import-spec.md` §8 Evidence | `engine/runtime/src/command_bus.rs` defines `Command::SnapRecall` / `SnapSave` / `SnapClear` and their `EventPayload` mirrors (lines 155-159, 185-193); `engine/runtime/src/state.rs::State` applies them via `apply_event` (lines 77-89) and exposes `active_snap_slot` / `snap_slot_saved` accessors (lines 114-122); `app/shell/src/commands/snap.rs` exposes the full Tauri round-trip (PR #98). The OSC discovery / announcement surface remains planned and is not part of the implemented rollup here. |
| MIDI mappings | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §MIDI Planning Contract | Command shapes can name MIDI as a mapping source, but learn, persistence, reconnect, and adapter behavior are planned. |
| OSC mappings | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Planning Contract; `adr/ADR-013-networking-defaults-and-security.md` | OSC defaults/routes have scaffolding; durable mapping state and full route side effects remain planned. |
| Show bundles | planned:EV-SHOW-BUNDLE-RECOVERY | `22-compatibility-versioning-policy.md` §Package Extensions; `03-product-requirements.md` FR-12 | `.chromaShow` is reserved and not implemented. |
| Rollback | planned:EV-STATE-MIGRATION | `03-product-requirements.md` FR-12 / FR-24; `32-state-preset-playlist-migration-policy.md` | Rollback snapshots and migration transforms remain planned. |
| Crash-resume | planned:EV-CRASH-RESUME | `03-product-requirements.md` FR-21; `31-supportability-privacy-ops-policy.md` §Support Bundle Policy | Resume previous show after crash is specified but not implemented. |
| Command/event deterministic replay baseline | implemented | `04-runtime-architecture.md` §Command Flow; `26-evidence-artifact-registry.md` `EV-DETERMINISM` | The current runtime has deterministic command-bus replay coverage; this is not full state reconciliation. |

## Integrations

| Feature | Status | Reference | Notes |
|---|---|---|---|
| OSC adapter (bind, poll, validate, rate-limit, allowlist) | planned:EV-INTEGRATION-MATRIX | `08-io-integrations.md` §OSC Planning Contract; `adr/ADR-013-networking-defaults-and-security.md` | `integrations/osc-adapter/src/adapter.rs` implements socket bind / unbind / poll and `validate_packet` hardening; the rate-limit precedence rule is pinned in `resolve_rate_limit`. Allowlist enforcement, rate-limit state machine, and hub-facing event channel remain planned. |
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
| Metric emission sites (per-metric) | implemented | `31-supportability-privacy-ops-policy.md` §Evidence Mapping | Emission-site wiring is complete across all five subsystems per `EV-OBSERVABILITY` Pass in `docs/26`: observability meta (`engine/observability/src/emit.rs`), runtime (`engine/runtime/src/command_bus.rs` per-`CommandKind` / per-`EventKind` / lifecycle transition counters), scene (`engine/scene/src/observability.rs` + `validator.rs` validation and lint counters), audio (`engine/audio/src/observability.rs::record_pull_outcome` from every `IngestBoundary::try_pull`), and OSC (`integrations/osc-adapter/src/adapter.rs::UdpOscAdapter` via `record_outcome` / `enter_state` / `record_bind_error`). |
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
| 14 | 46 | 1 | 4 | 0 |
