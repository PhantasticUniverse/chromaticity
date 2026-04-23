# Clean-Room and Legal Policy Baseline

## Purpose
Define legal controls that apply at implementation kickoff versus release/freeze, so gates are enforceable and non-contradictory.

## Gate Split

### Kickoff Policy Gate (Phase 0)
Required evidence: `EV-CLEANROOM-POLICY-BASELINE`

Required artifacts:
- Clean-room SOP.
- Provenance manifest template.
- Allowed-license policy baseline.
- Brand/trademark review checklist template.

Kickoff policy gate confirms controls exist and are approved; it does not require release-populated audit data.

### Release Audit Gate (Phase 5)
Required evidence:
- `EV-LEGAL-PROVENANCE`
- `EV-LICENSE-AUDIT`
- `EV-BRAND-REVIEW`

Release audit gate confirms shipped-scope compliance results and legal approvals are complete.

## Clean-Room SOP Minimum
- Reference product is used for behavior/spec extraction only.
- No direct copying of source, assets, or documentation text into shipped artifacts. **This includes embedded-documentation prose**: every page shipped under `Resources/docs/` is originally authored by Chromaticity per `adr/ADR-010-embedded-docs-generation.md`, with provenance tags (`author`, `reviewed_by`, `date`) attached to every page. Reference docs are consulted for coverage; paraphrase-only rewrites are disallowed.
- Any derived fixture from reference material must be tagged with origin class `derived-fixture` or `reference-adjacent`. See "Corpus Partitioning" below for where these live.
- Every shipped asset/code unit must map to a provenance record.
- Visual assets replacing reference assets (e.g. fallback thumbnails `thumbnail-default.png`, `thumbnail-fallback.png`, `thumbnail-shadertoy.png`, `thumbnail-isf.png`, scene/document icons) are original art commissioned or produced by Chromaticity with full provenance records.

## Corpus Partitioning (Internal Test vs Customer Sample)
Chromaticity maintains two disjoint scene/fixture corpora with different clean-room postures and different shipping rules. This distinction is normative and resolves the apparent conflict between "regression fixtures must test reference behavior" and "reference-adjacent artifacts must not ship."

### Internal Regression Corpus (CI-only, never shipped)
- Location: repository under `test/corpus/` (outside `release/`), `.gitignore`d from bundle-build targets.
- Populated from the fixture categories in `34-golden-corpus-definition.md` including the reference-adjacent categories (meta-pipeline fixtures, helper fixtures, transition-shape fixtures, pass-sizing sentinels, KM-01..KM-07 malformed fixtures).
- Clean-room posture: test inputs may be derived from observed reference behavior; fixture `scene.json`/`main.glsl` are authored clean-room by the Chromaticity team but are allowed to assert reference-equivalent numeric output as the pass/fail bar.
- Distribution: accessible to AI workcells and CI pipelines only. Never included in any release artifact (`.dmg`, `.msi`, installer, auto-update appcast).
- Provenance: every fixture directory has `PROVENANCE.md` with `origin=derived-fixture|reference-adjacent`, `release_inclusion=false` (enforced by bundle-build script).
- Build-pipeline check: `EV-LICENSE-AUDIT` scanner verifies no file with `release_inclusion=false` is present in any shipped artifact.

### Customer Sample Corpus (ships in release bundle)
- Location: `<AppBundle>/Resources/scenes/` and the Chromaticity scene library seed set.
- Exclusively original scenes authored clean-room by the Chromaticity team (origin class `original`).
- Mirrors the creative categories observed in the reference corpus (single-pass, multipass, script-heavy, media-dependent, raymarch, particle, etc.) but every file is a Chromaticity original with full provenance.
- Shadertoy-import golden fixtures (`EV-IMPORT-SHADERTOY`) are bundled as original Chromaticity-authored imports with permissive source licensing; reference-adjacent shadertoy fixtures live in the internal-only corpus.

### Rule of thumb
- If a fixture exists to assert "Chromaticity behaves like the reference behaves", it belongs in the internal regression corpus.
- If a scene exists to give customers a starting library or creative template, it belongs in the customer sample corpus.
- No fixture may live in both. Migration between corpora requires a provenance review and a new fixture authored from first principles, not a relabel.

## Provenance Manifest Template
Required fields:
- artifact identifier and location,
- author/workcell,
- origin classification (`original`, `third-party`, `derived-fixture`),
- license identifier (if applicable),
- reference to design/spec source,
- release inclusion flag.

## Allowed-License Policy Baseline
- Enumerate allowed, conditional, and blocked license classes.
- Any unknown license class is treated as blocked pending legal review.
- Conditional licenses require explicit approval record before release inclusion.

## Brand/Trademark Review Checklist
- **Name clearance (pre-MVP blocker)**: formal trademark clearance for "Chromaticity" (and the `.chroma` / `.chromaPack` / `.chromaShow` extension brand family) must complete before public beta announce. Scope covers (a) live-visuals / VJ / AV-performance software, (b) audio/music creative tools, (c) color-science and color-management software (where "chromaticity" is a term of art, raising confusion risk). Record outcome in `EV-BRAND-REVIEW`.
- **Domain clearance**: `chromaticity.app` (or equivalent owned domain) must resolve to Chromaticity-controlled infrastructure before any shipped `Info.plist` references it (Sparkle appcast, Sentry host, `OBSERVABILITY-INVENTORY.md` destinations).
- Product naming does not imply affiliation with Synesthesia, Gravity Current, or any other live-visuals product.
- The product name, UI copy, docs, icons, and launch surfaces are not confusingly similar to Synesthesia's in visual design, tone, or structure.
- Chromaticity does not claim any reference-app file-extension, UTI, ProgID, bundle identifier, or framework name (see `22-compatibility-versioning-policy.md` Sec.Package Extensions and `adr/ADR-014-signing-notarization-auto-update.md` Sec.7 bundle hygiene).
- Chromaticity's embedded-docs URL taxonomy is original and not a mirror of the reference's path structure (`adr/ADR-010-embedded-docs-generation.md` Sec.URL / slug convention).
- Chromaticity's scene vocabulary is allowed to include reference-parity terms (e.g. `syn_*` uniforms, `slider traveler` control type, `/favslots/{n}` OSC routes) **only** under `compat-syn-v1` for scene-author continuity; `native-chroma-v1` uses Chromaticity-first naming (`chroma_*` for new canonical symbol families) with `syn_*` retained only as documented compatibility aliases where required for import continuity. Known renames:
 - Reference **"favslot"** (per-scene preset slot) -> Chromaticity **"Snap"** / **"Snap slot"** in all UX copy, settings panels, tutorials, embedded docs, and marketing. The literal string `favslot` is retained only in the `compat-syn-v1` OSC route surface (`/favslots/{n}`) and in `adr/ADR-015-reference-app-user-data-import.md` import diagnostics so operator OSC bindings keep working unchanged across reference-to-Chromaticity migration. Specific UX renames are tracked per UX contract packet.
- **Comparative marketing claims** (e.g. "matches or exceeds Synesthesia") are evidence-backed (linked to specific `EV-*` runs) and legally reviewed before any customer-facing use; **internal planning language may cite the reference by name for clarity** (`02-synesthesia-contracts.md`, deep-dive docs 14-18).
- Crash/analytics/update endpoints are on Chromaticity-owned domains only. No reference-app domain appears in the bundle (`adr/ADR-016-observability-and-telemetry-posture.md`).
- Release sign-off requires the Brand/Trademark Review Checklist to be fully satisfied with explicit legal approval recorded in `EV-BRAND-REVIEW`.

## Bundled GLSL Library Handling
- Any bundled GLSL helper library shipped with Chromaticity must:
 - have an entry in the provenance manifest with origin classification and license identifier,
 - pass `EV-LICENSE-AUDIT` before inclusion in a release candidate,
 - not contain verbatim copies of reference product source where clean-room reconstruction is feasible.
- Reference-adjacent helper file names are reconstructed clean-room per `02-synesthesia-contracts.md` section 18.

### Candidate Bundled Libraries
The following libraries are candidate inclusions. Each requires completed provenance and license review before release.

| Library | Origin | License (observed) | Conditions for Inclusion | Release Gate |
|---|---|---|---|---|
| Lygia | Upstream: `github.com/patriciogonzalezvivo/lygia` (reference app ships v1.1.4 under `lib/lygia/`) | Check `LICENSE.md` at pinned version; Lygia upstream has used non-standard permissive/attribution-with-restrictions terms. Treat as **Conditional** pending legal review. | Pin a specific version; capture `LICENSE.md` verbatim into provenance record; confirm redistribution permitted for commercial desktop app; attribute in embedded docs and release notes. | `EV-LICENSE-AUDIT` |
| hg_sdf | Upstream: Mercury / `hg_sdf.glsl` | Dual MIT / CC-BY-NC-4.0 per the file header. **MIT is the required selection** for commercial distribution (CC-BY-NC would block commercial use). | Record MIT election in provenance; preserve original header; no modification that breaks license notice. | `EV-LICENSE-AUDIT` |
| Chromaticity helper shims (`_pulse`, `_fbm`, `_noise`, `_rotate`, `_loadMedia`, ...) | Original (clean-room) | Chromaticity proprietary | Must match signatures from `36-reactive-symbol-catalog.md` Sec.4; implementation by Chromaticity team with author credit in provenance. | `EV-LICENSE-AUDIT`, `EV-SYMBOL-CATALOG` |

If either Lygia or hg_sdf licensing resolves as blocking, Chromaticity ships only the clean-room helper shims and documents the delta in `22-compatibility-versioning-policy.md`.

## External Data Source Licensing Policy (reserved; activated post-MVP with ADR-017)
The real-time external-data differentiator (`41-data-ingestion-differentiator.md`) introduces a legal-compliance surface the GLSL-library policy above does not cover: **Chromaticity ships code that queries third-party data providers with heterogeneous licensing, commercial-use terms, attribution requirements, redistribution restrictions, and per-query costs.**

Every shipped data-source provider (post-MVP) must have a provenance entry extending the template in Sec.Provenance Manifest Template with these additional fields:

- **Provider**: canonical name + primary URL.
- **License class** (chosen from this enumeration; unknown class -> **blocked** pending legal review):
 - `public-domain` - e.g. NOAA GRIB2 grids, U.S.-government orbital elements. No attribution required; no commercial-use restriction; redistribution freely permitted.
 - `permissive-attribution` - e.g. OpenStreetMap (ODbL), Sentinel-2 imagery (CC-BY), Celestrak TLE (attribution request). Attribution required per provider terms; commercial use permitted; redistribution permitted subject to share-alike or attribution requirements.
 - `commercial-per-query` - provider requires a paid commercial API key per operator. Chromaticity may ship the **adapter** but **does not** ship a Chromaticity-issued key; operator supplies their own key.
 - `commercial-redistribution-restricted` - provider permits display but not redistribution of fetched bytes. Adapter must enforce: no cassette recording, no support-bundle inclusion, no screen-capture export of raw fetched data.
 - `restricted` - any other license class. Default: **blocked** until an explicit legal review places the provider into one of the above.
- **Attribution-surface obligation**: where in the app the provider must be credited (Settings -> About -> Data sources; in-app overlay for displayed tiles; release-notes line). Mandatory surface for `permissive-attribution` class.
- **Redistribution posture**: whether fetched payload bytes may be (a) cached on disk, (b) included in support bundles, (c) embedded in recorded cassettes, (d) exported via Show Bundle. `commercial-redistribution-restricted` providers default all four to **no**.
- **Per-query cost model**: free / metered / tiered / subscription. Chromaticity surfaces cost implications in the source's Settings UI when the operator opts in. Chromaticity does not silently consume paid quota.
- **Operator key storage**: if the adapter requires an operator-supplied API key, the key is stored in the OS secrets store (macOS Keychain / Windows Credential Manager), never in a support bundle, never in a crash payload (extends `EV-CRASH-PAYLOAD-REDACTION`). Re-emitted in network fetch headers only.
- **Offline posture**: adapters must tolerate provider unavailability without cascading runtime failure. `CR-07 data-source-unavailable` from `28-implementation-contract-pack.md` CP-01 is the behavioral contract.

**MVP impact**: zero. No data-source providers ship at MVP. This policy exists so that when ADR-017 lands, every provider onboarded has a required legal-gate that the policy baseline already covers. `EV-LICENSE-AUDIT` at the release gate will extend to cover every shipped data provider's provenance entry in addition to the library entries above.

## Governance
- Policy gate and release gate approvals are owned by the Accountable Owner.
- Legal gates are non-waivable per `23-decision-ownership-matrix.md`.
