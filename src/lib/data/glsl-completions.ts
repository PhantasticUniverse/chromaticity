/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */

/*
 * GLSL autocomplete vocabulary for the W21 PR3 Author CodeMirror editor.
 *
 * Sources, normative:
 *   - `docs/36-reactive-symbol-catalog.md` Sec.1 — Standard Uniforms
 *     (`TIME`, `RENDERSIZE`, `_xy`, `_uv`, `_uvc`, `_mouse`, `_click`,
 *     `_muv`, `_muvc`, `PASSINDEX`, `FRAMECOUNT`, `TIMEDELTA`, `PI`).
 *   - `docs/36` Sec.2.1-2.10 — Audio-reactive uniforms (`chroma_Level`,
 *     `chroma_*Hits`, `chroma_AudioTime` / `chroma_*Time`, `chroma_*Presence`,
 *     `chroma_OnBeat`, `chroma_BPM*`, `chroma_FadeInOut`, `chroma_Intensity`,
 *     `chroma_PrimeBang`, `chroma_InputLevel`, `chroma_RawLevel`,
 *     `chroma_BassExpoCurve`, `chroma_Threshold`).
 *   - `docs/36` Sec.4 — GLSL helper shim catalog (`_map`, `_smin`,
 *     `_pulse`, `_palette`, `_rgb2hsv`, `_hsv2rgb`, etc.).
 *
 * Namespace policy (`docs/adr/ADR-024-native-chroma-vocabulary.md`):
 *   - `chroma_*` is the only reactive namespace offered to authors.
 *     Reference-format names exist solely in the `.synScene` import
 *     adapters and are never suggested here.
 *
 * Sort priority (highest to lowest):
 *   - keyword (3)  — GLSL language keywords (`uniform`, `vec3`, ...)
 *   - native  (2)  — `chroma_*` symbols and standard uniforms
 *   - helper  (1)  — `_*` helper functions
 *
 * The boost values are passed verbatim to CodeMirror's
 * `Completion.boost`; higher = appears earlier in the suggestion
 * list. CodeMirror sorts by boost desc, then by label asc within the
 * same boost.
 *
 * No autocomplete entries are added here that are not documented in
 * `docs/36`. This is the contract surface the W20 PR #149 namespace
 * cleanup intended to enforce — autocomplete vocabulary is a downstream
 * binding of the symbol catalog, not a free-form list.
 */

/** Completion category — maps to the badge / styling shown in the
 *  CodeMirror autocomplete dropdown and the boost value used for
 *  sort-order. */
export type CompletionCategory = 'keyword' | 'type' | 'native' | 'helper';

/** A single autocomplete entry for the GLSL editor. */
export interface GlslCompletion {
  /** The literal token inserted on accept. */
  readonly label: string;
  /** Category for badge / sort-priority / styling. */
  readonly category: CompletionCategory;
  /** Brief description shown in the dropdown detail pane. */
  readonly detail: string;
  /** Optional info text rendered under the suggestion. */
  readonly info?: string;
}

/** GLSL language keywords — type names, control flow, qualifiers. */
const GLSL_KEYWORDS: readonly GlslCompletion[] = [
  { label: 'uniform', category: 'keyword', detail: 'GLSL storage qualifier' },
  { label: 'const', category: 'keyword', detail: 'GLSL storage qualifier' },
  { label: 'in', category: 'keyword', detail: 'GLSL parameter qualifier' },
  { label: 'out', category: 'keyword', detail: 'GLSL parameter qualifier' },
  { label: 'inout', category: 'keyword', detail: 'GLSL parameter qualifier' },
  { label: 'void', category: 'keyword', detail: 'GLSL return type' },
  { label: 'return', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'if', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'else', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'for', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'while', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'break', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'continue', category: 'keyword', detail: 'GLSL control flow' },
  { label: 'discard', category: 'keyword', detail: 'GLSL fragment-shader builtin' },
  { label: 'precision', category: 'keyword', detail: 'GLSL precision specifier' },
  { label: 'highp', category: 'keyword', detail: 'GLSL precision qualifier' },
  { label: 'mediump', category: 'keyword', detail: 'GLSL precision qualifier' },
  { label: 'lowp', category: 'keyword', detail: 'GLSL precision qualifier' },
  { label: 'true', category: 'keyword', detail: 'GLSL bool literal' },
  { label: 'false', category: 'keyword', detail: 'GLSL bool literal' },
];

/** GLSL primitive types — vec, mat, sampler, scalar primitives. */
const GLSL_TYPES: readonly GlslCompletion[] = [
  { label: 'float', category: 'type', detail: 'GLSL scalar' },
  { label: 'int', category: 'type', detail: 'GLSL scalar' },
  { label: 'bool', category: 'type', detail: 'GLSL scalar' },
  { label: 'vec2', category: 'type', detail: 'GLSL 2-component float vector' },
  { label: 'vec3', category: 'type', detail: 'GLSL 3-component float vector' },
  { label: 'vec4', category: 'type', detail: 'GLSL 4-component float vector' },
  { label: 'ivec2', category: 'type', detail: 'GLSL 2-component int vector' },
  { label: 'ivec3', category: 'type', detail: 'GLSL 3-component int vector' },
  { label: 'ivec4', category: 'type', detail: 'GLSL 4-component int vector' },
  { label: 'mat2', category: 'type', detail: 'GLSL 2x2 float matrix' },
  { label: 'mat3', category: 'type', detail: 'GLSL 3x3 float matrix' },
  { label: 'mat4', category: 'type', detail: 'GLSL 4x4 float matrix' },
  { label: 'sampler1D', category: 'type', detail: 'GLSL 1D sampler' },
  { label: 'sampler2D', category: 'type', detail: 'GLSL 2D sampler' },
  { label: 'sampler3D', category: 'type', detail: 'GLSL 3D sampler' },
];

/** Standard uniforms from docs/36 Sec.1 (frame/timing, pixel coords,
 *  mouse, multipass). All have the underscore-leading convention or
 *  legacy reference-locked spellings (`TIME`, `PI`, `RENDERSIZE`, etc.)
 *  and are the foundation of every Chromaticity scene. */
const STANDARD_UNIFORMS: readonly GlslCompletion[] = [
  // Sec.1.1 frame / timing
  { label: 'TIME', category: 'native', detail: 'float — seconds since scene start' },
  { label: 'FRAMECOUNT', category: 'native', detail: 'float — frames since scene load' },
  { label: 'TIMEDELTA', category: 'native', detail: 'float — seconds since previous frame' },
  { label: 'PI', category: 'native', detail: 'const float — 3.14159265358979323846' },
  { label: 'TAU', category: 'native', detail: 'const float — 2.0 * PI (legacy spelling)' },
  // Sec.1.2 pixel coordinates
  { label: '_xy', category: 'native', detail: 'vec2 — integer pixel position' },
  { label: '_uv', category: 'native', detail: 'vec2 — normalised [0,1] UV' },
  { label: '_uvc', category: 'native', detail: 'vec2 — aspect-corrected UV centred at origin' },
  // Sec.1.3 mouse
  { label: '_mouse', category: 'native', detail: 'vec4 — x,y,prev_click_x,prev_click_y' },
  { label: '_click', category: 'native', detail: 'vec3 — {left,right,middle} mouse buttons' },
  { label: '_muv', category: 'native', detail: 'vec2 — mouse normalised UV' },
  { label: '_muvc', category: 'native', detail: 'vec2 — aspect-corrected mouse UV' },
  // Sec.1.4 multipass
  { label: 'RENDERSIZE', category: 'native', detail: 'vec2 — current pass pixel size' },
  { label: 'RESOLUTION', category: 'native', detail: 'vec2 — alias of RENDERSIZE (legacy)' },
  { label: 'PASSINDEX', category: 'native', detail: 'int — current pass index' },
  // Sec.1.4 multipass sampler
  {
    label: 'chroma_FinalPass',
    category: 'native',
    detail: 'sampler2D — previous frame final output',
  },
  // Sec.1.5 media
  { label: 'chroma_Media', category: 'native', detail: 'sampler2D — active media input' },
  {
    label: 'chroma_MediaType',
    category: 'native',
    detail: 'float — 0 none, 1 image, 2 video, 3 webcam',
  },
  // Reference legacy spellings still in scenes
  { label: 'gl_FragCoord', category: 'native', detail: 'vec4 — GLSL fragment coordinate' },
  { label: 'gl_FragColor', category: 'native', detail: 'vec4 — GLSL fragment output' },
];

/** Native `chroma_*` symbols from `docs/36` (currently `chroma_PrimeBang`
 *  per Sec.2.9; if more land in `docs/36` between brief writing and
 *  implementation they should appear here). The `chroma_Data_*` reserved
 *  namespace from Sec.1.6 is parsed-but-not-populated at MVP and is NOT
 *  added to autocomplete to avoid suggesting symbols that fail strict-
 *  reject without a matching `DATA_SOURCES` declaration. */
const CHROMA_NATIVE: readonly GlslCompletion[] = [
  {
    label: 'chroma_PrimeBang',
    category: 'native',
    detail: 'float — Profile-D differentiation pulse',
    info: 'Large-scale-transition pulse heuristic.',
  },
];

/** Audio-reactive uniforms from `docs/36` Sec.2.1-2.8 + 2.10. */
const AUDIO_REACTIVE: readonly GlslCompletion[] = [
  // Sec.2.1 data textures
  {
    label: 'chroma_Spectrum',
    category: 'native',
    detail: 'sampler1D — packed FFT (rgba channels)',
  },
  { label: 'chroma_LevelTrail', category: 'native', detail: 'sampler1D — per-band level history' },
  // Sec.2.2 level
  { label: 'chroma_Level', category: 'native', detail: 'float — overall level [0,1]' },
  { label: 'chroma_BassLevel', category: 'native', detail: 'float — bass-band level [0,1]' },
  { label: 'chroma_MidLevel', category: 'native', detail: 'float — mid-band level [0,1]' },
  { label: 'chroma_MidHighLevel', category: 'native', detail: 'float — mid-high-band level [0,1]' },
  { label: 'chroma_HighLevel', category: 'native', detail: 'float — high-band level [0,1]' },
  // Sec.2.3 hits
  { label: 'chroma_Hits', category: 'native', detail: 'float — hit detection [0,1]' },
  { label: 'chroma_BassHits', category: 'native', detail: 'float — bass hit detection [0,1]' },
  { label: 'chroma_MidHits', category: 'native', detail: 'float — mid hit detection [0,1]' },
  {
    label: 'chroma_MidHighHits',
    category: 'native',
    detail: 'float — mid-high hit detection [0,1]',
  },
  { label: 'chroma_HighHits', category: 'native', detail: 'float — high hit detection [0,1]' },
  // Sec.2.4 time clocks
  { label: 'chroma_AudioTime', category: 'native', detail: 'float — overall time clock' },
  { label: 'chroma_BassTime', category: 'native', detail: 'float — bass-band time clock' },
  { label: 'chroma_MidTime', category: 'native', detail: 'float — mid-band time clock' },
  { label: 'chroma_MidHighTime', category: 'native', detail: 'float — mid-high-band time clock' },
  { label: 'chroma_HighTime', category: 'native', detail: 'float — high-band time clock' },
  { label: 'chroma_CurvedTime', category: 'native', detail: 'float — exponential-rise time clock' },
  // Sec.2.5 presence
  { label: 'chroma_Presence', category: 'native', detail: 'float — overall presence [0,1]' },
  { label: 'chroma_BassPresence', category: 'native', detail: 'float — bass presence [0,1]' },
  { label: 'chroma_MidPresence', category: 'native', detail: 'float — mid presence [0,1]' },
  {
    label: 'chroma_MidHighPresence',
    category: 'native',
    detail: 'float — mid-high presence [0,1]',
  },
  { label: 'chroma_HighPresence', category: 'native', detail: 'float — high presence [0,1]' },
  // Sec.2.6 beat
  { label: 'chroma_OnBeat', category: 'native', detail: 'float — 1.0 on beat, decays to 0' },
  { label: 'chroma_ToggleOnBeat', category: 'native', detail: 'float — alternates 0/1 per beat' },
  { label: 'chroma_RandomOnBeat', category: 'native', detail: 'float — new random value on beat' },
  { label: 'chroma_BeatTime', category: 'native', detail: 'float — beat-count integer clock' },
  // Sec.2.7 BPM
  { label: 'chroma_BPM', category: 'native', detail: 'float — estimated BPM, clamped [50,220]' },
  { label: 'chroma_BPMConfidence', category: 'native', detail: 'float — BPM stability [0,1]' },
  { label: 'chroma_BPMTwitcher', category: 'native', detail: 'float — exponential beat clock' },
  { label: 'chroma_BPMSin', category: 'native', detail: 'float — sine [0,1] at BPM frequency' },
  { label: 'chroma_BPMSin2', category: 'native', detail: 'float — sine [0,1] at BPM/2' },
  { label: 'chroma_BPMSin4', category: 'native', detail: 'float — sine [0,1] at BPM/4' },
  { label: 'chroma_BPMTri', category: 'native', detail: 'float — triangle [0,1] at BPM' },
  { label: 'chroma_BPMTri2', category: 'native', detail: 'float — triangle [0,1] at BPM/2' },
  { label: 'chroma_BPMTri4', category: 'native', detail: 'float — triangle [0,1] at BPM/4' },
  // Sec.2.8 large features
  { label: 'chroma_FadeInOut', category: 'native', detail: 'float — slow rise on music start' },
  {
    label: 'chroma_Intensity',
    category: 'native',
    detail: 'float — slow song-intensity accumulator',
  },
  // Sec.2.10 reference-surface uniforms
  { label: 'chroma_InputLevel', category: 'native', detail: 'float — pre-AGC instantaneous level' },
  { label: 'chroma_RawLevel', category: 'native', detail: 'float — alias of chroma_InputLevel' },
  {
    label: 'chroma_BassExpoCurve',
    category: 'native',
    detail: 'float — pow(chroma_BassLevel, 3.0)',
  },
  {
    label: 'chroma_Threshold',
    category: 'native',
    detail: 'float — operator-set noise-floor gate',
  },
];

/** Meta-control symbols from `docs/36` Sec.3 (`_Master`, `_Brightness`,
 *  `_Gamma`, etc.). Underscore-leading globals owned by StateCore. */
const META_CONTROLS: readonly GlslCompletion[] = [
  { label: '_Master', category: 'native', detail: 'float — master output level' },
  { label: '_Brightness', category: 'native', detail: 'float — brightness multiplier' },
  { label: '_Gamma', category: 'native', detail: 'float — gamma correction' },
  { label: '_Contrast', category: 'native', detail: 'float — contrast multiplier' },
  { label: '_Hue', category: 'native', detail: 'float — hue rotation [-1,1]' },
  { label: '_Saturation', category: 'native', detail: 'float — saturation [-1,1]' },
  { label: '_Invert', category: 'native', detail: 'float — colour invert amount [0,1]' },
];

/** GLSL helper functions from `docs/36` Sec.4 — Chromaticity helper
 *  shim catalog. The most-reached-for helpers; the full list is in
 *  the catalog. */
const HELPER_FUNCTIONS: readonly GlslCompletion[] = [
  // Sec.4.1 math
  { label: '_scale', category: 'helper', detail: 'fn(v, min, max) → float' },
  { label: '_map', category: 'helper', detail: 'fn(v, min, max, new_min, new_max) → T' },
  { label: '_smin', category: 'helper', detail: 'fn(a, b, k) → float — smooth min' },
  { label: '_pulse', category: 'helper', detail: 'fn(pos, center, size) → float — smooth pulse' },
  { label: '_sqPulse', category: 'helper', detail: 'fn(pos, center, size) → float — square pulse' },
  { label: '_inRange', category: 'helper', detail: 'fn(v[, min=0, max=1]) → float' },
  { label: '_triWave', category: 'helper', detail: 'fn(pos, period) → float [-0.5, 0]' },
  { label: '_pixelate', category: 'helper', detail: 'fn(v, amount) → T — quantise' },
  { label: '_mix3', category: 'helper', detail: 'fn(a, b, c, t) → T — 3-way mix' },
  { label: '_mix4', category: 'helper', detail: 'fn(a, b, c, d, t) → T — 4-way mix' },
  { label: '_mix5', category: 'helper', detail: 'fn(a, b, c, d, e, t) → T — 5-way mix' },
  { label: '_nsin', category: 'helper', detail: 'fn(x) → float — sin normalised to [0,1]' },
  { label: '_ncos', category: 'helper', detail: 'fn(x) → float — cos normalised to [0,1]' },
  { label: '_nclamp', category: 'helper', detail: 'fn(v) → T — clamp to [0,1]' },
  { label: '_rand', category: 'helper', detail: 'fn(seed) → float — pseudo-random' },
  // Sec.4.2 noise
  { label: '_noise', category: 'helper', detail: 'fn(p) → float — value noise' },
  { label: '_fbm', category: 'helper', detail: 'fn(p) → float — fractal Brownian motion' },
  // Sec.4.3 coordinates
  { label: '_uv2uvc', category: 'helper', detail: 'fn(uv) → vec2' },
  { label: '_uvc2uv', category: 'helper', detail: 'fn(uvc) → vec2' },
  { label: '_rotate', category: 'helper', detail: 'fn(p, rad) → vec2 / vec3' },
  { label: '_mirror', category: 'helper', detail: 'fn(p) → vec2' },
  { label: '_toPolar', category: 'helper', detail: 'fn(p) → vec2 — angle/radius' },
  { label: '_toRect', category: 'helper', detail: 'fn(p) → vec2' },
  // Sec.4.4 colour
  { label: '_rgb2hsv', category: 'helper', detail: 'fn(rgb) → vec3' },
  { label: '_hsv2rgb', category: 'helper', detail: 'fn(hsv) → vec3' },
  { label: '_palette', category: 'helper', detail: 'fn(t, a, b, c, d) → vec3 — IQ palette' },
  { label: '_brightness', category: 'helper', detail: 'fn(col, b) → vec3 / vec4' },
  { label: '_gamma', category: 'helper', detail: 'fn(col, g) → vec3 / vec4' },
  { label: '_contrast', category: 'helper', detail: 'fn(col, c) → vec3 / vec4' },
  { label: '_saturation', category: 'helper', detail: 'fn(col, s) → vec3 / vec4' },
  { label: '_hueRotate', category: 'helper', detail: 'fn(col, deg) → vec3 / vec4' },
  { label: '_invert', category: 'helper', detail: 'fn(col, a) → vec3 / vec4' },
  { label: '_luminance', category: 'helper', detail: 'fn(col) → float' },
  { label: '_grayscale', category: 'helper', detail: 'fn(col[, intensity=1]) → vec3 / vec4' },
  { label: '_dichrome', category: 'helper', detail: 'fn(col, low, high) → vec3 / vec4' },
  // Sec.4.5 media
  { label: '_loadMedia', category: 'helper', detail: 'fn([offset, lod]) → vec4' },
  { label: '_textureMedia', category: 'helper', detail: 'fn(uv[, lod]) → vec4' },
  { label: '_isMediaActive', category: 'helper', detail: 'fn() → bool' },
  // Sec.4.6 multipass
  { label: '_edgeDetect', category: 'helper', detail: 'fn(tex[, uv]) → vec4' },
  { label: '_edgeDetectSobel', category: 'helper', detail: 'fn(tex[, uv]) → vec4' },
];

/** Full vocabulary, exported for use by the CompletionSource. Order is
 *  not significant — sort priority is encoded in `category`. */
export const GLSL_COMPLETIONS: readonly GlslCompletion[] = [
  ...GLSL_KEYWORDS,
  ...GLSL_TYPES,
  ...STANDARD_UNIFORMS,
  ...CHROMA_NATIVE,
  ...META_CONTROLS,
  ...AUDIO_REACTIVE,
  ...HELPER_FUNCTIONS,
];

/** Boost weight per category. Higher = sorted earlier in the dropdown.
 *  CodeMirror's `Completion.boost` field accepts any number; the
 *  framework sorts by boost desc, then label asc within ties. The
 *  values here keep keywords above natives (so `vec3` beats `_uv` for
 *  a `v` prefix) and natives above helpers. */
export function categoryBoost(category: CompletionCategory): number {
  switch (category) {
    case 'keyword':
      return 3;
    case 'type':
      return 3;
    case 'native':
      return 2;
    case 'helper':
      return 1;
  }
}

/** Single-character "type" hint for the CodeMirror dropdown — feeds
 *  the per-row icon. The framework supports a constrained set of icon
 *  names; pick the closest mapping. */
export function categoryType(category: CompletionCategory): string {
  switch (category) {
    case 'keyword':
      return 'keyword';
    case 'type':
      return 'type';
    case 'native':
      return 'variable';
    case 'helper':
      return 'function';
  }
}
