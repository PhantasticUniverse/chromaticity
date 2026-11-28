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

// chroma_Media, _isMediaActive and _textureMedia come from the runtime's
// media prelude (injected because this shader references them).

void main() {
    float t = chroma_Time.x;
    vec2 p = (uv - 0.5) * vec2(chroma_Time.w, 1.0);
    float segments = floor(mix(3.0, 12.0, chroma_Ctrl0.x));
    float zoom = mix(0.6, 1.8, chroma_Ctrl0.y);
    float spin = mix(-0.4, 0.4, chroma_Ctrl0.z);

    float r = length(p);
    float wedge = 6.2831853 / segments;
    float a = mod(atan(p.y, p.x) + t * spin, wedge);
    a = abs(a - 0.5 * wedge);
    vec2 media_uv = clamp(vec2(cos(a), sin(a)) * r * zoom + 0.5, 0.0, 1.0);

    vec3 col;
    if (_isMediaActive()) {
        col = _textureMedia(media_uv).rgb;
    } else {
        col = 0.3 * (0.5 + 0.5 * cos(6.2831853 * (vec3(0.0, 0.33, 0.67) + r * 1.5 - t * 0.15)));
    }
    float level = chroma_AudioRMS.x;
    col *= 0.85 + 0.5 * level;
    float ring = exp(-abs(r - (0.22 + 0.25 * chroma_BPM.z)) * 28.0);
    col += vec3(0.95, 0.35, 0.6) * ring * chroma_BPM.z * 0.35;
    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
