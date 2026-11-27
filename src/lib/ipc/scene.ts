// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Typed Tauri IPC wrappers for the Wave 2.1 Lane A.1 scene-load
// round-trip. Frontend callers invoke `loadScene(path)` to parse a
// `.chroma` package on the engine side and surface the normalized
// `LoadedScenePayload` shape (control schema + author metadata) that
// the Perform view's Scene Controls panel binds against; the
// `loadedScene()` reader surfaces the most recently loaded scene
// when a route navigation re-mounts the consuming component.

import { invoke } from '@tauri-apps/api/core';

import type { LoadedScenePayload } from './bindings';

/// Load the `.chroma` package at `path` into the runtime and return
/// the normalized scene payload (id, name, controls, etc.). Resolves
/// once the engine has parsed `scene.json` and stored the result on
/// the shell-side AppState; rejects on directory-shape errors,
/// missing `scene.json`, or JSON parse failures with the engine-side
/// `LoadError`'s Display string.
export async function loadScene(path: string): Promise<LoadedScenePayload> {
  return await invoke<LoadedScenePayload>('load_scene', { path });
}

/// Read the most recently loaded scene's payload, or `null` when no
/// scene has been loaded since process start. Used by the Perform
/// view's mount-time hydration so the Scene Controls header survives
/// a route navigation that re-mounts the component.
export async function loadedScene(): Promise<LoadedScenePayload | null> {
  return await invoke<LoadedScenePayload | null>('loaded_scene');
}
