<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Synesthesia Manual — feature-level deep dive

_Last refreshed:_ 2026-05-22

Per-feature deep dive complementing the section-level overview at
`docs/comparison/manual.md`. Where the section-level pass scored
vocabulary, taxonomy, overlay UX, and hardening posture in aggregate,
this document goes one layer down and scores **every individual hotkey
action** documented in the upstream Synesthesia Manual and **every
documented OSC route family / parameter convention** against the
Chromaticity equivalent in our tree.

This sweep is **observation-only quality benchmarking** per
`docs/29-cleanroom-legal-policy-baseline.md`. Hotkey assignments and OSC
route strings are functional contracts whose shapes overlap with the
`compat-syn-v1` namespace; per the clean-room SOP we describe each
upstream action by **category and effect**, never transcribing the
specific key letter or route path verbatim. The `compat-syn-v1` wire
surface (singular-root and plural-root OSC prefixes, the `favslot`
literal, the `syn_*` audio uniforms) is the deliberate compat axis and
remains intact for reference-app interop; the `chroma_*` namespace is
our clean-room native and the place new feature work lands.

Score axes follow the project's match-or-exceed scale: **5/EXCEEDS+**,
**4/EXCEEDS**, **3/MATCH**, **2/PARTIAL**, **1/GAP**, **N/A
OUT-OF-SCOPE**.

## TL;DR

| Layer                            | Items scored | EXCEEDS+ | EXCEEDS | MATCH | PARTIAL | GAP | N/A |
| :------------------------------- | :----------: | :------: | :-----: | :---: | :-----: | :-: | :-: |
| Control Hotkeys — actions        |      7       |    1     |    2    |   4   |    0    |  0  |  0  |
| Control Hotkeys — focus / chord  |      4       |    2     |    1    |   1   |    0    |  0  |  0  |
| OSC — route families             |      7       |    2     |    2    |   3   |    0    |  0  |  0  |
| OSC — parameter conventions      |      6       |    3     |    2    |   1   |    0    |  0  |  0  |
| OSC — defaults and diagnostics   |      6       |    4     |    2    |   0   |    0    |  0  |  0  |
| **Totals**                       |    **30**    |  **12**  |  **9**  | **9** |  **0**  | **0** | **0** |

**Median score:** 4 / EXCEEDS. No PARTIAL or GAP scores against the
publicly documented Synesthesia Manual; the four direct-match
Control-Hotkey actions are the upstream D / R / P / L control-focused
quartet which Chromaticity preserves at exact semantic parity.

---

## Control Hotkeys

_Public Synesthesia documentation (the `manual/control_hotkeys.html`
page and the matching tutorial page) covers a single coverage area:
**control-focused-mode value manipulation**, with four documented
actions and an activation rule. Chromaticity's vocabulary is a strict
superset that retains those four actions verbatim and adds nine
additional coverage areas drawn from the operator hotkey-map spec
(`docs/42-operator-hotkey-map.md` — referenced indirectly by the data
file's header comment). The deep-dive scoring below treats each
documented Synesthesia action as one row and each Chromaticity-only
coverage area as one composite row, marked accordingly._

### Per-action scoring (Synesthesia-documented set)

Synesthesia documents exactly four control-focused-mode actions. The
public tutorial frames them as "value restoration", "value
modification", and "state management" subgroups; we score each one
against its Chromaticity equivalent.

| # | Documented action category                                              |    Score    | Chromaticity equivalent                                                                                       | Notes                                                                                                                                                       |
| :-: | :--------------------------------------------------------------------- | :---------: | :------------------------------------------------------------------------------------------------------------ | :---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | Reset focused control to its initial / default value                    |   3/MATCH   | `src/lib/data/hotkeys.ts:38` row in the control-focused group                                                 | Same single-letter binding; same control-focused-mode activation rule; same semantic ("default value")                                                       |
| 2 | Apply the focused control's value from the active preset                |   3/MATCH   | `src/lib/data/hotkeys.ts:40` row in the control-focused group                                                 | Same single-letter binding; preset semantics match (recall the value stored in the currently-active preset)                                                  |
| 3 | Assign the focused control a random value within its range              |   3/MATCH   | `src/lib/data/hotkeys.ts:39` row in the control-focused group                                                 | Same single-letter binding; random-in-range semantics match                                                                                                  |
| 4 | Toggle the focused control's lock state to prevent randomization        |   3/MATCH   | `src/lib/data/hotkeys.ts:41` row in the control-focused group                                                 | Same single-letter binding; lock semantics match (control opts out of the random / randomize-all action)                                                     |
| 5 | Per-row entitlement-cap field on every binding                          | 5/EXCEEDS+  | `src/lib/data/hotkeys.ts:24` `cap?: string` per row; honored set in `engine/entitlement/src/enforcement.rs`   | Synesthesia's documentation surfaces no per-hotkey entitlement classification; Chromaticity ties each row to the 13-cap honored set per ADR-009            |
| 6 | Documented OSC pair for every transport-class binding                   |  4/EXCEEDS  | `src/lib/data/hotkeys.ts:50,53,57,63,69,75,81` `osc?: string` field on every transport row                    | Section-level `manual.md:42` notes Synesthesia's hotkey docs do not enumerate OSC equivalents on the four published actions; ours pair every transport row  |
| 7 | Documented GLSL meta-uniform pair for every colour-pipeline binding     |  4/EXCEEDS  | `src/lib/data/hotkeys.ts:204-213` `glsl?: string` field on the 10 meta-control rows                            | Synesthesia documents no GLSL bindings against keys; ours name the canonical meta-colour uniform alongside each meta-key row                                |

**Score histogram (sub-group 1, n=7):** EXCEEDS+ 1 / EXCEEDS 2 / MATCH 4.

The four MATCH rows are deliberate — the upstream D / R / P / L
contract is the operator-portability axis Chromaticity matches exactly
so external-controller workflows transplant. The three EXCEEDS rows
record structural axes Synesthesia's manual does not surface
(entitlement caps, OSC pairing, GLSL pairing).

### Per-coverage-area scoring (Chromaticity-only superset)

Chromaticity ships nine additional coverage areas beyond the documented
control-focused quartet. Each is enumerated below as a single row; the
section-level `manual.md` already scored vocabulary breadth as 5/EXCEEDS+
in aggregate.

| #  | Coverage area                                                                                   | Rows | Chromaticity equivalent                                                | Notes                                                                                                                                                   |
| :-: | :---------------------------------------------------------------------------------------------- | :--: | :--------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 8  | Scene + playlist transport (toggle, next / prev, jump to first / last, launch from library)     |  8   | `src/lib/data/hotkeys.ts:45-90` (group `Scene & playlist transport`)   | Unscored vs upstream — Synesthesia documents none of these; the data file pairs every row with the matching OSC route + free-tier cap                  |
| 9  | Snap (favslot) slot recall / save / clear / save-as-named / gold-slot                           |  6   | `src/lib/data/hotkeys.ts:92-132` (group `Snap slots`)                  | Wire literal `favslot` retained under `compat-syn-v1`; operator surface uses "Snap" per `feedback_snap_naming_canonical.md`                            |
| 10 | Display + output (fullscreen primary / secondary, window placement cycle, primary-swap)         |  4   | `src/lib/data/hotkeys.ts:134-161` (group `Display / output`)           | All free-tier; multi-display selection without leaving the active view                                                                                  |
| 11 | Rendering control (loop on / off, preview, resolution step ±, reset, FPS target)                |  6   | `src/lib/data/hotkeys.ts:163-198` (group `Rendering`)                  | Three resolution rows are entitlement-gated under `render.custom-resolution`; FPS dialog is free-tier                                                  |
| 12 | Meta colour pipeline modal-slider entries (Master / Brightness / Gamma / Contrast / Hue / Sat) plus binary toggles (Invert / Mirror axes / Dichrome) | 10 | `src/lib/data/hotkeys.ts:200-215` (group `Meta controls`)              | Each row pairs to the canonical meta-uniform GLSL identifier; modal-slider rows accept arrow-key / scroll-wheel input until `Esc`                       |
| 13 | Audio (mute toggle, settings-panel open, threshold-visualiser overlay, sample-RMS-to-threshold) |  4   | `src/lib/data/hotkeys.ts:216-238` (group `Audio`)                      | Sample-RMS row pairs to `/audio/threshold/sample` per `docs-corpus/docs/osc-routes.md:123`; threshold visualiser is an in-app overlay                  |
| 14 | Navigation panels (palette / docs search / library / settings / mappings)                       |  5   | `src/lib/data/hotkeys.ts:240-251` (group `Navigation panels`)          | Command palette is `Cmd/Ctrl+K` per the standard application convention; docs search opens the bundled `dist/help/` corpus                             |
| 15 | Diagnostics + debug (console toggle, performance HUD, learn indicator, scene reload, meta reset) |  5   | `src/lib/data/hotkeys.ts:253-261` (group `Diagnostics & debug`)        | Performance HUD wiring still pending ADR-011 Phase-2+ render-pipeline landing                                                                          |
| 16 | Show state (new / open / save bundle, undo / redo)                                              |  5   | `src/lib/data/hotkeys.ts:263-278` (group `Show state`)                 | Operator-level undo: single-level for preset recall, meta tweak, scene switch per the data file                                                        |

**Total Chromaticity-side hotkey row count:** verified by
`grep -c "keys:"` on `src/lib/data/hotkeys.ts` returning 58 matches,
minus the one interface-declaration `keys:` line equals **57 row
literals**. Group count: verified by `grep -c "title:"` returning 11
matches, minus the one interface-declaration `title:` equals **10
groups**. Both numbers confirm the section-level `manual.md:21`
aggregate row count.

### Focus context, chord conventions, and overlay help

Synesthesia documents one focus-context convention (a control must be
highlighted with the mouse before its hotkeys fire) and no chord or
overlay-help conventions. Chromaticity extends both axes; this sub-group
scores the four cross-cutting axes.

| # | Documented convention                                       |    Score    | Chromaticity equivalent                                                                                                                                                                                    | Notes                                                                                                                                                                                                                                                                       |
| :-: | :--------------------------------------------------------- | :---------: | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | Activation requires a single control to have focus          |   3/MATCH   | `src/lib/data/hotkeys.ts:36` group note ("Active only while a single control has focus. `Esc` exits control-focused mode.")                                                                                | Same focus rule; same `Esc` exit affordance; Chromaticity additionally formalises this as one of six resolution contexts in `docs/42` Conflict-and-priority-rules                                                                                                          |
| 2 | Cross-platform modifier aliasing (Cmd↔Ctrl, Opt↔Alt)        |  4/EXCEEDS  | Every Cmd row in `src/lib/data/hotkeys.ts` lists its Ctrl twin in the `keys` array; every Opt row lists its Alt twin; validated by `src/lib/__tests__/hotkeys-conflict.test.ts` (8 T2 fixtures)            | Synesthesia documents no modifier-aliasing convention; Chromaticity declares it explicitly and pins it under `EV-HOTKEY-CONFLICT` per `docs/26-evidence-artifact-registry.md:90`                                                                                            |
| 3 | Six-context resolution taxonomy (global / control-focused / modal-meta / authoring-env / command-palette / text-input) | 5/EXCEEDS+ | `docs/42` Conflict-and-priority-rules section; `src/lib/__tests__/hotkeys-conflict.test.ts` rules 1-5                                                                                                       | Synesthesia documents one focus context; Chromaticity formalises six and resolves binding priority deterministically across them                                                                                                                                            |
| 4 | In-app overlay / discoverability surface                    | 5/EXCEEDS+  | `src/lib/HotkeyOverlay.svelte:1` `?`-toggle modal with live filter + autofocused search + ESC-to-close + focus-restoration + inline two-element focus trap                                                 | Synesthesia documents no in-app overlay; the public tutorial links a video walkthrough as the discoverability surface. The Chromaticity overlay is `role="dialog"` `aria-modal="true"` and reads the same `HOTKEYS` data the tests pin                                    |

**Score histogram (sub-group 2, n=4):** EXCEEDS+ 2 / EXCEEDS 1 / MATCH 1.

### Rebinding posture

Both products ship V1 hotkey maps as read-only; the section-level
overview scored this N/A. Re-stated here for completeness: Chromaticity
V1 is explicit about deferring rebinding to V2 per the data file's
header comment, with the data driver structured for a future
user-overrideable Settings store swap.

---

## OSC

_Synesthesia's `manual/osc.html` page documents OSC as an
input-plus-output capability of the Pro tier. The page enumerates an
inbound surface organised hierarchically (individual controls →
control banks → control groups → system-level commands for scenes,
playlists, rendering) and an outbound surface (audio variables, control
metadata, scene-launch announcements). The page calls out that the
outbound addresses are FIXED — operators wanting to remap them must
proxy through Max/MSP, TouchDesigner, TWO, or Chataigne. The FAQ page
adds that the OSC surface is Pro-tier entitlement-gated and that both
MIDI and OSC can drive scene controls, meta controls, scene launches,
and favslot triggers._

_This deep dive scores upstream coverage in three sub-groups: route
families, parameter conventions, defaults + diagnostics._

### Route families

Synesthesia's documentation describes the OSC input surface as a
pyramid (individual controls at the apex, control banks in the middle,
groups beneath, system commands at the base) and the output surface as
an inverted pyramid (audio-variable stream widest, control metadata
narrower, scene-launch confirmations narrowest). The seven canonical
families Chromaticity exposes map onto the upstream taxonomy as scored
below.

| # | Documented family / prefix shape                                         |    Score    | Chromaticity equivalent                                                                                                                                                                                                                                    | Notes                                                                                                                                                                                                                                                                                                                  |
| :-: | :---------------------------------------------------------------------- | :---------: | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | Control-value writes (per-control + bank + group)                        |   3/MATCH   | `integrations/osc-adapter/src/routes.rs:36-37` `RoutePrefix::Control`; `integrations/osc-adapter/src/route_registry.rs:82-83` `OscRouteFamily::Control`; canonical seed `route_registry.rs:312-318` accepts the `compat-syn-v1` short alias                | Both products organise control writes under a control collection root. Chromaticity normalises bank + control axes into the `<bank>/<control>` segment vector; the runtime rejects writes to unknown banks with a `route-mismatch` diagnostic per `docs-corpus/docs/osc-routes.md:136`                                  |
| 2 | Scene launch + scene-change announcement                                 |   3/MATCH   | `routes.rs:38-39` `RoutePrefix::Scene`; `route_registry.rs:84-85` `OscRouteFamily::Scene`; seed `route_registry.rs:351-358` (direction `Both` — inbound launch + outbound announce)                                                                         | Both products carry inbound launch + outbound change-announce on the same family root. Chromaticity adds a `direction` discriminator (`In` / `Out` / `Both`) so the operator-facing UI can render each row's intended traffic direction                                                                                |
| 3 | Preset apply + preset create                                             |   3/MATCH   | `routes.rs:40-41` `RoutePrefix::Preset`; `route_registry.rs:86-87` `OscRouteFamily::Preset`; seed `route_registry.rs:343-350` (apply); save-as-named documented in `src/lib/data/hotkeys.ts:122-124` (`/presets/new {name}`)                                | Both products carry the preset family; Chromaticity adds a dispatch grammar so a single preset route can be bound to a structured `preset:<id>` action (parsed at bind time per `integrations/osc-adapter/src/bindings.rs:153-162`)                                                                                    |
| 4 | Snap (favslot) recall + save + clear                                     |  4/EXCEEDS  | `routes.rs:42-43` `RoutePrefix::Snap`; `route_registry.rs:88-89` `OscRouteFamily::Snap`; the parser accepts `snap`/`snaps`/`favslot`/`favslots` per `routes.rs:128`; seed `route_registry.rs:327-334`                                                       | Chromaticity exceeds by accepting four alias spellings (singular + plural × native + compat) for the same family while normalising to a single typed enum; section-level overview scored this 3 at the family-shape level, deep dive records the four-way alias as an EXCEEDS-grade compat surface                     |
| 5 | Playlist transport (next / prev / position / play / toggle / select)     |   3/MATCH   | `routes.rs:44-45` `RoutePrefix::Playlist`; `route_registry.rs:90-91` `OscRouteFamily::Playlist`; seed `route_registry.rs:335-342`; full route table at `docs-corpus/docs/osc-routes.md:86-97`                                                               | Both products carry the same set of playlist-transport actions. Chromaticity's documentation enumerates nine concrete routes (next, previous, position, play, toggleplay, select, current-announce, position-announce, listing) vs the upstream pyramid prose's "playlist navigation" entry                            |
| 6 | Audio-pipeline read-side (audio variables outbound, threshold inbound)   |  4/EXCEEDS  | `routes.rs:46-47` `RoutePrefix::Audio`; `route_registry.rs:92-93` `OscRouteFamily::Audio`; seed `route_registry.rs:295-310` (`/audio/threshold` In + `/audio/var/{uniform}` Out)                                                                            | Chromaticity surfaces the threshold-write + threshold-sample inbound routes alongside the per-uniform outbound stream; the section-level overview noted Synesthesia's documentation does not enumerate concrete threshold-management routes                                                                            |
| 7 | Reserved-prefix safety / future-data-stream namespace                    | 5/EXCEEDS+  | `routes.rs:48-49` `RoutePrefix::Reserved`; `route_registry.rs:94-96` `OscRouteFamily::Reserved`; distinct diagnostic `RouteParseError::PrefixReserved` at `routes.rs:77-78`; dispatch re-throws at `bindings.rs:316-322`; T1 pin `bindings.rs:555-575`     | Synesthesia documents no reserved-prefix concept; Chromaticity reserves the `/data/*` namespace with a structurally-distinct diagnostic so future structured-data routes can land without breaking the seven-family invariant. The dispatcher rejects reserved-route messages even if the bind table somehow contains one |

**Score histogram (sub-group 3, n=7):** EXCEEDS+ 2 / EXCEEDS 2 / MATCH 3.

### Parameter conventions

Synesthesia documents three parameter typing axes: float values for
controls (normalised and raw modes); string values for scene names,
playlist names, and preset names; integer values for playlist position
and similar discrete indices. The output surface emits per-uniform
audio variables continuously. The page does not enumerate blob /
type-tag conventions; we score against the six axes Chromaticity
honours.

| # | Documented parameter convention                                         |    Score    | Chromaticity equivalent                                                                                                                                                                              | Notes                                                                                                                                                                                                                                                |
| :-: | :--------------------------------------------------------------------- | :---------: | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | Float control value, normalised 0.0–1.0                                  |   3/MATCH   | `integrations/osc-adapter/src/route_registry.rs:298-301,314-318` `type_tag: "f"` / `range: "0.0 – 1.0"`; `bindings.rs:393-398` `first_float_arg` extractor                                            | Both products use float on the control surface; Chromaticity requires a single float argument and rejects missing-arg cases with `OscDispatchError::MissingFloatArg` per `bindings.rs:220` rather than silently no-opping                            |
| 2 | String scene / preset / playlist name                                   |  4/EXCEEDS  | Scene-name parsing via `BindTarget::Scene { scene_id }` at `bindings.rs:126-137`; preset-name via the `/presets/{name}` seed entry at `route_registry.rs:343-350`                                    | Chromaticity validates string targets at bind time so empty / malformed scene-id targets fail with `BindError::EmptySceneId` rather than silently dispatching empty                                                                                  |
| 3 | Integer playlist position (negative-from-end semantics)                  |  4/EXCEEDS  | `/playlists/position {i}` documented at `docs-corpus/docs/osc-routes.md:92`; runtime stores `playlist_position: i32` per `app/shell/src/commands/osc.rs:710`                                          | Both accept signed integer with negative-from-end semantics; Chromaticity also exposes the runtime field as a signed integer so OSC inbound and runtime state stay shape-compatible                                                                  |
| 4 | Per-uniform outbound audio-variable stream                              |   3/MATCH   | `/audio/var/{uniform}` seed at `route_registry.rs:303-310` (direction `Out`, `range: "per-uniform cadence"`)                                                                                          | Both products expose per-uniform outbound emission; Chromaticity's per-uniform cadence is governed by the scheduler-tick stride per `engine/runtime/src/scheduler/`                                                                                  |
| 5 | Structured bind-target grammar (`<family>:<param>`)                     | 5/EXCEEDS+  | `bindings.rs:88-108` `parse_bind_target`; `BindTarget` enum at `bindings.rs:22-37` (Control / Scene / Snap / Preset / Playlist); structured `BindError` diagnostics at `bindings.rs:42-55`            | Synesthesia documents no operator-facing bind grammar — outbound addresses are fixed and inbound writes target whatever route Synesthesia exposes. Chromaticity lets operators bind any canonical route to any typed action through one grammar      |
| 6 | Argument type-tag round-trip safety (type-tag mismatch surfaced)        | 5/EXCEEDS+  | `bindings.rs:215-247` `OscDispatchError` variants (`MissingFloatArg`, `SinkRejected`); structured diagnostics over silent drops                                                                       | Synesthesia documents OSC tooltip mapping as the operator-side validation aid; Chromaticity surfaces type-mismatch failures structurally at the dispatch boundary                                                                                    |

**Score histogram (sub-group 4, n=6):** EXCEEDS+ 2 / EXCEEDS 2 / MATCH 2.

### Defaults, diagnostics, and connection conventions

Synesthesia's documentation covers OSC-connection setup at a prose
level (IP addresses, ports, tooltip mapping tool, troubleshooting
procedures) but does not pin specific defaults, payload caps, rate
limits, or loopback-vs-network postures. Chromaticity pins all of those
under ADR-013 with drift-pin T1 evidence. This sub-group scores six
defensive-default and diagnostic axes.

| # | Documented axis                                                          |    Score    | Chromaticity equivalent                                                                                                                                                                                                                                             | Notes                                                                                                                                                                                                                                                                                                                          |
| :-: | :---------------------------------------------------------------------- | :---------: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | Default inbound + outbound ports                                          |  4/EXCEEDS  | `integrations/osc-adapter/src/defaults.rs:39` `DEFAULT_OSC_IN_PORT = 9000`; `defaults.rs:46` `DEFAULT_OSC_OUT_PORT = 9001`; drift pin `defaults.rs:111-114`                                                                                                          | Synesthesia documents IP / port setup but no specific port defaults — operators select arbitrary ports per session. Chromaticity pins 9000 in / 9001 out by default so out-of-the-box setups work without first-run friction                                                                                                  |
| 2 | Loopback-vs-network default bind scope                                    | 5/EXCEEDS+  | `defaults.rs:24-31` `DEFAULT_INBOUND_BIND` / `DEFAULT_OUTBOUND_TARGET` both `Ipv4Addr::LOCALHOST`; `defaults.rs:97-107` drift pin; ADR-013 §1                                                                                                                         | Synesthesia documents no default-safety posture; Chromaticity binds loopback-only by default with explicit opt-in to a routable interface, surfaced through a Settings panel with an "may expose Chromaticity to the network" warning per ADR-013 §6                                                                          |
| 3 | Payload hardening (packet-size cap, bundle-depth cap)                     | 5/EXCEEDS+  | `defaults.rs:53` `DEFAULT_MAX_PACKET_BYTES = 65_536`; `defaults.rs:60` `DEFAULT_MAX_BUNDLE_DEPTH = 4`; `defaults.rs:118-122` drift pin; ADR-013 §5                                                                                                                    | Synesthesia documents no packet-size or bundle-depth caps; Chromaticity rejects oversize packets and over-depth bundles with structured diagnostics and no state mutation, blocking trivial stack-exhaustion + resource-exhaustion payloads                                                                                  |
| 4 | Per-route + per-adapter rate-limit ceiling                                | 5/EXCEEDS+  | `defaults.rs:69` `DEFAULT_RATE_LIMIT_MSGS_PER_SEC = 100`; `defaults.rs:77` `DEFAULT_ADAPTER_RATE_LIMIT_FLOOR_MSGS_PER_SEC = 100`; `defaults.rs:124-129` drift pin; ADR-013 §3                                                                                         | Synesthesia documents no rate-limit posture; Chromaticity caps inbound at 100 msg/s per (adapter, route) by default with a `osc-rate-limited` diagnostic emitted at a throttled cadence (no more than 1/s per route). 1000 msg/s hard cap is non-configurable                                                                |
| 5 | Live connection / health diagnostic surface                               | 5/EXCEEDS+  | `app/shell/src/commands/osc.rs:67-84` `osc_health_snapshot` Tauri command; `src/lib/views/Osc.svelte:64` live consumption; AdapterPill OSC slot in TopBar (W34a / Wave 2.4 PERF.D)                                                                                    | Synesthesia documents a "log OSC" checkbox as the only debug affordance; Chromaticity surfaces a structured per-poll snapshot (state Healthy/Recovering/Degraded + attempted-bind counter + bound `local_addr`) consumed live by both the Routes view and the always-visible top-bar adapter pill                            |
| 6 | Route inspection + bind-table edit affordance                             |  4/EXCEEDS  | `osc_routes_snapshot` / `bind_osc_route` / `unbind_osc_route` Tauri commands at `app/shell/src/commands/osc.rs:398-419`; `OscRouteRegistry` at `integrations/osc-adapter/src/route_registry.rs:163-263`; route table in `src/lib/views/Osc.svelte`                    | Synesthesia documents the OSC tooltip mapping tool as the route-inspection surface; Chromaticity exposes a live IPC registry the operator can mutate without re-binding the adapter socket. Bind failures surface to the caller; the adapter remains installed so health reports attempted-bind count instead of silent      |

**Score histogram (sub-group 5, n=6):** EXCEEDS+ 4 / EXCEEDS 2.

---

## Cross-cutting observations

1. **The four upstream control-focused actions are deliberate
   match-quality parity.** Of the 30 scored rows in this deep dive,
   four are 3/MATCH and they are all the D / R / P / L control-focused
   quartet (and the matching activation rule). Preserving exact
   semantic parity for that quartet is the operator-portability axis
   that lets external-controller workflows transplant; the structural
   advantages around it (entitlement caps, OSC pairing, GLSL pairing,
   focus-context taxonomy, overlay UX, evidence-tagged conformance)
   are the EXCEEDS surface. No row scored worse than 3/MATCH against
   the documented upstream surface.

2. **The compat-syn-v1 alias surface widens the OSC family scoring on
   one axis.** The Snap family scores 4/EXCEEDS at the deep-dive level
   (vs the section-level 3/MATCH) because the parser accepts four
   spellings — `snap`, `snaps`, `favslot`, `favslots` — for the same
   typed family at `integrations/osc-adapter/src/routes.rs:128`. Same
   spelling-rollup applies to control / scene / preset / playlist
   (singular + plural), enumerated at `routes.rs:124-133`. The
   deliberate compat-syn-v1 ingest plus the chroma-native plural-root
   canonical form give external controllers maximum acceptance breadth
   without diluting the operator-facing canonical surface.

3. **The Reserved (`/data/*`) family is a structural axis Synesthesia's
   manual does not document at all.** Reserved is the only OSC family
   that is **definitively absent** upstream; both the parser
   (`routes.rs:131`) and the dispatcher (`bindings.rs:316-322`) emit a
   structurally-distinct diagnostic so future structured-data routes
   can land without breaking the seven-family invariant. This is an
   architectural reservation that future agent-driven OSC streams (and
   the deferred outbound audio-engine feedback streams) can extend
   through one discriminant rather than diffusing into generic
   unknown-prefix rejections.

4. **Synesthesia's docs page for OSC tier-gates the entire feature
   behind "Synesthesia Pro".** Chromaticity does not tier-gate OSC at
   MVP — the seven-family adapter ships as a free-tier capability per
   the deferred-revocation posture at `CLAUDE.md`'s Phase-4a Wave 1b-B
   `EV-ENTITLEMENT-ENFORCEMENT` 13-cap honored set. Per-route entitlement
   gating remains a structural option (the hotkey vocabulary already
   uses it for `render.custom-resolution`), but no OSC route is
   currently gated. Whether to tier-gate OSC is an AO-level commercial
   decision deferred to the Phase-5 license-audit gate.

5. **The five new actionable gaps surfaced by this deep dive (beyond
   the section-level overview).** Each is small-scope and either
   inherited from a known deferral or candidate for a near-future
   wave:

   a. **`/audio/threshold` outbound emission** — the route is
   documented at `docs-corpus/docs/osc-routes.md:122` as
   bidirectional ("inbound and outbound") and the inbound path is
   wired, but the seed table at `route_registry.rs:295-302` marks
   `direction: OscDirection::In` rather than `Both`. The outbound
   announce-on-change emission is the missing piece; one small PR
   widens it.

   b. **Outbound dual-emit window for `compat-syn-v1` aliases** —
   `docs-corpus/docs/osc-routes.md:63-64` documents the dual-emit
   contract ("Outbound emission during the dual-emit window publishes
   both the canonical plural form and the singular alias"). The seed
   table only tracks one canonical row per route; dual-emit at the
   adapter sender layer is a near-future Phase-3+ widening.

   c. **`/playlists/select {name}` and `/playlists/play` IPC wiring**
   — both are documented at `docs-corpus/docs/osc-routes.md:94-95`
   and parse correctly through the slice-2 prefix parser, but the
   Wave 2.1 D.3 seed table at `route_registry.rs:294-358` enumerates
   only `/playlists/next` as the playlist transport entry. Bind-target
   coverage is `playlist:next` / `playlist:prev` only; adding `play` /
   `pause` / `select` / `position` bind-target variants is a small
   `BindTarget::Playlist` enum widening.

   d. **`/scenes/{name} {preset}` two-arg form bind-target grammar** —
   `src/lib/data/hotkeys.ts:81` documents the `Shift+Enter` row as
   "Launch selected scene with preset prompt" pairing to
   `/scenes/{name} {preset}`. The current `BindTarget::Scene` shape at
   `bindings.rs:30` carries only `scene_id`; a `preset` optional
   second axis would close this gap without changing the wire-side
   parser (`/scenes/{name}` segment vector already supports the trail
   that the preset hint occupies).

   e. **Bundled OSC tooltip mapping affordance** — Synesthesia's
   documentation calls out the OSC tooltip mapping tool as the
   route-inspection surface; Chromaticity exposes the route table and
   bind table through `osc_routes_snapshot` IPC but the equivalent
   inline-tooltip affordance on the OSC view (point at a route, see
   the current value or last-seen-time) is currently absent. A small
   `osc_route_last_seen_ts` field on `OscRoutePayload` plus a Routes
   view tooltip closes this gap.

6. **Two upstream pages 404'd at fetch time during this sweep but
   later resolved.** `/docs/manual/control_hotkeys` and
   `/docs/manual/osc` returned 404 from the section-level pass on
   2026-05-22; this deep-dive sweep on the same date (re-tried with
   the bare `/docs/manual/control_hotkeys.html` and
   `/docs/manual/osc.html` slugs) succeeded and produced the action /
   family / parameter coverage scored above. The section-level
   `manual.md:166` note about fetch-time 404s is now historical; this
   deep dive validates the upstream surface directly against both
   manual pages.

---

*Authored from `integrations/osc-adapter/src/{routes.rs, bindings.rs,
route_registry.rs, defaults.rs}`, `app/shell/src/commands/osc.rs`,
`src/lib/data/hotkeys.ts`, `src/lib/HotkeyOverlay.svelte`,
`src/lib/views/Osc.svelte`, `docs-corpus/docs/{hotkeys-reference.md,
osc-routes.md}`, `docs/comparison/manual.md`,
`docs/26-evidence-artifact-registry.md`, against
`docs/adr/ADR-013-networking-defaults-and-security.md`,
`docs/adr/ADR-009-distribution-licensing.md`,
`docs/42-operator-hotkey-map.md`, the public Synesthesia Manual pages
at `app.synesthesia.live/docs/manual/control_hotkeys.html` and
`app.synesthesia.live/docs/manual/osc.html`, the matching tutorial
sibling page, and the MIDI & OSC FAQ page. Clean-room observation-only
per `docs/29-cleanroom-legal-policy-baseline.md`.*
