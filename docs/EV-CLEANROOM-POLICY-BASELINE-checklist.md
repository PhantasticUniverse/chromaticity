<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# EV-CLEANROOM-POLICY-BASELINE — Phase 0 Exit Checklist

**Evidence ID:** `EV-CLEANROOM-POLICY-BASELINE`
**Gate role:** Phase 0 legal policy gate (`docs/26-evidence-artifact-registry.md` row
`EV-CLEANROOM-POLICY-BASELINE`).
**Purpose:** Validate the kickoff legal policy pack — the SOP, the corpus partitioning
rules, the provenance-manifest template, the allowed-license policy baseline, the
brand/trademark review checklist, the bundled GLSL-library handling, and the
external-data-source licensing policy — against `docs/29-cleanroom-legal-policy-baseline.md`.
**Checklist date:** 2026-04-29.
**Accountable Owner:** Xavier Butler (`@PhantasticUniverse`).

## Scope

This checklist instantiates the formal policy-pack completion sign-off the registry
row called for as the remaining `InProgress` work item. The clean-room and legal
policy baseline document `docs/29-cleanroom-legal-policy-baseline.md` is merged on
`main` and is the canonical artifact; this checklist records the AO's per-section
review and overall ratification of the policy pack.

`docs/29-cleanroom-legal-policy-baseline.md` was imported into `docs/` as part of the
single normative-corpus promotion in commit `deefc7a` (2026-04-23,
`docs(policy): promote normative planning corpus into docs/ + docs/adr/`). The
AO-local-partition-posture clarification followed in commit `df918ff` (2026-04-23,
`docs(policy): declare ao-local partition posture`). Together these are the
merge-evidence anchors for the per-section rows below.

### What this row is, and what it is not

This row is the **policy baseline gate**: it validates that the controls (SOP,
templates, checklists) exist, are written down, and are ratified by the AO. It is
the kickoff-policy-gate evidence per `docs/29` Sec.Gate Split.

This row is **not** the first per-shipping-artifact provenance-manifest instance.
Producing a provenance manifest for an actual shipped artifact is the work of the
release audit gate (Phase 5), and is tracked as separate evidence rows
`EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`, and `EV-BRAND-REVIEW`. Those rows depend
on this baseline existing; they do not gate this row's closure.

The registry row's earlier "Pending" prose conflated these two: "first
provenance-manifest instance for a shipping artifact" is Phase-2-or-later
deliverable work that flows into `EV-LEGAL-PROVENANCE`, not into this row.
`docs/29` Sec.Gate Split makes the split explicit: the kickoff policy gate
"confirms controls exist and are approved; it does not require release-populated
audit data."

## Per-section sign-off

- [x] **Clean-Room SOP Minimum (`docs/29` Sec.Clean-Room SOP Minimum).**
      Scope: reference product is used for behavior/spec extraction only; no
      direct copying of source/assets/documentation text into shipped artifacts;
      embedded-docs prose is originally authored per `docs/adr/ADR-010`; derived
      fixtures carry origin tagging (`derived-fixture`, `reference-adjacent`);
      every shipped asset/code unit maps to a provenance record; visual assets
      replacing reference assets are clean-room originals.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **Corpus Partitioning (`docs/29` Sec.Corpus Partitioning).**
      Scope: two-corpus model — internal regression corpus (`test/corpus/`,
      CI-only, never shipped) versus customer sample corpus
      (`<AppBundle>/Resources/scenes/`, exclusively original Chromaticity-authored
      scenes); two AO-local partitions outside version control (`chromaticity-planning/`
      and `Contents/`) per `docs/43-development-standards.md` §18.1; promotion
      paths and rule-of-thumb for fixture placement.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commits `deefc7a`, `df918ff`).

- [x] **Provenance Manifest Template (`docs/29` Sec.Provenance Manifest Template).**
      Scope: required fields — artifact identifier and location, author/workcell,
      origin classification (`original`, `third-party`, `derived-fixture`), license
      identifier (if applicable), reference to design/spec source, release-inclusion
      flag.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`). The template is the schema for every per-shipping-
      artifact instance produced under `EV-LEGAL-PROVENANCE` at the release audit
      gate.

- [x] **Allowed-License Policy Baseline (`docs/29` Sec.Allowed-License Policy Baseline).**
      Scope: enumerated allowed / conditional / blocked license classes; unknown
      classes treated as blocked pending legal review; conditional licenses
      require explicit approval record before release inclusion.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`). The license-class taxonomy is the gate input for
      `EV-LICENSE-AUDIT` at the release audit gate.

- [x] **Brand/Trademark Review Checklist (`docs/29` Sec.Brand/Trademark Review Checklist).**
      Scope: name clearance for "Chromaticity" + the `.chroma` / `.chromaPack` /
      `.chromaShow` extension brand family across live-visuals / VJ / AV-performance,
      audio/music creative, and color-science software classes; domain clearance for
      `chromaticity.app`; non-affiliation with Synesthesia, Gravity Current, and
      other live-visuals products; non-similarity in name, UI copy, docs, icons, and
      launch surfaces; non-claim of any reference-app file-extension, UTI, ProgID,
      bundle identifier, or framework name; original embedded-docs URL taxonomy;
      compat-syn-v1 vocabulary policy with native-chroma-v1 first-class naming
      (favslot → Snap rename); evidence-backed comparative marketing claims;
      Chromaticity-owned crash/analytics/update endpoints only.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`). Release sign-off depends on this checklist being
      fully satisfied with explicit legal approval recorded in `EV-BRAND-REVIEW`.

- [x] **Bundled GLSL Library Handling (`docs/29` Sec.Bundled GLSL Library Handling).**
      Scope: every bundled GLSL helper library carries a provenance-manifest entry
      with origin classification and license identifier; passes `EV-LICENSE-AUDIT`
      before inclusion in a release candidate; does not contain verbatim copies of
      reference product source where clean-room reconstruction is feasible;
      reference-adjacent helper file names are clean-room reconstructed.
      Candidate libraries enumerated: Lygia (Conditional pending legal review;
      AO-trial decision recorded 2026-04-28 deferring license election to Phase-5
      `EV-LICENSE-AUDIT`), hg_sdf (dual MIT / CC-BY-NC-4.0; MIT election required
      for commercial distribution), Chromaticity helper shims (clean-room, must
      match `docs/36` Sec.4 signatures).
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

- [x] **External Data Source Licensing Policy (`docs/29` Sec.External Data Source
      Licensing Policy).**
      Scope: reserved policy that activates post-MVP with `ADR-017`; required
      provenance-entry fields for every shipped data-source provider — provider
      identity, license class (from the closed-enum `public-domain` /
      `permissive-attribution` / `commercial-per-query` /
      `commercial-redistribution-restricted` / `restricted`), attribution-surface
      obligation, redistribution posture, per-query cost model, operator key
      storage (OS secrets store), offline posture (CR-07 contract per
      `docs/28-implementation-contract-pack.md` CP-01).
      MVP impact: zero (no data-source providers ship at MVP); the policy exists so
      that when ADR-017 lands, every onboarded provider has a required legal-gate
      that the policy baseline already covers.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29
      (merge commit `deefc7a`).

## Overall baseline sign-off

- [x] All seven sections of `docs/29-cleanroom-legal-policy-baseline.md` are
      complete, internally consistent, and AO-ratified. The kickoff legal policy
      gate has the controls it requires (SOP, corpus partitioning rules,
      provenance-manifest template, allowed-license policy baseline,
      brand/trademark review checklist, bundled-GLSL-library handling,
      external-data-source licensing policy). The
      `EV-CLEANROOM-POLICY-BASELINE` Phase 0 legal policy gate is closed.
      Per-shipping-artifact provenance-manifest instances and release-populated
      legal-audit results remain release-audit-gate work
      (`EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`, `EV-BRAND-REVIEW`) and do not
      gate this row.
      Signed by AO `@PhantasticUniverse` on 2026-04-29.
