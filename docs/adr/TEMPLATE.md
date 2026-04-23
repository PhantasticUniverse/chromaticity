<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity

Canonical ADR template per docs/43-development-standards.md Sec.14.1.
Based on MADR 4.0.0 (https://adr.github.io/madr/).

Filename: ADR-NNN-kebab-slug.md (three-digit zero-padded; never reuse a number).
Remove these comments after instantiating.
-->

# ADR-NNN: <short title, imperative where useful>

- **Status:** Proposed <!-- | Accepted | Rejected | Deprecated | Superseded by ADR-NNN -->
- **Date:** YYYY-MM-DD
- **Deciders:** Accountable Owner, Deputy, Independent Architecture Reviewer
- **Consulted:** <workcell names / contracted specialists>
- **Informed:** <stakeholders receiving the decision without veto>

## Context and Problem Statement

What is the situation? What forces are at play? State the decision problem
in one or two paragraphs. Cite planning docs, FRs/NFRs, risk rows, and
prior ADRs with path + section references (e.g.
`docs/03-product-requirements.md` FR-17).

## Decision Drivers

- <driver: e.g. "deterministic render-thread latency ≤1 frame worst case">
- <driver: e.g. "avoid a third-party dependency on an unmaintained crate">

## Considered Options

1. **<Option 1 name>** — one-sentence summary.
2. **<Option 2 name>** — one-sentence summary.
3. **<Option 3 name>** — one-sentence summary.

## Decision

State the chosen option in one paragraph. Be explicit. No hedging.

> "We will adopt Option N."

## Consequences

### Positive

- <what becomes easier, safer, faster, cheaper>

### Negative

- <what becomes harder, more expensive, more constrained>
- <what new risks or migration burdens are introduced>

### Neutral / trade-offs

- <context-dependent outcomes>

## Alternatives Considered

For each non-chosen option, explain why it was rejected. Be specific;
"it was worse" is not a reason. Cite the decision driver(s) it failed.

- **<Option 1>** — rejected because ….
- **<Option 2>** — rejected because ….

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this
decision. Required; put `N/A` with justification if genuinely none.

- EV-<ID> — <how this ADR affects that evidence row>
- EV-<ID> — …

## Traceability

Back-links to the planning corpus, risks, FRs/NFRs, and superseded or
superseding ADRs.

- Planning: `docs/<NN-name>.md` §<section>
- Risk: `docs/13-risk-register.md` R-<NN>
- FR/NFR: `docs/03-product-requirements.md` FR-<NN> / NFR-<NN>
- Supersedes: ADR-<NNN> (update that ADR's Status to "Superseded by ADR-NNN")
- Superseded by: <fill when applicable>

## Rollout and Review

- **Rollout:** how the decision propagates into the codebase, including
  which work package or phase owns the implementation.
- **Review cadence:** when this ADR is revisited (e.g. "at Phase 4c
  exit") or what signal would trigger a re-evaluation.

---

> This ADR is binding once `Status: Accepted`. Modification requires a
> new ADR or a superseding ADR; do not edit the Decision section of an
> Accepted ADR in place. Editorial corrections are permitted in other
> sections and must land as `docs` commits with an accompanying
> `CHANGELOG.md` entry under `Compatibility` when they change behavior
> guidance.
