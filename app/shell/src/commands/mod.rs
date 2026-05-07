// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Tauri `#[command]` handlers exposed to the frontend.
//!
//! Phase-1 D2 ships the control round-trip in [`control`]; D3 layers
//! the Snap{Recall, Save, Clear} round-trip in [`snap`]; D4 adds the
//! preset-recall round-trip in [`preset`] and the playlist-advance
//! round-trip in [`playlist`]. Subsequent slices grow this surface
//! with observability bridges.

pub mod camera;
pub mod control;
pub mod library;
pub mod library_index;
pub mod library_search;
pub mod license;
pub mod osc;
pub mod picker;
pub mod playlist;
pub mod preset;
pub mod snap;
pub mod thumbnail;
