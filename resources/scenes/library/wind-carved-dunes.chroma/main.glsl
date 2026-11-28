// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;        // .x = elapsed seconds, .y = dt, .z = frame, .w = aspect
    vec4 chroma_Resolution;  // .x = width, .y = height, .z = 1/w, .w = 1/h
    vec4 syn_BPM;            // .x = BPM
    vec4 chroma_AudioRMS;    // .x = RMS [0..1]
    vec4 chroma_AudioPeak;   // .x = Peak [0..1]
    float chroma_AudioFFT[8];// 8 log-spaced FFT bands [0..1]
    vec4 chroma_Ctrl0;       // .x..w = controls 0..3 [0..1]
    vec4 chroma_Ctrl1;       // .x..w = controls 4..7 [0..1]
};

float hash21(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2(1.0, 0.0)), u.x),
        mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), u.x),
        u.y
    );
}

float duneRidge(float x, float layer, float ts, float warp) {
    float n1 = sin(x * (1.8 + layer * 0.45) + ts * (0.25 + layer * 0.08) + layer * 2.1);
    float n2 = sin(x * (3.7 + layer * 0.7) - ts * 0.18 + layer * 4.3) * 0.45;
    float n3 = (vnoise(vec2(x * 2.2 + ts * 0.3, layer * 5.7)) - 0.5) * (0.65 + warp * 0.6);
    return (n1 * 0.55 + n2 * 0.35 + n3 * 0.45) * 0.11;
}

void main() {
    float aspect = chroma_Time.w > 0.01 ? chroma_Time.w : (chroma_Resolution.x / max(chroma_Resolution.y, 1.0));
    float t = chroma_Time.x;

    float speed = 0.35 + chroma_Ctrl0.x * 1.35;
    float intensity = 0.68 + chroma_Ctrl0.y * 0.75;
    float warp = chroma_Ctrl0.z;
    float invertFlag = chroma_Ctrl0.w;

    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]);
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]);

    float ts = t * speed;
    float x = uv.x * aspect;
    // Flip Y so y = 0 is bottom and y = 1 is top under wgpu top-down uv.
    float y = 1.0 - uv.y;

    // Warm desert golden-hour sunset sky
    vec3 skyTop = vec3(0.11, 0.08, 0.24);
    vec3 skyMid = vec3(0.68, 0.26, 0.28);
    vec3 skyLow = vec3(0.98, 0.68, 0.34);
    vec3 col = mix(skyLow, skyMid, smoothstep(0.32, 0.72, y));
    col = mix(col, skyTop, smoothstep(0.68, 1.0, y));

    // Low desert sun disc
    vec2 sunPos = vec2(0.68 * aspect, 0.72);
    float sunDist = length(vec2(x, y) - sunPos);
    col += vec3(1.0, 0.82, 0.48) * exp(-sunDist * 4.2) * 0.55;
    col += vec3(1.0, 0.94, 0.78) * smoothstep(0.085, 0.072, sunDist) * 0.9;

    // 6 layered wind-carved sand dune ranges from distant to foreground
    for (int i = 0; i < 6; i++) {
        float fi = float(i);
        float depth = fi / 5.0; // 0 = far, 1 = near
        float baseH = 0.62 - depth * 0.44;
        float audioLift = (0.015 + 0.025 * depth) * (bass * sin(x * 2.5 + fi) + rms * 0.5);
        float h = baseH + duneRidge(x * (1.2 + depth * 0.6), fi, ts, warp) + audioLift;
        float hNext = baseH + duneRidge((x + 0.02) * (1.2 + depth * 0.6), fi, ts, warp) + audioLift;
        float slope = (hNext - h) * 28.0;

        float mask = smoothstep(h + 0.004, h - 0.004, y);
        if (mask > 0.001) {
            // Wind-carved sand ripples on dune face
            float ripple = sin((x * 32.0 + y * 55.0 - ts * 2.8) + sin(x * 8.0 + fi) * 2.2) * 0.5 + 0.5;
            float crestGlow = exp(-max(h - y, 0.0) * 48.0);

            vec3 litSand = mix(vec3(0.86, 0.48, 0.28), vec3(0.96, 0.64, 0.32), depth);
            vec3 shadeSand = mix(vec3(0.34, 0.14, 0.22), vec3(0.24, 0.09, 0.12), depth);
            float sunLit = smoothstep(-0.35, 0.45, slope);

            vec3 duneCol = mix(shadeSand, litSand, sunLit);
            duneCol *= 0.92 + 0.12 * ripple * (0.5 + 0.5 * depth);
            // Wind-blown golden sand mist along dune crests
            duneCol += vec3(1.0, 0.82, 0.46) * crestGlow * (0.35 + 0.45 * high + 0.3 * peak);

            // Atmospheric haze on distant dunes
            duneCol = mix(skyLow * 0.85, duneCol, 0.38 + 0.62 * depth);
            col = mix(col, duneCol, mask);
        }
    }

    // Wind-blown dust shimmer streaks
    float windStreak = vnoise(vec2(x * 3.5 - ts * 1.8, y * 18.0 + sin(x * 4.0) * 0.6));
    col += vec3(1.0, 0.78, 0.52) * smoothstep(0.65, 0.95, windStreak) * (0.08 + 0.16 * high + 0.12 * rms);

    // Optional twilight blue-dune palette shift via Ctrl1.x
    vec3 twilight = vec3(col.b * 0.85 + col.r * 0.2, col.g * 0.75 + col.b * 0.25, col.r * 0.9 + col.b * 0.35);
    col = mix(col, twilight, chroma_Ctrl1.x * 0.7);

    col *= intensity;
    col = mix(col, vec3(1.0) - col, step(0.5, invertFlag));

    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
