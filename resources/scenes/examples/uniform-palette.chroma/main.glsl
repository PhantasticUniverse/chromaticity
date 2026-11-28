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
    float hue = uv.x + t * 0.12 + chroma_Ctrl0.x * 0.5;
    vec3 col = palette(hue, vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.0, 0.33, 0.67));
    float band = 0.85 + 0.15 * sin(uv.y * 18.0 + t * 2.0);
    fragColor = vec4(clamp(col * band, 0.0, 1.0), 1.0);
}
