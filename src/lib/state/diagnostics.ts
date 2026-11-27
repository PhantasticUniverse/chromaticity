// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Read-only state-machine diagnostics registry (DX + AX + UX observability).
 *
 * Every explicit state machine registers a lightweight snapshot reader so
 * operators, developers, and agents can inspect the current state of every
 * subsystem in one deterministic JSON snapshot (`inspectAllStateMachines()`).
 */

export type StateMachineReader = () => Readonly<Record<string, unknown>>;

const registry = new Map<string, StateMachineReader>();

/** Register (or replace) a read-only snapshot reader for a named machine. */
export function registerStateMachine(name: string, reader: StateMachineReader): void {
  registry.set(name, reader);
}

/** Produce a deterministic, sorted snapshot of all registered state machines. */
export function inspectAllStateMachines(): Readonly<
  Record<string, Readonly<Record<string, unknown>>>
> {
  const out: Record<string, Readonly<Record<string, unknown>>> = {};
  const names = [...registry.keys()].sort();
  for (const name of names) {
    const reader = registry.get(name);
    if (reader) {
      out[name] = reader();
    }
  }
  return out;
}

/** Return the sorted list of registered state-machine names. */
export function registeredStateMachineNames(): readonly string[] {
  return [...registry.keys()].sort();
}
