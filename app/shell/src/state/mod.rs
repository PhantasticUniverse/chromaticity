// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell-side application state managed by Tauri.
//!
//! Composes the canonical [`chromaticity_runtime::state::State`] with an
//! [`chromaticity_runtime::InMemoryBus`]. The shell holds the only
//! mutable reference to `State` behind a [`Mutex`]; Tauri command
//! handlers acquire the lock, dispatch through the bus, and apply
//! committed events via [`chromaticity_runtime::InMemoryBus::commit_one_into`].
//!
//! Phase-1 D2 scope: a single-window shell whose state is owned by one
//! Tauri-managed [`AppState`]. Multi-window or worker-thread variants
//! land with the slice that introduces them.

use chromaticity_runtime::InMemoryBus;
use chromaticity_runtime::state::State;
use std::sync::Mutex;

/// Tauri-managed shell state.
pub struct AppState {
    /// Canonical runtime state guarded behind a mutex so command
    /// handlers can apply committed events serially.
    pub state: Mutex<State>,
    /// In-memory command bus — the single producer/commit seam the
    /// shell uses to mutate `state`.
    pub bus: InMemoryBus,
}

impl AppState {
    /// Construct a fresh shell state with a default canonical
    /// [`State`] and a freshly-instantiated [`InMemoryBus`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            bus: InMemoryBus::in_memory(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
