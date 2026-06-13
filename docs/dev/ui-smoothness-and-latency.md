<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# UI Smoothness & Latency — DAW-derived techniques

The bar for every live-updating surface in Chromaticity (level meters, spectra,
sparklines, BPM rings, beat dots, scene preview, scrubbers/playheads, reactive
widgets) is **professional DAWs** — Ableton Live, Pro Tools, Logic. DAWs are
engineered for jank-free, low-latency real-time visuals; "buttery smooth" is the
explicit AO goal. This note captures the DAW-derived techniques so they are
applied consistently rather than rediscovered per-widget.

Source survey: a 2026-06-04 metering-standards research pass (citations at the
end). Related: `docs/adr/ADR-023-audio-metering-data-model.md` (metering data
model), `docs/adr/ADR-024-audio-reactivity-analysis-model.md` (AGC reactivity),
and the shared primitive `src/lib/audio/metering.ts`.

## The nine principles

1. **Decouple data cadence from render cadence.** Audio callbacks arrive far
   faster than the eye or the display needs (hundreds–thousands per second). DAWs
   sample the *latest* value and render at a fixed, comfortable rate. Rendering on
   every data event wastes frames and causes jank. In Chromaticity this is the
   engine producer's drain-all + fixed-cadence analysis (the heavy extract runs at
   a set interval, not per ingest frame; BPM autocorrelation runs at a 250 ms
   background cadence off the reactivity-critical path), the 60 Hz scheduler tick,
   and the UI's rAF loops reading the latest snapshot.

2. **Render in the 40–60 Hz band; ~25 ms is the snappiest *usable* refresh.**
   Faster than ~25 ms looks jittery and burns CPU for no perceptual gain; slower
   than ~50 ms feels laggy. A 60 Hz (16.7 ms) UI update sits comfortably inside
   the responsive band. Do not drive a visual faster than the eye resolves.

3. **Fast attack, slow release.** The perceptual core of "responsive *and*
   readable": snap up to a transient within a few ms, then glide back down over
   ~0.7–1 s so the eye can register where the peak was. This is exactly PPM
   ballistics (≈3–6 ms attack, ~0.7–1 s return). A symmetric smoother is either
   too sluggish on the way up or too twitchy on the way down — asymmetry is what
   reads as professional.

4. **dt-aware smoothing; never per-frame magic constants.** Use a
   wall-clock-dt-aware one-pole follower, `coeff = 1 − exp(−dt/τ)`, so behaviour is
   identical at 60 / 120 / 144 Hz. A per-frame EMA (`x += 0.25·(target−x)`) or a
   per-frame decay (`peak −= 0.004`) runs **twice as fast on a 120 Hz display** —
   the same code feels different on different hardware. `src/lib/audio/metering.ts`
   exposes dt-aware `ExpEnvelope` / `AttackReleaseEnvelope` / `PeakBallistics` /
   `PeakHold`; every live widget should consume these rather than hand-roll an EMA.

5. **The smoother is the ONLY smoothing layer — no CSS `transition` on a
   continuously-moving value.** A `transition: width 120ms` (or `background`,
   `height`, `transform`) layered on top of a value already smoothed in JS adds
   that duration as *fixed lag on a moving target* and makes the element smear
   instead of tracking. CSS transitions are for discrete state changes (hover,
   open/close), not for per-frame-updated live data. (This is the project's
   recurring "FIX 2".)

6. **Animate compositor-only properties.** Prefer `transform` (translate / scale)
   and `opacity` — they run on the GPU compositor with no layout or paint. Avoid
   animating `width` / `height` / `top` / `left` per frame, which forces layout
   and paint. A meter fill driven by `transform: scaleX()` (with
   `transform-origin` at the fill's anchored edge) is cheaper and smoother than one
   driven by `width%`. Reserve layout-property animation for one-shot, low-rate
   changes.

7. **Batch state writes to once per frame; untrack high-frequency reads.** Write
   the reactive `$state` once per rAF tick, not once per inbound data event, so the
   framework re-renders at most once per frame. Read the high-frequency source
   (the snapshot store) inside `untrack(...)` so an effect does not tear down and
   re-create its rAF loop on every ~60 Hz store write.

8. **The right latency *profile* beats zero latency.** Smoothness is not the
   absence of latency — it is the correct latency shape. The slow release in
   principle 3 is *intentional* latency that aids perception. A peak-hold marker
   that lags the true peak by ~40 ms is fine and standard; spend the responsiveness
   budget on the level fill's attack, not on chasing an instantaneous marker.

9. **Honour `prefers-reduced-motion`.** Freeze animated surfaces to a static,
   representative pose when the user has requested reduced motion — never a blank
   or a frozen-at-zero state.

## Where these are applied today

- `src/lib/audio/metering.ts` — dt-aware followers + dBFS log scale + calibrated
  ballistics (principles 3, 4, 8). Consumed by `Meter.svelte` and the Settings
  input monitor.
- Engine `engine/audio/src/runtime.rs` — drain-all + fixed-cadence analysis;
  reactivity-critical rms/peak/FFT on the fast tick, BPM autocorrelation on a
  250 ms background cadence (principle 1).
- `VuStereo.svelte` — migrated onto the dt-aware `metering.ts` followers + dBFS
  scale (principles 3, 4); its live fill no longer uses a per-frame EMA.
- `Meter.svelte`, `AudioReactivity.svelte` — no CSS transition on the live fill
  (principle 5). `AudioReactivity.svelte`'s band smoother is a
  wall-clock-dt-aware exponential (`coeff = 1 − exp(−dt/τ)`, τ = 21 ms, dt
  clamped to 100 ms for tab-restore), replacing the former fixed 0.55-per-frame
  alpha (principle 4; inline, not yet on `metering.ts`).
- `BpmRing.svelte`, `BeatDots.svelte` — beat phase integrates incrementally per
  rAF frame (`phase += dt·bpm/60`, wrapped) instead of elapsed-time ×
  instantaneous-BPM, so a live BPM update changes only the future sweep rate
  (principle 4); the ring's 100 ms CSS transition on the live stroke-dashoffset
  is removed (principle 5). The dots keep an 80 ms transition on the discrete
  on/off state flip, which principle 5 permits.
- `Xy.svelte` — no CSS transition on the dot position (left/top under
  continuous pointer-drag updates) (principle 5).

## Tracked follow-ups (not yet on the primitive)

- `AudioUniforms.svelte` still runs its own EMA + per-frame peak decay on the dB
  readout (principle 4); it is data-cadence (per-snapshot, ~60 Hz), not a
  display-refresh-jank source, but migrating it onto `metering.ts` is tracked.
- Meter fills animate `width%` / `height%`; moving them to `transform: scaleX/Y`
  (principle 6) is a candidate optimization once the primitive migration lands.

## Sources

- Peak programme meter — Wikipedia (IEC 60268-10/-18 ballistics, attack/return
  times, transient under-indication).
- VU meter — Wikipedia (IEC 60268-17, 300 ms rise/fall).
- dBFS — Wikipedia; EBU R128 + EBU TECH 3341/3343 (true-peak, −1 dBTP).
- Sound On Sound — "Peak Performance" (meter refresh rate, peak-marker lag);
  "PPM vs VU"; reference levels in digital audio (EBU R68 0 dBu = −18 dBFS).
- SonicScoop — audio metering overview (fast-attack/slow-release perception, ~3 s
  peak hold).
- Biamp Tesira — Audio Meter (configurable hold + decay rates).
