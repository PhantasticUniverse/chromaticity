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
export { default as Fader } from './Fader.svelte';
export { default as Knob } from './Knob.svelte';
export { default as Toggle } from './Toggle.svelte';
export { default as Swatch } from './Swatch.svelte';
export { default as ControlGroup } from './ControlGroup.svelte';
export { default as CueTransitionCurve } from './CueTransitionCurve.svelte';
export { default as TestCard } from './TestCard.svelte';
export { default as BpmRing } from './BpmRing.svelte';
export { default as BeatDots } from './BeatDots.svelte';
export { default as CrossFader } from './CrossFader.svelte';
export { default as VuStereo } from './VuStereo.svelte';
export { default as CuePad } from './CuePad.svelte';
export { default as AudioReactivity } from './AudioReactivity.svelte';
export { default as SnapBar } from './SnapBar.svelte';
export { DEFAULT_SNAP_SLOTS, type SnapSlot } from './snap-bar.fixtures';

// Per-cell shape for `CuePad`. Lifted into the barrel so consumers can
// type their `cells` arrays without reaching into the component file
// (Svelte's `*.svelte` module shim only re-exports the default).
export interface CuePadCell {
  readonly state?: 'idle' | 'armed' | 'playing';
  readonly hue?: number;
  readonly label?: string;
}
