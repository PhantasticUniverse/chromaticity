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

const float TWO_PI = 6.28318530718;

// Lissajous integer frequency ratio (mirrors the scene.json control defaults
// freq_x = 3, freq_y = 2; controls are not per-frame reactive symbols so they
// are baked here rather than bound as uniforms).
const float FREQ_X = 3.0;
const float FREQ_Y = 2.0;

void main() {
    // Aspect-correct coordinates centred on the origin, range roughly [-1, 1].
    vec2 p = (uv - 0.5) * 2.0;
    p.x *= max(RENDERSIZE.x, 1.0) / max(RENDERSIZE.y, 1.0);

    // Tempo phase: shader-side sine clock advancing at syn_BPM beats/min.
    // syn_BPM rests at the 120 neutral under silence, so this is always > 0.
    float bpm = max(syn_BPM.x, 1.0);
    float phase = TIME.x * bpm / 60.0;

    // RMS swells the figure amplitude and brightness. At zero audio the
    // baseline keeps a clearly visible curve (never collapses to a point).
    float energy = clamp(chroma_AudioRMS.x, 0.0, 4.0);
    float amp = 0.55 + 0.30 * (energy / (1.0 + energy));

    // Slow phase offset between the two axes makes the figure precess over time.
    float drift = 0.5 * sin(TIME.x * 0.15);

    // Trace the Lissajous path as a set of sampled points, accumulating a soft
    // glow wherever the current fragment is near the curve.
    float glow = 0.0;
    const int SAMPLES = 220;
    for (int i = 0; i < SAMPLES; i++) {
        float t = (float(i) / float(SAMPLES)) * TWO_PI;
        vec2 c = vec2(
            amp * sin(FREQ_X * t + phase),
            amp * sin(FREQ_Y * t + phase * 0.5 + drift)
        );
        float d = length(p - c);
        // Inverse-distance falloff -> thin bright filament along the curve.
        glow += 0.0016 / (d * d + 0.0006);
    }
    glow = glow / float(SAMPLES) * 24.0;
    glow = clamp(glow, 0.0, 1.5);

    // Hue cycles with the beat phase; brightness lifts with audio energy.
    float hue = fract(phase * 0.08 + 0.55);
    vec3 base = 0.5 + 0.5 * cos(TWO_PI * (hue + vec3(0.0, 0.33, 0.66)));
    vec3 color = base * glow * (1.0 + 0.6 * energy);

    // Dim radial backdrop so the frame is never fully black, even in silence.
    float vignette = 1.0 - 0.6 * dot(p, p);
    vec3 backdrop = vec3(0.02, 0.03, 0.05) * max(vignette, 0.0);

    color += backdrop;
    color = clamp(color, 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
