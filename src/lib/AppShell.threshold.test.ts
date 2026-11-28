// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

// Shift+T sets the audio noise gate to the current input level
// (audit 2026-11-28: the hotkey was listed but did nothing).

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { setAudioThresholdMock } = vi.hoisted(() => ({
  setAudioThresholdMock: vi.fn(async (value: number) => value),
}));
vi.mock('./ipc/audio', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc/audio')>()),
  setAudioThreshold: setAudioThresholdMock,
}));

import AppShell from './AppShell.svelte';
import { audioSnapshotStore } from './stores/audioSnapshot.svelte';

describe('AppShell — Shift+T audio threshold hotkey', () => {
  beforeEach(() => {
    vi.stubGlobal('matchMedia', (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));
    setAudioThresholdMock.mockClear();
  });
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    audioSnapshotStore.snapshot = null;
  });

  it('sets the threshold to the current RMS on Shift+T only', async () => {
    audioSnapshotStore.snapshot = {
      bpm: 120,
      bpm_availability: 'Ready',
      rms: 0.23,
      peak: 0.5,
      fft_bands: [0, 0, 0, 0, 0, 0, 0, 0],
      fft_availability: 'Ready',
      degraded: false,
    };
    render(AppShell);
    await fireEvent.keyDown(window, { key: 't' });
    expect(setAudioThresholdMock).not.toHaveBeenCalled();
    await fireEvent.keyDown(window, { key: 'T', shiftKey: true });
    expect(setAudioThresholdMock).toHaveBeenCalledWith(0.23);
  });
});
