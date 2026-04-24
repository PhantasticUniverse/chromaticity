<!-- SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Phase-4d handoff checklist — first operator-facing observability consumer PR

**Purpose:** one-page template for the first future PR that introduces an operator-facing
consumer of the Phase-1 metric catalog (Prometheus loopback endpoint, query pack, scraper
config, dashboard provisioning bundle, alerting rules — any artifact that reads the
catalog labels). Cite this checklist in the PR description and self-attest each row.

**Anchor on:** `docs/31-supportability-privacy-ops-policy.md` §Phase-1 baseline catalog
(authoritative schema), `docs/adr/ADR-016-observability-and-telemetry-posture.md` (posture
and transport phasing), and the Rust catalog at `engine/observability/src/catalog.rs`
(the in-tree mirror that the conformance harness validates).

## Required rows

### 1. Dedicated `snap_*` labels (no folding into `control_set` / `control_changed`)

- [ ] Every PromQL expression / dashboard panel / alert rule that targets a runtime command
      or event uses the post-PR-#83 closed set verbatim. Command kinds: `control_set`,
      `preset_recall`, `playlist_advance`, `snap_recall`, `snap_save`, `snap_clear`,
      `mapping_bind`. Event kinds: `control_changed`, `preset_recalled`,
      `playlist_advanced`, `snap_recalled`, `snap_saved`, `snap_cleared`, `mapping_bound`.
- [ ] No expression aggregates Snap traffic under `control_*`. The PR-#83 split exists
      precisely so observability can distinguish control writes from slot recall/save/clear
      traffic.
- [ ] Source-of-truth: `docs/31` lines 73-78 (closed-set vocab block) and
      `engine/observability/src/catalog.rs` `EXPECTED_RUNTIME_COMMAND_KIND_LABELS` /
      `EXPECTED_RUNTIME_EVENT_KIND_LABELS` (lines 667-687).

### 2. Dual-query compatibility — only if a real scraper history exists at merge time

- [ ] **If** an operator instance has been scraping a pre-PR-#83 build whose runtime bus
      folded Snap traffic into `control_set` / `control_changed`, the consumer artifact
      MAY ship a transitional query that covers both label spellings for the deprecation
      window. State the operator instance + history window in the PR description.
- [ ] **Otherwise** (no real scraper history, e.g. first-ever consumer landing into a
      fresh Phase-4d activation): ship single-label queries against the dedicated `snap_*`
      labels. Do not invent dual-query coverage on speculation; it adds permanent label
      cardinality the catalog policy (`docs/31` §Label policy) tries to keep bounded.
- [ ] If the dual-query path is taken, declare the deprecation window in the operator
      migration note (row 5) and reference `docs/31` §Schema change process bullet
      "Renames" (one-minor-release deprecation window for any metric that has appeared
      in a shipped release).

### 3. Loopback-only bind per `EV-HEALTH-METRICS-LOOPBACK-ONLY`

- [ ] The Prometheus text-format endpoint binds to `127.0.0.1:<port>` only. No `0.0.0.0`,
      no link-local, no UNIX-domain-socket bypass that a remote scraper could reach.
- [ ] Port chosen per `docs/adr/ADR-013-networking-defaults-and-security.md` bind defaults.
- [ ] Endpoint activation is gated by `telemetry.enabled` Settings toggle (opt-in per
      ADR-016).
- [ ] Evidence row: `EV-HEALTH-METRICS-LOOPBACK-ONLY` (named by lead; the `docs/26`
      registry wording is Team C's lane and may not exist at the time of PR — flag the
      dependency in the PR description if the row is still pending).
- [ ] CI must include a network-trace test confirming no traffic outside loopback when
      `telemetry.enabled = true` and an operator-pointed scraper is hitting the endpoint.

### 4. Redaction posture per ADR-016

- [ ] No metric label or value carries scene names, node IDs, packet IDs, session IDs,
      file paths, IP addresses, hostnames, user-provided strings, or any other unbounded
      value. Per `docs/31` §Label policy: labels permitted only for bounded enumerable
      categories; cardinality budget ≤ 64 per metric.
- [ ] No data-source payload bytes, audio sample buffers, scene source, script source, or
      operator-entered text appear in any exposition output. Reference: ADR-016 line 60
      ("No data-source payload logging at infrastructure level") and `engine/observability/src/lib.rs`
      lines 30-37 (crate-level posture constraints).
- [ ] If the consumer artifact rewrites or transforms labels (e.g. recording rules,
      Grafana variables), confirm the transformed surface stays inside the same closed
      set. Recording rules that fan out a label into per-instance series re-introduce
      cardinality the catalog policy explicitly forbids.

### 5. Operator migration note

- [ ] PR description includes a "Semantic change summary" subsection that diffs the
      operator-visible surface against the prior shipped state. For the first operator-
      facing consumer this reads: "first activation of `EV-HEALTH-METRICS-LOOPBACK-ONLY`;
      Phase-1 catalog (14 metrics) becomes operator-readable for the first time."
- [ ] If row 2's dual-query path is in use, the migration note states the deprecation
      window for the legacy folded-label expression and the calendar date of removal.
- [ ] Migration note links to `docs/31-supportability-privacy-ops-policy.md` §Phase-1
      baseline catalog (schema), `docs/adr/ADR-016-observability-and-telemetry-posture.md`
      (posture), and this checklist file (template provenance).

## Scope reminders for the PR author

- The runtime command-kind and event-kind label sets are **closed for Phase 1**. Do not
  introduce new label tokens in the consumer artifact; if the catalog needs widening,
  raise a `docs(policy)` PR amending `docs/31` first per `docs/31` §Schema change process.
- Conformance is enforced compile-time + test-time by `engine/observability/src/catalog.rs`
  and `engine/runtime/src/command_bus.rs`. Re-run the conformance bundle (`cargo test -p
chromaticity-observability -p chromaticity-runtime --no-fail-fast`) on the consumer-PR
  branch and attach the output. The bundle template lives at
  `evidence-runs/snap-label-surface-validation/closed-set-conformance-bundle.md`.
- The `docs/26-evidence-artifact-registry.md` row binding for the consumer artifact is
  Team C's lane; do not edit `docs/26` from this consumer PR. Coordinate via the
  `policy`-scoped channel.
- `.github/workflows/policy.yml` and `CODEOWNERS` are serial-list files (`CLAUDE.md`
  §"Agent parallelism playbook"); changes there require a separate `policy`-scoped PR
  and cannot ride this consumer PR.

## How to use this checklist

Copy the row block into the consumer PR description and tick each box. Where a box does
not apply (e.g. row 2 when no scraper history exists), mark it explicitly with `n/a — <one-line reason>`
rather than leaving it unchecked, so the reviewer can confirm the row was considered.
