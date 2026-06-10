// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// All per-frame reactive uniforms are marshalled into the single
// std140 block at (set = 0, binding = 0) by engine/render's multipass
// scheduler; each scalar / vector occupies one vec4-aligned 16-byte
// slot, and chroma_AudioFFT expands into 8 consecutive slots. Scalars
// are declared as vec4 and read via .x (matching the production
// audio-adapter fixture). Symbol names match docs/36 v1.3.0 exactly.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;
    vec4 RENDERSIZE;
    vec4 chroma_AudioFFT[8];
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 syn_BPM;
};

const float NUM_BARS = 8.0;

// Per-band hue ramp: low bands warm (red/orange), high bands cool
// (cyan/violet). Pure function of band index so it is stable at silence.
vec3 band_color(float band_index) {
    float t = band_index / (NUM_BARS - 1.0);
    vec3 lo = vec3(0.95, 0.30, 0.18);
    vec3 hi = vec3(0.25, 0.65, 0.98);
    vec3 base = mix(lo, hi, t);
    // Slight green lift in the middle bands for a fuller analyzer look.
    float mid = 1.0 - abs(t - 0.5) * 2.0;
    base += vec3(0.0, 0.20, 0.0) * mid;
    return clamp(base, 0.0, 1.0);
}

void main() {
    float time = TIME.x;
    float bpm = syn_BPM.x;
    float rms = chroma_AudioRMS.x;
    float peak = chroma_AudioPeak.x;

    // uv is the full-screen-triangle passthrough in [0, 1] with origin
    // bottom-left; x = horizontal column axis, y = vertical bar height.
    vec2 p = uv;

    // Which bar this fragment belongs to: 8 evenly spaced columns.
    float col = p.x * NUM_BARS;
    int band = int(clamp(floor(col), 0.0, NUM_BARS - 1.0));
    float frac = fract(col); // 0..1 position within this column

    // Carve a symmetric margin on each side so columns read as discrete
    // bars and never merge into a solid block.
    float gap = 0.12;
    float inside_x = step(gap * 0.5, frac) * step(frac, 1.0 - gap * 0.5);

    // Read this band's FFT magnitude. chroma_AudioFFT is non-normalised
    // [0, inf); compress it into a sane [0,1] viewing range via a soft knee.
    float mag = chroma_AudioFFT[band].x;
    float level = mag / (1.0 + mag); // never exceeds 1.0

    // BPM-phased pulse so the floor breathes even in silence. syn_BPM
    // defaults to 120; convert to beats-per-second for the phase.
    float bps = max(bpm, 1.0) / 60.0;
    float pulse = 0.5 + 0.5 * sin(time * bps * 6.2831853);

    // TIME shimmer staggered per band keeps a living baseline at silence.
    float shimmer = 0.5 + 0.5 * sin(time * 1.7 + float(band) * 0.9);

    // Animated floor: always-positive minimum bar height so the image is
    // never all-black when RMS = peak = FFT = 0.
    float floor_level = 0.06 + 0.05 * mix(shimmer, pulse, 0.5);

    // Composite bar height: floor + audio-driven growth. Overall energy
    // from RMS/peak lifts every bar a touch for a unified envelope.
    float energy = (rms + peak) * 0.5;
    float bar_h = floor_level + level * 0.85 + clamp(energy, 0.0, 1.0) * 0.10;
    bar_h = clamp(bar_h, 0.0, 1.0);

    // Fill: this fragment is lit if it is below the bar top and inside the
    // column margins. Smoothstep softens the bar's leading edge.
    float edge = 0.5 / max(RENDERSIZE.y, 1.0);
    float fill = smoothstep(bar_h + edge, bar_h - edge, p.y) * inside_x;

    // Color: brighten toward the bar tip for a gradient cap; pulse adds a
    // subtle BPM-tied glow.
    vec3 col_rgb = band_color(float(band));
    float tip = smoothstep(0.0, bar_h + 0.001, p.y); // 0 at base -> 1 near top
    vec3 lit = col_rgb * (0.55 + 0.45 * tip) * (0.85 + 0.15 * pulse);

    // Dim background so silence still shows the analyzer frame, not void.
    vec3 bg = vec3(0.03, 0.04, 0.06) + 0.01 * vec3(shimmer);

    vec3 color = mix(bg, lit, fill);
    fragColor = vec4(color, 1.0);
}
