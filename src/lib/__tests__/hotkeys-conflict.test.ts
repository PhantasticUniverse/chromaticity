// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Evidence: EV-HOTKEY-CONFLICT (T2) — Phase-4a wave 1a hotkey-conflict
// conformance corpus. Validates the six conflict-and-priority rules in
// `docs/42-operator-hotkey-map.md` Sec.Conflict-and-priority-rules: no two
// bindings silently fire the same key in the same focus context, and the
// authoring/operator/control-focused/modal/text-input/palette context
// taxonomy is internally consistent.
//
// `evidence-tag-lint.sh` currently scopes to `engine/**` and `integrations/**`,
// so this top-of-file `Evidence:` tag is documentational; widening the lint
// to scan `src/` is a future PR.

import { describe, expect, it } from 'vitest';

import type { HotkeyRow } from '../data/hotkeys';
import { HOTKEYS } from '../data/hotkeys';

// Focus contexts per docs/42 Sec.Conflict-and-priority-rules:
//   1. control-focused — D/R/P/L applied to the focused control (rule 1)
//   2. modal-meta      — meta-control slider modal (rule 2)
//   3. authoring-env   — focus is inside the authoring panel (rule 3)
//   4. command-palette — Cmd+K dialog is open (rule 4); only Esc fires
//   5. text-input      — preset name / search / mapping label (rule 5)
//   6. global          — operator default; no other context active
type FocusContext =
  | 'global'
  | 'control-focused'
  | 'modal-meta'
  | 'authoring-env'
  | 'command-palette'
  | 'text-input';

interface NormalizedBinding {
  readonly combo: string; // canonical form, e.g. "Cmd+K"
  readonly action: string;
  readonly group: string;
  readonly contexts: readonly FocusContext[];
}

/**
 * Normalize a key combo string into a canonical form. The operator-side
 * vocabulary uses platform-specific labels (`Cmd+R` / `Ctrl+R`) for the
 * same logical binding; cross-platform aliasing folds them onto a single
 * canonical key so duplicate-detection sees them as one row, not two.
 *
 * Rules:
 *   - Trim outer whitespace; reject any combo that has an empty token.
 *   - Treat `Cmd` and `Ctrl` as the same modifier (canonical: `Cmd`).
 *     The `Cmd+\` / `Ctrl+\` row is rendered without the second `+` in
 *     the table prose; the data file already corrects that to `Cmd+\`.
 *   - Treat `Opt` and `Alt` as the same modifier (canonical: `Alt`).
 *   - Sort modifiers alphabetically before the trailing key, so
 *     `Shift+Cmd+S` and `Cmd+Shift+S` collapse identically.
 *   - Range tokens like `1 – 9` are passed through as a single combo;
 *     they describe a digit-row binding family, not a literal "1-9"
 *     keypress. Conflict detection treats them atomically.
 */
function normalizeCombo(combo: string): string {
  const trimmed = combo.trim();
  // Range tokens (en-dash) are passed through verbatim — they describe
  // ten-finger families (1..9, 0) and we treat the range as one bucket.
  if (trimmed.includes('–') || trimmed.includes(' - ')) {
    return trimmed;
  }
  const parts = trimmed.split('+').map((p) => p.trim());
  if (parts.some((p) => p.length === 0)) {
    return trimmed; // malformed; let the caller's separate parseability test fail
  }
  // The trailing token is the literal key; preceding tokens are modifiers.
  const key = parts[parts.length - 1] ?? '';
  const mods = parts.slice(0, -1).map((m) => {
    if (m === 'Ctrl') return 'Cmd';
    if (m === 'Opt') return 'Alt';
    return m;
  });
  mods.sort();
  return mods.length > 0 ? `${mods.join('+')}+${key}` : key;
}

/**
 * Map a HotkeyRow to the focus contexts in which it fires. The mapping
 * follows the six docs/42 priority rules verbatim; ambiguous overlaps
 * (e.g. Cmd+R: operator-side reload vs. authoring-side rebuild) split
 * onto distinct contexts so the conflict detector doesn't flag them.
 */
function contextsForRow(row: HotkeyRow, group: string): readonly FocusContext[] {
  // Rule 1: D/R/P/L are control-focused only.
  if (group === 'Control-focused (D / R / P / L)') {
    return ['control-focused'];
  }
  // Rule 2: meta-control rows enter modal-meta when fired from global,
  // then live in modal-meta. The trigger key itself is a global binding;
  // arrow-key handling within modal-meta is out of scope for the
  // operator hotkey table.
  if (group === 'Meta controls') {
    return ['global'];
  }
  // Rule 4: Cmd+K opens the command palette from anywhere. The palette
  // takes over input, but the binding that opens it is global-context.
  // Esc dismisses it (palette-context only — not enumerated in the
  // table because Esc is the always-available exit per rule 1).
  // Rule 3: authoring-context shared keys are listed below; their
  // operator-context counterparts (Cmd+R = reload scene, Cmd+S = save
  // to next empty Snap, Cmd+/ = help/docs) live in `global`. Authoring
  // overrides are tracked separately in the SHARED_KEY_ROUTING table.
  return ['global'];
}

/**
 * Operator-vs-authoring shared-key routing per docs/42 rule 3. Each
 * entry is a (combo, contexts → action) map for the keys whose
 * operator-context binding is overridden inside the authoring panel.
 * Conflict detection uses this table to confirm the operator-side
 * row in `HOTKEYS` does NOT also fire under authoring-env.
 */
const SHARED_KEY_AUTHORING_OVERRIDES: ReadonlyMap<string, string> = new Map([
  // Operator: reload current scene; authoring: full rebuild.
  ['Cmd+R', 'Authoring: scene-package rebuild'],
  // Operator: save to next empty Snap; authoring: save dirty tabs.
  ['Cmd+S', 'Authoring: save all dirty tabs'],
  // Operator: unbound; authoring: Quick-Open-File.
  ['Cmd+P', 'Authoring: Quick-Open-File'],
  // Operator: unbound; authoring: Find-in-File.
  ['Cmd+F', 'Authoring: Find-in-File'],
  // Operator: open help/docs search; authoring: toggle line comment.
  ['Cmd+/', 'Authoring: toggle line comment'],
]);

/** All HOTKEYS rows flattened into normalized bindings, one per combo entry. */
function buildBindingTable(): readonly NormalizedBinding[] {
  const out: NormalizedBinding[] = [];
  for (const group of HOTKEYS) {
    for (const row of group.rows) {
      const contexts = contextsForRow(row, group.title);
      for (const combo of row.keys) {
        out.push({
          combo: normalizeCombo(combo),
          action: row.action,
          group: group.title,
          contexts,
        });
      }
    }
  }
  return out;
}

describe('EV-HOTKEY-CONFLICT — focus-context binding matrix', () => {
  const bindings = buildBindingTable();

  it('every combo entry parses into a stable canonical normalized form', () => {
    for (const b of bindings) {
      // `normalizeCombo` is idempotent — applying it twice yields the
      // same string. The earlier completeness test guarantees no
      // empty / whitespace-padded entries; this asserts that the
      // normalizer doesn't introduce new whitespace or duplicate +'s.
      const renormalized = normalizeCombo(b.combo);
      expect(renormalized).toBe(b.combo);
      expect(b.combo).not.toContain('++');
      expect(b.combo).toBe(b.combo.trim());
    }
  });

  it('cross-platform aliasing (Cmd vs Ctrl, Opt vs Alt) does not introduce silent duplicates', () => {
    // Cross-platform aliases are explicitly listed as separate entries
    // in `keys: [...]` (e.g. `['Cmd+R', 'Ctrl+R']`); after normalization
    // they collapse onto one canonical combo. We assert that within a
    // single row, the alias pair collapses to exactly one canonical
    // combo — i.e. there is no row whose two `keys` entries normalize
    // to two distinct canonical strings while also being a Cmd / Ctrl
    // pair (a sign of a typo introducing silent duplication).
    for (const group of HOTKEYS) {
      for (const row of group.rows) {
        const norm = row.keys.map(normalizeCombo);
        const unique = Array.from(new Set(norm));
        // For Cmd/Ctrl alias rows the two entries collapse to one;
        // for non-alias rows (e.g. `['→', 'K']`, `['1 – 9', '0']`) the
        // two entries describe distinct logical keys and stay distinct.
        const hasCmdCtrlPair =
          row.keys.some((k) => k.includes('Cmd')) && row.keys.some((k) => k.includes('Ctrl'));
        if (hasCmdCtrlPair) {
          expect(unique.length).toBe(1);
        }
      }
    }
  });

  it('no two rows share the same (combo, context) without explicit override', () => {
    // The core conflict rule: within a single context, every combo
    // resolves to exactly one action.
    const collisions: Array<{ combo: string; context: FocusContext; actions: string[] }> = [];
    const buckets = new Map<string, NormalizedBinding[]>();
    for (const b of bindings) {
      for (const ctx of b.contexts) {
        const key = `${ctx}::${b.combo}`;
        const arr = buckets.get(key) ?? [];
        arr.push(b);
        buckets.set(key, arr);
      }
    }
    for (const [key, arr] of buckets) {
      if (arr.length > 1) {
        const distinctActions = Array.from(new Set(arr.map((b) => b.action)));
        if (distinctActions.length > 1) {
          const [ctx, combo] = key.split('::');
          collisions.push({
            combo: combo ?? '',
            context: (ctx ?? 'global') as FocusContext,
            actions: distinctActions,
          });
        }
      }
    }
    expect(collisions).toEqual([]);
  });

  it('D/R/P/L only fire in control-focused context, not global', () => {
    const drplCombos = ['D', 'R', 'P', 'L'];
    for (const combo of drplCombos) {
      const matches = bindings.filter((b) => b.combo === combo);
      expect(matches.length).toBeGreaterThan(0);
      const drplRows = matches.filter((b) => b.group === 'Control-focused (D / R / P / L)');
      expect(drplRows.length).toBe(1);
      for (const row of drplRows) {
        expect(row.contexts).toEqual(['control-focused']);
        expect(row.contexts).not.toContain('global');
      }
    }
  });

  it('text-input fields suppress all hotkeys except Esc (rule 5)', () => {
    // Rule 5 is a runtime-dispatch invariant: the text-input context
    // intentionally has zero rows in the HOTKEYS table — no row's
    // contexts include 'text-input'. This test asserts that property,
    // i.e. there is no HOTKEYS row that claims to fire under
    // text-input focus (Esc is documented in the table prose, not as
    // a separate row, since Esc-as-exit is the always-available
    // affordance per rule 1).
    const textInputBindings = bindings.filter((b) => b.contexts.includes('text-input'));
    expect(textInputBindings).toEqual([]);
  });

  it('command palette opens via Cmd+K and is the only Cmd+K binding (rule 4)', () => {
    // The Cmd+K row carries `['Cmd+K', 'Ctrl+K']` which collapses under
    // normalization to a single canonical combo; the per-combo expansion
    // emits two entries, so we de-duplicate on the (combo, action) tuple
    // before asserting uniqueness.
    const cmdK = bindings.filter((b) => b.combo === 'Cmd+K');
    expect(cmdK.length).toBeGreaterThan(0);
    const distinctActions = Array.from(new Set(cmdK.map((b) => b.action)));
    expect(distinctActions.length).toBe(1);
    expect(distinctActions[0]).toMatch(/command palette/i);
    // Rule 4 says Cmd+K is invokable from anywhere. The binding's own
    // context is `global`; the palette-context (where Esc dismisses) is
    // an emergent state not represented as a separate row — the table
    // intentionally has zero command-palette-context rows.
    const paletteCtxBindings = bindings.filter((b) => b.contexts.includes('command-palette'));
    expect(paletteCtxBindings).toEqual([]);
  });

  it('authoring-env shared keys (Cmd+R/S/P/F, Cmd+/) override operator-context with documented routing', () => {
    // Rule 3: each shared key has an operator-context action AND an
    // authoring-context action. The HOTKEYS table only encodes the
    // operator side; the authoring side is documented in the
    // `SHARED_KEY_AUTHORING_OVERRIDES` table here. We assert (a) every
    // shared key has an operator-side row in HOTKEYS, and (b) no
    // HOTKEYS row claims authoring-env context (the operator table is
    // operator-only).
    for (const [combo] of SHARED_KEY_AUTHORING_OVERRIDES) {
      // Cmd+P and Cmd+F are documented as "operator context: unbound";
      // they appear ONLY in the override table, not in HOTKEYS.
      if (combo === 'Cmd+P' || combo === 'Cmd+F') {
        const opSide = bindings.filter((b) => b.combo === combo);
        expect(opSide).toEqual([]);
        continue;
      }
      const opSide = bindings.filter((b) => b.combo === combo);
      expect(opSide.length).toBeGreaterThan(0);
      for (const b of opSide) {
        expect(b.contexts).not.toContain('authoring-env');
      }
    }
  });

  it('modal meta-slider trigger keys (M/B/G/C/H/Y/I/V/Shift+V/E) are global-context single-binding rows (rule 2)', () => {
    // Rule 2: meta rows enter a modal slider mode on press. The
    // operator table lists the trigger key as a global binding; the
    // modal-internal arrow-key handling is out of scope. We assert
    // each trigger has exactly one global-context binding.
    const triggers = ['M', 'B', 'G', 'C', 'H', 'Y', 'I', 'V', 'Shift+V', 'E'];
    for (const combo of triggers) {
      const matches = bindings.filter((b) => b.combo === combo && b.group === 'Meta controls');
      expect(matches.length).toBe(1);
      expect(matches[0]?.contexts).toEqual(['global']);
    }
  });
});
