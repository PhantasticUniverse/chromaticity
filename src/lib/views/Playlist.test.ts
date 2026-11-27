// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W35 Playlist view tests — exercise the inspector edit-mode added on
// top of the W13 PR3 cue-table render. The W13 read-mostly contract is
// preserved (Save / Delete remain inert; cue-table rows still display
// the underlying fixture values not the in-progress draft); these
// tests focus on the form-fields-track-selection contract and on the
// `trigger` field round-trip.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import Playlist from './Playlist.svelte';
import { PLAYLIST_CUES_STORAGE_KEY, playlistStore } from '../stores/playlist.svelte';

describe('Playlist view (W35 inspector edit-mode)', () => {
  beforeEach(() => {
    playlistStore.reset();
  });

  afterEach(() => {
    cleanup();
    playlistStore.reset();
  });

  it('renders the playlist sidebar with the five fixture entries', () => {
    const { getByRole } = render(Playlist);
    // Each playlist is a button in the sidebar list.
    expect(getByRole('button', { name: /Festival Set \/ Dusk/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Ambient Pool/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Hard Techno/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Sound Check/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Demo Scenes/ })).toBeInTheDocument();
  });

  it('renders the 10-row cue table for the festival set', () => {
    const { container } = render(Playlist);
    const rows = container.querySelectorAll('.playlist-view__cue-table tbody tr');
    expect(rows.length).toBe(10);
  });

  it('opens the inspector populated with First Drop (default selection)', () => {
    const { getByLabelText } = render(Playlist);
    const labelInput = getByLabelText('Cue label') as HTMLInputElement;
    expect(labelInput.value).toBe('First Drop');
    const sceneSelect = getByLabelText('Cue scene') as HTMLSelectElement;
    expect(sceneSelect.value).toBe('Glitch');
    const durationInput = getByLabelText('Cue duration') as HTMLInputElement;
    expect(durationInput.value).toBe('03:30');
    const triggerSelect = getByLabelText('Cue trigger') as HTMLSelectElement;
    // First Drop has trigger='auto'.
    expect(triggerSelect.value).toBe('auto');
  });

  it('updates the form fields when a different cue is selected', async () => {
    const { container, getByLabelText } = render(Playlist);
    const rows = container.querySelectorAll('.playlist-view__cue-table tbody tr');
    // Row 4 in the table is "Peak" (index 3 in DOM = index 4 in fixture data,
    // since the table is 1-indexed by `cue.index`). Cue 4 has trigger='beat'.
    const peakRow = rows[3];
    expect(peakRow).toBeDefined();
    if (!peakRow) return;
    await fireEvent.click(peakRow);
    const labelInput = getByLabelText('Cue label') as HTMLInputElement;
    expect(labelInput.value).toBe('Peak');
    const sceneSelect = getByLabelText('Cue scene') as HTMLSelectElement;
    expect(sceneSelect.value).toBe('GPU_Melter');
    const triggerSelect = getByLabelText('Cue trigger') as HTMLSelectElement;
    expect(triggerSelect.value).toBe('beat');
  });

  it('typing into the label input updates the inspector heading', async () => {
    const { getByLabelText, getByRole } = render(Playlist);
    const labelInput = getByLabelText('Cue label') as HTMLInputElement;
    await fireEvent.input(labelInput, { target: { value: 'First Drop · edited' } });
    expect(labelInput.value).toBe('First Drop · edited');
    // The inspector heading binds to `cueDraft.label`, so it reflects the
    // edit live without any save-roundtrip.
    expect(getByRole('heading', { name: /First Drop · edited/ })).toBeInTheDocument();
  });

  it('discards in-progress edits when the cue selection changes', async () => {
    const { container, getByLabelText } = render(Playlist);
    const labelInput = getByLabelText('Cue label') as HTMLInputElement;
    await fireEvent.input(labelInput, { target: { value: 'edited but unsaved' } });
    expect(labelInput.value).toBe('edited but unsaved');
    // Click cue 4 (Peak) — the draft resets to Peak's stored values.
    const rows = container.querySelectorAll('.playlist-view__cue-table tbody tr');
    const peakRow = rows[3];
    expect(peakRow).toBeDefined();
    if (!peakRow) return;
    await fireEvent.click(peakRow);
    expect((getByLabelText('Cue label') as HTMLInputElement).value).toBe('Peak');
    // Re-selecting First Drop does NOT restore the prior edit — drafts are
    // local-state-only and discarded on selection swap (matching the
    // bundle's uncontrolled-input semantics).
    const firstDropRow = rows[2];
    expect(firstDropRow).toBeDefined();
    if (!firstDropRow) return;
    await fireEvent.click(firstDropRow);
    expect((getByLabelText('Cue label') as HTMLInputElement).value).toBe('First Drop');
  });

  it('renders Save and Delete action buttons in the inspector', () => {
    const { getByRole } = render(Playlist);
    expect(getByRole('button', { name: 'Save' })).toBeInTheDocument();
    expect(getByRole('button', { name: 'Delete' })).toBeInTheDocument();
  });

  it('renders a SceneStill canvas thumbnail per cue row (Wave 2.2 R.1)', () => {
    const { container } = render(Playlist);
    const thumbs = container.querySelectorAll('.playlist-view__cue-thumb canvas.scene-still');
    // One thumbnail per visible cue row in the festival fixture.
    expect(thumbs.length).toBe(10);
  });

  it('switching to an empty playlist hides the inspector form', async () => {
    const { getByRole, getByText, queryByLabelText } = render(Playlist);
    // Inspector form is present at default (Festival Set selected).
    expect(queryByLabelText('Cue label')).not.toBeNull();
    await fireEvent.click(getByRole('button', { name: /Ambient Pool/ }));
    // Empty playlist: cue table renders the EmptyState placeholder; the
    // inspector form unmounts (no `selectedCue`).
    expect(queryByLabelText('Cue label')).toBeNull();
    expect(getByText(/select a cue to inspect/)).toBeInTheDocument();
  });

  it('G6.5 — Play/Pause toggle updates playlistStore playback state', async () => {
    const { getByRole } = render(Playlist);
    const playBtn = getByRole('button', { name: 'Play' });
    expect(playBtn).toHaveAttribute('aria-pressed', 'false');
    expect(playlistStore.playback).toBe('stopped');

    await fireEvent.click(playBtn);
    expect(playlistStore.playback).toBe('playing');
    const pauseBtn = getByRole('button', { name: 'Pause' });
    expect(pauseBtn).toHaveAttribute('aria-pressed', 'true');

    await fireEvent.click(pauseBtn);
    expect(playlistStore.playback).toBe('stopped');
  });

  it('G6.5 — Shuffle switches to a different cue in the active playlist', async () => {
    const { getByLabelText, getByRole } = render(Playlist);
    expect((getByLabelText('Cue label') as HTMLInputElement).value).toBe('First Drop');

    await fireEvent.click(getByRole('button', { name: 'Shuffle' }));
    expect((getByLabelText('Cue label') as HTMLInputElement).value).not.toBe('First Drop');
  });

  it('G6.5 — Save commits cue edits to playlistStore and persists to localStorage', async () => {
    const { container, getByLabelText, getByRole } = render(Playlist);
    const labelInput = getByLabelText('Cue label') as HTMLInputElement;
    await fireEvent.input(labelInput, { target: { value: 'First Drop · Live' } });
    await fireEvent.click(getByRole('button', { name: 'Save' }));

    const rows = container.querySelectorAll('.playlist-view__cue-table tbody tr');
    expect(rows[2]?.textContent).toContain('First Drop · Live');
    expect(localStorage.getItem(PLAYLIST_CUES_STORAGE_KEY)).toContain('First Drop · Live');
  });

  it('G6.5 — Delete removes the selected cue from playlistStore and updates table count', async () => {
    const { container, getByRole } = render(Playlist);
    expect(container.querySelectorAll('.playlist-view__cue-table tbody tr').length).toBe(10);

    await fireEvent.click(getByRole('button', { name: 'Delete' }));
    expect(container.querySelectorAll('.playlist-view__cue-table tbody tr').length).toBe(9);
  });
});
