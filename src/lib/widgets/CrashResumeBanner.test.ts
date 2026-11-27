// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W44 CrashResumeBanner widget tests. The visual port is display-only
// (live `CrashResumeState` IPC arrives in Phase-3+/4a), so tests verify
// the prop → render contract: showName + p50 surface in the message,
// the five status rows render with the right per-status chip class,
// and the three callbacks fire on click.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import CrashResumeBanner from './CrashResumeBanner.svelte';

const BASE_PROPS = {
  showName: 'Warehouse · Saturday',
  controls: 'restored' as const,
  snaps: 'restored' as const,
  playlist: 'restored' as const,
  adapters: 'partial' as const,
  scene: 'restored' as const,
  p50: 480,
};

describe('CrashResumeBanner widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the showName fixture inside the message', () => {
    const { getByText } = render(CrashResumeBanner, { props: BASE_PROPS });
    expect(getByText('Warehouse · Saturday')).toBeInTheDocument();
  });

  it('surfaces the p50 latency in the message', () => {
    const { getByText } = render(CrashResumeBanner, { props: BASE_PROPS });
    expect(getByText('480 ms')).toBeInTheDocument();
  });

  it('renders five status chips with per-status class modifiers', () => {
    const { container } = render(CrashResumeBanner, { props: BASE_PROPS });
    const chips = container.querySelectorAll('.crash-resume__chip');
    expect(chips).toHaveLength(5);
    expect(chips[0]?.classList.contains('crash-resume__chip--restored')).toBe(true);
    expect(chips[1]?.classList.contains('crash-resume__chip--restored')).toBe(true);
    expect(chips[2]?.classList.contains('crash-resume__chip--restored')).toBe(true);
    expect(chips[3]?.classList.contains('crash-resume__chip--partial')).toBe(true);
    expect(chips[4]?.classList.contains('crash-resume__chip--restored')).toBe(true);
  });

  it('exposes role=status and aria-live=polite for assistive tech', () => {
    const { container } = render(CrashResumeBanner, { props: BASE_PROPS });
    const root = container.querySelector('.crash-resume');
    expect(root?.getAttribute('role')).toBe('status');
    expect(root?.getAttribute('aria-live')).toBe('polite');
  });

  it('fires onViewLog when the View log button is clicked', async () => {
    const onViewLog = vi.fn();
    const { getByText } = render(CrashResumeBanner, { props: { ...BASE_PROPS, onViewLog } });
    await fireEvent.click(getByText('View log'));
    expect(onViewLog).toHaveBeenCalledTimes(1);
  });

  it('fires onSupport when the Support bundle button is clicked', async () => {
    const onSupport = vi.fn();
    const { getByText } = render(CrashResumeBanner, { props: { ...BASE_PROPS, onSupport } });
    await fireEvent.click(getByText('Support bundle'));
    expect(onSupport).toHaveBeenCalledTimes(1);
  });

  it('fires onDismiss when the dismiss button is clicked', async () => {
    const onDismiss = vi.fn();
    const { getByLabelText } = render(CrashResumeBanner, {
      props: { ...BASE_PROPS, onDismiss },
    });
    await fireEvent.click(getByLabelText('Dismiss'));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it('omits the dismiss button when onDismiss is not provided', () => {
    const { queryByLabelText } = render(CrashResumeBanner, { props: BASE_PROPS });
    expect(queryByLabelText('Dismiss')).toBeNull();
  });
});
