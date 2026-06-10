// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! dt-aware audio ballistics for the GPU output window.
//!
//! The render thread reads a fresh [`AudioUniformSnapshot`] every frame
//! and feeds the reactive axes straight to the scene shader. Raw, those
//! axes are jumpy: the analysis window updates at its own cadence and the
//! values lurch frame-to-frame, so the reactive motion reads as both
//! laggy (a transient takes several frames to fully land) and jittery (it
//! flickers on the way back down).
//!
//! [`AudioSmoother`] interposes a per-channel **asymmetric one-pole
//! follower** (the same ballistics idea the UI meters use):
//!
//! - **instant attack** toward a rising target (transient axes use
//!   `tau = 0`, i.e. snap) kills perceived lag — a kick lands the same
//!   frame the engine reports it;
//! - **slow release** toward a falling target kills jitter — the value
//!   eases back down instead of chattering.
//!
//! This is the classic PPM-meter ballistic: a value that only ever JUMPS
//! up and DECAYS down never chatters, so instant attack adds no jitter —
//! it is the slow *release*, not a damped attack, that buys the smoothness.
//!
//! The follower is **dt-aware**: its coefficient is `1 - exp(-dt/tau)`,
//! so the smoothing is frame-rate-independent (a 30 fps frame and two
//! 60 fps frames converge to the same place). `tau` is the time constant
//! in seconds; smaller `tau` = snappier.
//!
//! ## Why instant attack (benchmark, 2026-06-05)
//!
//! The engine AGC (`engine/audio::reactive_bands`) ALREADY applies a
//! fast-attack short EMA before this stage, and the reference engines
//! (MilkDrop / butterchurn) feed that straight to the shader with **no
//! second smoothing stage at all**. A render-side *attack* tau here
//! therefore stacks a second delay onto an already-fast onset — pure
//! latency the references never pay. So the transient axes (peak / FFT /
//! reactive bands) use `attack = 0` (snap), matching the references'
//! directness, while the **slow release** (kept) is the extra smoothness
//! layer they lack. RMS — the body/energy axis — keeps a small attack so
//! it stays a touch smoother than the transient axes. Releases are NOT
//! tuned here: that slow-release decay is the behaviour the AO praised.
//! This removed the ~10 ms render-side attack term from the ~62 ms
//! band-axis audio→pixel budget; see
//! `docs/dev/audio-reactivity-latency-budget.md`.
//!
//! ## No `unsafe`
//!
//! Pure float arithmetic; the workspace `unsafe_code = "forbid"` lint is
//! inherited.

use chromaticity_audio::AudioUniformSnapshot;

/// Number of FFT bands smoothed, mirroring
/// [`AudioUniformSnapshot::fft_bands`].
const FFT_BAND_COUNT: usize = 8;

// --- Tau constants (SECONDS) -------------------------------------------
//
// Transient axes snap up (attack = 0, true PPM) and ease down on the slow
// release; RMS keeps a small attack as the body axis. Smaller tau =
// faster. The RELEASES own the praised smoothness — do NOT shorten them.
// See the module doc for why the attacks are (near-)instant.

/// RMS (window energy) attack — small, not zero: RMS is the body/energy
/// axis, so it stays a touch smoother on onset than the transient axes
/// (was 30 ms; the benchmark shaved ~20 ms of RMS-axis onset lag here).
const RMS_ATTACK: f32 = 0.009;
/// RMS release — long, for a smooth body-level decay.
const RMS_RELEASE: f32 = 0.250;

/// Peak attack — instant (`0` ⇒ snap): peaks ARE the transients.
const PEAK_ATTACK: f32 = 0.0;
/// Peak release — medium, so a hit rings down smoothly.
const PEAK_RELEASE: f32 = 0.160;

/// FFT-band attack — instant (`0` ⇒ snap): per-band transients pop.
const FFT_ATTACK: f32 = 0.0;
/// FFT-band release — medium.
const FFT_RELEASE: f32 = 0.160;

/// Reactive-band attack — instant (`0` ⇒ snap): the bass/mid/treb that
/// drive the main motion land the frame the engine reports them.
const BAND_ATTACK: f32 = 0.0;
/// Reactive-band release — medium-short, keeping the relative bands lively.
const BAND_RELEASE: f32 = 0.130;

/// dt-aware one-pole follower: fast attack toward a RISING target,
/// slow release toward a FALLING target. `coeff = 1 - exp(-dt/tau)`.
struct OnePole {
    current: f32,
}

impl OnePole {
    fn new() -> Self {
        Self { current: 0.0 }
    }

    fn follow(
        &mut self,
        target: f32,
        dt_secs: f32,
        attack_tau_secs: f32,
        release_tau_secs: f32,
    ) -> f32 {
        let tau = if target >= self.current {
            attack_tau_secs
        } else {
            release_tau_secs
        };
        // Guard tau>0; a non-positive dt or tau snaps to target.
        let coeff = if tau > 0.0 && dt_secs > 0.0 {
            1.0 - (-dt_secs / tau).exp()
        } else {
            1.0
        };
        self.current += (target - self.current) * coeff;
        self.current
    }
}

/// Per-channel ballistics state for the render thread's audio uniforms.
///
/// Holds one [`OnePole`] follower per smoothed axis. Construct once
/// (outside the loop) with [`AudioSmoother::new`] and call
/// [`AudioSmoother::smooth`] once per frame; the followers carry their
/// state across frames.
pub struct AudioSmoother {
    rms: OnePole,
    peak: OnePole,
    fft: [OnePole; FFT_BAND_COUNT],
    bass: OnePole,
    mid: OnePole,
    treb: OnePole,
    bass_att: OnePole,
    mid_att: OnePole,
    treb_att: OnePole,
}

impl AudioSmoother {
    /// Build a fresh smoother with every follower settled at `0.0`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rms: OnePole::new(),
            peak: OnePole::new(),
            fft: [
                OnePole::new(),
                OnePole::new(),
                OnePole::new(),
                OnePole::new(),
                OnePole::new(),
                OnePole::new(),
                OnePole::new(),
                OnePole::new(),
            ],
            bass: OnePole::new(),
            mid: OnePole::new(),
            treb: OnePole::new(),
            bass_att: OnePole::new(),
            mid_att: OnePole::new(),
            treb_att: OnePole::new(),
        }
    }

    /// Apply fast-attack/slow-release ballistics to the reactive axes of
    /// `raw`, returning a smoothed copy.
    ///
    /// `dt_secs` is the wall-clock time since the previous frame (the
    /// render loop's `time_delta`). `bpm`, the availability enums, and
    /// `degraded` are copied through untouched — they are not continuous
    /// reactive values and must not be ballistically filtered.
    ///
    /// [`AudioUniformSnapshot`] is `Copy`, so the input is copied (never
    /// cloned) into the returned smoothed snapshot.
    pub fn smooth(&mut self, raw: &AudioUniformSnapshot, dt_secs: f32) -> AudioUniformSnapshot {
        let mut out = *raw;

        out.rms = self.rms.follow(raw.rms, dt_secs, RMS_ATTACK, RMS_RELEASE);
        out.peak = self
            .peak
            .follow(raw.peak, dt_secs, PEAK_ATTACK, PEAK_RELEASE);

        for (band, follower) in out.fft_bands.iter_mut().zip(self.fft.iter_mut()) {
            *band = follower.follow(*band, dt_secs, FFT_ATTACK, FFT_RELEASE);
        }

        out.bass = self
            .bass
            .follow(raw.bass, dt_secs, BAND_ATTACK, BAND_RELEASE);
        out.mid = self.mid.follow(raw.mid, dt_secs, BAND_ATTACK, BAND_RELEASE);
        out.treb = self
            .treb
            .follow(raw.treb, dt_secs, BAND_ATTACK, BAND_RELEASE);
        out.bass_att = self
            .bass_att
            .follow(raw.bass_att, dt_secs, BAND_ATTACK, BAND_RELEASE);
        out.mid_att = self
            .mid_att
            .follow(raw.mid_att, dt_secs, BAND_ATTACK, BAND_RELEASE);
        out.treb_att = self
            .treb_att
            .follow(raw.treb_att, dt_secs, BAND_ATTACK, BAND_RELEASE);

        out
    }
}

impl Default for AudioSmoother {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Ballistics pins for the dt-aware one-pole follower. All pure
    //! arithmetic over small fixed `dt` values — no `Instant` / wall
    //! clock.

    use super::OnePole;

    /// Tolerance for float comparisons (never compare floats with `==`).
    const EPS: f32 = 1e-4;

    #[test]
    fn follower_converges_toward_constant_target() {
        // Drive a constant target for many steps; the follower must land
        // on it (a stable one-pole converges geometrically).
        let mut f = OnePole::new();
        let target = 0.8_f32;
        for _ in 0..2_000 {
            f.follow(target, 0.016, 0.030, 0.250);
        }
        assert!(
            (f.current - target).abs() < EPS,
            "follower converged to {} not {target}",
            f.current,
        );
    }

    #[test]
    fn attack_is_faster_than_release_asymmetry() {
        // From a settled low value, a single attack step toward a higher
        // target must move MORE than a single release step back down the
        // same distance (attack_tau < release_tau).
        let attack_tau = 0.010_f32;
        let release_tau = 0.250_f32;
        let dt = 0.016_f32;

        // Rise: settled at 0.0, target 1.0 (one big upward step).
        let mut rising = OnePole::new();
        let after_rise = rising.follow(1.0, dt, attack_tau, release_tau);
        let rise_delta = after_rise - 0.0;

        // Fall: settled at 1.0, target 0.0 (one big downward step of the
        // same magnitude).
        let mut falling = OnePole { current: 1.0 };
        let after_fall = falling.follow(0.0, dt, attack_tau, release_tau);
        let fall_delta = 1.0 - after_fall;

        assert!(
            rise_delta > fall_delta + EPS,
            "attack step ({rise_delta}) should exceed release step ({fall_delta})",
        );
    }

    #[test]
    fn dt_awareness_two_half_steps_match_one_full_step() {
        // Two follow() calls of dt=0.008 toward a fixed target should land
        // close to one call of dt=0.016 toward the same target. The
        // coefficient is 1 - exp(-dt/tau), which composes exactly for a
        // CONSTANT target (the residual decays multiplicatively), so the
        // two paths agree to within a small epsilon.
        let target = 1.0_f32;
        let tau = 0.030_f32;

        // Single full step.
        let mut one = OnePole::new();
        let single = one.follow(target, 0.016, tau, tau);

        // Two half steps.
        let mut two = OnePole::new();
        two.follow(target, 0.008, tau, tau);
        let double = two.follow(target, 0.008, tau, tau);

        assert!(
            (single - double).abs() < EPS,
            "one full dt step ({single}) and two half steps ({double}) should agree",
        );
    }

    #[test]
    fn zero_attack_snaps_up_but_release_still_eases() {
        // Transient-axis design (PPM): attack tau 0 ⇒ a rising target lands
        // in ONE step (instant attack), while a falling target still eases
        // on the slow release. Pins the "snap up, decay down" ballistic the
        // benchmark prescribed for the peak / FFT / reactive-band axes.
        let mut f = OnePole::new();
        let up = f.follow(1.0, 0.016, 0.0, 0.130); // rising, attack = 0
        assert!(
            (up - 1.0).abs() < EPS,
            "zero attack should snap straight to the target, got {up}",
        );
        let down = f.follow(0.0, 0.016, 0.0, 0.130); // falling, release 130 ms
        assert!(
            down > 0.5 && down < 1.0,
            "release should ease down slowly (not snap), got {down}",
        );
    }
}
