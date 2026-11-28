---
doc-type: reference
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Settings Reference

Settings is where operators bring up the audio device, configure the
output pipeline, register storage paths, and review the entitlement
surface. The view is laid out as four operator-bring-up sections at
the top, followed by the General toggles, the Hotkeys reference, and
the Theme switcher.

## 1. Audio Device

The Audio Device section enumerates host audio devices through the
audio adapter's input and output pickers. The fields surfaced are:

- Input device — the microphone or capture interface feeding the
  analysis pipeline.
- Output device — the playback target the runtime mixes to.
- Sample rate — typically 48 kHz on macOS, 44.1 kHz on Windows
  consumer hardware.
- Latency hint — `low`, `balanced`, or `high`. Lower hints reduce the
  wallclock between an audio event and its visual response, at the
  cost of buffer-underrun risk under CPU pressure.
- Channel layout — the input channel mapping used by the mono mix-down.
- Input gain — pre-AGC trim in dB.
- Live level / peak meter — confirms the chosen device is producing
  signal before scene launch.

Input device selection, DJ stereo-pair routing, and pre-AGC input gain
**hot-swap live without restarting Chromaticity or interrupting stage output**
(backed by `AppState::hot_swap_audio_input` and lock-free atomic gain/pair handles).

## 2. Graphics and Output

The Graphics section covers everything between the renderer and the
display:

- Resolution — preset list (1080p, 1440p, 4K, …) plus a custom
  width × height entry. The custom entry is gated by the
  `render.custom-resolution` capability.
- Render scale — fractional under-sample for performance-budget
  bring-up; defaults to 1.0.
- Target FPS — the render-loop ceiling. Lower targets free GPU
  headroom for heavy multipass scenes.
- V-Sync — when enabled, the present pacing is tied to the display's
  refresh; when disabled, the renderer free-runs at the FPS target.
- HDR target — capability-gated; reserved for V2 hardware that
  exposes a 10-bit HDR swap chain.
- Display assignment — selects which physical display the output
  window paints to in multi-display setups.

## 3. Storage

The Storage section pins the filesystem locations the runtime reads
and writes:

- **Media folders** — directories searched for `IMAGE_PATH`, video
  samplers, and the per-scene `images/` symlink targets.
- **Custom scene folders** — extra directories merged into the
  library results alongside `<UserDataRoot>/Scenes/`. See
  [Library Workflow](library-workflow.md) for how merging works.
- **Backups path** — destination for crash-restore snapshots and
  show-bundle exports, with a configurable retention window.
- Cache cleanup *(planned)* — reserved buttons to clear the thumbnail
  cache and the shader-compile cache once disk-backed caches ship
  (currently displayed as disabled in Settings).

Each path field has a Browse button that invokes the native
folder-picker through the shell IPC layer.

## 4. Network and adapter defaults

Chromaticity ships network-conservative by default — every adapter
binds to loopback unless the operator explicitly opts in. The defaults
matter because OSC and NDI carry no built-in authentication:

| Adapter | Default bind | Opt-in path |
| --- | --- | --- |
| OSC inbound | `127.0.0.1` (loopback) | Operator selects an interface or `0.0.0.0` |
| OSC outbound | `127.0.0.1` target | Operator sets per-subscriber target IP |
| NDI input | local subnet discovery | Operator restricts to allowlist |
| NDI output | local subnet advertise | Operator sets local-host-only or denylist |
| Syphon and Spout | local-host only | n/a (protocol constraint) |

Per-mapping source-IP allowlists honour CIDR notation
(`192.168.1.0/24`) so a venue-Wi-Fi setup can trust a single subnet
without opening every interface. Inbound traffic is rate-limited at
1000 messages per second per adapter (hard cap) and 100 messages per
second per `(adapter, route)` pair (operator-floor default).

!!! warning "Changing a network setting may expose Chromaticity to the network"

    Every override on this surface carries the explicit warning above.
    Overrides are persisted per show, so exporting a `.chromaShow`
    bundle reproduces the operator's network posture on the
    destination machine.

See [OSC Routes](osc-routes.md) for the route taxonomy and payload
hardening defaults the OSC adapter enforces.

## 5. Entitlement and capabilities

Settings surfaces the active entitlement token and the resolved
capability bitmask. Chromaticity's entitlement model is data-only —
the runtime evaluates per-capability flags against a single in-memory
`EntitlementSet`; there is no runtime concept of an "edition".

The following capabilities ship unconditionally free in every install,
including with no token present:

- Pause and transport control.
- Hard-cut transitions (transitions-off).
- Playlist navigation (next, previous, select).
- Preset recall and Snap slots.
- Scene export (for backup or hand-off).
- OSC and MIDI route discovery messages.
- Camera input (`io.camera`).

Capabilities that gate behind a valid token include the watermark
removal (`render.watermark.off`), the OSC and Syphon I/O surfaces,
the in-app authoring environment (`authoring.ide`), Shadertoy and
ISF import (`authoring.import`), scene editing (`authoring.edit`),
and the custom-resolution surface (`render.custom-resolution`).

## 6. Theme

The Theme switcher cycles among the three shipped themes (Pulse,
Atelier, Console). Themes are aesthetic options — they swap colour
palettes and surface treatments without changing layout or workflow.

---

<!-- Clean-room authored from src/lib/views/Settings.svelte (W21 PR1 layout commentary), docs/adr/ADR-013-networking-defaults-and-security.md sections 1-6, docs/adr/ADR-009-activation-and-entitlements.md sections on capability namespace + explicitly-free, and integrations/osc-adapter/src/defaults.rs; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
