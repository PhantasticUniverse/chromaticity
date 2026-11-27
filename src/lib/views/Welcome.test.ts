// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W53 Welcome view tests — first of the view-track backfill wave.
// Welcome is the smallest of the routed views (`onboard.jsx` port);
// observable behavior is the hero text + three quick-start cards
// (Quick Start enabled / Import disabled / Empty Project enabled),
// the audio / MIDI / Link hint row, and the two CTA handlers that
// route via `routeStore` (Quick Start → library, Empty Project →
// perform).
//
// `routeStore` is a runes-backed singleton — both Welcome CTAs write
// to it, so `afterEach` resets it back to `'perform'` (the store's
// initial value) before `cleanup()` to keep cross-test state from
// leaking. `Icon.svelte` and the radial-blob background CSS are not
// re-tested here; the focus is the view-level contract.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Welcome from './Welcome.svelte';
import { routeStore } from '../stores/route.svelte';

describe('Welcome view', () => {
  afterEach(() => {
    routeStore.set('perform');
    cleanup();
  });

  it('renders the hero greeting, headline, and subtitle copy', () => {
    const { getByText } = render(Welcome);
    expect(getByText('Welcome to Chromaticity')).toBeInTheDocument();
    expect(getByText(/Make/)).toBeInTheDocument();
    expect(getByText('light')).toBeInTheDocument();
    expect(getByText(/A VJ engine built for live performance/)).toBeInTheDocument();
  });

  it('renders all three quick-start cards with their titles + bodies', () => {
    const { getByText } = render(Welcome);
    expect(getByText('Quick Start')).toBeInTheDocument();
    expect(getByText(/Load a demo set · webcam input · first scene ready/)).toBeInTheDocument();
    expect(getByText('Import Setlist')).toBeInTheDocument();
    expect(getByText(/Bring an existing show in as JSON or .synx/)).toBeInTheDocument();
    expect(getByText('Empty Project')).toBeInTheDocument();
    expect(getByText(/Start from a blank canvas with default scenes/)).toBeInTheDocument();
  });

  it('Quick Start card Launch button is enabled and routes to Library', async () => {
    const { getByRole } = render(Welcome);
    const launch = getByRole('button', { name: /^Launch$/ });
    expect(launch).not.toBeDisabled();
    expect(routeStore.current).toBe('perform');
    await fireEvent.click(launch);
    expect(routeStore.current).toBe('library');
  });

  it('Empty Project card Create button is enabled and routes to Perform', async () => {
    const { getByRole } = render(Welcome);
    // Move route off the default first so the click is observable
    // even though the target route happens to be `perform`.
    routeStore.set('library');
    const create = getByRole('button', { name: /^Create$/ });
    expect(create).not.toBeDisabled();
    await fireEvent.click(create);
    expect(routeStore.current).toBe('perform');
  });

  it('Import Setlist card CTA is enabled and routes to Import', async () => {
    const { getByRole } = render(Welcome);
    const importBtn = getByRole('button', { name: /Import…/ });
    expect(importBtn).not.toBeDisabled();
    await fireEvent.click(importBtn);
    expect(routeStore.current).toBe('import');
  });

  it('renders the audio / MIDI / Ableton Link hint row', () => {
    const { getByText } = render(Welcome);
    expect(getByText(/Audio input required/)).toBeInTheDocument();
    expect(getByText(/MIDI optional/)).toBeInTheDocument();
    expect(getByText(/Ableton Link supported/)).toBeInTheDocument();
  });

  it('renders the decorative blob background marked aria-hidden', () => {
    const { container } = render(Welcome);
    const bg = container.querySelector('.welcome__bg');
    expect(bg).not.toBeNull();
    expect(bg).toHaveAttribute('aria-hidden', 'true');
    // Five blobs (bass / mid / midhi / high / full) per the bundle JSX.
    const blobs = container.querySelectorAll('.welcome__blob');
    expect(blobs.length).toBe(5);
  });

  it('routeStore mutation is idempotent across repeated Quick Start clicks', async () => {
    const { getByRole } = render(Welcome);
    const launch = getByRole('button', { name: /^Launch$/ });
    await fireEvent.click(launch);
    expect(routeStore.current).toBe('library');
    routeStore.set('settings');
    await fireEvent.click(launch);
    expect(routeStore.current).toBe('library');
  });
});
