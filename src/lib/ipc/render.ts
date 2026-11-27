// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC bridge for the live `wgpu` GPU preview renderer
 * (`app/shell/src/commands/render.rs`).
 */

export interface GpuRenderStatusPayload {
  readonly state: 'idle' | 'active' | 'error';
  readonly scene_id: string | null;
  readonly frames_rendered: number;
  readonly last_frame_ms: number;
  readonly error: string | null;
}

/** Query the current GPU preview engine status. Returns `null` outside Tauri. */
export async function fetchGpuRenderStatus(): Promise<GpuRenderStatusPayload | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<GpuRenderStatusPayload>('gpu_render_status');
  } catch {
    return null;
  }
}

/**
 * Compile and arm an in-memory GLSL 4.50 fragment shader from Author.
 * Resolves with the updated GPU status or throws the `naga`/`wgpu` diagnostic.
 */
export async function compileAndPreviewShader(
  sceneId: string,
  fragmentGlsl: string,
): Promise<GpuRenderStatusPayload> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<GpuRenderStatusPayload>('compile_and_preview_shader', {
    sceneId,
    fragmentGlsl,
  });
}

/**
 * Render one live GPU RGBA8 frame at `(width, height)`.
 * Returns a `Uint8ClampedArray` of length `width * height * 4`, or `null`
 * if outside Tauri or if no `main.glsl` shader is armed.
 */
export async function fetchGpuFrame(
  width: number,
  height: number,
): Promise<Uint8ClampedArray | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const raw = await invoke<ArrayBuffer | Uint8Array | number[]>('render_gpu_frame', {
      width,
      height,
    });
    const expectedLen = width * height * 4;
    if (raw instanceof ArrayBuffer) {
      return raw.byteLength === expectedLen ? new Uint8ClampedArray(raw) : null;
    }
    if (raw instanceof Uint8Array) {
      return raw.byteLength === expectedLen
        ? new Uint8ClampedArray(raw.buffer, raw.byteOffset, raw.byteLength)
        : null;
    }
    if (Array.isArray(raw) && raw.length === expectedLen) {
      return new Uint8ClampedArray(raw);
    }
    return null;
  } catch {
    return null;
  }
}
