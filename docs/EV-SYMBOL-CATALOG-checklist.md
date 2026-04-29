<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# EV-SYMBOL-CATALOG — Phase 0 Exit Checklist

**Evidence ID:** `EV-SYMBOL-CATALOG`
**Gate role:** Phase 0 exit gate (`docs/26-evidence-artifact-registry.md` row
`EV-SYMBOL-CATALOG`).
**Purpose:** Validate ratification of the Reactive Symbol Catalog
(`docs/36-reactive-symbol-catalog.md`) at version `1.1.0`, satisfying the
change-control criteria in `docs/36` Sec.8.
**Checklist date:** 2026-04-29.
**Catalog version:** `1.1.0` (per `docs/36` Sec.8).
**Accountable Owner:** Xavier Butler (`@PhantasticUniverse`).

## Scope

This checklist instantiates the formal catalog sign-off packet the registry row
called for as the remaining `InProgress` work item. The Reactive Symbol Catalog
`docs/36-reactive-symbol-catalog.md` is merged on `main` at version `1.1.0`; this
artifact records the AO's per-section review and overall ratification of the
catalog as the canonical inventory of every scene-facing symbol Chromaticity
publishes to GLSL and the Script Host.

`docs/36-reactive-symbol-catalog.md` was imported into `docs/` as part of the
single normative-corpus promotion in commit `deefc7a` (2026-04-23,
`docs(policy): promote normative planning corpus into docs/ + docs/adr/`). That
commit is the merge-evidence anchor for every per-section row below.

### Owner Workcell — cross-workcell review posture

The registry row lists `AI Runtime Workcell + AI Scene Workcell` as the Owner
Workcell. Per the AO-only governance posture documented in CODEOWNERS, AO sign-off
serves as proxy for cross-workcell review at the kickoff/Phase-0 gate. The AO has
read every section of the catalog and confirms the cross-workcell coverage between
runtime symbol exposure (Sec.1, 2, 3, 5) and scene-import / coercion behavior
(Sec.4, 6, 7) is internally consistent.

### `docs/36` Sec.8 change-control requirements (this row's exit criteria)

`docs/36` Sec.8 specifies that any addition to the catalog requires:

- (a) a bumped catalog version,
- (b) a new `EV-*` fixture, and
- (c) an entry in `22-compatibility-versioning-policy.md` release notes.

For the `1.1.0` ratification covered by this checklist:

- (a) is satisfied: catalog version is `1.1.0`, recorded at the bottom of `docs/36`
      Sec.8 ("Catalog version `1.1.0`: added Sec.2.10 reference-surface uniforms ...
      and Sec.2.1 non-parity per-band time-series/spectrum spelling disposition").
- (b) is deferred: `EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, and
      `EV-AUDIO-TRACE` (the `EV-*` rows that consume catalog symbols per `docs/36`
      Sec.Purpose) remain `Planned`. Fixture coverage for the `1.1.0` additions is
      part of the Phase-2 evidence-row closure work; no fixtures are required at
      Phase-0 ratification.
- (c) is satisfied by this PR: a Catalog Ratification Log entry was appended to
      `docs/22-compatibility-versioning-policy.md` covering the `1.1.0` ratification
      at 2026-04-29.

## Per-section sign-off

- [x] **Sec.1 — Standard Uniforms (`docs/36` Sec.1).**
      Scope: frame/timing (`TIME`, `FRAMECOUNT`, `TIMEDELTA`, `PI`); pixel
      coordinates (`_xy`, `_uv`, `_uvc`); mouse (`_mouse`, `_click`, `_muv`,
      `_muvc`); multipass (`RENDERSIZE`, `PASSINDEX`, `syn_FinalPass`); media
      (`syn_Media`, `syn_UserImage`, `syn_MediaType`); reserved external-data
      ingestion namespace (`chroma_Data_*` with `syn_Data_*` compat aliases,
      activated post-MVP).
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.2 — Audio Uniforms (`docs/36` Sec.2).**
      Scope: data textures (`syn_Spectrum`, `syn_LevelTrail`); Level / Hits / Time /
      Presence / Beat / BPM / Large-features families with their family-tolerance
      IDs (`LVL`, `HIT`, `TIM`, `PRS`, `BEAT`, `BPM`, `LRG`); heuristic
      differentiator `chroma_PrimeBang` with `syn_PrimeBang` compat alias
      (Profile D); reference-surface uniforms not in the documented catalog
      (`syn_InputLevel`, `syn_RawLevel`, `syn_BassExpoCurve`, `syn_Threshold`,
      `syn_PrimeAndBang`) — Sec.2.10, the principal `1.1.0` addition; non-parity
      per-band spelling disposition in Sec.2.1.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.3 — Meta Controls (`docs/36` Sec.3).**
      Scope: color meta (`_Master`, `_Brightness`, `_Gamma`, `_Contrast`, `_Hue`,
      `_Saturation`, `_Invert`); media meta (`_Media_*`, `_Invert_Media`,
      `_Media_Scale`); geometry meta (`_Vert_Mirror`, `_Hor_Mirror`,
      `_Alpha_Channel`); dichrome meta (`_Limit_Colors`, `_Low_Color`,
      `_High_Color`); preference-backed meta (`syn_Prefs_ZoomOrFit`); differentiation
      meta (edition-gated `_Reactivity`, `_Audio_Speed`); meta-pipeline ordering
      (Sec.3.7).
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.4 — GLSL Helper Shim Catalog (`docs/36` Sec.4).**
      Scope: math, noise, coordinate, color, media, multipass, convenience-alias
      (`_textureImage`), meta, and deprecated-compatibility-alias helper
      signatures. All helpers are Chromaticity originals (clean-room) per
      `docs/29` Sec.Bundled GLSL Library Handling; signatures match reference so
      existing scenes compile unchanged.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.5 — Script Host Mirror Rules (`docs/36` Sec.5).**
      Scope: every Sec.1-3 symbol is exposed as a JS global with matching name
      except fragment-only and pass-only symbols and direct sampler values; vector
      values appear as JS objects; controls appear as globals named exactly as
      `NAME`; deprecated `inputs[name]` / `uniforms[name]` shims under
      `compat-syn-v1` only.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.6 — Coercion Classes (`docs/36` Sec.6).**
      Scope: case mismatch on control `NAME` (strict reject, compat warn-and-coerce);
      reading an undeclared audio uniform (strict compile error, compat compile to
      `0.0`); writing to a read-only uniform (strict runtime error, compat
      warn-and-ignore); calling a helper with wrong arity (strict compile error,
      compat best-effort overload); namespace coercion (`syn_*` accepted as compat
      aliases under `compat-syn-v1`; native templates emit `chroma_*`). Cross-
      reference to `docs/28-implementation-contract-pack.md` CR-01..CR-04.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.7 — Drift Tolerance Cross-Reference (`docs/36` Sec.7).**
      Scope: family-tolerance ID-to-row mapping into
      `docs/10-performance-latency-budgets.md` for `LVL`, `HIT`, `TIM`, `PRS`,
      `BEAT`, `BPM`, `LRG`; meta-control visual drift covered by the meta-pipeline
      fixture set in `docs/34-golden-corpus-definition.md`.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Sec.8 — Change Control (`docs/36` Sec.8).**
      Scope: catalog-version-bump + `EV-*` fixture + `docs/22` release-notes-entry
      requirement for any addition; major-profile-bump requirement for any removal;
      explicit-justification requirement for any new `syn_*` symbol; catalog
      version `1.0.0` original ratification target; catalog version `1.1.0`
      (Sec.2.10 reference-surface uniforms + Sec.2.1 non-parity per-band spelling
      disposition).
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`). For the `1.1.0` ratification covered by this
      checklist: (a) version-bump satisfied; (b) `EV-*` fixture deferred to
      Phase-2 evidence-row closure (`EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`,
      `EV-AUDIO-TRACE`); (c) release-notes entry appended to
      `docs/22-compatibility-versioning-policy.md` Catalog Ratification Log by
      this PR.

## Cross-workcell sign-off

- [x] **AI Runtime Workcell coverage.** Sections 1, 2, 3, and 5 lock the symbol
      surface the runtime publishes to GLSL and the Script Host (standard uniforms,
      audio uniforms, meta controls, script-host mirror rules). The catalog is the
      single source of truth for `EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, and
      `EV-AUDIO-TRACE` per Sec.Purpose. AO sign-off as proxy for AI Runtime
      Workcell review per repo governance.

- [x] **AI Scene Workcell coverage.** Sections 4, 6, and 7 lock the scene-import
      and coercion surface (helper shim signatures, coercion classes,
      drift-tolerance cross-reference). The catalog is the single source of truth
      for parity-loader behavior under both `compat-syn-v1` and
      `native-chroma-v1` profiles. AO sign-off as proxy for AI Scene Workcell
      review per repo governance.

## Overall baseline sign-off

- [x] All eight sections of `docs/36-reactive-symbol-catalog.md` at version `1.1.0`
      are AO-ratified. The Sec.8 change-control criteria for the `1.1.0`
      ratification are satisfied (version-bump recorded; release-notes entry
      appended to `docs/22`; `EV-*` fixture coverage scheduled into Phase-2
      evidence-row closure). The `EV-SYMBOL-CATALOG` Phase 0 exit gate is closed.
      Signed by AO `@PhantasticUniverse` on 2026-04-29.
