// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Pure discriminated-union state machine for asynchronous IPC resources.
 *
 * Eliminates the classic `{ isLoading, error, data }` bag-of-booleans pattern
 * and prevents out-of-order response races via a monotonic `requestId`.
 * Existing stores expose `isLoading` / `error` / `data` as `$derived`
 * projections over this state so all existing callers stay 100% compatible.
 */
export type AsyncResource<T> =
  | { readonly kind: 'idle'; readonly data: T }
  | { readonly kind: 'loading'; readonly requestId: number; readonly data: T }
  | { readonly kind: 'ready'; readonly requestId: number; readonly data: T }
  | {
      readonly kind: 'error';
      readonly requestId: number;
      readonly data: T;
      readonly error: string;
    };

/** Create an initial idle resource holding `initialData`. */
export function idleResource<T>(initialData: T): AsyncResource<T> {
  return { kind: 'idle', data: initialData };
}

/**
 * Transition into `'loading'` with a fresh monotonic `requestId`,
 * preserving the previously displayed `data` until the request settles.
 */
export function beginRequest<T>(current: AsyncResource<T>, requestId: number): AsyncResource<T> {
  return { kind: 'loading', requestId, data: current.data };
}

/**
 * Settle an in-flight request with `data`. If a newer request has already
 * superseded `requestId` (or the store was reset to `'idle'`), the stale
 * response is dropped and `current` is returned unchanged.
 */
export function succeedRequest<T>(
  current: AsyncResource<T>,
  requestId: number,
  data: T,
): AsyncResource<T> {
  if (current.kind !== 'loading' || current.requestId !== requestId) {
    return current;
  }
  return { kind: 'ready', requestId, data };
}

/**
 * Fail an in-flight request with `error`. Drops stale failures whose
 * `requestId` no longer matches the active `'loading'` request.
 * Pass `fallbackData` when the store's contract clears previous data on error.
 */
export function failRequest<T>(
  current: AsyncResource<T>,
  requestId: number,
  error: string,
  fallbackData?: T,
): AsyncResource<T> {
  if (current.kind !== 'loading' || current.requestId !== requestId) {
    return current;
  }
  return {
    kind: 'error',
    requestId,
    data: fallbackData !== undefined ? fallbackData : current.data,
    error,
  };
}

/** Back-compat projection: `true` iff a request is currently in flight. */
export function isResourceLoading<T>(resource: AsyncResource<T>): boolean {
  return resource.kind === 'loading';
}

/** Back-compat projection: error string when in `'error'`, else `null`. */
export function resourceError<T>(resource: AsyncResource<T>): string | null {
  return resource.kind === 'error' ? resource.error : null;
}

/** Format an unknown thrown value into a deterministic operator-facing message. */
export function formatErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}
