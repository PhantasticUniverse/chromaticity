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

// Seamless (3,5) torus knot SDF by evaluating all 3 modular winding branches
float sdTorusKnot35(vec3 p, float ts, float ribTwist, float bass, float mid) {
    vec3 q = p;
    q.xz = rot2(q.xz, ts * 0.38);
    q.xy = rot2(q.xy, ts * 0.24);

    float majorR = 1.42;
    float minorR = 0.56 + 0.08 * bass;
    float tubeR = 0.21 + 0.03 * mid;

    float a = atan(q.z, q.x + 0.00001);
    vec2 cp = vec2(length(q.xz) - majorR, q.y);

    float dMin = 100.0;
    for (int k = 0; k < 3; k++) {
        float branchAngle = (a + 6.2831853 * float(k)) * (5.0 / 3.0) + ts * 0.7;
        vec2 lp = rot2(cp, branchAngle);
        vec2 offset = lp - vec2(minorR, 0.0);
        float localAng = atan(offset.y, offset.x + 0.00001);
        float ribs = 0.026 * ribTwist * sin(localAng * 6.0 + a * 9.0 - ts * 3.0) * (0.7 + 0.9 * mid);
        float dBranch = length(offset) - (tubeR + ribs);
        dMin = min(dMin, dBranch);
    }
    return dMin * 0.72;
}

vec3 calcKnotNormal(vec3 p, float ts, float ribTwist, float bass, float mid) {
    float e = 0.0022;
    vec2 h = vec2(1.0, -1.0);
    return normalize(
        h.xyy * sdTorusKnot35(p + h.xyy * e, ts, ribTwist, bass, mid) +
        h.yyx * sdTorusKnot35(p + h.yyx * e, ts, ribTwist, bass, mid) +
        h.yxy * sdTorusKnot35(p + h.yxy * e, ts, ribTwist, bass, mid) +
        h.xxx * sdTorusKnot35(p + h.xxx * e, ts, ribTwist, bass, mid)
    );
}

vec3 thinFilmIridescence(float phase, float hueShift) {
    return 0.52 + 0.48 * cos(6.2831853 * (phase + hueShift + vec3(0.0, 0.33, 0.67)));
}

vec3 studioEnv(vec3 rd, float ts, float hueShift, float high) {
    float up = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    vec3 bg = mix(vec3(0.015, 0.012, 0.032), vec3(0.04, 0.06, 0.13), up);

    // Dual studio neon softbox strips
    float stripA = exp(-pow((rd.y - 0.55) * 7.0, 2.0)) * smoothstep(-0.2, 0.6, cos(rd.x * 3.2 + ts * 0.4));
    float stripB = exp(-pow((rd.y - 0.18) * 11.0, 2.0)) * smoothstep(0.1, 0.85, sin(rd.z * 4.0 - ts * 0.5));
    bg += thinFilmIridescence(0.15 + ts * 0.04, hueShift) * stripA * (1.8 + 0.8 * high);
    bg += thinFilmIridescence(0.62 - ts * 0.03, hueShift) * stripB * 1.4;
    return bg;
}

vec3 shadeKnot(vec3 pos, vec3 rd, float ts, float ribTwist, float hueShift, float bass, float mid, float high, float beatPulse) {
    vec3 n = calcKnotNormal(pos, ts, ribTwist, bass, mid);
    vec3 viewDir = -rd;
    float ndv = clamp(dot(n, viewDir), 0.0, 1.0);
    float fresnel = pow(clamp(1.0 - ndv, 0.0001, 1.0), 3.6);

    vec3 reflDir = reflect(rd, n);
    vec3 envRefl = studioEnv(reflDir, ts, hueShift, high);

    // Thin-film interference chrome color
    vec3 irid = thinFilmIridescence(fresnel * 1.35 + ndv * 0.55 + pos.y * 0.18 + ts * 0.05, hueShift);

    // Key & rim directional lights
    vec3 l1 = normalize(vec3(0.65, 0.85, 0.55));
    vec3 l2 = normalize(vec3(-0.75, 0.35, -0.55));
    float spec1 = pow(max(dot(n, normalize(l1 + viewDir)), 0.0001), 64.0);
    float spec2 = pow(max(dot(n, normalize(l2 + viewDir)), 0.0001), 42.0);

    // Subsurface neon glow through translucent liquid-glass body
    float sss = pow(clamp(1.0 - ndv, 0.0001, 1.0), 1.8) * (0.45 + 0.85 * bass + 0.45 * beatPulse);
    vec3 sssCol = thinFilmIridescence(0.45 + pos.x * 0.22 - ts * 0.08, hueShift);

    vec3 col = mix(vec3(0.05, 0.06, 0.11), envRefl * (0.65 + 0.85 * irid), 0.35 + 0.65 * fresnel);
    col += sssCol * sss * 0.72;
    col += vec3(1.1, 0.96, 1.0) * spec1 * 1.25;
    col += irid * spec2 * (0.9 + 0.7 * mid);
    return col;
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

    float speed = 0.4 + chroma_Ctrl0.x * 1.3;
    float intensity = 0.78 + chroma_Ctrl0.y * 0.85;
    float ribTwist = 0.35 + chroma_Ctrl0.z * 1.4;
    float invertFlag = chroma_Ctrl0.w;
    float hueShift = chroma_Ctrl1.x - 0.5;

    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]);
    float mid = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]);
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]);

    float ts = t * speed;
    float floorY = -2.25;

    float orbitAng = ts * 0.22 + (chroma_Ctrl1.y - 0.5) * 3.0;
    float camY = 0.55 + (chroma_Ctrl1.z - 0.5) * 1.6;
    vec3 ro = vec3(sin(orbitAng) * 5.3, camY, cos(orbitAng) * 5.3);
    vec3 ta = vec3(0.0, -0.25, 0.0);

    vec3 fwd = normalize(ta - ro);
    vec3 right = normalize(cross(fwd, vec3(0.0, 1.0, 0.0)));
    vec3 camUp = cross(right, fwd);
    vec3 rd = normalize(p.x * right + p.y * camUp + 0.88 * fwd);

    // Primary raymarch for torus knot
    float tKnot = 0.0;
    bool hitKnot = false;
    float neonGlow = 0.0;

    for (int i = 0; i < 58; i++) {
        vec3 pos = ro + rd * tKnot;
        float d = sdTorusKnot35(pos, ts, ribTwist, bass, mid);
        neonGlow += exp(-max(d, 0.0) * 6.5) * 0.022;
        if (d < 0.0022) {
            hitKnot = true;
            break;
        }
        tKnot += max(d, 0.012);
        if (tKnot > 15.0) break;
    }

    // Analytical mirror floor intersection
    float tFloor = (rd.y < -0.001) ? (floorY - ro.y) / rd.y : 1000.0;
    bool hitFloor = (tFloor > 0.0 && tFloor < 36.0 && (!hitKnot || tFloor < tKnot));

    vec3 col = studioEnv(rd, ts, hueShift, high) * 0.25;

    if (hitKnot && !hitFloor) {
        vec3 pos = ro + rd * tKnot;
        col = shadeKnot(pos, rd, ts, ribTwist, hueShift, bass, mid, high, beatPulse);
    } else if (hitFloor) {
        vec3 fPos = ro + rd * tFloor;
        vec3 fNorm = vec3(0.0, 1.0, 0.0);
        float fFresnel = pow(clamp(1.0 - max(dot(fNorm, -rd), 0.0), 0.0001, 1.0), 4.0);

        // Floor cybernetic grid lines
        vec2 grid = abs(fract(fPos.xz * 0.65 - 0.5) - 0.5) / 0.035;
        float line = exp(-min(grid.x, grid.y) * 1.4);
        float radFade = exp(-length(fPos.xz) * 0.14);

        vec3 floorCol = vec3(0.012, 0.014, 0.024);
        floorCol += thinFilmIridescence(length(fPos.xz) * 0.08 - ts * 0.06, hueShift) * line * radFade * (0.28 + 0.45 * mid);

        // 1-bounce reflection raymarch of the floating torus knot
        vec3 rRd = reflect(rd, fNorm);
        vec3 rRo = fPos + fNorm * 0.01;
        float rt = 0.0;
        bool rHit = false;
        float rGlow = 0.0;
        for (int j = 0; j < 36; j++) {
            vec3 rp = rRo + rRd * rt;
            float rdDist = sdTorusKnot35(rp, ts, ribTwist, bass, mid);
            rGlow += exp(-max(rdDist, 0.0) * 5.0) * 0.02;
            if (rdDist < 0.005) {
                rHit = true;
                break;
            }
            rt += max(rdDist, 0.02);
            if (rt > 14.0) break;
        }

        vec3 reflCol = studioEnv(rRd, ts, hueShift, high) * 0.4;
        if (rHit) {
            reflCol = shadeKnot(rRo + rRd * rt, rRd, ts, ribTwist, hueShift, bass, mid, high, beatPulse);
        } else {
            reflCol += thinFilmIridescence(0.35 + ts * 0.05, hueShift) * rGlow * 0.6;
        }

        // Subtle contact shadow directly beneath the knot
        float underShadow = 1.0 - 0.55 * exp(-length(fPos.xz) * 0.55);
        col = mix(floorCol * underShadow, reflCol, clamp(0.28 + 0.65 * fFresnel, 0.0, 0.92) * radFade);
    }

    // Subsurface halo bloom around knot
    vec3 haloCol = thinFilmIridescence(0.55 + length(p) * 0.4 - ts * 0.07, hueShift);
    col += haloCol * neonGlow * (0.35 + 0.7 * bass + 0.45 * peak);

    col *= intensity * (0.95 + 0.28 * rms);
    col = acesToneMap(col);
    col = pow(clamp(col, 0.0, 1.0), vec3(0.4545));
    col = mix(col, vec3(1.0) - col, step(0.5, invertFlag));

    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
