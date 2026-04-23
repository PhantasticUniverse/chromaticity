// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Chromaticity desktop application shell entrypoint.
// Minimal Tauri 2 bootstrap at Phase-1 entry; real command handlers,
// shared-type registration (via ts-rs), and window choreography arrive
// in subsequent PRs per ADR-018.

/// Launch the Tauri application event loop.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
