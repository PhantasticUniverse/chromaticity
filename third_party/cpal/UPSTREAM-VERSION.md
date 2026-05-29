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

## The Chromaticity patch (the only deviation from upstream)

One file is modified vs the verbatim 0.17.1 crate:
`src/host/coreaudio/macos/loopback.rs`.

**Problem.** Upstream constructs the macOS system-output tap with the
device-scoped initializer:

```rust
CATapDescription::initWithProcesses_andDeviceUID_withStream(
    alloc, &empty_processes, device_uid, 0,
)
// + setExclusive(true)
```

On macOS 14.4+ this initializer, combined with an empty exclusive process
list, creates a Core Audio process tap that succeeds, starts, and delivers
**all-zero buffers** — system audio is never actually captured. This is
the long-standing cpal loopback bug tracked upstream as
[RustAudio/cpal#329]. It reproduced 1:1 in Chromaticity's pre-alpha smoke
run (PR #515): the TCC consent prompt fired, capture "ran", but every
`READY` snapshot reported `rms = 0.0000`.

**Fix.** Use the **global** stereo tap initializer instead:

```rust
CATapDescription::initStereoGlobalTapButExcludeProcesses(alloc, &empty_processes)
```

An empty exclude-list taps every process that outputs audio, i.e. the whole
system mix. This is the initializer used by AudioCap, MiniMeters, and other
known-working macOS system-audio capture apps. The aggregate-device wrapper
and drift compensation are unchanged. `setExclusive` is dropped (the
"...ButExcludeProcesses" initializer already encodes exclude semantics), and
the now-unused device-UID helper (`Device::uid`) is retained verbatim under
`#[allow(dead_code)]` to keep the vendor diff to a single semantic change.

**Verification posture.** The fix is **hardware-gated**: whether a tap
delivers non-zero buffers can only be confirmed on real macOS hardware with
TCC consent granted (there is no public preflight API and CI runners have no
audio device). The API choice mirrors the reference apps; final confirmation
is an AO hardware re-test (play system audio, observe the Settings monitoring
meter and the AUDIO adapter pill go live). Windows (WASAPI
`AUDCLNT_STREAMFLAGS_LOOPBACK`) is unaffected by this patch and uses the
correct, working upstream path.

See `docs/adr/ADR-022-system-audio-capture.md` for the capture decision and
`engine/audio/src/cpal_source.rs` for the Chromaticity-side loopback routing.

[RustAudio/cpal#329]: https://github.com/RustAudio/cpal/issues/329

## What we vendored

The complete crates.io `cpal` 0.17.1 source needed to build it as a path
dependency: `src/` (all backends — the macOS patch is the only change),
`build.rs`, the normalized `Cargo.toml`, `examples/`, and the upstream
docs (`README.md`, `CHANGELOG.md`, `UPGRADING.md`, `LICENSE`). A reviewer
can diff `third_party/cpal/` against a fresh `cargo` download of cpal
0.17.1 and see exactly the `loopback.rs` change above and these added
provenance/marker files.

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
- Apart from the documented `loopback.rs` patch and the stripped cruft
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
  verbatim, re-apply the `loopback.rs` change, and bump the `Version` /
  `Snapshot date` fields above.
