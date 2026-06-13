<!--
SPDX-FileCopyrightText: 2026 Chromaticity
SPDX-License-Identifier: CC-BY-4.0
-->

# cpal Vendor Snapshot (patched)

- **Origin:** <https://github.com/RustAudio/cpal> (published on crates.io)
- **Direct source:** `cpal` 0.17.1 from
  `registry+https://github.com/rust-lang/crates.io-index`
- **Version:** 0.17.1
- **Snapshot date:** 2026-05-29
- **License:** **Apache-2.0** (single license; verbatim text at
  `LICENSES/Apache-2.0.txt`, mirrored from the crate's own `LICENSE`).
  - REUSE 3.2 mapping at `REUSE.toml`'s `third_party/cpal/**` override.
  - `deny.toml` `[licenses].allow` already covers `Apache-2.0` (no
    `deny.toml` change needed).
- **Status:** Production override, wired via the root `Cargo.toml`
  `[patch.crates-io]` `cpal = { path = "third_party/cpal" }`. The vendored
  crate is **not** a workspace member, so the workspace lints
  (`unsafe_code = "forbid"`, `clippy::unwrap_used`/`expect_used` denials) do
  not apply to it — the CoreAudio FFI `unsafe` stays contained inside the
  dependency, exactly as it is for the registry build.

## The Chromaticity patches (the only deviations from upstream)

Changes span the macOS system-output **tap** path only: `Cargo.toml` and
`src/host/coreaudio/macos/{loopback,mod,device}.rs`. The ordinary microphone /
line-in input path and every non-macOS backend are untouched. Seven focused
changes — the tap is now READ through a direct Core Audio IOProc (Patch 6), which
supersedes the earlier attempt to read it through cpal's AUHAL `AudioUnit`
(Patch 4):

1. Global tap initializer (the all-zero-buffer fix) — `loopback.rs`
2. Aggregate-device out-param soundness fix — `loopback.rs`
3. Tap auto-start flag — `loopback.rs`
4. Loopback config branch (skip `set_sample_rate`) — `device.rs` _(superseded by
   Patch 6 and now **unreachable on the loopback path**:
   `build_input_stream_raw` early-returns into the IOProc reader before this
   branch, so it is retained only as upstream-diff context, not live code)_
5. Planar-interleave read — `device.rs` _(factored into `read_loopback_buffers`
   and reused by the Patch 6 IOProc reader; the AUHAL input callback retains an
   equivalent branch for the — interleaved-only — mic path)_
6. Direct-IOProc loopback read (no AUHAL) — `loopback.rs` (`tap_stream_format`) +
   `device.rs` (`build_loopback_ioproc_stream`) + `mod.rs` (`StreamInner`
   AUHAL-or-IOProc + `IoProcController`) + `Cargo.toml` (objc2-core-audio block
   feature flags + `block2` dep)
7. Tap-creation OSStatus check — `loopback.rs` (`from_device`): check
   `AudioHardwareCreateProcessTap`'s status before `assume_init` (upstream
   discards it, reading an uninitialized tap id on failure — formal UB)

> **Important — these patches are not the full fix.** Patches 1–5 got the tap
> *creating*, *auto-starting*, and *delivering* data, but on-hardware probing
> (2026-06-03) showed the captured signal is still corrupted: a known 440 Hz /
> 0.5-amplitude sine returns a flat FFT, a ~21 dB crest (a sine is 3 dB), and a
> peak (0.64) *exceeding* the 0.5 source amplitude — byte/channel-stride
> misinterpretation. The residual root cause is **not** in these files: it is
> cpal's AUHAL `AudioUnit` read stage (`src/host/coreaudio/macos/mod.rs` +
> `device.rs::build_input_stream_raw` → `AudioUnitRender`), which inserts an
> `AudioConverter` negotiated against the aggregate's flattened format rather than
> the tap's true `kAudioTapPropertyFormat` (Apple TN2091). **Patch 6 below
> replaces** that read stage, **for the loopback path only**, with a direct
> `AudioDeviceCreateIOProcIDWithBlock` `IOProc` (no AUHAL) using the
> already-transitive `objc2-core-audio` bindings — implemented, adversarially
> reviewed, and **hardware-verified clean on 2026-06-03** (the 440 Hz / 0.5 sine
> probe passed: `raw_peak` exactly 0.5; AO-confirmed live system-audio
> reactivity in the GUI on 2026-06-05). See
> `docs/adr/ADR-022-system-audio-capture.md` → *Amendment (2026-06-03)*.
> Patch 5's planar-interleave logic is reused by the `IOProc` reader;
> patches 1–3 (tap + aggregate creation) are retained as-is.

### Patch 1 — global tap initializer (the all-zero-buffer fix)

**Problem.** Upstream constructs the macOS system-output tap with the
device-scoped initializer:

```rust
CATapDescription::initWithProcesses_andDeviceUID_withStream(
    alloc, &empty_processes, device_uid, 0,
)
// + setExclusive(true)
```

On current macOS this initializer, combined with an empty exclusive process
list, creates a Core Audio process tap that succeeds, starts, and delivers
**all-zero buffers** — system audio is never actually captured. This is
the long-standing cpal loopback bug tracked upstream as
[RustAudio/cpal#329]. It reproduced 1:1 in Chromaticity's pre-alpha smoke
run (PR 515): the TCC consent prompt fired, capture "ran", but every
`READY` snapshot reported `rms = 0.0000`.

**Fix.** Use the **global** stereo tap initializer instead:

```rust
CATapDescription::initStereoGlobalTapButExcludeProcesses(alloc, &empty_processes)
```

An empty exclude-list taps every process that outputs audio, i.e. the whole
system mix. This is the initializer used by AudioCap, MiniMeters, and other
known-working macOS system-audio capture apps. The drift compensation is
unchanged. `setExclusive` is dropped (the "...ButExcludeProcesses" initializer
already encodes exclude semantics), and the now-unused device-UID helper
(`Device::uid`) is retained verbatim under `#[allow(dead_code)]` to keep the
tap-init diff minimal.

### Patch 2 — aggregate-device out-param soundness fix

Right below the tap, upstream creates the aggregate device with:

```rust
let aggregate_device_id: AudioObjectID = 0;
AudioHardwareCreateAggregateDevice(props, NonNull::from(&aggregate_device_id));
// ...later: Device::new(aggregate_device_id)
```

`AudioHardwareCreateAggregateDevice` **writes** the new device id through that
pointer, but upstream derives it from a shared `&` of a non-`mut` binding.
Writing through a read-only-provenance pointer is UB, and the optimizer is
permitted to fold the later read of `aggregate_device_id` back to `0` — which
would yield a broken (id-0) aggregate device. It works in practice today only
because the opaque FFI call forces a reload. The patch binds it `let mut` and
takes `NonNull::from(&mut aggregate_device_id)`, matching the out-param idiom
already used elsewhere in this same file (`Device::uid`'s `&mut size` /
`&mut cfstring`, and `tap_obj_id`'s `MaybeUninit` + `assume_init`). Surfaced by
the PR 516 review; a one-line change that removes the UB from code we now own.

### Patch 3 — tap auto-start flag (`loopback.rs`)

**Problem.** Upstream builds the aggregate device with
`kAudioAggregateDeviceTapAutoStartKey = false`. With auto-start off, the tap never
begins capturing when the aggregate device starts, so the input callback fires with
**zero data** (observed: `cb=0` — the callback count never advances).

**Fix.** Set the key to `true`:

```rust
kAudioAggregateDeviceTapAutoStartKey -> NSNumber::initWithBool(alloc, true)
```

so the tap starts with the aggregate and buffers flow. This matches the AudioCap
reference aggregate configuration.

### Patch 4 — loopback config branch: skip `set_sample_rate` (`device.rs`)

**Problem.** `build_input_stream_raw` unconditionally calls
`set_sample_rate(self.audio_device_id, config.sample_rate)` — correct for a real
input device, wrong for the loopback case. The tap is a tap on the *system mix* and
delivers its **own** format (commonly 48 kHz stereo). Forcing the source output
device's sample rate / ASBD onto the tap aggregate desyncs the drift-compensated
tap clock and makes Core Audio misread the buffers as **mostly zeros with rare
spikes** (observed: raw peak ~+4.9 dBFS — an impossible f32 amplitude — with an
analysed RMS near -48 dBFS).

**Fix.** Add a loopback branch (taken when `!supports_input()`): build the
`LoopbackDevice`, query the **aggregate's** `default_input_config()` for the
effective config, and **skip** `set_sample_rate` on this path. The microphone path
is unchanged.

### Patch 5 — planar-interleave handling in the input callback (`device.rs`)

**Problem.** Upstream reads only `mBuffers[0]` from the `AudioBufferList`, assuming
an interleaved layout. A macOS Core Audio process tap delivers **non-interleaved
(planar)** float — one mono buffer per channel, `mNumberBuffers == channel count`.
Reading only `mBuffers[0]` captures a single channel at reduced power (observed:
music at ~-49 dBFS, raw peak 0.18).

**Fix.** When `mNumberBuffers > 1` on the F32 path, transpose the planar mono
buffers into a short-lived interleaved `interleave_scratch` (`scratch[frame *
channels + ch] = planar[frame]`) and hand that to the downstream `Data` contract;
fall back to the single-buffer read otherwise. `effective_channels` (from the
aggregate config, Patch 4) replaces the hard-coded channel count in the
frames-per-buffer math.

> **Note.** Patches 4–5 (AUHAL era) made the tap deliver *non-zero,
> channel-correct-ish* data but did not eliminate the corruption — see the
> **Important** box above. Patch 4's AUHAL loopback branch is superseded by
> Patch 6; Patch 5's interleave logic is reused by the IOProc reader; Patches 1–3
> (tap + aggregate creation) stay as-is.

### Patch 6 — direct IOProc loopback read (replaces the AUHAL for the tap path)

**Problem.** Reading the tap aggregate through cpal's AUHAL `AudioUnit`
(`AudioUnitRender`) routes the planar-float tap stream through the AUHAL's
built-in `AudioConverter`, which negotiates against the aggregate's flattened
format rather than the tap's true `kAudioTapPropertyFormat` (Apple TN2091). On a
known 440 Hz / 0.5 sine that produced a flat FFT, a ~21 dB crest (a sine is 3 dB),
and a captured peak (0.64) **exceeding** the 0.5 source — byte/channel-stride
misinterpretation, not attenuation. This is structural to the AUHAL and cannot be
patched away inside the tap-creation code.

**Fix.** Read the tap aggregate with a **direct
`AudioDeviceCreateIOProcIDWithBlock` IOProc** (no AudioUnit, no implicit
converter) — the AudioCap/MiniMeters-proven mechanism:

- `loopback.rs::tap_stream_format()` reads `kAudioTapPropertyFormat` from the
  **tap object** for the authoritative format.
- `device.rs::build_loopback_ioproc_stream` installs a `block2::RcBlock` IOProc on
  the aggregate device, reads the `AudioBufferList` per the tap format via
  `read_loopback_buffers` (planar → interleaved scratch, pre-allocated so the
  realtime thread never reallocs), and bridges into cpal's `Data` /
  `InputCallbackInfo` contract. `build_input_stream_raw` early-returns to this for
  `!supports_input()`; the microphone path stays on the AUHAL.
- `mod.rs::StreamInner` becomes AUHAL-**or**-IOProc; `IoProcController` (with an
  explicit `StreamInner` `Drop`) reproduces the AudioCap teardown order: stop →
  `AudioDeviceDestroyIOProcID` → destroy aggregate → destroy tap.
- `Cargo.toml` enables the `block2` / `dispatch2` / `objc2-core-audio-types`
  features on `objc2-core-audio` and adds `block2` directly. All FFI `unsafe`
  stays in this vendored crate (not a workspace member).

Compile-clean, existing audio tests green, adversarially reviewed (unsafe
soundness + realtime safety + AudioCap-correctness), and **hardware-verified**
(see below).

**Verification.** Whether a tap delivers *non-zero, undistorted* buffers can
only be confirmed on real macOS hardware with TCC consent granted (there is no
public preflight API and CI runners have no audio device). The authoritative
probe is a known **440 Hz / 0.5-amplitude sine** played through the system
output (pass = peak ≤ 0.5, crest ~3 dB, a single 440 Hz FFT bin,
rms ≈ -9 dBFS). The 2026-06-03 re-test on the earlier patched (AUHAL) path
**failed** that probe — which is why the read stage moved to the direct
`IOProc`. The IOProc path then **passed** the same probe on hardware later
that day (`raw_peak` exactly 0.5, clean single-bin FFT; recorded in CLAUDE.md
at commit `c3e2f65`), and the AO confirmed live system-audio reactivity in the
GUI on 2026-06-05. The stable-signing dev harness in
`docs/dev/macos-desktop-run-and-audio-diagnostics.md` keeps TCC consent
persistent across rebuilds so regressions can be hardware-re-probed without a
fresh prompt each cycle. Windows (WASAPI `AUDCLNT_STREAMFLAGS_LOOPBACK`) is
unaffected by these patches and uses the correct, working upstream path.

See `docs/adr/ADR-022-system-audio-capture.md` for the capture decision and
`engine/audio/src/cpal_source.rs` for the Chromaticity-side loopback routing.

[RustAudio/cpal#329]: https://github.com/RustAudio/cpal/issues/329

### Patch 7 — tap-creation OSStatus check (`loopback.rs`)

**Problem.** Upstream's `LoopbackDevice::from_device` discards the
`AudioHardwareCreateProcessTap` return status and calls
`MaybeUninit::assume_init()` unconditionally. If tap creation fails (capture
consent denied, macOS below the tap API floor), that reads an **uninitialized**
`AudioObjectID` — formally UB — and then constructs the aggregate device
against a garbage tap id, surfacing as confusing downstream failures instead of
a clean build error.

**Fix.** Bind the `OSStatus`, route it through the file's existing
`check_os_status(..)?` idiom (the same shape as the
`AudioHardwareCreateAggregateDevice` call directly below), and only
`assume_init()` after `noErr`. Failure now surfaces as a recoverable
`BuildStreamError`, which `engine/audio`'s `try_system_audio_loopback` already
maps to the documented degrade-to-default-input path (never silent).

## What we vendored

The complete crates.io `cpal` 0.17.1 source needed to build it as a path
dependency: `src/` (all backends — the macOS `{loopback,device}.rs` patches are
the only changes), `build.rs`, the normalized `Cargo.toml`, `examples/`, and the
upstream docs (`README.md`, `CHANGELOG.md`, `UPGRADING.md`, `LICENSE`). A
reviewer can diff `third_party/cpal/` against a fresh `cargo` download of cpal
0.17.1 and see exactly the five `{loopback,device}.rs` changes above and these
added provenance/marker files.

## What's NOT included

Registry/VCS extraction cruft stripped from the snapshot: `.cargo-ok`,
`.cargo_vcs_info.json`, the upstream `.github/` CI tree, `.gitignore`,
`Cargo.toml.orig`, and the upstream `Cargo.lock` (a path dependency
resolves against the workspace-root lockfile, not its own).

## Why vendor (not a git fork)

A `[patch.crates-io]` vendored path keeps the override fully in-tree and
reproducible offline, and needs **no** `deny.toml` `[sources]` change. A git
fork would require widening `allow-git` (currently `[]`) to permit a
non-crates.io source — a clean-room/supply-chain regression — plus a live
external repository the build would depend on. Vendoring matches the
existing `third_party/` precedent (lygia, Syphon-Framework, hg_sdf).

## Upstream-provenance verification (clean-room hygiene)

- The snapshot was copied from the local `cargo` registry cache of the
  crates.io-published `cpal` 0.17.1 — no Synesthesia intermediation, no
  third-party mirror.
- Apart from the documented `loopback.rs` patches and the stripped cruft
  above, the tree is bit-for-bit the published crate.
- Per `docs/29-cleanroom-legal-policy-baseline.md`, source tooling MUST NOT
  parse `Contents/` or `chromaticity-planning/` — neither was consulted for
  this vendor.

## Future updates / upstream tracking

- When upstream cpal ships a release that fixes #329 natively, the right
  move is to **drop this vendor** entirely: remove `third_party/cpal/`, the
  `[patch.crates-io]` entry, and the `REUSE.toml` override, then bump the
  `engine/audio` cpal requirement to the fixed version. That is the
  preferred end-state — this vendor exists only to carry the one patch.
- To bump within the patched-vendor model: re-vendor the new crate version
  verbatim, re-apply the `{loopback,device}.rs` changes, and bump the `Version` /
  `Snapshot date` fields above.
