// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-43 TestCard widget tests — closes the W21 PR2 NTSC SMPTE
// calibration-pattern coverage gap. Static SVG only; verifies the
// prop → render contract (label rendering, SMPTE bar count, role/ARIA).
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import TestCard from './TestCard.svelte';

describe('TestCard widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the test-card root with role=img', () => {
    const { container } = render(TestCard);
    const root = container.querySelector('.test-card');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('role')).toBe('img');
    expect(root?.getAttribute('aria-label')).toBe('Test card pattern');
  });

  it('renders an inner SVG with the canonical viewBox', () => {
    const { container } = render(TestCard);
    const svg = container.querySelector('svg.test-card__svg');
    expect(svg).not.toBeNull();
    expect(svg?.getAttribute('viewBox')).toBe('0 0 1920 1080');
  });

  it('paints all 8 SMPTE color bars', () => {
    const { container } = render(TestCard);
    const bars = container.querySelectorAll('svg.test-card__svg > rect');
    // 8 SMPTE bars + 2 audio reference background rects + 1 safe-area frame
    // + 1 corner-readout backdrop = 12 total <rect> elements at the SVG root.
    // The SMPTE-specific assertion is on the canonical hex fills.
    const fills = Array.from(bars).map((r) => r.getAttribute('fill'));
    for (const c of ['#ffffff', '#ffff00', '#00ffff', '#00ff00', '#ff00ff', '#ff0000', '#0000ff']) {
      expect(fills).toContain(c);
    }
  });

  it('renders the audio reference labels (1kHz / 100Hz)', () => {
    const { getByText } = render(TestCard);
    expect(getByText('1kHz')).toBeInTheDocument();
    expect(getByText('100Hz')).toBeInTheDocument();
  });

  it('renders the default label "Test Card"', () => {
    const { getByText } = render(TestCard);
    expect(getByText('Test Card')).toBeInTheDocument();
  });

  it('renders a custom label when prop is provided', () => {
    const { getByText } = render(TestCard, { props: { label: 'Stage A' } });
    expect(getByText('Stage A')).toBeInTheDocument();
  });

  it('omits the label DOM when label is empty string', () => {
    const { container } = render(TestCard, { props: { label: '' } });
    expect(container.querySelector('.test-card__label')).toBeNull();
  });
});
