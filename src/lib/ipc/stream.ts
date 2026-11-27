// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC bridge for live Twitch RTMPS/RTMP broadcast publishing
 * (`app/shell/src/commands/stream.rs`).
 *
 * All commands degrade gracefully to `null` when invoked outside a Tauri
 * runtime (browser preview / Vitest) so UI controls remain testable and safe.
 * Stream keys are forwarded directly to the Rust backend and are NEVER logged
 * or persisted by this module.
 */

import { invoke } from '@tauri-apps/api/core';

export type StreamLifecycleState = 'offline' | 'connecting' | 'live' | 'reconnecting' | 'failed';

/** Serializable live broadcast status snapshot returned across IPC. */
export interface StreamStatusDto {
  readonly state: StreamLifecycleState | string;
  readonly is_live: boolean;
  readonly protocol: string;
  readonly ingest_url: string;
  readonly bitrate_kbps: number;
  readonly width: number;
  readonly height: number;
  readonly fps: number;
  readonly frames_pushed: number;
  readonly encoder: string;
  readonly last_error: string | null;
}

/** Parameters accepted by the `start_twitch_stream` Tauri command. */
export interface StartTwitchStreamParams {
  readonly ingestUrl: string;
  readonly streamKey: string;
  readonly bitrateKbps?: number;
  readonly width?: number;
  readonly height?: number;
  readonly fps?: number;
  readonly audioDevice?: string | null;
}

/** Start a live Twitch RTMPS/RTMP broadcast stream. Returns `null` outside Tauri or on IPC error. */
export async function startTwitchStream(
  params: StartTwitchStreamParams,
): Promise<StreamStatusDto | null> {
  try {
    const payload: {
      ingestUrl: string;
      streamKey: string;
      bitrateKbps?: number;
      width?: number;
      height?: number;
      fps?: number;
      audioDevice: string | null;
    } = {
      ingestUrl: params.ingestUrl,
      streamKey: params.streamKey,
      audioDevice: params.audioDevice ?? null,
    };
    if (params.bitrateKbps !== undefined) payload.bitrateKbps = params.bitrateKbps;
    if (params.width !== undefined) payload.width = params.width;
    if (params.height !== undefined) payload.height = params.height;
    if (params.fps !== undefined) payload.fps = params.fps;

    return await invoke<StreamStatusDto>('start_twitch_stream', payload);
  } catch {
    return null;
  }
}

/** Stop the active Twitch broadcast stream and reap any running encoder. Returns `null` outside Tauri. */
export async function stopTwitchStream(): Promise<StreamStatusDto | null> {
  try {
    return await invoke<StreamStatusDto>('stop_twitch_stream');
  } catch {
    return null;
  }
}

/** Query the current live broadcast status snapshot. Returns `null` outside Tauri. */
export async function getStreamStatus(): Promise<StreamStatusDto | null> {
  try {
    return await invoke<StreamStatusDto>('get_stream_status');
  } catch {
    return null;
  }
}

/** Push one raw RGBA frame (`width * height * 4` bytes) into the active encoder. Returns total frames pushed or `null`. */
export async function pushStreamFrame(
  rgbaBytes: readonly number[] | Uint8Array | Uint8ClampedArray,
): Promise<number | null> {
  try {
    const bytes = Array.isArray(rgbaBytes) ? rgbaBytes : Array.from(rgbaBytes);
    return await invoke<number>('push_stream_frame', { rgbaBytes: bytes });
  } catch {
    return null;
  }
}
