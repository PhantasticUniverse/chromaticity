/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

// Wave-8 widget barrel. Re-exports the visual control primitives ported
// from the chromaticity-planning bundle. W9 will wire pointer / IPC
// behaviour without changing this module's public shape.
//
// Wave-13 PR2 adds `CueTransitionCurve`, the bezier transition-curve
// editor consumed by the Playlist v2 view (W13 PR3).
export { default as Fader } from './Fader.svelte';
export { default as Knob } from './Knob.svelte';
export { default as Toggle } from './Toggle.svelte';
export { default as Swatch } from './Swatch.svelte';
export { default as ControlGroup } from './ControlGroup.svelte';
export { default as CueTransitionCurve } from './CueTransitionCurve.svelte';
