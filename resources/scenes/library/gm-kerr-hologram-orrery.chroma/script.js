// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

// Kerr Holographic Orrery — deterministic QuickJS motion & shockwave script.
// Drives beat-synced observer parallax drift (ctrl_13, ctrl_14) and a musical
// peak-triggered gravitational-wave shock decay envelope (ctrl_15).

let beatPhase = 0.0;
let shockEnv = 0.0;
let prevPeak = 0.0;

function setup() {
  beatPhase = 0.0;
  shockEnv = 0.0;
  prevPeak = 0.0;
  __chromaticityEmit('ctrl_13', 0.5);
  __chromaticityEmit('ctrl_14', 0.5);
  __chromaticityEmit('ctrl_15', 0.5);
}

function update(dt) {
  const safeDt = dt > 0 && dt < 0.25 ? dt : 1.0 / 60.0;
  const bpm =
    typeof globalThis.syn_BPM === 'number' && globalThis.syn_BPM > 20 ? globalThis.syn_BPM : 124.0;
  const peak = typeof globalThis.chroma_AudioPeak === 'number' ? globalThis.chroma_AudioPeak : 0.0;
  const rms = typeof globalThis.chroma_AudioRMS === 'number' ? globalThis.chroma_AudioRMS : 0.0;

  // Advance continuous beat phase locked to syn_BPM
  beatPhase += safeDt * (bpm / 60.0);

  // Smooth Lissajous off-axis observer parallax drift
  const driftX = Math.sin(beatPhase * 0.392699) * 0.55 + Math.cos(beatPhase * 0.196349) * 0.25;
  const driftY = Math.cos(beatPhase * 0.261799) * 0.48 + Math.sin(beatPhase * 0.523598) * 0.18;

  // Rising-edge audio peak detector -> exponential shockwave decay envelope
  const onset = peak > 0.72 && peak - prevPeak > 0.1;
  if (onset) {
    shockEnv = Math.min(1.0, shockEnv + 0.65 + rms * 0.35);
  }
  prevPeak = peak;
  shockEnv *= Math.exp(-safeDt * 4.2);
  if (shockEnv < 0.001) {
    shockEnv = 0.0;
  }

  // Emit into dedicated script uniform slots (0.5 = neutral baseline)
  __chromaticityEmit('ctrl_13', 0.5 + driftX * 0.45);
  __chromaticityEmit('ctrl_14', 0.5 + driftY * 0.45);
  __chromaticityEmit('ctrl_15', 0.5 + shockEnv * 0.5);
}
