// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import SceneFeedbackBar from './SceneFeedbackBar.svelte';
import { sceneFeedbackStore } from '../stores/sceneFeedback.svelte';

describe('SceneFeedbackBar widget', () => {
  beforeEach(() => {
    sceneFeedbackStore.reset();
  });

  afterEach(() => {
    cleanup();
    sceneFeedbackStore.reset();
    vi.restoreAllMocks();
  });

  it('renders 5 interactive star buttons and updates rating on click', async () => {
    const { getByRole } = render(SceneFeedbackBar, {
      props: { sceneId: 'scene-prism', sceneTitle: 'Prism' },
    });

    const star1 = getByRole('button', { name: 'Rate 1 star' });
    const star4 = getByRole('button', { name: 'Rate 4 stars' });
    const star5 = getByRole('button', { name: 'Rate 5 stars' });

    expect(star1).toHaveAttribute('aria-pressed', 'false');
    expect(star4).toHaveAttribute('aria-pressed', 'false');

    await fireEvent.click(star4);

    expect(sceneFeedbackStore.get('scene-prism')?.rating).toBe(4);
    expect(star1).toHaveAttribute('aria-pressed', 'true');
    expect(star4).toHaveAttribute('aria-pressed', 'true');
    expect(star5).toHaveAttribute('aria-pressed', 'false');

    // Clicking the currently selected star toggles rating back to 0.
    await fireEvent.click(star4);
    expect(sceneFeedbackStore.get('scene-prism')?.rating).toBe(0);
    expect(star4).toHaveAttribute('aria-pressed', 'false');
  });

  it('toggles the note panel, quick tags, comment textarea, and Copy Agent Digest', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });

    const { getByRole, getByLabelText, queryByRole } = render(SceneFeedbackBar, {
      props: { sceneId: 'scene-dunes', sceneTitle: 'Dunes', compact: true },
    });

    expect(queryByRole('button', { name: 'amazing' })).toBeNull();

    const noteBtn = getByRole('button', { name: /Note/i });
    expect(noteBtn).toHaveAttribute('aria-expanded', 'false');

    await fireEvent.click(noteBtn);
    expect(noteBtn).toHaveAttribute('aria-expanded', 'true');

    const tagBtn = getByRole('button', { name: 'favorite-optics' });
    await fireEvent.click(tagBtn);
    expect(tagBtn).toHaveAttribute('aria-pressed', 'true');
    expect(sceneFeedbackStore.get('scene-dunes')?.tags).toEqual(['favorite-optics']);

    const textarea = getByLabelText('Notes for shader iteration');
    await fireEvent.input(textarea, { target: { value: 'Sharper specular edge' } });
    expect(sceneFeedbackStore.get('scene-dunes')?.comment).toBe('Sharper specular edge');

    const copyBtn = getByRole('button', { name: /Copy Agent Digest/i });
    await fireEvent.click(copyBtn);
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0]?.[0]).toContain('scene-dunes');
    expect(writeText.mock.calls[0]?.[0]).toContain('favorite-optics');
    expect(writeText.mock.calls[0]?.[0]).toContain('Sharper specular edge');

    const clearBtn = getByRole('button', { name: 'Clear' });
    await fireEvent.click(clearBtn);
    expect(sceneFeedbackStore.get('scene-dunes')).toBeUndefined();
  });
});
