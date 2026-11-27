// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Typed Tauri IPC wrappers for the audio device-enumeration handlers
// registered from `app/shell/src/commands/audio.rs`.

import { invoke } from '@tauri-apps/api/core';

import type { AudioDevicePayload } from './bindings/AudioDevicePayload';

const LIST_AUDIO_DEVICES_COMMAND = 'list_audio_devices';
const LIST_AUDIO_OUTPUT_DEVICES_COMMAND = 'list_audio_output_devices';

/**
 * Enumerate the cpal input devices visible to the host.
 *
 * The shell-side handler collapses CI / headless / no-audio-subsystem
 * failures to an empty vec so caller fallback UI can treat "no
 * devices" and "audio subsystem unavailable" uniformly.
 */
export async function listAudioDevices(): Promise<AudioDevicePayload[]> {
  return invoke<AudioDevicePayload[]>(LIST_AUDIO_DEVICES_COMMAND);
}

/**
 * Enumerate the cpal output devices visible to the host.
 *
 * Mirrors `listAudioDevices`, but every payload carries the Output
 * discriminant from the shell-side bridge.
 */
export async function listAudioOutputDevices(): Promise<AudioDevicePayload[]> {
  return invoke<AudioDevicePayload[]>(LIST_AUDIO_OUTPUT_DEVICES_COMMAND);
}
