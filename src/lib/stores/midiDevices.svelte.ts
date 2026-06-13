// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// MIDI device-allowlist singleton store. Hoists the Devices-tab
// allowlist / ever-seen state out of `MIDI.svelte` so an operator's
// explicit "disallow" survives view switches — AppShell routes via an
// `{#if routeStore.current === ...}` ladder, so component-local sets
// die on every route change and the auto-enable-on-first-sight rule
// would silently re-enable a deliberately-disabled device on remount.
//
// The store is a class-instance singleton mirroring the
// `libraryRecent.svelte.ts` / `snapLabels.svelte.ts` shapes; the
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` convention.
//
// Semantics (unchanged from the pre-hoist component contract):
//   - `allowed`: device ids the operator has enabled. `SvelteSet` so
//     mutating add/delete drives reactive template updates.
//   - ever-seen: every device id ever surfaced by enumeration. A fresh
//     (never-seen) device auto-enables on first sight; a device the
//     operator explicitly disabled stays in ever-seen but absent from
//     `allowed`, so refreshes / remounts do NOT re-enable it.
//
// Persistence: both sets write through to `localStorage` (best-effort,
// try-catch) so the disallow also survives app restarts. Hydration runs
// at construction with a schema-validated parse — corrupt JSON, a
// non-object payload, non-array fields, or non-string entries are
// silently discarded so a poisoned storage key can never crash module
// init (mirrors the `libraryRecent.svelte.ts` defensive posture).

import { SvelteSet } from 'svelte/reactivity';

/** localStorage key for the persisted allowlist + ever-seen sets. */
export const MIDI_DEVICE_ALLOWLIST_KEY = 'chromaticity.midi.device-allowlist';

class MidiDeviceStore {
  /** Device ids the operator has enabled. Reactive (SvelteSet). */
  allowed = new SvelteSet<string>();

  /** Every device id enumeration has ever surfaced. Non-reactive — no
   *  template reads it; it only gates the auto-enable rule. */
  #everSeen = new Set<string>();

  constructor() {
    this.#hydrate();
  }

  /** True iff `id` is currently allowlisted. Reactive in templates. */
  isAllowed(id: string): boolean {
    return this.allowed.has(id);
  }

  /** Record the current enumeration result: any id never seen before
   *  (across the app lifetime + persisted history) auto-enables; ids
   *  the operator explicitly disabled stay disabled. */
  markSeen(ids: readonly string[]): void {
    let changed = false;
    for (const id of ids) {
      if (id.length === 0 || this.#everSeen.has(id)) continue;
      this.#everSeen.add(id);
      this.allowed.add(id);
      changed = true;
    }
    if (changed) this.#persist();
  }

  /** Flip the allow state for `id` (operator toggle). Marks the id as
   *  seen so a disallow sticks across refreshes and remounts. */
  toggle(id: string): void {
    this.#everSeen.add(id);
    if (this.allowed.has(id)) {
      this.allowed.delete(id);
    } else {
      this.allowed.add(id);
    }
    this.#persist();
  }

  /** Reset to the empty baseline (test harness). Does NOT clear
   *  `localStorage`; callers wanting a full wipe should remove
   *  [`MIDI_DEVICE_ALLOWLIST_KEY`] themselves. */
  reset(): void {
    this.allowed.clear();
    this.#everSeen.clear();
  }

  #hydrate(): void {
    try {
      if (typeof localStorage === 'undefined') return;
      const raw = localStorage.getItem(MIDI_DEVICE_ALLOWLIST_KEY);
      if (raw === null) return;
      const parsed: unknown = JSON.parse(raw);
      if (typeof parsed !== 'object' || parsed === null) return;
      const record = parsed as { allowed?: unknown; everSeen?: unknown };
      if (!Array.isArray(record.allowed) || !Array.isArray(record.everSeen)) return;
      for (const id of record.everSeen) {
        if (typeof id === 'string' && id.length > 0) this.#everSeen.add(id);
      }
      for (const id of record.allowed) {
        if (typeof id === 'string' && id.length > 0) {
          this.allowed.add(id);
          // Invariant: allowed ⊆ ever-seen, even if the stored everSeen
          // array was truncated or partially corrupt.
          this.#everSeen.add(id);
        }
      }
    } catch {
      // Swallow — corrupt storage, JSON parse failure, or localStorage
      // access denial. State stays at the empty default and the next
      // enumeration re-seeds via the auto-enable rule.
    }
  }

  #persist(): void {
    try {
      if (typeof localStorage === 'undefined') return;
      localStorage.setItem(
        MIDI_DEVICE_ALLOWLIST_KEY,
        JSON.stringify({ allowed: [...this.allowed], everSeen: [...this.#everSeen] }),
      );
    } catch {
      // Full / privacy-mode browsers can throw; swallow — the in-memory
      // sets still carry the session state.
    }
  }
}

export const midiDeviceStore = new MidiDeviceStore();
