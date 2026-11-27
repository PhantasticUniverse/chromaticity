<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-34b chrome primitive: LockedRow. Display-only port of the
  // bundle's `v02_supplement.jsx::LockedRow` (lines 171-188) — the
  // canonical "feature-locked affordance" treatment for any UI control
  // gated on a denied capability bit. Per `chromaticity-planning/
  // ui-predev-bundle-v0.2-beta/DESIGN.md` §2 the locked treatment is:
  // reduced opacity (0.68), muted color, desaturated fill, `cursor:
  // not-allowed`, trailing 10px lock glyph. Compact callsites may move
  // the lock / text cue into an adjacent label or tooltip but must not
  // rely on color or saturation alone (a11y).
  //
  // Per CLAUDE.md MVP entitlement enforcement is watermark-only;
  // non-watermark capability tags are dormant until post-MVP SKUs
  // activate denied-state styling. Practically that means callers
  // SHOULD NOT preemptively wrap UI in `<LockedRow>` today — the
  // wrapper is built so post-MVP rollouts have a single styling
  // primitive ready to drop in. The W33b `capabilitiesStore` is the
  // source of truth for capability state; the call-site pattern is:
  //
  //   {#if capabilitiesStore.has('io.ndi.input')}
  //     <NdiInputButton />
  //   {:else}
  //     <LockedRow
  //       capability="io.ndi.input"
  //       onNavigate={() => routeStore.set('license')}
  //     >
  //       NDI Input
  //     </LockedRow>
  //   {/if}
  //
  // The widget itself does NOT check the store — it is purely the
  // visual treatment for an already-decided locked state. This
  // matches the bundle JSX behavior verbatim: callers conditionally
  // render `<LockedRow>` only when the capability is denied.

  let {
    label,
    capability,
    onNavigate,
    children,
  }: {
    /** Plain-text affordance label. Either `label` or `children` must
     * be supplied; `children` wins when both are present (bundle
     * parity at line 186). */
    label?: string;
    /** Capability id this row gates on. Surfaced verbatim in the
     * tooltip + aria-describedby so AT users hear the gating
     * predicate. */
    capability: string;
    /** Optional click handler. Bundle pattern: navigate to Settings
     * → License so the operator can install / refresh the token that
     * grants this capability. When omitted the row stays click-inert
     * but keyboard-activatable still surfaces the tooltip. */
    onNavigate?: (() => void) | undefined;
    /** Optional Svelte 5 child snippet for callers that need richer
     * inline content than a plain text label. */
    children?: import('svelte').Snippet;
  } = $props();

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onNavigate?.();
    }
  }

  const tooltip = $derived(`Requires ${capability}. See Settings → License.`);
</script>

<span
  class="locked-row"
  role="button"
  tabindex="0"
  aria-disabled="true"
  aria-label="Locked: requires {capability}"
  title={tooltip}
  onclick={onNavigate ?? undefined}
  onkeydown={handleKeydown}
>
  <svg class="locked-row__lock" viewBox="0 0 12 12" aria-hidden="true">
    <rect
      x="2.5"
      y="5"
      width="7"
      height="5"
      rx="1"
      fill="none"
      stroke="currentColor"
      stroke-width="1.2"
    />
    <path d="M4 5V3.5a2 2 0 014 0V5" fill="none" stroke="currentColor" stroke-width="1.2" />
  </svg>
  <span class="locked-row__label">
    {#if children}{@render children()}{:else}{label ?? ''}{/if}
  </span>
</span>

<style>
  .locked-row {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: var(--radius-xs);
    /* Per bundle DESIGN.md §2: opacity 0.68, muted color, cursor
       not-allowed. The desaturation arrives via the muted-text token
       cascade rather than a `filter: saturate()` so the host theme's
       color choice still reads through. */
    opacity: 0.68;
    color: var(--color-text-2);
    background: transparent;
    border: 1px dashed var(--color-line-2);
    font-family: inherit;
    font-size: var(--font-size-body-sm);
    cursor: not-allowed;
    user-select: none;
  }
  .locked-row:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .locked-row__lock {
    width: 10px;
    height: 10px;
    flex-shrink: 0;
    color: var(--color-text-3);
  }
</style>
