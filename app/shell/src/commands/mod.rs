// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Tauri `#[command]` handlers exposed to the frontend.
//!
//! Phase-1 D2 ships the first IPC contract: the control round-trip in
//! [`control`]. Subsequent slices grow this surface with playlist,
//! preset, Snap, and observability bridges.

pub mod control;
