// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

// Quantum Ferro-Chromatic Singularity — deterministic QuickJS orbital & surge script.
// Drives beat-synced singularity camera drift (ctrl_13, ctrl_14) and a smooth
// quantum-tunneling transient decay envelope (ctrl_15).

let beatPhase = 0.0;
let surgeEnv = 0.0;
let prevPeak = 0.0;

function setup() {
  beatPhase = 0.0;
  surgeEnv = 0.0;
  prevPeak = 0.0;
  __chromaticityEmit('ctrl_13', 0.5);
  __chromaticityEmit('ctrl_14', 0.5);
  __chromaticityEmit('ctrl_15', 0.5);
}

function update(dt) {
  const safeDt = dt > 0 && dt < 0.25 ? dt : 1.0 / 60.0;
  const bpm =
    typeof globalThis.chroma_BPM === 'number' && globalThis.chroma_BPM > 20
      ? globalThis.chroma_BPM
      : 128.0;
  const peak = typeof globalThis.chroma_AudioPeak === 'number' ? globalThis.chroma_AudioPeak : 0.0;
  const rms = typeof globalThis.chroma_AudioRMS === 'number' ? globalThis.chroma_AudioRMS : 0.0;

  beatPhase += safeDt * (bpm / 60.0);

  // Harmonic dual-axis orbital drift locked to chroma_BPM
  const yawDrift = Math.sin(beatPhase * 0.261799) * 0.58 + Math.sin(beatPhase * 0.523598) * 0.16;
  const pitchDrift = Math.cos(beatPhase * 0.196349) * 0.45;

  // Audio transient detector -> exponential quantum surge decay
  if (peak > 0.7 && peak - prevPeak > 0.09) {
    surgeEnv = Math.min(1.0, surgeEnv + 0.68 + rms * 0.32);
  }
  prevPeak = peak;
  surgeEnv *= Math.exp(-safeDt * 4.5);
  if (surgeEnv < 0.001) {
    surgeEnv = 0.0;
  }

  __chromaticityEmit('ctrl_13', 0.5 + yawDrift * 0.45);
  __chromaticityEmit('ctrl_14', 0.5 + pitchDrift * 0.45);
  __chromaticityEmit('ctrl_15', 0.5 + surgeEnv * 0.5);
}
