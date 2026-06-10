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

// Standard public hue ramp: cosine palette (Inigo Quilez technique).
vec3 palette(float t) {
    vec3 a = vec3(0.5, 0.5, 0.5);
    vec3 b = vec3(0.5, 0.5, 0.5);
    vec3 c = vec3(1.0, 1.0, 1.0);
    vec3 d = vec3(0.00, 0.33, 0.67);
    return a + b * cos(6.28318530718 * (c * t + d));
}

void main() {
    // Aspect-correct centered coordinates in [-1, 1]-ish space.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    vec2 p = (uv * 2.0 - 1.0);
    p.x *= res.x / res.y;
    float dist = length(p);

    // Low-end energy: average of the two lowest FFT bands. At silence the
    // FFT array is all-zero, so this term is exactly 0.0 (no NaN).
    float bass = 0.5 * (chroma_AudioFFT[0].x + chroma_AudioFFT[1].x);

    // Slow breathing motion comes from TIME alone, so the disc still moves
    // and is visible at zero audio. Audio swells it further.
    float breathe = 0.5 + 0.5 * sin(TIME.x * 1.2);
    float baseRadius = 0.25 + 0.04 * breathe;
    float radius = baseRadius + 0.45 * bass;

    // Soft radial falloff: 1.0 at center fading to 0.0 past the radius edge.
    // smoothstep keeps the edge anti-aliased and bounded in [0, 1].
    float disc = 1.0 - smoothstep(0.0, radius, dist);
    // Outer glow halo so the bloom reads even when small.
    float halo = exp(-dist * dist * 3.0);
    float bloom = clamp(disc + 0.35 * halo, 0.0, 1.0);

    // Brightness floor (0.12) guarantees a non-black frame at silence;
    // RMS lifts it. clamp keeps everything in display range.
    float brightness = clamp(0.12 + 0.9 * chroma_AudioRMS.x + 0.4 * bass, 0.0, 1.5);

    // Hue drifts slowly with time and is pushed by bass for an audible kick.
    float hue = 0.55 + 0.08 * TIME.x + 0.5 * bass;
    vec3 tint = palette(hue);

    vec3 color = tint * bloom * brightness;
    color = clamp(color, 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
