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

/** Detailed result preserving backend Rust error strings for live diagnostics. */
export type StreamIpcResult<T> =
  | { readonly ok: true; readonly data: T }
  | { readonly ok: false; readonly error: string };

/** Start a live Twitch RTMPS/RTMP broadcast stream, preserving backend error text. */
export async function startTwitchStreamDetailed(
  params: StartTwitchStreamParams,
): Promise<StreamIpcResult<StreamStatusDto>> {
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

    const data = await invoke<StreamStatusDto>('start_twitch_stream', payload);
    return { ok: true, data };
  } catch (err) {
    return {
      ok: false,
      error: typeof err === 'string' ? err : err instanceof Error ? err.message : String(err),
    };
  }
}

/** Start a live Twitch RTMPS/RTMP broadcast stream. Returns `null` outside Tauri or on IPC error. */
export async function startTwitchStream(
  params: StartTwitchStreamParams,
): Promise<StreamStatusDto | null> {
  const res = await startTwitchStreamDetailed(params);
  return res.ok ? res.data : null;
}

/** Stop the active Twitch broadcast stream and reap any running encoder. Returns `null` outside Tauri. */
export async function stopTwitchStream(): Promise<StreamStatusDto | null> {
  try {
    return await invoke<StreamStatusDto>('stop_twitch_stream');
  } catch {
    return null;
  }
}

/** Serializable host audio capture device entry for Twitch broadcasting. */
export interface StreamAudioDeviceDto {
  readonly value: string;
  readonly label: string;
}

/** Query available live audio capture devices (`avfoundation`) for Twitch streaming. */
export async function listStreamAudioDevices(): Promise<readonly StreamAudioDeviceDto[]> {
  try {
    return await invoke<StreamAudioDeviceDto[]>('list_stream_audio_devices');
  } catch {
    return [];
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

/**
 * Push one JPEG frame into the active encoder (`image2pipe` / `mjpeg`)
 * over Tauri v2's zero-copy raw binary IPC channel (`Uint8Array` / `ArrayBuffer`),
 * avoiding multi-megabyte JSON serialization on the main thread.
 * Returns total frames pushed or `null` if the pipe closed or failed.
 */
export async function pushStreamFrame(
  jpegBytes: readonly number[] | Uint8Array | Uint8ClampedArray,
): Promise<number | null> {
  try {
    const raw =
      jpegBytes instanceof Uint8Array
        ? jpegBytes
        : jpegBytes instanceof Uint8ClampedArray
          ? new Uint8Array(jpegBytes.buffer, jpegBytes.byteOffset, jpegBytes.byteLength)
          : new Uint8Array(jpegBytes);
    return await invoke<number>('push_stream_frame', raw);
  } catch {
    return null;
  }
}
