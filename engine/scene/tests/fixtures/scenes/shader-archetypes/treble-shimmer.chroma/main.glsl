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
    float chroma_AudioFFT[8];
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 syn_BPM;
};

// Original 2D value-hash (public-technique sin-dot scramble). Deterministic,
// returns [0,1); no external textures.
float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 78.233);
    return fract(p.x * p.y);
}

void main() {
    // Pixel-aspect-corrected coordinates so the grain stays square on any
    // resolution. RENDERSIZE is always positive at render time; guard the
    // divide so a degenerate (zero) size never yields NaN/Inf.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    vec2 p = uv;
    p.x *= res.x / res.y;

    // Treble energy from the two highest log-spaced FFT bands. At silence
    // these are 0.0, so `treble` is 0.0 and the shimmer floors to a faint base.
    float treble = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]);
    treble = clamp(treble, 0.0, 4.0);

    // Fine grain cells: density scales with treble so louds highs add more,
    // smaller sparkles. Base density keeps a sane image at zero audio.
    float density = 90.0 + treble * 110.0;
    vec2 cell = floor(p * density);

    // Per-cell twinkle phase, animated by TIME. Each cell flickers on its own
    // schedule; abs(sin) keeps it in [0,1].
    float seed = hash21(cell);
    float twinkle = abs(sin(TIME.x * (2.0 + seed * 6.0) + seed * 6.2831853));

    // Sparkle gate: only the brightest twinkles light up. Treble lowers the
    // threshold (more sparkles) and lifts the gain (brighter sparkles).
    float threshold = 0.96 - treble * 0.20;
    float spark = smoothstep(threshold, 1.0, twinkle * (0.6 + seed * 0.4));
    spark *= (0.35 + treble * 0.9);

    // Cool-to-warm tint: base is a dim blue-violet haze (visible at silence),
    // treble pushes sparkles toward bright cyan-white.
    vec3 baseHaze = vec3(0.04, 0.05, 0.09);
    vec3 sparkColor = mix(vec3(0.55, 0.75, 1.0), vec3(1.0, 1.0, 0.95), treble * 0.5);

    vec3 color = baseHaze + sparkColor * spark;
    color = clamp(color, 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
