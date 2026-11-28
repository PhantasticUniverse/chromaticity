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
    float rot = t * 0.8 + chroma_Ctrl0.x * 6.2831853;
    mat2 m = mat2(cos(rot), -sin(rot), sin(rot), cos(rot));
    vec2 q = m * p;
    float d = max(abs(q.x), abs(q.y));
    float box = smoothstep(0.32, 0.28, d) - smoothstep(0.22, 0.18, d);
    vec3 col = palette(chroma_Ctrl0.x + t * 0.1, vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.1, 0.4, 0.8));
    fragColor = vec4(clamp(vec3(0.05, 0.06, 0.1) + col * (0.25 + box * 0.9), 0.0, 1.0), 1.0);
}
