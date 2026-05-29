<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-022: System-Audio Capture (Zero-Install Loopback)

## Status

Proposed

> The AO authorized the implementation work on 2026-05-29 (system-audio routing
> parity directive); formal acceptance lands at PR merge per the AO-only ADR
> approval convention in `CLAUDE.md` / `docs/29-cleanroom-legal-policy-baseline.md`.

## Date

2026-05-29

## Deciders

- Accountable Owner: Xavier Butler (@PhantasticUniverse)

## Context

Chromaticity is a live-visuals runtime; for most operators the reactive audio is
the **system output** the machine is playing (DJ / media / music apps), **not** a
microphone. Until now `engine/audio` bound the host **default input** device via
cpal 0.15 (`cpal_source.rs::try_default_input` → `host.default_input_device()` —
the microphone on macOS), and capturing system output required the operator to
hand-install a virtual loopback driver (BlackHole / Loopback / Soundflower) and
select it as an input. The Settings → Audio dropdown surfaced that as a hint.

A clean-room parity investigation (2026-05-29, observation of the reference's
published FAQ + declarative `Contents/Info.plist` only, per `docs/29`) established:

- **macOS — we are at parity, not behind.** The reference (Synesthesia 1.25.2)
  also has **no** zero-install system-audio capture on macOS: its FAQ says system
  audio "cannot be built-in … at this time" and directs users to install
  BlackHole / Loopback / Soundflower / Sound Siphon. Its bundle declares only
  `NSMicrophoneUsageDescription` + `NSCameraUsageDescription`, links CoreAudio but
  not ScreenCaptureKit, and floors at macOS 10.15.
- **Windows — we lagged.** The reference ships a first-party named **"Desktop
  Audio"** dropdown entry (whole-render-endpoint WASAPI loopback, zero
  third-party install, added v1.24). Chromaticity only ever called
  `default_input_device()`, never reaching a render endpoint.
- **The reference's bar is low (macOS) to medium (Windows).** A genuinely
  zero-install path on either platform therefore *exceeds* the reference, and on
  macOS it is a differentiator the reference does not have.

The AO's bar: **match or exceed** the reference and make it **ultra smooth** —
ideally zero-install. As of cpal 0.17.0 (2025-12-20) this is reachable on both
platforms behind one API, with all platform `unsafe` contained inside the
dependency (cpal / its `objc2-core-audio` shims), which satisfies the workspace
`unsafe_code = "forbid"` rule (`Cargo.toml`; the rule governs first-party code
only — the same precedent as our existing cpal use, documented at
`engine/audio/src/ingest.rs`).

## Decision Drivers

- Most reactive audio is system output; requiring a hand-installed driver fails
  the "ultra smooth" bar.
- `unsafe_code = "forbid"` is first-party-only; a dependency that exposes a SAFE
  public API may carry the platform `unsafe`. Raw `coreaudio-sys` / `objc2-*`
  sys-bindings would force a first-party `unsafe {}` block and do NOT qualify — so
  the binding must be a safe-surface crate (cpal) or a sidecar.
- MSRV is pinned to 1.85.0 (`rust-toolchain.toml`); a bump that needs a newer Rust
  is a separate `policy`-gated decision.
- The capability must **degrade gracefully** below its OS floor / on denied
  permission, never gate the whole application.
- Determinism is unaffected — `canonical_state_hash` does not observe the audio
  snapshot.
- Hardware-rig CI validation is impractical (no live audio bus / no TCC consent on
  GitHub Actions runners), mirroring the `EV-INTEGRATION-MATRIX` synthetic /
  compile-and-unit-test narrowing precedent.

## Considered Options

1. **Bump cpal 0.15 → 0.17 and capture from an OUTPUT device** — building an
   *input* stream on a device that does not support input (`!supports_input()`)
   makes cpal transparently construct the platform loopback path: a CoreAudio
   process-tap aggregate on macOS (> 14.6) or `AUDCLNT_STREAMFLAGS_LOOPBACK` on
   WASAPI (Windows). One cross-platform code path; `unsafe` stays in cpal.
   (Chosen.)
2. **Per-platform crates** — `wasapi-rs` (Windows, safe API, also offers
   per-process loopback) + `screencapturekit-rs` / `cidre` (macOS). Two backends,
   two dependencies, more surface; ScreenCaptureKit additionally needs the
   heavyweight Screen-Recording consent (wrong UX for an audio app).
3. **Swift / Objective-C sidecar helper** over IPC — a separate notarized binary
   owning the Core Audio tap calls. Largest architectural commitment; defers the
   `unsafe` entirely out of process but adds a build + notarization + IPC surface.
4. **Keep the virtual-loopback-only path** — status quo. Rejected: fails the
   zero-install bar and leaves the Windows lag open.

## Decision

Adopt **Option 1**.

### 1. Dependency bump

`engine/audio` bumps `cpal = "0.15"` → `cpal = "0.17"`. Cargo resolves **0.17.1**
as the latest version compatible with **MSRV 1.85.0** (0.17.2 / 0.17.3 require a
newer Rust), so **no `policy`-gated MSRV bump is required**. The bump swaps the old
`coreaudio-sys` / `bindgen` for the `objc2-core-audio` family and adds `alsa 0.10`;
`cargo deny check licenses bans sources` passes with no `deny.toml` change. Two
source adjustments: `StreamConfig.sample_rate` is now a `u32` (was a
`SampleRate(u32)` newtype), and `DeviceTrait::name()` is deprecated in favour of
`description()` / `id()` — isolated behind a single `#[allow(deprecated)]` helper
(`read_device_name`) because migrating the device-**id** derivation to the new
stable `id()` would change the persisted `settings.audio.input_device` id format
and the frontend device-id contract (a deliberate follow-up, not part of this bump).

**Vendored patch (post-smoke-run amendment).** The pre-alpha smoke run surfaced
that stock cpal 0.17.1's macOS process-tap initializer
(`initWithProcesses:andDeviceUID:withStream:` with an empty exclusive process
list) succeeds, starts, and then delivers **all-zero buffers** on macOS 14.4+ —
the long-standing [RustAudio/cpal#329]. `cpal = "0.17"` is therefore wired as a
vendored `[patch.crates-io]` path override at `third_party/cpal/` carrying a
single change to `src/host/coreaudio/macos/loopback.rs`: the **global**
`initStereoGlobalTapButExcludeProcesses:` initializer (the AudioCap/MiniMeters-
proven path) replaces the device-scoped one. The vendored crate is not a
workspace member, so the FFI `unsafe` containment is unchanged; Apache-2.0 needs
no `deny.toml` change and a path patch needs no `allow-git` widening (a git fork
would). Full provenance + diff rationale live at
`third_party/cpal/UPSTREAM-VERSION.md`. Windows (WASAPI loopback) is unaffected by
the patch.

[RustAudio/cpal#329]: https://github.com/RustAudio/cpal/issues/329

### 2. Capture path

`engine/audio/src/cpal_source.rs` adds `try_system_audio_loopback()`, mirroring
`try_default_input()` but resolving `host.default_output_device()` and a new
`CaptureDirection::OutputLoopback` that selects `default_output_config()`. The
shared `build_stream_against_device` then drives both directions identically
through the existing owner-thread + bounded-channel + FFI-seam `catch_unwind`
machinery. An idle loopback stream (nothing playing) produces no callbacks, which
surfaces as `IngestStatus::Pending` (a neutral warmup gap), never a failure — so
the `IngestNegotiator` does not demote an idle system-output source to silent.

### 3. Selection + routing

A `"system-audio"` preference sentinel (parallel to `"default"`) routes through the
existing `try_input_by_preference` → `with_preferred_device` plumbing, so no
runtime or negotiator change is needed; `app/shell` already feeds the persisted
`settings.audio.input_device` into `AudioRuntime::with_preferred_device` at startup.
The Settings → Audio input dropdown prepends a first-class **"System Audio
(built-in)"** entry (parallel to the reference's "Desktop Audio"); the BlackHole /
Loopback hint is demoted to a fallback note for older macOS / per-app capture.

### 4. Permission + OS floor

`app/shell/Info.plist` declares `NSAudioCaptureUsageDescription` (the system-audio
tap consent — the purple recording indicator, not the orange mic dot) **and**
`NSMicrophoneUsageDescription` (closing a latent gap: the entitlements declared
`com.apple.security.device.audio-input` but the mic usage string was absent). The
macOS floor for the built-in tap path is **14.6+** (cpal's gate); **the app-wide
minimum OS is NOT raised**. Below the floor, on denied consent, or with no output
device, `try_system_audio_loopback()` returns `CpalDeviceCapture` and
`try_input_by_preference` **degrades to the host default input** with a one-shot
warning rather than going silent. There is no public API to preflight tap consent,
so a persistent-silence health signal is the UI's only denial cue.

### 5. Permanent fallback

BlackHole **cannot be bundled** (GPL-3.0 vs `LicenseRef-Chromaticity-Proprietary`),
so the operator-installed virtual-loopback path stays a permanent fallback for
sub-floor OS / denied permission / per-app needs — which is exactly why OS-native
taps + WASAPI loopback are the only route to true zero-install.

## Consequences

### Positive

- Zero-install system-audio capture on macOS 14.6+ and Windows; matches the
  reference on Windows and **exceeds** it on macOS (where the reference still
  requires BlackHole). No first-party `unsafe`.
- Clean integration: a single `IngestBoundary` producer; downstream analysis,
  the `Arc<Mutex<AudioUniformSnapshot>>` seam (ADR-019), the scheduler-tick, and
  determinism are untouched.

### Negative / risks

- **macOS tap re-test pending (patched path).** The macOS TCC consent flow *was*
  exercised in the pre-alpha smoke run — the purple-dot prompt fired and capture
  ran — but every snapshot reported `rms = 0.0000`, surfacing the stock-cpal
  all-zero bug (cpal#329) that the vendored global-tap patch (§1) addresses. The
  patch mirrors the AudioCap/MiniMeters reference path, but whether it now
  delivers non-zero buffers can only be confirmed on macOS hardware with consent
  granted (no public preflight API; CI runners have no audio device, and the
  device-opening smoke tests stay `#[ignore]` to avoid a CI consent prompt). This
  AO hardware re-test on the patched path is the remaining gate.
- **Windows compile-only.** The WASAPI loopback path is unaffected by the patch
  and uses the correct upstream API, but is verified by `clippy` (windows-latest)
  only — no Windows hardware was exercised.
- New transitive deps (`objc2-core-audio` family, `alsa`) enter the graph;
  license-clean today, but expand the audit surface.
- A loopback device delivering a sample format outside `{f32, i16, u16}` (e.g.
  `i32`) currently errors and falls back to the default input; widening the format
  set is additive future work.

### Deferred (out of scope for this ADR)

- **Live re-bind without restart.** Switching the capture source still needs an app
  restart (at parity with the reference's manual Refresh). Seamless re-bind needs
  producer-thread restart + scheduler-tick coordination sharing the existing
  snapshot `Arc` — a separate wave.
- **Per-process loopback** (capture one app, not the whole mix) — Windows build
  20348+ via `wasapi-rs::new_application_loopback_client`; macOS via process-scoped
  `CATapDescription`. An EXCEED feature the reference lacks; needs hardware consent
  validation.
- **`default_output` device-picker** to capture a non-default output endpoint.

## Verification

- `cargo check / clippy -D warnings / fmt --check` clean; `cargo test -p
  chromaticity-audio` green (logic pins for the sentinel + `CaptureDirection`;
  device-opening loopback smoke tests `#[ignore]`).
- `cargo deny check licenses bans sources` clean (the vendored `third_party/cpal`
  is a `[patch.crates-io]` *path* source — not git/registry — so `sources` passes
  with no `allow-git` change; Apache-2.0 is pre-allowed); `cargo hakari generate`
  in sync (the patch re-unified cpal's Windows `windows` dep onto the shared
  `0.61.3`, reducing duplication — `workspace-hack` regenerated to match).
- `reuse lint` covers `third_party/cpal/**` (Apache-2.0) via `REUSE.toml`.
- Frontend `tsc` / `svelte-check` / Settings tests green.

## Evidence

`EV-INTEGRATION-MATRIX` (audio arm) — system-audio routing parity reached without a
hand-installed virtual loopback device. Related: `docs/06-audio-engine.md`,
`ADR-019-audio-uniform-ipc-bridge.md`, `ADR-011-rendering-backend.md` Sec.4,
`ADR-014-signing-notarization-auto-update.md` (entitlements / hardened runtime).

## Traceability

- `adr/ADR-019-audio-uniform-ipc-bridge.md` — the producer-thread audio seam
  (`AudioRuntime` → `Arc<Mutex<AudioUniformSnapshot>>`) this capture source feeds; the
  loopback `CpalSource` is an additional `IngestBoundary` implementor with no change to
  the snapshot contract.
- `adr/ADR-014-signing-notarization-auto-update.md` — hardened-runtime entitlements;
  `com.apple.security.device.audio-input` (in `release-eng/entitlements.plist`) plus the
  new `NSAudioCaptureUsageDescription` / `NSMicrophoneUsageDescription` Info.plist keys
  gate the Core Audio tap + mic TCC consent.
- `adr/ADR-011-rendering-backend.md` Sec.4 — macOS platform-interop posture the
  CoreAudio-tap path sits alongside.
- `adr/ADR-013-networking-defaults-and-security.md` — loopback-only / no-network posture;
  system-audio capture is local-only and adds no outbound surface.
- `26-evidence-artifact-registry.md` — `EV-INTEGRATION-MATRIX` (audio arm) remains the
  tagged surface; no new evidence row is added.
- `29-cleanroom-legal-policy-baseline.md` — the reference's system-audio behaviour was
  observed clean-room (published FAQ + declarative manifests only); BlackHole's GPL-3.0
  bars bundling, which is why OS-native taps are the only zero-install route.
- `30-mvp-support-matrix.md` — macOS + Windows scope that bounds the per-platform tap /
  WASAPI-loopback applicability.
- `06-audio-engine.md` — the §7.1 ingest baseline + Reliability/Fallback policy the
  loopback source's degrade-to-default-input path honours.
