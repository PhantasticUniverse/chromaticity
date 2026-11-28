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
- **Native `.chroma` Scenes:** Multi-pass GLSL shaders with JSON control schemas, sandboxed scripting, and live hot-reload.
- **Performance & I/O:** Dedicated **Perform** and **Mix** decks, MIDI learn and performance bindings, bidirectional OSC routing, Syphon/NDI live textures, and crash-safe fMP4 recording.

## Offline Documentation

All guides in this Help viewer are bundled directly inside Chromaticity so every reference page — from [Hotkeys](hotkeys-reference.md) to [Uniforms](chroma-uniforms.md) and [Supported Codecs](supported-codecs.md) — remains available in offline venue booths.
