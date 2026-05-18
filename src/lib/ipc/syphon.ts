// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Typed Tauri IPC wrappers for the Syphon source-picker and
// output-publish handlers registered from
// `app/shell/src/commands/syphon.rs`.

import { invoke } from '@tauri-apps/api/core';

import type { SyphonOutputStatusPayload } from './bindings/SyphonOutputStatusPayload';
import type { SyphonServerPayload } from './bindings/SyphonServerPayload';

const LIST_SYPHON_SERVERS_COMMAND = 'list_syphon_servers';
const PUBLISH_SYPHON_OUTPUT_COMMAND = 'publish_syphon_output';
const UNPUBLISH_SYPHON_OUTPUT_COMMAND = 'unpublish_syphon_output';

/**
 * Enumerate currently-advertised Syphon servers on the local machine.
 *
 * The command returns an empty vec under the current macOS scaffold and
 * rejects with `syphon: macOS-only feature` on non-macOS builds.
 */
export async function listSyphonServers(): Promise<SyphonServerPayload[]> {
  return invoke<SyphonServerPayload[]>(LIST_SYPHON_SERVERS_COMMAND);
}

/**
 * Publish a Chromaticity output server under the operator-supplied
 * name and return the shell-side active/inactive status payload.
 */
export async function publishSyphonOutput(name: string): Promise<SyphonOutputStatusPayload> {
  return invoke<SyphonOutputStatusPayload>(PUBLISH_SYPHON_OUTPUT_COMMAND, { name });
}

/**
 * Unpublish the Chromaticity Syphon output server and return the
 * inactive shell-side status payload.
 */
export async function unpublishSyphonOutput(): Promise<SyphonOutputStatusPayload> {
  return invoke<SyphonOutputStatusPayload>(UNPUBLISH_SYPHON_OUTPUT_COMMAND);
}
