// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// All per-frame reactive uniforms are marshalled into the single
// std140 block at (set = 0, binding = 0) by engine/render's multipass
// scheduler; each scalar / vector occupies one vec4-aligned 16-byte
// slot, and chroma_AudioFFT expands into 8 consecutive slots. Scalars
// are declared as vec4 and read via .x (matching the production
// audio-adapter fixture). Symbol names match docs/36 v1.3.0 exactly.
//
// The six relative reactive bands (docs/06 Sec.7.15, ADR-024) are the
// PLANNED MilkDrop-class auto-gain bands: volume-independent, centred on
// the neutral 1.0. The render-side exposure is the Phase-2 follow-up to
// the engine-side ReactiveBands wiring, so today the render path feeds
// the neutral baseline (each band == 1.0, each attack == 1.0). The
// shader is authored so neutral input yields a calm, TIME-animated
// three-lobe pose -- flat with respect to audio until the binding lands,
// but never black, static, or NaN. They are appended to the block as
// vec4 slots read via .x, matching the scalar convention above.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;
    vec4 RENDERSIZE;
    vec4 chroma_AudioFFT[8];
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 syn_BPM;
    vec4 chroma_AudioBass;
    vec4 chroma_AudioMid;
    vec4 chroma_AudioTreb;
    vec4 chroma_AudioBass_att;
    vec4 chroma_AudioMid_att;
    vec4 chroma_AudioTreb_att;
};

// Soft radial lobe. `p` is the fragment position relative to the lobe
// centre (aspect-corrected); `radius` sets the bright core size and
// `sharp` controls the falloff steepness. Always >= 0, never NaN.
float lobe(vec2 p, float radius, float sharp) {
    float d = length(p);
    // smoothstep gives a soft-edged disc that fades to 0 outside `radius`.
    // The inner edge scales with `sharp` so a high attack tightens the core.
    float inner = radius * mix(0.65, 0.15, clamp(sharp, 0.0, 1.0));
    return 1.0 - smoothstep(inner, radius, d);
}

void main() {
    // Aspect-corrected centred coords so the lobes stay round on any
    // RENDERSIZE. RENDERSIZE is positive on the render path; guard anyway.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    float aspect = res.x / res.y;
    vec2 p = (uv - 0.5) * vec2(aspect, 1.0);

    // --- Relative-band normalisation ------------------------------------
    // A relative band is multiplicative and centred on 1.0. Map the band
    // back to a 0-centred "lift" so neutral (1.0) is the rest size and a
    // loud band swells the lobe. The att axis (short/long ratio) reads as
    // a transient sharpness flash; neutral 1.0 -> 0 flash. Both are clamped
    // so a degenerate analyser value can never blow the geometry up.
    float bassLift = clamp(chroma_AudioBass.x, 0.0, 4.0) - 1.0;
    float midLift  = clamp(chroma_AudioMid.x,  0.0, 4.0) - 1.0;
    float trebLift = clamp(chroma_AudioTreb.x, 0.0, 4.0) - 1.0;

    float bassAtt = clamp(chroma_AudioBass_att.x - 1.0, 0.0, 2.0);
    float midAtt  = clamp(chroma_AudioMid_att.x  - 1.0, 0.0, 2.0);
    float trebAtt = clamp(chroma_AudioTreb_att.x - 1.0, 0.0, 2.0);

    // --- Idle breathing so neutral input still animates -----------------
    // Driven purely by TIME, this keeps the three lobes gently pulsing and
    // orbiting even when every band rests at the neutral 1.0 (the state the
    // render path feeds today). Phases are offset per lobe for a lively,
    // non-synchronised feel.
    float TAU = 6.28318530718;
    float breath = 0.5 + 0.5 * sin(TIME.x * 0.7);
    float bassBreath = 0.5 + 0.5 * sin(TIME.x * 0.55 + 0.0);
    float midBreath  = 0.5 + 0.5 * sin(TIME.x * 0.80 + 2.094);   // +120 deg
    float trebBreath = 0.5 + 0.5 * sin(TIME.x * 1.05 + 4.188);   // +240 deg

    // Slow orbit radius for the lobe centres; small so they stay legible.
    float orbit = 0.30;
    float spin = TIME.x * 0.20;

    // Lobe centres arranged as a triangle (bass low-left, mid top, treb
    // low-right), each drifting on a gentle TIME orbit.
    vec2 bassC = vec2(-0.42, -0.22) + orbit * 0.10 * vec2(cos(spin), sin(spin));
    vec2 midC  = vec2( 0.00,  0.40) + orbit * 0.10 * vec2(cos(spin + 2.094), sin(spin + 2.094));
    vec2 trebC = vec2( 0.42, -0.22) + orbit * 0.10 * vec2(cos(spin + 4.188), sin(spin + 4.188));

    // --- Lobe radii + sharpness -----------------------------------------
    // Base radius breathes on TIME; band lift swells it; att flash narrows
    // the core. The constant base guarantees a visible lobe at neutral.
    float baseR = 0.26;
    float bassR = baseR + 0.06 * bassBreath + 0.28 * bassLift;
    float midR  = baseR + 0.06 * midBreath  + 0.28 * midLift;
    float trebR = baseR + 0.06 * trebBreath + 0.28 * trebLift;

    float bassI = lobe(p - bassC, max(bassR, 0.05), bassAtt);
    float midI  = lobe(p - midC,  max(midR,  0.05), midAtt);
    float trebI = lobe(p - trebC, max(trebR, 0.05), trebAtt);

    // A brief additive flash on the attack axis (0 at neutral).
    bassI += bassAtt * 0.35 * lobe(p - bassC, max(bassR, 0.05) * 1.4, 0.0);
    midI  += midAtt  * 0.35 * lobe(p - midC,  max(midR,  0.05) * 1.4, 0.0);
    trebI += trebAtt * 0.35 * lobe(p - trebC, max(trebR, 0.05) * 1.4, 0.0);

    // --- Colour assembly -------------------------------------------------
    // Distinct hue per band so the three lobes read apart: bass -> warm
    // magenta-red, mid -> green, treb -> cyan-blue.
    vec3 col = vec3(0.0);
    col += vec3(0.95, 0.25, 0.45) * bassI;   // bass lobe (warm)
    col += vec3(0.35, 0.92, 0.45) * midI;    // mid lobe (green)
    col += vec3(0.30, 0.70, 1.00) * trebI;   // treb lobe (cyan)

    // Dark indigo base with a faint TIME breath so the field is never
    // pure black even where no lobe reaches.
    vec3 bg = vec3(0.03, 0.04, 0.07) + 0.04 * breath * vec3(0.4, 0.5, 0.8);
    col += bg;

    // Gentle vignette to seat the lobes in the frame.
    float vig = 1.0 - 0.35 * dot(p, p);
    col *= clamp(vig, 0.4, 1.0);

    col = clamp(col, 0.0, 1.0);
    fragColor = vec4(col, 1.0);
}
