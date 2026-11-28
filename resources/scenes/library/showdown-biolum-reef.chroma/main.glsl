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
    return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float smin(float a, float b, float k) {
    float h = clamp(0.5 + 0.5 * (b - a) / max(k, 0.0001), 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// Single pulsing alien jellyfish SDF (hollow dome bell + 4 undulating tendrils)
float sdJellyfish(vec3 p, vec3 center, float scale, float phase, float currentAmp, float bass) {
    vec3 q = (p - center) / max(scale, 0.05);
    float swim = sin(phase);
    // Subtle bell contraction
    q.xz *= 1.0 + 0.14 * swim + 0.1 * bass;
    q.y -= 0.16 * cos(phase);

    // Fluted translucent umbrella bell
    float ang = atan(q.z, q.x + 0.00001);
    float flute = 0.035 * sin(ang * 10.0 + phase * 0.5);
    float dOuter = length(vec3(q.x, (q.y - 0.1) * 1.25, q.z)) - (0.68 + flute);
    float dInner = length(vec3(q.x, (q.y + 0.12) * 1.15, q.z)) - 0.56;
    float dBell = max(dOuter, -dInner);
    dBell = max(dBell, -q.y - 0.25);

    // Undulating trailing bioluminescent tentacles
    vec3 qt = q;
    qt.x += sin(qt.y * 3.8 + phase * 1.4) * 0.16 * currentAmp * clamp(-qt.y, 0.0, 2.0);
    qt.z += cos(qt.y * 3.2 - phase * 1.1) * 0.16 * currentAmp * clamp(-qt.y, 0.0, 2.0);
    vec2 ta = abs(qt.xz) - vec2(0.18, 0.18);
    float tLen = clamp(-qt.y, 0.0, 1.75);
    float tRad = 0.048 * (1.0 - tLen * 0.45);
    float dTendrils = length(vec3(ta.x, qt.y + tLen, ta.y)) - max(tRad, 0.012);

    return min(dBell, dTendrils) * scale * 0.75;
}

// Returns vec2(distance, matId): 0 = seabed, 1 = biolum coral, 2 = jellyfish
vec2 mapReef(vec3 p, float ts, float currentAmp, float bass) {
    // Undulating alien seabed
    float floorH = -1.65
        + 0.34 * sin(p.x * 0.42 + p.z * 0.25)
        + 0.22 * cos(p.z * 0.55 - p.x * 0.31);
    float dFloor = p.y - floorH;

    // Domain-repeated swaying bioluminescent coral spires
    vec2 cellId = floor((p.xz + 1.6) / 3.2);
    vec2 localXZ = mod(p.xz + 1.6, 3.2) - 1.6;
    float h = hash21(cellId);
    vec2 coralOff = (vec2(h, fract(h * 17.3)) - 0.5) * 1.3;
    vec3 qc = vec3(localXZ.x - coralOff.x, p.y - floorH, localXZ.y - coralOff.y);

    // Swaying coral stalks with bulbous glowing polyps
    float sway = sin(ts * 1.3 + h * 6.283 + qc.y * 1.8) * 0.12 * currentAmp * clamp(qc.y, 0.0, 1.5);
    qc.x += sway;
    float stalkH = 0.75 + 0.65 * h;
    float cy = clamp(qc.y, 0.0, stalkH);
    float rad = 0.16 * (1.0 - 0.45 * (cy / max(stalkH, 0.1))) + 0.04 * sin(cy * 14.0 + h * 12.0);
    float dCoral = length(vec3(qc.x, qc.y - cy, qc.z)) - rad;
    // Glowing coral crown bulb
    float dBulb = length(qc - vec3(0.0, stalkH + 0.06, 0.0)) - (0.21 + 0.04 * bass);
    dCoral = smin(dCoral, dBulb, 0.14);

    // Keep central viewing lane clear of tall coral
    float laneClear = abs(p.x - sin(p.z * 0.2) * 0.8) - 0.85;
    if (laneClear < 0.0) {
        dCoral += 0.65;
    }

    // Two drifting jellyfish floating above the reef
    vec3 j1Pos = vec3(-0.72 + 0.35 * sin(ts * 0.45), 0.35 + 0.28 * sin(ts * 0.75), p.z - mod(p.z - 2.0, 9.0) + 4.5);
    vec3 j2Pos = vec3(0.95 + 0.3 * cos(ts * 0.38), 0.68 + 0.25 * cos(ts * 0.68), p.z - mod(p.z + 2.5, 9.0) + 4.5);
    float dJelly1 = sdJellyfish(p, j1Pos, 0.72, ts * 2.2, currentAmp, bass);
    float dJelly2 = sdJellyfish(p, j2Pos, 0.56, ts * 2.0 + 2.1, currentAmp, bass);
    float dJelly = min(dJelly1, dJelly2);

    float dTerrain = smin(dFloor, dCoral, 0.22);
    float matId = (dCoral < dFloor + 0.08) ? 1.0 : 0.0;
    float d = dTerrain;
    if (dJelly < d) {
        d = dJelly;
        matId = 2.0;
    }
    return vec2(d, matId);
}

vec3 calcNormal(vec3 p, float ts, float currentAmp, float bass) {
    float e = 0.003;
    vec2 h = vec2(1.0, -1.0);
    return normalize(
        h.xyy * mapReef(p + h.xyy * e, ts, currentAmp, bass).x +
        h.yyx * mapReef(p + h.yyx * e, ts, currentAmp, bass).x +
        h.yxy * mapReef(p + h.yxy * e, ts, currentAmp, bass).x +
        h.xxx * mapReef(p + h.xxx * e, ts, currentAmp, bass).x
    );
}

float causticPattern(vec2 p, float ts) {
    vec2 q = p * 1.4;
    float c1 = sin(q.x * 2.3 + ts * 1.1 + sin(q.y * 2.7 - ts * 0.8));
    float c2 = cos(q.y * 2.5 - ts * 0.9 + cos(q.x * 2.1 + ts * 1.2));
    return pow(clamp(0.5 + 0.5 * c1 * c2, 0.0001, 1.0), 4.0);
}

vec3 rotateHue(vec3 col, float shift) {
    if (abs(shift) < 0.001) return col;
    float a = shift * 6.2831853;
    float cs = cos(a);
    float sn = sin(a);
    vec3 k = vec3(0.57735);
    return max(col * cs + cross(k, col) * sn + k * dot(k, col) * (1.0 - cs), vec3(0.0));
}

vec3 acesToneMap(vec3 x) {
    vec3 a = x * (2.51 * x + 0.03);
    vec3 b = x * (2.43 * x + 0.59) + 0.14;
    return clamp(a / max(b, vec3(0.0001)), 0.0, 1.0);
}

void main() {
    float aspect = chroma_Time.w > 0.01 ? chroma_Time.w : (chroma_Resolution.x / max(chroma_Resolution.y, 1.0));
    vec2 p = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y);

    float t = chroma_Time.x;
    float bpm = chroma_BPM.x > 1.0 ? chroma_BPM.x : 120.0;
    float beatPulse = pow(max(0.5 + 0.5 * sin(t * (bpm / 60.0) * 6.2831853), 0.0001), 4.0);

    float speed = 0.35 + chroma_Ctrl0.x * 1.25;
    float intensity = 0.78 + chroma_Ctrl0.y * 0.85;
    float currentAmp = 0.45 + chroma_Ctrl0.z * 1.2;
    float invertFlag = chroma_Ctrl0.w;
    float hueShift = (chroma_Ctrl1.x - 0.5) * 0.14;

    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]);
    float mid = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]);
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]);

    float ts = t * speed;
    float zCam = ts * 1.25;
    vec3 ro = vec3(sin(zCam * 0.2) * 0.65, 0.05 + 0.14 * sin(ts * 0.5), zCam);
    vec3 ta = vec3(
        sin((zCam + 3.5) * 0.2) * 0.65 + (chroma_Ctrl1.y - 0.5) * 1.5,
        -0.08 + (chroma_Ctrl1.z - 0.5) * 1.2,
        zCam + 3.5
    );

    vec3 fwd = normalize(ta - ro);
    vec3 right = normalize(cross(fwd, vec3(0.0, 1.0, 0.0)));
    vec3 camUp = cross(right, fwd);
    vec3 rd = normalize(p.x * right + p.y * camUp + 0.85 * fwd);

    // Deep abyssal underwater gradient (+Y up toward surface light)
    float upGrad = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    vec3 waterCol = mix(vec3(0.006, 0.022, 0.048), vec3(0.025, 0.14, 0.22), pow(max(upGrad, 0.0001), 1.6));

    float rayT = 0.05;
    float maxT = 20.0;
    bool hit = false;
    float hitMat = 0.0;
    vec3 volumeGlow = vec3(0.0);

    for (int i = 0; i < 60; i++) {
        vec3 pos = ro + rd * rayT;
        vec2 res = mapReef(pos, ts, currentAmp, bass);
        float d = res.x;

        // Volumetric caustic light shafts streaming down from above (+Y)
        float shaft = causticPattern(pos.xz * 0.35 + vec2(pos.y * 0.25, 0.0), ts * 0.7);
        float heightFade = smoothstep(-2.0, 2.2, pos.y);
        float atten = exp(-rayT * 0.13);
        volumeGlow += vec3(0.08, 0.58, 0.78) * shaft * heightFade * atten * (0.014 + 0.012 * mid);

        // Bioluminescent jellyfish halo scatter
        if (res.y > 1.5) {
            volumeGlow += mix(vec3(0.12, 0.92, 1.15), vec3(0.85, 0.25, 1.2), 0.45)
                        * exp(-max(d, 0.0) * 5.0) * atten * (0.025 + 0.03 * bass);
        }

        if (d < 0.0025 * (1.0 + rayT * 0.12)) {
            hit = true;
            hitMat = res.y;
            break;
        }
        rayT += max(d * 0.85, 0.015);
        if (rayT > maxT) break;
    }

    vec3 col = waterCol;

    if (hit) {
        vec3 pos = ro + rd * rayT;
        vec3 n = calcNormal(pos, ts, currentAmp, bass);
        vec3 viewDir = -rd;
        float ndv = clamp(dot(n, viewDir), 0.0, 1.0);
        float fresnel = pow(clamp(1.0 - ndv, 0.0001, 1.0), 2.8);

        vec3 sunDir = normalize(vec3(0.25, 0.92, 0.28));
        float diff = max(dot(n, sunDir), 0.0);
        float caust = causticPattern(pos.xz * 0.75, ts) * max(n.y, 0.0);

        vec3 lit = vec3(0.0);
        if (hitMat > 1.5) {
            // Translucent bioluminescent jellyfish with cyan-violet-amber subsurface glow
            float bellPulse = 0.5 + 0.5 * sin(pos.y * 6.0 - ts * 3.0);
            vec3 sssCol = mix(vec3(0.12, 0.95, 1.25), vec3(0.88, 0.22, 1.25), bellPulse);
            vec3 coreAmber = vec3(1.25, 0.65, 0.18) * pow(max(ndv, 0.0001), 2.2);
            lit = sssCol * (0.35 + 1.35 * fresnel) * (0.9 + 0.85 * bass + 0.45 * beatPulse)
                + coreAmber * (0.55 + 0.75 * mid);
        } else if (hitMat > 0.5) {
            // Alien bioluminescent coral spires
            float tipGlow = smoothstep(-0.9, 0.05, pos.y);
            vec3 coralBase = vec3(0.06, 0.08, 0.18);
            vec3 bioCol = mix(vec3(0.12, 0.98, 0.85), vec3(1.15, 0.52, 0.18), 0.5 + 0.5 * sin(pos.x * 1.4 + pos.z * 1.1));
            lit = coralBase * (0.25 + 0.75 * diff)
                + bioCol * (0.35 * fresnel + 1.15 * tipGlow) * (0.85 + 0.85 * mid + 0.4 * beatPulse)
                + vec3(0.18, 0.72, 0.88) * caust * 0.45;
        } else {
            // Seabed with dappled underwater caustics
            vec3 sandCol = vec3(0.025, 0.055, 0.095);
            lit = sandCol * (0.3 + 0.7 * diff) + vec3(0.18, 0.78, 0.95) * caust * (0.65 + 0.5 * high);
        }

        float fog = exp(-rayT * 0.115);
        col = mix(waterCol, lit, fog);
    }

    // Floating 3D bioluminescent plankton specks
    vec2 sp = p * 9.0 + vec2(sin(ts * 0.35), ts * 0.45);
    vec2 cell = floor(sp);
    vec2 fl = fract(sp) - 0.5;
    float ph = hash21(cell);
    if (ph > 0.62) {
        vec2 drift = vec2(sin(ts * 1.2 + ph * 12.0), cos(ts * 0.9 + ph * 9.0)) * 0.25;
        float dp = length(fl - drift);
        float speck = exp(-dp * dp * 140.0) * (0.65 + 0.95 * high + 0.5 * peak);
        vec3 pCol = mix(vec3(0.15, 1.0, 0.95), vec3(1.05, 0.72, 0.25), fract(ph * 7.3));
        col += pCol * speck;
    }

    col += volumeGlow * (1.0 + 0.45 * rms);
    col = rotateHue(col, hueShift);
    col *= intensity * (0.95 + 0.28 * rms);
    col = acesToneMap(col);
    col = pow(clamp(col, 0.0, 1.0), vec3(0.4545));
    col = mix(col, vec3(1.0) - col, step(0.5, invertFlag));

    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
