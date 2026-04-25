// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity UI ↔ engine type bridge.
//!
//! Phase-1 D1 scaffold. Hosts the typed-IPC surface that crosses the
//! Tauri command boundary: serde + ts-rs newtype wrappers around the
//! engine-side [`chromaticity_runtime::ControlId`] and
//! [`chromaticity_runtime::ControlValue`], with `From` impls in both
//! directions so the Tauri handler converts at the boundary and engine
//! types stay free of IPC concerns.
//!
//! Subsequent slices add `Command`, `Event`, `SnapshotPayload`, and
//! other bridge-facing surfaces as the IPC contract grows.
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
/// (`SnapSlotId::MAX == 15`); this newtype carries only the raw `u8`.
///
/// Contract: callers MUST validate the slot via
/// `chromaticity_runtime::SnapSlotId::new(...)` at the IPC boundary
/// BEFORE constructing this wrapper or invoking the
/// `From<SnapSlotId> for chromaticity_runtime::SnapSlotId` impl. The
/// reverse impl from the engine type is always safe.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SnapSlotId(pub u8);

impl From<SnapSlotId> for chromaticity_runtime::SnapSlotId {
    // The IPC handler validates the slot via
    // `chromaticity_runtime::SnapSlotId::new(...)` BEFORE constructing
    // this wrapper, so the conversion is infallible by contract. The
    // targeted `expect_used` allow documents the invariant.
    #[allow(clippy::expect_used)]
    fn from(id: SnapSlotId) -> Self {
        chromaticity_runtime::SnapSlotId::new(id.0)
            .expect("SnapSlotId already validated at the IPC boundary")
    }
}

impl From<chromaticity_runtime::SnapSlotId> for SnapSlotId {
    fn from(id: chromaticity_runtime::SnapSlotId) -> Self {
        Self(id.get())
    }
}
