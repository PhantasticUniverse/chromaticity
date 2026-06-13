// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// Per-frame + audio uniforms, packed into the single std140 uniform
// block the engine binds at set=0, binding=0 (engine/render multipass
// scheme). Each scalar/vec symbol occupies one 16-byte slot, so they
// are declared as vec4 and read via .x / .xy. Block member order
// matches the scene.json UNIFORMS array (NAME->SYMBOL binding is by
// declaration order, not by name).
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;            // .x = seconds since scene start
    vec4 RENDERSIZE;      // .xy = current pass pixel size
    vec4 chroma_AudioRMS; // .x = raw RMS energy [0, inf)
    vec4 syn_BPM;         // .x = estimated BPM, clamped [50, 220] upstream
};

const float PI = 3.14159265359;
const float TWO_PI = 6.28318530718;

// Author controls (CONTROLS stanza). Defaults: base_speed=0.5,
// rms_gain=1.5, ring_density=8.0, twist=0.3. The scene-adapter does
// not yet bridge CONTROLS/UNIFORMS into GPU bindings (audio-binding
// slice owns that), so these are PLANNED-not-wired: hardcoded here to
// their declared defaults to keep the archetype self-contained and
// naga-valid today.
const float BASE_SPEED = 0.5;
const float RMS_GAIN = 1.5;
const float RING_DENSITY = 8.0;
const float TWIST = 0.3;

// Cheap cosine palette (Inigo Quilez style, original coefficients).
vec3 palette(float t) {
    vec3 a = vec3(0.55, 0.45, 0.55);
    vec3 b = vec3(0.45, 0.40, 0.45);
    vec3 c = vec3(1.00, 1.00, 1.00);
    vec3 d = vec3(0.00, 0.20, 0.45);
    return a + b * cos(TWO_PI * (c * t + d));
}

void main() {
    float time = TIME.x;

    // Aspect-corrected centered coordinates from uv + RENDERSIZE so we
    // never read gl_FragCoord. Guard against a degenerate resolution.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    float aspect = res.x / res.y;
    vec2 p = (uv - 0.5) * vec2(aspect, 1.0) * 2.0;

    // Radius / angle for the polar tunnel mapping. Floor the radius so
    // the 1/r depth term can never divide by zero (no NaN at center).
    float r = max(length(p), 1e-3);
    float angle = atan(p.y, p.x);

    // Sanitise audio: RMS is non-normalised [0, inf); clamp to a sane
    // reactive band. At silence rms == 0 and the tunnel still flies on
    // BASE_SPEED alone.
    float rms = clamp(chroma_AudioRMS.x, 0.0, 4.0);
    float speed = BASE_SPEED + rms * RMS_GAIN;

    // BPM ring cadence. syn_BPM is clamped [50, 220] upstream; guard
    // anyway. beats-per-second drives a beat-phase clock in [0,1).
    float bpm = clamp(syn_BPM.x, 1.0, 300.0);
    float beatHz = bpm / 60.0;
    float beat = fract(time * beatHz);
    // Smooth per-beat pulse (rises fast, decays) used as ring emphasis.
    float pulse = pow(1.0 - beat, 3.0);

    // Tunnel depth: travel forward along 1/r so the wall rushes toward
    // the camera. time * speed advances the apparent distance.
    float depth = 0.25 / r + time * speed;

    // Twist the angle as we travel for a corkscrew feel.
    float twistedAngle = angle + depth * (TWIST * 2.0);
    float u = twistedAngle / TWO_PI;

    // Concentric rings spaced by RING_DENSITY, modulated by the BPM beat
    // so rings brighten on the downbeat.
    float density = max(RING_DENSITY, 1.0);
    float rings = 0.5 + 0.5 * sin(depth * density);
    rings = mix(rings, rings * (1.0 + pulse), 0.6);

    // Texture the tunnel walls with a band along the angle.
    float bands = 0.5 + 0.5 * sin(u * TWO_PI * 6.0 + time * 0.5);

    // Base color from palette indexed by depth; energy shifts the hue.
    vec3 col = palette(depth * 0.15 + rms * 0.2);
    col *= rings;
    col = mix(col, col * bands, 0.35);

    // Center glow + distance fade keeps a non-black floor everywhere.
    float glow = smoothstep(1.2, 0.0, r);
    col += vec3(0.04, 0.05, 0.08) + glow * (0.15 + 0.4 * pulse);

    // Vignette toward the tunnel mouth; clamp to a safe display range.
    float vignette = smoothstep(2.2, 0.2, r);
    col *= vignette;
    col = clamp(col, 0.0, 1.0);

    fragColor = vec4(col, 1.0);
}
