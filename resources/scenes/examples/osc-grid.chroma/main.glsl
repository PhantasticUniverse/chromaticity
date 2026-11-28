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
    vec2 grid = fract(uv * vec2(12.0, 7.0));
    float line = max(smoothstep(0.06, 0.0, min(grid.x, 1.0 - grid.x)), smoothstep(0.06, 0.0, min(grid.y, 1.0 - grid.y)));
    float pulse = 0.5 + 0.5 * sin(t * 2.0 + uv.x * 6.0 + chroma_Ctrl0.x * 6.28);
    vec3 col = mix(vec3(0.04, 0.07, 0.11), vec3(0.18, 0.85, 0.95) * (0.6 + 0.4 * pulse), line);
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
