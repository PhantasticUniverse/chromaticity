// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../ipc/mix', () => ({
  resolveYouTubeStreamDetailed: vi.fn(),
}));

import { liveMediaStore } from './liveMedia.svelte';
import { resolveYouTubeStreamDetailed } from '../ipc/mix';
import { inspectAllStateMachines } from '../state/diagnostics';

const mockResolveYouTubeStreamDetailed = vi.mocked(resolveYouTubeStreamDetailed);

function createFakeStream() {
  const stop = vi.fn();
  const listeners = new Map<string, () => void>();
  const track = {
    stop,
    addEventListener: vi.fn((event: string, cb: () => void) => {
      listeners.set(event, cb);
    }),
  };
  const stream = {
    getTracks: () => [track],
  } as unknown as MediaStream;
  return { stream, stop, triggerEnded: () => listeners.get('ended')?.() };
}

describe('liveMediaStore state machines', () => {
  beforeEach(() => {
    liveMediaStore.reset();
    mockResolveYouTubeStreamDetailed.mockReset();
  });

  afterEach(() => {
    liveMediaStore.reset();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('cancels in-flight webcam requests cleanly and stops late-arriving tracks', async () => {
    let resolveGum!: (stream: MediaStream) => void;
    const fake = createFakeStream();
    vi.stubGlobal('navigator', {
      mediaDevices: {
        getUserMedia: vi.fn(
          () =>
            new Promise<MediaStream>((resolve) => {
              resolveGum = resolve;
            }),
        ),
      },
    });

    const startPromise = liveMediaStore.startWebcam();
    expect(liveMediaStore.webcam.kind).toBe('requesting');
    expect(liveMediaStore.webcamPending).toBe(true);

    // Cancel before getUserMedia resolves.
    liveMediaStore.stopWebcam();
    expect(liveMediaStore.webcam.kind).toBe('idle');

    resolveGum(fake.stream);
    await startPromise;

    expect(liveMediaStore.webcam.kind).toBe('idle');
    expect(liveMediaStore.webcamStream).toBeNull();
    expect(fake.stop).toHaveBeenCalledTimes(1);
  });

  it('transitions screen capture active -> idle when OS ends the capture track', async () => {
    const fake = createFakeStream();
    vi.stubGlobal('navigator', {
      mediaDevices: {
        getDisplayMedia: vi.fn().mockResolvedValue(fake.stream),
      },
    });

    await liveMediaStore.startScreenCapture();
    expect(liveMediaStore.screen.kind).toBe('active');
    expect(liveMediaStore.screenStream).toBe(fake.stream);

    fake.triggerEnded();
    expect(liveMediaStore.screen.kind).toBe('idle');
    expect(liveMediaStore.screenStream).toBeNull();
  });

  it('revokes prior blob URLs when replacing or clearing local media', () => {
    const createSpy = vi
      .spyOn(URL, 'createObjectURL')
      .mockReturnValueOnce('blob:first')
      .mockReturnValueOnce('blob:second');
    const revokeSpy = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

    const imgFile = new File(['abc'], 'logo.png', { type: 'image/png' });
    const vidFile = new File(['xyz'], 'loop.mp4', { type: 'video/mp4' });

    liveMediaStore.loadLocalMediaFile(imgFile);
    expect(liveMediaStore.localMedia).toEqual({
      kind: 'loaded',
      url: 'blob:first',
      mediaKind: 'image',
      name: 'logo.png',
    });

    liveMediaStore.loadLocalMediaFile(vidFile);
    expect(revokeSpy).toHaveBeenCalledWith('blob:first');
    expect(liveMediaStore.localMedia).toEqual({
      kind: 'loaded',
      url: 'blob:second',
      mediaKind: 'video',
      name: 'loop.mp4',
    });

    liveMediaStore.clearLocalMedia();
    expect(revokeSpy).toHaveBeenCalledWith('blob:second');
    expect(liveMediaStore.localMedia.kind).toBe('empty');
    expect(createSpy).toHaveBeenCalledTimes(2);
  });

  it('drops stale YouTube native resolutions and reports clean diagnostics without tokens', async () => {
    let resolveFirst!: (value: { stream: null; error: string }) => void;
    mockResolveYouTubeStreamDetailed
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveFirst = resolve;
        }),
      )
      .mockResolvedValueOnce({
        stream: {
          video_id: 'dQw4w9WgXcQ',
          title: 'Visual Loop',
          stream_url: 'http://127.0.0.1:43210/yt/1?token=secret_proxy_token',
        },
        error: null,
      });

    const first = liveMediaStore.resolveYouTube('aaaaaaaaaaa');
    expect(liveMediaStore.youtubeResolve).toEqual({
      kind: 'resolving',
      videoId: 'aaaaaaaaaaa',
    });

    const second = await liveMediaStore.resolveYouTube('dQw4w9WgXcQ');
    expect(second.kind).toBe('resolved');

    // First request finishing late must be ignored.
    resolveFirst({ stream: null, error: 'stale failure' });
    await first;
    expect(liveMediaStore.youtubeResolve.kind).toBe('resolved');

    const snapshot = inspectAllStateMachines();
    expect(snapshot.liveMedia).toEqual({
      webcam: 'idle',
      screen: 'idle',
      localMedia: { kind: 'empty' },
      youtubeResolve: { kind: 'resolved', videoId: 'dQw4w9WgXcQ' },
    });
    expect(JSON.stringify(snapshot)).not.toContain('secret_proxy_token');

    liveMediaStore.reportYouTubePlaybackError('dQw4w9WgXcQ', 4);
    expect(liveMediaStore.youtubeResolve).toEqual({
      kind: 'fallback',
      videoId: 'dQw4w9WgXcQ',
      error: 'Video playback error (code 4)',
    });
  });
});
