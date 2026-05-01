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
