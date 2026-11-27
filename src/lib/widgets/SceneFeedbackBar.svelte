<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import Icon from '../Icon.svelte';
  import {
    FEEDBACK_TAGS,
    sceneFeedbackStore,
    type FeedbackTag,
  } from '../stores/sceneFeedback.svelte';

  let {
    sceneId,
    sceneTitle,
    compact = false,
  }: {
    sceneId: string;
    sceneTitle?: string;
    compact?: boolean;
  } = $props();

  const STARS = [1, 2, 3, 4, 5] as const;

  const uid = $props.id();
  const panelId = $derived(`${uid}-panel`);
  const commentId = $derived(`${uid}-comment`);

  let panelOpen = $state(false);
  let copyStatus = $state<'idle' | 'copied' | 'failed'>('idle');

  const entry = $derived(sceneFeedbackStore.get(sceneId));
  const rating = $derived(entry?.rating ?? 0);
  const activeTags = $derived(entry?.tags ?? []);
  const comment = $derived(entry?.comment ?? '');
  const hasNotes = $derived(activeTags.length > 0 || comment.trim().length > 0);
  const hasAnyFeedback = $derived(rating > 0 || hasNotes);

  function handleRate(star: number): void {
    const nextRating = rating === star ? 0 : star;
    sceneFeedbackStore.setRating(sceneId, nextRating, sceneTitle);
  }

  function handleToggleTag(tag: FeedbackTag): void {
    sceneFeedbackStore.toggleTag(sceneId, tag);
  }

  function handleCommentInput(event: Event): void {
    const target = event.currentTarget as HTMLTextAreaElement;
    sceneFeedbackStore.setComment(sceneId, target.value);
  }

  function handleClear(): void {
    sceneFeedbackStore.clear(sceneId);
  }

  async function handleCopyDigest(): Promise<void> {
    const digest = sceneFeedbackStore.exportAgentDigest();
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(digest);
        copyStatus = 'copied';
      } else {
        copyStatus = 'failed';
      }
    } catch {
      copyStatus = 'failed';
    }
  }
</script>

<div
  class="sfb"
  class:sfb--compact={compact}
  role="group"
  aria-label={sceneTitle ? `Feedback for ${sceneTitle}` : `Feedback for ${sceneId}`}
>
  <div class="sfb__row">
    <div class="sfb__stars" role="group" aria-label="Scene star rating">
      {#each STARS as star (star)}
        {@const filled = rating >= star}
        <button
          type="button"
          class="sfb__star"
          class:sfb__star--filled={filled}
          aria-label={star === 1 ? 'Rate 1 star' : `Rate ${star} stars`}
          aria-pressed={filled ? 'true' : 'false'}
          onclick={() => handleRate(star)}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill={filled ? 'currentColor' : 'none'}
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <polygon
              points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
            />
          </svg>
        </button>
      {/each}
    </div>

    <button
      type="button"
      class="sfb__note-toggle"
      class:sfb__note-toggle--active={panelOpen || hasNotes}
      aria-expanded={panelOpen ? 'true' : 'false'}
      aria-controls={panelId}
      onclick={() => {
        panelOpen = !panelOpen;
        copyStatus = 'idle';
      }}
    >
      <Icon name="note" size={12} />
      <span>Note</span>
      {#if activeTags.length > 0}
        <span class="sfb__badge">{activeTags.length}</span>
      {/if}
    </button>
  </div>

  {#if panelOpen}
    <div id={panelId} class="sfb__panel">
      <div class="sfb__tags" role="group" aria-label="Quick feedback tags">
        {#each FEEDBACK_TAGS as tag (tag)}
          {@const selected = activeTags.includes(tag)}
          <button
            type="button"
            class="sfb__tag"
            class:sfb__tag--active={selected}
            aria-pressed={selected ? 'true' : 'false'}
            onclick={() => handleToggleTag(tag)}
          >
            {tag}
          </button>
        {/each}
      </div>

      <label class="sfb__label" for={commentId}>Notes for shader iteration</label>
      <textarea
        id={commentId}
        class="sfb__textarea"
        rows={compact ? 2 : 3}
        placeholder="Describe contrast, motion, optics, or tweaks..."
        value={comment}
        oninput={handleCommentInput}
      ></textarea>

      <div class="sfb__footer">
        <button type="button" class="sfb__action sfb__action--primary" onclick={handleCopyDigest}>
          <Icon name="copy" size={12} />
          <span>
            {#if copyStatus === 'copied'}
              Copied Digest
            {:else if copyStatus === 'failed'}
              Clipboard Unavailable
            {:else}
              Copy Agent Digest
            {/if}
          </span>
        </button>
        {#if hasAnyFeedback}
          <button type="button" class="sfb__action" onclick={handleClear}>Clear</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .sfb {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: 6px 8px;
    color: var(--color-text-0);
  }
  .sfb--compact {
    padding: 4px 8px;
  }
  .sfb__row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-xs);
  }
  .sfb__stars {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .sfb__star {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    color: var(--color-text-1);
    cursor: pointer;
    transition:
      color 120ms ease,
      background-color 120ms ease,
      border-color 120ms ease;
  }
  .sfb__star:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }
  .sfb__star--filled {
    color: #f5c542;
  }
  .sfb__star--filled:hover {
    color: #ffd666;
  }
  .sfb__star:focus-visible,
  .sfb__note-toggle:focus-visible,
  .sfb__tag:focus-visible,
  .sfb__textarea:focus-visible,
  .sfb__action:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .sfb__note-toggle {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
    cursor: pointer;
  }
  .sfb__note-toggle:hover {
    background: var(--color-bg-3);
  }
  .sfb__note-toggle--active {
    border-color: var(--color-primary);
    color: var(--color-text-0);
  }
  .sfb__badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--color-primary);
    color: var(--color-on-primary);
    font-size: 10px;
    font-weight: 700;
  }
  .sfb__panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding-top: 6px;
    border-top: 1px solid var(--color-line-1);
  }
  .sfb__tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .sfb__tag {
    padding: 2px 7px;
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    cursor: pointer;
  }
  .sfb__tag:hover {
    background: var(--color-bg-3);
  }
  .sfb__tag--active {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
    font-weight: 600;
  }
  .sfb__label {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
  }
  .sfb__textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    padding: 6px 8px;
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: inherit;
    font-size: var(--font-size-body-sm);
    line-height: 1.4;
  }
  .sfb__textarea::placeholder {
    color: var(--color-text-2);
  }
  .sfb__footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-xs);
  }
  .sfb__action {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    cursor: pointer;
  }
  .sfb__action:hover {
    background: var(--color-bg-3);
  }
  .sfb__action--primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
    font-weight: 600;
  }
</style>
