// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// Reactive uniform declarations by exact symbol name (docs/36 v1.3.0).
// naga's GLSL 4.50 frontend requires layout(binding=N) on every uniform
// and rejects a bare `uniform float arr[8]` (std140 array stride/alignment
// mismatch), so chroma_AudioFFT is declared inside a std140 uniform block.
// Binding names/order match the scene.json PASS UNIFORMS NAME->SYMBOL map.
layout(binding = 0) uniform float TIME;
layout(binding = 1) uniform vec2 RENDERSIZE;
layout(binding = 2) uniform float FRAMECOUNT;
layout(binding = 3) uniform float chroma_AudioPeak;
layout(binding = 4) uniform AudioFft { vec4 chroma_AudioFFT[8]; };

// Author controls (bound by scene.json NAME -> control schema).
layout(binding = 5) uniform float snap_steps;     // base radial-segment count
layout(binding = 6) uniform float sensitivity;    // transient gain into the snap
layout(binding = 7) uniform float strobe_amount;  // hard-edge flash on a sharp onset

const float PI = 3.14159265359;
const float TAU = 6.28318530718;

// Hard quantiser: snaps a continuous value onto a discrete ladder so
// motion lands on steps instead of sliding. This is the geometric
// "snap" the archetype is built around.
float snap_to(float x, float steps) {
    steps = max(steps, 1.0);
    return floor(x * steps + 0.5) / steps;
}

void main() {
    // Square-aspect centred coordinate from the passthrough UV. We use
    // the provided uv + RENDERSIZE rather than gl_FragCoord so the
    // shader stays portable across the full-screen-triangle pipeline.
    vec2 res = max(RENDERSIZE, vec2(1.0));
    float aspect = res.x / res.y;
    vec2 p = (uv - 0.5) * vec2(aspect, 1.0) * 2.0;

    // Transient strength. chroma_AudioPeak is the window peak and the
    // lowest FFT band carries the kick energy; together they spike hard
    // on a sharp onset. At silence (peak = 0, FFT all 0) this is 0, so
    // every audio-driven term collapses to its neutral resting value.
    float lowBand = chroma_AudioFFT[0].x;
    float transient = clamp((chroma_AudioPeak + lowBand * 0.5) * (sensitivity * 4.0), 0.0, 1.0);

    // Polar decomposition.
    float ang = atan(p.y, p.x);
    float rad = length(p);

    // Segment count snaps UP in discrete jumps on a transient: more
    // facets appear the instant a peak lands, then the geometry holds.
    // At zero audio this is the flat base count (a stable mandala).
    float segs = snap_to(snap_steps + transient * 8.0, 1.0);
    segs = max(segs, 3.0);

    // Kaleidoscopic fold: mirror the angle into one wedge so the figure
    // is hard-edged and symmetric.
    float wedge = TAU / segs;
    float a = mod(ang, wedge);
    a = abs(a - wedge * 0.5);

    // Rotation is QUANTISED and only advances on transients: TIME gives
    // a slow continuous baseline drift (so silence is never frozen),
    // while the transient kicks the phase onto the next discrete notch.
    float baseSpin = TIME * 0.15;
    float snapSpin = snap_to(baseSpin + transient, 16.0);
    a += snapSpin * wedge;

    // Re-fold into the wedge after the rotation so edges stay crisp.
    a = mod(a, wedge);

    // Concentric ring ladder, also snapped: rings tighten on a hit.
    float ringCount = 4.0 + floor(transient * 6.0);
    float rings = fract(rad * ringCount - TIME * 0.25);
    // Hard step edge -> geometric, not soft.
    float ringMask = step(0.5, rings);

    // Wedge spoke mask: a sharp bar through the centre of each wedge.
    float spoke = smoothstep(0.06, 0.0, abs(a - wedge * 0.25));

    // Base figure intensity (always > 0 somewhere, so never all-black).
    float figure = max(ringMask * 0.65, spoke);
    // Vignette keeps the centre lit even in dead silence.
    float core = smoothstep(1.6, 0.0, rad);
    figure = max(figure, core * 0.35);

    // FRAMECOUNT strobe: a hard per-frame flash gated by the transient,
    // so sharp onsets read as a snap-flash. At zero audio strobe is 0.
    // floor(FRAMECOUNT) parity gives a deterministic 1-frame-on cadence.
    float frameParity = mod(floor(FRAMECOUNT), 2.0);
    float strobe = transient * strobe_amount * frameParity;

    // Colour: a fixed teal->magenta ramp by radius, brightened by the
    // transient. Resting palette is fully defined at zero audio.
    vec3 cold = vec3(0.10, 0.55, 0.70);
    vec3 hot = vec3(0.95, 0.22, 0.62);
    vec3 ramp = mix(cold, hot, clamp(rad * 0.55, 0.0, 1.0));

    vec3 color = ramp * figure;
    color += vec3(1.0) * strobe;                 // white snap-flash
    color += hot * transient * spoke * 0.6;      // accent the spokes on hits
    // Gentle ambient floor guarantees a sane, non-black image at silence.
    color += vec3(0.03, 0.04, 0.06);

    color = clamp(color, 0.0, 1.0);
    fragColor = vec4(color, 1.0);
}
