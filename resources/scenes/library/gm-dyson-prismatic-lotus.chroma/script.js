// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

// Dyson Prismatic Lotus — deterministic QuickJS orbital drift & core flare script.
// Drives beat-synced camera orbit breath (ctrl_13, ctrl_14) and a smooth
// Dyson-core caustic pulse envelope (ctrl_15).

let beatPhase = 0.0;
let flareEnv = 0.0;
let prevPeak = 0.0;

function setup() {
  beatPhase = 0.0;
  flareEnv = 0.0;
  prevPeak = 0.0;
  __chromaticityEmit('ctrl_13', 0.5);
  __chromaticityEmit('ctrl_14', 0.5);
  __chromaticityEmit('ctrl_15', 0.5);
}

function update(dt) {
  const safeDt = dt > 0 && dt < 0.25 ? dt : 1.0 / 60.0;
  const bpm =
    typeof globalThis.syn_BPM === 'number' && globalThis.syn_BPM > 20 ? globalThis.syn_BPM : 122.0;
  const peak = typeof globalThis.chroma_AudioPeak === 'number' ? globalThis.chroma_AudioPeak : 0.0;
  const rms = typeof globalThis.chroma_AudioRMS === 'number' ? globalThis.chroma_AudioRMS : 0.0;

  beatPhase += safeDt * (bpm / 60.0);

  // Graceful orbital azimuth & elevation breathing locked to 8-bar cycles
  const orbitDriftX = Math.sin(beatPhase * 0.196349) * 0.65;
  const orbitDriftY = Math.sin(beatPhase * 0.392699) * 0.42 + Math.cos(beatPhase * 0.130899) * 0.2;

  // Musical transient detector -> exponential core flare decay
  if (peak > 0.7 && peak - prevPeak > 0.09) {
    flareEnv = Math.min(1.0, flareEnv + 0.6 + rms * 0.4);
  }
  prevPeak = peak;
  flareEnv *= Math.exp(-safeDt * 3.8);
  if (flareEnv < 0.001) {
    flareEnv = 0.0;
  }

  __chromaticityEmit('ctrl_13', 0.5 + orbitDriftX * 0.45);
  __chromaticityEmit('ctrl_14', 0.5 + orbitDriftY * 0.45);
  __chromaticityEmit('ctrl_15', 0.5 + flareEnv * 0.5);
}
