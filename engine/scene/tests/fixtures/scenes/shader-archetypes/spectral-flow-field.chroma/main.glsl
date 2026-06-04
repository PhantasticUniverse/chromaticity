// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// Per-frame + audio reactive uniforms. naga's GLSL 4.50 frontend requires
// uniforms to live in a layout(binding=X) block; the engine packs every
// scalar / vector into ONE std140 buffer at set 0, binding 0 (per
// engine/render/src/multipass.rs). Scalars occupy a vec4 slot and are read
// via `.x` / `.xy`; the FFT is a float[8] array. scene.json binds each field
// to a reactive symbol by NAME->SYMBOL declaration order, independent of the
// GLSL declaration shape.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;            // .x = seconds since scene start
    vec4 RENDERSIZE;      // .xy = current pass pixel size
    vec4 chroma_AudioRMS; // .x = raw RMS energy [0, inf)
    float chroma_AudioFFT[8];
};

// --- value noise (standard public technique: hash + bilerp) -----------------

float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float valueNoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    // smoothstep interpolation weights
    vec2 w = f * f * (3.0 - 2.0 * f);
    float a = hash21(i + vec2(0.0, 0.0));
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    float ab = mix(a, b, w.x);
    float cd = mix(c, d, w.x);
    return mix(ab, cd, w.y);
}

// Scalar potential field whose curl yields a divergence-free 2D flow.
float potential(vec2 p, float t) {
    float n = 0.0;
    float amp = 0.5;
    vec2 q = p;
    // a few fbm octaves, slowly drifting in time for life under silence
    for (int o = 0; o < 3; o++) {
        n += amp * valueNoise(q + vec2(t * 0.15, -t * 0.1));
        q *= 2.02;
        amp *= 0.5;
    }
    return n;
}

// Curl of the scalar potential -> flow vector (finite differences).
vec2 curlFlow(vec2 p, float t) {
    float e = 0.01;
    float dx = potential(p + vec2(e, 0.0), t) - potential(p - vec2(e, 0.0), t);
    float dy = potential(p + vec2(0.0, e), t) - potential(p - vec2(0.0, e), t);
    // 2D curl of a scalar field: (d/dy, -d/dx)
    return vec2(dy, -dx) / (2.0 * e);
}

// --- hue helper (standard hsv -> rgb) ---------------------------------------

vec3 hsv2rgb(float h, float s, float v) {
    vec3 k = vec3(1.0, 2.0 / 3.0, 1.0 / 3.0);
    vec3 p = abs(fract(vec3(h) + k) * 6.0 - 3.0);
    return v * mix(vec3(1.0), clamp(p - 1.0, 0.0, 1.0), s);
}

void main() {
    // Aspect-correct, centered coordinates from the passthrough UV.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    float aspect = res.x / res.y;
    vec2 p = (uv - 0.5) * vec2(aspect, 1.0);

    // Fixed authoring constants (controls are not bound as uniforms in this
    // archetype; the scene.json CONTROLS are exercised by the schema loader).
    float speed = 0.5;     // base flow speed (constant motion under silence)
    float scale = 3.0;     // base noise scale
    float gain = 1.0;      // audio influence
    float hueSpread = 0.6; // centroid -> hue mapping width

    // RMS is non-normalised [0, inf); soft-compress to a bounded reactive level.
    float rms = max(chroma_AudioRMS.x, 0.0);
    float level = (rms * gain) / (1.0 + rms * gain); // in [0, 1)

    // Spectral centroid: energy-weighted mean band index over the 8 FFT bands,
    // normalised to [0, 1]. Falls back to neutral 0.5 when the spectrum is
    // empty (silence), so hue is well-defined and never NaN.
    float weighted = 0.0;
    float total = 0.0;
    for (int i = 0; i < 8; i++) {
        float b = max(chroma_AudioFFT[i], 0.0);
        weighted += b * float(i);
        total += b;
    }
    float spectralCentroid = (total > 1e-5) ? (weighted / (total * 7.0)) : 0.5;

    // Advect the sample point through the curl flow over a few steps.
    float t = TIME.x * (0.4 + speed);
    vec2 sp = p * scale;
    float dt = 0.06;
    // audio nudges the advection strength but never to zero
    float advect = 0.5 + level * 1.5;
    for (int s = 0; s < 6; s++) {
        sp += curlFlow(sp, t) * dt * advect;
    }

    // Flow magnitude drives structure/brightness; always has TIME-driven motion.
    vec2 flow = curlFlow(sp, t);
    float streak = valueNoise(sp + flow * 0.5);
    float mag = clamp(length(flow) * 0.5, 0.0, 1.0);

    // Hue from centroid, spread around it, drifting slowly on TIME.
    float hue = fract(spectralCentroid * hueSpread + streak * 0.15 + TIME.x * 0.02);
    float sat = 0.55 + 0.35 * mag;
    // baseline brightness so silence is a calm lit field, not black
    float val = 0.22 + 0.55 * streak + 0.45 * level;

    vec3 color = hsv2rgb(hue, sat, val);

    // Subtle vignette for framing; guaranteed finite.
    float vig = smoothstep(1.25, 0.25, length(p));
    color *= mix(0.7, 1.0, vig);

    color = clamp(color, 0.0, 1.0);
    fragColor = vec4(color, 1.0);
}
