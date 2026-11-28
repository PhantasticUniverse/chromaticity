---
doc-type: explanation
---

<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# About Chromaticity

Chromaticity is a cross-platform GPU shader performance instrument and visual mixer built in Rust, `wgpu`, Tauri, and Svelte 5.

## Core Architecture

- **Real-time Audio Analysis:** Low-latency FFT band extraction, RMS/peak metering, onset detection, and live zero-restart input hot-swap.
- **Native `.chroma` Scenes:** GLSL 4.50 shaders with JSON control schemas, sandboxed QuickJS scripting, and live Author IDE compilation.
- **Performance & I/O:** Dedicated **Perform** and **Mix** decks, multi-monitor Stage Output, live Twitch RTMPS streaming, MIDI learn and performance bindings, bidirectional UDP OSC routing, and fMP4 recording/replay infrastructure.

## Offline Documentation

All guides in this Help viewer are bundled directly inside Chromaticity so every reference page — from [Hotkeys](hotkeys-reference.md) to [Uniforms](chroma-uniforms.md) and [Supported Codecs](supported-codecs.md) — remains available in offline venue booths.
