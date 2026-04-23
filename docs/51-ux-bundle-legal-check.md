# UX Bundle Legal Check - Icon + Font + CDN Dependency Sweep

Legal / licensing audit of `ui-predev-bundle/` (revision `ux-bundle-v0.1-alpha`) against:

- `29-cleanroom-legal-policy-baseline.md` (Brand/Trademark Review Checklist + license allowlist).
- `adr/ADR-018-non-scene-ui-toolkit.md` §"Typography - family" - "`font.mono` ships bundled (no network fetch); both must carry SPDX headers per `43-development-standards.md` Sec.10.1".
- `43-development-standards.md` §10 (SPDX / REUSE / SBOM coverage).
- `adr/ADR-016-observability-and-telemetry-posture.md` - "zero-default outbound traffic".

Feeds evidence rows: `EV-LICENSE-AUDIT`, `EV-LICENSE-HEADERS`, `EV-BRAND-REVIEW`, `EV-SBOM-COVERAGE` (all per `26-evidence-artifact-registry.md` + `43-development-standards.md`).

## Verdict summary

| Check | Status | Must-fix before port |
|---|---|---|
| Outbound network dependencies at runtime | **FAIL** (3 CDN hosts) | All three removed before shipping; acceptable in the mock, must not reach production. |
| Fonts bundled + SPDX-headered | **FAIL** (loaded from CDN in mock) | Bundle + SPDX-header before port; confirm OFL 1.1 license redistribution. |
| Icons original / license-clear | **at-risk** (visual similarity to known icon sets) | Legal review required; if adapted, attribute; if redrawn, document origination. |
| JSX dependencies (React, Babel-in-browser) | Acceptable for mock only | Remove entirely per `47-ui-port-plan.md` (Svelte replaces React). |
| Third-party docs link in HANDOFF.md | Informational only | Confirm cross-reference to external projects is policy-compliant. |

## 1. Outbound CDN dependencies (runtime network fetches)

Per `adr/ADR-016-observability-and-telemetry-posture.md`: **zero-default outbound traffic** is a locked posture. Per ADR-018 §"Typography - family": `font.mono` must ship bundled with no network fetch.

### 1.1. CDNs currently referenced in `Chromaticity.html`

| Line | Asset | Host | License | Status |
|---|---|---|---|---|
| L7 | preconnect | `fonts.googleapis.com` | Google Fonts ToS | **Remove** |
| L8 | preconnect (crossorigin) | `fonts.gstatic.com` | Google Fonts ToS | **Remove** |
| L9 | Inter + JetBrains Mono stylesheet | `fonts.googleapis.com/css2` | Google Fonts ToS; fonts themselves are OFL 1.1 | **Remove CDN; bundle fonts locally** |
| L12 | Fraunces stylesheet | `fonts.googleapis.com/css2` | same | **Remove CDN; bundle locally** (Atelier-theme only) |
| L17 | React 18.3.1 UMD | `unpkg.com` | MIT | **Remove** (Svelte replaces React per `47-ui-port-plan.md` §1) |
| L18 | React-DOM 18.3.1 UMD | `unpkg.com` | MIT | **Remove** (same) |
| L19 | Babel-standalone 7.29.0 | `unpkg.com` | MIT | **Remove** (compile-time transpile via Vite/SvelteKit; zero runtime transpile) |

### 1.2. Mitigation

All seven deps drop out naturally during the port per `47-ui-port-plan.md`:

- React / React-DOM / Babel: replaced by Svelte 5 compiler output + Vite.
- Fonts: move to `ui/static/fonts/` bundled at build time.
- `meta http-equiv="Content-Security-Policy"` post-port restricts to `self` + `data:` + explicit asset URLs; outbound network is `connect-src 'none'` by default with explicit allow-list for licence-refresh endpoint only (post-MVP).

### 1.3. Build-time verification

`EV-OUTBOUND-ZERO-DEFAULT` (implicit in `adr/ADR-016`) must be asserted at release time: CI smoke test runs the Tauri-built binary in a sandboxed namespace with no outbound network and confirms the app boots + functions offline.

## 2. Font licensing

Three font families referenced in the bundle. All three are OFL 1.1 and bundle-redistributable with attribution, but must ship bundled, not CDN-fetched.

### 2.1. Inter

- Source: rsms.me/inter; OFL 1.1.
- Files to bundle: Inter-Regular, Inter-Medium, Inter-SemiBold, Inter-Bold (weights 400 / 500 / 600 / 700 per `DESIGN.md` §typography).
- Subsetting: Latin + Latin-Ext (covers target locales at MVP per `30-mvp-support-matrix.md`).
- SPDX header: add `// SPDX-License-Identifier: OFL-1.1` in a sidecar `LICENSE.Inter.txt` alongside the font files.
- Attribution: required in `Settings -> About` + `/docs/legal/open-source-notices` page.

### 2.2. JetBrains Mono

- Source: jetbrains.com/mono; OFL 1.1.
- Files to bundle: JetBrainsMono-Regular, -Medium (weights 400 / 500).
- Subsetting: Latin + programming symbols + tabular-nums.
- SPDX header: `// SPDX-License-Identifier: OFL-1.1` + sidecar LICENSE file.
- Attribution: same as Inter.

### 2.3. Fraunces

- Source: Google Fonts; OFL 1.1.
- Atelier-theme only; optional to bundle if Atelier ships at V1.
- Files: Fraunces variable font (opsz + wght axes) OR static cuts (Regular, Medium, SemiBold).
- Subsetting: Latin + Latin-Ext.
- SPDX header + attribution: same as above.

### 2.4. Total font payload budget

OFL 1.1 fonts bundled, subsetted, WOFF2-compressed - expect ~300-500 KB total for all three families. Acceptable against the Tauri binary size budget (tens of MB per ADR-018 §Consequences).

### 2.5. Delivery

Fonts land in `ui/static/fonts/{Inter,JetBrainsMono,Fraunces}/` with:

- Font files (WOFF2 for shipping; WOFF / TTF for fallback).
- `LICENSE.txt` per family.
- `README.md` per family pointing at upstream + attribution line.
- `FONTS.md` at repo root enumerating each family + SPDX identifier (machine-readable per `43-development-standards.md` §10.1).

SBOM coverage: fonts are dependencies and enter the SBOM per `43-development-standards.md` §10.2 / `EV-SBOM-COVERAGE`.

## 3. Icon licensing

Icons in `ui-predev-bundle/components.jsx` are inlined SVG path sets, not references to an icon library. This is the correct final-state; but the paths themselves must be either originally authored or attributed to their source.

### 3.1. Observed paths (components.jsx L7-L67 approximately)

Hand-drawn SVG in the `Icon` component with names: `perform`, `library`, `midi`, `media`, `settings`, `edit`, `lock`, `help`, `user`, `heart`, `chevron`, `caret`, `play`, `pause`, `stop`, `record`, `prev`, `next`, and many more across the bundle.

Visual style: 24x24 viewBox, `stroke-width: 1.6`, rounded line caps / joins, `fill: none` stroke-based. This style closely matches Feather Icons (MIT) / Lucide (ISC; Feather fork). Path geometry is similar but not always byte-identical.

### 3.2. Risk

If paths are byte-identical or very close to Feather / Lucide without attribution, the bundle is either:

- Compliant by accident (Feather MIT / Lucide ISC both permit use with attribution notice in a LICENSE file), or
- Non-compliant (MIT / ISC require the attribution notice; bundle has none).

If paths are original, no attribution needed; but UX team must confirm provenance.

### 3.3. Required action

UX team confirms per-icon provenance. Two options:

1. **Adopt Lucide** (ISC, actively maintained, superset of Feather). Add dependency; attribution in `/docs/legal/open-source-notices`. Acceptable per `29-cleanroom-legal-policy-baseline.md` license allowlist (ISC is MIT-equivalent).
2. **Confirm original authorship** of every path. Requires per-icon sign-off + attestation in an internal record.

Recommendation: **adopt Lucide**. Maintenance saved, legal risk zero, attribution cost trivial. The icons that are product-specific (like `bang` which appears to be custom) remain original and attributed as Chromaticity-authored.

### 3.4. Brand-specific icons

The Chromaticity wordmark + brand logo (if any; bundle has `.tb-brand .name` per `styles.css` L231 which appears to be wordmark-only, no logo icon yet) must:

- Be cleared under `29-cleanroom-legal-policy-baseline.md` Brand/Trademark Review Checklist (name + wordmark).
- Be committed as an original asset with authorship attestation.
- Be trademark-registered per the brand-clearance milestone in the planning README §"Next milestone".

## 4. JSX dependencies (mock-only; removed in port)

Per §1.1 rows L17-L19. Acceptable in the mock; all three are MIT and do not taint a cleanroom process because they are reference-only and do not ship. `29-cleanroom-legal-policy-baseline.md` requires that reference-observation tooling not be inserted into the shipped product; the JSX mock is reference tooling.

Confirm during the port that **no React / Babel artifact survives** in the Svelte 5 port - grep gates should catch this in CI.

## 5. Third-party cross-references in handoff copy

| Location | Reference | Action |
|---|---|---|
| `HANDOFF.md` L5 | `google-labs-code/design.md` GitHub link | Informational link; no dependency. Keep for reference. |
| `HANDOFF.md` L213 | `npx @google/design.md lint` command | Dev-time tool only; not shipped. If used by CI, add to CI container image + license review (Apache-2.0 expected). |
| `views/scene.jsx` L18 | StackOverflow attribution comment in sample GLSL | Sample GLSL is CC0 per `// License CC0` in the source; acceptable. Rename / replace sample GLSL with a native `.chroma` sample per `44-ux-bundle-gap-audit.md` A-07. |
| `views/scene.jsx` L390 | Shadertoy URL `https://www.shadertoy.com/view/WdVfRh` | Placeholder; confirm no copyrighted scene is bundled without per-scene attribution at import time. Shadertoy ToS requires per-scene attribution; Chromaticity import pipeline must preserve this per `38-scene-library-and-import-spec.md`. |
| `views/import.jsx` L5 | Copy: "Paste a Shadertoy URL - we auto-convert common uniforms (`iTime`, `iResolution`, `iMouse`) to Chromaticity's native SSF." | Copy refers to native `.chroma` (not "SSF"); update per `44-ux-bundle-gap-audit.md` A-01. |

## 6. Deliverable inventory for bundle v0.2-beta

1. Remove all CDN references from `Chromaticity.html` (or move to a dev-only HTML file outside the shipped path).
2. Bundle Inter + JetBrains Mono + Fraunces locally under `ui-predev-bundle/fonts/` with `LICENSE.txt` per family.
3. Author `FONTS.md` at bundle root with SPDX IDs.
4. Decide on Lucide adoption vs original-icon-authorship-attestation; apply the consequent edits.
5. Update copy in `views/scene.jsx` and `views/import.jsx` per §5 findings (Shadertoy ToS attribution; native-format naming).
6. Confirm Chromaticity wordmark is attested as original work.

## 7. Policy intersections

### 7.1. Per `29-cleanroom-legal-policy-baseline.md`

- Name + domain clearance: tracked as a planning-README next-milestone item. Independent of bundle content but the bundle's wordmark must match the cleared name.
- Chromaticity-owned package extensions (`.chroma` / `.chromaPack` / `.chromaShow`): bundle does not currently claim other extensions. OK.
- Docs URL taxonomy: bundle's `HANDOFF.md` references these but does not ship docs pages. OK.
- Outbound endpoints: §1 above. FAIL until remediated.

### 7.2. Per `43-development-standards.md` §10 (SPDX / REUSE / SBOM)

- Every source file in the shipped product carries an SPDX header. The bundle mock is not shipped; SPDX headers are still recommended for consistency and to ease porting. For v0.2-beta: add SPDX headers to `DESIGN.md` + `HANDOFF.md` + every `.jsx` + `.css`.
- SBOM coverage: the Tauri-built product SBOM is Phase 4b work (`EV-SBOM-COVERAGE`). Bundle v0.2-beta does not need a full SBOM, but its dependency list must not grow.

### 7.3. Per `adr/ADR-018` §Enforcement

- Tokens pinned + license-audit-compatible: fonts OFL 1.1 (compatible); icons: see §3. OK once §3 closed.

## 8. Evidence closure

Once §§1-5 addressed in v0.2-beta:

- `EV-LICENSE-AUDIT` gets a design-phase evidence row pointing at this doc (as a pre-release inventory). Full audit still runs at Phase 5.
- `EV-LICENSE-HEADERS` gets a partial-coverage entry for bundle SPDX headers.
- `EV-BRAND-REVIEW` is unchanged by this doc directly; brand clearance runs independently per `29-cleanroom-legal-policy-baseline.md` §Brand/Trademark Review Checklist.
- `EV-SBOM-COVERAGE` picks up the bundled fonts as SBOM entries once the port lands; tracked for Phase 4b.

## 9. Traceability

- `29-cleanroom-legal-policy-baseline.md` §Brand/Trademark Review Checklist + license allowlist
- `adr/ADR-016-observability-and-telemetry-posture.md` zero-default outbound
- `adr/ADR-018-non-scene-ui-toolkit.md` §"Typography - family" + §Enforcement
- `43-development-standards.md` §10 (SPDX / REUSE / SBOM)
- `26-evidence-artifact-registry.md` `EV-LICENSE-AUDIT`, `EV-LICENSE-HEADERS`, `EV-BRAND-REVIEW`, `EV-SBOM-COVERAGE`, `EV-RELEASE-ATTESTATION`
- `47-ui-port-plan.md` §1, §8 (dependency pinning)
- `44-ux-bundle-gap-audit.md` A-01, A-07, X-03, X-04 (cross-references)
- `38-scene-library-and-import-spec.md` §5 (Shadertoy attribution preservation)
- `ui-predev-bundle/Chromaticity.html` (CDN references)
- `ui-predev-bundle/components.jsx` (icon inventory)
