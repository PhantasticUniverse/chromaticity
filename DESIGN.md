---
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# Chromaticity design contract per google-labs-code/design.md spec
# (alpha 0.1.0, Apache-2.0). Token values mirror the operator-default
# "Pulse" theme; the alternate "Atelier" and "Console" themes are
# acknowledged in the Overview prose but not enumerated in the
# frontmatter — multi-theme spread lives in `chromaticity-planning/`
# until `docs/47-ui-port-plan.md` §11 promotes per-theme token files.
#
# When this file disagrees with the rendered Svelte 5 + Tauri 2 shell:
# the shell wins and this file is updated in a follow-up `docs(repo)`
# commit. Prose explains the why; tokens are the contract.
version: alpha
name: chromaticity
description: |
  Operator-first, live-performance visual control surface. Three
  intentional themes (Pulse / Atelier / Console). Hardware-literate
  primitives, density-switchable scaling, tabular monospace for live
  numerics, neon-lime accent on Pulse defaults.
colors:
  bg-0: '#070810'
  bg-1: '#0C0E16'
  bg-2: '#11141D'
  bg-3: '#171A25'
  bg-4: '#1E2230'
  bg-5: '#262B3C'
  line-1: '#1E222E'
  line-2: '#2A3040'
  line-3: '#3A4158'
  text-0: '#F4F6FB'
  text-1: '#C9CFDB'
  text-2: '#838A9A'
  text-3: '#575E6E'
  text-4: '#3C4253'
  primary: '#C4FF3D'
  on-primary: '#0C0E16'
  success: '#3DE5FF'
  warning: '#FFB64D'
  danger: '#FF4D5E'
  info: '#3DE5FF'
  signal-cyan: '#3DE5FF'
  signal-magenta: '#FF3D9E'
  signal-orange: '#FF8A3D'
  signal-violet: '#9B6BFF'
  signal-lime: '#B8FF3D'
  signal-amber: '#FFB64D'
  signal-red: '#FF4D5E'
  band-bass: '#FF4D5E'
  band-mid: '#FFB64D'
  band-midhi: '#C4FF3D'
  band-high: '#3DE5FF'
  band-full: '#9B6BFF'
  cat-color: '#FF6B6B'
  cat-motion: '#3DE5FF'
  cat-fx: '#C4FF3D'
  cat-media: '#FFB64D'
  cat-limit: '#9B6BFF'
  cat-geom: '#3DE5FF'
  cat-general: '#8B93A3'
typography:
  display-lg:
    fontFamily: Inter
    fontSize: 28px
    fontWeight: 600
    lineHeight: 1.1
    letterSpacing: '-0.02em'
  display-md:
    fontFamily: Inter
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: '-0.02em'
  display-sm:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 600
    lineHeight: 1.0
    letterSpacing: '-0.02em'
  body-md:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.35
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.35
  label:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 500
    lineHeight: 1.2
  caps-sm:
    fontFamily: JetBrains Mono
    fontSize: 9px
    fontWeight: 500
    lineHeight: 1.0
    letterSpacing: '0.16em'
  caps-md:
    fontFamily: JetBrains Mono
    fontSize: 10px
    fontWeight: 500
    lineHeight: 1.0
    letterSpacing: '0.14em'
  mono-sm:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: 400
    lineHeight: 1.2
    fontFeature: '"tnum"'
  mono-md:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: 500
    lineHeight: 1.2
    fontFeature: '"tnum"'
  timecode:
    fontFamily: JetBrains Mono
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.0
    letterSpacing: '0.02em'
    fontFeature: '"tnum"'
rounded:
  xs: 2px
  sm: 3px
  md: 6px
  lg: 10px
  full: 999px
spacing:
  unit: 2px
  xs: 4px
  sm: 6px
  md: 8px
  lg: 12px
  xl: 16px
  2xl: 20px
  3xl: 24px
  4xl: 32px
components:
  rail:
    background: '{colors.bg-1}'
    width: 48px
    border-right: '1px solid {colors.line-1}'
  rail-icon:
    size: 36px
    color-resting: '{colors.text-3}'
    color-hover: '{colors.text-1}'
    color-active: '{colors.text-0}'
    background-active: '{colors.bg-2}'
    accent: '{colors.primary}'
  topbar:
    background: '{colors.bg-1}'
    height: 52px
    border-bottom: '1px solid {colors.line-1}'
  panel:
    background: '{colors.bg-2}'
    rounded: '{rounded.md}'
    padding: '{spacing.md}'
  panel-header:
    background: '{colors.bg-1}'
    height: 28px
    typography: '{typography.caps-md}'
    color: '{colors.text-2}'
  button-primary:
    background: '{colors.primary}'
    color: '{colors.on-primary}'
    typography: '{typography.label}'
    rounded: '{rounded.sm}'
    padding-x: '{spacing.lg}'
    padding-y: '{spacing.sm}'
  button-primary-hover:
    background: '{colors.primary}'
    glow: '0 0 12px {colors.primary}'
  button-ghost:
    background: 'transparent'
    color: '{colors.text-1}'
    border: '1px solid {colors.line-2}'
    rounded: '{rounded.sm}'
  button-danger:
    background: '{colors.danger}'
    color: '{colors.text-0}'
  fader:
    track-height: 2px
    track-color: '{colors.line-2}'
    track-fill: '{colors.primary}'
    thumb-size: 14px
    thumb-color: '{colors.primary}'
  knob:
    size-compact: 36px
    size-comfortable: 48px
    body: '{colors.bg-3}'
    indicator: '{colors.primary}'
    label-typography: '{typography.caps-sm}'
  toggle:
    track: '{colors.bg-3}'
    track-active: '{colors.primary}'
    thumb: '{colors.text-0}'
  pill:
    background: '{colors.bg-3}'
    color: '{colors.text-1}'
    typography: '{typography.caps-md}'
    rounded: '{rounded.full}'
    padding-x: '{spacing.md}'
  pill-cyan:
    accent: '{colors.signal-cyan}'
  pill-amber:
    accent: '{colors.signal-amber}'
  pill-red:
    accent: '{colors.signal-red}'
  meter:
    segment-color: '{colors.primary}'
    peak-color: '{colors.warning}'
    background: '{colors.bg-3}'
  timecode-display:
    typography: '{typography.timecode}'
    color: '{colors.primary}'
  tab:
    typography: '{typography.label}'
    color: '{colors.text-2}'
  tab-active:
    color: '{colors.text-0}'
    underline: '2px solid {colors.primary}'
  status-bar:
    background: '{colors.bg-1}'
    height: 22px
    typography: '{typography.mono-sm}'
    color: '{colors.text-2}'
    border-top: '1px solid {colors.line-1}'
  focus-ring:
    width: 2px
    color: '{colors.primary}'
    offset: 2px
---

## Overview

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring environment. Every pixel of chrome competes for attention with whatever the operator pushes to the stage output, so the design committed up front to **three intentional themes** rather than a neutral default:

- **Pulse** — dark canvas, neon-lime accent, soft glow, high saturation. Operator default. The token values in the YAML frontmatter above are Pulse.
- **Atelier** — warm-paper editorial, zero glow, serif display (Fraunces). Used during scene authoring when the operator is reading prose, not driving live.
- **Console** — desaturated hardware aesthetic, chicklet bevels, amber-LED accent. Mimics the look of a physical lighting desk for muscle-memory transfer.

Atelier and Console swap the color and elevation spreads but keep typography, spacing, and component anatomy identical; the contract above stays valid for all three with re-themed values. The full multi-theme spread lives in the planning bundle until promoted per `docs/47-ui-port-plan.md` §11.

The non-cosmetic intent behind the design:

1. **Live-first hierarchy.** Anything that changes in real time (timecode, beat dots, meters, FPS, BPM) is monospace with tabular numerals, tinted with `colors.primary` on Pulse, and ringed by a `--glow` coefficient that scales per theme. Static labels and metadata recede into `text-2` and `text-3`.
2. **Density as a first-class control.** The operator flips the entire UI between `compact` and `comfortable` density; row heights, knob diameters, and gutter sizes derive from a single set of density tokens so nothing escapes the switch.
3. **Hardware literacy.** Controls look like controls. Faders have 2px tracks and circular thumbs; knobs are 36 / 48 px discs with detents; pins on the Patch graph are colored by their audio band. The mouse is secondary; keyboard and MIDI are primary.

This document is the **token contract**. The Svelte 5 + Tauri 2 shell consumes it; AI tools that read `google-labs-code/design.md` consume it; reviewers cross-check it against `docs/adr/ADR-018-non-scene-ui-toolkit.md`. When prose disagrees with rendered behavior, the shell wins and this file is updated in a follow-up `docs(repo)` commit.

## Colors

The palette splits into seven layers, each with a distinct semantic job. Mixing layers (e.g. using a `band` hue for an action button) is prohibited — it severs the audio→visual mapping that the operator relies on for muscle memory.

- **Surfaces** (`bg-0` … `bg-5`). Six elevations from canvas floor to hovered affordance. Elevation, not shadow, is the primary depth signal.
- **Hairlines** (`line-1` … `line-3`). Three contrast levels for borders and dividers; never use surface colors as borders.
- **Text** (`text-0` … `text-4`). Five-level hierarchy. `text-4` is disabled-only and never used for resting copy.
- **Primary** (`primary`, `on-primary`). The single user-switchable accent. Used **only** for the most important action on a screen, the active tab underline, the live timecode, and the rail indicator.
- **Semantic state** (`success`, `warning`, `danger`, `info`). `danger` is reserved for destructive confirmations and the REC button — never decorative.
- **Signal palette** (`signal-*`). Seven fixed hues for data visualization. Independent of the user-switched accent so charts retain their meaning across theme swaps.
- **Audio bands** (`band-bass`, `band-mid`, `band-midhi`, `band-high`, `band-full`). Five-band canonical spectrum mapping. Pin/wire/mod-source coloring uses these.
- **Scene categories** (`cat-color`, `cat-motion`, `cat-fx`, `cat-media`, `cat-limit`, `cat-geom`, `cat-general`). Fixed per-function. Persist across theme swaps.

WCAG 2.1 SC 1.4.3 AA contrast is verified for all 16 foreground/background pairs across all three themes; the contrast registry lives in the planning bundle.

## Typography

Three families:

- **Inter** (sans-serif, OFL-1.1) — body, labels, panel headers, all sustained reading.
- **JetBrains Mono** (monospace, OFL-1.1) — live numerics (always with `font-feature-settings: "tnum"`), capitalised labels, metadata.
- **Fraunces** (display serif, OFL-1.1) — Atelier theme only. Brand wordmark and section titles. Optional at V1.

Fonts ship bundled under `fonts/<family>/` with the OFL-1.1 sidecar at promote-time per `docs/51-ux-bundle-legal-check.md` §2; the dev mock falls back to system stacks until then.

The scale (eleven steps in the frontmatter) is intentionally compact — no `display-xl`, no `body-lg`. The operator looks at sustained chrome from a stage-distance viewing position; oversized type wastes density without improving legibility.

Live numerics use `mono-md` or `timecode` and apply `--glow` per theme. The 9px floor on `caps-sm` is the smallest readable mono-tabular text in the stage-distance test; do not go smaller.

## Layout

The shell grid is fixed:

```
[ 48px rail | 52px topbar      ]
[           | main view (scrolls) ]
[           | Snap bar (~56px, Perform only) ]
[           | 22px status bar  ]
```

The rail is the persistent left navigation. The topbar carries product identity (left), live HUD on Perform/Scene/Audio views (center: timecode / beat dots / BPM / VU stereo), and adapter pills + stats + action cluster (right: Screenshot / Record / GO LIVE, always in that order). Only the main view scrolls.

**Density tokens** govern responsive scaling between `compact` and `comfortable`:

| token       | compact | comfortable |
| ----------- | ------- | ----------- |
| `--pad`     | 6px     | 10px        |
| `--gap`     | 6px     | 10px        |
| `--knob`    | 36px    | 48px        |
| `--track-h` | 64px    | 84px        |
| `--row-h`   | 24px    | 30px        |
| `--fs`      | 12px    | 13px        |

The `spacing` scale in the frontmatter is the underlying primitive (2px base, named steps); density tokens compose from it.

## Elevation & Depth

Elevation is signalled by **surface choice**, not shadow. The `bg-0` … `bg-5` ladder is the primary depth contract.

Theme-defined `--shadow-panel` and `--glow` coefficients add the secondary depth signal:

- **Pulse** — soft top-edge highlight + subtle drop shadow + `--glow: 1.0` (live controls emit a soft halo).
- **Atelier** — hairline-only borders, no shadow, no glow (`--glow: 0.0`). Scenes-as-paper aesthetic.
- **Console** — chicklet bevels (1px inset highlight + 1px outset shadow) + `--glow: 0.3` (LED-style restraint).

Modal dialogs sit above a scrim at `bg-0` × 0.6 alpha, with the modal panel at `bg-2` and `rounded.lg`. Modals do not receive `--glow`; the scrim provides isolation.

## Shapes

Five radius tokens, intentionally tight to signal "instrument" rather than "friendly consumer software." `rounded.lg` (10px) is the largest non-pill radius and is reserved for modal panels and inspector cards. The full pill radius (999px) appears only in pills and badges.

Per-theme overrides (Atelier and Console pull radii inward toward 2-4px) live in the planning bundle. The frontmatter encodes Pulse.

Circles only appear in three places: the rail active indicator (2px stadium), the fader thumb (14px disc), and the brand wordmark (22px disc). Do not introduce new circular shapes without an ADR.

## Components

The frontmatter encodes the core 22 primitives. Variants live as suffix-keyed siblings (`button-primary` / `button-primary-hover` / `button-danger`) per the google-labs spec.

Beyond the frontmatter contract, the operator-facing surface includes these primitives whose anatomy is documented in the port plan but whose tokens compose from the contract above:

- **Tabs (segmented)** — underline style. Active tab gets `text-0` plus `tab-active.underline`. No pill treatment.
- **Spectrum** — audio band coloring per `band-*`. LED-segment style (uniform bar heights, not staircase). Peak-hold tick.
- **VU stereo** — L/R level pair with peak-hold and live animation.
- **Cue transition curve** — SVG bezier editor; three modes (Cut / Fade / Beat Snap). Respects `prefers-reduced-motion`.
- **Empty state** — dotted orbit-ring (40s slow rotation, reduce-motion-safe) + accent-glowing icon disc + title + body + action row + mono meta.
- **Snap panel** — 16-slot grid above status bar, Perform view only. Empty slots `text-4` / `bg-3`. Filled slots `text-1` / `bg-2` with category-colored 2px left accent. Active slot adds a `primary` underline.
- **Adapter pill** — six glyphs `○ ◔ ● △ ✕ ⛉` for off / warming / ok / rate-limited / disconnected / blocked.
- **Feature-locked affordance** — `[data-cap]` declarative selector. MVP is watermark-only; non-watermark locks render under future `[data-cap-enforcement="denied"]` state at 0.68 opacity, desaturated fill, `cursor: not-allowed`, 10px lock glyph injected via `::after`. Unconditionally-free capabilities (`render.pause`, playlist nav, preset recall, Snap slots, `scene.export`, `transitions.off`) must never carry `[data-cap]`.

## Do's and Don'ts

**Do**

- Derive every spacing value from `spacing.*` tokens or the `--pad` / `--gap` density tokens. Inline numeric pixels are a code smell.
- Decouple `colors.primary` from data. Charts use `signal-*` and `band-*` so the user can re-theme without breaking semantics.
- Use `mono-md` or `timecode` with `tnum` for any number that changes during playback.
- Give icon-only buttons both `aria-label` and (when toggleable) `aria-pressed`.
- Honour `prefers-reduced-motion` — orbit rings, beat pulses, modulation halos, and ghost dots all collapse to instant.
- Ship every new surface in **all three** themes before merging to `main`.

**Don't**

- Don't introduce a fourth theme. The three-theme commitment is normative; alternates land via theme-internal token spreads, not new themes.
- Don't invent new radii. The five-step `rounded` scale plus per-theme overrides covers every approved surface.
- Don't use `colors.primary` for destructive actions. `colors.danger` exists for that.
- Don't pile elevation. Two surface levels of contrast is enough for any panel-in-panel composition.
- Don't leak Pulse `--glow` into Atelier. The zero-glow rule on Atelier is what gives it its scenes-as-paper feel.
- Don't mix `band-*` and `signal-*` in the same chart. The audio→visual mapping breaks.
- Don't recreate competitor UI. The clean-room policy at `docs/29-cleanroom-legal-policy-baseline.md` is non-negotiable.
