/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

// Wave-8 widget barrel. Re-exports the visual control primitives ported
// from the chromaticity-planning bundle. W9 will wire pointer / IPC
// behaviour without changing this module's public shape.
//
// Wave-13 PR2 adds `CueTransitionCurve`, the bezier transition-curve
// editor consumed by the Playlist v2 view (W13 PR3).
//
// Wave-21 PR2 adds `TestCard`, the static SVG NTSC SMPTE calibration
// pattern consumed by the Perform view's Preview hero when the Test
// Card MediaStrip card is active.
//
// Wave-23 PR1 adds `BpmRing` and `BeatDots`, the two synthetic-tempo
// visualizers ported from the bundle's `components.jsx` for the
// AudioUniforms hero block. Both render frozen-frame synthetic BPM —
// no IPC, no audio-pipeline data — and short-circuit to a static
// frame under `prefers-reduced-motion: reduce`.
//
// Wave-23 PR2 adds `CrossFader` (horizontal A↔B crossfader with pointer-
// drag; frozen-frame, no IPC) and `VuStereo` (synthetic stereo level
// meters with peak-hold decay) — both consumed by the Perform view's
// SceneControls and MetaControls panels respectively.
//
// Wave-23 PR3 adds `CuePad`, the rows × cols clickable pad-grid
// primitive consumed by the MIDI view's Virtual Pad mirror panel.
//
// Wave-23 PR4 refactors `CuePad` from a flat `CuePadState[]` to the
// bundle-faithful `CuePadCell` object shape (`{ state, hue, label }`)
// so each pad can carry its own per-pad hue (matching the Launchpad X
// rainbow-LED palette in the wireframe), and drops the debug `[r,c]`
// coord overlay that the W23 PR3 brief invented.
//
// Wave-25 PR2 adds `AudioReactivity`, the 4-band-tile + 48-bar FFT +
// tempo-strip composite consumed by the Perform view's middle column
// per the bundle's `views/perform.jsx` `AudioReactivity` JSX (lines
// 198-261). Inline-SVG animation closes the W18 gap where the panel
// was skipped wholesale because the bundle uses canvas.
//
// Wave-27 PR1 adds `SnapBar`, the 8-slot quick-access snap bar ported
// from the bundle's `extras.jsx` `SnapBar` JSX (lines 14-46). Mounted
// in `AppShell` between the TopBar and the routed view, gated on
// `routeStore.current === 'perform'` per the bundle's adoption site
// in `Chromaticity.html` (`{showSnapBar && <SnapBar … />}`). Re-exports
// `DEFAULT_SNAP_SLOTS` and the `SnapSlot` type so consumers can pass
// their own slot fixtures without reaching into the component file.
//
// Wave-27 PR2 adds `OutputRoutingTile`, the bordered display + Syphon /
// Spout / NDI / Record status panel consumed by the Settings →
// Graphics/Output section. Per-platform Syphon-on-mac / Spout-on-win
// row variant comes from the existing `stores/platform.ts` detection;
// row state is fixture per the frozen-frame contract.
//
// Wave-28 PR1 adds `DegradedBanner`, the warning / error banner ported
// from the bundle's `v02_supplement.jsx` `DegradedBanner` JSX (lines
// 47-67). Mounted in `AppShell` between the TopBar and the routed view
// (or the W27 PR1 SnapBar on Perform), surfacing fixture content per
// the bundle's `Chromaticity.html` adoption site (line 263). Real
// adapter-health probing is `EV-INTEGRATION-MATRIX` scope.
//
// Wave-29 PR1 adds `CommandPalette`, the Cmd/Ctrl+K command palette
// overlay ported from the bundle's `extras.jsx` `CommandPalette` JSX
// (lines 124-166). Mounted in `AppShell` at z-index 1010 (one layer
// above HotkeyOverlay) with a window-level keydown handler that toggles
// the palette on Cmd+K (macOS) / Ctrl+K (Windows / Linux). Re-exports
// `DEFAULT_COMMANDS` and the `CommandPaletteAction` interface so
// AppShell can attach `onSelect` route-dispatch stubs without reaching
// into the component file.
export { default as Fader } from './Fader.svelte';
export { default as Knob } from './Knob.svelte';
export { default as Toggle } from './Toggle.svelte';
export { default as Swatch } from './Swatch.svelte';
// Wave-32: xy / bang / dropdown round out the W8 widget primitive set
// per the bundle's `chroma_inspector.jsx::ChromaCtrlBody` switch
// statement. Inputs.svelte uses these to replace the W10 placeholder
// pill that previously rendered for non-mappable control types.
export { default as Xy } from './Xy.svelte';
export { default as Bang } from './Bang.svelte';
export { default as Dropdown } from './Dropdown.svelte';
export { default as ControlGroup } from './ControlGroup.svelte';
export { default as CueTransitionCurve } from './CueTransitionCurve.svelte';
export { default as TestCard } from './TestCard.svelte';
export { default as BpmRing } from './BpmRing.svelte';
export { default as BeatDots } from './BeatDots.svelte';
export { default as CrossFader } from './CrossFader.svelte';
export { default as VuStereo } from './VuStereo.svelte';
export { default as Meter } from './Meter.svelte';
export { default as CuePad } from './CuePad.svelte';
export { default as AudioReactivity } from './AudioReactivity.svelte';
export { default as SnapBar } from './SnapBar.svelte';
export { DEFAULT_SNAP_SLOTS, type SnapSlot } from './snap-bar.fixtures';
export { default as OutputRoutingTile } from './OutputRoutingTile.svelte';
export { default as DegradedBanner } from './DegradedBanner.svelte';
export { default as CommandPalette } from './CommandPalette.svelte';
// Wave-33a adds `WatermarkOverlay`, the bottom-right "Chromaticity"
// wordmark + ring overlay ported from the bundle's `v02_supplement.jsx`
// `WatermarkOverlay` JSX (lines 277-286). Display-only chrome; the
// operative watermark is drawn by the Rust rendering backend on the
// scene canvas per `adr/ADR-009-activation-and-entitlements.md`. The
// UI overlay here is the bundle's dev-time visual mock of the
// "watermark visible" state; AppShell mounts it gated on a hardcoded
// fixture today (W33b capabilities-store wiring inverts it from
// `render.watermark.off`, real verifier IPC lands in Phase-3+/4a).
export { default as WatermarkOverlay } from './WatermarkOverlay.svelte';
// Wave-34a adds `AdapterPill`, the six-state adapter-health pill
// ported from `v02_supplement.jsx` lines 20-44. The bundle mounts
// three pills (OUT / MIDI / OSC) in the TopBar HUD per
// `Chromaticity.html` lines 228-230; this port replaces the W15 PR1
// inline `top-bar__pill` placeholder markup with the bundle-faithful
// component. State vocabulary is the bundle's literal six
// (`off / warming / ok / rate-limited / disconnected / blocked`); the
// W4 `OscHealthState` (`Healthy` / `Recovering` / `Degraded`) maps
// onto a subset at the call-site. Real per-adapter health wiring is
// `EV-INTEGRATION-MATRIX` scope.
export { default as AdapterPill } from './AdapterPill.svelte';
export { type AdapterPillState } from './adapter-pill.types';
// Wave-34b adds `LockedRow`, the canonical "feature-locked
// affordance" treatment ported from `v02_supplement.jsx` lines 171-
// 188. Per CLAUDE.md MVP entitlement enforcement is watermark-only
// (only `render.watermark.off` is evaluated at runtime in MVP); the
// LockedRow primitive ships ready for post-MVP SKU rollouts to gate
// non-watermark capabilities. Call-site pattern: conditional render
// driven by `capabilitiesStore.has(capabilityId)` per the W33b
// store; the widget itself does not check the store, it is purely
// the visual treatment for an already-decided locked state.
export { default as LockedRow } from './LockedRow.svelte';
export {
  DEFAULT_COMMAND_PALETTE_ACTIONS,
  type CommandPaletteAction,
} from './command-palette.fixtures';

// Per-cell shape for `CuePad`. Lifted into the barrel so consumers can
// type their `cells` arrays without reaching into the component file
// (Svelte's `*.svelte` module shim only re-exports the default).
export interface CuePadCell {
  readonly state?: 'idle' | 'armed' | 'playing';
  readonly hue?: number;
  readonly label?: string;
}
