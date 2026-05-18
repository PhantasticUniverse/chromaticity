---
# SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
# SPDX-FileCopyrightText: 2026 Chromaticity
#
# Chromaticity design contract per google-labs-code/design.md spec
# (alpha 0.1.0, Apache-2.0). Token values mirror the operator-default
# "Pulse" theme; the alternate "Atelier" and "Console" themes are
# acknowledged in the Overview prose but not enumerated in the
# frontmatter — multi-theme spreads live in the planning bundle and
# land in `src/lib/tokens/tokens.css` (Pulse `:root` plus
# `[data-theme]` overrides) per `docs/47-ui-port-plan.md` §9.
#
# When this file disagrees with the rendered Svelte 5 + Tauri 2 shell:
# the shell wins and this file is updated in a follow-up `docs(repo)`
# commit. Prose explains the why; tokens are the contract.
version: alpha
name: chromaticity
description: |
  Live-visuals authoring + operator surface. Ships three built-in
  themes (Pulse / Atelier / Console) so users see the possibilities
  out of the box. Hardware-literate primitives, density-switchable
  scaling, tabular monospace for live numerics, neon-lime accent on
  the Pulse default.
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
    width: 52px
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
    clip-color: '{colors.danger}'
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
    height: 28px
    typography: '{typography.mono-sm}'
    color: '{colors.text-2}'
    border-top: '1px solid {colors.line-1}'
  focus-ring:
    width: 2px
    color: '{colors.primary}'
    offset: 2px
---

## Overview

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring environment. The default chrome runs dark and high-contrast so it doesn't compete with stage output, but the operator picks from three built-in themes — **Pulse**, **Atelier**, **Console** — based on mood. They're presets, not contracts; the three exist as a starter pack so users see the range right away.

- **Pulse** — dark canvas, neon-lime accent, soft glow, high saturation. The token values in the YAML frontmatter above are Pulse.
- **Atelier** — warm-paper editorial, zero glow, serif display (Fraunces).
- **Console** — desaturated hardware aesthetic, chicklet bevels, amber-LED accent.

Each theme swaps the color and elevation spreads but keeps typography, spacing, and component anatomy identical; the contract above stays valid for all three with re-themed values. The full multi-theme spread lives in the planning bundle until promoted into `src/lib/tokens/tokens.css` (Pulse `:root` plus `[data-theme]` overrides) per `docs/47-ui-port-plan.md` §9. A fourth theme is welcome as a design contribution — themes are aesthetic variety, not a closed contract.

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
[ 52px rail | 52px topbar      ]
[           | main view (scrolls) ]
[           | Snap bar (~56px, Perform only) ]
[           | 28px status bar  ]
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
- **Snap panel** — 8-slot grid above status bar, Perform view only. (Engine surface is 16 slots — `SnapSlotId::MAX = 15` — the SnapBar widget trims to the first 8 per the bundle's `extras.jsx` fixture.) Empty slots `text-4` / `bg-3`. Filled slots `text-1` / `bg-2` with category-colored 2px left accent. Active slot adds a `primary` underline. Each slot carries an operator-editable label below the slot number (localStorage-persisted; right-click to rename).
- **Adapter pill** — six glyphs `○ ◔ ● △ ✕ ⛉` for off / warming / ok / rate-limited / disconnected / blocked.
- **Feature-locked affordance** — `[data-cap]` declarative selector. MVP is watermark-only; non-watermark locks render under future `[data-cap-enforcement="denied"]` state at 0.68 opacity, desaturated fill, `cursor: not-allowed`, 10px lock glyph injected via `::after`. Unconditionally-free capabilities (`render.pause`, playlist nav, preset recall, Snap slots, `scene.export`, `transitions.off`) must never carry `[data-cap]`.

## Do's and Don'ts

**Do**

- Derive every spacing value from `spacing.*` tokens or the `--pad` / `--gap` density tokens. Inline numeric pixels are a code smell.
- Decouple `colors.primary` from data. Charts use `signal-*` and `band-*` so the user can re-theme without breaking semantics.
- Use `mono-md` or `timecode` with `tnum` for any number that changes during playback.
- Give icon-only buttons both `aria-label` and (when toggleable) `aria-pressed`.
- Honour `prefers-reduced-motion` — orbit rings, beat pulses, modulation halos, and ghost dots all collapse to instant.
- Verify new surfaces don't visually break under any of the three built-in themes. Fully theming a new surface for the operator default (Pulse) is the merge gate; full Atelier / Console parity is nice-to-have, not a release block.

**Don't**

- Don't invent new radii. The five-step `rounded` scale plus per-theme overrides covers every approved surface.
- Don't use `colors.primary` for destructive actions. `colors.danger` exists for that.
- Don't pile elevation. Two surface levels of contrast is enough for any panel-in-panel composition.
- Don't leak Pulse `--glow` into Atelier. The zero-glow rule on Atelier is what gives it its scenes-as-paper feel.
- Don't mix `band-*` and `signal-*` in the same chart. The audio→visual mapping breaks.
- Don't recreate competitor UI. The clean-room policy at `docs/29-cleanroom-legal-policy-baseline.md` is non-negotiable.

## Bundle alignment (2026-05-18)

Periodic comparison sweep against the canonical v0.2-beta UI bundle (`chromaticity-planning/ui-predev-bundle-v0.2-beta/`) per the wireframes-drive-UI/UX feedback memory. The shell wins, but the bundle is the wireframe contract; this section records where the shell currently exceeds the bundle (preserve), where the shell is at parity (no action), and where the shell trails (documented follow-ups, scope decisions for the AO).

### Where the shell EXCEEDS the bundle (preserve)

These divergences are explicit improvements and must not regress in future passes:

- **Semantic spacing scale.** Shell emits a 9-step `--space-*` ladder (`xs / sm / md / lg / xl / 2xl / 3xl / 4xl` per the frontmatter); bundle ships density tokens (`--pad / --gap / --knob / --track-h / --row-h / --fs`) plus raw pixel values. The `--space-*` ladder is the load-bearing primitive; density tokens compose from it.
- **Semantic typography tokens.** Shell emits role-keyed vars (`--font-size-display-lg`, `--font-size-timecode`, `--letter-spacing-caps-sm`, etc.) that the bundle ships as utility classes + inline sizes. The vars survive theme swaps without per-call-site editing.
- **TopBar adapter pills live-wired.** Shell's `OUT / AUDIO / MIDI / OSC` pills reflect `loadedSceneStore` + `audioSnapshotStore` + `midiStore` + `oscHealthStore` real cross-seam state; bundle ships a static `STATUS_PILLS` fixture.
- **AppShell explicit `grid-template-rows`** (`52px 1fr 28px`) + named grid areas (`topbar topbar / rail main / status status`); bundle uses implicit-row flex. The explicit grid is the more rigorous responsive contract.
- **SnapBar per-slot rename + localStorage labels** (Wave 2.4 PERF.F) — operator affordance the bundle has no equivalent of.
- **Perform variant switcher** (`STANDARD / CONSOLE`) ported with `localStorage`-persistent store; bundle's variant is a prop with no UI toggle.
- **Reduced-motion compliance.** Per-widget `matchMedia` gates plus positively-asserted frozen-frame test contracts (`BeatDots.test.ts` "pins the downbeat" / `BpmRing.test.ts` "pins beat 1" / `AudioReactivity.test.ts` "still renders 48 FFT bars under reduce"); bundle has CSS `animation: none` but no test parity.
- **Focus-trap discipline.** Five modal components implement first-and-last tab-cycle (`HotkeyOverlay.svelte`, `NewSceneModal`, `ConsoleDrawer`, `Settings` panels, `CommandPalette`); bundle's modal pattern is Esc-only.
- **`aria-live` taxonomy.** Nine `aria-live` call sites across console / import / inputs / author / banners; bundle's `v02_supplement.jsx` ships two. Verbose ARIA breadth in src/lib (~364 references) vs the bundle (~45 references).
- **rAF cancellation discipline.** Twelve components call `cancelAnimationFrame` in `$effect` teardown (Round 2 Lane C reactivity teardown fix); bundle JSX scaffold does not show explicit cancellation patterns.
- **WCAG-AA evidence.** `EV-ACCESSIBILITY-AA` axe-core T2 corpus runs against all 14 routed views; bundle ships hand-audit contrast table only.
- **Document-level paint guarantee.** `tokens.css` sets `html, body { background, color }` unconditionally; bundle's equivalent is conditional on density-fallback.
- **SPDX headers on tokens** (REUSE 3.2 compliance) — shell-side improvement.
- **Per-tick clean-room layered architecture** — typed `Command → Event` envelope + replay determinism + V2 forward migration + 7-class crash-resume restoration — none of which the bundle could express.

### Where the shell MATCHES the bundle (no action)

- Color palette (all 7 layers; per-theme variants per Pulse / Atelier / Console; signal / band / category fixtures).
- TopBar height 52px; status bar 28px (shell honours 28; bundle ships 26 — shell wins per Wave 2.3 Q.2 decision).
- SnapBar 8-slot grid (engine still 16; widget trims via `SNAP_SLOT_COUNT = 8`).
- Empty-state corpus (LIBRARY / LIBRARY_FILTERED / PLAYLIST / PLAYLIST_CUES / MIDI_DEVICES / OSC).
- Perform STANDARD layout grid (`minmax(0, 1fr) 420px`).

### Where the shell TRAILS the bundle (documented follow-ups)

Tracked as scope decisions for future waves; not pre-alpha-blocking. The AO decides scheduling:

- **Atelier display-font (Fraunces) not bound.** Bundle swaps `--font-display` to Fraunces under `[data-theme="atelier"]`; `tokens.css` defines `--font-family-display` as Pulse-only. §Typography promises Fraunces; sidecar OFL-1.1 binding deferred per `docs/51-ux-bundle-legal-check.md` §2. Defers on legal handling (OFL-1.1 sidecar packaging + REUSE 3.2 SPDX coverage) — AO-decision.
- **TopBar cell-divider chrome.** Bundle uses `.tb-section` with 1px right hairlines between brand / pills / metrics for the dense-instrument cadence; our TopBar is a flat 3-col grid with no internal dividers. DESIGN.md §Colors "hairlines over surface colour" spirit supports adding them. Defers on design call about brand placement.
- **SidebarRail `railtop` brand-mark cell.** Bundle's rail opens with a 40px railtop brand-mark; our rail goes straight into nav (brand-mark rides only in TopBar). Adding it would introduce visual brand-mark redundancy (rail + TopBar both carry the mark). Defers on design call.

### Where the shell CLOSED bundle-alignment gaps (post-2026-05-18 audit)

Items closed since the 2026-05-18 comparison sweep, with the PR that landed each closure:

- ✓ **Arrow-key slider nudge** on `Fader` / `Knob` / `CrossFader` — PR #491 (2026-05-18). Closes the WAI-ARIA APG slider keyboard-nudge gap on the three 1D widgets. 5 % / 20 % step, Home/End extremes, disabled blocked.
- ✓ **Global `*` reduced-motion kill-switch** in `tokens.css` — PR #491 (2026-05-18). Safety net under per-widget JS gates so a future decorative `@keyframes` degrades to instant by default.
- ✓ **`--shadow-inset` token** — Pulse `none`, Atelier `none`, Console chicklet-bevel `inset 0 1px 0 / inset 0 -1px 0` per DESIGN.md §Elevation.
- ✓ **Density tokens emitted** — `--pad / --gap / --knob / --track-h / --row-h / --fs` on `:root` (compact default) with `[data-density="comfortable"]` cascade widening to the comfortable values documented in DESIGN.md §Layout. Density switcher consumer TBD; tokens are now contract-available.
- ✓ **Per-theme radii overrides** — Atelier tightens to 2 / 2 / 4 / 8 px; Console tightens further to 2 / 2 / 3 / 4 px for hardware-instrument feel. Existing `var(--radius-*)` consumers across widgets pick up theme-overrides automatically.
- ✓ **Motion duration + easing token scale** — `--duration-fast / --duration-normal / --duration-slow` plus `--easing-out / --easing-in / --easing-inout`. Available for future widget retrofit; existing inline timings can be migrated incrementally.
- ✓ **z-index scale** — `--z-rail / --z-topbar / --z-overlay / --z-modal / --z-toast` for the five operator-visible stacking tiers. Future widgets compose from these tokens rather than raw integers.
- ✓ **StatusBar audio dBFS chip** — bundle-faithful `<Icon name="audio" /> {peak} dB` chip between MEM and warnings; reads `audioSnapshotStore.snapshot?.peak` (analysis-window peak per Round 3 Lane Q windowing) converted via `20 · log10`. `—` placeholder when no snapshot; `-∞ dB` floor on silent audio.
- ✓ **EmptyState `tertiaryAction`** — `EmptyStateCopy` widened with `readonly tertiaryAction?: EmptyStateAction`; `EMPTY_LIBRARY` now carries the bundle's 3-action pattern (Install Built-Ins / New Scene / Import). Consumers that render 3-action empty states iterate `[primary, secondary, tertiary].filter(Boolean)`. View-level adoption (Library.svelte rendering all three buttons) is a future visual-port follow-up.
- ✓ **`CueTransitionCurve` 2D keyboard nudge** — both bezier handles (cp1 / cp2) now respond to ArrowLeft / ArrowRight (X nudge) and ArrowUp / ArrowDown (Y nudge) per WAI-ARIA APG slider pattern extended for the 2D case. PageUp / PageDown nudge Y; Home / End snap X to extremes. 5 % small step / 20 % large step with Shift modifier. Clamped to [0, 1] both axes.

### Calibration anchors (false positives caught during the closeout)

- **Hotkey vocabulary** — the 2026-05-18 audit said "29 rows" but the actual `src/lib/data/hotkeys.ts` file ships **57 rows** (4 control-focused + 8 transport + 6 snap + 4 display + 6 rendering + 10 meta + 4 audio + 5 navigation + 5 diagnostics + 5 show-state). Trust-the-tree rule applies per CLAUDE.md "When this file is wrong"; no port needed.

Per the shell-wins rule, none of the still-open gaps invalidate the existing contract; they are operational improvements queued for a future `feat(ui)`-scope wave once the AO sets the priority. The clean-room `docs/29` posture applies — port behaviourally from the bundle, do not transcribe verbatim.
