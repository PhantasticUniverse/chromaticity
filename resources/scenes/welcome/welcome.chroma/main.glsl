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
    float r = length(p);
    float a = atan(p.y, p.x);
    float spiral = sin(r * 14.0 - t * 2.2 + a * 3.0 + chroma_AudioRMS.x * 3.0);
    float halo = exp(-r * 2.4) * (0.8 + 0.35 * chroma_BPM.w);
    vec3 col = palette(r * 0.6 + spiral * 0.2 + t * 0.08, vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.0, 0.2, 0.5));
    col = col * (0.45 + 0.45 * spiral) + vec3(0.25, 0.65, 1.0) * halo;
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
