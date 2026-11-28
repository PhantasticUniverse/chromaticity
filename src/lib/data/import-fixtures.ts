/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * Import-view fixtures — TypeScript mirror of the v0.2 bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/import.jsx`
 * `ImportView` constants. Powers the W19 PR2 full port that promotes
 * the previously-disabled sidebar entry to a routed view.
 *
 * Per the auto-memory `feedback_wireframes_normative_for_ui_ux.md`,
 * the bundle JSX is the canonical source; this fixture mirrors it
 * verbatim so future bundle revisions sync cleanly through both
 * surfaces.
 *
 * W19 PR2 posture: hardcoded fixtures only. Real conversion mechanics
 * (Shadertoy fetch + uniform remap, .chroma package open, .synScene
 * one-way ingest, ISF parser deferral diagnostic) are post-Phase-1
 * scope; this slice ships only the converter card chrome, recent-
 * imports list, and the frozen console transcript.
 */

/** A single converter card rendered in the main panel. Mirrors the
 *  bundle JSX `converters` array entries verbatim. */
export interface ImportConverter {
  readonly id: string;
  readonly title: string;
  /** Body prose under the title — typically explains the conversion's
   *  fidelity and operator follow-up expectations. */
  readonly description: string;
  /** Input field placeholder — URL pattern, file path, etc. */
  readonly placeholder: string;
  /** Status pill text — "Ready" / "V1 converter" / "V2 deferred". */
  readonly status: string;
  /** Whether Preview / Import buttons are disabled (V2-deferred row). */
  readonly disabled?: boolean;
}

/** Bundle JSX source: `views/import.jsx` lines 4-10. The five
 *  converters cover the three first-party formats (.chroma directory,
 *  .chromaPack single-file transport, Shadertoy URL ingest) plus the
 *  one-way Synesthesia compat path (.synScene → .chroma) and the
 *  V2-deferred ISF rejection escape hatch. */
export const IMPORT_CONVERTERS: readonly ImportConverter[] = [
  {
    id: 'shadertoy',
    title: 'Shadertoy.com',
    description:
      'Paste a Shadertoy URL — Chromaticity converts common uniforms and iChannel slots into a native .chroma package with diagnostics for manual follow-up.',
    placeholder: 'https://www.shadertoy.com/view/…',
    status: 'Built-in',
  },
  {
    id: 'chroma-scene',
    title: 'Chromaticity Scene (.chroma)',
    description:
      'Native directory package — full fidelity, scripts, render passes, and embedded assets.',
    placeholder: 'path/to/scene.chroma',
    status: 'Ready',
  },
  {
    id: 'chroma-pack',
    title: 'Chromaticity Pack (.chromaPack)',
    description: 'Single-file transport package for one scene and its assets.',
    placeholder: 'path/to/scene.chromaPack',
    status: 'Ready',
  },
  {
    id: 'syn-scene',
    title: 'Synesthesia Scene (.synScene)',
    description:
      'One-way compatibility conversion into .chroma. The source package is never modified.',
    placeholder: 'path/to/scene.synScene',
    status: 'Built-in',
  },
  {
    id: 'isf',
    title: 'Interactive Shader Format (ISF)',
    description: 'ISF (.fs) shader import is planned for a future release.',
    placeholder: 'ISF (.fs) support coming soon',
    status: 'Coming soon',
    disabled: true,
  },
];

/** A row in the "Recent Imports" list. Mirrors the bundle JSX
 *  `RECENT` array (lines 12-19); `seed` and `kind` originally
 *  parameterised the bundle's `<SceneStill>` thumbnail primitive
 *  and are preserved here so the placeholder thumbnail tile keeps
 *  the same per-row variation. */
export interface RecentImport {
  readonly id: string;
  /** Title shown on the row's first text line. */
  readonly title: string;
  /** Activity sub-line — converter / state ("Shadertoy → iq", "ISF
   *  rejected · blank escape hatch"). */
  readonly activity: string;
  /** Bundle's `<SceneStill kind>` parameter — preserved as a hint for
   *  when the renderer lands. */
  readonly kind: string;
  /** Bundle's `<SceneStill seed>` parameter — drives the placeholder
   *  thumbnail's hue. */
  readonly seed: number;
}

export const RECENT_IMPORTS: readonly RecentImport[] = [
  {
    id: 'shadertoy-WdVfRh',
    title: 'WdVfRh',
    activity: 'Shadertoy → iq',
    kind: 'plasma',
    seed: 5,
  },
  {
    id: 'syn-bipolar-feedback',
    title: 'Bipolar Feedback',
    activity: '.synScene → .chroma',
    kind: 'kaleido',
    seed: 11,
  },
  {
    id: 'shadertoy-cubic-dispersal',
    title: 'Cubic Dispersal',
    activity: 'Shadertoy → tutor',
    kind: 'lattice',
    seed: 18,
  },
  {
    id: 'syn-outline-2020-freestyle',
    title: 'Outline 2020 Freestyle',
    activity: '.synScene → .chroma',
    kind: 'filaments',
    seed: 22,
  },
  {
    id: 'shadertoy-fractal-136-gaz',
    title: 'Fractal 136_gaz',
    activity: 'Shadertoy → gaz',
    kind: 'tunnel',
    seed: 2,
  },
  {
    id: 'chroma-dvd-bounce',
    title: 'DVD Bounce',
    activity: '.chroma package imported',
    kind: 'grid',
    seed: 3,
  },
];

/** Console transcript severity — drives the per-line tag colour
 *  per the bundle's `c-info` / `c-ok` / `c-warn` / `c-err` rules
 *  in `styles.css` lines 600-604. */
export type ImportConsoleLevel = 'info' | 'ok' | 'warn' | 'err';

export interface ImportConsoleEntry {
  readonly id: string;
  readonly time: string;
  readonly level: ImportConsoleLevel;
  /** Tag word printed after the timestamp ("scan" / "load" / "fetch"
   *  / "compile" / "warn"). */
  readonly tag: string;
  /** Free-form body. */
  readonly message: string;
}

/** Frozen-frame console transcript mirroring `views/import.jsx` lines
 *  75-80. Six entries spanning a 13:27 → 13:31 window. */
export const IMPORT_CONSOLE: readonly ImportConsoleEntry[] = [
  {
    id: 'scan-lygia',
    time: '13:27',
    level: 'info',
    tag: 'scan',
    message: 'No scene.json file found in lygia_digitizer_demo.synScene',
  },
  {
    id: 'scan-spectrozonal',
    time: '13:27',
    level: 'info',
    tag: 'scan',
    message: 'No scene.json file found in spectrozonal.synScene',
  },
  {
    id: 'load-smooth-kaleidoscope',
    time: '13:28',
    level: 'ok',
    tag: 'load',
    message: 'Smooth Kaleidoscope 2 loaded successfully',
  },
  {
    id: 'fetch-shadertoy',
    time: '13:31',
    level: 'info',
    tag: 'fetch',
    message: 'shadertoy.com/view/WdVfRh',
  },
  {
    id: 'compile-shadertoy',
    time: '13:31',
    level: 'ok',
    tag: 'compile',
    message: 'shadertoy → Success',
  },
  {
    id: 'warn-imouse-remap',
    time: '13:31',
    level: 'warn',
    tag: 'warn',
    message: 'iMouse remap — no pointer bound. Using default.',
  },
];

/** Conversion-preview placeholder hint copy. The bundle uses its
 *  `<Preview seed={14} kind="plasma" />` runtime renderer; until the
 *  ADR-011 backend lands the W19 PR2 view paints a token-driven
 *  gradient and labels it accordingly. */
export const PREVIEW_LABEL = 'Conversion Preview';
export const PREVIEW_SEED = 14;
export const PREVIEW_KIND = 'plasma';

/** Top-right hint text rendered next to the panel title — bundle JSX
 *  line 27. */
export const CONVERTER_HINT =
  'Shadertoy uniforms are auto-remapped. Some shaders need manual tweaks.';
