<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# ADR-021: Syphon Adapter (macOS GPU Surface I/O)

## Status

Accepted

## Date

2026-05-07

## Deciders

- Accountable Owner: Xavier Butler (@PhantasticUniverse)

## Context

Phase-4a EXIT was achieved 2026-05-03 at commit `5acc452` with `EV-INTEGRATION-MATRIX` passing under an AO-approved narrowing scoping its conformance corpus to OSC routing-matrix only plus the Wave 1b MIDI input arm. The camera, Syphon, and audio-uniform-IPC arms were explicitly deferred into Wave 1c per `docs/11-roadmap-phases.md` and the Phase-4a EXIT closure section of `docs/EV-GATE-MODEL-AUDIT-checklist.md`. Wave 1c is structured into two independently shippable slices:

- **1c-α** — audio-uniform IPC bridge (governed by `adr/ADR-019-audio-uniform-ipc-bridge.md`; closed 2026-05-03 / 2026-05-06 across the `#338`-`#340` and `#343`-`#344` PR sessions).
- **1c-β** — camera + Syphon adapters. This ADR governs the Syphon arm; the parallel sibling `adr/ADR-020-camera-adapter.md` governs the camera arm.

Syphon is the macOS-native shared-GPU-texture protocol used in the live-visuals + VJ ecosystem (Resolume, MadMapper, Notch, TouchDesigner). It exposes named GPU surfaces over `IOSurface` for inter-app frame sharing without CPU round-trip. The reference application ships `Contents/Frameworks/Syphon.framework` (BSD-3-Clause) and exposes Syphon I/O on macOS only. `adr/ADR-011-rendering-backend.md` Sec.4 explicitly lists Syphon as a macOS-only `MTLTexture` / `IOSurface` interop path the wgpu Metal backend supports via `wgpu::hal`.

> **License classification note.** The canonical Syphon-Framework `License.txt` at the repo root is **BSD-3-Clause** — the third clause ("Neither the name of the Syphon Project nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission") makes it 3-Clause. The per-file inline headers within the framework reproduce only clauses 1+2 as a shorthand, but the authoritative `License.txt` governs. Earlier internal references to "BSD-2-Clause" reflected reading only the per-file headers and have been corrected throughout this ADR.

Adapter precedent in the workspace is the `integrations/osc-adapter/` crate shape (`src/{lib,adapter,defaults,routes}.rs`) per `adr/ADR-005-integration-adapter-boundary.md`'s `ProtocolAdapter` / `MediaInputAdapter` / `IntegrationHub` boundary. Entitlement infrastructure for Syphon is already in place: `IO_SYPHON_INPUT` and `IO_SYPHON_OUTPUT` capabilities entered the `HONORED_TOKEN_CAPABILITIES` honored set during the Wave 1b-B 13-cap widening (`adr/ADR-009-activation-and-entitlements.md` §Decision tier matrix).

The decision-problem this ADR locks is the chromaticity-native Syphon adapter contract: macOS-only, both input (consume frames published by other apps on the bus) and output (publish frames produced by the chromaticity render pipeline) directions, framework-binding strategy, public Rust surface, and the wgpu ↔ `IOSurface` bridge contract that `engine/render` will implement in Phase 3 to wire Syphon textures into the frame graph. **Spout (the Windows analog of Syphon) is explicitly out of scope** for this ADR — Spout is governed by a separate Phase-5+ task per the Wave 1c-β scoping plan.

## Decision Drivers

- macOS-only feature gate must compile cleanly on Windows and Linux workspace builds; non-macOS members must not pull in the framework dependency.
- Adapter shape mirrors the established `integrations/{osc,midi}-adapter/` precedent so the `IntegrationHub` translation layer per `adr/ADR-005` can compose Syphon as a `MediaInputAdapter` (input) and a sibling `MediaOutputAdapter` (output) without a per-adapter special case.
- Workspace lint posture (`Cargo.toml` `[workspace.lints.rust]` `unsafe_code = "forbid"`) requires explicit per-module `#[allow(unsafe_code)]` exemption for the Objective-C framework binding declarations per `docs/43-development-standards.md` §12.1.
- Reuse the wgpu Metal interop path `adr/ADR-011-rendering-backend.md` Sec.4 already names; do NOT introduce a parallel Metal device alongside the wgpu device.
- Entitlement surface is already widened to honor `IO_SYPHON_INPUT` + `IO_SYPHON_OUTPUT` per Wave 1b-B; this ADR does not extend `HONORED_TOKEN_CAPABILITIES`.
- Trust boundary: Syphon's shared-`IOSurface` model means OTHER macOS apps on the same machine can read published surfaces; the operator must be informed that publishing exposes the surface bus-wide.
- Hardware-rig CI validation is impractical (no live Syphon bus on GitHub Actions runners); narrowing to synthetic / mock conformance follows the `EV-INTEGRATION-MATRIX` precedent established by row 78 (Wave 1a OSC routing matrix) and row 80 (Wave 1b MIDI input).

## Considered Options

1. **Per-platform `integrations/syphon-adapter/` crate, macOS-only feature gate, `objc2` framework bindings, link against system `Syphon.framework`** — adapter mirrors OSC / MIDI shape; `lib.rs` opens with `#![cfg(target_os = "macos")]` so non-macOS workspace members compile cleanly; wgpu ↔ `IOSurface` bridge lives in `engine/render/src/syphon_adapter.rs`. (Chosen.)
2. **Shell out to a Syphon CLI helper subprocess** — chromaticity-app spawns a separate macOS binary that owns the Syphon framework calls; IPC over a Tauri command channel.
3. **Vendor the Syphon framework binary in `third_party/`** — embed the prebuilt `Syphon.framework` Mach-O bundle in the chromaticity repo so builds do not depend on a system framework search path.
4. **Use the `syphon` crate from crates.io** — depend on a third-party Rust binding to Syphon if one exists at the required maturity.

## Decision

Adopt **Option 1: per-platform `integrations/syphon-adapter/` crate, macOS-only feature gate, `objc2` framework bindings, link against system `Syphon.framework`** with the following locked contract.

### 1. Crate placement and feature gating

A new workspace member `integrations/syphon-adapter/` mirrors the `integrations/osc-adapter/` source shape:

- `src/lib.rs` — public re-exports; opens with `#![cfg(target_os = "macos")]` so the entire crate body is gated out of non-macOS compilation. The crate is included in `[workspace.members]` unconditionally, but on Windows / Linux it compiles to an empty rlib (no items, no link-time framework dependency).
- `src/adapter.rs` — `SyphonInputAdapter` + `SyphonOutputAdapter` definitions and lifecycle.
- `src/defaults.rs` — discovery interval, output-name template, and any other operator-overridable defaults.
- `src/routes.rs` — Syphon server-id discovery shape and (for output) registration shape; the equivalent of `osc-adapter`'s route map.

`Cargo.toml` declares `objc2` and `objc2-foundation` exclusively under `[target.'cfg(target_os = "macos")'.dependencies]`; the `[dependencies]` block lists only platform-portable crates (e.g. workspace error / tracing helpers). Non-macOS workspace builds therefore neither resolve nor compile the Objective-C binding stack.

### 2. Framework binding strategy

Use `objc2 = "0.5"` plus `objc2-foundation` (and `objc2-app-kit` for `IOSurface` typing where required) for the Objective-C method declarations. Raw `objc2::Encode` declarations target the Syphon framework's `SyphonServer` and `SyphonClient` Objective-C classes plus the `SyphonServerDirectory` shared dictionary used for discovery.

Module-level `#[allow(unsafe_code)]` exemption per `docs/43-development-standards.md` §12.1 is applied only to the binding-declaration module (`src/adapter.rs::ffi`); the rest of the crate is safe Rust. The exemption is documented inline with a comment block citing the specific Objective-C selector + framework version the bindings track, so a future framework upgrade has a one-stop diff target.

### 3. Framework vendor strategy

Link against the system-installed `Syphon.framework`. A Cargo build script `integrations/syphon-adapter/build.rs` emits, on macOS only:

```
cargo:rustc-link-lib=framework=Syphon
cargo:rustc-link-search=framework=<dev-box framework search path>
```

The framework binary itself is **NOT vendored** in the chromaticity repo. Reference path is the macOS dev-box system framework search path; production app builds will resolve the framework via the Tauri bundling step that pulls `Contents/Frameworks/Syphon.framework` (or the chromaticity-side equivalent) into the `.app` bundle. The license file IS vendored at `LICENSES/LicenseRef-Syphon-BSD-3-Clause.txt` and annotated in `REUSE.toml` per the REUSE 3.2 posture documented in `CLAUDE.md`. The license file lands in Phase 3 PR 1 alongside the crate scaffold; the framework search path is configurable via an environment variable for CI / dev-box flexibility.

The Cargo build script `integrations/syphon-adapter/build.rs` reads a `SYPHON_FRAMEWORK_PATH` environment variable to locate the framework, defaulting to `/Library/Frameworks/Syphon.framework` (the canonical macOS install location) and falling back to a vendored copy under `third_party/Syphon-Framework/` if present. The build script emits, on macOS only:

```
cargo:rustc-link-lib=framework=Syphon
cargo:rustc-link-search=framework=<resolved path from SYPHON_FRAMEWORK_PATH or default>
```

Chromaticity release bundles redistribute the framework binary at `Contents/Frameworks/Syphon.framework`. BSD-3-Clause attribution accompanies the redistribution (the LICENSES/-vendored file plus the chromaticity About / Acknowledgements UI surface listing Syphon under BSD-3-Clause). The redistribution is permitted by BSD-3-Clause clauses 1+2; the no-endorsement obligation under clause 3 is satisfied by the UI-copy compliance note in §License compliance below.

### 4. Public surface

The crate's public surface is two adapters plus the discovery primitive:

**`SyphonInputAdapter`** (consumes frames published by other macOS apps on the Syphon bus):

- `bind(server_uuid: SyphonServerId)` — attaches a `SyphonClient` to the named server. Idempotent under repeat calls with the same id; replaces the prior binding if called with a different id.
- `unbind()` — releases the `SyphonClient`; safe to call when not bound.
- `poll(timeout: Duration) -> Option<SyphonTexture>` — non-blocking pull of the most recent frame the client has received; returns `None` if no fresh frame since the last poll. Drop-frames-on-saturation: the adapter exposes ONLY the most recent frame, not a queue.

  **Polling-consume model preference.** The canonical Syphon API supports two consumption models on the input side: a block-callback handler passed to `SyphonMetalClient::initWithServerDescription:device:options:newFrameHandler:`, OR polling via `hasNewFrame` + `newFrameImage`. The framework allows `nil` for the `newFrameHandler` argument, opting into polling explicitly. Chromaticity passes `nil` and drives consumption from the scheduler-tick via `hasNewFrame` + `newFrameImage` reads. Rationale: simpler binding (no `objc2::rc::RcBlock` round-trip; no `Send`-bound closures to construct from the Rust side), and avoids cross-thread marshalling complexity — Syphon's new-frame handler runs on its own dispatch queue, which would force a marshalling step into the chromaticity render thread. Polling sidesteps that entirely. The cost is a per-tick `hasNewFrame` check; the canonical framework's polling path is cheap (atomic-bool freshness flag — `SyphonMetalClient.m:40, 91-104`), so the cost is negligible at 60 Hz.
- `health() -> AdapterHealth` — mirrors the `OscAdapter::health()` shape; surfaces bind state, last-frame-received timestamp, and discovery-poll status.
- `discover_servers() -> Vec<SyphonServerId>` — reads the `SyphonServerDirectory` shared-dictionary surface; returns the current set of advertised servers on the bus.

`SyphonServerId { uuid: String, name: String, app_name: String, dictionary_version: u32 }` mirrors the reference Syphon discovery shape and adds a defensive forward-compat field. UUID identifies the server uniquely; name is the operator-facing label; app_name identifies the publishing application, useful for UI grouping; `dictionary_version` carries the server's advertised `SyphonServerDescriptionDictionaryVersionKey` value:

```rust
pub struct SyphonServerId {
    pub uuid: String,
    pub name: String,
    pub app_name: String,
    pub dictionary_version: u32, // canonical kSyphonDictionaryVersion = 0; refuse forward-incompat servers
}
```

**Discovery-side filter behavior**: chromaticity's `discover_servers()` refuses servers with `dictionary_version > 0` (the current `kSyphonDictionaryVersion`), filtering them out of the returned list and emitting a `tracing::warn!` log line identifying the offending server's UUID + advertised version. This mirrors the canonical `SyphonClientBase.m:87-93` defensive forward-compat refusal pattern, where the client's `init` returns `nil` when the server's `dictionaryVersion > kSyphonDictionaryVersion`. Forward-compat refusal is the safe posture when chromaticity does not know how to interpret a server advertising a newer protocol revision; a future framework upgrade widens the accepted range as part of its binding refresh.

`SyphonTexture { iosurface_id: u64, width: u32, height: u32, pixel_format: SyphonPixelFormat }` is the cross-seam texture handle. `iosurface_id` is the `IOSurfaceID` the wgpu Metal interop path resolves into an `MTLTexture`; the engine-side bridge in §6 is responsible for that wrapping.

**`SyphonOutputAdapter`** (publishes frames produced by the chromaticity render pipeline):

- `bind(name: String)` — registers a `SyphonServer` advertising the supplied name on the bus. Idempotent under repeat calls with the same name; replaces the prior registration if called with a different name.
- `unbind()` — deregisters the server; safe to call when not bound.
- `publish(texture: SyphonTexture)` — wraps the supplied `IOSurface`-backed texture in a `SyphonServerFrame` and dispatches to the bus. The caller (the engine-side bridge in §6) is responsible for ensuring the texture is the operator's selected output and is `IOSurface`-exportable.
- `health() -> AdapterHealth` — surfaces register state, last-publish timestamp, and any framework-reported errors.

### 5. Defaults

`src/defaults.rs` declares:

- `DEFAULT_DISCOVERY_INTERVAL_MS: u64 = 500` — poll interval for `SyphonServerDirectory` change detection on the input side. Operator-facing UI surfaces a fresh server list within ~500 ms of bus changes, matching the reference Syphon Recorder cadence.
- `DEFAULT_OUTPUT_NAME_TEMPLATE: &str = "Chromaticity ${profile_name}"` — operator-overridable output server name. `${profile_name}` interpolates the active chromaticity profile (e.g. "Tour 2026") for multi-instance disambiguation when an operator runs two chromaticity processes on the same machine.
- No rate-limit or queue-depth defaults are declared. Syphon is GPU-direct via `IOSurface`; the operator scene's render cadence is the implicit rate gate, and the adapter does not add its own throttling.

### 6. wgpu ↔ `IOSurface` bridge

The cross-seam bridge between the adapter's `SyphonTexture` handle and the engine's wgpu render graph lives at `engine/render/src/syphon_adapter.rs` and is implemented in Phase 3 PR 3. The contract this ADR locks:

- **Output direction**: the engine selects an `IOSurface`-exportable wgpu texture (the final output target or a designated tap point in the frame graph), wraps the underlying `MTLTexture` via `wgpu::hal` Metal interop per `adr/ADR-011` Sec.4, and constructs a `SyphonTexture` whose `iosurface_id` references that surface. `SyphonOutputAdapter::publish` is invoked once per frame after frame-graph completion; the publish is non-blocking from the render thread's perspective because Syphon's bus delivery is GPU-direct.
- **Input direction**: `SyphonInputAdapter::poll` returns a `SyphonTexture` whose `iosurface_id` the bridge resolves into an `MTLTexture` and re-imports as a wgpu external texture for sampling within the frame graph. The bridge uses the same `wgpu::hal` Metal interop surface in reverse.
- **Thread affinity**: framework calls (`SyphonServer` / `SyphonClient` methods) are issued from the render thread per `adr/ADR-006-thread-and-process-model.md` (the same thread that owns the wgpu device). The adapter does NOT spawn its own thread; discovery polling is invoked from the render thread on a `DEFAULT_DISCOVERY_INTERVAL_MS` cadence interleaved with frame-graph dispatch. This preserves the ADR-006 single-renderer-thread invariant.
- **No parallel Metal device**: chromaticity does not create a second `MTLDevice` alongside the wgpu device. All Syphon I/O routes through the wgpu-owned device's interop surface. This matches the trajectory `adr/ADR-011` Sec.4 sets for Spout (where DX12 / DX11 interop on the Windows side reuses the wgpu DX12 device rather than spinning up an independent device).

### 7. Entitlement posture

The Wave 1b-B 13-cap widening already added `IO_SYPHON_INPUT` and `IO_SYPHON_OUTPUT` to `HONORED_TOKEN_CAPABILITIES` per `adr/ADR-009-activation-and-entitlements.md` §Decision tier matrix. This ADR does not extend the honored set. `EV-ENTITLEMENT-ENFORCEMENT` is unaffected; the existing capability-bitmask check at the `MediaOutputAdapter::bind` / `MediaInputAdapter::bind` IntegrationHub entry point gates Syphon I/O behind those two caps without further work in this wave.

### 8. Drop impl posture

Chromaticity's `SyphonServer` Rust wrapper MUST implement `Drop` calling `[server stop]` defensively. The canonical Syphon-Framework registers a process-destructor specifically because users forget to call `stop` — leaked servers spam the system with stale entries (visible in `SyphonServerDirectory`) even after the chromaticity process exits, polluting the bus for other VJ applications running on the same machine. Equivalent posture applies to `SyphonClient` wrappers: `Drop` calls `[client stop]` to release the framework-side connection deterministically rather than relying on Objective-C runtime cleanup.

Idiomatic Rust shape: the wrapper holds `Retained<SyphonServer>` (objc2 reference-counted handle) and the `Drop` impl calls the Objective-C `stop` method via the binding before the `Retained` drops. This is safe even if `stop` was already called explicitly (idempotent on the framework side per `SyphonServerBase.m`).

### 9. License compliance

**Attribution.** BSD-3-Clause clauses 1 + 2 require attribution in source + binary redistributions. Chromaticity satisfies this two ways:

- The `LICENSES/LicenseRef-Syphon-BSD-3-Clause.txt` file vendored at Phase 3 PR 1 reproduces the canonical `Syphon-Framework/License.txt` verbatim including the full copyright + permission text + no-endorsement clause + warranty disclaimer.
- The chromaticity About / Acknowledgements UI surface (the surface required for redistributing third-party code under attribution-bearing licenses) lists the Syphon framework with copyright credit:

  > Copyright 2010-2023 bangnoise (Tom Butterworth) & vade (Anton Marini); Metal subtree co-authored by Maxime Touroute & Philippe Chaurand (Millumin). Licensed under BSD-3-Clause.

  The Metal subtree credit lines reflect the additional attribution carried inline in `SyphonMetalServer.{h,m}`, `SyphonMetalClient.{h,m}`, `SyphonServerRendererMetal.{h,m}` for 2020-2023 Maxime Touroute + Philippe Chaurand (Millumin) contributions; omitting them would be an attribution gap on the Metal binding path chromaticity actually uses.

**No-endorsement clause compliance.** BSD-3-Clause clause 3 prohibits using "Syphon Project" or contributor names in marketing or UI copy that implies endorsement. Chromaticity may state factual interoperability ("Compatible with the Syphon Framework", "Supports Syphon input + output on macOS") but MUST NOT state or imply endorsement ("Powered by Syphon", "Syphon-certified", "endorsed by Syphon Project"). The chromaticity About / Acknowledgements UI surface lists Syphon attribution under BSD-3-Clause but uses neutral wording; marketing site + release notes use "Syphon-compatible" rather than "Powered by Syphon". Reviewer checklist for any future marketing-copy PR touching Syphon mentions: confirm the language is factual interoperability, not endorsement.

### 10. Threat posture and operator UX warning

Syphon's shared-`IOSurface` model is bus-wide on the local machine: any other macOS application that has the Syphon framework loaded can subscribe to a published server and read its frames. This is intentional per Syphon's design (it is the entire reason the protocol exists) but it has a privacy implication operators must be informed of: **publishing a Syphon output exposes the surface to any macOS app on the same machine; do not publish surfaces containing PII or secrets.** The operator-facing UI surface (the `Sources.svelte` / output-routing tile per the Wave 1c-β UI cohort) carries a one-line warning when Syphon output is enabled. This ADR's Decision binds Phase 3 PR 3 to ship that warning string alongside the bridge implementation; the warning text is operator-localizable via the embedded-docs i18n surface per `adr/ADR-010-embedded-docs-generation.md` and is NOT hardcoded into the adapter crate.

The threat-model addendum is documented in `docs/29-cleanroom-legal-policy-baseline.md` (parallel sibling Phase-1 PR, separate from this ADR) so the legal baseline reflects the bus-wide-exposure posture independently of the engineering ADR.

## Consequences

### Positive

- macOS-only feature gate means non-macOS workspace builds compile cleanly without the Objective-C dependency stack; the crate is included unconditionally in `[workspace.members]` and contributes zero items on Windows / Linux.
- Adapter shape composes into the existing `IntegrationHub` translation layer per `adr/ADR-005` without a per-adapter special case; the `MediaInputAdapter` and `MediaOutputAdapter` traits the hub will define for camera + Syphon are mirrors of the established `ProtocolAdapter` shape OSC and MIDI ship today.
- wgpu ↔ `IOSurface` bridge reuses the Metal interop path `adr/ADR-011` Sec.4 already names; no parallel Metal device, no duplicate frame-graph allocation logic.
- Entitlement gating is structurally complete the day the crate ships — `IO_SYPHON_INPUT` / `IO_SYPHON_OUTPUT` are honored caps already, and the `HONORED_TOKEN_CAPABILITIES` widening that admitted them was validated in Wave 1b-B against `EV-ENTITLEMENT-ENFORCEMENT`.
- Discovery cadence and output-name template are operator-overridable defaults, matching the reference Syphon Recorder UX expectations the live-visuals + VJ ecosystem already trains on.
- The Wave 1c-β scope is narrowly bounded (1000-1800 LoC across 3 PRs) because the entitlement / IPC / observability scaffolding is already in place from Phase-4a and Wave 1c-α.

### Negative

- The crate carries an `#[allow(unsafe_code)]` exemption per `docs/43-development-standards.md` §12.1 for the Objective-C binding-declaration module. The exemption is narrow (binding declarations only; the rest of the crate is safe Rust) and follows the `objc2 = "0.5"` cookbook for `Encode` / `Message` declarations, but it does widen the workspace's `unsafe_code = "forbid"` posture by one module. A future Syphon framework upgrade requires re-validating the binding declarations.
- The framework search path in `build.rs` is environment-dependent. Dev-box builds rely on the system framework search path; production app builds rely on the Tauri bundling step pulling `Syphon.framework` into the `.app` bundle. CI runners that lack the framework will fail to link the macOS Syphon-adapter crate; this is mitigated by gating the macOS-only CI matrix entry on framework presence (an environment-variable check in the Cargo build script that emits a `cargo:warning` and skips the link step when the framework is absent, allowing `cargo check` on CI runners that lack the framework).
- Hardware-rig CI validation of the live Syphon bus is not possible on GitHub Actions runners. The Phase 3 PR 2 T2 corpus uses synthetic / mock conformance per the row 78 / row 80 / row 93 precedent — a stub trait that mocks `SyphonServer` / `SyphonClient` lifecycle and asserts the adapter's bind / unbind / poll state machine behavior without exercising the real framework. This narrowing is recorded honestly on the closing-docs PR 3 against `EV-INTEGRATION-MATRIX`.
- Multi-server output (a single `SyphonOutputAdapter::bind` call publishing multiple surfaces under different names) is deferred to a post-MVP enhancement. MVP exposes one output server per `bind` call; operators who want to publish multiple chromaticity outputs from one process must instantiate multiple `SyphonOutputAdapter` instances. This matches the reference Syphon Recorder MVP UX.

### Neutral / trade-offs

- The crate is macOS-only by feature gate, not by workspace membership. This trades a tiny per-build no-op compile cost on non-macOS runners (the empty rlib) for the simplicity of a single workspace topology — `[workspace.members]` does not need per-platform conditionals. The trade-off matches the precedent the engine + integration crates already follow for other platform-bounded surfaces.
- The cross-seam bridge contract is locked here, but the implementation lands in Phase 3 PR 3. This is the same composition pattern `adr/ADR-019` used for the audio-uniform IPC bridge (contract in the ADR; implementation across a 2-PR / 3-PR engineering wave); reviewers familiar with that precedent will recognize this ADR's shape.
- Spout (the Windows analog) is deferred to a separate Phase-5+ task, not bundled into this ADR. Bundling would have been tempting (Spout's adapter shape mirrors Syphon's almost exactly) but the Windows DX11 / DX12 interop surface and the Spout-specific shared-handle protocol are different enough from Syphon's `IOSurface`-based model that a single ADR would obscure the per-platform decisions both surfaces require. Each gets a dedicated ADR when its phase activates.

## Alternatives Considered

- **Option 2 (Shell out to a Syphon CLI helper subprocess)** — rejected. Adds an IPC hop that the wgpu ↔ `IOSurface` bridge does not need (the framework calls are GPU-direct and require the same Metal device the renderer owns); a subprocess would force a CPU round-trip on every frame and defeat Syphon's zero-copy design point. Also adds release-engineering complexity (the helper binary needs separate signing + notarization per `adr/ADR-014-signing-notarization-auto-update.md`).
- **Option 3 (Vendor the Syphon framework binary in `third_party/`)** — rejected. The framework's BSD-3-Clause license permits redistribution, but vendoring a Mach-O binary in the chromaticity repo would (a) substantially inflate the repo size, (b) bind the workspace to a specific framework build versus tracking upstream, and (c) introduce a binary-vendoring precedent the workspace does not currently set (the only vendored third-party content per the `CLAUDE.md` "Workspace layout" section is the Lygia v1.1.4 source snapshot, which is text under trial-license deferral per `project_lygia_trial_deferred_legal.md`). System-framework linkage is the upstream-tracking choice.
- **Option 4 (`syphon` crate from crates.io)** — rejected. As of 2026-05, no `syphon` crate at the required maturity exists on crates.io; the candidates are dormant or thin wrappers. The `objc2` direct binding path is upstream-tracking and gives chromaticity ownership of the framework-version diff target. If a sufficiently maintained third-party crate emerges in a future cycle, this ADR can be revisited and superseded.

## Out of Scope

- **Spout (Windows Syphon analog)** — separate Phase-5+ task per the Wave 1c-β scoping plan. The DX11 shared-handle interop pattern `adr/ADR-011` Sec.4 names is the design seed; the Spout ADR will lock the Windows-side decisions when its phase activates.
- **Linux Syphon equivalent** — no canonical Linux equivalent exists at this writing; out of scope. Linux is a post-MVP platform per `docs/30-mvp-support-matrix.md` regardless.
- **iOS / iPadOS Syphon support** — chromaticity is a desktop product per `docs/30-mvp-support-matrix.md`. Syphon does ship an iOS surface, but mobile chromaticity is not in any roadmap phase.
- **Hardware-rig CI validation against a live Syphon bus** — synthetic narrowing per row 78 / row 80 / row 93 precedent; the Phase 3 PR 2 T2 corpus uses a mock framework trait. A future hardware-in-the-loop CI surface could exercise the real framework but is out of scope for Wave 1c-β.
- **Multi-server output (single `SyphonOutputAdapter::bind` publishing multiple surfaces)** — defer to a post-MVP enhancement. MVP exposes one output server per `bind` call; multi-output requires multiple adapter instances.
- **Audio-only Syphon variants** — Syphon protocol covers GPU surfaces only. Audio routing is `engine/audio` territory and is governed by `adr/ADR-019-audio-uniform-ipc-bridge.md`.
- **Per-pass Syphon output taps** (publishing intermediate frame-graph passes, not just the final output) — defer to a post-MVP enhancement. MVP publishes a single output target per `SyphonOutputAdapter` instance; per-pass tap routing requires additional frame-graph instrumentation outside this ADR's scope.

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision.

- **`EV-INTEGRATION-MATRIX`** — primary tagged evidence row. Phase-4a EXIT pass under the OSC routing-matrix + MIDI input narrowing remains unchanged at this ADR's acceptance. The Wave 1c-β engineering wave (Phase 3 PRs 1-3 gated by this ADR) widens the matrix's Syphon arm; the closing-docs PR 3 against this ADR records the synthetic-narrowing the Phase 3 PR 2 T2 corpus uses (no live Syphon bus on CI; mock framework trait) per the row 78 / row 80 / row 93 precedent.
- **`EV-ENTITLEMENT-ENFORCEMENT`** — unaffected by this ADR. `IO_SYPHON_INPUT` and `IO_SYPHON_OUTPUT` already entered `HONORED_TOKEN_CAPABILITIES` in the Wave 1b-B 13-cap widening; no entitlement work is needed in this wave.
- **`EV-RENDER-CONFORMANCE`** — the wgpu ↔ `IOSurface` bridge in §6 reuses the Metal interop path `adr/ADR-011` Sec.4 already names. Per-backend parity diffs (Metal / DX12 / Vulkan) are unaffected because Syphon is macOS-only by gate; the bridge composes additively over the existing render-thread frame graph.
- **`EV-PLATFORM-CONFORMANCE`** — neutral. The macOS-only feature gate means Windows / Linux platform builds have no Syphon code to validate; the macOS arm gains a Syphon-bridge presence row that the Phase 3 closing-docs PR records.
- **`EV-LICENSE-AUDIT`** — the `LICENSES/LicenseRef-Syphon-BSD-3-Clause.txt` file Phase 3 PR 1 vendors lands annotated in `REUSE.toml` per the REUSE 3.2 posture; the audit row is unaffected (BSD-3-Clause is on the workspace allowlist) but the row's coverage envelope widens by one license file. The §License compliance attribution + no-endorsement clauses are tracked under this row's audit envelope.

## Traceability

- `adr/ADR-005-integration-adapter-boundary.md` — `ProtocolAdapter` / `MediaInputAdapter` / `IntegrationHub` boundary this ADR's adapter shape mirrors. The boundary's "no protocol-specific logic in core rendering or state stores" decision is what places the Syphon framework calls in `integrations/syphon-adapter/` rather than in `engine/render` directly; only the wgpu ↔ `IOSurface` bridge lives engine-side.
- `adr/ADR-006-thread-and-process-model.md` — render-thread allocation that the `SyphonServer` / `SyphonClient` framework calls run on per §6.
- `adr/ADR-009-activation-and-entitlements.md` §Decision tier matrix — `IO_SYPHON_INPUT` and `IO_SYPHON_OUTPUT` capabilities honored per Wave 1b-B 13-cap widening.
- `adr/ADR-011-rendering-backend.md` Sec.4 — the wgpu Metal `IOSurface` interop path the Phase 3 PR 3 bridge composes over. ADR-011's Spout note (DirectX 11 shared handle alongside the wgpu DX12 device) is the design seed for the future Spout ADR but is out of scope here.
- `adr/ADR-016-observability-and-telemetry-posture.md` — `health()` shape on both `SyphonInputAdapter` and `SyphonOutputAdapter` mirrors the `OscAdapter::health()` precedent the OSC adapter pinned per ADR-016.
- `adr/ADR-019-audio-uniform-ipc-bridge.md` §Out of Scope — the "Syphon / Spout output IPC. Wave 1c-β scope; future ADR." reservation that this ADR fulfills for the Syphon arm.
- `adr/ADR-020-camera-adapter.md` — parallel sibling Phase-1 ADR governing the camera arm of Wave 1c-β; the two ADRs share the Wave 1c-β phase activation but are independently scoped.
- `26-evidence-artifact-registry.md` — `EV-INTEGRATION-MATRIX` row remains the primary tagged surface; this ADR does not add a new evidence row (the Syphon-arm widening lands at the Phase 3 closing-docs PR per the same dual-tag pattern Wave 1b used).
- `29-cleanroom-legal-policy-baseline.md` — Syphon framework license + threat-model addendum (parallel sibling Phase-1 PR, separate from this ADR).
- `30-mvp-support-matrix.md` — macOS-MVP scope that bounds this ADR's platform applicability; iOS / Linux are out of scope per matrix.
- `11-roadmap-phases.md` — Phase-4a Wave 1c-β sequencing this ADR opens (alongside ADR-020 for the camera arm).
- `EV-GATE-MODEL-AUDIT-checklist.md` Phase-4a EXIT closure section — the AO direction deferring camera / Syphon / audio-uniform-IPC arms to Wave 1c that this ADR partially closes for the Syphon slice.

## Rollout and Review

- **Rollout (status: proposed; will be Accepted at AO review time).** Wave 1c-β Phase 3 engineering plan, gated on this ADR's acceptance:
  - **PR 1** — `integrations/syphon-adapter/` scaffold: `[workspace.members]` registration, `Cargo.toml` with macOS-only `[target.'cfg(target_os = "macos")'.dependencies]` (`objc2 = "0.5"` + `objc2-foundation`), `src/lib.rs` opening with `#![cfg(target_os = "macos")]`, `src/adapter.rs` skeleton with `objc2` framework binding declarations under `#[allow(unsafe_code)]`, `src/defaults.rs` with the §5 constants, `src/routes.rs` skeleton, `build.rs` reading `SYPHON_FRAMEWORK_PATH` env var + emitting the framework link directives, `LICENSES/LicenseRef-Syphon-BSD-3-Clause.txt` plus `REUSE.toml` annotation, and 6-8 T1 pins covering the public surface declarations including the `Drop` impls + the `dictionary_version` discovery filter.
  - **PR 2** — `SyphonOutputAdapter` + `SyphonInputAdapter` implementation: discovery loop, bind / unbind state machine, `poll` / `publish` plumbing, `health()` surface, and a T2 corpus under synthetic narrowing (mock framework trait — no live Syphon bus on CI; the mock asserts adapter state-machine behavior end-to-end).
  - **PR 3** — `engine/render/src/syphon_adapter.rs`: wgpu ↔ `IOSurface` bridge for input texture binding + output surface publish per §6; operator-facing UI warning string in the `Sources.svelte` / output-routing tile per §8; closing-docs PR widens `EV-INTEGRATION-MATRIX` Syphon-arm coverage with the synthetic-narrowing recorded honestly.
  - Sub-phase scope-window: estimated 1000-1800 LoC across the 3 PRs.
- **Review cadence.** This ADR is revisited if (a) the `objc2 = "0.5"` framework binding pattern is materially superseded by a maintained third-party `syphon` crate at sufficient maturity, (b) the Spout ADR (separate Phase-5+ task) surfaces a cross-platform contract constraint that requires this ADR's surface to widen for symmetry, (c) a future hardware-in-the-loop CI surface lets the synthetic-narrowing on `EV-INTEGRATION-MATRIX` widen to a real-bus conformance corpus, or (d) the Syphon framework upstream releases a major-version change that breaks the binding declarations.

## Decision log / discussion summary

The decision-problem this ADR locks was bounded by three prior commitments: the `IntegrationHub` adapter boundary in ADR-005, the wgpu Metal interop path in ADR-011 Sec.4, and the `IO_SYPHON_INPUT` / `IO_SYPHON_OUTPUT` honored-caps widening in Wave 1b-B per ADR-009. The remaining axes — feature-gate strategy (per-platform vs unconditional with `cfg`), framework binding strategy (`objc2` vs subprocess vs vendored binary vs third-party crate), and the wgpu interop bridge placement (engine-side vs adapter-side) — are this ADR's contribution. The chosen shape mirrors the OSC + MIDI adapter precedent for `IntegrationHub` composition while honoring the `unsafe_code = "forbid"` workspace lint posture via a narrow per-module exemption documented at the binding-declaration site. Spout was deliberately not bundled into this ADR; the per-platform decisions Spout requires are different enough from Syphon's that a single ADR would obscure both.

---

> This ADR is binding once `Status: Accepted`. Modification requires a new ADR or a superseding ADR; do not edit the Decision section of an Accepted ADR in place.
