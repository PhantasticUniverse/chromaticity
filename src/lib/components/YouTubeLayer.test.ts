// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import YouTubeLayer from './YouTubeLayer.svelte';
import { youtubeLayerStore } from '../stores/youtubeLayer.svelte';

const SAMPLE_ID = 'dQw4w9WgXcQ';

describe('YouTubeLayer component', () => {
  beforeEach(() => {
    youtubeLayerStore.reset();
  });

  afterEach(() => {
    cleanup();
    youtubeLayerStore.reset();
  });

  it('renders the paste bar with Load and disabled Clear buttons initially', () => {
    const { getByPlaceholderText, getByRole, queryByTitle } = render(YouTubeLayer);

    const input = getByPlaceholderText('Paste YouTube URL or ID…');
    expect(input).toBeInTheDocument();
    expect(getByRole('button', { name: 'Load' })).toBeEnabled();
    expect(getByRole('button', { name: 'Clear' })).toBeDisabled();
    expect(queryByTitle(/YouTube visual layer/)).toBeNull();
  });

  it('shows an inline validation alert for invalid YouTube URLs', async () => {
    const { getByPlaceholderText, getByRole } = render(YouTubeLayer);

    const input = getByPlaceholderText('Paste YouTube URL or ID…');
    await fireEvent.input(input, { target: { value: 'https://evil.example/watch?v=123' } });
    await fireEvent.click(getByRole('button', { name: 'Load' }));

    const alert = getByRole('alert');
    expect(alert).toHaveTextContent(/Invalid YouTube URL or video ID/i);
    expect(input).toHaveAttribute('aria-invalid', 'true');
  });

  it('loads a valid YouTube URL into a muted, sandboxed nocookie embed iframe', async () => {
    const { getByPlaceholderText, getByRole, getByTitle } = render(YouTubeLayer);

    const input = getByPlaceholderText('Paste YouTube URL or ID…');
    await fireEvent.input(input, {
      target: { value: `https://www.youtube.com/watch?v=${SAMPLE_ID}` },
    });
    await fireEvent.click(getByRole('button', { name: 'Load' }));

    const iframe = getByTitle(`YouTube visual layer (${SAMPLE_ID})`) as HTMLIFrameElement;
    expect(iframe).toBeInTheDocument();
    expect(iframe.src).toContain(
      `https://www.youtube.com/embed/${SAMPLE_ID}?autoplay=1&mute=1&loop=1&playlist=${SAMPLE_ID}&controls=1&rel=0&playsinline=1&enablejsapi=1`,
    );
    expect(iframe).toHaveAttribute(
      'allow',
      'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture',
    );
    expect(iframe).toHaveAttribute('referrerpolicy', 'origin');
    expect(iframe.style.opacity).toBe('0.85');
    expect(iframe.style.mixBlendMode).toBe('screen');

    const muteBtn = getByRole('button', { name: 'Muted (DJ Safe)' });
    expect(muteBtn).toHaveAttribute('aria-pressed', 'false');

    const routeBtn = getByRole('button', { name: 'Route Audio to Visuals' });
    expect(routeBtn).toHaveAttribute('aria-pressed', 'false');
  });

  it('toggles audio mute, route audio, blend mode, and opacity with ASCII slider value', async () => {
    const { getByLabelText, getByPlaceholderText, getByRole, getByTitle } = render(YouTubeLayer);

    await fireEvent.input(getByPlaceholderText('Paste YouTube URL or ID…'), {
      target: { value: `https://youtu.be/${SAMPLE_ID}` },
    });
    await fireEvent.click(getByRole('button', { name: 'Load' }));

    // Toggle audio ON
    await fireEvent.click(getByRole('button', { name: 'Muted (DJ Safe)' }));
    const audioOnBtn = getByRole('button', { name: 'Audio On' });
    expect(audioOnBtn).toHaveAttribute('aria-pressed', 'true');
    const iframe = getByTitle(`YouTube visual layer (${SAMPLE_ID})`) as HTMLIFrameElement;
    expect(iframe.src).toContain('mute=0');

    // Opt-in Route Audio to Visuals
    const routeBtn = getByRole('button', { name: 'Route Audio to Visuals' });
    await fireEvent.click(routeBtn);
    expect(routeBtn).toHaveAttribute('aria-pressed', 'true');

    // Change Blend Mode
    const blendSelect = getByLabelText('Blend Mode') as HTMLSelectElement;
    await fireEvent.change(blendSelect, { target: { value: 'multiply' } });
    expect(blendSelect.value).toBe('multiply');
    expect(iframe.style.mixBlendMode).toBe('multiply');

    // Adjust Opacity slider (strict ASCII numeric .value)
    const opacitySlider = getByLabelText('Opacity') as HTMLInputElement;
    expect(opacitySlider.value).toBe('0.85');
    await fireEvent.input(opacitySlider, { target: { value: '0.45' } });
    expect(opacitySlider.value).toBe('0.45');
    expect(iframe.style.opacity).toBe('0.45');
  });

  it('clears the active stream and resets the input cleanly', async () => {
    const { getByPlaceholderText, getByRole, queryByTitle } = render(YouTubeLayer);

    const input = getByPlaceholderText('Paste YouTube URL or ID…') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: SAMPLE_ID } });
    await fireEvent.click(getByRole('button', { name: 'Load' }));
    expect(queryByTitle(`YouTube visual layer (${SAMPLE_ID})`)).not.toBeNull();

    await fireEvent.click(getByRole('button', { name: 'Clear' }));
    expect(input.value).toBe('');
    expect(queryByTitle(`YouTube visual layer (${SAMPLE_ID})`)).toBeNull();
    expect(youtubeLayerStore.state).toEqual({ kind: 'empty' });
  });
});
