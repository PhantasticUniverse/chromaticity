# UI/UX Bundle Open-Question Routing and Closure Ledger

Routing of the six open design questions parked in `ui-predev-bundle/HANDOFF.md` §"Open design questions for the native team" to accountable owners and target closure artifacts per `23-decision-ownership-matrix.md`.

Closures must land in the cited ADR or FR, **not** as bundle side-notes. This document is the tracking ledger until all six are closed.

## Ledger

| # | Question | Severity | Accountable | Responsible Workcell | Closure artifact | Phase-0 dependency | Status |
|---|---|---|---|---|---|---|---|
| Q1 | Timecode semantics - does it start at session-open or app-boot? Bundle currently increments freely. | P1 | Xavier Butler | AI Runtime Workcell | `adr/ADR-001-runtime-model.md` amendment (or a dedicated Session Lifecycle note under `04-runtime-architecture.md`) | Not a Phase-0 blocker; must be resolved before `EV-UX-HUD-CONTRACT` can close to `Pass` because the HUD timecode copy depends on the semantics. | Open; routed |
| Q2 | Render-pipeline shader-wrapper format - bundle's SSF Inspector assumes ISF-style JSON INPUTS. Chromaticity rejects ISF and ships `.chroma` packages. | P0 | Xavier Butler | AI Scene Workcell | `37-authoring-environment-spec.md` §3 "Input schema" edit + `36-reactive-symbol-catalog.md` cross-reference; SSF Inspector retarget is in bundle v0.2-beta scope per `44-ux-bundle-gap-audit.md` A-01 / S-01 | Phase-0 blocker for `EV-UX-AUTHORING-CONTRACT` and `EV-SYMBOL-CATALOG`. | Open; routed |
| Q3 | Output-routing capability per-platform - bundle references `DISP 2`, `Syphon`, `NDI`, `Record`. macOS ships Syphon; Windows ships Spout; NDI is cross-platform but license-encumbered. | P0 | Xavier Butler | AI Integration Workcell | `adr/ADR-013-networking-defaults-and-security.md` §"Output routing" section (new) or a new lightweight ADR if the scope justifies. `30-mvp-support-matrix.md` cross-reference to document per-GPU-class capability. | Phase-0 blocker for `EV-UX-HUD-CONTRACT` (H-10) and the "Output routing" supplement tile commissioned in `49-ux-bundle-supplement-scope.md`. | Open; routed |
| Q4 | Screenshot capture - bundle wires `onCapture` to a mock frame. Real native hook is a wgpu readback; EXR format in the picker for HDR users needs feasibility check. | P1 | Xavier Butler | AI Runtime Workcell (wgpu owner) | `adr/ADR-011-rendering-backend.md` §"Readback path" addendum; EXR decision recorded in the same ADR. MVP guidance: ship PNG + JPEG only; EXR deferred to V2 pending wide-gamut / HDR scope clarification in NFR-12 V2 work. | Not a Phase-0 blocker; affects `EV-UX-HUD-CONTRACT` screenshot-modal row in v0.2-beta. | Open; routed |
| Q5 | MIDI-learn parameter-graph binding surface - the flow is modeled; what does `learn -> parameter` actually resolve to? | P0 | Xavier Butler | AI Integration Workcell + AI Scene Workcell | `03-product-requirements.md` FR-22 refinement (parameter-graph resolution rule set) and `07-control-state-presets.md` amendment describing the binding surface. WP-5 outputs feed the resolution. | Phase-0 blocker for `EV-UX-MIDI-SETTINGS-CONTRACT` (M-11) - mode select values must match the canonical enum once resolved. | Open; routed |
| Q6 | Record destination / codec - REC button is decorative. Container and codec (MOV / MP4 / ProRes) and "record stage output vs include UI" unknown. | P1 | Xavier Butler | AI Integration Workcell | `adr/ADR-013-networking-defaults-and-security.md` (new §"Recording") **or** if recording is scoped out for MVP: explicit V2 deferral in `11-roadmap-phases.md` and visible "record disabled at MVP" affordance in bundle v0.2-beta. | Not a Phase-0 blocker if scoped to V2. If V1 ships with REC, this blocks `EV-UX-HUD-CONTRACT`. | Open; routed |

## Default recommendations (strawman pending Accountable Owner sign-off)

These are strawman answers so implementation is not blocked longer than necessary. Each is pre-ratified by the Responsible Workcell; Accountable Owner confirms or overrides at a single sitting.

### Q1 - Timecode semantics

**Recommendation.** Session-open. Reset to `00:00:00.000` when the operator creates a new show or opens an existing `.chromaShow` bundle. App-boot is not meaningful for a show-session metric. Display in monospace with tabular numerals per `HANDOFF.md` §"Key conventions".

Rationale: operators tour with pre-built shows; they want session-scoped timecode for setlist compliance, not wall-clock or uptime. Session-open is consistent with DAW convention.

Target artifact: new section in `04-runtime-architecture.md` titled "Session lifecycle and timecode semantics"; FR-none (timecode was not previously lifted into FR text).

### Q2 - Shader-wrapper format

**Recommendation.** Drop SSF-style JSON INPUTS entirely from the Inspector. Use `36-reactive-symbol-catalog.md` as the single source of the symbol surface; `.chroma` package manifest defines per-scene input declarations per `38-scene-library-and-import-spec.md` §"`.chroma` layout". Imports of Shadertoy source convert on intake (Shadertoy accepted; ISF rejected).

Rationale: already the normative decision in Round 5 closure; bundle is out of date. No ADR change needed; edit is to the bundle, not the plan. SSF Inspector renames to "Scene Inputs Inspector" to avoid legacy-terminology carry.

Target artifact: bundle v0.2-beta revision (authored by UX team) + cross-reference line in `37-authoring-environment-spec.md` §3 clarifying the Inspector contract.

### Q3 - Output routing

**Recommendation.** MVP ships with the following outputs per platform:

| Output class | macOS | Windows | Capability-gated | Notes |
|---|---|---|---|---|
| Primary display (fullscreen) | yes | yes | free | `42-operator-hotkey-map.md` §4 `F` / `Shift+F` / `Cmd+Shift+F`. |
| Secondary display (multi-monitor) | yes | yes | free | Same. |
| Syphon | yes | no | free | macOS only; license-clear. |
| Spout | no | yes | free | Windows only; license-clear. |
| NDI | **V2** | **V2** | post-MVP | NDI SDK license review required (`29-cleanroom-legal-policy-baseline.md`); defer to V2. |
| Record (disk) | **V2** | **V2** | post-MVP | See Q6. |

Rationale: Syphon / Spout cover the 90% use case for touring visual performers (feeding VJ software, Resolume, mixers). NDI + Record are V2 scope clarifications that de-risk the legal-gate closure timeline.

Target artifact: `adr/ADR-013-networking-defaults-and-security.md` new §"Output routing defaults"; `11-roadmap-phases.md` V2 scope bullet.

### Q4 - Screenshot capture and EXR

**Recommendation.** MVP ships PNG + JPEG only. Format picker in `ui-predev-bundle/screenshot_modal.jsx` drops EXR for v0.2-beta. Capture hook in native code is a wgpu texture readback at current stage resolution, written via `image` crate. Destination options at MVP: folder, clipboard (macOS + Windows native), library (Scene library attach as cover thumbnail if current scene is selected).

Rationale: EXR HDR-grade capture requires NFR-12 V2 wide-gamut scope; V1 is sRGB. Shipping EXR at V1 forces premature color-management decisions.

Target artifact: `adr/ADR-011-rendering-backend.md` new §"Stage readback path" (two paragraphs); bundle v0.2-beta updates `screenshot_modal.jsx` format picker.

### Q5 - MIDI-learn binding surface

**Recommendation.** Learn resolves to a parameter-graph node identified by:

```
route := {
  scope: "global" | "scene",
  scene_id: ScenePackageId | null,   // null when scope=global
  bank: "controls" | "meta" | "media" | "snap" | "start_scene" | "device",
  name: string,                       // control name in bank
  variant: "raw" | "normalized"       // default: "normalized"
}
```

Binding is one-to-many (a single MIDI CC can drive multiple routes) but one-to-one for a given `(scope, scene_id, bank, name)` - re-learning overwrites. Persistence: global scope saves to app data; scene scope saves into the `.chroma` package's `mappings.json` per `38-scene-library-and-import-spec.md` §".chroma layout".

Rationale: mirrors the OSC route surface in `08-io-integrations.md` - same resolution, different transport.

Target artifact: `03-product-requirements.md` FR-22 refinement (MIDI learn resolution subsection); `07-control-state-presets.md` new §"MIDI / OSC binding surface".

### Q6 - Record destination and codec

**Recommendation.** Defer to V2 per Q3 table. V1 ships with the REC button visibly disabled + tooltip "Recording available in V2 - use Syphon / Spout to record externally." This preserves the topbar UI affordance for V2 without wiring.

Rationale: codec selection (ProRes on macOS vs h.264/h.265 on both) has license implications (h.265 patent pool) and engineering cost (GPU-side encode path) that are not MVP-critical; external recording via Syphon / Spout covers the use case.

Target artifact: `11-roadmap-phases.md` V2 scope bullet; bundle v0.2-beta disables REC visually + adds the tooltip copy.

## Ratification process

1. Accountable Owner reviews this ledger at a single sitting and either:
   - confirms each recommendation verbatim (single review pass), or
   - overrides with a revised recommendation (must record in the `Status` column as "Decision: ..." before ADR / FR edit lands).
2. Each closure artifact is produced under `43-development-standards.md` §14 ADR amendment process (for ADR edits) or the normal FR refinement process (for `03-product-requirements.md` edits).
3. Bundle v0.2-beta landing waits on Q2 closure (P0) only; Q1 / Q4 / Q5 / Q6 can follow in parallel tracks.
4. This ledger is updated with the actual closure artifact path + date + reviewer once each decision is ratified.

## Traceability

- `ui-predev-bundle/HANDOFF.md` §"Open design questions for the native team"
- `23-decision-ownership-matrix.md` §Decision Matrix
- `03-product-requirements.md` FR-22
- `04-runtime-architecture.md`
- `07-control-state-presets.md`
- `adr/ADR-001-runtime-model.md`
- `adr/ADR-011-rendering-backend.md`
- `adr/ADR-013-networking-defaults-and-security.md`
- `11-roadmap-phases.md` (for V2 scope bullets)
- `38-scene-library-and-import-spec.md`
- `44-ux-bundle-gap-audit.md` (A-01, S-01, H-10, M-11)
