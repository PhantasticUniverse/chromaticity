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
    vec4 chroma_Ctrl0;       // .x=speed, .y=audioDrive, .z=warp, .w=ribbonMix
    vec4 chroma_Ctrl1;       // .x=mirrorAxis, .y=quantumGlow, .z=chromeSheen, .w=accentTint
    vec4 chroma_Ctrl2;       // .x=intensity, .y=invertSolar, .z=quantumSurge, .w=focusX
    vec4 chroma_Ctrl3;       // .x=focusY, .y=scriptDriftX, .z=scriptDriftY, .w=scriptPulseEnv
};

vec2 rot2(vec2 v, float a) {
    float c = cos(a);
    float s = sin(a);
    return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}

float smin(float a, float b, float k) {
    float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

vec3 oilSlick(float x) {
    return 0.52 + 0.48 * cos(6.28318 * (x + vec3(0.0, 0.31, 0.65)));
}

float gyroid(vec3 p) {
    return dot(sin(p), cos(p.yzx));
}

// 3D quantum-orbital standing-wave filaments |psi1 + psi2 + psi3|^2
float quantumField(vec3 p, float t, float warp) {
    vec3 q = p;
    q.xz = rot2(q.xz, t * 0.35 + length(p) * 0.48 * warp);
    q.xy = rot2(q.xy, -t * 0.25);
    float psi1 = sin(q.x * 3.4 + t * 1.4) * cos(q.y * 3.4 - t * 1.1);
    float psi2 = sin(q.y * 3.6 - t * 1.2) * cos(q.z * 3.6 + t * 1.5);
    float psi3 = gyroid(q * (2.4 + 1.1 * warp) - vec3(0.0, t * 0.8, 0.0)) * 0.65;
    float prob = abs(psi1 + psi2 + psi3) * 0.36;
    float filament = pow(clamp(prob, 0.0, 1.0), 3.2) * exp(-length(p) * 0.48);
    return filament;
}

// Gyroid-carved liquid-chrome & black-opal core + orbiting mercury trefoil ribbons
float mapSculpture(vec3 p, float t, float warp, float ribbonMix, float surge, float bass, float mid, float high) {
    vec3 q = p;
    q.xz = rot2(q.xz, t * 0.42);
    q.xy = rot2(q.xy, t * 0.26);

    float r = length(q);
    vec3 dir = q / max(r, 0.0001);
    float h1 = sin(dir.x * 6.0 + t * 2.2) * cos(dir.y * 6.0 - t * 1.8) * (0.14 + 0.14 * bass + 0.06 * surge);
    float h2 = sin(dir.z * 9.0 - t * 2.6) * sin(dir.x * 7.0 + dir.y * 7.0) * (0.08 + 0.10 * mid);
    float g = gyroid(q * (2.8 + 1.8 * warp) + vec3(t * 0.7, -t * 0.5, t * 0.4)) * (0.14 + 0.10 * high);
    float core = r - (0.76 + h1 + h2 + g * 0.55);

    vec3 tr = q;
    float a = atan(tr.z, tr.x);
    float ribRad = mix(1.08, 1.52, ribbonMix) + 0.12 * sin(a * 3.0 + t * 1.8);
    vec2 ring = vec2(length(tr.xz) - ribRad, tr.y - 0.28 * cos(a * 3.0 + t * 1.8));
    ring = rot2(ring, a * 2.5 + t * 1.5);
    float ribThick = mix(0.06, 0.16, ribbonMix) + 0.03 * bass + 0.03 * surge;
    float ribbon = length(ring * vec2(1.0, 2.2)) - ribThick;

    return smin(core, ribbon, mix(0.10, 0.34, ribbonMix)) * 0.68;
}

vec3 calcNormal(vec3 p, float t, float warp, float ribbonMix, float surge, float bass, float mid, float high) {
    vec2 e = vec2(0.0022, -0.0022);
    return normalize(
        e.xyy * mapSculpture(p + e.xyy, t, warp, ribbonMix, surge, bass, mid, high) +
        e.yyx * mapSculpture(p + e.yyx, t, warp, ribbonMix, surge, bass, mid, high) +
        e.yxy * mapSculpture(p + e.yxy, t, warp, ribbonMix, surge, bass, mid, high) +
        e.xxx * mapSculpture(p + e.xxx, t, warp, ribbonMix, surge, bass, mid, high)
    );
}

// High-contrast obsidian studio chrome environment (dark base + razor-sharp neon softbox strips)
vec3 studioChromeEnv(vec3 rd, float t, float high, float accentPhase) {
    vec3 env = vec3(0.006, 0.008, 0.022);
    vec3 s1 = normalize(vec3(cos(t * 0.7), 0.72, sin(t * 0.7)));
    vec3 s2 = normalize(vec3(-cos(t * 0.55), -0.42, sin(t * 0.55)));
    vec3 s3 = normalize(vec3(sin(t * 0.45), 0.25, -cos(t * 0.45)));

    float strip1 = smoothstep(0.92, 0.996, abs(dot(rd, s1)));
    float strip2 = smoothstep(0.93, 0.996, abs(dot(rd, s2)));
    float strip3 = smoothstep(0.94, 0.997, abs(dot(rd, s3)));

    vec3 c1 = mix(vec3(0.28, 1.18, 1.55), oilSlick(accentPhase) * 1.55, 0.45);
    vec3 c2 = mix(vec3(1.48, 0.28, 1.05), oilSlick(accentPhase + 0.38) * 1.50, 0.45);
    vec3 c3 = mix(vec3(1.52, 0.98, 0.28), oilSlick(accentPhase + 0.72) * 1.50, 0.35);

    env += c1 * strip1 * 2.8;
    env += c2 * strip2 * 2.4;
    env += c3 * strip3 * (2.1 + 0.6 * high);
    env += oilSlick(rd.y * 0.8 + atan(rd.z, rd.x) * 0.25 + t * 0.08 + accentPhase) * 0.09;
    return env;
}

vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    float aspect = chroma_Time.w;
    vec2 p = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y);

    // Unpack all 12 live controls + script.js beat drift & quantum surge envelope
    float speedRaw    = chroma_Ctrl0.x;
    float audioDrive  = mix(0.25, 1.65, clamp(chroma_Ctrl0.y, 0.0, 1.0));
    float warp        = mix(0.22, 1.42, clamp(chroma_Ctrl0.z, 0.0, 1.0));
    float ribbonMix   = clamp(chroma_Ctrl0.w, 0.0, 1.0);

    float mirrorAxis  = step(0.55, chroma_Ctrl1.x);
    float quantumGlow = mix(0.18, 1.55, clamp(chroma_Ctrl1.y, 0.0, 1.0));
    float chromeSheen = mix(0.30, 1.38, clamp(chroma_Ctrl1.z, 0.0, 1.0));
    float accentPhase = chroma_Ctrl1.w;

    float intensity   = mix(0.55, 1.58, clamp(chroma_Ctrl2.x, 0.0, 1.0));
    float invertSolar = step(0.55, chroma_Ctrl2.y);
    float bangTrig    = clamp((chroma_Ctrl2.z - 0.5) * 2.0, 0.0, 1.0);
    vec2 focus        = (vec2(chroma_Ctrl2.w, chroma_Ctrl3.x) - 0.5) * 2.0;

    vec2 scriptDrift  = (chroma_Ctrl3.yz - 0.5) * 2.0;
    float scriptPulse = clamp((chroma_Ctrl3.w - 0.5) * 2.0, 0.0, 1.0);
    float surge       = clamp(max(bangTrig, scriptPulse * 0.75), 0.0, 1.0);

    // Optional quad-mirror Rorschach chrome symmetry
    p = mix(p, abs(p) - vec2(0.06 * aspect, 0.05), mirrorAxis);

    float speed = (abs(speedRaw) < 0.03 ? 0.08 : speedRaw * 1.85);
    float t = chroma_Time.x * speed;
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]) * audioDrive;
    float mid  = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]) * audioDrive;
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]) * audioDrive;
    float rms  = chroma_AudioRMS.x * audioDrive;

    float yaw = t * 0.28 + focus.x * 1.5 + scriptDrift.x * 0.35;
    float pitch = clamp(0.18 + focus.y * 0.65 + scriptDrift.y * 0.18, -0.75, 0.85);
    float camR = 3.55 - 0.20 * bass - 0.18 * surge;
    vec3 ro = vec3(sin(yaw) * cos(pitch) * camR, sin(pitch) * camR, cos(yaw) * cos(pitch) * camR);
    vec3 fw = normalize(-ro);
    vec3 rt = normalize(cross(fw, vec3(0.0, 1.0, 0.0)));
    vec3 up = cross(rt, fw);
    vec3 rd = normalize(fw * 1.42 + rt * p.x + up * p.y);

    float dist = 0.0;
    vec3 volCol = vec3(0.0);
    float trans = 1.0;
    bool hit = false;

    for (int i = 0; i < 78; i++) {
        vec3 pos = ro + rd * dist;
        float d = mapSculpture(pos, t, warp, ribbonMix, surge, bass, mid, high);

        float qd = quantumField(pos, t, warp);
        if (qd > 0.003) {
            vec3 qTint = oilSlick(length(pos) * 0.42 - t * 0.14 + qd * 2.4 + accentPhase);
            float dens = qd * 0.22 * quantumGlow * (0.75 + 0.65 * rms + 0.5 * mid + 0.75 * surge);
            volCol += trans * qTint * dens * 1.75;
            trans *= exp(-dens * 0.42);
        }

        if (d < 0.0025) {
            hit = true;
            break;
        }
        dist += clamp(d * 0.82, 0.018, 0.16);
        if (dist > 7.8) break;
    }

    vec3 bg = vec3(0.006, 0.008, 0.020)
            + oilSlick(rd.y * 0.65 + t * 0.05 + accentPhase) * 0.03 * exp(-abs(rd.y) * 3.0);
    vec3 col = bg * trans + volCol;

    if (hit) {
        vec3 pos = ro + rd * dist;
        vec3 n = calcNormal(pos, t, warp, ribbonMix, surge, bass, mid, high);
        vec3 v = -rd;
        float ndv = max(dot(n, v), 0.0);
        float fres = pow(1.0 - ndv, 3.4);

        vec3 reflDir = reflect(rd, n);
        vec3 refl = studioChromeEnv(reflDir, t, high, accentPhase);

        float film = ndv * 1.4 + gyroid(pos * 2.4 - vec3(0.0, t * 0.6, 0.0)) * 0.28 + t * 0.08 + accentPhase;
        vec3 irid = oilSlick(film);

        vec3 l1 = normalize(vec3(0.65, 0.85, 0.55));
        float spec = pow(max(dot(reflect(-l1, n), v), 0.0), 88.0);

        vec3 surf = vec3(0.018, 0.022, 0.042) * (0.35 + 0.65 * ndv)
                  + refl * (0.42 + 0.95 * chromeSheen)
                  + irid * (0.14 + (1.55 + 0.65 * surge) * fres)
                  + vec3(1.5, 1.42, 1.3) * spec * 1.75;

        col = volCol + surf * trans;
    }

    col *= intensity * (1.0 + 0.20 * surge);
    col = aces(col);
    col = mix(col, vec3(1.0) - col * 0.92, invertSolar);
    col = pow(clamp(col, 0.0, 1.0), vec3(1.0 / 2.2));
    fragColor = vec4(col, 1.0);
}
