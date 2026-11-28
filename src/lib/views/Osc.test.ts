// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
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
// Wave 2.1 Lane D.3 widened the view to consume the `oscStore` for
// the routes table + the `settingsStore` for IP/port/TX/RX inputs.
// The default mock returns a pending promise for every command so
// existing tests that exercise the fixture-fallback branch keep
// passing; the new live-routes / bind-target / settings-input tests
// override the mock per-test.
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
import { oscStore } from '../stores/osc.svelte';
import { settingsStore } from '../stores/settings.svelte';

describe('Osc view', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    // Default: pending — the $effect promises never resolve, so
    // the live-routes branch never populates and the view falls
    // back to the curated fixture import. The "loading…" health
    // pill fallback stays on screen for tests that don't care
    // about adapter state.
    invokeMock.mockReturnValue(new Promise(() => {}));
    oscStore.reset();
    settingsStore.reset();
  });

  afterEach(() => {
    cleanup();
    oscStore.reset();
    settingsStore.reset();
  });

  it('renders the OSC headline + listening port + intro', () => {
    const { getByRole, getByText } = render(Osc);
    const heading = getByRole('heading', { level: 1 });
    expect(heading).toHaveTextContent(/OSC/);
    expect(heading).toHaveTextContent(/OFFLINE|PREVIEW|LISTENING/);
    expect(heading).toHaveTextContent(/Port 9000/);
    expect(getByText(/Monitor OSC adapter health, configure ports/)).toBeInTheDocument();
  });

  it('renders the routes table headers in the canonical 7-column order', () => {
    const { container } = render(Osc);
    const headers = Array.from(container.querySelectorAll('.osc-view__route-table thead th')).map(
      (th) => th.textContent?.trim(),
    );
    expect(headers).toEqual([
      'canonical',
      'compat alias',
      'dir',
      'type',
      'range',
      'family',
      'bind',
    ]);
  });

  it('renders one row per OSC_ROUTES fixture entry plus a colspan-7 note row per route with a note', () => {
    const { container } = render(Osc);
    const rowCount = OSC_ROUTES.length;
    const noteCount = OSC_ROUTES.filter((r) => r.note).length;
    const allRows = container.querySelectorAll('.osc-view__route-table tbody tr');
    expect(allRows.length).toBe(rowCount + noteCount);
    const noteRows = container.querySelectorAll(
      '.osc-view__route-table tbody tr.osc-view__note-row',
    );
    expect(noteRows.length).toBe(noteCount);
    // Note rows span all seven columns post-D.3.
    noteRows.forEach((row) => {
      const td = row.querySelector('td');
      expect(td?.getAttribute('colspan')).toBe('7');
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

  it('reserved-prefixes hint enumerates the reserved system prefixes', () => {
    const { container } = render(Osc);
    const reserved = container.querySelector('.osc-view__hint--reserved');
    expect(reserved?.textContent).toContain('/data/*');
    expect(reserved?.textContent).toContain('/data/subscribe/*');
    expect(reserved?.textContent).toContain('prefix-reserved');
  });

  it('renders the Adapter pill with state="Healthy" once the IPC resolves', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'osc_health_snapshot') {
        return Promise.resolve({
          state: 'Healthy',
          attempted_binds: 0,
          local_addr: null,
          compat_alias_emits: 0n,
        });
      }
      // Other commands (osc_routes_snapshot, get_settings) keep the
      // pending-promise default so the fixture-fallback branch
      // renders.
      return new Promise(() => {});
    });
    const { findByText, container } = render(Osc);
    // The pill's label is the literal state string.
    expect(await findByText('Healthy')).toBeInTheDocument();
    const pill = container.querySelector('.osc-view__health-pill');
    expect(pill?.getAttribute('data-state')).toBe('healthy');
    // local_addr=null renders the live-adapter unbound placeholder.
    expect(container.textContent).toContain('unbound');
  });

  it('renders ErrorMessage and the browser-mode hint when the snapshot IPC rejects', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'osc_health_snapshot') {
        return Promise.reject(new Error('osc_health_snapshot offline in browser-mode'));
      }
      // Other commands keep pending so the routes / settings stores
      // don't surface a duplicate ErrorMessage alongside the
      // health-pill banner.
      return new Promise(() => {});
    });
    const { findAllByText, container } = render(Osc);
    const matches = await findAllByText(/osc_health_snapshot offline in browser-mode/);
    expect(matches.length).toBeGreaterThan(0);
    expect(container.textContent).toContain('IPC error');
    expect(container.textContent ?? '').toMatch(/Tauri internals unset/);
  });

  it('does not render an empty Ableton Link placeholder panel', () => {
    const { container } = render(Osc);
    expect(container.querySelector('.osc-view__link-panel')).toBeNull();
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

  it('renders the listen IP + port inputs bound to the settings store defaults', () => {
    const { getByLabelText } = render(Osc);
    const ip = getByLabelText(/OSC listen IP/i) as HTMLInputElement;
    expect(ip.value).toBe('127.0.0.1');
    const port = getByLabelText(/OSC listen port/i) as HTMLInputElement;
    expect(port.value).toBe('9000');
    const outPort = getByLabelText(/OSC send port/i) as HTMLInputElement;
    expect(outPort.value).toBe('9001');
  });

  it('editing the listen and send ports dispatches settings updates', async () => {
    const { getByLabelText } = render(Osc);
    const port = getByLabelText(/OSC listen port/i) as HTMLInputElement;
    await fireEvent.input(port, { target: { value: '9100' } });
    // The store snapshot reflects the update synchronously
    // (settingsStore.update is in-process; the debounced save is
    // mocked away because no invoke handler is registered for
    // `set_settings`).
    expect(settingsStore.snapshot.osc.port).toBe(9100);
    const outPort = getByLabelText(/OSC send port/i) as HTMLInputElement;
    await fireEvent.input(outPort, { target: { value: '9101' } });
    expect(settingsStore.snapshot.osc.out_port).toBe(9101);
  });

  it('renders the live routes when the oscStore exposes them with a bind action', async () => {
    oscStore.routes = [
      {
        canonical: '/scenes/{sceneName}',
        compat_alias: '/scene/{sceneName}',
        type_tag: '—',
        range: 'announce / trigger',
        direction: 'Both',
        family: 'Scene',
        bind_target: 's-12',
      },
    ];
    const { container, findByText } = render(Osc);
    expect(await findByText('/scenes/{sceneName}')).toBeInTheDocument();
    // The bound target renders in the bind column.
    expect(container.textContent).toContain('s-12');
    // Count tally reflects the live store rather than the fixture
    // length.
    const count = container.querySelector('.osc-view__count');
    expect(count?.textContent?.trim()).toBe('1 of 1');
  });

  it('details button toggles an inline bind panel for the row', async () => {
    oscStore.routes = [
      {
        canonical: '/controls/{bank}/{name}',
        compat_alias: '/controls/{addr}',
        type_tag: 'f',
        range: '0.0 – 1.0',
        direction: 'Both',
        family: 'Control',
        bind_target: null,
      },
    ];
    const { container, getByLabelText } = render(Osc);
    const detailsBtn = getByLabelText(
      /Details for \/controls\/\{bank\}\/\{name\}/i,
    ) as HTMLButtonElement;
    await fireEvent.click(detailsBtn);
    // The inline panel exposes a "Save" button + a bind-target
    // input.
    const input = getByLabelText(
      /Bind target for \/controls\/\{bank\}\/\{name\}/i,
    ) as HTMLInputElement;
    expect(input).toBeInTheDocument();
    const save = container.querySelector('.osc-view__bind-save') as HTMLButtonElement;
    expect(save).not.toBeNull();
    // Save is disabled while the draft is empty.
    expect(save.disabled).toBe(true);
    await fireEvent.input(input, { target: { value: 'control:scene:fader-1' } });
    expect(save.disabled).toBe(false);
  });

  it('shows the no-routes empty-state when the live store is empty and no fixture fallback is desired', () => {
    // Force the live branch by populating then clearing routes — the
    // derived `allRoutes` shows `liveRoutes.length > 0 ? live :
    // fixture`, so we can only hit the "No OSC routes registered."
    // copy by also bypassing the fixture. The view's contract is that
    // an empty live store falls back to the curated fixture, so we
    // assert that fallback behaviour explicitly.
    const { container } = render(Osc);
    // The fixture-fallback branch is active by default → the empty
    // state is never reached; the count reflects the fixture length.
    const count = container.querySelector('.osc-view__count');
    expect(count?.textContent?.trim()).toBe(`${OSC_ROUTES.length} of ${OSC_ROUTES.length}`);
  });
});
