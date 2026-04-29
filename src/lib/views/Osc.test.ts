// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W55 Osc view tests — exercise the W14 PR4 / W16 routes-table render
// (canonical / compatAlias / dir / type / range / family columns), the
// Adapter health-pill branches driven by the `osc_health_snapshot`
// stub IPC, the route filter narrowing + empty-state copy, the
// audio-var families list + reserved-prefixes hint, and the static
// fixture-driven panels (operator settings, live log, Ableton Link
// stub, hints).
//
// `@tauri-apps/api/core` is mocked (per the License.test.ts precedent
// in spirit; this file mocks `invoke` directly because Osc.svelte
// imports it inline). The `$effect`-driven snapshot fetch is
// re-targeted per test by adjusting the mock's resolved/rejected
// value before render. Assertions on the resolved-snapshot DOM use
// `findBy*` to wait for the post-microtask state update.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// `vi.mock` is hoisted above any non-hoisted top-level statements; the
// mock function itself must therefore live inside `vi.hoisted` so it
// can be referenced from both the factory and the per-test setup
// without tripping a temporal-dead-zone access on the hoisted module
// initialization.
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import Osc from './Osc.svelte';
import { OSC_AUDIO_VAR_FAMILIES, OSC_ROUTES } from '../data/osc-routes';

describe('Osc view', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    // Default: pending — the $effect promise never resolves, so the
    // "loading…" fallback stays on screen for tests that don't care
    // about adapter state.
    invokeMock.mockReturnValue(new Promise(() => {}));
  });

  afterEach(() => {
    cleanup();
  });

  it('renders the OSC headline + listening port + intro', () => {
    const { getByRole, getByText } = render(Osc);
    const heading = getByRole('heading', { level: 1 });
    expect(heading).toHaveTextContent(/OSC/);
    expect(heading).toHaveTextContent(/LISTENING/);
    expect(heading).toHaveTextContent(/Port 7000/);
    expect(getByText(/Wireframe-06 port from the bundle's/)).toBeInTheDocument();
  });

  it('renders the routes table headers in canonical 6-column order', () => {
    const { container } = render(Osc);
    const headers = Array.from(container.querySelectorAll('.osc-view__route-table thead th')).map(
      (th) => th.textContent?.trim(),
    );
    expect(headers).toEqual(['canonical', 'compat alias', 'dir', 'type', 'range', 'family']);
  });

  it('renders one row per OSC_ROUTES fixture entry plus a colspan-6 note row per route with a note', () => {
    const { container } = render(Osc);
    const rowCount = OSC_ROUTES.length;
    const noteCount = OSC_ROUTES.filter((r) => r.note).length;
    const allRows = container.querySelectorAll('.osc-view__route-table tbody tr');
    expect(allRows.length).toBe(rowCount + noteCount);
    const noteRows = container.querySelectorAll(
      '.osc-view__route-table tbody tr.osc-view__note-row',
    );
    expect(noteRows.length).toBe(noteCount);
    // Note rows span all six columns.
    noteRows.forEach((row) => {
      const td = row.querySelector('td');
      expect(td?.getAttribute('colspan')).toBe('6');
    });
  });

  it('count tally reads "N of N" with no filter active', () => {
    const { container } = render(Osc);
    const count = container.querySelector('.osc-view__count');
    expect(count?.textContent?.trim()).toBe(`${OSC_ROUTES.length} of ${OSC_ROUTES.length}`);
  });

  it('typing into the filter narrows the routes table by canonical / alias / family / range substring', async () => {
    const { container, getByLabelText } = render(Osc);
    const search = getByLabelText(/filter OSC routes/) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'audio' } });
    // Filter matches every route whose canonical / compatAlias / family / range
    // string contains "audio" (caseless). For the W14 fixture that's exactly
    // the two `family: 'audio'` routes.
    const expected = OSC_ROUTES.filter((r) =>
      [r.canonical, r.compatAlias ?? '', r.family, r.range]
        .join(' ')
        .toLowerCase()
        .includes('audio'),
    );
    expect(expected.length).toBe(2);
    const count = container.querySelector('.osc-view__count');
    expect(count?.textContent?.trim()).toBe(`${expected.length} of ${OSC_ROUTES.length}`);
  });

  it('renders the empty-state copy when the filter excludes every route', async () => {
    const { getByLabelText, getByText, container } = render(Osc);
    const search = getByLabelText(/filter OSC routes/) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'zzzz-no-route-zzzz' } });
    expect(getByText('No routes match the current filter.')).toBeInTheDocument();
    const count = container.querySelector('.osc-view__count');
    expect(count?.textContent?.trim()).toBe(`0 of ${OSC_ROUTES.length}`);
  });

  it('renders the audio-var families panel with default-on/off state per fixture', () => {
    const { container } = render(Osc);
    const items = container.querySelectorAll('.osc-view__families li');
    expect(items.length).toBe(OSC_AUDIO_VAR_FAMILIES.length);
    // First family ("LVL") is default-on → li.active and "on" state label.
    const first = items[0];
    expect(first).toBeDefined();
    expect(first?.classList.contains('active')).toBe(true);
    expect(first?.querySelector('.osc-view__family-id')?.textContent?.trim()).toBe('LVL');
    expect(first?.querySelector('.osc-view__family-state')?.textContent?.trim()).toBe('on');
    // "TIM" (index 3) is default-off → no .active and "off" state label.
    const tim = items[3];
    expect(tim).toBeDefined();
    expect(tim?.classList.contains('active')).toBe(false);
    expect(tim?.querySelector('.osc-view__family-state')?.textContent?.trim()).toBe('off');
  });

  it('reserved-prefixes hint enumerates the OSC-CONF-16 prefixes', () => {
    const { container } = render(Osc);
    const reserved = container.querySelector('.osc-view__hint--reserved');
    expect(reserved?.textContent).toContain('/data/*');
    expect(reserved?.textContent).toContain('/data/subscribe/*');
    expect(reserved?.textContent).toContain('OSC-CONF-16');
  });

  it('renders the Adapter pill with state="Healthy" once the IPC resolves', async () => {
    invokeMock.mockResolvedValue({
      state: 'Healthy',
      attempted_binds: 0,
      local_addr: null,
    });
    const { findByText, container } = render(Osc);
    // The pill's label is the literal state string.
    expect(await findByText('Healthy')).toBeInTheDocument();
    const pill = container.querySelector('.osc-view__health-pill');
    expect(pill?.getAttribute('data-state')).toBe('healthy');
    // local_addr=null → "unbound — Phase-3 wires bind()" placeholder.
    expect(container.textContent).toContain('unbound — Phase-3 wires bind()');
  });

  it('renders ErrorMessage and the Phase-3 hint when the snapshot IPC rejects', async () => {
    invokeMock.mockRejectedValue(new Error('osc_health_snapshot offline in browser-mode'));
    const { findByText, container } = render(Osc);
    expect(await findByText(/osc_health_snapshot offline in browser-mode/)).toBeInTheDocument();
    expect(container.textContent).toContain('IPC error');
    // The hint paragraph wraps across lines; tolerate intervening whitespace.
    expect(container.textContent ?? '').toMatch(/Real adapter\s+wiring lands in Phase-3/);
  });

  it('renders the Ableton Link stub panel with em-dash placeholders', () => {
    const { container } = render(Osc);
    const linkPanel = container.querySelector('.osc-view__link-panel');
    expect(linkPanel).not.toBeNull();
    expect(linkPanel?.textContent).toContain('Ableton Link');
    // Status / peers / tempo / quantum each render `—` (or `— BPM`/`— beats`).
    const dts = Array.from(linkPanel?.querySelectorAll('dt') ?? []).map((dt) =>
      dt.textContent?.trim(),
    );
    expect(dts).toEqual(['status', 'peers', 'tempo', 'quantum']);
  });

  it('hints panel includes the canonical hotkey rows with kbd glyphs', () => {
    const { container } = render(Osc);
    const hints = container.querySelector('.osc-view__hints');
    expect(hints).not.toBeNull();
    const kbds = Array.from(hints?.querySelectorAll('kbd') ?? []).map((k) => k.textContent?.trim());
    expect(kbds).toContain('?');
    expect(kbds).toContain('Space');
    expect(kbds).toContain('B');
  });
});
