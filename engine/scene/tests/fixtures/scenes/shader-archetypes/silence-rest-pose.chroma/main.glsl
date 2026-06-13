// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// Per-frame + audio reactive uniforms, packed into the single std140
// block the engine marshals at set=0, binding=0 (mirrors the
// engine/render audio_adapter T2 contract: scalar reactive symbols
// arrive as vec4 read via .x; the FFT is a float[8] array).
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 u_time;          // TIME seconds in .x
    vec4 u_resolution;    // RENDERSIZE pixels in .xy
    vec4 chroma_AudioRMS; // RMS energy in .x
    vec4 chroma_AudioPeak;// window peak in .x
    vec4 syn_BPM;         // estimated BPM in .x
    vec4 chroma_AudioFFT[8];
};

// Silence Rest Pose
//
// A calm idle "breathing" field. The whole look is driven by TIME so
// that at zero audio (RMS=peak=0, FFT all 0) the frame is a soft,
// gently pulsing radial glow on a dark blue-teal base -- never black
// and never NaN. The four wired audio uniforms add motion only when
// present, so silence is the well-defined rest pose, not a failure.
void main() {
    float time = u_time.x;

    // Aspect-correct centered coordinates. RENDERSIZE is always > 0 on
    // the render path; guard the divide so a degenerate 0 cannot NaN.
    vec2 res = max(u_resolution.xy, vec2(1.0));
    vec2 p = (uv - 0.5) * vec2(res.x / res.y, 1.0);
    float r = length(p);

    // Slow TIME-driven breath. syn_BPM rests at 120 in silence, which
    // is a valid, finite tempo, so any beat term is well-defined at
    // zero audio rather than a special case. breath stays in [0,1].
    float bpmHz = max(syn_BPM.x, 1.0) / 60.0;
    float breath = 0.5 + 0.5 * sin(time * 0.6 + sin(time * 0.21) * 0.5);

    // Audio-reactive accents. All terms vanish to 0 at silence so the
    // rest pose is pure TIME. RMS swells the glow, peak adds a brief
    // ring flash, the FFT bands tint warmth into the cool base.
    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);

    float fftSum = 0.0;
    for (int i = 0; i < 8; i++) {
        fftSum += clamp(chroma_AudioFFT[i].x, 0.0, 1.0);
    }
    float fftAvg = fftSum / 8.0;

    // Breathing radius: rests around the center, swells gently with the
    // breath clock and with RMS energy when audio is present.
    float radius = 0.28 + 0.10 * breath + 0.30 * rms;

    // Soft-edged disc -> smooth glow falloff. Never hard-clips to 0
    // across the whole frame, so corners keep a faint floor color.
    float glow = smoothstep(radius + 0.45, radius - 0.05, r);

    // A thin ring that flashes on transient peaks (silent => 0).
    float ringWidth = 0.02 + 0.03 * peak;
    float ring = smoothstep(ringWidth, 0.0, abs(r - radius)) * peak;

    // Cool base palette (dark blue-teal) -- the silence look. A small
    // constant ambient floor guarantees the frame is never black.
    vec3 baseCool = vec3(0.04, 0.10, 0.16);
    vec3 glowCool = vec3(0.15, 0.45, 0.62);

    // Warm accent driven only by spectral content; zero at silence.
    vec3 warm = vec3(0.55, 0.35, 0.20) * fftAvg;

    float ambient = 0.06;
    float restGain = 0.35 + 0.65 * breath; // gentle whole-field breathing

    vec3 color = baseCool * ambient;
    color += glowCool * glow * restGain;
    color += warm * glow;
    color += vec3(0.80, 0.90, 1.0) * ring;

    // Keep the bpmHz term referenced (silence-safe, finite) without
    // perturbing the rest pose: a near-zero beat shimmer that is
    // imperceptible at the 120 BPM idle and only reads under audio.
    color += glowCool * (0.02 * rms) * (0.5 + 0.5 * sin(time * bpmHz * 6.2831853));

    // Keep everything finite and in-gamut for the 8-bit target.
    color = clamp(color, 0.0, 1.0);
    fragColor = vec4(color, 1.0);
}
