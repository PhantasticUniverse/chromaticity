// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Opt-in, world-class live audio-capture telemetry for the terminal.
//!
//! Off by default. When the operator sets the `CHROMATICITY_AUDIO_DIAG`
//! env var (see [`chromaticity_audio::AUDIO_DIAG_ENV`]) to anything other
//! than `0` / `false` / `off`, [`spawn`] runs a low-priority reader thread
//! that prints a compact, dBFS-scaled readout of the live
//! [`AudioUniformSnapshot`] at [`READOUT_HZ`]. It reads the *same*
//! `Arc<Mutex<AudioUniformSnapshot>>` the scheduler-tick consumer pulls
//! from, so it reports exactly what the rest of the app sees — never a
//! separate capture path.
//!
//! Pair it with the bind-time `[audio-diag]` line emitted by
//! `engine/audio` (which device + direction + negotiated format bound):
//! together they answer "did system-audio loopback bind, and is it
//! delivering signal or silence?" without a debugger.
//!
//! The render is split into pure, unit-tested helpers
//! ([`amp_to_dbfs`], [`meter_bar`], [`fft_sparkline`], [`format_readout`])
//! so the formatting is verified independently of the thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chromaticity_audio::{AudioAxisAvailability, AudioUniformSnapshot, DiagCaptureStats};

/// Readout cadence (Hz). 4 Hz is smooth enough to watch a meter respond
/// to program audio without flooding a captured log.
pub const READOUT_HZ: u64 = 4;

/// Horizontal dBFS meter width in cells.
const BAR_WIDTH: usize = 20;

/// dBFS floor for the meter scale. Matches the frontend metering floor
/// (`src/lib/audio/metering.ts` `METER_FLOOR_DB`) so the CLI and the UI
/// speak the same scale.
const FLOOR_DB: f32 = -60.0;

/// Amplitudes at or below this map to `-inf` dBFS (true silence). 1e-5 is
/// ~-100 dBFS — well below the -60 floor, so a genuinely silent tap reads
/// `-inf`, not a misleadingly-finite floor value.
const SILENCE_AMP: f32 = 1.0e-5;

/// Convert a linear amplitude in `[0, 1]` to dBFS. Returns
/// [`f32::NEG_INFINITY`] for (near-)silence so the readout can show a
/// distinct `-inf` rather than a clamped floor number.
pub fn amp_to_dbfs(amp: f32) -> f32 {
    if amp <= SILENCE_AMP {
        f32::NEG_INFINITY
    } else {
        20.0 * amp.log10()
    }
}

/// Format a dBFS value into a fixed 5-wide field: `-18.3`, ` -6.0`, or
/// ` -inf` for silence.
pub fn format_dbfs(db: f32) -> String {
    if db.is_infinite() {
        " -inf".to_string()
    } else {
        format!("{db:>5.1}")
    }
}

/// Render a left-to-right dBFS meter bar of `width` cells over the
/// `[FLOOR_DB, 0]` range, using 1/8-cell Unicode sub-blocks for the
/// leading partial cell. Always returns exactly `width` characters.
pub fn meter_bar(db: f32, width: usize) -> String {
    const EIGHTHS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    let frac = if db.is_infinite() && db < 0.0 {
        0.0
    } else {
        ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0)
    };
    let total_eighths = (frac * (width as f32) * 8.0).round() as usize;
    let full = (total_eighths / 8).min(width);
    let rem = total_eighths % 8;
    let mut bar = String::with_capacity(width * 3);
    for _ in 0..full {
        bar.push('█');
    }
    let mut used = full;
    if rem > 0 && used < width {
        bar.push(EIGHTHS[rem]);
        used += 1;
    }
    for _ in used..width {
        bar.push('·');
    }
    bar
}

/// Render the 8 FFT bands as an 8-cell sparkline using block characters.
/// Each band magnitude in `[0, 1]` maps to one of eight heights.
pub fn fft_sparkline(bands: &[f32]) -> String {
    const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    bands
        .iter()
        .map(|&b| {
            let idx = (b.clamp(0.0, 1.0) * 7.0).round() as usize;
            LEVELS[idx.min(7)]
        })
        .collect()
}

/// Coarse capture state for the readout: `DEGRADED` when the snapshot
/// adapter is in its recovered-last-known state, `WARMING` while either
/// windowed axis is still filling, else `READY`.
pub fn state_label(snapshot: &AudioUniformSnapshot) -> &'static str {
    if snapshot.degraded {
        "DEGRADED"
    } else if matches!(snapshot.fft_availability, AudioAxisAvailability::Warming)
        || matches!(snapshot.bpm_availability, AudioAxisAvailability::Warming)
    {
        "WARMING"
    } else {
        "READY"
    }
}

/// Assemble one readout line for a snapshot.
pub fn format_readout(snapshot: &AudioUniformSnapshot) -> String {
    let rms_db = amp_to_dbfs(snapshot.rms);
    let peak_db = amp_to_dbfs(snapshot.peak);
    let bpm = if matches!(snapshot.bpm_availability, AudioAxisAvailability::Ready) {
        format!("{:>3.0}", snapshot.bpm)
    } else {
        " --".to_string()
    };
    format!(
        "[audio-diag] {state:<8} rms {rms} peak {peak} dBFS  [{bar}]  bpm {bpm}  fft {fft}  \
         bands b{bass:.2} m{mid:.2} t{treb:.2}  fftpk {fftpk:.3}",
        state = state_label(snapshot),
        rms = format_dbfs(rms_db),
        peak = format_dbfs(peak_db),
        bar = meter_bar(rms_db, BAR_WIDTH),
        fft = fft_sparkline(&snapshot.fft_bands),
        bass = snapshot.bass,
        mid = snapshot.mid,
        treb = snapshot.treb,
        fftpk = snapshot.fft_bands.iter().copied().fold(0.0_f32, f32::max),
    )
}

/// Render the raw capture-callback probe counters: how many cpal data
/// callbacks have fired and the all-time peak sample amplitude in dBFS.
/// `cb=0` while audio plays isolates a no-buffers (consent / tap-start)
/// failure from a buffers-arriving-but-silent one — the analyser snapshot
/// alone cannot tell them apart.
pub fn format_capture_stats(stats: DiagCaptureStats) -> String {
    format!(
        "cb={callbacks:<7} raw_peak {peak} dBFS",
        callbacks = stats.callbacks,
        peak = format_dbfs(amp_to_dbfs(stats.peak_amplitude)),
    )
}

/// Spawn the detached telemetry reader thread if diagnostics are enabled.
/// Reads `snapshot` every `1/READOUT_HZ` s and prints a [`format_readout`]
/// line to stderr until `shutdown` flips. A failure to spawn is non-fatal
/// (logged, then skipped) — diagnostics never gate the app.
pub fn spawn(snapshot: Arc<Mutex<AudioUniformSnapshot>>, shutdown: Arc<AtomicBool>) {
    let builder = thread::Builder::new().name("chromaticity-audio-diag".into());
    let spawned = builder.spawn(move || {
        eprintln!(
            "[audio-diag] live capture telemetry ON ({READOUT_HZ} Hz) — rms/peak in dBFS, \
             {BAR_WIDTH}-cell meter over [{FLOOR_DB:.0}, 0]; set {env}=0 to silence",
            env = chromaticity_audio::AUDIO_DIAG_ENV,
        );
        let period = Duration::from_millis(1000 / READOUT_HZ);
        while !shutdown.load(Ordering::Relaxed) {
            let snap = match snapshot.lock() {
                Ok(guard) => *guard,
                Err(poisoned) => *poisoned.into_inner(),
            };
            let stats = chromaticity_audio::diag_capture_stats();
            eprintln!("{}  {}", format_readout(&snap), format_capture_stats(stats));
            thread::sleep(period);
        }
        eprintln!("[audio-diag] telemetry stopped");
    });
    if let Err(err) = spawned {
        eprintln!("[audio-diag] failed to spawn telemetry thread: {err}");
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    use super::*;

    fn snapshot() -> AudioUniformSnapshot {
        AudioUniformSnapshot {
            bpm: 0.0,
            bpm_availability: AudioAxisAvailability::Warming,
            rms: 0.0,
            peak: 0.0,
            fft_bands: [0.0; 8],
            fft_availability: AudioAxisAvailability::Warming,
            // Reactive bands (ADR-024) default to the neutral 1.0; this
            // diagnostics helper exercises the dBFS math, not the bands.
            ..Default::default()
        }
    }

    #[test]
    fn dbfs_reference_points() {
        assert!((amp_to_dbfs(1.0) - 0.0).abs() < 1e-4);
        assert!((amp_to_dbfs(0.1) - (-20.0)).abs() < 1e-3);
        assert!((amp_to_dbfs(0.001) - (-60.0)).abs() < 1e-3);
        assert!(amp_to_dbfs(0.0).is_infinite());
        assert!(amp_to_dbfs(0.0) < 0.0);
    }

    #[test]
    fn format_dbfs_silence_and_value() {
        assert_eq!(format_dbfs(f32::NEG_INFINITY), " -inf");
        assert_eq!(format_dbfs(-18.34), "-18.3");
        assert_eq!(format_dbfs(-6.0), " -6.0");
    }

    #[test]
    fn meter_bar_is_exactly_width_chars() {
        for db in [-120.0_f32, -60.0, -30.0, -9.0, -0.1, 0.0] {
            assert_eq!(
                meter_bar(db, BAR_WIDTH).chars().count(),
                BAR_WIDTH,
                "db={db}"
            );
        }
        assert_eq!(
            meter_bar(f32::NEG_INFINITY, BAR_WIDTH).chars().count(),
            BAR_WIDTH
        );
    }

    #[test]
    fn meter_bar_endpoints() {
        // Full scale → all full blocks; floor/silence → all empty cells.
        assert_eq!(meter_bar(0.0, 8), "████████");
        assert_eq!(meter_bar(-60.0, 8), "········");
        assert_eq!(meter_bar(f32::NEG_INFINITY, 8), "········");
        // Half scale lights roughly half the bar.
        assert!(meter_bar(-30.0, 8).starts_with('█'));
    }

    #[test]
    fn fft_sparkline_endpoints_and_length() {
        assert_eq!(fft_sparkline(&[0.0; 8]), "▁▁▁▁▁▁▁▁");
        assert_eq!(fft_sparkline(&[1.0; 8]), "████████");
        assert_eq!(
            fft_sparkline(&[1.5, -0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
                .chars()
                .count(),
            8
        );
    }

    #[test]
    fn state_label_transitions() {
        let mut s = snapshot();
        assert_eq!(state_label(&s), "WARMING");
        s.fft_availability = AudioAxisAvailability::Ready;
        s.bpm_availability = AudioAxisAvailability::Ready;
        assert_eq!(state_label(&s), "READY");
        s.degraded = true;
        assert_eq!(state_label(&s), "DEGRADED");
    }

    #[test]
    fn readout_shape_silent_vs_signal() {
        let silent = snapshot();
        let line = format_readout(&silent);
        assert!(line.contains("[audio-diag]"));
        assert!(line.contains("WARMING"));
        assert!(line.contains(" -inf"));
        assert!(line.contains("bpm  --"));

        let mut hot = snapshot();
        hot.rms = 0.5;
        hot.peak = 0.7;
        hot.fft_bands = [0.4; 8];
        hot.fft_availability = AudioAxisAvailability::Ready;
        hot.bpm_availability = AudioAxisAvailability::Ready;
        hot.bpm = 128.0;
        let line = format_readout(&hot);
        assert!(line.contains("READY"));
        assert!(line.contains("bpm 128"));
        assert!(!line.contains("-inf"));
    }

    #[test]
    fn capture_stats_render() {
        let none = DiagCaptureStats {
            callbacks: 0,
            samples: 0,
            peak_amplitude: 0.0,
        };
        let line = format_capture_stats(none);
        assert!(line.contains("cb=0"));
        assert!(
            line.contains("-inf"),
            "silent probe shows -inf raw_peak: {line}"
        );

        let hot = DiagCaptureStats {
            callbacks: 4096,
            samples: 524288,
            peak_amplitude: 0.5,
        };
        let line = format_capture_stats(hot);
        assert!(line.contains("cb=4096"));
        assert!(
            !line.contains("-inf"),
            "non-silent probe shows a dB value: {line}"
        );
    }
}
