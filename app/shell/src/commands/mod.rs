// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Tauri `#[command]` handlers exposed to the frontend.
//!
//! Phase-1 D2 ships the control round-trip in [`control`]; D3 layers
//! the Snap{Recall, Save, Clear} round-trip in [`snap`]. Subsequent
//! slices grow this surface with playlist, preset, and observability
//! bridges.

pub mod control;
pub mod snap;
