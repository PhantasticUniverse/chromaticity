/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Per-view empty-state copy library.
 *
 * Ported from `chromaticity-planning/ui-predev-bundle-v0.2-beta/empty_states.jsx`.
 * Titles and body prose are taken verbatim from that bundle; the action
 * `label` strings likewise mirror the bundle's primary / secondary actions
 * (the JSX exposes 2-3 actions per view, this library keeps the first two
 * since W8 only needs a primary / optional-secondary surface).
 *
 * Action handlers are W9+ wiring; this module ships copy + types only.
 * The `EmptyState` Svelte primitive consumes these constants and renders
 * the action labels inside `disabled` buttons until W9 routes them.
 */

export interface EmptyStateAction {
  readonly label: string;
  readonly hint?: string;
}

export interface EmptyStateCopy {
  readonly title: string;
  readonly body: string;
  readonly primaryAction?: EmptyStateAction;
  readonly secondaryAction?: EmptyStateAction;
}

export const EMPTY_LIBRARY: EmptyStateCopy = {
  title: 'No scenes yet',
  body: 'Your library is where every scene lives — built-ins, Shadertoy conversions, .chroma packages, and your own Chromaticity drafts. Pull down the built-in pack to get going, or start from scratch.',
  primaryAction: { label: 'Install Built-Ins' },
  secondaryAction: { label: 'New Scene' },
};

export const EMPTY_PLAYLIST: EmptyStateCopy = {
  title: 'No playlists',
  body: 'Build a set by grouping scenes into a playlist. Cues fire in order with per-cue duration, crossfade curve, and control preset.',
  primaryAction: { label: 'New Playlist' },
  secondaryAction: { label: 'Import Set…' },
};

export const EMPTY_PLAYLIST_CUES: EmptyStateCopy = {
  title: 'This playlist has no cues',
  body: 'Add scenes from your Library — each cue can carry its own duration, crossfade curve, and control preset.',
  primaryAction: { label: 'Add Scene' },
  secondaryAction: { label: 'Go to Library' },
};

export const EMPTY_MIDI_DEVICES: EmptyStateCopy = {
  title: 'No MIDI devices detected',
  body: 'Connect a MIDI controller — a Launchpad, APC40, fader bank, or any class-compliant USB device — and it will show up here automatically.',
  primaryAction: { label: 'Rescan Devices' },
  secondaryAction: { label: 'Open MIDI Settings' },
};

export const EMPTY_OSC: EmptyStateCopy = {
  title: 'No OSC port bound',
  body: 'Receive OSC from any device on your network — iPad, TouchOSC, Ableton, QLab, or another Chromaticity instance. Bind a port to start routing messages.',
  primaryAction: { label: 'Bind Port 7000' },
  secondaryAction: { label: 'Choose Port…' },
};

/*
 * Wave-16 PR2 stub copy for one remaining newly-routed view in the
 * v0.2 sidebar redesign. The remaining entry stands in for the
 * eventual W17+ port of the bundle's `audio.jsx` per-view JSX. Body
 * prose names the canonical bundle source path so the eventual port
 * lands deterministically. Action fields stay `undefined` — this is a
 * placeholder view, not an entrypoint to disabled actions.
 *
 * Three of the original four W16 PR2 stubs have shipped their full
 * ports and the corresponding entries have been removed:
 *   - `EMPTY_WELCOME` removed in W16 PR3 alongside the `Welcome.svelte`
 *     port of `views/onboard.jsx`.
 *   - `EMPTY_LICENSE` removed in W17 PR2 alongside the `License.svelte`
 *     port of `v02_supplement.jsx::LicenseView`.
 *   - `EMPTY_SOURCES` removed in W17 PR1 alongside the `Sources.svelte`
 *     port of `views/media.jsx`. All three retirements followed the
 *     W16 PR3 "sole-consumer dead-code policy" precedent.
 */

export const EMPTY_AUDIO_UNIFORMS: EmptyStateCopy = {
  title: 'Audio Uniforms view coming in W17+',
  body: 'Live audio analysis monitor — frequency bands + time uniforms + BPM ring (per v0.2 audio.jsx). Canvas visualizers gate on the audio adapter Phase-3+ wiring.',
};
