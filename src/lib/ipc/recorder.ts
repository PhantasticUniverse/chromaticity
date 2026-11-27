// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC bridge for the Phase 7a crash-safe fMP4 recorder and rolling
 * replay buffer (`app/shell/src/commands/recorder.rs`).
 *
 * All commands degrade gracefully to `null` when invoked outside a Tauri
 * runtime (browser preview / Vitest) so UI controls remain testable and safe.
 */

import { invoke } from '@tauri-apps/api/core';

export type RecorderLifecycleState = 'idle' | 'recording' | 'stopping';

/** Serializable recorder + rolling replay-buffer snapshot returned across IPC. */
export interface RecorderStatusDto {
  readonly state: RecorderLifecycleState;
  readonly is_recording: boolean;
  readonly width: number;
  readonly height: number;
  readonly started_ms: number | null;
  readonly fragments_written: number;
  readonly bytes_written: number;
  readonly replay_gop_count: number;
  readonly replay_packet_count: number;
  readonly replay_retained_bytes: number;
  readonly replay_retained_secs: number;
  readonly replay_max_duration_secs: number;
}

/** Serializable result returned when flushing the rolling replay buffer to disk. */
export interface ReplaySaveResultDto {
  readonly path: string;
  readonly bytes_written: number;
  readonly gop_count: number;
  readonly packet_count: number;
  readonly duration_90khz: number;
}

export interface StartRecordingOptions {
  readonly width?: number;
  readonly height?: number;
}

/** Start a crash-safe fMP4 recording session. Returns `null` outside Tauri. */
export async function startRecording(
  options?: StartRecordingOptions,
): Promise<RecorderStatusDto | null> {
  try {
    const args: Record<string, number> = {};
    if (options?.width !== undefined) args.width = options.width;
    if (options?.height !== undefined) args.height = options.height;
    if (Object.keys(args).length > 0) {
      return await invoke<RecorderStatusDto>('start_recording', args);
    }
    return await invoke<RecorderStatusDto>('start_recording');
  } catch {
    return null;
  }
}

/** Stop the active fMP4 recording session and flush buffered GOPs. Returns `null` outside Tauri. */
export async function stopRecording(): Promise<RecorderStatusDto | null> {
  try {
    return await invoke<RecorderStatusDto>('stop_recording');
  } catch {
    return null;
  }
}

/** Query the current recorder lifecycle and rolling replay-buffer status. Returns `null` outside Tauri. */
export async function getRecorderStatus(): Promise<RecorderStatusDto | null> {
  try {
    return await invoke<RecorderStatusDto>('get_recorder_status');
  } catch {
    return null;
  }
}

/** Flush the rolling replay buffer to a standalone `.mp4` file. Returns `null` outside Tauri. */
export async function saveReplayBuffer(outputPath?: string): Promise<ReplaySaveResultDto | null> {
  try {
    if (outputPath !== undefined) {
      return await invoke<ReplaySaveResultDto>('save_replay_buffer', { outputPath });
    }
    return await invoke<ReplaySaveResultDto>('save_replay_buffer');
  } catch {
    return null;
  }
}
