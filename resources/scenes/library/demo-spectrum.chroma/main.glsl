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
    float idx = clamp(floor(uv.x * 8.0), 0.0, 7.0);
    float band = (idx < 4.0)
        ? (idx < 1.0 ? chroma_AudioFFT[0].x : (idx < 2.0 ? chroma_AudioFFT[0].y : (idx < 3.0 ? chroma_AudioFFT[0].z : chroma_AudioFFT[0].w)))
        : (idx < 5.0 ? chroma_AudioFFT[1].x : (idx < 6.0 ? chroma_AudioFFT[1].y : (idx < 7.0 ? chroma_AudioFFT[1].z : chroma_AudioFFT[1].w)));
    float height = clamp(0.18 + band * 0.82 + 0.12 * sin(t * 2.5 + idx * 0.9), 0.05, 0.96);
    float barX = smoothstep(0.06, 0.12, fract(uv.x * 8.0)) * smoothstep(0.06, 0.12, 1.0 - fract(uv.x * 8.0));
    float barY = smoothstep(height + 0.02, height - 0.02, uv.y);
    float glow = exp(-abs(uv.y - height) * 14.0) * 0.7;
    vec3 col = palette(idx / 8.0 + t * 0.08, vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.1, 0.4, 0.7));
    vec3 bg = vec3(0.04, 0.05, 0.09) + 0.05 * vec3(chroma_AudioRMS.x);
    vec3 outCol = mix(bg, col * (0.85 + 0.35 * chroma_BPM.z), barX * barY) + col * glow * barX;
    fragColor = vec4(clamp(outCol, 0.0, 1.0), 1.0);
}
