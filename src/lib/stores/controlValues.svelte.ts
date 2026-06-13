// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Control-value cache store — the engine-committed value per scoped
// control key, hoisted out of `Perform.svelte` (where it was a
// component-local `$state` map, Wave 2.1 Lane A.2). `AppShell.svelte`
// routes through an `{#if}` ladder that destroys and recreates each
// view on every switch, so a component-local cache reset to schema
// defaults on Perform → Library → Perform while the engine kept the
// operator's committed values — and the next pointer-down then jumped
// the engine to wherever the stale visual sat. A module singleton
// survives the remount; widgets keep rendering the last committed
// value.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// `loadedScene.svelte.ts` + `controlLock.svelte.ts` are the live
// precedents for this shape (class-instance singleton + module-level
// export).
//
// **Key scheme (unchanged from the Lane A.2 cache).** Scene controls
// key as `{loadedSceneId}:{ControlId}` (the `fixture` sentinel when no
// scene is loaded); Meta Controls key as `meta:{ControlId}`. The
// scene-scoped string key keeps same-position controls isolated across
// scene loads without clearing the cross-scene meta namespace. Do NOT
// seed this cache from `get_control` read-backs at scene load: the
// engine's control namespace is raw `ControlId` shared across scenes,
// so a hydration sweep would resurrect the cross-scene value-0 leak
// the scoped key exists to prevent (an engine-side scene-scoped
// control map has to land first).
//
// **Dispatch sequence tokens.** During a fader / knob / XY drag,
// multiple `set_control` → `get_control` round-trips are in flight; a
// read-back issued for position N can resolve after the optimistic
// write for position N+1 and rubber-band the widget. `beginDispatch`
// hands the caller a per-key monotonic token and `isCurrentDispatch`
// tells it whether its read-back is still the latest — stale
// read-backs are dropped while the final dispatch's read-back (the
// engine clamp / quantisation correction) still lands. The counters
// are deliberately plain (non-reactive) bookkeeping.

import { loadedSceneStore } from './loadedScene.svelte';
import { META_CONTROL_ID_BASE } from '../data/perform-fixtures';

/** Cache entry: scalar controls store a `[0, 1]`-clamped float
 * (toggles encode as `0` / `1` — `ControlValue` is `f32` on the
 * wire); XY controls store a `[x, y]` pair. */
export type ControlValueEntry = number | readonly [number, number];
export type ControlValueCache = Record<string, ControlValueEntry>;

/** Scene-namespace scope when no scene is loaded (fixture-driven UI). */
const SCENELESS_CONTROL_SCOPE = 'fixture';

class ControlValuesStore {
  /** Last engine-committed (or optimistic in-flight) value per scoped key. */
  values: ControlValueCache = $state({});

  /** Per-key dispatch counters — plain field, intentionally non-reactive. */
  private dispatchSeq: Record<string, number> = {};

  /** Scoped cache key for a control id: `meta:{id}` for the Meta
   * Controls namespace, `{sceneId}:{id}` otherwise. */
  key(id: number): string {
    if (id >= META_CONTROL_ID_BASE) return `meta:${id}`;
    return `${loadedSceneStore.snapshot?.id ?? SCENELESS_CONTROL_SCOPE}:${id}`;
  }

  get(key: string): ControlValueEntry | undefined {
    return this.values[key];
  }

  set(key: string, value: ControlValueEntry): void {
    this.values = { ...this.values, [key]: value };
  }

  /** Claim the next dispatch token for `key` (call before the IPC). */
  beginDispatch(key: string): number {
    const seq = (this.dispatchSeq[key] ?? 0) + 1;
    this.dispatchSeq[key] = seq;
    return seq;
  }

  /** `true` while `seq` is still the newest dispatch for `key` —
   * apply the read-back; `false` means a newer dispatch owns the key
   * and the read-back is stale. */
  isCurrentDispatch(key: string, seq: number): boolean {
    return this.dispatchSeq[key] === seq;
  }

  /** Reset to the empty-cache baseline (test harness). */
  reset(): void {
    this.values = {};
    this.dispatchSeq = {};
  }
}

export const controlValuesStore = new ControlValuesStore();
