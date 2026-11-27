// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W33a WatermarkOverlay primitive tests — exercises the visible-prop
// gate and the aria-hidden invariant (the watermark is decorative
// chrome; AT should not announce it).
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import WatermarkOverlay from './WatermarkOverlay.svelte';

describe('WatermarkOverlay widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the wordmark + glyph when visible (default)', () => {
    const { container, getByText } = render(WatermarkOverlay);
    expect(getByText('Chromaticity')).toBeInTheDocument();
    expect(container.querySelector('.watermark svg circle')).not.toBeNull();
  });

  it('suppresses render entirely when visible={false}', () => {
    const { container, queryByText } = render(WatermarkOverlay, { props: { visible: false } });
    expect(queryByText('Chromaticity')).toBeNull();
    expect(container.querySelector('.watermark')).toBeNull();
  });

  it('marks the overlay aria-hidden so AT does not announce decorative chrome', () => {
    const { container } = render(WatermarkOverlay, { props: { visible: true } });
    const overlay = container.querySelector('.watermark');
    expect(overlay).not.toBeNull();
    expect(overlay).toHaveAttribute('aria-hidden', 'true');
  });
});
