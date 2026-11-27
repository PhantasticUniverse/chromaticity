// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Discriminated union for IPC error shapes thrown across the Tauri command
 * boundary. The {@link IpcError} `kind` field distinguishes browser-mode
 * unavailability (no Tauri runtime injected — typical when the Vite dev
 * server is opened directly in a browser tab without `tauri dev`) from real
 * backend command failures and from unclassified errors.
 *
 * The backend keeps its `Result<T, String>` shape; this typing is purely
 * frontend. PR3 of Wave-11 introduces it alongside the `<ErrorMessage>`
 * primitive so consumers can hold a typed handle to the error and still
 * pass `.message` to the renderer.
 */
export type IpcError =
  | { kind: 'invoke_unavailable'; message: string }
  | { kind: 'command_failed'; command: string; message: string }
  | { kind: 'unknown'; message: string };

/**
 * Brittle-string sentinel for the V8 TypeError that surfaces when
 * `window.__TAURI_INTERNALS__.invoke` is undefined — the canonical signal
 * that a Tauri command was issued from a plain-browser context. The exact
 * message text is V8-specific; a future engine update could change the
 * format. Failure mode is graceful: a stale match falls through to
 * `command_failed` (when a command name is supplied) or `unknown`, and
 * `<ErrorMessage>` still renders the underlying message verbatim.
 */
const INVOKE_UNAVAILABLE_SIGNAL = "Cannot read properties of undefined (reading 'invoke')";

/**
 * Convert an unknown thrown value into an {@link IpcError}. Pass `command`
 * when the call site knows which Tauri command produced the error, so the
 * `command_failed` branch can carry it.
 */
export function toIpcError(e: unknown, command?: string): IpcError {
  const message = e instanceof Error ? e.message : String(e);
  if (message.includes(INVOKE_UNAVAILABLE_SIGNAL)) {
    return { kind: 'invoke_unavailable', message };
  }
  if (command !== undefined) {
    return { kind: 'command_failed', command, message };
  }
  return { kind: 'unknown', message };
}
