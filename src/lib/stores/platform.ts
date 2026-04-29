// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave-22 PR2 platform-detection helper. Single read of `navigator.userAgent`
// at module load to derive the current desktop platform (`'mac' | 'win'`),
// driving per-platform UI gates such as the W22 Sources view's Spout-on-mac
// disabled-row state.
//
// No runes here — the platform identity is fixed for the process lifetime
// (Tauri rebuilds on platform change), so `.svelte.ts` would be over-spec.
// Plain `.ts` keeps the import lightweight for non-Svelte consumers (eg.
// future Rust-bridge IPC code that might gate on the same enum).
//
// SSR / non-browser environments: `globalThis.navigator` is undefined under
// `tsc --noEmit` and Vitest unless jsdom is loaded. We default to `'mac'`
// when `navigator` is absent — the field is informational for fixtures only;
// real platform-gated behaviour ships through Tauri's `os::platform()` IPC
// boundary in Phase-3+.

export type Platform = 'mac' | 'win';

function detectPlatform(): Platform {
  if (typeof navigator === 'undefined') return 'mac';
  const ua = navigator.userAgent.toLowerCase();
  // Word-boundary match: real Windows UAs carry `Windows NT ...` (the `\b`
  // anchors the start of `windows`), and `Win64`-only UAs still match.
  // The previous `ua.includes('win')` matched `darwin` substring, which
  // misclassified jsdom (UA `darwin`) and Mac/Linux UAs containing the
  // `darwin` token as Windows.
  if (/\bwin/.test(ua)) return 'win';
  return 'mac';
}

export const currentPlatform: Platform = detectPlatform();
