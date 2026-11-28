// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../ipc/stream', () => ({
  getStreamStatus: vi.fn(),
  startTwitchStreamDetailed: vi.fn(),
  stopTwitchStream: vi.fn(),
}));

import { DEFAULT_BROADCAST_INGEST_URL, liveBroadcastStore } from './liveBroadcast.svelte';
import {
  getStreamStatus,
  startTwitchStreamDetailed,
  stopTwitchStream,
  type StreamStatusDto,
} from '../ipc/stream';
import { inspectAllStateMachines } from '../state/diagnostics';

const mockGetStreamStatus = vi.mocked(getStreamStatus);
const mockStartTwitchStreamDetailed = vi.mocked(startTwitchStreamDetailed);
const mockStopTwitchStream = vi.mocked(stopTwitchStream);

const LIVE_STATUS: StreamStatusDto = {
  state: 'live',
  is_live: true,
  protocol: 'rtmps',
  ingest_url: DEFAULT_BROADCAST_INGEST_URL,
  bitrate_kbps: 6000,
  width: 1280,
  height: 720,
  fps: 30,
  frames_pushed: 0,
  encoder: 'h264_videotoolbox',
  last_error: null,
};

const OFFLINE_STATUS: StreamStatusDto = {
  ...LIVE_STATUS,
  state: 'offline',
  is_live: false,
  encoder: 'none',
};

describe('liveBroadcastStore state machine', () => {
  beforeEach(() => {
    liveBroadcastStore.reset();
    mockGetStreamStatus.mockReset().mockResolvedValue(null);
    mockStartTwitchStreamDetailed.mockReset();
    mockStopTwitchStream.mockReset().mockResolvedValue(OFFLINE_STATUS);
  });

  afterEach(() => {
    liveBroadcastStore.reset();
  });

  it('rejects empty stream keys without calling IPC and stays offline with error', async () => {
    const next = await liveBroadcastStore.start({
      ingestUrl: DEFAULT_BROADCAST_INGEST_URL,
      streamKey: '   ',
    });

    expect(next.kind).toBe('offline');
    expect(liveBroadcastStore.isLive).toBe(false);
    expect(liveBroadcastStore.error).toMatch(/stream key/i);
    expect(mockStartTwitchStreamDetailed).not.toHaveBeenCalled();
  });

  it('transitions offline -> starting -> live -> stopping -> offline and never exposes secrets in diagnostics', async () => {
    const secretKey = 'live_999999_super_secret_token';
    let resolveStart!: (
      value: { ok: true; data: StreamStatusDto } | { ok: false; error: string },
    ) => void;
    mockStartTwitchStreamDetailed.mockReturnValue(
      new Promise((resolve) => {
        resolveStart = resolve;
      }),
    );

    const startPromise = liveBroadcastStore.start({
      ingestUrl: DEFAULT_BROADCAST_INGEST_URL,
      streamKey: secretKey,
      bitrateKbps: 6000,
    });

    expect(liveBroadcastStore.state.kind).toBe('starting');
    expect(liveBroadcastStore.isPending).toBe(true);
    expect(liveBroadcastStore.isLive).toBe(false);

    // Re-entrant start while starting is a no-op.
    await liveBroadcastStore.start({
      ingestUrl: DEFAULT_BROADCAST_INGEST_URL,
      streamKey: secretKey,
    });
    expect(mockStartTwitchStreamDetailed).toHaveBeenCalledTimes(1);

    resolveStart({ ok: true, data: LIVE_STATUS });
    const liveState = await startPromise;
    expect(liveState.kind).toBe('live');
    expect(liveBroadcastStore.isLive).toBe(true);
    expect(liveBroadcastStore.sessionSeq).toBeTypeOf('number');

    const snapshot = inspectAllStateMachines();
    expect(snapshot.liveBroadcast).toMatchObject({
      kind: 'live',
      isLive: true,
      encoder: 'h264_videotoolbox',
      hasError: false,
    });
    expect(JSON.stringify(snapshot)).not.toContain(secretKey);

    const stopState = await liveBroadcastStore.stop();
    expect(stopState.kind).toBe('offline');
    expect(liveBroadcastStore.isLive).toBe(false);
    expect(liveBroadcastStore.error).toBeNull();
  });

  it('rejects stale sessionSeq frame pushes and disconnect reports after stop', async () => {
    mockStartTwitchStreamDetailed.mockResolvedValue({ ok: true, data: LIVE_STATUS });
    await liveBroadcastStore.start({
      ingestUrl: DEFAULT_BROADCAST_INGEST_URL,
      streamKey: 'live_test_key',
    });

    const seq = liveBroadcastStore.sessionSeq;
    expect(seq).not.toBeNull();
    liveBroadcastStore.recordFramePushed(seq!, 2);
    expect(liveBroadcastStore.status.frames_pushed).toBe(2);

    await liveBroadcastStore.stop();

    // Stale callbacks from the stopped session must never corrupt offline state.
    liveBroadcastStore.recordFramePushed(seq!, 30);
    expect(liveBroadcastStore.status.frames_pushed).toBe(0);
    expect(liveBroadcastStore.reportDisconnect(seq!, null)).toBe(false);
    expect(liveBroadcastStore.state.kind).toBe('offline');
    expect(liveBroadcastStore.error).toBeNull();
  });

  it('transitions live -> failed on unexpected disconnect for the active session', async () => {
    mockStartTwitchStreamDetailed.mockResolvedValue({ ok: true, data: LIVE_STATUS });
    await liveBroadcastStore.start({
      ingestUrl: DEFAULT_BROADCAST_INGEST_URL,
      streamKey: 'live_test_key',
    });

    const seq = liveBroadcastStore.sessionSeq!;
    const disconnected = liveBroadcastStore.reportDisconnect(seq, {
      ...OFFLINE_STATUS,
      state: 'failed',
      last_error: 'rtmps broken pipe',
    });

    expect(disconnected).toBe(true);
    expect(liveBroadcastStore.state.kind).toBe('failed');
    expect(liveBroadcastStore.error).toBe('rtmps broken pipe');

    liveBroadcastStore.clearError();
    expect(liveBroadcastStore.state.kind).toBe('offline');
    expect(liveBroadcastStore.error).toBeNull();
  });
});
