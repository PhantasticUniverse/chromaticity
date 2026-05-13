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
/// list rather than failing. Cards are tagged `source: "User"`
/// server-side.
export async function listScenePackages(root: string): Promise<SceneCard[]> {
  return invoke<SceneCard[]>('list_scene_packages', { root });
}

/// Enumerate every `.chroma` package directly under the Tauri-bundled
/// scene root (`<AppBundle>/Resources/scenes/`). Cards are tagged
/// `source: "Bundled"` server-side.
///
/// Surfaces the W11 ship-time corpus per
/// `docs/52-parity-coverage-matrix.md` row 37.
export async function listBundledScenePackages(): Promise<SceneCard[]> {
  return invoke<SceneCard[]>('list_bundled_scene_packages');
}

/// Enumerate every `.chroma` package directly under an operator-
/// supplied custom scene folder (Wave 2.8 LIB.LOCAL). The handler
/// skips the descendant-of-default-root path-traversal guard because
/// the operator has explicitly nominated the folder via Settings →
/// Storage → Add Custom Scene Folder (persisted as
/// `settings.storage.scene_folders` per Wave 2.7 SET.B). Cards are
/// tagged `source: "User"` server-side; a nonexistent root surfaces
/// as the empty vec rather than an error so the `Promise.allSettled`
/// composition in `Library.svelte::loadCards` stays resilient when
/// the persisted path no longer exists on disk.
export async function listCustomScenePackages(root: string): Promise<SceneCard[]> {
  return invoke<SceneCard[]>('list_custom_scene_packages', { root });
}

/// Read the platform-default scene-library root path.
export async function defaultSceneRoot(): Promise<string> {
  return invoke<string>('default_scene_root');
}

/// Read the Tauri-bundled scene-library root path.
export async function bundledSceneRoot(): Promise<string> {
  return invoke<string>('bundled_scene_root');
}
