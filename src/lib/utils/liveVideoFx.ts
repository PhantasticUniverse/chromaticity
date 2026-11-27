// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Pure, testable real-time pixel keying and CSS/VJ effect helpers for Mix Studio.
 *
 * Implements:
 * - Rec. 601/709 YCbCr chroma keying with similarity, smoothness, and spill suppression.
 * - Rec. 709 luma keying with threshold, feather, and invert.
 * - Composite CSS filter string synthesis combining Color Grade, Stylize/VJ FX,
 *   and Audio-Reactive Link modulation.
 */

import type {
  AudioLinkFilter,
  ChromaKeyFilter,
  ColorGradeFilter,
  LayerFilter,
  LumaKeyFilter,
  MixLayer,
  StylizeFilter,
} from '../stores/videoMixer.svelte';

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.max(min, Math.min(max, value));
}

function roundTo(value: number, decimals: number): number {
  const factor = 10 ** decimals;
  return Math.round(value * factor) / factor;
}

function findLayerFilter<T extends LayerFilter>(layer: MixLayer, kind: T['kind']): T | null {
  const match = layer.filters.find((item) => item.kind === kind);
  return match ? (match as T) : null;
}

/**
 * Parse a 6-digit or 3-digit hex color string into `[r, g, b]` bytes (0..255).
 * Falls back to chroma green `[0, 210, 106]` on malformed input.
 */
export function parseHexRgb(hex: string): [number, number, number] {
  const cleaned = hex.trim().replace(/^#/, '');
  if (/^[0-9a-fA-F]{6}$/.test(cleaned)) {
    return [
      Number.parseInt(cleaned.slice(0, 2), 16),
      Number.parseInt(cleaned.slice(2, 4), 16),
      Number.parseInt(cleaned.slice(4, 6), 16),
    ];
  }
  if (/^[0-9a-fA-F]{3}$/.test(cleaned)) {
    const r = cleaned.charAt(0);
    const g = cleaned.charAt(1);
    const b = cleaned.charAt(2);
    return [Number.parseInt(r + r, 16), Number.parseInt(g + g, 16), Number.parseInt(b + b, 16)];
  }
  return [0, 210, 106];
}

export interface PixelKeyingOptions {
  readonly chromaKey?: ChromaKeyFilter | null;
  readonly lumaKey?: LumaKeyFilter | null;
}

/**
 * Apply real-time YCbCr chroma keying (with spill suppression) and/or Rec. 709
 * luma keying directly in-place on an RGBA `Uint8ClampedArray` pixel buffer.
 */
export function applyPixelKeyingToImageData(
  data: Uint8ClampedArray,
  options: PixelKeyingOptions,
): void {
  const ck = options.chromaKey?.enabled ? options.chromaKey : null;
  const lk = options.lumaKey?.enabled ? options.lumaKey : null;
  if (ck === null && lk === null) return;

  let keyCb = 0;
  let keyCr = 0;
  let ckSim = 0;
  let ckSmooth = 0.001;
  let ckSpill = 0;
  let spillChannel: 'r' | 'g' | 'b' = 'g';

  if (ck !== null) {
    const [krByte, kgByte, kbByte] = parseHexRgb(ck.keyColor);
    const kr = krByte / 255;
    const kg = kgByte / 255;
    const kb = kbByte / 255;
    keyCb = -0.168736 * kr - 0.331264 * kg + 0.5 * kb;
    keyCr = 0.5 * kr - 0.418688 * kg - 0.081312 * kb;
    ckSim = clamp(ck.similarity, 0, 1) * 0.5;
    ckSmooth = Math.max(0.001, clamp(ck.smoothness, 0, 1) * 0.35);
    ckSpill = clamp(ck.spill, 0, 1);

    if (kgByte >= kbByte && kgByte >= krByte) {
      spillChannel = 'g';
    } else if (kbByte >= krByte && kbByte >= kgByte) {
      spillChannel = 'b';
    } else {
      spillChannel = 'r';
    }
  }

  const lkThresh = lk !== null ? clamp(lk.threshold, 0, 1) : 0;
  const lkFeather = lk !== null ? Math.max(0.001, clamp(lk.feather, 0, 1)) : 0.001;
  const lkInvert = lk !== null ? lk.invert : false;

  const len = data.length;
  for (let i = 0; i < len; i += 4) {
    let rByte = data[i] ?? 0;
    let gByte = data[i + 1] ?? 0;
    let bByte = data[i + 2] ?? 0;
    let alpha = data[i + 3] ?? 255;

    if (alpha === 0) continue;

    if (ck !== null) {
      const r = rByte / 255;
      const g = gByte / 255;
      const b = bByte / 255;
      const cb = -0.168736 * r - 0.331264 * g + 0.5 * b;
      const cr = 0.5 * r - 0.418688 * g - 0.081312 * b;
      const dist = Math.hypot(cb - keyCb, cr - keyCr);
      const mask = clamp((dist - ckSim) / ckSmooth, 0, 1);
      alpha = Math.round(alpha * mask);

      if (ckSpill > 0 && alpha > 0) {
        if (spillChannel === 'g') {
          const avg = (rByte + bByte) * 0.5;
          const excess = gByte - avg;
          if (excess > 0) {
            gByte = Math.round(gByte - excess * ckSpill);
          }
        } else if (spillChannel === 'b') {
          const avg = (rByte + gByte) * 0.5;
          const excess = bByte - avg;
          if (excess > 0) {
            bByte = Math.round(bByte - excess * ckSpill);
          }
        } else {
          const avg = (gByte + bByte) * 0.5;
          const excess = rByte - avg;
          if (excess > 0) {
            rByte = Math.round(rByte - excess * ckSpill);
          }
        }
      }
    }

    if (lk !== null && alpha > 0) {
      const luma = (0.2126 * rByte + 0.7152 * gByte + 0.0722 * bByte) / 255;
      const raw = lkInvert ? (lkThresh - luma) / lkFeather : (luma - lkThresh) / lkFeather;
      const mask = clamp(raw, 0, 1);
      alpha = Math.round(alpha * mask);
    }

    data[i] = rByte;
    data[i + 1] = gByte;
    data[i + 2] = bByte;
    data[i + 3] = alpha;
  }
}

/**
 * Build the composite CSS `filter` string for a stage layer, combining Color Grade
 * (including gamma approximation, invert, sepia, grayscale), Stylize FX (blur, bloom),
 * and Audio-Reactive Link modulation (`glow`, `hue`, `blur`).
 */
export function buildLayerCssFilter(layer: MixLayer, audioPulse = 1): string {
  const parts: string[] = [];
  const pulse = clamp(audioPulse, 0, 2);

  const grade = findLayerFilter<ColorGradeFilter>(layer, 'colorGrade');
  const stylize = findLayerFilter<StylizeFilter>(layer, 'stylize');
  const audio = findLayerFilter<AudioLinkFilter>(layer, 'audioLink');

  const audioHueDeg =
    audio?.enabled && audio.target === 'hue' ? Math.round(audio.amount * 180 * pulse) : 0;
  const audioBlurPx =
    audio?.enabled && audio.target === 'blur' ? roundTo(audio.amount * 8 * pulse, 1) : 0;

  if (grade?.enabled) {
    const hasGamma = grade.gamma !== 1;
    const effBrightness = hasGamma
      ? roundTo(grade.brightness * grade.gamma ** 0.45, 3)
      : grade.brightness;
    const effContrast = hasGamma
      ? roundTo(grade.contrast * grade.gamma ** -0.2, 3)
      : grade.contrast;
    const effHue = grade.hue + audioHueDeg;

    parts.push(
      `brightness(${effBrightness}) contrast(${effContrast}) saturate(${grade.saturation}) hue-rotate(${effHue}deg)`,
    );
    if (grade.invert > 0) {
      parts.push(`invert(${grade.invert})`);
    }
    if (grade.sepia > 0) {
      parts.push(`sepia(${grade.sepia})`);
    }
    if (grade.grayscale > 0) {
      parts.push(`grayscale(${grade.grayscale})`);
    }
  } else if (audioHueDeg !== 0) {
    parts.push(`hue-rotate(${audioHueDeg}deg)`);
  }

  const totalBlur = roundTo((stylize?.enabled ? stylize.blur : 0) + audioBlurPx, 1);
  if (totalBlur > 0) {
    parts.push(`blur(${totalBlur}px)`);
  }

  if (stylize?.enabled && stylize.bloom > 0) {
    const px = Math.round(6 + stylize.bloom * 24);
    const alpha = roundTo(0.25 + stylize.bloom * 0.55, 2);
    parts.push(`drop-shadow(0 0 ${px}px rgba(255, 255, 255, ${alpha}))`);
  }

  if (audio?.enabled && audio.target === 'glow') {
    const px = Math.round((4 + audio.amount * 18) * pulse);
    parts.push(`drop-shadow(0 0 ${px}px rgba(196, 255, 61, 0.55))`);
  }

  return parts.length > 0 ? parts.join(' ') : 'none';
}
