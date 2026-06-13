// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;

// Reactive uniforms by exact symbol name (bound via scene.json NAME->SYMBOL).
// naga's GLSL frontend requires uniforms inside a layout(binding=X) block;
// scalar reactive symbols arrive as vec4 (read via .x), mirroring the
// engine/render audio-adapter marshalling layout (std140).
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 TIME;
    vec4 RENDERSIZE;
    vec4 syn_BPM;
    vec4 chroma_AudioPeak;
};

void main() {
    float t = TIME.x;

    // Tempo-locked phase. Fall back to a sane 120 BPM if the BPM
    // estimate has not warmed up (avoids a divide-toward-zero or a
    // frozen flash at silence). One full phase cycle per beat.
    float bpm = max(syn_BPM.x, 30.0);
    float beats_per_sec = bpm / 60.0;
    float phase = fract(t * beats_per_sec);

    // Beat envelope: a sharp on-beat spike that decays across the
    // beat interval. cos() gives a smooth 1.0-at-phase-0 peak; the
    // exponent steepens the attack so the flash reads as a strobe
    // rather than a sine swell.
    float beat = pow(max(cos(phase * 6.2831853), 0.0), 6.0);

    // Audio peak gates the flash intensity. At zero audio the gate
    // floor keeps a faint tempo-driven pulse alive (never black),
    // while a strong transient slams the strobe to full brightness.
    float gate = 0.18 + 0.82 * clamp(chroma_AudioPeak.x, 0.0, 1.0);
    float flash = beat * gate;

    // A subtle vignette gives the full-frame flash some shape so the
    // image is not a literally uniform color field. Aspect-correct via
    // RENDERSIZE so the falloff stays circular on non-square targets.
    float aspect = max(RENDERSIZE.x, 1.0) / max(RENDERSIZE.y, 1.0);
    vec2 centered = (uv - 0.5) * vec2(aspect, 1.0);
    float vignette = 1.0 - 0.45 * dot(centered, centered);

    // Warm strobe color, shifting slightly across the beat phase so
    // successive flashes are not identical.
    vec3 flash_color = vec3(1.0, 0.92, 0.78);
    vec3 tint = mix(flash_color, vec3(0.62, 0.78, 1.0), 0.5 * (1.0 - beat));

    // Base ambient so silence renders a dim breathing field rather
    // than full black.
    float ambient = 0.10 + 0.05 * sin(t * 0.5);

    vec3 color = tint * (ambient + flash) * vignette;
    color = clamp(color, 0.0, 1.0);

    fragColor = vec4(color, 1.0);
}
