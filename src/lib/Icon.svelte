<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts" module>
  // Wave-19 PR3 Icon primitive — verbatim port of the v0.2 bundle's
  // `Icon` component
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 5-66). Every icon's path data matches the bundle byte-for-byte;
  // the per-name `{#if}` ladder mirrors the bundle's `P` lookup object
  // and gives compile-time tree-shaking + type-safe `name` consumers.
  //
  // Locked design choices (per the W19 PR3 brief):
  // - 24×24 viewBox, `stroke="currentColor"`, `stroke-width="1.6"`,
  //   `stroke-linecap="round"`, `stroke-linejoin="round"`, `fill="none"`
  //   are hardcoded; consumers control color via CSS (the SVG inherits
  //   `currentColor`) and size via the `size` prop (default 16, matching
  //   the bundle).
  // - `aria-hidden="true"` by default — icons are decorative when paired
  //   with text labels; consumers needing an accessible icon wrap with
  //   their own `aria-label`.
  // - No `@html` — all paths are literal `<path>` / `<rect>` / `<circle>`
  //   / `<polygon>` / `<polyline>` elements so Svelte's static parser can
  //   tree-shake unused icons and keep SSR / a11y tooling happy.
  //
  // Bundle parity: the 50 names here match `components.jsx` exactly. If
  // the bundle adds icons in a future revision, append to the union and
  // ladder; do not approximate path data — read the bundle source.

  export type IconName =
    | 'perform'
    | 'library'
    | 'midi'
    | 'media'
    | 'settings'
    | 'edit'
    | 'lock'
    | 'help'
    | 'user'
    | 'heart'
    | 'chevron'
    | 'caret'
    | 'play'
    | 'pause'
    | 'stop'
    | 'record'
    | 'prev'
    | 'next'
    | 'shuffle'
    | 'random'
    | 'mic'
    | 'fullscreen'
    | 'camera'
    | 'eye'
    | 'folder'
    | 'plus'
    | 'minus'
    | 'x'
    | 'refresh'
    | 'sliders'
    | 'search'
    | 'grid'
    | 'list'
    | 'link'
    | 'undo'
    | 'trash'
    | 'bang'
    | 'drag'
    | 'beat'
    | 'wave'
    | 'cpu'
    | 'gpu'
    | 'arrow'
    | 'cue'
    | 'flash'
    | 'audio'
    | 'loop'
    | 'deck'
    | 'stems'
    | 'save'
    | 'copy'
    | 'upload'
    | 'note'
    | 'palette'
    | 'layers'
    | 'cancel';
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === 'perform'}
    <rect x="3" y="4" width="18" height="12" rx="1.5" />
    <path d="M8 20h8M10 16v4M14 16v4" />
  {:else if name === 'library'}
    <rect x="3" y="3" width="7" height="7" rx="1" />
    <rect x="14" y="3" width="7" height="7" rx="1" />
    <rect x="3" y="14" width="7" height="7" rx="1" />
    <rect x="14" y="14" width="7" height="7" rx="1" />
  {:else if name === 'midi'}
    <circle cx="12" cy="12" r="8" />
    <circle cx="8" cy="12" r="1" />
    <circle cx="16" cy="12" r="1" />
    <circle cx="12" cy="8" r="1" />
    <circle cx="12" cy="16" r="1" />
  {:else if name === 'media'}
    <rect x="3" y="5" width="18" height="14" rx="1.5" />
    <path d="M10 9l5 3-5 3z" fill="currentColor" stroke="none" />
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3" />
    <path
      d="M19.4 15a1.7 1.7 0 0 0 .3 1.8 2 2 0 1 1-2.8 2.8 1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5 2 2 0 0 1-4 0 1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.8.3 2 2 0 1 1-2.8-2.8 1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1 2 2 0 0 1 0-4 1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.8 2 2 0 1 1 2.8-2.8 1.7 1.7 0 0 0 1.8.3 1.7 1.7 0 0 0 1-1.5 2 2 0 0 1 4 0 1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3 2 2 0 1 1 2.8 2.8 1.7 1.7 0 0 0-.3 1.8 1.7 1.7 0 0 0 1.5 1 2 2 0 0 1 0 4 1.7 1.7 0 0 0-1.5 1z"
    />
  {:else if name === 'edit'}
    <path d="M12 20h9" />
    <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4z" />
  {:else if name === 'lock'}
    <rect x="4" y="11" width="16" height="10" rx="2" />
    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
  {:else if name === 'help'}
    <circle cx="12" cy="12" r="9" />
    <path d="M9.5 9a2.5 2.5 0 0 1 5 0c0 1.5-2.5 2-2.5 3.5M12 17h.01" />
  {:else if name === 'user'}
    <circle cx="12" cy="8" r="4" />
    <path d="M4 21c0-4 4-6 8-6s8 2 8 6" />
  {:else if name === 'heart'}
    <path
      d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.6l-1-1a5.5 5.5 0 1 0-7.8 7.8l1 1L12 21l7.8-7.6 1-1a5.5 5.5 0 0 0 0-7.8z"
    />
  {:else if name === 'chevron'}
    <path d="M9 6l6 6-6 6" />
  {:else if name === 'caret'}
    <path d="M6 9l6 6 6-6" />
  {:else if name === 'play'}
    <path d="M6 4l14 8-14 8z" fill="currentColor" stroke="none" />
  {:else if name === 'pause'}
    <rect x="6" y="4" width="4" height="16" fill="currentColor" />
    <rect x="14" y="4" width="4" height="16" fill="currentColor" />
  {:else if name === 'stop'}
    <rect x="5" y="5" width="14" height="14" fill="currentColor" />
  {:else if name === 'record'}
    <circle cx="12" cy="12" r="7" fill="currentColor" />
  {:else if name === 'prev'}
    <path d="M15 18l-7-6 7-6M5 18V6" />
  {:else if name === 'next'}
    <path d="M9 6l7 6-7 6M19 6v12" />
  {:else if name === 'shuffle'}
    <path d="M16 3h5v5M4 20l17-17M21 16v5h-5M15 15l6 6M4 4l5 5" />
  {:else if name === 'random'}
    <rect x="3" y="3" width="18" height="18" rx="2" />
    <circle cx="8" cy="8" r="1" fill="currentColor" />
    <circle cx="16" cy="16" r="1" fill="currentColor" />
    <circle cx="16" cy="8" r="1" fill="currentColor" />
    <circle cx="8" cy="16" r="1" fill="currentColor" />
  {:else if name === 'mic'}
    <rect x="9" y="3" width="6" height="12" rx="3" />
    <path d="M5 11v1a7 7 0 0 0 14 0v-1M12 19v3" />
  {:else if name === 'fullscreen'}
    <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" />
  {:else if name === 'camera'}
    <path d="M23 7l-7 5 7 5z" />
    <rect x="1" y="5" width="15" height="14" rx="2" />
  {:else if name === 'eye'}
    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
    <circle cx="12" cy="12" r="3" />
  {:else if name === 'folder'}
    <path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  {:else if name === 'plus'}
    <path d="M12 5v14M5 12h14" />
  {:else if name === 'minus'}
    <path d="M5 12h14" />
  {:else if name === 'x'}
    <path d="M6 6l12 12M18 6l-12 12" />
  {:else if name === 'refresh'}
    <path d="M1 4v6h6M23 20v-6h-6" />
    <path d="M20.5 9A9 9 0 0 0 4 6l-3 3M3.5 15A9 9 0 0 0 20 18l3-3" />
  {:else if name === 'sliders'}
    <path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6" />
  {:else if name === 'search'}
    <circle cx="11" cy="11" r="7" />
    <path d="M21 21l-4.3-4.3" />
  {:else if name === 'grid'}
    <rect x="3" y="3" width="7" height="7" rx="1" />
    <rect x="14" y="3" width="7" height="7" rx="1" />
    <rect x="3" y="14" width="7" height="7" rx="1" />
    <rect x="14" y="14" width="7" height="7" rx="1" />
  {:else if name === 'list'}
    <path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" />
  {:else if name === 'link'}
    <path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1" />
    <path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1" />
  {:else if name === 'undo'}
    <path d="M9 14l-4-4 4-4" />
    <path d="M5 10h9a6 6 0 0 1 0 12h-3" />
  {:else if name === 'trash'}
    <path
      d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"
    />
  {:else if name === 'bang'}
    <circle cx="12" cy="12" r="9" />
    <circle cx="12" cy="12" r="4" fill="currentColor" />
  {:else if name === 'drag'}
    <circle cx="9" cy="6" r="1" fill="currentColor" />
    <circle cx="15" cy="6" r="1" fill="currentColor" />
    <circle cx="9" cy="12" r="1" fill="currentColor" />
    <circle cx="15" cy="12" r="1" fill="currentColor" />
    <circle cx="9" cy="18" r="1" fill="currentColor" />
    <circle cx="15" cy="18" r="1" fill="currentColor" />
  {:else if name === 'beat'}
    <path d="M9 18V5l12-2v13" />
    <circle cx="6" cy="18" r="3" />
    <circle cx="18" cy="16" r="3" />
  {:else if name === 'wave'}
    <path d="M2 12c2-4 4-4 6 0s4 4 6 0 4-4 6 0 4 4 2 0" />
  {:else if name === 'cpu'}
    <rect x="4" y="4" width="16" height="16" rx="2" />
    <rect x="9" y="9" width="6" height="6" />
    <path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 15h3M1 9h3M1 15h3" />
  {:else if name === 'gpu'}
    <rect x="2" y="6" width="20" height="12" rx="2" />
    <circle cx="8" cy="12" r="2" />
    <circle cx="16" cy="12" r="2" />
  {:else if name === 'arrow'}
    <path d="M5 12h14M13 6l6 6-6 6" />
  {:else if name === 'cue'}
    <polygon points="5 3 19 12 5 21 5 3" fill="currentColor" stroke="none" />
  {:else if name === 'flash'}
    <path d="M13 2L3 14h7l-1 8 10-12h-7l1-8z" fill="currentColor" stroke="none" />
  {:else if name === 'audio'}
    <path d="M3 12h2l3-9 4 18 3-9h2l3 9h2" />
  {:else if name === 'loop'}
    <polyline points="17 1 21 5 17 9" />
    <path d="M3 11V9a4 4 0 0 1 4-4h14" />
    <polyline points="7 23 3 19 7 15" />
    <path d="M21 13v2a4 4 0 0 1-4 4H3" />
  {:else if name === 'deck'}
    <rect x="3" y="4" width="7" height="16" rx="1" />
    <rect x="14" y="4" width="7" height="16" rx="1" />
  {:else if name === 'stems'}
    <path d="M4 21V10M4 7V3M12 21V14M12 11V3M20 21V16M20 13V3" />
    <rect x="2" y="7" width="4" height="3" rx="0.5" fill="currentColor" />
    <rect x="10" y="11" width="4" height="3" rx="0.5" fill="currentColor" />
    <rect x="18" y="13" width="4" height="3" rx="0.5" fill="currentColor" />
  {:else if name === 'save'}
    <path d="M5 3h11l3 3v13a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" />
    <path d="M7 3v6h9V3M7 21v-7h10v7" />
  {:else if name === 'copy'}
    <rect x="8" y="8" width="13" height="13" rx="2" />
    <path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" />
  {:else if name === 'upload'}
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v13" />
  {:else if name === 'note'}
    <path d="M9 18V5l12-2v13" />
    <circle cx="6" cy="18" r="3" />
    <circle cx="18" cy="16" r="3" />
  {:else if name === 'palette'}
    <circle cx="12" cy="12" r="9" />
    <circle cx="7.5" cy="10.5" r="1" fill="currentColor" />
    <circle cx="12" cy="7.5" r="1" fill="currentColor" />
    <circle cx="16.5" cy="10.5" r="1" fill="currentColor" />
    <path
      d="M17 16a2 2 0 0 1-2 2h-1a2 2 0 0 0-2 2 1 1 0 0 1-1.5.87A9 9 0 0 1 12 3c4.97 0 9 3.58 9 8 0 2.76-2.24 5-5 5z"
      fill="currentColor"
      fill-opacity="0.2"
    />
  {:else if name === 'layers'}
    <polygon points="12 2 2 7 12 12 22 7 12 2" />
    <polyline points="2 17 12 22 22 17" />
    <polyline points="2 12 12 17 22 12" />
  {:else if name === 'cancel'}
    <circle cx="12" cy="12" r="9" />
    <path d="M4.9 4.9l14.2 14.2" />
  {/if}
</svg>
