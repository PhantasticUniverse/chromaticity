<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity

PR template per docs/43-development-standards.md Sec.7.5.
Fill every section. "N/A" is acceptable for optional sections only when
explicitly stated so. Do not delete section headings.

The PR title becomes the squash-merge commit subject and MUST be a
Conventional Commit: <type>[!](<scope>): <subject>

The PR **body** (this document, once filled in) becomes the squash-merge
commit **body**. The repo's squash template is set to "Default to PR
title and description", so every main-side commit carries this body
verbatim. Consequences:
  1. The Trailers section at the bottom MUST be preserved and completed
     — `scripts/trailer-check.sh` validates the main-side commit and
     will fail CI if `Signed-off-by` or `Profile-Impact` is missing.
  2. Do NOT add content below the Trailers section. `git interpret-
     trailers` only recognizes a trailer block that is the LAST
     paragraph; any prose after the trailers terminates the trailer
     zone and re-breaks trailer-check.
  3. Markdown inside this body renders fine on GitHub but becomes
     plaintext in the main-side commit. Keep it readable either way.
-->

## Summary

<!-- One paragraph, plain prose. What changed, and why. -->

## Linked ADR

<!-- ADR-NNN or explicit "N/A - non-architectural". Architecture-
impacting PRs MUST link an Accepted ADR (or include the ADR in this PR). -->

ADR-

## Evidence impact

<!-- EV-* rows whose status may change as a result of this PR. Explicit
"none" is allowed; "unknown" is not. -->

- EV-<ID>: <advances to Pass | advances run ID | no change expected>

## Profile impact

<!-- Must equal the Profile-Impact trailer on the squash-merge commit. -->

- [ ] `none`
- [ ] `compat-syn-v1-minor`
- [ ] `compat-syn-v1-major`
- [ ] `native-chroma-v1-minor`
- [ ] `native-chroma-v1-major`

## Docs impact

<!-- Check the boxes that apply. -->

- [ ] No docs impact.
- [ ] Embedded-docs coverage changes (see docs/adr/ADR-010-embedded-docs-generation.md).
- [ ] Planning-corpus update (docs/NN-\*.md).
- [ ] ADR addition / amendment (docs/adr/ADR-NNN-\*.md).
- [ ] `CHANGELOG.md` entry required (per Sec.6).

Affected pages (if any): `/reference/…`, `/manual/…`, `/guides/…`, `/faq/…`, `/resources/…`

## Risk

<!-- Optional. R-* IDs from docs/13-risk-register.md this PR closes or
materially changes. -->

## Checklist

- [ ] PR title is a valid Conventional Commit (commitlint-ready).
- [ ] Every commit on this branch carries `Signed-off-by:` (DCO).
- [ ] The squash-merge commit will carry a valid `Profile-Impact:` trailer.
- [ ] Every new source file carries an SPDX header.
- [ ] `reuse lint` passes locally (or CI).
- [ ] Tests contributing to an evidence row carry the module-level prose
      tag `//! Evidence: EV-<ID> (T<n>) — explanation` per
      `docs/43-development-standards.md` §15.2 — the present-tense form
      that `scripts/evidence-tag-lint.sh` actually enforces (the
      `#[evidence(...)]` attribute macro is RESERVED in §15.2 but not yet
      implemented; the doc-comment shape is the only conforming declaration
      today). PRs outside an evidence-producing subtree (`engine/**`,
      `integrations/**`) may skip this item.
- [ ] Migration note is attached if this PR bumps a Sec.5 versioned axis
      in a breaking way.
- [ ] `adr-lint` passes if `docs/adr/` was touched.
- [ ] I have read `CONTRIBUTING.md` and confirm this PR complies with it.

<!-- Reviewer guidance (delete this comment block before requesting review if it
distracts; the body below is all that ends up on `main`):
- Non-architecture PR: one reviewer (per CODEOWNERS) is sufficient; auto-merge allowed.
- Architecture-impacting (`architecture` label): AO sign-off + Independent
  Architecture Reviewer sign-off with `[INDEPENDENT-ARCH-REVIEW]` in the
  review comment body. Auto-merge disabled.
- Do not approve a PR that fails required checks.
- Do not bypass clean-room, legal, or brand-review gates.
-->

---

## Trailers

<!--
These lines become git trailers on the main-side squash-merge commit.
Do not add any content below the last trailer. `scripts/trailer-check.sh`
enforces that every commit on main carries `Signed-off-by` and
`Profile-Impact`; ADR-impacting and evidence-impacting PRs additionally
carry `ADR` and `Evidence` trailers per CONTRIBUTING.md.

Replace the placeholder values. Leave the key name and colon intact.
-->

Signed-off-by: Your Name <you@example.com>
Profile-Impact: none
