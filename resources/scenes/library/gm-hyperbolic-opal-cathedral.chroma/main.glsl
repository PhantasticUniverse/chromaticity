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
    vec4 chroma_Ctrl0;       // .x=speed, .y=audioDrive, .z=foldMorph, .w=ribScale
    vec4 chroma_Ctrl1;       // .x=mirrorNave, .y=opalSheen, .z=godRays, .w=accentTint
    vec4 chroma_Ctrl2;       // .x=intensity, .y=strobeGate, .z=vaultPulse, .w=lookX
    vec4 chroma_Ctrl3;       // .x=lookY, .y=scriptDriftX, .z=scriptDriftY, .w=scriptPulseEnv
};

vec3 opalPalette(float x) {
    return 0.52 + 0.48 * cos(6.28318 * (x + vec3(0.02, 0.33, 0.67)));
}

float gyroid(vec3 p) {
    return dot(sin(p), cos(p.yzx));
}

// Solid Gothic-Menger vaulted cathedral with carved arches, columns, and narrow liquid-opal canal
vec2 mapCathedral(vec3 p, float t, float foldMorph, float ribScale, float bass) {
    vec3 q = p;
    q.x -= sin(q.z * 0.24) * 0.35;

    // Periodic cathedral bays along Z (spacing = 2.4)
    vec3 bay = q;
    bay.z = mod(bay.z + 1.2, 2.4) - 1.2;

    // Twin rows of fluted obsidian-gold nave columns at |x| = 1.22
    vec2 colP = vec2(abs(bay.x) - 1.22, bay.z);
    float colAng = atan(colP.y, colP.x);
    float colRad = mix(0.18, 0.31, ribScale);
    float columns = length(colP) - (colRad + 0.022 * sin(colAng * 8.0));

    // Pointed gothic overhead vault arches spanning the upper nave (open below y = 0.25)
    vec3 archP = vec3(abs(bay.x) - 0.42, bay.y - 0.25, bay.z);
    float archRing = abs(length(archP.xy) - 1.18) - mix(0.10, 0.17, ribScale);
    float archRib = max(max(archRing, abs(bay.z) - 0.18), -(bay.y - 0.25));

    // Outer cathedral walls + vaulted ceiling shell
    float sideWall = 1.65 - abs(q.x);
    float vaultCeil = 1.55 - (q.y + 0.22 * q.x * q.x);
    float shell = min(sideWall, vaultCeil);

    // Fractal Gothic tracery ribs carved into walls & arches (4-iter box fold)
    vec3 f = bay;
    float s = 1.0;
    float trap = 1e5;
    float kMorph = mix(1.42, 1.88, foldMorph);
    for (int i = 0; i < 4; i++) {
        f = abs(f) - vec3(0.42, 0.52, 0.42);
        if (f.x < f.y) f.xy = f.yx;
        if (f.y < f.z) f.yz = f.zy;
        f *= kMorph;
        s *= kMorph;
        trap = min(trap, length(f.xy));
    }
    float tracery = (length(max(abs(f) - vec3(0.45, 0.35, 0.45), vec3(0.0))) - 0.06) / s;

    // Combine bold architecture + fractal filigree + liquid-opal ripple relief
    float arch = min(columns, archRib);
    float structure = min(arch, max(shell, -tracery * 0.45));
    float opalRipple = mix(0.005, 0.018, ribScale) * gyroid(p * 3.4 + vec3(0.0, t * 0.7, -t * 0.9));
    float d = structure + opalRipple;

    // Recessed glossy obsidian-opal floor canal at y = -0.92
    float floorD = q.y + 0.92 + 0.008 * sin(q.x * 6.0 + t * 1.4) * cos(q.z * 4.0 - t * 1.6);
    if (floorD < d) {
        return vec2(floorD, -1.0);
    }
    float matTrap = min(trap, arch * 3.0 + 0.25 - 0.08 * bass);
    return vec2(d, max(matTrap, 0.01));
}

vec3 calcNormal(vec3 p, float t, float foldMorph, float ribScale, float bass) {
    vec2 e = vec2(0.0024, -0.0024);
    return normalize(
        e.xyy * mapCathedral(p + e.xyy, t, foldMorph, ribScale, bass).x +
        e.yyx * mapCathedral(p + e.yyx, t, foldMorph, ribScale, bass).x +
        e.yxy * mapCathedral(p + e.yxy, t, foldMorph, ribScale, bass).x +
        e.xxx * mapCathedral(p + e.xxx, t, foldMorph, ribScale, bass).x
    );
}

float calcAO(vec3 p, vec3 n, float t, float foldMorph, float ribScale, float bass) {
    float occ = 0.0;
    float sca = 1.0;
    for (int i = 1; i <= 4; i++) {
        float h = 0.055 * float(i);
        float d = mapCathedral(p + n * h, t, foldMorph, ribScale, bass).x;
        occ += (h - d) * sca;
        sca *= 0.65;
    }
    return clamp(1.0 - 2.2 * occ, 0.14, 1.0);
}

vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
    float aspect = chroma_Time.w;
    vec2 p = vec2((uv.x - 0.5) * aspect, 0.5 - uv.y);

    // Unpack all 12 live controls + script.js beat drift & vault pulse envelope
    float speedRaw    = chroma_Ctrl0.x;
    float audioDrive  = mix(0.25, 1.65, clamp(chroma_Ctrl0.y, 0.0, 1.0));
    float foldMorph   = clamp(chroma_Ctrl0.z, 0.0, 1.0);
    float ribScale    = clamp(chroma_Ctrl0.w, 0.0, 1.0);

    float mirrorNave  = step(0.55, chroma_Ctrl1.x);
    float opalSheen   = mix(0.25, 1.45, clamp(chroma_Ctrl1.y, 0.0, 1.0));
    float godRays     = mix(0.15, 1.45, clamp(chroma_Ctrl1.z, 0.0, 1.0));
    float accentPhase = chroma_Ctrl1.w;

    float intensity   = mix(0.55, 1.50, clamp(chroma_Ctrl2.x, 0.0, 1.0));
    float strobeGate  = step(0.55, chroma_Ctrl2.y);
    float bangTrig    = clamp((chroma_Ctrl2.z - 0.5) * 2.0, 0.0, 1.0);
    vec2 look         = (vec2(chroma_Ctrl2.w, chroma_Ctrl3.x) - 0.5) * 2.0;

    vec2 scriptDrift  = (chroma_Ctrl3.yz - 0.5) * 2.0;
    float scriptPulse = clamp((chroma_Ctrl3.w - 0.5) * 2.0, 0.0, 1.0);
    float surge       = clamp(max(bangTrig, scriptPulse * 0.72), 0.0, 1.0);

    // Optional Escher dual-vault vertical mirror
    p.y = mix(p.y, abs(p.y) - 0.08, mirrorNave);

    float speed = (abs(speedRaw) < 0.03 ? 0.08 : speedRaw * 1.85);
    float t = chroma_Time.x * speed;
    float bass = 0.5 * (chroma_AudioFFT[0] + chroma_AudioFFT[1]) * audioDrive;
    float mid  = 0.5 * (chroma_AudioFFT[3] + chroma_AudioFFT[4]) * audioDrive;
    float high = 0.5 * (chroma_AudioFFT[6] + chroma_AudioFFT[7]) * audioDrive;

    float zPos = t * 1.25;
    vec3 ro = vec3(sin(zPos * 0.24) * 0.30, -0.18 + scriptDrift.y * 0.08, zPos);
    vec3 ta = vec3(
        sin((zPos + 2.2) * 0.24) * 0.30 + look.x * 0.65 + scriptDrift.x * 0.25,
        0.12 + look.y * 0.55 + scriptDrift.y * 0.16,
        zPos + 2.2
    );
    vec3 fw = normalize(ta - ro);
    vec3 rt = normalize(cross(fw, vec3(0.0, 1.0, 0.0)));
    vec3 up = cross(rt, fw);
    vec3 rd = normalize(fw * 1.32 + rt * p.x + up * p.y);

    float dist = 0.0;
    float hitTrap = 0.0;
    bool hit = false;
    vec3 volShafts = vec3(0.0);

    for (int i = 0; i < 86; i++) {
        vec3 pos = ro + rd * dist;
        vec2 res = mapCathedral(pos, t, foldMorph, ribScale, bass);

        // Prismatic stained-glass clerestory light shafts descending diagonally
        float bayPulse = pow(sin(pos.z * 1.309) * 0.5 + 0.5, 4.0);
        float cx = pos.x - sin(pos.z * 0.24) * 0.35;
        float shaft = exp(-abs(cx - 0.38 * pos.y) * 4.2) * smoothstep(-0.8, 1.2, pos.y) * bayPulse;
        vec3 rayCol = opalPalette(pos.z * 0.12 + t * 0.06 + 0.15 + accentPhase);
        volShafts += rayCol * shaft * 0.024 * godRays * (0.75 + 0.55 * mid + 0.65 * surge);

        if (res.x < 0.0024) {
            hit = true;
            hitTrap = res.y;
            break;
        }
        dist += clamp(res.x * 0.80, 0.015, 0.26);
        if (dist > 18.0) break;
    }

    vec3 fogCol = vec3(0.010, 0.014, 0.032) + opalPalette(t * 0.05 + 0.4 + accentPhase) * 0.035;
    vec3 col = fogCol;

    if (hit) {
        vec3 pos = ro + rd * dist;
        vec3 n = calcNormal(pos, t, foldMorph, ribScale, bass);
        vec3 v = -rd;
        float ndv = max(dot(n, v), 0.0);
        float fres = pow(1.0 - ndv, 3.2);
        float ao = calcAO(pos, n, t, foldMorph, ribScale, bass);

        vec3 l1 = normalize(vec3(0.4, 0.9, -0.35));
        vec3 l2 = normalize(vec3(-0.5, 0.4, 0.75));
        float diff1 = max(dot(n, l1), 0.0);
        float spec1 = pow(max(dot(reflect(-l1, n), v), 0.0), 52.0);
        float spec2 = pow(max(dot(reflect(-l2, n), v), 0.0), 28.0);

        float film = ndv * 1.2 + gyroid(pos * 2.2) * 0.22 + pos.z * 0.08 + accentPhase;
        vec3 opal = opalPalette(film);

        // Forward-surging energy wave down the nave on vaultPulse bang
        float surgeWave = exp(-abs((pos.z - zPos) - (1.5 + 10.0 * (1.0 - surge))) * 1.4) * surge;

        if (hitTrap < 0.0) {
            float cx = abs(pos.x - sin(pos.z * 0.24) * 0.35);
            float canal = smoothstep(0.42, 0.04, cx);
            float rail = exp(-abs(cx - 0.44) * 28.0);
            vec3 refl = opalPalette(reflect(rd, n).x * 1.5 + pos.z * 0.14 - t * 0.1 + accentPhase);
            col = vec3(0.014, 0.018, 0.034) * ao
                + refl * (0.18 + 0.95 * fres) * opalSheen * (0.35 + 0.65 * canal)
                + mix(vec3(0.32, 0.98, 1.38), opalPalette(accentPhase + 0.2) * 1.4, 0.45) * rail * (1.1 + 0.9 * bass + 1.2 * surgeWave)
                + vec3(1.2, 1.1, 0.95) * spec1 * 1.1;
        } else {
            float goldMask = smoothstep(0.42, 0.12, hitTrap);
            vec3 goldAccent = mix(vec3(1.18, 0.84, 0.36), opalPalette(accentPhase) * 1.35, 0.32);
            vec3 base = mix(vec3(0.035, 0.045, 0.082), goldAccent, goldMask * 0.85);
            float vein = exp(-hitTrap * 8.5) * (0.5 + 0.9 * high + 1.4 * surgeWave);
            vec3 veinCol = opalPalette(pos.z * 0.16 + t * 0.14 + accentPhase) * vein * 1.35;

            col = base * (0.20 + 0.85 * diff1) * ao
                + opal * (0.18 + 1.25 * fres) * opalSheen * ao
                + veinCol
                + vec3(1.3, 1.18, 0.98) * spec1 * 1.3
                + vec3(0.32, 0.95, 1.32) * spec2 * 0.72;
        }

        float fog = exp(-dist * 0.115);
        col = mix(fogCol, col, fog);
    }

    col += volShafts;
    float beatStrobe = mix(1.0, 0.42 + 1.28 * pow(clamp(chroma_BPM.z, 0.0, 1.0), 1.5), strobeGate);
    col *= beatStrobe * intensity * (1.0 + 0.20 * surge);
    col = aces(col);
    col = pow(col, vec3(1.0 / 2.2));
    fragColor = vec4(col, 1.0);
}
