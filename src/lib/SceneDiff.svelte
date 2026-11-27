<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  /*
   * Side-by-side scene diff for the W21 PR3 "compare a scene" workflow.
   *
   * Renders two source files line-by-line with a simple Longest-
   * Common-Subsequence (LCS) classifier:
   *   - unchanged lines render in both columns at the same row
   *   - removed lines (left only) get a danger tint, blank row right
   *   - added lines (right only) get a success tint, blank row left
   *
   * The LCS implementation here is the classic O(n*m) DP table with
   * an O(n+m) backtrack. Scene files are bounded (~100s of lines), so
   * the cost is negligible and the implementation is the simplest
   * thing that works correctly. This is not a high-fidelity diff
   * (no in-line word-level highlighting) but the brief calls for
   * "simple line-by-line highlighting", which this is.
   *
   * Phase-3+/4a will replace the fixture-driven comparison source
   * with real scene-content fetch via the EV-SCRIPT-CONTRACT IPC slice.
   */

  type Props = {
    /** Left-pane label (typically the current scene title). */
    leftLabel: string;
    /** Right-pane label (typically the comparison scene title). */
    rightLabel: string;
    /** Left-pane source content. */
    leftSource: string;
    /** Right-pane source content. */
    rightSource: string;
  };

  const { leftLabel, rightLabel, leftSource, rightSource }: Props = $props();

  type DiffOp = 'same' | 'add' | 'del';

  interface DiffRow {
    readonly op: DiffOp;
    readonly left: string;
    readonly right: string;
    readonly leftNo: number | null;
    readonly rightNo: number | null;
  }

  /** Pair-aligned diff rows derived from `leftSource` / `rightSource`.
   *  Each row carries either matched content (`same`), a left-only
   *  removal (`del`), or a right-only addition (`add`). */
  const rows = $derived<readonly DiffRow[]>(buildDiff(leftSource, rightSource));

  function buildDiff(a: string, b: string): readonly DiffRow[] {
    const left = a.split('\n');
    const right = b.split('\n');
    const lcs = computeLcsTable(left, right);
    return backtrackDiff(left, right, lcs);
  }

  /** Build the LCS DP table. `dp[i][j]` is the length of the longest
   *  common subsequence of `left[0..i]` and `right[0..j]`. */
  function computeLcsTable(left: readonly string[], right: readonly string[]): number[][] {
    const m = left.length;
    const n = right.length;
    const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
    for (let i = 1; i <= m; i += 1) {
      const li = left[i - 1] ?? '';
      const row = dp[i] ?? [];
      const prev = dp[i - 1] ?? [];
      for (let j = 1; j <= n; j += 1) {
        if (li === right[j - 1]) {
          row[j] = (prev[j - 1] ?? 0) + 1;
        } else {
          row[j] = Math.max(prev[j] ?? 0, row[j - 1] ?? 0);
        }
      }
    }
    return dp;
  }

  /** Walk the LCS table from the bottom-right corner back to the
   *  origin, emitting one `DiffRow` per step. We reverse the result
   *  before returning so rendering proceeds top-to-bottom. */
  function backtrackDiff(
    left: readonly string[],
    right: readonly string[],
    dp: readonly (readonly number[])[],
  ): readonly DiffRow[] {
    const out: DiffRow[] = [];
    let i = left.length;
    let j = right.length;
    while (i > 0 && j > 0) {
      const li = left[i - 1] ?? '';
      const rj = right[j - 1] ?? '';
      if (li === rj) {
        out.push({ op: 'same', left: li, right: rj, leftNo: i, rightNo: j });
        i -= 1;
        j -= 1;
        continue;
      }
      const up = dp[i - 1]?.[j] ?? 0;
      const lf = dp[i]?.[j - 1] ?? 0;
      if (up >= lf) {
        out.push({ op: 'del', left: li, right: '', leftNo: i, rightNo: null });
        i -= 1;
      } else {
        out.push({ op: 'add', left: '', right: rj, leftNo: null, rightNo: j });
        j -= 1;
      }
    }
    while (i > 0) {
      out.push({
        op: 'del',
        left: left[i - 1] ?? '',
        right: '',
        leftNo: i,
        rightNo: null,
      });
      i -= 1;
    }
    while (j > 0) {
      out.push({
        op: 'add',
        left: '',
        right: right[j - 1] ?? '',
        leftNo: null,
        rightNo: j,
      });
      j -= 1;
    }
    return out.reverse();
  }
</script>

<section class="chr-diff" aria-label="Scene comparison diff">
  <header class="chr-diff__head">
    <span class="chr-diff__head-pane chr-diff__head-pane--left mono">{leftLabel}</span>
    <span class="chr-diff__head-pane chr-diff__head-pane--right mono">{rightLabel}</span>
  </header>
  <div class="chr-diff__body" role="document" aria-label="Diff body">
    {#each rows as row, i (i)}
      <div class="chr-diff__row chr-diff__row--{row.op}">
        <span class="chr-diff__gutter mono" aria-hidden="true">
          {row.leftNo === null ? '' : String(row.leftNo).padStart(3, ' ')}
        </span>
        <span class="chr-diff__cell chr-diff__cell--left mono">{row.left}</span>
        <span class="chr-diff__gutter mono" aria-hidden="true">
          {row.rightNo === null ? '' : String(row.rightNo).padStart(3, ' ')}
        </span>
        <span class="chr-diff__cell chr-diff__cell--right mono">{row.right}</span>
      </div>
    {/each}
  </div>
</section>

<style>
  .chr-diff {
    display: flex;
    flex-direction: column;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
    max-height: 360px;
    min-height: 180px;
  }
  .chr-diff__head {
    display: grid;
    grid-template-columns: 1fr 1fr;
    background: var(--color-bg-3);
    border-bottom: 1px solid var(--color-line-1);
  }
  .chr-diff__head-pane {
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .chr-diff__head-pane--right {
    border-left: 1px solid var(--color-line-1);
  }
  .chr-diff__body {
    overflow: auto;
    flex: 1 1 auto;
    padding: var(--space-xs) 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    line-height: 1.5;
  }
  .chr-diff__row {
    display: grid;
    grid-template-columns: 36px minmax(0, 1fr) 36px minmax(0, 1fr);
    align-items: baseline;
  }
  .chr-diff__row--add {
    background: color-mix(in oklab, var(--color-success) 14%, transparent);
  }
  .chr-diff__row--del {
    background: color-mix(in oklab, var(--color-danger) 14%, transparent);
  }
  .chr-diff__gutter {
    color: var(--color-text-4);
    text-align: right;
    padding: 0 var(--space-xs);
    user-select: none;
  }
  .chr-diff__cell {
    padding: 0 var(--space-sm);
    color: var(--color-text-1);
    white-space: pre;
    overflow-x: auto;
  }
  .chr-diff__cell--left {
    border-right: 1px solid var(--color-line-1);
  }
  .chr-diff__row--add .chr-diff__cell--right {
    color: var(--color-success);
  }
  .chr-diff__row--del .chr-diff__cell--left {
    color: var(--color-danger);
  }
</style>
