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

// Baked archetype constants (mirror the scene.json control DEFAULTs; the
// CONTROLS are declared for schema completeness but are not wired as GLSL
// uniforms in the render path's PASS UNIFORMS block).
const float WARN_THRESHOLD = 0.7;  // peak above which the border goes hot
const float BORDER_WIDTH = 0.18;   // normalized border thickness
const float INTERIOR_GAIN = 0.6;   // interior brightness response to RMS

void main() {
    // uv arrives in [0,1]; center it for distance-from-edge work.
    vec2 p = uv - 0.5;

    // Non-normalised audio is range [0, inf); tame it into [0,1] so silence
    // (RMS=peak=0) is a sane, non-black image and never NaN.
    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);

    // --- Interior field ---------------------------------------------------
    // Slow TIME-driven motion guarantees a living image even at silence.
    float radial = length(p);
    float wave = 0.5 + 0.5 * sin(radial * 12.0 - TIME.x * 1.5);
    // Base luminance floor keeps the interior visible with zero audio.
    float interior = 0.18 + INTERIOR_GAIN * rms * wave;
    // Cool teal interior; brightens with RMS energy.
    vec3 color = vec3(0.10, 0.32, 0.42) * (0.6 + interior);

    // --- Clip-warning border ----------------------------------------------
    // Distance to the nearest frame edge, in [0, 0.5].
    float edgeDist = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    // 1.0 hard against the frame edge, 0.0 once inside the border band.
    float edgeMask = 1.0 - smoothstep(0.0, BORDER_WIDTH, edgeDist);

    // How hot is the peak relative to the warn threshold? 0 below, 1 at clip.
    float warn = smoothstep(WARN_THRESHOLD, 1.0, peak);

    // Hot red that saturates as peak -> 1.0. Add a TIME pulse near clip so a
    // genuine full-scale peak visibly throbs at the frame edge.
    float pulse = 0.85 + 0.15 * sin(TIME.x * 8.0);
    vec3 warnColor = vec3(1.0, 0.05, 0.02) * mix(1.0, pulse, warn);

    // Blend the border in proportional to both edge proximity and warn level.
    color = mix(color, warnColor, edgeMask * warn);

    // At true clip (peak ~= 1.0) drive the border fully hot so the edge case
    // reads unambiguously red/hot for the renderer-test assertion.
    color = mix(color, warnColor, edgeMask * step(0.999, peak));

    // Guard against any stray non-finite path; clamp to a displayable range.
    color = clamp(color, vec3(0.0), vec3(1.0));

    fragColor = vec4(color, 1.0);
}
