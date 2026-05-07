# Phase-4 EXIT Scoping Inventory

> **Status note (2026-05-07)**: this scoping doc was authored at the start of
> the Phase-4 EXIT planning wave. As of HEAD `d6b266d` (PR #384 merge), 17 of
> the 18 rows inventoried below have transitioned to `Pass` per
> `docs/26-evidence-artifact-registry.md`; only row 94
> `EV-ANALYTICS-ZERO-DEFAULT` remains `Planned` (calendar-gated ~2026-05-20).
> The §4 / §5 sequencing recommendations are preserved as historical record;
> the canonical EXIT readiness matrix lives at
> `docs/46-phase-4-exit-readiness.md`.

Sequencing aid for Phase-4 EXIT planning. Inventories every Planned evidence row owed by sub-phases 4b (Authoring + Library + Import), 4c (Embedded Documentation), and 4d (Hardening) so the Accountable Owner can sequence ADR-drafting waves before engineering waves and slot independent rows into parallel lanes where the dependency graph permits.

## 1. Scope and intent

This doc is a planning artifact, not a gate. It does not declare Phase-4 EXIT and does not pre-empt the Phase-4b / 4c / 4d sub-phase gate decisions, which remain AO-held per `11-roadmap-phases.md` §Gate Rule line 194 ("a phase is complete only when all required evidence IDs for that phase are `Pass` in `26-evidence-artifact-registry.md`"). The canonical evidence rows live in `26-evidence-artifact-registry.md`; this doc cross-references those rows and adds three columns the registry does not carry: dependency-chain notes, ADR coverage status, and rough wave-PR sizing for sequencing the engineering work.

The narrowing decisions, ADR drafts, evidence-tag wiring, and conformance corpora that turn each Planned row into a Pass row are out of scope here. Treat the per-row entries below as scoping seeds the AO can spend against — not as commitments to a specific implementation path.

## 2. Phase-4 EXIT framing

Per `11-roadmap-phases.md` lines 118-171, Phase 4 ("Integrations, Authoring, Docs, and Hardening") is split into four orthogonal sub-phases: 4a Integrations and Entitlement, 4b Authoring + Library + Import, 4c Embedded Documentation, 4d Hardening. The orthogonality is load-bearing — line 119 reads "a stall in one sub-phase does not block others from closing" — so the AO can sequence sub-phases in any order, run them in parallel, or interleave waves across them as throughput allows.

Phase-4 EXIT itself remains the umbrella achievement and is gated on all four sub-phases reaching Pass under §Gate Rule line 194. A sub-phase EXIT is independently declarable; cumulative Phase-4 EXIT is the AO's call once the four sub-phase declarations land.

## 3. Phase-4a status snapshot

Phase-4a EXIT was declared 2026-05-03 at commit `5acc452` per the Phase-4a EXIT closure sign-off in `EV-GATE-MODEL-AUDIT-checklist.md` (PR 336). All 11 Phase-4a evidence rows landed at `Pass` under collective AO-approved narrowings:

- Row 37 `EV-INTEGRATION-MATRIX` Pass with OSC routing-matrix + MIDI input arms; camera + Syphon + audio-uniform-IPC arms deferred to Wave 1c.
- Rows 73 / 74 / 75 hotkey trio (`EV-HOTKEY-PARITY-CONFORMANCE` + `EV-HOTKEY-COMPLETENESS` + `EV-HOTKEY-CONFLICT`) Pass at full scope.
- Row 80 `EV-MIDI-EXTENDED` Pass with MPE deferred to V2 + SysEx logged-only at MVP.
- Row 82 `EV-NET-HARDENING` Pass at full scope.
- Rows 87-89 + 91 entitlement bundle (`EV-ENTITLEMENT-ENFORCEMENT` + `EV-ENTITLEMENT-OFFLINE` + `EV-ENTITLEMENT-REVOCATION` + `EV-ENTITLEMENT-FREE-FEATURES`) Pass at MVP watermark + 13-cap honored set with IO_CAMERA intentionally excluded (free at MVP per ADR-009) and deny-list activation deferred to post-MVP.
- Row 95 `EV-HEALTH-METRICS-LOOPBACK-ONLY` Pass at full scope.

Wave 1c (camera + Syphon adapters + audio-uniform-IPC bridge) is explicitly deferred per AO direction; it is estimated at multi-week 5,000-10,000 LoC scope, requires platform integration decisions, and likely needs new ADRs. Wave 1c chronological ordering against Phase-4b is one of the AO decision points surfaced in §7 below.

## 4. Per-row inventory

Every row below is `Planned` in `26-evidence-artifact-registry.md` at HEAD. Row numbers are the line numbers in that registry. ADR coverage status is "no ADR dependency" when the evidence is purely conformance against a `docs/NN-*.md` spec pointer (no architecture decision is being made — the row simply asserts a measurable property holds). Wave-PR sizing is rough: 1 PR for small surfaces (corpus + tag wiring), 2-3 PRs for surface-extending rows that need new IPC commands or new producer crates.

### 4.1 Phase-4b: Authoring + Library + Import (11 rows)

#### Row 61 — `EV-AUTHORING-LIVE-RELOAD`

Hot-reload behavior, state preservation, latency budgets. Generation Source: authoring harness + timing capture (AI Scene Workcell + AI QA/Ops Workcell). ADR coverage: no ADR dependency (conformance against `37-authoring-environment-spec.md` §3.3 hot-reload semantics). Dependencies: requires `EV-AUTHORING-DIAGNOSTICS` error-mapping surface for the "post-reload error path produces inline diagnostics" arm. Wave-PR sizing: 2-3 PRs (file-watcher + reload pipeline + state-preservation contract; corpus separate).

#### Row 62 — `EV-AUTHORING-DIAGNOSTICS`

Inline + console diagnostic mapping for compile, script, JSON, and coercion errors. Generation Source: authoring diagnostic fixture suite. ADR coverage: no ADR dependency (conformance against `37-authoring-environment-spec.md` §3.4 diagnostic taxonomy). Dependencies: independent — sits at the root of the authoring dependency chain. `EV-AUTHORING-LIVE-RELOAD` and `EV-AUTHORING-UX-LATENCY` both depend on the diagnostic surface this row establishes. Wave-PR sizing: 2 PRs (diagnostic-renderer surface + multi-class fixture corpus).

#### Row 63 — `EV-AUTHORING-SCAFFOLDER`

New-Scene scaffolder output compiles and runs warning-free. Generation Source: scaffolder output suite (AI Scene Workcell). ADR coverage: no ADR dependency. Dependencies: independent — the scaffolder template is its own surface and does not depend on the diagnostic / hot-reload surfaces. Can run fully in parallel with rows 61 / 62. Wave-PR sizing: 1 PR (scaffolder template + golden-output corpus).

#### Row 83 — `EV-AUTHORING-UX-LATENCY`

Hover-docs p95 ≤ 80 ms; symbol-search p95 ≤ 150 ms; first-paint-on-open p95 ≤ 300 ms via the harness in `37-authoring-environment-spec.md` §3.3a. Generation Source: authoring latency harness. ADR coverage: no ADR dependency. Dependencies: requires the hover-docs and symbol-search surfaces that row 62 (`EV-AUTHORING-DIAGNOSTICS`) and the embedded-docs surface (row 84) co-land; runs after both. Wave-PR sizing: 1-2 PRs (instrumentation + measurement harness).

#### Row 64 — `EV-IMPORT-SHADERTOY`

Shadertoy → Chromaticity import converts, compiles, and renders. Generation Source: import harness + 3+ golden fixtures. ADR coverage: no ADR dependency (conformance against `38-scene-library-and-import-spec.md` §3 Shadertoy import semantics). Dependencies: independent of the authoring chain; depends on the scene-import infrastructure landed in Phase-1 `EV-SCENE-IMPORT`. Can run in parallel with row 65. Wave-PR sizing: 2 PRs (Shadertoy converter + golden-fixture corpus).

#### Row 65 — `EV-IMPORT-ISF-REJECT`

ISF input produces actionable reject diagnostic and escape hatch. Generation Source: import rejection suite. ADR coverage: no ADR dependency. Dependencies: independent of row 64; both can land in parallel. The reject path is a smaller surface than the Shadertoy converter — likely a single rejection rule plus a diagnostic surface plus a corpus of representative ISF inputs. Wave-PR sizing: 1 PR.

#### Row 85 — `EV-IMPORT-REFERENCE-SHOW`

Reference-app user-data import covering scenes, presets, playlists, Snap slots, MIDI maps, OSC mappings, hotkey remaps, settings snapshot — every artifact class declared in ADR-015. Generation Source: import dry-run + commit + rollback drill. ADR coverage: ADR-015 (`adr/ADR-015-reference-app-user-data-import.md`) — already exists; no new ADR needed. Dependencies: largest 4b row; depends on prior import infrastructure (rows 64 / 65) plus the Library indexing surface (row 66) plus per-artifact-class reference parsers. Wave-PR sizing: 3+ PRs (per-class importers, dry-run + rollback drill harness, golden corpus per artifact class).

#### Row 66 — `EV-LIBRARY-DISCOVERY`

Scene library scan, index, search, filter. Generation Source: library index suite. ADR coverage: no ADR dependency (conformance against `38-scene-library-and-import-spec.md` §2 library semantics). Dependencies: depends on row 101 (`EV-LIBRARY-BUNDLE-LAYOUT`) — the bundle layout taxonomy must exist before the indexer scopes its scan roots. Wave-PR sizing: 2 PRs (indexer + search/filter surface; corpus separate).

#### Row 67 — `EV-LIBRARY-THUMBNAIL`

Thumbnail fallback chain and deterministic regeneration. Generation Source: thumbnail pipeline suite. ADR coverage: no ADR dependency. Dependencies: depends on row 66 (`EV-LIBRARY-DISCOVERY`) — thumbnails attach to library entries the indexer surfaces. Wave-PR sizing: 1-2 PRs (renderer + cache + deterministic-regen corpus).

#### Row 101 — `EV-LIBRARY-BUNDLE-LAYOUT`

Bundled scene root layout `<AppBundle>/Resources/scenes/{welcome,library,examples,shadertoy-fixtures}/` per `38-scene-library-and-import-spec.md` §2.1. Generation Source: bundle-walk assertion. ADR coverage: no ADR dependency. Dependencies: foundational for Phase-4b — sits at the root of the library + import dependency chain. The W11 PR #120 partial advance (per the registry's Status column) landed bundled-resources plumbing in `app/shell/tauri.conf.json` + `app/shell/src/commands/library.rs`; the remaining work is the four-subdirectory taxonomy plus the bundle-walk assertion corpus. Wave-PR sizing: 1 PR (small completion of an already-partial row).

#### Row 76 — `EV-ACCESSIBILITY-AA`

WCAG 2.2 AA conformance on non-scene UI per NFR-11 (contrast, keyboard operability, focus indicators, target size, screen-reader labels, reduced-motion respect). Generation Source: axe-core audit + manual VoiceOver / NVDA review. ADR coverage: no ADR dependency. Dependencies: cross-cuts every other 4b surface — runs late after the authoring + library + import surfaces are stable enough that an axe-core sweep produces signal rather than noise. Wave-PR sizing: 2-3 PRs (axe-core CI integration, automated-failure remediation pass, manual-review remediation pass).

### 4.2 Phase-4c: Embedded Documentation (1 row)

#### Row 84 — `EV-DOCS-UX-LATENCY`

Embedded-docs hover/search responsiveness — hover-docs p95 ≤ 80 ms; full-text doc-search p95 ≤ 150 ms; doc-page first-paint p95 ≤ 300 ms. Same measurement harness as `EV-AUTHORING-UX-LATENCY` per `37-authoring-environment-spec.md` §3.3a. Generation Source: docs UX latency harness. ADR coverage: no ADR dependency (the embedded-docs ADR — `adr/ADR-010-embedded-docs-generation.md` — covers `EV-DOCS-CLEANROOM` and `EV-DOCS-VERSION-SYNC`, both of which carry separate Phase-4c gating; this row is the responsiveness conformance, not the generation contract). Dependencies: shares the latency-measurement instrumentation with row 83 (`EV-AUTHORING-UX-LATENCY`); the harness itself can be co-developed once and used by both rows. Wave-PR sizing: 1 PR (extend the row-83 harness over the docs surface).

Note: rows `EV-DOCS-CLEANROOM` and `EV-DOCS-VERSION-SYNC` are NOT in scope for this inventory — they were excluded from the brief's 18-row Planned set because they have separate Phase-4c trajectory (likely closer to Pass than the 18 rows below) and because `EV-DOCS-COVERAGE` is release-cumulative per §11 line 159. AO can sequence those alongside `EV-DOCS-UX-LATENCY` or independently.

### 4.3 Phase-4d: Hardening (6 rows)

#### Row 93 — `EV-CRASH-PAYLOAD-REDACTION`

Crash-report payloads exclude `main.glsl`, `script.js`, media buffers, and free-text fields per `adr/ADR-016-observability-and-telemetry-posture.md`. Generation Source: crash-injection + payload inspection. ADR coverage: ADR-016 — already exists; no new ADR needed. Dependencies: independent — the redaction filter is a contained surface. Sequence early in 4d as a security-baseline guardrail. Wave-PR sizing: 1 PR (redaction filter + payload-inspection corpus).

#### Row 94 — `EV-ANALYTICS-ZERO-DEFAULT`

Fresh install with untouched settings produces zero packets to third-party analytics domains over a 14-day observation window. Generation Source: network-observation soak. ADR coverage: no ADR dependency (conformance against ADR-013 + ADR-016 default-off posture). Dependencies: independent of the other 4d rows; the 14-day soak window is the calendar gate, not a technical dependency. Sequence early in 4d so the 14-day window starts as soon as possible. Wave-PR sizing: 1 PR (soak harness + zero-traffic assertion).

#### Row 79 — `EV-SHADER-CACHE`

Cold vs warm shader-compile budgets per `10-performance-latency-budgets.md`. Generation Source: shader compile benchmark pipeline. ADR coverage: no ADR dependency. Dependencies: render-side; independent of the crash-recovery + analytics rows. Sequence in parallel with rows 93 / 94. Wave-PR sizing: 1-2 PRs (cache instrumentation + cold/warm benchmark harness; nightly criterion bench likely the right home, mirroring the `EV-PERF-BUDGETS` Path B posture per `engine/runtime/benches/budgets.rs`).

#### Row 78 — `EV-GPU-SELECTION`

Deterministic multi-GPU adapter selection per `adr/ADR-011-rendering-backend.md` §7; operator-override persistence; battery / TDR-history warnings; hot-plug recovery. Generation Source: dual-GPU laptop VM test + workstation discrete-pair test. ADR coverage: ADR-011 — already exists; no new ADR needed. Dependencies: depends on render-backend stability landed in Phase-1 + Phase-2; depends on row 79 (`EV-SHADER-CACHE`) only inasmuch as both touch the render backend's adapter-selection layer. Wave-PR sizing: 2 PRs (selection-policy + operator-override surface; physical / VM dual-adapter validation corpus separate).

#### Row 80 — `EV-CRASH-RESUME`

Mid-show crash → continuation recovery with declared per-class state-restoration acceptance: time-to-resume p50 ≤ 5 s / p95 ≤ 10 s; visible-artifact window ≤ 2 s; state restoration covers seven classes (active scene + presets, Snap slot + scene-specific preset history, playlist position + iteration mode, OSC/MIDI session bindings, audio input device + `syn_Threshold`, operator meta-controls, render resolution + FPS target). Audio trailing-reactivity state resets on resume by design. Generation Source: crash-injection drill with controlled `SIGKILL` / `SIGSEGV` / panic-in-render-thread / OOM at three wallclock points within a 10-minute scripted show. ADR coverage: no ADR dependency (conformance against the seven-class acceptance carried in the registry row + crash-recovery semantics in `06-audio-engine.md` §7.6 for the audio-state reset). Dependencies: depends on Phase-3 `EV-STATE-REPLAY` + `EV-STATE-MIGRATION` + `EV-PRESET-PLAYLIST` + `EV-SHOW-BUNDLE-RECOVERY` foundations (all Pass at HEAD per the Phase-3 EXIT closure). Largest 4d row; sequence late. Wave-PR sizing: 3+ PRs (crash-detection + resume pipeline, per-class state-restoration adapters, multi-injection drill harness).

#### Row 40 — `EV-SOAK-CHAOS`

Session stability and recovery under soak + chaos. Generation Source: soak + chaos validation runs (≥ 4 hour soak achieved per `10-performance-latency-budgets.md` per the registry's evidence-format-by-class table line 164). ADR coverage: no ADR dependency. Dependencies: capstone for 4d — depends on every other 4d row being stable enough that a 4+ hour soak run produces signal rather than noise. Sequence last in 4d. Wave-PR sizing: 2-3 PRs (soak harness + chaos-injection + multi-profile validation corpus).

## 5. Per-sub-phase sequencing recommendation

These are dependency-respecting wave orderings, not commitments. Where two rows sit in the same wave, they can run as parallel sub-agent lanes per the `CLAUDE.md` agent-parallelism playbook (each lane on its own branch off `main`; rebase-on-merge for any choke-point conflicts). Within a sub-phase, rows that share a Generation Source (e.g. the row 83 / 84 latency harness) should ideally be co-owned to avoid duplicating the harness twice.

### 5.1 Phase-4b sequencing

1. **Wave 4b.1 — taxonomy foundation**: row 101 `EV-LIBRARY-BUNDLE-LAYOUT` (small completion of W11 PR #120 partial advance; scaffolds the four-subdir taxonomy that the indexer scans).
2. **Wave 4b.2 — library indexing**: row 66 `EV-LIBRARY-DISCOVERY` (uses the bundle layout from 4b.1).
3. **Wave 4b.3 — library completion + authoring foundation** (parallel-safe, two lanes):
   - Lane A: row 67 `EV-LIBRARY-THUMBNAIL` (uses the indexer surface from 4b.2).
   - Lane B: row 62 `EV-AUTHORING-DIAGNOSTICS` (independent of the library chain; foundational for the rest of authoring).
4. **Wave 4b.4 — authoring extension** (parallel-safe, two lanes):
   - Lane A: row 63 `EV-AUTHORING-SCAFFOLDER` (independent surface).
   - Lane B: row 61 `EV-AUTHORING-LIVE-RELOAD` (uses diagnostics from 4b.3 lane B).
5. **Wave 4b.5 — import surfaces** (parallel-safe, two lanes):
   - Lane A: row 64 `EV-IMPORT-SHADERTOY`.
   - Lane B: row 65 `EV-IMPORT-ISF-REJECT`.
6. **Wave 4b.6 — UX latency**: row 83 `EV-AUTHORING-UX-LATENCY` (depends on the hover-docs surface from 4b.3 lane B; ideally co-developed with row 84 `EV-DOCS-UX-LATENCY` from sub-phase 4c).
7. **Wave 4b.7 — reference-app import**: row 85 `EV-IMPORT-REFERENCE-SHOW` (depends on import infrastructure from 4b.5 + library indexing from 4b.2; large multi-PR wave).
8. **Wave 4b.8 — accessibility sweep**: row 76 `EV-ACCESSIBILITY-AA` (cross-cuts every 4b surface; runs last so the axe-core sweep covers all UI added by 4b.1-4b.7).

### 5.2 Phase-4c sequencing

Single-row sub-phase from the 18-row Planned set. Row 84 `EV-DOCS-UX-LATENCY` co-develops with row 83 `EV-AUTHORING-UX-LATENCY` (shared harness per `37-authoring-environment-spec.md` §3.3a) — the AO can either land both in a single cross-sub-phase wave or sequence them in adjacent waves. The other Phase-4c rows (`EV-DOCS-CLEANROOM`, `EV-DOCS-VERSION-SYNC`) are out of scope here and likely have shorter trajectories; the AO can sequence those independently.

### 5.3 Phase-4d sequencing

1. **Wave 4d.1 — security baseline** (parallel-safe, two lanes):
   - Lane A: row 93 `EV-CRASH-PAYLOAD-REDACTION` (small; ADR-016 already exists).
   - Lane B: row 94 `EV-ANALYTICS-ZERO-DEFAULT` (small; calendar-gated 14-day soak — start the window early).
2. **Wave 4d.2 — render-side hardening** (parallel-safe, two lanes):
   - Lane A: row 79 `EV-SHADER-CACHE` (independent; nightly criterion bench harness).
   - Lane B: row 78 `EV-GPU-SELECTION` (depends on render-backend stability; ADR-011 already exists).
3. **Wave 4d.3 — crash recovery**: row 80 `EV-CRASH-RESUME` (depends on Phase-3 state-replay + state-migration + preset-playlist + show-bundle-recovery foundations, all Pass at HEAD; large multi-PR wave).
4. **Wave 4d.4 — capstone soak**: row 40 `EV-SOAK-CHAOS` (runs after every other 4d row is stable).

## 6. Phase-4 EXIT achievement-prerequisite framing

Phase-4a EXIT was declared 2026-05-03 at commit `5acc452` per PR 336. Phase-4 EXIT umbrella declaration is gated on 4b + 4c + 4d each independently reaching Pass per §Gate Rule line 194. The 18 Planned rows above, plus the two out-of-scope 4c rows (`EV-DOCS-CLEANROOM`, `EV-DOCS-VERSION-SYNC`), plus Wave 1c carry-forward (camera + Syphon + audio-uniform-IPC narrowing on row 37 + IO_CAMERA tier-gating on row 86) constitute the remaining Phase-4 EXIT surface.

Wave 1c is technically a Phase-4a follow-on, not a Phase-4 EXIT prerequisite — Phase-4a EXIT is already declared. The AO may sequence Wave 1c against any of 4b / 4c / 4d, ahead of them, after them, or in parallel; the orthogonality clause at line 119 covers that flexibility.

## 7. AO decision points surfaced for sequencing

These are the open questions worth answering before the first Phase-4b wave kicks off. None are blockers for opening this inventory; each is a sequencing knob the AO controls.

- **Sub-phase parallelism posture**: run 4b / 4c / 4d serially (single sub-agent stream), in two parallel streams (e.g. 4b authoring lanes + 4d security lanes), or fully parallel (one lane per sub-phase plus shared infra)? The CODEOWNERS subtree boundaries support fully-parallel work without choke-point conflicts; the constraint is sub-agent throughput + review bandwidth, not the dependency graph.
- **Wave 1c chronological slot**: open Wave 1c (camera + Syphon adapters + audio-uniform-IPC bridge) before Phase-4b kicks off, in parallel with Phase-4b, or after Phase-4 EXIT? Wave 1c needs new ADRs and likely new integration crates; sequencing it before Phase-4b expands the active ADR queue but unblocks the audio-uniform-IPC surface that downstream UI work depends on.
- **Authoring-side vs library-side first within Phase-4b**: the library chain (rows 101 → 66 → 67) is shorter and shares no surface with the authoring chain (rows 62 → 61 / 63); either can lead. Preference signal: which sub-agent workcell has more bandwidth at kickoff.
- **Latency-harness ownership for row 83 + row 84**: co-develop the `37-authoring-environment-spec.md` §3.3a harness once and use it for both rows, or split the harness ownership. Co-development reduces total LoC and produces a single instrumentation contract; split ownership lets two lanes run in parallel.
- **`EV-CRASH-RESUME` state-restoration class scope**: the registry's row-80 acceptance carries seven classes (a-g). Land all seven in one wave, or sequence the harness around individual classes (e.g. ship class-a + class-b first as a partial-Pass advance, then widen)? The Phase-3 narrowing precedent (rows 33 / 34 / 35 / 36) supports partial-advance landings if the AO wants smaller PRs.
- **`EV-ANALYTICS-ZERO-DEFAULT` 14-day soak window kick-off**: the 14-day calendar gate is the longest single-row latency in 4d. Start the window as early as possible — likely as soon as the network-observation harness lands, ahead of any other 4d row reaching Pass.
- **Embedded-docs out-of-scope rows**: should `EV-DOCS-CLEANROOM` and `EV-DOCS-VERSION-SYNC` be folded into the same Phase-4c wave as `EV-DOCS-UX-LATENCY`, or sequenced independently? They have separate Generation Sources and likely separate workcell ownership, but co-landing them simplifies the Phase-4c EXIT declaration into a single AO sign-off rather than three.
