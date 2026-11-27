// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const { invokeMock, isTauriMock } = vi.hoisted(() => ({
  invokeMock: vi.fn<(cmd: string, args?: unknown) => Promise<unknown>>(),
  isTauriMock: vi.fn<() => boolean>(() => false),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

import { inspectAllStateMachines } from '../state';
import {
  SCENE_FEEDBACK_STORAGE_KEY,
  sceneFeedbackStore,
  type SceneFeedbackEntry,
} from './sceneFeedback.svelte';

describe('sceneFeedbackStore', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
    sceneFeedbackStore.reset();
  });

  afterEach(() => {
    sceneFeedbackStore.reset();
  });

  test('sets, clamps, updates, and clears ratings while persisting to localStorage', () => {
    const entry = sceneFeedbackStore.setRating('kaleido-prism', 5, 'Kaleido Prism');
    expect(entry.rating).toBe(5);
    expect(entry.presetName).toBe('Kaleido Prism');
    expect(sceneFeedbackStore.get('kaleido-prism')?.rating).toBe(5);
    expect(localStorage.getItem(SCENE_FEEDBACK_STORAGE_KEY)).toContain('kaleido-prism');

    sceneFeedbackStore.setRating('kaleido-prism', 99);
    expect(sceneFeedbackStore.get('kaleido-prism')?.rating).toBe(5);

    sceneFeedbackStore.setRating('kaleido-prism', -3);
    expect(sceneFeedbackStore.get('kaleido-prism')?.rating).toBe(0);

    sceneFeedbackStore.clear('kaleido-prism');
    expect(sceneFeedbackStore.get('kaleido-prism')).toBeUndefined();
    expect(sceneFeedbackStore.allEntries()).toHaveLength(0);
  });

  test('toggles tags in canonical order and updates comments without losing rating', () => {
    sceneFeedbackStore.setRating('plasma-wave', 4);
    sceneFeedbackStore.toggleTag('plasma-wave', 'too-fast');
    sceneFeedbackStore.toggleTag('plasma-wave', 'amazing');
    expect(sceneFeedbackStore.get('plasma-wave')?.tags).toEqual(['amazing', 'too-fast']);

    sceneFeedbackStore.setComment('plasma-wave', 'Slow down radial drift by 20%.');
    const updated = sceneFeedbackStore.get('plasma-wave');
    expect(updated?.rating).toBe(4);
    expect(updated?.comment).toBe('Slow down radial drift by 20%.');

    sceneFeedbackStore.toggleTag('plasma-wave', 'too-fast');
    expect(sceneFeedbackStore.get('plasma-wave')?.tags).toEqual(['amazing']);
  });

  test('exports a deterministic Markdown Agent Feedback Digest grouped by priority', () => {
    expect(sceneFeedbackStore.exportAgentDigest()).toContain('_No scene feedback recorded yet._');

    sceneFeedbackStore.setRating('zeta-optics', 5, 'Zeta Optics');
    sceneFeedbackStore.toggleTag('zeta-optics', 'favorite-optics');
    sceneFeedbackStore.toggleTag('zeta-optics', 'smooth');

    sceneFeedbackStore.setRating('alpha-dim', 2, 'Alpha Dim');
    sceneFeedbackStore.toggleTag('alpha-dim', 'too-dim');
    sceneFeedbackStore.toggleTag('alpha-dim', 'needs-contrast');
    sceneFeedbackStore.setComment('alpha-dim', 'Boost highlights and midtone gain.');

    sceneFeedbackStore.setComment('beta-notes', 'Try subtle chromatic aberration.');

    const digestA = sceneFeedbackStore.exportAgentDigest();
    const digestB = sceneFeedbackStore.exportAgentDigest();
    expect(digestA).toBe(digestB);

    expect(digestA).toContain('# Chromaticity Scene Feedback Digest');
    expect(digestA).toContain('## Needs Attention');
    expect(digestA).toContain('### `alpha-dim` — Alpha Dim');
    expect(digestA).toContain('★★☆☆☆ (2/5)');
    expect(digestA).toContain('`needs-contrast`, `too-dim`');
    expect(digestA).toContain('Boost highlights and midtone gain.');

    expect(digestA).toContain('## Highlights');
    expect(digestA).toContain('### `zeta-optics` — Zeta Optics');
    expect(digestA).toContain('★★★★★ (5/5)');

    expect(digestA).toContain('## Notes Only');
    expect(digestA).toContain('### `beta-notes`');
    expect(digestA).toContain('Try subtle chromatic aberration.');
  });

  test('invokes save_scene_feedback and load_scene_feedback when isTauri() is true', async () => {
    isTauriMock.mockReturnValue(true);

    sceneFeedbackStore.setRating('nebula', 4);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'save_scene_feedback',
        expect.objectContaining({
          entries: expect.arrayContaining([
            expect.objectContaining({ sceneId: 'nebula', rating: 4 }),
          ]),
        }),
      );
    });

    const diskEntries: SceneFeedbackEntry[] = [
      {
        sceneId: 'from-disk',
        rating: 5,
        tags: ['amazing'],
        comment: 'Loaded from app_data_dir',
        updatedAt: '2026-11-27T05:00:00.000Z',
      },
    ];
    invokeMock.mockResolvedValueOnce(diskEntries);

    await sceneFeedbackStore.hydrateFromDisk();
    expect(invokeMock).toHaveBeenCalledWith('load_scene_feedback', undefined);
    expect(sceneFeedbackStore.get('from-disk')?.rating).toBe(5);
    expect(sceneFeedbackStore.get('from-disk')?.comment).toBe('Loaded from app_data_dir');
  });

  test('registers sceneFeedback diagnostics in the state-machine registry', () => {
    sceneFeedbackStore.setRating('diag-scene', 5);
    sceneFeedbackStore.toggleTag('diag-scene', 'smooth');
    sceneFeedbackStore.setComment('diag-scene', 'Crisp 60fps');

    const diagnostics = inspectAllStateMachines();
    expect(diagnostics.sceneFeedback).toEqual({
      entryCount: 1,
      ratedCount: 1,
      taggedCount: 1,
      commentedCount: 1,
    });
  });
});
