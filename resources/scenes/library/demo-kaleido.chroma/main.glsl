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
    float aspect = chroma_Time.w;
    vec2 p = (uv - 0.5) * vec2(aspect, 1.0);
    float segs = max(3.0, floor(2.0 + chroma_Ctrl0.x * 10.0));
    float angle = atan(p.y, p.x) + t * (0.25 + chroma_Ctrl0.y * 0.75);
    float sector = 6.2831853 / segs;
    angle = abs(mod(angle, sector) - 0.5 * sector);
    float r = length(p);
    vec2 k = vec2(cos(angle), sin(angle)) * r;
    float warp = 0.35 + chroma_Ctrl0.z * 0.8 + chroma_AudioRMS.x * 0.5;
    float wave = sin(k.x * 12.0 + t * 1.4) * cos(k.y * 12.0 - t * 1.1) * warp;
    float ring = smoothstep(0.08, 0.0, abs(fract(r * 4.0 - t * 0.35 + wave * 0.25) - 0.5) - 0.12);
    vec3 col = palette(r * 0.8 + wave * 0.3 + t * 0.12, vec3(0.5), vec3(0.5), vec3(1.0), vec3(0.0, 0.33, 0.67));
    col = mix(col * 0.28, col * (1.1 + syn_BPM.z * 0.4), ring);
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
