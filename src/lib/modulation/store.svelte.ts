// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Live visual modulation store (Phase 6) — explicit discriminated state
// machine (`stopped` | `running`) managing modulation routes, localStorage
// persistence, deterministic stepping, and throttled `setControl` dispatch.

import { SvelteMap } from 'svelte/reactivity';

import {
  CONTROL_INDEX_MAX,
  CONTROL_INDEX_MIN,
  evaluateModulationGraph,
  extractAudioInputs,
  MOD_OPS,
  MOD_SOURCES,
  type ModOp,
  type ModRoute,
  type ModSourceId,
} from './engine';
import { setControl } from '../ipc';
import { registerStateMachine } from '../state';
import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

export type PatchModulationState =
  | { readonly kind: 'stopped' }
  | { readonly kind: 'running'; readonly tickCount: number };

const STORAGE_KEY = 'chromaticity.patchModulation.routes.v1';
const DISPATCH_EPSILON = 0.002;

export const DEFAULT_MOD_ROUTES: readonly ModRoute[] = [
  {
    id: 'mod-bass-ctrl0',
    source: 'audio.bass',
    targetControlIndex: 0,
    op: 'gain',
    amount: 1.2,
    bias: 0,
    smoothing: 0.25,
    enabled: true,
  },
  {
    id: 'mod-rms-ctrl1',
    source: 'audio.rms',
    targetControlIndex: 1,
    op: 'smooth',
    amount: 1.0,
    bias: 0,
    smoothing: 0.7,
    enabled: true,
  },
  {
    id: 'mod-lfo-ctrl2',
    source: 'lfo.sine',
    targetControlIndex: 2,
    op: 'passthrough',
    amount: 1.0,
    bias: 0,
    smoothing: 0,
    enabled: true,
  },
] as const;

function isModSourceId(value: unknown): value is ModSourceId {
  return typeof value === 'string' && (MOD_SOURCES as readonly string[]).includes(value);
}

function isModOp(value: unknown): value is ModOp {
  return typeof value === 'string' && (MOD_OPS as readonly string[]).includes(value);
}

function isValidStoredRoute(entry: unknown): entry is ModRoute {
  if (typeof entry !== 'object' || entry === null) return false;
  const r = entry as Record<string, unknown>;
  return (
    typeof r.id === 'string' &&
    r.id.length > 0 &&
    isModSourceId(r.source) &&
    typeof r.targetControlIndex === 'number' &&
    Number.isInteger(r.targetControlIndex) &&
    r.targetControlIndex >= CONTROL_INDEX_MIN &&
    r.targetControlIndex <= CONTROL_INDEX_MAX &&
    isModOp(r.op) &&
    typeof r.amount === 'number' &&
    Number.isFinite(r.amount) &&
    typeof r.bias === 'number' &&
    Number.isFinite(r.bias) &&
    typeof r.smoothing === 'number' &&
    Number.isFinite(r.smoothing) &&
    typeof r.enabled === 'boolean'
  );
}

function loadPersistedRoutes(): ModRoute[] {
  try {
    if (typeof globalThis.localStorage === 'undefined') {
      return DEFAULT_MOD_ROUTES.map((r) => ({ ...r }));
    }
    const raw = globalThis.localStorage.getItem(STORAGE_KEY);
    if (raw === null) return DEFAULT_MOD_ROUTES.map((r) => ({ ...r }));
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return DEFAULT_MOD_ROUTES.map((r) => ({ ...r }));
    const valid = parsed.filter(isValidStoredRoute);
    return valid.length > 0
      ? valid.map((r) => ({ ...r }))
      : DEFAULT_MOD_ROUTES.map((r) => ({ ...r }));
  } catch {
    return DEFAULT_MOD_ROUTES.map((r) => ({ ...r }));
  }
}

function persistRoutes(routes: readonly ModRoute[]): void {
  try {
    if (typeof globalThis.localStorage === 'undefined') return;
    globalThis.localStorage.setItem(STORAGE_KEY, JSON.stringify(routes));
  } catch {
    // Ignore storage quota or sandbox errors.
  }
}

export class PatchModulationStore {
  /** Explicit discriminated lifecycle state (`stopped` | `running`). */
  machine: PatchModulationState = $state({ kind: 'stopped' });

  /** Configured modulation routes. */
  routes: ModRoute[] = $state(loadPersistedRoutes());

  /** Latest evaluated output per control index (`0..7`). */
  lastOutputs: ReadonlyMap<number, number> = $state(new SvelteMap());

  /** Latest evaluated value per route id. */
  lastRouteValues: ReadonlyMap<string, number> = $state(new SvelteMap());

  #emaState = new SvelteMap<string, number>();
  #lastDispatched = new SvelteMap<number, number>();
  #rafHandle: number | null = null;
  #startTimeMs = 0;
  #nextId = 1;

  addRoute(partial?: Partial<Omit<ModRoute, 'id'>>): ModRoute {
    const id = `mod-route-${Date.now()}-${this.#nextId++}`;
    const route: ModRoute = {
      id,
      source: partial?.source ?? 'lfo.triangle',
      targetControlIndex: partial?.targetControlIndex ?? 3,
      op: partial?.op ?? 'gain',
      amount: partial?.amount ?? 1.0,
      bias: partial?.bias ?? 0,
      smoothing: partial?.smoothing ?? 0,
      enabled: partial?.enabled ?? true,
    };
    this.routes = [...this.routes, route];
    persistRoutes(this.routes);
    return route;
  }

  removeRoute(id: string): void {
    this.routes = this.routes.filter((r) => r.id !== id);
    this.#emaState.delete(id);
    persistRoutes(this.routes);
  }

  toggleRoute(id: string): void {
    this.routes = this.routes.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r));
    persistRoutes(this.routes);
  }

  updateRoute(id: string, patch: Partial<Omit<ModRoute, 'id'>>): void {
    this.routes = this.routes.map((r) => (r.id === id ? { ...r, ...patch } : r));
    persistRoutes(this.routes);
  }

  resetDefaults(): void {
    this.routes = DEFAULT_MOD_ROUTES.map((r) => ({ ...r }));
    this.#emaState.clear();
    this.#lastDispatched.clear();
    persistRoutes(this.routes);
  }

  start(): void {
    if (this.machine.kind === 'running') return;
    this.#emaState.clear();
    this.#lastDispatched.clear();
    this.machine = { kind: 'running', tickCount: 0 };
    this.#startTimeMs = typeof performance !== 'undefined' ? performance.now() : Date.now();

    if (typeof requestAnimationFrame === 'function') {
      const tick = (nowMs: number): void => {
        if (this.machine.kind !== 'running') {
          this.#rafHandle = null;
          return;
        }
        const elapsedSecs = Math.max(0, (nowMs - this.#startTimeMs) / 1000);
        this.step(elapsedSecs);
        this.#rafHandle = requestAnimationFrame(tick);
      };
      this.#rafHandle = requestAnimationFrame(tick);
    }
  }

  stop(): void {
    if (this.#rafHandle !== null && typeof cancelAnimationFrame === 'function') {
      cancelAnimationFrame(this.#rafHandle);
    }
    this.#rafHandle = null;
    this.machine = { kind: 'stopped' };
  }

  /**
   * Deterministically evaluate one modulation frame at `elapsedSecs` against
   * `audioSnapshotStore.snapshot`, update live readouts, and dispatch changed
   * controls via `setControl`.
   */
  step(elapsedSecs: number): Map<number, number> {
    const audio = extractAudioInputs(audioSnapshotStore.snapshot);
    const result = evaluateModulationGraph(this.routes, { elapsedSecs, audio }, this.#emaState);
    this.#emaState = new SvelteMap(result.nextState);
    this.lastOutputs = result.outputs;
    this.lastRouteValues = result.routeValues;

    if (this.machine.kind === 'running') {
      this.machine = { kind: 'running', tickCount: this.machine.tickCount + 1 };
    }

    for (const [controlIndex, value] of result.outputs.entries()) {
      const prev = this.#lastDispatched.get(controlIndex);
      if (prev === undefined || Math.abs(value - prev) >= DISPATCH_EPSILON) {
        this.#lastDispatched.set(controlIndex, value);
        void setControl(controlIndex, value).catch(() => {
          // Swallow IPC errors when running outside a live Tauri backend.
        });
      }
    }

    return result.outputs;
  }
}

export const patchModulationStore = new PatchModulationStore();

registerStateMachine('patchModulation', () => ({
  kind: patchModulationStore.machine.kind,
  tickCount:
    patchModulationStore.machine.kind === 'running' ? patchModulationStore.machine.tickCount : 0,
  routeCount: patchModulationStore.routes.length,
  enabledRouteCount: patchModulationStore.routes.filter((r) => r.enabled).length,
}));
