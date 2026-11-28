// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
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

  it('renders bundled markdown documentation when resolveResource rejects', async () => {
    resolveResourceMock.mockRejectedValueOnce(new Error('resource not found'));
    const { container } = render(Help);
    await waitFor(() => {
      const article = container.querySelector('article.help__doc');
      expect(article).not.toBeNull();
      expect(article?.textContent?.length).toBeGreaterThan(20);
    });
  });

  it('exports a HELP_PAGES catalog whose htmlPaths point under the `help/` bundle prefix', () => {
    // Drift pin on the catalog slug order + length lives in
    // `src/lib/stores/helpStore.test.ts`; the Help view test only
    // verifies the htmlPath shape contract every page must satisfy
    // for `app/shell/tauri.conf.json` `bundle.resources` to resolve.
    for (const page of HELP_PAGES) {
      expect(page.htmlPath.startsWith('help/')).toBe(true);
      expect(page.htmlPath.endsWith('.html')).toBe(true);
    }
  });
});
