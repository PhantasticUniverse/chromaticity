// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Standalone "mini meter" — exercises the real engine reactive-band analysis
//! ([`chromaticity_audio::ReactiveBands`], the MilkDrop-class AGC) against known
//! synthetic signals, with NO Tauri app, macOS tap, TCC consent, or GUI. The
//! fix transfers directly because this drives the same engine code the app's
//! `AudioRuntime` will run.
//!
//! Run:  `cargo run -p chromaticity-audio --example mini_meter`
//!
//! It prints a live-style readout per scenario plus a rubric scorecard. The
//! load-bearing property is **volume independence**: the same tone at very
//! different amplitudes must converge to the same relative band value (~1.0),
//! which is what makes reactivity look good on loud AND quiet material.

use std::f32::consts::TAU;

use chromaticity_audio::{REACTIVE_FFT_SIZE, ReactiveBandValues, ReactiveBands};

const SAMPLE_RATE: u32 = 44_100;
/// One analysis block of wall-clock time (FFT window / sample rate).
const DT: f32 = REACTIVE_FFT_SIZE as f32 / SAMPLE_RATE as f32;
const BAR_WIDTH: usize = 24;

fn sine(freq_hz: f32, amp: f32, start_sample: u64, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = (start_sample + i as u64) as f32 / SAMPLE_RATE as f32;
            amp * (TAU * freq_hz * t).sin()
        })
        .collect()
}

/// Drive a steady tone for `blocks` analysis blocks; return the converged values.
fn converge(freq_hz: f32, amp: f32, blocks: usize) -> ReactiveBandValues {
    let mut rb = ReactiveBands::new(SAMPLE_RATE);
    let mut last = ReactiveBandValues::default();
    for blk in 0..blocks {
        let buf = sine(
            freq_hz,
            amp,
            blk as u64 * REACTIVE_FFT_SIZE as u64,
            REACTIVE_FFT_SIZE,
        );
        last = rb.observe(&buf, DT);
    }
    last
}

/// Drive a broadband mix (80 + 1000 + 8000 Hz) for `blocks` blocks — energy in
/// every band, the realistic music-like case the AGC is designed for.
fn converge_broadband(blocks: usize) -> ReactiveBandValues {
    let mut rb = ReactiveBands::new(SAMPLE_RATE);
    let mut last = ReactiveBandValues::default();
    for blk in 0..blocks {
        let start = blk as u64 * REACTIVE_FFT_SIZE as u64;
        let mut buf = vec![0.0_f32; REACTIVE_FFT_SIZE];
        for (freq, amp) in [(80.0_f32, 0.3_f32), (1_000.0, 0.3), (8_000.0, 0.3)] {
            for (i, s) in sine(freq, amp, start, REACTIVE_FFT_SIZE).iter().enumerate() {
                buf[i] += *s;
            }
        }
        last = rb.observe(&buf, DT);
    }
    last
}

/// A 0..2 relative-value bar (1.0 = centre = "normal").
fn bar(value: f32) -> String {
    let filled = ((value / 2.0) * BAR_WIDTH as f32)
        .round()
        .clamp(0.0, BAR_WIDTH as f32) as usize;
    let mut s = String::with_capacity(BAR_WIDTH + 2);
    s.push('[');
    for i in 0..BAR_WIDTH {
        s.push(if i < filled { '#' } else { '·' });
    }
    s.push(']');
    s
}

fn print_values(label: &str, v: &ReactiveBandValues) {
    println!("  {label}");
    println!("    bass {:>5.2} {}", v.bass, bar(v.bass));
    println!("    mid  {:>5.2} {}", v.mid, bar(v.mid));
    println!("    treb {:>5.2} {}", v.treb, bar(v.treb));
    println!(
        "    att: bass {:.2}  mid {:.2}  treb {:.2}",
        v.bass_att, v.mid_att, v.treb_att
    );
}

fn check(label: &str, pass: bool, detail: &str) -> bool {
    println!(
        "  [{}] {label}: {detail}",
        if pass { "PASS" } else { "FAIL" }
    );
    pass
}

fn main() {
    println!("=== Chromaticity mini meter — engine reactive-band analysis (no app) ===\n");
    println!(
        "Signals at {SAMPLE_RATE} Hz, {REACTIVE_FFT_SIZE}-sample blocks (~{:.1} ms/block).",
        DT * 1000.0
    );
    println!("Relative bands centre on ~1.0 (MilkDrop AGC); bar spans 0..2.\n");

    // --- Scenario readouts -------------------------------------------------
    println!("steady 80 Hz @ 0.50 (bass tone):");
    print_values("converged", &converge(80.0, 0.50, 400));
    println!("steady 2 kHz @ 0.50 (mid tone):");
    print_values("converged", &converge(2_000.0, 0.50, 400));
    println!("steady 8 kHz @ 0.50 (treb tone):");
    print_values("converged", &converge(8_000.0, 0.50, 400));
    println!("  (note: a PURE tone's silent bands read leakage-noise, not ~1.0 —");
    println!("   real broadband music has energy in every band; see below.)");

    // Broadband: energy in all three bands -> the realistic music-like case,
    // where the AGC normalises every band cleanly to ~1.0.
    println!("steady broadband (80 + 1k + 8k Hz, mixed) — the realistic case:");
    print_values("converged", &converge_broadband(400));

    // --- Rubric scorecard --------------------------------------------------
    println!("\n--- rubric scorecard ---");
    let mut all = true;

    // Volume independence (#1) — the AGC's defining property.
    let quiet = converge(2_000.0, 0.02, 400).mid;
    let loud = converge(2_000.0, 0.80, 400).mid;
    all &= check(
        "volume independence",
        (quiet - loud).abs() < 0.10 && (quiet - 1.0).abs() < 0.25,
        &format!(
            "2 kHz mid: quiet@0.02={quiet:.2}  loud@0.80={loud:.2}  Δ={:.3}",
            (quiet - loud).abs()
        ),
    );

    // Band selectivity (#6) — visible in the transient phase (the AGC self-
    // normalises every active band to ~1.0 at steady state).
    let b = converge(80.0, 0.50, 6);
    let t = converge(8_000.0, 0.50, 6);
    all &= check(
        "band selectivity (transient onset)",
        b.bass > b.treb + 0.3 && t.treb > t.bass + 0.3,
        &format!(
            "80 Hz bass={:.2}>treb={:.2}; 8 kHz treb={:.2}>bass={:.2}",
            b.bass, b.treb, t.treb, t.bass
        ),
    );

    // Silence gating / centering (#8).
    let mut rb = ReactiveBands::new(SAMPLE_RATE);
    let mut sil = ReactiveBandValues::default();
    for _ in 0..120 {
        sil = rb.observe(&[0.0; REACTIVE_FFT_SIZE], DT);
    }
    all &= check(
        "silence centres on 1.0",
        (sil.bass - 1.0).abs() < 1e-3 && (sil.mid - 1.0).abs() < 1e-3,
        &format!(
            "bass={:.3} mid={:.3} treb={:.3}",
            sil.bass, sil.mid, sil.treb
        ),
    );

    // Transient response (#3) — spike after a quiet baseline.
    let mut rb = ReactiveBands::new(SAMPLE_RATE);
    for blk in 0..200 {
        let buf = sine(
            2_000.0,
            0.05,
            blk as u64 * REACTIVE_FFT_SIZE as u64,
            REACTIVE_FFT_SIZE,
        );
        rb.observe(&buf, DT);
    }
    let burst = rb.observe(
        &sine(
            2_000.0,
            0.90,
            200 * REACTIVE_FFT_SIZE as u64,
            REACTIVE_FFT_SIZE,
        ),
        DT,
    );
    all &= check(
        "transient spikes above baseline",
        burst.mid > 1.5 && burst.mid_att <= burst.mid + 1e-3,
        &format!(
            "burst mid={:.2} (att {:.2}, smoother)",
            burst.mid, burst.mid_att
        ),
    );

    println!(
        "\n{}",
        if all {
            "ALL RUBRIC CHECKS PASS — reactive-band AGC behaves world-class on synthetic signals."
        } else {
            "SOME CHECKS FAILED — see above."
        }
    );
    if !all {
        std::process::exit(1);
    }
}
