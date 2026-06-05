// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

// Audio Reactivity Test Card -- INTERNAL DIAGNOSTIC, not a shipped scene.
//
// Purpose: an unambiguous, MEASURABLE readout of the live audio uniforms so a
// human can judge capture-to-pixel latency and per-frame smoothness by eye.
// This is deliberately plain (no decorative motion): every pixel maps to a
// number. Original clean-room authoring from standard public
// spectrum-analyzer / level-bar / VU-meter technique.
//
// Readouts:
//   - 8 hue-ramped FFT spectrum bars (lower ~75% of the frame),
//     height = clamp(chroma_AudioFFT[b], 0, 1).
//   - Far-left RMS column, height = chroma_AudioRMS.x, with a bright PEAK tick
//     at height chroma_AudioPeak.x.
//   - Central beat disc whose radius + brightness grow with low-end energy;
//     FLASHES on a bass hit (this is the latency read).
//   - Faint 25/50/75% gridlines so bar heights are eyeball-measurable.
//   - Near-black background; finite + flat at silence.

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// std140 single uniform block at (set = 0, binding = 0). The render
// scheduler binds the scene.json UNIFORMS array POSITIONALLY into this
// block, so member order here MUST match the scene.json UNIFORMS order
// exactly. Scalars are declared as vec4 (one 16-byte std140 slot) and read
// via .x.
//
// chroma_AudioFFT expands into 8 consecutive vec4-aligned 16-byte slots
// (the FloatArray encoding writes each band into the FIRST 4 bytes of its
// slot). We declare it as `vec4 [8]` and read each band via `.x`: this
// matches the 16-byte-per-element buffer stride EXACTLY. (A bare
// `float chroma_AudioFFT[8]` is mis-read by naga's GLSL frontend, which
// assumes a tight 4-byte array stride and therefore only sees every 4th
// band -- verified empirically. The vec4[8] form is stride-correct.)
// Symbol names match docs/36 exactly.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;                 // .x = seconds since scene start
    vec4 RENDERSIZE;           // .xy = current pass pixel size
    vec4 chroma_AudioFFT[8];   // [b].x = band b, [0]=low .. [7]=high, ~[0,1]
    vec4 chroma_AudioRMS;      // .x = raw RMS energy, ~[0,1]
    vec4 chroma_AudioPeak;     // .x = window peak magnitude, ~[0,1]
    vec4 chroma_AudioBass;     // .x = relative low band, ~1.0 at silence
    vec4 chroma_AudioMid;      // .x = relative mid band, ~1.0 at silence
    vec4 chroma_AudioTreb;     // .x = relative high band, ~1.0 at silence
};

// --- Coordinate / orientation convention --------------------------------
// uv is the passthrough fragment UV in [0,1] with uv.y = 0 at the TOP of the
// frame and uv.y = 1 at the BOTTOM (standard framebuffer top-left origin).
// For all measurement we work in a MEASUREMENT-HEIGHT coordinate
//     h = 1.0 - uv.y
// so h = 0 is the BOTTOM edge and h = 1 is the TOP edge. Bars and the RMS
// column therefore grow UPWARD from the bottom, which is the intuitive way to
// read a level / spectrum display. x runs left (0) to right (1) unchanged.

// Fixed layout constants (fractions of the frame).
const float BAR_AREA_TOP   = 0.75; // FFT bars fill the lower 75% of the frame
const float RMS_COL_RIGHT  = 0.035; // RMS column occupies the far-left x slot
const float BAR_GAP        = 0.22;  // fraction of each bar slot left as dark gap
const float TICK_HALF      = 0.006; // peak-tick half-thickness in h
const float GRID_HALF      = 0.0016; // gridline half-thickness in h

// Original 8-step hue ramp for the spectrum bars: a smooth low->high colour
// sweep (blue/cyan for lows ... red/magenta for highs) via an in-gamut cosine
// palette. Authored from scratch; t in [0,1].
vec3 barHue(float t) {
    vec3 a = vec3(0.50, 0.50, 0.50);
    vec3 b = vec3(0.50, 0.50, 0.50);
    vec3 c = vec3(1.00, 1.00, 1.00);
    vec3 d = vec3(0.65, 0.40, 0.15); // phase: sweeps blue -> cyan -> ... -> red
    return clamp(a + b * cos(6.28318530718 * (c * t + d)), 0.0, 1.0);
}

void main() {
    // Measurement-height coordinate (0 = bottom, 1 = top); see convention note.
    float h = 1.0 - uv.y;
    float x = uv.x;

    // Aspect for the round beat disc (so the circle is not egg-shaped).
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    float aspect = res.x / res.y;

    // Near-black background.
    vec3 col = vec3(0.015, 0.015, 0.022);

    // --- Faint 25/50/75% gridlines --------------------------------------
    // Drawn across the whole width so any bar/column height is measurable.
    for (int g = 1; g <= 3; g++) {
        float gh = 0.25 * float(g);
        float line = 1.0 - smoothstep(0.0, GRID_HALF, abs(h - gh));
        col = mix(col, vec3(0.16, 0.16, 0.20), line * 0.6);
    }

    // --- 8 FFT spectrum bars (lower BAR_AREA_TOP of the frame) -----------
    // The bar field spans x in [RMS_COL_RIGHT .. 1.0]; each of 8 bars owns a
    // fixed equal x-slot with a dark gap between, so the bars are clearly
    // separated and each band's height is independently readable. Bar height
    // is the band value mapped into the [0 .. BAR_AREA_TOP] vertical range.
    float fieldL = RMS_COL_RIGHT + 0.02; // small margin past the RMS column
    float fieldR = 0.99;
    if (x >= fieldL && x <= fieldR) {
        float fx = (x - fieldL) / (fieldR - fieldL); // 0..1 across the bar field
        float slot = fx * 8.0;
        int b = int(floor(slot));
        b = clamp(b, 0, 7);
        float inSlot = slot - float(b); // 0..1 within this bar's slot
        // Keep a dark gap on each side of the bar within its slot.
        if (inSlot > BAR_GAP * 0.5 && inSlot < 1.0 - BAR_GAP * 0.5) {
            float band = clamp(chroma_AudioFFT[b].x, 0.0, 1.0);
            float barTopH = band * BAR_AREA_TOP; // height of this bar in h
            if (h <= barTopH) {
                vec3 hue = barHue((float(b) + 0.5) / 8.0);
                // Slightly brighter toward the bar's top edge for a crisp cap.
                float cap = smoothstep(barTopH - 0.012, barTopH, h);
                col = hue * (0.55 + 0.45 * (h / max(barTopH, 1e-3)));
                col += vec3(0.25) * cap;
            }
        }
    }

    // --- Far-left RMS column + PEAK tick ---------------------------------
    // A thin vertical bar pinned to the left edge: fill height = RMS, with a
    // bright horizontal tick at the PEAK height. Always present so silence
    // reads as an empty column (not a missing one).
    if (x <= RMS_COL_RIGHT) {
        // Column frame so the empty column is still visible at silence.
        col = mix(col, vec3(0.08, 0.08, 0.10), 0.7);
        float rms = clamp(chroma_AudioRMS.x, 0.0, 1.0);
        if (h <= rms) {
            // Green-to-amber level fill (classic level-meter ballistics colour).
            vec3 lo = vec3(0.10, 0.85, 0.30);
            vec3 hi = vec3(0.95, 0.75, 0.10);
            col = mix(lo, hi, clamp(rms, 0.0, 1.0)) * (0.6 + 0.4 * (h / max(rms, 1e-3)));
        }
        // Peak tick: a bright marker at the peak height.
        float peak = clamp(chroma_AudioPeak.x, 0.0, 1.0);
        float tick = 1.0 - smoothstep(0.0, TICK_HALF, abs(h - peak));
        col = mix(col, vec3(1.0, 0.95, 0.85), tick);
    }

    // --- Central beat disc ----------------------------------------------
    // Low-end energy: prefer the relative bass band's reactive part
    // (chroma_AudioBass.x - 1.0, which is 0 at silence), OR fall back to the
    // mean of the two lowest FFT bands -- whichever reads stronger. This makes
    // the disc FLASH on a bass hit regardless of which path carries the energy.
    float bassRel = max(chroma_AudioBass.x - 1.0, 0.0);
    float fftLow  = 0.5 * (clamp(chroma_AudioFFT[0].x, 0.0, 1.0)
                         + clamp(chroma_AudioFFT[1].x, 0.0, 1.0));
    float lowEnergy = clamp(max(bassRel, fftLow), 0.0, 1.0);

    // Disc centred at (0.5, 0.5). Use aspect-corrected distance so it is round.
    vec2 cp = vec2((uv.x - 0.5) * aspect, (uv.y - 0.5));
    float dist = length(cp);
    // Base radius at silence is small but clearly visible; grows with energy.
    float baseRadius = 0.05;
    float radius = baseRadius + 0.16 * lowEnergy;
    float disc = 1.0 - smoothstep(radius - 0.012, radius + 0.012, dist);
    // Brightness ramps hard with low energy so a bass hit is an obvious flash.
    float discBright = 0.22 + 0.95 * lowEnergy;
    // Warm white-hot core for the flash; reads instantly against the dark field.
    vec3 discColor = mix(vec3(0.35, 0.40, 0.55), vec3(1.0, 0.92, 0.70), lowEnergy);
    col = mix(col, discColor * discBright, disc);

    // Final guard: clamp to a displayable range; no NaN/blown path at silence.
    col = clamp(col, vec3(0.0), vec3(1.0));
    fragColor = vec4(col, 1.0);
}
