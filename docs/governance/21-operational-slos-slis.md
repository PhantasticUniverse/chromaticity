# Operational SLOs and SLIs

This document formalizes live-operation service levels for Chromaticity. Every SLO row has a named owner, a measurement method with explicit population and window, targets at the percentiles that matter for live performance, and a link to the evidence artifact that validates it.

## SLI Measurement Conventions

- **Window**: rolling 14 days of release-channel telemetry unless a specific SLO overrides. Pre-GA windows use the golden-corpus regression runs in `34-golden-corpus-definition.md`.
- **Population**: defined per SLO below.
- **Instrumentation source**: `CAMetalLayer` nextDrawable presented-ack timestamp (macOS) and `IDXGISwapChain::Present` completion timestamp (Windows) for frame boundaries; `CADisplayLink` / DXGI `WaitForVBlank` callbacks for wall-clock vsync alignment; OS high-resolution timers for latency paths. Instrumentation overhead must not inflate the measured percentile by more than 1 ms at p95.
- **Cross-SLO rule**: when a release candidate is evaluated, all SLOs are measured against the same build, same fixture set, and same hardware class as declared in the evidence row.

## SLO-01 Session Stability
- **Owner**: AI QA/Ops Workcell.
- **Objective**: maintain uninterrupted runtime during live operation.
- **SLIs**:
  - **crash-free session ratio**: fraction of sessions that end in operator-initiated quit rather than unhandled crash. A "session" is defined as continuous `.chromaShow` operation from load to quit with a minimum duration of 5 minutes (shorter sessions are excluded as non-live).
  - **unrecoverable scene-failure ratio**: fraction of scene launches that end in a `degraded` state requiring operator action to recover, divided by total scene launches. "Unrecoverable" = the scene entered the fallback-scene state in `37-authoring-environment-spec.md` §6 and did not self-recover within 10 seconds.
- **Targets**:
  - crash-free sessions >= 99.5% (rolling release window).
  - unrecoverable scene failures <= 0.5% of scene launches.
- **Measurement method**: self-hosted Sentry crash reports (opt-in only) merged with in-app `show.session.end` telemetry events under NFR-8a; pre-GA uses 24 h soak harness in `EV-SOAK-CHAOS`.
- **Evidence**: `EV-SOAK-CHAOS` (pre-GA) + release telemetry roll-up (post-GA).

## SLO-02 Input-to-Output Responsiveness
- **Owner**: AI Runtime Workcell + AI Integration Workcell.
- **Objective**: preserve performable control latency from external input to visible output.
- **SLIs**:
  - OSC/MIDI input-to-visible-output latency.
  - Population: every control-value change received from a loopback OSC or USB MIDI test harness during the latency test, measured from inbound message timestamp to presented-frame timestamp for the first frame reflecting the new value.
- **Targets**:
  - p50 <= 25 ms.
  - p95 <= 45 ms.
  - p99 <= 65 ms.
- **Rationale for p99**: a p99 miss above 60 ms during a live cue is visible to an audience. Live-performance discipline matches SLO-04's tail discipline.
- **Measurement method**: closed-loop latency harness: a local-loop OSC/MIDI emitter stamps an outbound packet, the runtime applies the value, and a GPU timer-query on the first frame containing the new uniform is compared to the emitter stamp. Harness documented in `EV-PERF-BUDGETS` methodology appendix.
- **Evidence**: `EV-PERF-BUDGETS`.

## SLO-03 Frame Delivery Health
- **Owner**: AI Runtime Workcell.
- **Objective**: maintain stable rendering under target complexity.
- **SLIs**:
  - frame-time on `baseline-profile-fixture-subset` (defined in `34-golden-corpus-definition.md` Sec.Baseline Fixture Subset).
  - dropped-frame ratio during sustained runs on the same subset.
- **Targets**:
  - p50 frame-time <= 12.0 ms.
  - p95 frame-time <= 16.67 ms.
  - p99 frame-time <= 33.33 ms.
  - dropped-frame ratio <= 1.0% averaged over the measurement window.
- **Rationale for p99**: a dropped frame is visible; the p99 target explicitly allows occasional two-frame pauses but not three-frame stalls. Relaxed from the p95 target because shader-cache misses and OS scheduler jitter are bounded, not eliminable.
- **Measurement method**: per-frame timestamp delta on baseline-profile fixture subset, run on the GPU-A reference-hardware list in `30-mvp-support-matrix.md` MED-GPU table. Warm shader cache; no hot-reload; no cold scene-load inside the sampled window. Fixture subset declared in `34`.
- **Rationale (scope)**: SLO-03 is measured on a documented fixture subset (single-pass + up-to-3-pass, no cold shader cache, no hot-reload). The `stress-profile` on `performance-profile` covers higher complexity with relaxed targets. See `10-performance-latency-budgets.md`.
- **Evidence**: `EV-PERF-BUDGETS`, `EV-SHADER-CACHE`.

## SLO-04 Recovery from External Device Disruption
- **Owner**: AI Integration Workcell.
- **Objective**: recover quickly from controller/network input disruption without operator action.
- **SLIs**:
  - automatic reconnect success ratio for known devices/routes.
  - reconnect recovery time (p50, p95, p99).
- **Targets**:
  - reconnect success >= 99.0% for known devices/routes within the measurement window.
  - p50 recovery <= 3 s.
  - p95 recovery <= 10 s.
  - p99 recovery <= 20 s.
- **Rationale**: median alone is insufficient for live shows - a p99 reconnect over 30 s is show-ending; tight tails are the live-performance discipline. Reconnect cadence `100/250/500/1000/2000 ms` in `08-io-integrations.md` sums to 1850 ms through four attempts, well inside p50; loops at 2000 ms cap to cover p95/p99.
- **Definition of "known device"**: any device that has been successfully bound at least once during the current show file and whose device ID appears in the show bundle's mapping state.
- **Measurement method**: chaos-injection harness induces disconnects on loopback OSC, USB MIDI, and Syphon/Spout/NDI adapters on a randomized schedule; measures wall-clock between the disconnect event and the subsequent `Connected` state transition.
- **Evidence**: `EV-INTEGRATION-MATRIX`, `EV-SOAK-CHAOS`.

## SLO-05 Deterministic Replay Confidence
- **Owner**: AI Runtime Workcell + AI QA/Ops Workcell.
- **Objective**: preserve deterministic output progression for test harness runs.
- **SLIs**:
  - deterministic replay pass ratio on golden corpus.
- **Targets**:
  - deterministic replay pass ratio >= 99.0% for supported profiles.
- **Measurement method**: golden-corpus harness replays each fixture with pinned audio, pinned RNG seed, pinned profile version; per-frame output compared pixel-wise within drift epsilons defined in `10-performance-latency-budgets.md` Determinism and Drift Acceptance Table.
- **Drift epsilons**: per-family thresholds are defined in the Determinism and Drift Acceptance Table in `10-performance-latency-budgets.md`.
- **Evidence**: `EV-DETERMINISM`.

## SLO-06 Docs Version-Sync Integrity (post-release)
- **Owner**: AI Docs Workcell + AI QA/Ops Workcell.
- **Objective**: every shipped catalog symbol and normative contract has a live docs page; docs never drift behind symbol surface.
- **SLIs**:
  - coverage ratio: (catalog symbols with a matching in-app docs page) / (total catalog symbols in `36-reactive-symbol-catalog.md` + `08-io-integrations.md` OSC route set).
- **Targets**:
  - coverage >= 98% at every release candidate.
  - no shipped release with coverage < 95%.
- **Measurement method**: CI generator cross-references the symbol catalog and OSC route table against the docs site map; missing pages fail the build. See `EV-DOCS-VERSION-SYNC`.
- **Evidence**: `EV-DOCS-VERSION-SYNC`, `EV-DOCS-COVERAGE`.

## SLO-07 Outbound Traffic Zero-Default Conformance
- **Owner**: AI QA/Ops Workcell + Accountable Owner.
- **Objective**: no release build produces outbound network traffic to a destination not declared in `OBSERVABILITY-INVENTORY.md` when all observability toggles are off.
- **SLIs**:
  - packet count to non-declared destinations during the declared measurement window with all opt-in toggles off.
- **Targets**:
  - 0 (zero) packets to non-declared destinations.
- **Measurement method**: isolated-network test bed records all outbound traffic for 60 minutes of cold-start + idle + one scene load + one show load; any DNS lookup, TCP SYN, or UDP send to a host not in the inventory fails the release.
- **Evidence**: `EV-ANALYTICS-ZERO-DEFAULT`, `EV-OBSERVABILITY-INVENTORY`.

## Monitoring and Alerting
- Emit structured telemetry for each SLI; telemetry itself obeys NFR-8a (loopback-only by default; outbound only when operator has opted in).
- Trigger investigation when two consecutive validation windows miss target.
- Escalate to release block when any critical SLO misses by >10% relative margin.

## Release Gate Use
- Any release candidate must satisfy the cumulative release rule in `26-evidence-artifact-registry.md`.
- SLO-specific minimum evidence:
  - `EV-SOAK-CHAOS`
  - `EV-PERF-BUDGETS`
  - `EV-SHADER-CACHE`
  - `EV-INTEGRATION-MATRIX`
  - `EV-DETERMINISM`
  - `EV-DOCS-VERSION-SYNC`
  - `EV-ANALYTICS-ZERO-DEFAULT`
- If registry evidence status and SLO checks disagree, registry gate status is blocked until discrepancy is resolved.
- SLO evidence is necessary but not sufficient; missing non-SLO requirement evidence still blocks release.
