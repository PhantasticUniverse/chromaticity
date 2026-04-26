/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */

/*
 * Sources-view fixtures — TypeScript mirror of the v0.2 bundle's
 * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/media.jsx`
 * `MediaManagerView` constants. Powers the W17 PR1 full port that
 * replaces the W16 PR2 `EmptyState` stub with the canonical sidebar
 * + asset grid + inspector layout.
 *
 * Per the auto-memory `feedback_wireframes_normative_for_ui_ux.md`,
 * the bundle JSX is the canonical source; this fixture mirrors it
 * verbatim so future bundle revisions sync cleanly through both
 * surfaces. Where the W17 PR1 brief approximated different counts /
 * names (e.g. "120 assets · 5 collections", `jellyfish_loop_06.mp4`
 * default selection), the bundle's values (247 assets · 9 collections,
 * `jellyfish_loop_04.mp4` default selection) win per the same memory.
 *
 * W17 PR1 posture: hardcoded fixtures only. Real media adapter wiring
 * (filesystem walk, thumbnail extraction, codec / dimensions / duration
 * probe) is `EV-INTEGRATION-MATRIX` Phase-4a scope; the inspector-side
 * preview render gates on the ADR-011 rendering backend (Phase-2+).
 */

/** A logical source collection rendered in the left sidebar. */
export interface SourceCollection {
  readonly id: string;
  /** Display label, may include backslashes for D:\Media etc. */
  readonly name: string;
  /** Asset count chip rendered on the right of each row. */
  readonly count: number;
  /** Glyph hint — single-character placeholder until W17+ adopts the
   * bundle's `<Icon name="..." />` primitive. */
  readonly glyph: string;
  /** Optional capability gate (mirrors the bundle's `data-cap` attr).
   * Present on rows that need an entitlement (camera capture, screen
   * capture); the entitlement infrastructure is Phase-4+ scope. */
  readonly cap?: string;
}

/** Bundle line-by-line:
 *   { n: "All Media",         c: 247, icon: "folder",     active: true },
 *   { n: "Live Sources",      c:   3, icon: "camera",     cap: "io.camera" },
 *   { n: "Favorites",         c:  42, icon: "heart" },
 *   { n: "Recently Added",    c:  14, icon: "refresh" },
 *   { n: "D:\\Media",         c: 136, icon: "folder" },
 *   { n: "D:\\Media\\Loops",  c:  44, icon: "folder" },
 *   { n: "D:\\Media\\Stills", c:  67, icon: "folder" },
 *   { n: "D:\\Stock\\Textures", c: 18, icon: "folder" },
 *   { n: "Desktop Capture",   c:   1, icon: "fullscreen" },
 */
export const SOURCE_COLLECTIONS: readonly SourceCollection[] = [
  { id: 'all-media', name: 'All Media', count: 247, glyph: '▣' },
  { id: 'live-sources', name: 'Live Sources', count: 3, glyph: '◉', cap: 'io.camera' },
  { id: 'favorites', name: 'Favorites', count: 42, glyph: '♡' },
  { id: 'recently-added', name: 'Recently Added', count: 14, glyph: '↻' },
  { id: 'd-media', name: 'D:\\Media', count: 136, glyph: '▤' },
  { id: 'd-media-loops', name: 'D:\\Media\\Loops', count: 44, glyph: '▤' },
  { id: 'd-media-stills', name: 'D:\\Media\\Stills', count: 67, glyph: '▤' },
  { id: 'd-stock-textures', name: 'D:\\Stock\\Textures', count: 18, glyph: '▤' },
  { id: 'desktop-capture', name: 'Desktop Capture', count: 1, glyph: '⛶' },
];

/** Default-active sidebar row per the bundle JSX `active: true` flag. */
export const DEFAULT_COLLECTION_ID = 'all-media';

/** Asset-format filter dropdown vocabulary mirroring the bundle's
 * `<select>` options: `<option>Any type</option><option>Video</option>
 * <option>Image</option><option>Live</option>`. */
export type AssetFormat = 'any' | 'video' | 'loop' | 'image' | 'live';

/** A single media asset rendered in the centre grid. */
export interface MediaAsset {
  readonly id: string;
  /** Display filename (e.g. `jellyfish_loop_04.mp4`). */
  readonly filename: string;
  /** Media kind — drives both the format filter and the meta line. */
  readonly format: Exclude<AssetFormat, 'any'>;
  /** File size string ("35 MB" / "12 MB" / "2 MB" / "—" per bundle). */
  readonly fileSize: string;
  /** Resolution overlay badge ("4K" / "1080p" / "HD" / "720p"). */
  readonly resolution: string;
  /** Duration overlay shown on every third tile per bundle JSX
   * `(i % 3 === 0)`; absent on others. */
  readonly duration?: string;
  /** Per-tile variation seed (drives placeholder gradient hue). */
  readonly seed: number;
}

/* Bundle JSX rotates over a 12-name array for the filename root and a
 * 4-step format / size / resolution rotation. The expressions below
 * mirror the JSX exactly — see `media.jsx` lines 47-65. */

const ASSET_NAMES: readonly string[] = [
  'jellyfish_loop',
  'road_trip_01',
  'sand_dunes',
  'star_trails',
  'glossy_triangles',
  'growing_shapes',
  'mirror_tunnel',
  'noise_02',
  'cloud_atlas',
  'hadopelagic',
  'capybara_01',
  'texture_brushed',
];

const ASSET_FORMATS: readonly Exclude<AssetFormat, 'any'>[] = ['video', 'loop', 'image', 'live'];
const ASSET_SIZES: readonly string[] = ['35 MB', '12 MB', '2 MB', '—'];
const ASSET_RESOLUTIONS: readonly string[] = ['4K', '1080p', 'HD', '720p'];

/** Asset count rendered in the bundle JSX: `Array.from({ length: 32 })`.
 * The grid uses `auto-fill minmax(140px, 1fr)` so the visible row count
 * adapts to viewport width; 32 cells is the canonical fixture size. */
const ASSET_COUNT = 32;

function buildAssets(): readonly MediaAsset[] {
  const out: MediaAsset[] = [];
  for (let i = 0; i < ASSET_COUNT; i += 1) {
    const root = ASSET_NAMES[i % ASSET_NAMES.length] ?? 'asset';
    const format = ASSET_FORMATS[i % 4] ?? 'video';
    const fileSize = ASSET_SIZES[i % 4] ?? '—';
    const resolution = ASSET_RESOLUTIONS[i % 4] ?? 'HD';
    // Bundle JSX: tiles where `i % 3 === 0` carry an inline duration
    // overlay `0:{(12 + i).padStart(2, "0")}`. Other tiles render no
    // duration chip in the grid; the inspector renders its own value.
    const padded = String(i).padStart(2, '0');
    const filename = `${root}_${padded}.mp4`;
    const asset: MediaAsset = {
      id: filename,
      filename,
      format,
      fileSize,
      resolution,
      seed: i * 5 + 2,
      ...(i % 3 === 0 ? { duration: `0:${String(12 + i).padStart(2, '0')}` } : {}),
    };
    out.push(asset);
  }
  return out;
}

export const MEDIA_ASSETS: readonly MediaAsset[] = buildAssets();

/** Default-selected asset id per the bundle JSX inspector header
 * `"Inspector · jellyfish_loop_04.mp4"`. The W17 PR1 brief named
 * `jellyfish_loop_06.mp4` but the bundle wins per the wireframe-
 * alignment memory. */
export const DEFAULT_ASSET_ID = 'jellyfish_loop_04.mp4';

/** Inspector-side metadata for the selected asset. Mirrors the
 * bundle's static six-row table (DIMENSIONS / DURATION / CODEC / SIZE
 * / LOOP / ADDED) verbatim — these values are wireframe-canonical and
 * not derived from the asset row, since the bundle hardcodes them
 * against the default `jellyfish_loop_04.mp4` selection. */
export interface AssetInspectorMeta {
  readonly dimensions: string;
  readonly duration: string;
  readonly codec: string;
  readonly size: string;
  readonly loop: string;
  readonly added: string;
  readonly tags: readonly string[];
}

export const INSPECTOR_META: AssetInspectorMeta = {
  dimensions: '1920 × 1080',
  duration: '00:00:14',
  codec: 'H.264 · 30 fps',
  size: '34.2 MB',
  loop: 'seamless',
  added: 'Apr 14, 2026',
  tags: ['organic', 'blue', 'loop', 'underwater', 'slow'],
};
