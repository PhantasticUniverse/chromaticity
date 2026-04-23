<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity

PR template per docs/43-development-standards.md Sec.7.5.
Fill every section. "N/A" is acceptable for optional sections only when
explicitly stated so. Do not delete section headings.

The PR title becomes the squash-merge commit subject and MUST be a
Conventional Commit: <type>[!](<scope>): <subject>
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
- [ ] Planning-corpus update (docs/NN-*.md).
- [ ] ADR addition / amendment (docs/adr/ADR-NNN-*.md).
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
- [ ] Tests contributing to an evidence row carry `#[evidence(...)]` tags
      (or the PR is outside an evidence-producing subtree).
- [ ] Migration note is attached if this PR bumps a Sec.5 versioned axis
      in a breaking way.
- [ ] `adr-lint` passes if `docs/adr/` was touched.
- [ ] I have read `CONTRIBUTING.md` and confirm this PR complies with it.

---

<!-- Reviewer guidance:
- Non-architecture PR: one reviewer (per CODEOWNERS) is sufficient; auto-merge allowed.
- Architecture-impacting (`architecture` label): AO sign-off + Independent
  Architecture Reviewer sign-off with `[INDEPENDENT-ARCH-REVIEW]` in the
  review comment body. Auto-merge disabled.
- Do not approve a PR that fails required checks.
- Do not bypass clean-room, legal, or brand-review gates.
-->
