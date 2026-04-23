// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match chromaticity_app_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("fatal: chromaticity-app runtime error: {err}");
            ExitCode::FAILURE
        }
    }
}
