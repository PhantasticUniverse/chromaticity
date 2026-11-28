// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;        // .x = elapsed seconds, .y = dt, .z = frame, .w = aspect
    vec4 chroma_Resolution;  // .x = width, .y = height, .z = 1/w, .w = 1/h
    vec4 chroma_BPM;            // .x = BPM
    vec4 chroma_AudioRMS;    // .x = RMS [0..1]
    vec4 chroma_AudioPeak;   // .x = Peak [0..1]
    float chroma_AudioFFT[8];// 8 log-spaced FFT bands [0..1]
    vec4 chroma_Ctrl0;       // .x..w = controls 0..3 [0..1]
    vec4 chroma_Ctrl1;       // .x..w = controls 4..7 [0..1]
};

vec2 rot2(vec2 v, float a) {
    float c = cos(a);
    float s = sin(a);
    return vec2(c * v.x - s * v.y, s * v.x + c * v.y);
}

float sampleFFT(float x) {
    float t = clamp(x, 0.0, 1.0) * 7.0;
    float idx = floor(t);
    float f = fract(t);
    if (idx < 0.5) return mix(chroma_AudioFFT[0], chroma_AudioFFT[1], f);
    if (idx < 1.5) return mix(chroma_AudioFFT[1], chroma_AudioFFT[2], f);
    if (idx < 2.5) return mix(chroma_AudioFFT[2], chroma_AudioFFT[3], f);
    if (idx < 3.5) return mix(chroma_AudioFFT[3], chroma_AudioFFT[4], f);
    if (idx < 4.5) return mix(chroma_AudioFFT[4], chroma_AudioFFT[5], f);
    if (idx < 5.5) return mix(chroma_AudioFFT[5], chroma_AudioFFT[6], f);
    return mix(chroma_AudioFFT[6], chroma_AudioFFT[7], f);
}

vec3 acesToneMap(vec3 x) {
    vec3 num = x * (2.51 * x + 0.03);
    vec3 den = x * (2.43 * x + 0.59) + 0.14;
    return clamp(num / max(den, vec3(0.0001)), 0.0, 1.0);
}

// Luxury thin-film opal / oil-slick spectral palette (deep indigo, electric cyan, molten gold, magenta)
vec3 thinFilmOpal(float opd, float shift) {
    vec3 a = vec3(0.32, 0.28, 0.42);
    vec3 b = vec3(0.55, 0.48, 0.52);
    vec3 c = vec3(1.0, 0.92, 0.88);
    vec3 d = vec3(0.05, 0.36, 0.68) + shift;
    vec3 col = clamp(a + b * cos(6.2831853 * (c * opd + d)), 0.0, 1.25);
    return col * col * 1.25;
}

// Silky C-infinity curl-advected liquid mercury heightfield (zero sharp crease seams)
vec3 opalField(vec2 p, float t, float turb, float bass, float mid, float high) {
    vec2 q = p * 1.25;
    float flowAcc = 0.0;
    float amp = 0.62;
    float freq = 1.15;
    float h = 0.0;

    for (int i = 0; i < 4; i++) {
        float fi = float(i);
        q = rot2(q, 0.52 + fi * 0.41 + 0.06 * sin(t * 0.3 + fi));
        vec2 w = vec2(
            sin(q.y * freq + t * (0.58 + 0.16 * fi) + cos(q.x * freq * 0.85 - t * 0.42)),
            cos(q.x * freq - t * (0.52 + 0.13 * fi) + sin(q.y * freq * 0.92 + t * 0.38))
        );
        // Smooth curl-like fluid advection
        q += vec2(-w.y, w.x) * (0.34 + 0.38 * turb + 0.12 * mid) * amp;

        float wave = sin(q.x * freq * 1.25 + q.y * freq * 0.95 - t * (0.82 + 0.22 * fi));
        // C-infinity rounded crest (no sharp abs() kink!)
        float smoothRidge = 1.0 - sqrt(wave * wave + 0.18);
        smoothRidge = max(smoothRidge, 0.0);

        float fftBoost = sampleFFT(fi * 0.25 + 0.1);
        h += (wave * 0.68 + smoothRidge * 0.45) * amp * (1.0 + 0.32 * fftBoost);
        flowAcc += smoothRidge * amp;

        freq *= 1.54;
        amp *= 0.52;
    }

    float r = max(length(p), 0.0001);
    float ripple = sin(r * 9.5 - t * 3.1 - bass * 2.0) * exp(-r * 1.25) * (0.14 + 0.28 * bass);
    float micro = sin(q.x * 5.2 + t * 1.6) * cos(q.y * 5.0 - t * 1.4) * (0.02 + 0.04 * high);

    h += ripple + micro;
    return vec3(h, flowAcc, length(q));
}

// High-contrast dark studio chrome environment map with gleaming gold/cyan/magenta softbox ribbons
vec3 chromeEnv(vec3 R, float t, float iridShift) {
    vec3 d = normalize(R + vec3(0.0, 0.0, 0.00001));
    float elev = d.y;
    float az = atan(d.z, d.x);

    // Deep obsidian-indigo dark base for dramatic liquid-chrome contrast
    vec3 env = mix(vec3(0.008, 0.006, 0.028), vec3(0.028, 0.018, 0.085), smoothstep(-0.5, 0.85, elev));

    // Crisp studio reflection ribbons
    float band1 = exp(-pow(elev - 0.36 - 0.14 * sin(az * 2.0 + t * 0.55), 2.0) * 42.0);
    float band2 = exp(-pow(elev + 0.22 - 0.15 * cos(az * 2.0 - t * 0.48), 2.0) * 34.0);
    float meridian = pow(max(0.5 + 0.5 * cos(az * 3.0 + t * 0.75 + elev * 3.0), 0.0), 14.0);

    vec3 goldLight = vec3(1.0, 0.82, 0.36);
    vec3 cyanLight = vec3(0.12, 0.96, 1.0);
    vec3 magentaLight = vec3(1.0, 0.14, 0.78);

    env += goldLight * band1 * 2.1;
    env += cyanLight * band2 * 1.85;
    env += mix(magentaLight, cyanLight, 0.5 + 0.5 * sin(az + t + iridShift * 6.2831853)) * meridian * 1.65;

    // Bright overhead white-chrome specular strip
    float zenithStrip = smoothstep(0.84, 0.98, abs(sin(az * 1.5 - t * 0.32))) * smoothstep(0.22, 0.88, elev);
    env += vec3(0.96, 0.98, 1.0) * zenithStrip * 1.45;

    return env;
}

void main() {
    float aspect = chroma_Time.w > 0.01 ? chroma_Time.w : (chroma_Resolution.x / max(chroma_Resolution.y, 1.0));
    float t = chroma_Time.x;
    float bpm = chroma_BPM.x > 1.0 ? chroma_BPM.x : 120.0;
    float beat = pow(max(0.5 + 0.5 * sin(t * (bpm / 60.0) * 6.2831853), 0.0), 3.0);

    float speed = 0.35 + chroma_Ctrl0.x * 1.35;
    float intensity = 0.74 + chroma_Ctrl0.y * 0.86;
    float turb = 0.25 + chroma_Ctrl0.z * 0.95;
    float iridAmt = 0.35 + chroma_Ctrl0.w * 0.95;
    float chromeAmt = 0.5 + chroma_Ctrl1.x * 0.5;

    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]);
    float mid = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]);
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]);

    vec2 focusOff = (chroma_Ctrl1.y > 0.001 || chroma_Ctrl1.z > 0.001) ? (chroma_Ctrl1.yz - 0.5) * 0.4 : vec2(0.0);
    vec2 p = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y) * 2.0 - focusOff;
    float ts = t * speed;

    vec2 camP = rot2(p, sin(ts * 0.14) * 0.16) * (1.0 - 0.07 * bass - 0.03 * beat);

    // Central-difference 3D surface normal
    float eps = 0.007;
    vec3 fC = opalField(camP, ts, turb, bass, mid, high);
    vec3 fX = opalField(camP + vec2(eps, 0.0), ts, turb, bass, mid, high);
    vec3 fY = opalField(camP + vec2(0.0, eps), ts, turb, bass, mid, high);

    float relief = 0.62 + 0.25 * bass + 0.2 * turb;
    vec3 N = normalize(vec3((fC.x - fX.x) * relief / eps, (fC.x - fY.x) * relief / eps, 1.0));
    vec3 V = normalize(vec3(-camP * 0.42, 1.38));
    float NdotV = clamp(dot(N, V), 0.0001, 1.0);
    vec3 R = reflect(-V, N);

    float fresnel = 0.10 + 0.90 * pow(max(1.0 - NdotV, 0.0001), 3.6);

    // Optical thin-film interference
    float sin2 = max(1.0 - NdotV * NdotV, 0.0);
    float filmThickness = (0.95 + 0.72 * fC.x + 0.45 * fC.y) * (0.65 + 0.65 * iridAmt);
    float opd = filmThickness * sqrt(max(1.0 - 0.42 * sin2, 0.0001)) + ts * 0.09;
    vec3 opalCol = thinFilmOpal(opd, ts * 0.04 + fC.z * 0.04);

    // Deep obsidian shadow troughs for luxury liquid-metal depth
    float ao = smoothstep(-0.85, 0.75, fC.x) * 0.85 + 0.15;

    vec3 refl = chromeEnv(R, ts, opd * 0.12);

    // 3 orbiting luxury point lights
    vec3 L1 = normalize(vec3(cos(ts * 0.65) * 1.5, sin(ts * 0.5) * 1.2 + 0.5, 1.1));
    vec3 L2 = normalize(vec3(sin(ts * 0.55 + 2.1) * 1.6, cos(ts * 0.7) * 1.1, 0.9));
    vec3 L3 = normalize(vec3(-cos(ts * 0.42) * 1.4, -sin(ts * 0.62) * 1.3, 1.2));

    vec3 H1 = normalize(L1 + V);
    vec3 H2 = normalize(L2 + V);
    vec3 H3 = normalize(L3 + V);

    float spec1 = pow(max(dot(N, H1), 0.0001), 130.0) * (2.2 + 1.3 * bass);
    float spec2 = pow(max(dot(N, H2), 0.0001), 86.0) * (1.7 + 1.0 * mid);
    float spec3 = pow(max(dot(N, H3), 0.0001), 180.0) * (2.4 + 1.5 * high);

    float diff1 = max(dot(N, L1), 0.0);
    float diff2 = max(dot(N, L2), 0.0);

    vec3 baseBody = vec3(0.012, 0.008, 0.036) + opalCol * (0.14 + 0.36 * diff1 + 0.22 * diff2) * ao;
    vec3 chromeSheen = refl * mix(vec3(1.0), opalCol * 1.45 + vec3(0.12), 0.52 * iridAmt) * (0.45 + 0.55 * ao);

    vec3 col = mix(baseBody, chromeSheen, clamp(chromeAmt * (0.45 + 0.55 * fresnel), 0.0, 0.96));

    // Luminous opal crest shimmer
    float crest = smoothstep(0.35, 0.95, fC.y) * (0.28 + 0.65 * rms + 0.4 * beat);
    col += thinFilmOpal(opd + 0.32, 0.12) * crest * 0.52 * ao;

    // Razor-sharp wet chrome specular glints
    col += vec3(1.0, 0.88, 0.52) * spec1;
    col += vec3(0.22, 0.98, 1.0) * spec2;
    col += vec3(1.0, 0.28, 0.86) * spec3;
    col += opalCol * fresnel * (0.38 + 0.48 * peak);

    float vig = 1.0 - smoothstep(0.75, 2.15, length(p));
    col *= (0.42 + 0.58 * vig) * intensity;

    col = acesToneMap(col);
    col = pow(max(col, vec3(0.0001)), vec3(0.92));

    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
