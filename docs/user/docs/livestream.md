---
doc-type: howto
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Livestreaming Chromaticity

This page documents the operator pathway for routing the Chromaticity
render output into a livestreaming application such as OBS Studio, and
covers what is wired today, what is scaffolded but not yet operational,
and what is planned. Livestream pathing depends on a frame-publishing
back end (Syphon on macOS, Spout on Windows, NDI cross-platform);
several of those back ends are partial or future-wave work at the time
of writing.

## Direct Twitch RTMPS Broadcasting (Wired Today)

Chromaticity ships built-in **Twitch RTMPS/RTMP** live broadcasting directly from the **Mix** deck (`LiveBroadcast` controls) using hardware-accelerated `h264_videotoolbox` (macOS) or `libx264` via host `ffmpeg`, with automatic stream-key redaction and live audio capture on macOS. You can broadcast your multi-layer Mix composite (GPU shaders, webcam, local video/images, YouTube stream-in, and text overlays) straight to Twitch without needing OBS in between.

## External Frame-Sharing Bridges (Syphon / Spout / NDI)

**External GPU frame-sharing publish is not operational yet.** Chromaticity ships the
Syphon adapter scaffold and the engine-side bridge, but the
`publish_syphon_output` real implementation is a Wave 1c-β follow-up
that has not landed. Operators who attempt to add a Syphon Client
source in OBS today will not see a Chromaticity server on the bus,
because the chromaticity output adapter does not yet hand wgpu-rendered
textures into the framework's `publishFrameTexture:` call site.

This page captures the recipe operators will follow **once** that
real-impl lands. The directory + receiver scaffolding, the error
taxonomy, the operator-facing health surface, and the engine bridge
are all in place; what remains is the wgpu ↔ `IOSurface` extraction
plumbing on the publish side. The receiver side (consuming a Syphon
feed from another macOS app **into** Chromaticity as a video input)
is also scaffold-complete pending the same hardware-validation pass.

For the current state of the underlying integration work, see the
"Wave 1c-β follow-ups" section in the repository's project memory.
The honest follow-ups list at the bottom of this page tracks the
remaining work items.

## What is wired today

### macOS — Syphon

The Syphon I/O adapter (`integrations/syphon-adapter/`) compiles on
macOS and exposes the public surface locked by ADR-021:

- `SyphonInputAdapter` — for consuming frames published by other
  macOS apps on the Syphon bus into Chromaticity as a video input.
  Polling-consume model: the engine scheduler-tick pulls the most
  recent frame each tick; backlog frames are dropped on saturation.
- `SyphonOutputAdapter` — for publishing Chromaticity's render
  output to the bus. The type-shape contract (`bind` / `unbind` /
  `publish` / `health`) is wired; the framework-side
  `publishFrameTexture:onCommandBuffer:imageRegion:flipped:` call
  site is the deferred follow-up.
- `SyphonServerDirectory` — singleton discovery surface keyed off
  the `SyphonServerDirectory.sharedDirectory` Objective-C class
  method. The `NSDistributedNotificationCenter` observer that fires
  on server-add / server-update / server-remove notifications is a
  carried-forward follow-up; the directory snapshot path
  (initial enumeration) is wired.
- Texture binding namespace — input feeds bind under the
  `chroma_SyphonInput[N]` uniform namespace on the engine side;
  output servers register under an operator-overridable name
  template.

The texture pixel format is locked to BGRA8Unorm, which is the
universally-supported format across Syphon consumers including OBS's
Syphon plugin, Resolume, Madmapper, and VDMX.

### macOS — what OBS will see

Once the `publish_syphon_output` real-impl lands, OBS Studio with the
Syphon plugin installed will list "Chromaticity" (or the operator-
chosen name) under **Add Source → Syphon Client**. The plugin reads
the same `SyphonServerDirectory` Chromaticity registers against; no
additional wiring is required on the OBS side.

### Windows + cross-platform — current state

- **Spout (Windows)**: not yet integrated. The adapter does not
  exist; a future PR will add it under
  `integrations/spout-adapter/`. The cross-platform parity bring-up
  is tracked as a Wave 1c-β follow-up cluster sibling of the Syphon
  publish work.
- **NDI (cross-platform)**: scaffolded only. NDI is referenced in
  the planning corpus as the macOS-and-Windows fallback for
  operators who cannot route through Syphon or Spout, but no
  adapter crate exists yet.
- **Virtual webcam output (cross-platform)**: not planned for MVP.
  Operators on a single workstation should prefer Syphon (macOS)
  or Spout (Windows) once those ship; cross-machine routing should
  prefer NDI.

## Operator pathway — forward-looking recipe

The recipe below describes how operators will livestream Chromaticity
**once** the publish path lands. Until then, treat the workflow as a
preview of the intended operator flow; the commands and UI surfaces
named here may shift in detail.

### 1. Install OBS Studio and the Syphon plugin (macOS)

1. Install **OBS Studio** (28.0 or later recommended). Download from
   the OBS Studio website; the macOS build is a universal binary.
2. Install the **obs-syphon plugin**. Plugin packages exist for both
   Apple Silicon and Intel macOS; the plugin appears under
   `Sources → Add → Syphon Client` once installed.
3. Restart OBS so the plugin loads.

On Windows the equivalent will be the Spout plugin for OBS once
Chromaticity's Spout adapter lands; the configuration steps below
will map across with the source type renamed from "Syphon Client" to
"Spout".

### 2. Configure audio loopback

Chromaticity's audio input adapter binds the system's **default input
device** (microphone) on macOS via cpal, not the system audio output.
Capturing system audio for livestreaming requires a virtual loopback:

- **BlackHole** (free; Apple Silicon and Intel macOS).
- **Loopback** (Rogue Amoeba; commercial).
- **VB-Cable** (free; Windows and macOS).

Configure the loopback device as the system output (or as an aggregate
device that mirrors output to loopback and to physical speakers), and
point both Chromaticity's audio input and OBS's audio source at the
loopback. For first-launch verification of the audio bootstrap path,
see the audio reactivity reference page in this corpus.

### 3. Enable the Syphon output in Chromaticity

Once the publish path lands, the **Settings → Output** tab will
expose a Syphon Server toggle and an operator-overridable server
name. The default name follows the
`DEFAULT_OUTPUT_NAME_TEMPLATE` constant in the adapter (a
predictable template the operator can override per show). With the
toggle enabled, the `SyphonServerDirectory` will advertise
"Chromaticity" (or the chosen name) on the bus immediately; OBS will
pick it up without restart.

The output adapter exposes an operator-facing health surface
mirroring the camera adapter's health model — Bound, Disconnected,
Negotiating, or Faulted with an operator-readable diagnostic. The
TopBar adapter pills already render an OUT indicator that will read
this health value once wired; see the Perform Mode page for the
adapter-pill model.

### 4. Add the Syphon Client source in OBS

In OBS Studio:

1. Click the `+` button under the **Sources** panel.
2. Choose **Syphon Client**.
3. Give the source a name (this is OBS's internal label, separate
   from the Chromaticity server name).
4. In the source properties, the **Source** dropdown will list
   every Syphon server currently on the bus. Pick the Chromaticity
   server.
5. Confirm the preview pane shows live Chromaticity output.

Resolution, frame rate, and color space are negotiated through the
underlying `IOSurface` (BGRA8Unorm; the resolution matches the
Chromaticity render target). No additional configuration is required.

### 5. Confirm audio sync and round-trip latency

Both video (Chromaticity → Syphon → OBS) and audio
(system → loopback → OBS) flow into OBS as independent sources. Audio
sync is the operator's responsibility:

- In OBS, right-click the audio source and choose **Advanced Audio
  Properties**. The **Sync Offset** column shifts audio relative to
  video in milliseconds (positive values delay audio).
- A reasonable starting offset is 0 ms; tune by ear against beat-
  reactive scenes. Syphon adds negligible video latency
  (sub-millisecond on modern Apple Silicon); the loopback audio path
  is typically near-zero.

### 6. Stream key configuration and bitrate are OBS concerns

Chromaticity does not configure stream keys, RTMP endpoints, or
encoder bitrates; those live entirely in OBS's **Settings → Stream**
and **Settings → Output** tabs. Refer to the streaming platform's
documentation (Twitch, YouTube, Kick, etc.) for endpoint details.

## Honest follow-ups

The list below tracks the work needed to make this page's recipe
operational. Each item maps to a specific gap in the current
integration surface.

| Item | Status | Notes |
| --- | --- | --- |
| Syphon `publish_syphon_output` real-impl | Wave 1c-β follow-up | The `SyphonOutputAdapter::publish` method accepts a texture handle today; the engine-side wgpu ↔ `IOSurface` extraction and the framework-side `publishFrameTexture:onCommandBuffer:imageRegion:flipped:` call site are deferred. Once landed, OBS's Syphon Client will see the Chromaticity server. |
| `list_syphon_servers` `NSDistributedNotificationCenter` observer | Wave 1c-β follow-up | The directory snapshot path is wired; the live notification observer that fires on server-add / server-update / server-remove is deferred. Affects only operator-facing server-list freshness in Chromaticity's input picker; OBS-side discovery already works through the canonical framework. |
| Per-platform nokhwa concrete bindings | Wave 1c-β follow-up | Camera-adapter sibling work for the input side of the integration matrix. Not directly required for Syphon livestream output but tracked in the same cluster. |
| Syphon `Send`-trait reconciliation | Wave 1c-β follow-up | Thread-safety reconciliation between the framework's main-thread affinity and the chromaticity scheduler-tick thread. |
| Real hardware validation | Wave 1c-β follow-up | The current T2 corpus uses a mock framework trait; live-bus validation on AO dev rigs is deferred. |
| Spout (Windows) adapter | Future PR | The Windows-equivalent integration is not yet started. Once shipped, this page will gain a Windows recipe that mirrors the macOS flow with source-type substitution. |
| NDI adapter activation | Future PR | Scaffolded reference only at present. NDI's value is cross-machine routing (Chromaticity on one workstation, OBS on another over LAN); when the adapter lands, this page will gain a multi-machine recipe. |
| First-party video recorder | Future PR | A direct-to-disk video capture path internal to Chromaticity is an FAQ-surfaced operator request that depends on the same `publish_syphon_output` real-impl as a prerequisite (the encoder consumes the same `IOSurface` extraction). |

## Audio capture posture

Chromaticity binds the cpal default input device on macOS at startup
(microphone, not system audio output). The operator path for
capturing system audio into either Chromaticity (for audio-reactive
visuals) **or** OBS (for the stream) is a virtual loopback as listed
in step 2 above. There is no in-app system-audio capture path; this
is a platform constraint (macOS does not expose system audio to user-
space without a kernel extension or audio HAL plugin), not a
Chromaticity choice.

The audio loopback applies to **both** sides of the pipeline:

- Chromaticity reads the loopback as its audio input → audio-reactive
  visuals respond to the music being played.
- OBS reads the loopback as an audio source → the stream carries the
  music.

If the operator needs both at once, configure the loopback once at
the system level (aggregate device or BlackHole multi-output device)
and point both applications at it.

## Cross-references

- [Audio Reactivity](audio-reactivity.md) — for the audio input
  bootstrap path and the synthetic-fallback gate that drives
  visualizers when no live snapshot is present.
- [Perform Mode](perform-mode.md) — for the adapter-pill liveness
  model that will surface the Syphon output health in the TopBar.
- [Settings Reference](settings-reference.md) — for the output
  routing tile that will expose the Syphon Server toggle once the
  real-impl lands.

---

<!-- Clean-room authored against integrations/syphon-adapter/src/{lib,adapter}.rs module-level doc-prose + CLAUDE.md "Wave 1c-β follow-ups" + "Out-of-scope (AO-deferred)" sections + ADR-021 (Syphon I/O adapter) + the Wave 1c-α-2 audio bootstrap producer notes; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
