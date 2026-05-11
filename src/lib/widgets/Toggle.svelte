<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: on/off switch. Wave 2.1 Lane A.2 wires
  // click → `onchange(!value)` so the Perform view's Scene Controls +
  // Meta Controls panels can dispatch `set_control` IPC on user
  // interaction. The widget stays a controlled component — `value` is
  // the source of truth from the caller, and a click emits
  // `onchange(next)` for the caller to commit (or ignore, for display-
  // only callers like the Inputs Inspector view).
  //
  // Vertical default mirrors the bundle's vertical-fader sibling
  // layout; the horizontal variant is a pill switch with thumb. Both
  // orientations share the same click + onchange contract.
  let {
    value,
    label,
    orientation = 'vertical',
    onchange,
    disabled = false,
  }: {
    value: boolean;
    label?: string;
    orientation?: 'vertical' | 'horizontal';
    onchange?: (next: boolean) => void;
    disabled?: boolean;
  } = $props();

  // Render gate: presence of `onchange` switches to a button surface.
  // Action gate: `disabled` separately suppresses the dispatch path so
  // the button still renders (with `aria-disabled`) and the test
  // surface can assert disabled-state styling. Matches the
  // `CrossFader.svelte` precedent where the wrapper element renders
  // unconditionally and `disabled` just early-returns the handler.
  const isButton = $derived(onchange !== undefined);
  const actionable = $derived(!disabled && onchange !== undefined);

  function onClick(): void {
    if (!actionable) return;
    onchange?.(!value);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (!actionable) return;
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      onchange?.(!value);
    }
  }
</script>

{#if isButton}
  <button
    type="button"
    class="tog tog--button"
    class:on={value}
    class:tog-h={orientation === 'horizontal'}
    class:tog--disabled={disabled}
    role="switch"
    aria-checked={value}
    aria-label={label ?? 'toggle'}
    aria-disabled={disabled ? 'true' : undefined}
    onclick={onClick}
    onkeydown={onKeydown}
  >
    <span class="tog-switch" aria-hidden="true"></span>
    {#if label}<span class="tog-label">{label}</span>{/if}
  </button>
{:else}
  <div
    class="tog"
    class:on={value}
    class:tog-h={orientation === 'horizontal'}
    class:tog--disabled={disabled}
    role="presentation"
  >
    <div class="tog-switch"></div>
    {#if label}<div class="tog-label">{label}</div>{/if}
  </div>
{/if}

<style>
  .tog {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 7px;
    min-width: 40px;
    position: relative;
    padding-top: 4px;
  }
  .tog--button {
    background: none;
    border: 0;
    padding: 4px 0 0;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .tog--button:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
    border-radius: var(--radius-xs);
  }
  .tog--disabled {
    opacity: 0.5;
    pointer-events: none;
  }
  .tog-switch {
    position: relative;
    display: block;
    width: 10px;
    height: 38px;
    background: var(--color-bg-1);
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-1);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
  }
  .tog-switch::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    background: var(--tint, var(--color-primary));
    opacity: 0.95;
    top: 100%;
    bottom: 0;
    transition:
      top 0.1s ease-out,
      bottom 0.1s ease-out;
  }
  .tog-switch::after {
    content: '';
    position: absolute;
    left: -5px;
    right: -5px;
    height: 8px;
    bottom: -4px;
    background: var(--color-text-1);
    border: 1px solid var(--color-line-3);
    border-radius: 1px;
    transition:
      top 0.1s ease-out,
      bottom 0.1s ease-out;
  }
  .tog.on .tog-switch::before {
    top: 0;
    bottom: 0;
  }
  .tog.on .tog-switch::after {
    top: -4px;
    bottom: auto;
  }
  .tog-label {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-1);
    text-align: center;
    line-height: 1.15;
  }
  .tog.tog-h {
    flex-direction: row;
    align-items: center;
    gap: var(--space-md);
    padding-top: 0;
  }
  .tog.tog-h .tog-switch {
    width: 32px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
  }
  .tog.tog-h .tog-switch::before {
    display: none;
  }
  .tog.tog-h .tog-switch::after {
    position: absolute;
    top: 2px;
    left: 2px;
    right: auto;
    bottom: auto;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-text-0);
    border: 1px solid var(--color-line-3);
    transform: translateX(0);
    transition: transform 0.15s ease;
  }
  .tog.tog-h.on .tog-switch {
    background: var(--tint, var(--color-primary));
    border-color: var(--tint, var(--color-primary));
  }
  .tog.tog-h.on .tog-switch::after {
    transform: translateX(14px);
    background: var(--color-on-primary);
    border-color: var(--color-on-primary);
  }
</style>
