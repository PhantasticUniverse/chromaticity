// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 chroma_BPM;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 chroma_AudioFFT[2];
    vec4 chroma_Ctrl0;
    vec4 chroma_Ctrl1;
    vec4 chroma_Ctrl2;
    vec4 chroma_Ctrl3;
};

vec3 palette(float t, vec3 a, vec3 b, vec3 c, vec3 d) {
    return a + b * cos(6.2831853 * (c * t + d));
}
void main() {
    float t = chroma_Time.x;
    vec2 p = (uv - 0.5) * vec2(chroma_Time.w, 1.0);
    float scan = 0.5 + 0.5 * sin(uv.y * 120.0 + t * 4.0);
    float frame = smoothstep(0.02, 0.0, abs(max(abs(p.x) - 0.58, abs(p.y) - 0.24)));
    float bars = smoothstep(0.02, 0.0, abs(p.y)) * step(abs(p.x), 0.42);
    vec3 col = palette(p.x * 0.5 + t * 0.15, vec3(0.45), vec3(0.45), vec3(1.0), vec3(0.05, 0.25, 0.55));
    vec3 base = mix(vec3(0.05, 0.06, 0.11), col * (0.7 + 0.3 * scan), 0.35 + 0.4 * bars + frame);
    fragColor = vec4(clamp(base, 0.0, 1.0), 1.0);
}
