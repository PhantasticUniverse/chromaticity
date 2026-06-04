// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// All per-frame reactive uniforms are marshalled into the single std140
// block at (set = 0, binding = 0) by engine/render's multipass scheduler.
// Each scalar / vector occupies one vec4-aligned 16-byte slot (declared as
// vec4, read via .x / .xy); chroma_AudioFFT expands into 8 consecutive
// slots as a float[8] array. Block member order matches the scene.json
// UNIFORMS array (NAME->SYMBOL binding is by declaration order). Symbol
// names match docs/36 v1.3.0 exactly.
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;             // .x = seconds since scene start
    vec4 RENDERSIZE;       // .xy = current pass pixel size
    vec4 chroma_AudioRMS;  // .x = raw RMS energy [0, inf)
    vec4 chroma_AudioPeak; // .x = window peak magnitude [0, inf)
    float chroma_AudioFFT[8]; // 8 log-spaced FFT bands [0]=low .. [7]=high
    vec4 syn_BPM;          // .x = estimated BPM, clamped [50, 220] upstream
};

const float PI = 3.14159265359;
const float TWO_PI = 6.28318530718;

// Author controls (CONTROLS stanza). Defaults: warp_amount=0.6,
// plasma_scale=3.0, crest_gain=1.0, tint_strength=0.6. The scene-adapter
// does not yet bridge CONTROLS into GPU bindings, so these are
// PLANNED-not-wired: hardcoded here to their declared defaults to keep the
// archetype self-contained and naga-valid today.
const float WARP_AMOUNT = 0.6;
const float PLASMA_SCALE = 3.0;
const float CREST_GAIN = 1.0;
const float TINT_STRENGTH = 0.6;

// Inigo-Quilez-style cosine palette (original coefficients) -- a smooth,
// always-in-gamut color ramp parameterised by a scalar t.
vec3 palette(float t) {
    vec3 a = vec3(0.50, 0.45, 0.55);
    vec3 b = vec3(0.45, 0.42, 0.48);
    vec3 c = vec3(1.00, 1.00, 1.00);
    vec3 d = vec3(0.10, 0.30, 0.55);
    return a + b * cos(TWO_PI * (c * t + d));
}

// log1p-style soft compressor: maps [0, inf) -> [0, ~1) so a tiny value and
// a huge full-scale value both land in a legible band. ln(1 + k*x)/ln(1 + k).
// This is the wide-dynamic legibility lever: it compresses the whole RMS /
// peak range into a viewable response instead of clipping or staying flat.
float softLog(float x, float k) {
    x = max(x, 0.0);
    return log(1.0 + k * x) / log(1.0 + k);
}

void main() {
    float time = TIME.x;

    // Aspect-corrected, centered coordinates from the passthrough UV; never
    // read gl_FragCoord. Guard against a degenerate resolution.
    vec2 res = max(RENDERSIZE.xy, vec2(1.0));
    float aspect = res.x / res.y;
    vec2 p = (uv - 0.5) * vec2(aspect, 1.0) * 2.0;

    // --- Wide-dynamic energy split ---------------------------------------
    // RMS and peak are non-normalised [0, inf). Soft-compress each so the
    // shader reads sanely from silence to full-scale. Both are 0 at silence.
    float rmsRaw = chroma_AudioRMS.x;
    float peakRaw = chroma_AudioPeak.x;
    float softRms = softLog(rmsRaw, 6.0);
    float softPeak = softLog(peakRaw, 6.0);

    // Crest factor: how "peaky" the signal is vs. its sustained energy.
    // Physically peak >= rms, so peak/rms >= 1; subtract 1 for a [0, inf)
    // transient-spikiness measure, then soft-compress. The +1e-3 floors keep
    // this finite at silence (rms == 0 -> crest == 0, no divide blow-up).
    float crestRatio = (peakRaw + 1e-3) / (rmsRaw + 1e-3);
    float crest = softLog(max(crestRatio - 1.0, 0.0), 4.0) * CREST_GAIN;

    // Sustained loudness drives overall brightness/contrast; the crest
    // (transient spikiness) drives extra warp turbulence. Both bounded [0,1].
    float loud = clamp(softRms, 0.0, 1.0);
    float turbulence = clamp(crest, 0.0, 1.0);

    // --- BPM-locked bloom -------------------------------------------------
    // syn_BPM is clamped [50,220] upstream; guard anyway. A smooth per-beat
    // pulse (rises fast, decays) gives a living cadence even at silence.
    float bpm = clamp(syn_BPM.x, 1.0, 300.0);
    float beat = fract(time * bpm / 60.0);
    float bloom = pow(1.0 - beat, 3.0);

    // --- Full-spectrum FFT tint ------------------------------------------
    // Energy-weighted spectral centroid over all eight bands, normalised to
    // [0,1]. Falls back to a neutral 0.5 when the spectrum is empty (silence),
    // so the hue index is always well-defined and never NaN. Also accumulate
    // a soft-compressed total spectral energy for a tint-intensity term.
    float weighted = 0.0;
    float total = 0.0;
    for (int i = 0; i < 8; i++) {
        float band = max(chroma_AudioFFT[i], 0.0);
        weighted += band * float(i);
        total += band;
    }
    float centroidIdx = (total > 1e-5) ? (weighted / (total * 7.0)) : 0.5;
    float spectralEnergy = softLog(total, 3.0);

    // --- Plasma field -----------------------------------------------------
    // Classic summed-sine plasma, domain-warped. Warp depth scales with
    // turbulence (crest) and the BPM bloom; even at zero audio the
    // WARP_AMOUNT floor plus TIME keep the field alive and flowing.
    float t = time * 0.6;
    float warp = WARP_AMOUNT * (0.5 + 0.5 * turbulence + 0.4 * bloom);
    float scale = PLASMA_SCALE * (1.0 + 0.35 * loud);

    vec2 q = p * scale;
    // Two-step domain warp: offset the sample point by a sine field, which
    // gives the fluid, floor-to-ceiling stretch characteristic of plasma.
    vec2 warped = q + warp * vec2(
        sin(q.y * 1.3 + t * 1.1),
        cos(q.x * 1.7 - t * 0.9)
    );

    float field = 0.0;
    field += sin(warped.x + t);
    field += sin(warped.y * 1.3 + t * 1.2);
    field += sin((warped.x + warped.y) * 0.7 - t * 0.8);
    field += sin(length(warped) * 1.1 + t * 1.5);
    // Normalise the 4-term sum from [-4,4] to [0,1].
    field = field * 0.125 + 0.5;

    // --- Color composition ------------------------------------------------
    // Index the palette by the plasma field plus a slow centroid-driven hue
    // drift, so the FFT spectral tint shifts the whole plasma's color.
    float hueIndex = field + centroidIdx * 0.5 * TINT_STRENGTH + time * 0.02;
    vec3 plasmaColor = palette(hueIndex);

    // Spectral tint: lerp toward a hue picked purely from the centroid, by an
    // amount set by spectral energy. At silence (spectralEnergy == 0) this is
    // a no-op, so the base plasma palette shows through unmodified.
    vec3 tintColor = palette(centroidIdx + 0.5);
    plasmaColor = mix(plasmaColor, tintColor, TINT_STRENGTH * spectralEnergy * 0.6);

    // Wide-dynamic brightness shaping: a comfortable baseline keeps silence a
    // calm lit plasma (never black); loud + bloom lift it; the response is
    // already soft-compressed so full-scale saturates gracefully instead of
    // blowing out to flat white.
    float brightness = 0.32 + 0.5 * loud + 0.25 * bloom * (0.3 + loud);
    vec3 col = plasmaColor * brightness;

    // Crest highlights: transient spikiness adds bright filaments along the
    // plasma ridges so a peaky signal reads distinctly from a sustained one.
    float ridge = smoothstep(0.72, 0.98, field);
    col += vec3(0.9, 0.85, 1.0) * ridge * turbulence * 0.6;

    // Gentle vignette for framing; always finite.
    float vig = smoothstep(2.3, 0.3, length(p));
    col *= mix(0.65, 1.0, vig);

    // Final guard: clamp to a displayable range so no path is NaN/blown.
    col = clamp(col, vec3(0.0), vec3(1.0));
    fragColor = vec4(col, 1.0);
}
