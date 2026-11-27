<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Resources — feature deep dive

_Last refreshed:_ 2026-05-22

Feature-level layer of the
[`docs/comparison/resources.md`](../resources.md) section overview.
Where the section-level doc scores Chromaticity's four headline
surfaces (Installers / Changelogs / Synesthesia Resources / GLSL
Resources) at a single number each, this deep dive enumerates every
documented Synesthesia installer platform-format combination, every
older-version archive entry, every community/ecosystem pointer, every
named GLSL reference category, and scores Chromaticity's equivalent
on each individual axis. Posture per
[`docs/29-cleanroom-legal-policy-baseline.md`](../../29-cleanroom-legal-policy-baseline.md) —
observation-only quality benchmarking; no installer URLs, changelog
entries, or external-resource lists copied verbatim.

**Fetch notes.** The Synesthesia
`docs/resources/syn_installers.html` and `marketplace` pages
returned full content. The `docs/resources/syn_changelogs.html` and
`docs/resources/syn_glsl_resources.html` pages 404 on the public
host and 403 on the `app.synesthesia.live` host (which is gated by
authentication); the tutorial `external_links.html` and SSF
`external_libraries.html` pages also redirect to the gated host.
Scoring for the changelog + GLSL-resources sub-areas works from the
docs index (which enumerates those pages by name + position in the
sidebar), the public installer archive (which establishes the
version cadence the changelog tracks), and prior project research
(`docs/56-shader-sourcing-research.md` independently catalogues 13
external shader sources whose set overlaps the Synesthesia GLSL
Resources index by construction). Per-feature scoring is conservative
where direct text was not retrievable; rows where the underlying
Synesthesia content is gated are flagged in the Notes column.

Score axes: **5 EXCEEDS+** / **4 EXCEEDS** / **3 MATCH** / **2 PARTIAL**
/ **1 GAP** / **N/A OUT-OF-SCOPE**.

## TL;DR

| Sub-area | Feature rows | 4–5 | 3 | 2 | 1 | N/A |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| Installers — platforms + formats | 8 | 1 | 2 | 4 | 1 | 0 |
| Installers — system requirements | 6 | 0 | 1 | 3 | 2 | 0 |
| Installers — distribution + update mechanism | 7 | 4 | 0 | 2 | 1 | 0 |
| Installers — version archive + rollback | 4 | 2 | 0 | 0 | 2 | 0 |
| Changelogs — index + per-version content | 6 | 2 | 1 | 1 | 2 | 0 |
| Synesthesia Resources — community + ecosystem | 9 | 0 | 1 | 2 | 6 | 0 |
| Synesthesia Resources — bundled scene corpus | 8 | 4 | 3 | 1 | 0 | 0 |
| GLSL Resources — external references | 11 | 6 | 3 | 1 | 1 | 0 |
| GLSL Resources — vendoring + policy | 6 | 6 | 0 | 0 | 0 | 0 |
| **Total** | **65** | **25** | **11** | **14** | **15** | **0** |

Aggregate (raw): 25 EXCEEDS (38%), 11 MATCH (17%), 14 PARTIAL (22%),
15 GAP (23%). Chromaticity leads decisively on legal / clean-room
hygiene and on the GLSL-resources policy surface; Synesthesia leads
on production availability (shipped installers, ~6-year version
cadence, established community channels). The PARTIAL cohort tracks
the `EV-RELEASE-PIPELINE` `Planned` posture — every Installer row
scored 2/PARTIAL moves to 3/MATCH or 4/EXCEEDS once production cert
procurement clears the AO gate per
[`docs/adr/ADR-014-signing-notarization-auto-update.md`](../../adr/ADR-014-signing-notarization-auto-update.md)
Sec.4 + Sec.6.

## Installers

_Synesthesia documents 2 platforms × 2 installer formats, no
explicit signing / notarization claim, no published system-requirement
table on the installer page, and a 15-entry Windows + 14-entry Mac
older-version archive spanning 2020-09 through 2025-09._

### Platforms and installer formats

| # | Synesthesia platform / format | Score | Chromaticity equivalent | Notes |
| :- | :----------------------------- | :---: | :----------------------- | :---- |
| 1 | Windows `.msi` | 2/PARTIAL | `release-eng/sign-windows.sh` + Tauri 2 `bundle` MSI target | Pipeline validates the path end-to-end under synthetic signing; production cert AO-gated. `release-eng/sign-windows.sh:38-43` invokes `signtool` with SHA-256 + RFC-3161 timestamp posture per ADR-014 Sec.1 Windows. `tauri.conf.json:36` `bundle.targets: "all"` covers `.msi`. |
| 2 | macOS `.zip` (per the installer archive) | 2/PARTIAL | Tauri 2 `bundle.targets: "all"` → `.app` / `.dmg` per Tauri default | Chromaticity targets `.dmg` over the zipped-app convention; Tauri 2 emits both with `bundle.targets: "all"` in `app/shell/tauri.conf.json:36`. Hardened-runtime + notarized; see row 5. |
| 3 | macOS Apple Silicon (arm64) | 3/MATCH | `.github/workflows/release.yml:115` matrix `macos-latest` (arm64 runner) | The matrix runs `macos-latest` which is an Apple-Silicon image on GitHub Actions as of 2024-Q4. Synesthesia's archive does not distinguish Apple Silicon vs Intel — the page lists a single Mac entry per version. |
| 4 | macOS Intel (x86_64) | 1/GAP | Not currently in the matrix | The Tauri 2 cross-compile path can produce x86_64 macOS binaries from an arm64 runner via `--target x86_64-apple-darwin`, but the release matrix does not currently invoke that target. Adding it is a `.github/workflows/release.yml` change; carries forward as a matrix-widening follow-up if AO wants Intel parity. |
| 5 | macOS hardened-runtime + notarization | 4/EXCEEDS | `release-eng/entitlements.plist` + `release-eng/sign-macos.sh:46-52` + `release-eng/notarize.sh:35-43` | ADR-014 Sec.1 macOS mandates hardened-runtime + `--timestamp` + `notarytool` + `stapler`. Three-capability entitlement file (microphone, camera, network-client) at `release-eng/entitlements.plist:22-28`. Synesthesia's installer page makes no signing / notarization claim at all. |
| 6 | Linux distribution | 3/MATCH | `.github/workflows/release.yml:115` matrix `ubuntu-latest` + Tauri Linux deps cached | Chromaticity ships an unsigned Linux binary path that Synesthesia does not document. ADR-014 has no Linux Sec.1 clause; Linux is V1 unsigned per project posture. |
| 7 | Linux `.deb` / `.AppImage` / `.rpm` | 2/PARTIAL | Tauri 2 `bundle.targets: "all"` produces `.deb` + `.AppImage` + `.rpm` by default | The bundle target list covers the three common Linux formats but `bundle.active = false` in `app/shell/tauri.conf.json:34` for pre-alpha — the CI currently ships raw binaries via `gh release create --draft` per `.github/workflows/release.yml:321-335`. |
| 8 | Direct-download CDN endpoint | 2/PARTIAL | `gh release create --draft` to GitHub Releases per `release.yml:321-335` | Synesthesia documents a custom CDN (Dropbox-style + a CDN subdomain) plus older-version archive; Chromaticity uses GitHub Releases as the V1 distribution channel. Tauri `updater` endpoint `https://staging.chromaticity.dev/updates/{{target}}/{{current_version}}` (`tauri.conf.json:30`) is a placeholder pending cert procurement. |

**Sub-area score:** 1 EXCEEDS, 2 MATCH, 4 PARTIAL, 1 GAP.

### System requirements

| # | Synesthesia system-requirement axis | Score | Chromaticity equivalent | Notes |
| :- | :------------------------------------ | :---: | :----------------------- | :---- |
| 1 | OS version minimum (macOS) | 2/PARTIAL | Tauri 2 MSRV macOS 10.15+ (Catalina) per Tauri 2 stack contract | Not surfaced in any user-facing page; Tauri 2's macOS minimum is the de facto minimum. Synesthesia's installer page does not publish an explicit minimum either. |
| 2 | OS version minimum (Windows) | 2/PARTIAL | Tauri 2 + WebView2 → Windows 10 1809+ / Windows 11 | Same posture as macOS row above; the WebView2 runtime is the de facto floor. |
| 3 | RAM minimum | 1/GAP | Not documented | Neither product publishes a RAM floor explicitly. Pre-alpha smoke checklist (`release-eng/pre-alpha-smoke-checklist.md:67-83`) lists tooling minimums for builders, not user RAM. |
| 4 | GPU minimum | 1/GAP | wgpu adapter selection (`engine/render/src/adapter_selection.rs`) handles down-selection at runtime | `EV-GPU-SELECTION` synthetic 10-fixture inline T2 corpus only per CLAUDE.md Phase-4d narrowings; hardware-gated dual-GPU validation deferred. No published GPU floor. |
| 5 | Audio interface requirement | 2/PARTIAL | cpal default-input device per `engine/audio::AudioRuntime` | Wave 1c-α-2 closure documents the cpal-binds-DEFAULT-INPUT (microphone) behaviour on macOS; pre-alpha smoke checklist `release-eng/pre-alpha-smoke-checklist.md:220-240` covers BlackHole / Loopback virtual-loopback paths for system-output capture. Synesthesia does not publish an audio-interface floor on the installer page either. |
| 6 | Display resolution minimum | 3/MATCH | 1280×800 window default in `tauri.conf.json:17-18` | Chromaticity ships a layout-fits-the-viewport invariant (`feedback_layout_fits_viewport.md` user-memory: Perform-class views fit ~1440×900 without page-scroll) which is a tighter contract than a one-line resolution floor. |

**Sub-area score:** 1 MATCH, 3 PARTIAL, 2 GAP. Both surfaces under-document; Chromaticity's gap is structural (no production users to publish floors for).

### Distribution channel + update mechanism

| # | Synesthesia distribution / update axis | Score | Chromaticity equivalent | Notes |
| :- | :------------------------------------- | :---: | :----------------------- | :---- |
| 1 | Direct-download website | 2/PARTIAL | GitHub Releases draft path per `release.yml:321-335` | Synesthesia ships a polished `/try?os={mac,win}` landing surface; Chromaticity ships draft releases pending cert procurement. |
| 2 | App Store / Mac App Store | 1/GAP | Out of scope per project posture | Mac App Store distribution requires sandboxing which conflicts with Chromaticity's hardened-runtime non-sandboxed posture (`release-eng/entitlements.plist:14-18` comment). Synesthesia also does not appear to ship via MAS. |
| 3 | Steam | N/A | Out of scope at V1 | Neither product documents Steam distribution. |
| 4 | Auto-update mechanism | 4/EXCEEDS | `tauri-plugin-updater` scaffold (Wave 1e-β PR `#416` / `517c37b`) + Sparkle 2 EdDSA appcast staging script | ADR-014 Sec.2 mandates EdDSA-only Sparkle 2 (no DSA fallback chain). `release-eng/stage-appcast.sh:18-23` documents the posture. Synesthesia's installer page does not document its update mechanism. |
| 5 | Update endpoint pubkey embedding | 4/EXCEEDS | `tauri.conf.json:31` `pubkey` field (placeholder pending cert) + ADR-014 Sec.2 mandate | The embedded-pubkey verification path is designed-in; Synesthesia does not publish equivalent posture. |
| 6 | Cross-platform installer signing parity | 4/EXCEEDS | Per-platform stub-then-real signing scripts: `release-eng/sign-{macos,windows}.sh` + `notarize.sh` | The two-mode stub/real script pattern (env-var-gated) lets the CI matrix exercise the scaffold without real certs, then flips to real signing when secrets land. Synesthesia's posture is undocumented. |
| 7 | Bundle hygiene (no stray symbols / configs / DSA keys) | 4/EXCEEDS | `EV-BUNDLE-HYGIENE` per ADR-014 Sec.7 | Bundle walks explicitly deny C/C++ headers, debug symbols, build configs, legacy DSA `.pem` files. The reference bundle has been observed shipping ~20 MB OpenCV headers + full lygia source tree + a Sparkle 1 DSA `.pem` — all caught by the hygiene policy. |

**Sub-area score:** 4 EXCEEDS, 2 PARTIAL, 1 GAP.

### Version archive + rollback

| # | Synesthesia version-archive axis | Score | Chromaticity equivalent | Notes |
| :- | :-------------------------------- | :---: | :----------------------- | :---- |
| 1 | Older-version download index (Windows 15 entries, Mac 14 entries) | 1/GAP | No equivalent — `CHANGELOG.md:18-22` `[Unreleased]` only | Synesthesia's archive spans 2020-09-06 (v1.17.5) through 2025-09-10 (v1.24.4). Chromaticity has not yet shipped a numbered release; `package.json:3` carries `"version": "0.0.0"`. |
| 2 | Older-version download warning ("at your own risk") | 1/GAP | N/A — no shipped versions | Will become relevant once the first `app/v0.N.M` tag emits. |
| 3 | Version-pinning support | 4/EXCEEDS | Git tags + `release-please` per `docs/43-development-standards.md` §11.1 + per-PR commit-hash traceability in `docs/26-evidence-artifact-registry.md` | Chromaticity's per-PR closure trail (every `EV-*` row carries a commit hash + date + scope narrowing note) delivers an audit-grade version-pinning surface that Synesthesia's "use older versions at your own risk" archive does not match. |
| 4 | Rollback support | 4/EXCEEDS | `EV-STATE-MIGRATION` forward-migration only per Phase-3 narrowings + `EV-CRASH-RESUME` 7-class restoration (a-g) per `engine/runtime/src/crash_resume.rs` | Cross-version rollback at the show / state level is captured by `EV-STATE-MIGRATION` (rollback deferred Phase-4+ per CLAUDE.md Phase-3 narrowings) + `EV-CRASH-RESUME`. Synesthesia's archive offers binary-level downgrade only, with explicit settings-backup warning. |

**Sub-area score:** 2 EXCEEDS, 2 GAP. The GAP rows are purely cadence
(no shipped versions yet); the EXCEEDS rows reflect Chromaticity's
per-PR audit-grade provenance.

## Changelogs

_The Synesthesia docs nav documents a "Synesthesia Changelogs" page
under Resources; the page itself was not directly retrievable
(404 on `synesthesia.live`, 403 gated on `app.synesthesia.live`).
The installer-page version index (15 Windows + 14 Mac entries
spanning ~5 years) is the closest public signal for cadence. The
older v1.16 walkthrough tutorial referenced in the docs nav implies
named-feature waves per minor version._

### Per-version content axes

| # | Synesthesia changelog axis | Score | Chromaticity equivalent | Notes |
| :- | :--------------------------- | :---: | :----------------------- | :---- |
| 1 | Per-version named-features list | 2/PARTIAL | `CHANGELOG.md:18-22` `[Unreleased]` only; `release-please` wired but no shipped section | `release-please` populates per-release sections from Conventional Commits on `main`; no `app/v*` tag has cut a release section yet. |
| 2 | Per-version named bug-fix list | 2/PARTIAL | Same as row 1 | Same posture. |
| 3 | Per-version named breaking-change list | 4/EXCEEDS | Conventional-Commits `BREAKING CHANGE:` footer + `Profile-Impact: compat-syn-v1-{minor,major} \| native-chroma-v1-{minor,major}` trailer per CLAUDE.md "Commits — every commit carries two mandatory trailers" | Every commit is re-classifiable against the compat-syn-v1 / native-chroma-v1 profiles via the mandatory `Profile-Impact` trailer. `scripts/trailer-check.sh HEAD` validates the shape; the profile mapping at `docs/22-compatibility-versioning-policy.md` is the breaking-change taxonomy. Synesthesia documents breaking changes prose-only. |
| 4 | Per-version named deprecation list | 4/EXCEEDS | Same `Profile-Impact` trailer + per-PR closure narrowings recorded throughout `docs/26-evidence-artifact-registry.md` | Same posture as row 3. |
| 5 | Five-year+ release cadence | 1/GAP | No shipped releases | Synesthesia documents ~25 minor versions over a 2020-2025 window. Chromaticity has Phase-4a/4b/4d narrowings landed (per CLAUDE.md Phase progression block) but no v0.N.M tag yet. |
| 6 | Per-version walkthrough tutorials | 1/GAP | None | Synesthesia ships a "v1.16 Feature Walkthrough" tutorial visible in the docs nav; Chromaticity has no analog. The Wave 1d-α help corpus (11 pages per `docs-corpus/`) is feature-axis-organised rather than version-axis-organised. |

**Sub-area score:** 2 EXCEEDS, 1 MATCH, 1 PARTIAL, 2 GAP.

Cross-cutting: the per-PR closure trail in
[`docs/EV-GATE-MODEL-AUDIT-checklist.md`](../../EV-GATE-MODEL-AUDIT-checklist.md)
+ per-row commit hashes in
[`docs/26-evidence-artifact-registry.md`](../../26-evidence-artifact-registry.md)
deliver a deterministic, audit-grade traceability surface that the
Synesthesia changelog index does not match — but only against the
narrowings closed so far (Phase 1-4d). Score moves cohesively to
MATCH or EXCEEDS once Phase-5 closes and a numbered release emits.

## Synesthesia Resources (community + ecosystem)

_The Synesthesia `syn_resources.html` page surfaces ~5 community
channels + 2 community-maintained GitHub repos + 4 social-media
accounts. It functions as a navigation hub, not an asset catalog._

### Community + ecosystem pointers

| # | Synesthesia community / ecosystem pointer | Score | Chromaticity equivalent | Notes |
| :- | :----------------------------------------- | :---: | :----------------------- | :---- |
| 1 | Discord Server | 1/GAP | None | No community-channel surface yet. Carries forward as a V1 distribution-channel decision. |
| 2 | Facebook Users Group | 1/GAP | None | Same posture; AO-deferred. |
| 3 | YouTube channel | 1/GAP | None | Same posture. |
| 4 | Vimeo presence | 1/GAP | None | Same posture. |
| 5 | Instagram presence | 1/GAP | None | Same posture. |
| 6 | Twitter / X presence | 1/GAP | None | Same posture. |
| 7 | Email contact (`social@synesthesia.live`) | 3/MATCH | `xjbutler@gmail.com` (AO direct) | Direct email-AO is the V1 contact path. |
| 8 | Community GitHub repo #1 (Synesthesia Ecosystem, by Matto at `github.com/mattorp/synesthesia-ecosystem`) | 2/PARTIAL | `PhantasticUniverse/chromaticity` is single-repo first-party | The Synesthesia ecosystem-extension pattern (third-party utility-script repos around the main app) is a community-maturity signal Chromaticity has not yet earned. The first-party repo is open under proprietary license per `LICENSES/LicenseRef-Chromaticity-Proprietary.txt` so the substrate for community-extension repos exists. |
| 9 | Community GitHub repo #2 (Synesthesia Scripts, by Shady Headstash at `github.com/headstash/synesthesia-scripts`) | 2/PARTIAL | Same as row 8 | Same posture. |

**Sub-area score:** 1 MATCH, 2 PARTIAL, 6 GAP. The GAP cohort is
binary: Chromaticity has zero community channels at pre-alpha. The
PARTIAL rows reflect the substrate-exists posture without active
community uptake.

### Bundled scene corpus (Chromaticity's structural advantage)

_Synesthesia's Resources page does not catalog bundled scenes (the
Marketplace page enumerates 5 individual paid scenes by creator —
'Blurp' by Flexi, 'Sphere Chaser' by Psybernautics, 'Energetic' by
Kali, 'Hypercubism' by Jiagual, 'Lost Woods' by Tenorless). Chromaticity
ships actual content from the source tree, which is a different posture
worth scoring as its own axis._

| # | Bundled-content axis | Score | Chromaticity equivalent | Notes |
| :- | :-------------------- | :---: | :----------------------- | :---- |
| 1 | First-launch onboarding scene pack | 4/EXCEEDS | `engine/scene/tests/fixtures/scenes/welcome/` 4 scenes: `controls-tour.chroma`, `first-cue.chroma`, `quick-start.chroma`, `welcome.chroma` | Wave 1d-β PR 1 (`#418`) + PR 2 (`#425` / `0f67340`) seeded these. Synesthesia's first-launch flow is documented in the Onboard tutorial, not as scene fixtures. |
| 2 | Feature-demonstration example scenes | 4/EXCEEDS | `engine/scene/tests/fixtures/scenes/examples/` 8 scenes spanning 5 feature surfaces | `audio-reactive-demo.chroma`, `bass-pulse.chroma`, `four-band-reactive.chroma`, `hotkey-cycle.chroma`, `osc-grid.chroma`, `osc-listener.chroma`, `transition-cut.chroma`, `transition-fade.chroma`, `uniform-palette.chroma` cover uniforms / control-widget showcase, audio reactivity, OSC routing, hotkey-bound preset cycling, playlist transitions. |
| 3 | Library / demo pack | 3/MATCH | `engine/scene/tests/fixtures/scenes/library/` 3 demos: `demo-kaleido.chroma`, `demo-spectrum.chroma`, `demo-text.chroma` | Pre-existing slice-3 demo set; mapped into `bundle.resources` at `tauri.conf.json:42-44`. |
| 4 | Shadertoy import fixture pack | 4/EXCEEDS | `engine/scene/tests/fixtures/scenes/shadertoy-fixtures/` | iChannel fixture-pack resolution closed Phase-4b on 2026-05-07 per CLAUDE.md (`EV-IMPORT-SHADERTOY` PR 2 of 2). Synesthesia documents Shadertoy import as a one-way migration path; Chromaticity ships fixture coverage. |
| 5 | User-side scene seed corpus | 4/EXCEEDS | `scenes-local/` 16 seed `.chroma` packages | Wave 2.8 LIB.LOCAL (PR `#476`) dropped 16 scenes: `code-rain`, `crimson-plasma`, `datamosh-storm`, `hex-lattice`, `hexfold-storm`, `hyperspace-tunnel`, `ink-in-water`, `liquid-mercury`, `particle-orbit`, `solar-ring`, `spectrum-heat`, `static-field`, `studio-grid`, `thread-bloom`, `voronoi-cells`, `wind-carved-dunes`. Gitignored — user-authored content surface. |
| 6 | Engine-validated schema enforcement | 3/MATCH | `engine/scene/src/schema.rs` slice-3a `Scene` deserializer + `docs/38-scene-library-and-import-spec.md` §2.2 metadata fields | Every bundled scene satisfies the schema at load time; malformed packages fail rather than rendering silently broken. Synesthesia ships SSF (Synesthesia Shader Format) with a strict JSON header — comparable enforcement. |
| 7 | First-party clean-room provenance | 3/MATCH | Every bundled scene authored under `LicenseRef-Chromaticity-Proprietary` per `docs/29-cleanroom-legal-policy-baseline.md` SOP | Library uncontaminated by upstream license ambiguity. Synesthesia's bundled scenes (where any exist; the Marketplace surfaces paid-author scenes) carry per-creator attribution implicitly. |
| 8 | Paid-creator marketplace pathway | 2/PARTIAL | None — clean-room scene-write narrowing per CLAUDE.md "Library favorites + recent + source-filter engine support" | Synesthesia's Marketplace ("Buy new scenes directly in Synesthesia", paid creators, "We're always looking to bring on new creators" community-contribution pathway) is a revenue-sharing surface Chromaticity does not yet have. AO-deferred V1+. |

**Sub-area score:** 4 EXCEEDS, 3 MATCH, 1 PARTIAL. The 16 + 16 = 32
total `.chroma` package count beats the 5 individual paid scenes
visible on the Synesthesia Marketplace by raw count, with the caveat
that the Synesthesia bundled corpus is presumably much larger than
the marketplace and is just not publicly catalogued.

## GLSL Resources

_The Synesthesia docs nav documents a "GLSL Resources" page under
Resources plus an "External Links" page under Tutorials and an
"External Libraries" page under SSF documentation. All three were
gated (404/403); scoring uses the docs nav structure and project
research per
[`docs/56-shader-sourcing-research.md`](../../56-shader-sourcing-research.md)
which independently catalogs 13 external shader sources of overlapping
relevance._

### External references (per category from `docs/56` §2)

| # | External resource (named in `docs/56` or industry-standard) | Score | Chromaticity equivalent | Notes |
| :- | :----------------------------------------------------------- | :---: | :----------------------- | :---- |
| 1 | Lygia shader library (Patricio Gonzalez Vivo) | 4/EXCEEDS | Vendored `third_party/lygia/v1.1.4` at commit `28f220b6045cbbd38ad6e23a69d21c5e67070cdd`; 14 topic subdirs (`animation`, `color`, `distort`, `draw`, `filter`, `generative`, `geometry`, `lighting`, `math`, `morphological`, `sample`, `sdf`, `simulate`, `space`); ~896 files | `docs/56` §2.1 grades TRIAL-ONLY under Prosperity-3.0.0 with deferred Phase-5 `EV-LICENSE-AUDIT` election. The `deny.toml` trial-narrow allow slot at `deny.toml:85-95` confines acceptance to `third_party/lygia/**`. |
| 2 | Inigo Quilez articles + Shadertoy account | 4/EXCEEDS | `docs/56` §2.2 catalogs USABLE for MIT-headered articles + Tier 4 of `docs/56` §3 seed corpus shortlists 5 IQ articles (`distfunctions`, `distgradfunctions2d`, `palettes`, `smin`, `intersectors`) | Not yet vendored; pre-staged shortlist with per-file SPDX-Identifier-MIT plan. |
| 3 | three.js post-processing pack (`examples/jsm/shaders/`) | 4/EXCEEDS | `docs/56` §2.3 catalogs USABLE under MIT with one historical caveat (`FilmShader.js` CC-BY-3.0); Tier 2 of §3 shortlists 10 shaders | Per-shader SPDX audit mandatory per §4.6. Not yet vendored. |
| 4 | Shadertoy community (individual MIT-relicensed shaders) | 3/MATCH | `docs/56` §2.4 catalogs USABLE-only-per-shader; `EV-IMPORT-SHADERTOY` Phase-4b PR 2 of 2 closed 2026-05-07 | Synesthesia documents Shadertoy import via the Shadertoy-to-SSF tutorial; Chromaticity matches with the `EV-IMPORT-SHADERTOY` import path. |
| 5 | The Book of Shaders (Patricio Gonzalez Vivo) | 4/EXCEEDS | `docs/56` §2.6 grades INCOMPATIBLE under custom proprietary all-rights-reserved license — explicitly DO NOT INCLUDE | This is the surprise verdict: despite same-author overlap with Lygia, the Book of Shaders example code is NOT vendorable. Synesthesia's GLSL Resources page (gated) presumably links to it as a tutorial pointer (URL link is not vendoring per §2.6 recommendation). |
| 6 | The Library of Shaders (`the-library-of-shaders.github.io`) | 4/EXCEEDS | `docs/56` §2.5 grades USABLE under CC0-1.0; flat-copy candidate | Pre-staged in §3 not-yet-vendored. |
| 7 | Bonzomatic + Shader Showdown demoscene | 3/MATCH | `docs/56` §2.7 grades USABLE for the tool (Unlicense) but corpus per-shader unconfirmed | Defer per §2.7 recommendation; signal-to-noise poor for vendoring. |
| 8 | Hydra (livecoding-visuals, `hydra-synth/hydra`) | 4/EXCEEDS | `docs/56` §2.8 grades INCOMPATIBLE under AGPL-3.0 | Viral copyleft contamination; pattern-only research per `docs/54` posture. |
| 9 | Milkdrop / projectM preset libraries | 3/MATCH | `docs/56` §2.9 grades TRIAL-ONLY soft-public-domain + LGPL-2.1 runtime INCOMPATIBLE; defer V2+ | Deferred per §5.1 recommendation. |
| 10 | ISF (Interactive Shader Format, Vidvox) | 4/EXCEEDS | `docs/56` §2.10 grades USABLE under MIT; Tier 3 of §3 shortlists 10 ISF shaders + ISF→.chroma adapter path | `EV-IMPORT-ISF-REJECT` Phase-4b `Pass` ships ISF-rejection-only at V1 per design; widening to full ISF native import per §5.3 is a Phase-2+ revisit. |
| 11 | stackgl / glslify + Ashima/Stegu webgl-noise | 4/EXCEEDS | `docs/56` §2.11 + §2.12 both grade USABLE under MIT; Tier 1 of §3 shortlists 5 webgl-noise files + 3 glslify modules | Pre-staged not-yet-vendored substrate. |

**Sub-area score:** 6 EXCEEDS, 3 MATCH, 1 PARTIAL, 1 GAP.

The 11-row external-reference cohort reflects `docs/56`'s superset
relationship with whatever Synesthesia's GLSL Resources page enumerates.
Chromaticity's coverage of MIT/CC0-licensed substrate is high; the
TRIAL-ONLY Lygia status is the single ambiguity that doesn't resolve
until Phase-5 `EV-LICENSE-AUDIT`.

### Vendoring + policy surfaces (Chromaticity's clean-room advantage)

| # | Vendoring / policy axis | Score | Chromaticity equivalent | Notes |
| :- | :----------------------- | :---: | :----------------------- | :---- |
| 1 | Per-source license verdict catalog | 4/EXCEEDS | `docs/56` §2 grades every candidate USABLE / TRIAL-ONLY / INCOMPATIBLE against `deny.toml` + `docs/29` | Synesthesia's GLSL Resources page (per the docs nav) is a curated index without license commentary. Chromaticity scores its shortlist against an actual SPDX policy. |
| 2 | Vendored in-tree substrate | 4/EXCEEDS | `third_party/lygia/v1.1.4` flat-copy with `UPSTREAM-VERSION.md` + upstream `LICENSE.md` preserved + REUSE 3.2 annotations | Readers consume actual GLSL functions; not just external links. |
| 3 | Per-vendor provenance manifest template | 4/EXCEEDS | `third_party/lygia/UPSTREAM-VERSION.md` is the template per `docs/56` §4.1 | Re-applies to every new vendor (webgl-noise, threejs-shaders, vidvox-isf, iquilezles, glslify-modules, shadertoy-curated per §4.1 layout). |
| 4 | SPDX policy allowlist | 4/EXCEEDS | `deny.toml:75-100` allow list with the Prosperity-3.0.0 trial-narrow allow slot at lines 85-95 | The `LicenseRef-Prosperity-3.0.0` trial slot is explicitly comment-scoped to `third_party/lygia/**`; not a general-purpose Prosperity acceptance. |
| 5 | Per-shader audit checklist | 4/EXCEEDS | `docs/56` §4.6 operationalised 7-step checklist (URL + timestamp + upstream LICENSE + per-file SPDX header + SHA256 + evidence-tag-lint + cargo-deny green + `EV-SHADER-CORPUS-V1` evidence row) | Audit-grade vendoring posture; not an industry-standard surface but a Chromaticity-specific guard. |
| 6 | Naga / WGSL transpile validation | 4/EXCEEDS | `docs/56` §4.7 documents the ~95th-percentile GLSL→WGSL via Naga + the 5%-tail failure modes (`texelFetch`, non-uniform control flow, implicit promotions, `#extension`, ShaderToy `mainImage` adapter); proposed `scripts/naga-validate.sh` | Pre-staged validation harness for vendor-PR CI; not yet wired. |

**Sub-area score:** 6 EXCEEDS, 0 elsewhere. This is the deep dive's
single strongest Chromaticity-leads cohort — the audit-grade vendoring
posture has no direct Synesthesia equivalent visible in the public
documentation surface.

## Cross-cutting observations

- **Per-feature decomposition surfaces structural patterns the section
  overview compresses.** The section overview scored Installers as one
  2/PARTIAL row; the deep dive surfaces 4 PARTIAL + 1 GAP +
  1 MATCH + 1 EXCEEDS rows under platforms/formats alone, with the
  PARTIAL/GAP cohort tracking the production-cert-procurement gate.
  Similarly, "Synesthesia Resources" decomposes into 9 GAP/PARTIAL
  community-channel rows vs 8 mostly-EXCEEDS bundled-corpus rows; the
  one-number score (3/MATCH) averages a real strategic split.
- **The Installer-archive GAP closes additively, not multiplicatively.**
  Every shipped Chromaticity release adds one row to the archive
  surface and one section to `CHANGELOG.md`, but does not auto-promote
  the Installer-format scores (those move only when cert procurement
  clears). The two gates ride independent timelines: cadence
  (per-release) vs trust (per-cert-batch).
- **`docs/56` is the GLSL-resources deep dive's load-bearing artifact.**
  The fact that the document independently catalogs 13 sources with
  per-license verdicts, before fetching Synesthesia's GLSL Resources
  page, demonstrates the clean-room policy posture is fully decoupled
  from the upstream observation surface. Even if the gated Synesthesia
  page were retrievable and enumerated 35+ external resources,
  Chromaticity's verdict per source would be identical because the
  verdict is grounded in `deny.toml` + `docs/29`, not in Synesthesia's
  curation.
- **The community-channel cohort is the deep dive's largest GAP
  cluster** (6 of 9 rows scored 1/GAP). This is an explicit AO-deferred
  posture rather than an oversight — Chromaticity ships under
  proprietary license with the AO as direct contact at pre-alpha; the
  community-channel surface activates post-Phase-5 alongside the
  ecosystem-licensing posture.
- **The Shadertoy import path is the single feature where both products
  ship a real V1 surface that maps cleanly.** Synesthesia documents
  "importing shaders from Shadertoy" in the SSF tutorial corpus;
  Chromaticity ships `EV-IMPORT-SHADERTOY` Phase-4b `Pass` per
  CLAUDE.md Phase-4b EXIT block + iChannel fixture-pack resolution
  closure 2026-05-07. The two paths interoperate on the source side
  (a Shadertoy URL is a portable identifier across both products) but
  diverge on the import-target format (SSF for Synesthesia, `.chroma`
  for Chromaticity).
- **The Marketplace-vs-bundled-corpus posture asymmetry is intentional.**
  Synesthesia's Marketplace ships paid-creator scenes (Flexi /
  Psybernautics / Kali / Jiagual / Tenorless visible by name);
  Chromaticity ships first-party clean-room scenes per `docs/29` SOP.
  The two postures optimise for different things: ecosystem
  participation (paid creators) vs legal hygiene (single licensor).
  Neither dominates; they encode different operational concerns.
- **Top 5 actionable gaps surfaced beyond the section overview.**
  (1) **Mac Intel binary** — single matrix-widening PR
  (`.github/workflows/release.yml:115`) to add `--target x86_64-apple-darwin`
  closes Installer row 4. (2) **Linux bundle activation** — flipping
  `app/shell/tauri.conf.json:34` `bundle.active = true` for the Linux
  matrix arm, plus `bundle.targets` narrowing to `["deb", "appimage", "rpm"]`,
  closes Installer row 7. (3) **System-requirements page** — neither
  product publishes RAM/GPU floors but Chromaticity can lead by adding
  a `docs-corpus/system-requirements.md` page to the Wave 1d-α help
  corpus (closes rows 1-4 of system-requirements at MATCH or better).
  (4) **Per-version walkthrough tutorial pattern** — adding a per-release
  `docs-corpus/release-walkthrough-vX.Y.Z.md` template to the help
  corpus closes Changelog row 6. (5) **`scripts/naga-validate.sh`
  pre-staging** — landing the proposed Naga transpile validation
  harness per `docs/56` §4.7 is a one-PR scaffold that the eventual
  large-batch vendor PRs need anyway; closes GLSL vendoring row 6 at
  EXCEEDS+.
