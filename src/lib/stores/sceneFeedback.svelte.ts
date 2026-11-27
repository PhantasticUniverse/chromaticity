// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Scene rating, quick-tag, and comment store for in-app operator feedback and
 * deterministic Agent Feedback Digest export.
 *
 * Persists to `localStorage` under `SCENE_FEEDBACK_STORAGE_KEY` and, when
 * running inside the native Tauri shell (`isTauri()`), best-effort mirrors
 * entries to disk via `save_scene_feedback` / `load_scene_feedback` so agents
 * can read `app_data_dir/scene-feedback.json` directly.
 */

import { registerStateMachine } from '../state';

export const SCENE_FEEDBACK_STORAGE_KEY = 'chromaticity.sceneFeedback.v1';

export const FEEDBACK_TAGS = [
  'amazing',
  'smooth',
  'needs-contrast',
  'too-dim',
  'too-fast',
  'favorite-optics',
] as const;

export type FeedbackTag = (typeof FEEDBACK_TAGS)[number];

export interface SceneFeedbackEntry {
  sceneId: string;
  presetName?: string;
  rating: number;
  tags: FeedbackTag[];
  comment: string;
  updatedAt: string;
}

const VALID_TAG_SET: ReadonlySet<string> = new Set(FEEDBACK_TAGS);
const ATTENTION_TAG_SET: ReadonlySet<FeedbackTag> = new Set([
  'needs-contrast',
  'too-dim',
  'too-fast',
]);

function clampRating(rating: number): number {
  if (!Number.isFinite(rating)) return 0;
  return Math.max(0, Math.min(5, Math.round(rating)));
}

function normalizeTags(raw: unknown): FeedbackTag[] {
  if (!Array.isArray(raw)) return [];
  const present: Record<string, true> = {};
  for (const item of raw) {
    if (typeof item === 'string' && VALID_TAG_SET.has(item)) {
      present[item] = true;
    }
  }
  return FEEDBACK_TAGS.filter((tag) => present[tag] === true);
}

function normalizeEntry(raw: unknown): SceneFeedbackEntry | null {
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return null;
  const obj = raw as Record<string, unknown>;
  if (typeof obj.sceneId !== 'string' || obj.sceneId.trim().length === 0) return null;

  const entry: SceneFeedbackEntry = {
    sceneId: obj.sceneId,
    rating: typeof obj.rating === 'number' ? clampRating(obj.rating) : 0,
    tags: normalizeTags(obj.tags),
    comment: typeof obj.comment === 'string' ? obj.comment : '',
    updatedAt: typeof obj.updatedAt === 'string' ? obj.updatedAt : new Date(0).toISOString(),
  };

  if (typeof obj.presetName === 'string' && obj.presetName.trim().length > 0) {
    entry.presetName = obj.presetName;
  }

  return entry;
}

function parseEntryList(raw: unknown): Record<string, SceneFeedbackEntry> {
  const result: Record<string, SceneFeedbackEntry> = {};
  if (Array.isArray(raw)) {
    for (const item of raw) {
      const entry = normalizeEntry(item);
      if (entry) result[entry.sceneId] = entry;
    }
    return result;
  }
  if (typeof raw === 'object' && raw !== null) {
    for (const value of Object.values(raw as Record<string, unknown>)) {
      const entry = normalizeEntry(value);
      if (entry) result[entry.sceneId] = entry;
    }
  }
  return result;
}

function loadPersistedEntries(): Record<string, SceneFeedbackEntry> {
  if (typeof localStorage === 'undefined') return {};
  try {
    const raw = localStorage.getItem(SCENE_FEEDBACK_STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    return parseEntryList(parsed);
  } catch {
    return {};
  }
}

async function tryTauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  try {
    const mod = await import('@tauri-apps/api/core');
    if (typeof mod.isTauri !== 'function' || !mod.isTauri()) {
      return null;
    }
    return await mod.invoke<T>(command, args);
  } catch {
    return null;
  }
}

function formatStars(rating: number): string {
  const clamped = clampRating(rating);
  if (clamped === 0) return 'Unrated';
  return `${'★'.repeat(clamped)}${'☆'.repeat(5 - clamped)} (${clamped}/5)`;
}

function hasMeaningfulFeedback(entry: SceneFeedbackEntry): boolean {
  return entry.rating > 0 || entry.tags.length > 0 || entry.comment.trim().length > 0;
}

function formatEntryMarkdown(entry: SceneFeedbackEntry): string {
  const lines: string[] = [];
  const heading = entry.presetName
    ? `### \`${entry.sceneId}\` — ${entry.presetName}`
    : `### \`${entry.sceneId}\``;
  lines.push(heading);
  lines.push(`- **Rating:** ${formatStars(entry.rating)}`);
  if (entry.tags.length > 0) {
    lines.push(`- **Tags:** ${entry.tags.map((t) => `\`${t}\``).join(', ')}`);
  }
  const trimmedComment = entry.comment.trim();
  if (trimmedComment.length > 0) {
    lines.push(`- **Notes:** ${trimmedComment.replace(/\r?\n/g, ' ')}`);
  }
  return lines.join('\n');
}

class SceneFeedbackStore {
  entries = $state<Record<string, SceneFeedbackEntry>>(loadPersistedEntries());

  /** Read the current feedback entry for `sceneId`, if any. */
  get(sceneId: string): SceneFeedbackEntry | undefined {
    return this.entries[sceneId];
  }

  /** Set the 0..5 star rating for `sceneId` (optionally recording `presetName`). */
  setRating(sceneId: string, rating: number, presetName?: string): SceneFeedbackEntry {
    const clamped = clampRating(rating);
    const existing = this.entries[sceneId];
    const next: SceneFeedbackEntry = {
      sceneId,
      rating: clamped,
      tags: existing ? [...existing.tags] : [],
      comment: existing ? existing.comment : '',
      updatedAt: new Date().toISOString(),
    };
    const resolvedPreset = presetName ?? existing?.presetName;
    if (resolvedPreset !== undefined && resolvedPreset.trim().length > 0) {
      next.presetName = resolvedPreset;
    }
    this.entries = { ...this.entries, [sceneId]: next };
    this.#persist();
    return next;
  }

  /** Toggle a quick feedback tag for `sceneId`, preserving canonical tag order. */
  toggleTag(sceneId: string, tag: FeedbackTag): SceneFeedbackEntry {
    const existing = this.entries[sceneId];
    const currentTags = existing ? existing.tags : [];
    const present: Record<string, true> = {};
    for (const current of currentTags) {
      present[current] = true;
    }
    if (present[tag]) {
      delete present[tag];
    } else {
      present[tag] = true;
    }
    const nextTags = FEEDBACK_TAGS.filter((t) => present[t] === true);
    const next: SceneFeedbackEntry = {
      sceneId,
      rating: existing ? existing.rating : 0,
      tags: nextTags,
      comment: existing ? existing.comment : '',
      updatedAt: new Date().toISOString(),
    };
    if (existing?.presetName !== undefined) {
      next.presetName = existing.presetName;
    }
    this.entries = { ...this.entries, [sceneId]: next };
    this.#persist();
    return next;
  }

  /** Set the free-text comment for `sceneId`. */
  setComment(sceneId: string, comment: string): SceneFeedbackEntry {
    const existing = this.entries[sceneId];
    const next: SceneFeedbackEntry = {
      sceneId,
      rating: existing ? existing.rating : 0,
      tags: existing ? [...existing.tags] : [],
      comment,
      updatedAt: new Date().toISOString(),
    };
    if (existing?.presetName !== undefined) {
      next.presetName = existing.presetName;
    }
    this.entries = { ...this.entries, [sceneId]: next };
    this.#persist();
    return next;
  }

  /** Remove all recorded feedback for `sceneId`. */
  clear(sceneId: string): void {
    if (!(sceneId in this.entries)) return;
    const next: Record<string, SceneFeedbackEntry> = {};
    for (const [id, entry] of Object.entries(this.entries)) {
      if (id !== sceneId) next[id] = entry;
    }
    this.entries = next;
    this.#persist();
  }

  /** Return all feedback entries in deterministic `sceneId` order. */
  allEntries(): SceneFeedbackEntry[] {
    return Object.values(this.entries)
      .map((entry) => ({ ...entry, tags: [...entry.tags] }))
      .sort((a, b) => a.sceneId.localeCompare(b.sceneId));
  }

  /**
   * Hydrate persisted feedback from `localStorage` and (when in Tauri) merge
   * entries from `app_data_dir/scene-feedback.json` via `load_scene_feedback`.
   */
  async hydrateFromDisk(): Promise<void> {
    const fromStorage = loadPersistedEntries();
    let merged: Record<string, SceneFeedbackEntry> = { ...fromStorage, ...this.entries };

    const diskPayload = await tryTauriInvoke<unknown>('load_scene_feedback');
    if (diskPayload !== null) {
      let parsed: unknown = diskPayload;
      if (typeof diskPayload === 'string') {
        try {
          parsed = JSON.parse(diskPayload);
        } catch {
          parsed = null;
        }
      }
      const fromDisk = parseEntryList(parsed);
      merged = { ...merged };
      for (const [sceneId, diskEntry] of Object.entries(fromDisk)) {
        const localEntry = merged[sceneId];
        if (!localEntry || diskEntry.updatedAt > localEntry.updatedAt) {
          merged[sceneId] = diskEntry;
        }
      }
    }

    this.entries = merged;
    this.#persistLocalOnly();
  }

  /**
   * Generate a deterministic Markdown report grouping rated, tagged, and
   * commented scenes for agent-driven shader iteration.
   */
  exportAgentDigest(): string {
    const active = this.allEntries().filter(hasMeaningfulFeedback);
    const lines: string[] = ['# Chromaticity Scene Feedback Digest', ''];

    if (active.length === 0) {
      lines.push('_No scene feedback recorded yet._');
      return lines.join('\n');
    }

    const rated = active.filter((e) => e.rating > 0);
    const avgRating =
      rated.length > 0
        ? (rated.reduce((sum, e) => sum + e.rating, 0) / rated.length).toFixed(2)
        : 'n/a';

    lines.push(
      `- **Scenes with feedback:** ${active.length}`,
      `- **Rated scenes:** ${rated.length} (avg ${avgRating}/5)`,
      '',
    );

    const needsAttention = active
      .filter(
        (e) => (e.rating > 0 && e.rating <= 3) || e.tags.some((tag) => ATTENTION_TAG_SET.has(tag)),
      )
      .sort((a, b) => {
        const aScore = a.rating === 0 ? 4 : a.rating;
        const bScore = b.rating === 0 ? 4 : b.rating;
        if (aScore !== bScore) return aScore - bScore;
        return a.sceneId.localeCompare(b.sceneId);
      });

    const attentionIds = new Set(needsAttention.map((e) => e.sceneId));

    const highlights = active
      .filter((e) => !attentionIds.has(e.sceneId) && e.rating >= 4)
      .sort((a, b) => {
        if (b.rating !== a.rating) return b.rating - a.rating;
        return a.sceneId.localeCompare(b.sceneId);
      });

    const notesOnly = active
      .filter((e) => !attentionIds.has(e.sceneId) && e.rating === 0)
      .sort((a, b) => a.sceneId.localeCompare(b.sceneId));

    if (needsAttention.length > 0) {
      lines.push('## Needs Attention', '');
      for (const entry of needsAttention) {
        lines.push(formatEntryMarkdown(entry), '');
      }
    }

    if (highlights.length > 0) {
      lines.push('## Highlights', '');
      for (const entry of highlights) {
        lines.push(formatEntryMarkdown(entry), '');
      }
    }

    if (notesOnly.length > 0) {
      lines.push('## Notes Only', '');
      for (const entry of notesOnly) {
        lines.push(formatEntryMarkdown(entry), '');
      }
    }

    return lines.join('\n').trimEnd();
  }

  /** Clear in-memory state and remove the `localStorage` key (used in tests). */
  reset(): void {
    this.entries = {};
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.removeItem(SCENE_FEEDBACK_STORAGE_KEY);
      } catch {
        // Ignore storage errors in restricted environments.
      }
    }
  }

  #persistLocalOnly(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(SCENE_FEEDBACK_STORAGE_KEY, JSON.stringify(this.allEntries()));
    } catch {
      // Ignore quota / private-mode errors.
    }
  }

  #persist(): void {
    this.#persistLocalOnly();
    const snapshot = this.allEntries();
    void tryTauriInvoke<void>('save_scene_feedback', { entries: snapshot });
  }
}

export const sceneFeedbackStore = new SceneFeedbackStore();

registerStateMachine('sceneFeedback', () => {
  const entries = sceneFeedbackStore.allEntries();
  return {
    entryCount: entries.length,
    ratedCount: entries.filter((e) => e.rating > 0).length,
    taggedCount: entries.filter((e) => e.tags.length > 0).length,
    commentedCount: entries.filter((e) => e.comment.trim().length > 0).length,
  };
});
