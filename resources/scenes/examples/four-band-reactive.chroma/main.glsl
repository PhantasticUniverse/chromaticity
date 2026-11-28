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
    vec4 bands = vec4(chroma_AudioFFT[0].x, chroma_AudioFFT[0].z, chroma_AudioFFT[1].x, chroma_AudioFFT[1].z);
    float q = floor(uv.x * 4.0);
    float val = (q < 1.0) ? bands.x : (q < 2.0 ? bands.y : (q < 3.0 ? bands.z : bands.w));
    float fill = smoothstep(0.15 + val * 0.8 + 0.05 * sin(t * 2.0 + q), 0.05, uv.y);
    vec3 col = palette(q * 0.25 + t * 0.05, vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.0, 0.33, 0.67));
    fragColor = vec4(clamp(mix(vec3(0.04, 0.05, 0.08), col, fill), 0.0, 1.0), 1.0);
}
