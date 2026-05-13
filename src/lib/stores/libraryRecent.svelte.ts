// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.5 Lane LIB.C — recent-tracking store for the Library view.
//
// Operators want a quick way to find scenes they have loaded recently
// without scrolling the full grid. The Library RECENT filter chip +
// the "recent" sort axis both read off this store. `push(id)` is wired
// in the Library view's `$effect` watching
// `loadedSceneStore.snapshot?.id` so ANY successful scene load —
// from Library's double-click handler or from the Perform scene-list
// click-to-load surface — updates the recent list.
//
// The store is a class-instance singleton mirroring the
// `snapLabels.svelte.ts` Wave 2.4 PERF.F + `performVariant.svelte.ts`
// Wave 2.1 B.1 shapes; the `.svelte.ts` extension is required for
// runes-in-module per the `feedback_eslint_svelte_dot_ts_parser.md`
// convention.
//
// Defensive posture for localStorage access mirrors the
// `snapLabels.svelte.ts` precedent: read may throw on corrupt JSON
// (the parsed payload's shape is also unchecked — anything that
// is a string-array of length <= MAX_ENTRIES is accepted), and
// write may throw under privacy-mode browsers or quota-exhaustion.
// Both failure modes are swallowed; the in-memory state stays at
// the empty / pre-push default.

const STORAGE_KEY = 'library-recent';
const MAX_ENTRIES = 20;

class LibraryRecentStore {
  /** Ordered most-recent-first. Capped at MAX_ENTRIES (20) entries. */
  ids: string[] = $state([]);

  /** Hydrate from `localStorage`. Idempotent — callers can invoke
   *  from a view-mount `$effect`. Corrupt JSON / non-array / non-
   *  string payloads silently reset to the empty list. */
  hydrate(): void {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          const next: string[] = [];
          for (const entry of parsed) {
            if (typeof entry === 'string' && entry.length > 0) {
              next.push(entry);
              if (next.length >= MAX_ENTRIES) break;
            }
          }
          this.ids = next;
        }
      }
    } catch {
      // Swallow — corrupt storage, JSON parse failure, or
      // localStorage access denial. State stays at the empty default.
    }
  }

  /** Prepend `id` to the recent list, removing any prior occurrence
   *  so the most-recent-load wins. Caps the list at MAX_ENTRIES (20)
   *  and writes through to `localStorage` (best-effort). */
  push(id: string): void {
    if (!id) return;
    const filtered = this.ids.filter((existing) => existing !== id);
    const next = [id, ...filtered].slice(0, MAX_ENTRIES);
    this.ids = next;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    } catch {
      // Full / privacy-mode browsers can throw; swallow.
    }
  }

  /** True iff `id` is one of the tracked recents. */
  isRecent(id: string): boolean {
    return this.ids.includes(id);
  }

  /** Reset the store to the empty baseline (test harness). Does NOT
   *  clear `localStorage`; callers wanting a full wipe should remove
   *  the storage key themselves. */
  reset(): void {
    this.ids = [];
  }
}

export const libraryRecentStore = new LibraryRecentStore();
