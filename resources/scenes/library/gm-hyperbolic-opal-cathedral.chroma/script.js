// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

// Hyperbolic Liquid-Opal Cathedral — deterministic QuickJS nave sway & vault surge script.
// Drives beat-synced architectural gaze drift (ctrl_13, ctrl_14) and a smooth
// transient-triggered canal energy surge envelope (ctrl_15).

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
      : 126.0;
  const peak = typeof globalThis.chroma_AudioPeak === 'number' ? globalThis.chroma_AudioPeak : 0.0;
  const rms = typeof globalThis.chroma_AudioRMS === 'number' ? globalThis.chroma_AudioRMS : 0.0;

  beatPhase += safeDt * (bpm / 60.0);

  // Subtle cathedral sway synced to 4-bar musical phrases
  const swayX = Math.sin(beatPhase * 0.392699) * 0.52;
  const pitchY = Math.sin(beatPhase * 0.196349) * 0.45 + Math.cos(beatPhase * 0.785398) * 0.12;

  // Kick/snare peak onset -> smooth forward vault surge decay
  if (peak > 0.72 && peak - prevPeak > 0.1) {
    surgeEnv = Math.min(1.0, surgeEnv + 0.65 + rms * 0.35);
  }
  prevPeak = peak;
  surgeEnv *= Math.exp(-safeDt * 3.6);
  if (surgeEnv < 0.001) {
    surgeEnv = 0.0;
  }

  __chromaticityEmit('ctrl_13', 0.5 + swayX * 0.42);
  __chromaticityEmit('ctrl_14', 0.5 + pitchY * 0.42);
  __chromaticityEmit('ctrl_15', 0.5 + surgeEnv * 0.5);
}
