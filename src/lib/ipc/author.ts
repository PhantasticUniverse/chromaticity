// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Typed Tauri IPC bridge for `.chroma` scene package file I/O
 * (`app/shell/src/commands/author.rs`).
 */

export type EditableSceneFileName = 'main.glsl' | 'scene.json' | 'script.js';

export interface SceneFilesBundle {
  readonly main_glsl: string | null;
  readonly scene_json: string | null;
  readonly script_js: string | null;
}

/**
 * Read `main.glsl`, `scene.json`, and `script.js` from a `.chroma` package
 * directory. Returns `null` outside Tauri or if the command fails.
 */
export async function readSceneFiles(scenePath: string): Promise<SceneFilesBundle | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<SceneFilesBundle>('read_scene_files', { scenePath });
  } catch {
    return null;
  }
}

/**
 * Atomically write an allowlisted scene file (`main.glsl`, `scene.json`, or
 * `script.js`) inside a `.chroma` package directory.
 * Returns `true` on success, or `false` outside Tauri / on failure.
 */
export async function writeSceneFile(
  scenePath: string,
  fileName: EditableSceneFileName,
  content: string,
): Promise<boolean> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke<string>('write_scene_file', { scenePath, fileName, content });
    return true;
  } catch {
    return false;
  }
}
