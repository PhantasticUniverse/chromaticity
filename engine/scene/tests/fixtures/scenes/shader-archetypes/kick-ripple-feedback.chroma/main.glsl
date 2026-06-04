// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// All per-frame reactive scalar/vector uniforms are marshalled into the
// single std140 block at (set = 0, binding = 0) by engine/render's
// multipass scheduler; each scalar / vector occupies one vec4-aligned
// 16-byte slot, and chroma_AudioFFT expands into 8 consecutive slots.
// Scalars are declared as vec4 and read via .x (matching the production
// audio-adapter fixture). Symbol names match docs/36 v1.3.0 exactly.
// TIMEDELTA is appended after syn_BPM because this shader uses it.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;
    vec4 RENDERSIZE;
    float chroma_AudioFFT[8];
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 syn_BPM;
    vec4 TIMEDELTA;
};

// Feedback source: previous frame's final output (syn_FinalPass). naga's
// GLSL frontend rejects a combined `uniform sampler2D` declaration (both
// inside a UBO block and as a top-level resource, surfacing as
// NotImplemented "variable qualifier"); the Vulkan-GLSL form with a
// separate texture2D + sampler combined at use-site via the
// sampler2D(tex, samp) constructor is required. This matches engine/
// render's multipass bind layout (engine/render/src/multipass.rs): the
// uniform block is binding 0, then each pass input binds a texture at
// `1 + 2*i` and its sampler at `1 + 2*i + 1`. A single self-feedback
// pass with no explicit prior-pass inputs binds its previous-frame
// texture at slot 1 and the sampler at slot 2. At frame zero this reads
// a cleared (black) texture, so the fresh idle ripple seeds a sane image.
layout(set = 0, binding = 1) uniform texture2D syn_FinalPass;
layout(set = 0, binding = 2) uniform sampler syn_FinalPass_sampler;

// Hue-rotation helper: cheap, branch-free, no transcendental edge cases.
vec3 palette(float t) {
    vec3 a = vec3(0.50, 0.50, 0.55);
    vec3 b = vec3(0.45, 0.45, 0.50);
    vec3 c = vec3(1.00, 1.00, 1.00);
    vec3 d = vec3(0.00, 0.33, 0.67);
    return a + b * cos(6.28318530718 * (c * t + d));
}

void main() {
    // Aspect-corrected coordinates centered at origin. RENDERSIZE is
    // always positive in the render path; guard anyway so a degenerate
    // zero size cannot divide-by-zero into NaN.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    vec2 p = (uv * 2.0 - 1.0);
    p.x *= res.x / res.y;

    float r = length(p);

    // Bass transient drives ripple amplitude. chroma_AudioFFT[0] is the
    // lowest log-spaced band; at silence it is 0. A small idle floor
    // keeps the image alive (never all-black) when no audio is present.
    float bass = clamp(chroma_AudioFFT[0] * 6.0 + chroma_AudioRMS.x * 2.0, 0.0, 4.0);
    float idle = 0.12;
    float drive = idle + bass;

    // Ripple front speed scales gently with tempo. syn_BPM defaults to
    // 120 at silence, so motion is continuous regardless of audio.
    float tempo = clamp(syn_BPM.x, 50.0, 220.0) / 120.0;

    // Concentric wavefront. TIME gives continuous phase; TIMEDELTA nudges
    // the outward front so the ripple advances per-frame even if TIME is
    // quantized. Density is fixed here (control wiring is host-side).
    float density = 14.0;
    float phase = r * density - TIME.x * 2.4 * tempo - TIMEDELTA.x * 18.0;
    float wave = 0.5 + 0.5 * sin(phase);

    // Sharpen the rings as bass rises; falloff keeps the center bright and
    // the edges calm so silence still shows soft idle rings.
    float rings = pow(wave, 2.0 + drive * 3.0);
    float falloff = exp(-r * 1.6);
    float intensity = rings * falloff * (0.6 + drive);

    vec3 fresh = palette(r * 0.5 + TIME.x * 0.05 + bass * 0.1) * intensity;

    // Feedback: blend decaying prior frame with the fresh ripple so
    // transients leave expanding trails. A slight inward zoom on the
    // sample coordinate pushes old rings outward over time.
    vec2 zoomUv = (uv - 0.5) * 0.992 + 0.5;
    vec3 prior = texture(sampler2D(syn_FinalPass, syn_FinalPass_sampler), zoomUv).rgb;
    float decay = 0.88;

    vec3 color = prior * decay + fresh;

    // Tone-guard: clamp to a sane range so accumulation cannot blow up
    // into Inf/NaN across many feedback frames.
    color = clamp(color, 0.0, 1.2);

    fragColor = vec4(color, 1.0);
}
