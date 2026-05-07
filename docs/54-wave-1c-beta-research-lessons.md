<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

# Wave 1c-β research-lessons

**Generated:** 2026-05-07. Anchor: `main` HEAD `<TBD-on-merge>` (post-Phase-1
ADR drafts + Phase-1.5 multi-agent research pass).

This document captures implementation-level lessons surfaced by reading
ffmpeg, VLC, and the canonical Syphon-Framework source under public-OSS
posture (LGPL/GPL/BSD-3-Clause; pattern-extraction OK, source-copying
forbidden — `LicenseRef-Chromaticity-Proprietary` incompatible with GPL
+ requires attribution under BSD-3-Clause). Posture differs from
`Contents/` (closed-source competitor; clean-room SOP applies — see
`docs/29-cleanroom-legal-policy-baseline.md`).

The contract-level findings are folded into ADR-020 + ADR-021 directly
(amendments shipped alongside this doc). The implementation-level
findings below are Phase 2 + Phase 3 PR brief territory; PR authors
should cite the relevant section when scoping each PR.

## Posture

The reference repos read for this research pass were:

- **ffmpeg** — `libavdevice/avfoundation.m`, `libavdevice/dshow.{c,h}`,
  `libavdevice/dshow_filter.cpp`, `libavutil/hwcontext_videotoolbox.c`.
  Licensed LGPL 2.1+ with optional GPL 2+ components (the `--enable-gpl`
  build flag pulls libpostproc + some filters). Pattern extraction is
  freely permitted; chromaticity does not link against any ffmpeg
  artifact, and no source is copied. The lessons below describe
  patterns observed in the source — algorithms, state machines,
  pitfalls — and cite file:line for traceability, NOT for verbatim
  reproduction.

- **VLC** — `modules/access/avcapture.m` (macOS AVFoundation capture
  path), `modules/access/dshow/dshow.cpp`, `modules/access/dshow/filter.cpp`,
  `modules/access/dshow/filter.h`. Licensed GPL 2+. Same pattern-only
  posture as ffmpeg; chromaticity's `LicenseRef-Chromaticity-Proprietary`
  is incompatible with GPL re-distribution, so no source is copied —
  only patterns + cautionary precedents (regression commits, blocking
  patterns, conflations) are cited for design guidance.

- **canonical Syphon-Framework** — Tom Butterworth + Anton Marini's
  upstream framework source at `Syphon-Framework/SyphonServerBase.{h,m}`,
  `SyphonClientBase.{h,m}`, `SyphonMetalServer.{h,m}`, `SyphonMetalClient.{h,m}`,
  `SyphonServerRendererMetal.{h,m}`, `SyphonServerDirectory.{h,m}`,
  plus the `Exported_Symbols.exp` modulemap. Licensed BSD-3-Clause; the
  authoritative `License.txt` carries the no-endorsement clause that
  per-file headers shorthand-omit. Chromaticity's adapter is a Rust
  binding via `objc2` against the system-installed framework; the
  framework binary is redistributed under the BSD-3-Clause attribution
  + no-endorsement compliance posture documented in ADR-021 §License
  compliance.

**Source-copying forbidden, pattern extraction OK.** This is the
critical legal posture. The cited file:line references in this doc are
traceability anchors — a future reviewer can open the cited source to
verify the pattern claim, but chromaticity's implementation is written
from the contract derived from the pattern, NOT translated line-for-line
from the cited source. The fewer than three lines of paraphrased
pseudocode that appear below are stylized examples, not transcriptions.
Any future PR that reproduces more than ~3 lines verbatim from any of
the cited sources requires AO review.

## Phase 2 (camera adapter) implementation lessons

### Lesson 2.1 — Deque-swap drain semantics for the bounded MPSC poll path

**Pattern observed.** `vlc/modules/access/dshow/filter.cpp:244-256` (the
DirectShow capture filter's `GetNewSamples` method) implements a
**deque-swap drain**: the filter holds a `samples_queue` (a
`std::deque<VLCMediaSample>`); on the consumer's poll call, the filter
acquires the lock, swaps the entire deque into a local empty deque,
releases the lock, and returns the local. The producer side pushes new
samples into whichever deque the filter currently holds.

The deque-swap pattern minimizes lock-held time to a single pointer
swap (`std::deque::swap` is O(1)), avoiding the per-element copy that
naive "drain into vec" patterns incur. Producer + consumer race only
on the swap point; the rest of consumption happens lock-free on the
local deque the consumer now owns.

**Implication for chromaticity Phase 2 PR 1.** The `NokhwaInputAdapter`'s
`poll(timeout: Duration) -> Vec<CameraFrame>` method drains the bounded
MPSC channel between the nokhwa capture thread and the consumer-side
poll. nokhwa's `Camera::frame_raw` API exposes a single-frame poll, not
a drain — so the adapter wraps nokhwa's surface in a `crossbeam_channel`
(or `std::sync::mpsc`) bounded MPSC, and `poll` drains from that
channel. The drain pattern: take the receiver's lock, swap into a local
buffer, release. For chromaticity's MVP this maps to
`receiver.try_iter().collect::<Vec<_>>()` (which under the hood does
the equivalent), but the drain-not-copy intent is explicit. Future
optimization: if the receiver becomes a `VecDeque<CameraFrame>` behind
an `Arc<Mutex<...>>` (e.g. for the `queue_depth=1` single-slot case),
use `std::mem::swap(&mut local_deque, &mut *guard.deref_mut())` to
match VLC's pattern exactly.

**Confidence: high.** The pattern is well-established in lock-bounded
producer-consumer queues; the nokhwa-as-MPSC-wrap shape is the natural
chromaticity-side adoption.

### Lesson 2.2 — Async permission-flow contract (do NOT block on OS prompt)

**Pattern observed.** `vlc/modules/access/avcapture.m:420` blocks on
the macOS permission prompt via `dispatch_semaphore_wait(sema,
DISPATCH_TIME_FOREVER)`. The pattern wraps `[AVCaptureDevice
requestAccessForMediaType:AVMediaTypeVideo completionHandler:^(BOOL
granted) { ... dispatch_semaphore_signal(sema); }]` and waits on the
semaphore in the calling thread, synchronously returning the grant
result. This works for VLC's threaded demux model (a dedicated capture
thread can block without freezing the UI).

**Implication for chromaticity Phase 2 PR 2.** The same code in
chromaticity's Tauri main-thread-bound IPC handler would deadlock the
entire frontend — Tauri runs IPC handlers on the main thread, and
blocking the main thread on a UI-synchronous permission prompt freezes
the Tauri runtime that would otherwise process the prompt's
acknowledgment events. Contract: `enumerate()` returns immediately with
`AdapterHealth::Negotiating` if a TCC prompt is in flight; the OS
completion handler transitions the adapter health asynchronously to
`Bound` (on grant) or `Faulted("permission-denied")` (on deny). The
Tauri shell shows a non-modal "awaiting permission" state during
negotiation. ADR-020 §5 codifies this contract.

**Confidence: high.** The deadlock failure mode is testable; the
async-handler shape is the only viable Tauri-compatible option.

### Lesson 2.3 — Device-supplied timestamps; never wallclock

**Pattern observed.** `ffmpeg/libavdevice/dshow.c` commit
`7dc33aad45` (2021-12-21) reverted a five-year regression where
`av_gettime()` (wallclock-derived) had been substituted for the
device-supplied DirectShow sample timestamp. The regression surfaced
as inter-frame jitter for users with well-behaved devices: NTP slew
events would shift the wallclock by ~milliseconds while the device
emitted frames at hardware-cadence-stable intervals, producing
inter-frame deltas that did not match the device's actual frame rate.
The fix reads `pSample->GetTime(&start, &stop)` from the IMediaSample
and uses that as the canonical frame timestamp.

**Implication for chromaticity Phase 2 PR 1.** `CameraFrame::timestamp_ns`
sources from device-supplied timestamps where nokhwa exposes them
(`CMSampleBufferGetPresentationTimeStamp` on macOS,
`IMFSample::GetSampleTime` on Windows); fallback is
`std::time::Instant::now()` since adapter `bind` (monotonic, but
reset-from-bind, not the device's clock). Wallclock (`SystemTime::now()`,
`chrono::Utc::now()`) is **forbidden** for this field. Module-level
prose in `frames.rs` documents this as an invariant. ADR-020 §2 codifies
this as the timestamp policy.

**Confidence: high.** The ffmpeg commit history is the cautionary
precedent; the fix is well-known among AV-capture engineers.

### Lesson 2.4 — Format-name byte-order discipline (RGB/BGR multi-year saga)

**Pattern observed.** VLC's git log carries a multi-year sequence of
byte-order-correction commits: `VLC_CODEC_RGB24 → RGB24M → BGR24` over
months as the channel-order ambiguity surfaced on different platforms;
`MEDIASUBTYPE_ARGB32 → VLC_CODEC_BGRA` in the dshow filter
(`modules/access/dshow/dshow.cpp` historic commit log); `RGB32 → BGRX`
in the avcodec mapping table. Each correction fixed a wrong-color
regression on at least one platform. The root cause: format-name
ambiguity. "RGB24" can mean "byte order R, G, B" (the in-memory
interpretation) or "channels with R most-significant" (the
component-significance interpretation that DirectX and wgpu use).
Different consumers picked different interpretations, and the wrong one
shipped on at least one platform per cycle.

**Implication for chromaticity Phase 2 PR 1.** `PixelFormat` variants
name **in-memory byte order** explicitly. `Bgra32` means "bytes B, G,
R, A in memory" — not "ARGB with A most-significant". `Rgb24` means
"bytes R, G, B in memory, no alpha". Module-level prose in `frames.rs`
pins these definitions. ADR-020 §2 "Byte-order discipline" sub-section
codifies this; future widening of the `#[non_exhaustive]` enum reuses
the convention.

**Confidence: high.** The VLC git log is the historical evidence.

### Lesson 2.5 — Tauri Info.plist `NSCameraUsageDescription` release-validation step

**Pattern observed.** Tauri 2's plist merging has historically been
finicky around bundle-resource overrides. Multiple Tauri community
issues report that `bundle.macOS.entitlements` and Info.plist key
overrides specified in `tauri.conf.json` silently drop on certain
config-file shapes (whitespace differences, key-ordering issues with
`Info.plist.xml` template, etc.). For App Store submission and macOS
TCC compliance, missing `NSCameraUsageDescription` causes the macOS
prompt subsystem to skip the consent dialog entirely — the OS denies
access without surfacing a UI, leaving the app in a permission-denied
state with no operator-visible explanation.

**Implication for chromaticity Phase 2 PR 2.** A release-validation
step (or Phase 2 PR 2 reviewer-checklist line) asserts that the bundled
`.app/Contents/Info.plist` contains the `NSCameraUsageDescription` key
after `tauri build`. The check is a one-line `plutil -extract
NSCameraUsageDescription raw .../Info.plist || exit 1` invocation in
the release-validation script that fires before signing. The check is
binary (key present / absent) and does not validate the copy text — the
copy is reviewed manually for App Store compliance. ADR-020 §5
"Build-time verification" sub-section codifies this.

**Confidence: medium-high.** The Tauri plist-merge issues are
well-documented but not ubiquitous; the validation step is cheap
insurance against the silent-drop failure mode.

### Lesson 2.6 — nokhwa-as-MPSC-wrap if its threaded surface returns unbounded queue

**Pattern observed.** nokhwa's `CallbackCamera` (the threaded camera
surface) accepts a per-frame callback closure. The closure fires from
the capture thread on each new frame. Without a bounded buffer
between the callback and the consumer, a slow consumer accumulates
unbounded latency (or, with naive `Mutex<Vec<Frame>>` patterns,
unbounded memory).

**Implication for chromaticity Phase 2 PR 1.** Wrap the nokhwa
threaded surface in a bounded MPSC of `queue_depth` capacity. The
callback pushes to the channel; the consumer's `poll` drains it. On
full, the producer drops the **oldest** frame (replacing it with the
newest), not the newest — visualizers prefer fresh frames; stale
backlog adds latency that is worse than a single dropped frame. The
crossbeam_channel `bounded(queue_depth)` API + a custom send-with-drop
wrapper (`try_send` + on `Full`, drain one + retry) implements this.
ADR-020 §4 "Backpressure — bounded drop-on-full" codifies this.

**Confidence: high.** The bounded-with-drop-oldest pattern is
established (see ADR-019 §Decision §4 for the audio-uniform IPC bridge
precedent).

### Lesson 2.7 — Single-slot atomic exchange OK if depth=1 is acceptable (AVFoundation pattern)

**Pattern observed.** `ffmpeg/libavdevice/avfoundation.m` uses a
single-slot CMSampleBuffer cache (`AVFContext::current_frame`) that the
delegate replaces atomically when a new frame arrives; the consumer
reads + clears the slot. Effective queue depth = 1 (only the newest
frame is kept; if the consumer hasn't read the previous one, it is
overwritten). This is the simplest viable "drop-stale-keep-fresh"
pattern.

**Implication for chromaticity Phase 2 PR 1.** A `queue_depth = 1`
camera consumer (e.g. a live-show preview tile that always wants the
freshest frame and never cares about catching up) is a valid
configuration. The MPSC-with-bounded-drop pattern handles this case
naturally (`crossbeam_channel::bounded(1)` + the drop-oldest wrapper),
but for ultra-low-latency cases the single-slot atomic exchange (a
`std::sync::atomic::AtomicPtr<CameraFrame>` swapped on each producer
write) is even cheaper. MVP defaults to `DEFAULT_QUEUE_DEPTH = 4`; the
single-slot path is a future-wave option if the 4-frame buffer surfaces
as a latency floor.

**Confidence: medium.** The single-slot pattern is correct in principle
but the MVP shape is the bounded-MPSC; this lesson is forward-looking
optimization guidance, not Phase 2 PR 1 scope.

### Lesson 2.8 — Synthetic T2 corpus is strictly improving on zero (ffmpeg + VLC ship zero camera-capture unit tests)

**Pattern observed.** ffmpeg's `tests/` tree carries no
camera-capture unit tests (the `avdevice` tests focus on I/O format
parsers, not live device interaction). VLC's `test/` tree similarly
omits camera unit tests; live-device coverage is done manually on the
maintainer rigs. Both projects rely on user bug reports as the camera
regression-detection surface.

**Implication for chromaticity Phase 2 PR 4.** A synthetic T2 corpus
that exercises the adapter contract end-to-end with deterministic
synthetic frame producers is **strictly improving on zero**. Even
without live-hardware coverage, the synthetic corpus catches:

- State-machine regressions (bind / unbind / bind-while-bound /
  unbind-while-unbound transitions).
- Backpressure regressions (queue-depth overflow drops the oldest
  frame, not the newest).
- Reason-string regressions (the canonical fault vocabulary stays
  pinned across refactors).
- Format-negotiation regressions (the adapter falls back to offered
  formats correctly when the preference is unmet).

ADR-020 §Out of Scope reaffirms the synthetic-narrowing posture per
row 78 / row 40 / row 93 precedent. This lesson is the "why we still
ship the synthetic corpus despite no live-hardware CI" answer for the
Phase 2 PR 4 reviewer.

**Confidence: high.** The strictly-improving-on-zero argument is
empirically validated by the upstream test posture.

### Lesson 2.9 — Continuity / DeskView / External device-kind discriminator at enumeration layer

**Pattern observed.** `ffmpeg/libavdevice/avfoundation.m` enumerates
devices via `AVCaptureDeviceDiscoverySession` with a device-type list
that includes `AVCaptureDeviceTypeBuiltInWideAngleCamera`,
`AVCaptureDeviceTypeExternalUnknown`, and the macOS-specific
`AVCaptureDeviceTypeContinuityCamera` (iPhone-as-webcam) and
`AVCaptureDeviceTypeDeskViewCamera` (iPhone overhead-mount). The
enumeration returns a `position` + `deviceType` pair that lets the UI
group built-in vs external vs Continuity vs DeskView devices.

**Implication for chromaticity Phase 2 PR 1.** `DeviceSummary::kind`
covers `BuiltIn`, `External`, `Virtual` per ADR-020 §2 — the MVP
three-way grouping is sufficient for UI grouping, but Continuity and
DeskView are a fourth grouping operators may want to surface
distinctly (Continuity is on-the-fly, often appears + disappears as
the iPhone connects/disconnects). Future-wave option to widen
`CameraDeviceKind` with `Continuity` and `DeskView` variants if
operator feedback surfaces a need; the `#[non_exhaustive]` enum
shape allows additive widening without API break.

**Confidence: medium.** Continuity is macOS-only; the MVP three-way
grouping is the safe cross-platform default.

### Lesson 2.10 — `isInUseByAnotherApplication` precheck before access prompt

**Pattern observed.** `vlc/modules/access/avcapture.m:405-409`
checks `[AVCaptureDevice isInUseByAnotherApplication]` before
attempting to open the device. If true, the failure mode is
"device-busy" not "permission-denied" — the OS would not even fire the
permission prompt because the device is held by another process. Without
the precheck, the open call fails with a generic AVFoundation error
that's hard to distinguish from a permission denial, and the operator
gets a confusing "permission denied" banner when the actual fix is
"quit the other app holding the camera".

**Implication for chromaticity Phase 2 PR 2.** Before invoking the
`AVCaptureDevice.requestAccess` flow on macOS, the adapter checks
`isInUseByAnotherApplication` (via nokhwa's device-state surface, or
direct `objc2` query if nokhwa does not expose it). On busy, the
adapter routes to `AdapterHealth::Faulted("device-busy")` per the
canonical reason vocabulary in ADR-020 §2. The operator-facing UI
surface shows the busy-device banner ("Camera is in use by <App
Name>") instead of the permission-denied banner. ADR-020 §2's
canonical reason vocabulary explicitly separates `device-busy` from
`permission-denied` to enable this UX.

**Confidence: high.** The precheck pattern is well-documented in
AVFoundation docs; VLC's adoption is the precedent.

## Phase 3 (Syphon adapter) implementation lessons

### Lesson 3.1 — CVPixelBufferPool + CVMetalTextureCache canonical bridge

**Pattern observed.** `ffmpeg/libavutil/hwcontext_videotoolbox.c:213-218`
(VideoToolbox hardware context) shows the canonical bridge between
CVPixelBuffer (the CoreVideo surface type that backs IOSurface) and
Metal textures: a `CVMetalTextureCache` is created from the `MTLDevice`,
and `CVMetalTextureCacheCreateTextureFromImage` produces a
`CVMetalTexture` that wraps the underlying `MTLTexture` zero-copy. The
texture cache pre-allocates internal mappings so the per-frame call
is cheap (typically <100µs).

**Implication for chromaticity Phase 3 PR 3.** The wgpu ↔ IOSurface
bridge (`engine/render/src/syphon_adapter.rs`) uses
`wgpu::hal::metal::Device::texture_from_raw` to wrap the
`MTLTexture` produced by the cache. The cache is created once per
adapter binding (not per frame) and pre-warmed with a 2-3 surface
ring (see Lesson 3.3). ADR-021 §6 codifies the bridge contract.

**Confidence: high.** The CVMetalTextureCache pattern is the canonical
zero-copy CoreVideo↔Metal bridge documented by Apple.

### Lesson 3.2 — BGRA8 universal Syphon format

**Pattern observed.** `Syphon-Framework/SyphonMetalServer.m` and the
client side both default to `MTLPixelFormatBGRA8Unorm` /
`MTLPixelFormatBGRA8Unorm_sRGB` for the published surface. The reference
applications (Resolume, MadMapper, etc.) that interop via Syphon all
support BGRA8 as the lowest-common-denominator format. Other formats
are technically supported by IOSurface but are not portable across the
Syphon ecosystem — publishing a `MTLPixelFormatRGBA16Float` surface
works, but most consumers will not interpret it correctly.

**Implication for chromaticity Phase 3 PR 3.** The output side
publishes `wgpu::TextureFormat::Bgra8UnormSrgb` (matching the macOS
display surface format that the wgpu Metal backend already uses for
the swapchain). The input side similarly expects BGRA8 from
consumed servers; non-BGRA formats are observed but routed to a
diagnostic log without rendering until a per-format decoder lands
(post-MVP enhancement). ADR-021 §6 reaffirms BGRA8 as the canonical
format.

**Confidence: high.** The Syphon ecosystem convention is well
documented; the wgpu backend's BGRA8 default aligns naturally.

### Lesson 3.3 — Pre-warm 2-3 surface ring (CVPixelBufferPool minimum)

**Pattern observed.** Apple's CoreVideo guidance + the
`Syphon-Framework/SyphonServerRendererMetal.m` server-side rendering
path both use a `CVPixelBufferPool` with
`kCVPixelBufferPoolMinimumBufferCountKey` set to 2-3. The pool
pre-allocates the IOSurfaces so the per-frame allocation cost is
amortized to zero; without the pre-warm, the first 2-3 frames of a
new server pay an allocation cost (~5-10ms each) that surfaces as
visible jitter at session start.

**Implication for chromaticity Phase 3 PR 2 + PR 3.** The
`SyphonOutputAdapter::bind` operation pre-warms a 2-3 surface ring
before returning success. The `SyphonInputAdapter` does not own the
producer's pool but observes the convention indirectly via the
server's pre-warm. ADR-021 §6 implicitly assumes the pre-warm; the
PR 3 reviewer-checklist line should make it explicit ("the
adapter pre-warms a 2-3 IOSurface ring on bind to amortize first-frame
allocation cost").

**Confidence: high.** The CoreVideo pool minimum-count pattern is
documented by Apple.

### Lesson 3.4 — `addCompletedHandler:^{ [self publish]; }` GPU-completion synchronization

**Pattern observed.** `Syphon-Framework/SyphonMetalServer.m`
`publishFrameTexture:onCommandBuffer:imageRegion:flipped:` schedules
the publish call via `[commandBuffer addCompletedHandler:^{ ... }]` —
the publish happens after the GPU finishes rendering into the
texture, not synchronously when the publish call is made. Without this
gate, the consumer would observe partially-rendered surfaces (the GPU
is still writing into the IOSurface when the consumer reads it),
producing visible tearing or all-black frames.

**Implication for chromaticity Phase 3 PR 3.** The wgpu ↔ IOSurface
bridge routes `SyphonOutputAdapter::publish` through the wgpu command
buffer's completion handler. wgpu's `Queue::submit` returns a
`SubmissionIndex`; the adapter waits on
`Device::poll(Maintain::WaitForSubmissionIndex(idx))` (or uses the
wgpu callback API) before invoking the framework-side `publish`. The
publish is non-blocking from the render thread's perspective in steady
state because the wgpu submission has typically already completed by
the time the next frame's publish is called.

**Confidence: high.** The GPU-completion-gate pattern is the canonical
correctness invariant for any GPU-shared-surface protocol.

### Lesson 3.5 — `SyphonServerDirectory` polling on Tauri main thread (NSDistributedNotificationCenter)

**Pattern observed.** `Syphon-Framework/SyphonServerDirectory.m`
uses `NSDistributedNotificationCenter` to broadcast server
add/remove events across processes. The notification center requires
a runloop to deliver notifications — without an active runloop on the
observing thread, the notifications queue up and the observer never
sees them. macOS apps satisfy this via the main-thread NSRunLoop
that the AppKit framework runs by default.

**Implication for chromaticity Phase 3 PR 2.** The `discover_servers()`
poll runs on the Tauri main thread (which has an active runloop via
the Tauri/winit integration). If the poll were spawned to a worker
thread, the worker would need its own runloop (`CFRunLoopRun()`) for
the notifications to deliver — added complexity that the main-thread
poll avoids. ADR-021 §6 codifies render-thread affinity for framework
calls; the discovery poll respects this. The poll cadence is
`DEFAULT_DISCOVERY_INTERVAL_MS = 500` per ADR-021 §5.

**Confidence: high.** The NSDistributedNotificationCenter +
runloop dependency is documented in Apple's NSNotificationCenter
guide.

### Lesson 3.6 — Atomic-bool freshness flag pattern for cached wgpu-import

**Pattern observed.** `Syphon-Framework/SyphonMetalClient.m:40, 91-104`
maintains a `_hasNewFrame` atomic boolean that flips to true when the
server publishes a new frame and back to false when the client reads
it. The client's `hasNewFrame` getter is a simple atomic load; the
read-and-clear is a compare-and-swap. This avoids the overhead of a
full mutex for the common-case "is there a new frame?" check.

**Implication for chromaticity Phase 3 PR 2.** The `SyphonInputAdapter`
caches the most recently received `MTLTexture` (the wrapped one) and
flips an `AtomicBool` when a new frame arrives. `poll(timeout)` checks
the flag with a relaxed-ordering atomic load; on true, it
compare-and-swaps to false and returns the cached texture. The
adapter's poll path is lock-free in steady state. The same pattern
applies on the wgpu-side cache (the `wgpu::Texture` re-imported from
the `MTLTexture`); chromaticity caches the wgpu re-import once per
underlying IOSurface change, not once per frame.

**Confidence: high.** The atomic-bool freshness flag is a standard
single-producer / single-consumer pattern.

### Lesson 3.7 — CFType-as-Drop refcounting (`Retained<>` wrappers)

**Pattern observed.** Throughout the canonical Syphon-Framework, NS-
prefixed Objective-C objects are managed by ARC; CF-prefixed
CoreFoundation objects (`CVPixelBufferRef`, `CVMetalTextureCacheRef`,
`IOSurfaceRef`) are managed manually via `CFRetain`/`CFRelease`. The
two refcounting systems are bridged via `__bridge_retained` /
`__bridge_transfer` casts. Mixing manual `CFRelease` with ARC-managed
`Retained<>` wrappers in objc2 produces double-free or use-after-free
bugs.

**Implication for chromaticity Phase 3 PR 2.** Use objc2's
`Retained<>` for all NS-prefixed Objective-C objects (the framework
classes `SyphonServer`, `SyphonClient`, `SyphonServerDirectory`).
Use `core_foundation::base::TCFType`-style wrappers (or the
`core-foundation` crate) for CF-prefixed types. NEVER mix the two:
the `Drop` impl on each wrapper is responsible for its own release
path. ADR-021 §8 (Drop impl posture) documents the `Drop` calling
`[server stop]` defensively; the same Drop discipline applies to
CF-type wrappers calling `CFRelease`.

**Confidence: high.** The CFRelease vs ARC refcounting hazard is a
well-known Objective-C / Rust binding pitfall; the objc2 docs warn
about it explicitly.

### Lesson 3.8 — Raw IOSurface flag patterns (`kCVPixelBufferLock_ReadOnly` for consumer)

**Pattern observed.** Consumers reading from an IOSurface backing a
CVPixelBuffer call `CVPixelBufferLockBaseAddress(buf,
kCVPixelBufferLock_ReadOnly)` before reading and the matching
unlock after. The `_ReadOnly` flag is critical: without it, the lock
acquires write semantics (more expensive; fences against the GPU's
ongoing reads), and on some hardware paths it can stall the producer.
The Syphon-Framework consumer side uses `_ReadOnly` exclusively
because the consumer never writes the surface.

**Implication for chromaticity Phase 3 PR 2.** The `SyphonInputAdapter`
consumer-side bridge uses `kCVPixelBufferLock_ReadOnly` when accessing
the surface for the wgpu re-import. Module-level prose in
`syphon_adapter.rs` documents this as an invariant. The output side
does not lock — wgpu writes to the texture via the Metal backend, and
the publish gate (Lesson 3.4) handles the synchronization.

**Confidence: high.** The `_ReadOnly` flag pattern is documented in
Apple's CoreVideo programming guide.

### Lesson 3.9 — Symbol export list per modulemap (`Exported_Symbols.exp`)

**Pattern observed.** `Syphon-Framework/Exported_Symbols.exp` lists
the exact set of symbols the framework exports to consumers:
`SyphonServer`, `SyphonClient`, `SyphonServerDirectory`, the various
Metal subclasses, plus the `kSyphonServerDescription*Key` notification
names. Internal helpers (the various `*Base` classes) are not
exported. Consumers (chromaticity's objc2 bindings) MUST reference
only the exported symbols; relying on internal classes produces
linker failures with the system framework on minor version updates
because internal symbols may be renamed or removed without notice.

**Implication for chromaticity Phase 3 PR 1.** The objc2 binding
declarations in `integrations/syphon-adapter/src/adapter.rs::ffi`
target only the framework's exported symbols. The list of acceptable
classes and selectors is enumerated inline as a comment block (so a
future framework upgrade has a one-stop diff target). ADR-021 §2
"Framework binding strategy" implicitly assumes this; the PR 1
reviewer-checklist line should make the exported-symbols-only rule
explicit.

**Confidence: high.** The `Exported_Symbols.exp` modulemap is the
authoritative public-API surface of the framework.

### Lesson 3.10 — Mach IPC opaque (CFMessagePort underneath; objc2 wraps the API)

**Pattern observed.** Underneath the `SyphonServer` / `SyphonClient`
objc2-binding-friendly API, the framework uses `CFMessagePort` for
the cross-process bus messaging that announces server registration
and frame-published notifications. The Mach-IPC layer is an internal
implementation detail; `Exported_Symbols.exp` does not export the
CFMessagePort wrappers, and the canonical client/server API is what
consumers bind against.

**Implication for chromaticity Phase 3 PR 1 + PR 2.** Chromaticity's
objc2 bindings target the public Objective-C API
(`SyphonMetalServer`, `SyphonMetalClient`, `SyphonServerDirectory`).
NO raw Mach IPC code exists in the Rust adapter. The framework is the
single integration point; bypassing it to talk Mach-IPC directly
would (a) violate the framework's own internal contracts, (b)
re-implement the wheel, (c) require chromaticity to track Mach-IPC
protocol changes the framework abstracts. ADR-021 §2 codifies the
objc2 + framework binding strategy.

**Confidence: high.** The "use the documented public API, not the
internal IPC" rule is universal across framework integrations.

## Cross-cutting lessons (apply to both adapters)

### Lesson X.1 — Zero upstream unit tests; chromaticity's synthetic T2 corpus is strictly improving

**Pattern observed.** Both ffmpeg and VLC ship zero camera-capture
unit tests (the `tests/` and `test/` trees focus on parsers + codecs
+ container muxing/demuxing, not live device interaction). The
canonical Syphon-Framework's test suite is similarly thin (covers
the discovery dictionary lifecycle, not the cross-process IPC end-
to-end). Both ecosystems rely on user bug reports + manual maintainer-
rig validation as the regression-detection surface.

**Implication for chromaticity Phase 2 PR 4 + Phase 3 PR 2.** The
synthetic T2 corpus that chromaticity ships under the row-78 / row-40
/ row-93 synthetic-narrowing precedent is **strictly improving on
zero**. Even without live-hardware coverage, the synthetic corpus
catches state-machine + reason-string + format-negotiation
regressions that upstream relies on user bug reports to surface. The
narrowing is honest: chromaticity does NOT claim live-bus coverage,
but the contract-level coverage is real and material. ADR-020 +
ADR-021 §Out of Scope reaffirm this posture.

**Confidence: high.** Empirically validated by the upstream test
posture.

### Lesson X.2 — License attribution discipline (BSD-3-Clause vs LGPL/GPL)

**Pattern observed.** The three reference repos carry three different
license constraints:

- **ffmpeg (LGPL 2.1+ / GPL 2+ optional)**: pattern extraction is
  freely permitted under LGPL fair-use posture; chromaticity does
  not link against ffmpeg, so the LGPL dynamic-linking exception is
  moot. No attribution required for pattern-only extraction.
- **VLC (GPL 2+)**: source-copying is forbidden because chromaticity's
  `LicenseRef-Chromaticity-Proprietary` is GPL-incompatible (GPL
  requires the combined work to be GPL-licensed; chromaticity's
  proprietary license cannot satisfy that). Pattern extraction (idea,
  algorithm, state machine) is free under fair-use / 17 USC § 102(b)
  "ideas, procedures, processes, systems" exclusion from copyright.
- **Syphon-Framework (BSD-3-Clause)**: redistribution permitted under
  attribution + no-endorsement compliance; chromaticity links against
  + redistributes the framework binary, requiring full BSD-3-Clause
  compliance per ADR-021 §License compliance.

**Implication for chromaticity (all Wave 1c-β PRs).** The
`Contents/` clean-room SOP (`docs/29-cleanroom-legal-policy-baseline.md`)
applies ONLY to the closed-source competitor under reverse-engineering
posture. The Wave 1c-β reference repos are public-OSS and follow
their respective licenses. The discipline is:

- Cite file:line for traceability, NOT verbatim reproduction.
- Paraphrase patterns + cautionary precedents, never copy source.
- For Syphon-Framework, satisfy attribution + no-endorsement compliance
  per ADR-021 §License compliance at redistribution time.
- For ffmpeg + VLC, no attribution required (no linking, no source
  copying).

ADR-021 §License compliance + this lesson together form the
canonical guidance.

**Confidence: high.** The license analysis has been AO-reviewed.

### Lesson X.3 — Device-id persistence across sessions (MIDI-CONF-09 precedent)

**Pattern observed.** The MIDI-CONF-09 precedent (per `docs/26`
`EV-MIDI-EXTENDED` row, MIDI Learn save-quit-relaunch persistence
fixture) established the chromaticity pattern: device identifiers
persist across app sessions in app settings, and the adapter
auto-reconnects on startup if the persisted device is enumerable.
ffmpeg + VLC do not persist device-ids automatically — each session
re-enumerates and the user re-selects.

**Implication for chromaticity Phase 2 PR 1 + Phase 3 PR 2.**
Camera + Syphon adapters BOTH persist their bound-device id across
sessions. For camera: `CameraConfig::device_id` is a platform-stable
nokhwa identifier. For Syphon: `SyphonServerId::uuid` is the
canonical Syphon framework identifier. Both adapters auto-reconnect
on startup if the persisted id is in the current `enumerate()` /
`discover_servers()` output; absent → `AdapterHealth::Disconnected`
with re-enumeration on hardware change triggering an auto-reconnect
attempt. ADR-020 §6 codifies camera-side persistence; ADR-021's
discovery surface implies Syphon-side persistence (codified at
PR 2 time).

**Confidence: high.** The MIDI-CONF-09 precedent is the in-repo
authority.

### Lesson X.4 — Drop impl on Rust wrappers calling stop / unbind defensively

**Pattern observed.** Both the canonical Syphon-Framework
(`SyphonServer` registers a process-destructor in `+initialize` for
servers users forgot to `stop`) and the AVFoundation /
MediaFoundation backends (capture sessions hold OS-level resources
that leak if the session is not deterministically released) carry
defensive cleanup paths because users routinely forget to call the
stop/release method. The framework-side defensive cleanup is a
last-resort process-level finalizer.

**Implication for chromaticity (all Wave 1c-β adapter wrappers).**
Rust wrappers around Objective-C objects (`SyphonServer`,
`SyphonClient`, `AVCaptureSession` if exposed directly; nokhwa wraps
this) implement `Drop` calling the appropriate stop/release method
defensively. The Rust borrow checker enforces single-owner semantics,
so the `Drop` runs at scope exit deterministically — no relying on
process-level finalizers. ADR-021 §8 codifies Drop posture for
Syphon; analogous discipline applies to camera wrappers (nokhwa
already implements this internally for the `Camera` type).

**Confidence: high.** The defensive-Drop pattern is universal across
FFI bindings.

### Lesson X.5 — Operator-disambiguation of duplicate-name devices (DirectShow `" #N"` suffix)

**Pattern observed.** When multiple cameras of the same model are
plugged in (e.g. two identical Logitech C920s), DirectShow's
enumeration assigns them human-readable display names that collide
("Logitech HD Pro Webcam C920", "Logitech HD Pro Webcam C920"). VLC's
dshow filter (`modules/access/dshow/dshow.cpp`) appends a `" #N"`
suffix in the operator-facing display name for the second + later
collisions ("Logitech HD Pro Webcam C920 #2"). The unique device-id
under the hood is platform-stable (a path-like string), but the UI
needs help to disambiguate.

**Implication for chromaticity Phase 2 PR 5.** The
`DeviceSummary::display_name` is operator-facing; the UI surface (the
camera-source dropdown in the inputs view) MUST disambiguate
duplicates. Pattern: scan the enumeration output for collision groups,
append `" #2"`, `" #3"`, etc. to the second + later occurrences. The
underlying `DeviceSummary::device_id` (the nokhwa-stable string)
remains untouched — it is what `bind` consumes. The UI-side
disambiguation lives in the chrome layer (`src/lib/views/`), not the
adapter crate. ADR-020 §2 implicitly assumes this UI behavior; the
PR 5 reviewer-checklist line should make it explicit.

**Confidence: high.** The duplicate-name UX is a known
camera-input-UX failure mode; the suffix pattern is the established
remediation.

## Cross-references

- `docs/adr/ADR-020-camera-adapter.md` — camera contract (amended
  alongside this doc).
- `docs/adr/ADR-021-syphon-adapter.md` — Syphon contract (amended
  alongside this doc; BSD-3-Clause corrected from BSD-2-Clause).
- `docs/29-cleanroom-legal-policy-baseline.md` — clean-room posture
  (Wave 1c-β threat-model addendum applies to `Contents/`, NOT to
  the public-OSS reference repos this doc reads).
- `docs/adr/ADR-009-activation-and-entitlements.md` — `IO_CAMERA`
  stays-free permission policy.
- `docs/26-evidence-artifact-registry.md` — `EV-INTEGRATION-MATRIX`
  row + cumulative narrowings; Wave 1c-β widens the camera + Syphon
  arms at Phase 2 PR 5 + Phase 3 PR 3 closing-docs respectively.
- `docs/11-roadmap-phases.md` — Phase-4a Wave 1c-β sequencing this
  research pass supports.
- `docs/EV-GATE-MODEL-AUDIT-checklist.md` — Phase-4a EXIT closure
  section narrowing-clauses this wave addresses for the camera +
  Syphon arms.
- `docs/13-risk-register.md` — R-37 cross-platform integration scope
  risk this research pass partially mitigates by surfacing the
  cautionary precedents above.
- `/Users/xaviermac/Documents/2_Areas/Coding-Projects/reference-repos/{ffmpeg,vlc,Syphon-Framework}` —
  reference sources (NOT to be copied; pattern extraction OK under
  their respective licenses).

## When this doc is wrong

Lessons are derived from a 2026-05-07 multi-agent research pass. As
Phase 2 + Phase 3 progress through implementation, real-world
behavior may surface that some lessons don't apply or were superseded
(e.g. nokhwa upstream changes the threading model; Syphon-Framework
ships a major version with new exported symbols; macOS deprecates
the AVCaptureDeviceTypeContinuityCamera surface). Update this doc
in-place under a new `docs(repo)`-scoped commit; ADRs remain the
contract of record.
