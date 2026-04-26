<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-21 PR2 Test Card preview surface. Static SVG calibration pattern
  // for output diagnostics — operators activate it via the Perform view's
  // MediaStrip alongside `none / webcam / display`. Mirrors the v0.2
  // bundle's `<Preview kind="bars" />` precedent (`shaders.jsx#bars`,
  // animated 32-band visualizer) re-interpreted as the classic NTSC SMPTE
  // operator pattern: 8 vertical color bars (full-luminance), audio
  // reference markers (1 kHz / 100 Hz), center cross-hair + circle for
  // geometry verification, corner resolution / FPS readout, and a 10%
  // safe-area frame.
  //
  // Locked W21 PR2 design choices:
  //   - Static SVG only — no canvas, no requestAnimationFrame, no
  //     dependency on the ADR-011 rendering backend. The test card is a
  //     calibration surface, not a live render.
  //   - Bar colors are bundle-canonical NTSC SMPTE values (literal hex);
  //     surrounding chrome (background, border, label text) uses the
  //     `--color-*` token cascade so the three theme cascades (Pulse /
  //     Atelier / Console) remain visible at the edges.
  //   - `role="img"` + `aria-label` on the SVG so screen readers announce
  //     the surface as a single decorative element, not 30+ shape leaves.
  //
  // Reusable across Perform / Author / Patch preview hosts; the host owns
  // the aspect-ratio container and any HUD overlays. This component just
  // paints inside whatever box it's given via `width="100%" height="100%"`.

  let { label = 'Test Card' }: { label?: string } = $props();

  // SMPTE color-bars top band (75% luminance traditional, 100% here for a
  // crisper static pattern). Bars are equal-width 1/8 columns.
  // Order: white, yellow, cyan, green, magenta, red, blue, black.
  const BARS = [
    '#ffffff',
    '#ffff00',
    '#00ffff',
    '#00ff00',
    '#ff00ff',
    '#ff0000',
    '#0000ff',
    '#000000',
  ] as const;

  // SVG canonical viewport — 1920×1080 mirrors the readout text and gives
  // 8 even 240px bar columns. Caller scales via CSS.
  const W = 1920;
  const H = 1080;
  const BAR_TOP_H = 720; // bars occupy top two thirds
  const BAR_W = W / BARS.length;

  // 10% safe-area inset (per side).
  const SAFE_INSET_X = W * 0.1;
  const SAFE_INSET_Y = H * 0.1;

  // Center geometry.
  const CX = W / 2;
  const CY = H / 2;
  const CIRCLE_R = H * 0.4; // large geometry circle
  const CROSS_R = 60; // center cross-hair half-length
</script>

<div class="test-card" role="img" aria-label="Test card pattern">
  <svg
    class="test-card__svg"
    viewBox="0 0 {W} {H}"
    preserveAspectRatio="xMidYMid meet"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <!-- 8 vertical SMPTE color bars (top band, full luminance) -->
    {#each BARS as color, i (i)}
      <rect x={i * BAR_W} y="0" width={BAR_W} height={BAR_TOP_H} fill={color} />
    {/each}

    <!-- Audio reference tone markers (bottom band, placeholder rectangles) -->
    <rect x="0" y={BAR_TOP_H} width={W * 0.5} height={H - BAR_TOP_H} fill="#222222" />
    <rect x={W * 0.5} y={BAR_TOP_H} width={W * 0.5} height={H - BAR_TOP_H} fill="#0c0c0c" />
    <text
      x={W * 0.25}
      y={BAR_TOP_H + (H - BAR_TOP_H) / 2 + 18}
      text-anchor="middle"
      font-family="monospace"
      font-size="56"
      font-weight="700"
      fill="#ffffff"
      letter-spacing="6"
    >
      1kHz
    </text>
    <text
      x={W * 0.75}
      y={BAR_TOP_H + (H - BAR_TOP_H) / 2 + 18}
      text-anchor="middle"
      font-family="monospace"
      font-size="56"
      font-weight="700"
      fill="#bbbbbb"
      letter-spacing="6"
    >
      100Hz
    </text>

    <!-- 10% safe-area frame -->
    <rect
      x={SAFE_INSET_X}
      y={SAFE_INSET_Y}
      width={W - SAFE_INSET_X * 2}
      height={H - SAFE_INSET_Y * 2}
      fill="none"
      stroke="#ffffff"
      stroke-width="2"
      stroke-dasharray="14 10"
      opacity="0.8"
    />

    <!-- Center geometry circle (verifies square pixels) -->
    <circle cx={CX} cy={CY} r={CIRCLE_R} fill="none" stroke="#ffffff" stroke-width="3" />

    <!-- Center cross-hair -->
    <line x1={CX - CROSS_R} y1={CY} x2={CX + CROSS_R} y2={CY} stroke="#ffffff" stroke-width="3" />
    <line x1={CX} y1={CY - CROSS_R} x2={CX} y2={CY + CROSS_R} stroke="#ffffff" stroke-width="3" />

    <!-- Corner readout (top-left): resolution / fps / version -->
    <g>
      <rect x="24" y="24" width="640" height="56" fill="rgba(0,0,0,0.7)" rx="4" />
      <text
        x="40"
        y="64"
        font-family="monospace"
        font-size="32"
        font-weight="700"
        fill="#ffffff"
        letter-spacing="2"
      >
        1920×1080 · 60 FPS · v0.0.0
      </text>
    </g>
  </svg>

  {#if label}
    <span class="test-card__label">{label}</span>
  {/if}
</div>

<style>
  .test-card {
    position: relative;
    width: 100%;
    height: 100%;
    background: #000000;
    overflow: hidden;
    display: block;
  }

  .test-card__svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .test-card__label {
    position: absolute;
    left: var(--space-sm);
    bottom: var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-0);
    background: color-mix(in oklab, var(--color-bg-0) 65%, transparent);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
  }
</style>
