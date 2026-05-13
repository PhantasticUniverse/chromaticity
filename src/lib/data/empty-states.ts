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
 *
 * Historical: the W16 PR2 sidebar redesign added four stub copy entries
 * (`EMPTY_WELCOME`, `EMPTY_SOURCES`, `EMPTY_AUDIO_UNIFORMS`,
 * `EMPTY_LICENSE`) for the newly-routed v0.2 placeholder views. All four
 * have since been retired alongside their full ports, per the W16 PR3
 * "sole-consumer dead-code policy" precedent:
 *   - `EMPTY_WELCOME` retired in W16 PR3 (`Welcome.svelte` from
 *     `views/onboard.jsx`).
 *   - `EMPTY_LICENSE` retired in W17 PR2 (`License.svelte` from
 *     `v02_supplement.jsx::LicenseView`).
 *   - `EMPTY_SOURCES` retired in W17 PR1 (`Sources.svelte` from
 *     `views/media.jsx`).
 *   - `EMPTY_AUDIO_UNIFORMS` retired in W17 PR3 (`AudioUniforms.svelte`
 *     from `views/audio.jsx`).
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
  /** Optional small monospace footnote rendered below the actions
   * (mirrors the bundle's `meta` slot on `<EmptyState>`). Used by
   * Library to surface the supported-formats line per
   * `empty_states.jsx::EmptyLibrary` line 55. */
  readonly meta?: string;
}

export const EMPTY_LIBRARY: EmptyStateCopy = {
  title: 'No scenes yet',
  body: 'Your library is where every scene lives — built-ins, Shadertoy conversions, .chroma packages, and your own Chromaticity drafts. Pull down the built-in pack to get going, or start from scratch.',
  primaryAction: { label: 'Install Built-Ins' },
  secondaryAction: { label: 'New Scene' },
  meta: 'Supported: .chroma · .chromaPack · .synScene conversion · Shadertoy URL · ISF reject',
};

/** Wave 2.5 LIB.E — filter-excludes-everything empty state. Reuses
 * the EmptyState primitive with copy tailored to the "library has
 * scenes but the current search query or active category filter chips
 * exclude all of them" case. Mirrors the bundle's per-view empty
 * pattern; the bundle itself ships only EmptyLibrary (zero-scenes)
 * since the React surface kept search-empty inline, but the
 * supported-formats meta line ports across both cases so the operator
 * can see the format vocabulary at any empty grid. */
export const EMPTY_LIBRARY_FILTERED: EmptyStateCopy = {
  title: 'No scenes match',
  body: 'Clear the search query or deselect category chips to see your full library.',
  meta: 'Supported: .chroma · .chromaPack · .synScene conversion · Shadertoy URL · ISF reject',
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
