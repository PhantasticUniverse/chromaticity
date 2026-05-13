// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.4 Lane PERF.F — SnapBar slot-label store.
//
// Operators want to name SnapBar slots ("Intro" / "Drop" / "Outro") so
// the bar is actually navigable mid-show. Engine snap state stays binary
// saved / unsaved per slot per the Phase-3 `EV-PRESET-PLAYLIST` posture;
// disk persistence (Settings TOML store) is a Wave 2.5+ concern if user
// feedback warrants. Until then, labels are localStorage-only.
//
// The store is a class-instance singleton mirroring the
// `performVariant.svelte.ts` Wave 2.1 B.1 + `snap.svelte.ts` Wave 2.1
// A.3 shapes; the `.svelte.ts` extension is required for runes-in-
// module per the `feedback_eslint_svelte_dot_ts_parser.md` convention.
//
// Defensive posture for localStorage access mirrors the
// `performVariant.svelte.ts` precedent: read may throw on corrupt JSON
// (the parsed payload's shape is also unchecked — anything that looks
// like `Record<string, string>` is accepted; bogus keys outside the
// 1..8 SnapBar surface harmlessly carry on as dead entries until next
// `set()`), and write may throw under privacy-mode browsers or quota-
// exhaustion. Both failure modes are swallowed.

const STORAGE_KEY = 'snap-labels';

class SnapLabelsStore {
  /** Per-slot label map keyed by 1-indexed SnapBar slot id. Empty when
   *  no labels are set; an entry is removed by `set(slot, '')`. */
  labels: Record<number, string> = $state({});

  /** Hydrate labels from `localStorage`. Idempotent — callers can
   *  invoke from `$effect` on every widget mount. Corrupt JSON
   *  silently resets to the empty map (caller never sees an error). */
  hydrate(): void {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
          const next: Record<number, string> = {};
          for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
            const slot = Number(key);
            if (Number.isFinite(slot) && typeof value === 'string' && value.length > 0) {
              next[slot] = value;
            }
          }
          this.labels = next;
        }
      }
    } catch {
      // Swallow — corrupt storage, JSON parse failure, or
      // localStorage access denial. State stays at the empty default.
    }
  }

  /** Set the label for `slot`. An empty / whitespace-only `label`
   *  removes the entry. Writes through to `localStorage`; the write
   *  is best-effort (privacy-mode + quota errors swallowed). */
  set(slot: number, label: string): void {
    const next = { ...this.labels };
    const trimmed = label.trim();
    if (trimmed) {
      next[slot] = trimmed;
    } else {
      delete next[slot];
    }
    this.labels = next;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    } catch {
      // Full / privacy-mode browsers can throw; swallow.
    }
  }

  /** Read the label for `slot`. Returns the persisted label if one is
   *  set, else the `fallback` string if supplied, else the slot id as
   *  a string. */
  label(slot: number, fallback?: string): string {
    return this.labels[slot] ?? fallback ?? String(slot);
  }

  /** Reset the store to the empty-label baseline (test harness). Does
   *  NOT clear `localStorage`; callers wanting a full wipe should
   *  remove the storage key themselves. */
  reset(): void {
    this.labels = {};
  }
}

export const snapLabelsStore = new SnapLabelsStore();
