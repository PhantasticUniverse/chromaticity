// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W33c Inputs view tests — exercise the search + group-tab filter
// toolbar added on top of the W10/W32 grouped-controls render. Existing
// W32 widget primitive renders are not separately covered here (those
// have their own widget-level tests at `src/lib/widgets/{Xy,Bang,
// Dropdown}.test.ts`); these tests focus on the filtering contract.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention; `within`
// per the W31 Patch.test.ts precedent for queries that need to scope
// past the toolbar's own copies of group / control names.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Inputs from './Inputs.svelte';
import { CHROMA_FIXTURE_NATIVE } from '../data/scene-fixtures';

describe('Inputs view', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the scene title + credit + format-tag header', () => {
    const { getByText } = render(Inputs, { props: { scene: CHROMA_FIXTURE_NATIVE } });
    expect(getByText(CHROMA_FIXTURE_NATIVE.title)).toBeInTheDocument();
    expect(getByText(CHROMA_FIXTURE_NATIVE.credit)).toBeInTheDocument();
    expect(getByText(CHROMA_FIXTURE_NATIVE.format_tag)).toBeInTheDocument();
  });

  it('renders the toolbar with search + count + group-tab radiogroup', () => {
    const { getByRole, getByLabelText } = render(Inputs, {
      props: { scene: CHROMA_FIXTURE_NATIVE },
    });
    expect(getByLabelText(/Filter inputs by name/)).toBeInTheDocument();
    expect(getByRole('radiogroup', { name: /Filter by group/ })).toBeInTheDocument();
    expect(getByRole('radio', { name: 'all' })).toHaveAttribute('aria-checked', 'true');
  });

  it('shows total count tally with all controls visible by default', () => {
    const { container } = render(Inputs, { props: { scene: CHROMA_FIXTURE_NATIVE } });
    const count = container.querySelector('.inputs__count');
    expect(count).not.toBeNull();
    const total = CHROMA_FIXTURE_NATIVE.controls.length;
    expect(count?.textContent?.trim()).toBe(`${total}/${total}`);
  });

  it('search input filters control rows by caseless substring on name', async () => {
    const { container, getByLabelText } = render(Inputs, {
      props: { scene: CHROMA_FIXTURE_NATIVE },
    });
    const search = getByLabelText(/Filter inputs by name/) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'rotation' } });
    // Only `rotation_speed` survives the filter; the count tally drops
    // to 1.
    const count = container.querySelector('.inputs__count');
    expect(count?.textContent?.trim()).toBe(`1/${CHROMA_FIXTURE_NATIVE.controls.length}`);
  });

  it('group-tab clicks restrict the visible group set', async () => {
    const { container, getByRole } = render(Inputs, {
      props: { scene: CHROMA_FIXTURE_NATIVE },
    });
    const colorTab = getByRole('radio', { name: 'color' });
    await fireEvent.click(colorTab);
    expect(colorTab).toHaveAttribute('aria-checked', 'true');
    expect(getByRole('radio', { name: 'all' })).toHaveAttribute('aria-checked', 'false');
    // Only the `color` group section renders in the body.
    const groupHeadings = container.querySelectorAll('.inputs__group-name');
    expect(groupHeadings.length).toBe(1);
    expect(groupHeadings[0]?.textContent?.trim()).toBe('color');
  });

  it('search clear button resets the query', async () => {
    const { getByLabelText, queryByLabelText } = render(Inputs, {
      props: { scene: CHROMA_FIXTURE_NATIVE },
    });
    const search = getByLabelText(/Filter inputs by name/) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'rotation' } });
    expect(search.value).toBe('rotation');
    const clearBtn = getByLabelText(/Clear search/);
    await fireEvent.click(clearBtn);
    expect(search.value).toBe('');
    // After clear, the clear button itself disappears.
    expect(queryByLabelText(/Clear search/)).toBeNull();
  });

  it('renders the empty-state when no controls match the active filter', async () => {
    const { getByLabelText, getByRole } = render(Inputs, {
      props: { scene: CHROMA_FIXTURE_NATIVE },
    });
    const search = getByLabelText(/Filter inputs by name/) as HTMLInputElement;
    await fireEvent.input(search, { target: { value: 'zzzz-no-match-zzzz' } });
    const empty = getByRole('status');
    expect(within(empty).getByText(/no controls match/)).toBeInTheDocument();
    expect(within(empty).getByText('"zzzz-no-match-zzzz"')).toBeInTheDocument();
  });
});
