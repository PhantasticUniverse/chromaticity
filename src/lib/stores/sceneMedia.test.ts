// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { liveMediaStore } from './liveMedia.svelte';
import { fitWithin, sceneMediaStore, testCardRgba } from './sceneMedia.svelte';
import { encodeSceneMediaFrame } from '../ipc/media';

describe('scene media helpers', () => {
  it('fits sources inside 640x360 without upscaling', () => {
    expect(fitWithin(1920, 1080)).toEqual({ width: 640, height: 360 });
    expect(fitWithin(300, 200)).toEqual({ width: 300, height: 200 });
    expect(fitWithin(1080, 1920)).toEqual({ width: 203, height: 360 });
    expect(fitWithin(0, 10)).toEqual({ width: 1, height: 1 });
  });

  it('draws colour bars over a grey ramp', () => {
    const px = testCardRgba(8, 3);
    const at = (x: number, y: number) => Array.from(px.slice((y * 8 + x) * 4, (y * 8 + x) * 4 + 4));
    expect(at(0, 0)).toEqual([255, 255, 255, 255]);
    expect(at(5, 0)).toEqual([255, 0, 0, 255]);
    expect(at(7, 1)).toEqual([0, 0, 0, 255]);
    expect(at(0, 2)).toEqual([0, 0, 0, 255]);
    expect(at(7, 2)).toEqual([255, 255, 255, 255]);
  });

  it('encodes a little-endian header followed by the exact RGBA payload', () => {
    const payload = encodeSceneMediaFrame(2, 1, 3, new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]));
    const view = new DataView(payload.buffer);
    expect([view.getUint32(0, true), view.getUint32(4, true), view.getUint32(8, true)]).toEqual([
      2, 1, 3,
    ]);
    expect(Array.from(payload.slice(12))).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(() => encodeSceneMediaFrame(2, 2, 1, new Uint8Array(4))).toThrow(RangeError);
  });
});

describe('sceneMediaStore', () => {
  beforeEach(() => {
    sceneMediaStore.reset();
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(1);
  });
  afterEach(() => {
    sceneMediaStore.reset();
    vi.restoreAllMocks();
  });

  it('pushes the test card once as a 640x360 image frame', async () => {
    await sceneMediaStore.select('test');
    const pushes = invokeMock.mock.calls.filter(([cmd]) => cmd === 'push_scene_media_frame');
    expect(pushes).toHaveLength(1);
    const payload = pushes[0]?.[1] as Uint8Array;
    const view = new DataView(payload.buffer);
    expect([view.getUint32(0, true), view.getUint32(4, true), view.getUint32(8, true)]).toEqual([
      640, 360, 1,
    ]);
    expect(payload.length).toBe(12 + 640 * 360 * 4);
    expect(sceneMediaStore.machine).toEqual({
      kind: 'streaming',
      source: 'test',
      width: 640,
      height: 360,
    });
  });

  it('clears the shell slot when switching back to none', async () => {
    await sceneMediaStore.select('test');
    invokeMock.mockClear();
    await sceneMediaStore.select('none');
    expect(invokeMock).toHaveBeenCalledWith('clear_scene_media');
    expect(sceneMediaStore.machine).toEqual({ kind: 'off' });
    expect(sceneMediaStore.source).toBe('none');
  });

  it('reports an error and clears when a local file is selected but none is loaded', async () => {
    await sceneMediaStore.select('local');
    expect(sceneMediaStore.machine.kind).toBe('error');
    expect(invokeMock).toHaveBeenCalledWith('clear_scene_media');
    expect(invokeMock).not.toHaveBeenCalledWith('push_scene_media_frame', expect.anything());
  });

  it('reports an error instead of streaming when the shell rejects the push', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'push_scene_media_frame') throw new Error('rejected');
      return 1;
    });
    await sceneMediaStore.select('test');
    expect(sceneMediaStore.machine).toEqual({
      kind: 'error',
      source: 'test',
      message: 'scene media push rejected by the shell',
    });
  });

  it('stops a webcam that a superseded selection started', async () => {
    let resolveStart: (m: { kind: 'active'; stream: MediaStream }) => void = () => {};
    const startSpy = vi
      .spyOn(liveMediaStore, 'startWebcam')
      .mockImplementation(() => new Promise((r) => (resolveStart = r)));
    const stopSpy = vi.spyOn(liveMediaStore, 'stopWebcam').mockImplementation(() => {});
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);

    const pending = sceneMediaStore.select('webcam');
    await sceneMediaStore.select('none');
    resolveStart({ kind: 'active', stream: {} as MediaStream });
    await pending;

    expect(startSpy).toHaveBeenCalledTimes(1);
    expect(stopSpy).toHaveBeenCalledTimes(1);
    expect(sceneMediaStore.source).toBe('none');
    expect(sceneMediaStore.machine).toEqual({ kind: 'off' });
  });
});
