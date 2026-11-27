// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC wrappers for live showtime controls:
 * - GPU shader BPM override & beatmatch phase scrub (`app/shell/src/commands/render.rs`)
 * - DJ / Rekordbox device profile detection & stereo channel-pair routing
 *   (`app/shell/src/commands/audio.rs`)
 *
 * Every wrapper degrades gracefully to `null` when invoked outside the Tauri
 * runtime (browser preview, Vite dev server, or Vitest jsdom).
 */

import { invoke } from '@tauri-apps/api/core';

export type BpmOverrideMode = 'auto' | 'manual' | 'off';

export interface SetBpmOverrideArgs {
  readonly mode: BpmOverrideMode;
  readonly bpm?: number;
  readonly phaseOffsetBeats?: number;
  readonly resyncEpochMs?: number;
  readonly resyncNow?: boolean;
}

export interface BpmOverridePayload {
  readonly mode: BpmOverrideMode;
  readonly bpm?: number;
  readonly phase_offset_beats: number;
  readonly resync_epoch_ms: number;
}

export interface AudioChannelPairPayload {
  readonly left_channel: number;
  readonly right_channel: number;
  readonly label: string;
}

export type DjDeviceKind =
  | 'ddj_flx10'
  | 'ddj_flx4'
  | 'ddj_1000'
  | 'opus_quad'
  | 'xdj_xz'
  | 'djm_a9'
  | 'djm_900nxs2'
  | 'rekordbox_pc_master_out'
  | 'generic_multi_channel';

export interface DjDeviceProfilePayload {
  readonly kind: DjDeviceKind | string;
  readonly display_name: string;
  readonly expected_channels: number;
  readonly master_pair: AudioChannelPairPayload;
  readonly master_label: string;
  readonly cue_pair?: AudioChannelPairPayload | null;
  readonly cue_label?: string | null;
  readonly supports_pc_master_out: boolean;
  readonly notes: string;
  readonly recognized: boolean;
}

const SET_BPM_OVERRIDE_COMMAND = 'set_bpm_override';
const GET_BPM_OVERRIDE_COMMAND = 'get_bpm_override';
const GET_DJ_DEVICE_PROFILE_COMMAND = 'get_dj_device_profile';
const SET_AUDIO_CHANNEL_PAIR_COMMAND = 'set_audio_channel_pair';
const GET_AUDIO_CHANNEL_PAIR_COMMAND = 'get_audio_channel_pair';

/**
 * Push live BPM override mode, manual tempo, phase scrub offset, and optional
 * downbeat resync to the Rust GPU uniform pipeline.
 */
export async function setBpmOverride(args: SetBpmOverrideArgs): Promise<BpmOverridePayload | null> {
  const payload: Record<string, unknown> = { mode: args.mode };
  if (args.bpm !== undefined) payload.bpm = args.bpm;
  if (args.phaseOffsetBeats !== undefined) payload.phaseOffsetBeats = args.phaseOffsetBeats;
  if (args.resyncEpochMs !== undefined) payload.resyncEpochMs = args.resyncEpochMs;
  if (args.resyncNow !== undefined) payload.resyncNow = args.resyncNow;

  try {
    const result = await invoke<BpmOverridePayload | undefined>(SET_BPM_OVERRIDE_COMMAND, payload);
    return result ?? null;
  } catch {
    return null;
  }
}

/** Read the currently active live BPM override state from the Rust shell. */
export async function getBpmOverride(): Promise<BpmOverridePayload | null> {
  try {
    const result = await invoke<BpmOverridePayload | undefined>(GET_BPM_OVERRIDE_COMMAND);
    return result ?? null;
  } catch {
    return null;
  }
}

/**
 * Query the Rekordbox / Pioneer DJ / AlphaTheta hardware routing profile for
 * `deviceName` and `channelCount`.
 */
export async function getDjDeviceProfile(
  deviceName: string,
  channelCount = 4,
): Promise<DjDeviceProfilePayload | null> {
  try {
    const result = await invoke<DjDeviceProfilePayload | undefined>(GET_DJ_DEVICE_PROFILE_COMMAND, {
      deviceName,
      channelCount,
    });
    return result ?? null;
  } catch {
    return null;
  }
}

/**
 * Select the active stereo channel pair (0-based indices, e.g. `(0, 1)` = Ch 1–2)
 * for multi-channel DJ / Rekordbox capture so Headphone Cue stays isolated.
 */
export async function setAudioChannelPair(
  leftChannel: number,
  rightChannel: number,
): Promise<AudioChannelPairPayload | null> {
  try {
    const result = await invoke<AudioChannelPairPayload | undefined>(
      SET_AUDIO_CHANNEL_PAIR_COMMAND,
      {
        leftChannel,
        rightChannel,
      },
    );
    return result ?? null;
  } catch {
    return null;
  }
}

/** Read the currently selected stereo channel pair from the Rust shell. */
export async function getAudioChannelPair(): Promise<AudioChannelPairPayload | null> {
  try {
    const result = await invoke<AudioChannelPairPayload | undefined>(
      GET_AUDIO_CHANNEL_PAIR_COMMAND,
    );
    return result ?? null;
  } catch {
    return null;
  }
}

/**
 * Pure client-side DJ hardware routing heuristic used as an immediate fallback
 * in browser preview or Vitest when the Tauri IPC bridge is unavailable.
 */
export function inferDjDeviceProfileFallback(
  deviceName: string,
  channelCount = 4,
): DjDeviceProfilePayload {
  const normalized = deviceName.trim().toLowerCase();
  const masterPair: AudioChannelPairPayload = {
    left_channel: 0,
    right_channel: 1,
    label: 'Ch 1-2',
  };
  const cuePair: AudioChannelPairPayload = {
    left_channel: 2,
    right_channel: 3,
    label: 'Ch 3-4',
  };

  if (normalized.includes('flx10') || normalized.includes('flx-10')) {
    return {
      kind: 'ddj_flx10',
      display_name: 'Pioneer DJ DDJ-FLX10',
      expected_channels: 4,
      master_pair: masterPair,
      master_label: 'Ch 1–2 (Master / REC OUT)',
      cue_pair: cuePair,
      cue_label: 'Ch 3–4 (Headphone Cue / Deck)',
      supports_pc_master_out: true,
      notes: 'Tap Ch 1–2 (Master / REC OUT); Cue Ch 3–4 isolated.',
      recognized: true,
    };
  }

  if (normalized.includes('flx4') || normalized.includes('flx-4')) {
    return {
      kind: 'ddj_flx4',
      display_name: 'Pioneer DJ DDJ-FLX4',
      expected_channels: 4,
      master_pair: masterPair,
      master_label: 'Ch 1–2 (Master / REC OUT)',
      cue_pair: cuePair,
      cue_label: 'Ch 3–4 (Headphone Cue / Deck)',
      supports_pc_master_out: true,
      notes: 'Tap Ch 1–2 (Master / REC OUT); Cue Ch 3–4 isolated.',
      recognized: true,
    };
  }

  if (
    normalized.includes('ddj-1000') ||
    normalized.includes('ddj1000') ||
    normalized.includes('opus-quad') ||
    normalized.includes('opus quad') ||
    normalized.includes('xdj-xz') ||
    normalized.includes('djm-a9') ||
    normalized.includes('djm-900') ||
    normalized.includes('rekordbox') ||
    normalized.includes('pioneer') ||
    normalized.includes('alphatheta')
  ) {
    return {
      kind: normalized.includes('rekordbox') ? 'rekordbox_pc_master_out' : 'djm_a9',
      display_name: deviceName.trim() || 'Rekordbox / DJ Controller',
      expected_channels: Math.max(4, channelCount),
      master_pair: masterPair,
      master_label: 'Ch 1–2 (Master / REC OUT)',
      cue_pair: cuePair,
      cue_label: 'Ch 3–4 (Headphone Cue / Deck)',
      supports_pc_master_out: true,
      notes: 'Tap Ch 1–2 (Master / REC OUT); Cue Ch 3–4 isolated.',
      recognized: true,
    };
  }

  return {
    kind: 'generic_multi_channel',
    display_name: deviceName.trim() || 'Stereo Input',
    expected_channels: Math.max(2, channelCount),
    master_pair: masterPair,
    master_label: 'Ch 1–2 (Master / REC OUT)',
    cue_pair: null,
    cue_label: null,
    supports_pc_master_out: false,
    notes:
      'Tap Ch 1–2 for standard stereo input, or pick a dedicated bus pair for multi-channel interfaces.',
    recognized: false,
  };
}
