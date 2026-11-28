// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform Uniforms {
    vec4 chroma_Time;        // .x = elapsed seconds, .y = dt, .z = frame, .w = aspect
    vec4 chroma_Resolution;  // .x = width, .y = height, .z = 1/w, .w = 1/h
    vec4 syn_BPM;            // .x = BPM, .y = beat_time, .z = on_beat, .w = bpm_sin
    vec4 chroma_AudioRMS;    // .x = RMS [0..1]
    vec4 chroma_AudioPeak;   // .x = Peak [0..1]
    float chroma_AudioFFT[8];// 8 log-spaced FFT bands [0..1]
    vec4 chroma_Ctrl0;       // .x=speed, .y=audioDrive, .z=bloomFold, .w=poolRipple
    vec4 chroma_Ctrl1;       // .x=kaleidoMirror, .y=dispersion, .z=causticGlow, .w=accentTint
    vec4 chroma_Ctrl2;       // .x=intensity, .y=invertFlash, .z=energyFlash, .w=orbitX
    vec4 chroma_Ctrl3;       // .x=orbitY, .y=scriptDriftX, .z=scriptDriftY, .w=scriptPulseEnv
};

const float PI = 3.14159265359;

vec2 rot2(vec2 v, float a) {
    float c = cos(a);
    float s = sin(a);
    return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

float smin(float a, float b, float k) {
    float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

vec3 thinFilm(float x) {
    return 0.52 + 0.48 * cos(6.28318 * (x + vec3(0.0, 0.30, 0.64)));
}

vec3 polarFold(vec3 p, float n) {
    float an = 2.0 * PI / n;
    float a = atan(p.z, p.x) + an * 0.5;
    float sector = mod(a, an) - an * 0.5;
    float r = length(p.xz);
    return vec3(cos(sector) * r, p.y, sin(sector) * r);
}

float sdPetal(vec3 p, float openAmt) {
    vec3 q = p;
    q.xy = rot2(q.xy, -openAmt);
    q.y -= 0.22 * q.x * q.x;
    vec3 c = q - vec3(0.64, 0.0, 0.0);
    float taper = clamp(1.0 - abs(c.x) * 1.48, 0.07, 1.0);
    vec3 e = vec3(0.54, 0.042 * taper + 0.014, 0.21 * taper);
    vec3 k = c / e;
    float d = (length(k) - 1.0) * min(min(e.x, e.y), e.z);
    d -= 0.012 * exp(-abs(c.z) * 18.0) * taper;
    return d * 0.72;
}

vec2 mapLotus(vec3 p, float t, float bloomFold, float flare, float bass, float mid) {
    vec3 q = p - vec3(0.0, 0.14 + 0.06 * sin(t * 1.1) + 0.05 * flare, 0.0);
    q.xz = rot2(q.xz, t * 0.26);

    vec3 p1 = polarFold(q, 8.0);
    float open1 = 0.22 + 0.32 * bloomFold + 0.08 * bass + 0.12 * flare;
    float dOuter = sdPetal(p1, open1);

    vec3 q2 = q - vec3(0.0, 0.12, 0.0);
    q2.xz = rot2(q2.xz, PI / 8.0 - t * 0.14);
    vec3 p2 = polarFold(q2 * 1.28, 8.0);
    float open2 = 0.54 + 0.28 * bloomFold + 0.08 * mid + 0.09 * flare;
    float dMid = sdPetal(p2, open2) / 1.28;

    vec3 q3 = q - vec3(0.0, 0.22, 0.0);
    q3.xz = rot2(q3.xz, t * 0.20);
    vec3 p3 = polarFold(q3 * 1.72, 6.0);
    float dInner = sdPetal(p3, 0.86 + 0.18 * bloomFold + 0.06 * flare) / 1.72;

    float dLotus = smin(dOuter, smin(dMid, dInner, 0.04), 0.05);

    vec3 qc = q - vec3(0.0, 0.42 + 0.04 * sin(t * 2.4), 0.0);
    qc.xy = rot2(qc.xy, t * 1.3);
    qc.yz = rot2(qc.yz, t * 0.9);
    vec3 ac = abs(qc);
    float dCore = (ac.x + ac.y + ac.z - (0.21 + 0.045 * bass + 0.05 * flare)) * 0.57735;

    if (dCore < dLotus) return vec2(dCore, 2.0);
    return vec2(dLotus, 1.0);
}

vec3 lotusNormal(vec3 p, float t, float bloomFold, float flare, float bass, float mid) {
    vec2 e = vec2(0.0022, -0.0022);
    return normalize(
        e.xyy * mapLotus(p + e.xyy, t, bloomFold, flare, bass, mid).x +
        e.yyx * mapLotus(p + e.yyx, t, bloomFold, flare, bass, mid).x +
        e.yxy * mapLotus(p + e.yxy, t, bloomFold, flare, bass, mid).x +
        e.xxx * mapLotus(p + e.xxx, t, bloomFold, flare, bass, mid).x
    );
}

// Deep obsidian cosmic studio environment (high contrast so crystal facets sparkle!)
vec3 sampleEnv(vec3 rd, float t, float high, float accentPhase) {
    float y = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    vec3 bg = mix(vec3(0.006, 0.010, 0.024), vec3(0.022, 0.012, 0.048), y);

    float arc1 = smoothstep(0.93, 0.995, dot(rd, normalize(vec3(0.55, 0.65, 0.52))));
    float arc2 = smoothstep(0.92, 0.994, dot(rd, normalize(vec3(-0.72, 0.45, 0.52))));
    float horizon = exp(-abs(rd.y) * 14.0) * (0.4 + 0.6 * sin(atan(rd.z, rd.x) * 4.0 + t * 0.6));

    vec3 tintA = mix(vec3(0.35, 1.15, 1.45), thinFilm(accentPhase) * 1.5, 0.45);
    vec3 tintB = mix(vec3(1.35, 0.35, 0.95), thinFilm(accentPhase + 0.42) * 1.45, 0.45);

    bg += tintA * arc1 * 2.4;
    bg += tintB * arc2 * 2.0;
    bg += thinFilm(atan(rd.z, rd.x) * 0.32 + t * 0.08 + accentPhase) * horizon * (0.28 + 0.25 * high);
    return bg;
}

vec3 traceLotus(
    vec3 ro, vec3 rd, float t, float bloomFold, float dispersion,
    float causticGlow, float accentPhase, float flare, float bass, float mid, float high
) {
    float dist = 0.0;
    float glow = 0.0;
    float mat = 0.0;
    bool hit = false;

    for (int i = 0; i < 74; i++) {
        vec3 p = ro + rd * dist;
        vec2 res = mapLotus(p, t, bloomFold, flare, bass, mid);
        float coreDist = length(p - vec3(0.0, 0.56, 0.0));
        glow += exp(-coreDist * 4.2) * (0.032 + 0.028 * flare) + exp(-max(res.x, 0.0) * 16.0) * 0.010;
        if (res.x < 0.0025) {
            hit = true;
            mat = res.y;
            break;
        }
        dist += clamp(res.x * 0.85, 0.014, 0.24);
        if (dist > 10.5) break;
    }

    vec3 col = sampleEnv(rd, t, high, accentPhase);

    if (hit) {
        vec3 p = ro + rd * dist;
        vec3 n = lotusNormal(p, t, bloomFold, flare, bass, mid);
        vec3 v = -rd;
        float ndv = max(dot(n, v), 0.0);
        float fres = pow(1.0 - ndv, 3.2);

        if (mat > 1.5) {
            vec3 heart = mix(thinFilm(accentPhase) * 1.4, vec3(1.5, 0.92, 0.38), fres);
            col = heart * (1.7 + 0.8 * bass + 0.9 * flare) + vec3(1.4) * pow(ndv, 5.0);
        } else {
            // Snell's-law RGB dispersive crystal refraction
            float disp = 0.075 * dispersion;
            vec3 rR = refract(rd, n, 1.0 / (1.46 - disp));
            vec3 rG = refract(rd, n, 1.0 / 1.52);
            vec3 rB = refract(rd, n, 1.0 / (1.58 + disp));

            vec3 refrCol = vec3(
                sampleEnv(rR, t, high, accentPhase).r,
                sampleEnv(rG, t, high, accentPhase).g,
                sampleEnv(rB, t, high, accentPhase).b
            );
            float thickness = clamp(1.0 - ndv * 0.72, 0.18, 1.0);
            vec3 beer = exp(-vec3(0.85, 0.28, 0.12) * thickness * 1.6);
            vec3 reflCol = sampleEnv(reflect(rd, n), t, high, accentPhase);
            vec3 irid = thinFilm(ndv * 1.25 + length(p.xz) * 0.45 - t * 0.12 + accentPhase);

            vec3 l1 = normalize(vec3(0.55, 0.88, 0.55));
            float spec = pow(max(dot(reflect(-l1, n), v), 0.0), 72.0);

            col = mix(refrCol * beer * 1.6 + irid * 0.24, reflCol * 1.45 + irid * (0.82 + 0.4 * flare), 0.20 + 0.78 * fres)
                + vec3(1.4, 1.28, 1.1) * spec * 1.65;
        }
    }

    vec3 haloCol = mix(thinFilm(accentPhase + 0.15) * 1.35, vec3(1.35, 0.82, 0.32), 0.5 + 0.5 * sin(t * 0.7));
    col += haloCol * glow * causticGlow * (0.45 + 0.38 * bass + 0.55 * flare);
    return col;
}

vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    float aspect = chroma_Time.w;
    vec2 p = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y);

    // Unpack all 12 live controls + script.js beat drift & flare envelope
    float speedRaw      = chroma_Ctrl0.x;
    float audioDrive    = mix(0.25, 1.65, clamp(chroma_Ctrl0.y, 0.0, 1.0));
    float bloomFold     = clamp(chroma_Ctrl0.z, 0.0, 1.0);
    float poolRipple    = mix(0.08, 1.15, clamp(chroma_Ctrl0.w, 0.0, 1.0));

    float kaleidoMirror = step(0.55, chroma_Ctrl1.x);
    float dispersion    = mix(0.35, 1.55, clamp(chroma_Ctrl1.y, 0.0, 1.0));
    float causticGlow   = mix(0.25, 1.65, clamp(chroma_Ctrl1.z, 0.0, 1.0));
    float accentPhase   = chroma_Ctrl1.w;

    float intensity     = mix(0.55, 1.55, clamp(chroma_Ctrl2.x, 0.0, 1.0));
    float invertFlash   = step(0.55, chroma_Ctrl2.y);
    float bangTrig      = clamp((chroma_Ctrl2.z - 0.5) * 2.0, 0.0, 1.0);
    vec2 orbit          = (vec2(chroma_Ctrl2.w, chroma_Ctrl3.x) - 0.5) * 2.0;

    vec2 scriptDrift    = (chroma_Ctrl3.yz - 0.5) * 2.0;
    float scriptPulse   = clamp((chroma_Ctrl3.w - 0.5) * 2.0, 0.0, 1.0);
    float flare         = clamp(max(bangTrig, scriptPulse * 0.72), 0.0, 1.0);

    // Optional sacred vertical-axis mirror fold
    p.x = mix(p.x, abs(p.x), kaleidoMirror);

    float speed = (abs(speedRaw) < 0.03 ? 0.08 : speedRaw * 1.8);
    float t = chroma_Time.x * speed;
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]) * audioDrive;
    float mid  = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]) * audioDrive;
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]) * audioDrive;

    // Framed hero camera showing the entire lotus + its mercury mirror reflection
    float camYaw = t * 0.25 + orbit.x * 1.6 + scriptDrift.x * 0.35;
    float camElev = clamp(0.32 + orbit.y * 0.38 + scriptDrift.y * 0.14, 0.12, 1.05);
    float camR = 3.95 - 0.22 * flare;
    vec3 ro = vec3(sin(camYaw) * cos(camElev) * camR, sin(camElev) * camR + 0.12, cos(camYaw) * cos(camElev) * camR);
    vec3 ta = vec3(0.0, 0.32, 0.0);
    vec3 fw = normalize(ta - ro);
    vec3 rt = normalize(cross(fw, vec3(0.0, 1.0, 0.0)));
    vec3 up = cross(rt, fw);
    vec3 rd = normalize(fw * 1.45 + rt * p.x + up * p.y);

    float poolY = -0.34;
    float tPool = (poolY - ro.y) / rd.y;
    bool hitPool = (rd.y < -0.001) && (tPool > 0.0) && (tPool < 18.0);

    vec3 col;
    if (hitPool) {
        vec3 fp = ro + rd * tPool;
        float rPool = length(fp.xz);
        float dw = cos(rPool * 10.0 - t * 3.2) * 10.0 * exp(-rPool * 0.42) * 0.005 * poolRipple;
        vec2 grad = (fp.xz / max(rPool, 0.001)) * dw;
        vec3 nPool = normalize(vec3(-grad.x, 1.0, -grad.y));

        vec3 reflDir = reflect(rd, nPool);
        vec3 reflCol = traceLotus(fp + nPool * 0.02, reflDir, t, bloomFold, dispersion, causticGlow, accentPhase, flare, bass, mid, high);

        float caustic = pow(max(0.0, sin(rPool * 8.0 - t * 2.4 + atan(fp.z, fp.x) * 4.0)), 6.0) * exp(-rPool * 0.48);
        float flareRing = exp(-abs(rPool - (0.6 + 2.8 * (1.0 - flare))) * 6.5) * flare;
        vec3 causticCol = thinFilm(rPool * 0.32 - t * 0.14 + accentPhase)
                        * (caustic * (0.48 + 0.55 * bass) + flareRing * 1.4) * causticGlow;

        float fresPool = 0.30 + 0.70 * pow(1.0 - max(dot(nPool, -rd), 0.0), 3.2);
        float fog = exp(-tPool * 0.075);
        col = mix(vec3(0.006, 0.009, 0.018), reflCol * (0.62 + 0.38 * fresPool) + causticCol, fog);

        // Foreground lotus occlusion check
        float dCheck = 0.0;
        bool inFront = false;
        for (int i = 0; i < 44; i++) {
            vec3 cp = ro + rd * dCheck;
            float dL = mapLotus(cp, t, bloomFold, flare, bass, mid).x;
            if (dL < 0.005) { inFront = (dCheck < tPool); break; }
            dCheck += max(dL * 0.9, 0.02);
            if (dCheck >= tPool) break;
        }
        if (inFront) {
            col = traceLotus(ro, rd, t, bloomFold, dispersion, causticGlow, accentPhase, flare, bass, mid, high);
        }
    } else {
        col = traceLotus(ro, rd, t, bloomFold, dispersion, causticGlow, accentPhase, flare, bass, mid, high);
    }

    col *= intensity * (1.0 + 0.18 * flare);
    col = aces(col);
    col = mix(col, vec3(1.0) - col * 0.92, invertFlash);
    col = pow(clamp(col, 0.0, 1.0), vec3(1.0 / 2.2));
    fragColor = vec4(col, 1.0);
}
