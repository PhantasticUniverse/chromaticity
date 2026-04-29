// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Shared-primitive backfill — `MediaCard` widget tests. Bundle source:
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
// lines 251-264. Covers structural mount, label round-trip, the four
// kind-conditional surface treatments (`none` / `webcam` / `video` /
// `test`), the video-only play glyph + duration overlay, the active
// state, and click dispatch.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import MediaCard from './MediaCard.svelte';

describe('MediaCard widget', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders a button labeled by the label prop', () => {
    const { container } = render(MediaCard, { props: { label: 'jellyfish.mp4' } });
    const btn = container.querySelector('button.media-card');
    expect(btn).not.toBeNull();
    expect(btn?.getAttribute('aria-label')).toBe('jellyfish.mp4');
    expect(btn?.getAttribute('title')).toBe('jellyfish.mp4');
    expect(container.querySelector('.media-card__label')?.textContent).toBe('jellyfish.mp4');
  });

  it('defaults to kind=video and renders the SceneStill thumbnail + play glyph', () => {
    const { container } = render(MediaCard, { props: { label: 'video.mp4' } });
    const card = container.querySelector('.media-card');
    expect(card?.getAttribute('data-kind')).toBe('video');
    expect(container.querySelector('.media-card__thumb canvas')).not.toBeNull();
    expect(container.querySelector('.media-card__kind')).not.toBeNull();
  });

  it('renders the empty-slot glyph for kind=none and skips the play glyph', () => {
    const { container } = render(MediaCard, {
      props: { label: 'none', kind: 'none' },
    });
    expect(container.querySelector('.media-card__none')).not.toBeNull();
    expect(container.querySelector('.media-card__thumb')).toBeNull();
    expect(container.querySelector('.media-card__kind')).toBeNull();
  });

  it('renders the webcam treatment for kind=webcam', () => {
    const { container } = render(MediaCard, {
      props: { label: 'webcam', kind: 'webcam' },
    });
    expect(container.querySelector('.media-card__webcam')).not.toBeNull();
    expect(container.querySelector('.media-card__thumb')).toBeNull();
    expect(container.querySelector('.media-card__kind')).toBeNull();
  });

  it('renders eight SMPTE bars for kind=test', () => {
    const { container } = render(MediaCard, {
      props: { label: 'test card', kind: 'test' },
    });
    const bars = container.querySelectorAll('.media-card__test-bar');
    expect(bars).toHaveLength(8);
  });

  it('renders the duration overlay only for kind=video when dur is set', () => {
    const { container, unmount } = render(MediaCard, {
      props: { label: 'no-dur', kind: 'video' },
    });
    expect(container.querySelector('.media-card__dur')).toBeNull();
    unmount();

    const withDur = render(MediaCard, {
      props: { label: 'with-dur', kind: 'video', dur: '0:23' },
    });
    const dur = withDur.container.querySelector('.media-card__dur');
    expect(dur?.textContent).toBe('0:23');
  });

  it('skips the duration overlay for kind=test even when dur is set', () => {
    const { container } = render(MediaCard, {
      props: { label: 'test', kind: 'test', dur: '0:30' },
    });
    expect(container.querySelector('.media-card__dur')).toBeNull();
  });

  it('reflects the active state via aria-pressed and the active class', () => {
    const { container } = render(MediaCard, {
      props: { label: 'active', kind: 'video', active: true },
    });
    const card = container.querySelector('.media-card');
    expect(card?.classList.contains('active')).toBe(true);
    expect(card?.getAttribute('aria-pressed')).toBe('true');
  });

  it('reports aria-pressed=false when inactive', () => {
    const { container } = render(MediaCard, {
      props: { label: 'inactive', kind: 'video' },
    });
    expect(container.querySelector('.media-card')?.getAttribute('aria-pressed')).toBe('false');
  });

  it('invokes onClick when clicked', async () => {
    const onClick = vi.fn();
    const { container } = render(MediaCard, {
      props: { label: 'click', kind: 'video', onClick },
    });
    const card = container.querySelector('.media-card');
    expect(card).not.toBeNull();
    await fireEvent.click(card as Element);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
