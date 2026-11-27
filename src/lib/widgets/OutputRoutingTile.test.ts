// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave-43 OutputRoutingTile widget tests — closes the W27 PR2 status-
// panel coverage gap. Verifies the per-platform Syphon (mac) / Spout
// (win) variant, the always-rendered cross-platform sibling row, the
// state-label palette (active / off / standby), and capability hints.
//
// `afterEach(cleanup)` per `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import OutputRoutingTile from './OutputRoutingTile.svelte';

describe('OutputRoutingTile widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the tile with region role + heading', () => {
    const { container } = render(OutputRoutingTile);
    const root = container.querySelector('.output-routing');
    expect(root).not.toBeNull();
    expect(root?.getAttribute('role')).toBe('region');
    expect(root?.getAttribute('aria-label')).toBe('Output routing');
  });

  it('renders 6 default rows on macOS (incl. Syphon + greyed Spout)', () => {
    const { container } = render(OutputRoutingTile, { props: { platform: 'mac' } });
    const rows = container.querySelectorAll('.output-routing__row');
    expect(rows.length).toBe(6);
    const names = Array.from(rows).map(
      (r) => r.querySelector('.output-routing__name')?.textContent,
    );
    expect(names).toContain('Primary display');
    expect(names).toContain('Secondary display');
    expect(names).toContain('Syphon — Chromaticity/Stage');
    expect(names).toContain('Spout');
    expect(names).toContain('NDI');
    expect(names).toContain('Record');
  });

  it('renders the Spout row + greyed Syphon sibling on Windows', () => {
    const { container } = render(OutputRoutingTile, { props: { platform: 'win' } });
    const names = Array.from(container.querySelectorAll('.output-routing__name')).map(
      (n) => n.textContent,
    );
    expect(names).toContain('Spout — Chromaticity');
    expect(names).toContain('Syphon');
  });

  it('shows the platform header label (macOS / Windows)', () => {
    const { getByText, unmount } = render(OutputRoutingTile, { props: { platform: 'mac' } });
    expect(getByText('macOS')).toBeInTheDocument();
    unmount();
    const winRender = render(OutputRoutingTile, { props: { platform: 'win' } });
    expect(winRender.getByText('Windows')).toBeInTheDocument();
  });

  it('emits the right state-label per row', () => {
    const { container } = render(OutputRoutingTile, { props: { platform: 'mac' } });
    const states = Array.from(container.querySelectorAll('.output-routing__state')).map((s) => ({
      text: s.textContent,
      data: s.getAttribute('data-state'),
    }));
    // Primary + Syphon are active; Secondary + cross-platform sibling are off; NDI + Record are standby.
    expect(states.filter((s) => s.data === 'active').length).toBe(2);
    expect(states.filter((s) => s.data === 'off').length).toBe(2);
    expect(states.filter((s) => s.data === 'standby').length).toBe(2);
  });

  it('records capability hints via data-cap on Syphon/NDI rows', () => {
    const { container } = render(OutputRoutingTile, { props: { platform: 'mac' } });
    const caps = Array.from(container.querySelectorAll('.output-routing__row')).map((r) =>
      r.getAttribute('data-cap'),
    );
    expect(caps).toContain('io.syphon.output');
    expect(caps).toContain('io.ndi.output');
  });

  it('honors a caller-provided rows fixture', () => {
    const { container } = render(OutputRoutingTile, {
      props: {
        platform: 'mac',
        rows: [
          {
            id: 'only',
            name: 'Custom output',
            state: 'active',
            resolution: '4K',
            fps: 30,
            icon: 'display',
          },
        ],
      },
    });
    const rows = container.querySelectorAll('.output-routing__row');
    expect(rows.length).toBe(1);
    expect(rows[0]?.querySelector('.output-routing__name')?.textContent).toBe('Custom output');
  });
});
