<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-22 PR3 New-Scene modal stub. Frozen-frame: Create closes the
  // modal and emits a `console.info` only — real scene creation is
  // Phase-3+ scope. The inline focus-trap mirrors HotkeyOverlay's
  // minimal trap (limited tabbables; no shared primitive yet).
  let {
    open,
    onCreate,
    onClose,
  }: { open: boolean; onCreate: (name: string) => void; onClose: () => void } = $props();

  let name = $state('');
  let dialogEl: HTMLDivElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let previouslyFocused: HTMLElement | null = null;

  $effect(() => {
    if (open) {
      const a = document.activeElement;
      previouslyFocused = a instanceof HTMLElement ? a : null;
      name = '';
      queueMicrotask(() => inputEl?.focus());
    } else if (previouslyFocused) {
      previouslyFocused.focus();
      previouslyFocused = null;
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key !== 'Tab' || !dialogEl) return;
    const tabs = Array.from(dialogEl.querySelectorAll<HTMLElement>('input, button')).filter(
      (el) => !el.hasAttribute('disabled'),
    );
    const first = tabs[0];
    const last = tabs[tabs.length - 1];
    if (!first || !last) return;
    const active = document.activeElement;
    if (e.shiftKey && active === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

{#if open}
  <div class="new-scene__backdrop" role="presentation" onclick={onClose}></div>
  <div
    class="new-scene"
    role="dialog"
    aria-modal="true"
    aria-labelledby="new-scene-title"
    tabindex="-1"
    bind:this={dialogEl}
    onkeydown={handleKeydown}
  >
    <h2 id="new-scene-title" class="new-scene__title">New scene</h2>
    <label class="new-scene__field">
      <span>Scene name</span>
      <input
        type="text"
        placeholder="untitled scene"
        bind:value={name}
        bind:this={inputEl}
        aria-label="Scene name"
      />
    </label>
    <div class="new-scene__actions">
      <button type="button" class="new-scene__btn" onclick={onClose}>Cancel</button>
      <button type="button" class="new-scene__btn primary" onclick={() => onCreate(name.trim())}>
        Create
      </button>
    </div>
  </div>
{/if}

<style>
  .new-scene__backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 950;
  }
  .new-scene {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    padding: var(--space-lg);
    width: min(420px, calc(100vw - 32px));
    z-index: 951;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .new-scene__title {
    margin: 0;
    font-size: var(--font-size-body-md);
  }
  .new-scene__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }
  .new-scene__field input {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-body-sm);
  }
  .new-scene__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-sm);
  }
  .new-scene__btn {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
  }
  .new-scene__btn:hover {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .new-scene__btn.primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .new-scene__btn.primary:hover {
    filter: brightness(1.05);
  }
  .new-scene__btn:focus,
  .new-scene__field input:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
</style>
