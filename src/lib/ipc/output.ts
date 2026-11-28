// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC wrappers for the dedicated secondary-display / fullscreen
 * stage output window (`app/shell/src/commands/output.rs`).
 */

import { isTauri } from '@tauri-apps/api/core';

export interface OutputWindowStatus {
  open: boolean;
  fullscreen: boolean;
  width: number;
  height: number;
}

export interface MonitorSummary {
  index: number;
  name: string;
  width: number;
  height: number;
  scale_factor: number;
  is_primary: boolean;
}

/** Graphics `display` setting value meaning "secondary display when one is
 * connected, otherwise the primary" (the shell's default placement). */
export const DISPLAY_AUTO = 'auto';

/** Map the persisted Graphics `display` setting (`auto` or `display-N`,
 * 1-based) to the `open_output_window` monitor index (0-based), or
 * `undefined` for automatic placement. */
export function monitorIndexForDisplaySetting(value: string): number | undefined {
  const match = /^display-(\d+)$/.exec(value);
  if (!match) return undefined;
  const n = Number(match[1]);
  return Number.isInteger(n) && n >= 1 ? n - 1 : undefined;
}

/** Settings `display` value for a monitor index (0-based). */
export function displaySettingForMonitor(index: number): string {
  return `display-${index + 1}`;
}

async function invokeOutput<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export async function listOutputMonitors(): Promise<MonitorSummary[]> {
  if (!isTauri()) {
    return [
      {
        index: 0,
        name: 'Browser Display',
        width: typeof window !== 'undefined' ? window.innerWidth : 1920,
        height: typeof window !== 'undefined' ? window.innerHeight : 1080,
        scale_factor: typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1,
        is_primary: true,
      },
    ];
  }
  return invokeOutput<MonitorSummary[]>('list_output_monitors');
}

export async function openOutputWindow(options?: {
  fullscreen?: boolean;
  monitorIndex?: number | undefined;
}): Promise<OutputWindowStatus> {
  if (!isTauri()) {
    return {
      open: true,
      fullscreen: Boolean(options?.fullscreen),
      width: 1280,
      height: 720,
    };
  }
  return invokeOutput<OutputWindowStatus>('open_output_window', {
    fullscreen: options?.fullscreen ?? false,
    monitorIndex: options?.monitorIndex ?? null,
  });
}

export async function closeOutputWindow(): Promise<OutputWindowStatus> {
  if (!isTauri()) {
    return { open: false, fullscreen: false, width: 0, height: 0 };
  }
  return invokeOutput<OutputWindowStatus>('close_output_window');
}

export async function toggleOutputFullscreen(): Promise<OutputWindowStatus> {
  if (!isTauri()) {
    return { open: true, fullscreen: true, width: 1280, height: 720 };
  }
  return invokeOutput<OutputWindowStatus>('toggle_output_fullscreen');
}

export async function getOutputWindowStatus(): Promise<OutputWindowStatus> {
  if (!isTauri()) {
    return { open: false, fullscreen: false, width: 0, height: 0 };
  }
  return invokeOutput<OutputWindowStatus>('output_window_status');
}
