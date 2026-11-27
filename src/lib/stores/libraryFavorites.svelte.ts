// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.5 Lane LIB.B — Library favorites store.
//
// Operators want to mark scene cards as "favorites" so the Library
// grid can be narrowed to the personal short-list without re-typing a
// search query each show. Engine has no scene-favorites concept (the
// `SceneCard` IPC payload is read-only metadata); the surface is
// purely a UI affordance backed by `localStorage` for cross-restart
// persistence — same posture as `snapLabels.svelte.ts` (Wave 2.4
// PERF.F).
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the `controlLock.svelte.ts` Wave 2.4 PERF.A precedent is the live
// `SvelteSet` shape this store mirrors (membership-as-state, change
// notifications without whole-collection reassignment).
//
// Defensive posture for `localStorage` access mirrors
// `snapLabels.svelte.ts`: read may throw on corrupt JSON, write may
// throw under privacy-mode browsers or quota exhaustion. Both
// failure modes are swallowed — the worst case is that the favorites
// set falls back to the empty default, which the operator can
// re-populate via the heart-icon affordance.

import { SvelteSet } from 'svelte/reactivity';

const STORAGE_KEY = 'library-favorites';

class LibraryFavoritesStore {
  /** Set of favorited scene card IDs. Membership ↔ favorite. */
  ids: SvelteSet<string> = new SvelteSet();

  /** Hydrate favorites from `localStorage`. Idempotent — callers can
   *  invoke from `$effect` on every view mount. Corrupt JSON silently
   *  resets to the empty set (caller never sees an error). */
  hydrate(): void {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          this.ids.clear();
          for (const id of parsed) {
            if (typeof id === 'string' && id.length > 0) {
              this.ids.add(id);
            }
          }
        }
      }
    } catch {
      // Swallow — corrupt storage, JSON parse failure, or
      // localStorage access denial. State stays at the empty default.
    }
  }

  /** Toggle favorite state for a single scene card id. Writes through
   *  to `localStorage`; the write is best-effort (privacy-mode +
   *  quota errors swallowed). */
  toggle(id: string): void {
    if (this.ids.has(id)) {
      this.ids.delete(id);
    } else {
      this.ids.add(id);
    }
    this.persist();
  }

  /** Read-only membership check. */
  isFavorite(id: string): boolean {
    return this.ids.has(id);
  }

  /** Reset the store to the no-favorites baseline (test harness).
   *  Does NOT clear `localStorage`; callers wanting a full wipe
   *  should remove the storage key themselves. */
  reset(): void {
    this.ids.clear();
  }

  private persist(): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify([...this.ids]));
    } catch {
      // Full / privacy-mode browsers can throw; swallow.
    }
  }
}

export const libraryFavoritesStore = new LibraryFavoritesStore();
