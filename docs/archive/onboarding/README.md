<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Onboarding Seed

User-facing onboarding documentation for first-time scene authors. This
cluster is the Phase-0 seed authored against `EV-DOCS-PHASE0-SEED` (row 66
of `26-evidence-artifact-registry.md`).

The pages are intentionally short and concrete — they introduce the system,
the scene file format, the reactive-symbol set, and a hands-on first scene
without re-stating the normative specs. Each page links forward to the
authoritative document for readers who want depth.

## Reading order

1. **`01-intro.md`** — what Chromaticity is, who it is for, what you can
   do today, and what is deferred.
2. **`02-scene-format.md`** — the `.chroma` directory layout and
   `scene.json` manifest reference (slice-3a wire surface).
3. **`03-standard-uniforms.md`** — first-author rendering of
   `36-reactive-symbol-catalog.md` v1.1.0; the slice-3a representative
   subset and forward-pointers to the full catalog.
4. **`04-first-scene-tutorial.md`** — hands-on walkthrough that builds a
   single-pass audio-reactive scene end-to-end.

## Authority and provenance

These pages are tutorial-class documentation; the normative authorities
they summarise are:

- `02-synesthesia-contracts.md` and `05-scene-system.md` — scene model.
- `06-audio-engine.md` — audio analysis spec.
- `28-implementation-contract-pack.md` — CP-01 / CP-02a / CP-12 contracts.
- `36-reactive-symbol-catalog.md` — reactive-symbol catalog.
- `engine/scene/src/schema.rs` and `engine/scene/src/validator.rs` —
  in-source schema and validator.

Every page in this directory is a clean-room original authored by the
Chromaticity team per `29-cleanroom-legal-policy-baseline.md`. None of the
prose, examples, or fixture references are copied from reference-app
material; the example scene mirrors the in-tree
`engine/scene/tests/fixtures/scenes/reactive-audio.chroma` shape but is
independently written.

Pages inherit `CC-BY-4.0` via the `docs/**` path-rule in `REUSE.toml`;
each file additionally carries an inline SPDX header for clarity.
