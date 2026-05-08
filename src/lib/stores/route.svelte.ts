// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave-12 PR3 app-shell route store. In-memory `$state`-backed singleton
// keyed on a TypeScript string-literal union. NO URL routing — deep links
// and history sync are W13+ if signal warrants. The class instance pattern
// is the Svelte 5 idiomatic approach for shared module-level reactive
// state: `$state` runes inside a class field auto-track on read, and
// importing components react to `routeStore.current` changes without any
// subscription glue.
//
// `.svelte.ts` extension lets the Svelte compiler process `$state` outside
// a `.svelte` component. Plain `.ts` (like `library.ts`) cannot host runes
// — the W11 cache module sidesteps that by holding a non-reactive `Map`.
//
// Route names are explicitly enumerated rather than open-string so the
// SidebarRail's nav button list and AppShell's view dispatch share a
// single source of truth via exhaustiveness checks.
//
// Wave-16 PR2 widens the union from 6 to 10 routes per the v0.2 sidebar
// redesign: `welcome` / `sources` / `audio-uniforms` / `license` are
// new enabled-but-stub routes that mirror the bundle's `VIEWS` array
// (`chromaticity-planning/ui-predev-bundle-v0.2-beta/Chromaticity.html`
// lines 57-71). The bundle's `patch` route stays out of this union —
// the full node-graph port is W17+ scope and the sidebar entry remains
// disabled until then. Member ordering follows the bundle's VIEWS
// declaration order.
//
// Wave-18 PR3 adds `midi` to the union: the W18 MIDI view full port from
// `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/midi.jsx`
// promotes the previously-disabled MIDI sidebar entry to a routed view.
// Insert position mirrors the bundle's VIEWS ordering (between `sources`
// and `osc`).
//
// Wave-19 PR1 adds `patch` to the union: the W19 Patch view full port
// from `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/patch.jsx`
// promotes the previously-disabled Patch sidebar entry to a routed view.
// Insert position mirrors the bundle's VIEWS ordering (after `osc`,
// before `settings`).
//
// Wave-19 PR2 adds `import` to the union: the W19 Import view full port
// from `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/import.jsx`
// promotes the previously-disabled Import sidebar entry to a routed view,
// closing the v0.2 surface inventory. Insert position mirrors the bundle's
// VIEWS ordering (between `osc` and `patch`).
//
// Wave 1d-α PR 1 adds `help` to the union: the in-app embedded
// documentation corpus rails (`docs-corpus/` mkdocs source +
// `dist/help/` static-HTML build output bundled via Tauri
// `bundle.resources`) get a routed view at `src/lib/views/Help.svelte`,
// promoting the previously-disabled bottom-section Help sidebar entry.
// This is the first scaffold PR; corpus content authoring is PR 2+
// scope per `docs/26` row `EV-DOCS-CORPUS`.

export type RouteName =
  | 'perform'
  | 'library'
  | 'author'
  | 'audio-uniforms'
  | 'playlist'
  | 'sources'
  | 'midi'
  | 'osc'
  | 'import'
  | 'patch'
  | 'settings'
  | 'license'
  | 'welcome'
  | 'help';

class RouteStore {
  current: RouteName = $state('perform');

  set(name: RouteName): void {
    this.current = name;
  }
}

export const routeStore = new RouteStore();
