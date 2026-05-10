<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

# V1 Release Parity Gaps (Clean-Room Observable Audit)

**Generated:** 2026-05-07. Anchor: `main` HEAD `d6b266d` (after PR #384 merge). Audit derived under clean-room observable-only posture per `29-cleanroom-legal-policy-baseline.md`.

## 1. Posture

This document is a derivative of an observable-data-only inspection of the reference application's `Contents/Resources/` bundle (file-tree shapes, public plist, JSON schema shapes, asset filenames, embedded documentation page filenames). NO source code, NO `.synScene` package contents, NO Mach-O binary, NO `.cpp` / `.h` / `.m` / `.storyboardc` internals were inspected. The audit's purpose is V1 feature-surface parity, not implementation parity.

Entries are scoped at the feature-dimension level. Implementation work derived from this doc must continue to honor the clean-room SOP per `29-cleanroom-legal-policy-baseline.md`. Per the brand-trademark-review checklist in that SOP, no reference-app product name appears in the user-facing description columns of evidence rows or shipped UI strings derived from this audit.

This doc is a planning artifact, not a gate. It does not declare V1 readiness, does not pre-empt Phase-4 EXIT decisions, and does not commit any specific row to a specific implementation path. The canonical evidence rows live in `26-evidence-artifact-registry.md`; this doc adds a parity-gap dimension that the registry does not carry.

## 2. Cumulative parity rollup

Approximately **75-80% of reference observable feature surface implemented** in chromaticity at HEAD `fb2bc75`, weighted by V1-relevance:

- **Engine-side surfaces** (scene model, runtime, audio, scheduler, render, scripting, integrations, entitlement, observability): **~90%**. Phase-4a EXIT achieved at commit `5acc452`; Wave 1c-β camera + Syphon adapters closed 2026-05-07 (synthetic-narrowing); Phase-4b passable under collective narrowings; 5 of 6 Phase-4d rows at `Pass` (only `EV-ANALYTICS-ZERO-DEFAULT` calendar-gated to ~2026-05-20 per the 14-day soak).
- **UI-shell parity** (views, widgets, hotkey / OSC / MIDI vocabularies): **~95%**. Wireframe-faithful complete via the v0.2-beta bundle port (W6-W34 sprint waves). Custom macOS menu bar parity beyond Tauri default is the only minor gap.
- **Content / asset bundling** (default scenes, library corpus, shadertoy assets, demo media, embedded docs, custom thumbnails): **~30-35%** (post-2026-05-10 V1-RC content fanout). Wave 1d-α PR 2 + PR 3 shipped a clean-room mkdocs corpus of 11 in-app help pages (under `docs-corpus/`, bundled into `dist/help/` and resolved through `tauri.conf.json` `bundle.resources`); Wave 1d-β PR 1 + PR 2 shipped a clean-room scene starter library of 12 `.chroma` packages spanning five feature surfaces (under `engine/scene/tests/fixtures/scenes/{welcome,examples}/`). Both surfaces clean-room per `docs/29` discipline (provenance metadata + per-subdir READMEs attest). Shadertoy asset library + per-source-class fallback thumbnails + demo media remain the residual V1 gaps.
- **Update / telemetry infrastructure**: **~50-55%** (post-2026-05-10 release-pipeline fanout). Wave 1e-α + 1e-β + 1e-γ shipped the `release-eng/` stub-signing scripts + `tauri-plugin-updater` plugin scaffold + `.github/workflows/release.yml` cross-platform matrix; the workflow validated end-to-end via a synthetic-signing smoke test (run `25628685936`). Production cert procurement (Apple Developer ID + Windows EV + Sparkle 2 EdDSA keypair) is the only remaining gate on `EV-RELEASE-PIPELINE` Pass; AO-gated per ADR-014 Sec.4 + Sec.6. Analytics correctly stricter-by-design at zero-default per ADR-013 + ADR-016, parity-by-omission rather than a gap.

## 3. Per-feature-dimension findings

The audit covers twelve observable feature dimensions. Each row notes implementation status against the reference observable surface and the V1-relevance of any gap.

| Dimension                                  | Status                | Notes                                                                                                                                          |
| ------------------------------------------ | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Scene packaging                            | Implemented           | Chromaticity-native `.chroma` / `.chromaPack`. Non-import of the reference scene format is intentional per the clean-room baseline.            |
| Preset / scene JSON schema                 | Partial               | Slider control type covered. Control types beyond slider need explicit equivalent inventory in `38-scene-library-and-import-spec.md`.          |
| Default / welcome scene + thumbnails       | Partial               | Per-source-class thumbnail fallback set + default seeded scene engine-side both missing. See gap G4.                                           |
| Library scenes (bundled starter pack)      | Missing               | Reference bundle ships ~87 scenes. V1 needs an authored-from-scratch (clean-room) scene starter pack. See gap G2.                              |
| Shader library (Lygia + sdf primitives)    | Partial               | Lygia matches reference at v1.1.4 (trial vendor under `third_party/lygia/`). Reference-bundle SDF + metacontrols GLSL analogs absent.          |
| Image / asset coverage                     | Missing               | Shadertoy-channel image set + demo media absent. Required for `EV-IMPORT-SHADERTOY` runtime correctness beyond PR #360 scaffolding. See G3.    |
| UI views / app shell                       | Implemented           | Wireframe-faithful native parity via the v0.2-beta bundle port. Custom menu bar minor gap.                                                     |
| Hotkeys / OSC routes / MIDI vocabulary     | Implemented           | First-class wireframe parity. 57-row hotkey vocabulary, OSC route taxonomy, and MIDI-CONF-11..20 fixtures all landed.                          |
| Embedded documentation                     | Missing               | Reference bundle ships an in-app help corpus. No equivalent in chromaticity. See gap G1.                                                       |
| Auto-update / crash reporting / analytics  | Partial               | Analytics zero-default by design (ADR-013 + ADR-016); crash payload redaction scaffolded (PR #346). Auto-update absent. See gap G5.            |
| Camera capture                             | Implemented           | Wave 1c-β closed 2026-05-07 (16-PR sequence). `integrations/camera-adapter/` (nokhwa cross-platform; CoreMediaIO + Media Foundation) + `integrations/syphon-adapter/` (macOS GPU IO via `objc2` framework bindings) + `engine/render` bridges + `Sources.svelte` device pickers. Synthetic-narrowing posture per ADR-020 + ADR-021 + `docs/54`; per-platform binding stubs + hardware validation deferred per `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a Wave 1c-β closure. |
| OpenCV-backed video processing             | Deferred-Per-AO       | OpenCV-backed video processing (filters, vision pipelines, object detection) is out of scope for V1; no clean-room ADR or evidence row owns it at HEAD. The reference observable surface includes OpenCV-class processing nodes; chromaticity intentionally defers this scope. |
| Localization                               | Matches reference     | English-only V1 across both surfaces.                                                                                                          |

## 4. Top V1 release gaps (engineering work)

Five gaps surface as engineering work that would close measurable V1 parity. Each entry covers the dimension, what is missing, why it matters for V1, rough sizing, and a recommended sequence note.

### G1. In-app embedded documentation bundle — substantially closed (2026-05-10)

**Dimension:** Embedded documentation. **Status:** Substantially closed via Wave 1d-α PR 1 + PR 2 + PR 3 (PRs `#413` / `#419` / `#426`). Eleven clean-room corpus pages live under `docs-corpus/docs/` (`index` / `about` / `hotkeys-reference` / `osc-routes` / `chroma-uniforms` / `audio-reactivity` / `scene-workflow` / `library-workflow` / `perform-mode` / `settings-reference` / `import-shadertoy`); `mkdocs build` output is committed under `dist/help/`; `app/shell/tauri.conf.json` `bundle.resources` resolves all pages at `generate_context!()` time; `src/lib/views/Help.svelte` reads bundled HTML through Tauri's `convertFileSrc`. `EV-DOCS-CORPUS` row at `Pass`. **Residual:** CI integration of `mkdocs build` (so authors stop committing the built tree manually) + full-text search threading + dynamic page enumeration carry forward as `release-eng` follow-ups; `EV-DOCS-UX-LATENCY` synthetic harness already at `Pass`.

### G2. Bundled scene starter library — substantially closed (2026-05-10)

**Dimension:** Library scenes. **Status:** Substantially closed via Wave 1d-β PR 1 + PR 2 (PRs `#418` / `#425`). Twelve clean-room `.chroma` packages live under `engine/scene/tests/fixtures/scenes/{welcome,examples}/` spanning five feature surfaces (uniforms / control-widget showcase, audio reactivity, OSC routing, hotkey-bound preset cycling, playlist transitions). All scenes use clean-room author-coined NAMEs (no `syn_*` Synesthesia-compat namespace) and round-trip through `chromaticity_scene::loader::load_chroma_dir` at the slice-3a schema shape. Per-subdirectory READMEs updated with scene listings + clean-room provenance attestation per `29-cleanroom-legal-policy-baseline.md`. `EV-LIBRARY-CORPUS-MIN` row at `Pass`. **Residual:** First-launch seed-migration UX (copy bundled scenes to user-writable library on first launch) and per-source-class fallback thumbnails (G4 below) remain follow-ups.

### G3. Shadertoy-channel asset library

**Dimension:** Image / asset coverage. **Missing:** A licensed shadertoy-equivalent texture / cubemap pack covering the channel slots a typical Shadertoy port references. **Why it matters for V1:** `EV-IMPORT-SHADERTOY` currently lands at PR-1-of-N scaffolding only (PR #360 / `7497955`). Runtime correctness of imported Shadertoy scenes requires the channel-asset set to resolve. **Sizing:** Mostly licensing / sourcing work plus a small bundling PR. Rough estimate: 1-2 PRs after license posture is confirmed. **Sequence:** Blocks the `EV-IMPORT-SHADERTOY` PR-2 closure; should land before that PR opens.

### G4. Default + per-source-class fallback thumbnails

**Dimension:** Default / welcome scene + thumbnails. **Missing:** Four PNG fallback thumbnails — default, fallback (generic), ISF source-class, Shadertoy source-class — wiring `EV-LIBRARY-THUMBNAIL` (currently `Pass` under stub byte-hash narrowing) to a real first-launch UX without requiring the deferred GPU-backed renderer. **Why it matters for V1:** Library view first-launch UX surface; the byte-hash stub renderer ships a "no thumbnail" affordance that would read as polish-debt at V1. **Sizing:** 1 PR (assets + wiring). **Sequence:** Independent; small wave-tail PR.

### G5. Tauri auto-updater

**Dimension:** Auto-update / crash reporting / analytics. **Missing:** A Tauri-native auto-updater flow paired with ADR-014 release signing. **Why it matters for V1:** Without auto-update, post-V1 users do not pick up patch releases without manual download. The reference observable surface ships an updater equivalent (Sparkle-class). **Sizing:** Multi-PR — Tauri updater configuration + signing chain integration + UX surface (in-app update banner, defer / install-now affordance). Rough estimate: 3-5 PRs. **Sequence:** Pair with ADR-014 release signing; should land before V1 RC.

## 5. Items recommended for AO-deferral (post-MVP)

Seven items map to existing AO direction or are appropriate to defer past V1:

- **ISF native import** — `EV-IMPORT-ISF-REJECT` already landed at `Pass` under design-deferred-to-V2 narrowing per PR #358 / `8fadc55`. No V1 work owed.
- **OpenCV / camera live-feed** — Wave 1c-β closed 2026-05-07 (camera + Syphon adapters at synthetic-narrowing per ADR-020 + ADR-021); per-platform nokhwa concrete bindings + Syphon `Send`-trait reconciliation + `list_syphon_servers` `NSDistributedNotificationCenter` observer + `publish_syphon_output` real-impl + real hardware validation carry forward as known follow-up PRs per `docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a Wave 1c-β closure.
- **Custom macOS menu bar parity beyond Tauri default** — UX polish; Tauri default is acceptable for V1.
- **Demo media** (`demo_media.mp4` analog or equivalent) — nice-to-have, not gating for V1 RC.
- **Sparkle EdDSA exact-mechanism parity** — the Tauri updater is the chromaticity-native path per ADR-014; reference-side mechanism is not the parity target.
- **`hg_sdf.glsl` + metacontrols.glsl analogs** — clean-room rewrite or vendor-after-license-review; not required for V1 wireframe parity.
- **Spectrum widget retrofit + `AudioReactivity` 48-bar live-driven mode** — already deferred to post-MVP per the Wave 1c-α narrowing (PRs #339 / #340) and the Wave 1c-α-2 narrowing reduction (PRs #343 / #344 / Bootstrap PR 3).

## 6. Cross-reference

- `26-evidence-artifact-registry.md` — canonical EV-ID inventory.
- `29-cleanroom-legal-policy-baseline.md` — clean-room SOP and brand-trademark-review checklist.
- `38-scene-library-and-import-spec.md` — scene library + import design.
- `46-phase-4-exit-readiness.md` — Phase-4 EXIT readiness matrix.
- `44-phase-4-exit-scoping.md` — companion sequencing aid for Phase-4b / 4c / 4d.

## 7. Source audit reference

The `Contents/` audit was performed by a sub-agent on 2026-05-07. The agent inspected only:

- `Contents/Info.plist` (public app metadata).
- `Contents/Resources/new_scene/scene.json` (template schema shape).
- `Contents/Resources/{lib,images,shadertoy_images,views,Base.lproj,default,misc,synesthesia_reference_files}/` directory listings + filenames.
- Public file-tree shapes via `ls` and `find`.

The agent did NOT inspect:

- `Contents/MacOS/` binary.
- `Contents/Frameworks/*.framework/<binary>`.
- `Contents/_CodeSignature/`.
- `Contents/Resources/release/` per CLAUDE.md anti-patterns.
- Any `.synScene` / `.synScenePack` files.
- Any `.cpp` / `.h` / `.m` / `.mm` files.
- Internal XML of `.xib` / `.storyboardc` / `.nib` files.

Per the clean-room SOP brand-trademark-review checklist, no reference-app product name appears in the user-facing description columns of evidence rows or shipped UI strings derived from this audit.

## 8. When this doc is wrong

This doc is a planning artifact derived from an observable audit at a single moment in time. If a parity claim disagrees with the tree at a later HEAD, the tree wins — refresh this doc in a follow-up `docs(repo)` commit. Stale parity-gap inventory is worse than none.

## 9. Refresh log

- **2026-05-10 refresh** — V1-RC content + release pipeline fanout (PRs `#425` / `0f67340` Wave 1d-β PR 2 scene library expansion, `#426` / `b57bbfc` Wave 1d-α PR 3 corpus expansion, `#423` / `ad3c96b` Wave 1e-γ release.yml workflow scaffold, plus the three release.yml bug-fix PRs `#428` / `#429` / `#430`) closed `EV-DOCS-CORPUS` and `EV-LIBRARY-CORPUS-MIN` to `Pass`. Section 2 cumulative parity rollup updated (content / asset bundling `~15%` → `~30-35%`; update / telemetry infrastructure `~30%` → `~50-55%`); G1 + G2 gap statements rewritten to reflect closure; the prior "None of the bundled-content surfaces have been authored from a clean-room baseline yet" claim was retracted (PR `#425` + PR `#426` both clean-room per `docs/29` discipline). G3 (Shadertoy asset library) + G4 (per-source-class fallback thumbnails) + G5 (Tauri auto-updater production cert procurement) unchanged from the 2026-05-07 baseline.
