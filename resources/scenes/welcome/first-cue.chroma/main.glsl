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
    float pulse = 0.35 + 0.25 * chroma_BPM.z + 0.25 * chroma_AudioRMS.x;
    float circle = smoothstep(pulse + 0.06, pulse - 0.06, length(p));
    vec3 col = mix(vec3(0.06, 0.08, 0.14), palette(t * 0.15 + length(p), vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.2, 0.5, 0.8)), circle);
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
