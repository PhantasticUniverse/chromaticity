<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

# Phase-4 EXIT Readiness Matrix

**Generated:** 2026-05-07. Anchor: `main` HEAD `bcc4b5b` (after PR #380 merge).

This document is the AO-facing EXIT-declaration packet. Every Phase-4
EXIT row's current state, narrowing axes, and remaining gates are
recorded below so AO can declare Phase-4 EXIT in one pass once the
sole remaining calendar gate (row 94 `EV-ANALYTICS-ZERO-DEFAULT`)
clears at ~2026-05-20.

This file is a condensation of facts already in
`docs/26-evidence-artifact-registry.md` and
`docs/EV-GATE-MODEL-AUDIT-checklist.md`; no new claims are introduced.

## Phase-4a — Integrations + Entitlement (11 rows)

**Status:** Phase-4a EXIT achieved 2026-05-03 at commit `<merge-sha>`
per the Phase-4a EXIT closure section in
`docs/EV-GATE-MODEL-AUDIT-checklist.md`. All 11 rows at `Pass` under
collective narrowings.

| Row | Evidence ID                   | Status | Narrowing                                                                                                                          |
| --- | ----------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| 37  | EV-INTEGRATION-MATRIX         | Pass   | OSC routing-matrix conformance plus MIDI input matrix arm; camera / Syphon / audio-uniform-IPC arms deferred to Wave 1c-β          |
| 73  | EV-HOTKEY-PARITY-CONFORMANCE  | Pass   | full scope                                                                                                                         |
| 74  | EV-HOTKEY-COMPLETENESS        | Pass   | full scope                                                                                                                         |
| 75  | EV-HOTKEY-CONFLICT            | Pass   | full scope                                                                                                                         |
| 80  | EV-MIDI-EXTENDED              | Pass   | MPE V2 deferred (diagnostic-fires-once rejection at MVP); SysEx logged-only at MVP                                                 |
| 82  | EV-NET-HARDENING              | Pass   | full scope                                                                                                                         |
| 87  | EV-ENTITLEMENT-ENFORCEMENT    | Pass   | 13-cap honored set; IO_CAMERA intentionally excluded (free at MVP per ADR-009)                                                     |
| 88  | EV-ENTITLEMENT-OFFLINE        | Pass   | full scope                                                                                                                         |
| 89  | EV-ENTITLEMENT-REVOCATION     | Pass   | expiry-based revocation only; deny-list activation deferred to post-MVP                                                            |
| 91  | EV-ENTITLEMENT-FREE-FEATURES  | Pass   | full scope                                                                                                                         |
| 95  | EV-HEALTH-METRICS-LOOPBACK-ONLY | Pass | full scope per `docs/31` Phase-1 baseline catalog                                                                                  |

Wave 1c-α-2 production audio bootstrap (2026-05-06) further narrowed
the `EV-AUDIO-UNIFORM-IPC` cross-tag for the audio-uniform-IPC arm of
row 37; live IPC dispatch in production. See the Phase-4a Wave 1c-α-2
closure section in `docs/EV-GATE-MODEL-AUDIT-checklist.md`.

## Phase-4b — Authoring + Library + Import (11 rows)

**Status:** 10 of 11 rows at `Pass` as of 2026-05-07 under collective
narrowings. 1 row remains `Planned` pending follow-on engineering PR.

| Row | Evidence ID                | Status   | Narrowing                                                                                                                         |
| --- | -------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 60  | EV-AUTHORING-LIVE-RELOAD   | Pass     | state-preservation cross-seam contract only; runtime-side wiring deferred to future composition wave; latency-budget arm cross-tagged with row 83 `EV-AUTHORING-UX-LATENCY` |
| 62  | EV-AUTHORING-DIAGNOSTICS   | Pass     | spans for JSON class only; compile / script / coercion classes emit message-only diagnostics                                      |
| 63  | EV-AUTHORING-SCAFFOLDER    | Pass     | full scope                                                                                                                        |
| 81  | EV-AUTHORING-UX-LATENCY    | Pass     | synthetic harness only; full UI-driven validation deferred post-MVP; cross-tagged with `EV-DOCS-UX-LATENCY`                       |
| 64  | EV-IMPORT-SHADERTOY        | Planned  | PR `#360` PR 1 of 2 landed (3 starter golden fixtures); PR 2 broader corpus + edge-case sweep pending                             |
| 65  | EV-IMPORT-ISF-REJECT       | Pass     | full scope; ISF native import deferred to V2 per design (the reject-with-escape-hatch path IS the MVP scope)                      |
| 66  | EV-LIBRARY-DISCOVERY       | Pass     | full scope                                                                                                                        |
| 67  | EV-LIBRARY-THUMBNAIL       | Pass     | stub byte-hash renderer; real GPU-backed thumbnail rendering deferred post-MVP                                                    |
| 78  | EV-LIBRARY-BUNDLE-LAYOUT   | Pass     | full scope                                                                                                                        |
| 76  | EV-ACCESSIBILITY-AA        | Pass     | axe-core automated audit only; 10 page-scope rules disabled at component scope; manual VoiceOver / NVDA review AO-led; full-app integration audit deferred post-MVP |
| 86  | EV-IMPORT-REFERENCE-SHOW   | Pass     | synthetic-commit + synthetic-rollback + heuristic + mocked-empty-library narrowings (full reference-app user-data store mutation deferred post-MVP V1)              |

Phase-4b EXIT remains AO-held per `docs/11-roadmap-phases.md`. Per the
Phase-4b Wave 4b.8 closure section in
`docs/EV-GATE-MODEL-AUDIT-checklist.md`, "all 11 Phase-4b EXIT rows are
at `Pass` under collective narrowings" was the closure language for
the last directly-engineered Phase-4b EXIT prerequisite. Row 60
`EV-AUTHORING-LIVE-RELOAD` transitioned to `Pass` 2026-05-07 (Wave
4b.2 PR 2 of 2) under the state-preservation cross-seam-contract
narrowing per the Phase-4b Row 61 closure section in
`docs/EV-GATE-MODEL-AUDIT-checklist.md`. Row 64 carries forward as
Planned pending its broader-corpus follow-on PR and is recorded as a
Phase-4b EXIT prerequisite with PR-1-of-N shipped per `docs/26` row
description column.

## Phase-4c — Embedded Documentation (1 row)

**Status:** at `Pass` — co-developed with row 81
`EV-AUTHORING-UX-LATENCY` via PR `#363`.

| Row | Evidence ID         | Status | Narrowing                                                                                            |
| --- | ------------------- | ------ | ---------------------------------------------------------------------------------------------------- |
| 84  | EV-DOCS-UX-LATENCY  | Pass   | synthetic harness; full UI-driven validation deferred post-MVP; cross-tagged with `EV-AUTHORING-UX-LATENCY` |

The wider Phase-4c gate set (`EV-DOCS-CLEANROOM`, `EV-DOCS-VERSION-SYNC`)
carries forward separately; this row is the only Phase-4c EXIT row
that participates in the EXIT-readiness narrowing posture under
`docs/EV-GATE-MODEL-AUDIT-checklist.md` Phase-4b cluster closure.

## Phase-4d — Hardening (6 rows)

**Status:** 5 of 6 rows at `Pass` as of 2026-05-07 under narrowings.
**Sole remaining gate: row 94 calendar (~2026-05-20).**

| Row | Evidence ID                | Status              | Narrowing                                                                                                                                | Remaining Gate                                |
| --- | -------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| 40  | EV-SOAK-CHAOS              | Pass                | CI-short multi-profile synthetic harness; nightly ≥4-hour soak deferred to calendar-gated criterion-bench-style harness; live-runtime soak future-wave | none                                          |
| 78  | EV-GPU-SELECTION           | Pass                | synthetic dual-adapter T2 corpus exercising `rank_synthetic` decision tree; physical / VM dual-GPU validation + TDR-warning UI + battery OS-watcher deferred | none                                          |
| 79  | EV-SHADER-CACHE            | Pass                | observation-only — no `scripts/budget-check.sh` extension; CI-enforced numeric threshold deferred to post-MVP (gated on warm-cache key derivation stability) | none                                          |
| 80  | EV-CRASH-RESUME            | Pass                | synthetic restore-helper validation across all 7 classes (a/b/c/d/e/f/g); full crash-injection drill (real `SIGKILL` / `SIGSEGV` / panic-in-render-thread / OOM) requires real platform crash-handler infrastructure deferred to future-wave | none                                          |
| 93  | EV-CRASH-PAYLOAD-REDACTION | Pass                | synthetic crash-payload constructor validation across 4 simulated crash classes covering all 5 `RedactCategory` arms; real platform-specific crash-handler injection + live Sentry transport deferred | none                                          |
| 94  | EV-ANALYTICS-ZERO-DEFAULT  | Planned (calendar)  | Wave 4d.1 PR `#347` scaffolding landed (default-off telemetry config + opt-in surface + synthetic T2 fixture corpus); 14-day analytics-zero soak fires from a fresh-install timestamp | calendar — closing-docs ~2026-05-20           |

## Phase-4 EXIT decision

**AO-declarable** once row 94 closing-docs lands (~2026-05-20). At
that point all Phase-4 EXIT prerequisites that have been transitioned
to `Pass` will be at `Pass` under the collective narrowings recorded
in their respective `docs/26` description columns; the Phase-4b
prerequisite with PR-1-of-N work shipped (row 64) carries forward at
the AO's discretion with the per-row PR-shipped record in `docs/26`.
Phase-4 EXIT itself remains AO-held per `docs/11-roadmap-phases.md`.

## Out-of-scope items carried forward

- **Phase-4a Wave 1c-β** (camera + Syphon adapters) — explicitly
  AO-deferred per ADR-019 and the cumulative narrowings on row 37;
  estimated 5,000-10,000 LoC multi-week scope; requires platform
  integration decisions + likely new ADRs. Unblocked from EXIT.
- **Production-bootstrap stale-but-last-good IPC surfacing through
  the wire** — narrowing axis carried forward on row 83
  `EV-AUDIO-UNIFORM-IPC`; producer-side covered at T1, transitive
  IPC-seam coverage future-wave option. Unblocked from EXIT.
- **Spectrum widget retrofit + AudioReactivity 48-bar FFT live
  visualiser** — post-MVP UX enhancements carried forward as
  narrowing axes on row 83. The row's `Pass` posture is unaffected.
- **Phase-4c remainder** (`EV-DOCS-CLEANROOM` on shipped-to-date
  pages, `EV-DOCS-VERSION-SYNC`) — separate AO-held gate per
  `docs/11`; out of scope for the Phase-4 EXIT readiness reading
  recorded in this document.
- **Repo-chore backlog** — backlogged per AO direction. Unblocked
  from EXIT.
- **Phase-4b PR-1-shipped follow-ons** — `EV-IMPORT-SHADERTOY` PR 2
  (broader corpus + edge-case fixture sweep) carries forward as
  Planned per its `docs/26` row description column.
  `EV-AUTHORING-LIVE-RELOAD` transitioned to `Pass` 2026-05-07 via
  Wave 4b.2 PR 2 of 2 under the state-preservation cross-seam-contract
  narrowing; runtime-side wiring deferred to a future composition wave.

## Anchor commit table for the AO declaration

| Wave                                           | Closure date    | Anchor commit       |
| ---------------------------------------------- | --------------- | ------------------- |
| Phase-4a EXIT closure                          | 2026-05-03      | post-`#334` merge (closure section in `docs/EV-GATE-MODEL-AUDIT-checklist.md`) |
| Phase-4a Wave 1c-α                             | 2026-05-03      | post-`#340` merge   |
| Phase-4a Wave 1c-α-2 (audio production bootstrap) | 2026-05-06   | `9a1131e` (PR `#344`) |
| Phase-4b cluster (Wave 4b.1 — 4b.6) + 4d.1/4d.2/4d.3 partial | 2026-05-06 | post-`#363` merge   |
| Phase-4b Wave 4b.7 (`EV-IMPORT-REFERENCE-SHOW`) | 2026-05-07     | `87b1a62` (PR `#370`) |
| Phase-4b Wave 4b.8 (`EV-ACCESSIBILITY-AA`)     | 2026-05-07      | `e751752` (PR `#373`) |
| Phase-4d Wave 4d.4 (`EV-SOAK-CHAOS`)           | 2026-05-07      | `ca7ddd8` (PR `#376`) |
| Phase-4d Row 93 (`EV-CRASH-PAYLOAD-REDACTION`) | 2026-05-07      | `c57e09b` (PR `#377`) |
| Phase-4d Row 80 (`EV-CRASH-RESUME`)            | 2026-05-07      | `23b45ba` (PR `#379`) |
| Phase-4d Row 78 (`EV-GPU-SELECTION`)           | 2026-05-07      | `bcc4b5b` (PR `#380`) |
| Phase-4b Row 61 (`EV-AUTHORING-LIVE-RELOAD`)   | 2026-05-07      | `<TBD-on-merge>` (PR `<TBD>`) |
| Phase-4d Row 94 (`EV-ANALYTICS-ZERO-DEFAULT`)  | TBD ~2026-05-20 | TBD (calendar)      |

---

_Document is read-once at AO declaration. Once Phase-4 EXIT is declared,
this file becomes a historical artifact and may be archived alongside
the closing entries in `docs/EV-GATE-MODEL-AUDIT-checklist.md`._
