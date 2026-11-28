// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Scene media (`chroma_Media`) ingest IPC — see `app/shell/src/commands/media.rs`.
 *
 * Wire format: 12-byte little-endian header `[width u32][height u32][kind u32]`
 * followed by exactly `width * height * 4` RGBA8 bytes, row 0 = top.
 */

import { invoke } from '@tauri-apps/api/core';

/** `chroma_MediaType` codes accepted by the shell. */
export type SceneMediaKindCode = 1 | 2 | 3;
export const MEDIA_KIND_IMAGE: SceneMediaKindCode = 1;
export const MEDIA_KIND_VIDEO: SceneMediaKindCode = 2;
export const MEDIA_KIND_WEBCAM: SceneMediaKindCode = 3;

/** Build the raw IPC payload for one RGBA8 frame. */
export function encodeSceneMediaFrame(
  width: number,
  height: number,
  kind: SceneMediaKindCode,
  rgba: Uint8Array | Uint8ClampedArray,
): Uint8Array {
  if (rgba.length !== width * height * 4) {
    throw new RangeError(`expected ${width * height * 4} RGBA bytes, got ${rgba.length}`);
  }
  const out = new Uint8Array(12 + rgba.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, width, true);
  view.setUint32(4, height, true);
  view.setUint32(8, kind, true);
  out.set(rgba, 12);
  return out;
}

/** Push one encoded frame. Returns the slot version, or `null` outside Tauri / on rejection. */
export async function pushSceneMediaFrame(payload: Uint8Array): Promise<number | null> {
  try {
    return await invoke<number>('push_scene_media_frame', payload);
  } catch {
    return null;
  }
}

/** Clear the scene media. Returns the slot version, or `null` outside Tauri. */
export async function clearSceneMedia(): Promise<number | null> {
  try {
    return await invoke<number>('clear_scene_media');
  } catch {
    return null;
  }
}
