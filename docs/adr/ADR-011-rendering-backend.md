# ADR-011: Rendering Backend and Shader Language

## Status
Accepted

## Context
`ADR-002-rendering-backend.md` set the architectural shape ("engine-neutral frame graph") but deliberately left the concrete GPU API stack and shader-authoring dialect open. Implementation kickoff cannot proceed without these decisions because `CP-10 Pass-Sizing Algebra` (RGBA32F default + RGBA16F fallback), `02-synesthesia-contracts.md` Sec.16, the `#include` resolution contract in Sec.18, and `EV-RENDER-CONFORMANCE` all have behaviors that depend on which backend is hosting them.

Constraints observed from the reference app and Chromaticity's charter:
- Reference app is shipped as a macOS x86_64 Mach-O binary. Apple Silicon runs under Rosetta. Native Apple Silicon is credible Chromaticity differentiation.
- `30-mvp-support-matrix.md` requires macOS-MVP (Apple Silicon) and Windows-MVP.
- Author-facing GLSL is the single largest source of compatibility risk. Scene authors must be able to write the same `renderMain()` body on both platforms without per-OS conditionals.
- NDI and Syphon/Spout (`FR-8`) require zero-copy GPU texture interop; the chosen backend must expose a portable surface-sharing mechanism.
- Multipass + feedback (`ADR-002`, `02` Sec.15-16) depends on float targets (`RGBA32F` and `RGBA16F`) and sampler state (wrap, filter) parity across platforms.

## Decision

### 1. GPU API stack
Use **wgpu-rs** (WebGPU implementation in Rust) as the cross-platform backend.
- macOS: wgpu routes to Metal.
- Windows: wgpu routes to DirectX 12 (primary) or Vulkan (fallback if DX12 is unavailable on the target GPU class).
- Linux (post-MVP): wgpu routes to Vulkan.

### 2. Shader-authoring dialect
Scene authors write **GLSL 4.50 fragment** dialect with Chromaticity preprocessor extensions (`#include`, `PASSINDEX`, `RENDERSIZE`, meta helper prepend). Scene GLSL is transpiled to SPIR-V via **naga** (wgpu's native cross-compiler), then to Metal Shading Language (macOS) or DXIL (Windows) automatically by wgpu.

Authoring dialect constraints:
- No compute shaders in MVP (fragment-only, matching reference).
- No vertex shader authoring (Chromaticity provides a fullscreen-triangle vertex shader; authors never touch vertex stage).
- Samplers are 2D only for MVP (`syn_Media`, `syn_UserImage`, `syn_FinalPass`, pass samplers, `syn_Spectrum`/`syn_LevelTrail` as emulated 2D height-1). No `sampler3D`, no cubemaps in MVP.
- Integer textures and bitwise ops are allowed (naga supports them).
- `#version` directive is Chromaticity-managed and prepended by the compiler; author sources declare neither `#version` nor precision qualifiers.

### 3. Pixel format parity
Chromaticity normatively supports:

| Format | macOS (Metal) | Windows (DX12) | Windows (Vulkan) | Used for |
|---|---|---|---|---|
| RGBA8Unorm | native | native | native | default pass target |
| RGBA16Float | native | native | native | `FLOAT: true` on GPU-A where RGBA32F is budget-hostile |
| RGBA32Float | native | native | native | `FLOAT: true` default |
| R32Float | native | native | native | 1D data textures (emulated as 2D height-1) |

`CP-10` `FLOAT: true` default remains RGBA32F. Per-MVP-class RGBA16F downgrade per `22-compatibility-versioning-policy.md` is implemented as a deterministic backend override with `coercion-applied` logging.

### 4. Texture interop
- **Syphon (macOS)**: uses Metal `IOSurface` interop. wgpu's Metal backend exposes raw `MTLTexture` handles via `wgpu::hal`, sufficient to bridge into Syphon.
- **Spout (Windows)**: uses DirectX 11 shared handle. Chromaticity creates a DX11 interop device alongside the wgpu DX12 device and shares via `ID3D11Texture2D` + `HANDLE`. DX12/DX11 interop is a known wgpu pattern.
- **NDI (post-MVP per Core-only edition trim)**: CPU-side frame handoff; no GPU interop requirement for MVP.
- **Camera capture**: AVFoundation on macOS, Media Foundation on Windows; capture frames land in CPU memory then upload to wgpu texture.

### 5. Shader compile cache
A persistent on-disk shader cache keyed by `(scene_package_hash, author_source_hash, backend_variant, Chromaticity_version)` stores compiled SPIR-V + backend blobs. Cold-start compile is budgeted in `EV-SHADER-CACHE`; warm hits bypass compile.

### 6. Error and fallback posture
- Shader compile failure at scene load -> previous successful compile remains live (per `37-authoring-environment-spec.md` Sec.6 and `NFR-1`).
- Backend device loss (GPU reset, driver crash) -> warm restart with state preservation, one black frame acceptable, diagnostic surfaced.
- Unsupported pixel format at runtime (e.g. RGBA32F unavailable on an exotic Windows configuration) -> deterministic downgrade per Sec.3 + coercion log.

### 7. Multi-GPU selection policy
Laptops and desktops with multiple GPUs (integrated + discrete, or multiple discrete) must have deterministic adapter selection. The reference app inherits the OS default, which routinely places the runtime on the low-power integrated GPU mid-show and causes stutter. Chromaticity takes explicit control.

- **Default selection**: Chromaticity enumerates all adapters via wgpu's `Adapter` API and picks the **high-performance** adapter by OS-specific hints (macOS: `MTLDevice.recommendedMaxWorkingSetSize`; Windows: `D3D_FEATURE_LEVEL_12_2` / `DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE`; Linux: `VK_KHR_performance_query` where available, otherwise discrete-PCI preference).
- **Operator override**: Settings -> Output -> GPU shows a picker listing every enumerated adapter with vendor, device name, and backend (Metal / Vulkan / DX12). Operator selection is persisted per installation and survives adapter hot-plug.
- **Per-OS refusal policy**:
 - macOS: if the system is on battery and the operator has not explicitly selected the high-power GPU, Chromaticity surfaces a warning ("Running on integrated graphics; frame pacing may be unstable. Change in Settings -> Output -> GPU.") and continues. This is the mitigation for laptop-lid-closed / battery-conservation scenarios that surprise operators during sound-check.
 - Windows: if the selected adapter's driver reports TDR (Timeout Detection and Recovery) events within the last 24 hours, surface a warning and suggest adapter switch.
 - Linux: adapter switch on Prime-offload systems requires setting `DRI_PRIME=1` externally; Chromaticity documents this in `/manual/display-output`.
- **Hot-plug**: if the selected adapter disappears (external GPU unplug, driver crash), Chromaticity warm-restarts on the next-best adapter per Sec.6 fallback and surfaces a diagnostic. The override setting is preserved; if the preferred adapter returns, the next app launch will select it again.
- **Diagnostics**: the performance HUD and support bundle record vendor, device name, backend, driver version, `VRAM / working set`, and whether the current selection matches the operator's Settings choice.

Enforcement: `EV-GPU-SELECTION` (new evidence) validates deterministic selection on a test matrix of dual-GPU configurations; `EV-SUPPORT-OPS-READINESS` extends to include GPU-metadata capture in the support bundle.

### 8. Color management (V1 scope: sRGB)
V1 output is sRGB-target, respecting the OS display profile. Explicit scope boundary because the "correct" answer for V2+ (HDR, wide gamut, ACES) is a materially larger design surface than MVP can carry.

- **V1 render target format**: RGBA8 sRGB backbuffer (or RGBA16F linear when `FLOAT: true` is declared per Sec.3, downcast to sRGB at final present).
- **V1 color pipeline**:
 - Scene GLSL authors in sRGB texel space (matching reference-parity expectations).
 - Meta-control pipeline (`_applyColorCorrection`, `_applyDichrome`, `_applyMediaContrast`, etc. per `36-reactive-symbol-catalog.md` Sec.3) operates in sRGB texel space; this preserves reference parity and avoids a `compat-syn-v1` breaking change.
 - Final backbuffer write is in sRGB-encoded RGBA8 (or downcast from RGBA16F).
 - OS compositor converts backbuffer sRGB -> display profile. Chromaticity does **not** perform its own ICC conversion.
- **Display profile detection**: at startup and on display change, Chromaticity queries the active display profile (macOS ColorSync via `CGDisplayCopyColorSpace`; Windows via `IDXGIOutput::GetColorSpace` and the monitor's ICC). The detected profile is logged to the performance HUD and to the support bundle. A profile outside the sRGB family surfaces a "non-sRGB display detected" informational diagnostic (not an error).
- **V1 does NOT ship**: HDR output (HDR10, Dolby Vision, or HLG), Display P3, Rec.2020, ACES pipelines, Chromaticity-custom ICC conversion, or scene-level color-space declarations.
- **V2+ scope** (reserved, not designed here): HDR rendering would require a scene-level color-space declaration (new `COLOR_SPACE` scene.json root key), meta-control pipeline math reworked for linear extended-range processing, and a new profile `compat-syn-v1` -> `compat-syn-v1-hdr` bump. An `ADR-018-color-management-hdr` slot is implicitly reserved but not named here to avoid ADR-slot sprawl; when HDR is in scope, the ADR is authored then.
- **Evidence**: `EV-COLOR-MANAGEMENT-SRGB` (per NFR-12 in `03-product-requirements.md`) validates the V1 posture on sRGB and non-sRGB displays.

## Consequences

### Positive
- One authoring GLSL dialect for macOS + Windows; no platform forks in scene source.
- Apple Silicon native via Metal (no Rosetta).
- wgpu ecosystem is mature, actively maintained, and used by Firefox and Bevy - stable dependency floor.
- Future Linux and browser (WebGPU) support via the same codebase.
- naga provides deterministic transpilation with good error mapping back to author source lines (`EV-AUTHORING-DIAGNOSTICS`).

### Negative
- Adds a Rust dependency to the engine. The rest of Chromaticity may be Rust or a language with Rust-FFI bindings.
- naga's GLSL frontend has known gaps (no geometry/tessellation, some exotic built-ins missing); these gaps must be documented in `FR-14 Scene Import Adapters` Shadertoy converter and the embedded docs.
- Persistent shader cache adds disk footprint and invalidation complexity.
- wgpu version upgrades can ripple into shader-compile regressions; pin version per release and include in `EV-RENDER-CONFORMANCE` run metadata.

## Alternatives Considered

- **Native Metal (macOS) + DX12 (Windows) with bespoke frame graph**: rejected. Two shader-compile pipelines, two frame-graph implementations, ~2x engineering surface. Loses the "single dialect" author contract.
- **ANGLE GL ES 3.0 on both platforms**: rejected. Legacy GL is deprecated on macOS 10.14+; Apple has signaled no future investment. ANGLE on macOS routes to Metal anyway, so wgpu is a more direct path.
- **bgfx**: rejected. Smaller community, less active development than wgpu. Shader authoring via bgfx's custom `varying.def.sc`/BX is a regression vs GLSL.
- **SPIR-V Cross (MoltenVK)**: rejected. Adds a second transpile hop (GLSL -> SPIR-V -> MSL) where naga does it in one; MoltenVK is a Vulkan-over-Metal layer and does not help on Windows.
- **Direct Metal shaders (.metal) on macOS**: rejected. Not portable to Windows; authors would maintain two shader copies.

## Consequences for downstream documents
- `02-synesthesia-contracts.md` Sec.16 (Pass-Sizing Algebra) references this ADR for backend pixel-format availability.
- `28-implementation-contract-pack.md` CP-10 references this ADR for the RGBA32F/RGBA16F normative defaults and the deterministic downgrade path.
- `30-mvp-support-matrix.md` gains an "API stack" row per platform class.
- `EV-SHADER-CACHE` (new evidence row) validates cold-start and warm-hit shader-compile budgets.
- `ADR-002-rendering-backend.md` is superseded on the "which backend" question and forwards to this ADR.

## Evidence
- `EV-RENDER-CONFORMANCE` now includes per-backend (Metal/DX12/Vulkan) parity diffs as part of the cross-profile row in `10-performance-latency-budgets.md`.
- `EV-SHADER-CACHE` benchmarks cold-compile (no cache) vs warm-hit (cached) for the Phase-0 scene set.
- `EV-PLATFORM-CONFORMANCE` covers the platform x GPU x backend matrix in `30-mvp-support-matrix.md`.

## Traceability
- `02-synesthesia-contracts.md` Sec.16, Sec.18
- `28-implementation-contract-pack.md` CP-10
- `30-mvp-support-matrix.md` API-stack row
- `ADR-002-rendering-backend.md` (supersedes backend-selection portion)
- `37-authoring-environment-spec.md` Sec.3.2 (autocomplete dialect), Sec.3.5 (inline diagnostics line mapping)
