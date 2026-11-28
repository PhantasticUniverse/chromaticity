<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# EV-ARCH-CONTRACT-BASELINE — Phase 0 Exit Checklist

**Evidence ID:** `EV-ARCH-CONTRACT-BASELINE`
**Gate role:** Phase 0 exit gate (`docs/26-evidence-artifact-registry.md` row
`EV-ARCH-CONTRACT-BASELINE`).
**Purpose:** Validate the frozen implementation-contract pack and ADR closure for the
seven gating architectural decisions (ADR-011 through ADR-018, excluding ADR-017 which
is not on the gating list for this row).
**Checklist date:** 2026-04-29.
**Accountable Owner:** Xavier Butler (`@PhantasticUniverse`).

## Scope

This checklist instantiates the per-ADR sign-off and contract-pack audit closure that
the registry row called for as the remaining `InProgress` work item. All seven gating
ADRs are merged on `main` and carry `Status: Accepted` in their respective files; this
artifact records the AO's contract-by-contract review and overall baseline sign-off.

The seven ADRs were imported into `docs/adr/` as a single normative-corpus promotion
in commit `deefc7a` (2026-04-23, `docs(policy): promote normative planning corpus into
docs/ + docs/adr/`). That commit is the merge-evidence anchor for every per-ADR row
below; the file-level history collapses to the same SHA because the ADRs were
authored in the gitignored `chromaticity-planning/` archive prior to promotion and
imported atomically.

## Per-ADR sign-off

- [x] **ADR-011 — Rendering backend (`docs/adr/ADR-011-rendering-backend.md`).**
      Contract scope: locks the GPU API stack to wgpu-rs (Metal on macOS, DX12/Vulkan
      on Windows), the shader-authoring dialect to GLSL 4.50 fragment with naga
      transpilation, the MVP pixel-format parity table (RGBA8/RGBA16F/RGBA32F/R32F),
      and the `CP-10 FLOAT: true` default plus per-MVP-class downgrade rule.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

- [x] **ADR-012 — Script sandbox engine (`docs/adr/ADR-012-script-sandbox-engine.md`).**
      Contract scope: pins the script runtime to QuickJS (BSD-2-Clause), fixes the
      per-frame CPU and memory budgets (1.75 ms p99 update, 4 ms watchdog, 16 MiB
      heap, 256 KiB stack), and codifies the watchdog-preempt + last-known-state
      degradation policy.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

- [x] **ADR-013 — Networking defaults and security
      (`docs/adr/ADR-013-networking-defaults-and-security.md`).**
      Contract scope: fixes per-adapter bind defaults (OSC loopback, NDI subnet,
      Syphon/Spout local-host), the per-mapping CIDR allowlist semantics, the
      inbound rate ceilings (1000 msg/s adapter, 100 msg/s route), and the
      no-mDNS-discovery posture for MVP.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

- [x] **ADR-014 — Signing, notarization, and auto-update
      (`docs/adr/ADR-014-signing-notarization-auto-update.md`).**
      Contract scope: pins macOS Developer ID + hardware-key + hardened-runtime +
      notarytool + stapler, Windows Authenticode EV signature, Sparkle 2 with
      EdDSA-only verification (no DSA fallback path ever ships), and HTTPS-only
      appcast on a Chromaticity-controlled domain.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

- [x] **ADR-015 — Reference-app user-data import
      (`docs/adr/ADR-015-reference-app-user-data-import.md`).**
      Contract scope: enumerates the six importable artifact classes (settings,
      scene presets, snap slots, meta/media presets, playlists, MIDI mappings, OSC
      aliases, hotkey rebinds), specifies the dry-run + per-class operator-toggle
      mechanism, and pins the neutral "reference-app" naming required by
      `docs/29-cleanroom-legal-policy-baseline.md`.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

- [x] **ADR-016 — Observability and telemetry posture
      (`docs/adr/ADR-016-observability-and-telemetry-posture.md`).**
      Contract scope: locks the zero-outbound-traffic-by-default posture (Sparkle
      appcast is the only exception), bans third-party analytics SDKs at MVP, pins
      crash reporting to a self-hosted Sentry with explicit payload redaction, and
      restricts health metrics to a loopback Prometheus endpoint.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

- [x] **ADR-018 — Non-scene UI toolkit
      (`docs/adr/ADR-018-non-scene-ui-toolkit.md`).**
      Contract scope: fixes the application shell to Tauri 2 with system web view
      (WebKit / WebView2), the UI framework to Svelte 5 in runes mode, the language
      to TypeScript with `ts-rs`-generated bindings from the Rust core, the
      authoring editor to CodeMirror 6, and the wgpu-surface composition + native
      accessibility-bridge approach.
      Reviewed and adopted by AO `@PhantasticUniverse` on 2026-04-29 (merge commit
      `deefc7a`).

## Overall baseline sign-off

- [x] All seven gating ADRs are `Accepted`, merged on `main`, and the contract scope
      summarized above is complete and internally consistent. The
      `EV-ARCH-CONTRACT-BASELINE` Phase 0 exit gate is closed.
      Signed by AO `@PhantasticUniverse` on 2026-04-29.
