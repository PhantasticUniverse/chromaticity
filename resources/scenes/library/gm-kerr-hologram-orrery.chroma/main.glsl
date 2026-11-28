// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;        // .x = elapsed seconds, .y = dt, .z = frame, .w = aspect
    vec4 chroma_Resolution;  // .x = width, .y = height, .z = 1/w, .w = 1/h
    vec4 chroma_BPM;            // .x = BPM, .y = beat_time, .z = on_beat, .w = bpm_sin
    vec4 chroma_AudioRMS;    // .x = RMS [0..1]
    vec4 chroma_AudioPeak;   // .x = Peak [0..1]
    float chroma_AudioFFT[8];// 8 log-spaced FFT bands [0..1]
    vec4 chroma_Ctrl0;       // .x=speed, .y=audioDrive, .z=lensing, .w=ringScale
    vec4 chroma_Ctrl1;       // .x=mirrorFold, .y=dispersion, .z=glassGlare, .w=accentTint
    vec4 chroma_Ctrl2;       // .x=intensity, .y=strobeGate, .z=dropShockwave, .w=tiltX
    vec4 chroma_Ctrl3;       // .x=tiltY, .y=scriptDriftX, .z=scriptDriftY, .w=scriptPulseEnv
};

vec2 rot2(vec2 v, float a) {
    float c = cos(a);
    float s = sin(a);
    return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

// Analytic CIE 1931 spectral sensitivity -> linear sRGB
vec3 spectralRGB(float wl) {
    float t1 = (wl - 442.0) * (wl < 442.0 ? 0.0624 : 0.0374);
    float t2 = (wl - 599.8) * (wl < 599.8 ? 0.0264 : 0.0323);
    float t3 = (wl - 501.1) * (wl < 501.1 ? 0.0490 : 0.0382);
    float x = 0.362 * exp(-0.5 * t1 * t1) + 1.056 * exp(-0.5 * t2 * t2) - 0.065 * exp(-0.5 * t3 * t3);
    float u1 = (wl - 568.8) * (wl < 568.8 ? 0.0213 : 0.0247);
    float u2 = (wl - 530.9) * (wl < 530.9 ? 0.0613 : 0.0322);
    float y = 0.821 * exp(-0.5 * u1 * u1) + 0.286 * exp(-0.5 * u2 * u2);
    float v1 = (wl - 437.0) * (wl < 437.0 ? 0.0845 : 0.0278);
    float v2 = (wl - 459.0) * (wl < 459.0 ? 0.0385 : 0.0725);
    float z = 1.217 * exp(-0.5 * v1 * v1) + 0.681 * exp(-0.5 * v2 * v2);
    vec3 rgb = vec3(
         3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
         0.0557 * x - 0.2040 * y + 1.0570 * z
    );
    return max(rgb, vec3(0.0));
}

float hash21(vec2 p) {
    p = fract(p * vec2(234.34, 435.345));
    p += dot(p, p + 34.23);
    return fract(p.x * p.y);
}

float sdTorus(vec3 p, vec2 t) {
    vec2 q = vec2(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

float sdOcta(vec3 p, float s) {
    p = abs(p);
    return (p.x + p.y + p.z - s) * 0.57735027;
}

// Recessed 3D celestial orrery SDF (3 gyroscopic rings + orbiting faceted jewels)
vec2 mapOrrery(vec3 p, float t, float ringScale, float shock, float bass, float mid, float high) {
    float d = 1e5;
    float mat = 0.0;
    float scaleK = mix(0.78, 1.24, ringScale) + shock * 0.08;

    // Outer calibrated holographic ring
    vec3 p1 = p;
    p1.xy = rot2(p1.xy, t * 0.40 + 0.35);
    p1.yz = rot2(p1.yz, 0.58 + 0.14 * sin(t * 0.6));
    float ring1 = sdTorus(p1, vec2(1.42 * scaleK + 0.05 * bass, 0.038 + 0.008 * shock));
    if (ring1 < d) { d = ring1; mat = 1.0; }

    // Middle dichroic gyro ring
    vec3 p2 = p;
    p2.xz = rot2(p2.xz, -t * 0.62);
    p2.xy = rot2(p2.xy, 1.08 + 0.20 * cos(t * 0.5));
    float ring2 = sdTorus(p2, vec2(1.12 * scaleK + 0.045 * mid, 0.032 + 0.006 * shock));
    if (ring2 < d) { d = ring2; mat = 2.0; }

    // Inner fast harmonic ring
    vec3 p3 = p;
    p3.yz = rot2(p3.yz, t * 0.88);
    p3.xz = rot2(p3.xz, 0.72 + t * 0.25);
    float ring3 = sdTorus(p3, vec2(0.84 * scaleK + 0.04 * high, 0.025 + 0.005 * shock));
    if (ring3 < d) { d = ring3; mat = 3.0; }

    // 3 orbiting faceted crystal planets
    for (int i = 0; i < 3; i++) {
        float fi = float(i);
        float ang = t * (0.75 + 0.22 * fi) + fi * 2.0944;
        float rad = (1.12 + 0.30 * mod(fi, 2.0)) * scaleK;
        vec3 mp = vec3(cos(ang) * rad, 0.28 * sin(ang * 1.5 + fi), sin(ang) * rad);
        vec3 q = p - mp;
        q.xy = rot2(q.xy, t * 1.5 + fi);
        q.yz = rot2(q.yz, t * 1.0);
        float moon = sdOcta(q, (0.13 + 0.03 * bass + 0.03 * shock) * mix(0.85, 1.15, ringScale));
        if (moon < d) { d = moon; mat = 4.0; }
    }

    return vec2(d, mat);
}

vec3 calcNormal(vec3 p, float t, float ringScale, float shock, float bass, float mid, float high) {
    vec2 e = vec2(0.0018, -0.0018);
    return normalize(
        e.xyy * mapOrrery(p + e.xyy, t, ringScale, shock, bass, mid, high).x +
        e.yyx * mapOrrery(p + e.yyx, t, ringScale, shock, bass, mid, high).x +
        e.yxy * mapOrrery(p + e.yxy, t, ringScale, shock, bass, mid, high).x +
        e.xxx * mapOrrery(p + e.xxx, t, ringScale, shock, bass, mid, high).x
    );
}

vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    float aspect = chroma_Time.w;
    vec2 pRaw = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y);

    // Unpack all 12 live controls + script.js beat drift & shockwave envelope
    float speedRaw    = chroma_Ctrl0.x;
    float audioDrive  = mix(0.25, 1.65, clamp(chroma_Ctrl0.y, 0.0, 1.0));
    float lensing     = mix(0.35, 1.55, clamp(chroma_Ctrl0.z, 0.0, 1.0));
    float ringScale   = clamp(chroma_Ctrl0.w, 0.0, 1.0);

    float mirrorFold  = step(0.55, chroma_Ctrl1.x);
    float dispersion  = mix(0.35, 1.55, clamp(chroma_Ctrl1.y, 0.0, 1.0));
    float glassGlare  = mix(0.10, 1.15, clamp(chroma_Ctrl1.z, 0.0, 1.0));
    float accentPhase = chroma_Ctrl1.w;

    float intensity   = mix(0.55, 1.60, clamp(chroma_Ctrl2.x, 0.0, 1.0));
    float strobeGate  = step(0.55, chroma_Ctrl2.y);
    float bangTrig    = clamp((chroma_Ctrl2.z - 0.5) * 2.0, 0.0, 1.0);
    vec2 tiltCtrl     = (vec2(chroma_Ctrl2.w, chroma_Ctrl3.x) - 0.5) * 2.0;

    vec2 scriptDrift  = (chroma_Ctrl3.yz - 0.5) * 2.0;
    float scriptPulse = clamp((chroma_Ctrl3.w - 0.5) * 2.0, 0.0, 1.0);
    float shock       = clamp(max(bangTrig, scriptPulse * 0.75), 0.0, 1.0);

    // Bipolar speed: negative reverses spin, near-zero yields slow meditative drift
    float speed = (abs(speedRaw) < 0.03 ? 0.08 : speedRaw * 1.85);
    float t = chroma_Time.x * speed;
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]) * audioDrive;
    float mid  = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]) * audioDrive;
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]) * audioDrive;
    float rms  = chroma_AudioRMS.x * audioDrive;

    // Accent tint color derived from accentTint control
    vec3 accentCol = 0.58 + 0.42 * cos(6.28318 * (accentPhase + vec3(0.0, 0.30, 0.64)));

    // Physical optical-plate coordinates at z = 0 (museum dark crown-glass frame)
    vec2 plateHalf = vec2(0.465 * aspect, 0.445);
    vec2 qPlate = abs(pRaw) - plateHalf;
    float plateEdge = length(max(qPlate, vec2(0.0))) + min(max(qPlate.x, qPlate.y), 0.0);
    float insidePlate = smoothstep(0.006, -0.003, plateEdge);
    float bevelOuter = smoothstep(0.012, 0.001, abs(plateEdge)) * smoothstep(0.008, -0.002, plateEdge);
    float bevelInner = smoothstep(0.005, 0.001, abs(plateEdge + 0.018)) * insidePlate;

    // Optional dual-axis kaleidoscopic hologram mirror fold inside the glass plate
    vec2 p = mix(pRaw, abs(pRaw) * vec2(1.0, 1.0) - vec2(0.08 * aspect, 0.06), mirrorFold);

    // Off-axis observer parallax combining user XY pad + beat-synced script.js drift
    vec2 obs = vec2(sin(t * 0.55) * 0.34, cos(t * 0.38) * 0.20)
             + tiltCtrl * 0.52
             + scriptDrift * 0.22;
    vec3 ro = vec3(obs.x, obs.y, 4.35 - 0.18 * shock);
    vec3 rd = normalize(vec3(p.x * 1.42 - obs.x * 0.18, p.y * 1.42 - obs.y * 0.18, -1.78));

    // Seamless deep-cavity Bragg sheen + subtle precision optical coordinate grid
    float braggCoord = p.x * 0.55 + p.y * 0.38 + obs.x * 0.45 + 0.08 * sin(t * 0.35) + accentPhase * 0.6;
    float braggWl = 430.0 + 240.0 * fract(braggCoord * 0.65 * dispersion + 0.35);
    float fineSheen = exp(-pow(p.x * 0.8 + p.y * 0.6 - 0.15 * sin(t * 0.5), 2.0) * 6.0);
    vec3 cavityBg = vec3(0.007, 0.010, 0.018)
                  + mix(spectralRGB(braggWl), accentCol, 0.28) * (0.025 + 0.048 * fineSheen) * insidePlate;

    // Expanding gravitational shockwave ring on bang / beat peak
    float waveR = length(p);
    float shockRing = exp(-abs(waveR - (0.14 + 0.65 * (1.0 - shock))) * 26.0) * shock;
    cavityBg += mix(spectralRGB(480.0 + 180.0 * shock), accentCol, 0.5) * shockRing * 0.65 * insidePlate;

    // Subtle starry background inside the holographic cavity
    vec2 starCell = floor((p + obs * 0.12) * 34.0);
    vec2 starUv = fract((p + obs * 0.12) * 34.0) - 0.5;
    float sh = hash21(starCell);
    float star = smoothstep(0.12, 0.01, length(starUv)) * step(0.955, sh);
    cavityBg += mix(vec3(0.65, 0.88, 1.0), accentCol, 0.35) * star * 0.55 * insidePlate;

    // Curved-spacetime geodesic raymarch through central Kerr singularity + tilted accretion disk + orrery
    vec3 pos = ro;
    vec3 dir = rd;
    vec3 accum = vec3(0.0);
    float transmittance = 1.0;
    float rs = 0.38 + 0.028 * bass + 0.03 * shock;
    bool hitSolid = false;
    vec3 hitPos = vec3(0.0);
    float hitMat = 0.0;

    for (int i = 0; i < 88; i++) {
        float r = length(pos);
        if (r < rs * 0.96) {
            transmittance = 0.0;
            break;
        }
        float bend = (0.18 * lensing * rs * rs) / max(r * r * r, 0.022);
        dir = normalize(dir - pos * bend * 0.09);

        vec2 res = mapOrrery(pos, t, ringScale, shock, bass, mid, high);
        if (res.x < 0.0032) {
            hitSolid = true;
            hitPos = pos;
            hitMat = res.y;
            break;
        }

        // Dramatic tilted relativistic accretion disk
        vec3 dp = pos;
        dp.xy = rot2(dp.xy, 0.24);
        dp.yz = rot2(dp.yz, 0.30);
        float diskR = length(dp.xz);
        float diskY = abs(dp.y);
        float diskBand = exp(-diskY * 22.0) * smoothstep(rs * 1.15, rs * 1.55, diskR) * smoothstep(1.42, 0.56, diskR);
        if (diskBand > 0.002) {
            float ang = atan(dp.z, dp.x) + t * 2.4 + 3.8 / max(diskR, 0.22);
            float filaments = 0.45 + 0.55 * sin(ang * 6.0 + diskR * 15.0) * cos(ang * 3.0 - diskR * 8.0);
            float doppler = clamp(0.5 + 0.55 * (dp.x / max(diskR, 0.01)), 0.0, 1.0);
            vec3 receding = mix(vec3(1.25, 0.28, 0.14), accentCol * 1.35, 0.28);
            vec3 approaching = mix(vec3(0.45, 0.92, 1.45), accentCol * 1.45, 0.38);
            vec3 diskCol = mix(receding, approaching, doppler);
            diskCol = mix(diskCol, vec3(1.45, 1.25, 0.95), smoothstep(0.76, 0.46, diskR));
            float density = diskBand * filaments * (0.28 + 0.22 * rms + 0.18 * bass + 0.22 * shock);
            accum += transmittance * diskCol * density;
            transmittance *= exp(-density * 0.65);
        }

        // Razor-sharp Einstein photon ring right outside the horizon
        float photonRing = exp(-abs(r - rs * 1.22) * 38.0);
        accum += transmittance * mix(vec3(1.25, 0.95, 0.65), accentCol * 1.5, 0.35) * photonRing * (0.075 + 0.06 * shock);

        float stepLen = clamp(min(res.x * 0.85, max(0.028, (r - rs) * 0.20)), 0.022, 0.15);
        pos += dir * stepLen;
        if (length(pos) > 8.2) break;
    }

    vec3 col = cavityBg * transmittance + accum;

    // Shade solid dichroic-chrome orrery geometry if hit
    if (hitSolid) {
        vec3 n = calcNormal(hitPos, t, ringScale, shock, bass, mid, high);
        vec3 v = -dir;
        vec3 l1 = normalize(vec3(0.75 + obs.x * 0.4, 0.85, 0.65));
        vec3 l2 = normalize(vec3(-0.75, -0.35, 0.72));
        float ndv = max(dot(n, v), 0.0);
        float fres = pow(1.0 - ndv, 3.4);

        float wl = 420.0 + 265.0 * fract(ndv * 0.78 * dispersion + hitMat * 0.24 + obs.x * 0.45 + t * 0.06 + accentPhase * 0.5);
        vec3 bragg = mix(spectralRGB(wl), accentCol, 0.24);
        vec3 baseMetal = mix(vec3(0.05, 0.07, 0.11), vec3(0.95, 0.78, 0.42), step(3.5, hitMat) * 0.55 + 0.22);

        float diff1 = max(dot(n, l1), 0.0);
        float spec1 = pow(max(dot(reflect(-l1, n), v), 0.0), 56.0);
        float spec2 = pow(max(dot(reflect(-l2, n), v), 0.0), 32.0);

        vec3 surf = baseMetal * (0.20 + 0.80 * diff1)
                  + bragg * (0.45 + 1.55 * fres + 0.45 * shock)
                  + vec3(1.3, 1.22, 1.1) * spec1 * 1.45
                  + mix(vec3(0.35, 0.95, 1.25), accentCol, 0.45) * spec2 * 0.75;

        col = accum + surf * transmittance;
    }

    // Subtle coherent laser speckle grain inside the hologram plate
    float speckle = (hash21(floor(uv * chroma_Resolution.xy) + floor(t * 18.0) * 0.01) - 0.5);
    col *= 1.0 + speckle * (0.045 + 0.03 * glassGlare) * insidePlate;

    // Beat-synced strobe gate when armed
    float beatStrobe = mix(1.0, 0.38 + 1.35 * pow(clamp(chroma_BPM.z, 0.0, 1.0), 1.6), strobeGate);
    col *= beatStrobe;

    // Front z=0 optical crown-glass plate: diagonal gallery spotlight glare + crisp dual-bevel frame
    float glareBand = exp(-pow((pRaw.x * 0.62 - pRaw.y * 0.88 - 0.14 * sin(t * 0.35)), 2.0) * 34.0);
    vec3 frontGlass = mix(vec3(0.72, 0.88, 1.0), accentCol, 0.25) * glareBand * 0.14 * glassGlare * insidePlate;

    float bezelWl = 420.0 + 250.0 * fract(pRaw.x * 0.6 + pRaw.y * 0.5 + obs.x * 0.5 + t * 0.08 + accentPhase * 0.4);
    vec3 bezelCol = (vec3(0.28, 0.36, 0.46) + spectralRGB(bezelWl) * 1.15) * bevelOuter * (0.65 + 0.4 * glassGlare)
                  + mix(vec3(0.45, 0.72, 0.85), accentCol, 0.4) * bevelInner * (0.35 + 0.35 * shock);

    vec3 frameBg = vec3(0.009, 0.011, 0.016);
    col = mix(frameBg, col + frontGlass, insidePlate) + bezelCol;

    col *= intensity * (1.0 + 0.22 * shock);
    col = aces(col);
    col = pow(col, vec3(1.0 / 2.2));
    fragColor = vec4(col, 1.0);
}
