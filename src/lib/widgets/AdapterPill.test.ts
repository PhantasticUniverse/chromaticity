// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W34a AdapterPill primitive tests — exercise the six bundle states +
// the kind/detail/onclick contract. Per-state glyph / label tables are
// validated against the bundle's `v02_supplement.jsx` lines 22-29
// values verbatim.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import AdapterPill from './AdapterPill.svelte';

describe('AdapterPill widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders kind + label + glyph for the ok state', () => {
    const { getByRole, container } = render(AdapterPill, {
      props: { kind: 'OUT', state: 'ok' },
    });
    const btn = getByRole('button');
    expect(btn).toHaveTextContent('OUT');
    expect(btn).toHaveTextContent('OK');
    expect(btn).toHaveAttribute('data-state', 'ok');
    const glyph = container.querySelector('.adapter-pill__glyph');
    expect(glyph?.textContent?.trim()).toBe('●');
  });

  it('maps each of the six states to the bundle-verbatim label and glyph', () => {
    const cases: Array<{
      state: 'off' | 'warming' | 'ok' | 'rate-limited' | 'disconnected' | 'blocked';
      label: string;
      glyph: string;
    }> = [
      { state: 'off', label: 'OFF', glyph: '○' },
      { state: 'warming', label: 'WARMING', glyph: '◔' },
      { state: 'ok', label: 'OK', glyph: '●' },
      { state: 'rate-limited', label: 'LIMIT', glyph: '△' },
      { state: 'disconnected', label: 'OFFLINE', glyph: '✕' },
      { state: 'blocked', label: 'BLOCKED', glyph: '⛉' },
    ];
    for (const c of cases) {
      cleanup();
      const { container, getByRole } = render(AdapterPill, {
        props: { kind: 'MIDI', state: c.state },
      });
      const btn = getByRole('button');
      expect(btn).toHaveAttribute('data-state', c.state);
      expect(btn).toHaveTextContent(c.label);
      expect(container.querySelector('.adapter-pill__glyph')?.textContent?.trim()).toBe(c.glyph);
    }
  });

  it('falls back the title to "<KIND> · <LABEL>" when detail is omitted', () => {
    const { getByRole } = render(AdapterPill, {
      props: { kind: 'OSC', state: 'warming' },
    });
    expect(getByRole('button')).toHaveAttribute('title', 'OSC · WARMING');
  });

  it('uses the supplied detail string as the title', () => {
    const { getByRole } = render(AdapterPill, {
      props: { kind: 'MIDI', state: 'ok', detail: 'LPX · 64 maps' },
    });
    expect(getByRole('button')).toHaveAttribute('title', 'LPX · 64 maps');
  });

  it('renders disabled when no onclick is supplied', () => {
    const { getByRole } = render(AdapterPill, {
      props: { kind: 'OUT', state: 'ok' },
    });
    expect(getByRole('button')).toBeDisabled();
  });

  it('fires onclick when supplied and the pill is clicked', async () => {
    const onclick = vi.fn();
    const { getByRole } = render(AdapterPill, {
      props: { kind: 'OUT', state: 'ok', onclick },
    });
    const btn = getByRole('button');
    expect(btn).not.toBeDisabled();
    await fireEvent.click(btn);
    expect(onclick).toHaveBeenCalledTimes(1);
  });

  it('aria-label includes detail when supplied', () => {
    const { getByRole } = render(AdapterPill, {
      props: { kind: 'OUT', state: 'ok', detail: 'DISP 2 · 1920×1080 @ 60' },
    });
    expect(getByRole('button')).toHaveAccessibleName('OUT adapter OK: DISP 2 · 1920×1080 @ 60');
  });
});
