/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * Wave-21 PR4 global console drawer fixtures — frozen-frame log entries
 * that populate the app-level Console drawer until the real log-stream
 * subscription lands in Phase-3+/4a (EV-OBSERVABILITY).
 *
 * The 30-row corpus covers every severity chip the drawer surfaces
 * (scan / load / fetch / compile / warn / error) plus a handful of
 * `info` echoes, and spans seven distinct source labels so the
 * drawer's source dropdown demonstrates non-trivial filtering on
 * first paint. Timestamps are HH:MM:SS in a single 13:24-13:32
 * window — the format matches the v0.2 bundle's
 * `views/import.jsx::IMPORT_CONSOLE` precedent.
 *
 * Per CLAUDE.md anti-patterns: no abstraction beyond what the drawer
 * needs. The shape mirrors `ImportConsoleEntry` with the schema
 * widened to the global-drawer severity union and a `source` label.
 */

import type { ConsoleSeverity, ConsoleSource } from '../stores/console.svelte';

export interface ConsoleEntry {
  readonly id: string;
  /** HH:MM:SS — wall-clock timestamp formatted at fixture time. */
  readonly time: string;
  readonly severity: ConsoleSeverity;
  /** Source label — a routed view name or `scene-import` worker tag.
   *  Drives the source dropdown filter. */
  readonly source: Exclude<ConsoleSource, 'all'>;
  readonly message: string;
}

/** Frozen-frame transcript. Ordered chronologically (oldest first);
 *  the drawer renders bottom-up so the newest row sits at the bottom
 *  matching terminal-output convention. */
export const CONSOLE_FIXTURES: readonly ConsoleEntry[] = [
  {
    id: 'lib-scan-1',
    time: '13:24:02',
    severity: 'scan',
    source: 'library',
    message: 'Indexing scenes from ~/Library/Application Support/Chromaticity/library',
  },
  {
    id: 'lib-scan-2',
    time: '13:24:03',
    severity: 'scan',
    source: 'library',
    message: 'Found 47 scenes (12 .chroma, 5 .chromaPack, 30 built-ins)',
  },
  {
    id: 'lib-load-1',
    time: '13:24:05',
    severity: 'load',
    source: 'library',
    message: 'Smooth Kaleidoscope 2 loaded successfully',
  },
  {
    id: 'lib-load-2',
    time: '13:24:06',
    severity: 'load',
    source: 'library',
    message: 'Spectral Bloom v3 loaded successfully',
  },
  {
    id: 'sources-info-1',
    time: '13:24:11',
    severity: 'info',
    source: 'sources',
    message: 'Scarlett 2i2 USB selected as default input',
  },
  {
    id: 'audio-load-1',
    time: '13:24:12',
    severity: 'load',
    source: 'audio-uniforms',
    message: 'FFT plan compiled: 2048 samples @ 48 kHz',
  },
  {
    id: 'midi-scan-1',
    time: '13:24:18',
    severity: 'scan',
    source: 'midi',
    message: 'Probing CoreMIDI clients (3 detected)',
  },
  {
    id: 'midi-load-1',
    time: '13:24:19',
    severity: 'load',
    source: 'midi',
    message: 'APC mini mk2 (port 1) — 64 pads bound',
  },
  {
    id: 'midi-warn-1',
    time: '13:24:20',
    severity: 'warn',
    source: 'midi',
    message: 'Launchpad X handshake retry 1/3',
  },
  {
    id: 'midi-load-2',
    time: '13:24:22',
    severity: 'load',
    source: 'midi',
    message: 'Launchpad X handshake succeeded on retry 2',
  },
  {
    id: 'osc-scan-1',
    time: '13:25:01',
    severity: 'scan',
    source: 'osc',
    message: 'Binding UDP port 9000 (0.0.0.0)',
  },
  {
    id: 'osc-info-1',
    time: '13:25:02',
    severity: 'info',
    source: 'osc',
    message: 'Listening for OSC on udp://0.0.0.0:9000',
  },
  {
    id: 'osc-warn-1',
    time: '13:25:14',
    severity: 'warn',
    source: 'osc',
    message: '/chroma/control/zoom — value 1.42 outside 0..1 range, clamping',
  },
  {
    id: 'perform-load-1',
    time: '13:26:03',
    severity: 'load',
    source: 'perform',
    message: 'Set "Friday Mainstage" — 12 cues mounted',
  },
  {
    id: 'perform-compile-1',
    time: '13:26:04',
    severity: 'compile',
    source: 'perform',
    message: 'Cue "Bloom Build" — pipeline compiled in 184 ms',
  },
  {
    id: 'perform-compile-2',
    time: '13:26:05',
    severity: 'compile',
    source: 'perform',
    message: 'Cue "Tunnel Drop" — pipeline compiled in 217 ms',
  },
  {
    id: 'perform-warn-1',
    time: '13:26:08',
    severity: 'warn',
    source: 'perform',
    message: 'GPU memory at 78% — consider lowering render scale',
  },
  {
    id: 'import-scan-1',
    time: '13:27:11',
    severity: 'scan',
    source: 'scene-import',
    message: 'No scene.json file found in lygia_digitizer_demo.chroma',
  },
  {
    id: 'import-fetch-1',
    time: '13:28:02',
    severity: 'fetch',
    source: 'scene-import',
    message: 'shadertoy.com/view/WdVfRh — 14.2 KB',
  },
  {
    id: 'import-compile-1',
    time: '13:28:03',
    severity: 'compile',
    source: 'scene-import',
    message: 'shadertoy → Success (3 uniforms remapped)',
  },
  {
    id: 'import-warn-1',
    time: '13:28:04',
    severity: 'warn',
    source: 'scene-import',
    message: 'iChannel0 (cubemap) substituted with default checker — manual follow-up needed',
  },
  {
    id: 'author-load-1',
    time: '13:29:14',
    severity: 'load',
    source: 'author',
    message: 'Scene "Tunnel Drop" opened for editing',
  },
  {
    id: 'author-compile-1',
    time: '13:29:21',
    severity: 'compile',
    source: 'author',
    message: 'Fragment shader compiled in 92 ms',
  },
  {
    id: 'author-error-1',
    time: '13:29:43',
    severity: 'error',
    source: 'author',
    message:
      "Fragment shader: ERROR: 0:42: 'iTime' : undeclared identifier (try uniform float u_time)",
  },
  {
    id: 'author-compile-2',
    time: '13:30:02',
    severity: 'compile',
    source: 'author',
    message: 'Fragment shader compiled in 98 ms',
  },
  {
    id: 'playlist-load-1',
    time: '13:30:38',
    severity: 'load',
    source: 'playlist',
    message: 'Playlist "Friday Mainstage" — 12 cues, 1h 04m total',
  },
  {
    id: 'osc-error-1',
    time: '13:31:09',
    severity: 'error',
    source: 'osc',
    message: 'Bind failed: udp://0.0.0.0:9000 already in use',
  },
  {
    id: 'osc-info-2',
    time: '13:31:10',
    severity: 'info',
    source: 'osc',
    message: 'Falling back to udp://0.0.0.0:7001',
  },
  {
    id: 'patch-load-1',
    time: '13:31:54',
    severity: 'load',
    source: 'patch',
    message: 'Patch graph "default" — 9 nodes, 14 edges',
  },
  {
    id: 'perform-fetch-1',
    time: '13:32:12',
    severity: 'fetch',
    source: 'perform',
    message: 'Loading next cue thumbnail (cache miss)',
  },
];
