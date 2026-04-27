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
