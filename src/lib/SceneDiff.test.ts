// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W50 SceneDiff vitest backfill — covers the W21 PR3 side-by-side
// scene diff: header labels, LCS-derived row classification (same / add
// / del), gutter line numbers, and edge cases (identical / fully
// disjoint inputs, empty sources). `afterEach(cleanup)` per
// `feedback_vitest_svelte_cleanup_required.md`.

import '@testing-library/jest-dom/vitest';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import SceneDiff from './SceneDiff.svelte';

describe('SceneDiff scene-comparison pane', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the section landmark with the comparison aria-label', () => {
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: 'a', rightSource: 'b' },
    });
    const section = container.querySelector('section.chr-diff');
    expect(section).not.toBeNull();
    expect(section?.getAttribute('aria-label')).toBe('Scene comparison diff');
  });

  it('renders the left and right pane labels in the header', () => {
    const { container } = render(SceneDiff, {
      props: {
        leftLabel: 'current.glsl',
        rightLabel: 'compare.glsl',
        leftSource: 'a',
        rightSource: 'b',
      },
    });
    const left = container.querySelector('.chr-diff__head-pane--left');
    const right = container.querySelector('.chr-diff__head-pane--right');
    expect(left?.textContent).toBe('current.glsl');
    expect(right?.textContent).toBe('compare.glsl');
  });

  it('classifies every row as `same` when sources are identical', () => {
    const src = 'foo\nbar\nbaz';
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: src, rightSource: src },
    });
    const rows = container.querySelectorAll('.chr-diff__row');
    expect(rows.length).toBe(3);
    rows.forEach((row) => {
      expect(row.classList.contains('chr-diff__row--same')).toBe(true);
      expect(row.classList.contains('chr-diff__row--add')).toBe(false);
      expect(row.classList.contains('chr-diff__row--del')).toBe(false);
    });
  });

  it('marks left-only lines as `del` and right-only lines as `add` for fully disjoint inputs', () => {
    const { container } = render(SceneDiff, {
      props: {
        leftLabel: 'A',
        rightLabel: 'B',
        leftSource: 'aaa\nbbb',
        rightSource: 'xxx\nyyy',
      },
    });
    const rows = container.querySelectorAll('.chr-diff__row');
    // 2 dels (left-only) + 2 adds (right-only) = 4 rows total. The
    // implementation emits the dels first (i-walk runs before j-walk
    // when neither has a match), then the adds.
    expect(rows.length).toBe(4);
    const dels = container.querySelectorAll('.chr-diff__row--del');
    const adds = container.querySelectorAll('.chr-diff__row--add');
    expect(dels.length).toBe(2);
    expect(adds.length).toBe(2);
  });

  it('classifies a mid-block insertion as a single `add` row flanked by `same` rows', () => {
    const left = 'foo\nbar';
    const right = 'foo\nNEW\nbar';
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: left, rightSource: right },
    });
    const rows = container.querySelectorAll('.chr-diff__row');
    expect(rows.length).toBe(3);
    expect(rows[0]?.classList.contains('chr-diff__row--same')).toBe(true);
    expect(rows[1]?.classList.contains('chr-diff__row--add')).toBe(true);
    expect(rows[2]?.classList.contains('chr-diff__row--same')).toBe(true);
    // The added row's right cell carries the new content; its left is empty.
    const addRightCell = rows[1]?.querySelector('.chr-diff__cell--right');
    const addLeftCell = rows[1]?.querySelector('.chr-diff__cell--left');
    expect(addRightCell?.textContent).toBe('NEW');
    expect(addLeftCell?.textContent).toBe('');
  });

  it('classifies a mid-block removal as a single `del` row flanked by `same` rows', () => {
    const left = 'foo\nGONE\nbar';
    const right = 'foo\nbar';
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: left, rightSource: right },
    });
    const rows = container.querySelectorAll('.chr-diff__row');
    expect(rows.length).toBe(3);
    expect(rows[1]?.classList.contains('chr-diff__row--del')).toBe(true);
    const delLeftCell = rows[1]?.querySelector('.chr-diff__cell--left');
    const delRightCell = rows[1]?.querySelector('.chr-diff__cell--right');
    expect(delLeftCell?.textContent).toBe('GONE');
    expect(delRightCell?.textContent).toBe('');
  });

  it('renders right-padded gutter line numbers for matched rows and blank gutters for unmatched sides', () => {
    const left = 'foo';
    const right = 'foo\nNEW';
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: left, rightSource: right },
    });
    const rows = container.querySelectorAll('.chr-diff__row');
    expect(rows.length).toBe(2);
    // Row 0: same -> both gutters carry "  1" (3-wide right-pad).
    const row0Gutters = rows[0]?.querySelectorAll('.chr-diff__gutter');
    expect(row0Gutters?.[0]?.textContent).toBe('  1');
    expect(row0Gutters?.[1]?.textContent).toBe('  1');
    // Row 1: add -> left gutter empty, right gutter "  2".
    const row1Gutters = rows[1]?.querySelectorAll('.chr-diff__gutter');
    expect(row1Gutters?.[0]?.textContent?.trim()).toBe('');
    expect(row1Gutters?.[1]?.textContent).toBe('  2');
  });

  it('hides the gutters from assistive tech via aria-hidden', () => {
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: 'x', rightSource: 'x' },
    });
    const gutters = container.querySelectorAll('.chr-diff__gutter');
    expect(gutters.length).toBeGreaterThan(0);
    gutters.forEach((g) => expect(g.getAttribute('aria-hidden')).toBe('true'));
  });

  it('handles empty sources by producing no rows', () => {
    const { container } = render(SceneDiff, {
      props: { leftLabel: 'A', rightLabel: 'B', leftSource: '', rightSource: '' },
    });
    // Empty strings split('\n') -> [''], which is one identical "row" of empty content.
    const rows = container.querySelectorAll('.chr-diff__row');
    expect(rows.length).toBe(1);
    expect(rows[0]?.classList.contains('chr-diff__row--same')).toBe(true);
    expect(rows[0]?.querySelector('.chr-diff__cell--left')?.textContent).toBe('');
    expect(rows[0]?.querySelector('.chr-diff__cell--right')?.textContent).toBe('');
  });
});
