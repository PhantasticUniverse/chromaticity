// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Standalone "live probe" — the LIVE-CAPTURE counterpart to the synthetic
//! [`mini_meter`](../mini_meter.rs). It opens the REAL input device through the
//! same [`chromaticity_audio::runtime::AudioRuntime`] the app uses, runs the
//! producer for a few seconds, and prints the capture-callback counters
//! (`cb` / `raw_peak`) alongside the analysis snapshot (rms / peak / bands).
//!
//! It answers the one question `mini_meter` and the analysis snapshot alone
//! CANNOT: *is the OS actually delivering capture buffers, or is the stream
//! silently dead?* A quiet room and a denied stream both read rms ≈ 0 in the
//! snapshot — only the `cb` callback counter distinguishes "buffers arriving,
//! full of silence" from "no buffers at all" (e.g. macOS TCC denying consent to
//! a binary with no `Info.plist`).
//!
//! Run (the `cb` counters require diagnostics ON):
//!
//! ```text
//! CHROMATICITY_AUDIO_DIAG=1 cargo run -p chromaticity-audio --example live_probe [device]
//! ```
//!
//! where `device` is `default` (host mic — the default), `system-audio` (the
//! zero-install output tap), or a device id/name. Play audio / make sound near
//! the input while it runs to see a level.
//!
//! macOS caveat: a bare `cargo run` binary may be denied microphone consent the
//! same way the raw app binary is — a `cb=0` verdict here from a terminal
//! WITHOUT mic permission IS the silent-denial signature. For a consent-capable
//! run use the signed `.app` via `release-eng/dev-run-macos.sh`.
//!
//! Exit code: `0` = real buffers arriving (capture alive), `2` = no real
//! capture (denied / stalled / no device), `3` = undetermined (diagnostics off).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chromaticity_audio::runtime::{AudioRuntime, AudioRuntimeConfig};
use chromaticity_audio::{
    AUDIO_DIAG_ENV, AudioAxisAvailability, AudioUniformSnapshot, diag_capture_stats,
    diagnostics_enabled,
};

/// How long to observe live capture.
const PROBE_SECONDS: u64 = 8;
/// Readout cadence.
const TICK: Duration = Duration::from_millis(250);
/// Amplitudes at or below this read as `-inf` dBFS (true silence). ~-100 dBFS.
const SILENCE_AMP: f32 = 1.0e-5;

/// Linear amplitude in `[0, 1]` → a fixed-width dBFS string (`-inf` for silence).
fn dbfs(amp: f32) -> String {
    if amp <= SILENCE_AMP {
        " -inf".to_string()
    } else {
        format!("{:>5.1}", 20.0 * amp.log10())
    }
}

/// Read the shared snapshot, recovering through a poisoned lock (mirrors the
/// producer / scheduler `match lock()` posture; no `.unwrap()` per workspace
/// clippy lints).
fn read_snapshot(handle: &Arc<Mutex<AudioUniformSnapshot>>) -> AudioUniformSnapshot {
    match handle.lock() {
        Ok(guard) => *guard,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

fn main() {
    let device = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "default".to_string());
    let diag_on = diagnostics_enabled();

    println!("=== Chromaticity live probe — REAL capture through AudioRuntime ===");
    println!(
        "device preference: {device:?}   observe: {PROBE_SECONDS}s, readout every {}ms",
        TICK.as_millis()
    );
    if diag_on {
        println!(
            "diagnostics: ON (cb / raw_peak live; a [audio-diag] bind line shows which device bound)"
        );
    } else {
        println!(
            "diagnostics: OFF — cb / raw_peak read 0. Re-run:\n  \
             {AUDIO_DIAG_ENV}=1 cargo run -p chromaticity-audio --example live_probe {device}"
        );
    }
    println!("Play audio / make sound near the input while this runs.\n");

    let runtime = AudioRuntime::with_preferred_device(Some(&device), AudioRuntimeConfig::default());
    let snapshot = runtime.snapshot_handle();
    let handle = runtime.spawn();

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(PROBE_SECONDS) {
        std::thread::sleep(TICK);
        let snap = read_snapshot(&snapshot);
        let stats = diag_capture_stats();
        println!(
            "  t={:>4.1}s  cb={:<8} raw_peak {}  |  rms {} peak {} dBFS   bass {:.2} mid {:.2} treb {:.2}",
            start.elapsed().as_secs_f32(),
            stats.callbacks,
            dbfs(stats.peak_amplitude),
            dbfs(snap.rms),
            dbfs(snap.peak),
            snap.bass,
            snap.mid,
            snap.treb,
        );
    }

    let snap = read_snapshot(&snapshot);
    let stats = diag_capture_stats();
    handle.shutdown();

    let producer_wrote = !matches!(snap.fft_availability, AudioAxisAvailability::Warming);
    println!("\n--- verdict ---");
    let code = if !diag_on {
        println!("[UNDETERMINED] diagnostics off — cannot read the capture-callback counters.");
        println!(
            "  analysis layer: producer {} (fft {:?}).",
            if producer_wrote {
                "wrote frames"
            } else {
                "never wrote a frame"
            },
            snap.fft_availability
        );
        println!("  re-run with {AUDIO_DIAG_ENV}=1 for a definitive cb-based verdict.");
        3
    } else if stats.callbacks == 0 {
        if producer_wrote {
            println!("[NO REAL DEVICE] zero capture callbacks but the producer wrote frames —");
            println!("  running on the SilentSource fallback (no input device was opened).");
        } else {
            println!("[DEAD CAPTURE] zero callbacks AND the producer never wrote a frame.");
            println!("  A device opened but the OS delivered NO buffers — almost always denied");
            println!("  capture consent (macOS: a binary with no Info.plist is denied SILENTLY).");
            println!("  The producer's fail-loud guard fires exactly this case.");
        }
        2
    } else if stats.peak_amplitude <= SILENCE_AMP {
        println!(
            "[ALIVE, SILENT] {} callbacks arrived but raw_peak is -inf — capture works;",
            stats.callbacks
        );
        println!("  nothing is playing into it. Make sound / play audio and re-run.");
        0
    } else {
        println!(
            "[HEALTHY] {} callbacks + finite raw_peak — real audio is flowing",
            stats.callbacks
        );
        println!(
            "  capture → analysis end-to-end (rms {} dBFS).",
            dbfs(snap.rms)
        );
        0
    };
    std::process::exit(code);
}
