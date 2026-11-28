---
doc-type: reference
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# System Requirements

Concrete system-requirements reference for Chromaticity. The tables
below describe the minimum, recommended, and known-limited
configurations the runtime is engineered against. The figures are
pragmatic operator guidance rather than certified hardware floors —
deterministic performance on any specific rig depends on the active
scene, the audio analysis load, and which adapters are enabled.

## 1. Supported platforms

Chromaticity ships as a Tauri 2 desktop binary built against the Rust
2024 edition (MSRV 1.85.0). Today's bundle targets are:

| Platform                | Status              | Notes                                                                  |
| ----------------------- | ------------------- | ---------------------------------------------------------------------- |
| macOS 12 Monterey+ (Apple Silicon) | Supported           | Primary development target. Metal backend via wgpu.                    |
| macOS 12 Monterey+ (Intel x86_64)  | Pending Wave 2.10 Lane A | Build target exists; bundle + signing pipeline rides the Lane A widening. |
| Windows 10 (1809+) / Windows 11 (x86_64) | Supported           | DX12 primary, Vulkan fallback.                                         |
| Linux (x86_64, glibc ≥ 2.31) | Pending Wave 2.10 Lane A | Vulkan backend. Bundle pipeline + AppImage / `.deb` targets land with Lane A. |
| 32-bit x86 / ARMv7 / iOS / Android | Not supported    | No work planned.                                                       |

The Mac Intel and Linux entries above reflect the active widening
pending in Wave 2.10 Lane A — the engine and shell already compile
against both targets, but the signing / bundle / appcast pipeline
remains macOS-arm64 + Windows-x86_64 only at the Wave 1e-γ scaffold.

## 2. CPU and RAM floors

Audio analysis runs on a dedicated cpal capture thread and the FFT /
BPM pipeline is single-threaded; scene rendering runs on a wgpu device
thread. The CPU floor reflects the latter — comfortable headroom for
audio analysis is well below the floors below.

| Tier        | CPU                                                  | RAM   | Notes                                                              |
| ----------- | ---------------------------------------------------- | ----- | ------------------------------------------------------------------ |
| Minimum     | 4-core x86_64 / Apple Silicon, ≥ 2.0 GHz             | 8 GB  | Single-output workflow, lighter scenes, no multipass post-stack.   |
| Recommended | 6-core+ at ≥ 3.0 GHz; Apple M1 / M2 / M3 / M4        | 16 GB | Headroom for shader compile, library thumbnailing, OSC + MIDI together. |
| Comfortable | 8-core+; Apple M-series Pro / Max; Ryzen 7 / i7 / i9 | 32 GB | Heavy multipass scenes, ≥ 1440p output, parallel scene authoring.  |

The runtime forbids `unsafe` Rust workspace-wide and the release
profile builds with `lto = "fat"` and a single codegen unit; binaries
land in the small-low-hundreds-of-MB range and idle RAM use is
typically 200-400 MB before any scene is loaded.

## 3. GPU floors and adapter selection

Chromaticity selects a GPU adapter deterministically at launch per
ADR-011 §7. The selection helper (`engine/render::adapter_selection`)
ranks every enumerated adapter on the lexicographic tuple:

1. operator override (if a persisted adapter ID still matches);
2. adapter-type rank (Discrete > Integrated > Virtual > CPU > Other);
3. backend-rank (Metal on macOS, DX12 on Windows, Vulkan on Linux);
4. battery-friendly flag (flips Integrated above Discrete when set);
5. alphabetic name tie-break for full determinism.

| Tier        | GPU                                              | VRAM    | Notes                                                              |
| ----------- | ------------------------------------------------ | ------- | ------------------------------------------------------------------ |
| Minimum     | Integrated GPU with wgpu-supported backend       | shared  | Apple M-series, Intel Iris Xe, AMD Radeon iGPU, modern Linux integrated GPUs. |
| Recommended | Discrete GPU released 2020 or later              | ≥ 4 GB  | NVIDIA RTX 30/40 series, AMD RX 6000/7000 series, Apple M-Pro/M-Max. |
| Comfortable | Workstation discrete GPU                         | ≥ 8 GB  | Headroom for 4K output, heavy multipass, future HDR target work.   |

### Dual-GPU laptops

On laptops with both an integrated and a discrete GPU the default
posture is to pin the discrete adapter for performance. The persisted
operator override (Settings → Output → GPU dropdown) is honoured ahead
of the type-rank — set it when you want to pin a specific adapter
across launches. If the persisted adapter is no longer attached (an
unplugged eGPU, for example) the selector silently falls through to
the next-best adapter rather than refusing to launch.

A future widening will surface a power-source-aware battery preference
+ a TDR-history warning when the selected Windows adapter has reported
device-removed events in the last 24 hours; the policy plumbing is in
place, the UI banner is not yet wired.

## 4. Audio interface guidance

Audio capture binds the host's **default input device** by default on
each platform. On macOS that resolves to the built-in microphone (not
system audio output); routing system audio through Chromaticity
requires a virtual loopback driver (BlackHole, Loopback, or
equivalent) configured as the default input. On Windows the equivalent
is WASAPI loopback or Voicemeeter; on Linux, a PulseAudio /
PipeWire monitor source.

| Property      | Floor                       | Recommended                       |
| ------------- | --------------------------- | --------------------------------- |
| Interface     | Built-in input              | Class-compliant USB or Thunderbolt |
| Sample rate   | 44.1 kHz                    | 48 kHz                            |
| Bit depth     | 16-bit                      | 24-bit                            |
| Channel count | Mono (analysis is mono-mix) | Stereo                            |
| Buffer / latency hint | `balanced`          | `low` on capable hardware         |

The analysis pipeline mixes input down to mono before FFT / RMS / peak
extraction so multichannel interfaces are not necessary for visual
reactivity. The `latency_hint` field on the Audio settings (`low` /
`balanced` / `high`) trades audio-to-visual response latency against
buffer-underrun resilience under CPU pressure. Audio devices do not
hot-swap mid-show — changes apply at the next analysis-thread restart
(typically the next scene load).

## 5. MIDI device guidance

MIDI is provided by `midir 0.10` and the platform's native MIDI stack:
CoreMIDI on macOS, WinMM on Windows, ALSA seq on Linux. Any
class-compliant USB MIDI controller works without a vendor driver.

| Surface             | Supported                              | Notes                                              |
| ------------------- | -------------------------------------- | -------------------------------------------------- |
| USB MIDI controllers | Class-compliant, no vendor driver      | Pads, knobs, faders, transport surfaces.           |
| 5-pin DIN MIDI      | Through a class-compliant USB adapter   | Standard DIN-to-USB interfaces.                    |
| Bluetooth MIDI      | Apple BLE-MIDI on macOS                | Subject to OS pairing latency.                     |
| Network MIDI (RTP)  | Through OS-side virtual port           | Configured outside Chromaticity.                   |

What the adapter parses and surfaces today: Note On / Off, Control
Change (7-bit), Pitch Bend, Program Change, Channel Pressure,
Polyphonic Aftertouch, and 14-bit CC pairs. MPE is intentionally
deferred to V2 with a `diagnostic-fires-once` rejection. SysEx is
logged-only.

MIDI learn captures a single message at a time at this build; the
binding-to-control wire is a known follow-up.

## 6. Network

Chromaticity binds every network adapter to **loopback (127.0.0.1)**
by default per ADR-013. Exposing OSC or any other inbound adapter on a
routable interface is an explicit operator opt-in in the Settings
view, and overrides carry an explicit "may expose Chromaticity to the
network" warning.

| Property                  | Default               | Configurable in Settings |
| ------------------------- | --------------------- | ------------------------ |
| OSC inbound bind          | `127.0.0.1`           | Yes                      |
| OSC inbound port          | `9000` / UDP          | Yes                      |
| OSC outbound target       | `127.0.0.1`           | Yes (per subscriber)     |
| OSC outbound port         | `9001` / UDP          | Yes                      |
| Maximum OSC packet size   | 64 KiB                | No (hard cap)            |
| Maximum OSC bundle depth  | 4                     | No (hard cap)            |
| Per-route rate limit      | 100 msgs/sec          | Yes (floor 100)          |
| Per-adapter rate cap      | 1000 msgs/sec         | No (hard cap)            |
| NDI / Syphon / Spout      | Local subnet / host   | Local-only by protocol   |

No outbound network traffic occurs at startup without an explicit
operator action. Crash payloads are redacted before write, settings
files persist locally, and the updater endpoint (when configured)
talks only to the staging or production appcast URL pinned in the
Tauri config.

## 7. Storage

Persistent operator state lives in the platform-native per-user app
data directory through Tauri's `app.path().app_data_dir()` resolver:

| Platform | Directory                                                       |
| -------- | --------------------------------------------------------------- |
| macOS    | `~/Library/Application Support/app.chromaticity.desktop/`       |
| Windows  | `%APPDATA%\app.chromaticity.desktop\`                           |
| Linux    | `$XDG_DATA_HOME/app.chromaticity.desktop/` (typically `~/.local/share/app.chromaticity.desktop/`) |

Files written into that directory today:

| File                                  | Purpose                                                                  |
| ------------------------------------- | ------------------------------------------------------------------------ |
| `settings.toml`                       | Persisted Audio / Graphics / Storage / General settings, theme, OSC ports. |
| `settings.toml.malformed-<timestamp>` | Quarantined copy of an unparseable settings file. Hand-correctable.      |
| `crash-resume.bin`                    | Crash-resume snapshot for next-launch state restoration.                 |

Settings persistence uses TOML so operators can hand-correct a file
the runtime cannot parse; a malformed file is quarantined before the
next save so the damaged file is preserved for repair. New settings
fields land with `serde(default)` so an older `settings.toml` continues
to deserialise cleanly under a newer build.

Disk-space footprint at install is typically under 200 MB. The
thumbnail cache and the shader-compile cache live under the same app
data directory and can be cleared from Settings → Storage.

## 8. Display, input, and ergonomic guidance

| Property        | Floor                | Recommended                                         |
| --------------- | -------------------- | --------------------------------------------------- |
| Display size    | 1440 × 900           | 1920 × 1080+ for Perform; 2560 × 1440+ for Author. |
| Output displays | Single output        | Dedicated output display for visuals, separate operator display for chrome. |
| Pointer input   | Mouse or trackpad    | Trackpad gestures for parameter scrubbing; tablet stylus for swatch / curve work. |
| Keyboard        | Standard layout      | The 57-row hotkey catalogue assumes a standard layout; remappers work but are not validated. |

The application window opens at 1280 × 800 by default and is
resizable. Perform-class views are designed to fit ~1440 × 900 without
page-level scrolling; specific inner panels (long scene lists, the
hotkey catalogue) scroll within their container.

## 9. Known-limited configurations

These configurations are excluded by current engineering or platform
choice. Items marked "Pending" describe a known follow-up rather than
a refusal.

| Configuration                                | Status        | Notes                                                                  |
| -------------------------------------------- | ------------- | ---------------------------------------------------------------------- |
| macOS 11 Big Sur or earlier                  | Not supported | wgpu Metal backend baseline.                                           |
| Windows 7 / 8 / 8.1, 32-bit Windows          | Not supported | DX12 + Tauri 2 baseline.                                               |
| Linux distributions with glibc < 2.31        | Not supported | Bundle pipeline targets a modern glibc baseline.                       |
| Headless / no-GPU CI runners                 | Compiles only | Adapter selection returns `None` without a driver; the renderer refuses to start. |
| Dual-GPU validated against discrete + integrated hardware | Pending | Synthetic T2 corpus covers the decision tree; live dual-GPU validation is hardware-gated. |
| Crash-injection drill (`SIGKILL` / panic-in-render) | Pending | Restoration helpers shipped; full-injection harness is hardware-gated. |
| Power-source-aware battery preference UI     | Pending       | Selection policy plumbs the flag; OS-side power query + banner deferred. |
| TDR-history warning on Windows               | Pending       | Counter is plumbed in policy; warning surface deferred.                |
| Camera input on Windows / Linux              | Pending       | macOS path lands first; per-platform `nokhwa` bindings are a Wave 1c-β follow-up. |
| Syphon output                                | macOS only    | Protocol constraint; cross-platform alternative (Spout on Windows) is a future widening. |
| MPE MIDI                                     | Deferred (V2) | Rejection fires a one-time diagnostic.                                 |

For day-to-day troubleshooting, the
[Settings Reference](settings-reference.md) walks every operator
adjustment surface and explains what each field changes; the
[OSC Routes](osc-routes.md) page covers the inbound and outbound
network surface in full.

---

<!-- Clean-room authored from Cargo.toml MSRV / edition pins, rust-toolchain.toml channel, app/shell/tauri.conf.json bundle target list, docs/adr/ADR-011 §7 GPU selection clauses, docs/adr/ADR-013 networking defaults, engine/render/src/adapter_selection.rs ranking contract, integrations/osc-adapter/src/defaults.rs constants, integrations/midi-adapter/src/lib.rs midir 0.10 surface, engine/runtime/src/settings.rs TOML persistence + quarantine model, and app/shell/src/lib.rs settings.toml + crash-resume.bin path conventions; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
