// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Vitest jsdom environment setup.
 *
 * On Node 25+, Node's built-in `globalThis.localStorage` is exposed as a
 * non-functional stub unless `--localstorage-file` is passed, which shadows
 * jsdom's `localStorage` and breaks tests calling `localStorage.removeItem()`.
 * Install a standard in-memory `Storage` implementation only when
 * `localStorage.getItem` / `removeItem` are missing.
 */
if (
  typeof globalThis.localStorage === 'undefined' ||
  typeof globalThis.localStorage.getItem !== 'function' ||
  typeof globalThis.localStorage.removeItem !== 'function'
) {
  const backing = new Map<string, string>();
  const memoryStorage: Storage = {
    get length(): number {
      return backing.size;
    },
    clear(): void {
      backing.clear();
    },
    getItem(key: string): string | null {
      return backing.has(key) ? (backing.get(key) ?? null) : null;
    },
    key(index: number): string | null {
      return [...backing.keys()][index] ?? null;
    },
    removeItem(key: string): void {
      backing.delete(key);
    },
    setItem(key: string, value: string): void {
      backing.set(key, String(value));
    },
  };
  Object.defineProperty(globalThis, 'localStorage', {
    value: memoryStorage,
    configurable: true,
    writable: true,
  });
}
