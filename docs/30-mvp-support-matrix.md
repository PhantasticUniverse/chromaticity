# MVP Support Matrix and Benchmark Profiles

## Purpose
Define the supported environment envelope for MVP so performance, portability, and integration evidence is reproducible.

## Support Classes

### Operating Systems (MVP)
| Class | Scope | GPU API stack |
|---|---|---|
| macOS-MVP | Current major and previous major macOS release on Apple Silicon | wgpu -> Metal per `adr/ADR-011-rendering-backend.md` |
| Windows-MVP | Current major and previous major Windows release on supported desktop GPUs | wgpu -> DirectX 12 (primary) / Vulkan (fallback) per `adr/ADR-011-rendering-backend.md` |

### GPU Classes (MVP)
Each GPU class is pinned to an explicit minimum-chip list so SLO-03 and `EV-PERF-BUDGETS` measurements are reproducible. GPU classes hold through the MVP support window; any uplift requires a phase-local waiver and a re-baseline of the fixture subset.

| Class | Role | macOS minimum | Windows minimum | Notes |
|---|---|---|---|---|
| GPU-A | Baseline compatibility (SLO-03 primary measurement target) | Apple M1 integrated GPU (8-core) | Intel Iris Xe (Tiger Lake / Alder Lake, DG2 Alchemist A350 or later), **or** AMD Radeon 780M (Phoenix), **or** AMD Radeon 890M (Strix Point) | Baseline-profile-fixture-subset in `34-golden-corpus-definition.md` is measured on this class. Devices below these thresholds are out of MVP support. |
| GPU-B | Target performance profile | Apple M1 Pro / M2 / M2 Pro / M3 / M3 Pro | NVIDIA GeForce RTX 3060 (Ampere, 12 GB), **or** AMD Radeon RX 6600 XT, **or** Intel Arc A580 | Drives `EV-PERF-BUDGETS` performance-profile. |
| GPU-C | Stress / headroom profile | Apple M1 Max / M2 Max / M3 Max / M2 Ultra / M3 Ultra | NVIDIA GeForce RTX 4070 or higher, **or** AMD Radeon RX 7800 XT or higher | Drives stress-profile soak and reconnect stress. |

GPU class selection and multi-GPU fallback is governed by `adr/ADR-011-rendering-backend.md` Sec.7; `EV-GPU-SELECTION` in Phase 4d validates the pinning policy and fallback behavior on machines with more than one GPU. When a test bed does not meet the minimum for its declared class, the run is excluded from evidence rather than downgraded silently.

### Audio Device Classes (MVP)
| Class | Scope |
|---|---|
| AUD-1 | Built-in device input/output |
| AUD-2 | USB audio interface (2+ channels) |
| AUD-3 | Virtual/system-audio loopback path for supported platforms |

### Media Path Classes (MVP)
| Class | Scope |
|---|---|
| MED-1 | Static image and local video ingest |
| MED-2 | Camera capture source |
| MED-3 | External texture feed (Syphon on macOS, Spout on Windows) |
| MED-4 | NDI input/output (post-MVP, design-only at MVP freeze) |

### Controller Classes (MVP)
| Class | Scope |
|---|---|
| CTRL-1 | MIDI controller with reconnect drills |
| CTRL-2 | OSC show-control client over local network |

### Output Surface Classes (MVP)
| Class | Scope |
|---|---|
| OUT-1 | Fullscreen, multi-display placement, constant vs scene-resolution output |

### Capability Presets ("Editions")

**MVP ships a single edition (Core).** The runtime uses a **capability-bitmask substrate** (`adr/ADR-009-activation-and-entitlements.md`); at MVP, only the `render.watermark.off` capability is exercised (on/off based on token presence). The "editions" below are marketing-only curated presets over the capability bitmask; the runtime has no edition concept. Activating per-capability gating post-MVP does not require architecture changes (adapter-boundary enforcement hooks are wired during Phase 4a) - it is a data change to the issued tokens.

#### MVP Edition (Core)

| Capability | Scope at MVP |
|---|---|
| Parity-Core FR/NFR | all in scope |
| Media classes | `MED-1` (files), `MED-2` (camera), `MED-3` (Syphon/Spout) |
| Controller classes | `CTRL-1` MIDI, `CTRL-2` OSC (full Chromaticity route surface per `08-io-integrations.md`; rate-limited per `adr/ADR-013-networking-defaults-and-security.md`) |
| Output | `OUT-1` fullscreen/multi-display/constant-or-scene-resolution |
| Watermark | on when no valid entitlement token is present; off when any valid token is present |

OSC route partitioning between Core and Advanced is retired at MVP; all OSC routes in `08-io-integrations.md` are Core-available, subject to the network-hardening defaults in `adr/ADR-013-networking-defaults-and-security.md`.

#### Post-MVP Editions (Design-only at MVP freeze)

These editions and their entitlement tokens are specified in `adr/ADR-009-activation-and-entitlements.md` and reserved for post-MVP rollout. No Phase 4 evidence row is required against tiered capabilities at MVP freeze. Tokens below must remain parseable by the MVP runtime so that post-MVP tokens issued to pre-existing installs are forward-compatible.

| Preset | Additional capabilities beyond Core | Required capability strings in `entitlement-v2` token |
|---|---|---|
| Advanced (post-MVP) | `MED-4` NDI input/output, independent OSC input/output gating, Syphon/Spout independent in/out gating, `_Reactivity` and `_Audio_Speed` meta controls (`36-reactive-symbol-catalog.md` Sec.3.6), custom output resolution, extended audio-variable OSC output namespace | `io.ndi.input`, `io.ndi.output`, `io.osc.input`, `io.osc.output`, `io.syphon.input`, `io.syphon.output`, `io.spout.input`, `io.spout.output`, `meta.reactivity`, `meta.audio-speed`, `render.custom-resolution` |
| Developer (post-MVP) | Advanced plus unrestricted script host, telemetry firehose, authoring IDE and scene import/edit | Advanced capabilities plus `authoring.ide`, `authoring.import`, `authoring.edit`, `script.host.unrestricted`, `telemetry.firehose` |
| Education (post-MVP) | Advanced, time-bounded | Advanced capabilities with shorter `expires_at` |

Note: "free" capabilities (pause, transitions-off, playlist navigation, preset recall, Snap slots, scene export) are unconditionally available and never require a capability string. The OSC route surface retains `favslot`/`favslots` wire names under `compat-syn-v1` for reference-app parity, but the end-user concept is **Snap slot**. See `adr/ADR-009-activation-and-entitlements.md` Sec.Explicitly free.

## Benchmark Profiles
| Profile | Environment Class Set | Use |
|---|---|---|
| `baseline-profile` | macOS-MVP + GPU-A + AUD-1 + MED-1 + CTRL-1 | Minimum accepted parity envelope |
| `performance-profile` | Windows-MVP + GPU-B + AUD-2 + MED-2 + CTRL-1/2 | Primary latency and frame budget target |
| `stress-profile` | Windows-MVP + GPU-C + AUD-3 + MED-3 + CTRL-2 | High-complexity soak and reconnect stress |

## Evidence Binding
- `EV-PERF-BUDGETS` must include runs for `baseline-profile` and `performance-profile`.
- `EV-PLATFORM-CONFORMANCE` must include at least one pass per OS/GPU/audio/media/controller/output class.
- `EV-AUDIO-ROUTING-MATRIX` must include class coverage for `AUD-1` through `AUD-3`.
- `EV-INTEGRATION-MATRIX` must include coverage for `CTRL-1`, `CTRL-2`, `MED-3`, and `OUT-1`. `MED-4` NDI coverage is post-MVP.
- `EV-ENTITLEMENT-ENFORCEMENT` at MVP validates watermark-on/off transitions only (token present vs absent, token expired, token revoked). Post-MVP tier capability gating uses the same evidence row with expanded scenarios.

## Change Control
- Any change to MVP support classes requires:
 - updated matrix row(s),
 - updated evidence coverage mapping,
 - impact note in roadmap and risk register.
