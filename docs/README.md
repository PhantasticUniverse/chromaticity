<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Documentation Map

Chromaticity organizes all documentation under `docs/` by lifecycle and audience:

| Directory | Purpose |
| :--- | :--- |
| [`docs/user/`](user/) | In-app operator help corpus (MkDocs Material, Diátaxis 4-quadrant navigation: Tutorials, How-To Guides, Reference, Explanation). Built to `resources/help/` via `npm run build:help`. |
| [`docs/adr/`](adr/) | Architecture Decision Records (`ADR-001` through `ADR-023` + `TEMPLATE.md`, MADR 4.0.0). Enforced by `scripts/adr-lint.sh`. |
| [`docs/architecture/`](architecture/) | Active engineering contracts, runtime/audio/scene/I/O specifications, reactive symbol catalog (`36-`), hotkey map (`42-`), and design system (`design-system.md`, `45-`). |
| [`docs/governance/`](governance/) | Charter (`01-`), PRD (`03-`), standards (`43-`), clean-room legal baseline (`29-`), evidence registry (`26-`), checklists (`checklists/`), evidence runs (`evidence-runs/`), and pre-alpha smoke checklist. |
| [`docs/research/`](research/) | Non-normative competitive comparison (`comparison/`) and sourcing/research memos (`54-`, `56-`). |
| [`docs/archive/`](archive/) | Read-only historical snapshots (completed Phase 0–4 exit/scoping docs, superseded onboarding seed, legacy agent notes). |
