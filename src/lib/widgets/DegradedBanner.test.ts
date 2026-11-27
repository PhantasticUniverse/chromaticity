// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-43 DegradedBanner widget tests — closes the W28 PR1 banner
// coverage gap. Verifies severity → ARIA role/aria-live mapping,
// optional action / dismiss buttons, and callback wiring.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import DegradedBanner from './DegradedBanner.svelte';

describe('DegradedBanner widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('defaults to warning severity (role=status, aria-live=polite)', () => {
    const { container } = render(DegradedBanner, { props: { message: 'GPU swap pending' } });
    const root = container.querySelector('.degraded-banner');
    expect(root?.getAttribute('role')).toBe('status');
    expect(root?.getAttribute('aria-live')).toBe('polite');
    expect(root?.classList.contains('degraded-banner--warning')).toBe(true);
  });

  it('uses role=alert + aria-live=assertive for error severity', () => {
    const { container } = render(DegradedBanner, {
      props: { severity: 'error', message: 'Adapter offline' },
    });
    const root = container.querySelector('.degraded-banner');
    expect(root?.getAttribute('role')).toBe('alert');
    expect(root?.getAttribute('aria-live')).toBe('assertive');
    expect(root?.classList.contains('degraded-banner--error')).toBe(true);
  });

  it('renders the message text', () => {
    const { getByText } = render(DegradedBanner, { props: { message: 'Integrated GPU detected' } });
    expect(getByText('Integrated GPU detected')).toBeInTheDocument();
  });

  it('omits the action button when actionLabel is not set', () => {
    const { container } = render(DegradedBanner, { props: { message: 'no action' } });
    expect(container.querySelector('.degraded-banner__action')).toBeNull();
  });

  it('renders the action button when actionLabel is provided', () => {
    const onAction = vi.fn();
    const { getByText } = render(DegradedBanner, {
      props: { message: 'click me', actionLabel: 'Retry', onAction },
    });
    expect(getByText('Retry')).toBeInTheDocument();
  });

  it('fires onAction when the action button is clicked', async () => {
    const onAction = vi.fn();
    const { getByText } = render(DegradedBanner, {
      props: { message: 'click me', actionLabel: 'Retry', onAction },
    });
    await fireEvent.click(getByText('Retry'));
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it('omits the dismiss button when onDismiss is not set', () => {
    const { container } = render(DegradedBanner, { props: { message: 'no dismiss' } });
    expect(container.querySelector('.degraded-banner__dismiss')).toBeNull();
  });

  it('fires onDismiss when the dismiss button is clicked', async () => {
    const onDismiss = vi.fn();
    const { container } = render(DegradedBanner, {
      props: { message: 'with dismiss', onDismiss },
    });
    const dismissBtn = container.querySelector('.degraded-banner__dismiss');
    expect(dismissBtn).not.toBeNull();
    await fireEvent.click(dismissBtn as Element);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
