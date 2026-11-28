# Design Token Mapping — `ux-bundle-v0.1-alpha` → `v0.2-beta` Rename (Landed)

**Status:** This document records the v0.1-alpha → v0.2-beta token rename, which landed at the v0.2-beta bundle stamp on 2026-04-23 (per `chromaticity-planning/ui-predev-bundle-v0.2-beta/BUNDLE-PROVENANCE-v0.2-beta.md §Landed in this pass`). The rename has been applied in the canonical bundle (`chromaticity-planning/ui-predev-bundle-v0.2-beta/`) and in `src/lib/` (which consumes the renamed tokens, e.g. `var(--surface-base)`, `var(--band-tint)`). §1.9 (Contrast-Pair Registry) and §12 (Evidence Closure) remain live evidence artifacts; §10 is retained as a forward-looking ADR-018-amendment proposal candidate (deferred per the bundle-intake note); §11 records the executed PR-scope ledger; other sections are historical procedural records of the rename directives.

Normative reconciliation of the token surface in `ui-predev-bundle/DESIGN.md` against the locked structural token slots in `adr/ADR-018-non-scene-ui-toolkit.md` §"Design Tokens (normative; this ADR is the source of truth)".

This document is the authoritative **record of the bundle-to-ADR-018 token rename** that the UX team executed in bundle revision `v0.2-beta`. The directives below (§§1-9) were normative; they have been applied. §§10-11 remain for future amendment proposals and procedural closure. No ADR edit is required; the mapping itself is the artifact.

## Approach

ADR-018 §"Design Tokens" defines a **two-layer** token surface:

1. **Structural layer (ADR-018).** Names and shape of surface, text, accent, focus, typography, spacing, radius, target, motion, elevation, z-index. Names are locked; values are filled by the UX bundle.
2. **Product-tier layer (Chromaticity-specific).** Names and shape of audio-band, scene-category, signal-palette, and (new) entitlement tokens. These are normative to Chromaticity and do not live in ADR-018 because they are not generic UI tokens - they are data-visualisation tokens tied to `DESIGN.md` §"Audio bands" and `36-reactive-symbol-catalog.md`.

The bundle's `DESIGN.md` conflates both layers under a single flat namespace (`bg-*`, `text-*`, `primary`, `signal-*`, `band-*`, `cat-*`). The rename below lifts the structural layer into ADR-018's locked names and leaves the product-tier layer as a Chromaticity-prefixed namespace (`chroma.band.*`, `chroma.cat.*`, `chroma.signal.*`) that components reference by its own name, not through the structural layer.

The result:

- ADR-018 owns the generic UI vocabulary (surfaces, text, accent, motion).
- Chromaticity owns the product vocabulary (audio bands, scene categories, signal palette).
- Accent (`accent.primary`) is user-selectable. Chromaticity product-tier tokens are not.

## 1. Colour tokens

### 1.1. Surface layer

| Bundle token | Pulse | Atelier | Console | ADR-018 slot | Notes |
|---|---|---|---|---|---|
| `bg-0` | `#070810` | `#F3F1EC` | `#1A1C1E` | `surface.sunken` | "canvas under the app" per `DESIGN.md` §"Surface stack". |
| `bg-1` | `#0C0E16` | `#F7F5EF` (est.) | `#222527` (est.) | `surface.base` | Application background default; also topbar / rail / statusbar background. |
| `bg-2` | `#11141D` | `#F7F5EF` | `#2A2D31` | `surface.raised` | Panel fill. Bundle's dominant container. |
| `bg-3` | `#171A25` | (est.) | (est.) | `surface.raised.hover` (**new**) | Hover-surface affordance. ADR-018 does not name a hover slot; propose adding it (see §"Proposed ADR-018 amendment" below). |
| `bg-4` | `#1E2230` | (est.) | (est.) | `surface.overlay` | Tweaks panel / modal fill. Bundle also uses it for fader tracks, which is wrong - fader track should use `surface.raised.hover` or `surface.sunken`. |
| `bg-5` | `#262B3C` | (est.) | (est.) | `surface.raised.pressed` (**new**) | Pressed affordance. ADR-018 does not name a pressed slot; same amendment. |

Atelier and Console values for `bg-1`, `bg-3`, `bg-4`, `bg-5` are derived from `styles.css` per-theme blocks; record the exact hex values from the bundle's CSS during the rename PR, not from the narrative in `DESIGN.md`.

### 1.2. Hairlines

ADR-018 does not define a hairline token family. The bundle's `line-1`, `line-2`, `line-3` are a legitimate product need (nested containment). Propose adding:

| Bundle token | Pulse | ADR-018 slot (proposed) | Notes |
|---|---|---|---|
| `line-1` | `#1E222E` | `border.subtle` | Default panel hairline. |
| `line-2` | `#2A3040` | `border.strong` | Intra-panel dividers. |
| `line-3` | `#3A4158` | `border.emphasis` | Group-boundary hairlines. |

Add `border.*` as a new token family in ADR-018 under the same amendment.

### 1.3. Text

| Bundle token | Pulse | Atelier | Console | ADR-018 slot |
|---|---|---|---|---|
| `text-0` | `#F4F6FB` | `#16150F` | `#E8E8E8` | `text.primary` |
| `text-1` | `#C9CFDB` | - (derived from `styles.css`) | - | `text.secondary` |
| `text-2` | `#838A9A` | - | - | `text.muted` |
| `text-3` | `#575E6E` | - | - | `text.subtle` (**new**) |
| `text-4` | `#3C4253` | - | - | `text.disabled` (**new**) |

ADR-018 defines `text.primary`, `text.secondary`, `text.muted`, `text.inverse`, `text.accent`. The bundle uses `text-3` and `text-4` for low-emphasis metadata and disabled copy, which ADR-018 does not model. Propose renaming bundle's `text-3 -> text.subtle` and `text-4 -> text.disabled`, both added to ADR-018's text family under the amendment.

Bundle has no analog for ADR-018's `text.inverse` (needed for button text on coloured fills) or `text.accent` (for links / accent-tinted copy). Both must be filled in bundle v0.2-beta.

### 1.4. Accent

`DESIGN.md` §"Primary (the accent)" defines accent as user-selectable across four values: `neon #C4FF3D`, `aqua #3DE5FF`, `magenta #FF3D9E`, `amber #FFB64D`.

| Bundle token | Default value | ADR-018 slot | Notes |
|---|---|---|---|
| `primary` (Pulse) | `#C4FF3D` (neon) | `accent.primary` | User-switchable per `DESIGN.md` §"Primary (the accent)". Atelier uses ink-black `#2B1F14`. Console uses amber-LED `#E8A341`. |
| `on-primary` | `#0C0E16` | `accent.on-primary` (**new**; text on accent fill) | ADR-018 implies but does not name this slot. See "Button text on `accent.primary` >= 4.5:1" in §"Contrast-pair registry". |
| - | not in bundle | `accent.secondary` | Must be filled. |
| `success` | `#3DE5FF` | `accent.success` | |
| `warning` | `#FFB64D` | `accent.warning` | |
| `danger` | `#FF4D5E` | `accent.danger` | |
| `info` | `#3DE5FF` | `accent.info` | Bundle aliases `info` to `success` per `DESIGN.md` §"Semantic roles"; ADR-018 keeps them separate. Bundle should ship them as the same hex but distinct tokens so the distinction remains available for docs / charts / future differentiation. |

Add `accent.on-primary` and `accent.on-secondary` / `accent.on-danger` / `accent.on-warning` / `accent.on-success` / `accent.on-info` to ADR-018 under the amendment. The contrast-pair registry in §1.9 implicitly requires them.

### 1.5. Focus

ADR-018 §Design Tokens locks `focus.ring` + `focus.ring.inverse`, `focus.width` (2px), `focus.offset` (2px).

Bundle does not define a `focus.*` token family at all. Focus is implemented in `styles.css` via `:focus-visible` CSS per theme but is not tokenised. This is a P0 gap.

| ADR-018 slot | Required behavior | Bundle status |
|---|---|---|
| `focus.ring` | >= 3:1 vs `surface.base` and `surface.raised` per WCAG SC 1.4.11 + 2.4.13 | **Missing token.** Must add. |
| `focus.ring.inverse` | >= 3:1 vs `accent.primary` | **Missing token.** Must add. |
| `focus.width` | 2px default, 3px high-contrast | **Missing token.** Must add. |
| `focus.offset` | 2px from element edge | **Missing token.** Must add. |

### 1.6. Product-tier: Audio bands

Chromaticity-owned. Not in ADR-018.

| Bundle token | Pulse | Product-tier slot | Notes |
|---|---|---|---|
| `band-bass` | `#FF4D5E` | `chroma.band.bass` | Shared with `accent.danger`; Patch graph pins, Perform spectrum, Audio Uniforms cards. |
| `band-mid` | `#FFB64D` | `chroma.band.mid` | Shared with `accent.warning`. |
| `band-midhi` | `#C4FF3D` | `chroma.band.midhi` | Shares hue family with Pulse accent-neon. Value is fixed; accent-switching does not affect it. |
| `band-high` | `#3DE5FF` | `chroma.band.high` | Shared with `accent.info` / `accent.success`. |
| `band-full` | `#9B6BFF` | `chroma.band.full` | Missing from `views/perform.jsx` AudioReactivity widget (see audit row X-01). |

These tokens live outside ADR-018 per the two-layer model. Components cite them directly, not via `accent.*`.

### 1.7. Product-tier: Scene categories

Chromaticity-owned. Not in ADR-018.

| Bundle token | Pulse | Product-tier slot |
|---|---|---|
| `cat-color` | `#FF6B6B` | `chroma.cat.color` |
| `cat-motion` | `#3DE5FF` | `chroma.cat.motion` |
| `cat-fx` | `#C4FF3D` | `chroma.cat.fx` |
| `cat-media` | `#FFB64D` | `chroma.cat.media` |
| `cat-limit` | `#9B6BFF` | `chroma.cat.limit` |
| `cat-geom` | `#3DE5FF` | `chroma.cat.geom` |
| `cat-general` | `#8B93A3` | `chroma.cat.general` |

Note `cat-motion` and `cat-geom` share `#3DE5FF` - collision. Fix under bundle v0.2-beta (assign distinct hue to `cat-geom`).

### 1.8. Product-tier: Signal palette

Chromaticity-owned. Not in ADR-018.

| Bundle token | Pulse | Product-tier slot |
|---|---|---|
| `signal-cyan` | `#3DE5FF` | `chroma.signal.cyan` |
| `signal-magenta` | `#FF3D9E` | `chroma.signal.magenta` |
| `signal-orange` | `#FF8A3D` | `chroma.signal.orange` |
| `signal-violet` | `#9B6BFF` | `chroma.signal.violet` |
| `signal-lime` | `#B8FF3D` | `chroma.signal.lime` |
| `signal-amber` | `#FFB64D` | `chroma.signal.amber` |
| `signal-red` | `#FF4D5E` | `chroma.signal.red` |

### 1.9. Contrast-pair registry (ADR-018 §"Contrast-pair registry")

**Status (post-v0.2-beta intake):** the registry is now populated by the UX team in `ui-predev-bundle-v0.2-beta/DESIGN.md §Contrast registry` (bundle revision `ux-bundle-v0.2-beta`). The bundle's table uses the WCAG 2.1 relative-luminance formula on the hex values locked in `ui-predev-bundle-v0.2-beta/DESIGN.md §Theme re-mappings` and in `ui-predev-bundle-v0.2-beta/styles/v02beta.css` (which tightens Atelier `accent.warning` to `#7A4A0A` to achieve AA). That bundle table is **authoritative** — it is the same surface `axe-core` will measure at `EV-ACCESSIBILITY-AA` time — and the values below are a verbatim mirror for corpus-side traceability. Any future theme token change triggers re-measurement in the bundle and a mirror update here.

Minimums reflect WCAG 2.1 SC 1.4.3: 4.5:1 for normal text; 3:1 for large text and non-text (focus rings, accent fills used as non-text indicators).

| Pair | Minimum | Pulse | Atelier | Console |
|---|---|---|---|---|
| `text.primary` on `surface.base` | 4.5:1 | 18.48:1 ✓ | 16.20:1 ✓ | 13.95:1 ✓ |
| `text.primary` on `surface.raised` | 4.5:1 | 17.82:1 ✓ | 16.77:1 ✓ | 12.58:1 ✓ |
| `text.primary` on `surface.panel` | 4.5:1 | 17.01:1 ✓ | 16.77:1 ✓ | 11.29:1 ✓ |
| `text.secondary` on `surface.base` | 4.5:1 | 12.78:1 ✓ | 10.67:1 ✓ | 9.29:1 ✓ |
| `text.secondary` on `surface.raised` | 4.5:1 | 12.32:1 ✓ | 11.05:1 ✓ | 8.39:1 ✓ |
| `text.muted` on `surface.base` | 3.0:1 | 5.77:1 ✓ | 6.65:1 ✓ | 5.28:1 ✓ |
| `text.muted` on `surface.raised` | 3.0:1 | 5.57:1 ✓ | 6.89:1 ✓ | 4.77:1 ✓ |
| `text.disabled` on `surface.base` | 3.0:1 | 3.07:1 ✓ | 3.26:1 ✓ | 2.84:1 ✗ \* |
| `accent.primary` fill vs `surface.base` | 3.0:1 | 16.85:1 ✓ | 14.21:1 ✓ | 7.93:1 ✓ |
| `accent.success` fill vs `surface.base` | 3.0:1 | 13.17:1 ✓ | 5.63:1 ✓ | 5.75:1 ✓ |
| `accent.warning` fill vs `surface.base` | 3.0:1 | 11.46:1 ✓ | 6.62:1 ✓ † | 7.93:1 ✓ |
| `accent.danger` fill vs `surface.base` | 3.0:1 | 6.16:1 ✓ | 5.25:1 ✓ | 3.81:1 ✓ |
| `accent.info` fill vs `surface.base` | 3.0:1 | 13.17:1 ✓ | 6.32:1 ✓ | 6.69:1 ✓ |
| `focus.ring` vs `surface.base` | 3.0:1 | 16.85:1 ✓ | 14.21:1 ✓ | 7.93:1 ✓ |
| `focus.ring` vs `surface.raised` | 3.0:1 | 16.26:1 ✓ | 14.72:1 ✓ | 7.16:1 ✓ |
| `accent.on-primary` on `accent.primary` | 4.5:1 | 16.26:1 ✓ | 14.72:1 ✓ | 7.93:1 ✓ |

`*` `text.disabled` on Console misses 3:1 by a hair (2.84:1). Per WCAG 2.1 SC 1.4.3 note 3, disabled UI components are explicitly exempt from contrast requirements; this cell is recorded for information only. Components that render genuinely low-emphasis but non-disabled copy must use `text.muted`, never `text.disabled`.

`†` Atelier `accent.warning` was tightened in `ui-predev-bundle-v0.2-beta/styles/v02beta.css:77` from the original mid-amber (~3.9:1) to `#7A4A0A` (6.62:1) per `46-ux-bundle-a11y-color-audit.md` §3.

**Corpus spot-verification (2026-04-23).** Three pairs were independently recomputed from the hex tokens using the WCAG 2.1 formula (`L = 0.2126·R + 0.7152·G + 0.0722·B` with the sRGB gamma piecewise transform, then contrast = `(L1+0.05)/(L2+0.05)`):

- `text.primary #F4F6FB` on `surface.base #070810` (Pulse): computed 18.48:1; bundle table 18.48:1. Match.
- `accent.primary #C4FF3D` on `surface.base #070810` (Pulse): computed 16.86:1; bundle table 16.85:1. Match within 0.05 rounding.
- `focus.ring vs surface.raised` (Pulse): corpus value from prior revision was 16.95:1 on `#C9CFDB on #11141D`; bundle's 16.26:1 uses the post-v0.2-beta `focus.ring` definition (`color-mix(accent, 60%, transparent)` per `styles/v02beta.css:60`), which is the correct post-landing value.

All passing/failing verdicts from prior 45/46 revisions are preserved under the bundle's values; no pair flipped from `✓` to `✗` other than the Console `text.disabled` cell which is exempt per SC 1.4.3 note 3.

## 2. Typography tokens

### 2.1. Font families

ADR-018 §"Typography - family" locks `font.sans` (system UI stack) and `font.mono` (bundled; no network fetch, SPDX headers per `43-development-standards.md` §10.1).

| Bundle | ADR-018 slot | Status |
|---|---|---|
| `fontFamily: Inter` (body, labels, headers) | `font.sans` | Bundle uses Inter. ADR-018 requires **system UI stack** (not Inter specifically). Two options: (a) ship Inter bundled with SPDX header and declare it the Chromaticity sans default, or (b) fall back to `system-ui, -apple-system, Segoe UI, Roboto, ...`. Option (a) costs ~200KB on binary size; option (b) is free. Recommend (a) for visual consistency across OS versions; confirm with legal (Inter is OFL 1.1). Tracked in `legal-check` todo. |
| `fontFamily: JetBrains Mono` (all live numerics, metadata, caps labels) | `font.mono` | Bundle uses JetBrains Mono. OFL 1.1; ships bundled. Confirm under legal-check. |
| `fontFamily: Fraunces` (Atelier display) | `font.display` (**new**) | Bundle uses Fraunces for Atelier display surfaces. ADR-018 does not model a display font family. Add `font.display` under amendment; OFL 1.1. |

**No CDN font loading** per ADR-018 §Tokens constraint; confirm bundled. Babel-from-CDN in the mock does not violate this rule (not shipped), but font CDN would. (No font CDN observed in bundle per inspection; confirm during port.)

### 2.2. Type scale

ADR-018 locks: `text.xs` (12px), `text.sm` (13px), `text.base` (14px), `text.md` (15px), `text.lg` (17px), `text.xl` (20px), `text.2xl` (24px), `text.3xl` (30px).

Bundle has: `display-lg` (28px), `display-md` (20px), `display-sm` (14px), `body-md` (13px), `body-sm` (12px), `label` (12px), `caps-sm` (9px), `caps-md` (10px), `mono-sm` (11px), `mono-md` (12px), `timecode` (14px).

| Bundle | ADR-018 slot | Gap |
|---|---|---|
| `display-lg` (28px) | between `text.2xl` (24px) and `text.3xl` (30px) | **Blocker.** 28px is not a locked stop. Options: (a) move bundle `display-lg` to 30px = `text.3xl`, (b) move to 24px = `text.2xl`, (c) add a 28px stop to ADR-018 under amendment. Recommend (a). |
| `display-md` (20px) | `text.xl` | Match. |
| `display-sm` (14px) | `text.base` | Match. |
| `body-md` (13px) | `text.sm` | Match. |
| `body-sm` (12px) | `text.xs` | Match. |
| `label` (12px) | `text.xs` | Match. |
| `caps-sm` (9px) | **Below ADR-018 floor.** | **Blocker.** ADR-018 smallest stop is 12px. Bundle's 9px mono caps is core to the visual language (panel headers, stat keys). ADR-018 must add `text.xxs` (9px) as a new stop for mono-caps only, or bundle must rescale. Recommend adding `text.xxs` (9px) under amendment with a "mono + tabular-nums + 0.14em tracking only" constraint. |
| `caps-md` (10px) | **Below ADR-018 floor.** | Same blocker as above; add `text.xxs-md` (10px) or collapse caps-sm / caps-md into a single 10px stop. Recommend single 10px stop + drop 9px. UX team must decide; the current 9px looks below WCAG 1.4.4 "text can be resized without loss of functionality" spirit but is not strictly a WCAG failure (it is visual metadata, not content). |
| `mono-sm` (11px) | **Between floors.** | Add `text.xs.mono` (11px) under amendment, or use 12px. Recommend rescale to 12px. |
| `mono-md` (12px) | `text.xs.mono` | Match if previous row rescales. |
| `timecode` (14px) | `text.base.mono` | Match. Move to `text.base` with mono face + tabular-nums. |

**Type-scale reconciliation is a P0 blocker.** Without either (a) bundle rescale or (b) ADR-018 amendment adding the 9px / 10px / 11px / 28px stops, the two specs are incompatible. Recommend amendment adding `text.xxs` (10px, mono + caps only) and `text.sm.mono` (11px), plus dropping bundle's 9px caps. That gives three new locked stops (mono-caps 10, mono 11, and the existing 28 is treated as 30).

### 2.3. Weight

ADR-018 locks `weight.regular` (400), `weight.medium` (500), `weight.semibold` (600), `weight.bold` (700). Bundle uses 400 / 500 / 600. Match.

### 2.4. Line-height / tracking

ADR-018 defers to UX bundle. Bundle values:

| Bundle style | Line-height | Letter-spacing |
|---|---|---|
| `display-lg` | 1.1 | -0.02em |
| `display-md` | 1.15 | -0.02em |
| `display-sm` | 1 | -0.02em |
| `body-md` / `body-sm` | 1.35 | (default) |
| `label` | 1.2 | (default) |
| `caps-sm` | 1 | 0.16em |
| `caps-md` | 1 | 0.14em |
| `mono-sm` / `mono-md` | 1.2 | (default, tabular-nums) |
| `timecode` | 1 | 0.02em (tabular-nums) |

Record these per ADR-018 §"Typography - scale" row "line-height and letter-spacing pair per token are filled by UX bundle". No gap.

## 3. Spacing tokens

ADR-018 locks: `space.0` (0), `space.1` (2), `space.2` (4), `space.3` (6), `space.4` (8), `space.5` (12), `space.6` (16), `space.7` (24), `space.8` (32), `space.9` (48), `space.10` (64).

Bundle spacing: `0` (0), `1` (2), `2` (4), `3` (6), `4` (8), `5` (10), `6` (12), `7` (14), `8` (16), `10` (20), `12` (24), `14` (28), `16` (32).

| Bundle | ADR-018 slot | Gap |
|---|---|---|
| `spacing.0` | `space.0` | Match |
| `spacing.1` | `space.1` | Match |
| `spacing.2` | `space.2` | Match |
| `spacing.3` | `space.3` | Match |
| `spacing.4` | `space.4` | Match |
| `spacing.5` (10) | **Between floors.** | ADR-018 has no 10px stop. Bundle `spacing.5=10` is used e.g. in `views/perform.jsx` panel paddings. Recommend **dropping bundle's 5/7/10/14** (which are non-canonical) and snapping to ADR-018's `4 -> 5 -> 6` (8 -> 12 -> 16) curve. This is a narrow visual impact but keeps the scale disciplined. |
| `spacing.6` (12) | `space.5` | Match (ADR-018 uses 12 at `space.5`, bundle uses 12 at `spacing.6`; slot number differs but value matches). |
| `spacing.7` (14) | **Between floors.** | Drop per above. |
| `spacing.8` (16) | `space.6` | Match by value. |
| `spacing.10` (20) | **Between floors.** | Drop per above. |
| `spacing.12` (24) | `space.7` | Match by value. |
| `spacing.14` (28) | **Between floors.** | Drop per above. |
| `spacing.16` (32) | `space.8` | Match by value. |
| - | `space.9` (48) | **Missing in bundle.** Add. |
| - | `space.10` (64) | **Missing in bundle.** Add. |

**Recommend: bundle adopts ADR-018's slot names and values verbatim** (`space.0` through `space.10`), drop the non-canonical 5/7/10/14/20/28 stops, add 48 and 64. Any current spacing at a non-canonical stop snaps up to the next canonical stop during the rename PR.

## 4. Radius tokens

ADR-018 locks: `radius.none` (0), `radius.sm` (4px), `radius.md` (6px), `radius.lg` (10px), `radius.pill` (9999px).

Bundle: `xs` (2), `sm` (3), `md` (6), `lg` (10), `full` (999).

| Bundle | ADR-018 slot | Gap |
|---|---|---|
| `rounded.xs` (2px) | **Below floor.** | Drop. ADR-018's smallest is 4px. |
| `rounded.sm` (3px) | **Below floor.** | **Blocker.** Bundle uses `sm`=3px extensively (buttons, fader thumbs). Must rescale to 4px or add 3px stop to ADR-018 under amendment. Recommend rescale to 4px. Visual impact negligible. |
| `rounded.md` (6px) | `radius.md` | Match. |
| `rounded.lg` (10px) | `radius.lg` | Match. |
| `rounded.full` (999px) | `radius.pill` (9999px) | **Mismatch.** 999px caps at that pixel value; 9999px effectively infinite. For pill rendering they are equivalent; standardise on 9999px and rename. |

## 5. Target-size tokens

ADR-018 locks: `target.min` (24 CSS px), `target.comfortable` (32 CSS px), `target.touch` (44 CSS px).

Bundle does not define target tokens. Instead component-by-component:

- Rail buttons: 36px.
- Tabs: 28px.
- Fader thumbs: 14px. **P0 violation** of `target.min >= 24px`.
- Knobs: 36px.
- Row height (`--row-h` compact): 24px. **Equal to `target.min` floor**; any smaller affordance inside a compact row violates.

| Token | Required | Bundle status |
|---|---|---|
| `target.min` (24px) | Floor for all interactive affordances | **Violated** by 14px fader thumb, 14px row-height in compact density where rows host clickable items. See `46-ux-bundle-a11y-color-audit.md`. |
| `target.comfortable` (32px) | Default for primary buttons | Rail buttons 36px (pass); tabs 28px (fail); knobs 36px (pass). |
| `target.touch` (44px) | Touch layouts | Not modelled. Bundle is desktop-first; touch not in MVP scope. |

**Required bundle change in v0.2-beta:** move fader thumb to 24px (or 20px with a larger hit target via pseudo-element padding); move tabs to >= 24px; confirm compact-density row clickability.

## 6. Focus-indicator tokens

ADR-018 locks: `focus.width` (2px default, 3px high-contrast), `focus.offset` (2px), `focus.radius` inherits.

Bundle: implemented via `:focus-visible` in `styles.css` per theme, but not tokenised. See §1.5. Must add the token family in v0.2-beta.

## 7. Motion tokens

ADR-018 locks: `motion.instant` (0ms), `motion.fast` (120ms), `motion.base` (200ms), `motion.slow` (320ms). All reduce to `motion.instant` when OS reduce-motion is on.

Bundle: no motion-token family. `styles.css` has hard-coded durations (`250ms`, `400ms` observed in common transitions). Must tokenise.

| ADR-018 slot | Bundle value (to fill) | Notes |
|---|---|---|
| `motion.instant` | 0ms | N/A |
| `motion.fast` | 120ms | Hover feedback, button press. Bundle's `250ms` hover transitions must move to 120ms. |
| `motion.base` | 200ms | Panel transitions, tab switches. |
| `motion.slow` | 320ms | Modal open, overlay fade. |
| `ease.standard` | (UX bundle) | Default cubic-bezier. |
| `ease.decelerate` | (UX bundle) | For elements entering the viewport. |
| `ease.accelerate` | (UX bundle) | For elements leaving the viewport. |
| `ease.linear` | (UX bundle) | For timing-matched motion (beat pulse). |

**Beat-pulse and audio-reactive animations** are exempt from `motion.instant` on reduce-motion **only if** they are functionally load-bearing (e.g. a meter indicator). Decorative beat pulses must fall back per `HANDOFF.md` §"Key conventions for native implementation" "prefers-reduced-motion" row.

## 8. Elevation tokens

ADR-018 locks: `elev.0` (none), `elev.1` (subtle shadow or 1px border), `elev.2` (popover), `elev.3` (modal), `elev.4` (toast). Values filled by UX bundle.

Bundle: no explicit elevation tokens. Shadows are inline in `styles.css`. Extract:

| ADR-018 slot | Proposed value (to confirm during rename) | Bundle current |
|---|---|---|
| `elev.0` | `none` | N/A |
| `elev.1` | `0 1px 0 var(--line-1)` (per `DESIGN.md` §"Surface stack" stacking rule) | Inline |
| `elev.2` | `0 4px 12px rgba(0,0,0,0.3)` | Inline (tweaks panel) |
| `elev.3` | `0 8px 24px rgba(0,0,0,0.5)` | Inline (modal) |
| `elev.4` | `0 12px 32px rgba(0,0,0,0.6)` | Inline (not observed; add) |

Atelier has zero-glow (`--glow: 0`); Console has `--glow: 0.3`. Shadow tokens must respect `--glow` for per-theme suppression.

## 9. Z-index tokens

ADR-018 locks: `z.base` (0), `z.docked` (100), `z.dropdown` (200), `z.sticky` (300), `z.fixed` (400), `z.popover` (500), `z.modal` (600), `z.toast` (700), `z.tooltip` (800).

Bundle does not define a z-index scale. `styles.css` has ad-hoc `z-index` values. Must tokenise and normalise in the rename PR.

## 10. Forward-looking ADR-018 token proposals (deferred; not in v0.2-beta scope)

> **Status (post-v0.2-beta intake, 2026-04-23):** this section was originally drafted as a pre-v0.2-beta ADR-018 amendment proposal. The v0.2-beta bundle at `ui-predev-bundle-v0.2-beta/` implemented **only the minimal ADR-018 alias layer** per §1's existing slot set (see `ui-predev-bundle-v0.2-beta/styles/v02beta.css` lines 32-73); the expanded slots enumerated below were **not** added. This section is retained as a **forward-looking proposal** for a future ADR-018 amendment (candidate for v0.3+ or for a post-port structural-tokens pass). No ADR-018 amendment is in flight as of v0.2-beta intake. Any future adoption follows the change-control process in `43-development-standards.md` §14 (ADR amendment) and requires Primary + Deputy sign-off per `23-decision-ownership-matrix.md`.

The rename originally identified these new token slots as structural (not product-tier) additions that would round out the non-scene UI surface. They are **deferred, not rejected**:

| New slot | Family | Rationale | v0.2-beta treatment |
|---|---|---|---|
| `surface.raised.hover` | Colour - surface | Hover affordance at bundle `bg-3` equivalent. | Resolved via existing `surface.hover` (`--bg-5`) alias; hover-on-raised compounded at consumer site. |
| `surface.raised.pressed` | Colour - surface | Pressed affordance at bundle `bg-5` equivalent. | Same as above; no separate pressed slot. |
| `border.emphasis` | Colour - border | Emphasised hairline beyond `border.strong`. | Deferred; bundle uses `border.subtle/default/strong` only. |
| `text.subtle` | Colour - text | Low-emphasis text distinct from `text.muted`/`text.disabled`. | Deferred; bundle uses `text.primary/secondary/muted/disabled`. |
| `accent.on-primary` / `accent.on-secondary` / `accent.on-danger` / `accent.on-warning` / `accent.on-success` / `accent.on-info` | Colour - accent | Per-accent readable-foreground pair; simplifies button text colour resolution. | Deferred; v0.2-beta resolves accent foregrounds via direct ADR-018-slot math at consumer sites. |
| `font.display` | Typography - family | Atelier's Fraunces; serif display slot. | Deferred; Atelier overrides `font.heading` locally where display copy appears. |
| `text.xxs` (10px, mono+caps constraint) | Typography - scale | Panel header / stat-key cap labels. | Deferred; bundle reuses `text.xs` with letter-spacing + `text-transform: uppercase` at site. |
| `text.xs.mono` (11px) | Typography - scale | `mono-sm` replacement. | Deferred; bundle normalised mono to 12px per §2.2. |
| `text.3xl` at 30px | Typography - scale | Bundle's 28px `display-lg` rescales to 30px. | Landed (applied during v0.2-beta typography rescale). |

**If and when this section activates.** Scope would be structural-only additions with no value changes to existing slots; the amendment would land as a PR against `adr/ADR-018-non-scene-ui-toolkit.md` §"Design Tokens" accompanied by a corresponding bundle-side rename (`surface.raised.hover` uses, `accent.on-*` adoption, etc.) and a re-run of the Contrast-Pair Registry in §1.9. Until that PR is drafted, the slots above are **not** part of the v0.2-beta token surface and MUST NOT be referenced by any code landing during the port phase.

## 11. Rename PR scope (landed at v0.2-beta; retained for historical reference)

> **Status (post-v0.2-beta intake).** The 12-item PR scope below was handed to the UI/UX team prior to v0.2-beta. The bundle's `BUNDLE-PROVENANCE-v0.2-beta.md §Landed in this pass` records what was actually executed: the ADR-018 minimal alias layer (item 1 partial; per §10 deferral), the Chromaticity-product-tier token families (item 2), typography rescale (item 3), spacing consolidation (item 4), radius scale (item 5), focus family (item 6), motion/elevation/z-index tokens (items 7 in part), Contrast-Pair Registry (item 9), fader thumb ≥ 24px (item 10), and alias-layer consumption in `styles/v02beta.css` with legacy `styles.css` retained for fallback (item 11 partial; native-slot flip deferred to dev-team queue item 4). Item 8 (`accent.on-*` tokens) was deferred per §10 above. Item 12 (SHA-256 restamp + UTF-8 provenance rewrite) is dev-team queue item 5.

Single PR against the bundle (v0.1-alpha -> v0.2-beta):

1. Rename every colour token in `DESIGN.md` per §1. **Landed (minimal ADR-018 alias set).**
2. Introduce `chroma.band.*`, `chroma.cat.*`, `chroma.signal.*` as Chromaticity-product-tier tokens. **Landed.**
3. Rescale typography per §2.2 decisions (drop 9px; move 28px to 30px; normalise mono 11px to 12px). **Landed.**
4. Consolidate spacing to the ADR-018 scale per §3. **Landed.**
5. Drop radius 2px / rescale 3px -> 4px per §4. **Landed.**
6. Add `focus.*` token family per §1.5 and §6. **Landed** (`--focus-ring`, `--focus-offset`, `--focus-width`).
7. Tokenise motion, elevation, z-index per §7/§8/§9. **Landed (motion); elevation/z-index retained ad-hoc pending port phase.**
8. Add `accent.on-*` tokens per §1.4. **Deferred to v0.3+ per §10; v0.2-beta resolves accent foregrounds via direct slot math.**
9. Produce the Contrast-Pair Registry (Pulse / Atelier / Console values measured) per §1.9. **Landed** (populated in `ui-predev-bundle-v0.2-beta/DESIGN.md §Contrast registry`; Atelier `accent.warning` tightened to `#7a4a0a` for AA).
10. Fix fader thumb >= 24px per §5. **Landed** (rule in `styles/v02beta.css`).
11. Update `styles.css` to consume the new token names (single rev + build verification). **Partial:** alias layer at `styles/v02beta.css` consumed by `.jsx` callsites that opt in; legacy `var(--bg-0)` etc. retained in `styles.css` and most callsites. **Dev-team queue item 4** flips to native (`var(--bg-0)` → `var(--surface-base)` sweep + alias-layer deletion).
12. Re-stamp `BUNDLE-PROVENANCE.md` with v0.2-beta version and new per-file SHA-256s. **Partial:** new UTF-8 `BUNDLE-PROVENANCE-v0.2-beta.md` written alongside the carried-forward UTF-16 `BUNDLE-PROVENANCE.md`. **Dev-team queue item 5** regenerates SHA-256s and rewrites the canonical provenance doc as UTF-8.

## 12. Evidence closure

This document becomes part of the `EV-UX-CONTROL-CONTRACT` + `EV-ACCESSIBILITY-AA` evidence packets per `26-evidence-artifact-registry.md`. Re-verification at every token change is explicitly required by ADR-018 §"Token change control".

Transition criteria:

- `EV-ACCESSIBILITY-AA` Phase 0: Contrast-Pair Registry in §1.9 is fully populated (no TBD cells) and every pair meets or exceeds its minimum. **v0.2-beta status:** registry populated in `ui-predev-bundle-v0.2-beta/DESIGN.md §Contrast registry`; `46-ux-bundle-a11y-color-audit.md` is reconciled to those values.
- `EV-UX-CONTROL-CONTRACT` Phase 0: the bundle's `DESIGN.md` and `styles.css` use ADR-018 slot names per §§1-9 above. **v0.2-beta status:** ADR-018 alias layer landed at `ui-predev-bundle-v0.2-beta/styles/v02beta.css`; native-slot flip queued as dev-team queue item 4.
- `EV-ADR-LINT` per `43-development-standards.md`: **no ADR-018 amendment is in flight as of v0.2-beta intake.** §10 above is a forward-looking proposal, not an amendment draft; if the slots in §10 are ever adopted, a fresh amendment PR against `adr/ADR-018-non-scene-ui-toolkit.md` must pass MADR lint per `43-development-standards.md` §14.

## Traceability

- `adr/ADR-018-non-scene-ui-toolkit.md` §"Design Tokens (normative)" + §"Contrast-pair registry" + §"Token change control"
- `43-development-standards.md` §14 (ADR amendment process) + §10.1 (SPDX/font licensing)
- `26-evidence-artifact-registry.md` `EV-ACCESSIBILITY-AA`, `EV-COLOR-MANAGEMENT-SRGB`, `EV-UX-CONTROL-CONTRACT`
- `23-decision-ownership-matrix.md` "UX interaction contract authoring"
- `ui-predev-bundle/DESIGN.md` (the bundle's token definitions)
- `ui-predev-bundle/styles.css` (the bundle's token implementation)
- `46-ux-bundle-a11y-color-audit.md` (consumes the registry in §1.9)
