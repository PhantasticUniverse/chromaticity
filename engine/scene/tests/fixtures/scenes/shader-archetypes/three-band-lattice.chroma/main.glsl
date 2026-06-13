// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// Per-frame + audio reactive uniforms. naga's GLSL 4.50 frontend requires
// uniforms to live in a layout(binding=X) block; the engine packs every
// scalar / vector into ONE std140 buffer at set 0, binding 0 (per
// engine/render/src/multipass.rs). Scalars occupy a vec4 slot and are read
// via `.x`; the FFT is a float[8] array. scene.json binds each field to a
// reactive symbol by NAME->SYMBOL, independent of the GLSL declaration shape.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;
    vec4 RENDERSIZE;
    vec4 chroma_AudioRMS;
    vec4 chroma_AudioPeak;
    vec4 chroma_AudioFFT[8];
    vec4 syn_BPM;
};

// Soft band kernel: bright at the cell edge, dark in the interior, so the
// lattice reads as a grid of lines rather than filled tiles. `t` is a
// fractional cell coordinate in [0,1); `w` is the line half-width.
float lineBand(float t, float w) {
    float d = min(t, 1.0 - t); // distance to the nearest cell boundary
    return 1.0 - smoothstep(0.0, w, d);
}

// Soft radial core for the per-cell dot. `p` is centred cell coords in
// [-0.5,0.5]; `r` is the dot radius. Saturates to 0 outside the radius.
float cellCore(vec2 p, float r) {
    float dist = length(p);
    return 1.0 - smoothstep(r * 0.5, r, dist);
}

void main() {
    float time = TIME.x;

    // Aspect-corrected coords so the lattice stays square on any RENDERSIZE.
    // RENDERSIZE is guaranteed positive by the render path; guard anyway.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    float aspect = res.x / res.y;
    vec2 p = uv;
    p.x *= aspect;

    // --- Three-band audio split (FFT low / mid / high groupings) ---------
    // Element [0] is the lowest band, [7] the highest (docs/36 v1.3.0).
    float bass = (chroma_AudioFFT[0].x + chroma_AudioFFT[1].x + chroma_AudioFFT[2].x) / 3.0;
    float mid  = (chroma_AudioFFT[3].x + chroma_AudioFFT[4].x + chroma_AudioFFT[5].x) / 3.0;
    float treb = (chroma_AudioFFT[6].x + chroma_AudioFFT[7].x) * 0.5;

    // Tame the non-normalised FFT magnitudes into a tractable [0,~1] feel
    // without ever forcing them negative or NaN. At silence all three are 0.
    bass = clamp(bass, 0.0, 4.0) * 0.6;
    mid  = clamp(mid, 0.0, 4.0) * 0.6;
    treb = clamp(treb, 0.0, 4.0) * 0.6;

    // BPM-derived pulse cadence; default 120 BPM => 2 beats/sec. Clamp away
    // from zero so the period is always finite.
    float bpm = clamp(syn_BPM.x, 30.0, 300.0);
    float beat = time * (bpm / 60.0);
    float pulse = 0.5 + 0.5 * sin(beat * 6.28318530718);

    // --- Lattice geometry ------------------------------------------------
    // Fixed lattice density (8 cells across). A slow TIME drift keeps motion
    // present even when audio is silent.
    float density = 8.0;
    vec2 drift = vec2(time * 0.03, time * 0.02);
    vec2 grid = (p + drift) * density;
    vec2 cellId = floor(grid);
    vec2 cellUv = fract(grid);              // [0,1) within the cell
    vec2 centered = cellUv - 0.5;           // [-0.5,0.5) within the cell

    // --- Channel split ---------------------------------------------------
    // Rows (horizontal lines) carry BASS; columns (vertical lines) carry
    // MID; the per-cell dot core carries TREBLE. RMS lifts the whole field.
    float lineW = 0.06 + 0.10 * bass;       // bass thickens the row lines
    float rowLine = lineBand(cellUv.y, lineW);
    float colLineW = 0.06 + 0.10 * mid;     // mid thickens the column lines
    float colLine = lineBand(cellUv.x, colLineW);

    // Peak briefly expands the treble cores; pulse animates them on-beat.
    float coreR = 0.18 + 0.22 * treb + 0.10 * clamp(chroma_AudioPeak.x, 0.0, 2.0);
    float core = cellCore(centered, coreR) * (0.35 + 0.65 * pulse);

    // Base floors guarantee a visible static lattice at zero audio: the
    // grid lines and a dim core always render, so the frame is never black.
    float rowI = rowLine * (0.18 + 0.82 * bass);
    float colI = colLine * (0.18 + 0.82 * mid);
    float coreI = core * (0.10 + 0.90 * treb);

    float rms = clamp(chroma_AudioRMS.x, 0.0, 2.0);
    float glow = 0.12 + 0.30 * rms;          // ambient lift, never zero

    // --- Colour assembly -------------------------------------------------
    // Map each band to a distinct hue channel so the split is legible:
    // bass -> warm red rows, mid -> green columns, treble -> blue cores.
    vec3 col = vec3(0.0);
    col += vec3(0.95, 0.30, 0.20) * rowI;    // bass rows (warm)
    col += vec3(0.25, 0.90, 0.45) * colI;    // mid columns (green)
    col += vec3(0.35, 0.55, 1.00) * coreI;   // treble cores (blue)

    // Ambient background so empty cells are a dim neutral lattice, not void.
    vec3 bg = vec3(0.04, 0.05, 0.08) + glow * vec3(0.20, 0.22, 0.30);
    col += bg;

    // Subtle checkerboard tint keyed off the cell id gives the lattice a
    // woven feel without depending on audio.
    float checker = mod(cellId.x + cellId.y, 2.0);
    col *= mix(0.92, 1.08, checker);

    col = clamp(col, 0.0, 1.0);
    fragColor = vec4(col, 1.0);
}
