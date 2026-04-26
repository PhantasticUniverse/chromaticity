/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Typed Tauri IPC wrappers for the W9 scene-library directory scan.
// Frontend callers invoke `listScenePackages` to enumerate `.chroma`
// packages under a given root, and `defaultSceneRoot` to read the
// platform-default user scene root per
// `docs/38-scene-library-and-import-spec.md` §2.1.

import { invoke } from '@tauri-apps/api/core';

import type { SceneCard } from './bindings';

/// Enumerate every `.chroma` package directly under `root`. The
/// handler canonicalises `root` and refuses paths outside the
/// platform-default scene root; a non-existent path returns the empty
/// list rather than failing.
export async function listScenePackages(root: string): Promise<SceneCard[]> {
  return invoke<SceneCard[]>('list_scene_packages', { root });
}

/// Read the platform-default scene-library root path.
export async function defaultSceneRoot(): Promise<string> {
  return invoke<string>('default_scene_root');
}
