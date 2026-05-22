<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> -->

# Resources comparison

_Last refreshed:_ 2026-05-22

Match-or-exceed quality benchmarking of Chromaticity's release
pipeline, scene corpus, and external-resource surfaces against the
public Synesthesia "Resources" documentation group (installers,
changelogs, scene/asset resources, GLSL resources). Posture per
[`docs/comparison/README.md`](README.md) and
[`docs/29-cleanroom-legal-policy-baseline.md`](../29-cleanroom-legal-policy-baseline.md) —
observation-only quality benchmarking; no installer URLs, changelog
entries, or external-resource lists copied verbatim.

## Scope

The Synesthesia Resources group has four distinct pages: Installers
(per-OS download index + version history), Changelogs (index of
per-version release notes), Synesthesia Resources (community / social
hub pointing to ecosystem repos), and GLSL Resources (curated index
of external shader learning material). This doc scores Chromaticity's
equivalent surface for each.

## TL;DR

| Topic                  | Score      | Headline                                                                                                  |
| :--------------------- | :--------- | :-------------------------------------------------------------------------------------------------------- |
| Installers             | 2 / PARTIAL | Pipeline scaffold + workflow validated; production certs AO-gated (`EV-RELEASE-PIPELINE` `Planned`).      |
| Changelogs             | 3 / MATCH  | `release-please` + Keep-a-Changelog + per-PR closing-docs trail; no shipped releases yet.                 |
| Synesthesia Resources (asset library) | 3 / MATCH | 16-package clean-room bundled corpus + 16-scene `scenes-local/` seed; engine indexer live. |
| GLSL Resources         | 4 / EXCEEDS | Vendored Lygia substrate + 13-source `docs/56` research catalog with per-source license verdicts.        |

Cumulative: 1 PARTIAL, 2 MATCH, 1 EXCEEDS. Chromaticity leads on
legal / clean-room hygiene; Synesthesia leads on availability and
ecosystem maturity (shipped installers, decade-long release history,
established community channels).

## Per-topic scoring

### Installers

- **Score:** 2 / PARTIAL
- **Chromaticity equivalent:** `release-eng/` operator scaffold +
  `.github/workflows/release.yml` cross-platform matrix +
  [`docs/adr/ADR-014-signing-notarization-auto-update.md`](../adr/ADR-014-signing-notarization-auto-update.md) +
  `EV-RELEASE-PIPELINE` (`docs/26-evidence-artifact-registry.md:115`).
- **Match notes:** Chromaticity targets the same two desktop
  platforms Synesthesia documents (macOS + Windows) and adds
  `ubuntu-latest` to the matrix (`.github/workflows/release.yml:115`).
  The release pipeline is wired end-to-end in CI: per-platform
  Tauri 2 builds (`release.yml:186-187`), per-platform stub
  signing (`release.yml:199-225`), staging appcast assembly
  (`release.yml:239-252`), and `gh release create --draft`
  publication (`release.yml:321-335`). The Wave 1e scaffold landed
  2026-05-08 through 2026-05-10 with synthetic-signing smoke run
  `25628685936` validating the full chain end-to-end.
- **Exceed notes:** Two structural advantages over the documented
  Synesthesia posture: (a) **Hardened-runtime + notarization is
  designed-in from Phase 0** via ADR-014 (Apple Developer ID +
  hardened runtime + `--timestamp` + `notarytool` + EdDSA-only
  Sparkle 2 — `ADR-014-signing-notarization-auto-update.md:17-34`)
  and surfaced as a hard `entitlements.plist` declaring exactly
  three capabilities (microphone, camera, network-client per
  `release-eng/entitlements.plist`); the Synesthesia installer
  page makes no mention of signing or notarization at all.
  (b) **Bundle hygiene is enforced by `EV-BUNDLE-HYGIENE`** per
  ADR-014 Sec.7 — release candidates that contain stray C/C++
  headers, debug symbols, build-tool configs, or legacy DSA `.pem`
  files fail the pipeline (`ADR-014-signing-notarization-auto-update.md:67-81`).
  This catches the exact class of artifact that the reference
  bundle ships in error.
- **Gap notes:** **Synesthesia ships installers today; Chromaticity
  does not.** `EV-RELEASE-PIPELINE` remains `Planned` per
  `docs/26-evidence-artifact-registry.md:115`; the pipeline can
  exercise its stubs but cannot emit signed/notarized artifacts
  until production cert procurement (Apple Developer ID
  hardware-stored on YubiKey/FIPS-140-2 key, Windows EV
  code-signing cert from a trusted CA, Sparkle 2 EdDSA keypair)
  clears the AO gate per ADR-014 Sec.4 + Sec.6. The pre-alpha smoke
  path (`release-eng/pre-alpha-smoke-checklist.md`) covers unsigned
  local builds for the AO + a small trusted cohort, with
  Gatekeeper / SmartScreen warnings expected. The Synesthesia
  surface also exposes a multi-year version history index
  (~2020 through 2025) which Chromaticity has no equivalent of
  pending the first numbered release. Score will move to MATCH or
  EXCEEDS once the cert procurement clears and the first
  `app/v0.N.M` tag emits a non-draft signed release.

### Changelogs

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `CHANGELOG.md` (root, Keep-a-Changelog
  1.1.0 format, `release-please`-populated per
  [`docs/43-development-standards.md`](../43-development-standards.md) §11.1) +
  closing-docs sections in
  [`docs/EV-GATE-MODEL-AUDIT-checklist.md`](../EV-GATE-MODEL-AUDIT-checklist.md) +
  per-PR commit hashes recorded throughout
  [`docs/26-evidence-artifact-registry.md`](../26-evidence-artifact-registry.md).
- **Match notes:** Both projects centralise release notes in a
  documentation hub. Chromaticity uses an in-repo `CHANGELOG.md`
  populated automatically from Conventional Commits on every
  merge to `main` — header at `CHANGELOG.md:1-19` declares the
  Keep-a-Changelog format + `release-please` automation. Synesthesia
  uses a per-version index that links out to external
  release-note pages. Both surfaces target a mixed
  user-and-developer audience.
- **Exceed notes:** Chromaticity's per-PR closure trail in the
  evidence registry (every `EV-*` row carries a commit hash + date
  + scope narrowing note) and the Phase-EXIT records in the
  EV-GATE-MODEL-AUDIT-checklist deliver a deterministic,
  audit-grade traceability surface that the Synesthesia changelog
  index does not document. Every Phase-4 closure cited in
  `CLAUDE.md` is reproducible against the tree (e.g. Wave 1c-α-2
  audio bootstrap at PR `#343` + `#344` + `#345`; Wave 2.1
  cross-seam at PRs `#435-#445`; pre-alpha round 3 at PR `#488`
  + `#489`). The **two mandatory commit trailers** (`Signed-off-by`
  + `Profile-Impact`) make every commit re-classifiable against
  the compat-syn-v1 / native-chroma-v1 profiles
  (`CLAUDE.md` "Commits — every commit carries two mandatory
  trailers" section).
- **Gap notes:** Synesthesia documents ~25 minor versions over a
  five-year window (v1.0 through v1.25.1) — a real-world ship
  cadence Chromaticity has not yet exercised. The
  `CHANGELOG.md`'s `[Unreleased]` section currently reads "Phase-1
  implementation slices are in progress" (`CHANGELOG.md:18-22`) —
  no numbered release section exists yet. `package.json` carries
  `version: "0.0.0"`. The `release-please` automation is wired
  but has not produced a release section because no `app/v*` tag
  has been cut. Score moves to EXCEEDS when the first numbered
  release populates a real section and the per-PR closing-docs
  trail composes with the changelog.

### Synesthesia Resources (scene/asset library)

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** Bundled scene corpus under
  `engine/scene/tests/fixtures/scenes/` (16 `.chroma` packages
  across `welcome/` + `examples/` + `library/`) +
  `scenes-local/` user-side seed (16 additional `.chroma` packages,
  gitignored) + library indexer surfaces (`EV-LIBRARY-BUNDLE-LAYOUT`
  at `docs/26-evidence-artifact-registry.md:117`,
  `EV-LIBRARY-CORPUS-MIN` at line 118, `EV-LIBRARY-DISCOVERY` at
  line 80) + the in-app Library view + ADR-015 reference-app
  import path.
- **Match notes:** The Synesthesia Resources page functions as a
  directory hub pointing to community channels (Discord, Facebook
  group) and two community-maintained GitHub repositories for
  ecosystem extensions; it does not itself catalogue assets.
  Chromaticity's equivalent **ships actual content**: 16 clean-room
  `.chroma` packages organised across the canonical four-subdir
  taxonomy (`welcome/` first-launch onboarding, `examples/`
  per-feature demonstrations, `library/` bundled demos,
  `shadertoy-fixtures/` Shadertoy iChannel asset pack). The
  Library view's discovery surface (indexer + search + filter +
  thumbnails + tag chips + favorites + recent + source filter)
  closed in Wave 4b.2 / 4b.6 and was page-by-page solidified in
  Wave 2.5 (PRs `#457`-`#461`). User-authored content lives under
  `scenes-local/` (16 seed scenes per the Wave 2.8 LIB.LOCAL
  drop) with the same Library surfaces honouring user vs bundled
  source filters.
- **Exceed notes:** Two structural advantages: (a) **Engine-validated
  schema.** Every bundled scene is a `.chroma` package whose
  `scene.json` satisfies the slice-3a `Scene` deserializer in
  `engine/scene/src/schema.rs` and the
  `docs/38-scene-library-and-import-spec.md` §2.2 metadata fields;
  malformed packages fail at load time rather than rendering
  silently broken. (b) **First-party clean-room corpus** —
  every bundled scene is authored under
  `LicenseRef-Chromaticity-Proprietary` per `docs/29` SOP, so
  the library is uncontaminated by upstream license ambiguity.
- **Gap notes:** Synesthesia's ecosystem reach (Discord +
  Facebook group + community-maintained extension repos) is
  meaningful availability that Chromaticity does not yet have.
  Community-contribution pathways are explicitly deferred per the
  Wave 2.5 LIB.B / LIB.C narrowing — favorites / recent /
  source-filter all ship frontend-only via localStorage with
  engine-side support intentionally deferred per the clean-room
  scene-write narrowing (`CLAUDE.md` "Library favorites + recent
  + source-filter engine support"). The
  `shadertoy-fixtures/` subdirectory holds asset fixtures only;
  the broader Shadertoy-import path is `EV-IMPORT-SHADERTOY`
  (Phase-4b `Pass` per `CLAUDE.md` Phase-4b EXIT block).
  `EV-LIBRARY-CORPUS-MIN` (`docs/26-evidence-artifact-registry.md:118`)
  is `InProgress` — the welcome/examples packages are seeded in
  the source corpus but `app/shell/tauri.conf.json` does not yet
  map them into `bundle.resources`, so the bundled installer
  carries only the three `library/demo-*` packages at present.
  Score moves to EXCEEDS once the bundle-resource mapping widens
  and a community-contribution pathway opens.

### GLSL Resources

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** Vendored
  [`third_party/lygia/`](../../third_party/lygia/) v1.1.4 (437 `.glsl`
  files plus HLSL/Metal/WGSL variants spanning 13 topic
  subdirectories; ~896-file total surface per
  `docs/56-shader-sourcing-research.md:91-93`) +
  [`docs/56-shader-sourcing-research.md`](../56-shader-sourcing-research.md)
  13-source research catalog + ADR-014 Sec.7 bundle-hygiene
  posture +
  [`docs/29-cleanroom-legal-policy-baseline.md`](../29-cleanroom-legal-policy-baseline.md)
  Bundled GLSL Library Handling +
  `deny.toml` SPDX allowlist +
  `EV-LIBRARY-LICENSE-TRIAL` /
  `EV-LICENSE-AUDIT`
  (`docs/26-evidence-artifact-registry.md:65-66`).
- **Match notes:** Synesthesia's GLSL Resources page organises
  ~35+ external resources into five categories (editors /
  communities, language reference, tutorials, raymarching,
  meta-lists). Chromaticity's `docs/56` research catalog covers
  13 candidate upstream sources by name (Lygia, Inigo Quilez
  articles, three.js post-FX pack, Shadertoy community,
  Library of Shaders, Book of Shaders, Bonzomatic, Hydra,
  projectM/Milkdrop, ISF-Files, glslify/stackgl, Ashima/Stegu
  webgl-noise, commercial proprietary VJ tools) with
  comparable breadth of editor / community / tutorial /
  raymarching / library coverage. The reference points
  Synesthesia surfaces (Book of Shaders, Shadertoy, IQ's
  articles, three.js shaders) all appear in `docs/56` §2 with
  per-source compatibility verdicts.
- **Exceed notes:** Three meaningful exceeds: (a) **Per-source
  license verdict** — `docs/56` §2 grades every candidate as
  USABLE / TRIAL-ONLY / INCOMPATIBLE against the `deny.toml`
  allowlist + `docs/29` policy baseline, with the legal reasoning
  captured per source (e.g. Book of Shaders flagged INCOMPATIBLE
  despite same-author overlap with Lygia per `docs/56` §2.6;
  Hydra flagged INCOMPATIBLE under AGPL-3.0 per §2.8;
  Milkdrop/projectM deferred under LGPL static-link concerns per
  §2.9). The Synesthesia GLSL Resources page links to external
  material without license commentary — Chromaticity scores its
  shortlist against an actual SPDX policy. (b) **Vendored substrate
  ships in-tree.** `third_party/lygia/` is a flat-copy snapshot of
  Lygia v1.1.4 at commit `28f220b6045cbbd38ad6e23a69d21c5e67070cdd`
  with per-snapshot provenance (`third_party/lygia/UPSTREAM-VERSION.md`),
  the upstream LICENSE preserved, REUSE 3.2 annotations, and a
  trial-narrow `deny.toml` allow slot
  (`LicenseRef-Prosperity-3.0.0`) — readers can consume actual
  GLSL functions, not just external links. The `engine/render`
  preprocessor consumes Lygia includes via
  `engine/render/src/glsl_includes.rs` per
  `EV-LIBRARY-LICENSE-TRIAL` (`docs/26-evidence-artifact-registry.md:65`).
  (c) **Seed corpus shortlist** — `docs/56` §3 proposes a 33-shader
  V1 baseline (noise + post-processing + compositional + raymarch
  fundamentals) with per-shader license, GLSL→WGSL portability
  estimate, and required `chroma_*` uniform binding cost, all
  pre-cleared against the policy baseline. This is engineering-grade
  scope ready for the eventual ADR-011 render-pipeline lane.
- **Gap notes:** Lygia ships under
  `LicenseRef-Prosperity-3.0.0` (30-day commercial trial), so the
  long-term election is AO-gated and deferred to Phase-5
  `EV-LICENSE-AUDIT` per `docs/56` §2.1 + §7.1.1. Until that
  closes, the vendored substrate carries TRIAL-ONLY status and a
  fallback (clean-room helper shims per `docs/29` line 104, or
  the MIT-licensed glslify/webgl-noise/three.js alternatives per
  `docs/56` §3 Tier 1 + Tier 2) is pre-staged as a hedge. No
  user-facing "Open-source notices" surface exists yet (`docs/56`
  §4.3 attribution surfaces are designed but not wired — a Wave
  1d-α extension pending). No vendored shaders are currently
  exposed through Library / Author beyond the engine-side Lygia
  include path. Score moves to EXCEEDS+ when (a) the V1 seed
  corpus from `docs/56` §3 vendors, (b) the
  Settings → About → Open-source notices page wires, and (c)
  the Lygia license election resolves.

## Cross-cutting observations

- **Chromaticity leads on legal / clean-room hygiene.** The combined
  posture of `deny.toml` SPDX allowlist, REUSE 3.2 per-file
  SPDX headers (`docs/43-development-standards.md`), the
  per-third-party-vendor `LICENSES/` directory with verbatim
  upstream text, ADR-014 Sec.7 bundle-hygiene deny-list (no
  C/C++ headers, no debug symbols, no legacy DSA `.pem` files, no
  build configs), and the `docs/56` per-shader-source compatibility
  verdicts produce an audit-grade legal trail that the documented
  Synesthesia surface does not advertise. The reference bundle has
  been observed shipping `~20 MB` of OpenCV C++ headers, the full
  `lygia/` source tree including `webpack.config.js`, and a legacy
  Sparkle 1 DSA `.pem` that exposes a verification path
  Chromaticity does not use — all explicitly banned by
  `EV-BUNDLE-HYGIENE` (`ADR-014-signing-notarization-auto-update.md:80`).
- **Synesthesia leads on availability and ecosystem maturity.**
  Five years of shipped releases (~25 minor versions), an
  established community (Discord + Facebook group + per-product
  extension repos), and a curated external-resource index that has
  benefited from years of community input. Chromaticity's
  equivalents are designed and scaffolded (release pipeline,
  scene corpus, GLSL research catalog) but most have not yet
  shipped to users — `EV-RELEASE-PIPELINE` is `Planned` pending
  cert procurement, `EV-LIBRARY-CORPUS-MIN` is `InProgress`
  pending bundle-resource mapping widening, `EV-LICENSE-AUDIT`
  is `Planned` pending Phase-5. The next Phase boundary
  (`EV-RELEASE-PIPELINE` `Planned` → `Pass`) is the single
  largest score-mover for this group: it converts the Installers
  PARTIAL into MATCH or EXCEEDS and unlocks the changelog cadence
  + the corpus-distribution loop.
- **Asymmetric posture by design.** Synesthesia's Resources hub
  emphasises *distribution* (installers + changelogs + community
  channels + curated links); Chromaticity's equivalent surface
  emphasises *provenance* (per-PR commit hashes + per-source
  license verdicts + bundle-walk deny lists + EdDSA-only
  signature verification). Neither posture dominates — they
  weight different operational concerns. Chromaticity's
  intentional non-shipment of a Sparkle 1 DSA fallback
  (`ADR-014-signing-notarization-auto-update.md:34`) and the
  three-capability hardened-runtime entitlements file are
  examples where the project deliberately ships less surface
  than the reference because the omitted material is
  unnecessary attack surface.
