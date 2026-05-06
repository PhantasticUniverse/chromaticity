<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# `EV-ANALYTICS-ZERO-DEFAULT` 14-day soak observation methodology

Phase-4d row 95 of `docs/26-evidence-artifact-registry.md` declares the
invariant:

> Validate fresh install with untouched settings produces zero packets to
> third-party analytics domains over 14-day observation window.

This document describes the methodology by which Chromaticity demonstrates
that invariant under the AO-approved Phase-4d closure path. The soak
opens the day the harness lands; the row's `Planned` → `Pass` transition
lands in a separate closing-docs PR after 14 calendar days of green
nightly runs (target ~14 calendar days after the harness merge commit).

## Two assertion modes

The harness ships two assertion modes that together cover the row's
invariant under workspace constraints (`unsafe_code = forbid`,
cross-platform macOS + Windows, no extra system dependencies in CI).

### 1. Static analysis (per-PR)

`engine/observability/src/analytics_audit.rs` exposes
`audit_source_tree()`, which walks the production source tree under
`engine/`, `app/`, `integrations/`, and `src/lib/` looking for any
known third-party analytics tracker hostname as a substring in any
`.rs`, `.ts`, `.svelte`, or `.svelte.ts` file. Findings are
line-anchored.

The corresponding test
`engine/observability/tests/analytics_zero_default.rs ::
static_analysis_finds_no_analytics_endpoints_in_production_source_tree`
asserts the report is empty against the live tree and runs as part of
`cargo test --workspace`, so every PR exercises the audit. A regression
introducing a tracker URL in a comment, a string literal, or a
configuration manifest fails the test.

Comments are NOT stripped before matching: an analytics URL in a
comment is still a leak signal (a developer who knows the hostname has
documented intent to talk to it).

The audit module's own source plus its sibling integration test live on
a small self-exclusion list (the blocklist text legitimately appears
there to compose the audit). Adding to the self-exclusion set requires
AO sign-off.

### 2. Runtime code-shape audit (per-PR + per-nightly)

The sibling test
`runtime_default_state_observes_no_egress_intent` enumerates every
subsystem that COULD make a network call at boot with fresh-install
defaults and asserts each is in its zero-egress configuration. The
enumeration is hand-maintained: a future subsystem with network
potential MUST be added to the test via a `policy`-scoped PR
alongside the relevant ADR / `docs/31` amendment.

This is a code-shape audit rather than a real socket sniff. Real-time
process-level egress observation in Rust at the workspace's portability
bar (`unsafe_code = forbid`, cross-platform, no extra system deps) does
not have a drop-in stable solution: `pcap` requires capabilities,
`tracing-subscriber` does not see syscalls, and `LD_PRELOAD` shim
crates are unsafe. The 14-day calendar gate plus the static-analysis
sweep plus the code-shape enumeration is the AO-approved Phase-4d
path; real socket sniffing during nightly soak is a Phase-5+
hardening enhancement.

Subsystems audited at this writing:

- `engine/observability` (Phase-1 in-process emitter only; Phase-4d
  Prometheus endpoint loopback-only opt-in via `EV-HEALTH-METRICS-LOOPBACK-ONLY`)
- `integrations/osc-adapter` (ADR-013 §1 inbound / outbound bind on
  `127.0.0.1`)
- `integrations/midi-adapter` (CoreMIDI / WinMM / ALSA only; no
  socket surface)
- `engine/audio` (local CoreAudio / WASAPI input; in-process
  `Arc<Mutex<AudioUniformSnapshot>>`)
- `engine/render` (GPU device only)
- `engine/runtime` (frame-loop scheduler; same-process Tauri IPC only)
- `engine/scene`, `engine/script`, `engine/entitlement` (pure logic
  crates with no I/O)
- `app/shell` (Tauri 2 binary; ADR-013 §2 — no outbound HTTPS at
  fresh-install defaults)

## Blocklist source and selection rationale

`known_analytics_hostnames()` returns 13 entries at this writing. The
selection draws on widely-deployed third-party analytics, product-
analytics, error-reporting, and session-replay SaaS products — the
classes of vendor a desktop application is most likely to phone home
to during integration. The list is NOT exhaustive (no static blocklist
can be); the 14-day calendar gate is the compensating control.

| Hostname               | Category               | Rationale                                                                |
| ---------------------- | ---------------------- | ------------------------------------------------------------------------ |
| `google-analytics.com` | Web analytics          | GA4 endpoint                                                             |
| `googletagmanager.com` | Tag manager            | GTM endpoint (companion to GA)                                           |
| `facebook.net`         | Ad / analytics pixel   | Meta Pixel (excludes `facebook.com` — too broad, catches social links)   |
| `segment.io`           | Customer-data platform | Segment unified analytics                                                |
| `mixpanel.com`         | Product analytics      | Mixpanel events                                                          |
| `amplitude.com`        | Product analytics      | Amplitude events                                                         |
| `posthog.com`          | Product analytics      | PostHog Cloud (open-source SaaS)                                         |
| `sentry.io`            | Error reporting        | Sentry SaaS (legitimate-feeling but still default-off per ADR-016)       |
| `hotjar.com`           | Session replay         | Hotjar pageview tracking                                                 |
| `fullstory.com`        | Session replay         | FullStory                                                                |
| `datadoghq.com`        | RUM / APM              | Datadog browser RUM                                                      |
| `newrelic.com`         | Browser monitoring     | New Relic browser agent                                                  |
| `matomo.cloud`         | Web analytics          | Matomo Cloud (excludes `matomo` alone — too generic for substring match) |

`facebook.com`, `google.com`, and `matomo` are intentionally excluded
to avoid false positives on legitimate references (social-link UA
strings, public-domain code-signing roots, the matomo-as-product-name
in package descriptions). The 14-day soak observation methodology
permits widening the blocklist mid-soak via a `policy`-scoped PR;
each widening resets the 14-day calendar window for that hostname
class only, not the row as a whole, per AO direction.

## How to run the harness locally

```bash
# Static analysis + runtime code-shape audit (both run together):
cargo test -p chromaticity-observability --test analytics_zero_default

# Static analysis T1 pins (planted-fixture coverage of the walker):
cargo test -p chromaticity-observability analytics_audit_pins

# Full observability test suite (catalog + emit + no-network-egress + analytics):
cargo test -p chromaticity-observability
```

A green run is a single Pass observation against the soak window. The
14-day calendar gate observes 14 consecutive nightly runs all green
(scheduled runs of the workspace `cargo test` invocation count
equally).

## Calendar framing

| Milestone                            | Date                 |
| ------------------------------------ | -------------------- |
| Harness merges to `main`             | 2026-05-06 (target)  |
| Soak observation window opens        | 2026-05-06           |
| First nightly run                    | 2026-05-06 / 05-07   |
| Soak observation window closes       | 2026-05-20 (~14 days)|
| Closing-docs PR transitions row 95   | 2026-05-20 (target)  |

The closing-docs PR cites the harness merge SHA, the 14 nightly run IDs,
the per-run `cargo test` lines for the two test functions, and the
final blocklist count, then transitions row 95 from `Planned` to `Pass`
with the AO-approved per-row anchor run id.

## Forward scope

A future Phase-5+ hardening enhancement may swap the runtime
code-shape audit for a real socket-level egress observation harness
(pcap-based or eBPF-based on Linux CI runners). That swap is not in
scope for the Phase-4d row 95 transition; the calendar gate plus the
two assertion modes shipped here are the AO-approved closure path.

## Governing documents

- `docs/26-evidence-artifact-registry.md` row 95 — this row.
- `docs/adr/ADR-013-network-defaults.md` — bind / connect defaults.
- `docs/adr/ADR-016-observability-and-telemetry-posture.md` — telemetry
  posture (no outbound HTTPS at any phase except the Phase-4d
  loopback-only health endpoint).
- `docs/31-supportability-privacy-ops-policy.md` — telemetry scope
  and metric catalog baseline.
- `docs/43-development-standards.md` §15 — evidence-tag and tier
  conventions enforced by `scripts/evidence-tag-lint.sh`.
