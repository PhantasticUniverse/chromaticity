<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Running & testing the desktop app on macOS (+ audio diagnostics)

A practical guide for exercising the Chromaticity desktop app on real macOS
hardware — the only place audio capture (microphone / system-audio tap) can be
validated, because CI runners have no audio device and no TCC consent. It covers
the one-command dev harness, the consent model, the `CHROMATICITY_AUDIO_DIAG`
telemetry, the authoritative capture self-test, and a live-vs-synthetic map of
the whole audio pipeline.

> **TL;DR**
>
> ```bash
> release-eng/dev-run-macos.sh --diag --device system-audio --tone
> ```
>
> Builds, signs with a **stable** identity (so the audio-consent prompt does
> **not** re-appear on every rebuild), launches with diagnostics on, then plays a
> known 440 Hz sine and tails the result. Grant the consent prompt **once**; from
> then on you can rebuild + relaunch autonomously.

---

## 1. Why you can't just `cargo run`

Two macOS realities shape the whole workflow:

1. **TCC consent needs a signed `.app` launched via LaunchServices.** A bare
   terminal-run binary attributes audio-capture consent to the terminal (or is
   denied outright). You need a real `.app` bundle (`Contents/Info.plist` with the
   usage strings) launched with `open`.
2. **TCC keys consent on the code signature's _designated requirement_.** An
   ad-hoc signature (`codesign -s -`) is hashed from the binary, so every rebuild
   changes it and macOS re-shows the consent prompt. A **stable self-signed
   certificate** gives a constant designated requirement, so consent granted once
   survives rebuilds. This is the single thing that turns audio debugging from a
   "click Allow every build" marathon into an autonomous loop.

`release-eng/dev-run-macos.sh` handles both. It is a **dev tool only** — the real
release path is `sign-macos.sh` + `notarize.sh` with the Developer ID cert
(ADR-014).

## 2. The dev harness — `release-eng/dev-run-macos.sh`

```bash
release-eng/dev-run-macos.sh [--diag] [--device <sentinel>] [--no-build] \
                             [--log <path>] [--tone] [-h]
```

| Flag             | Effect                                                                       |
| ---------------- | ---------------------------------------------------------------------------- |
| `--diag`         | Launch with `CHROMATICITY_AUDIO_DIAG=1`; telemetry streams to the log file.  |
| `--device <s>`   | Set `settings.audio.input_device` before launch (`default`, `system-audio`, or a device id). Applied on restart only. |
| `--no-build`     | Skip `npm run build` + `cargo build`; just re-sign + relaunch.               |
| `--log <path>`   | Diagnostics log path (default `/tmp/chroma-diag.log`).                       |
| `--tone`         | After launch, play a known 440 Hz / 0.5 sine and tail the readout. Implies `--diag`. |

What it does, in order: ensures the stable signing identity exists (creates a
dedicated keychain `chromaticity-dev.keychain-db` with a self-signed
code-signing cert on first run — fully non-interactive, no login-keychain
prompts) → builds → assembles `target/dev-macos/Chromaticity.app` with a complete
`Info.plist` (both `NSMicrophoneUsageDescription` and
`NSAudioCaptureUsageDescription`) → codesigns it with the stable identity and the
hardened-runtime entitlements → optionally sets the input device → relaunches via
`open` → optionally runs the tone probe.

**First run:** you will see the macOS audio-capture consent prompt once. Grant it.
**Every run after:** no prompt — same identity, persistent consent.

## 3. Consent model cheat-sheet

- **Microphone** → orange dot, `NSMicrophoneUsageDescription`. The default-input
  path.
- **System-audio tap** (zero-install output capture) → purple recording
  indicator, `NSAudioCaptureUsageDescription`, macOS 14.6+ Core Audio process
  taps.
- Reset all consent for a clean re-test: `tccutil reset Microphone $BUNDLE_ID` and
  `tccutil reset AudioCapture $BUNDLE_ID` (bundle id `app.chromaticity.desktop`).
- The dedicated dev keychain is reusable; delete it only to rotate the identity
  (which will re-prompt): `security delete-keychain ~/Library/Keychains/chromaticity-dev.keychain-db`.

## 4. `CHROMATICITY_AUDIO_DIAG` — the diagnostics

Setting the env var `CHROMATICITY_AUDIO_DIAG=1` turns on opt-in, zero-cost-when-off
telemetry across the whole capture→IPC→webview path. It is the tool that isolated
the system-audio corruption definitively (see ADR-022 Amendment 2026-06-03).

Lines you'll see in the log (`/tmp/chroma-diag.log` by default):

- **Bind line** (once, at capture start) —
  `[audio-diag] capture bound: <source> · device "<name>" · F32 · <rate> Hz · <ch> ch`
- **Engine readout** (4 Hz) — the snapshot the scheduler tick will emit:
  `[audio-diag] READY rms -XX.X peak -XX.X dBFS [meter-bar] bpm NNN fft ▁▃▅… cb=NNNN raw_peak -X.X dBFS`
  - `rms`/`peak` are the analysed snapshot values in dBFS; `cb` is the raw
    capture-callback count (climbing ⇒ the OS is delivering buffers); `raw_peak`
    is the peak amplitude of the raw captured samples (climbing/high ⇒ data is
    arriving even if the analyser disagrees — a key discriminator).
- **IPC emit** (throttled) — `[audio-diag] ipc emit #N -> 'audio_uniform_snapshot_changed' (result=ok)`
  confirms the engine side actually fires the Tauri event.
- **Webview rx** (throttled, round-trip) — `[audio-diag] webview rx #N rms=… peak=…`
  is the value the **webview** received, mirrored back via the `audio_diag_rx`
  command. Comparing engine-emit vs webview-rx pinpoints transport vs render
  bugs (they match ⇒ transport is fine).

Reading it (the bisection ladder):

| Symptom in the log                                   | Conclusion                                  |
| ---------------------------------------------------- | ------------------------------------------- |
| no bind line                                          | capture never started (source/permission)   |
| bind line but `cb` never climbs                       | blocked on consent, or OS delivers nothing  |
| `cb` climbs, `raw_peak` near-silent                   | nothing is actually playing into the tap     |
| `raw_peak` loud but `rms` near-zero / flat FFT        | **capture corruption** (the AUHAL bug)       |
| engine emit varies, webview rx matches                | transport + render are fine                  |

## 5. The authoritative capture self-test (the sine probe)

A pure tone is the cleanest possible probe because its expected metrics are exact.
`--tone` plays a **440 Hz / 0.5-amplitude** sine at full volume; for a faithful
capture expect:

| Metric       | Expected      |
| ------------ | ------------- |
| RMS          | ≈ **-9 dBFS** |
| Peak         | ≈ **-6 dBFS** (0.50) |
| FFT          | a single **440 Hz** bin |
| Crest factor | ≈ **3 dB** (a sine) |

Failure signatures and what they mean:

- **Peak _exceeds_ 0.5 / crest ≫ 3 dB / flat FFT** → byte/channel-stride
  corruption (this is the current macOS system-audio tap state — cpal's AUHAL
  read stage; see below).
- **rms ≈ -inf with `cb` climbing** → the tap delivers silence (nothing playing,
  or the all-zero bug).

You can also drive audio manually instead of `--tone`: any normal playback
(`afplay file.wav`, a browser, a DJ app) routes through the system output the
global tap captures. (Note: `say`/TTS may route around the captured mix — prefer
real file/app playback for a valid probe.)

## 6. Audio pipeline map — what's live, what's synthetic, what's broken

```
                          ┌─────────────────────────────── macOS system-audio TAP
                          │   CATapDescription (global) → private auto-start
                          │   aggregate → [cpal AUHAL AudioUnit read]  ✗ CORRUPTS
  capture source ─────────┤
                          │   microphone / line-in (cpal AUHAL)        ✓ WORKS
                          │   virtual loopback (BlackHole/Loopback)     ✓ WORKS
                          │   Windows WASAPI loopback                   ✓ (untested HW)
                          ▼
   engine/audio AudioRuntime producer thread                           ✓ WORKS
     → analyser (rms/peak/FFT/BPM over mono mix)                       ✓ WORKS
     → Arc<Mutex<AudioUniformSnapshot>>                                ✓ WORKS
        ▼
   scheduler tick (60 Hz): MutexAudioPull → state.latest_audio_snapshot ✓ WORKS
     → TauriIpcEmitter.emit("audio_uniform_snapshot_changed", payload)  ✓ WORKS
        ▼
   webview: audioSnapshotStore.listen(...)                             ✓ WORKS
     → TopBar VU/BPM, AudioReactivity 4 band tiles, BpmRing, BeatDots   ✓ WORKS (live)
     → AudioReactivity 48-bar FFT / AudioUniforms 64-bar Spectrum       ~ SYNTHETIC (post-MVP)
```

**The only broken link is the macOS system-audio tap's _read_ stage.** Everything
downstream of capture — analyser, the `Arc<Mutex>` seam, the scheduler tick, the
IPC bridge, and the webview render — is proven end-to-end (a
`CHROMATICITY_AUDIO_DIAG` round-trip confirmed the webview receives exactly what
the engine emits). With a microphone or a virtual loopback selected, the live
meters and band tiles work today.

### The macOS tap fix (in progress)

The tap *creation* is correct (global initializer + auto-start + private
aggregate). The corruption is cpal's **AUHAL `AudioUnit`** read stage inserting an
`AudioConverter` negotiated against the aggregate's format rather than the tap's
true `kAudioTapPropertyFormat`. The fix replaces that read stage — **for the
loopback path only** — with a direct `AudioDeviceCreateIOProcIDWithBlock` `IOProc`
via the already-transitive `objc2-core-audio`. Full decision record:
`docs/adr/ADR-022-system-audio-capture.md` → *Amendment (2026-06-03)*; patch
ledger: `third_party/cpal/UPSTREAM-VERSION.md`.

Until that lands and passes the sine probe on hardware, selecting **System Audio
(built-in)** on macOS degrades to the default input; use the microphone or a
virtual loopback to capture system output.

## 7. Troubleshooting

- **Consent prompt re-appears every build** → you're not using the harness (or the
  dev keychain was deleted). Re-run `dev-run-macos.sh`; the stable identity fixes it.
- **`codesign` can't find the identity** → the dedicated keychain isn't in the
  search list; re-running the script re-adds it. Inspect with
  `security find-identity -p codesigning ~/Library/Keychains/chromaticity-dev.keychain-db`.
- **No `cb=` lines** → the app is blocked on the consent prompt (grant it) or the
  selected source delivers nothing.
- **Meters flat with the tap selected on macOS** → expected; the tap read is under
  repair (§6). Switch to mic / virtual loopback.
- **`dist/.gitkeep` shows as deleted after a build** → `npm run build` clobbers it;
  the harness restores it, but if you build manually run
  `git checkout -- dist/.gitkeep`.

## 8. Related

- `docs/adr/ADR-022-system-audio-capture.md` — capture decision + the 2026-06-03
  AUHAL→IOProc amendment.
- `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` — the IPC bridge contract.
- `third_party/cpal/UPSTREAM-VERSION.md` — the vendored-cpal patch ledger.
- `release-eng/pre-alpha-smoke-checklist.md` §E — the human smoke-test steps.
- `docs/06-audio-engine.md` — the audio-engine baseline + fallback policy.
