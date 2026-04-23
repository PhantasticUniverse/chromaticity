# Compatibility and Versioning Policy

## Purpose
Define explicit guarantees for scene compatibility and migration behavior.

Related state migration policy: `32-state-preset-playlist-migration-policy.md`.

## Compatibility Profiles
Compatibility profiles govern runtime behavior and shim activation. They are distinct from scene-format tags (see below).

- **Profile `compat-syn-v1`**
 - Accepts the reference-app schema variants and legacy keys observed in `02-synesthesia-contracts.md`.
 - Enables compatibility shims for legacy script/control semantics.
- **Profile `native-chroma-v1`**
 - Uses canonical Chromaticity schema and modern runtime semantics.

### Scene Format Tags (not profiles)
Scene format tags such as `shadertoy-v1-import` and `isf-import` referenced in `38-scene-library-and-import-spec.md` Sec.2.2 are catalog/metadata tags identifying the **source format** of an imported scene. They are not runtime compatibility profiles. On import, every scene is assigned a tag (for library display and provenance) AND a runtime compatibility profile (usually `compat-syn-v1` for Shadertoy imports; Chromaticity-authored scenes default to `native-chroma-v1`). Only two runtime compatibility profiles exist at MVP: `compat-syn-v1` and `native-chroma-v1`.

## Symbol Namespace Governance

- `syn_*` is reserved for compatibility continuity with the reference scene surface under `compat-syn-v1`.
- `native-chroma-v1` is native-first: new canonical symbols must use `chroma_*` (or existing non-prefixed legacy standards already locked by the catalog, e.g. `TIME`, `RENDERSIZE`, `_Meta` controls).
- A `syn_*` spelling may appear under `native-chroma-v1` only as a documented alias to a `chroma_*` canonical symbol when required for import continuity.
- New feature work must not introduce fresh `syn_*` canonical names.

### Namespace migration rule

- If a symbol exists only as `syn_*` today and is needed natively, add `chroma_*` as canonical in the symbol catalog, keep `syn_*` as compat alias, and document coercion/alias behavior in `36-reactive-symbol-catalog.md`.
- Any proposal to introduce a new `syn_*` symbol must include an explicit compatibility justification and Accountable Owner sign-off in the ADR/policy packet.

## Version Axes
- **Scene Schema Version**: version of `scene.json` contract.
- **Runtime Compatibility Version**: version of behavior shims and coercions.
- **Profile Version**: selected compatibility profile + revision.

## Guarantees
- Minor profile revisions shall remain backward compatible within major profile version.
- Any breaking compatibility change requires:
 - new major compatibility profile version,
 - migration guide,
 - warning period before default changes.

## Deprecation Policy
- Deprecated fields/behaviors must emit structured warnings.
- Deprecations require:
 - rationale,
 - affected scenes/features,
 - migration path,
 - removal eligibility criteria.

## Migration Requirements
- Provide deterministic normalization rules and migration logs.
- Store migrated output with explicit profile and schema version metadata.
- Preserve ability to run original artifact under prior compatible profile.
- Record coercion logs for each compatibility rewrite class (missing type, range inversion, legacy-key precedence, deprecated script globals).
- Coercion logs must include: source field(s), applied rewrite, deterministic rule ID, and warning code.
- Keep scene migration and state/preset/playlist migration policy versions synchronized at release boundaries.

## Validation Requirements
- Golden corpus definition, coverage, and provenance rules are in `34-golden-corpus-definition.md`.
- Golden corpus must include profile-pinned scenes for each active profile version.
- Any profile change requires:
 - regression diff report,
 - compatibility risk assessment,
 - sign-off by accountable owner with scene-system/runtime workcell review.

## Package Extensions and File-Type Ownership Policy

Chromaticity uses its own scene-package extension. Chromaticity does **not** claim, register, re-register, or declare a UTI for `.synScene` or any extension owned by the reference application. The reference's extension is imported via a one-way converter only.

### Chromaticity-owned extensions

| Extension | Form | Purpose | Chromaticity UTI (macOS) | Chromaticity ProgID (Windows) |
|---|---|---|---|---|
| `.chroma` | directory package | Native Chromaticity scene package. Contains `scene.json`, `main.glsl`, optional `script.js`, optional `images/`, optional `glsl/`, optional `data/` (V2+). | `com.chromaticity.scene` conforming to `com.apple.package` + `public.composite-content` | `Chromaticity.Scene.1` (HKCU-scoped) |
| `.chromaPack` | zip archive | Transport/archive form of a `.chroma` package. Single-file double-clickable bundle for distribution, email, and showfile-handoff. | `com.chromaticity.scene-pack` conforming to `public.zip-archive` | `Chromaticity.ScenePack.1` (HKCU-scoped) |
| `.chromaShow` | zip archive | Show bundle: one or more `.chroma` scenes plus playlists, preset banks, Snap slots, OSC/MIDI bindings, and operator settings. Reserved for the show-export feature planned in `32-state-preset-playlist-migration-policy.md`. | `com.chromaticity.show` conforming to `public.zip-archive` | `Chromaticity.Show.1` (HKCU-scoped) |

### Reference-app extension handling (import-only)

- Chromaticity's scene library and import pipeline (`38-scene-library-and-import-spec.md`) reads `.synScene` directory packages **as input** for conversion to `.chroma`. This is one-way conversion: the source `.synScene` directory is read non-destructively and a new `.chroma` package is written to the operator's chosen location. The original `.synScene` is never modified, renamed, or moved.
- Chromaticity **does not** declare any UTI with `tag-spec public.filename-extension = "synScene"`. Chromaticity **does not** declare any `Info.plist` `CFBundleDocumentTypes` entry that claims `.synScene`. Chromaticity **does not** write any Windows file-association that makes it a handler for `.synScene`.
- As a direct consequence: double-clicking a `.synScene` file on a Chromaticity-only install does nothing (the OS has no handler) unless the operator has manually chosen Chromaticity via "Open With -> Always". This is the correct trade: we would rather operators have an explicit configuration step than silently convert a reference-owned file.
- Reference-app user-data folder imports (`adr/ADR-015-reference-app-user-data-import.md`) operate on the reference's directory tree directly and do not rely on file-extension association at all.

### macOS UTI declarations (`Info.plist` `UTExportedTypeDeclarations`)
Chromaticity declares exactly three exported UTIs, as tabulated above. Chromaticity declares zero `UTImportedTypeDeclarations` that reference Synesthesia's UTIs; importing a `.synScene` is an in-app File -> Import operation, not a Finder association.

### Windows file-association policy
- Associations for `.chroma`, `.chromaPack`, and `.chromaShow` are written under HKCU (user-scoped) to avoid UAC friction. No HKLM override is attempted.
- On install, Chromaticity becomes the default handler for its own extensions only. No other extension's default handler is touched.

### Uninstall behavior
- Uninstalling Chromaticity removes only the three Chromaticity-owned UTIs (macOS) and the three Chromaticity ProgIDs (Windows). It does not touch any other application's UTI, ProgID, or file-association.

### Renaming on import
- When a `.synScene` package is imported, the resulting `.chroma` package takes the same base name by default (e.g. `boreal.synScene` -> `boreal.chroma`). The operator may override the name in the import dialog.
- The import writes a `source-provenance.json` sidecar recording the original `.synScene` path, import timestamp, and profile-tag per `38-scene-library-and-import-spec.md` Sec.2.2.

### Evidence
- `EV-EXTENSION-OWNERSHIP` - on a test VM: Chromaticity-owned extensions (`.chroma`, `.chromaPack`, `.chromaShow`) route to Chromaticity; `.synScene` files are not associated with Chromaticity by the OS unless the operator explicitly selects "Always Open With Chromaticity"; uninstalling Chromaticity leaves no orphan file associations.

Icon art (`chroma.icns`, `chromaPack.icns`, `chromaShow.icns`, Windows `.ico` equivalents) is clean-room-original per `29-cleanroom-legal-policy-baseline.md`. No reference icon is reused and no icon visually mimics the reference's `synScene.icns`.

## Data-Pinned Fixtures (reserved; activated post-MVP)
Scenes declaring `DATA_SOURCES` per `28-implementation-contract-pack.md` CP-12 consume inputs that are inherently non-deterministic (live weather, live satellite positions, live spectrum). `NFR-2 Determinism` requires deterministic replay of any fixture in the golden corpus. These two statements reconcile via **data-pinned fixtures**: a scene under validation pairs its `DATA_SOURCES` declarations with recorded cassette files that the runtime replays in place of live fetches during fixture runs.

- **Cassette**: a signed, time-indexed recording of every observable output of a data source over a bounded window. Stored alongside the fixture as a sidecar (`<fixture>.cassette.*` file). Concrete cassette format is specified by ADR-017; this policy only locks that cassettes exist and are the reproducibility mechanism.
- **Fixture profile**: every fixture that declares `DATA_SOURCES` carries a `data-profile` tag (`data-live`, `data-cassette`, `data-unavailable`). Only `data-cassette` and `data-unavailable` fixtures are eligible for `EV-RENDER-CONFORMANCE`. `data-live` fixtures run only under explicit "live drill" gates and never block release.
- **MVP behavior**: the runtime accepts `data-cassette` as a tag in the scene metadata and ignores it (no cassette replay wired). `EV-RENDER-CONFORMANCE` fixture selection at MVP therefore excludes any `DATA_SOURCES`-declaring scene unless it is tagged `data-unavailable` (which at MVP is the only real behavior).
- **Migration rule**: when ADR-017 ships with cassette replay, existing `data-cassette`-tagged fixtures must compile and run unchanged; the policy does not permit a breaking change to cassette semantics once ADR-017 is accepted.
- **Profile interaction**: data-pinned fixtures are orthogonal to `compat-syn-v1` / `native-chroma-v1`. A scene may run under either runtime profile with the same cassette; the profile governs coercion behavior, and the cassette governs input reproducibility.

This reservation prevents MVP from either (a) silently excluding data-ingesting scenes from the determinism contract, or (b) inventing a non-forward-compatible reproducibility mechanism.
