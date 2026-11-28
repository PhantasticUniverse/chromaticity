<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-024: Native `chroma_*` Vocabulary; Reference-Format Names Only at the Import Boundary

- **Status:** Accepted
- **Date:** 2026-11-28
- **Deciders:** Accountable Owner (directed in session, 2026-11-28)
- **Consulted:** Engine (render, script, scene), Authoring, UI workcells
- **Informed:** Docs and help corpus maintainers

## Context and Problem Statement

The reactive-symbol catalog (`docs/architecture/36-reactive-symbol-catalog.md`)
adopted the reference scene format's `syn_*` spellings as Chromaticity's own
authoring vocabulary, with a few `chroma_*` twins (`chroma_PrimeBang`). The
policy in `docs/architecture/22-compatibility-versioning-policy.md`
§Symbol Namespace Governance allowed `syn_*` under `native-chroma-v1` as
"compat retentions". In practice the prefix had spread through every native
surface: the shader uniform block (`vec4 syn_BPM`), the script host globals,
the `ReactiveSymbol` catalog, the scene validator, bundled scenes, Author
autocomplete and the user documentation.

The Accountable Owner ruled that, unless it concerns importing, the `syn_`
prefix must not appear anywhere in Chromaticity. A product-owned vocabulary
also keeps the clean-room boundary in `29-cleanroom-legal-policy-baseline.md`
obvious: reference-format names live in exactly one translation layer.

## Decision Drivers

- One native vocabulary for authors, scripts, shaders, UI and docs.
- Reference-format names confined to code whose job is import.
- Already-imported packages (including user packages under `.local/`) keep
  working without re-import.
- No visual or behavioral change for bundled scenes.

## Considered Options

1. **Native-only `chroma_*`, translation table at the import boundary** —
   rename every native surface; `.synScene` names are translated by import
   adapters.
2. **Keep `syn_*` as accepted aliases in the native catalog** — native
   parsers accept both spellings indefinitely.
3. **Status quo** — keep `syn_*` as the documented native spelling.

## Decision

We adopt Option 1.

- The native vocabulary is `chroma_*` (plus the non-prefixed standard names
  already locked by the catalog: `TIME`, `RENDERSIZE`, `_Meta` controls,
  helper functions). Mapping is a prefix swap except
  `syn_Time → chroma_AudioTime` (energy-integrated clock; `chroma_Time` is
  wall-clock) and `syn_UserImage` / `syn_Media → chroma_Media`.
- The single translation table is `SYN_TO_NATIVE` in
  `engine/authoring/src/syn_import.rs`.
- Reference-format names may appear only in:
  `engine/authoring/src/syn_import.rs` (package conversion and the table),
  `app/shell/src/syn_compat.rs` (live shader ABI bridge and load-time
  `script.js` adaptation) and `engine/authoring/tests/**`.
- Native parsers (`ReactiveSymbol::from_name`, the scene validator) accept
  `chroma_*` only. The `SynPrimeBang` alias variant is removed.
- `scripts/syn-prefix-lint.sh` enforces the boundary over engine, app,
  integrations, frontend, bundled scenes, user docs and built help.

## Consequences

### Positive

- Authors see one vocabulary everywhere; autocomplete has no "compat" tier.
- The import boundary is auditable by path.
- Imported scripts and shaders keep working: both are adapted at load time,
  so packages imported before this change need no re-import.

### Negative

- Breaking for native `scene.json` files whose `UNIFORMS` bindings name a
  `syn_*` `SYMBOL`: those now fail validation and must use the `chroma_*`
  name (`Profile-Impact: native-chroma-v1-major`). No such scene has shipped;
  all bundled scenes and fixtures were migrated in the same change.
- Native shaders that still declare the uniform-block member as `syn_BPM`
  are unaffected (std140 members bind by position), but new docs and
  autocomplete only show `chroma_BPM`.

### Neutral / trade-offs

- The `compat-syn-v1` profile name and `EV-*` identifiers are unchanged; they
  name the import compatibility profile, not a native vocabulary.
- Design and research documents that describe the reference format keep its
  spellings; they are outside the lint scope.

## Alternatives Considered

- **Native aliases (Option 2)** — rejected: it keeps reference names as a
  first-class native surface, which is what the Accountable Owner ruled out,
  and leaves two spellings for every symbol in docs and autocomplete.
- **Status quo (Option 3)** — rejected for the same reason, and because the
  prefix had leaked into new work despite the existing governance rule.

## Evidence

- EV-SYMBOL-CATALOG — catalog canonical names become `chroma_*`; pinned by
  `engine/script/tests/reactive_symbol.rs`
  (`reference_format_names_are_not_native_symbols`).
- EV-SCENE-IMPORT — import translation pinned by
  `engine/authoring/src/syn_import.rs` tests and
  `app/shell/src/syn_compat.rs` (`syn_compat_pins`); fixture golden hashes
  in `engine/scene/tests/import.rs` re-pinned for the renamed `SYMBOL`s.
- EV-SCRIPT-CONTRACT — script globals renamed; imported scripts adapted at
  load time.
- EV-CLEANROOM-POLICY-BASELINE — reference-format names confined to the
  import layer.

## Traceability

- Planning: `docs/architecture/36-reactive-symbol-catalog.md`;
  `docs/architecture/22-compatibility-versioning-policy.md` §Symbol
  Namespace Governance (amended by this ADR).
- Clean-room: `docs/governance/29-cleanroom-legal-policy-baseline.md`.
- Supersedes: none (amends the namespace-governance policy text, not an ADR).

## Rollout and Review

- **Rollout:** code, bundled scenes and frontend in one change; catalog,
  user docs, built help and governance text in a follow-up docs change;
  `syn-prefix-lint` wired into lefthook and CI last.
- **Review cadence:** revisit if a new import format needs its own
  translation layer.
