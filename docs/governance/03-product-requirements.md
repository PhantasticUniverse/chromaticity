# Chromaticity Product Requirements

## 1) Goals
- Deliver a live visual runtime that matches or exceeds the incumbent reference-app's (Synesthesia's) capability envelope.
- Preserve high-value scene authoring and performance workflows under the `compat-syn-v1` compatibility profile.
- Improve determinism, observability, and fault isolation.
- Ship a commercially safe product under clean-room development controls per `29-cleanroom-legal-policy-baseline.md`.

## 2) Parity Profiles
- **Parity-Core**: runtime, schema, scene/script lifecycle, controls, presets/playlists, audio uniforms, and diagnostics.
- **Parity-Advanced**: full OSC route surface, MIDI parity, reconnect behavior, camera/media adapters, and external texture I/O.
- **Differentiation**: capability and reliability improvements that exceed reference behavior without breaking compatibility profiles.

## 3) Functional Requirements

### FR-1 Scene Package Support
- System shall load scene packages containing `scene.json`, `main.glsl`, and optional `script.js`.
- System shall support bundled scene images and declared pass targets.
- System shall provide compatibility mode for legacy keys and outlier control types.

### FR-2 Scene Schema and Validation
- System shall validate scene schema in strict mode with actionable diagnostics.
- System shall accept compatibility mode inputs and normalize to canonical internal representation.
- System shall generate non-fatal warnings for deprecated and legacy fields.

### FR-3 Rendering Runtime
- System shall execute multipass rendering with deterministic pass ordering.
- System shall expose previous-frame feedback texture contract (`final pass` equivalent).
- System shall support configurable pass resolution, precision, filter, and wrap behavior.

### FR-4 Script Runtime
- System shall support scene script lifecycle (`setup`, `update(dt)`, `transition()` lifecycle hook).
- System shall expose a controlled host API for uniform publishing and control manipulation.
- System shall support event registration primitives equivalent to the reference-app behavior captured in `02-synesthesia-contracts.md` Sec.3.
- Script faults shall not crash the entire runtime process.

### FR-5 Audio Reactivity
- System shall produce a compatibility set of reactive audio uniforms (levels, hits, time, presence, beat/BPM families).
- System shall provide texture-like data streams for FFT, waveform, and trails.
- Uniform update cadence shall align with render loop requirements.

### FR-6 Controls and State
- System shall support control types equivalent to the reference-app contracts observed in `02-synesthesia-contracts.md`, including compatibility outliers.
- System shall persist and restore control values with scene-aware defaults.
- System shall support control grouping, randomization, defaulting, and lock semantics.

### FR-7 Presets and Playlists
- System shall support scene presets, startup preset selection, and playlist progression semantics.
- System shall allow scene launch with explicit preset targeting.

### FR-8 External Integrations
- System shall expose OSC and MIDI control routes through stable adapters.
- System shall support camera and media ingestion.
- System shall support **operator-managed Media Folders**: operator adds one or more filesystem folders, runtime scans for supported image/video formats, and exposes discovered items through the media adapter and control surface. Supported formats at MVP: `.png`, `.jpg`, `.jpeg` for images; `.mp4`, `.mov` for video. Unsupported files are listed in a diagnostic without blocking scan completion.
- System shall support external texture pipeline adapters (Syphon on macOS, Spout on Windows) for input and output, with operator-renamable sender and selectable resolution mode.
- **Post-MVP**: System shall support NDI input and output, with single active input/output and source persistence across reconnect. NDI is deferred to post-MVP under `30-mvp-support-matrix.md`; the adapter interface is stubbed at the `ADR-005` boundary so post-MVP activation does not require architecture changes.
- System shall support display/output surface workflows including fullscreen toggle, multi-display placement, and constant vs scene-resolution output modes.
- System shall support system-audio ingestion and channel-routing selection for supported platforms.
- Entitlement gating at MVP is binary (watermark-on vs watermark-off); any valid signed token removes the watermark. Post-MVP tiered capabilities shall be declared explicitly in `30-mvp-support-matrix.md` and enforced via `adr/ADR-009-activation-and-entitlements.md`.

### FR-9 Developer Experience
- System shall surface compile and runtime diagnostics for scene JSON, GLSL, and scripts.
- Diagnostics shall identify subsystem, scene, and likely remediation path.

### FR-10 Clean-Room and Commercial Compliance
- System shall not ship copied reference code, assets, or documentation text from Synesthesia.
- System shall keep provenance records for shipped code/assets and derived fixtures.
- System shall enforce behavior/spec extraction only from reference materials.
- Kickoff requires legal policy baseline artifacts (clean-room SOP, provenance template, license policy baseline, brand-review checklist).
- Release requires legal audit artifacts populated from shipped scope evidence.

### FR-11 Audio Routing and System Audio
- System shall provide per-device input selection and per-channel routing matrix for supported audio interfaces.
- System shall provide first-class system-audio capture workflows on supported platforms with diagnostics.
- Routing and capture failures shall degrade gracefully with actionable operator diagnostics.

### FR-12 Show-State Safety
- System shall support signed show bundle export/import for reproducible handoff.
- System shall validate show bundle schema before import and reject corrupted payloads without mutating active show state.
- System shall provide rollback snapshots for presets/playlists/mappings with deterministic restore semantics.

### FR-13 Scene Authoring Environment
- System shall provide an in-app scene authoring environment with multi-file editing (`scene.json`, `main.glsl`, `script.js`, `glsl/*.glsl`, `images/*`).
- System shall hot-reload authored changes per the file-class matrix in `37-authoring-environment-spec.md`, preserving state where that spec requires it.
- System shall surface compile and runtime diagnostics inline and in a dedicated console panel with subsystem, scene, and severity tags.
- System shall provide a New Scene scaffolder that outputs a valid minimal `.chroma` package.
- System shall export Chromaticity scenes as `.chroma` directory packages or `.chromaPack` zip archives, and shall import `.chroma`, `.chromaPack`, reference-app `.synScene`, and reference-app `.synScenePack` inputs. Reference-app inputs are converted one-way to `.chroma` without modifying the source.
- System shall not claim, register, or declare a UTI or file-association for `.synScene` or any other reference-owned extension per `22-compatibility-versioning-policy.md` Sec.Package Extensions.
- Authoring-environment performance shall meet the hot-reload latency budgets in `37-authoring-environment-spec.md`.

### FR-14 Scene Import Adapters
- System shall accept Shadertoy scenes through a conversion importer that produces a valid Chromaticity scene package with mapped uniforms per `38-scene-library-and-import-spec.md`.
- System shall reject ISF scenes in V1 with an actionable diagnostic and an "import as blank with raw body" escape hatch; ISF support is scheduled for V2.
- Import flows shall emit coercion diagnostics for every pattern that cannot be auto-mapped and leave author-visible TODO markers in the output.

### FR-15 Scene Library
- System shall provide a library browser over bundled scenes and operator-owned scene folders with grid and list views, filter, full-text search across `TITLE`/`CREDIT`/`DESCRIPTION`/`TAGS`/`CATEGORIES`, and sort.
- Library shall expose thumbnails per `38-scene-library-and-import-spec.md` Sec.3 including a deterministic fallback chain.
- Library shall support drag-to-playlist, launch-with-preset, and keyboard navigation.

### FR-16 Embedded Documentation
- System shall ship an offline documentation site inside the app bundle and render it in an in-app help view with no network dependency.
- Documentation contents shall at minimum cover Scene JSON reference, standard uniforms, audio uniforms, meta controls, GLSL helpers, script host API, external libraries, control hotkeys, OSC routes, MIDI mapping, FAQ, and tutorials per `adr/ADR-010-embedded-docs-generation.md`.
- Documentation version shall match the reactive symbol catalog version; release gate `EV-DOCS-VERSION-SYNC` enforces this.

### FR-17 Operator Hotkey Map
- System shall bind the following operator hotkeys when a control is focused: `D` default, `R` random, `P` preset, `L` lock (reference parity).
- System shall ship the normative operator hotkey table in `42-operator-hotkey-map.md`. That table covers scene and playlist transport, Snap slots, display/output, rendering, meta controls, audio, navigation panels, diagnostics, and show-state management.
- Hotkeys shall be redefinable via a settings panel in V2; V1 ships the table as read-only defaults.

### FR-18 Activation and Entitlement
- System shall read and verify a signed `entitlement-v2` token at startup per `adr/ADR-009-activation-and-entitlements.md`.
- Runtime model is a **capability bitmask** keyed by dotted-namespace capability strings (`render.watermark.off`, `io.osc.input`, `io.osc.output`, `io.syphon.input`, `io.syphon.output`, `io.spout.input`, `io.spout.output`, `io.ndi.input`, `io.ndi.output`, `io.camera`, `authoring.ide`, `authoring.import`, `authoring.edit`, `authoring.builtin-scenes`, `render.custom-resolution`, `meta.reactivity`, `meta.audio-speed`). Editions are marketing-only presets over this bitmask and are not a runtime concept.
- MVP enforcement scope: binary watermark (no valid token = watermark on; any valid token = watermark off). Per-capability adapter gating is wired at the adapter boundary but not activated at MVP freeze; activation post-MVP is a data change, not a runtime rebuild.
- System shall operate offline against a valid (unexpired) entitlement token and shall never require network reachability to render frames.
- The following features are **unconditionally free** and ship enabled regardless of token presence: pause (transport), transitions-off (hard-cut scene switches), playlist navigation (next/previous/select), preset recall, Snap slots (the Chromaticity UX name for per-scene quick-recall preset slots; retained as `favslot` only in the `compat-syn-v1` OSC route surface), scene export. Gating any of these is a banned behavior per the ADR.
- MVP token distribution does not require a commerce backend: tokens are issued statically (closed-beta list, owner-signed tokens for internal use). Trial/revocation/machine-binding/session claims are defined in the token schema for forward compatibility; commerce-backed issuance and trial-code workflows are post-MVP.

### FR-19 Reference-App User Data Import
- System shall import a reference-app user-data folder and produce a dry-run report covering scene presets, meta/media preset channels, Snap slots (imported from the reference's `favslot` payloads), playlists, MIDI mappings, OSC aliases, and settings per `adr/ADR-015-reference-app-user-data-import.md`.
- Import shall be non-destructive to the source reference folder and shall require operator confirmation before commit.
- Import shall emit structured coercion/reject diagnostics and write a pre-migration rollback snapshot per `32-state-preset-playlist-migration-policy.md`.
- Imported artifacts shall be tagged with compatibility profile `compat-syn-v1`.

### FR-20 Network Surface Hardening
- System shall default OSC listeners to loopback (127.0.0.1) until the operator opts into a specific network interface per `adr/ADR-013-networking-defaults-and-security.md`.
- System shall enforce per-adapter inbound rate limits (OSC default 100 msg/s operator-overridable to 1000 msg/s cap).
- System shall support source-IP allowlist per mapping; unbound senders are rejected with diagnostic.
- System shall scope NDI discovery to the local subnet by default, with allowlist/denylist support.
- Diagnostics shall surface firewall/port issues with actionable remediation copy.

### FR-21 Crash Recovery and Show Resume
- System shall serialize live show state (active scene, active preset, playlist position, meta-control values, media bindings) at a cadence bounded by `max_show_state_age <= 5 s` during a live session.
- After a process crash, on next launch the system shall offer the operator a "Resume previous show" action that restores the serialized show state atomically.
- Resume shall never silently mutate persistent state; a rollback snapshot is written before the resume restore.
- Crash-report payloads shall continue to comply with `31-supportability-privacy-ops-policy.md` (no scene source, no media content).

### FR-22 Extended MIDI Surface
- System shall accept MIDI Pitch Bend messages and map to `slider` / `xy` controls with 14-bit resolution.
- System shall accept MIDI Program Change messages and map to preset apply per operator mapping table.
- System shall accept MIDI Clock (`0xF8`) and Song Position Pointer messages for external BPM sync; when external sync is active, `chroma_BPM` follows the MIDI Clock rate per `adr/ADR-012-script-sandbox-engine.md` determinism rules and `06-audio-engine.md` Sec.7.9 fusion policy.
- System shall accept 14-bit CC (MSB 0-31 + LSB 32-63 pair), NRPN, and RPN at 14-bit resolution per `08-io-integrations.md` Sec.Additional MIDI Surface Decisions.
- System shall emit MPE-configuration messages as a `mpe-deferred` diagnostic; MPE is V2 scope.
- System shall parse MIDI Time Code (MTC) quarter-frame and full-frame SysEx and publish a `chroma_Timecode` uniform family for SMPTE-slaved playback. SMPTE LTC (audio-carried) is V2 scope with a `ltc-not-ingested` diagnostic at MVP.
- System shall persist MIDI-learn bindings across show save, app quit, and relaunch per MIDI-CONF-20.
- SysEx (beyond MTC full-frame) is explicitly deferred to V2. Incoming SysEx is logged but not routed.

## 4) Non-Functional Requirements

### NFR-1 Reliability
- Runtime shall isolate scene-level failures from process-level failures.
- Recovery strategy shall include scene restart and fallback behavior.

### NFR-2 Determinism
- Given identical inputs and seed/state, frame graph progression shall be reproducible within numeric tolerance.
- Tolerance thresholds are normatively defined in the Determinism and Drift Acceptance Table in `10-performance-latency-budgets.md`.

### NFR-3 Performance
- Runtime shall meet defined latency and frame-time budgets in `10-performance-latency-budgets.md`.
- Performance degradation shall be graceful with clear quality controls.

### NFR-4 Observability
- System shall expose structured metrics and logs for frame timing, pipeline stalls, and integration errors.
- Profiling hooks shall be available in development builds.

### NFR-5 Portability
- System shall isolate platform and protocol dependencies behind adapter boundaries.

### NFR-6 Security and Isolation
- Script runtime shall use a constrained execution environment with explicit host API allowlist.

### NFR-7 Legal and Licensing Hygiene
- All third-party dependencies and runtime assets shall have auditable license metadata.
- Release candidates shall pass legal compliance gate checks before launch sign-off.
- Launch-facing naming and copy shall pass trademark/brand-distinction review.

### NFR-8 Supportability and Privacy
- System shall provide support bundle export with explicit data inclusion controls.
- Telemetry and logs shall enforce retention and redaction policy controls.
- Crash reporting and operator runbook requirements shall be policy-bound before release.

### NFR-8a Observability Posture (outbound-traffic contract)
- On a fresh install with untouched settings, the runtime shall emit outbound traffic only to the Sparkle update appcast URL. All analytics, crash-reporting, and telemetry transmission are opt-in per `adr/ADR-016-observability-and-telemetry-posture.md`.
- No third-party `NSExceptionDomains` entries shall appear in the shipped `Info.plist`.
- Crash reporting, when opt-in-enabled, shall transmit to a Chromaticity-owned self-hosted Sentry instance, with payload redaction per `adr/ADR-016-observability-and-telemetry-posture.md`.
- Health metrics, when opt-in-enabled, shall expose only on a loopback socket.
- Every release shall ship `OBSERVABILITY-INVENTORY.md` enumerating all possible outbound destinations keyed to the toggle that enables each.

### NFR-9 Network Surface
- System shall honor network defaults, rate limits, and payload hardening locked in `adr/ADR-013-networking-defaults-and-security.md`.
- All network exposure beyond loopback shall be explicit operator opt-in with diagnostic affordance.

### NFR-10 Authoring Responsiveness
- Hover-docs lookup from the in-app editor: p95 <= 80 ms keystroke-to-render (`EV-AUTHORING-UX-LATENCY`).
- Symbol-search panel: p95 <= 150 ms query-to-first-result.
- Embedded-docs page open inside help view: p95 <= 300 ms, p99 <= 450 ms (`EV-DOCS-UX-LATENCY`).
- Embedded-docs search inside help view: p95 <= 150 ms, p99 <= 220 ms (`EV-DOCS-UX-LATENCY`).

### NFR-11 Accessibility Floor
- Non-scene-surface UI (control panels, library, settings, authoring environment chrome, embedded docs) shall meet **WCAG 2.2 AA** for perceivable contrast, keyboard operability, focus indicators, target-size minimums, and screen-reader labeling. Assistive-technology support: VoiceOver (macOS) and NVDA (Windows) at minimum.
- Every action reachable via mouse shall also be reachable via keyboard (with the same semantics), including scene launch, Snap recall, and settings navigation. The command palette (`Cmd+K` / `Ctrl+K`) is the unified keyboard entry point to any named action.
- Focus indicators shall be visible on dark and light backgrounds at >= 3:1 contrast against the element being focused.
- **Scene render output (the visual canvas) is explicitly excluded** from WCAG criteria - the product's core value is arbitrary artistic visuals, which cannot be contrast-bounded or motion-bounded without destroying the product. This rider is documented in the `a11y-policy.md` page referenced by `adr/ADR-010-embedded-docs-generation.md`.
- **Reduced-motion respect**: when the OS-level "reduce motion" preference is active, non-scene UI chrome shall disable parallax, bounce, and fade animations; transitions between UI states shall be instant. Scene-surface motion is unaffected (per the exclusion above).
- **High-contrast mode** compatibility: the app shall render correctly when macOS "Increase contrast" or Windows "High contrast" is active; no custom paint path breaks the accessibility rendering.
- **Target size**: all interactive targets in the control panel, library, and settings shall be >= 24x24 CSS px equivalent (WCAG 2.2 SC 2.5.8).
- Accessibility regressions are release-blocking per `EV-ACCESSIBILITY-AA`.

### NFR-12 Color Management and Output Color Space
- V1 output color space: **sRGB** (linearized-to-gamma on backbuffer write). All scene rendering, meta-control pipeline math, and preview windows use sRGB.
- V1 **does not** ship HDR output, Rec.2020, Display P3, or ACES pipelines. These are explicitly V2+ per `adr/ADR-011-rendering-backend.md` Sec.Color management.
- V1 **respects the display profile** on macOS (via ColorSync) and Windows (via the display's ICC profile); output pixels are converted from sRGB to the display's profile by the OS compositor. Chromaticity does not perform its own profile conversion.
- V1 reports the detected output color space in the diagnostics HUD and in support bundles. If the operator connects a non-sRGB-calibrated display, the HUD surfaces a "non-sRGB display detected" informational diagnostic; this is not an error.
- V1 shipped bundled scenes and fixture thumbnails are authored against the sRGB target. Reference-app scene imports are treated as sRGB-source per `22-compatibility-versioning-policy.md`.
- V2+ HDR / wide-gamut path is scoped in `adr/ADR-011-rendering-backend.md` Sec.Color management and requires a new evidence row `EV-COLOR-MANAGEMENT-HDR`.
- V1 color-management conformance is validated by `EV-COLOR-MANAGEMENT-SRGB`.
- Live-reload latency budgets are owned by `37-authoring-environment-spec.md` Sec.3.3.

## 5) Compatibility Requirements
- Compatibility mode shall support the reference-app schema variants and legacy keys observed in `02-synesthesia-contracts.md`.
- Compatibility claims require test artifacts in golden-scene regression suite.
- Any intentional incompatibility must be documented with migration guidance.

## 6) Live-Performance Requirements
- Startup and scene-switch behavior shall be predictable and low-jitter.
- Input protocols (MIDI and OSC) shall remain responsive under high visual complexity.
- Fault handling shall prioritize continuity of output.

### Integration Acceptance Requirements
- Supported device/media/controller classes and failure-mode expectations are defined in `08-io-integrations.md` and `30-mvp-support-matrix.md`.
- Reconnect behavior shall preserve mappings for recoverable disconnects and emit actionable diagnostics when recovery exceeds threshold.
- Invalid integration payloads shall be rejected without mutating durable state.

## 7) Profile and Evidence Mapping

| Requirement | Profile | Primary Evidence |
|---|---|---|
| FR-1 | Parity-Core | `EV-SCENE-IMPORT` |
| FR-2 | Parity-Core | `EV-SCHEMA-LINT` |
| FR-3 | Parity-Core | `EV-RENDER-CONFORMANCE` |
| FR-4 | Parity-Core | `EV-SCRIPT-CONTRACT` |
| FR-5 | Parity-Core | `EV-AUDIO-TRACE` |
| FR-6 | Parity-Core | `EV-STATE-REPLAY` |
| FR-7 | Parity-Core | `EV-PRESET-PLAYLIST` |
| FR-8 | Parity-Advanced | `EV-INTEGRATION-MATRIX` |
| FR-9 | Differentiation | `EV-DIAGNOSTIC-QUALITY` |
| FR-10 | Parity-Core | `EV-CLEANROOM-POLICY-BASELINE` |
| FR-11 | Parity-Core | `EV-AUDIO-ROUTING-MATRIX` |
| FR-12 | Parity-Core | `EV-SHOW-BUNDLE-RECOVERY` |
| FR-13 | Parity-Core | `EV-AUTHORING-LIVE-RELOAD`, `EV-AUTHORING-DIAGNOSTICS`, `EV-AUTHORING-SCAFFOLDER` |
| FR-14 | Parity-Core | `EV-IMPORT-SHADERTOY`, `EV-IMPORT-ISF-REJECT` |
| FR-15 | Parity-Core | `EV-LIBRARY-DISCOVERY`, `EV-LIBRARY-THUMBNAIL` |
| FR-16 | Parity-Core | `EV-DOCS-CLEANROOM`, `EV-DOCS-VERSION-SYNC`, `EV-DOCS-COVERAGE` |
| FR-17 | Parity-Core | `EV-HOTKEY-PARITY-DESIGN` (Phase 0), `EV-HOTKEY-PARITY-CONFORMANCE` / `EV-HOTKEY-COMPLETENESS` / `EV-HOTKEY-CONFLICT` (Phase 4a) |
| FR-18 | Parity-Core | `EV-ENTITLEMENT-ENFORCEMENT`, `EV-ENTITLEMENT-OFFLINE`, `EV-ENTITLEMENT-REVOCATION`, `EV-ENTITLEMENT-TOKEN-FORMAT`, `EV-ENTITLEMENT-FREE-FEATURES` |
| FR-19 | Parity-Core | `EV-IMPORT-REFERENCE-SHOW` |
| FR-20 | Parity-Core | `EV-NET-HARDENING` |
| FR-21 | Parity-Core | `EV-CRASH-RESUME` |
| FR-22 | Parity-Core | `EV-MIDI-EXTENDED` |
| NFR-1 | Parity-Core | `EV-SOAK-CHAOS` |
| NFR-2 | Parity-Core | `EV-DETERMINISM` |
| NFR-3 | Parity-Core | `EV-PERF-BUDGETS` |
| NFR-4 | Parity-Core | `EV-OBSERVABILITY` |
| NFR-5 | Parity-Advanced | `EV-PLATFORM-CONFORMANCE` |
| NFR-6 | Parity-Core | `EV-SANDBOX-SAFETY` |
| NFR-7 | Parity-Core | `EV-LICENSE-AUDIT` |
| NFR-8 | Parity-Core | `EV-SUPPORT-OPS-READINESS` |
| NFR-8a | Parity-Core | `EV-OBSERVABILITY-INVENTORY`, `EV-ANALYTICS-ZERO-DEFAULT`, `EV-HEALTH-METRICS-LOOPBACK-ONLY`, `EV-CRASH-PAYLOAD-REDACTION`, `EV-ATS-NO-THIRD-PARTY-EXCEPTIONS` |
| NFR-9 | Parity-Core | `EV-NET-HARDENING` |
| NFR-10 | Parity-Core | `EV-AUTHORING-UX-LATENCY`, `EV-DOCS-UX-LATENCY` |
| NFR-11 | Parity-Core | `EV-ACCESSIBILITY-AA` |
| NFR-12 | Parity-Core | `EV-COLOR-MANAGEMENT-SRGB` |

## 8) Acceptance Criteria
- All FR/NFR items shall map to subsystem ownership, profile, and test evidence.
- Any temporary deferral from full feature parity shall be explicitly profile-tagged and owner-approved.
- No unresolved critical requirement gaps shall remain before implementation begins.
- No implementation kickoff may pass without legal policy baseline evidence (`EV-CLEANROOM-POLICY-BASELINE`).
- No release candidate may pass without legal audit evidence artifacts (`EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`, `EV-BRAND-REVIEW`).

## 9) Required Companion Artifacts
- `20-requirements-traceability-matrix.md`
- `21-operational-slos-slis.md`
- `22-compatibility-versioning-policy.md`
- `26-evidence-artifact-registry.md`
- `36-reactive-symbol-catalog.md`
- `37-authoring-environment-spec.md`
- `38-scene-library-and-import-spec.md`
- `adr/ADR-009-activation-and-entitlements.md`
- `adr/ADR-010-embedded-docs-generation.md`
- `adr/ADR-011-rendering-backend.md`
- `adr/ADR-012-script-sandbox-engine.md`
- `adr/ADR-013-networking-defaults-and-security.md`
- `adr/ADR-014-signing-notarization-auto-update.md`
- `adr/ADR-015-reference-app-user-data-import.md`
- `adr/ADR-016-observability-and-telemetry-posture.md`
