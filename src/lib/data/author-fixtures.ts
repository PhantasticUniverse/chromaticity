/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Author view fixtures — TypeScript mirror of the predev bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/scene.jsx`
 * `SceneEditorView` component (the `GLSL` / `SCRIPT` constants, the
 * inline file tabs, console-output rows, render-pass list, and
 * preset / scene-list / import side-rail data shape).
 *
 * W18 PR2 posture: HARDCODED frozen-frame fixtures only. The bundle's
 * editor is a read-only DOM rendering of a syntax-tokenised line list
 * (see the `tokens()` helper in scene.jsx) — there is no CodeMirror
 * dependency in the bundle source. The Svelte port preserves that
 * approach: a `<pre><code>`-style line grid with regex-derived token
 * spans. Real editing (input handling, autocomplete, save / compile
 * IPC) waits for the script-contract slice landing in Phase-2
 * (`EV-SCRIPT-CONTRACT`, docs/26). At that point the read-only view
 * can be swapped to a real editor (CodeMirror or otherwise) without
 * re-deriving the GLSL / script content; the fixture file is the
 * substitution boundary.
 *
 * Per the wireframe-alignment memory
 * (`feedback_wireframes_normative_for_ui_ux.md`), the bundle JSX is the
 * canonical structural reference; this fixture file is its data mirror.
 */

/** GLSL fragment displayed as the `main.glsl` tab content. Verbatim from
 *  the bundle's `GLSL` constant — the prose header retains the original
 *  CC0 licence note and attribution. */
export const AUTHOR_GLSL_SOURCE = `// License CC0: Smooth Kaleidoscope 2
// Been working a bit too much and lacked the motivation do shaders for awhile
// Wanted to do something simple to get started again so went back to kaleidoscopic
// effects that tend to be rather simple yet produce appealing results.

// SABS by o11j.
// rgb2hsv from: https://stackoverflow.com/questions/15095909/from-rgb-to-hsv-in-opengl-glsl

#define TIME        TIME
#define RESOLUTION  RENDERSIZE
#define PI          3.141592654
#define TAU         (2.0*PI)
#define LESS(a,b,c) mix(a,b,step(0.,c))
#define SABS(x,k)   LESS((.5/(k))*(x)*(x)+(k)*.5,abs(x),abs(x)-(k))
#define ROT(x)      mat2(cos(x), -sin(x), sin(x), cos(x))
#define PSIN(x)     (0.5+0.5*sin(x))

const mat2[] rotations = mat2[](ROT(0.0*PI/2.0), ROT(1.0*PI/2.0), ROT(2.0*PI/2.0), ROT(3.0*PI/2.0));

float rand(vec2 n) {
  return fract(sin(dot(n, vec2(12.9898, 4.1414))) * 43758.5453);
}

vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec2 toPolar(vec2 p) {
  return vec2(length(p), atan(p.y, p.x));
}

vec2 toRect(vec2 p) {
  return p.x*vec2(cos(p.y), sin(p.y));
}`;

/** Script content displayed as the `script.js` tab. Verbatim from the
 *  bundle's `SCRIPT` constant. */
export const AUTHOR_SCRIPT_SOURCE = `// script.js — runs per frame. Access uniforms via ctx.
const rainbowize = ctx.controls.rainbowize;  // 0 .. 1

ctx.setUniform("rainbowize", rainbowize);
ctx.setUniform("fiddle", Math.sin(ctx.time * 0.3) * 0.5 + 0.5);

// respond to onBeat
ctx.on("beat", () => {
  ctx.flash(0.12);
});
`;

/** JSON content for the `config.json` tab. Bundle wireframe shows the
 *  scene metadata as a separate tab; keep the shape aligned with the
 *  bundle's Scene Info pane defaults so a future writeable editor can
 *  round-trip via this file. */
export const AUTHOR_CONFIG_SOURCE = `{
  "title": "Smooth Kaleidoscope 2",
  "credit": "mrange",
  "license": "CC0",
  "thumbnail": "smooth_kaleidoscope_2.png",
  "controls": [
    { "name": "rainbowize", "type": "slider", "min": 0, "max": 1, "default": 0 }
  ],
  "passes": [
    { "name": "bufferA", "res": "full", "format": "RGBA16F", "persistent": true },
    { "name": "bufferB", "res": "full", "format": "RGBA16F", "persistent": false },
    { "name": "final",   "res": "full", "format": "RGBA8",   "persistent": false, "main": true }
  ]
}
`;

/** README content for the `readme.md` tab. */
export const AUTHOR_README_SOURCE = `# Smooth Kaleidoscope 2

License CC0 · by mrange.

A kaleidoscopic GLSL fragment shader that produces appealing rotational
patterns with smooth absolute-value primitives. Originally adapted from
SABS by o11j.

## Controls

- **rainbowize** — creates vivid rainbow bands. 0..1 slider.
`;

/** A file tab in the editor toolbar — the bundle uses `main.glsl`,
 *  `script.js`, `config.json`, `readme.md` per the file-button row. */
export type AuthorFileTab = 'main.glsl' | 'script.js' | 'config.json' | 'readme.md';

/** Right-rail inspector tab — mirrors the bundle's `tab` state machine
 *  (`list | palette | edit | plus | ssf`). */
export type AuthorInspectorTab = 'list' | 'palette' | 'edit' | 'ssf' | 'plus';

/** Token kind emitted by the syntax pseudo-highlighter. Mapping mirrors
 *  the bundle's `tokens()` regex (`C` comment, `K` keyword, `U` uniform
 *  / builtin, `N` number, `S` string). */
export type AuthorTokenKind = 'comment' | 'keyword' | 'uniform' | 'number' | 'string' | 'plain';

/** A single token span emitted by the line tokeniser. */
export interface AuthorToken {
  readonly kind: AuthorTokenKind;
  readonly text: string;
}

/** Full source text per file tab. */
export const AUTHOR_FILE_SOURCES: Readonly<Record<AuthorFileTab, string>> = {
  'main.glsl': AUTHOR_GLSL_SOURCE,
  'script.js': AUTHOR_SCRIPT_SOURCE,
  'config.json': AUTHOR_CONFIG_SOURCE,
  'readme.md': AUTHOR_README_SOURCE,
};

/** Highlighter "kind" per file tab — `glsl` for GLSL, `js` for JS.
 *  JSON / Markdown fall back to a non-tokenising plain pass to keep the
 *  rendering aligned with the bundle (the bundle source only defines
 *  GLSL and JS tokenisation paths; everything else gets the catch-all
 *  GLSL regex which is harmless on JSON / MD but not visually useful).
 *  This port renders JSON / MD as plain monospaced text. */
export type AuthorHighlightKind = 'glsl' | 'js' | 'plain';

export const AUTHOR_FILE_KINDS: Readonly<Record<AuthorFileTab, AuthorHighlightKind>> = {
  'main.glsl': 'glsl',
  'script.js': 'js',
  'config.json': 'plain',
  'readme.md': 'plain',
};

/** Console row displayed in the bottom output panel. */
export interface AuthorConsoleRow {
  readonly time: string;
  readonly level: 'info' | 'ok' | 'warn';
  readonly tag: string;
  readonly message: string;
}

/** Verbatim from the bundle's hard-coded console rows. */
export const AUTHOR_CONSOLE_ROWS: readonly AuthorConsoleRow[] = [
  {
    time: '13:27:02',
    level: 'info',
    tag: 'load',
    message: 'No scene.json file found in lygia_digitizer_demo.synScene',
  },
  {
    time: '13:27:02',
    level: 'info',
    tag: 'load',
    message: 'No scene.json file found in simple_tygia_scene.synScene',
  },
  {
    time: '13:27:02',
    level: 'info',
    tag: 'load',
    message: 'No scene.json file found in spectrozonal.synScene',
  },
  {
    time: '13:28:00',
    level: 'ok',
    tag: 'load',
    message: 'Smooth Kaleidoscope 2 loaded successfully',
  },
  {
    time: '13:28:10',
    level: 'ok',
    tag: 'load',
    message: 'Gaussian Smoothrite loaded successfully',
  },
  {
    time: '13:31:42',
    level: 'info',
    tag: 'shadertoy',
    message: 'Success',
  },
];

/** Render-pass row in the inspector "Render Passes" sub-section. */
export interface AuthorRenderPass {
  readonly name: string;
  readonly res: string;
  readonly fmt: string;
  readonly persistent: boolean;
  readonly main: boolean;
}

/** Verbatim from the bundle's hard-coded render-pass list. */
export const AUTHOR_RENDER_PASSES: readonly AuthorRenderPass[] = [
  { name: 'bufferA', res: 'full', fmt: 'RGBA16F', persistent: true, main: false },
  { name: 'bufferB', res: 'full', fmt: 'RGBA16F', persistent: false, main: false },
  { name: 'final', res: 'full', fmt: 'RGBA8', persistent: false, main: true },
];

/** Control-preset row in the `palette` inspector tab. */
export interface AuthorPreset {
  readonly id: string;
  readonly label: string;
  readonly active: boolean;
}

/** Verbatim from the bundle's hard-coded preset list. */
export const AUTHOR_PRESETS: readonly AuthorPreset[] = [
  { id: 'default', label: 'default', active: true },
  { id: 'warm-dusk', label: 'Warm Dusk', active: false },
  { id: 'hard-techno', label: 'Hard Techno', active: false },
  { id: 'ambient-drift', label: 'Ambient Drift', active: false },
];

/** Scene-list row in the `list` inspector tab. */
export interface AuthorSceneRow {
  readonly idx: number;
  readonly name: string;
  readonly tag: string;
  readonly duration: string;
  readonly active: boolean;
}

/** Hand-picked scene-list rows mirroring the bundle's `SCENES.slice(0, 8)`
 *  output shape. Names are clean-room — they evoke the bundle's
 *  "kaleido / fluid / oscillator" style without copying the bundle's
 *  internal `_data.jsx` corpus verbatim. */
export const AUTHOR_SCENE_ROWS: readonly AuthorSceneRow[] = [
  { idx: 1, name: 'Spectrozonal', tag: 'kaleido', duration: '0:32', active: false },
  { idx: 2, name: 'Lygia Digitizer Demo', tag: 'lygia', duration: '1:04', active: false },
  { idx: 3, name: 'Simple Tygia', tag: 'fluid', duration: '0:48', active: false },
  { idx: 4, name: 'Smooth Kaleidoscope 2', tag: 'kaleido', duration: '1:12', active: true },
  { idx: 5, name: 'Gaussian Smoothrite', tag: 'plasma', duration: '0:56', active: false },
  { idx: 6, name: 'Iridescent Drift', tag: 'gradient', duration: '0:44', active: false },
  { idx: 7, name: 'Polar Bloom', tag: 'kaleido', duration: '1:00', active: false },
  { idx: 8, name: 'Vector Pulse', tag: 'oscillator', duration: '0:36', active: false },
];

/** Scene meta panel — bundle's "Scene Info" header card data. */
export interface AuthorSceneMeta {
  readonly title: string;
  readonly credit: string;
  readonly license: string;
  readonly description: string;
  readonly thumbnail: string;
}

export const AUTHOR_SCENE_META: AuthorSceneMeta = {
  title: 'Smooth Kaleidoscope 2',
  credit: 'mrange',
  license: 'CC0',
  description:
    'License CC0: Smooth Kaleidoscope 2. Been working a bit too much and lacked ' +
    'the motivation do shaders for awhile. Wanted to do something simple...',
  thumbnail: 'smooth_kaleidoscope_2.png',
};

/** Control edit form data — the "rainbowize" slider being edited in the
 *  bundle's Controls sub-section. */
export interface AuthorControlEdit {
  readonly name: string;
  readonly group: string;
  readonly description: string;
  readonly type: string;
  readonly modifier: string;
  readonly min: string;
  readonly max: string;
  readonly default: string;
  readonly value: number;
}

export const AUTHOR_CONTROL_EDIT: AuthorControlEdit = {
  name: 'rainbowize',
  group: 'default',
  description: 'creates vivid rainbow bands',
  type: 'slider',
  modifier: 'none',
  min: '0',
  max: '1',
  default: '0',
  value: 0,
};

/* ---------- Tokenisation ---------- */

/* The bundle's regex-based highlighter is the canonical reference. Port
 * it here so the view template stays declarative — the function is a
 * pure transform from `(line, kind)` → `AuthorToken[]`. */

const GLSL_KEYWORDS = new Set([
  'uniform',
  'void',
  'vec2',
  'vec3',
  'vec4',
  'float',
  'bool',
  'mat2',
  'mat3',
  'mat4',
  'const',
  'return',
  'if',
  'else',
  'for',
  'in',
  'out',
  'define',
]);

const GLSL_UNIFORMS = new Set([
  'TIME',
  'RENDERSIZE',
  'RESOLUTION',
  'PI',
  'TAU',
  'gl_FragCoord',
  'gl_FragColor',
]);

const JS_KEYWORDS = new Set([
  'const',
  'let',
  'var',
  'function',
  'if',
  'else',
  'return',
  'new',
  'this',
]);

const JS_UNIFORMS = new Set(['ctx', 'Math', 'console']);

const GLSL_TOKEN_PATTERN =
  /(#define\b|\b(?:uniform|void|vec[234]|float|bool|mat[234]|const|return|if|else|for|in|out|define)\b|\b(?:TIME|RENDERSIZE|RESOLUTION|PI|TAU|gl_FragCoord|gl_FragColor|syn_\w+)\b|\b\d+\.?\d*\b)/g;

const JS_TOKEN_PATTERN =
  /(\b(?:const|let|var|function|if|else|return|new|this)\b|\b(?:ctx|Math|console)\b|\b\d+\.?\d*\b)/g;

const JS_STRING_PATTERN = /("[^"]*")/g;

/** Tokenise a single source line into a span list. Mirrors the bundle's
 *  `tokens()` regex but emits a structured array rather than React JSX. */
export function tokeniseLine(line: string, kind: AuthorHighlightKind): readonly AuthorToken[] {
  if (kind === 'plain') {
    return [{ kind: 'plain', text: line }];
  }
  if (kind === 'js') {
    return tokeniseJsLine(line);
  }
  return tokeniseGlslLine(line);
}

function tokeniseGlslLine(line: string): readonly AuthorToken[] {
  // Comment paths first — capture the comment to end-of-line and return.
  const commentIdx = findLineCommentStart(line, '//');
  if (commentIdx === 0) {
    return [{ kind: 'comment', text: line }];
  }
  if (commentIdx > 0) {
    const before = line.slice(0, commentIdx);
    const after = line.slice(commentIdx);
    return [
      ...splitByPattern(before, GLSL_TOKEN_PATTERN, classifyGlslWord),
      { kind: 'comment', text: after },
    ];
  }
  return splitByPattern(line, GLSL_TOKEN_PATTERN, classifyGlslWord);
}

function classifyGlslWord(word: string): AuthorTokenKind {
  if (word === '#define' || GLSL_KEYWORDS.has(word)) return 'keyword';
  if (GLSL_UNIFORMS.has(word) || word.startsWith('syn_')) return 'uniform';
  if (/^\d+\.?\d*$/.test(word)) return 'number';
  return 'plain';
}

function tokeniseJsLine(line: string): readonly AuthorToken[] {
  const commentIdx = findLineCommentStart(line, '//');
  if (commentIdx === 0) {
    return [{ kind: 'comment', text: line }];
  }
  let codePortion = line;
  let trailingComment: AuthorToken | null = null;
  if (commentIdx > 0) {
    trailingComment = { kind: 'comment', text: line.slice(commentIdx) };
    codePortion = line.slice(0, commentIdx);
  }
  // Strings first (capture as a single token, then tokenise the surroundings).
  const out: AuthorToken[] = [];
  const stringSpans = splitByPattern(codePortion, JS_STRING_PATTERN, () => 'string');
  for (const span of stringSpans) {
    if (span.kind === 'string') {
      out.push(span);
    } else {
      out.push(...splitByPattern(span.text, JS_TOKEN_PATTERN, classifyJsWord));
    }
  }
  if (trailingComment) out.push(trailingComment);
  return out;
}

function classifyJsWord(word: string): AuthorTokenKind {
  if (JS_KEYWORDS.has(word)) return 'keyword';
  if (JS_UNIFORMS.has(word)) return 'uniform';
  if (/^\d+\.?\d*$/.test(word)) return 'number';
  return 'plain';
}

/** Split a string by a sticky regex pattern, classifying each match
 *  via the supplied `classify` callback and emitting `plain` spans for
 *  the gaps between matches. The pattern MUST be `/g`-flagged. */
function splitByPattern(
  input: string,
  pattern: RegExp,
  classify: (word: string) => AuthorTokenKind,
): readonly AuthorToken[] {
  const out: AuthorToken[] = [];
  // Reset lastIndex so the global regex starts at 0 for each call.
  pattern.lastIndex = 0;
  let lastIdx = 0;
  // Use String.matchAll to avoid stateful iteration patterns.
  for (const match of input.matchAll(pattern)) {
    const matchIdx = match.index ?? 0;
    if (matchIdx > lastIdx) {
      out.push({ kind: 'plain', text: input.slice(lastIdx, matchIdx) });
    }
    const word = match[0];
    out.push({ kind: classify(word), text: word });
    lastIdx = matchIdx + word.length;
  }
  if (lastIdx < input.length) {
    out.push({ kind: 'plain', text: input.slice(lastIdx) });
  }
  return out;
}

/** Find the index of a line-comment marker that is not enclosed in a
 *  string literal. The bundle's regex is naive in this regard; this
 *  helper applies the same naive rule but stays explicit so future
 *  tightening is local. */
function findLineCommentStart(line: string, marker: string): number {
  let inString = false;
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i];
    if (ch === '"' && line[i - 1] !== '\\') {
      inString = !inString;
      continue;
    }
    if (!inString && line.startsWith(marker, i)) {
      return i;
    }
  }
  return -1;
}
