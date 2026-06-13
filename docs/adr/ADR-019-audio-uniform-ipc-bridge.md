<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-019: Engine→UI Audio-Uniform IPC Bridge

## Status

Accepted

## Date

2026-05-03

## Deciders

- Accountable Owner: Xavier Butler (@PhantasticUniverse)

## Context

Phase-4a EXIT was achieved 2026-05-03 at commit `5acc452` with `EV-INTEGRATION-MATRIX` passing under an AO-approved narrowing that scopes its conformance corpus to OSC routing-matrix only plus the Wave 1b MIDI input arm. The camera, Syphon, and audio-uniform-IPC arms were explicitly deferred into a Wave 1c follow-on per `docs/11-roadmap-phases.md` and the Phase-4a EXIT closure section of `docs/EV-GATE-MODEL-AUDIT-checklist.md`. Wave 1c has been restructured into two independently shippable slices:

- **1c-α** — audio-uniform IPC bridge (this ADR governs).
- **1c-β** — camera + Syphon adapters (deferred; future ADR scope).

The producer side of the audio-uniform path is at EXIT quality. `chromaticity_audio::AudioUniformSnapshot` (`engine/audio/src/uniforms.rs` lines 170-191) is a `#[derive(Debug, Clone, Copy, PartialEq)]` value type carrying `bpm: f32`, `rms: f32`, `peak: f32`, and `fft_bands: [f32; FFT_BAND_COUNT]` (8 log-spaced FFT bands). The scheduler-tick (`engine/runtime/src/scheduler/audio.rs` lines 16-28) commits a fresh snapshot once per tick before frame advancement, on the state-commit thread, against the ADR-006 audio-analysis-thread → state-commit-thread `Arc<Mutex<AudioUniformSnapshot>>` seam.

The consumer side is also at EXIT quality. The Svelte widgets `AudioReactivity.svelte`, `BpmRing.svelte`, and `BeatDots.svelte` (under `src/lib/widgets/`) already accept `bpm` / `bands` props and run rAF loops against synthetic time-domain functions. The bundle's standalone Spectrum widget was never ported as a separate component — its surface was inlined into `src/lib/views/AudioUniforms.svelte` with a 64-bar synthetic visualisation. Each widget carries an in-source "Frozen frame contract: synthetic, no IPC dispatch" comment marking the intentional placeholder; the W14-W34 UI port waves shipped them as wireframe-faithful chrome without engine wiring.

The IPC bridge between producer and consumer does not yet exist. ADR-018 locks Tauri 2 + Svelte 5 + TypeScript as the consumer stack and `ts-rs` as the bridge for shared types, but it does not specify the IPC contract for engine-driven push events. The existing IPC patterns in `app/shell/src/commands/` (e.g. `control.rs::set_control` / `get_control`, `osc.rs::osc_health_snapshot`) are exclusively request-response `#[tauri::command]` handlers; there is no high-frequency push path in production. Tauri's `app.emit()` event infrastructure is available but unused.

The decision-problem this ADR locks is the IPC contract for one-way Engine→UI delivery of `AudioUniformSnapshot` data at scheduler-tick cadence so the visualizer widgets can replace synthetic time-domain functions with real audio. Wave 1c-α (2-PR engineering wave) will ship the implementation against this contract.

## Decision Drivers

- Lower latency than rAF-driven pull: visualizers run at display refresh (~60 Hz) and need fresh data every frame.
- Match the existing `OscHealthSnapshot` ts-rs payload pattern in `app/ui-bridge/src/lib.rs` so new bridge-contract surface is recognisable to IPC reviewers.
- Preserve the ADR-006 thread-affinity invariant: durable state mutation stays on the state-commit thread; the IPC bridge MUST NOT introduce a second writer to `StateCore` or to the audio snapshot.
- Preserve the ADR-001 single-canonical-state-model invariant: the snapshot is observed not authored on the UI side.
- Drop frames safely under consumer-side or Tauri-event-queue saturation; the visualizer surface degrades gracefully (last-good frame stays painted).
- Zero-cost when no consumer is mounted; the producer must not maintain a subscriber count or pay enumeration cost on each tick.

## Considered Options

1. **Push via `app.emit()` event** — engine emits `audio_uniform_snapshot_changed` once per scheduler tick; Svelte listeners attach via `listen(...)` when mounted. (Chosen.)
2. **Pull via `#[tauri::command]` handler** — Svelte rAF loop calls `get_audio_uniform_snapshot()` once per frame.
3. **Shared-memory ring-buffer through the Tauri custom-protocol surface** — engine writes the snapshot into a memory-mapped region; the web view reads via a synchronous fetch.

## Decision

Adopt Option 1: **PUSH delivery via Tauri `app.emit()`** with the following locked contract.

1. **Push vs pull — push.** The engine emits a single Tauri event named `audio_uniform_snapshot_changed` once per scheduler tick. Lower latency than an rAF-driven pull command (eliminates the per-frame round-trip), matches the trajectory the OSC health surface is on (which moves from request-response polling to event-pushed health changes), and gives a simpler Svelte consumer (`listen(...)` on mount; auto-cleanup on unmount).

2. **Payload shape.** A new ts-rs-bridged Rust newtype, `AudioUniformSnapshotPayload`, is added in `app/ui-bridge/src/lib.rs` mirroring the `OscHealthSnapshot` pattern. Fields are a verbatim mirror of the engine type:
   - `bpm: f32`
   - `rms: f32`
   - `peak: f32`
   - `fft_bands: [f32; 8]` (the existing `FFT_BAND_COUNT` from `engine/audio/src/uniforms.rs`).
   The payload is a value type (`Copy + Clone` in Rust); the IPC layer takes a value, not a reference. ts-rs derives the matching TypeScript declaration for the Svelte side; the engine's `AudioUniformSnapshot` is converted to the payload at the emit call site (no impl block on the engine type — the bridge crate owns the conversion to keep `engine/audio` free of UI-shell coupling).

3. **Cadence — once per scheduler tick.** Emission cadence is fixed to the scheduler-tick rate (60 Hz nominal frame loop per ADR-001 / ADR-006 plus the slice-3 wave that closed `EV-SCHEDULER-TICK` at `EV-SCHEDULER-TICK`). The emit step is added to the scheduler-tick step ordering AFTER observability emit (the existing last step) so it does not affect the load-bearing audio-pull → bus-drain → frame-advance → render-dispatch → observability-emit ordering already pinned by `engine/runtime/src/scheduler/`. The IPC bridge MUST NOT introduce its own throttling; cadence is bounded upstream by the audio-analysis thread that writes the snapshot into the `Arc<Mutex<AudioUniformSnapshot>>` the scheduler reads.

4. **Backpressure — drop-frames on Tauri event-queue saturation.** Visualizers do not need every frame: rAF caps consumer-side at display refresh (typically 60 Hz), which matches the emit cadence. The producer is non-blocking — Tauri's `app.emit()` is best-effort and silently drops if the event queue is full or all listeners are unsubscribed. No queueing, batching, or coalescing is added on the producer side; if the consumer can't keep up the most recent snapshot is the only one visualizers care about, and the next tick emits a fresher one.

5. **Thread affinity.** The state-commit thread emits (the same thread that runs the scheduler tick per `engine/runtime/src/scheduler/audio.rs`). The Svelte main thread receives via Tauri's event delivery. There is no lock contention introduced on emit because the snapshot is a `Copy` value type — the scheduler-tick already holds the snapshot by value at the emit-step boundary. The ADR-006 single-writer-`StateCore`-on-commit-thread invariant is preserved verbatim: the bridge reads not writes.

6. **Error handling — silent listener cleanup; stale-but-last-good on audio fault.** Listener disconnect (widget unmount, web-view reload) is handled silently by Tauri's event infrastructure — the producer takes no action and pays no enumeration cost. On an audio-analysis fault (audio thread panics or stalls), the existing `MutexAudioPull` poison-recovery path (`engine/runtime/src/scheduler/audio.rs` lines 91-106) returns the last-committed snapshot, so the scheduler-tick continues to emit the same value until the audio thread recovers. Consumers see frozen visualizers; no error toast or console message at MVP. A future post-MVP "audio-degraded" diagnostic surface can layer on without changing the snapshot contract.

7. **Subscription semantics — opt-in.** Svelte widgets call `listen('audio_uniform_snapshot_changed', cb)` only when mounted; the returned unlisten handle runs at component unmount. The Rust side does not maintain a subscriber count; emit is unconditional whether or not any listener is attached. This trades a per-tick emit cost (negligible at 60 Hz with a value-type payload) for the simplicity of avoiding subscription bookkeeping and the associated thread-affinity hazards.

## Consequences

### Positive

- Visualizer widgets gain real audio data without changing their existing `bpm` / `bands` prop shape; the per-widget rAF loop becomes a `listen(...)`-fed `$state` rune update.
- Latency floor is bounded by Tauri event delivery (sub-frame on both macOS WKWebView and Windows WebView2 in measurements taken during ADR-018 evaluation), which is below the 16 ms 60 Hz frame budget by a comfortable margin.
- The producer-side emit-step composition matches the established `engine/runtime/src/scheduler/` sub-module + builder + tick-step extension pattern (`with_metric_emitter` / `with_audio_pull` / `with_render_dispatch`); the new emit step is purely additive and cannot regress prior `EV-STATE-REPLAY` / `EV-PRESET-PLAYLIST` / `EV-STATE-MIGRATION` / `EV-SCHEDULER-TICK` corpus goldens because they do not observe the IPC seam.
- ts-rs continues to be the single shared-type generator per ADR-018; the new `AudioUniformSnapshotPayload` follows the `OscHealthSnapshot` precedent so reviewers recognise the bridge surface immediately.
- The `AudioReactivity` widget's existing visual stubs (band-tile values, BPM, peer-count) lift directly onto the real snapshot field-by-field without a structural rewrite.

### Negative

- The producer pays a per-tick emit cost (a small Rust → JSON serialization on the Tauri event path) whether or not any listener is attached. Measured cost is negligible at 60 Hz with a `f32 × 11` payload (bpm + rms + peak + 8 FFT bands), but the cost is structural and cannot be elided without adding a subscriber-count gate that would re-introduce thread-affinity hazards.
- Drop-frames-on-saturation is invisible to the consumer at MVP. A widget that wants to detect sustained drops would need a diagnostic surface that does not exist yet (post-MVP follow-on; ADR-019 does not preclude it but does not specify it).
- The contract is one-way Engine→UI; tap-to-tempo / transport-control round-trips, beat-grid event push, and any per-beat (`audio_beat_detected`) push surface live outside ADR-019 and must be specified separately if pursued.

### Neutral / trade-offs

- The decision aligns the audio surface with the existing OSC health-snapshot trajectory (push event over request-response), which sets a precedent the camera / Syphon arms in Wave 1c-β can mirror or deviate from independently.
- ADR-019 does not commit to Tauri's event channel as the long-term IPC pattern for all subsystems; it is a per-surface decision and other subsystems remain free to keep request-response semantics where appropriate.

## Alternatives Considered

- **Pull via `#[tauri::command]` handler.** Rejected. Per-frame round-trip latency stacks on top of the rAF callback budget and is empirically the dominant tail-latency contributor on touring-laptop hardware. The pull pattern also makes per-tick freshness non-deterministic — the consumer's rAF clock and the producer's scheduler-tick clock drift, and the pull window spans an arbitrary number of producer ticks. The push contract pins the snapshot to the scheduler tick that produced it and matches the upstream cadence exactly.
- **Shared-memory ring-buffer through the Tauri custom-protocol surface.** Rejected. ADR-018's "Tauri 2 + native web view" composition explicitly does not assume shared-memory IPC between Rust and the web view; the custom-protocol path is for resource serving (`tauri.conf.json` `frontendDist`) not arbitrary memory mapping. Adding a ring-buffer surface would also widen the attack surface (web-view-accessible memory) and conflict with the ADR-013 §1/3/5 hardening posture for IPC channels. The complexity is unjustified at the f32×11 payload size — Tauri events handle this cleanly.

## Out of Scope

- **IO_CAMERA / camera-input IPC.** Wave 1c-β scope; future ADR.
- **Syphon / Spout output IPC.** Wave 1c-β scope; future ADR.
- **Per-beat event push** (e.g. `audio_beat_detected`). Out of MVP. The tick-rate snapshot push gives the visualizer enough data via per-frame `bpm` + amplitude; a per-beat surface would duplicate semantics that already live on `bpm` and would require a separate timing-jitter contract.
- **MIDI / OSC route push events.** Wave 1b's MIDI adapter ships its own diagnostic counters (`integrations/midi-adapter/`); OSC has its own health snapshot. ADR-019 governs ONLY audio-uniform snapshot IPC.
- **Tap-to-tempo / transport-control round-trips.** ADR-019 is one-way Engine→UI for the audio surface. Bidirectional surfaces are outside its scope and remain on the existing `#[tauri::command]` request-response pattern.
- **Audio-degraded diagnostic surface.** The "stale-but-last-good" behaviour on audio fault is the MVP error model; a layered diagnostic (e.g. a UI banner indicating frozen-audio detection) is post-MVP and does not change the snapshot contract.

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. ADR-019 adds a new evidence row to `docs/26-evidence-artifact-registry.md` covering the bridge round-trip; the existing audio-trace and integration-matrix rows are unaffected.

- **`EV-AUDIO-UNIFORM-IPC`** — new T2 conformance corpus row added to `docs/26` per this ADR. Targets the IPC bridge's payload-identity round-trip: Rust producer emits `AudioUniformSnapshot { bpm, rms, peak, fft_bands }` → ts-rs marshals to TypeScript → Svelte listener receives → field-by-field equality assertion. Status `Pass under reduced narrowing` per Wave 1c-α closure (anchor run ids `25278169331` + `25277801336`) and Wave 1c-α-2 production-bootstrap closure (anchor run ids `25431406979` + `25431406965`). Live producer→Tauri-emit→consumer wiring flows in production with synthetic fallback retained for the no-snapshot startup window. (Scope note: this row covers the IPC **bridge** — payload-identity round-trip + producer→emit→consumer wiring — verified end-to-end with microphone + virtual-loopback capture sources; the macOS zero-install system-audio **tap source** is a separate concern owned by `ADR-022` and is non-functional pending its `IOProc` fix, which does not change this bridge contract.) Remaining narrowing axes are stale-but-last-good through the wire (post-MVP) plus the Spectrum 8 → 64 bar widening + `AudioReactivity` 48-bar FFT visualiser (post-MVP UX enhancements).
- **`EV-INTEGRATION-MATRIX`** — Phase-4a EXIT pass under the OSC routing-matrix + MIDI input narrowing remains unchanged. ADR-019 does not widen the row this PR; the audio-uniform-IPC arm's contribution to the matrix lands when Wave 1c-α corpus closes (and may be cross-tagged into `EV-INTEGRATION-MATRIX` at that point per the dual-tag pattern Wave 1b used for `EV-NET-HARDENING` ↔ `EV-INTEGRATION-MATRIX`).
- **`EV-AUDIO-TRACE`** — unaffected. The bridge consumes the same `AudioUniformSnapshot` value `EV-AUDIO-TRACE` already pins on the producer side.
- **`EV-SCHEDULER-TICK`** (`EV-SCHEDULER-TICK`) — unaffected by ADR-019. The new emit step composes additively per the established `with_metric_emitter` / `with_audio_pull` / `with_render_dispatch` builder pattern; the four pre-existing T2 corpora preserve their goldens because none observes the IPC seam.

## Traceability

- `adr/ADR-001-runtime-model.md` — the AudioRuntime publishes-sampled-reactive-state contract and single-canonical-state-model invariant ADR-019 honours.
- `adr/ADR-006-thread-and-process-model.md` — the dedicated audio-analysis thread + state-commit thread allocation that ADR-019's emit-on-state-commit-thread placement preserves.
- `adr/ADR-018-non-scene-ui-toolkit.md` — the Tauri 2 + Svelte 5 + ts-rs consumer-stack lock ADR-019 builds on.
- `adr/ADR-013-networking-defaults-and-security.md` — the IPC-hardening posture the rejected shared-memory ring-buffer alternative would have conflicted with.
- `adr/ADR-017-real-time-external-data-ingestion.md` — `Proposed`; covers data-ingest cadence/backpressure but explicitly NOT audio IPC, leaving the audio surface as ADR-019's scope.
- `26-evidence-artifact-registry.md` — `EV-AUDIO-UNIFORM-IPC` row added in the same PR as this ADR.
- `engine/audio/src/uniforms.rs` lines 170-191 — producer-side `AudioUniformSnapshot` definition.
- `engine/runtime/src/scheduler/audio.rs` lines 16-28 — scheduler-tick audio-pull seam the emit step composes after.
- `app/ui-bridge/src/lib.rs` — the ts-rs bridge crate the new `AudioUniformSnapshotPayload` newtype lives in alongside `OscHealthSnapshot`.
- `src/lib/widgets/AudioReactivity.svelte`, `src/lib/widgets/BpmRing.svelte`, `src/lib/widgets/BeatDots.svelte` — consumer surfaces with existing "Frozen frame contract: synthetic, no IPC dispatch" comments updated by Wave 1c-α PR 2 to dual-mode (live IPC OR synthetic fallback per "live snapshot present" gate). The bundle's Spectrum widget was inlined into `src/lib/views/AudioUniforms.svelte` rather than ported standalone; the 8 → 64 bar widening for that surface is documented as a post-MVP UX enhancement.
- `11-roadmap-phases.md` — Phase-4a Wave 1c sequencing this ADR opens.
- `EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a EXIT closure section — the AO direction deferring camera / Syphon / audio-uniform-IPC arms to Wave 1c that ADR-019 partially closes for the audio-uniform-IPC slice.

## Rollout and Review

- **Rollout (status: shipped).** Wave 1c-α landed 2026-05-03 across 3 PRs (`#338` ADR acceptance + new `EV-AUDIO-UNIFORM-IPC` at `Planned`; `#339` producer-side payload + `IpcEmit` trait + scheduler-tick emit step + T1 pins + T2 round-trip corpus; `#340` consumer-side dual-mode retrofits to `AudioReactivity` / `BpmRing` / `BeatDots` with synthetic fallback gated on "live snapshot present"). Cumulative scope landed within the under-1,500-LoC estimate. The bundle's standalone Spectrum widget was inlined into `src/lib/views/AudioUniforms.svelte` rather than retrofit; the inlined 64-bar synthetic surface stays synthetic-driven as a post-MVP UX enhancement.
- **Update — 2026-05-06 (Wave 1c-α-2 closure).** Production audio bootstrap landed across 3 PRs (`#343` `engine/audio/src/runtime.rs` `AudioRuntime` producer thread; `#344` `app/shell/src/state/ipc_emit.rs` `TauriIpcEmitter` + `AppState` lifecycle extension + drift-corrected 60 Hz scheduler-tick thread spawned in `app/shell/src/lib.rs::run`; closing PR adding 3 T2 cross-seam wire tests via `tauri::test::mock_app()`). Two of the original Wave 1c-α deferred axes drop per `docs/26` `EV-AUDIO-UNIFORM-IPC` — (i) live end-to-end producer→Tauri-emit→consumer wiring now flows in production; (ii) drop-frames-on-saturation panic-free posture (per Decision §4) exercised by the `emit_after_listener_unlisten_does_not_panic` fixture. Remaining narrowing axes are (iii) stale-but-last-good IPC surfacing through the wire (covered at producer-side T1 by `engine/audio/src/runtime.rs::MutexAudioPull` poison-recovery but not transitively through the IPC seam — future-wave option), (iv) Spectrum 8 → 64 widening (post-MVP UX), and (v) `AudioReactivity` 48-bar FFT visualiser staying synthetic-driven (post-MVP UX).
- **Review cadence.** ADR-019 is revisited if (a) Tauri's event-delivery latency exceeds the 16 ms 60 Hz frame budget on either MVP platform during continued validation, (b) the Wave 1c-β camera / Syphon work surfaces a payload-shape constraint that requires ADR-019 to widen the bridge contract beyond audio uniforms, or (c) a future post-MVP audio-degraded diagnostic surface needs a layered protocol that the current best-effort emit cannot accommodate.

---

> This ADR is binding once `Status: Accepted`. Modification requires a new ADR or a superseding ADR; do not edit the Decision section of an Accepted ADR in place.
