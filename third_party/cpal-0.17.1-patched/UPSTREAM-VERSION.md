<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: cpal contributors
-->

# Upstream Provenance — `cpal` v0.17.1 (Patched)

- **Origin:** <https://github.com/RustAudio/cpal>
- **Upstream Version:** `0.17.1`
- **License:** `Apache-2.0` (canonical text at `LICENSES/Apache-2.0.txt` and `third_party/cpal-0.17.1-patched/LICENSE`)
- **Chromaticity Patch Summary:**
  - `src/host/coreaudio/macos/loopback.rs`: sets `CATapMuteBehavior::Unmuted` on macOS CoreAudio Process Tap loopback streams so system-audio capture does not mute host speakers during live DJ sets.
