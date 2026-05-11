// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity UI ↔ engine type bridge.
//!
//! Hosts the typed-IPC surface that crosses the Tauri command boundary:
//! serde + ts-rs newtype wrappers around the engine-side
//! [`chromaticity_runtime::ControlId`] / [`chromaticity_runtime::ControlValue`]
//! / [`chromaticity_runtime::SnapSlotId`] / [`chromaticity_runtime::PresetId`]
//! plus the [`SceneCard`] / [`SceneSource`] library-card shape and the
//! [`OscHealthSnapshot`] / [`OscHealthState`] adapter mirror, with `From`
//! impls in both directions so the Tauri handler converts at the boundary
//! and engine types stay free of IPC concerns.
//!
//! Post-Phase-3 status (`ec10ee7`): the bridge surface above is the
//! current ts-rs binding-generation source consumed by
//! `app/shell/tests/export_bindings.rs` and re-exported under
//! `src/lib/ipc/bindings/`. The next surface to land here is the W34+
//! adapter-pill IPC mirror; subsequent slices may add `Command` /
//! `Event` / `SnapshotPayload` shapes if the IPC contract grows beyond
//! the typed-newtype + scene-card + osc-health set.
//!
//! Wave 1c-α (PR `#339` / `c1d93e8`) added the
//! [`AudioUniformSnapshotPayload`] ts-rs newtype mirror of
//! [`chromaticity_audio::AudioUniformSnapshot`] per
//! `docs/adr/ADR-019-audio-uniform-ipc-bridge.md`. The payload's
//! producer-side emit step rides
//! [`chromaticity_runtime::scheduler::ipc::IpcEmit`]; the
//! consumer-side Svelte listener (`audioSnapshot.svelte.ts`) shipped
//! in PR `#340`. Wave 1c-α-2 production-bootstrap (PRs `#343` /
//! `#344`) closed the live producer→Tauri-emit→consumer wiring loop.
//!
//! See `docs/47-ui-port-plan.md` §7.2 for the bridge-crate role.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// IPC-facing wrapper for [`chromaticity_runtime::ControlId`].
///
/// Carries the serde + ts-rs derives needed to cross the Tauri command
/// boundary. Use `.into()` to convert to the engine-side type when
/// dispatching a `Command::ControlSet`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ControlId(pub u32);

impl From<ControlId> for chromaticity_runtime::ControlId {
    fn from(id: ControlId) -> Self {
        chromaticity_runtime::ControlId(id.0)
    }
}

impl From<chromaticity_runtime::ControlId> for ControlId {
    fn from(id: chromaticity_runtime::ControlId) -> Self {
        Self(id.0)
    }
}

/// IPC-facing wrapper for [`chromaticity_runtime::ControlValue`].
///
/// Carries the serde + ts-rs derives needed to cross the Tauri command
/// boundary. `f32` precludes `Eq`/`Hash`; use `PartialEq` for tests.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ControlValue(pub f32);

impl From<ControlValue> for chromaticity_runtime::ControlValue {
    fn from(v: ControlValue) -> Self {
        chromaticity_runtime::ControlValue(v.0)
    }
}

impl From<chromaticity_runtime::ControlValue> for ControlValue {
    fn from(v: chromaticity_runtime::ControlValue) -> Self {
        Self(v.0)
    }
}

/// IPC-facing wrapper for [`chromaticity_runtime::SnapSlotId`].
///
/// Carries the serde + ts-rs derives needed to cross the Tauri command
/// boundary. The engine-side type validates `0..=15` at construction
/// (`SnapSlotId::MAX == 15`); this newtype carries only the raw `u8`
/// because the wire shape is a single number per the ts-rs export.
///
/// Validation lives on the conversion edges, not the constructor:
/// `TryFrom<u8>` enforces `0..=15` when an IPC payload deserializes,
/// and `TryFrom<SnapSlotId> for chromaticity_runtime::SnapSlotId`
/// re-validates before the runtime type is constructed. The reverse
/// `From<chromaticity_runtime::SnapSlotId>` is always infallible
/// because the runtime constructor already enforces the invariant.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapSlotId(pub u8);

/// Returned by [`SnapSlotId`]'s fallible conversions when the slot
/// index lies outside `0..=chromaticity_runtime::SnapSlotId::MAX`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SnapSlotOutOfRange(pub u8);

impl core::fmt::Display for SnapSlotOutOfRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "snap slot {} out of range (max {})",
            self.0,
            chromaticity_runtime::SnapSlotId::MAX,
        )
    }
}

impl std::error::Error for SnapSlotOutOfRange {}

impl TryFrom<u8> for SnapSlotId {
    type Error = SnapSlotOutOfRange;

    fn try_from(slot: u8) -> Result<Self, Self::Error> {
        if slot <= chromaticity_runtime::SnapSlotId::MAX {
            Ok(Self(slot))
        } else {
            Err(SnapSlotOutOfRange(slot))
        }
    }
}

impl TryFrom<SnapSlotId> for chromaticity_runtime::SnapSlotId {
    type Error = SnapSlotOutOfRange;

    fn try_from(id: SnapSlotId) -> Result<Self, Self::Error> {
        chromaticity_runtime::SnapSlotId::new(id.0).ok_or(SnapSlotOutOfRange(id.0))
    }
}

impl From<chromaticity_runtime::SnapSlotId> for SnapSlotId {
    fn from(id: chromaticity_runtime::SnapSlotId) -> Self {
        Self(id.get())
    }
}

/// IPC-facing wrapper for [`chromaticity_runtime::PresetId`].
///
/// Carries the serde + ts-rs derives needed to cross the Tauri command
/// boundary. Mirrors [`ControlId`]'s transparent-newtype shape: the
/// engine-side type is a transparent `u32` newtype with no validator,
/// so both `From` impls are infallible.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PresetId(pub u32);

impl From<PresetId> for chromaticity_runtime::PresetId {
    fn from(id: PresetId) -> Self {
        chromaticity_runtime::PresetId(id.0)
    }
}

impl From<chromaticity_runtime::PresetId> for PresetId {
    fn from(id: chromaticity_runtime::PresetId) -> Self {
        Self(id.0)
    }
}

/// Provenance tag for a [`SceneCard`]: whether the package was
/// surfaced from the Tauri bundle (ship-time corpus) or from the
/// user's writable scene root.
///
/// Realises the W11 ship-time-bundling slice flagged by
/// `docs/52-parity-coverage-matrix.md` row 37 ("Bundled scenes under
/// `Resources/scenes/`"). The Library view reads the discriminant to
/// render the `BUILT-IN` chip on bundled cards; future Phase-4b work
/// (`EV-LIBRARY-BUNDLE-LAYOUT`) extends the bundled root to the full
/// `welcome / library / examples / shadertoy-fixtures` layout per
/// `docs/38-scene-library-and-import-spec.md` §2.1.
///
/// ts-rs renders the enum as a flat string union (`"Bundled" | "User"`)
/// because there are no payload-bearing variants, so the wire shape
/// is a single JSON string.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SceneSource {
    Bundled,
    User,
}

/// IPC-facing scene library card.
///
/// One entry per `.chroma` package surfaced by the W9 scene-library
/// directory scan. Carries the `docs/38-scene-library-and-import-spec.md`
/// §2.2 metadata (`TITLE`, `CREDIT`, `DESCRIPTION`, `TAGS`,
/// `CATEGORIES`) plus the package path and the format tag from §2.2's
/// catalog-metadata bullet — slice-1 only emits the `native-chroma-scene`
/// tag because the loader reads `.chroma` packages today.
///
/// The shape is intentionally a record struct (not a transparent
/// newtype): every field is independently consumed by the library
/// browser's grid / list / search surfaces. The trailing `source`
/// field (W11 PR2) tags the card's provenance — either the ship-time
/// Tauri bundle or the user's writable scene root — so the UI can
/// render a `BUILT-IN` chip without inferring from the path prefix.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct SceneCard {
    pub id: String,
    pub title: String,
    pub credit: String,
    pub description: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub path: String,
    pub format_tag: String,
    pub source: SceneSource,
}

/// IPC mirror of the W4 [`integrations::osc_adapter::OscHealthState`]
/// enum.
///
/// Surfaces the OSC adapter's bind-retry state machine to the UI per
/// `docs/08-io-integrations.md` §OSC. The W4 hardening pass added
/// `Healthy / Recovering / Degraded` to the internal adapter; this
/// IPC mirror lets the W14 OSC view render the operator-facing
/// adapter-health pill.
///
/// W14 scope: the stub `osc_health_snapshot` command in
/// `app/shell/src/commands/osc.rs` returns a placeholder
/// `Healthy / 0 attempted / unbound` snapshot since the OSC adapter
/// is not yet wired into [`AppState`]. Phase-3 wiring will replace
/// the stub with a real read-through to the live adapter.
///
/// ts-rs renders the enum as a flat string union
/// (`"Healthy" | "Recovering" | "Degraded"`).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum OscHealthState {
    Healthy,
    Recovering,
    Degraded,
}

/// IPC-facing default-value shape for a single control declared by a
/// `.chroma` scene's `CONTROLS` array.
///
/// Mirrors [`chromaticity_scene::schema::ControlValue`] across the
/// Tauri command boundary. ts-rs renders an `#[serde(untagged)]` enum
/// as a TypeScript union of the variant payload types, so the
/// frontend sees `boolean | number | [number, number] | [number,
/// number, number] | [number, number, number, number]` — the same
/// shape the wire format already carries.
///
/// Variant order mirrors the engine-side enum: `Bool` first so JSON
/// `true` / `false` matches before falling through to the numeric
/// path; the array variants discriminate by length.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(untagged)]
#[ts(export)]
pub enum ControlDefaultValuePayload {
    Bool(bool),
    Float(f64),
    Vec2([f64; 2]),
    Vec3([f64; 3]),
    Vec4([f64; 4]),
}

impl From<chromaticity_scene::schema::ControlValue> for ControlDefaultValuePayload {
    fn from(value: chromaticity_scene::schema::ControlValue) -> Self {
        match value {
            chromaticity_scene::schema::ControlValue::Bool(b) => Self::Bool(b),
            chromaticity_scene::schema::ControlValue::Float(f) => Self::Float(f),
            chromaticity_scene::schema::ControlValue::Vec2(v) => Self::Vec2(v),
            chromaticity_scene::schema::ControlValue::Vec3(v) => Self::Vec3(v),
            chromaticity_scene::schema::ControlValue::Vec4(v) => Self::Vec4(v),
            // Catch-all for the engine-side `#[non_exhaustive]` enum:
            // any future variant flows into `Float(0.0)` as the safe
            // fallback until the bridge surface is widened explicitly.
            _ => Self::Float(0.0),
        }
    }
}

impl From<ControlDefaultValuePayload> for chromaticity_scene::schema::ControlValue {
    fn from(payload: ControlDefaultValuePayload) -> Self {
        match payload {
            ControlDefaultValuePayload::Bool(b) => Self::Bool(b),
            ControlDefaultValuePayload::Float(f) => Self::Float(f),
            ControlDefaultValuePayload::Vec2(v) => Self::Vec2(v),
            ControlDefaultValuePayload::Vec3(v) => Self::Vec3(v),
            ControlDefaultValuePayload::Vec4(v) => Self::Vec4(v),
        }
    }
}

/// IPC-facing classification of a control's UI affordance, mirroring
/// [`chromaticity_scene::schema::ControlKind`] across the Tauri
/// command boundary.
///
/// ts-rs renders the enum as a flat string union
/// (`"Slider" | "Toggle" | ...`). Mirrors the engine-side enum's
/// `#[non_exhaustive]` posture by reproducing every existing variant
/// explicitly; future additions need a matching bridge variant
/// before the IPC surface widens.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ControlKindPayload {
    Slider,
    Toggle,
    Color,
    Xy,
    Bang,
    Dropdown,
    Knob,
}

impl From<chromaticity_scene::schema::ControlKind> for ControlKindPayload {
    fn from(kind: chromaticity_scene::schema::ControlKind) -> Self {
        match kind {
            chromaticity_scene::schema::ControlKind::Slider => Self::Slider,
            chromaticity_scene::schema::ControlKind::Toggle => Self::Toggle,
            chromaticity_scene::schema::ControlKind::Color => Self::Color,
            chromaticity_scene::schema::ControlKind::Xy => Self::Xy,
            chromaticity_scene::schema::ControlKind::Bang => Self::Bang,
            chromaticity_scene::schema::ControlKind::Dropdown => Self::Dropdown,
            chromaticity_scene::schema::ControlKind::Knob => Self::Knob,
            // Catch-all for the engine-side `#[non_exhaustive]` enum:
            // any future variant flows into `Slider` as the safe
            // fallback until the bridge surface is widened explicitly.
            _ => Self::Slider,
        }
    }
}

impl From<ControlKindPayload> for chromaticity_scene::schema::ControlKind {
    fn from(payload: ControlKindPayload) -> Self {
        match payload {
            ControlKindPayload::Slider => Self::Slider,
            ControlKindPayload::Toggle => Self::Toggle,
            ControlKindPayload::Color => Self::Color,
            ControlKindPayload::Xy => Self::Xy,
            ControlKindPayload::Bang => Self::Bang,
            ControlKindPayload::Dropdown => Self::Dropdown,
            ControlKindPayload::Knob => Self::Knob,
        }
    }
}

/// IPC-facing single-control declaration, mirroring a normalized
/// subset of [`chromaticity_scene::schema::ControlDecl`] across the
/// Tauri command boundary.
///
/// Lane A.1 only carries the fields the Perform view's Scene
/// Controls panel binds against (name + kind + default + min/max +
/// optional UI grouping label + description). The richer
/// `LABELS` / `VALUES` / `PARAMS` / `DEF_COLOR` set stays in the
/// engine-side `ControlDecl::extensions` map and rides forward
/// when Wave 2.2's control-widget refactor consumes them; mirroring
/// every field today would widen the bridge surface ahead of any
/// frontend consumer.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct ControlSchemaPayload {
    pub name: String,
    pub kind: Option<ControlKindPayload>,
    pub default: ControlDefaultValuePayload,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub ui_group: Option<String>,
    pub description: Option<String>,
    pub label: Option<String>,
}

impl From<chromaticity_scene::schema::ControlDecl> for ControlSchemaPayload {
    fn from(decl: chromaticity_scene::schema::ControlDecl) -> Self {
        Self {
            name: decl.name,
            kind: decl.kind.map(Into::into),
            default: decl.default.into(),
            min: decl.min,
            max: decl.max,
            ui_group: decl.ui_group,
            description: decl.description,
            label: decl.label,
        }
    }
}

/// IPC-facing loaded-scene snapshot returned by the `load_scene`
/// Tauri command.
///
/// One value per `.chroma` package the user has loaded into the
/// runtime via the Library view's double-click handler. The shape
/// mirrors the subset of [`chromaticity_scene::schema::Scene`] the
/// Perform view's Scene Controls header + control-panel surfaces
/// bind against per Wave 2.1 Lane A.1:
///
/// - `id` — file-stem of the `.chroma` directory (matches the
///   [`SceneCard::id`] surfaced by the library scan).
/// - `path` — absolute path to the `.chroma` package the runtime
///   loaded from.
/// - `name` — author-declared display name (`TITLE`), falling back
///   to `id` when the wire field is absent.
/// - `credit` / `description` — author metadata mirroring
///   [`SceneCard::credit`] / [`SceneCard::description`].
/// - `controls` — normalized control schema the Perform Scene
///   Controls panel binds against.
/// - `schema_version` — the `SCHEMA_VERSION` integer from the
///   compat-syn-v1 baseline (defaults to `1` when absent per the
///   engine-side [`SceneHeader::default`] contract).
///
/// Lane A.1 does NOT carry passes, transition descriptors, media
/// bindings, script lifecycle metadata, or the broader
/// `extensions` map; those ride forward when Wave 2.2's render
/// pipeline + control dispatch refactor needs them.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct LoadedScenePayload {
    pub id: String,
    pub path: String,
    pub name: String,
    pub credit: String,
    pub description: String,
    pub controls: Vec<ControlSchemaPayload>,
    pub schema_version: u32,
}

/// IPC-facing snapshot of OSC adapter health metrics.
///
/// The W14 OSC view reads this on mount to render the adapter
/// health pill + listen-address readout. Fields:
///
/// - `state` — the W4 `OscHealthState` discriminant, mirroring the
///   internal bind-retry state machine.
/// - `attempted_binds` — running count of bind attempts since the
///   adapter was constructed; consumed by the operator's debug
///   readout when adapter recovery is in progress.
/// - `local_addr` — `Some(<addr>)` when the adapter is bound;
///   `None` when unbound (initial state, or after a fatal
///   bind-loop exit). Stringified at the IPC boundary because
///   [`std::net::SocketAddr`] doesn't have a ts-rs binding.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct OscHealthSnapshot {
    pub state: OscHealthState,
    pub attempted_binds: u32,
    pub local_addr: Option<String>,
}

/// IPC-facing snapshot of the engine's
/// [`chromaticity_audio::AudioUniformSnapshot`] audio-analysis
/// uniforms.
///
/// The Wave 1c-α audio-uniform IPC bridge per
/// `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` carries the four
/// audio-analysis primitives (BPM, RMS, instantaneous peak, 8-band
/// log-spaced FFT magnitudes) across the Tauri command boundary so
/// the Svelte visualizer surface (Spectrum / BpmRing / BeatDots /
/// AudioReactivity) can replace its synthetic-time-domain stubs with
/// the real engine producer's data. Payload-shape parity with the
/// engine type is the load-bearing contract: field names, field
/// types, and field ordering must match
/// [`chromaticity_audio::AudioUniformSnapshot`] exactly so the
/// bidirectional `From` impls (and the JSON wire shape consumed by
/// the Tauri event channel) preserve the 11-field surface (one
/// `bpm`, one `rms`, one `peak`, eight `fft_bands`).
///
/// `Copy` rides for cheap once-per-tick emit copies; `f32` precludes
/// `Eq` / `Hash` so tests use `PartialEq` only (matching the
/// engine-side derive set).
///
/// The `8` in `fft_bands` re-derives the
/// [`chromaticity_audio::FFT_BAND_COUNT`] constant locally — the
/// duplication is intentional. ts-rs renders a fixed-length
/// `[f32; 8]` as a TypeScript tuple type with eight numeric slots
/// (one per band); referencing the constant via `[f32; FFT_BAND_COUNT]`
/// in the struct literal does not affect the ts-rs export and adds
/// a non-trivial transitive constant-resolution at the bridge edge.
/// The pair of T2 round-trip corpora pins the arity drift so a
/// future band-count change surfaces as a test failure rather than
/// silent wire-shape divergence.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq)]
#[ts(export)]
pub struct AudioUniformSnapshotPayload {
    pub bpm: f32,
    pub rms: f32,
    pub peak: f32,
    pub fft_bands: [f32; 8],
}

impl From<chromaticity_audio::AudioUniformSnapshot> for AudioUniformSnapshotPayload {
    fn from(snapshot: chromaticity_audio::AudioUniformSnapshot) -> Self {
        Self {
            bpm: snapshot.bpm,
            rms: snapshot.rms,
            peak: snapshot.peak,
            fft_bands: snapshot.fft_bands,
        }
    }
}

impl From<AudioUniformSnapshotPayload> for chromaticity_audio::AudioUniformSnapshot {
    fn from(payload: AudioUniformSnapshotPayload) -> Self {
        Self {
            bpm: payload.bpm,
            rms: payload.rms,
            peak: payload.peak,
            fft_bands: payload.fft_bands,
        }
    }
}

/// IPC-facing classification of a camera device, mirroring
/// [`chromaticity_camera_adapter::CameraDeviceKind`] across the Tauri
/// command boundary.
///
/// Wave 1c-β Phase 4 Lane A introduces this enum so the operator-facing
/// camera device-picker in `src/lib/views/Sources.svelte` can render
/// per-row kind badges (built-in vs external vs Continuity vs virtual)
/// without hard-coding the adapter's internal vocabulary in the
/// frontend. ts-rs renders the enum as a flat string union
/// (`"BuiltIn" | "External" | "Virtual" | "Continuity" | "Other"`)
/// because there are no payload-bearing variants.
///
/// Mirrors the `#[non_exhaustive]` posture of the engine-side enum;
/// the bridge wrapper reproduces every existing variant explicitly so
/// future additions (e.g. a macOS DeskView discriminator) need a
/// matching ts-rs variant before the IPC surface widens.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum CameraDeviceKindPayload {
    BuiltIn,
    External,
    Virtual,
    Continuity,
    Other,
}

impl From<chromaticity_camera_adapter::CameraDeviceKind> for CameraDeviceKindPayload {
    fn from(kind: chromaticity_camera_adapter::CameraDeviceKind) -> Self {
        match kind {
            chromaticity_camera_adapter::CameraDeviceKind::BuiltIn => Self::BuiltIn,
            chromaticity_camera_adapter::CameraDeviceKind::External => Self::External,
            chromaticity_camera_adapter::CameraDeviceKind::Virtual => Self::Virtual,
            chromaticity_camera_adapter::CameraDeviceKind::Continuity => Self::Continuity,
            // Catch-all for the engine-side `#[non_exhaustive]` enum:
            // any future variant flows into `Other` until the bridge
            // surface is widened explicitly.
            _ => Self::Other,
        }
    }
}

impl From<CameraDeviceKindPayload> for chromaticity_camera_adapter::CameraDeviceKind {
    fn from(payload: CameraDeviceKindPayload) -> Self {
        match payload {
            CameraDeviceKindPayload::BuiltIn => Self::BuiltIn,
            CameraDeviceKindPayload::External => Self::External,
            CameraDeviceKindPayload::Virtual => Self::Virtual,
            CameraDeviceKindPayload::Continuity => Self::Continuity,
            CameraDeviceKindPayload::Other => Self::Other,
        }
    }
}

/// IPC-facing camera device summary, mirroring
/// [`chromaticity_camera_adapter::DeviceSummary`] across the Tauri
/// command boundary.
///
/// Wave 1c-β Phase 4 Lane A: backs the `list_camera_devices` Tauri
/// command consumed by `src/lib/stores/cameraStore.svelte.ts`. Field
/// names mirror the engine-side struct verbatim
/// (`device_id` / `display_name` / `kind`) so the wire shape is a
/// 1:1 ts-rs export — the persisted device-id round-trips through the
/// bridge unchanged so reconnect against a previously selected camera
/// matches the platform-stable nokhwa identifier per ADR-020 §6.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct CameraDevicePayload {
    pub device_id: String,
    pub display_name: String,
    pub kind: CameraDeviceKindPayload,
}

impl From<chromaticity_camera_adapter::DeviceSummary> for CameraDevicePayload {
    fn from(summary: chromaticity_camera_adapter::DeviceSummary) -> Self {
        Self {
            device_id: summary.device_id,
            display_name: summary.display_name,
            kind: summary.kind.into(),
        }
    }
}

impl From<CameraDevicePayload> for chromaticity_camera_adapter::DeviceSummary {
    fn from(payload: CameraDevicePayload) -> Self {
        Self {
            device_id: payload.device_id,
            display_name: payload.display_name,
            kind: payload.kind.into(),
        }
    }
}

/// IPC-facing Syphon server identifier, mirroring
/// `chromaticity_syphon_adapter::SyphonServerId` across the Tauri
/// command boundary.
///
/// Wave 1c-β Phase 4 Lane B introduces this struct so the operator-
/// facing Syphon source-picker in `src/lib/views/Sources.svelte` can
/// render the discovered server list (UUID, name, app_name) plus
/// surface the `dictionary_version` value the canonical
/// `SyphonClientBase.m:87-93` forward-compat refusal pattern keys off
/// of. Field names mirror the engine-side struct verbatim per
/// ADR-021 §4 so the wire shape is a 1:1 ts-rs export.
///
/// macOS-only at the engine seam (the syphon-adapter crate body is
/// gated by `#![cfg(target_os = "macos")]` per ADR-021 §1), but this
/// payload type itself is unconditionally compiled — non-macOS Tauri
/// builds simply never construct it (the command handlers in
/// `app/shell/src/commands/syphon.rs` return an "unavailable on this
/// platform" error on those targets so the frontend store renders the
/// "Unavailable on this platform" empty-state branch).
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct SyphonServerPayload {
    pub uuid: String,
    pub name: String,
    pub app_name: String,
    pub dictionary_version: u32,
}

#[cfg(target_os = "macos")]
impl From<chromaticity_syphon_adapter::SyphonServerId> for SyphonServerPayload {
    fn from(id: chromaticity_syphon_adapter::SyphonServerId) -> Self {
        Self {
            uuid: id.uuid,
            name: id.name,
            app_name: id.app_name,
            dictionary_version: id.dictionary_version,
        }
    }
}

#[cfg(target_os = "macos")]
impl From<SyphonServerPayload> for chromaticity_syphon_adapter::SyphonServerId {
    fn from(payload: SyphonServerPayload) -> Self {
        Self {
            uuid: payload.uuid,
            name: payload.name,
            app_name: payload.app_name,
            dictionary_version: payload.dictionary_version,
        }
    }
}

/// IPC-facing Syphon output-publish status snapshot.
///
/// Wave 1c-β Phase 4 Lane B: backs the operator-facing
/// publish/unpublish toggle in `src/lib/views/Sources.svelte`. The
/// Tauri commands `publish_syphon_output` + `unpublish_syphon_output`
/// return one of these so the frontend store can drive the
/// non-dismissible trust-boundary warning banner per ADR-021 §10 +
/// `docs/29` Wave 1c-β threat-model addendum.
///
/// Fields:
///
/// - `active` — `true` when a Syphon output server is currently
///   registered on the bus; `false` after unpublish or before first
///   publish.
/// - `server_name` — `Some(name)` when active, carrying the
///   operator-supplied server name (default
///   [`DEFAULT_OUTPUT_NAME_TEMPLATE`]); `None` when inactive.
///
/// Default-off output posture per ADR-013 + ADR-016 default-off
/// telemetry precedent: the initial state read by the frontend on
/// view mount is `{active: false, server_name: None}` until the
/// operator explicitly opts in via the publish toggle.
///
/// [`DEFAULT_OUTPUT_NAME_TEMPLATE`]: chromaticity_syphon_adapter::DEFAULT_OUTPUT_NAME_TEMPLATE
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct SyphonOutputStatusPayload {
    pub active: bool,
    pub server_name: Option<String>,
}

impl SyphonOutputStatusPayload {
    /// Construct the inactive (default-off) output-status snapshot.
    /// Used by `unpublish_syphon_output` and as the initial mount-
    /// time state.
    #[must_use]
    pub fn inactive() -> Self {
        Self {
            active: false,
            server_name: None,
        }
    }

    /// Construct an active output-status snapshot carrying the
    /// operator-supplied server name. Used by `publish_syphon_output`.
    #[must_use]
    pub fn active(server_name: String) -> Self {
        Self {
            active: true,
            server_name: Some(server_name),
        }
    }
}

/// IPC-facing theme discriminant mirroring
/// [`chromaticity_runtime::settings::Theme`].
///
/// Wave 2.1 Lane C.3 settings persistence bridge: the operator's
/// three-theme cascade (Pulse / Atelier / Console) round-trips
/// through the bridge as a flat string union so the Tauri IPC
/// payload + the frontend `settingsStore` see the same wire shape.
/// ts-rs renders this as `"Pulse" | "Atelier" | "Console"`;
/// the engine-side `serde(rename_all = "kebab-case")` lives on
/// the engine enum where the on-disk TOML representation matters,
/// so the bridge edge translates between the IPC PascalCase variant
/// and the engine kebab-case TOML form via the `From` impls.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ThemePayload {
    Pulse,
    Atelier,
    Console,
}

impl From<chromaticity_runtime::settings::Theme> for ThemePayload {
    fn from(theme: chromaticity_runtime::settings::Theme) -> Self {
        match theme {
            chromaticity_runtime::settings::Theme::Pulse => Self::Pulse,
            chromaticity_runtime::settings::Theme::Atelier => Self::Atelier,
            chromaticity_runtime::settings::Theme::Console => Self::Console,
        }
    }
}

impl From<ThemePayload> for chromaticity_runtime::settings::Theme {
    fn from(payload: ThemePayload) -> Self {
        match payload {
            ThemePayload::Pulse => Self::Pulse,
            ThemePayload::Atelier => Self::Atelier,
            ThemePayload::Console => Self::Console,
        }
    }
}

/// IPC-facing audio preferences mirroring
/// [`chromaticity_runtime::settings::AudioSettings`].
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct AudioSettingsPayload {
    pub input_device: String,
    pub output_device: String,
    pub sample_rate_hz: u32,
    pub latency_hint: String,
    pub channel_layout: String,
    pub channel_left: String,
    pub channel_right: String,
    pub input_gain: f32,
}

impl From<chromaticity_runtime::settings::AudioSettings> for AudioSettingsPayload {
    fn from(s: chromaticity_runtime::settings::AudioSettings) -> Self {
        Self {
            input_device: s.input_device,
            output_device: s.output_device,
            sample_rate_hz: s.sample_rate_hz,
            latency_hint: s.latency_hint,
            channel_layout: s.channel_layout,
            channel_left: s.channel_left,
            channel_right: s.channel_right,
            input_gain: s.input_gain,
        }
    }
}

impl From<AudioSettingsPayload> for chromaticity_runtime::settings::AudioSettings {
    fn from(p: AudioSettingsPayload) -> Self {
        Self {
            input_device: p.input_device,
            output_device: p.output_device,
            sample_rate_hz: p.sample_rate_hz,
            latency_hint: p.latency_hint,
            channel_layout: p.channel_layout,
            channel_left: p.channel_left,
            channel_right: p.channel_right,
            input_gain: p.input_gain,
        }
    }
}

/// IPC-facing graphics preferences mirroring
/// [`chromaticity_runtime::settings::GraphicsSettings`].
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct GraphicsSettingsPayload {
    pub resolution_preset: String,
    pub custom_resolution_enabled: bool,
    pub custom_width: u32,
    pub custom_height: u32,
    pub fps_target: String,
    pub vsync: bool,
    pub hdr_target: String,
    pub display: String,
    pub color_space: String,
    pub multi_display_mode: String,
    pub render_scale: f32,
    pub screen_fitting: String,
    pub fps_slider_enabled: bool,
}

impl From<chromaticity_runtime::settings::GraphicsSettings> for GraphicsSettingsPayload {
    fn from(s: chromaticity_runtime::settings::GraphicsSettings) -> Self {
        Self {
            resolution_preset: s.resolution_preset,
            custom_resolution_enabled: s.custom_resolution_enabled,
            custom_width: s.custom_width,
            custom_height: s.custom_height,
            fps_target: s.fps_target,
            vsync: s.vsync,
            hdr_target: s.hdr_target,
            display: s.display,
            color_space: s.color_space,
            multi_display_mode: s.multi_display_mode,
            render_scale: s.render_scale,
            screen_fitting: s.screen_fitting,
            fps_slider_enabled: s.fps_slider_enabled,
        }
    }
}

impl From<GraphicsSettingsPayload> for chromaticity_runtime::settings::GraphicsSettings {
    fn from(p: GraphicsSettingsPayload) -> Self {
        Self {
            resolution_preset: p.resolution_preset,
            custom_resolution_enabled: p.custom_resolution_enabled,
            custom_width: p.custom_width,
            custom_height: p.custom_height,
            fps_target: p.fps_target,
            vsync: p.vsync,
            hdr_target: p.hdr_target,
            display: p.display,
            color_space: p.color_space,
            multi_display_mode: p.multi_display_mode,
            render_scale: p.render_scale,
            screen_fitting: p.screen_fitting,
            fps_slider_enabled: p.fps_slider_enabled,
        }
    }
}

/// IPC-facing storage preferences mirroring
/// [`chromaticity_runtime::settings::StorageSettings`].
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct StorageSettingsPayload {
    pub media_folders: Vec<String>,
    pub scene_folders: Vec<String>,
    pub backup_path: String,
    pub backup_retention: String,
}

impl From<chromaticity_runtime::settings::StorageSettings> for StorageSettingsPayload {
    fn from(s: chromaticity_runtime::settings::StorageSettings) -> Self {
        Self {
            media_folders: s.media_folders,
            scene_folders: s.scene_folders,
            backup_path: s.backup_path,
            backup_retention: s.backup_retention,
        }
    }
}

impl From<StorageSettingsPayload> for chromaticity_runtime::settings::StorageSettings {
    fn from(p: StorageSettingsPayload) -> Self {
        Self {
            media_folders: p.media_folders,
            scene_folders: p.scene_folders,
            backup_path: p.backup_path,
            backup_retention: p.backup_retention,
        }
    }
}

/// IPC-facing general preferences mirroring
/// [`chromaticity_runtime::settings::GeneralSettings`].
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export)]
pub struct GeneralSettingsPayload {
    pub theme: ThemePayload,
}

impl From<chromaticity_runtime::settings::GeneralSettings> for GeneralSettingsPayload {
    fn from(s: chromaticity_runtime::settings::GeneralSettings) -> Self {
        Self {
            theme: s.theme.into(),
        }
    }
}

impl From<GeneralSettingsPayload> for chromaticity_runtime::settings::GeneralSettings {
    fn from(p: GeneralSettingsPayload) -> Self {
        Self {
            theme: p.theme.into(),
        }
    }
}

/// IPC-facing operator preferences mirroring
/// [`chromaticity_runtime::settings::Settings`].
///
/// Wave 2.1 Lane C.3 settings persistence bridge: the
/// `get_settings` / `set_settings` Tauri commands route this payload
/// across the IPC boundary. Field-by-field shape parity with the
/// engine type is the load-bearing contract; the bridge mirror lives
/// in [`chromaticity_ui_bridge`] (this crate) so the engine stays
/// free of ts-rs / IPC concerns and the frontend bindings under
/// `src/lib/ipc/bindings/` generate from a single source.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct SettingsPayload {
    pub general: GeneralSettingsPayload,
    pub audio: AudioSettingsPayload,
    pub graphics: GraphicsSettingsPayload,
    pub storage: StorageSettingsPayload,
}

impl From<chromaticity_runtime::settings::Settings> for SettingsPayload {
    fn from(s: chromaticity_runtime::settings::Settings) -> Self {
        Self {
            general: s.general.into(),
            audio: s.audio.into(),
            graphics: s.graphics.into(),
            storage: s.storage.into(),
        }
    }
}

impl From<SettingsPayload> for chromaticity_runtime::settings::Settings {
    fn from(p: SettingsPayload) -> Self {
        Self {
            general: p.general.into(),
            audio: p.audio.into(),
            graphics: p.graphics.into(),
            storage: p.storage.into(),
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Wave-7 round-trip and edge-case coverage for the bridge newtypes.
    //!
    //! Pins: bidirectional `From`/`TryFrom` round-trips with the runtime
    //! types; `SnapSlotId` `0..=15` validation on both fallible edges
    //! (`TryFrom<u8>` and `TryFrom<SnapSlotId> for runtime::SnapSlotId`);
    //! transparent JSON shape (single scalar, no tuple wrapping) so the
    //! Tauri IPC payload matches the ts-rs `type X = number` exports;
    //! ts-rs type-name stability so an accidental rename surfaces here
    //! before the committed bindings drift.
    use super::{ControlId, ControlValue, PresetId, SnapSlotId, SnapSlotOutOfRange};
    use ts_rs::TS;

    #[test]
    fn control_id_runtime_round_trip_preserves_value() {
        let bridge = ControlId(7);
        let runtime: chromaticity_runtime::ControlId = bridge.into();
        assert_eq!(runtime, chromaticity_runtime::ControlId(7));
        let back: ControlId = runtime.into();
        assert_eq!(back, bridge);
    }

    #[test]
    fn control_value_runtime_round_trip_preserves_value() {
        let bridge = ControlValue(0.25);
        let runtime: chromaticity_runtime::ControlValue = bridge.into();
        assert_eq!(runtime, chromaticity_runtime::ControlValue(0.25));
        let back: ControlValue = runtime.into();
        assert_eq!(back, bridge);
    }

    #[test]
    fn preset_id_runtime_round_trip_preserves_value() {
        let bridge = PresetId(42);
        let runtime: chromaticity_runtime::PresetId = bridge.into();
        assert_eq!(runtime, chromaticity_runtime::PresetId(42));
        let back: PresetId = runtime.into();
        assert_eq!(back, bridge);
    }

    #[test]
    fn snap_slot_id_runtime_round_trip_preserves_value() {
        let runtime = chromaticity_runtime::SnapSlotId::new(5).unwrap();
        let bridge: SnapSlotId = runtime.into();
        assert_eq!(bridge, SnapSlotId(5));
        let back = chromaticity_runtime::SnapSlotId::try_from(bridge).unwrap();
        assert_eq!(back.get(), 5);
    }

    #[test]
    fn snap_slot_id_try_from_u8_accepts_zero_through_fifteen() {
        for slot in 0_u8..=chromaticity_runtime::SnapSlotId::MAX {
            assert_eq!(SnapSlotId::try_from(slot).unwrap(), SnapSlotId(slot));
        }
    }

    #[test]
    fn snap_slot_id_try_from_u8_rejects_sixteen_and_above() {
        assert_eq!(
            SnapSlotId::try_from(16_u8).unwrap_err(),
            SnapSlotOutOfRange(16),
        );
        assert_eq!(
            SnapSlotId::try_from(u8::MAX).unwrap_err(),
            SnapSlotOutOfRange(u8::MAX),
        );
    }

    #[test]
    fn snap_slot_id_try_into_runtime_rejects_out_of_range_payload() {
        // Models a hostile IPC payload that bypassed `TryFrom<u8>` by
        // constructing the `pub` tuple directly. The runtime-side
        // conversion MUST refuse the value rather than panic — this is
        // the latent-panic eviction the W7 audit called out.
        let hostile = SnapSlotId(16);
        let result = chromaticity_runtime::SnapSlotId::try_from(hostile);
        assert_eq!(result.unwrap_err(), SnapSlotOutOfRange(16));
    }

    #[test]
    fn control_id_json_round_trip_uses_transparent_scalar() {
        let json = serde_json::to_string(&ControlId(7)).unwrap();
        assert_eq!(json, "7");
        let parsed: ControlId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ControlId(7));
    }

    #[test]
    fn control_value_json_round_trip_uses_transparent_scalar() {
        let json = serde_json::to_string(&ControlValue(0.5)).unwrap();
        assert_eq!(json, "0.5");
        let parsed: ControlValue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ControlValue(0.5));
    }

    #[test]
    fn snap_slot_id_json_round_trip_uses_transparent_scalar() {
        let json = serde_json::to_string(&SnapSlotId(3)).unwrap();
        assert_eq!(json, "3");
        let parsed: SnapSlotId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SnapSlotId(3));
    }

    #[test]
    fn preset_id_json_round_trip_uses_transparent_scalar() {
        let json = serde_json::to_string(&PresetId(99)).unwrap();
        assert_eq!(json, "99");
        let parsed: PresetId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PresetId(99));
    }

    #[test]
    fn ts_rs_exported_names_match_committed_bindings() {
        assert_eq!(<ControlId as TS>::name(), "ControlId");
        assert_eq!(<ControlValue as TS>::name(), "ControlValue");
        assert_eq!(<SnapSlotId as TS>::name(), "SnapSlotId");
        assert_eq!(<PresetId as TS>::name(), "PresetId");
    }
}
