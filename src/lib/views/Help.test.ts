// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 1d-α PR 1 — Help view scaffold tests.
//
// Help is a thin scaffold (sidebar + iframe) so the test surface is
// kept narrow: page-list rendering, page selection routing through the
// `helpStore`, and the iframe `src` resolution flow via mocked Tauri
// APIs. Real corpus content + dynamic enumeration + search land in
// PR 2+, with their own tests.
//
// Mocks lifted via `vi.hoisted` per the project's
// `feedback_vitest_invoke_mock_hoisted.md` memory — `vi.mock` factories
// run before module-scope variables initialise, so any mock fn the
// factory references must come from `vi.hoisted`.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { resolveResourceMock, convertFileSrcMock } = vi.hoisted(() => ({
  resolveResourceMock: vi.fn<(path: string) => Promise<string>>(),
  convertFileSrcMock: vi.fn<(path: string) => string>(),
}));

vi.mock('@tauri-apps/api/path', () => ({
  resolveResource: resolveResourceMock,
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: convertFileSrcMock,
}));

import Help from './Help.svelte';
import { HELP_PAGES, helpStore } from '../stores/helpStore.svelte';

describe('Help view', () => {
  beforeEach(() => {
    resolveResourceMock.mockReset();
    convertFileSrcMock.mockReset();
    resolveResourceMock.mockImplementation(async (path: string) => `/resolved/${path}`);
    convertFileSrcMock.mockImplementation((path: string) => `tauri-asset://${path}`);
    // Clear any persisted slug so the store falls back to the default.
    window.localStorage.removeItem('chromaticity:help:selected-page');
    helpStore.set('index');
  });

  afterEach(() => {
    cleanup();
  });

  it('renders the help title and every page-list entry from the catalog', () => {
    const { getByText, getByRole } = render(Help);
    expect(getByRole('heading', { name: 'Help' })).toBeInTheDocument();
    for (const page of HELP_PAGES) {
      expect(getByText(page.label)).toBeInTheDocument();
    }
  });

  it('marks the currently selected page with aria-current=page', () => {
    const { getByRole } = render(Help);
    const welcome = getByRole('button', { name: 'Welcome' });
    expect(welcome).toHaveAttribute('aria-current', 'page');
  });

  it('clicking a page-list entry routes the helpStore selection', async () => {
    const { getByRole } = render(Help);
    expect(helpStore.selectedSlug).toBe('index');
    const aboutBtn = getByRole('button', { name: 'About' });
    await fireEvent.click(aboutBtn);
    expect(helpStore.selectedSlug).toBe('about');
    expect(aboutBtn).toHaveAttribute('aria-current', 'page');
  });

  it('renders an iframe sourcing the resolved bundled HTML path', async () => {
    const { container } = render(Help);
    await waitFor(() => {
      expect(resolveResourceMock).toHaveBeenCalledWith('help/index.html');
    });
    await waitFor(() => {
      const iframe = container.querySelector('iframe.help__iframe');
      expect(iframe).not.toBeNull();
      expect(iframe?.getAttribute('src')).toBe('tauri-asset:///resolved/help/index.html');
    });
  });

  it('renders the missing-corpus placeholder when resolveResource rejects', async () => {
    resolveResourceMock.mockRejectedValueOnce(new Error('resource not found'));
    const { getByRole } = render(Help);
    // Initial render shows the "Loading Help…" status; once the
    // rejected promise settles, the same role region transitions to
    // the missing-corpus placeholder copy.
    await waitFor(() => {
      expect(getByRole('status')).toHaveTextContent(/Help corpus was not found/);
    });
    expect(getByRole('status')).toHaveTextContent(/mkdocs build/);
  });

  it('exports a HELP_PAGES catalog matching the wave 1d-alpha corpus pages', () => {
    expect(HELP_PAGES.map((p) => p.slug)).toEqual([
      'index',
      'perform-mode',
      'scene-workflow',
      'library-workflow',
      'audio-reactivity',
      'import-shadertoy',
      'hotkeys-reference',
      'osc-routes',
      'chroma-uniforms',
      'settings-reference',
      'about',
    ]);
    // Every htmlPath must point under the `help/` resource prefix so
    // it lines up with the per-page `bundle.resources` entries in
    // `app/shell/tauri.conf.json`.
    for (const page of HELP_PAGES) {
      expect(page.htmlPath.startsWith('help/')).toBe(true);
      expect(page.htmlPath.endsWith('.html')).toBe(true);
    }
  });
});
