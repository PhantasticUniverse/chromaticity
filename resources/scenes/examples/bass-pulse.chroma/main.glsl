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
    float bass = max(chroma_AudioFFT[0].x, chroma_AudioRMS.x);
    float r = length(p);
    float shock = exp(-abs(r - (0.18 + bass * 0.45 + 0.12 * chroma_BPM.z)) * 14.0);
    vec3 col = vec3(0.05, 0.03, 0.09) + vec3(1.0, 0.25, 0.55) * shock + vec3(0.25, 0.5, 1.0) * exp(-r * 3.0) * (0.4 + bass);
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
