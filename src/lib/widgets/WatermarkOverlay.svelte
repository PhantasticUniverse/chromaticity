<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-33a chrome primitive: WatermarkOverlay. Display-only port of
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/v02_supplement.jsx`
  // lines 277-286 (the `WatermarkOverlay` component). Bundle DESIGN.md
  // §1 frames the watermark as bottom-right anchored at 24px inset,
  // semi-transparent (~0.50 opacity), single-channel white with a dark
  // outer stroke for legibility against both bright and dark scene
  // content, static (no motion per WCAG).
  //
  // Per CLAUDE.md and `adr/ADR-009-activation-and-entitlements.md`
  // §Capability namespace, the operative watermark in MVP is drawn by
  // the Rust rendering backend on the scene canvas itself when
  // `render.watermark.off` is denied; this UI overlay is the bundle's
  // dev-time visual mock of that "watermark visible" state. AppShell
  // gates the mount via a hardcoded fixture today (`visible={false}`
  // when running with the License view's `activated` fixture); W33b
  // capabilities-store wiring threads the live `render.watermark.off`
  // capability through and inverts it (per `BUNDLE-PROVENANCE-v0.2-beta
  // .md` line 32: "WatermarkOverlay threaded into Perform live Preview
  // via ChromaCtx.licensed"). Real entitlement-token verifier wiring
  // lands in Phase-3+/4a.
  let { visible = true }: { visible?: boolean } = $props();
</script>

{#if visible}
  <div class="watermark" aria-hidden="true">
    <svg width="16" height="16" viewBox="0 0 16 16">
      <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.6" />
    </svg>
    <span>Chromaticity</span>
  </div>
{/if}

<style>
  .watermark {
    position: fixed;
    bottom: 24px;
    right: 24px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    /* Single-channel white text + 2px dark outer drop-shadow for
       legibility against both bright and dark scene content per the
       bundle DESIGN.md §1 contract. */
    color: rgba(255, 255, 255, 0.5);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    text-shadow:
      0 0 2px rgba(0, 0, 0, 0.85),
      0 1px 2px rgba(0, 0, 0, 0.85);
    pointer-events: none;
    user-select: none;
    z-index: 990;
  }
  .watermark svg {
    flex-shrink: 0;
    filter: drop-shadow(0 0 1.5px rgba(0, 0, 0, 0.85));
  }
  /* Static — no motion per WCAG + operator-expectation per the bundle
     DESIGN.md §1. No animation declarations on this element. */
</style>
