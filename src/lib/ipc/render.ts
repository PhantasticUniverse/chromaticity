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

const thumbnailCache = new Map<string, Uint8ClampedArray>();
const MAX_THUMBNAIL_CACHE_ENTRIES = 96;

/**
 * Render (or retrieve from in-memory cache) a real Metal/Vulkan GPU RGBA8
 * thumbnail for a `.chroma` scene package via `render_scene_thumbnail`.
 * Returns `null` outside Tauri or if the package has no `main.glsl`.
 */
export async function fetchSceneThumbnail(
  scenePath: string,
  width = 320,
  height = 180,
): Promise<Uint8ClampedArray | null> {
  const key = `${scenePath}@${width}x${height}`;
  const cached = thumbnailCache.get(key);
  if (cached) return cached;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const raw = await invoke<ArrayBuffer | Uint8Array | number[]>('render_scene_thumbnail', {
      scenePath,
      width,
      height,
    });
    const expectedLen = width * height * 4;
    let pixels: Uint8ClampedArray | null = null;
    if (raw instanceof ArrayBuffer && raw.byteLength === expectedLen) {
      pixels = new Uint8ClampedArray(raw);
    } else if (raw instanceof Uint8Array && raw.byteLength === expectedLen) {
      pixels = new Uint8ClampedArray(raw.buffer, raw.byteOffset, raw.byteLength);
    } else if (Array.isArray(raw) && raw.length === expectedLen) {
      pixels = new Uint8ClampedArray(raw);
    }
    if (!pixels) return null;

    if (thumbnailCache.size >= MAX_THUMBNAIL_CACHE_ENTRIES) {
      const oldest = thumbnailCache.keys().next().value;
      if (oldest !== undefined) thumbnailCache.delete(oldest);
    }
    thumbnailCache.set(key, pixels);
    return pixels;
  } catch {
    return null;
  }
}

/** Clear the in-memory GPU thumbnail cache (used when a scene is re-saved in Author). */
export function clearSceneThumbnailCache(scenePath?: string): void {
  if (!scenePath) {
    thumbnailCache.clear();
    return;
  }
  for (const key of thumbnailCache.keys()) {
    if (key.startsWith(`${scenePath}@`)) {
      thumbnailCache.delete(key);
    }
  }
}
