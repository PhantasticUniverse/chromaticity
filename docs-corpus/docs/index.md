<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Welcome to Chromaticity Help

This is the in-app help corpus shipped inside the Chromaticity desktop
runtime. Pages here are authored clean-room from chromaticity's own
ADRs and development standards — none of this content derives from any
reference application's embedded documentation.

The Wave 1d-α PR 1 scaffold establishes the rails:

- A `mkdocs` source tree under `docs-corpus/docs/`.
- A static-HTML build emitted to `dist/help/` and bundled into the
  Tauri binary via `app/shell/tauri.conf.json` `bundle.resources`.
- A `Help` view (`src/lib/views/Help.svelte`) that surfaces the
  bundled HTML pages through Tauri's `convertFileSrc` resource path.

Future waves author the real corpus content: scene-format reference,
operator hotkey catalog, MIDI / OSC routing, troubleshooting guides,
and onboarding walkthroughs. The two pages shipped in PR 1 (this page
and [About](about.md)) are placeholders that validate the routing +
build pipeline end-to-end.
