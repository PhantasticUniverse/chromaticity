---
doc-type: reference
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Supported Formats and Codecs

A reference for every format Chromaticity ingests and emits. The runtime
is deliberately narrow at MVP: a small, well-tested set of audio sample
formats, one native scene package shape (plus a one-way Shadertoy
import), and a small image set used for thumbnails. Video input,
recording, and signed show bundles are documented honestly with
their current wiring status — some are wired, several are deferred to
post-MVP follow-ups, and a few are reject-with-diagnostic by design.

The wiring status column uses three states:

- **Wired** — implemented and exercised by tests; available on the
  default build.
- **Synthetic** — surface implemented to its public contract, but the
  concrete backend is a stub or fixture pending a follow-up.
- **Deferred** — intentionally out of scope at MVP; tracked for a
  future wave.

---

## Audio input

Audio capture goes through our patched `cpal` crate (`third_party/cpal-0.17.1-patched`),
including zero-install **macOS Core Audio Process Tap** system-audio loopback
(capturing Rekordbox 7 `PC MASTER OUT` without BlackHole) and multi-channel DJ
Master-vs-Cue stereo-pair extraction (`DDJ-FLX10`, `DJM-A9`). cpal opens the
**host default input device** (or configured System Audio / DJ interface) at startup
and supports **zero-restart live device and gain hot-swap** from Settings (see
[Settings Reference](settings-reference.md)). The audio analysis thread
mixes the selected stereo pair down to mono, runs RMS / peak / 8-band FFT /
BPM, and publishes one `AudioUniformSnapshot` per analysis hop; see
[Audio Reactivity](audio-reactivity.md) for the analysis pipeline.

### Sample formats

cpal can deliver many sample formats; Chromaticity converts everything
to `f32` in the cpal callback before the analyser sees the frame.

| cpal `SampleFormat` | Status   | Conversion to `f32`            | Common host                            |
| ------------------- | -------- | ------------------------------ | -------------------------------------- |
| `f32`               | Wired    | Passthrough                    | macOS (CoreAudio), Windows (WASAPI)    |
| `i16`               | Wired    | `s / i16::MAX`                 | Linux (ALSA default on many devices)   |
| `u16`               | Wired    | `(s - 32768) / 32768`          | Windows (occasional WASAPI default)    |
| `i8` / `u8`         | Rejected | `AudioError::CpalDeviceCapture` reason at stream build |
| `i32` / `u32`       | Rejected | `AudioError::CpalDeviceCapture` reason at stream build |
| `i64` / `u64` / `f64` | Rejected | `AudioError::CpalDeviceCapture` reason at stream build |

Unsupported sample formats fail loudly at stream build with a
human-readable reason; the runtime does not silently transcode formats
outside the supported set.

The `i16` and `u16` conversions are exact divisions by integer
constants. `i16::MIN` maps to `-1.00003...` (slightly below `-1.0` from
two's-complement asymmetry) — the analyser tolerates the minority case
through its clamp.

### Channel count and sample rate

Chromaticity is host-format-tolerant on the input side:

| Property        | Behaviour                                                    |
| --------------- | ------------------------------------------------------------ |
| Channel count   | Accepts any device-reported channel count; mixed to mono in analysis |
| Sample rate     | Accepts the device default config; canonical analysis baseline is 48 kHz, hop cadence 375 Hz |
| Buffer size     | cpal default for the host; the analyser is non-blocking and tolerates back-pressure via the bounded channel between cpal and the analyser |

If the host has no audio subsystem (CI, headless, a Linux box without
ALSA), device enumeration collapses to an empty list and the analyser
falls back to a silent source rather than erroring.

### Default-input behaviour by platform

The "default input device" is platform-dependent and can surprise
operators on first run.

| Platform | Default input device                                              |
| -------- | ----------------------------------------------------------------- |
| macOS    | CoreAudio default input, plus zero-install **System Audio (Process Tap)** loopback (`ADR-022`) selectable in Settings → Audio without BlackHole. |
| Windows  | OS-selected default recording device (WASAPI).                    |
| Linux    | ALSA default; varies by host configuration.                       |

On macOS, select **System Audio** in Settings → Audio to react directly to
system output (including Rekordbox 7 `PC MASTER OUT`) with zero virtual drivers.
Virtual loopbacks (**BlackHole**, **Loopback**, **VB-Cable**) also work if desired.

---

## Scene packages

Chromaticity has one native scene format and a one-way Shadertoy
import. Other scene formats either round-trip-convert or are rejected
with a diagnostic.

| Format        | Status     | Direction          | Notes                                                                      |
| ------------- | ---------- | ------------------ | -------------------------------------------------------------------------- |
| `.chroma`     | Wired      | Read + author      | Directory package; canonical native scene format. Mandatory `scene.json` at the root. |
| `.chromaPack` | Deferred   | Read               | Zip-transport wrapper around a `.chroma` directory. Reserved; not yet wired. |
| Shadertoy URL | Wired      | Read (one-way)     | Converter wraps the Shadertoy body in a `.chroma` shell. See [Import Shadertoy](import-shadertoy.md). |
| `.synScene`   | Wired      | Read (conversion)  | Non-destructive `.synScene -> .chroma` importer wired in **Import** (`commands/import.rs`). |
| ISF (`.fs`)   | Reject     | Reject             | Imports surface a diagnostic; native ISF execution deferred to V2.         |

### `.chroma` package layout

A `.chroma` package is a directory whose name ends in `.chroma`. The
loader rejects any other extension and any non-directory path. The
required entry is `scene.json` at the package root.

```
my-scene.chroma/
├── scene.json        # required, JSON root
├── main.glsl         # optional, fragment shader
├── script.js         # optional, scene-author script
└── images/           # optional, scene-author asset bundle
```

The wire format for `scene.json` is documented under the in-app
[Scene Workflow](scene-workflow.md) and
[Chroma Uniforms](chroma-uniforms.md) pages; key root keys include
`SCHEMA_VERSION`, `TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`,
`CATEGORIES`, `CONTROLS`, and `PASSES`. Unknown root keys are
preserved in an extensions map rather than rejected, so authoring
tools can add new metadata without breaking loaders.

### What is NOT imported

The Shadertoy import path is intentionally narrow:

- **Multi-buffer Shadertoy compositions** — out of scope for the V1
  import; a multipass-import pathway is a future-wave ADR.
- **Sound shaders** — Chromaticity is a visual runtime; Shadertoy
  sound-shader bodies are rejected.
- **VR mode (`mainVR`)** — rejected; no plausible auto-mapping into a
  single-pass `mainImage`.
- **ISF (`.fs`)** — reject-with-diagnostic at MVP per design.

See [Import Shadertoy](import-shadertoy.md) for the full converter
behaviour.

---

## Images

Image support is narrow because Chromaticity is a procedural-visuals
runtime, not an image-processing host. The current consumers are
scene thumbnails, Library cover images, and (for imported Shadertoy
scenes) iChannel texture binding slots.

| Image format | Status     | Used for                                       | Notes                                                          |
| ------------ | ---------- | ---------------------------------------------- | -------------------------------------------------------------- |
| PNG          | Wired      | Scene thumbnails, Library cover images         | Operator-supplied PNGs are loaded through the standard asset path. |
| JPEG         | Wired      | Library cover images, iChannel texture binding | Standard `.jpg` / `.jpeg`.                                     |
| GIF          | Deferred   | —                                              | No animated-GIF playback at MVP.                               |
| HEIC / WebP  | Deferred   | —                                              | Not in scope; convert externally.                              |
| Cubemaps     | Deferred   | —                                              | Shadertoy `mainCubemap` entrypoint is deferred to a future multipass-import pathway. |

Scene thumbnails are rendered live on the GPU via `render_scene_thumbnail`
(Metal/`wgpu`) with an in-memory LRU cache, falling back to a deterministic
placeholder only when a scene has no `main.glsl`.

---

## Video

Video input and output are partially scaffolded but **not wired
end-to-end at MVP**.

| Surface                | Status       | Notes                                                              |
| ---------------------- | ------------ | ------------------------------------------------------------------ |
| Camera capture (input) | Synthetic    | `CameraAdapter` trait + `NokhwaInputAdapter` surface ship; per-platform concrete bindings (`input-avfoundation` on macOS, `input-msmf` on Windows) are deferred. `bind` returns `DeviceNotFound`; `enumerate` returns an empty list until the deferred backend lands. |
| Webcam in scenes       | Deferred     | Shadertoy iChannel webcam mapping is reserved with a diagnostic; the body still compiles, but no live webcam feed binds at MVP. |
| Syphon input (macOS)   | Synthetic    | `SyphonInputAdapter` surface ships per ADR-021. Concrete cross-process consumption is gated behind the Wave 1c-β `Send`-trait reconciliation + directory observer follow-ups. |
| Syphon output (macOS)  | Synthetic    | `SyphonOutputAdapter` surface ships; `publish_syphon_output` real-impl is a deferred follow-up. |
| Spout (Windows)        | Deferred     | Not yet scoped; the Syphon adapter is macOS-only by design.        |
| Video file playback    | Wired (Mix)  | Local `.mp4` / `.webm` / image layers and Direct YouTube URL stream-in play live inside the **Mix** deck (`liveMediaStore`); native GPU texture decode for GLSL `syn_Media` samplers remains planned. |

### Camera pixel formats (planned)

The camera adapter surface declares the following pixel-format taxonomy
so the deferred backend wave can attach without breaking the public
contract. Byte order is named explicitly in every variant to avoid the
ambiguity that has historically broken multi-year regression cycles in
upstream camera codecs.

| Variant         | Layout                                                                |
| --------------- | --------------------------------------------------------------------- |
| `Bgra32`        | Bytes B, G, R, A in memory; 4-byte stride.                            |
| `Rgb24`         | Bytes R, G, B in memory; no alpha; 3-byte stride.                     |
| `Nv12`          | Planar Y' (full-res luma) + interleaved U/V at 4:2:0 chroma subsampling. |
| `Yuyv`          | Packed Y0, U, Y1, V repeating (4:2:2 chroma subsampling); stride `width * 2`. |
| `Mjpeg`         | Complete JPEG bitstream (SOI...EOI). Decoded to `Bgra32` / `Rgb24` at the render-side wiring (deferred). |

Syphon textures are pinned to `BGRA8Unorm` per ADR-021 — the format
universally supported across Syphon consumers on macOS.

---

## Recording and export

| Surface                   | Status     | Notes                                                                    |
| ------------------------- | ---------- | ------------------------------------------------------------------------ |
| `.chroma` scene export    | Wired      | Saving a scene from the in-app authoring environment is a free-tier capability. |
| `.chromaShow` show bundle | Synthetic  | Format harness ships: tar container with a flat `manifest.json` plus per-file SHA-256 + manifest SHA-256. Recovery / rollback semantics are Phase-4+ scope; bundle-level ed25519 signing is `EV-ENTITLEMENT-ENFORCEMENT` scope and layers on top of the checksum surface. |
| Output recording (video)  | Planned    | Crash-safe `Fmp4Muxer` + `ReplayBuffer` ship in `engine/runtime`; standalone hardware H.264/AAC packet encoding remains Phase 7a. |
| Output recording (audio)  | Planned    | System audio capture (macOS Core Audio Process Tap) is wired for live analysis and Twitch RTMPS; standalone file recording muxing remains Phase 7a. |
| Screenshot (PNG)          | Deferred   | Not yet wired through the rendering backend.                             |

### `.chromaShow` container shape

The bundle is a tar archive — not a zip — chosen on determinism
grounds: identical inputs in identical order produce byte-identical
output. The container carries:

- A flat `manifest.json` at the archive root listing every payload
  entry plus its SHA-256.
- Each payload at the path declared in the manifest.
- A self-elided manifest SHA-256 (computed over the canonical JSON
  of the manifest with the `manifest_sha256` field elided) for
  tamper detection on the file list itself.

Path-traversal defence is structural: tar entries that are not in the
manifest path-set fail at the entry-iteration boundary; paths with
`..` segments, absolute paths, and Windows drive prefixes are
rejected before archive emission. Compression, if ever wanted, layers
on top (`.chromaShow.gz`) as a transport-only concern that does not
perturb the format-conformance corpus.

---

## What this page does NOT cover

- **OSC / MIDI wire formats** — Open Sound Control and MIDI are
  control protocols, not codecs. See [OSC Routes](osc-routes.md) and
  Settings → MIDI for those surfaces.
- **GLSL dialects** — Shader-side format coverage (Shadertoy
  iChannel uniforms, native `chroma_*` symbols) is documented in
  [Chroma Uniforms](chroma-uniforms.md).
- **Render-output codecs** — There is no rendered-output codec
  surface at MVP; the live preview targets the in-app window, not
  a file. Output capture is a deferred follow-up.

---

<!-- Clean-room authored from engine/audio/src/cpal_source.rs module-level doc-prose, engine/scene/src/loader.rs slice-3a contract, engine/scene/src/show_bundle.rs Phase-3 wave PR 5 harness, integrations/camera-adapter/src/frames.rs ADR-020 pixel-format taxonomy, integrations/syphon-adapter/src/lib.rs ADR-021 surface, app/shell/tauri.conf.json bundle resources, docs/26-evidence-artifact-registry.md (EV-IMPORT-SHADERTOY / EV-IMPORT-ISF-REJECT / EV-SHOW-BUNDLE-RECOVERY / EV-ENTITLEMENT-FREE-FEATURES), and CLAUDE.md Live cross-seam dispatch section; never derived from Contents/ or chromaticity-planning/ per docs/29. -->
