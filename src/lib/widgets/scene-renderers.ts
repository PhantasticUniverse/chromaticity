// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W36 procedural scene-thumbnail renderers, ported clean-room from
//   chromaticity-planning/ui-predev-bundle-v0.2-beta/shaders.jsx
// (the bundle's `RENDERERS` map + `renderStill` helper). Each `SceneKind`
// is a small canvas-2d routine keyed by (time, seed, width, height,
// audio-level) producing a recognizable still frame at thumbnail scale.
//
// Output is the static `renderStill` form used by Library / SnapBar /
// Playlist sidebar / Inspector preview tiles, AND the live-animated
// `SceneCanvas` form used by the Perform preview hero / Author live-
// preview tile.
//
// Renderer signature mirrors the bundle: `(ctx, t, seed, w, h, lvl)`
// with `lvl` ∈ [0, 1] (audio level proxy). For SceneStill, `lvl` is
// fixed at 0.5 and `t` defaults to `seed * 13.7` per the bundle
// `renderStill` body. The fix on `t` keeps thumbnails deterministic per
// seed across reloads.
//
// **Wave 2.3 Q.1 audio-reactivity widening.** The bundle source carries
// `lvl` in every renderer signature but only consumes it from
// `rings` / `grid` / `kaleido` / `noise` / `orbit` (5 of 16). The
// AO smoke-test of Wave 2.2 R.1 reported the `bars` SceneKind preview
// (`library/demo-spectrum.chroma`) as visually inert and asked that
// EVERY renderer pulse audibly so the preview pane confirms the live
// IPC seam end-to-end. This module widens the audio-coupling beyond
// bundle parity per the AO brief — each renderer now reads `lvl` for
// at least one visual axis (bar height, ring width, particle radius,
// hue jitter, etc.). The coupling pattern is
// `pulse = 0.6 + lvl * 0.4` so silent audio (`lvl ≈ 0` from
// MutexAudioPull's all-zero snapshot on a no-mic host) still
// renders 60% intensity — readable and recognisable, not invisible.
// The `(0.6 + lvl * 0.4)` envelope is documented at each call site.
//
// `Math.random()` appears only in the `glitch` renderer (per the bundle
// source); it is intentionally non-deterministic — `glitch` is meant to
// look chaotic. All other renderers are deterministic in (t, seed, lvl).

export type SceneKind =
  | 'rings'
  | 'fluid'
  | 'grid'
  | 'kaleido'
  | 'noise'
  | 'cells'
  | 'rain'
  | 'plasma'
  | 'orbit'
  | 'tunnel'
  | 'filaments'
  | 'glitch'
  | 'ink'
  | 'bars'
  | 'dunes'
  | 'lattice';

export const SCENE_KINDS: readonly SceneKind[] = [
  'rings',
  'fluid',
  'grid',
  'kaleido',
  'noise',
  'cells',
  'rain',
  'plasma',
  'orbit',
  'tunnel',
  'filaments',
  'glitch',
  'ink',
  'bars',
  'dunes',
  'lattice',
];

export type SceneRenderer = (
  ctx: CanvasRenderingContext2D,
  t: number,
  seed: number,
  w: number,
  h: number,
  lvl: number,
) => void;

/**
 * Wave 2.3 Q.1 audio-coupling envelope. Maps `lvl ∈ [0, 1]` to
 * `[0.6, 1.0]` so silent-audio renders at 60% intensity (readable) and
 * loud-audio at 100% (pulse). Used as a multiplier on the dominant
 * visual axis in each renderer (bar height, particle radius, etc.).
 */
function pulse(lvl: number): number {
  return 0.6 + Math.max(0, Math.min(1, lvl)) * 0.4;
}

const rings: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  const cx = w / 2;
  const cy = h / 2;
  const g = ctx.createRadialGradient(cx, cy, 10, cx, cy, Math.max(w, h));
  g.addColorStop(0, `hsl(${(t * 30 + seed * 40) % 360} 90% ${50 + lvl * 20}%)`);
  g.addColorStop(0.6, `hsl(${(t * 40 + seed * 70 + 180) % 360} 80% 20%)`);
  g.addColorStop(1, '#000');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  ctx.globalCompositeOperation = 'screen';
  for (let i = 0; i < 14; i++) {
    const r = (Math.sin(t * 0.5 + i * 0.4 + seed) * 0.5 + 0.5) * Math.min(w, h) * 0.45 + i * 6;
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.strokeStyle = `hsla(${(t * 70 + i * 30 + seed * 90) % 360},95%,70%,${0.35 - i * 0.02})`;
    ctx.lineWidth = 1.5 + lvl * 3;
    ctx.stroke();
  }
  ctx.globalCompositeOperation = 'source-over';
};

const fluid: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, w, h);
  ctx.globalCompositeOperation = 'screen';
  // Wave 2.3 Q.1 audio-coupling: blob radius pulses with `lvl` so
  // louder audio inflates the fluid blobs visibly.
  const p = pulse(lvl);
  for (let i = 0; i < 8; i++) {
    const x = w * (0.5 + 0.4 * Math.sin(t * 0.3 + i + seed));
    const y = h * (0.5 + 0.4 * Math.cos(t * 0.4 + i * 1.3 + seed));
    const r = Math.min(w, h) * (0.25 + 0.15 * Math.sin(t * 0.7 + i + seed)) * p;
    const g = ctx.createRadialGradient(x, y, 0, x, y, r);
    g.addColorStop(0, `hsla(${(t * 40 + i * 45 + seed * 30) % 360},90%,60%,0.6)`);
    g.addColorStop(1, 'transparent');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }
  ctx.globalCompositeOperation = 'source-over';
};

const grid: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = `hsl(${(seed * 30) % 360} 40% 8%)`;
  ctx.fillRect(0, 0, w, h);
  const cols = 16;
  const rows = 10;
  const cw = w / cols;
  const ch = h / rows;
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const n = Math.sin(x * 0.6 + y * 0.9 + t * 1.2 + seed) * 0.5 + 0.5;
      if (n > 0.6 - lvl * 0.2) {
        ctx.fillStyle = `hsl(${(t * 60 + x * 10 + y * 8 + seed * 40) % 360} 90% ${40 + n * 40}%)`;
        ctx.fillRect(x * cw + 1, y * ch + 1, cw - 2, ch - 2);
      }
    }
  }
};

const kaleido: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  const cx = w / 2;
  const cy = h / 2;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.fillStyle = '#000';
  ctx.fillRect(-w / 2, -h / 2, w, h);
  const slices = 6;
  for (let s = 0; s < slices; s++) {
    ctx.save();
    ctx.rotate((s / slices) * Math.PI * 2 + t * 0.2);
    const g = ctx.createLinearGradient(0, 0, w * 0.5, 0);
    g.addColorStop(0, `hsl(${(t * 50 + seed * 40 + s * 30) % 360} 90% 60%)`);
    g.addColorStop(1, 'transparent');
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(w * 0.5, -h * 0.1 - lvl * 20);
    ctx.lineTo(w * 0.5, h * 0.1 + lvl * 20);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  }
  ctx.restore();
};

const noise: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  const cell = 4;
  for (let y = 0; y < h; y += cell) {
    for (let x = 0; x < w; x += cell) {
      const n = Math.sin(x * 0.05 + seed + t * 2) * Math.cos(y * 0.05 + seed * 2 + t * 1.3);
      const hue = (n * 180 + t * 40 + seed * 20) % 360;
      ctx.fillStyle = `hsl(${hue} 80% ${30 + n * 30 + lvl * 10}%)`;
      ctx.fillRect(x, y, cell, cell);
    }
  }
};

const cells: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = '#050712';
  ctx.fillRect(0, 0, w, h);
  const pts = 14;
  const ps: Array<[number, number]> = [];
  for (let i = 0; i < pts; i++) {
    ps.push([
      w * (0.5 + 0.4 * Math.sin(t * 0.3 + i * 1.7 + seed)),
      h * (0.5 + 0.4 * Math.cos(t * 0.4 + i * 1.3 + seed * 2)),
    ]);
  }
  const step = 6;
  // Wave 2.3 Q.1 audio-coupling: pulse the per-cell lightness so the
  // voronoi cells brighten in time with audio.
  const lightnessBoost = lvl * 20;
  for (let y = 0; y < h; y += step) {
    for (let x = 0; x < w; x += step) {
      let d1 = 1e9;
      let d2 = 1e9;
      for (const [px, py] of ps) {
        const d = (px - x) ** 2 + (py - y) ** 2;
        if (d < d1) {
          d2 = d1;
          d1 = d;
        } else if (d < d2) {
          d2 = d;
        }
      }
      const v = Math.sqrt(d2) - Math.sqrt(d1);
      const hue = (v * 2 + t * 30 + seed * 30) % 360;
      ctx.fillStyle = `hsl(${hue} 85% ${30 + v * 0.3 + lightnessBoost}%)`;
      ctx.fillRect(x, y, step, step);
    }
  }
};

const rain: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = 'rgba(0,10,5,1)';
  ctx.fillRect(0, 0, w, h);
  const cols = 40;
  // Wave 2.3 Q.1 audio-coupling: trail length + per-glyph brightness
  // scale with `lvl` — quieter audio shows a sparse green rain,
  // louder audio thickens and brightens it.
  const trail = Math.round(12 + lvl * 16);
  const headBoost = lvl * 20;
  for (let c = 0; c < cols; c++) {
    const x = (c / cols) * w;
    const y0 = (t * 120 + seed * 40 + c * 71) % h;
    for (let k = 0; k < trail; k++) {
      const y = (y0 - k * 14 + h) % h;
      const alpha = 1 - k / trail;
      ctx.fillStyle = `hsla(${120 + seed * 20} 90% ${70 + headBoost - k * 2}% / ${alpha * 0.9})`;
      ctx.fillRect(x, y, w / cols - 1, 12);
    }
  }
};

const plasma: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  const step = 4;
  // Wave 2.3 Q.1 audio-coupling: brighten the plasma's lightness band
  // with `lvl` so silent audio renders muted, loud audio renders vivid.
  const lightnessBoost = lvl * 15;
  for (let y = 0; y < h; y += step) {
    for (let x = 0; x < w; x += step) {
      const v =
        Math.sin(x * 0.02 + t + seed) +
        Math.sin(y * 0.02 + t * 1.3) +
        Math.sin((x + y) * 0.015 + t * 0.7) +
        Math.sin(Math.sqrt(x * x + y * y) * 0.02 + t * 2);
      const hue = (v * 60 + t * 40 + seed * 30) % 360;
      ctx.fillStyle = `hsl(${(hue + 360) % 360} 90% ${40 + v * 10 + lightnessBoost}%)`;
      ctx.fillRect(x, y, step, step);
    }
  }
};

const orbit: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = 'rgba(0,0,0,0.25)';
  ctx.fillRect(0, 0, w, h);
  const cx = w / 2;
  const cy = h / 2;
  for (let i = 0; i < 50; i++) {
    const a = t * 0.5 + i * 0.3 + seed;
    const r = 20 + i * 5 + Math.sin(t + i) * 30;
    const x = cx + Math.cos(a) * r;
    const y = cy + Math.sin(a) * r;
    ctx.fillStyle = `hsl(${(i * 15 + t * 40 + seed * 50) % 360} 95% ${60 + lvl * 10}%)`;
    ctx.beginPath();
    ctx.arc(x, y, 2 + lvl * 3, 0, Math.PI * 2);
    ctx.fill();
  }
};

const tunnel: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  const cx = w / 2;
  const cy = h / 2;
  const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(w, h));
  g.addColorStop(0, `hsl(${(t * 60 + seed * 40) % 360} 90% 60%)`);
  g.addColorStop(1, '#000');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  ctx.strokeStyle = 'rgba(255,255,255,0.5)';
  // Wave 2.3 Q.1 audio-coupling: spoke and ring line-width pulses
  // with `lvl` — silent audio shows thin tunnel lines, loud audio
  // thickens them.
  const lineW = 1 + lvl * 2;
  for (let i = 0; i < 40; i++) {
    const a = (i / 40) * Math.PI * 2 + t * 0.3;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(a) * w, cy + Math.sin(a) * h);
    ctx.lineWidth = lineW;
    ctx.stroke();
  }
  for (let r = 20; r < Math.max(w, h); r += 40) {
    const rr = r + ((t * 60) % 40);
    ctx.beginPath();
    ctx.arc(cx, cy, rr, 0, Math.PI * 2);
    ctx.strokeStyle = `hsla(${(t * 50 + r + seed * 40) % 360} 90% 70% / ${0.4 + lvl * 0.3})`;
    ctx.lineWidth = 2 + lvl * 2;
    ctx.stroke();
  }
};

const filaments: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = 'rgba(0,0,0,0.15)';
  ctx.fillRect(0, 0, w, h);
  // Wave 2.3 Q.1 audio-coupling: filament line width + sine
  // amplitude scale with `lvl` so louder audio whips the filaments
  // wider.
  ctx.lineWidth = 1 + lvl * 2.5;
  const amp = 0.4 * pulse(lvl);
  for (let i = 0; i < 16; i++) {
    ctx.beginPath();
    for (let p = 0; p < 80; p++) {
      const ph = p / 80;
      const x = ph * w;
      const y =
        h / 2 +
        Math.sin(ph * 8 + t * 2 + i + seed) * h * amp * (1 - ph * 0.3) +
        Math.sin(ph * 20 + t + i * 2) * 10;
      if (p === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = `hsla(${(i * 20 + t * 50 + seed * 30) % 360} 90% 65% / 0.7)`;
    ctx.stroke();
  }
};

const glitch: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = '#050a0f';
  ctx.fillRect(0, 0, w, h);
  // Wave 2.3 Q.1 audio-coupling: number of glitch slices scales with
  // `lvl` so louder audio shows denser tearing.
  const slices = Math.round(20 + lvl * 30);
  for (let i = 0; i < slices; i++) {
    const y = Math.random() * h;
    const sh = 2 + Math.random() * h * 0.1;
    ctx.fillStyle = `hsla(${(t * 60 + i * 30 + seed * 40) % 360} 95% 60% / ${0.5 + Math.random() * 0.5})`;
    ctx.fillRect(Math.random() * w - w * 0.2, y, w * (0.4 + Math.random() * 0.8), sh);
  }
  ctx.fillStyle = `rgba(255,0,120,${0.1 + lvl * 0.15})`;
  for (let y = 0; y < h; y += 4) ctx.fillRect(0, y, w, 1);
};

const ink: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = '#0a0810';
  ctx.fillRect(0, 0, w, h);
  // Wave 2.3 Q.1 audio-coupling: ink-blob radius pulses with `lvl`.
  const p = pulse(lvl);
  for (let i = 0; i < 6; i++) {
    const x = w * (0.5 + 0.3 * Math.sin(t * 0.2 + i + seed));
    const y = h * (0.5 + 0.3 * Math.cos(t * 0.3 + i + seed));
    const r = (30 + i * 20 + Math.sin(t + i) * 20) * p;
    const g = ctx.createRadialGradient(x, y, 0, x, y, r);
    g.addColorStop(0, `hsla(${(i * 60 + seed * 30) % 360} 80% 40% / 0.8)`);
    g.addColorStop(0.7, `hsla(${(i * 60 + seed * 30 + 40) % 360} 70% 20% / 0.3)`);
    g.addColorStop(1, 'transparent');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }
};

const bars: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = '#050507';
  ctx.fillRect(0, 0, w, h);
  const n = 32;
  // Wave 2.3 Q.1 audio-coupling: bar heights pulse with `lvl` so the
  // spectrum widget visibly reacts. Pattern `(0.6 + lvl * 0.4)` keeps
  // silent audio at 60% bar height (readable) while loud audio
  // pushes to full extent.
  const p = pulse(lvl);
  for (let i = 0; i < n; i++) {
    const v = Math.abs(Math.sin(t * 2 + i * 0.4 + seed)) * (1 - (i / n) * 0.3);
    const bh = v * h * 0.95 * p;
    const x = (i / n) * w;
    const bw = w / n - 2;
    const g = ctx.createLinearGradient(0, h - bh, 0, h);
    g.addColorStop(0, `hsl(${(i * 10 + t * 30 + seed * 40) % 360} 95% 65%)`);
    g.addColorStop(1, `hsl(${(i * 10 + t * 30 + seed * 40 + 30) % 360} 95% 30%)`);
    ctx.fillStyle = g;
    ctx.fillRect(x, h - bh, bw, bh);
  }
};

const dunes: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, `hsl(${20 + seed * 10} 80% 30%)`);
  g.addColorStop(0.5, `hsl(${(seed * 15 + t * 10) % 360} 80% 45%)`);
  g.addColorStop(1, `hsl(${350 + seed * 5} 70% 10%)`);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  // Wave 2.3 Q.1 audio-coupling: dune crest amplitude scales with
  // `lvl` so louder audio shows bigger dunes rolling.
  const ampBoost = 1 + lvl * 0.8;
  for (let layer = 0; layer < 4; layer++) {
    ctx.beginPath();
    ctx.moveTo(0, h);
    for (let x = 0; x <= w; x += 6) {
      const y =
        h * 0.5 +
        layer * h * 0.1 +
        Math.sin(x * 0.01 + layer + seed + t * 0.3) * 20 * ampBoost +
        Math.sin(x * 0.04 + layer * 2) * 10;
      ctx.lineTo(x, y);
    }
    ctx.lineTo(w, h);
    ctx.closePath();
    ctx.fillStyle = `hsla(${20 - layer * 10 + seed * 5} 70% ${15 - layer * 3}% / 0.9)`;
    ctx.fill();
  }
};

const lattice: SceneRenderer = (ctx, t, seed, w, h, lvl) => {
  ctx.fillStyle = '#030508';
  ctx.fillRect(0, 0, w, h);
  const s = 20;
  const cx = w / 2;
  const cy = h / 2;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.rotate(Math.sin(t * 0.2 + seed) * 0.2);
  // Wave 2.3 Q.1 audio-coupling: per-node dot radius + lightness
  // jitter pulse with `lvl`.
  const dot = 3 + lvl * 3;
  const lightnessBoost = lvl * 15;
  for (let y = -h; y < h; y += s) {
    for (let x = -w; x < w; x += s) {
      const d = Math.hypot(x, y);
      const v = Math.sin(d * 0.03 - t * 2 + seed);
      const hue = (v * 60 + t * 30 + seed * 40) % 360;
      ctx.fillStyle = `hsla(${hue} 85% ${40 + v * 20 + lightnessBoost}% / ${0.8 - d / Math.max(w, h)})`;
      ctx.fillRect(x - dot / 2, y - dot / 2, dot, dot);
      if (Math.abs(x) < w / 2 && Math.abs(y) < h / 2) {
        ctx.strokeStyle = `hsla(${hue} 85% 50% / 0.2)`;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x + s, y);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x, y + s);
        ctx.stroke();
      }
    }
  }
  ctx.restore();
};

export const RENDERERS: Readonly<Record<SceneKind, SceneRenderer>> = {
  rings,
  fluid,
  grid,
  kaleido,
  noise,
  cells,
  rain,
  plasma,
  orbit,
  tunnel,
  filaments,
  glitch,
  ink,
  bars,
  dunes,
  lattice,
};

/**
 * Draw a single deterministic still frame for a scene thumbnail.
 *
 * Per the bundle's `renderStill` body: fixed audio level (0.5), default
 * `t = seed * 13.7` (overridable), and a vignette post-pass for cinematic
 * thumbnail framing. Unknown `kind` falls back to `rings` mirroring the
 * bundle's `RENDERERS[kind] || RENDERERS.rings` dispatch.
 */
export function renderStill(
  canvas: HTMLCanvasElement,
  kind: SceneKind,
  seed: number,
  tOverride?: number,
): void {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  const r = RENDERERS[kind] ?? RENDERERS.rings;
  const t = tOverride ?? seed * 13.7;
  r(ctx, t, seed, w, h, 0.5);
  const vg = ctx.createRadialGradient(
    w / 2,
    h / 2,
    Math.min(w, h) * 0.3,
    w / 2,
    h / 2,
    Math.max(w, h) * 0.7,
  );
  vg.addColorStop(0, 'transparent');
  // Wave 2.3 Q.1: soften the vignette from 0.7 → 0.55 so wide preview
  // panes (~1000×600 in the Perform hero) don't darken edges into
  // unreadability. The bundle's intent is a subtle CRT vignette, not
  // a heavy darken.
  vg.addColorStop(1, 'rgba(0,0,0,0.55)');
  ctx.fillStyle = vg;
  ctx.fillRect(0, 0, w, h);
}
