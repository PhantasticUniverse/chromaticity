# ADR-018: Non-Scene UI Toolkit

## Status
Accepted

## Context
NFR-11 Accessibility Floor mandates WCAG 2.2 AA on the non-scene UI surface, including a `>= 24 x 24 CSS px` target-size floor, `3:1` focus-indicator contrast on both dark and light backgrounds, `4.5:1` body-text contrast, reduced-motion respect, and axe-core + VoiceOver/NVDA conformance. NFR-12 Color Management mandates V1 sRGB output with OS display-profile respect on the non-scene UI. NFR-10 Authoring Responsiveness mandates p95 latency budgets for hover-docs, symbol-search, and docs-search measured at the OS event boundary per `37-authoring-environment-spec.md` §3.3a.

Chromaticity's scene canvas uses wgpu + naga per `adr/ADR-011-rendering-backend.md`. The non-scene UI (authoring environment, operator HUD, library browser, settings panels, docs viewer, dialogs) has no such decision and is an implicit architectural gap that blocks the UX team: they cannot specify target sizes in `CSS px`, focus-indicator tokens, or DPI scaling behavior without a toolkit decision.

The UI toolkit must:
1. run identically on macOS and Windows (the two MVP platforms per `30-mvp-support-matrix.md`),
2. expose native or near-native accessibility APIs (VoiceOver on macOS, UIA on Windows) so `EV-ACCESSIBILITY-AA` can close,
3. respect OS display-profile and render the non-scene UI in sRGB per NFR-12,
4. meet the NFR-10 latency budgets on the GPU-A baseline hardware class,
5. embed the wgpu scene canvas without teardown or colour-space reinterpretation on profile switches,
6. be cleanroom-compatible: no dependency that would force adoption of reference-app code or require license terms incompatible with `EV-LICENSE-AUDIT`,
7. support DPI scaling correctly at `1.0x`, `1.25x`, `1.5x`, `1.75x`, `2.0x`, `2.5x`, `3.0x` without pixel-rounding regressions that would violate the target-size floor,
8. keep authoring-editor hot-reload latency under the `37` §3.3 budgets (editor component rendering is part of the latency path).

## Decision

**Chromaticity's non-scene UI uses Tauri 2 as the application shell with a Svelte 5 + TypeScript front-end.**

- **Tauri 2** provides a native Rust app shell that embeds the platform's system web view (WebKit on macOS via `WKWebView`, WebView2 on Windows via Microsoft Edge Chromium). The app is a single-binary native application; there is no bundled Chromium. Frame budget and memory footprint are materially better than Electron for the same surface area, which is the single largest factor in hitting NFR-10.
- **Svelte 5** (runes mode) is the UI component framework. Svelte's compile-time output is small, has measurably better keystroke-to-paint latency than React for the panel-heavy surface Chromaticity ships, and integrates cleanly with the CodeMirror 6 editor (default for authoring env per `37` §3.2 candidate list).
- **TypeScript** is the UI language; shared UI types (control types, symbol catalog entries, OSC route models) are generated from the Rust core via `ts-rs` or equivalent so the UI cannot drift from the contracts in `02-synesthesia-contracts.md`, `36-reactive-symbol-catalog.md`, and `08-io-integrations.md`.
- **CodeMirror 6** is the authoring editor component. GLSL/JavaScript/JSON syntax highlighting, multi-cursor, folding, soft-tab policy, and hover/completion integrations all come from the CodeMirror ecosystem; per-symbol autocomplete and hover-docs are wired from the ratified symbol catalog.
- **Scene canvas embedding**: the wgpu scene output is rendered to a native surface (not a web-rendered surface) and composited under a transparent section of the web view. This preserves GPU colour space and avoids web-view-induced colour-space conversion on sRGB/HDR boundaries. The operator fullscreen path exits the web view entirely and places the wgpu surface full-screen natively.
- **Accessibility bridge**: the system web view surfaces to the OS accessibility tree natively (VoiceOver on macOS, UIA on Windows). axe-core runs inside the web view as part of CI; VoiceOver/NVDA manual audit runs per release cycle per `EV-ACCESSIBILITY-AA`.

## Consequences

### Positive
- Single UI codebase across macOS and Windows; behavioural divergence is isolated to the scene canvas native surface.
- NFR-10 latency budgets are realistic (Svelte 5 + native web view keystroke-to-paint p95 in the low tens of ms on GPU-A baseline hardware).
- NFR-11 accessibility is inherited from the OS accessibility tree rather than reimplemented; axe-core closes the automated portion and manual audits close the subjective portion.
- NFR-12 sRGB-only is enforceable: the system web view respects OS display profile; the scene canvas is a separate native surface whose colour management is owned by ADR-011.
- Cleanroom posture preserved: Tauri is MIT/Apache-2.0, Svelte is MIT, CodeMirror 6 is MIT; all three have `EV-LICENSE-AUDIT`-compatible terms.
- Developer velocity: shared TypeScript types from Rust core prevent contract drift; hot reload during UX iteration is immediate.
- Operator binary size is bounded to Rust binary + UI bundle (tens of MB, not hundreds), which is favourable vs Electron for notarized macOS distribution.

### Negative
- Two rendering stacks (native wgpu for scene + web view for UI) require a composition policy and a careful fullscreen-transition path. This is called out as a Phase 1 spike.
- WebView2 must be present on Windows; Tauri ships a per-install bootstrapping path but ops needs to confirm enterprise/offline installer behaviour pre-GA.
- CodeMirror 6 is the newer major; its plugin ecosystem is more fragmented than CodeMirror 5. Chromaticity pins the exact version and tracks plugin health.
- Two languages in the repository (Rust core, TypeScript UI); requires shared-type generation discipline. Mitigated by `ts-rs`.

## Alternatives Considered

- **Electron + React + Monaco**. Rejected on binary size (chronically large for a touring-laptop context), memory footprint, and the licensing/ops overhead of bundled Chromium. Monaco editor overhead alone is comparable to the full CodeMirror 6 bundle.
- **Qt Widgets / Qt Quick**. Rejected. Accessibility story is historically weaker on macOS; high-DPI behaviour on Windows has footguns; commercial licensing for a closed-source product creates an `EV-LICENSE-AUDIT` failure mode under the LGPL-static-link branch; adoption of Qt for the UI forks us further from the web-ecosystem editor-component maturity we need for authoring.
- **SwiftUI (macOS) + WinUI 3 (Windows)**. Rejected. Two completely separate UI codebases maintained in parallel, each with independent accessibility implementations; does not satisfy the single-codebase requirement and doubles UX review load.
- **Dioxus / Iced / egui / native Rust UI**. Rejected. Accessibility story is immature on both macOS and Windows; the maturity gap is the single biggest risk to NFR-11 closure. Reconsider post-MVP.
- **Flutter Desktop**. Rejected. Non-native rendering tree means accessibility and IME behaviour regress; CPU/GPU overhead competes with the scene canvas for the same budget.
- **Tauri 1 + React + Monaco**. Rejected in favour of Tauri 2 + Svelte 5 + CodeMirror 6; same app-shell upside, smaller bundle, better keystroke-to-paint.

## Design Tokens (normative; this ADR is the source of truth)

Design tokens for colour, type, spacing, focus-indicator, motion, and target size live in this ADR rather than in `37-authoring-environment-spec.md`. This is the single normative table; Svelte components, documentation pages, and UX contract artifacts (`EV-UX-*`) all reference the tokens defined here. Final visual design language (the concrete hex values of accent, surface, and brand colours) remains out of scope per `01-charter.md` and is filled in by the first UI/UX bundle against the structure locked below.

### Token structure

| Token family | Tokens (normative names) | Constraints | Filled by |
|---|---|---|---|
| Colour — surface | `surface.base`, `surface.raised`, `surface.sunken`, `surface.overlay` | Light + dark theme pairs; WCAG 2.2 contrast pairs documented below | UX bundle |
| Colour — text | `text.primary`, `text.secondary`, `text.muted`, `text.inverse`, `text.accent` | `text.primary` vs `surface.base`: contrast >= 4.5:1 in both themes; `text.secondary` vs `surface.base`: >= 4.5:1 | UX bundle |
| Colour — accent | `accent.primary`, `accent.secondary`, `accent.danger`, `accent.warning`, `accent.success`, `accent.info` | Each accent vs `surface.base` >= 3:1 when used as non-text UI; text on accent >= 4.5:1 | UX bundle |
| Colour — focus | `focus.ring`, `focus.ring.inverse` | Focus ring vs adjacent surface >= 3:1 on both light and dark backgrounds per WCAG SC 1.4.11 + 2.4.13 | UX bundle (values); structure locked here |
| Typography — family | `font.sans`, `font.mono` | `font.sans` = system UI stack; `font.mono` ships bundled (no network fetch); both must carry SPDX headers per `43-development-standards.md` Sec.10.1 | UX bundle |
| Typography — scale | `text.xs` (12px), `text.sm` (13px), `text.base` (14px), `text.md` (15px), `text.lg` (17px), `text.xl` (20px), `text.2xl` (24px), `text.3xl` (30px) | Sizes are normative; line-height and letter-spacing pair per token are filled by UX bundle | UX bundle (line-height/tracking); scale locked here |
| Typography — weight | `weight.regular` (400), `weight.medium` (500), `weight.semibold` (600), `weight.bold` (700) | Only these weights ship; no font-variation-settings beyond weight at V1 | Locked here |
| Spacing scale | `space.0` (0px), `space.1` (2px), `space.2` (4px), `space.3` (6px), `space.4` (8px), `space.5` (12px), `space.6` (16px), `space.7` (24px), `space.8` (32px), `space.9` (48px), `space.10` (64px) | Normative. Components may interpolate but MUST NOT introduce new stops | Locked here |
| Target size | `target.min` (24 CSS px), `target.comfortable` (32 CSS px), `target.touch` (44 CSS px) | NFR-11 floor is `target.min`; hit areas use `target.min` minimum, `target.comfortable` default. Touch layouts use `target.touch` | Locked here |
| Focus indicator | `focus.width` (2px default; 3px high-contrast mode), `focus.offset` (2px from element edge), `focus.radius` inherits from element | Focus ring drawn outside element bounds; must not be clipped by parent overflow:hidden unless overridden | Locked here |
| Motion — duration | `motion.instant` (0ms), `motion.fast` (120ms), `motion.base` (200ms), `motion.slow` (320ms) | All motion tokens reduce to `motion.instant` when OS reduce-motion signal is on (NFR-11 reduced-motion conformance) | Locked here |
| Motion — easing | `ease.standard`, `ease.decelerate`, `ease.accelerate`, `ease.linear` | Concrete cubic-bezier tuples filled by UX bundle | UX bundle (tuples); structure locked here |
| Radius | `radius.none` (0), `radius.sm` (4px), `radius.md` (6px), `radius.lg` (10px), `radius.pill` (9999px) | Locked here | Locked here |
| Elevation | `elev.0` (none), `elev.1` (subtle shadow or 1px border), `elev.2` (popover), `elev.3` (modal), `elev.4` (toast) | Shadow parameters filled by UX bundle; structure locked here | UX bundle (shadow values) |
| Z-index | `z.base` (0), `z.docked` (100), `z.dropdown` (200), `z.sticky` (300), `z.fixed` (400), `z.popover` (500), `z.modal` (600), `z.toast` (700), `z.tooltip` (800) | Locked here | Locked here |

### Contrast-pair registry (locked structure; values from UX bundle)

The UX bundle MUST provide, for each `content-origin: first-party` doc-page theme and for the app shell (light + dark themes), a contrast-pair registry asserting the following comparisons meet their minimums, with measured ratios recorded:

| Pair | Minimum |
|---|---|
| `text.primary` on `surface.base` | 4.5:1 |
| `text.primary` on `surface.raised` | 4.5:1 |
| `text.secondary` on `surface.base` | 4.5:1 |
| `text.muted` on `surface.base` | 3:1 (non-essential text per WCAG SC 1.4.3 exception); must be 4.5:1 if used for instructional copy |
| `accent.primary` fill vs `surface.base` | 3:1 |
| `accent.danger` fill vs `surface.base` | 3:1 |
| `focus.ring` vs `surface.base` | 3:1 |
| `focus.ring` vs `surface.raised` | 3:1 |
| `focus.ring.inverse` vs `accent.primary` | 3:1 |
| Button text on `accent.primary` | 4.5:1 |

The registry ships alongside `EV-UX-CONTROL-CONTRACT` and is re-verified at every token change under `EV-ACCESSIBILITY-AA`.

### Token change control

Token names and the token families above are normative and change only via an ADR amendment (this file) under `43-development-standards.md` Sec.14. Token **values** (concrete colour hex, shadow parameters, easing tuples, line-height/tracking) are filled by the UX bundle and amendable under the UX contract change-control process per `23-decision-ownership-matrix.md` Sec."UX interaction contract authoring". A value change that would invalidate a contrast-pair minimum requires a Decision Ownership Matrix waiver per `23-...md` Sec.Waiver Process.

## Enforcement

- Design tokens above are authoritative. Components may not hard-code colour, spacing, radius, or motion outside the token set; `eslint-plugin-chromaticity-ui-tokens` (Phase-0 bootstrap lint) enforces this on Svelte component files.
- `EV-ACCESSIBILITY-AA` runs axe-core CI on every PR touching non-scene UI code and includes the per-release VoiceOver/NVDA manual audit checklist; the contrast-pair registry is part of the packet.
- `EV-COLOR-MANAGEMENT-SRGB` asserts the non-scene UI surface is sRGB; render conformance tests include a snapshot diff on calibrated vs uncalibrated display configurations.
- NFR-10 measurement harness per `37-authoring-environment-spec.md` §3.3a validates keystroke-to-paint latency against declared budgets.
- Tauri, Svelte, CodeMirror 6, and `ts-rs` versions are pinned per release per `22-compatibility-versioning-policy.md`; bumps run the full regression suite.

## Traceability
- `03-product-requirements.md` NFR-10, NFR-11, NFR-12
- `13-risk-register.md` R-29 (accessibility regression), R-30 (color management regression)
- `20-requirements-traceability-matrix.md` NFR-11, NFR-12 rows
- `26-evidence-artifact-registry.md` `EV-ACCESSIBILITY-AA`, `EV-COLOR-MANAGEMENT-SRGB`, `EV-AUTHORING-UX-LATENCY`, `EV-DOCS-UX-LATENCY`
- `37-authoring-environment-spec.md` §3.2 editor surface + §3.3a NFR-10 measurement harness
- `adr/ADR-011-rendering-backend.md` for scene canvas colour management
- `29-cleanroom-legal-policy-baseline.md` `EV-LICENSE-AUDIT` for dependency license posture
