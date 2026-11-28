// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;
    vec4 chroma_Resolution;
    vec4 syn_BPM;
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
    float r = length(p);
    float ripple = 0.5 + 0.5 * sin(r * 22.0 - t * 3.5 - chroma_Ctrl0.x * 8.0);
    vec3 col = palette(r * 0.8 + t * 0.12, vec3(0.45), vec3(0.45), vec3(1.0), vec3(0.05, 0.45, 0.85));
    fragColor = vec4(clamp(col * (0.25 + 0.75 * ripple) * exp(-r * 0.8), 0.0, 1.0), 1.0);
}
