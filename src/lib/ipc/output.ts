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
  monitorIndex?: number;
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
