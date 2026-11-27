// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC bridge for Mix view GPU cost telemetry, adaptive resolution
 * scaling, and compositor capabilities (`app/shell/src/commands/mix.rs`).
 *
 * All commands degrade gracefully to `null` when invoked outside a Tauri
 * runtime (browser preview / Vitest) so UI controls remain testable and safe.
 */

import { invoke } from '@tauri-apps/api/core';

export type GpuCostTier = 'Light' | 'Medium' | 'Heavy';

/** Serializable GPU cost & adaptive-scaling telemetry snapshot returned across IPC. */
export interface GpuCostTelemetryDto {
  readonly tier: GpuCostTier;
  readonly ema_ms: number;
  readonly current_scale: number;
  readonly scaled_width: number;
  readonly scaled_height: number;
  readonly adaptive_enabled: boolean;
}

/** Serializable GPU multi-layer compositor capability summary returned across IPC. */
export interface CompositorCapabilitiesDto {
  readonly blend_modes: readonly string[];
  readonly max_layers: number;
  readonly active_layer_count: number;
  readonly ready: boolean;
}

/** Query live GPU cost tier, EMA frame time, and adaptive scale state. Returns `null` outside Tauri. */
export async function getGpuCostTelemetry(): Promise<GpuCostTelemetryDto | null> {
  try {
    return await invoke<GpuCostTelemetryDto>('get_gpu_cost_telemetry');
  } catch {
    return null;
  }
}

/** Enable or disable adaptive resolution scaling and return the updated telemetry snapshot. Returns `null` outside Tauri. */
export async function setAdaptiveScaleEnabled(
  enabled: boolean,
): Promise<GpuCostTelemetryDto | null> {
  try {
    return await invoke<GpuCostTelemetryDto>('set_adaptive_scale_enabled', { enabled });
  } catch {
    return null;
  }
}

/** Query supported compositor blend modes, layer capacity, and readiness. Returns `null` outside Tauri. */
export async function getCompositorCapabilities(): Promise<CompositorCapabilitiesDto | null> {
  try {
    return await invoke<CompositorCapabilitiesDto>('get_compositor_capabilities');
  } catch {
    return null;
  }
}
