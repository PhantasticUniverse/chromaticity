# ADR-010: Embedded Documentation Generation

## Status
Accepted

## Context
The reference app ships its entire documentation site inside the bundle under `Contents/Resources/views/embedded_docs/`. Authors rely on in-app lookup of uniform names, helper signatures, and JSON keys while writing scenes. Chromaticity must match this offline-first authoring experience while obeying clean-room constraints: no reference documentation text may be copied (see `29-cleanroom-legal-policy-baseline.md`).

## Decision

### Source of truth
- **Single source**: `36-reactive-symbol-catalog.md` plus structured YAML sidecars under `chromaticity-planning/docs-source/` for prose sections (FAQ, tutorials, best practices, etc.).
- Prose is Chromaticity-originated; code samples are Chromaticity-originated; every paragraph carries a provenance tag (`author`, `reviewed_by`, `date`).
- Reference documentation is consulted for behavior and feature coverage only; no verbatim text, no paraphrased-to-synonym text. A cleanroom review signoff is required per doc page before it ships (`EV-DOCS-CLEANROOM`).

### Generation pipeline
1. A build task reads `36-reactive-symbol-catalog.md` and YAML sidecars, and emits normalized JSON (`docs-index.json`) describing every symbol, signature, parameter, example, and cross-link.
2. A static site generator (e.g. MkDocs with a Chromaticity theme, or a custom SSG) renders HTML with offline search index.
3. Output is written to `<AppBundle>/Resources/docs/` and is the only docs surface the app loads at runtime.

### In-app consumption
- The in-app help view loads `docs/index.html` from the bundle.
- Help view has no network dependency. All assets are bundled.
- The authoring environment (`37-authoring-environment-spec.md`) links autocomplete hover docs to `docs-index.json` entries.

### Version sync
- Each release pins a docs version equal to the symbol catalog version (`1.0.0` at ratification). Mismatches fail release gate `EV-DOCS-VERSION-SYNC`.
- A CI check asserts every symbol in `36-reactive-symbol-catalog.md` has a corresponding doc page, and vice versa (no orphan symbols, no orphan pages).

### Content coverage (minimum)
The shipped docs set MUST cover, at minimum, the following topics. "Topical parity" below means that Chromaticity covers the same author-facing concerns as the reference at a topic level; page copy is authored clean-room and no reference prose is consulted beyond topic discovery.

- Introduction / Quickstart
- Creating Your First Scene
- Scene JSON reference (all keys from `02-synesthesia-contracts.md` Sec.1 and Sec.15)
- Package Formats reference (`.chroma`, `.chromaPack`, `.chromaShow` - definition, when to use, import-only `.synScene` stance per `22-compatibility-versioning-policy.md`)
- Standard Uniforms (from `36-reactive-symbol-catalog.md` Sec.1)
- Audio Uniforms (from Sec.2)
- Meta Controls (from Sec.3)
- GLSL Helpers (from Sec.4)
- Script Host API (from Sec.5 and CP-07)
- External Libraries (bundled lygia and hg_sdf alternatives per `29-cleanroom-legal-policy-baseline.md`)
- Control Hotkeys (from FR-17)
- Operator Hotkey Map reference (rendered from `42-operator-hotkey-map.md` as single source of truth)
- OSC route reference (from `08-io-integrations.md`, including the `compat-syn-v1` alias table and the OSC Operator Settings Panel Contract)
- MIDI mapping guide (including the extended surface MIDI-CONF-11..20)
- FAQ: audio, video, settings, troubleshooting, license/entitlement (license page canonicalized to `/faq/license`; `/manual/license` is a redirect alias)
- Tutorials: Shadertoy conversion (topical parity - Chromaticity-authored walkthrough, no reference prose consulted), livestream, presets/Snaps/playlists, authoring events with JavaScript
- Resources: release notes / changelog (`/resources/release-notes`), glossary, external shader resources (links only, no inlined third-party prose)

### Search
- Offline search index built from `docs-index.json` and prose pages.
- Keyboard shortcut opens a command-palette-style search.

### URL / slug convention (normative; Chromaticity-original)
The shipped docs site uses a Chromaticity-original top-level path taxonomy. In particular it does **not** adopt the reference's `/ssf/` path namespace (which stands for the reference's "Synesthesia Shader Format"). The five top-level sections (four Diátaxis-aligned sections per `43-development-standards.md` Sec.13 plus `/resources` for release-cycle content) are:

- `/reference/<topic>` - API and symbol reference pages. Examples: `/reference/scene-json`, `/reference/package-formats`, `/reference/standard-uniforms`, `/reference/audio-uniforms`, `/reference/meta-controls`, `/reference/glsl-helpers`, `/reference/script-api`, `/reference/transitions`, `/reference/external-libraries`.
- `/manual/<topic>` - operator manual pages. Examples: `/manual/osc`, `/manual/midi`, `/manual/hotkeys`, `/manual/audio-setup`, `/manual/display-output`.
- `/guides/<topic>` - tutorial and how-to content. Examples: `/guides/first-scene`, `/guides/shadertoy-import`, `/guides/best-practices`, `/guides/presets-snaps-playlists`, `/guides/live-performance`.
- `/faq/<topic>` - FAQ. Examples: `/faq/audio`, `/faq/video`, `/faq/settings`, `/faq/troubleshooting`, `/faq/license` (canonical path for the license/entitlement page).
- `/resources/<topic>` - release notes, glossary, external resource links. Examples: `/resources/release-notes`, `/resources/glossary`, `/resources/external-shader-resources`.

Slugs are lowercase-hyphenated. Chromaticity does not use the reference's `/ssf/*` path, `/manual/*` specific page names that mirror the reference verbatim, or any URL that would suggest the Chromaticity docs site is a mirror of the reference's site. The `/manual/*` and `/faq/*` topic list above is curated-by-topic rather than page-for-page cloned.

In-app hover-docs, the autocomplete panel, and the scaffolder template (`37-authoring-environment-spec.md` Sec.3.6) reference pages by these slugs; any hard-coded doc URL in shipped source or templates MUST match this convention. `EV-DOCS-COVERAGE` asserts no shipped template or code emits a `/ssf/*` URL.

## Consequences

### Positive
- Zero network dependency for authoring reference lookup.
- Single source of truth for symbols (no drift between catalog, autocomplete, docs).
- Clean-room posture is enforceable via provenance tags and signoff.

### Negative
- Requires sustained doc authoring effort from the Chromaticity team.
- CI gate adds release friction until docs reach coverage parity.

## Alternatives Considered
- **Online-only docs** - rejected: offline authoring is table stakes.
- **Copy reference docs** - rejected: violates `29-cleanroom-legal-policy-baseline.md`.
- **Generate docs ad-hoc at runtime from symbol metadata** - rejected: prose-heavy sections (tutorials, FAQ) can't be generated; also loses typographic quality.

## Evidence
- `EV-DOCS-CLEANROOM` - every doc page has a signed provenance record.
- `EV-DOCS-VERSION-SYNC` - docs version matches catalog version at release.
- `EV-DOCS-COVERAGE` - every public symbol has a doc page; every doc page references a real symbol or covers an approved prose section.

## Traceability
- `36-reactive-symbol-catalog.md` (source of truth)
- `29-cleanroom-legal-policy-baseline.md` (cleanroom constraints)
- `37-authoring-environment-spec.md` (autocomplete integration)
- `03-product-requirements.md` FR-16 Embedded Documentation.
