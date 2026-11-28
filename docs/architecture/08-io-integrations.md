# IO and Integration Blueprint

## Objective
Define adapter boundaries and route semantics for external control and media integrations.

## Integration Principles
- Core runtime remains protocol-agnostic.
- Adapters translate protocol details to canonical command/events.
- Integration failures are isolated and recoverable.

## Adapter Categories
- **Control Protocol Adapters**
 - OSC Adapter
 - MIDI Adapter
- **Media Input Adapters**
 - Camera Adapter
 - Syphon/Spout external texture feed adapter (platform-gated)
 - NDI input adapter (edition-gated)
- **Output Adapters**
 - OSC state/audio output
 - Syphon/Spout external texture output adapter (platform-gated)
 - NDI output adapter (edition-gated)
- **Display/Output Surface Workflows**
 - Fullscreen toggle and pop-out window handling
 - Multi-display placement and target selection
 - Constant vs scene-resolution output modes

## IntegrationHub Responsibilities
- Route incoming adapter events into `CommandBus`.
- Publish outbound state updates using compatibility route patterns.
- Maintain connection and health status per adapter.
- Enforce rate limits and command deduping policies.

## OSC Planning Contract (`osc-parity-v1`)
- Version every route contract. Initial version: `osc-parity-v1`.
- **Plurality convention**: collection roots use the plural form (`/scenes`, `/presets`, `/playlists`, `/favslots`, `/controls`); action verbs and listing queries live under the collection root. Chromaticity V1 canonical form is plural-root throughout inbound and outbound. The reference-app single-form paths (`/scene/{name}`, `/preset/{name}`) are supported as compat-syn-v1 aliases per Sec.OSC Compatibility Alias Table.
- Required route families:
 - **Control value routes:** `/controls/{bank}/{name}`, `/controls/global/{type}/{position}`.
 - **Control option suffixes:** `/raw`, `/default`, `/random`, `/lock`, `/undo`.
 - **Bank/group addressing:** `/controls/banks/...` and `/controls/groups/...` for bank-level and group-level targeting.
 - **Bank/group actions:** `default`, `random`, `undo`, `lock`. `undo` is wired to the operator-level undo stack (same stack as the `Cmd+Z` hotkey in `42-operator-hotkey-map.md` §10), scoped to control-value mutations within the bank/group; it does not affect scene switches or preset recalls. `lock` takes an integer argument `0|1` (set to unlocked/locked); an absent argument is rejected with `payload-required` diagnostic and no side effect.
 - **Global control grid:** fixed indexed slots per control type (default grid size 16), empty slots padded with empty-string names in discovery responses.
 - **Dropdown discovery:** `/controls/{bank}/{name}/numoptions` returns option count; `/controls/{bank}/{name}/label/{index}` returns option label at index. Under `compat-syn-v1` the legacy serial-label form `/controls/{addr}/label {string}` is emitted alongside the indexed form so existing TouchOSC dropdown templates continue to work; this dual-emit is toggleable in the OSC operator settings panel per Sec.OSC Operator Settings Panel Contract.
 - **Scene routes (canonical, plural):** `/scenes` (list, outbound); `/scenes/{name} {preset?}` (inbound scene launch); `/scenes/{name}` (outbound announcement on scene change). `{preset}` is optional; omitted preset resolves to the scene's gold/Snap-0 preset per `28-implementation-contract-pack.md` CP-05.
 - **Preset routes (canonical, plural):** `/presets` (list, outbound); `/presets/{name}` (inbound preset apply); `/presets/scene`, `/presets/meta`, `/presets/media` (channel filters); `/presets/new {name}` (save-current-state as new named preset, prompts via the app UI only if the name collides).
 - **Snap routes (canonical, plural; `favslot` is the wire literal retained for compat-syn-v1):**
   - `/favslots/{n}` (apply): recall Snap slot `n` for the current scene (`n` in `[0, 15]`).
   - `/favslots/{n}/save` (save): overwrite Snap slot `n` with current state. Triggers `/favslots/{n}/name` announcement on the outbound side.
   - `/favslots/{n}/clear` (clear): delete Snap slot `n`; slot returns to unbound state.
   - Slot `0` is the gold preset used by `StartSceneWithPreset` when no preset is supplied (CP-05).
   - The `apply` vs `save` vs `clear` disambiguation is **wire-level explicit** via the route suffix; there is no implicit mode selection. Legacy `/favslots/{n} {mode}` with a string argument `"apply"|"save"|"clear"` is accepted as a `compat-syn-v1` alias.
 - **Playlist routes:** `/playlists` (list, outbound); `/playlists/next`, `/playlists/previous`, `/playlists/position {i}` (negative indices count from end), `/playlists/play`, `/playlists/toggleplay`, `/playlists/select {name}`.
 - **Media routes:** `/media/name {value}`, `/media/position {i}`, media source listing and availability updates.
 - **Rendering/status routes:**
 - `/render/enabled {0|1}` (inbound): toggles the render loop on/off. When `0`, the GPU scheduler halts scene rendering while the UI and adapter threads remain responsive; outbound audio/MIDI/OSC state continues. Useful for headless SMPTE-slaved setups, A/V calibration, and operator-triggered "hold current output" cues. Restoration is instantaneous (no warm-up) because compiled pipelines are cached per `adr/ADR-011-rendering-backend.md` Sec.Shader cache.
 - `/render/resolution {w} {h}` (inbound, capability-gated `render.custom-resolution`): resize output surface.
 - `/render/fps {float}` (inbound): override render FPS target.
 - Runtime status publication and diagnostics.
 - **Audio routes:**
 - `/audio/threshold {float}` (inbound and outbound discovery): sets `chroma_Threshold` per `36-reactive-symbol-catalog.md` Sec.2.10 and `06-audio-engine.md` Sec.7.12. Clamped to `[0, 1]`. Emitted on settings panel change.
 - `/audio/threshold/sample` (inbound, no argument): samples the current 200 ms rolling RMS of `chroma_Level` and sets `chroma_Threshold` to that value. Used by the `Shift+T` hotkey and by hardware-surface momentary buttons. Emits `/audio/threshold {float}` outbound on completion.
 - **Audio-variable output routes (canonical prefix: `/audio/var/*`):** reactive audio uniforms are published outbound under `/audio/var/{uniform_name_lowercase_stripped}` at the cadence configured in the OSC operator settings panel, where `uniform_name_lowercase_stripped` is the catalog symbol name with the `chroma_` prefix removed, camelCase preserved (e.g. `chroma_BassLevel` -> `/audio/var/bassLevel`, `chroma_OnBeat` -> `/audio/var/onBeat`). The operator may filter per-uniform in the settings panel; the default set is the `LVL`, `HIT`, and `BEAT` families.
 - **Reserved prefixes (inbound, reject-with-diagnostic at MVP, activated post-MVP):**
 - `/data/*` and `/data/subscribe/*` - reserved for the post-MVP external-data-ingestion differentiator (`41-data-ingestion-differentiator.md`, reserved ADR-017). At MVP the adapter rejects inbound messages under these prefixes with a `prefix-reserved` diagnostic and no side effects. This prevents third-party integrations from squatting on the prefix before the V2 schema is published.
- Outbound behavior:
 - on scene load, a canonical state burst is emitted once for all bound control/route classes,
 - burst includes both `/raw` and normalized values per subscriber configuration,
 - outbound payloads include profile/version metadata for compatibility routes.
- Route aliases for legacy addressing are allowed under `compat-syn-v1` and are enumerated in Sec.OSC Compatibility Alias Table below. Aliases must resolve deterministically to a canonical route; collisions between two legacy routes that would map to the same canonical route are prohibited.

### OSC Compatibility Alias Table (`compat-syn-v1`)

These aliases are accepted on inbound and emitted on outbound when `compat-syn-v1` mode is enabled in the OSC operator settings panel (default: on, for migration friendliness). Toggling `compat-syn-v1` off disables both alias acceptance and alias emission atomically.

| Legacy route (reference) | Canonical Chromaticity route | Direction | Behavior |
|---|---|---|---|
| `/scene/{name} {preset?}` | `/scenes/{name} {preset?}` | inbound | Same semantics; `{preset}` optional; unsupplied preset resolves to Snap 0. |
| `/scene/{name}` | `/scenes/{name}` | outbound (scene-change announcement) | Emitted as both forms during the dual-emit window. |
| `/preset {string}` (payload-form launch) | `/presets/{string}` (path-form launch) | inbound | Legacy payload form is translated to the path form before command dispatch. |
| `/controls/{addr}/label {string}` (serial dropdown labels) | `/controls/{bank}/{name}/label/{index} {string}` (indexed dropdown labels) | outbound | Dual-emit on scene load; serial form omits the index and is emitted once per option in order. Settings panel controls whether dual-emit stays on or the canonical-only form is used. |
| `/favslots/{n} {mode}` where mode is `"apply"`/`"save"`/`"clear"` | `/favslots/{n}`, `/favslots/{n}/save`, `/favslots/{n}/clear` | inbound | Legacy mode-argument form is accepted; the wire adapter rewrites to the explicit suffix form. |
| `/playlist/*` (singular-root action paths) | `/playlists/*` | inbound | Singular-root paths `/playlist/next`, `/playlist/previous`, etc. are accepted as aliases for their plural-root canonical forms. |

Aliases are removed under `osc-parity-v2` (post-MVP); removal is a profile bump per `22-compatibility-versioning-policy.md`. `EV-INTEGRATION-MATRIX` exercises both legacy and canonical forms with a differential-output assertion.

### OSC Discovery and Announcement Routes
- **Scene announcement:** on scene change, the runtime emits `/scenes/{sceneName}` (canonical) with the new scene's URL-safe slug of its `TITLE`. Under `compat-syn-v1` the legacy `/scene/{sceneName}` form is also emitted during dual-emit.
- **Control name discovery per global grid slot:** `/controls/global/{type}/{position}/name` -> `{string}`; empty-string for unbound slots. Emitted on scene load and on any global-binding change.
- **Dropdown enumeration:** `/controls/{bank}/{name}/numoptions` -> `{int}` and `/controls/{bank}/{name}/label/{index}` -> `{string}` for each `index` in `[0, numOptions)`. Emitted on scene load and on any dropdown label change. Under `compat-syn-v1` the serial `/controls/{addr}/label {string}` form is dual-emitted in index order.
- **Playlist announcement:** `/playlists/current {name}` and `/playlists/position {i}` emitted on change (plural-root canonical; singular-root `/playlist/*` dual-emitted under `compat-syn-v1`).
- **Snap slot announcement:** `/favslots/{n}/name {string}` emitted on show load and after any save-to-slot or clear-slot action. (The literal `favslot` is retained on the wire under `compat-syn-v1`; the UX surface uses "Snap".)
- **Snap slot count:** exactly **16** Snap slots per show (retained as `favslot` in `compat-syn-v1` wire literal); slot `0` is the gold preset used by `StartSceneWithPreset` when no preset is supplied (see `28-implementation-contract-pack.md` CP-05).
- **Snap keyboard surface:** operator hotkeys in `42-operator-hotkey-map.md` §3 cover slots 1-10 and the gold slot 0. Slots 11-15 are addressable only via OSC, via the Snap panel UI, or via MIDI mapping. This is intentional; the UX design locks the hotkey row at 11 bindings (0-10) for physical-keyboard ergonomics.

### OSC Conformance Test IDs (under `EV-INTEGRATION-MATRIX`)
- `OSC-CONF-01` Control value routes (`/controls/{bank}/{name}`) normalize and clamp per payload typing rules.
- `OSC-CONF-02` Control option suffixes (`/raw`, `/default`, `/random`, `/lock`, `/undo`) each produce the documented side effect exactly once per inbound message.
- `OSC-CONF-03` Global control grid positions are addressable; empty-slot names appear as empty string in discovery.
- `OSC-CONF-04` Bank and group actions (`default`, `random`, `undo`, `lock`) affect every control in scope atomically; `lock` requires an integer argument and rejects an absent argument with `payload-required` diagnostic.
- `OSC-CONF-05` Scene launch (canonical `/scenes/{name}`, legacy `/scene/{name}`) resolves unsupplied preset to Snap 0 / gold under both forms.
- `OSC-CONF-06` Preset channel routing (`/presets/scene`, `/presets/meta`, `/presets/media`) isolates channels.
- `OSC-CONF-07` Playlist transports (`next`, `previous`, `position`, `play`, `toggleplay`, `select`) move state deterministically with documented wrap and negative-index semantics.
- `OSC-CONF-08` Snap slot apply, save, and clear each use explicit path suffixes (`/favslots/{n}`, `/favslots/{n}/save`, `/favslots/{n}/clear`); each completes before the inbound-ack frame and emits the corresponding outbound announcement.
- `OSC-CONF-09` Dropdown discovery (`/numoptions`, `/label/{i}`) returns stable values per scene load; under `compat-syn-v1` the serial form is dual-emitted in index order without gaps.
- `OSC-CONF-10` Scene-load state burst includes every bound route class and emits version metadata.
- `OSC-CONF-11` Outbound audio-variable namespace publishes at the cadence configured under the canonical `/audio/var/*` prefix, independent of inbound traffic. Uniform-name mapping is exact per the convention in this doc.
- `OSC-CONF-12` Malformed inbound payloads are rejected with diagnostic and no side effects.
- `OSC-CONF-13` `/render/enabled {0|1}` halts and resumes the render loop without dropping adapter connections or outbound audio-variable publication; restoration frame is within one frame budget of the resume message.
- `OSC-CONF-14` `/render/custom-resolution` is capability-gated: without `render.custom-resolution` entitlement, inbound messages are dropped with `entitlement-denied` diagnostic and no side effects.
- `OSC-CONF-15` `/audio/threshold` clamps to `[0, 1]`, updates `chroma_Threshold` within one audio hop, and reemits current value on any settings panel change. `/audio/threshold/sample` sets the threshold to the current 200 ms rolling RMS of `chroma_Level` and emits a discovery message.
- `OSC-CONF-16` Inbound messages under reserved prefixes `/data/*` and `/data/subscribe/*` are rejected with `prefix-reserved` diagnostic and produce no side effects at MVP.
- `OSC-CONF-17` Plurality alias conformance: every canonical plural-root route in the Alias Table accepts inbound under both the plural-root canonical form and the singular-root legacy form; differential-output assertion confirms both forms produce identical internal command-bus output.
- `OSC-CONF-18` Alias round-trip: an outbound dual-emit pair (canonical + legacy) arriving back via a loopback subscriber produces a single internal command (no double-application).
- `OSC-CONF-19` Dropdown label serial/indexed dual-emit: on scene load with `compat-syn-v1` enabled, both forms appear in the outbound burst in the same order; with `compat-syn-v1` disabled, only the indexed form appears.
- `OSC-CONF-20` Bank `/undo` action unwinds exactly the bank's last control-value change and emits an announcement of the restored values; does not affect scene or preset history.

## OSC Payload Typing and Normalization
- Value payload classes:
 - scalar numeric (`float`), normalized numeric (`0.0..1.0`), integer index, boolean toggle.
 - symbolic payloads for scene/preset/playlist identifiers.
- Normalization rules:
 - Convert integer/boolean payloads for toggle and option routes to canonical boolean.
 - Clamp normalized control payloads to `[0.0, 1.0]` before command dispatch.
 - Reject malformed payload dimension/type with explicit diagnostics and no side effects.
- Outbound typing mirrors canonical state model and includes profile/version metadata for compatibility routes.

## MIDI Planning Contract (`midi-parity-v1`)
- Version every MIDI behavior contract. Initial version: `midi-parity-v1`.
- Mapping modes:
 - learned mappings (MIDI-learn flow) with per-scene persistence,
 - static mappings via importable bindings file,
 - overwrite-old-mappings per scene on re-learn (previous binding for the same control in that scene is replaced).
- Message-class to control-type mapping:
 - CC (0-127) to `slider` / `knob`: linear by default with per-mapping `MIN`/`MAX` override.
 - CC to `toggle`: value crossing configurable threshold (default 64) latches on/off.
 - CC to `bang`: rising-edge above threshold triggers bang once.
 - CC to `dropdown`: CC (0-127) maps to dropdown index `floor(value / 128 * numOptions)` clamped to `[0, numOptions-1]`.
 - Note On/Off to `toggle` / `bang`: Note On above `velocity > 0` latches/triggers; Note Off releases.
 - Note On/Off to `slider` / `xy`: velocity routed as normalized scalar (`value/127`).
- Per-mapping options:
 - custom input `MIN`/`MAX` clamp before control value mapping,
 - invert,
 - soft-takeover (pickup) to avoid jumps when controller value diverges from current control,
 - lock (ignore incoming events without removing binding).
- Device handling:
 - device IDs persist across reconnect; bindings restore on re-enumeration,
 - multi-device support with per-device allowlist configuration,
 - unknown device arrival surfaces diagnostic without mutating bindings.
- Output:
 - outbound MIDI reflects canonical control state changes for bound controls,
 - outbound payload types mirror the inbound mapping type.
- Health and reconnect behavior inherit from the adapter conformance criteria below.

### MIDI Conformance Test IDs (under `EV-INTEGRATION-MATRIX`)
- `MIDI-CONF-01` CC-to-slider linear mapping with custom MIN/MAX.
- `MIDI-CONF-02` CC-to-toggle threshold latching.
- `MIDI-CONF-03` CC-to-bang rising-edge triggering.
- `MIDI-CONF-04` CC-to-dropdown index mapping across full 0-127 range.
- `MIDI-CONF-05` Note On/Off to toggle and bang lifecycles.
- `MIDI-CONF-06` Note On velocity to slider/XY normalized mapping.
- `MIDI-CONF-07` Soft-takeover/pickup semantics on reconnect.
- `MIDI-CONF-08` Overwrite-old-mappings per scene on re-learn.
- `MIDI-CONF-09` Device reconnect binding restoration.
- `MIDI-CONF-10` Outbound MIDI mirrors control state for bound controls.

### Extended MIDI Surface (`midi-parity-v1` extension; tracked under `EV-MIDI-EXTENDED`)
Extensions beyond the reference-observed CC/Note surface. These are Chromaticity differentiation, not parity, and are in scope for MVP per FR-22.

- **Pitch Bend**:
 - 14-bit resolution (MSB+LSB) -> `slider` / `xy` axis with normalization `value14 / 16383`.
 - per-mapping invert and center-rest-at-0.5 options.
- **Program Change**:
 - maps to preset apply via operator mapping table `(device, channel, program) -> preset_id`.
 - rising-edge only (re-sending the same program re-applies the preset).
- **MIDI Clock** (`0xF8`) + **Song Position Pointer** (`0xF2`) + **Start/Stop/Continue** (`0xFA`/`0xFC`/`0xFB`):
 - when "External BPM Sync" is enabled, the MIDI Clock rate drives `chroma_BPM` directly; audio-engine fusion policy in `06-audio-engine.md` Sec.7.9 yields to the external clock with `chroma_BPMConfidence = 1.0`.
 - Start/Stop toggles a `beat_phase_locked` flag; internal beat estimator resumes on Stop.
 - Song Position Pointer resets `chroma_BeatTime` to the indicated position (1 SPP unit = 6 MIDI Clock ticks = 1/16 note).
- **SysEx**: logged with size + manufacturer ID only; no routing. Full SysEx support deferred to V2.

### Extended MIDI Conformance Test IDs (under `EV-MIDI-EXTENDED`)
- `MIDI-CONF-11` Pitch Bend 14-bit accuracy across full range with center rest.
- `MIDI-CONF-12` Program Change -> preset apply via mapping table.
- `MIDI-CONF-13` MIDI Clock -> `chroma_BPM` external-sync lock, confidence reports 1.0.
- `MIDI-CONF-14` Song Position Pointer resets `chroma_BeatTime` deterministically.
- `MIDI-CONF-15` Start/Stop/Continue toggles beat-phase-lock without glitches.

### Additional MIDI Surface Decisions (tracked under `EV-MIDI-EXTENDED`)

- **14-bit CC (MSB 0-31 + LSB 32-63 pair)**:
 - In scope for MVP.
 - Operator mapping mode "14-bit CC" pairs MSB CC `n` with LSB CC `n+32` and normalizes `value14 / 16383` to the bound control.
 - MIDI-CONF-16 validates the pair across full range with and without LSB activity and confirms that an MSB message arriving without a subsequent LSB within 50 ms applies with implicit LSB=0.
- **NRPN / RPN**:
 - In scope for MVP, 14-bit resolution only (value range 0-16383).
 - Mapping mode selects NRPN or RPN controller number `(MSB, LSB)` pair and target control.
 - SysEx-transported parameter changes are out of scope for V1; tracked separately.
 - MIDI-CONF-17 validates full-range NRPN and RPN value transmission and the reset-null-parameter-number sequence.
- **MPE (MIDI Polyphonic Expression)**:
 - Deferred to V2. MVP rejects Lower Zone and Upper Zone Configuration messages with a `mpe-deferred` diagnostic and no side effects; per-note Pitch Bend, CC 74, and Channel Pressure are ignored when arriving on MPE member channels.
 - MIDI-CONF-18 validates that MPE-configuration messages do not corrupt global state and that the diagnostic fires exactly once per session.
- **MTC / SMPTE LTC stance**:
 - **MTC** (MIDI Time Code, quarter-frame `0xF1` + full-frame SysEx): in scope for MVP as a `chroma_Timecode` uniform family (hours/minutes/seconds/frames/framerate-id), consumable by scenes for SMPTE-slaved playback. MIDI-CONF-19 validates 24/25/29.97/30 fps modes and quarter-frame assembly correctness.
 - **SMPTE LTC** (audio-carried timecode): deferred to V2 via a dedicated LTC decoder; MVP surfaces a `ltc-not-ingested` diagnostic if the operator selects an LTC input without the V2 decoder.
- **MIDI learn persistence**:
 - In scope for MVP; MIDI mappings survive show-save, app quit, and relaunch at parity with MIDI-CONF-09 reconnect restoration.
 - MIDI-CONF-20 validates the save-quit-relaunch-load cycle: a binding learned in session A is present and functional in session B without operator re-learning.

## Camera/Media Contract
- Distinguish media type semantics in runtime state.
- Provide explicit behavior for unavailable devices and reconnect.
- Abstract media texture orientation and aspect correction policies.

## External Texture I/O Contract (Syphon/Spout)
- Output:
 - operator-toggled output surface with runtime-rename-capable sender name,
 - selectable output resolution modes: `constant-resolution` (fixed user-configured frame size) and `scene-resolution` (follows active scene render size),
 - output is visible to other applications on the local host only.
- Input:
 - single active input source per runtime,
 - source list is discovered from local senders and refresh-capable,
 - input appears in the media UI/state as an external-feed media source,
 - input failure surfaces as missing-media state without mutating scene bindings.
- Platform binding:
 - Syphon used on macOS, Spout on Windows, behind a unified adapter interface per `adr/ADR-005-integration-adapter-boundary.md`.

## NDI I/O Contract
- Feature is edition-gated; behavior below applies only when the edition exposes NDI.
- Output:
 - single active NDI output stream,
 - sender name is operator-configurable and persists per show,
 - output availability is reported via adapter health channel.
- Input:
 - one active NDI input source at a time,
 - source selection persists across reconnect,
 - source loss triggers bounded retry with operator diagnostic and fallback to no-media state.
- Network handling:
 - bind scope defaults to local subnet; allowlist/denylist support is exposed in settings.
 - firewall/port remediation diagnostics are surfaced when sender discovery fails.

## Display/Output Surface Workflow
- Fullscreen toggle must be reachable without scene interaction and is non-destructive to scene state.
- Multi-display placement:
 - macOS: window may be dragged to target display before maximizing; operator runbook documents `Cmd+S` / `Cmd+Shift+F` equivalents where applicable,
 - Windows: output display is selectable from a dropdown before fullscreen toggle.
- Constant vs scene-resolution output:
 - constant-resolution maintains a fixed output frame size and scales scene output appropriately,
 - scene-resolution matches output frame size to active scene render size,
 - switching modes is atomic and must not tear or desync external texture/NDI outputs.
- Pop-out output window is allowed as an alternative to fullscreen for specific operator workflows.

## Device and Media Acceptance Matrix (MVP)

| Class | Required Behavior | Failure Behavior | Evidence |
|---|---|---|---|
| `AUD-1` built-in audio | Selectable input/output and stable level signal path | fall back to silent reactive outputs + operator warning | `EV-AUDIO-ROUTING-MATRIX` |
| `AUD-2` USB interface | Device and channel selection persist across reconnect | restore prior mapping after reconnect; if fail, preserve runtime and raise actionable diagnostic | `EV-AUDIO-ROUTING-MATRIX` |
| `AUD-3` system-audio/loopback | Supported platform workflow is first-class and diagnosable | if unavailable, expose guided remediation and keep render pipeline alive | `EV-AUDIO-ROUTING-MATRIX` |
| `MED-1` file media | Deterministic media load and type assignment | fail closed for invalid media with non-fatal diagnostic | `EV-INTEGRATION-MATRIX` |
| `MED-2` camera | Device availability and orientation policy are explicit | auto-retry reconnect with bounded backoff; fallback to no-media state | `EV-INTEGRATION-MATRIX` |
| `MED-3` external texture feed (Syphon/Spout) | Single active input; selectable output resolution mode; sender rename without teardown | connection failure isolates adapter and preserves scene runtime | `EV-INTEGRATION-MATRIX` |
| `MED-4` NDI (edition-gated) | Single active input/output; source persistence across reconnect; network diagnostics | bounded retry on source loss; fallback to no-media state without scene mutation | `EV-INTEGRATION-MATRIX` |
| `CTRL-1` MIDI | Mapping persistence and soft-takeover options | recover bindings on reconnect; never crash runtime | `EV-INTEGRATION-MATRIX` |
| `CTRL-2` OSC | Versioned route behavior with normalization rules | reject invalid payloads with diagnostics and zero side effects | `EV-INTEGRATION-MATRIX` |
| `OUT-1` display/output surface | Fullscreen toggle, multi-display placement, constant vs scene-resolution output | mode switches are atomic; no output tear on toggle | `EV-INTEGRATION-MATRIX` |

## OSC Operator Settings Panel Contract

This section is the normative contract the UX team implements as **Settings -> OSC**. Every row maps to a wire-level behavior defined above; no row introduces new wire semantics.

### Panel layout

| Group | Control | Type | Default | Wire binding |
|---|---|---|---|---|
| Input | Enable OSC input | toggle | off | Binds OSC server to the configured port; off detaches adapter. |
| Input | Input port | integer `[1, 65535]` | 8888 | Rebind on change; reconnect per `Health and Recovery`. |
| Input | Network exposure | radio: Loopback only \| Local subnet \| Allowlisted IPs | Loopback only | `ADR-013-networking-defaults-and-security.md` bind scope; non-loopback requires explicit opt-in. |
| Input | Source-IP allowlist (CIDR) | text-list | empty | Applied per mapping when "Allowlisted IPs" is selected. |
| Input | Rate limits (msg/s per adapter) | integer | 1000 (hard cap) | ADR-013 rate cap; per-route cap fixed at 100 msg/s. |
| Output | Enable OSC output | toggle | off | Detach mirrors Input enable behavior. |
| Output | Output destinations | list of `(host, port)` | empty | Per-destination filtering below is applied independently. |
| Output | Output normalization | radio: Normalized only \| Raw only \| Both (dual-emit) | Normalized only | When "Raw only" or "Both", the `/raw` suffix path is emitted with the raw-space value. |
| Output | Scene-specific vs global address format | radio: Scene-specific \| Global \| Both | Scene-specific | "Scene-specific" uses the scene's own bank name; "Global" uses the global grid slot address. |
| Output | `compat-syn-v1` alias mode | toggle | on | When on, both canonical and legacy aliases are accepted inbound and emitted outbound per the Alias Table. |
| Output | Audio-variable output enabled | toggle | off | When on, `/audio/var/*` publishes at the configured cadence. |
| Output | Audio-variable cadence | dropdown: `per-frame` \| `60 Hz` \| `30 Hz` \| `10 Hz` | 60 Hz | Affects only `/audio/var/*`; all other outbound routes are event-driven. |
| Output | Audio-variable uniform set | multi-select of catalog symbols | LVL+HIT+BEAT families | Per-uniform filter for `/audio/var/*`. |
| Mapping | In-app OSC Mapping Tooltip | toggle | on | When on, operator can hover/click any control in the panel UI to reveal its current OSC address(es) including any `compat-syn-v1` alias. Uses the command-palette disclosure affordance as a secondary path. |
| Mapping | Log OSC traffic (inbound) | toggle | off | When on, every accepted inbound message is logged at `info` severity to the diagnostics console under subsystem `osc.wire`; rate-limited to 1000 log lines/s. |
| Mapping | Log OSC traffic (outbound) | toggle | off | Same, subsystem `osc.wire`, `out` sub-tag. |

### UX contract deliverables

The UX team produces `EV-UX-OSC-SETTINGS-CONTRACT` covering:
1. wireframe of the panel with the row list above,
2. per-row interaction state (dirty/valid/conflict) and save semantics,
3. persistence: every row persists in the show bundle (`.chromaShow`) and survives save/quit/relaunch,
4. accessibility conformance per NFR-11 (target sizes, contrast, screen-reader labels),
5. layout spec for the in-app OSC Mapping Tooltip (position, dismissal, keyboard invocation).

A parallel `EV-UX-MIDI-SETTINGS-CONTRACT` covers the MIDI settings panel with: device list + allowlist, per-scene mapping list, MIDI-learn indicator state, external-BPM-sync toggle, extended-surface opt-ins (14-bit CC, NRPN/RPN, MPE deferral notice, MTC framerate dropdown), and the same persistence/accessibility contract.

## Health and Recovery
- Connection state machine per adapter: `Disconnected`, `Connecting`, `Connected`, `Error`, `Recovering`.
- Automatic recovery with backoff on recoverable transport errors.
- Clear operator-visible diagnostics and actionable remediation text.
- Reconnect semantics:
 - retry schedule: 100 ms, 250 ms, 500 ms, 1000 ms, then 2000 ms capped loop.
 - cumulative time through first 4 attempts is <= 1850 ms, landing a typical reconnect within the SLO-04 p50 <= 3 s target.
 - preserve mapping/binding state for recoverable disconnects.
 - emit `Recovering` diagnostics after second retry window.
 - require successful state reconciliation before `Connected` transition.

## Adapter Conformance Criteria
- Reconnect behavior:
 - preserve binding/mapping state across recoverable disconnects.
 - meet reconnect targets defined in `21-operational-slos-slis.md`.
- Route compatibility drift:
 - every adapter release runs `EV-INTEGRATION-MATRIX` against route baseline for `osc-parity-v1`.
 - incompatible route behavior requires profile/version update and migration notes.
- Alias handling:
 - aliases must remain deterministic and collision-free.
 - canonical route takes precedence when alias and canonical commands arrive concurrently.

## Security and Safety
Networking defaults, bind scope, source-IP allowlists, rate limits, payload hardening, and discovery posture are locked in `adr/ADR-013-networking-defaults-and-security.md`. Summary:
- OSC default bind is loopback `127.0.0.1`; network exposure requires explicit operator opt-in.
- Per-adapter inbound rate cap 1000 msg/s hard, 100 msg/s default; per-route cap 100 msg/s.
- Source-IP allowlist per OSC mapping (CIDR supported).
- OSC packet size cap 64 KiB; bundle depth cap 4.
- NDI discovery scoped to local subnet by default with allowlist/denylist overrides.
- No service-discovery broadcast on unbound interfaces.
- Operator overrides persist in show bundles.
- Restrict accepted command space by adapter policy.
- Validate payload dimensions and ranges before command conversion.

## Implementation Contract Baseline
- Adapter interfaces and translation semantics are fixed by `adr/ADR-005-integration-adapter-boundary.md`.
- Route matrix and payload normalization policy are locked in `28-implementation-contract-pack.md`.
- Compatibility drift handling and migration behavior is governed by `22-compatibility-versioning-policy.md`.
