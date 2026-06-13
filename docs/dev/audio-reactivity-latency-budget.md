<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Audio-reactivity latency budget (vs MilkDrop / butterchurn / Synesthesia)

Where Chromaticity's live audio reactivity stands against the reference engines,
term by term, so we always know **where we are and where we are going** — and
never confuse "smooth" with "as fast as it can be."

Sourced from a clean-room benchmark (2026-06-05): three independent profilers
(ours from source, MilkDrop/butterchurn from public/MIT source, Synesthesia by
clean-room observation of `Contents/` per `docs/29`), synthesised into a
scorecard, then an **adversarial verification pass** that downgraded every
verdict not backed by evidence (so we under-claim rather than over-claim).
Clean-room firewall honoured throughout: facts and architecture only, no copied
expression.

## TL;DR

- **Quality core: we match or exceed all three references.** Our per-band AGC is
  parameter-identical to butterchurn (MIT — an independent reconstruction, see
  `reactive_bands.rs` and EV-LICENSE-AUDIT), our 1024-pt FFT is finer than
  MilkDrop's, our perceptual band split is byte-identical to butterchurn's, and
  our **10 ms decoupled analysis cadence exceeds** both references' frame-locked
  schemes.
- **We lag on one axis — audio→pixel latency — and now only vs butterchurn.**
  ~52 ms (reactive-band axis, after fix #1) vs butterchurn ~35 ms; we now beat
  Synesthesia's smoothed family (~70 ms) and sit at the edge of its snappy
  "Hits/OnBeat" family (~30–50 ms). The 2026-06-05 benchmark measured ~62 ms
  before fix #1 landed.
- The gap is **controllable** delay the frame-locked references don't pay, and it
  shaves **without** touching the slow-release smoothness.
- **Inherent floor ≈ 36 ms** (driver buffer + half-frame staleness + GPU
  present) — essentially butterchurn's *whole* budget. Once the controllable
  terms are gone we land at the references.

## The band-axis budget (the reactive-critical path)

Signal-onset → pixel, at 48 kHz capture / 60 fps render. The live render window
reads the shared `Arc<Mutex<AudioUniformSnapshot>>` **directly** each frame
(`app/shell/src/lib.rs:245`, `render/thread.rs:264-278`); the Tauri
`audio_uniform_snapshot_changed` IPC emit feeds the webview meters only and is
**off** this pixel path.

| Term | ms | Class | Source |
| --- | --- | --- | --- |
| Capture / driver buffer (`BufferSize::Default`, ~512 frames) | 10.7 | inherent (OS) | `cpal_source.rs:1148` |
| **Symmetric-Hann group delay** (window centre-weights the buffer middle) | 10.7 | **controllable** | `reactive_bands.rs:136-176` |
| Engine AGC short EMA | ~2 | inherent (= references) | `reactive_bands.rs:197-211` |
| Analysis cadence staleness (10 ms fixed extract) | ~5 | **controllable** | `runtime.rs:149` |
| Producer hop backoff | ~0.25 | negligible | `runtime.rs:126,133` |
| Snapshot/frame staleness (read once per 16.7 ms frame) | ~8.3 | inherent (decoupled arch) | `thread.rs:264-278` |
| Render-side ballistics attack (BAND attack tau → 0; fix #1) | 0 (was ~10) | **landed** | `render/audio_smooth.rs` |
| GPU render + present | ~16.7 | inherent | `render/output_window.rs` |
| **Total (band axis)** | **~52** (was ~62 pre-fix-#1) | | |

FFT/peak axis ≈ 51 ms (was ~57; their ~6 ms attacks also went to 0 under
fix #1); RMS axis ≈ 60 ms (was ~81; its render attack was the longest at
30 ms, now 9 ms under fix #2). BPM is a background, 250 ms-cadence,
parity-only backup and is **0 ms on this path** (`runtime.rs:159`).

## Where we stand per metric (condensed scorecard)

| Metric | Verdict | Note |
| --- | --- | --- |
| Audio→pixel latency (band axis) | **lag** (vs butterchurn only) | ~52 ms vs butterchurn ~35 ms — beats Synesthesia's smoothed ~70 ms; the remaining gap is fixes #3/#4 |
| Render-side ballistics (2nd stage) | match | attack = 0 since fix #1 — adds no onset latency; the slow release is extra smoothness the references lack |
| Analysis cadence / buffering | **exceed** | 10 ms decoupled + drain-all beats frame-locked ~16.7 ms / ~23 ms-hop |
| FFT size | match | 1024-pt: finer than MilkDrop, snappier window than Synesthesia's 2048 |
| Window function | match | Hann beats MilkDrop (rectangular); Synesthesia offers a *configurable* superset |
| Band split | match | byte-identical to butterchurn's perceptual 20-320/320-2800/2800-11025 Hz |
| AGC / volume independence | match | param-identical to butterchurn (MIT clean-room) |
| Attack/release smoothing (engine EMA) | match | algebraically identical to butterchurn — **the praised smoothness; do not touch** |
| Beat / BPM detection | lag (feature) | Synesthesia has dual-onset + confidence + phase-locked clocks; we match MilkDrop/butterchurn |
| FFT post-processing / spectral shaping | lag (feature) | Synesthesia's "Juiced" (dB + log-freq + HF boost + bin blur) is richer; not a latency term |
| Dynamic range | match | centered-ratio semantics identical to MilkDrop/butterchurn |
| Capture / OS input buffer | match | OS-owned, comparable across all |

Feature gaps (beat detection, Juiced shaping, a 4th band, configurable window)
are **not** latency lags and are tracked separately; they do not block the
latency work.

## Fix plan (every fix preserves the slow-release smoothness)

Ranked by leverage × safety. None touch any *release* tau or the engine AGC
`SHORT_RELEASE` / `LONG_STEADY` — that is where the AO-praised smoothness lives.

| # | Fix | ms | Risk | Status |
| --- | --- | --- | --- | --- |
| 1 | **PPM instant attack** on peak/FFT/band axes (attack tau → 0, keep release) | ~9 (band) | low | **✅ landed 2026-06-05** (`audio_smooth.rs`) |
| 2 | **RMS attack 30 → 9 ms** (keep 250 ms release) | ~20 (RMS axis) | low | **✅ landed 2026-06-05** (`audio_smooth.rs`) |
| 3 | **Causal/asymmetric analysis window** (or 512-pt FFT) to kill the Hann group delay | ~5–8 | medium | planned — engine-core; needs `reactive_bands` test validation + an offscreen A/B |
| 4 | **Halve `ANALYSIS_INTERVAL` 10 → 5 ms** | ~2 | low | planned — measure producer CPU duty (`mini_meter`) first |
| 5 | Faster present cadence (e.g. 120 Hz) to cut frame staleness | ~1 (read-order); more needs higher Hz | — | out of scope — trades GPU/power; read-ordering alone does **not** help |

**Projected band-axis trajectory:** ~62 ms (before) → ~52 ms (after #1) → ~44 ms
(after #3) → ~42 ms (after #4) → floor ≈ 38 ms. butterchurn ≈ 35 ms. So #1 + #3
(+ #4) lands us at the references, with the slow-release smoothness intact.

## Methodology & provenance

- Benchmark workflow: 3-profiler fan-out → synthesis → adversarial verdict-check.
- Clean-room: Synesthesia profiled by **observation** of `Contents/` (facts and
  architecture only, no expression copied) per `docs/29`. butterchurn/MilkDrop
  are public/MIT.
- The butterchurn parameter identity is recorded for **EV-LICENSE-AUDIT** as an
  independent reconstruction with MIT compatibility (see ADR-024).

## Related

- `app/shell/src/render/audio_smooth.rs` — the render-side ballistics (fixes #1/#2).
- `engine/audio/src/reactive_bands.rs` — the AGC + analysis window (fix #3).
- `engine/audio/src/runtime.rs` — the analysis cadence (fix #4).
- `docs/adr/ADR-024-audio-reactivity-analysis-model.md` — the AGC model.
- `docs/dev/macos-desktop-run-and-audio-diagnostics.md` — how to run/test it.
- `docs/dev/ui-smoothness-and-latency.md` — the general fast-attack/slow-release principles.
