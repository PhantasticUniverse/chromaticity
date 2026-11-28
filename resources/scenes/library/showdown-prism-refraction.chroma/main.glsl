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

vec2 rot2(vec2 v, float a) {
    float c = cos(a);
    float s = sin(a);
    return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

// Sharp faceted floating crystal (octahedron intersected with rotated icosa-box)
float sdCrystal(vec3 p, float ts, float bass) {
    vec3 q = p - vec3(0.0, 0.18 + 0.12 * sin(ts * 1.1), 0.0);
    q.xz = rot2(q.xz, ts * 0.45);
    q.xy = rot2(q.xy, 0.35 + 0.18 * sin(ts * 0.6));

    float s = 1.18 + 0.09 * bass;
    vec3 qa = abs(q);
    float dOct = (qa.x + qa.y + qa.z - s) * 0.57735027;

    // Secondary facet cut for jewel-like crown bevels
    vec3 q2 = q;
    q2.xz = rot2(q2.xz, 0.785398);
    q2.yz = rot2(q2.yz, 0.523599);
    vec3 qa2 = abs(q2);
    float dFac = (qa2.x + qa2.y + qa2.z - s * 1.12) * 0.57735027;

    return max(dOct, dFac);
}

vec3 calcCrystalNormal(vec3 p, float ts, float bass) {
    float e = 0.0018;
    vec2 h = vec2(1.0, -1.0);
    return normalize(
        h.xyy * sdCrystal(p + h.xyy * e, ts, bass) +
        h.yyx * sdCrystal(p + h.yyx * e, ts, bass) +
        h.yxy * sdCrystal(p + h.yxy * e, ts, bass) +
        h.xxx * sdCrystal(p + h.xxx * e, ts, bass)
    );
}

// Smooth spectral wavelength RGB color for t in [0..1] (Red -> Violet)
vec3 spectralColor(float w, float hueShift) {
    float x = clamp(w, 0.0, 1.0);
    vec3 col = 0.52 + 0.48 * cos(6.2831853 * (x * 0.82 + hueShift + vec3(0.0, 0.33, 0.67)));
    return max(col, vec3(0.0));
}

// Shortest distance from 3D point p to ray (origin o, unit dir d, forward only)
float distToRay(vec3 p, vec3 o, vec3 d) {
    vec3 op = p - o;
    float proj = max(dot(op, d), 0.0);
    return length(op - d * proj);
}

vec3 shadeCrystal(vec3 pos, vec3 rd, vec3 n, float ts, float disp, float hueShift, float bass, float mid, float high) {
    vec3 viewDir = -rd;
    float ndv = clamp(dot(n, viewDir), 0.0, 1.0);
    float fresnel = pow(clamp(1.0 - ndv, 0.0001, 1.0), 3.2);

    // Chromatic RGB internal refraction vectors (different IOR per channel)
    vec3 refrR = refract(rd, n, 1.0 / (1.44 + 0.06 * disp));
    vec3 refrG = refract(rd, n, 1.0 / (1.52 + 0.10 * disp));
    vec3 refrB = refract(rd, n, 1.0 / (1.62 + 0.15 * disp));

    // Internal total-internal-reflection sparkle & prismatic bands
    float bandR = pow(max(0.5 + 0.5 * sin(dot(refrR, vec3(7.0, 5.0, 6.0)) + ts), 0.0001), 3.0);
    float bandG = pow(max(0.5 + 0.5 * sin(dot(refrG, vec3(7.0, 5.0, 6.0)) + ts + 0.4), 0.0001), 3.0);
    float bandB = pow(max(0.5 + 0.5 * sin(dot(refrB, vec3(7.0, 5.0, 6.0)) + ts + 0.8), 0.0001), 3.0);
    vec3 internalDisp = vec3(bandR, bandG, bandB) * (0.95 + 0.65 * mid);

    // Crisp facet specular highlights
    vec3 l1 = normalize(vec3(-0.7, 0.75, 0.5));
    vec3 l2 = normalize(vec3(0.75, 0.45, -0.45));
    float spec1 = pow(max(dot(n, normalize(l1 + viewDir)), 0.0001), 96.0);
    float spec2 = pow(max(dot(n, normalize(l2 + viewDir)), 0.0001), 64.0);

    vec3 edgeSpec = spectralColor(fresnel * 0.9 + pos.x * 0.25 + 0.2, hueShift);
    vec3 col = internalDisp * 0.85 + edgeSpec * fresnel * (1.25 + 0.75 * bass);
    col += vec3(1.25, 1.2, 1.3) * spec1 * (1.35 + 0.6 * high);
    col += edgeSpec * spec2 * 0.9;
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
    float bpm = syn_BPM.x > 1.0 ? syn_BPM.x : 120.0;
    float beatPulse = pow(max(0.5 + 0.5 * sin(t * (bpm / 60.0) * 6.2831853), 0.0001), 4.0);

    float speed = 0.4 + chroma_Ctrl0.x * 1.3;
    float intensity = 0.8 + chroma_Ctrl0.y * 0.85;
    float disp = 0.45 + chroma_Ctrl0.z * 1.1;
    float invertFlag = chroma_Ctrl0.w;
    float hueShift = chroma_Ctrl1.x - 0.5;

    float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
    float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]);
    float mid = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]);
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]);

    float ts = t * speed;
    float floorY = -1.28;

    // Camera framing the crystal, full reflection, and glowing spectral caustic fan on the dark mirror plane
    float camA = 0.22 * sin(ts * 0.25) + (chroma_Ctrl1.y - 0.5) * 1.6;
    float camY = 0.95 + (chroma_Ctrl1.z - 0.5) * 1.1;
    vec3 ro = vec3(sin(camA) * 5.2, camY, cos(camA) * 5.2);
    vec3 ta = vec3(0.35, -0.35, 0.0);

    vec3 fwd = normalize(ta - ro);
    vec3 right = normalize(cross(fwd, vec3(0.0, 1.0, 0.0)));
    vec3 camUp = cross(right, fwd);
    vec3 rd = normalize(p.x * right + p.y * camUp + 0.92 * fwd);

    // Crystal core center & sweeping incoming white-cyan laser beam
    vec3 prismPos = vec3(0.0, 0.18 + 0.12 * sin(ts * 1.1), 0.0);
    float sweepY = 0.55 + 0.35 * sin(ts * 0.75) + 0.2 * bass;
    vec3 laserSrc = vec3(-5.2, sweepY + 0.65, -0.35);
    vec3 laserDir = normalize(prismPos - laserSrc);

    // Raymarch crystal
    float tCrystal = 0.0;
    bool hitCrystal = false;
    float crystalGlow = 0.0;

    for (int i = 0; i < 56; i++) {
        vec3 pos = ro + rd * tCrystal;
        float d = sdCrystal(pos, ts, bass);
        crystalGlow += exp(-max(d, 0.0) * 8.0) * 0.022;
        if (d < 0.0018) {
            hitCrystal = true;
            break;
        }
        tCrystal += max(d, 0.01);
        if (tCrystal > 14.0) break;
    }

    // Analytical mirror plane intersection
    float tFloor = (rd.y < -0.001) ? (floorY - ro.y) / rd.y : 1000.0;
    bool hitFloor = (tFloor > 0.0 && tFloor < 32.0 && (!hitCrystal || tFloor < tCrystal));

    // Volumetric laser + 7-band spectral dispersion fan along the camera ray
    float maxVolT = hitCrystal ? tCrystal : (hitFloor ? tFloor : 11.0);
    vec3 volBeams = vec3(0.0);
    float volDt = maxVolT / 26.0;

    for (int s = 0; s < 26; s++) {
        vec3 vp = ro + rd * ((float(s) + 0.5) * volDt);

        // Incoming white laser beam (only from laserSrc to prismPos)
        vec3 relIn = vp - laserSrc;
        float projIn = dot(relIn, laserDir);
        float lenIn = length(prismPos - laserSrc);
        if (projIn > 0.0 && projIn < lenIn) {
            float dIn = length(relIn - laserDir * projIn);
            float beamIn = exp(-dIn * dIn * 240.0) + 0.22 * exp(-dIn * 9.0);
            volBeams += vec3(1.1, 1.15, 1.35) * beamIn * volDt * (0.48 + 0.45 * peak);
        }

        // 7 dispersed spectral exit beams fanning out from prismPos
        for (int b = 0; b < 7; b++) {
            float fb = float(b) / 6.0;
            float spreadY = -0.22 - (0.24 + 0.32 * disp) * (fb - 0.15);
            float spreadZ = (fb - 0.5) * (0.72 + 0.55 * disp) + 0.1 * sin(ts * 0.9 + fb * 3.0);
            vec3 exitDir = normalize(vec3(1.0, spreadY, spreadZ));
            float dExit = distToRay(vp, prismPos, exitDir);
            float beam = exp(-dExit * dExit * 110.0) + 0.22 * exp(-dExit * 6.0);
            vec3 sCol = spectralColor(fb, hueShift);
            volBeams += sCol * beam * volDt * (0.36 + 0.4 * mid + 0.3 * beatPulse);
        }
    }

    vec3 col = vec3(0.008, 0.009, 0.018);
    // Subtle dark studio radial backdrop glow
    col += vec3(0.03, 0.035, 0.07) * exp(-length(p) * 2.4);

    if (hitCrystal && !hitFloor) {
        vec3 pos = ro + rd * tCrystal;
        vec3 n = calcCrystalNormal(pos, ts, bass);
        col = shadeCrystal(pos, rd, n, ts, disp, hueShift, bass, mid, high);
    } else if (hitFloor) {
        vec3 fPos = ro + rd * tFloor;
        vec3 fNorm = vec3(0.0, 1.0, 0.0);
        float fFresnel = pow(clamp(1.0 - max(dot(fNorm, -rd), 0.0), 0.0001, 1.0), 3.6);
        float radFade = exp(-length(fPos.xz) * 0.14);

        vec3 floorCol = vec3(0.01, 0.012, 0.02);

        // Spectral RGB chromatic dispersion caustic fan projected onto the dark mirror plane
        for (int b = 0; b < 7; b++) {
            float fb = float(b) / 6.0;
            float spreadY = -0.22 - (0.24 + 0.32 * disp) * (fb - 0.15);
            float spreadZ = (fb - 0.5) * (0.72 + 0.55 * disp) + 0.1 * sin(ts * 0.9 + fb * 3.0);
            vec3 exitDir = normalize(vec3(1.0, spreadY, spreadZ));
            if (exitDir.y < -0.02) {
                float hitT = (floorY - prismPos.y) / exitDir.y;
                vec3 hitP = prismPos + exitDir * hitT;
                vec2 delta = fPos.xz - hitP.xz;
                // Elongated anisotropic caustic streak along beam azimuth
                vec2 az = normalize(exitDir.xz + vec2(0.0001));
                float along = dot(delta, az);
                float across = dot(delta, vec2(-az.y, az.x));
                float caustic = exp(-across * across * 16.0) * exp(-along * along * 1.4);
                floorCol += spectralColor(fb, hueShift) * caustic * (1.65 + 1.2 * bass + 0.75 * beatPulse);
            }
        }

        // Planar reflection of the floating crystal on the mirror plane
        vec3 rRd = reflect(rd, fNorm);
        vec3 rRo = fPos + fNorm * 0.01;
        float rt = 0.0;
        bool rHit = false;
        for (int j = 0; j < 34; j++) {
            float rdDist = sdCrystal(rRo + rRd * rt, ts, bass);
            if (rdDist < 0.004) {
                rHit = true;
                break;
            }
            rt += max(rdDist, 0.018);
            if (rt > 12.0) break;
        }

        vec3 reflCol = vec3(0.01, 0.012, 0.024);
        if (rHit) {
            vec3 rp = rRo + rRd * rt;
            vec3 rn = calcCrystalNormal(rp, ts, bass);
            reflCol = shadeCrystal(rp, rRd, rn, ts, disp, hueShift, bass, mid, high);
        }

        col = floorCol + reflCol * clamp(0.24 + 0.68 * fFresnel, 0.0, 0.9) * radFade;
    }

    // Add volumetric laser dispersion beams & crystal halo bloom
    col += volBeams;
    col += spectralColor(0.18 + ts * 0.04, hueShift) * crystalGlow * (0.45 + 0.65 * bass + 0.45 * peak);

    col *= intensity * (0.95 + 0.28 * rms);
    col = acesToneMap(col);
    col = pow(clamp(col, 0.0, 1.0), vec3(0.4545));
    col = mix(col, vec3(1.0) - col, step(0.5, invertFlag));

    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
