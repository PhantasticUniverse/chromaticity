/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Fixture data for `Inputs.svelte`. Hardcoded to keep the inspector
 * self-contained; the structural template mirrors
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/chroma_inspector.jsx`
 * lines 28-78 (port source). The NATIVE fixture mirrors the content of
 * `engine/scene/tests/fixtures/scenes/demo-kaleido.chroma/scene.json` (W10
 * PR1) — same TITLE / CREDIT / control NAMEs, with one synthesized row
 * appended (`palette`) so the inspector exercises the dropdown placeholder
 * alongside the four mappable widget primitives and the xy placeholder.
 *
 * The COMPAT fixture is a synthesized compat-syn-v1 example demonstrating
 * the CP-01 missing-TYPE coercion variants (slider, toggle, color, xy)
 * plus a bang placeholder. See `docs/28-implementation-contract-pack.md`
 * §CP-01 for the canonical coercion table.
 *
 * Type-string normalisation: scene.json `TYPE` values follow the
 * "first ASCII-whitespace-split token" rule (per `engine/scene/src/schema.rs`
 * `ControlKind::deserialize`). The fixture data here uses the bare base
 * token only (`"slider"`, not `"slider smooth"`); the W11 IPC-fed loader
 * will apply the same token split when it lands.
 *
 * Default-value normalisation: scene.json color controls carry a Vec4
 * float tuple (`[r, g, b, a]` in 0..1). The inspector renders colors via
 * the W8 Swatch primitive which takes a CSS color string, so this fixture
 * pre-converts the demo-kaleido `tint` Vec4 to a `#rrggbb` literal.
 *
 * Manual sync: when the demo-kaleido fixture changes, update
 * CHROMA_FIXTURE_NATIVE accordingly. W11+ will replace this with
 * auto-loaded data via IPC.
 */

export type ChromaControlValue =
  | number
  | boolean
  | string
  | readonly [number, number]
  | readonly [number, number, number]
  | readonly [number, number, number, number];

export type ChromaControlType = 'slider' | 'knob' | 'toggle' | 'bang' | 'xy' | 'color' | 'dropdown';

export interface ChromaControl {
  readonly name: string;
  readonly label?: string;
  readonly type: ChromaControlType;
  readonly default?: ChromaControlValue;
  readonly min?: number;
  readonly max?: number;
  readonly group?: string;
  readonly description?: string;
}

export interface ChromaSceneFixture {
  readonly title: string;
  readonly credit: string;
  readonly description: string;
  readonly format_tag: 'native-chroma-v1' | 'compat-syn-v1';
  readonly controls: readonly ChromaControl[];
}

export const CHROMA_FIXTURE_NATIVE: ChromaSceneFixture = {
  title: 'Demo Kaleido',
  credit: 'chromaticity demo',
  description: 'Six-fold radial mirror with drifting hue and a soft warp envelope.',
  format_tag: 'native-chroma-v1',
  controls: [
    // Mirror of demo-kaleido.chroma/scene.json CONTROLS — six rows.
    {
      name: 'segments',
      type: 'slider',
      default: 6.0,
      min: 2.0,
      max: 24.0,
      group: 'geometry',
      description: 'kaleidoscope segment count',
    },
    {
      name: 'rotation_speed',
      label: 'rotation speed',
      type: 'slider',
      default: 0.25,
      min: 0.0,
      max: 1.0,
      group: 'motion',
      description: 'time multiplier on rotation',
    },
    {
      name: 'warp_amount',
      label: 'warp amount',
      type: 'knob',
      default: 0.4,
      min: 0.0,
      max: 1.0,
      group: 'geometry',
      description: 'domain warp depth',
    },
    {
      name: 'invert',
      type: 'toggle',
      default: false,
      group: 'fx',
      description: 'invert final colors',
    },
    {
      name: 'tint',
      type: 'color',
      // demo-kaleido tint is [0.62, 0.31, 0.88, 1.0]; pre-converted
      // to #rrggbb for Swatch's CSS-color string contract.
      default: '#9e4fe0',
      group: 'color',
      description: 'global tint, mix 0.2',
    },
    {
      name: 'focus',
      type: 'xy',
      default: [0.5, 0.5],
      group: 'geometry',
      description: 'center of symmetry',
    },
    // Synthesized rows so the inspector exercises the dropdown placeholder.
    {
      name: 'palette',
      type: 'dropdown',
      default: 1,
      group: 'color',
      description: 'preset palette index',
    },
  ],
};

export const CHROMA_FIXTURE_COMPAT: ChromaSceneFixture = {
  title: 'Compat Demo',
  credit: 'chromaticity compat synthesis',
  description: 'Synthesized compat-syn-v1 example demonstrating CP-01 coercion variants.',
  format_tag: 'compat-syn-v1',
  controls: [
    // 1. Slider (numeric DEFAULT → CP-01 slider coercion).
    {
      name: 'amount',
      type: 'slider',
      default: 0.5,
      min: 0.0,
      max: 1.0,
      group: 'fx',
      description: 'numeric default → slider coercion',
    },
    // 2. Toggle (boolean DEFAULT → CP-01 toggle coercion).
    {
      name: 'enabled',
      type: 'toggle',
      default: true,
      group: 'fx',
      description: 'boolean default → toggle coercion',
    },
    // 3. Color (Vec4 DEFAULT → CP-01 color coercion; pre-rendered hex here).
    {
      name: 'accent',
      type: 'color',
      default: '#3de5ff',
      group: 'color',
      description: 'vec4 default → color coercion',
    },
    // 4. XY (Vec2 DEFAULT → CP-01 xy coercion).
    {
      name: 'origin',
      type: 'xy',
      default: [0.0, 0.0],
      group: 'motion',
      description: 'vec2 default → xy coercion',
    },
    // 5. Bang (no DEFAULT → momentary trigger; placeholder row).
    {
      name: 'trigger',
      type: 'bang',
      group: 'fx',
      description: 'momentary edge — no default',
    },
  ],
};
