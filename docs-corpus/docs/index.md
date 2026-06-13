<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Welcome to Chromaticity Help

This is the in-app help corpus shipped inside the Chromaticity desktop
runtime. Pages here are authored clean-room from chromaticity's own
ADRs and development standards — none of this content derives from any
reference application's embedded documentation.

The corpus is delivered as:

- A `mkdocs` source tree under `docs-corpus/docs/`.
- A static-HTML build emitted to `dist/help/` and bundled into the
  Tauri binary via `app/shell/tauri.conf.json` `bundle.resources`.
- A `Help` view (`src/lib/views/Help.svelte`) that surfaces the
  bundled HTML pages through Tauri's `convertFileSrc` resource path.

Use the navigation sidebar to browse the corpus: start with the
[Quickstart](quickstart.md) onboarding walkthrough, then the Guides
([Perform Mode](perform-mode.md), [Scene Workflow](scene-workflow.md),
[Library Workflow](library-workflow.md),
[Audio Reactivity](audio-reactivity.md),
[Livestreaming](livestream.md),
[Importing Shadertoy Scenes](import-shadertoy.md), and
[Best Practices](best-practices.md)) and the Reference pages
([Hotkeys](hotkeys-reference.md), [OSC Routes](osc-routes.md),
[Uniforms](chroma-uniforms.md), [Settings](settings-reference.md),
[System Requirements](system-requirements.md), and
[Supported Codecs](supported-codecs.md)).
